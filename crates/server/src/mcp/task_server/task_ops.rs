use db::models::{
    project::Project,
    task::{CreateTask, Priority, Task, TaskWithAttemptStatus},
};
use rmcp::{
    ErrorData,
    handler::server::tool::Parameters,
    model::{CallToolResult, Content},
    tool,
};
use serde_json::Value;
use sqlx::types::Json as SqlxJson;
use uuid::Uuid;

use super::TaskServer;
use super::helpers::*;
use super::types::*;

impl TaskServer {
    #[tool(
        description = "Create a new task/ticket in a project with full field support. Always pass the `project_id` — it is required!"
    )]
    pub(super) async fn create_task(
        &self,
        Parameters(req): Parameters<CreateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Project::exists(&self.pool, &project_uuid.to_string()).await {
            Ok(false) => return Ok(error_result("Project not found", None)),
            Err(e) => return Ok(error_result("Failed to check project", Some(&e.to_string()))),
            Ok(true) => {}
        }

        let priority = req.priority.as_deref().and_then(parse_priority);
        let due_date = req.due_date.as_deref().and_then(parse_iso_datetime);
        let scheduled_start = req.scheduled_start.as_deref().and_then(parse_iso_datetime);
        let scheduled_end = req.scheduled_end.as_deref().and_then(parse_iso_datetime);
        let parent_task_id = req
            .parent_task_id
            .clone();
        let board_id = req
            .board_id
            .clone();

        let task_id = Uuid::new_v4();
        let task_id_str = task_id.to_string();
        let create_task_data = CreateTask {
            project_id: project_uuid.to_string(),
            pod_id: None,
            board_id,
            title: req.title.clone(),
            description: req.description.clone(),
            parent_task_attempt: None,
            image_ids: None,
            priority,
            assignee_id: req.assignee_id.clone(),
            assignee_type: None,
            assigned_agent: req.assigned_agent.clone(),
            agent_id: None,
            assigned_mcps: None,
            created_by: self
                .user_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "mcp".to_string()),
            requires_approval: req.requires_approval,
            parent_task_id,
            tags: req.tags.clone(),
            due_date,
            custom_properties: req.custom_properties.clone(),
            scheduled_start,
            scheduled_end,
            screenshot: None,
            completion_criteria: req.completion_criteria.clone(),
            output_format: req.output_format.clone(),
            collaborators: None,
        };

        match Task::create(&self.pool, &create_task_data, &task_id_str).await {
            Ok(_task) => Ok(success_json(&CreateTaskResponse {
                success: true,
                task_id: task_id_str,
                message: "Task created successfully".to_string(),
            })),
            Err(e) => Ok(error_result("Failed to create task", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "List tasks in a project with filtering, search, sorting, and pagination. `project_id` is required!"
    )]
    pub(super) async fn list_tasks(
        &self,
        Parameters(req): Parameters<ListTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let status_filter = if let Some(ref status_str) = req.status {
            match parse_task_status(status_str) {
                Some(s) => Some(s),
                None => return Ok(error_result(
                    "Invalid status filter. Valid: todo, inprogress, inreview, done, cancelled",
                    None,
                )),
            }
        } else {
            None
        };

        let priority_filter = req.priority.as_deref().and_then(parse_priority);

        let project = match Project::find_by_id(&self.pool, &project_uuid.to_string()).await {
            Ok(Some(p)) => p,
            Ok(None) => return Ok(error_result("Project not found", None)),
            Err(e) => return Ok(error_result("Failed to check project", Some(&e.to_string()))),
        };

        let task_limit = req.limit.unwrap_or(50).clamp(1, 200);
        let page = req.page.unwrap_or(1).max(1);
        let sort_by = req.sort_by.clone().unwrap_or_else(|| "created_at".to_string());
        let sort_dir = req.sort_direction.clone().unwrap_or_else(|| "desc".to_string());

        let tasks_result =
            Task::find_by_project_id_with_attempt_status(&self.pool, &project_uuid.to_string()).await;

        match tasks_result {
            Ok(tasks) => {
                // Apply filters
                let mut filtered: Vec<&TaskWithAttemptStatus> = tasks
                    .iter()
                    .filter(|t| {
                        if let Some(ref fs) = status_filter {
                            if &t.status != fs {
                                return false;
                            }
                        }
                        if let Some(ref fp) = priority_filter {
                            if &t.priority != fp {
                                return false;
                            }
                        }
                        if let Some(ref fa) = req.assignee_id {
                            if t.assignee_id.as_deref() != Some(fa.as_str()) {
                                return false;
                            }
                        }
                        if let Some(ref fag) = req.assigned_agent {
                            if t.assigned_agent.as_deref() != Some(fag.as_str()) {
                                return false;
                            }
                        }
                        if let Some(ref ft) = req.tag {
                            let has_tag = t
                                .tags
                                .as_deref()
                                .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
                                .map(|tags| tags.iter().any(|tag| tag.eq_ignore_ascii_case(ft)))
                                .unwrap_or(false);
                            if !has_tag {
                                return false;
                            }
                        }
                        if let Some(ref search) = req.search {
                            let kw = search.to_lowercase();
                            let in_title = t.title.to_lowercase().contains(&kw);
                            let in_desc = t
                                .description
                                .as_deref()
                                .map(|d| d.to_lowercase().contains(&kw))
                                .unwrap_or(false);
                            if !in_title && !in_desc {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                // Sort
                filtered.sort_by(|a, b| {
                    let cmp = match sort_by.as_str() {
                        "updated_at" => a.updated_at.cmp(&b.updated_at),
                        "priority" => {
                            let p = |pr: &Priority| match pr {
                                Priority::Critical => 0,
                                Priority::High => 1,
                                Priority::Medium => 2,
                                Priority::Low => 3,
                            };
                            p(&a.priority).cmp(&p(&b.priority))
                        }
                        "due_date" => a.due_date.cmp(&b.due_date),
                        "title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                        _ => a.created_at.cmp(&b.created_at), // default: created_at
                    };
                    if sort_dir == "asc" {
                        cmp
                    } else {
                        cmp.reverse()
                    }
                });

                let total_count = filtered.len();
                let total_pages = ((total_count as f64) / (task_limit as f64)).ceil() as i32;
                let skip = ((page - 1) * task_limit) as usize;

                let page_tasks: Vec<TaskSummary> = filtered
                    .into_iter()
                    .skip(skip)
                    .take(task_limit as usize)
                    .map(task_with_status_to_summary)
                    .collect();

                let count = page_tasks.len();

                Ok(success_json(&ListTasksResponse {
                    success: true,
                    tasks: page_tasks,
                    count,
                    total_count,
                    page,
                    total_pages: total_pages.max(1),
                    project_id: req.project_id.clone(),
                    project_name: Some(project.name),
                    applied_filters: ListTasksFilters {
                        status: req.status.clone(),
                        priority: req.priority.clone(),
                        assignee_id: req.assignee_id.clone(),
                        assigned_agent: req.assigned_agent.clone(),
                        tag: req.tag.clone(),
                        search: req.search.clone(),
                        sort_by,
                        sort_direction: sort_dir,
                        limit: task_limit,
                        page,
                    },
                }))
            }
            Err(e) => Ok(error_result("Failed to retrieve tasks", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Update an existing task with full field support. `project_id` and `task_id` are required!"
    )]
    pub(super) async fn update_task(
        &self,
        Parameters(req): Parameters<UpdateTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let task_uuid = match parse_uuid(&req.task_id, "task_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let status_enum = if let Some(ref s) = req.status {
            match parse_task_status(s) {
                Some(st) => Some(st),
                None => return Ok(error_result(
                    "Invalid status. Valid: todo, inprogress, inreview, done, cancelled",
                    None,
                )),
            }
        } else {
            None
        };

        let current_task =
            match Task::find_by_id_and_project_id(&self.pool, &task_uuid.to_string(), &project_uuid.to_string()).await {
                Ok(Some(t)) => t,
                Ok(None) => return Ok(error_result("Task not found in the specified project", None)),
                Err(e) => return Ok(error_result("Failed to retrieve task", Some(&e.to_string()))),
            };

        let new_title = req.title.unwrap_or_else(|| current_task.title.clone());
        let new_description = req.description.or_else(|| current_task.description.clone());
        let new_status = status_enum.unwrap_or_else(|| current_task.status.clone());

        let new_priority = req
            .priority
            .as_deref()
            .and_then(parse_priority)
            .unwrap_or_else(|| current_task.priority.clone());

        let new_assignee_id = match &req.assignee_id {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(s.clone()),
            None => current_task.assignee_id.clone(),
        };

        let new_assigned_agent = match &req.assigned_agent {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(s.clone()),
            None => current_task.assigned_agent.clone(),
        };

        let new_tags = match &req.tags {
            Some(t) => serde_json::to_string(t).ok().or_else(|| current_task.tags.clone()),
            None => current_task.tags.clone(),
        };

        let new_due_date = match &req.due_date {
            Some(s) if s.is_empty() => None,
            Some(s) => parse_iso_datetime(s).or(current_task.due_date),
            None => current_task.due_date,
        };

        let new_scheduled_start = match &req.scheduled_start {
            Some(s) if s.is_empty() => None,
            Some(s) => parse_iso_datetime(s).or(current_task.scheduled_start),
            None => current_task.scheduled_start,
        };

        let new_scheduled_end = match &req.scheduled_end {
            Some(s) if s.is_empty() => None,
            Some(s) => parse_iso_datetime(s).or(current_task.scheduled_end),
            None => current_task.scheduled_end,
        };

        let new_parent_task_id = match &req.parent_task_id {
            Some(s) if s.is_empty() => None,
            Some(s) => Uuid::parse_str(s).ok().map(|u| u.to_string()).or(current_task.parent_task_id.clone()),
            None => current_task.parent_task_id.clone(),
        };

        let new_board_id = match &req.board_id {
            Some(s) if s.is_empty() => None,
            Some(s) => Uuid::parse_str(s).ok().map(|u| u.to_string()).or(current_task.board_id.clone()),
            None => current_task.board_id.clone(),
        };

        let new_requires_approval = req.requires_approval.unwrap_or(current_task.requires_approval);

        let new_completion_criteria = req.completion_criteria.or_else(|| current_task.completion_criteria.clone());
        let new_output_format = req.output_format.or_else(|| current_task.output_format.clone());

        let new_custom_properties = match &req.custom_properties {
            Some(Value::Null) => None,
            Some(v) => Some(SqlxJson(v.clone())),
            None => current_task
                .custom_properties
                .as_ref()
                .map(|j| SqlxJson(j.0.clone())),
        };

        match Task::update(
            &self.pool,
            &task_uuid.to_string(),
            &project_uuid.to_string(),
            new_title,
            new_description,
            new_status,
            current_task.parent_task_attempt.clone(),
            current_task.pod_id.clone(),
            new_board_id,
            new_priority,
            new_assignee_id,
            current_task.assignee_type.clone(),
            new_assigned_agent,
            current_task.assigned_mcps.clone(),
            new_requires_approval,
            current_task.approval_status.clone(),
            new_parent_task_id,
            new_tags,
            new_due_date,
            new_custom_properties,
            new_scheduled_start,
            new_scheduled_end,
            new_completion_criteria,
            new_output_format,
        )
        .await
        {
            Ok(updated) => Ok(success_json(&UpdateTaskResponse {
                success: true,
                message: "Task updated successfully".to_string(),
                task: Some(task_to_summary(&updated)),
            })),
            Err(e) => Ok(error_result("Failed to update task", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Delete a task/ticket from a project. `project_id` and `task_id` are required!"
    )]
    pub(super) async fn delete_task(
        &self,
        Parameters(DeleteTaskRequest {
            project_id,
            task_id,
        }): Parameters<DeleteTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let task_uuid = match parse_uuid(&task_id, "task_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Task::exists(&self.pool, &task_uuid.to_string(), &project_uuid.to_string()).await {
            Ok(true) => match Task::delete(&self.pool, &task_uuid.to_string()).await {
                Ok(rows) if rows > 0 => Ok(success_json(&DeleteTaskResponse {
                    success: true,
                    message: "Task deleted successfully".to_string(),
                    deleted_task_id: Some(task_id),
                })),
                Ok(_) => Ok(error_result("Task not found or already deleted", None)),
                Err(e) => Ok(error_result("Failed to delete task", Some(&e.to_string()))),
            },
            Ok(false) => Ok(error_result("Task not found in the specified project", None)),
            Err(e) => Ok(error_result("Failed to check task existence", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Get detailed information about a specific task. `project_id` and `task_id` are required!"
    )]
    pub(super) async fn get_task(
        &self,
        Parameters(GetTaskRequest {
            project_id,
            task_id,
        }): Parameters<GetTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let task_uuid = match parse_uuid(&task_id, "task_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let task_result =
            Task::find_by_id_and_project_id(&self.pool, &task_uuid.to_string(), &project_uuid.to_string()).await;
        let project_result = Project::find_by_id(&self.pool, &project_uuid.to_string()).await;

        match (task_result, project_result) {
            (Ok(Some(task)), Ok(Some(project))) => Ok(success_json(&GetTaskResponse {
                success: true,
                task: Some(task_to_summary(&task)),
                project_name: Some(project.name),
            })),
            (Ok(None), _) | (_, Ok(None)) => Ok(error_result("Task or project not found", None)),
            (Err(e), _) | (_, Err(e)) => Ok(error_result(
                "Failed to retrieve task or project",
                Some(&e.to_string()),
            )),
        }
    }

    #[tool(
        description = "Assign a task to a team member. Pass 'unassign' as the assignee to clear assignment. `project_id` and `task_id` are required!"
    )]
    pub(super) async fn assign_task(
        &self,
        Parameters(AssignTaskRequest {
            project_id,
            task_id,
            assignee,
        }): Parameters<AssignTaskRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match Uuid::parse_str(&project_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    r#"{"success": false, "error": "Invalid project ID format"}"#,
                )]));
            }
        };
        let task_uuid = match Uuid::parse_str(&task_id) {
            Ok(uuid) => uuid,
            Err(_) => {
                return Ok(CallToolResult::error(vec![Content::text(
                    r#"{"success": false, "error": "Invalid task ID format"}"#,
                )]));
            }
        };

        // Find the task
        let current_task =
            match Task::find_by_id_and_project_id(&self.pool, &task_uuid.to_string(), &project_uuid.to_string()).await {
                Ok(Some(task)) => task,
                Ok(None) => {
                    return Ok(CallToolResult::error(vec![Content::text(
                        r#"{"success": false, "error": "Task not found in project"}"#,
                    )]));
                }
                Err(e) => {
                    let msg = format!(r#"{{"success": false, "error": "{}"}}"#, e);
                    return Ok(CallToolResult::error(vec![Content::text(msg)]));
                }
            };

        // Resolve assignee — either "unassign" or a username/UUID
        let assignee_id: Option<String> = if assignee.to_lowercase() == "unassign" {
            None
        } else {
            // Try as UUID first, then look up by username
            match Uuid::parse_str(&assignee) {
                Ok(uuid) => Some(uuid.to_string()),
                Err(_) => {
                    // Look up by username
                    #[derive(sqlx::FromRow)]
                    struct UserId { id: Vec<u8> }
                    match sqlx::query_as::<_, UserId>(
                        "SELECT id FROM users WHERE username = ? COLLATE NOCASE AND is_active = 1",
                    )
                    .bind(&assignee)
                    .fetch_optional(&self.pool)
                    .await
                    {
                        Ok(Some(row)) => Uuid::from_slice(&row.id).ok().map(|u| u.to_string()),
                        _ => {
                            let msg = format!(
                                r#"{{"success": false, "error": "User '{}' not found"}}"#,
                                assignee
                            );
                            return Ok(CallToolResult::error(vec![Content::text(msg)]));
                        }
                    }
                }
            }
        };

        let custom_properties = current_task
            .custom_properties
            .as_ref()
            .map(|json| json.0.clone())
            .map(SqlxJson);

        let task_title = current_task.title.clone();
        match Task::update(
            &self.pool,
            &task_uuid.to_string(),
            &project_uuid.to_string(),
            current_task.title,
            current_task.description,
            current_task.status,
            current_task.parent_task_attempt,
            current_task.pod_id,
            current_task.board_id,
            current_task.priority,
            assignee_id.clone(),
            current_task.assignee_type,
            current_task.assigned_agent,
            current_task.assigned_mcps,
            current_task.requires_approval,
            current_task.approval_status,
            current_task.parent_task_id,
            current_task.tags,
            current_task.due_date,
            custom_properties,
            current_task.scheduled_start,
            current_task.scheduled_end,
            current_task.completion_criteria,
            current_task.output_format,
        )
        .await
        {
            Ok(_) => {
                let msg = if assignee_id.is_some() {
                    format!("Task '{}' assigned to '{}'", task_title, assignee)
                } else {
                    format!("Task '{}' unassigned", task_title)
                };
                let response = serde_json::json!({ "success": true, "message": msg });
                Ok(CallToolResult::success(vec![Content::text(
                    to_json_pretty(&response),
                )]))
            }
            Err(e) => {
                let msg = format!(r#"{{"success": false, "error": "{}"}}"#, e);
                Ok(CallToolResult::error(vec![Content::text(msg)]))
            }
        }
    }

    #[tool(
        description = "Create multiple tasks at once (max 50). Returns per-item success/failure. `project_id` is required!"
    )]
    pub(super) async fn bulk_create_tasks(
        &self,
        Parameters(req): Parameters<BulkCreateTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match Project::exists(&self.pool, &project_uuid.to_string()).await {
            Ok(false) => return Ok(error_result("Project not found", None)),
            Err(e) => return Ok(error_result("Failed to check project", Some(&e.to_string()))),
            Ok(true) => {}
        }

        if req.tasks.is_empty() {
            return Ok(error_result("No tasks provided", None));
        }
        if req.tasks.len() > 50 {
            return Ok(error_result("Maximum 50 tasks per bulk create", None));
        }

        let created_by = self
            .user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "mcp".to_string());

        let mut results: Vec<Value> = Vec::new();
        let mut success_count = 0;

        for (i, item) in req.tasks.iter().enumerate() {
            let task_id = Uuid::new_v4().to_string();
            let priority = item.priority.as_deref().and_then(parse_priority);
            let due_date = item.due_date.as_deref().and_then(parse_iso_datetime);
            let parent_task_id = item
                .parent_task_id
                .clone();

            let create_data = CreateTask {
                project_id: project_uuid.to_string(),
                pod_id: None,
                board_id: None,
                title: item.title.clone(),
                description: item.description.clone(),
                parent_task_attempt: None,
                image_ids: None,
                priority,
                assignee_id: item.assignee_id.clone(),
                assignee_type: None,
                assigned_agent: None,
                agent_id: None,
                assigned_mcps: None,
                created_by: created_by.clone(),
                requires_approval: None,
                parent_task_id,
                tags: item.tags.clone(),
                due_date,
                custom_properties: None,
                scheduled_start: None,
                scheduled_end: None,
                screenshot: None,
                completion_criteria: None,
                output_format: None,
                collaborators: None,
            };

            match Task::create(&self.pool, &create_data, &task_id).await {
                Ok(_) => {
                    success_count += 1;
                    results.push(serde_json::json!({
                        "index": i,
                        "success": true,
                        "task_id": task_id.to_string(),
                        "title": item.title,
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "index": i,
                        "success": false,
                        "title": item.title,
                        "error": e.to_string(),
                    }));
                }
            }
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "total_requested": req.tasks.len(),
            "total_created": success_count,
            "total_failed": req.tasks.len() - success_count,
            "results": results,
        })))
    }

    #[tool(
        description = "Update multiple tasks at once (max 50). Returns per-item results. `project_id` is required!"
    )]
    pub(super) async fn bulk_update_tasks(
        &self,
        Parameters(req): Parameters<BulkUpdateTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        if req.updates.is_empty() {
            return Ok(error_result("No updates provided", None));
        }
        if req.updates.len() > 50 {
            return Ok(error_result("Maximum 50 updates per bulk update", None));
        }

        let mut results: Vec<Value> = Vec::new();
        let mut success_count = 0;

        for (i, item) in req.updates.iter().enumerate() {
            let task_uuid = match Uuid::parse_str(&item.task_id) {
                Ok(u) => u,
                Err(_) => {
                    results.push(serde_json::json!({
                        "index": i, "success": false, "task_id": item.task_id,
                        "error": "Invalid task_id UUID"
                    }));
                    continue;
                }
            };

            let current =
                match Task::find_by_id_and_project_id(&self.pool, &task_uuid.to_string(), &project_uuid.to_string()).await {
                    Ok(Some(t)) => t,
                    Ok(None) => {
                        results.push(serde_json::json!({
                            "index": i, "success": false, "task_id": item.task_id,
                            "error": "Task not found"
                        }));
                        continue;
                    }
                    Err(e) => {
                        results.push(serde_json::json!({
                            "index": i, "success": false, "task_id": item.task_id,
                            "error": e.to_string()
                        }));
                        continue;
                    }
                };

            let new_status = item
                .status
                .as_deref()
                .and_then(parse_task_status)
                .unwrap_or_else(|| current.status.clone());
            let new_priority = item
                .priority
                .as_deref()
                .and_then(parse_priority)
                .unwrap_or_else(|| current.priority.clone());
            let new_assignee = match &item.assignee_id {
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s.clone()),
                None => current.assignee_id.clone(),
            };
            let new_agent = match &item.assigned_agent {
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s.clone()),
                None => current.assigned_agent.clone(),
            };
            let new_tags = match &item.tags {
                Some(t) => serde_json::to_string(t).ok().or_else(|| current.tags.clone()),
                None => current.tags.clone(),
            };

            let cp = current
                .custom_properties
                .as_ref()
                .map(|j| SqlxJson(j.0.clone()));

            match Task::update(
                &self.pool,
                &task_uuid.to_string(),
                &project_uuid.to_string(),
                current.title.clone(),
                current.description.clone(),
                new_status,
                current.parent_task_attempt.clone(),
                current.pod_id.clone(),
                current.board_id.clone(),
                new_priority,
                new_assignee,
                current.assignee_type,
                new_agent,
                current.assigned_mcps,
                current.requires_approval,
                current.approval_status,
                current.parent_task_id,
                new_tags,
                current.due_date,
                cp,
                current.scheduled_start,
                current.scheduled_end,
                current.completion_criteria.clone(),
                current.output_format.clone(),
            )
            .await
            {
                Ok(_) => {
                    success_count += 1;
                    results.push(serde_json::json!({
                        "index": i, "success": true, "task_id": item.task_id,
                    }));
                }
                Err(e) => {
                    results.push(serde_json::json!({
                        "index": i, "success": false, "task_id": item.task_id,
                        "error": e.to_string()
                    }));
                }
            }
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "total_requested": req.updates.len(),
            "total_updated": success_count,
            "total_failed": req.updates.len() - success_count,
            "results": results,
        })))
    }

    #[tool(
        description = "Search tasks by keyword across title and description. Optionally scope to a project."
    )]
    pub(super) async fn search_tasks(
        &self,
        Parameters(req): Parameters<SearchTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = req.limit.unwrap_or(20).clamp(1, 100);
        let status_filter = req.status.as_deref().and_then(parse_task_status);
        let priority_filter = req.priority.as_deref().and_then(parse_priority);

        // Determine which projects to search
        let project_ids: Vec<String> = if let Some(ref pid) = req.project_id {
            match parse_uuid(pid, "project_id") {
                Ok(u) => vec![u.to_string()],
                Err(r) => return Ok(r),
            }
        } else {
            // Search accessible projects
            match self.accessible_project_ids().await {
                Ok(None) => {
                    // Admin: get all projects
                    match Project::find_all(&self.pool).await {
                        Ok(ps) => ps.into_iter().map(|p| p.id).collect(),
                        Err(e) => return Ok(error_result("Failed to list projects", Some(&e.to_string()))),
                    }
                }
                Ok(Some(ids)) => ids.into_iter().map(|id| id.to_string()).collect(),
                Err(e) => return Ok(error_result("Failed to determine accessible projects", Some(&e.to_string()))),
            }
        };

        let mut all_tasks: Vec<TaskSummary> = Vec::new();

        for pid in &project_ids {
            let tasks = match Task::find_by_project_id_with_attempt_status(&self.pool, pid).await {
                Ok(t) => t,
                Err(_) => continue,
            };

            for t in &tasks {
                let kw_lower = req.query.to_lowercase();
                let in_title = t.title.to_lowercase().contains(&kw_lower);
                let in_desc = t
                    .description
                    .as_deref()
                    .map(|d| d.to_lowercase().contains(&kw_lower))
                    .unwrap_or(false);
                let in_tags = t
                    .tags
                    .as_deref()
                    .map(|s| s.to_lowercase().contains(&kw_lower))
                    .unwrap_or(false);

                if !in_title && !in_desc && !in_tags {
                    continue;
                }
                if let Some(ref sf) = status_filter {
                    if &t.status != sf {
                        continue;
                    }
                }
                if let Some(ref pf) = priority_filter {
                    if &t.priority != pf {
                        continue;
                    }
                }

                all_tasks.push(task_with_status_to_summary(t));
                if all_tasks.len() >= limit as usize {
                    break;
                }
            }
            if all_tasks.len() >= limit as usize {
                break;
            }
        }

        Ok(success_json(&serde_json::json!({
            "success": true,
            "query": req.query,
            "count": all_tasks.len(),
            "tasks": all_tasks,
        })))
    }
}
