//! Feedback submission endpoints
//!
//! Creates tasks in the Bug Reports project for user feedback.

use axum::{Router, extract::State, response::Json as ResponseJson, routing::post};
use db::constants::{BUGREPORTS_BOARD_ID, BUGREPORTS_PROJECT_ID};
use db::models::agent::Agent;
use db::models::data_source::{CreateDataSource, DataSource};
use db::models::task::{CreateTask, Priority, Task};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/feedback", post(submit_feedback))
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SubmitFeedbackRequest {
    /// Type of feedback: bug, feature, improvement, question, other
    pub feedback_type: String,
    /// Brief title/summary
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Reporter's email (optional)
    pub email: Option<String>,
    /// Severity for bugs: low, medium, high, critical
    pub severity: Option<String>,
    /// Base64 encoded screenshot image (optional)
    pub screenshot: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SubmitFeedbackResponse {
    pub task_id: Uuid,
    pub message: String,
}

/// POST /api/feedback
///
/// Submit user feedback which creates a task in the Bug Reports project.
pub async fn submit_feedback(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<SubmitFeedbackRequest>,
) -> Result<ResponseJson<ApiResponse<SubmitFeedbackResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    // Map severity to priority
    let priority = match req.severity.as_deref() {
        Some("critical") => Priority::Critical,
        Some("high") => Priority::High,
        Some("low") => Priority::Low,
        _ => Priority::Medium,
    };

    // Build title with type prefix
    let type_prefix = match req.feedback_type.as_str() {
        "bug" => "[Bug]",
        "feature" => "[Feature Request]",
        "improvement" => "[Improvement]",
        "question" => "[Question]",
        _ => "[Feedback]",
    };
    let full_title = format!("{} {}", type_prefix, req.title);

    // Build description with metadata
    let mut full_description = req.description.clone();
    if let Some(email) = &req.email {
        full_description.push_str(&format!("\n\n---\nReporter: {}", email));
    }
    full_description.push_str(&format!("\nType: {}", req.feedback_type));
    if let Some(severity) = &req.severity {
        full_description.push_str(&format!("\nSeverity: {}", severity));
    }

    // Build tags based on feedback type
    let tags = vec![
        req.feedback_type.clone(),
        "user-submitted".to_string(),
    ];

    let task_id = Uuid::new_v4();
    let task_id_str = task_id.to_string();
    let create_task = CreateTask {
        project_id: BUGREPORTS_PROJECT_ID.to_string(),
        pod_id: None,
        board_id: Some(BUGREPORTS_BOARD_ID.to_string()),
        title: full_title.clone(),
        description: Some(full_description.clone()),
        parent_task_attempt: None,
        image_ids: None,
        priority: Some(priority),
        assignee_id: None,
        assignee_type: None,
        assigned_agent: None,
        agent_id: None,
        assigned_mcps: None,
        created_by: req.email.clone().unwrap_or_else(|| "anonymous".to_string()),
        requires_approval: None,
        parent_task_id: None,
        tags: Some(tags),
        due_date: None,
        custom_properties: None,
        scheduled_start: None,
        scheduled_end: None,
        screenshot: req.screenshot.clone(),
        completion_criteria: None,
        output_format: None,
    };

    Task::create(pool, &create_task, &task_id_str)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create feedback task: {}", e)))?;

    // Assign a default dev agent to the task
    match Agent::find_default_assignee(pool).await {
        Ok(Some(agent)) => {
            if let Err(e) = Task::assign_agent(pool, &task_id_str, &agent.id).await {
                tracing::warn!("Failed to assign dev agent to feedback task: {e}");
            } else {
                tracing::info!("Assigned dev agent '{}' to feedback task {}", agent.short_name, task_id_str);
            }
        }
        Ok(None) => {
            tracing::warn!("No active agents found for feedback task assignment");
        }
        Err(e) => {
            tracing::warn!("Failed to look up dev agents for feedback task: {e}");
        }
    }

    // Also create a DataSource so workflow triggers (Bug Triage Pipeline) fire automatically
    let ds_metadata = json!({
        "feedback_type": req.feedback_type,
        "severity": req.severity,
        "email": req.email,
        "task_id": task_id_str,
    });
    // Look up project's organization_id for the trigger filter
    let project_id_str = BUGREPORTS_PROJECT_ID.to_string();
    let org_id_for_trigger = {
        use db::models::project::Project;
        Project::find_by_id(pool, &project_id_str)
            .await
            .ok()
            .flatten()
            .and_then(|p| p.organization_id)
    };
    let create_ds = CreateDataSource {
        organization_id: org_id_for_trigger,
        project_id: Some(BUGREPORTS_PROJECT_ID.to_string()),
        created_by: req.email.clone(),
        title: format!("{} {}", type_prefix, req.title),
        description: Some(req.description.clone()),
        data_type: "report".to_string(),
        source_type: Some("integration".to_string()),
        file_type: None,
        content: Some(full_description),
        file_name: None,
        file_path: None,
        file_size_bytes: None,
        file_hash: None,
        metadata: Some(ds_metadata.to_string()),
        folder: Some("Feedback".to_string()),
    };
    if let Ok(ds) = DataSource::create(pool, create_ds).await {
        let trigger_pool = pool.clone();
        let ds_id = ds.id.clone();
        let trigger_dep = deployment.clone();
        tokio::spawn(async move {
            super::data_source_workflows::fire_triggers_for_data_source(trigger_pool, ds_id, trigger_dep).await;
        });
    }

    Ok(ResponseJson(ApiResponse::success(SubmitFeedbackResponse {
        task_id,
        message: "Thank you for your feedback! We'll review it shortly.".to_string(),
    })))
}
