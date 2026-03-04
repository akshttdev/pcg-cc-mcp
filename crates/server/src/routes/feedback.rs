//! Feedback submission endpoints
//!
//! Creates tasks in the Bug Reports project for user feedback.

use axum::{Router, extract::State, response::Json as ResponseJson, routing::post};
use db::constants::{BUGREPORTS_BOARD_ID, BUGREPORTS_PROJECT_ID};
use db::models::task::{CreateTask, Priority, Task};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
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
    let create_task = CreateTask {
        project_id: BUGREPORTS_PROJECT_ID,
        pod_id: None,
        board_id: Some(BUGREPORTS_BOARD_ID),
        title: full_title,
        description: Some(full_description),
        parent_task_attempt: None,
        image_ids: None,
        priority: Some(priority),
        assignee_id: None,
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
    };

    Task::create(pool, &create_task, task_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create feedback task: {}", e)))?;

    Ok(ResponseJson(ApiResponse::success(SubmitFeedbackResponse {
        task_id,
        message: "Thank you for your feedback! We'll review it shortly.".to_string(),
    })))
}
