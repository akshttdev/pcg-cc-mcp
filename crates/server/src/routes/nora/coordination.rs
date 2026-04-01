//! Nora coordination handlers: stats, agents, directives, SSE/WebSocket

use axum::{
    extract::WebSocketUpgrade,
    response::{
        sse::{Event, KeepAlive, Sse},
        Response,
    },
};
use futures::stream::Stream;

use super::*;

/// Get coordination statistics
pub async fn get_coordination_stats(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<CoordinationStats>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let stats = nora
        .coordination_manager
        .get_coordination_stats()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get coordination stats: {}", e)))?;

    Ok(Json(stats))
}

/// Get coordination agent list
pub async fn get_coordination_agents(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<Vec<AgentCoordinationState>>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let agents = nora
        .coordination_manager
        .get_all_agents()
        .await
        .map_err(|e| {
            ApiError::InternalError(format!("Failed to get coordination agents: {}", e))
        })?;

    Ok(Json(agents))
}

/// Send directives to specific agents (non-Nora) via the global console
pub async fn send_agent_directive(
    Path(agent_id): Path<String>,
    State(_state): State<DeploymentImpl>,
    Json(request): Json<AgentDirectiveRequest>,
) -> Result<Json<AgentDirectiveResponse>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    let priority_label = request.priority.as_ref().map(|priority| match priority {
        RequestPriority::Low => "low".to_string(),
        RequestPriority::Normal => "normal".to_string(),
        RequestPriority::High => "high".to_string(),
        RequestPriority::Urgent => "urgent".to_string(),
        RequestPriority::Executive => "executive".to_string(),
    });

    let coordination_manager = nora.coordination_manager.clone();
    let agent_state = coordination_manager
        .get_agent(&agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to lookup agent: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", agent_id)))?;

    coordination_manager
        .record_directive(
            &agent_id,
            &request.session_id,
            &request.content,
            priority_label.clone(),
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to record directive: {}", e)))?;

    let acknowledgement = format!(
        "{} acknowledges directive and is prioritizing: {}",
        agent_state.agent_type, request.content
    );

    let echoed_command = request
        .command
        .clone()
        .filter(|cmd| !cmd.is_empty())
        .unwrap_or_else(|| request.content.clone());

    let response = AgentDirectiveResponse {
        agent_id,
        agent_label: agent_state.agent_type,
        acknowledgement,
        echoed_command,
        priority: priority_label,
        timestamp: Utc::now(),
    };

    Ok(Json(response))
}

/// WebSocket endpoint for coordination events
pub async fn get_coordination_events_ws(
    ws: WebSocketUpgrade,
    State(_state): State<DeploymentImpl>,
) -> Response {
    ws.on_upgrade(handle_coordination_events_websocket)
}

/// SSE endpoint for coordination events (fallback when WebSockets are unavailable)
pub async fn get_coordination_events_sse(
    State(_state): State<DeploymentImpl>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError> {
    let nora_instance = get_nora_instance().await?;
    let receiver = {
        let instance = nora_instance.read().await;
        let nora = instance
            .as_ref()
            .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;
        nora.coordination_manager.subscribe_to_events().await
    };

    let stream = futures::stream::unfold(receiver, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let payload = coordination_event_payload(event);
                    let data = payload.to_string();
                    return Some((
                        Ok(Event::default().event("coordination_event").data(data)),
                        rx,
                    ));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::new()))
}

/// Emit a coordination event so other routes can surface activity without recreating handles
pub async fn emit_coordination_event(event: CoordinationEvent) {
    if let Some(instance) = NORA_INSTANCE.get() {
        let agent = instance.read().await;
        if let Some(nora) = agent.as_ref() {
            if let Err(err) = nora.coordination_manager.emit_event(event).await {
                tracing::debug!("Failed to emit coordination event: {}", err);
            }
        }
    }
}

// ── Internal helpers ───────────────────────────────────────────────────

async fn handle_coordination_events_websocket(mut socket: axum::extract::ws::WebSocket) {
    tracing::info!("Coordination events WebSocket connection established");

    let Ok(nora_instance) = get_nora_instance().await else {
        let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
        return;
    };

    let receiver_result = {
        let instance = nora_instance.read().await;
        if let Some(nora) = instance.as_ref() {
            Ok(nora.coordination_manager.subscribe_to_events().await)
        } else {
            Err(ApiError::NotFound("Nora not initialized".to_string()))
        }
    };

    let mut receiver = match receiver_result {
        Ok(rx) => rx,
        Err(_) => {
            let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
            return;
        }
    };

    loop {
        tokio::select! {
            socket_msg = socket.recv() => {
                match socket_msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(axum::extract::ws::Message::Ping(p))) => {
                        if socket.send(axum::extract::ws::Message::Pong(p)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        tracing::warn!("Coordination WebSocket receive error: {}", e);
                        break;
                    }
                }
            }
            event = receiver.recv() => {
                match event {
                    Ok(event) => {
                        let payload = coordination_event_payload(event);
                        let text = payload.to_string();
                        if socket
                            .send(axum::extract::ws::Message::Text(text.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!("Coordination event receiver lagged by {} messages", skipped);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Coordination event channel closed");
                        break;
                    }
                }
            }
        }
    }

    let _ = socket.send(axum::extract::ws::Message::Close(None)).await;
}

pub(crate) fn coordination_event_payload(event: CoordinationEvent) -> serde_json::Value {
    match event {
        CoordinationEvent::AgentStatusUpdate {
            agent_id,
            status,
            capabilities,
            timestamp,
        } => json!({
            "type": "AgentStatusUpdate",
            "agentId": agent_id,
            "status": status,
            "capabilities": capabilities,
            "timestamp": timestamp,
        }),
        CoordinationEvent::TaskHandoff {
            from_agent,
            to_agent,
            task_id,
            context,
            timestamp,
        } => json!({
            "type": "TaskHandoff",
            "fromAgent": from_agent,
            "toAgent": to_agent,
            "taskId": task_id,
            "context": context,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ConflictResolution {
            conflict_id,
            involved_agents,
            description,
            priority,
            timestamp,
        } => json!({
            "type": "ConflictResolution",
            "conflictId": conflict_id,
            "involvedAgents": involved_agents,
            "description": description,
            "priority": priority,
            "timestamp": timestamp,
        }),
        CoordinationEvent::HumanAvailabilityUpdate {
            user_id,
            availability,
            available_until,
            timestamp,
        } => json!({
            "type": "HumanAvailabilityUpdate",
            "userId": user_id,
            "availability": availability,
            "availableUntil": available_until,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ApprovalRequest {
            request_id,
            requesting_agent,
            action_description,
            required_approver,
            urgency,
            timestamp,
        } => json!({
            "type": "ApprovalRequest",
            "requestId": request_id,
            "requestingAgent": requesting_agent,
            "actionDescription": action_description,
            "requiredApprover": required_approver,
            "urgency": urgency,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutiveAlert {
            alert_id,
            source,
            message,
            severity,
            requires_action,
            timestamp,
        } => json!({
            "type": "ExecutiveAlert",
            "alertId": alert_id,
            "source": source,
            "message": message,
            "severity": severity,
            "requiresAction": requires_action,
            "timestamp": timestamp,
        }),
        CoordinationEvent::AgentDirectiveIssued {
            agent_id,
            issued_by,
            content,
            priority,
            timestamp,
        } => json!({
            "type": "AgentDirective",
            "agentId": agent_id,
            "issuedBy": issued_by,
            "content": content,
            "priority": priority,
            "timestamp": timestamp,
        }),
        CoordinationEvent::WorkflowProgress {
            workflow_instance_id,
            agent_id,
            agent_codename,
            workflow_name,
            current_stage,
            total_stages,
            stage_name,
            status,
            project_id,
            timestamp,
        } => json!({
            "type": "WorkflowProgress",
            "workflowInstanceId": workflow_instance_id,
            "agentId": agent_id,
            "agentCodename": agent_codename,
            "workflowName": workflow_name,
            "currentStage": current_stage,
            "totalStages": total_stages,
            "stageName": stage_name,
            "status": status,
            "projectId": project_id,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionStarted {
            execution_id,
            project_id,
            agent_codename,
            workflow_name,
            timestamp,
        } => json!({
            "type": "ExecutionStarted",
            "executionId": execution_id,
            "projectId": project_id,
            "agentCodename": agent_codename,
            "workflowName": workflow_name,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionStageStarted {
            execution_id,
            stage_index,
            stage_name,
            agent_codename,
            timestamp,
        } => json!({
            "type": "ExecutionStageStarted",
            "executionId": execution_id,
            "stageIndex": stage_index,
            "stageName": stage_name,
            "agentCodename": agent_codename,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionStageCompleted {
            execution_id,
            stage_index,
            stage_name,
            output_summary,
            timestamp,
        } => json!({
            "type": "ExecutionStageCompleted",
            "executionId": execution_id,
            "stageIndex": stage_index,
            "stageName": stage_name,
            "outputSummary": output_summary,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionCompleted {
            execution_id,
            project_id,
            tasks_created,
            artifacts_count,
            duration_ms,
            timestamp,
        } => json!({
            "type": "ExecutionCompleted",
            "executionId": execution_id,
            "projectId": project_id,
            "tasksCreated": tasks_created,
            "artifactsCount": artifacts_count,
            "durationMs": duration_ms,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionFailed {
            execution_id,
            error,
            stage,
            timestamp,
        } => json!({
            "type": "ExecutionFailed",
            "executionId": execution_id,
            "error": error,
            "stage": stage,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionTaskCreated {
            execution_id,
            task_id,
            task_title,
            board_id,
            timestamp,
        } => json!({
            "type": "ExecutionTaskCreated",
            "executionId": execution_id,
            "taskId": task_id,
            "taskTitle": task_title,
            "boardId": board_id,
            "timestamp": timestamp,
        }),
        CoordinationEvent::ExecutionArtifactProduced {
            execution_id,
            artifact_type,
            title,
            stage,
            timestamp,
        } => json!({
            "type": "ExecutionArtifactProduced",
            "executionId": execution_id,
            "artifactType": artifact_type,
            "title": title,
            "stage": stage,
            "timestamp": timestamp,
        }),
    }
}
