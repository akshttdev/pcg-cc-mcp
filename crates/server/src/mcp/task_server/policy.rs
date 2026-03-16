use db::models::{
    comment::{AuthorType, CommentType, CreateTaskComment, TaskComment},
    person::{ListPersonsQuery, Person},
    project::Project,
    project_knowledge_source::ProjectKnowledgeSource,
    task::Task,
};
use rmcp::{
    ErrorData,
    handler::server::tool::Parameters,
    model::{CallToolResult, Content},
    tool,
};
use serde_json::Value;
use services::services::pcg_policy::{self, PolicyAction, PolicyCheckContext};
use uuid::Uuid;

use super::TaskServer;
use super::helpers::*;
use super::types::*;

impl TaskServer {
    #[tool(description = "Evaluate PCG governance policies for a task before execution.")]
    pub(super) async fn evaluate_policy(
        &self,
        Parameters(EvaluatePolicyRequest {
            project_id,
            task_id,
            severity,
            tags,
            actor,
        }): Parameters<EvaluatePolicyRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let tags_vec = tags.unwrap_or_default();
        let context = PolicyCheckContext {
            task_id: task_id.clone(),
            project_id: project_id.clone(),
            severity: severity.clone(),
            tags: tags_vec.clone(),
            actor,
        };

        match pcg_policy::evaluate(context.clone()) {
            Ok(decision) => {
                pcg_policy::record_decision(&context, &decision);
                let actions: Vec<String> = decision
                    .actions
                    .iter()
                    .map(|action| match action {
                        PolicyAction::RequireHumanReview => "require_human_review".to_string(),
                        PolicyAction::EscalateCrisis => "escalate_crisis".to_string(),
                        PolicyAction::StartWorkflow { workflow } => {
                            format!("start_workflow:{workflow}")
                        }
                        PolicyAction::Annotate { key, value } => {
                            format!("annotate:{key}={value}")
                        }
                    })
                    .collect();

                Ok(success_json(&EvaluatePolicyResponse {
                    allow: decision.allow,
                    actions,
                    notes: decision.notes.clone(),
                }))
            }
            Err(e) => Ok(error_result("Failed to evaluate PCG policy", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Add a comment to a task. Use this to leave notes, status updates, or handoff messages. `project_id`, `task_id`, and `content` are required!"
    )]
    pub(super) async fn add_comment(
        &self,
        Parameters(AddCommentRequest {
            project_id,
            task_id,
            content,
            comment_type,
        }): Parameters<AddCommentRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let _project_uuid = match Uuid::parse_str(&project_id) {
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

        let ct = match comment_type.as_deref() {
            Some("status_update") => CommentType::StatusUpdate,
            Some("review") => CommentType::Review,
            Some("system") => CommentType::System,
            Some("handoff") => CommentType::Handoff,
            _ => CommentType::Comment,
        };

        let author_id = self
            .user_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "mcp-agent".to_string());

        let data = CreateTaskComment {
            task_id: task_uuid,
            author_id,
            author_type: AuthorType::Agent,
            content,
            comment_type: Some(ct),
            parent_comment_id: None,
            mentions: None,
            metadata: None,
        };

        match TaskComment::create(&self.pool, &data).await {
            Ok(comment) => {
                let response = AddCommentResponse {
                    success: true,
                    comment_id: comment.id.to_string(),
                    message: "Comment added successfully".to_string(),
                };
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
        description = "Search across projects, tasks, knowledge sources, and persons in a single query. Returns categorized results. Use entity_types to narrow scope."
    )]
    pub(super) async fn unified_search(
        &self,
        Parameters(req): Parameters<UnifiedSearchRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let limit = req.limit.unwrap_or(10).clamp(1, 50);
        let search_all = req.entity_types.is_none();
        let types: Vec<String> = req
            .entity_types
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.to_lowercase())
            .collect();

        let search_projects = search_all || types.contains(&"projects".to_string());
        let search_tasks = search_all || types.contains(&"tasks".to_string());
        let search_knowledge = search_all || types.contains(&"knowledge".to_string());
        let search_persons = search_all || types.contains(&"persons".to_string());

        let mut results = serde_json::json!({ "query": req.query });

        // Search projects
        if search_projects {
            let all_projects = match self.accessible_project_ids().await {
                Ok(Some(ids)) => {
                    let mut matched = Vec::new();
                    for pid in ids {
                        if let Ok(Some(p)) = Project::find_by_id(&self.pool, &pid.to_string()).await {
                            let q_lower = req.query.to_lowercase();
                            if p.name.to_lowercase().contains(&q_lower)
                                || p.git_repo_path
                                    .to_string_lossy()
                                    .to_lowercase()
                                    .contains(&q_lower)
                            {
                                matched.push(serde_json::json!({
                                    "id": p.id.to_string(),
                                    "name": p.name,
                                    "git_repo_path": p.git_repo_path.to_string_lossy(),
                                }));
                            }
                        }
                        if matched.len() >= limit as usize {
                            break;
                        }
                    }
                    matched
                }
                Ok(None) => {
                    // Admin — search all
                    match Project::find_all(&self.pool).await {
                        Ok(projects) => {
                            let q_lower = req.query.to_lowercase();
                            projects
                                .iter()
                                .filter(|p| {
                                    p.name.to_lowercase().contains(&q_lower)
                                        || p.git_repo_path
                                            .to_string_lossy()
                                            .to_lowercase()
                                            .contains(&q_lower)
                                })
                                .take(limit as usize)
                                .map(|p| {
                                    serde_json::json!({
                                        "id": p.id.to_string(),
                                        "name": p.name,
                                        "git_repo_path": p.git_repo_path.to_string_lossy(),
                                    })
                                })
                                .collect()
                        }
                        Err(_) => vec![],
                    }
                }
                Err(_) => vec![],
            };
            results["projects"] = serde_json::json!({
                "count": all_projects.len(),
                "results": all_projects,
            });
        }

        // Search tasks
        if search_tasks {
            let like = format!("%{}%", req.query);
            let project_filter = req.project_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());

            let tasks: Vec<Value> = if let Some(pid) = project_filter {
                match Task::find_by_project_id_with_attempt_status(&self.pool, &pid.to_string()).await {
                    Ok(tasks) => {
                        let q_lower = req.query.to_lowercase();
                        tasks
                            .iter()
                            .filter(|t| {
                                t.title.to_lowercase().contains(&q_lower)
                                    || t.description
                                        .as_deref()
                                        .unwrap_or("")
                                        .to_lowercase()
                                        .contains(&q_lower)
                            })
                            .take(limit as usize)
                            .map(|t| to_json_value(&task_with_status_to_summary(t)))
                            .collect()
                    }
                    Err(_) => vec![],
                }
            } else {
                // Cross-project search using raw SQL LIKE
                match sqlx::query_as::<_, Task>(
                    "SELECT * FROM tasks WHERE (title LIKE ?1 OR description LIKE ?1) ORDER BY updated_at DESC LIMIT ?2"
                )
                .bind(&like)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await {
                    Ok(tasks) => tasks.iter().map(|t| to_json_value(&task_to_summary(t))).collect(),
                    Err(_) => vec![],
                }
            };
            results["tasks"] = serde_json::json!({
                "count": tasks.len(),
                "results": tasks,
            });
        }

        // Search knowledge
        if search_knowledge {
            let project_filter = req.project_id.as_deref().and_then(|s| Uuid::parse_str(s).ok());
            let like = format!("%{}%", req.query);

            let knowledge: Vec<Value> = if let Some(pid) = project_filter {
                match ProjectKnowledgeSource::find_by_project(&self.pool, pid).await {
                    Ok(sources) => {
                        let q_lower = req.query.to_lowercase();
                        sources
                            .iter()
                            .filter(|s| {
                                s.source_title
                                    .to_lowercase()
                                    .contains(&q_lower)
                                    || s.source_summary
                                        .as_deref()
                                        .unwrap_or("")
                                        .to_lowercase()
                                        .contains(&q_lower)
                            })
                            .take(limit as usize)
                            .map(|s| {
                                serde_json::json!({
                                    "id": s.id.to_string(),
                                    "project_id": s.project_id.to_string(),
                                    "source_type": s.source_type,
                                    "source_title": s.source_title,
                                    "source_summary": s.source_summary,
                                })
                            })
                            .collect()
                    }
                    Err(_) => vec![],
                }
            } else {
                match sqlx::query_as::<_, ProjectKnowledgeSource>(
                    "SELECT * FROM project_knowledge_sources WHERE (source_title LIKE ?1 OR source_summary LIKE ?1) ORDER BY updated_at DESC LIMIT ?2"
                )
                .bind(&like)
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await {
                    Ok(sources) => sources.iter().map(|s| serde_json::json!({
                        "id": s.id.to_string(),
                        "project_id": s.project_id.to_string(),
                        "source_type": s.source_type,
                        "source_title": s.source_title,
                        "source_summary": s.source_summary,
                    })).collect(),
                    Err(_) => vec![],
                }
            };
            results["knowledge"] = serde_json::json!({
                "count": knowledge.len(),
                "results": knowledge,
            });
        }

        // Search persons
        if search_persons {
            let persons: Vec<Value> = match Person::list(
                &self.pool,
                &ListPersonsQuery {
                    person_type: None,
                    financial_role: None,
                    lifecycle_stage: None,
                    organization_id: None,
                    query: Some(req.query.clone()),
                    limit: Some(limit as i64),
                    offset: None,
                },
            )
            .await
            {
                Ok(people) => people
                    .iter()
                    .map(|p| {
                        serde_json::json!({
                            "id": p.id.to_string(),
                            "full_name": p.full_name,
                            "email": p.email,
                            "person_type": p.person_type,
                            "company_name": p.company_name,
                            "job_title": p.job_title,
                        })
                    })
                    .collect(),
                Err(_) => vec![],
            };
            results["persons"] = serde_json::json!({
                "count": persons.len(),
                "results": persons,
            });
        }

        Ok(success_json(&results))
    }
}
