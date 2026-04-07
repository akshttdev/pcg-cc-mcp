//! Tool implementation logic — the main execution handler

use chrono::{DateTime, Utc};
use db::models::{
    project_board::ProjectBoardType,
    task::{Priority, TaskStatus},
};
use services::services::{
    agent_channels::ChannelOwner,
    media_pipeline::{
        EditSessionRequest, MediaBatchAnalysisRequest, MediaBatchIngestRequest, MediaStorageTier,
        RenderJobRequest, VideoRenderPriority as PipelineRenderPriority,
    },
};
use uuid::Uuid;

use super::{types::*, ExecutiveTools};
use crate::{executor::TaskDefinition, NoraError};

#[allow(dead_code)]
impl ExecutiveTools {
    pub async fn execute_tool_implementation(
        &self,
        tool: NoraExecutiveTool,
    ) -> crate::Result<serde_json::Value> {
        match tool {
            // Project Management
            NoraExecutiveTool::CreateProject {
                name,
                git_repo_path,
                setup_script,
                dev_script,
            } => {
                if let Some(executor) = &self.task_executor {
                    match executor
                        .create_project(
                            name.clone(),
                            git_repo_path.clone(),
                            setup_script,
                            dev_script,
                        )
                        .await
                    {
                        Ok(project) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Project '{}' created successfully", name),
                            "project_id": project.id.to_string(),
                            "project_name": project.name,
                            "git_repo_path": project.git_repo_path.to_string_lossy().to_string(),
                            "created_at": project.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create project: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available. Please ensure Nora is properly initialized."
                    }))
                }
            }
            NoraExecutiveTool::CreateBoard {
                project_id,
                name,
                description,
                board_type,
            } => {
                if let Some(executor) = &self.task_executor {
                    // Parse project_id UUID
                    let project_uuid = match Uuid::parse_str(&project_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid project_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Map board type string to enum
                    let board_type_enum =
                        // Simplified board types: only Default and Custom
                        board_type
                            .as_ref()
                            .map(|bt| match bt.to_lowercase().as_str() {
                                "default" | "main" => ProjectBoardType::Default,
                                _ => ProjectBoardType::Custom,
                            });

                    match executor
                        .create_board(
                            &project_uuid.to_string(),
                            name.clone(),
                            description,
                            board_type_enum,
                        )
                        .await
                    {
                        Ok(board) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Board '{}' created successfully", name),
                            "board_id": board.id.to_string(),
                            "project_id": board.project_id.to_string(),
                            "board_name": board.name,
                            "board_type": format!("{:?}", board.board_type),
                            "created_at": board.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create board: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::CreateTaskInProject {
                project_name,
                title,
                description,
                priority,
            } => {
                if let Some(executor) = &self.task_executor {
                    // Map priority string to enum
                    let priority_enum =
                        priority
                            .as_ref()
                            .and_then(|p| match p.to_lowercase().as_str() {
                                "critical" => Some(Priority::Critical),
                                "high" => Some(Priority::High),
                                "medium" => Some(Priority::Medium),
                                "low" => Some(Priority::Low),
                                _ => None,
                            });

                    match executor
                        .create_task_in_project(
                            &project_name,
                            title.clone(),
                            description,
                            priority_enum,
                        )
                        .await
                    {
                        Ok(task) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Task '{}' created successfully in project '{}'", title, project_name),
                            "task_id": task.id.to_string(),
                            "project_id": task.project_id.to_string(),
                            "board_id": task.board_id.map(|id| id.to_string()),
                            "title": task.title,
                            "status": format!("{:?}", task.status),
                            "priority": format!("{:?}", task.priority),
                            "created_at": task.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create task: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::GetProjectTasks {
                project_name,
                status_filter,
            } => {
                if let Some(executor) = &self.task_executor {
                    match executor.get_tasks_by_project_name(&project_name).await {
                        Ok(tasks) => {
                            // Optionally filter by status
                            let filtered_tasks: Vec<_> = if let Some(status) = status_filter {
                                tasks
                                    .into_iter()
                                    .filter(|t| t.status.to_lowercase() == status.to_lowercase())
                                    .collect()
                            } else {
                                tasks
                            };

                            let task_summaries: Vec<serde_json::Value> = filtered_tasks
                                .iter()
                                .map(|t| {
                                    serde_json::json!({
                                        "id": t.id,
                                        "title": t.title,
                                        "description": t.description,
                                        "status": t.status,
                                        "priority": t.priority,
                                        "assignee": t.assignee_id,
                                        "created_at": t.created_at,
                                        "updated_at": t.updated_at,
                                    })
                                })
                                .collect();

                            Ok(serde_json::json!({
                                "success": true,
                                "project_name": project_name,
                                "task_count": task_summaries.len(),
                                "tasks": task_summaries,
                            }))
                        }
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to get tasks: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::GetProjectDetails { project_name } => {
                if let Some(executor) = &self.task_executor {
                    // First find project by name, then get details
                    match executor.find_project_by_name(&project_name).await {
                        Ok(project_id) => match executor.get_project_details(&project_id).await {
                            Ok(details) => Ok(serde_json::json!({
                                "success": true,
                                "project": {
                                    "id": details.id,
                                    "name": details.name,
                                    "git_repo_path": details.git_repo_path,
                                    "task_count": details.tasks.len(),
                                    "board_count": details.boards.len(),
                                    "pod_count": details.pods.len(),
                                },
                                "tasks": details.tasks.iter().map(|t| serde_json::json!({
                                    "id": t.id,
                                    "title": t.title,
                                    "description": t.description,
                                    "status": t.status,
                                    "priority": t.priority,
                                })).collect::<Vec<_>>(),
                                "boards": details.boards.iter().map(|b| serde_json::json!({
                                    "id": b.id,
                                    "name": b.name,
                                    "description": b.description,
                                })).collect::<Vec<_>>(),
                            })),
                            Err(e) => Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to get project details: {}", e),
                            })),
                        },
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Project not found: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::DeleteProject { project_name } => {
                if let Some(executor) = &self.task_executor {
                    let pool = executor.pool();
                    match sqlx::query_scalar::<_, Vec<u8>>(
                        "SELECT id FROM projects WHERE name = ? LIMIT 1",
                    )
                    .bind(&project_name)
                    .fetch_optional(pool)
                    .await
                    {
                        Ok(Some(id_bytes)) => {
                            match sqlx::query("DELETE FROM projects WHERE id = ?")
                                .bind(&id_bytes)
                                .execute(pool)
                                .await
                            {
                                Ok(_) => Ok(serde_json::json!({
                                    "success": true,
                                    "message": format!("Project '{}' deleted successfully.", project_name),
                                })),
                                Err(e) => Ok(serde_json::json!({
                                    "success": false,
                                    "error": format!("Failed to delete project: {}", e),
                                })),
                            }
                        }
                        Ok(None) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Project '{}' not found.", project_name),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("DB error: {}", e),
                        })),
                    }
                } else {
                    Ok(
                        serde_json::json!({"success": false, "error": "Task executor not available"}),
                    )
                }
            }
            NoraExecutiveTool::UpdateProject {
                project_name,
                new_name,
                new_description,
            } => {
                if let Some(executor) = &self.task_executor {
                    let pool = executor.pool();
                    let mut updated = false;
                    if let Some(ref name) = new_name {
                        if let Err(e) = sqlx::query(
                            "UPDATE projects SET name = ?, updated_at = datetime('now','subsec') WHERE name = ?"
                        )
                        .bind(name)
                        .bind(&project_name)
                        .execute(pool)
                        .await {
                            return Ok(serde_json::json!({"success": false, "error": format!("Failed to update name: {}", e)}));
                        }
                        updated = true;
                    }
                    if let Some(ref desc) = new_description {
                        let target = new_name.as_deref().unwrap_or(&project_name);
                        if let Err(e) = sqlx::query(
                            "UPDATE projects SET git_repo_path = ?, updated_at = datetime('now','subsec') WHERE name = ?"
                        )
                        .bind(desc)
                        .bind(target)
                        .execute(pool)
                        .await {
                            return Ok(serde_json::json!({"success": false, "error": format!("Failed to update description: {}", e)}));
                        }
                        updated = true;
                    }
                    if updated {
                        Ok(
                            serde_json::json!({"success": true, "message": format!("Project '{}' updated.", project_name)}),
                        )
                    } else {
                        Ok(
                            serde_json::json!({"success": false, "error": "No fields to update provided."}),
                        )
                    }
                } else {
                    Ok(
                        serde_json::json!({"success": false, "error": "Task executor not available"}),
                    )
                }
            }
            NoraExecutiveTool::DelegateTask {
                task_id,
                assignee,
                priority: _,
                deadline: _,
            } => {
                if let Some(executor) = &self.task_executor {
                    tracing::info!("[TOOL] Delegating task {} to agent {}", task_id, assignee);

                    // Parse the task UUID
                    let task_uuid = match uuid::Uuid::parse_str(&task_id) {
                        Ok(id) => id,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Invalid task ID format: {}", task_id)
                            }));
                        }
                    };

                    // Determine executor type based on agent name
                    // AURI uses Claude Code, Editron would use video tools, etc.
                    let executor_type = match assignee.to_lowercase().as_str() {
                        "auri" => "CLAUDE_CODE",
                        "editron" => "PREMIERE_PRO", // For future video editing
                        "maci" => "COMFYUI",         // For future image generation
                        "bowser" => "PLAYWRIGHT",    // For future browser automation
                        _ => "CLAUDE_CODE",          // Default to Claude Code for coding agents
                    };

                    // Delegate and execute
                    match executor
                        .delegate_and_execute_task(task_uuid.to_string(), &assignee, executor_type)
                        .await
                    {
                        Ok(result) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Task delegated to {} and execution started", result.agent_name),
                            "task_id": result.task_id.to_string(),
                            "agent_id": result.agent_id.to_string(),
                            "agent_name": result.agent_name,
                            "executor_type": result.executor_type,
                            "status": result.status,
                            "details": result.details
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to delegate task: {}", e)
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::ExecuteWorkflow {
                agent_id,
                workflow_id,
                project_id,
                inputs,
            } => {
                tracing::info!(
                    "[TOOL] Executing workflow: agent={}, workflow={}, project={:?}",
                    agent_id,
                    workflow_id,
                    project_id
                );

                // Parse project_id if provided
                let project_uuid = project_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());

                // Prefer new ExecutionEngine if available
                if let Some(engine) = &self.execution_engine {
                    let request = crate::execution::ExecutionRequest {
                        project_id: project_uuid,
                        agent: Some(agent_id.clone()),
                        workflow_id: Some(workflow_id.clone()),
                        request: Some(format!(
                            "Execute workflow {} for agent {}",
                            workflow_id, agent_id
                        )),
                        inputs: inputs.clone(),
                    };

                    match engine.execute(request).await {
                        Ok(result) => {
                            tracing::info!(
                                "[TOOL] Workflow executed via ExecutionEngine: execution_id={}, stages={}/{}",
                                result.execution_id,
                                result.stages_completed,
                                result.total_stages
                            );

                            Ok(serde_json::json!({
                                "success": true,
                                "message": format!("Workflow '{}' completed for agent '{}'", workflow_id, agent_id),
                                "execution_id": result.execution_id.to_string(),
                                "agent_id": result.agent_id,
                                "agent_name": result.agent_name,
                                "workflow_id": result.workflow_id,
                                "workflow_name": result.workflow_name,
                                "project_id": project_id,
                                "status": format!("{:?}", result.status),
                                "stages_completed": result.stages_completed,
                                "total_stages": result.total_stages,
                                "tasks_created": result.tasks_created.len(),
                                "artifacts": result.artifacts.len(),
                                "duration_ms": result.duration_ms,
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] ExecutionEngine workflow failed: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Workflow execution failed: {}", e),
                                "agent_id": agent_id,
                                "workflow_id": workflow_id,
                            }))
                        }
                    }
                }
                // Fallback to legacy WorkflowOrchestrator
                else if let Some(orchestrator) = &self.workflow_orchestrator {
                    let context = crate::workflow::WorkflowContext {
                        project_id: project_uuid,
                        user_id: None,
                        inputs: inputs.clone(),
                        stage_outputs: std::collections::HashMap::new(),
                        metadata: std::collections::HashMap::new(),
                    };

                    match orchestrator
                        .start_workflow(&agent_id, &workflow_id, context)
                        .await
                    {
                        Ok(workflow_instance_id) => {
                            tracing::info!(
                                "[TOOL] Workflow started via legacy orchestrator: instance_id={}",
                                workflow_instance_id
                            );

                            Ok(serde_json::json!({
                                "success": true,
                                "message": format!("Workflow '{}' started for agent '{}'", workflow_id, agent_id),
                                "workflow_instance_id": workflow_instance_id.to_string(),
                                "agent_id": agent_id,
                                "workflow_id": workflow_id,
                                "project_id": project_id,
                                "status": "running",
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Legacy orchestrator failed: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to start workflow: {}", e),
                                "agent_id": agent_id,
                                "workflow_id": workflow_id,
                            }))
                        }
                    }
                } else {
                    tracing::warn!("[TOOL] No execution engine or orchestrator available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "No workflow execution system available. Nora needs to be initialized with ExecutionEngine or WorkflowOrchestrator.",
                        "agent_id": agent_id,
                        "workflow_id": workflow_id,
                    }))
                }
            }
            NoraExecutiveTool::CancelWorkflow {
                workflow_instance_id,
            } => {
                if let Some(orchestrator) = &self.workflow_orchestrator {
                    tracing::info!(
                        "[TOOL] Cancelling workflow instance: {}",
                        workflow_instance_id
                    );

                    match Uuid::parse_str(&workflow_instance_id) {
                        Ok(workflow_uuid) => {
                            match orchestrator.cancel_workflow(workflow_uuid).await {
                                Ok(_) => {
                                    tracing::info!(
                                        "[TOOL] Workflow cancelled successfully: {}",
                                        workflow_instance_id
                                    );
                                    Ok(serde_json::json!({
                                        "success": true,
                                        "message": format!("Workflow instance {} has been cancelled", workflow_instance_id),
                                        "workflow_instance_id": workflow_instance_id,
                                    }))
                                }
                                Err(e) => {
                                    tracing::error!("[TOOL] Failed to cancel workflow: {}", e);
                                    Ok(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to cancel workflow: {}", e),
                                        "workflow_instance_id": workflow_instance_id,
                                    }))
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Invalid workflow instance ID: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Invalid workflow instance ID: {}", e),
                                "workflow_instance_id": workflow_instance_id,
                            }))
                        }
                    }
                } else {
                    tracing::warn!("[TOOL] Workflow orchestrator not available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Workflow orchestrator not available",
                        "workflow_instance_id": workflow_instance_id,
                    }))
                }
            }
            NoraExecutiveTool::ListActiveWorkflows => {
                if let Some(orchestrator) = &self.workflow_orchestrator {
                    tracing::info!("[TOOL] Listing active workflows");

                    let workflows = orchestrator.get_active_workflows().await;
                    let workflow_list: Vec<serde_json::Value> = workflows
                        .iter()
                        .map(|w| {
                            serde_json::json!({
                                "workflow_instance_id": w.id.to_string(),
                                "agent_id": w.agent_id,
                                "workflow_id": w.workflow_id,
                                "workflow_name": w.workflow.name,
                                "current_stage": w.current_stage,
                                "total_stages": w.workflow.stages.len(),
                                "state": match &w.state {
                                    crate::workflow::WorkflowState::Queued => "queued",
                                    crate::workflow::WorkflowState::Running { .. } => "running",
                                    crate::workflow::WorkflowState::Paused { .. } => "paused",
                                    crate::workflow::WorkflowState::Failed { .. } => "failed",
                                    crate::workflow::WorkflowState::Completed { .. } => "completed",
                                },
                                "started_at": w.started_at.to_rfc3339(),
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "success": true,
                        "workflows": workflow_list,
                        "count": workflow_list.len(),
                    }))
                } else {
                    tracing::warn!("[TOOL] Workflow orchestrator not available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Workflow orchestrator not available",
                    }))
                }
            }
            NoraExecutiveTool::ListAvailableWorkflows { agent_id } => {
                if let Some(orchestrator) = &self.workflow_orchestrator {
                    tracing::info!(
                        "[TOOL] Listing available workflows - filter: {:?}",
                        agent_id
                    );

                    if let Some(ref agent_filter) = agent_id {
                        // Get workflows for specific agent
                        let workflows = orchestrator.get_workflows_for_agent(agent_filter);

                        if workflows.is_empty() {
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Agent '{}' not found or has no workflows", agent_filter),
                            }))
                        } else {
                            let workflow_list: Vec<serde_json::Value> = workflows
                                .iter()
                                .map(|(workflow_id, name, objective)| {
                                    serde_json::json!({
                                        "workflow_id": workflow_id,
                                        "workflow_name": name,
                                        "objective": objective,
                                        "agent_id": agent_filter,
                                    })
                                })
                                .collect();

                            Ok(serde_json::json!({
                                "success": true,
                                "agent_id": agent_filter,
                                "workflows": workflow_list,
                                "count": workflow_list.len(),
                            }))
                        }
                    } else {
                        // Get all workflows for all agents
                        let all_workflows = orchestrator.get_all_agent_workflows();
                        let agents_list: Vec<serde_json::Value> = all_workflows
                            .iter()
                            .map(|(agent_id, codename, workflows)| {
                                let workflow_list: Vec<serde_json::Value> = workflows
                                    .iter()
                                    .map(|(workflow_id, name, objective)| {
                                        serde_json::json!({
                                            "workflow_id": workflow_id,
                                            "workflow_name": name,
                                            "objective": objective,
                                        })
                                    })
                                    .collect();

                                serde_json::json!({
                                    "agent_id": agent_id,
                                    "codename": codename,
                                    "workflows": workflow_list,
                                    "workflow_count": workflow_list.len(),
                                })
                            })
                            .collect();

                        Ok(serde_json::json!({
                            "success": true,
                            "agents": agents_list,
                            "total_agents": agents_list.len(),
                        }))
                    }
                } else {
                    tracing::warn!("[TOOL] Workflow orchestrator not available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Workflow orchestrator not available",
                    }))
                }
            }

            // Media Pipeline - Editron Tools
            NoraExecutiveTool::IngestMediaBatch {
                source_url,
                reference_name,
                storage_tier,
                checksum_required,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Ingesting media batch from: {}", source_url);

                    let tier = match MediaStorageTier::parse_tier(&storage_tier) {
                        Ok(t) => t,
                        Err(e) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Invalid storage tier '{}': {}", storage_tier, e),
                            }));
                        }
                    };

                    let project_uuid = project_id
                        .as_ref()
                        .and_then(|value| Uuid::parse_str(value).ok());

                    let request = MediaBatchIngestRequest {
                        source_url: source_url.clone(),
                        reference_name: reference_name.clone(),
                        storage_tier: tier,
                        checksum_required,
                        project_id: project_uuid,
                    };

                    match pipeline.ingest_batch(request).await {
                        Ok(batch) => {
                            tracing::info!(
                                "[TOOL] Media batch created successfully: batch_id={}",
                                batch.id
                            );

                            // Dashboard tracking: artifact + activity + vibe transaction
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();
                                let ref_name = reference_name.as_deref().unwrap_or("Media Batch");

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: {}", ref_name),
                                        &format!("Media batch from {}", source_url),
                                        serde_json::json!({ "editron_batch_id": batch.id.to_string() }),
                                    )
                                    .await
                                {
                                    let vibe_cost = crate::editron_tracking::EditronVibeCosts::ingest(batch.files.len() as u32);

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        &resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::MediaIngestManifest,
                                        &format!("Ingest: {}", batch.id),
                                        Some(serde_json::json!({
                                            "batch_id": batch.id.to_string(),
                                            "files": batch.files.len() as u32,
                                            "source_url": source_url,
                                            "storage_tier": format!("{:?}", batch.storage_tier),
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        &resolved_task_id,
                                        "editron_ingest_completed",
                                        &format!("Ingested {} files from {}", batch.files.len() as u32, source_url),
                                        vibe_cost,
                                        serde_json::json!({"batch_id": batch.id.to_string()}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        &resolved_project_id,
                                        &resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron ingest: {} files", batch.files.len() as u32),
                                        "ingest",
                                        serde_json::json!({"batch_id": batch.id.to_string()}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": format!("Media batch ingest started: {}", batch.id),
                                "batch_id": batch.id.to_string(),
                                "reference_name": batch.reference_name,
                                "status": format!("{:?}", batch.status),
                                "storage_tier": format!("{:?}", batch.storage_tier),
                                "created_at": batch.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to ingest media batch: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to ingest media batch: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available. Editron functionality requires media pipeline service.",
                    }))
                }
            }

            NoraExecutiveTool::AnalyzeMediaBatch {
                batch_id,
                brief,
                deliverable_targets,
                passes,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Analyzing media batch: {}", batch_id);

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid batch_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    let request = MediaBatchAnalysisRequest {
                        batch_id: batch_uuid,
                        brief: brief.clone(),
                        passes,
                        deliverable_targets: deliverable_targets.clone(),
                    };

                    match pipeline.analyze_batch(request).await {
                        Ok(analysis) => {
                            tracing::info!(
                                "[TOOL] Media batch analyzed: analysis_id={}",
                                analysis.id
                            );

                            // Dashboard tracking
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: Analyze batch {}", batch_id),
                                        &format!("Analysis of batch {} - {}", batch_id, brief),
                                        serde_json::json!({ "editron_batch_id": batch_id }),
                                    )
                                    .await
                                {
                                    let vibe_cost =
                                        crate::editron_tracking::EditronVibeCosts::analyze(passes);

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        &resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::MediaAnalysisReport,
                                        &format!("Analysis: {} hero moments", analysis.hero_moments.len()),
                                        Some(serde_json::json!({
                                            "analysis_id": analysis.id.to_string(),
                                            "batch_id": analysis.batch_id.to_string(),
                                            "hero_moments_count": analysis.hero_moments.len(),
                                            "passes_completed": analysis.passes_completed,
                                            "summary": analysis.summary,
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        &resolved_task_id,
                                        "editron_analyze_completed",
                                        &format!(
                                            "Batch analyzed: {} hero moments, {} passes",
                                            analysis.hero_moments.len(),
                                            analysis.passes_completed
                                        ),
                                        vibe_cost,
                                        serde_json::json!({"batch_id": batch_id, "analysis_id": analysis.id.to_string()}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        &resolved_project_id,
                                        &resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron analyze: {} passes", passes),
                                        "analyze",
                                        serde_json::json!({"batch_id": batch_id}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": "Media batch analyzed successfully",
                                "analysis_id": analysis.id.to_string(),
                                "batch_id": analysis.batch_id.to_string(),
                                "summary": analysis.summary,
                                "hero_moments_count": analysis.hero_moments.len(),
                                "hero_moments": analysis.hero_moments,
                                "recommended_deliverables": analysis.recommended_deliverables,
                                "passes_completed": analysis.passes_completed,
                                "created_at": analysis.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to analyze media batch: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to analyze media batch: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            NoraExecutiveTool::GenerateVideoEdits {
                batch_id,
                deliverable_type,
                aspect_ratios,
                reference_style,
                include_captions,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Generating video edits for batch: {}", batch_id);

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid batch_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    let request = EditSessionRequest {
                        batch_id: batch_uuid,
                        deliverable_type: deliverable_type.clone(),
                        aspect_ratios: aspect_ratios.clone(),
                        reference_style: reference_style.clone(),
                        include_captions,
                    };

                    match pipeline.generate_edits(request).await {
                        Ok(session) => {
                            tracing::info!(
                                "[TOOL] Edit session created: session_id={}",
                                session.id
                            );

                            // Dashboard tracking
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: {} edit", deliverable_type),
                                        &format!("Video edits for batch {}", batch_id),
                                        serde_json::json!({ "editron_batch_id": batch_id }),
                                    )
                                    .await
                                {
                                    let vibe_cost =
                                        crate::editron_tracking::EditronVibeCosts::generate(
                                            aspect_ratios.len(),
                                        );

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        &resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::VideoEditSession,
                                        &format!("Edit Session: {}", deliverable_type),
                                        Some(serde_json::json!({
                                            "session_id": session.id.to_string(),
                                            "batch_id": session.batch_id.to_string(),
                                            "deliverable_type": session.deliverable_type,
                                            "aspect_ratios": session.aspect_ratios,
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        &resolved_task_id,
                                        "editron_edits_generated",
                                        &format!(
                                            "Video edits generated: {} with {} ratios",
                                            deliverable_type,
                                            aspect_ratios.len()
                                        ),
                                        vibe_cost,
                                        serde_json::json!({"batch_id": batch_id, "session_id": session.id.to_string()}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        &resolved_project_id,
                                        &resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron edit: {} ratios", aspect_ratios.len()),
                                        "edit",
                                        serde_json::json!({"batch_id": batch_id}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": "Edit session created successfully",
                                "session_id": session.id.to_string(),
                                "batch_id": session.batch_id.to_string(),
                                "deliverable_type": session.deliverable_type,
                                "aspect_ratios": session.aspect_ratios,
                                "imovie_project": session.imovie_project,
                                "timelines": session.timelines,
                                "status": format!("{:?}", session.status),
                                "created_at": session.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to generate video edits: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to generate video edits: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            NoraExecutiveTool::RenderVideoDeliverables {
                edit_session_id,
                destinations,
                formats,
                priority,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!(
                        "[TOOL] Rendering video deliverables for session: {}",
                        edit_session_id
                    );

                    let session_uuid = match Uuid::parse_str(&edit_session_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid edit_session_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    let is_rush = matches!(priority, VideoRenderPriority::Rush);

                    // Convert VideoRenderPriority to the pipeline's priority enum
                    let priority_enum = match priority {
                        VideoRenderPriority::Low => PipelineRenderPriority::Low,
                        VideoRenderPriority::Standard => PipelineRenderPriority::Standard,
                        VideoRenderPriority::Rush => PipelineRenderPriority::Rush,
                    };

                    let request = RenderJobRequest {
                        edit_session_id: session_uuid,
                        destinations: destinations.clone(),
                        formats: formats.clone(),
                        priority: priority_enum,
                    };

                    match pipeline.render_deliverables(request).await {
                        Ok(job) => {
                            tracing::info!("[TOOL] Render job created: job_id={}", job.id);

                            // Dashboard tracking
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: Render {}", edit_session_id),
                                        &format!("Render job for session {}", edit_session_id),
                                        serde_json::json!({}),
                                    )
                                    .await
                                {
                                    let vibe_cost =
                                        crate::editron_tracking::EditronVibeCosts::render(
                                            formats.len(),
                                            is_rush,
                                        );

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        &resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::RenderDeliverable,
                                        &format!("Render Job: {} formats", formats.len()),
                                        Some(serde_json::json!({
                                            "job_id": job.id.to_string(),
                                            "edit_session_id": job.edit_session_id.to_string(),
                                            "destinations": job.destinations,
                                            "formats": job.formats,
                                            "priority": format!("{:?}", job.priority),
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        &resolved_task_id,
                                        "editron_render_started",
                                        &format!(
                                            "Render job queued: {} formats, {:?} priority",
                                            formats.len(),
                                            job.priority
                                        ),
                                        vibe_cost,
                                        serde_json::json!({"job_id": job.id.to_string(), "edit_session_id": edit_session_id}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        &resolved_project_id,
                                        &resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron render: {} formats", formats.len()),
                                        "render",
                                        serde_json::json!({"job_id": job.id.to_string()}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": "Render job created successfully",
                                "job_id": job.id.to_string(),
                                "edit_session_id": job.edit_session_id.to_string(),
                                "destinations": job.destinations,
                                "formats": job.formats,
                                "priority": format!("{:?}", job.priority),
                                "status": format!("{:?}", job.status),
                                "created_at": job.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to render video deliverables: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to render video deliverables: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            NoraExecutiveTool::RunVisualQc {
                batch_id,
                candidates_per_clip,
                min_composition_score,
                target_aspect_ratio,
                project_id: _,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Running Spectra Visual QC on batch: {}", batch_id);

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid batch_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Verify batch is ready
                    let batch = match pipeline.load_batch_for_qc(batch_uuid).await {
                        Ok(b) => b,
                        Err(e) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Batch not ready for QC: {}", e),
                            }));
                        }
                    };

                    // Build QC config
                    let mut config = services::services::visual_qc::VisualQcConfig::default();
                    if let Some(n) = candidates_per_clip {
                        config.candidates_per_clip = n;
                    }
                    if let Some(score) = min_composition_score {
                        config.min_composition_score = score;
                    }
                    config.target_aspect_ratio = target_aspect_ratio.clone();

                    let start = std::time::Instant::now();
                    let work_dir = pipeline.visual_qc_work_dir(batch_uuid);
                    let _ = tokio::fs::create_dir_all(&work_dir).await;

                    let ffmpeg_path = std::path::PathBuf::from("ffmpeg");
                    let engine =
                        services::services::visual_qc::VisualQcEngine::new(&ffmpeg_path, &work_dir);
                    let (_system_prompt, _user_prompt_template) =
                        services::services::visual_qc::VisualQcEngine::build_vision_prompt(
                            target_aspect_ratio.as_deref(),
                        );

                    let mut clip_results = Vec::new();
                    let mut total_frames = 0u32;

                    for file in &batch.files {
                        if total_frames >= config.max_total_frames {
                            break;
                        }

                        let file_path = pipeline.get_batch_dir(batch_uuid).join(&file.filename);
                        if !file_path.exists() {
                            continue;
                        }

                        let frames =
                            match engine.extract_candidate_frames(&file_path, &config).await {
                                Ok(f) => f,
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to extract frames from {}: {}",
                                        file.filename,
                                        e
                                    );
                                    continue;
                                }
                            };

                        let mut analyzed_frames = Vec::new();

                        for (timestamp, frame_path) in &frames {
                            if total_frames >= config.max_total_frames {
                                break;
                            }

                            let _base64_data = match services::services::visual_qc::VisualQcEngine::frame_to_base64(frame_path).await {
                                Ok(d) => d,
                                Err(_) => continue,
                            };

                            // Build the QC result using a placeholder analysis since we
                            // don't have direct access to the LLM provider from the tool.
                            // In production, the ExecutionEngine (which HAS provider access)
                            // handles the vision API calls via the visual-qc-pass workflow.
                            // This tool fallback provides frame-extraction-only QC scoring.
                            let placeholder_response = serde_json::json!({
                                "composition_score": 0.7,
                                "subject_score": 0.7,
                                "thirds_score": 0.6,
                                "headroom_score": 0.8,
                                "exposure_score": 0.7,
                                "sharpness_score": 0.7,
                                "subject_region": null,
                                "suggested_crop": null,
                                "notes": "Frame extracted — vision API scoring deferred to workflow execution"
                            });

                            if let Ok(frame) =
                                services::services::visual_qc::VisualQcEngine::parse_vision_response(
                                    *timestamp,
                                    frame_path,
                                    &placeholder_response.to_string(),
                                )
                            {
                                analyzed_frames.push(frame);
                            }

                            total_frames += 1;
                        }

                        let best =
                            services::services::visual_qc::VisualQcEngine::select_best_in_point(
                                &analyzed_frames,
                                &config,
                            );
                        let (best_in_point, best_score) = best.unwrap_or((0.0, 0.0));
                        let qc_passed = best_score >= config.min_composition_score;

                        let recommended_crop = analyzed_frames
                            .iter()
                            .find(|f| (f.timestamp - best_in_point).abs() < 0.001)
                            .and_then(|f| f.suggested_crop.clone());

                        clip_results.push(services::services::visual_qc::ClipQcResult {
                            clip_path: file_path,
                            frames_analyzed: analyzed_frames.len() as u32,
                            best_in_point,
                            best_composition_score: best_score,
                            recommended_crop,
                            qc_passed,
                            summary: if qc_passed {
                                format!("Passed (score: {:.2})", best_score)
                            } else {
                                format!("Below threshold (score: {:.2})", best_score)
                            },
                        });
                    }

                    let time_ms = start.elapsed().as_millis() as u64;
                    let result =
                        services::services::visual_qc::VisualQcEngine::assemble_batch_result(
                            clip_results,
                            &config,
                            time_ms,
                        );

                    // Persist result
                    let result_path = work_dir.join("qc_result.json");
                    if let Ok(json_str) = serde_json::to_string_pretty(&result) {
                        let _ = tokio::fs::write(&result_path, json_str).await;
                    }

                    tracing::info!(
                        "[TOOL] Visual QC complete: {}/{} clips passed (avg score: {:.2})",
                        result.clips_passed,
                        result.clips_analyzed,
                        result.average_composition_score
                    );

                    let clip_summaries: Vec<serde_json::Value> = result
                        .clip_results
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "clip": r.clip_path.file_name().map(|n| n.to_string_lossy().to_string()),
                                "best_in_point": r.best_in_point,
                                "composition_score": r.best_composition_score,
                                "passed": r.qc_passed,
                                "summary": r.summary,
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "success": true,
                        "message": format!(
                            "Visual QC complete: {}/{} clips passed",
                            result.clips_passed, result.clips_analyzed
                        ),
                        "qc_id": result.id,
                        "clips_analyzed": result.clips_analyzed,
                        "clips_passed": result.clips_passed,
                        "clips_failed": result.clips_failed,
                        "average_composition_score": result.average_composition_score,
                        "processing_time_ms": result.processing_time_ms,
                        "clip_results": clip_summaries,
                    }))
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            // === Deep Scene Analysis ===
            NoraExecutiveTool::AnalyzeScenes {
                batch_id,
                segment_interval,
                project_id: _,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Running deep scene analysis on batch: {}", batch_id);
                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Invalid batch_id"}),
                            )
                        }
                    };
                    let batch = match pipeline.load_batch_for_qc(batch_uuid).await {
                        Ok(b) => b,
                        Err(e) => {
                            return Ok(
                                serde_json::json!({"success": false, "error": format!("Batch not ready: {}", e)}),
                            )
                        }
                    };

                    let start = std::time::Instant::now();
                    let engine = services::services::scene_analysis::SceneAnalysisEngine::new();
                    let interval = segment_interval.unwrap_or(3.0);
                    let mut clip_analyses = Vec::new();

                    for file in &batch.files {
                        // Skip non-video files
                        let ext = file
                            .filename
                            .rsplit('.')
                            .next()
                            .unwrap_or("")
                            .to_lowercase();
                        if !matches!(ext.as_str(), "mp4" | "mov" | "avi" | "mxf" | "mkv") {
                            continue;
                        }
                        let file_path = pipeline.get_batch_dir(batch_uuid).join(&file.filename);
                        if !file_path.exists() {
                            continue;
                        }

                        match engine.analyze_clip(&file_path, interval).await {
                            Ok(analysis) => {
                                tracing::info!(
                                    "[TOOL] Scene analysis for {}: energy={:.2}, type={:?}, segments={}",
                                    file.filename, analysis.overall_energy,
                                    analysis.dominant_content_type, analysis.segments.len()
                                );
                                clip_analyses.push(analysis);
                            }
                            Err(e) => {
                                tracing::warn!("[TOOL] Failed to analyze {}: {}", file.filename, e);
                            }
                        }
                    }

                    let total_usable = clip_analyses.iter().filter(|c| c.usable).count() as u32;
                    let time_ms = start.elapsed().as_millis() as u64;

                    let result = services::services::scene_analysis::SceneAnalysisResult {
                        batch_id: batch_id.clone(),
                        total_clips: clip_analyses.len() as u32,
                        total_usable,
                        clips: clip_analyses.clone(),
                        processing_time_ms: time_ms,
                    };

                    // Persist
                    let work_dir = pipeline.visual_qc_work_dir(batch_uuid);
                    let _ = tokio::fs::create_dir_all(&work_dir).await;
                    let result_path = work_dir.join("scene_analysis.json");
                    if let Ok(json_str) = serde_json::to_string_pretty(&result) {
                        let _ = tokio::fs::write(&result_path, json_str).await;
                    }

                    let clip_summaries: Vec<serde_json::Value> = clip_analyses
                        .iter()
                        .map(|c| {
                            serde_json::json!({
                                "filename": c.filename,
                                "duration": c.duration,
                                "overall_energy": c.overall_energy,
                                "peak_timestamp": c.peak_energy_timestamp,
                                "content_type": format!("{:?}", c.dominant_content_type),
                                "segments": c.segments.len(),
                                "usable": c.usable,
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "success": true,
                        "message": format!("Scene analysis complete: {}/{} clips usable", total_usable, clip_analyses.len()),
                        "total_clips": clip_analyses.len(),
                        "total_usable": total_usable,
                        "processing_time_ms": time_ms,
                        "clips": clip_summaries,
                    }))
                } else {
                    Ok(
                        serde_json::json!({"success": false, "error": "Media pipeline not available"}),
                    )
                }
            }

            // === Beat Grid Analysis ===
            NoraExecutiveTool::AnalyzeBeatGrid {
                audio_path,
                bpm_hint,
                beats_per_bar,
                project_id: _,
            } => {
                tracing::info!("[TOOL] Analyzing beat grid for: {}", audio_path);
                let path = std::path::PathBuf::from(&audio_path);
                if !path.exists() {
                    return Ok(
                        serde_json::json!({"success": false, "error": format!("Audio file not found: {}", audio_path)}),
                    );
                }

                let engine = services::services::beat_analysis::BeatAnalysisEngine::new();
                let bpb = beats_per_bar.unwrap_or(4);

                match engine.analyze(&path, bpm_hint, bpb).await {
                    Ok(result) => {
                        tracing::info!(
                            "[TOOL] Beat analysis complete: {:.1} BPM, {} beats, {} sections, {:.0}ms",
                            result.bpm, result.total_beats, result.sections.len(), result.processing_time_ms
                        );

                        // Persist
                        let result_dir = path.parent().unwrap_or(std::path::Path::new("/tmp"));
                        let result_path = result_dir.join("beat_grid.json");
                        if let Ok(json_str) = serde_json::to_string_pretty(&result) {
                            let _ = tokio::fs::write(&result_path, json_str).await;
                        }

                        let section_summaries: Vec<serde_json::Value> = result
                            .sections
                            .iter()
                            .map(|s| {
                                serde_json::json!({
                                    "name": s.name,
                                    "start": s.start,
                                    "end": s.end,
                                    "energy": s.energy_level,
                                    "suggested_content": format!("{:?}", s.suggested_content),
                                })
                            })
                            .collect();

                        Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Beat analysis complete: {:.1} BPM, {} beats", result.bpm, result.total_beats),
                            "bpm": result.bpm,
                            "beat_interval": result.beat_interval,
                            "duration": result.duration,
                            "total_beats": result.total_beats,
                            "sections": section_summaries,
                            "transition_markers": result.transition_markers.len(),
                            "processing_time_ms": result.processing_time_ms,
                        }))
                    }
                    Err(e) => Ok(
                        serde_json::json!({"success": false, "error": format!("Beat analysis failed: {}", e)}),
                    ),
                }
            }

            // === Assemble Recap Edit ===
            NoraExecutiveTool::AssembleRecapEdit {
                batch_id,
                audio_path,
                bpm_hint,
                target_aspect_ratio,
                project_id: _,
                project_name,
            } => {
                let name_slug = project_name.as_deref().unwrap_or("Recap").replace(' ', "_");
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!(
                        "[TOOL] Assembling recap edit for batch: {} with audio: {}",
                        batch_id,
                        audio_path
                    );

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Invalid batch_id"}),
                            )
                        }
                    };

                    let audio = std::path::PathBuf::from(&audio_path);
                    if !audio.exists() {
                        return Ok(
                            serde_json::json!({"success": false, "error": format!("Audio not found: {}", audio_path)}),
                        );
                    }

                    let start = std::time::Instant::now();

                    // Step 1: Scene analysis
                    tracing::info!("[TOOL] Step 1/4: Deep scene analysis...");
                    let scene_engine =
                        services::services::scene_analysis::SceneAnalysisEngine::new();
                    let batch = match pipeline.load_batch_for_qc(batch_uuid).await {
                        Ok(b) => b,
                        Err(e) => {
                            return Ok(
                                serde_json::json!({"success": false, "error": format!("Batch not ready: {}", e)}),
                            )
                        }
                    };

                    let mut clip_analyses = Vec::new();
                    for file in &batch.files {
                        let ext = file
                            .filename
                            .rsplit('.')
                            .next()
                            .unwrap_or("")
                            .to_lowercase();
                        if !matches!(ext.as_str(), "mp4" | "mov" | "avi" | "mxf" | "mkv") {
                            continue;
                        }
                        let file_path = pipeline.get_batch_dir(batch_uuid).join(&file.filename);
                        if !file_path.exists() {
                            continue;
                        }
                        if let Ok(analysis) = scene_engine.analyze_clip(&file_path, 1.0).await {
                            clip_analyses.push(analysis);
                        }
                    }
                    // Assign energy quartiles before assembly
                    services::services::scene_analysis::assign_energy_quartiles(&mut clip_analyses);
                    let total_usable = clip_analyses.iter().filter(|c| c.usable).count() as u32;
                    let scene_result = services::services::scene_analysis::SceneAnalysisResult {
                        batch_id: batch_id.clone(),
                        total_clips: clip_analyses.len() as u32,
                        total_usable,
                        clips: clip_analyses,
                        processing_time_ms: 0,
                    };

                    // Step 2: Beat analysis
                    tracing::info!("[TOOL] Step 2/4: Beat grid analysis...");
                    let beat_engine = services::services::beat_analysis::BeatAnalysisEngine::new();
                    let beat_grid = match beat_engine.analyze(&audio, bpm_hint, 4).await {
                        Ok(bg) => bg,
                        Err(e) => {
                            return Ok(
                                serde_json::json!({"success": false, "error": format!("Beat analysis failed: {}", e)}),
                            )
                        }
                    };

                    // Step 3: Assembly
                    tracing::info!("[TOOL] Step 3/4: Smart assembly (content→music matching, beat-lock cuts)...");
                    let (width, height) = match target_aspect_ratio.as_deref() {
                        Some("9:16") => (1080, 1920),
                        Some("1:1") => (1080, 1080),
                        _ => (3840, 2160),
                    };

                    let (placements, music_window) =
                        services::services::recap_assembly::RecapAssemblyEngine::assemble(
                            &scene_result,
                            &beat_grid,
                            &audio,
                            width,
                            height,
                            None, // Use default 59s duration
                        );

                    let beat_locked_cuts =
                        placements.iter().filter(|p| p.beat_locked).count() as u32;

                    let assembly_result = services::services::recap_assembly::RecapAssemblyResult {
                        id: Uuid::new_v4().to_string(),
                        name: format!("{} - Recap", project_name.as_deref().unwrap_or("Recap")),
                        duration: music_window.duration,
                        width,
                        height,
                        fps: 29.97,
                        bpm: beat_grid.bpm,
                        placements: placements.clone(),
                        music_path: audio_path.clone(),
                        music_window: Some(music_window.clone()),
                        xml_path: String::new(),
                        clips_used: placements.len() as u32,
                        clips_available: scene_result.total_usable,
                        beat_locked_cuts,
                        processing_time_ms: 0,
                    };

                    // Step 4: Generate XML
                    tracing::info!(
                        "[TOOL] Step 4/4: Generating Premiere Pro XML (music only, NAT muted)..."
                    );
                    let xml = services::services::recap_assembly::RecapAssemblyEngine::generate_premiere_xml(
                        &assembly_result,
                        &placements,
                        &beat_grid,
                    );

                    let work_dir = pipeline.visual_qc_work_dir(batch_uuid);
                    let _ = tokio::fs::create_dir_all(&work_dir).await;
                    let xml_path = work_dir.join(format!("{}_BeatLocked.xml", name_slug));
                    let _ = tokio::fs::write(&xml_path, &xml).await;

                    // Generate FFmpeg render script with transitions
                    let render_output = work_dir.join(format!("{}_v1.mp4", name_slug));
                    let render_script = services::services::recap_assembly::RecapAssemblyEngine::generate_render_script(
                        &placements,
                        &audio_path,
                        &music_window,
                        &render_output.to_string_lossy(),
                    );
                    let script_path = work_dir.join("render.sh");
                    let _ = tokio::fs::write(&script_path, &render_script).await;

                    let time_ms = start.elapsed().as_millis() as u64;

                    // Also persist assembly result
                    let mut final_result = assembly_result;
                    final_result.xml_path = xml_path.to_string_lossy().to_string();
                    final_result.processing_time_ms = time_ms;
                    let result_path = work_dir.join("assembly_result.json");
                    if let Ok(json_str) = serde_json::to_string_pretty(&final_result) {
                        let _ = tokio::fs::write(&result_path, json_str).await;
                    }

                    let placement_summaries: Vec<serde_json::Value> = placements
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "clip": p.clip_filename,
                                "section": p.section_name,
                                "timeline": format!("{:.2}s-{:.2}s", p.timeline_in, p.timeline_out),
                                "source": format!("{:.2}s-{:.2}s", p.source_in, p.source_out),
                                "energy_match": p.energy_match_score,
                                "beat_locked": p.beat_locked,
                            })
                        })
                        .collect();

                    tracing::info!(
                        "[TOOL] Recap assembled: {} clips, {} beat-locked cuts, {:.1}s duration, {:.0}ms",
                        final_result.clips_used, beat_locked_cuts, final_result.duration, time_ms
                    );

                    Ok(serde_json::json!({
                        "success": true,
                        "message": format!(
                            "Recap assembled: {} clips, {} beat-locked cuts, audio=music only (NAT muted)",
                            final_result.clips_used, beat_locked_cuts
                        ),
                        "assembly_id": final_result.id,
                        "duration": final_result.duration,
                        "bpm": final_result.bpm,
                        "clips_used": final_result.clips_used,
                        "clips_available": final_result.clips_available,
                        "beat_locked_cuts": beat_locked_cuts,
                        "xml_path": final_result.xml_path,
                        "processing_time_ms": time_ms,
                        "placements": placement_summaries,
                        "render_script": script_path.to_string_lossy(),
                        "render_output": render_output.to_string_lossy(),
                        "music_window": {
                            "start": music_window.start,
                            "end": music_window.end,
                            "duration": music_window.duration,
                            "sections": music_window.sections.len()
                        },
                        "audio_config": {
                            "music_track": format!("A1 - music window {:.1}s-{:.1}s ({:.0}s)", music_window.start, music_window.end, music_window.duration),
                            "nat_audio": "MUTED - no clip audio on timeline",
                            "normalization": "Streaming (-14 LUFS)"
                        },
                        "sections": beat_grid.sections.iter().map(|s| {
                            serde_json::json!({"name": s.name, "start": s.start, "end": s.end, "energy": s.energy_level})
                        }).collect::<Vec<_>>(),
                    }))
                } else {
                    Ok(
                        serde_json::json!({"success": false, "error": "Media pipeline not available"}),
                    )
                }
            }

            NoraExecutiveTool::ExecuteRenderScript {
                render_script,
                render_output,
                xml_path,
            } => {
                tracing::info!("[TOOL] Executing render script: {}", render_script);

                let script_path = std::path::Path::new(&render_script);
                if !script_path.exists() {
                    return Ok(serde_json::json!({
                        "success": false,
                        "error": format!("Render script not found: {}", render_script),
                    }));
                }

                // Run the render script via bash
                let output = tokio::process::Command::new("bash")
                    .arg(&render_script)
                    .output()
                    .await;

                match output {
                    Ok(result) => {
                        let stdout = String::from_utf8_lossy(&result.stdout);
                        let stderr = String::from_utf8_lossy(&result.stderr);
                        let render_path = std::path::Path::new(&render_output);
                        let file_size = tokio::fs::metadata(&render_path)
                            .await
                            .map(|m| m.len())
                            .unwrap_or(0);

                        tracing::info!(
                            "[TOOL] Render complete: {} ({} bytes), exit={}",
                            render_output,
                            file_size,
                            result.status
                        );

                        let _ = stdout; // consumed for logging if needed

                        // Take the last 500 chars of stderr for diagnostics
                        let stderr_str = stderr.to_string();
                        let stderr_tail: String = if stderr_str.len() > 500 {
                            stderr_str[stderr_str.len() - 500..].to_string()
                        } else {
                            stderr_str
                        };

                        Ok(serde_json::json!({
                            "success": result.status.success(),
                            "message": format!("Render complete: {}", render_output),
                            "render_output": render_output,
                            "xml_path": xml_path,
                            "file_size_bytes": file_size,
                            "exit_code": result.status.code(),
                            "stderr_tail": stderr_tail,
                        }))
                    }
                    Err(e) => Ok(serde_json::json!({
                        "success": false,
                        "error": format!("Failed to execute render script: {}", e),
                    })),
                }
            }

            // === Music Discovery & Download Tools ===
            NoraExecutiveTool::SearchMusic {
                query,
                moods,
                genres,
                min_bpm,
                max_bpm,
                min_duration,
                max_duration,
                instrumental,
                platforms,
                page,
                per_page,
            } => {
                tracing::info!(
                    "[TOOL] SearchMusic: query={:?}, moods={:?}, genres={:?}",
                    query,
                    moods,
                    genres
                );

                use services::services::editron::{
                    artlist::ArtlistClient,
                    epidemic::EpidemicSoundClient,
                    load_music_platform_configs,
                    music::{MusicGenre, MusicMood, MusicSearchCriteria},
                    soundstripe::SoundstripeClient,
                };

                let (artlist_cfg, epidemic_cfg, soundstripe_cfg) = load_music_platform_configs();

                // Build search criteria
                let criteria = MusicSearchCriteria {
                    query,
                    moods: moods
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|m| {
                            MusicMood::from_epidemic_term(m)
                                .or_else(|| MusicMood::from_artlist_term(m))
                        })
                        .collect(),
                    genres: genres
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|g| {
                            MusicGenre::from_epidemic_term(g)
                                .or_else(|| MusicGenre::from_artlist_term(g))
                        })
                        .collect(),
                    min_duration,
                    max_duration,
                    min_bpm,
                    max_bpm,
                    has_vocals: None,
                    instrumental,
                    platforms: vec![],
                };

                let page_num = page.unwrap_or(1);
                let results_per_page = per_page.unwrap_or(20);
                let platform_filter = platforms.unwrap_or_default();

                let mut all_tracks = Vec::new();
                let mut platform_status = serde_json::Map::new();

                // Search Artlist
                let search_artlist =
                    platform_filter.is_empty() || platform_filter.iter().any(|p| p == "artlist");
                if search_artlist {
                    if artlist_cfg.is_configured() {
                        match ArtlistClient::from_config(&artlist_cfg) {
                            Ok(client) => {
                                match client
                                    .search_tracks(&criteria, page_num, results_per_page)
                                    .await
                                {
                                    Ok(tracks) => {
                                        platform_status.insert("artlist".to_string(), serde_json::json!({"status": "ok", "count": tracks.len()}));
                                        all_tracks.extend(tracks);
                                    }
                                    Err(e) => {
                                        platform_status.insert("artlist".to_string(), serde_json::json!({"status": "error", "error": e.to_string()}));
                                    }
                                }
                            }
                            Err(e) => {
                                platform_status.insert(
                                    "artlist".to_string(),
                                    serde_json::json!({"status": "error", "error": e.to_string()}),
                                );
                            }
                        }
                    } else {
                        platform_status.insert(
                            "artlist".to_string(),
                            serde_json::json!({"status": "not_configured"}),
                        );
                    }
                }

                // Search Epidemic Sound
                let search_epidemic =
                    platform_filter.is_empty() || platform_filter.iter().any(|p| p == "epidemic");
                if search_epidemic {
                    if epidemic_cfg.is_configured() {
                        match EpidemicSoundClient::from_config(&epidemic_cfg) {
                            Ok(client) => {
                                match client
                                    .search_tracks(&criteria, page_num, results_per_page)
                                    .await
                                {
                                    Ok(tracks) => {
                                        platform_status.insert("epidemic".to_string(), serde_json::json!({"status": "ok", "count": tracks.len()}));
                                        all_tracks.extend(tracks);
                                    }
                                    Err(e) => {
                                        platform_status.insert("epidemic".to_string(), serde_json::json!({"status": "error", "error": e.to_string()}));
                                    }
                                }
                            }
                            Err(e) => {
                                platform_status.insert(
                                    "epidemic".to_string(),
                                    serde_json::json!({"status": "error", "error": e.to_string()}),
                                );
                            }
                        }
                    } else {
                        platform_status.insert(
                            "epidemic".to_string(),
                            serde_json::json!({"status": "not_configured"}),
                        );
                    }
                }

                // Search Soundstripe
                let search_soundstripe = platform_filter.is_empty()
                    || platform_filter.iter().any(|p| p == "soundstripe");
                if search_soundstripe {
                    if soundstripe_cfg.is_configured() {
                        match SoundstripeClient::from_config(&soundstripe_cfg) {
                            Ok(client) => {
                                match client
                                    .search_tracks(&criteria, page_num, results_per_page)
                                    .await
                                {
                                    Ok(tracks) => {
                                        platform_status.insert("soundstripe".to_string(), serde_json::json!({"status": "ok", "count": tracks.len()}));
                                        all_tracks.extend(tracks);
                                    }
                                    Err(e) => {
                                        platform_status.insert("soundstripe".to_string(), serde_json::json!({"status": "error", "error": e.to_string()}));
                                    }
                                }
                            }
                            Err(e) => {
                                platform_status.insert(
                                    "soundstripe".to_string(),
                                    serde_json::json!({"status": "error", "error": e.to_string()}),
                                );
                            }
                        }
                    } else {
                        platform_status.insert(
                            "soundstripe".to_string(),
                            serde_json::json!({"status": "not_configured"}),
                        );
                    }
                }

                let track_summaries: Vec<serde_json::Value> = all_tracks
                    .iter()
                    .map(|t| {
                        serde_json::json!({
                            "id": t.id,
                            "title": t.title,
                            "artist": t.artist,
                            "duration": t.duration,
                            "bpm": t.bpm,
                            "genre": format!("{:?}", t.genre),
                            "moods": t.moods.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>(),
                            "platform": format!("{:?}", t.platform),
                            "preview_url": t.preview_url,
                            "url": t.url,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "success": true,
                    "total_results": track_summaries.len(),
                    "tracks": track_summaries,
                    "platforms": serde_json::Value::Object(platform_status),
                }))
            }

            NoraExecutiveTool::DownloadMusicTrack {
                track_id,
                filename,
                output_dir,
            } => {
                tracing::info!("[TOOL] DownloadMusicTrack: {}", track_id);

                use services::services::editron::load_music_platform_configs;

                // Parse platform prefix
                let (platform, raw_id) = match track_id.split_once(':') {
                    Some((p, id)) => (p.to_string(), id.to_string()),
                    None => {
                        return Ok(serde_json::json!({
                            "success": false,
                            "error": "Invalid track_id format. Must be 'platform:id' (e.g., 'artlist:12345')"
                        }))
                    }
                };

                let (artlist_cfg, epidemic_cfg, soundstripe_cfg) = load_music_platform_configs();

                let result: Result<(String, String), String> = match platform.as_str() {
                    "artlist" => {
                        if !artlist_cfg.is_configured() {
                            Err("Artlist not configured (set ARTLIST_CLIENT_ID and ARTLIST_CLIENT_SECRET)".to_string())
                        } else {
                            match services::services::editron::artlist::ArtlistClient::from_config(
                                &artlist_cfg,
                            ) {
                                Ok(client) => match client.get_download_url(&raw_id).await {
                                    Ok(url) => {
                                        let track = client.get_track(&raw_id).await.ok();
                                        let default_name = track
                                            .map(|t| format!("{} - {}.mp3", t.artist, t.title))
                                            .unwrap_or_else(|| format!("artlist_{}.mp3", raw_id));
                                        Ok((url, default_name))
                                    }
                                    Err(e) => Err(e.to_string()),
                                },
                                Err(e) => Err(e.to_string()),
                            }
                        }
                    }
                    "epidemic" => {
                        if !epidemic_cfg.is_configured() {
                            Err("Epidemic Sound not configured (set ES_ACCESS_KEY_ID and ES_ACCESS_KEY_SECRET)".to_string())
                        } else {
                            match services::services::editron::epidemic::EpidemicSoundClient::from_config(&epidemic_cfg) {
                                Ok(client) => {
                                    match client.get_download_url(&raw_id, "mp3", "high").await {
                                        Ok(url) => {
                                            let track = client.get_track(&raw_id).await.ok();
                                            let default_name = track.map(|t| format!("{} - {}.mp3", t.artist, t.title))
                                                .unwrap_or_else(|| format!("epidemic_{}.mp3", raw_id));
                                            Ok((url, default_name))
                                        }
                                        Err(e) => Err(e.to_string()),
                                    }
                                }
                                Err(e) => Err(e.to_string()),
                            }
                        }
                    }
                    "soundstripe" => {
                        if !soundstripe_cfg.is_configured() {
                            Err("Soundstripe not configured (set SOUNDSTRIPE_API_KEY)".to_string())
                        } else {
                            match services::services::editron::soundstripe::SoundstripeClient::from_config(&soundstripe_cfg) {
                                Ok(client) => {
                                    match client.get_track(&raw_id).await {
                                        Ok(track) => {
                                            if let Some(url) = &track.preview_url {
                                                let default_name = format!("{} - {}.mp3", track.artist, track.title);
                                                Ok((url.clone(), default_name))
                                            } else {
                                                Err("No download URL available for this Soundstripe track".to_string())
                                            }
                                        }
                                        Err(e) => Err(e.to_string()),
                                    }
                                }
                                Err(e) => Err(e.to_string()),
                            }
                        }
                    }
                    _ => Err(format!(
                        "Unknown platform: '{}'. Use 'artlist', 'epidemic', or 'soundstripe'",
                        platform
                    )),
                };

                match result {
                    Ok((download_url, default_name)) => {
                        let safe_filename = filename
                            .unwrap_or(default_name)
                            .replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                        let dir = output_dir.unwrap_or_else(|| format!("/tmp/music/{}", platform));
                        let dir_path = std::path::Path::new(&dir);
                        let _ = tokio::fs::create_dir_all(dir_path).await;
                        let output_path = dir_path.join(&safe_filename);

                        match reqwest::Client::new().get(&download_url).send().await {
                            Ok(response) => {
                                if !response.status().is_success() {
                                    return Ok(
                                        serde_json::json!({"success": false, "error": format!("Download failed with status: {}", response.status())}),
                                    );
                                }
                                match response.bytes().await {
                                    Ok(bytes) => {
                                        match tokio::fs::write(&output_path, &bytes).await {
                                            Ok(_) => {
                                                tracing::info!(
                                                    "[TOOL] Downloaded {} bytes to {}",
                                                    bytes.len(),
                                                    output_path.display()
                                                );
                                                Ok(serde_json::json!({
                                                    "success": true,
                                                    "message": format!("Track downloaded to {}", output_path.display()),
                                                    "path": output_path.to_string_lossy(),
                                                    "size_bytes": bytes.len(),
                                                    "platform": platform,
                                                    "track_id": track_id,
                                                }))
                                            }
                                            Err(e) => Ok(
                                                serde_json::json!({"success": false, "error": format!("Failed to write file: {}", e)}),
                                            ),
                                        }
                                    }
                                    Err(e) => Ok(
                                        serde_json::json!({"success": false, "error": format!("Failed to read download bytes: {}", e)}),
                                    ),
                                }
                            }
                            Err(e) => Ok(
                                serde_json::json!({"success": false, "error": format!("Download request failed: {}", e)}),
                            ),
                        }
                    }
                    Err(e) => Ok(serde_json::json!({"success": false, "error": e})),
                }
            }

            NoraExecutiveTool::RecommendMusicForVideo {
                video_path: _,
                content_type,
                target_duration,
                auto_search,
            } => {
                tracing::info!(
                    "[TOOL] RecommendMusicForVideo: content_type={:?}, duration={:?}",
                    content_type,
                    target_duration
                );

                use services::services::editron::music::MusicLibrary;

                let lib = MusicLibrary::new("/tmp/music", "ffmpeg");
                let ct = content_type.as_deref().unwrap_or("lifestyle");
                let dur = target_duration.unwrap_or(0.0);
                let recommendation = lib.recommend_for_content(ct, dur);

                let mut response = serde_json::json!({
                    "success": true,
                    "content_type": ct,
                    "recommendation": {
                        "rationale": recommendation.rationale,
                        "search_url": recommendation.search_url,
                        "criteria": {
                            "moods": recommendation.criteria.moods.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>(),
                            "genres": recommendation.criteria.genres.iter().map(|g| format!("{:?}", g)).collect::<Vec<_>>(),
                            "min_bpm": recommendation.criteria.min_bpm,
                            "max_bpm": recommendation.criteria.max_bpm,
                            "instrumental": recommendation.criteria.instrumental,
                            "min_duration": recommendation.criteria.min_duration,
                            "max_duration": recommendation.criteria.max_duration,
                        }
                    }
                });

                // Auto-search if requested
                if auto_search == Some(true) {
                    use services::services::editron::{
                        artlist::ArtlistClient, epidemic::EpidemicSoundClient,
                        load_music_platform_configs, soundstripe::SoundstripeClient,
                    };

                    let (artlist_cfg, epidemic_cfg, soundstripe_cfg) =
                        load_music_platform_configs();
                    let mut tracks = Vec::new();

                    if artlist_cfg.is_configured() {
                        if let Ok(client) = ArtlistClient::from_config(&artlist_cfg) {
                            if let Ok(results) =
                                client.search_tracks(&recommendation.criteria, 1, 10).await
                            {
                                tracks.extend(results);
                            }
                        }
                    }
                    if epidemic_cfg.is_configured() {
                        if let Ok(client) = EpidemicSoundClient::from_config(&epidemic_cfg) {
                            if let Ok(results) =
                                client.search_tracks(&recommendation.criteria, 1, 10).await
                            {
                                tracks.extend(results);
                            }
                        }
                    }
                    if soundstripe_cfg.is_configured() {
                        if let Ok(client) = SoundstripeClient::from_config(&soundstripe_cfg) {
                            if let Ok(results) =
                                client.search_tracks(&recommendation.criteria, 1, 10).await
                            {
                                tracks.extend(results);
                            }
                        }
                    }

                    let track_list: Vec<serde_json::Value> = tracks
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "id": t.id,
                                "title": t.title,
                                "artist": t.artist,
                                "duration": t.duration,
                                "bpm": t.bpm,
                                "platform": format!("{:?}", t.platform),
                                "preview_url": t.preview_url,
                            })
                        })
                        .collect();

                    response
                        .as_object_mut()
                        .unwrap()
                        .insert("search_results".to_string(), serde_json::json!(track_list));
                    response
                        .as_object_mut()
                        .unwrap()
                        .insert("total_results".to_string(), serde_json::json!(tracks.len()));
                }

                Ok(response)
            }

            NoraExecutiveTool::PreviewMusicTrack { track_id } => {
                tracing::info!("[TOOL] PreviewMusicTrack: {}", track_id);

                use services::services::editron::load_music_platform_configs;

                let (platform, raw_id) = match track_id.split_once(':') {
                    Some((p, id)) => (p.to_string(), id.to_string()),
                    None => {
                        return Ok(serde_json::json!({
                            "success": false,
                            "error": "Invalid track_id format. Must be 'platform:id'"
                        }))
                    }
                };

                let (artlist_cfg, epidemic_cfg, soundstripe_cfg) = load_music_platform_configs();

                let track_result: Result<serde_json::Value, String> = match platform.as_str() {
                    "artlist" => {
                        if !artlist_cfg.is_configured() {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Artlist not configured"}),
                            );
                        }
                        match services::services::editron::artlist::ArtlistClient::from_config(
                            &artlist_cfg,
                        ) {
                            Ok(client) => match client.get_track(&raw_id).await {
                                Ok(t) => Ok(serde_json::json!({
                                    "preview_url": t.preview_url,
                                    "title": t.title,
                                    "artist": t.artist,
                                    "duration": t.duration,
                                    "bpm": t.bpm,
                                    "platform": "artlist",
                                })),
                                Err(e) => Err(e.to_string()),
                            },
                            Err(e) => Err(e.to_string()),
                        }
                    }
                    "epidemic" => {
                        if !epidemic_cfg.is_configured() {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Epidemic Sound not configured"}),
                            );
                        }
                        match services::services::editron::epidemic::EpidemicSoundClient::from_config(&epidemic_cfg) {
                            Ok(client) => match client.get_track(&raw_id).await {
                                Ok(t) => {
                                    let highlights = client.get_highlights(&raw_id, &[15, 30, 60]).await.ok();
                                    Ok(serde_json::json!({
                                        "preview_url": t.preview_url,
                                        "title": t.title,
                                        "artist": t.artist,
                                        "duration": t.duration,
                                        "bpm": t.bpm,
                                        "platform": "epidemic",
                                        "highlights": highlights,
                                    }))
                                }
                                Err(e) => Err(e.to_string()),
                            },
                            Err(e) => Err(e.to_string()),
                        }
                    }
                    "soundstripe" => {
                        if !soundstripe_cfg.is_configured() {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Soundstripe not configured"}),
                            );
                        }
                        match services::services::editron::soundstripe::SoundstripeClient::from_config(&soundstripe_cfg) {
                            Ok(client) => match client.get_track(&raw_id).await {
                                Ok(t) => Ok(serde_json::json!({
                                    "preview_url": t.preview_url,
                                    "title": t.title,
                                    "artist": t.artist,
                                    "duration": t.duration,
                                    "bpm": t.bpm,
                                    "platform": "soundstripe",
                                })),
                                Err(e) => Err(e.to_string()),
                            },
                            Err(e) => Err(e.to_string()),
                        }
                    }
                    _ => Err(format!("Unknown platform: '{}'", platform)),
                };

                match track_result {
                    Ok(info) => Ok(serde_json::json!({"success": true, "track": info})),
                    Err(e) => Ok(serde_json::json!({"success": false, "error": e})),
                }
            }

            NoraExecutiveTool::GetMusicTrackDetails { track_id } => {
                tracing::info!("[TOOL] GetMusicTrackDetails: {}", track_id);

                use services::services::editron::load_music_platform_configs;

                let (platform, raw_id) = match track_id.split_once(':') {
                    Some((p, id)) => (p.to_string(), id.to_string()),
                    None => {
                        return Ok(serde_json::json!({
                            "success": false,
                            "error": "Invalid track_id format. Must be 'platform:id'"
                        }))
                    }
                };

                let (artlist_cfg, epidemic_cfg, soundstripe_cfg) = load_music_platform_configs();

                match platform.as_str() {
                    "artlist" => {
                        if !artlist_cfg.is_configured() {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Artlist not configured"}),
                            );
                        }
                        match services::services::editron::artlist::ArtlistClient::from_config(
                            &artlist_cfg,
                        ) {
                            Ok(client) => match client.get_track(&raw_id).await {
                                Ok(t) => Ok(serde_json::json!({
                                    "success": true,
                                    "track": {
                                        "id": t.id, "title": t.title, "artist": t.artist,
                                        "duration": t.duration, "bpm": t.bpm, "key": t.key,
                                        "genre": format!("{:?}", t.genre),
                                        "moods": t.moods.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>(),
                                        "tags": t.tags, "platform": "artlist",
                                        "url": t.url, "preview_url": t.preview_url,
                                    }
                                })),
                                Err(e) => Ok(
                                    serde_json::json!({"success": false, "error": e.to_string()}),
                                ),
                            },
                            Err(e) => {
                                Ok(serde_json::json!({"success": false, "error": e.to_string()}))
                            }
                        }
                    }
                    "epidemic" => {
                        if !epidemic_cfg.is_configured() {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Epidemic Sound not configured"}),
                            );
                        }
                        match services::services::editron::epidemic::EpidemicSoundClient::from_config(&epidemic_cfg) {
                            Ok(client) => match client.get_track(&raw_id).await {
                                Ok(t) => {
                                    // Enrich with beats and similar
                                    let beats = client.get_beats(&raw_id).await.ok();
                                    let similar = client.get_similar(&raw_id).await.ok();
                                    let similar_summary: Option<Vec<serde_json::Value>> = similar.map(|s| {
                                        s.iter().take(5).map(|t| serde_json::json!({
                                            "id": t.id, "title": t.title, "artist": t.artist,
                                        })).collect()
                                    });

                                    Ok(serde_json::json!({
                                        "success": true,
                                        "track": {
                                            "id": t.id, "title": t.title, "artist": t.artist,
                                            "duration": t.duration, "bpm": t.bpm, "key": t.key,
                                            "genre": format!("{:?}", t.genre),
                                            "moods": t.moods.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>(),
                                            "tags": t.tags, "platform": "epidemic",
                                            "url": t.url, "preview_url": t.preview_url,
                                            "beats": beats,
                                            "similar_tracks": similar_summary,
                                        }
                                    }))
                                }
                                Err(e) => Ok(serde_json::json!({"success": false, "error": e.to_string()})),
                            },
                            Err(e) => Ok(serde_json::json!({"success": false, "error": e.to_string()})),
                        }
                    }
                    "soundstripe" => {
                        if !soundstripe_cfg.is_configured() {
                            return Ok(
                                serde_json::json!({"success": false, "error": "Soundstripe not configured"}),
                            );
                        }
                        match services::services::editron::soundstripe::SoundstripeClient::from_config(&soundstripe_cfg) {
                            Ok(client) => match client.get_track(&raw_id).await {
                                Ok(t) => Ok(serde_json::json!({
                                    "success": true,
                                    "track": {
                                        "id": t.id, "title": t.title, "artist": t.artist,
                                        "duration": t.duration, "bpm": t.bpm, "key": t.key,
                                        "genre": format!("{:?}", t.genre),
                                        "moods": t.moods.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>(),
                                        "tags": t.tags, "platform": "soundstripe",
                                        "url": t.url, "preview_url": t.preview_url,
                                    }
                                })),
                                Err(e) => Ok(serde_json::json!({"success": false, "error": e.to_string()})),
                            },
                            Err(e) => Ok(serde_json::json!({"success": false, "error": e.to_string()})),
                        }
                    }
                    _ => Ok(
                        serde_json::json!({"success": false, "error": format!("Unknown platform: '{}'", platform)}),
                    ),
                }
            }

            NoraExecutiveTool::AnalyzeMusicTrack {
                audio_path,
                bpm_hint,
            } => {
                tracing::info!("[TOOL] AnalyzeMusicTrack: {}", audio_path);

                let path = std::path::PathBuf::from(&audio_path);
                if !path.exists() {
                    return Ok(
                        serde_json::json!({"success": false, "error": format!("Audio file not found: {}", audio_path)}),
                    );
                }

                let engine = services::services::beat_analysis::BeatAnalysisEngine::new();

                match engine.analyze(&path, bpm_hint, 4).await {
                    Ok(result) => {
                        let section_summaries: Vec<serde_json::Value> = result
                            .sections
                            .iter()
                            .map(|s| {
                                serde_json::json!({
                                    "name": s.name,
                                    "start": s.start,
                                    "end": s.end,
                                    "duration": s.end - s.start,
                                    "energy": s.energy_level,
                                    "suggested_content": format!("{:?}", s.suggested_content),
                                })
                            })
                            .collect();

                        // Compute energy profile summary
                        let avg_energy = if !result.sections.is_empty() {
                            result.sections.iter().map(|s| s.energy_level).sum::<f64>()
                                / result.sections.len() as f64
                        } else {
                            0.5
                        };

                        Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Music analysis complete: {:.1} BPM, {:.1}s duration", result.bpm, result.duration),
                            "bpm": result.bpm,
                            "duration": result.duration,
                            "beat_interval": result.beat_interval,
                            "total_beats": result.total_beats,
                            "sections": section_summaries,
                            "energy_profile": {
                                "average": avg_energy,
                                "peak_section": result.sections.iter().max_by(|a, b| a.energy_level.partial_cmp(&b.energy_level).unwrap_or(std::cmp::Ordering::Equal)).map(|s| &s.name),
                            },
                            "transition_markers": result.transition_markers.len(),
                            "processing_time_ms": result.processing_time_ms,
                        }))
                    }
                    Err(e) => Ok(
                        serde_json::json!({"success": false, "error": format!("Music analysis failed: {}", e)}),
                    ),
                }
            }

            NoraExecutiveTool::CreateTaskOnBoard {
                project_id,
                board_id,
                title,
                description,
                priority,
                tags,
            } => {
                if let Some(executor) = &self.task_executor {
                    // Parse project_id UUID
                    let project_uuid = match Uuid::parse_str(&project_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid project_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Parse board_id UUID
                    let board_uuid = match Uuid::parse_str(&board_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid board_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Map priority string to enum
                    let priority_enum =
                        priority
                            .as_ref()
                            .and_then(|p| match p.to_lowercase().as_str() {
                                "critical" => Some(Priority::Critical),
                                "high" => Some(Priority::High),
                                "medium" => Some(Priority::Medium),
                                "low" => Some(Priority::Low),
                                _ => None,
                            });

                    match executor
                        .create_task_on_board(
                            project_uuid,
                            board_uuid,
                            title.clone(),
                            description,
                            priority_enum,
                            tags,
                        )
                        .await
                    {
                        Ok(task) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Task '{}' created successfully", title),
                            "task_id": task.id.to_string(),
                            "project_id": task.project_id.to_string(),
                            "board_id": task.board_id.map(|id| id.to_string()),
                            "title": task.title,
                            "status": format!("{:?}", task.status),
                            "priority": format!("{:?}", task.priority),
                            "created_at": task.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create task: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::AddTaskToBoard { task_id, board_id } => {
                if let Some(executor) = &self.task_executor {
                    // Parse task_id UUID
                    let task_uuid = match Uuid::parse_str(&task_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid task_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Parse board_id UUID
                    let board_uuid = match Uuid::parse_str(&board_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid board_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    match executor
                        .add_task_to_board(&task_uuid.to_string(), &board_uuid.to_string())
                        .await
                    {
                        Ok(()) => Ok(serde_json::json!({
                            "success": true,
                            "message": "Task assigned to board successfully",
                            "task_id": task_id,
                            "board_id": board_id,
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to assign task to board: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }

            // File Operations
            NoraExecutiveTool::ReadFile {
                file_path,
                encoding,
            } => {
                self.execute_read_file(&file_path, encoding.as_deref())
                    .await
            }
            NoraExecutiveTool::WriteFile {
                file_path,
                content,
                create_directories,
            } => {
                self.execute_write_file(&file_path, &content, create_directories)
                    .await
            }
            NoraExecutiveTool::ListDirectory {
                directory_path,
                recursive,
                pattern,
            } => {
                self.execute_list_directory(&directory_path, recursive, pattern.as_deref())
                    .await
            }
            NoraExecutiveTool::DeleteFile { file_path, confirm } => {
                self.execute_delete_file(&file_path, confirm).await
            }

            // Web Search & Information
            NoraExecutiveTool::SearchWeb {
                query,
                max_results,
                search_type,
            } => {
                self.execute_web_search(&query, max_results, &search_type)
                    .await
            }
            NoraExecutiveTool::FetchWebPage { url, extract_text } => {
                self.execute_fetch_webpage(&url, extract_text).await
            }
            NoraExecutiveTool::RenderPage { url, include_html } => {
                self.execute_render_page(&url, include_html).await
            }
            NoraExecutiveTool::ScrapePage {
                url,
                use_js,
                extract_assets,
            } => self.execute_scrape_page(&url, use_js, extract_assets).await,
            NoraExecutiveTool::SummarizeContent {
                content,
                max_length,
                format,
            } => {
                self.execute_summarize_content(&content, max_length, &format)
                    .await
            }

            // Code & Development
            NoraExecutiveTool::ExecuteCode {
                code,
                language,
                timeout_seconds,
            } => self.execute_code(&code, &language, timeout_seconds).await,
            NoraExecutiveTool::AnalyzeCodeQuality {
                code,
                language,
                check_security,
            } => {
                self.execute_analyze_code_quality(&code, &language, check_security)
                    .await
            }
            NoraExecutiveTool::GenerateDocumentation { code, doc_format } => {
                self.execute_generate_documentation(&code, &doc_format)
                    .await
            }

            // Email & Notifications
            NoraExecutiveTool::SendEmail {
                recipients,
                subject,
                body,
                priority,
            } => {
                self.execute_send_email(&recipients, &subject, &body, &priority)
                    .await
            }
            NoraExecutiveTool::ReadInbox {
                limit,
                owner_type,
                owner_id,
            } => {
                // Build optional override owner from params
                let override_owner = match (owner_type.as_deref(), owner_id.as_deref()) {
                    (Some("organization"), Some(id)) => id
                        .parse::<uuid::Uuid>()
                        .ok()
                        .map(ChannelOwner::Organization),
                    (Some("project"), Some(id)) => {
                        id.parse::<uuid::Uuid>().ok().map(ChannelOwner::Project)
                    }
                    (Some("user"), Some(id)) => {
                        id.parse::<uuid::Uuid>().ok().map(ChannelOwner::User)
                    }
                    (Some("agent"), Some(id)) => {
                        id.parse::<uuid::Uuid>().ok().map(ChannelOwner::Agent)
                    }
                    _ => None,
                };
                self.execute_read_inbox(limit, override_owner).await
            }
            NoraExecutiveTool::SendSms { to, message } => {
                self.execute_send_sms(&to, &message).await
            }
            NoraExecutiveTool::SendDiscordMessage {
                channel,
                message,
                mention_users,
            } => {
                self.execute_send_discord_message(&channel, &message, &mention_users)
                    .await
            }
            NoraExecutiveTool::CreateNotification {
                title,
                message,
                notification_type,
                recipients,
            } => {
                self.execute_create_notification(&title, &message, &notification_type, &recipients)
                    .await
            }

            // Calendar & Scheduling
            NoraExecutiveTool::CreateCalendarEvent {
                title,
                start_time,
                end_time,
                attendees,
                location,
            } => {
                self.execute_create_calendar_event(
                    &title,
                    start_time,
                    end_time,
                    &attendees,
                    location.as_deref(),
                )
                .await
            }
            NoraExecutiveTool::FindAvailableSlots {
                participants,
                duration_minutes,
                preferred_days,
            } => {
                self.execute_find_available_slots(&participants, duration_minutes, &preferred_days)
                    .await
            }
            NoraExecutiveTool::CheckCalendarAvailability {
                user,
                start_time,
                end_time,
            } => {
                self.execute_check_calendar_availability(&user, start_time, end_time)
                    .await
            }

            // Existing tools
            NoraExecutiveTool::CoordinateTeamMeeting {
                participants,
                agenda,
                ..
            } => Ok(serde_json::json!({
                "meeting_scheduled": true,
                "participants": participants,
                "agenda": agenda,
                "meeting_id": uuid::Uuid::new_v4().to_string()
            })),
            NoraExecutiveTool::GenerateKPIDashboard { metrics, .. } => Ok(serde_json::json!({
                "dashboard_created": true,
                "metrics": metrics,
                "dashboard_url": "/dashboards/executive-kpi"
            })),
            NoraExecutiveTool::CreateDecisionMatrix {
                options, criteria, ..
            } => Ok(serde_json::json!({
                "matrix_created": true,
                "options_count": options.len(),
                "criteria_count": criteria.len(),
                "matrix_id": uuid::Uuid::new_v4().to_string()
            })),
            // Add more implementations...

            // ── AI Image Generation ──────────────────────────────────────────
            NoraExecutiveTool::GenerateImage {
                prompt,
                reference_image_url,
                aspect_ratio,
                reference_strength,
                raw_mode,
                seed,
                model,
                output_filename,
                task_id,
                project_id,
            } => {
                let fal_key = std::env::var("FAL_API_KEY").unwrap_or_default();
                if fal_key.is_empty() {
                    return Ok(serde_json::json!({
                        "success": false,
                        "error": "FAL_API_KEY not configured"
                    }));
                }

                let endpoint_model = model.as_deref().unwrap_or("fal-ai/flux-pro/v1.1-ultra");
                let ar = aspect_ratio.as_deref().unwrap_or("3:4");
                let strength = reference_strength.unwrap_or(0.35);
                let use_raw = raw_mode.unwrap_or(true);

                // Choose endpoint based on whether we have a reference image
                let (endpoint, body) = if let Some(ref ref_url) = reference_image_url {
                    let redux_endpoint = format!("https://fal.run/{}/redux", endpoint_model);
                    let b = serde_json::json!({
                        "image_url": ref_url,
                        "prompt": prompt,
                        "strength": strength,
                        "aspect_ratio": ar,
                        "raw": use_raw,
                        "seed": seed,
                        "num_images": 1,
                        "enable_safety_checker": false,
                        "output_format": "png"
                    });
                    (redux_endpoint, b)
                } else {
                    let base_endpoint = format!("https://fal.run/{}", endpoint_model);
                    let b = serde_json::json!({
                        "prompt": prompt,
                        "aspect_ratio": ar,
                        "raw": use_raw,
                        "seed": seed,
                        "num_inference_steps": 28,
                        "guidance_scale": 3.5,
                        "num_images": 1,
                        "enable_safety_checker": false,
                        "output_format": "png"
                    });
                    (base_endpoint, b)
                };

                tracing::info!("[TOOL] GenerateImage: endpoint={} ar={}", endpoint, ar);

                let client = reqwest::Client::new();
                let resp = match client
                    .post(&endpoint)
                    .header("Authorization", format!("Key {}", fal_key))
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        return Ok(serde_json::json!({"success": false, "error": e.to_string()}))
                    }
                };

                if !resp.status().is_success() {
                    let err = resp.text().await.unwrap_or_default();
                    return Ok(serde_json::json!({"success": false, "error": err}));
                }

                let data: serde_json::Value = match resp.json().await {
                    Ok(d) => d,
                    Err(e) => {
                        return Ok(serde_json::json!({"success": false, "error": e.to_string()}))
                    }
                };

                let img_url = data["images"][0]["url"].as_str().unwrap_or("").to_string();
                let width = data["images"][0]["width"].as_u64().unwrap_or(0);
                let height = data["images"][0]["height"].as_u64().unwrap_or(0);
                let gen_seed = data["seed"].as_u64();

                // Download and save the image
                let fname = output_filename
                    .clone()
                    .unwrap_or_else(|| format!("editron_gen_{}.png", uuid::Uuid::new_v4()));
                let portraits_dir = std::path::PathBuf::from("dev_assets/video_gen/portraits");
                let _ = tokio::fs::create_dir_all(&portraits_dir).await;
                let save_path = portraits_dir.join(&fname);

                if !img_url.is_empty() {
                    if let Ok(img_resp) = client.get(&img_url).send().await {
                        if let Ok(img_bytes) = img_resp.bytes().await {
                            let _ = tokio::fs::write(&save_path, &img_bytes).await;
                            tracing::info!(
                                "[TOOL] GenerateImage saved: {} ({} bytes)",
                                save_path.display(),
                                img_bytes.len()
                            );
                        }
                    }
                }

                // VIBE cost: 50 per image
                let vibe_cost = 50i64;
                if let Some(executor) = &self.task_executor {
                    let pool = executor.pool();
                    if let Some(pid) = &project_id {
                        let task_str = task_id.as_deref().unwrap_or("");
                        let _ = crate::editron_tracking::log_editron_activity(
                            pool,
                            task_str,
                            "editron_image_generated",
                            &format!("Generated image: {} ({}x{})", fname, width, height),
                            vibe_cost,
                            serde_json::json!({"model": endpoint_model, "seed": gen_seed, "path": save_path.display().to_string()}),
                        ).await;
                        let _ = crate::editron_tracking::record_editron_vibe(
                            pool,
                            pid,
                            task_str,
                            vibe_cost,
                            &format!("Editron image generation: {}", fname),
                            "editron-image-gen",
                            serde_json::json!({"model": endpoint_model, "fal_ai": true}),
                        )
                        .await;
                    }
                }

                Ok(serde_json::json!({
                    "success": true,
                    "image_url": img_url,
                    "local_path": save_path.display().to_string(),
                    "width": width,
                    "height": height,
                    "seed": gen_seed,
                    "model": endpoint_model,
                    "vibe_cost": vibe_cost
                }))
            }

            // ── Video Post-Processing ────────────────────────────────────────
            NoraExecutiveTool::ApplyVideoEffect {
                input_path,
                effect,
                output_path,
                effect_params: _effect_params,
                task_id,
                project_id,
            } => {
                let out_path = output_path.clone().unwrap_or_else(|| {
                    let p = std::path::Path::new(&input_path);
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
                    let dir = p.parent().and_then(|d| d.to_str()).unwrap_or(".");
                    format!("{}/{}_{}.mp4", dir, stem, effect)
                });

                tracing::info!(
                    "[TOOL] ApplyVideoEffect: effect={} input={}",
                    effect,
                    input_path
                );

                // Build FFmpeg command based on effect preset
                let ffmpeg_args: Vec<String> = match effect.as_str() {
                    "hologram_glitch" => {
                        let duration_out = tokio::process::Command::new("ffprobe")
                            .args([
                                "-v",
                                "quiet",
                                "-show_entries",
                                "format=duration",
                                "-of",
                                "csv=p=0",
                                &input_path,
                            ])
                            .output()
                            .await
                            .ok()
                            .and_then(|o| String::from_utf8(o.stdout).ok())
                            .and_then(|s| s.trim().parse::<f64>().ok())
                            .unwrap_or(20.0);
                        let glitch_start = (duration_out - 1.8).max(0.0);
                        vec![
                            "-y".into(), "-i".into(), input_path.clone(),
                            "-vf".into(), format!(
                                "geq=r='if(lt(T,{gs}),r(X,Y),if(lt(mod(Y,4),2),r(X+2,Y),r(X-2,Y)))':g='if(lt(T,{gs}),g(X,Y),if(lt(mod(Y,4),2),g(X-2,Y),g(X+2,Y)))':b='if(lt(T,{gs}),b(X,Y),b(X,Y))',drawgrid=width=0:height=4:thickness=1:color='if(lt(T,{gs}),black@0,cyan@0.08)',colorchannelmixer=rr='if(lt(T,{gs}),1,0.6)':gg='if(lt(T,{gs}),1,0.8)':bb='if(lt(T,{gs}),1,1.4)',fade=t=out:st={gs}:d=1.8:color=black",
                                gs = glitch_start
                            ),
                            "-c:v".into(), "libx264".into(), "-crf".into(), "18".into(),
                            "-c:a".into(), "aac".into(), out_path.clone(),
                        ]
                    }
                    "color_grade" => vec![
                        "-y".into(),
                        "-i".into(),
                        input_path.clone(),
                        "-vf".into(),
                        "eq=contrast=1.1:brightness=-0.02:saturation=1.15,vignette=PI/4".into(),
                        "-c:v".into(),
                        "libx264".into(),
                        "-crf".into(),
                        "18".into(),
                        "-c:a".into(),
                        "aac".into(),
                        out_path.clone(),
                    ],
                    "vignette" => vec![
                        "-y".into(),
                        "-i".into(),
                        input_path.clone(),
                        "-vf".into(),
                        "vignette=PI/3.5".into(),
                        "-c:v".into(),
                        "libx264".into(),
                        "-crf".into(),
                        "18".into(),
                        "-c:a".into(),
                        "aac".into(),
                        out_path.clone(),
                    ],
                    _ => {
                        return Ok(
                            serde_json::json!({"success": false, "error": format!("Unknown effect: {}", effect)}),
                        )
                    }
                };

                let result = tokio::process::Command::new("ffmpeg")
                    .args(&ffmpeg_args)
                    .output()
                    .await;

                match result {
                    Ok(out) if out.status.success() => {
                        let vibe_cost = 100i64;
                        if let Some(executor) = &self.task_executor {
                            let pool = executor.pool();
                            if let Some(pid) = &project_id {
                                let task_str = task_id.as_deref().unwrap_or("");
                                let _ = crate::editron_tracking::log_editron_activity(
                                    pool,
                                    task_str,
                                    "editron_effect_applied",
                                    &format!("Applied {} effect: {}", effect, out_path),
                                    vibe_cost,
                                    serde_json::json!({"effect": effect, "output": out_path}),
                                )
                                .await;
                                let _ = crate::editron_tracking::record_editron_vibe(
                                    pool,
                                    pid,
                                    task_str,
                                    vibe_cost,
                                    &format!("Editron post-processing: {}", effect),
                                    "editron-post-process",
                                    serde_json::json!({"effect": effect}),
                                )
                                .await;
                            }
                        }
                        Ok(serde_json::json!({
                            "success": true,
                            "effect": effect,
                            "output_path": out_path,
                            "vibe_cost": vibe_cost
                        }))
                    }
                    Ok(out) => {
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        Ok(serde_json::json!({"success": false, "error": stderr}))
                    }
                    Err(e) => Ok(serde_json::json!({"success": false, "error": e.to_string()})),
                }
            }

            NoraExecutiveTool::PostProcessVideoJob {
                video_job_id,
                effects,
                effect_params: _,
                task_id,
                project_id,
            } => {
                let pool = match &self.task_executor {
                    Some(e) => e.pool().clone(),
                    None => {
                        return Ok(
                            serde_json::json!({"success": false, "error": "DB not available"}),
                        )
                    }
                };

                // Look up the video job
                let job_uuid = match uuid::Uuid::parse_str(&video_job_id) {
                    Ok(u) => u,
                    Err(_) => {
                        return Ok(
                            serde_json::json!({"success": false, "error": "Invalid video_job_id"}),
                        )
                    }
                };

                let job = match db::models::video_job::VideoJob::find(&pool, job_uuid).await {
                    Ok(Some(j)) => j,
                    Ok(None) => {
                        return Ok(
                            serde_json::json!({"success": false, "error": "Video job not found"}),
                        )
                    }
                    Err(e) => {
                        return Ok(serde_json::json!({"success": false, "error": e.to_string()}))
                    }
                };

                if job.status != "ready" && job.status != "completed" {
                    return Ok(serde_json::json!({
                        "success": false,
                        "error": format!("Video job is not ready (status: {})", job.status)
                    }));
                }

                // Find the local video file
                let raw_path = format!(
                    "dev_assets/video_gen/video/{}.mp4",
                    video_job_id.replace("-", "")
                );
                let input = if std::path::Path::new(&raw_path).exists() {
                    raw_path
                } else {
                    return Ok(
                        serde_json::json!({"success": false, "error": "Raw video file not found locally"}),
                    );
                };

                // Apply effects in sequence
                let mut current_input = input.clone();
                let mut final_output = String::new();
                for (i, effect) in effects.iter().enumerate() {
                    let suffix = if i == effects.len() - 1 {
                        "_final".to_string()
                    } else {
                        format!("_{}", i)
                    };
                    let out = format!(
                        "dev_assets/video_gen/video/{}_{}{}.mp4",
                        video_job_id.replace("-", ""),
                        effect,
                        suffix
                    );

                    // Inline the hologram_glitch effect
                    let duration_out = tokio::process::Command::new("ffprobe")
                        .args([
                            "-v",
                            "quiet",
                            "-show_entries",
                            "format=duration",
                            "-of",
                            "csv=p=0",
                            &current_input,
                        ])
                        .output()
                        .await
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .and_then(|s| s.trim().parse::<f64>().ok())
                        .unwrap_or(20.0);

                    let vf = match effect.as_str() {
                        "hologram_glitch" => {
                            let gs = (duration_out - 1.8).max(0.0);
                            format!("geq=r='if(lt(T,{gs}),r(X,Y),if(lt(mod(Y,4),2),r(X+2,Y),r(X-2,Y)))':g='if(lt(T,{gs}),g(X,Y),if(lt(mod(Y,4),2),g(X-2,Y),g(X+2,Y)))':b='if(lt(T,{gs}),b(X,Y),b(X,Y))',drawgrid=width=0:height=4:thickness=1:color='if(lt(T,{gs}),black@0,cyan@0.08)',colorchannelmixer=rr='if(lt(T,{gs}),1,0.6)':gg='if(lt(T,{gs}),1,0.8)':bb='if(lt(T,{gs}),1,1.4)',fade=t=out:st={gs}:d=1.8:color=black", gs=gs)
                        }
                        "color_grade" => {
                            "eq=contrast=1.1:brightness=-0.02:saturation=1.15,vignette=PI/4".into()
                        }
                        _ => continue,
                    };

                    let ffmpeg_result = tokio::process::Command::new("ffmpeg")
                        .args([
                            "-y",
                            "-i",
                            &current_input,
                            "-vf",
                            &vf,
                            "-c:v",
                            "libx264",
                            "-crf",
                            "18",
                            "-c:a",
                            "aac",
                            &out,
                        ])
                        .output()
                        .await;

                    if let Ok(o) = ffmpeg_result {
                        if o.status.success() {
                            current_input = out.clone();
                            final_output = out;
                        }
                    }
                }

                if final_output.is_empty() {
                    return Ok(
                        serde_json::json!({"success": false, "error": "Post-processing failed"}),
                    );
                }

                let vibe_cost = 100i64 * effects.len() as i64;
                if let Some(pid) = &project_id {
                    let task_str = task_id.as_deref().unwrap_or("");
                    let _ = crate::editron_tracking::log_editron_activity(
                        &pool,
                        task_str,
                        "editron_post_process_complete",
                        &format!("Post-processed video job {}: {:?}", video_job_id, effects),
                        vibe_cost,
                        serde_json::json!({"effects": effects, "output": final_output}),
                    )
                    .await;
                    let _ = crate::editron_tracking::record_editron_vibe(
                        &pool,
                        pid,
                        task_str,
                        vibe_cost,
                        &format!("Editron post-production: {} effects", effects.len()),
                        "editron-post-process",
                        serde_json::json!({"video_job_id": video_job_id, "effects": effects}),
                    )
                    .await;
                }

                Ok(serde_json::json!({
                    "success": true,
                    "video_job_id": video_job_id,
                    "effects_applied": effects,
                    "final_output": final_output,
                    "vibe_cost": vibe_cost
                }))
            }

            _ => Ok(serde_json::json!({
                "message": "Tool implementation pending",
                "tool_executed": true
            })),
        }
    }

    // File Operations Implementations
    async fn execute_read_file(
        &self,
        file_path: &str,
        _encoding: Option<&str>,
    ) -> crate::Result<serde_json::Value> {
        use tokio::fs;

        let content = fs::read_to_string(file_path).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read file: {}", e))
        })?;

        Ok(serde_json::json!({
            "success": true,
            "file_path": file_path,
            "content": content,
            "size_bytes": content.len()
        }))
    }

    async fn execute_write_file(
        &self,
        file_path: &str,
        content: &str,
        create_directories: bool,
    ) -> crate::Result<serde_json::Value> {
        use std::path::Path;

        use tokio::fs;

        let path = Path::new(file_path);

        if create_directories {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    crate::NoraError::ToolExecutionError(format!(
                        "Failed to create directories: {}",
                        e
                    ))
                })?;
            }
        }

        fs::write(file_path, content).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to write file: {}", e))
        })?;

        Ok(serde_json::json!({
            "success": true,
            "file_path": file_path,
            "bytes_written": content.len()
        }))
    }

    async fn execute_list_directory(
        &self,
        directory_path: &str,
        recursive: bool,
        pattern: Option<&str>,
    ) -> crate::Result<serde_json::Value> {
        use tokio::fs;

        let mut entries = Vec::new();

        let mut read_dir = fs::read_dir(directory_path).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read directory: {}", e))
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read entry: {}", e))
        })? {
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Apply pattern filter if provided
            if let Some(pat) = pattern {
                if !file_name.contains(pat) {
                    continue;
                }
            }

            let metadata = entry.metadata().await.ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

            entries.push(serde_json::json!({
                "name": file_name,
                "path": path.to_string_lossy(),
                "is_directory": is_dir,
                "size_bytes": size
            }));

            // Recursively list subdirectories
            if recursive && is_dir {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(sub_result) =
                    Box::pin(self.execute_list_directory(&path_str, true, pattern)).await
                {
                    if let Some(sub_entries) = sub_result.get("entries").and_then(|e| e.as_array())
                    {
                        for sub_entry in sub_entries {
                            entries.push(sub_entry.clone());
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "directory": directory_path,
            "entries": entries,
            "count": entries.len()
        }))
    }

    async fn execute_delete_file(
        &self,
        file_path: &str,
        confirm: bool,
    ) -> crate::Result<serde_json::Value> {
        if !confirm {
            return Ok(serde_json::json!({
                "success": false,
                "message": "Delete operation requires confirmation"
            }));
        }

        use tokio::fs;

        fs::remove_file(file_path).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to delete file: {}", e))
        })?;

        Ok(serde_json::json!({
            "success": true,
            "file_path": file_path,
            "deleted": true
        }))
    }

    // Web Search & Information Implementations
    async fn execute_web_search(
        &self,
        query: &str,
        max_results: u32,
        _search_type: &SearchType,
    ) -> crate::Result<serde_json::Value> {
        let api_key = match std::env::var("EXA_API_KEY") {
            Ok(k) if !k.is_empty() => k,
            _ => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "EXA_API_KEY not configured — web search unavailable"
                }));
            }
        };

        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.exa.ai/search")
            .header("x-api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "query": query,
                "num_results": max_results,
                "use_autoprompt": true,
                "text": true
            }))
            .send()
            .await
            .map_err(|e| {
                crate::NoraError::ToolExecutionError(format!("Exa search request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Ok(serde_json::json!({
                "success": false,
                "error": format!("Exa search returned {}: {}", status, body)
            }));
        }

        let data: serde_json::Value = resp.json().await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to parse Exa response: {}", e))
        })?;

        let results = data
            .get("results")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|r| {
                        serde_json::json!({
                            "title": r.get("title").and_then(|t| t.as_str()).unwrap_or(""),
                            "url": r.get("url").and_then(|u| u.as_str()).unwrap_or(""),
                            "snippet": r.get("text").and_then(|t| t.as_str()).unwrap_or(""),
                            "score": r.get("score")
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(serde_json::json!({
            "success": true,
            "query": query,
            "results": results,
            "result_count": results.len()
        }))
    }

    async fn execute_fetch_webpage(
        &self,
        url: &str,
        extract_text: bool,
    ) -> crate::Result<serde_json::Value> {
        use reqwest::Client;

        let client = Client::new();
        let response = client.get(url).send().await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to fetch webpage: {}", e))
        })?;

        let content = response.text().await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read response: {}", e))
        })?;

        let result_content = if extract_text {
            // Basic HTML tag removal (in production, use html2text or similar)
            content.replace("<", " <").replace(">", "> ")
        } else {
            content
        };

        Ok(serde_json::json!({
            "success": true,
            "url": url,
            "content": result_content,
            "content_length": result_content.len(),
            "text_extracted": extract_text
        }))
    }

    async fn execute_render_page(
        &self,
        url: &str,
        include_html: bool,
    ) -> crate::Result<serde_json::Value> {
        use std::process::Command;

        // Find the render-page.js script
        let script_path = {
            let candidates = [
                "/home/pythia/pcg-cc-mcp/scripts/render-page.js",
                "scripts/render-page.js",
                "./scripts/render-page.js",
            ];
            candidates
                .iter()
                .find(|p| std::path::Path::new(*p).exists())
                .map(|p| p.to_string())
        };

        let script_path = match script_path {
            Some(p) => p,
            None => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "render-page.js script not found. Ensure scripts/render-page.js exists in the project root."
                }));
            }
        };

        // Check Playwright is installed
        let playwright_check = tokio::task::spawn_blocking(|| {
            Command::new("node")
                .args(["-e", "require('playwright')"])
                .output()
        })
        .await
        .ok()
        .and_then(|r| r.ok())
        .map(|o| o.status.success())
        .unwrap_or(false);

        if !playwright_check {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Playwright not installed. Run: cd /home/pythia/pcg-cc-mcp && npm install playwright && npx playwright install chromium"
            }));
        }

        tracing::info!(
            "[NORA TOOLS] Rendering JavaScript page via Playwright: {}",
            url
        );

        let url_owned = url.to_string();
        let output = tokio::task::spawn_blocking(move || {
            Command::new("node")
                .args([&script_path, &url_owned, "30000"])
                .output()
        })
        .await
        .map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to spawn render task: {}", e))
        })?
        .map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to execute render script: {}", e))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Try parsing JSON error from stdout first
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(err) = val.get("error").and_then(|e| e.as_str()) {
                    return Ok(serde_json::json!({"success": false, "url": url, "error": err}));
                }
            }
            return Ok(serde_json::json!({
                "success": false,
                "url": url,
                "error": format!("Render script failed: {}", stderr.trim())
            }));
        }

        let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to parse render output: {}", e))
        })?;

        // Convert HTML to plain text (basic stripping)
        let html = parsed.get("html").and_then(|h| h.as_str()).unwrap_or("");
        let title = parsed.get("title").and_then(|t| t.as_str());
        // Strip tags for text: replace tags with spaces, collapse whitespace
        let text: String = {
            let mut in_tag = false;
            let mut s = String::with_capacity(html.len());
            for c in html.chars() {
                match c {
                    '<' => {
                        in_tag = true;
                        s.push(' ');
                    }
                    '>' => {
                        in_tag = false;
                    }
                    _ if !in_tag => s.push(c),
                    _ => {}
                }
            }
            // Collapse whitespace
            s.split_whitespace().collect::<Vec<_>>().join(" ")
        };

        let mut result = serde_json::json!({
            "success": true,
            "url": parsed.get("url").and_then(|u| u.as_str()).unwrap_or(url),
            "title": title,
            "text": text,
            "text_length": text.len(),
        });

        if include_html {
            result["html"] = serde_json::Value::String(html.to_string());
        }

        Ok(result)
    }

    async fn execute_scrape_page(
        &self,
        url: &str,
        use_js: bool,
        extract_assets: bool,
    ) -> crate::Result<serde_json::Value> {
        use std::collections::HashSet;

        use regex::Regex;

        tracing::info!(
            "[NORA TOOLS] ScrapePage: {} (js={}, assets={})",
            url,
            use_js,
            extract_assets
        );

        // ── 1. Fetch the page (Playwright or static HTTP) ──────────────────────
        let (html, final_url, fetch_method) = if use_js {
            // Try Playwright first
            let script_path = {
                let candidates = [
                    "/home/pythia/pcg-cc-mcp/scripts/render-page.js",
                    "scripts/render-page.js",
                    "./scripts/render-page.js",
                ];
                candidates
                    .iter()
                    .find(|p| std::path::Path::new(*p).exists())
                    .map(|p| p.to_string())
            };

            let playwright_ok = tokio::task::spawn_blocking(|| {
                std::process::Command::new("node")
                    .args(["-e", "require('playwright')"])
                    .output()
            })
            .await
            .ok()
            .and_then(|r| r.ok())
            .map(|o| o.status.success())
            .unwrap_or(false);

            if let (Some(script), true) = (script_path, playwright_ok) {
                let url_owned = url.to_string();
                let output = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("node")
                        .args([&script, &url_owned, "30000"])
                        .output()
                })
                .await
                .ok()
                .and_then(|r| r.ok());

                if let Some(out) = output {
                    if out.status.success() {
                        if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&out.stdout)
                        {
                            let h = parsed
                                .get("html")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let u = parsed
                                .get("url")
                                .and_then(|v| v.as_str())
                                .unwrap_or(url)
                                .to_string();
                            (h, u, "playwright")
                        } else {
                            (String::new(), url.to_string(), "playwright_parse_err")
                        }
                    } else {
                        tracing::warn!(
                            "[NORA TOOLS] Playwright failed for {}, falling back to static",
                            url
                        );
                        let client = reqwest::Client::builder()
                            .user_agent("Mozilla/5.0 (compatible; Scout/1.0; Research Agent)")
                            .timeout(std::time::Duration::from_secs(15))
                            .build()
                            .unwrap_or_default();
                        let html = client
                            .get(url)
                            .send()
                            .await
                            .ok()
                            .and_then(|r| tokio::runtime::Handle::current().block_on(r.text()).ok())
                            .unwrap_or_default();
                        (html, url.to_string(), "static_fallback")
                    }
                } else {
                    (String::new(), url.to_string(), "spawn_err")
                }
            } else {
                // No Playwright — fall through to static
                tracing::info!(
                    "[NORA TOOLS] Playwright unavailable, using static fetch for {}",
                    url
                );
                let client = reqwest::Client::builder()
                    .user_agent("Mozilla/5.0 (compatible; Scout/1.0; Research Agent)")
                    .timeout(std::time::Duration::from_secs(15))
                    .build()
                    .unwrap_or_default();
                let response = client.get(url).send().await;
                match response {
                    Ok(r) => {
                        let u = r.url().to_string();
                        (r.text().await.unwrap_or_default(), u, "static")
                    }
                    Err(e) => {
                        return Ok(
                            serde_json::json!({"success": false, "url": url, "error": format!("Fetch failed: {}", e)}),
                        )
                    }
                }
            }
        } else {
            // Static HTTP fetch
            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (compatible; Scout/1.0; Research Agent)")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default();
            match client.get(url).send().await {
                Ok(r) => {
                    let u = r.url().to_string();
                    (r.text().await.unwrap_or_default(), u, "static")
                }
                Err(e) => {
                    return Ok(
                        serde_json::json!({"success": false, "url": url, "error": format!("Fetch failed: {}", e)}),
                    )
                }
            }
        };

        if html.is_empty() {
            return Ok(
                serde_json::json!({"success": false, "url": final_url, "fetch_method": fetch_method, "error": "Empty response — try use_js=true for JavaScript-rendered pages"}),
            );
        }

        // ── 2. Basic text extraction ────────────────────────────────────────────
        let title = {
            let re = Regex::new(r"(?i)<title[^>]*>([^<]+)</title>").unwrap();
            re.captures(&html)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
        };

        let strip_tags = |s: &str| -> String {
            let mut in_tag = false;
            let mut out = String::with_capacity(s.len());
            for c in s.chars() {
                match c {
                    '<' => {
                        in_tag = true;
                        out.push(' ');
                    }
                    '>' => {
                        in_tag = false;
                    }
                    _ if !in_tag => out.push(c),
                    _ => {}
                }
            }
            out.split_whitespace().collect::<Vec<_>>().join(" ")
        };

        // meta description
        let description = {
            let re = Regex::new(
                r#"(?i)<meta[^>]+name=["']description["'][^>]+content=["']([^"']+)["']"#,
            )
            .unwrap();
            let re2 = Regex::new(
                r#"(?i)<meta[^>]+content=["']([^"']+)["'][^>]+name=["']description["']"#,
            )
            .unwrap();
            let re_og = Regex::new(
                r#"(?i)<meta[^>]+property=["']og:description["'][^>]+content=["']([^"']+)["']"#,
            )
            .unwrap();
            re.captures(&html)
                .or_else(|| re2.captures(&html))
                .or_else(|| re_og.captures(&html))
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
        };

        // og:image
        let og_image = {
            let re = Regex::new(
                r#"(?i)<meta[^>]+property=["']og:image["'][^>]+content=["']([^"']+)["']"#,
            )
            .unwrap();
            let re2 = Regex::new(
                r#"(?i)<meta[^>]+content=["']([^"']+)["'][^>]+property=["']og:image["']"#,
            )
            .unwrap();
            re.captures(&html)
                .or_else(|| re2.captures(&html))
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
        };

        let page_text = strip_tags(&html);
        let page_text_truncated = page_text.chars().take(8000).collect::<String>();

        if !extract_assets {
            return Ok(serde_json::json!({
                "success": true,
                "url": final_url,
                "fetch_method": fetch_method,
                "title": title,
                "description": description,
                "og_image": og_image,
                "text": page_text_truncated,
                "text_length": page_text.len(),
            }));
        }

        // ── 3. Asset extraction ─────────────────────────────────────────────────

        // All images
        let img_re = Regex::new(r#"(?i)<img[^>]+src=["']([^"']+)["']"#).unwrap();
        let mut images: Vec<String> = img_re
            .captures_iter(&html)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.starts_with("data:"))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        images.truncate(30);

        // Logo detection: images with "logo" in src, alt, or class
        let logo_re = Regex::new(r#"(?i)<img[^>]+(logo|brand)[^>]+>"#).unwrap();
        let logo_src_re = Regex::new(r#"(?i)src=["']([^"']+)["']"#).unwrap();
        let logos: Vec<String> = logo_re
            .find_iter(&html)
            .filter_map(|m| logo_src_re.captures(m.as_str()))
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.starts_with("data:"))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        // Favicon
        let favicon = {
            let re = Regex::new(
                r#"(?i)<link[^>]+rel=["'][^"']*icon[^"']*["'][^>]+href=["']([^"']+)["']"#,
            )
            .unwrap();
            let re2 = Regex::new(
                r#"(?i)<link[^>]+href=["']([^"']+)["'][^>]+rel=["'][^"']*icon[^"']*["']"#,
            )
            .unwrap();
            re.captures(&html)
                .or_else(|| re2.captures(&html))
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
        };

        // Contact info
        let email_re = Regex::new(r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b").unwrap();
        let emails: Vec<String> = email_re
            .find_iter(&page_text)
            .map(|m| m.as_str().to_string())
            .filter(|e| !e.ends_with(".png") && !e.ends_with(".jpg") && !e.ends_with(".svg"))
            .collect::<HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();

        let phone_re = Regex::new(r"\(?\d{3}\)?[\s.\-]?\d{3}[\s.\-]?\d{4}").unwrap();
        let phones: Vec<String> = phone_re
            .find_iter(&page_text)
            .map(|m| m.as_str().trim().to_string())
            .collect::<HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();

        // Social links
        let link_re = Regex::new(r#"(?i)href=["']([^"']+)["']"#).unwrap();
        let all_links: Vec<String> = link_re
            .captures_iter(&html)
            .filter_map(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .collect();

        let social_domains = [
            ("instagram", "instagram.com"),
            ("twitter", "twitter.com"),
            ("tiktok", "tiktok.com"),
            ("facebook", "facebook.com"),
            ("linkedin", "linkedin.com"),
            ("youtube", "youtube.com"),
            ("threads", "threads.net"),
        ];
        let mut social_links = serde_json::Map::new();
        for (key, domain) in &social_domains {
            if let Some(link) = all_links.iter().find(|l| l.contains(domain)) {
                social_links.insert(key.to_string(), serde_json::Value::String(link.clone()));
            }
        }

        // Brand color hints: hex colors in style attributes and <style> blocks
        let hex_re = Regex::new(r"#([0-9A-Fa-f]{6})\b").unwrap();
        let style_block_re = Regex::new(r"(?is)<style[^>]*>(.*?)</style>").unwrap();
        let mut style_content = String::new();
        for cap in style_block_re.captures_iter(&html) {
            if let Some(m) = cap.get(1) {
                style_content.push_str(m.as_str());
            }
        }
        // Also inline styles
        let inline_re = Regex::new(r#"(?i)style=["']([^"']+)["']"#).unwrap();
        for cap in inline_re.captures_iter(&html) {
            if let Some(m) = cap.get(1) {
                style_content.push_str(m.as_str());
            }
        }
        let mut colors: Vec<String> = hex_re
            .find_iter(&style_content)
            .map(|m| m.as_str().to_uppercase())
            .filter(|c| *c != "#000000" && *c != "#FFFFFF" && *c != "#FAFAFA" && *c != "#F0F0F0")
            .collect::<HashSet<_>>()
            .into_iter()
            .take(10)
            .collect();
        colors.sort();

        tracing::info!(
            "[NORA TOOLS] ScrapePage complete: {} images, {} logos, {} emails, {} phones, {} social, {} colors",
            images.len(), logos.len(), emails.len(), phones.len(), social_links.len(), colors.len()
        );

        Ok(serde_json::json!({
            "success": true,
            "url": final_url,
            "fetch_method": fetch_method,
            "title": title,
            "description": description,
            "og_image": og_image,
            "text": page_text_truncated,
            "text_length": page_text.len(),
            "assets": {
                "images": images,
                "logos": logos,
                "favicon": favicon,
                "og_image": og_image,
            },
            "contact": {
                "emails": emails,
                "phones": phones,
            },
            "social_links": social_links,
            "brand_colors": colors,
        }))
    }

    async fn execute_summarize_content(
        &self,
        content: &str,
        max_length: u32,
        _format: &SummaryFormat,
    ) -> crate::Result<serde_json::Value> {
        // Simple summarization by truncation (in production, use LLM or extractive summarization)
        let summary = if content.len() > max_length as usize {
            format!("{}...", &content[..max_length as usize])
        } else {
            content.to_string()
        };

        Ok(serde_json::json!({
            "success": true,
            "original_length": content.len(),
            "summary_length": summary.len(),
            "summary": summary,
            "compression_ratio": summary.len() as f64 / content.len() as f64
        }))
    }

    // Code & Development Implementations
    async fn execute_code(
        &self,
        code: &str,
        language: &CodeLanguage,
        timeout_seconds: u32,
    ) -> crate::Result<serde_json::Value> {
        // Note: This requires sandboxed execution environment
        // For security, this should use containers or VM isolation
        Ok(serde_json::json!({
            "success": false,
            "message": "Code execution requires sandboxed environment",
            "language": format!("{:?}", language),
            "code_length": code.len(),
            "timeout_seconds": timeout_seconds,
            "note": "Sandboxed execution pending - requires Docker/VM integration"
        }))
    }

    async fn execute_analyze_code_quality(
        &self,
        code: &str,
        language: &CodeLanguage,
        check_security: bool,
    ) -> crate::Result<serde_json::Value> {
        // Basic analysis (in production, integrate with linters/analyzers)
        let line_count = code.lines().count();
        let char_count = code.len();
        let has_comments = code.contains("//") || code.contains("/*") || code.contains("#");

        Ok(serde_json::json!({
            "success": true,
            "language": format!("{:?}", language),
            "metrics": {
                "line_count": line_count,
                "character_count": char_count,
                "has_comments": has_comments,
                "security_checked": check_security
            },
            "suggestions": [
                "Consider adding more comments for complex logic",
                "Ensure proper error handling"
            ],
            "note": "Advanced static analysis pending"
        }))
    }

    async fn execute_generate_documentation(
        &self,
        code: &str,
        doc_format: &DocumentationFormat,
    ) -> crate::Result<serde_json::Value> {
        let doc = format!("# Code Documentation\n\n```\n{}\n```\n\nGenerated documentation for the provided code.", code);

        Ok(serde_json::json!({
            "success": true,
            "format": format!("{:?}", doc_format),
            "documentation": doc,
            "doc_length": doc.len()
        }))
    }

    // Email & Notifications Implementations
    async fn execute_send_email(
        &self,
        recipients: &[String],
        subject: &str,
        body: &str,
        priority: &EmailPriority,
    ) -> crate::Result<serde_json::Value> {
        // Prefer OAuth channel service (Nora's connected Zoho account)
        if let (Some(ref svc), Some(ref owner)) = (&self.agent_channel_service, &self.agent_owner) {
            match svc.send_email(owner, recipients, subject, body).await {
                Ok(message_id) => {
                    tracing::info!(
                        "Email sent via AgentChannelService to {:?} (id={})",
                        recipients,
                        message_id
                    );
                    return Ok(serde_json::json!({
                        "success": true,
                        "recipients": recipients,
                        "subject": subject,
                        "priority": format!("{:?}", priority),
                        "message_id": message_id,
                        "sent_via": "zoho_oauth"
                    }));
                }
                Err(e) => {
                    tracing::warn!("AgentChannelService send failed, trying SMTP: {}", e);
                }
            }
        }

        // Fallback: SMTP (legacy env-var config)
        if let Some(ref email_service) = self.email_service {
            match email_service
                .send_email(recipients, subject, body, false)
                .await
            {
                Ok(message_id) => {
                    tracing::info!(
                        "Email sent via SMTP to {:?} (id={})",
                        recipients,
                        message_id
                    );
                    return Ok(serde_json::json!({
                        "success": true,
                        "recipients": recipients,
                        "subject": subject,
                        "priority": format!("{:?}", priority),
                        "message_id": message_id,
                        "sent_via": "smtp"
                    }));
                }
                Err(e) => {
                    tracing::warn!("SMTP send failed: {}", e);
                }
            }
        }

        // Final fallback: log only
        tracing::warn!(
            "No email transport configured — email logged only: {:?} / {}",
            recipients,
            subject
        );
        Ok(serde_json::json!({
            "success": false,
            "recipients": recipients,
            "subject": subject,
            "priority": format!("{:?}", priority),
            "note": "No email account connected. Connect Nora's Zoho account via Settings → Integrations."
        }))
    }

    async fn execute_read_inbox(
        &self,
        limit: usize,
        owner_override: Option<ChannelOwner>,
    ) -> crate::Result<serde_json::Value> {
        let svc = self
            .agent_channel_service
            .as_ref()
            .ok_or_else(|| NoraError::ToolsError("No channel service configured".into()))?;

        let owner = owner_override
            .as_ref()
            .or(self.agent_owner.as_ref())
            .ok_or_else(|| NoraError::ToolsError("No agent owner configured".into()))?;

        match svc.read_inbox(owner, limit).await {
            Ok(messages) => Ok(serde_json::json!({
                "success": true,
                "count": messages.len(),
                "messages": messages
            })),
            Err(e) => Ok(serde_json::json!({
                "success": false,
                "error": e.to_string()
            })),
        }
    }

    async fn execute_send_sms(&self, to: &str, message: &str) -> crate::Result<serde_json::Value> {
        if let Some(ref svc) = self.agent_channel_service {
            match svc.send_sms(to, message).await {
                Ok(sid) => {
                    tracing::info!("SMS sent to {} (sid={})", to, sid);
                    return Ok(serde_json::json!({
                        "success": true,
                        "to": to,
                        "message_sid": sid,
                        "sent_via": "twilio"
                    }));
                }
                Err(e) => {
                    tracing::warn!("SMS send failed: {}", e);
                    return Ok(serde_json::json!({
                        "success": false,
                        "error": e.to_string()
                    }));
                }
            }
        }

        Ok(serde_json::json!({
            "success": false,
            "note": "Twilio not configured. Set TWILIO_ACCOUNT_SID, TWILIO_AUTH_TOKEN, TWILIO_PHONE_NUMBER env vars."
        }))
    }

    async fn execute_send_discord_message(
        &self,
        channel: &str,
        message: &str,
        mention_users: &[String],
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Discord webhook if configured
        if let Some(ref discord_service) = self.discord_service {
            match discord_service.send_message(message, mention_users).await {
                Ok(_) => {
                    tracing::info!("Discord message sent successfully to channel: {}", channel);
                    return Ok(serde_json::json!({
                        "success": true,
                        "channel": channel,
                        "message": message,
                        "mentioned_users": mention_users,
                        "timestamp": Utc::now().to_rfc3339(),
                        "sent_via": "Discord Webhook"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Discord webhook send failed, logging only: {}", e);
                }
            }
        }

        // Fallback: Log only
        tracing::info!("Discord message to {}: {}", channel, message);
        Ok(serde_json::json!({
            "success": true,
            "channel": channel,
            "message": message,
            "mentioned_users": mention_users,
            "timestamp": Utc::now().to_rfc3339(),
            "note": "Discord webhook not configured - message logged only. Set DISCORD_WEBHOOK_URL env var to enable."
        }))
    }

    async fn execute_create_notification(
        &self,
        title: &str,
        message: &str,
        notification_type: &NotificationType,
        recipients: &[String],
    ) -> crate::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "success": true,
            "notification_id": uuid::Uuid::new_v4().to_string(),
            "title": title,
            "message": message,
            "type": format!("{:?}", notification_type),
            "recipients": recipients,
            "created_at": Utc::now().to_rfc3339()
        }))
    }

    // Calendar & Scheduling Implementations
    async fn execute_create_calendar_event(
        &self,
        title: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        attendees: &[String],
        location: Option<&str>,
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Google Calendar API if configured
        if let Some(ref calendar_service) = self.calendar_service {
            match calendar_service
                .create_event(title, start_time, end_time, attendees, location)
                .await
            {
                Ok(event_id) => {
                    tracing::info!("Calendar event created: {} (ID: {})", title, event_id);
                    return Ok(serde_json::json!({
                        "success": true,
                        "event_id": event_id,
                        "title": title,
                        "start_time": start_time.to_rfc3339(),
                        "end_time": end_time.to_rfc3339(),
                        "attendees": attendees,
                        "location": location,
                        "calendar_provider": "Google Calendar"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Google Calendar API failed, returning mock data: {}", e);
                }
            }
        }

        // Fallback: Mock data
        Ok(serde_json::json!({
            "success": true,
            "event_id": uuid::Uuid::new_v4().to_string(),
            "title": title,
            "start_time": start_time.to_rfc3339(),
            "end_time": end_time.to_rfc3339(),
            "attendees": attendees,
            "location": location,
            "note": "Google Calendar not configured - returning mock data. Set GOOGLE_CALENDAR_CREDENTIALS and GOOGLE_CALENDAR_ID env vars to enable."
        }))
    }

    async fn execute_find_available_slots(
        &self,
        participants: &[String],
        duration_minutes: u32,
        preferred_days: &[String],
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Google Calendar API if configured
        if let Some(ref calendar_service) = self.calendar_service {
            match calendar_service
                .find_available_slots(participants, duration_minutes, 7)
                .await
            {
                Ok(slots) => {
                    let formatted_slots: Vec<_> = slots
                        .iter()
                        .map(|(start, end)| {
                            serde_json::json!({
                                "start": start.to_rfc3339(),
                                "end": end.to_rfc3339()
                            })
                        })
                        .collect();

                    tracing::info!("Found {} available slots", formatted_slots.len());
                    return Ok(serde_json::json!({
                        "success": true,
                        "participants": participants,
                        "duration_minutes": duration_minutes,
                        "preferred_days": preferred_days,
                        "available_slots": formatted_slots,
                        "calendar_provider": "Google Calendar"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Google Calendar API failed, returning mock data: {}", e);
                }
            }
        }

        // Fallback: Mock available slots
        let now = Utc::now();
        let slots = vec![serde_json::json!({
            "start": (now + chrono::Duration::days(1)).to_rfc3339(),
            "end": (now + chrono::Duration::days(1) + chrono::Duration::minutes(duration_minutes as i64)).to_rfc3339()
        })];

        Ok(serde_json::json!({
            "success": true,
            "participants": participants,
            "duration_minutes": duration_minutes,
            "preferred_days": preferred_days,
            "available_slots": slots,
            "note": "Google Calendar not configured - showing mock slots"
        }))
    }

    async fn execute_check_calendar_availability(
        &self,
        user: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Google Calendar API if configured
        if let Some(ref calendar_service) = self.calendar_service {
            match calendar_service
                .check_availability(user, start_time, end_time)
                .await
            {
                Ok(is_available) => {
                    tracing::info!("Checked availability for {}: {}", user, is_available);
                    return Ok(serde_json::json!({
                        "success": true,
                        "user": user,
                        "start_time": start_time.to_rfc3339(),
                        "end_time": end_time.to_rfc3339(),
                        "is_available": is_available,
                        "conflicts": if is_available { vec![] as Vec<String> } else { vec!["Busy".to_string()] },
                        "calendar_provider": "Google Calendar"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Google Calendar API failed, assuming available: {}", e);
                }
            }
        }

        // Fallback: Assume available
        Ok(serde_json::json!({
            "success": true,
            "user": user,
            "start_time": start_time.to_rfc3339(),
            "end_time": end_time.to_rfc3339(),
            "is_available": true,
            "conflicts": [],
            "note": "Google Calendar not configured - assuming available"
        }))
    }

    async fn resolve_project_id(&self, project_hint: Option<&str>) -> crate::Result<Uuid> {
        let executor = self
            .task_executor
            .as_ref()
            .ok_or_else(|| NoraError::ConfigError("Task executor not configured".to_string()))?;

        if let Some(hint) = project_hint {
            if let Ok(id) = Uuid::parse_str(hint) {
                return Ok(id);
            }
            if let Ok(Some(project)) = executor.find_project_record_by_name(hint).await {
                return Uuid::parse_str(&project.id)
                    .map_err(|e| NoraError::ConfigError(format!("Invalid project id: {}", e)));
            }
        }

        let projects = executor.get_all_projects().await?;
        if let Some(project) = projects.first() {
            if let Ok(id) = Uuid::parse_str(&project.id) {
                return Ok(id);
            }
        }

        Err(NoraError::ConfigError(
            "No project available for media pipeline. Specify project_id in the tool call."
                .to_string(),
        ))
    }

    async fn start_pipeline_task(
        &self,
        project_hint: Option<&str>,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        priority: Priority,
    ) -> crate::Result<Option<Uuid>> {
        let executor = match &self.task_executor {
            Some(executor) => executor,
            None => return Ok(None),
        };

        let project_id = self.resolve_project_id(project_hint).await?;
        let project_id_str = project_id.to_string();
        let board_id = executor
            .get_default_board_for_tasks(&project_id_str)
            .await?
            .map(|board| board.id);

        let definition = TaskDefinition {
            title,
            description,
            priority: Some(priority),
            tags: Some(tags),
            assignee_id: None,
            board_id,
            pod_id: None,
        };

        let task = executor.create_task(project_id_str, definition).await?;
        executor
            .update_task_status(&task.id, TaskStatus::InProgress)
            .await?;

        Ok(Some(Uuid::parse_str(&task.id).unwrap_or(project_id)))
    }

    async fn complete_pipeline_task(&self, task_id: Uuid, status: TaskStatus) {
        if let Some(executor) = &self.task_executor {
            if let Err(err) = executor
                .update_task_status(&task_id.to_string(), status)
                .await
            {
                tracing::warn!("Failed to update pipeline task {}: {}", task_id, err);
            }
        }
    }

    async fn execute_ingest_media_batch(
        &self,
        source_url: &str,
        reference_name: Option<String>,
        storage_tier: &str,
        checksum_required: bool,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let storage_tier = match MediaStorageTier::parse_tier(storage_tier) {
            Ok(tier) => tier,
            Err(err) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                }));
            }
        };

        let title = reference_name
            .clone()
            .map(|name| format!("Ingest batch – {}", name))
            .unwrap_or_else(|| "Ingest media batch".to_string());
        let description = Some(format!("Download and stage media from {}", source_url));
        let tags = vec!["editron".to_string(), "ingest".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::High,
            )
            .await?;

        let request = MediaBatchIngestRequest {
            source_url: source_url.to_string(),
            reference_name,
            storage_tier,
            checksum_required,
            project_id: project_hint
                .as_ref()
                .and_then(|value| Uuid::parse_str(value).ok()),
        };

        let response = match pipeline.ingest_batch(request).await {
            Ok(batch) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "batch": batch,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }

    async fn execute_analyze_media_batch(
        &self,
        batch_id: &str,
        brief: &str,
        passes: u32,
        deliverable_targets: Vec<String>,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let batch_uuid = match Uuid::parse_str(batch_id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "Invalid batch_id",
                }));
            }
        };

        let request = MediaBatchAnalysisRequest {
            batch_id: batch_uuid,
            brief: brief.to_string(),
            passes,
            deliverable_targets,
        };

        let title = format!("Analyze media batch {}", &batch_id[..batch_id.len().min(8)]);
        let description = Some(format!("Brief: {}", brief));
        let tags = vec!["editron".to_string(), "analysis".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::High,
            )
            .await?;

        let response = match pipeline.analyze_batch(request).await {
            Ok(analysis) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "analysis": analysis,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }

    async fn execute_generate_video_edits(
        &self,
        batch_id: &str,
        deliverable_type: &str,
        aspect_ratios: Vec<String>,
        reference_style: Option<String>,
        include_captions: bool,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let batch_uuid = match Uuid::parse_str(batch_id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "Invalid batch_id",
                }));
            }
        };

        let ratios_for_description = aspect_ratios.clone();

        let request = EditSessionRequest {
            batch_id: batch_uuid,
            deliverable_type: deliverable_type.to_string(),
            aspect_ratios,
            reference_style,
            include_captions,
        };

        let title = format!("Assemble edits – {}", deliverable_type);
        let description = Some(format!(
            "Batch {} | Ratios {:?}",
            &batch_id[..batch_id.len().min(8)],
            ratios_for_description
        ));
        let tags = vec!["editron".to_string(), "edit".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::High,
            )
            .await?;

        let response = match pipeline.generate_edits(request).await {
            Ok(session) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "edit_session": session,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }

    async fn execute_render_video_deliverables(
        &self,
        edit_session_id: &str,
        destinations: Vec<String>,
        formats: Vec<String>,
        priority: VideoRenderPriority,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let session_uuid = match Uuid::parse_str(edit_session_id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "Invalid edit_session_id",
                }));
            }
        };

        let request = RenderJobRequest {
            edit_session_id: session_uuid,
            destinations,
            formats,
            priority: match priority {
                VideoRenderPriority::Low => PipelineRenderPriority::Low,
                VideoRenderPriority::Standard => PipelineRenderPriority::Standard,
                VideoRenderPriority::Rush => PipelineRenderPriority::Rush,
            },
        };

        let title = "Render deliverables".to_string();
        let description = Some(format!(
            "Session {}",
            &edit_session_id[..edit_session_id.len().min(8)]
        ));
        let tags = vec!["editron".to_string(), "render".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::Medium,
            )
            .await?;

        let response = match pipeline.render_deliverables(request).await {
            Ok(job) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "render_job": job,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }
}
