//! Workflow template routes — list templates and convert deals to projects

use axum::{
    Router,
    extract::{Path, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use db::{
    db_uuid::DbUuid,
    models::{
        crm_activity::CrmActivity,
        crm_contact::CrmContact,
        project::{CreateProject, Project},
        project_board::{CreateProjectBoard, ProjectBoard, ProjectBoardType},
        project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource},
        task::{CreateTask, Priority, Task},
        task_dependency::{CreateTaskDependency, DependencyType, TaskDependency},
        workflow_template::{DealConversionResult, WorkflowTemplate},
    },
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

/// GET /api/workflow-templates — list all available templates
async fn list_templates() -> Result<ResponseJson<ApiResponse<Vec<WorkflowTemplate>>>, ApiError> {
    let templates = WorkflowTemplate::defaults();
    Ok(ResponseJson(ApiResponse::success(templates)))
}

/// GET /api/workflow-templates/:id — get a single template
async fn get_template(
    Path(id): Path<String>,
) -> Result<ResponseJson<ApiResponse<WorkflowTemplate>>, ApiError> {
    let templates = WorkflowTemplate::defaults();
    let template = templates
        .into_iter()
        .find(|t| t.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("Template '{}' not found", id)))?;
    Ok(ResponseJson(ApiResponse::success(template)))
}

#[derive(Debug, Deserialize)]
pub struct ConvertDealRequest {
    pub template_id: String,
    pub project_name: Option<String>,
    pub organization_id: Option<Uuid>,
    pub client_id: Option<Uuid>,
    pub git_repo_path: Option<String>,
}

/// POST /api/crm/deals/:deal_id/convert — convert a deal into a scaffolded project
async fn convert_deal(
    Path(deal_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    axum::Json(payload): axum::Json<ConvertDealRequest>,
) -> Result<ResponseJson<ApiResponse<DealConversionResult>>, ApiError> {
    let pool = &deployment.db().pool;

    // 1. Look up the template
    let templates = WorkflowTemplate::defaults();
    let template = templates
        .into_iter()
        .find(|t| t.id == payload.template_id)
        .ok_or_else(|| {
            ApiError::NotFound(format!("Template '{}' not found", payload.template_id))
        })?;

    // 2. Look up the deal to get context
    let deal_db_id = DbUuid::from(deal_id);
    let deal = db::models::crm_deal::CrmDeal::find_by_id(pool, &deal_db_id).await?;

    // 3. Create the project
    let project_name = payload
        .project_name
        .unwrap_or_else(|| format!("{} — {}", deal.name, template.name));
    let git_repo_path = payload.git_repo_path.unwrap_or_else(|| {
        let slug = project_name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
            .trim_matches('-')
            .to_string();
        format!("/tmp/topos/clients/{}", slug)
    });

    let project_id = Uuid::new_v4();
    let create_project = CreateProject {
        name: project_name.clone(),
        git_repo_path,
        use_existing_repo: false,
        setup_script: None,
        dev_script: None,
        cleanup_script: None,
        copy_files: None,
        organization_id: payload.organization_id.map(|u| u.to_string()),
        client_id: payload.client_id.map(|u| u.to_string()),
        folder_id: None,
        parent_project_id: None,
    };

    let project = Project::create(pool, &create_project, &project_id.to_string())
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create project: {}", e)))?;

    // 4. Scaffold boards and tasks from the template
    let mut boards_created = 0i32;
    let mut tasks_created = 0i32;
    let mut dependencies_created = 0i32;

    // Track task UUIDs by (phase_position, task_position) for dependency wiring
    let mut task_uuid_map: std::collections::HashMap<(i32, i32), Uuid> =
        std::collections::HashMap::new();

    // Track the last review gate per phase for cross-phase dependencies
    let mut phase_last_review_task: std::collections::HashMap<i32, Uuid> =
        std::collections::HashMap::new();

    for phase in &template.phases {
        // Create a board for each phase
        let slug = phase
            .name
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
            .trim_matches('-')
            .to_string();

        let board = ProjectBoard::create(
            pool,
            &CreateProjectBoard {
                project_id: project.id.clone(),
                name: phase.name.clone(),
                slug,
                board_type: ProjectBoardType::Custom,
                description: Some(phase.description.clone()),
                metadata: Some(
                    serde_json::json!({
                        "workflow_phase": phase.position,
                        "is_recurring": phase.is_recurring,
                        "template_id": template.id,
                    })
                    .to_string(),
                ),
            },
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create board: {}", e)))?;
        boards_created += 1;

        // Create tasks for this phase
        for task_tmpl in &phase.tasks {
            let task_id = Uuid::new_v4();

            // Build custom properties with workflow metadata
            let custom_properties = serde_json::json!({
                "workflow_template_id": template.id,
                "workflow_phase": phase.name,
                "workflow_phase_position": phase.position,
                "task_type": format!("{:?}", task_tmpl.task_type).to_lowercase(),
                "agent_role": task_tmpl.agent_role,
                "is_recurring": phase.is_recurring,
                "knowledge_inputs": task_tmpl.knowledge_inputs,
                "knowledge_outputs": task_tmpl.knowledge_outputs,
            });

            let priority = match task_tmpl.priority.as_str() {
                "critical" => Priority::Critical,
                "high" => Priority::High,
                "low" => Priority::Low,
                _ => Priority::Medium,
            };

            let tags: Option<Vec<String>> = if task_tmpl.tags.is_empty() {
                None
            } else {
                Some(task_tmpl.tags.clone())
            };

            let create_task = CreateTask {
                project_id: project_id.to_string(),
                pod_id: None,
                board_id: Some(board.id.clone()),
                title: task_tmpl.title.clone(),
                description: Some(task_tmpl.description.clone()),
                parent_task_attempt: None,
                image_ids: None,
                priority: Some(priority),
                assignee_id: None,
                assignee_type: None,
                assigned_agent: task_tmpl.agent_role.clone(),
                agent_id: None,
                assigned_mcps: None,
                created_by: "system".to_string(),
                requires_approval: Some(task_tmpl.requires_approval),
                parent_task_id: None,
                tags,
                due_date: None,
                custom_properties: Some(custom_properties),
                scheduled_start: None,
                scheduled_end: None,
                screenshot: None,
                completion_criteria: None,
                output_format: None,
                collaborators: None,
            };

            Task::create(pool, &create_task, &task_id.to_string())
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to create task: {}", e)))?;

            task_uuid_map.insert((phase.position, task_tmpl.position), task_id);
            tasks_created += 1;

            // Track the last review gate for cross-phase dependency
            if task_tmpl.requires_approval {
                phase_last_review_task.insert(phase.position, task_id);
            }
        }

        // Create intra-phase dependencies
        for task_tmpl in &phase.tasks {
            let target_id = task_uuid_map[&(phase.position, task_tmpl.position)];

            for &dep_pos in &task_tmpl.depends_on {
                if let Some(&source_id) = task_uuid_map.get(&(phase.position, dep_pos)) {
                    TaskDependency::create(
                        pool,
                        &CreateTaskDependency {
                            project_id: project_id,
                            source_task_id: source_id,
                            target_task_id: target_id,
                            dependency_type: DependencyType::Blocks,
                        },
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalError(format!("Failed to create dependency: {}", e))
                    })?;
                    dependencies_created += 1;
                }
            }
        }

        // Create cross-phase dependencies:
        // First task of each phase depends on the last review gate of the previous phase
        if phase.position > 0 {
            if let Some(&prev_review_id) = phase_last_review_task.get(&(phase.position - 1)) {
                // Find the first task (position 0) in this phase
                if let Some(&first_task_id) = task_uuid_map.get(&(phase.position, 0)) {
                    TaskDependency::create(
                        pool,
                        &CreateTaskDependency {
                            project_id: project_id,
                            source_task_id: prev_review_id,
                            target_task_id: first_task_id,
                            dependency_type: DependencyType::Blocks,
                        },
                    )
                    .await
                    .map_err(|e| {
                        ApiError::InternalError(format!(
                            "Failed to create cross-phase dependency: {}",
                            e
                        ))
                    })?;
                    dependencies_created += 1;
                }
            }
        }
    }

    // 5. Register initial knowledge sources for the project
    let knowledge_domains: Vec<&str> = template
        .phases
        .iter()
        .flat_map(|p| p.tasks.iter())
        .flat_map(|t| t.knowledge_outputs.iter().map(|s| s.as_str()))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for domain in &knowledge_domains {
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            project_id,
            &KnowledgeSourceType::Entity,
            &format!("workflow_{}", domain),
            &format!("Knowledge: {}", domain.replace('_', " "),),
            Some(&format!("Auto-registered from {} template", template.name)),
            0.0, // starts at 0 coverage — builds as tasks execute
        )
        .await;
    }

    // 6. Seed knowledge from deal context (η: F_crm → F_knowledge)
    //    Carry deal metadata, contact profile, and activity history into the
    //    new project's knowledge sheaf so agents start with full context.
    seed_deal_knowledge(pool, project_id, &deal, deal_id).await;

    // 7. Update the deal's custom_fields to link to the new project
    let existing_custom_fields: serde_json::Value = deal
        .custom_fields
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::json!({}));

    let mut updated_fields = existing_custom_fields
        .as_object()
        .cloned()
        .unwrap_or_default();
    updated_fields.insert(
        "converted_project_id".to_string(),
        serde_json::Value::String(project.id.to_string()),
    );
    updated_fields.insert(
        "converted_template".to_string(),
        serde_json::Value::String(template.id.clone()),
    );

    let custom_fields_json = serde_json::to_string(&updated_fields).unwrap_or_default();
    let _ = sqlx::query(
        "UPDATE crm_deals SET custom_fields = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
    )
    .bind(&custom_fields_json)
    .bind(&deal_db_id)
    .execute(pool)
    .await;

    Ok(ResponseJson(ApiResponse::success(DealConversionResult {
        project_id: project.id.to_string(),
        project_name,
        boards_created,
        tasks_created,
        dependencies_created,
        template_used: template.id,
    })))
}

/// Seed project knowledge from deal context.
/// Implements the natural transformation η: F_crm → F_knowledge,
/// carrying CRM data into the project's knowledge sheaf at conversion time.
async fn seed_deal_knowledge(
    pool: &sqlx::SqlitePool,
    project_id: Uuid,
    deal: &db::models::crm_deal::CrmDeal,
    deal_id: Uuid,
) {
    // 1. Deal context → context_injection source
    let mut deal_parts: Vec<String> = Vec::new();
    deal_parts.push(format!("Deal: {}", deal.name));
    if let Some(ref desc) = deal.description {
        deal_parts.push(format!("Description: {}", desc));
    }
    if let Some(amount) = deal.amount {
        deal_parts.push(format!("Value: {} {}", amount, deal.currency));
    }
    deal_parts.push(format!(
        "Stage: {} ({}% probability)",
        deal.stage, deal.probability
    ));
    if let Some(ref close) = deal.expected_close_date {
        deal_parts.push(format!("Expected close: {}", close.format("%Y-%m-%d")));
    }
    if let Some(ref tags) = deal.tags {
        deal_parts.push(format!("Tags: {}", tags));
    }
    let deal_summary = deal_parts.join("\n");

    let _ = ProjectKnowledgeSource::upsert_source(
        pool,
        project_id,
        &KnowledgeSourceType::ContextInjection,
        &format!("deal_context_{}", deal_id),
        "Deal Context",
        Some(&deal_summary),
        0.5,
    )
    .await;

    // 2. Contact profile → entity source (reusable across projects)
    if let Some(ref contact_id) = deal.crm_contact_id {
        if let Ok(contact) = CrmContact::find_by_id(pool, contact_id).await {
            let mut contact_parts: Vec<String> = Vec::new();
            if let Some(ref name) = contact.full_name {
                contact_parts.push(format!("Name: {}", name));
            }
            if let Some(ref company) = contact.company_name {
                contact_parts.push(format!("Company: {}", company));
            }
            if let Some(ref title) = contact.job_title {
                contact_parts.push(format!("Title: {}", title));
            }
            if let Some(ref email) = contact.email {
                contact_parts.push(format!("Email: {}", email));
            }
            if let Some(ref phone) = contact.phone {
                contact_parts.push(format!("Phone: {}", phone));
            }
            if let Some(ref linkedin) = contact.linkedin_url {
                contact_parts.push(format!("LinkedIn: {}", linkedin));
            }
            if let Some(ref website) = contact.website {
                contact_parts.push(format!("Website: {}", website));
            }
            contact_parts.push(format!("Lifecycle: {}", contact.lifecycle_stage));
            contact_parts.push(format!("Lead score: {}", contact.lead_score));
            if contact.total_revenue > 0.0 {
                contact_parts.push(format!("Total revenue: ${:.2}", contact.total_revenue));
            }
            let contact_summary = contact_parts.join("\n");

            let contact_title = contact.full_name.as_deref().unwrap_or("Contact");

            let _ = ProjectKnowledgeSource::upsert_source(
                pool,
                project_id,
                &KnowledgeSourceType::Entity,
                &format!("contact_{}", contact_id),
                &format!("Contact: {}", contact_title),
                Some(&contact_summary),
                0.6,
            )
            .await;
        }
    }

    // 3. Recent deal activities → context_injection source
    if let Ok(activities) = CrmActivity::find_by_deal(pool, deal_id, Some(20)).await {
        if !activities.is_empty() {
            let activity_lines: Vec<String> = activities
                .iter()
                .map(|a| {
                    let subject = a.subject.as_deref().unwrap_or("");
                    let outcome = a
                        .outcome
                        .as_deref()
                        .map(|o| format!(" → {}", o))
                        .unwrap_or_default();
                    format!(
                        "[{}] {}: {}{}",
                        a.activity_at.format("%Y-%m-%d"),
                        a.activity_type,
                        subject,
                        outcome,
                    )
                })
                .collect();
            let activity_summary = activity_lines.join("\n");

            let _ = ProjectKnowledgeSource::upsert_source(
                pool,
                project_id,
                &KnowledgeSourceType::ContextInjection,
                &format!("deal_activities_{}", deal_id),
                &format!("Deal Activity History ({} activities)", activities.len()),
                Some(&activity_summary),
                0.4,
            )
            .await;
        }
    }
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/workflow-templates", get(list_templates))
        .route("/workflow-templates/{id}", get(get_template))
        .route("/crm/deals/{deal_id}/convert", post(convert_deal))
}
