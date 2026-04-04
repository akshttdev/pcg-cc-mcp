//! User-scoped tool execution with project membership validation

use uuid::Uuid;

use super::ExecutiveTools;

#[allow(dead_code)]
impl ExecutiveTools {
    /// Execute a tool call for user-scoped agents
    /// Validates project membership before executing project-scoped operations
    pub async fn execute_user_scoped_tool(
        name: &str,
        arguments: &serde_json::Value,
        pool: &sqlx::SqlitePool,
        user_id: Uuid,
    ) -> serde_json::Value {
        match name {
            "create_project" => {
                let project_name = match arguments.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return serde_json::json!({"success": false, "error": "Missing project name"})
                    }
                };
                let project_id = Uuid::new_v4();
                let slug = project_name.to_lowercase().replace(' ', "-");
                let result = sqlx::query(
                    r#"INSERT INTO projects (id, name, slug, git_repo_path, created_at, updated_at)
                       VALUES (?, ?, ?, ?, datetime('now', 'subsec'), datetime('now', 'subsec'))"#,
                )
                .bind(project_id.to_string())
                .bind(project_name)
                .bind(&slug)
                .bind(format!("/projects/{}", slug))
                .execute(pool)
                .await;

                match result {
                    Ok(_) => {
                        // Add user as owner
                        let member_id = Uuid::new_v4();
                        let _ = sqlx::query(
                            r#"INSERT INTO project_members (id, project_id, user_id, role, created_at)
                               VALUES (?, ?, ?, 'owner', datetime('now', 'subsec'))"#,
                        )
                        .bind(member_id.to_string())
                        .bind(project_id.to_string())
                        .bind(user_id.to_string())
                        .execute(pool)
                        .await;

                        serde_json::json!({
                            "success": true,
                            "message": format!("Project '{}' created successfully", project_name),
                            "project_id": project_id.to_string()
                        })
                    }
                    Err(e) => {
                        serde_json::json!({"success": false, "error": format!("Failed to create project: {}", e)})
                    }
                }
            }
            "list_my_projects" => {
                #[derive(sqlx::FromRow, serde::Serialize)]
                struct ProjectRow {
                    #[sqlx(try_from = "Vec<u8>")]
                    id: Uuid,
                    name: String,
                    slug: Option<String>,
                }
                let projects = sqlx::query_as::<_, ProjectRow>(
                    r#"SELECT p.id, p.name, p.slug
                       FROM projects p
                       JOIN project_members pm ON p.id = pm.project_id
                       WHERE pm.user_id = ?
                       ORDER BY p.name ASC"#,
                )
                .bind(user_id.to_string())
                .fetch_all(pool)
                .await;

                match projects {
                    Ok(rows) => {
                        let project_list: Vec<serde_json::Value> = rows
                            .iter()
                            .map(|p| {
                                serde_json::json!({
                                    "id": p.id.to_string(),
                                    "name": p.name,
                                    "slug": p.slug
                                })
                            })
                            .collect();
                        serde_json::json!({"success": true, "projects": project_list, "count": project_list.len()})
                    }
                    Err(e) => {
                        serde_json::json!({"success": false, "error": format!("Failed to list projects: {}", e)})
                    }
                }
            }
            "create_task" => {
                let project_name = match arguments.get("project_name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return serde_json::json!({"success": false, "error": "Missing project_name"})
                    }
                };
                let title = match arguments.get("title").and_then(|v| v.as_str()) {
                    Some(t) => t,
                    None => return serde_json::json!({"success": false, "error": "Missing title"}),
                };
                let description = arguments.get("description").and_then(|v| v.as_str());
                let priority = arguments
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .unwrap_or("medium");

                // Find project by name and verify membership
                #[derive(sqlx::FromRow)]
                struct ProjectId {
                    #[sqlx(try_from = "Vec<u8>")]
                    id: Uuid,
                }
                let project = sqlx::query_as::<_, ProjectId>(
                    r#"SELECT p.id FROM projects p
                       JOIN project_members pm ON p.id = pm.project_id
                       WHERE LOWER(p.name) = LOWER(?) AND pm.user_id = ?"#,
                )
                .bind(project_name)
                .bind(user_id.to_string())
                .fetch_optional(pool)
                .await;

                let project_id = match project {
                    Ok(Some(p)) => p.id,
                    Ok(None) => {
                        return serde_json::json!({"success": false, "error": format!("Project '{}' not found or you don't have access", project_name)})
                    }
                    Err(e) => {
                        return serde_json::json!({"success": false, "error": format!("Database error: {}", e)})
                    }
                };

                // Find default board for project
                #[derive(sqlx::FromRow)]
                struct BoardId {
                    #[sqlx(try_from = "Vec<u8>")]
                    id: Uuid,
                }
                let board = sqlx::query_as::<_, BoardId>(
                    "SELECT id FROM project_boards WHERE project_id = ? ORDER BY created_at ASC LIMIT 1",
                )
                .bind(project_id.to_string())
                .fetch_optional(pool)
                .await;

                let board_id = match board {
                    Ok(Some(b)) => Some(b.id),
                    _ => None,
                };

                let task_id = Uuid::new_v4();
                let result = sqlx::query(
                    r#"INSERT INTO tasks (id, project_id, board_id, title, description, status, priority, created_at, updated_at)
                       VALUES (?, ?, ?, ?, ?, 'todo', ?, datetime('now', 'subsec'), datetime('now', 'subsec'))"#,
                )
                .bind(task_id.to_string())
                .bind(project_id.to_string())
                .bind(board_id.map(|b| b.to_string()))
                .bind(title)
                .bind(description)
                .bind(priority)
                .execute(pool)
                .await;

                match result {
                    Ok(_) => serde_json::json!({
                        "success": true,
                        "message": format!("Task '{}' created in project '{}'", title, project_name),
                        "task_id": task_id.to_string(),
                        "project_id": project_id.to_string()
                    }),
                    Err(e) => {
                        serde_json::json!({"success": false, "error": format!("Failed to create task: {}", e)})
                    }
                }
            }
            "get_project_tasks" => {
                let project_name = match arguments.get("project_name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return serde_json::json!({"success": false, "error": "Missing project_name"})
                    }
                };
                let status_filter = arguments.get("status_filter").and_then(|v| v.as_str());

                // Verify membership
                #[derive(sqlx::FromRow)]
                struct ProjectId {
                    #[sqlx(try_from = "Vec<u8>")]
                    id: Uuid,
                }
                let project = sqlx::query_as::<_, ProjectId>(
                    r#"SELECT p.id FROM projects p
                       JOIN project_members pm ON p.id = pm.project_id
                       WHERE LOWER(p.name) = LOWER(?) AND pm.user_id = ?"#,
                )
                .bind(project_name)
                .bind(user_id.to_string())
                .fetch_optional(pool)
                .await;

                let project_id = match project {
                    Ok(Some(p)) => p.id,
                    Ok(None) => {
                        return serde_json::json!({"success": false, "error": format!("Project '{}' not found or you don't have access", project_name)})
                    }
                    Err(e) => {
                        return serde_json::json!({"success": false, "error": format!("Database error: {}", e)})
                    }
                };

                #[derive(sqlx::FromRow, serde::Serialize)]
                struct TaskRow {
                    #[sqlx(try_from = "Vec<u8>")]
                    id: Uuid,
                    title: String,
                    description: Option<String>,
                    status: String,
                    priority: String,
                    created_at: String,
                }

                let query = if let Some(status) = status_filter {
                    // Normalize status variants (LLMs may send snake_case or kebab-case)
                    let canonical_status = match status {
                        "in_progress" | "in-progress" => "inprogress",
                        "in_review" | "in-review" => "inreview",
                        other => other,
                    };
                    sqlx::query_as::<_, TaskRow>(
                        "SELECT id, title, description, status, priority, created_at FROM tasks WHERE project_id = ? AND LOWER(status) = LOWER(?)"
                    )
                    .bind(project_id.to_string())
                    .bind(canonical_status)
                    .fetch_all(pool)
                    .await
                } else {
                    sqlx::query_as::<_, TaskRow>(
                        "SELECT id, title, description, status, priority, created_at FROM tasks WHERE project_id = ?"
                    )
                    .bind(project_id.to_string())
                    .fetch_all(pool)
                    .await
                };

                match query {
                    Ok(tasks) => {
                        let task_list: Vec<serde_json::Value> = tasks
                            .iter()
                            .map(|t| {
                                serde_json::json!({
                                    "id": t.id.to_string(),
                                    "title": t.title,
                                    "description": t.description,
                                    "status": t.status,
                                    "priority": t.priority,
                                    "created_at": t.created_at
                                })
                            })
                            .collect();
                        serde_json::json!({
                            "success": true,
                            "project_name": project_name,
                            "task_count": task_list.len(),
                            "tasks": task_list
                        })
                    }
                    Err(e) => {
                        serde_json::json!({"success": false, "error": format!("Failed to get tasks: {}", e)})
                    }
                }
            }
            "update_task_status" => {
                let task_id_str = match arguments.get("task_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => {
                        return serde_json::json!({"success": false, "error": "Missing task_id"})
                    }
                };
                let status = match arguments.get("status").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => return serde_json::json!({"success": false, "error": "Missing status"}),
                };

                let task_id = match Uuid::parse_str(task_id_str) {
                    Ok(id) => id,
                    Err(_) => {
                        return serde_json::json!({"success": false, "error": "Invalid task_id format"})
                    }
                };

                // Verify user has access to the project this task belongs to
                let has_access: bool = sqlx::query_scalar::<_, i64>(
                    r#"SELECT COUNT(*) FROM tasks t
                       JOIN project_members pm ON t.project_id = pm.project_id
                       WHERE t.id = ? AND pm.user_id = ?"#,
                )
                .bind(task_id.to_string())
                .bind(user_id.to_string())
                .fetch_one(pool)
                .await
                .map(|c| c > 0)
                .unwrap_or(false);

                if !has_access {
                    return serde_json::json!({"success": false, "error": "Task not found or you don't have access"});
                }

                let result = sqlx::query(
                    "UPDATE tasks SET status = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
                )
                .bind(status)
                .bind(task_id.to_string())
                .execute(pool)
                .await;

                match result {
                    Ok(_) => {
                        serde_json::json!({"success": true, "message": format!("Task status updated to '{}'", status)})
                    }
                    Err(e) => {
                        serde_json::json!({"success": false, "error": format!("Failed to update task: {}", e)})
                    }
                }
            }
            "search_web" => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                serde_json::json!({
                    "success": true,
                    "message": format!("Web search for '{}' - search integration pending", query),
                    "results": []
                })
            }
            "fetch_web_page" => {
                let url = arguments.get("url").and_then(|v| v.as_str()).unwrap_or("");
                serde_json::json!({
                    "success": true,
                    "message": format!("Fetching '{}' - web fetch integration pending", url),
                    "content": ""
                })
            }
            "render_page" => {
                let url = arguments.get("url").and_then(|v| v.as_str()).unwrap_or("");
                let include_html = arguments
                    .get("include_html")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                serde_json::json!({
                    "success": true,
                    "message": format!("Rendering '{}' - use execute_tool for full browser rendering", url),
                    "include_html": include_html,
                    "text": ""
                })
            }
            "scrape_page" => {
                let url = arguments.get("url").and_then(|v| v.as_str()).unwrap_or("");
                serde_json::json!({
                    "success": false,
                    "message": format!("Scraping '{}' requires admin tool access", url),
                    "error": "scrape_page is an executive tool — invoke via Nora or a research workflow"
                })
            }
            _ => {
                serde_json::json!({"success": false, "error": format!("Unknown tool: {}", name)})
            }
        }
    }
}
