//! Nora mode handlers: list/apply modes, rapid playbook, graph plans

use super::*;
use super::coordination::emit_coordination_event;

pub async fn list_modes_handler() -> Json<Vec<NoraModeSummary>> {
    let summaries = NORA_MODE_PRESETS
        .iter()
        .map(|preset| NoraModeSummary {
            id: preset.id,
            label: preset.label,
            description: preset.description,
        })
        .collect();
    Json(summaries)
}

pub async fn apply_mode_handler(
    Json(body): Json<ApplyModeRequest>,
) -> Result<Json<ApplyModeResponse>, ApiError> {
    let manager = NoraManager::new().await;
    let preset = NORA_MODE_PRESETS
        .iter()
        .find(|preset| preset.id == body.mode_id)
        .ok_or_else(|| ApiError::BadRequest("Unknown Nora mode".to_string()))?;

    let nora_id = manager
        .reinitialize_with_config(preset.config.clone(), body.preserve_memory)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApplyModeResponse {
        active_mode: preset.label.to_string(),
        nora_id,
    }))
}

pub async fn rapid_playbook_handler(
    Json(body): Json<RapidPlaybookBody>,
) -> Result<Json<RapidPlaybookResult>, ApiError> {
    let manager = NoraManager::new().await;
    let payload = RapidPlaybookRequest {
        project_name: body.project_name,
        objectives: body.objectives,
        repo_hint: body.repo_hint,
        notes: body.notes,
    };

    let result = manager
        .run_rapid_playbook(payload)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(result))
}

pub async fn list_graph_plans_handler(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<Vec<GraphPlanSummary>>, ApiError> {
    let manager = NoraManager::new().await;
    let plans = manager
        .list_graph_plans()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(plans))
}

pub async fn get_graph_plan_handler(
    State(_state): State<DeploymentImpl>,
    Path(plan_id): Path<String>,
) -> Result<Json<GraphPlan>, ApiError> {
    let manager = NoraManager::new().await;
    let plan = manager
        .get_graph_plan(&plan_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(plan))
}

pub async fn update_graph_node_handler(
    State(_state): State<DeploymentImpl>,
    Path((plan_id, node_id)): Path<(String, String)>,
    Json(body): Json<UpdateNodeStatusBody>,
) -> Result<Json<GraphPlan>, ApiError> {
    let manager = NoraManager::new().await;
    let plan = manager
        .update_graph_node_status(&plan_id, &node_id, body.status.clone())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    if let Some(node) = plan.nodes.iter().find(|node| node.id == node_id) {
        emit_coordination_event(CoordinationEvent::AgentDirectiveIssued {
            agent_id: node
                .agent
                .clone()
                .unwrap_or_else(|| "NORA_GRAPH".to_string()),
            issued_by: "NORA_GRAPH".to_string(),
            content: format!(
                "Node '{}' advanced to {:?}",
                node.label, body.status
            ),
            priority: Some(format!("{:?}", body.status)),
            timestamp: Utc::now(),
        })
        .await;
    }

    Ok(Json(plan))
}
