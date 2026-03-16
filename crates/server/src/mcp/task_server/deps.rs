use db::models::{
    task::Task,
    task_dependency::{CreateTaskDependency, DependencyType, TaskDependency},
};
use rmcp::{
    ErrorData,
    handler::server::tool::Parameters,
    model::CallToolResult,
    tool,
};
use serde_json::Value;
use uuid::Uuid;

use super::TaskServer;
use super::helpers::*;
use super::types::*;

impl TaskServer {
    #[tool(
        description = "Manage task dependencies: add, remove, or list. Use action 'add' with source_task_id + target_task_id, 'remove' with source+target, or 'list' with task_id."
    )]
    pub(super) async fn manage_task_dependencies(
        &self,
        Parameters(req): Parameters<ManageDependenciesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match req.action.as_str() {
            "add" => {
                let source = match req.source_task_id.as_deref().map(|s| parse_uuid(s, "source_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("source_task_id required for 'add'", None)),
                };
                let target = match req.target_task_id.as_deref().map(|s| parse_uuid(s, "target_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("target_task_id required for 'add'", None)),
                };

                let dep_type = match req.dependency_type.as_deref().unwrap_or("blocks") {
                    "blocks" => DependencyType::Blocks,
                    "relates_to" => DependencyType::RelatesTo,
                    _ => return Ok(error_result("dependency_type must be 'blocks' or 'relates_to'", None)),
                };

                let payload = CreateTaskDependency {
                    project_id: project_uuid,
                    source_task_id: source,
                    target_task_id: target,
                    dependency_type: dep_type,
                };

                match TaskDependency::create(&self.pool, &payload).await {
                    Ok(dep) => Ok(success_json(&serde_json::json!({
                        "success": true,
                        "message": "Dependency created",
                        "dependency_id": dep.id.to_string(),
                        "source_task_id": dep.source_task_id.to_string(),
                        "target_task_id": dep.target_task_id.to_string(),
                        "dependency_type": format!("{:?}", dep.dependency_type).to_lowercase(),
                    }))),
                    Err(e) => Ok(error_result("Failed to create dependency", Some(&e.to_string()))),
                }
            }
            "remove" => {
                let source = match req.source_task_id.as_deref().map(|s| parse_uuid(s, "source_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("source_task_id required for 'remove'", None)),
                };
                let target = match req.target_task_id.as_deref().map(|s| parse_uuid(s, "target_task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => return Ok(error_result("target_task_id required for 'remove'", None)),
                };

                // Find matching dependency and delete
                let deps = match TaskDependency::list_by_task(&self.pool, source).await {
                    Ok(d) => d,
                    Err(e) => return Ok(error_result("Failed to list dependencies", Some(&e.to_string()))),
                };

                let matching = deps.iter().find(|d| {
                    (d.source_task_id == source && d.target_task_id == target)
                        || (d.source_task_id == target && d.target_task_id == source)
                });

                match matching {
                    Some(dep) => match TaskDependency::delete(&self.pool, dep.id).await {
                        Ok(_) => Ok(success_json(&serde_json::json!({
                            "success": true,
                            "message": "Dependency removed",
                        }))),
                        Err(e) => Ok(error_result("Failed to delete dependency", Some(&e.to_string()))),
                    },
                    None => Ok(error_result("No matching dependency found", None)),
                }
            }
            "list" => {
                let task_uuid = match req.task_id.as_deref().map(|s| parse_uuid(s, "task_id")) {
                    Some(Ok(u)) => u,
                    Some(Err(r)) => return Ok(r),
                    None => {
                        // Fall back to listing all project dependencies
                        match TaskDependency::list_by_project(&self.pool, project_uuid).await {
                            Ok(deps) => {
                                let items: Vec<Value> = deps
                                    .iter()
                                    .map(|d| serde_json::json!({
                                        "id": d.id.to_string(),
                                        "source_task_id": d.source_task_id.to_string(),
                                        "target_task_id": d.target_task_id.to_string(),
                                        "dependency_type": format!("{:?}", d.dependency_type).to_lowercase(),
                                        "created_at": d.created_at.to_rfc3339(),
                                    }))
                                    .collect();
                                return Ok(success_json(&serde_json::json!({
                                    "success": true,
                                    "project_id": req.project_id,
                                    "count": items.len(),
                                    "dependencies": items,
                                })));
                            }
                            Err(e) => return Ok(error_result("Failed to list dependencies", Some(&e.to_string()))),
                        }
                    }
                };

                match TaskDependency::list_by_task(&self.pool, task_uuid).await {
                    Ok(deps) => {
                        let items: Vec<Value> = deps
                            .iter()
                            .map(|d| serde_json::json!({
                                "id": d.id.to_string(),
                                "source_task_id": d.source_task_id.to_string(),
                                "target_task_id": d.target_task_id.to_string(),
                                "dependency_type": format!("{:?}", d.dependency_type).to_lowercase(),
                                "created_at": d.created_at.to_rfc3339(),
                            }))
                            .collect();
                        Ok(success_json(&serde_json::json!({
                            "success": true,
                            "task_id": req.task_id,
                            "count": items.len(),
                            "dependencies": items,
                        })))
                    }
                    Err(e) => Ok(error_result("Failed to list dependencies", Some(&e.to_string()))),
                }
            }
            _ => Ok(error_result("Invalid action. Use 'add', 'remove', or 'list'", None)),
        }
    }

    #[tool(
        description = "Check whether all blocking dependencies for a task are resolved (status = 'done'). Returns a structured report: all_resolved (bool), total blockers, resolved count, and details per blocker. Use this before starting work on a task to verify blockers are cleared."
    )]
    pub(super) async fn check_dependencies(
        &self,
        Parameters(req): Parameters<CheckDependenciesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let task_uuid = match parse_uuid(&req.task_id, "task_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        // Verify task exists
        match Task::find_by_id_and_project_id(&self.pool, &task_uuid.to_string(), &project_uuid.to_string()).await {
            Ok(Some(_)) => {}
            Ok(None) => return Ok(error_result("Task not found in the specified project", None)),
            Err(e) => return Ok(error_result("Failed to retrieve task", Some(&e.to_string()))),
        }

        // Get all dependencies where this task is the target (i.e., things that block this task)
        let deps = match TaskDependency::list_by_task(&self.pool, task_uuid).await {
            Ok(d) => d,
            Err(e) => return Ok(error_result("Failed to list dependencies", Some(&e.to_string()))),
        };

        // Filter to only "blocks" dependencies where this task is the target
        let blockers: Vec<&TaskDependency> = deps
            .iter()
            .filter(|d| d.dependency_type == DependencyType::Blocks && d.target_task_id == task_uuid)
            .collect();

        if blockers.is_empty() {
            return Ok(success_json(&serde_json::json!({
                "task_id": req.task_id,
                "all_resolved": true,
                "total_blockers": 0,
                "resolved": 0,
                "unresolved": 0,
                "blockers": [],
                "message": "No blocking dependencies — task is ready to start"
            })));
        }

        let mut blocker_details = Vec::new();
        let mut resolved_count = 0;

        for dep in &blockers {
            let source_task = Task::find_by_id(&self.pool, &dep.source_task_id.to_string()).await;
            match source_task {
                Ok(Some(t)) => {
                    let status_str = serde_json::to_value(&t.status)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_else(|| "unknown".to_string());
                    let is_done = status_str == "done";
                    if is_done {
                        resolved_count += 1;
                    }
                    blocker_details.push(serde_json::json!({
                        "task_id": dep.source_task_id.to_string(),
                        "title": t.title,
                        "status": status_str,
                        "resolved": is_done,
                    }));
                }
                Ok(None) => {
                    blocker_details.push(serde_json::json!({
                        "task_id": dep.source_task_id.to_string(),
                        "title": null,
                        "status": "not_found",
                        "resolved": false,
                    }));
                }
                Err(_) => {
                    blocker_details.push(serde_json::json!({
                        "task_id": dep.source_task_id.to_string(),
                        "title": null,
                        "status": "error",
                        "resolved": false,
                    }));
                }
            }
        }

        let all_resolved = resolved_count == blockers.len();
        let message = if all_resolved {
            "All blocking dependencies resolved — task is ready to start".to_string()
        } else {
            format!(
                "{} of {} blockers unresolved — task is blocked",
                blockers.len() - resolved_count,
                blockers.len()
            )
        };

        Ok(success_json(&serde_json::json!({
            "task_id": req.task_id,
            "all_resolved": all_resolved,
            "total_blockers": blockers.len(),
            "resolved": resolved_count,
            "unresolved": blockers.len() - resolved_count,
            "blockers": blocker_details,
            "message": message,
        })))
    }
}
