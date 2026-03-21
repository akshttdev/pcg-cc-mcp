//! Feedback submission endpoints
//!
//! Creates tasks in the Bug Reports project for user feedback.

use axum::{Extension, Router, extract::State, response::Json as ResponseJson, routing::post};
use db::{
    constants::{BUGREPORTS_BOARD_ID, BUGREPORTS_PROJECT_ID},
    db_uuid::DbUuid,
    models::{
        agent::Agent,
        data_source::{CreateDataSource, DataSource},
        task::{CreateTask, Priority, Task},
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::json;
use ts_rs::TS;
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/feedback", post(submit_feedback))
}

// Validation limits
const MAX_TITLE_LEN: usize = 200;
const MAX_DESCRIPTION_LEN: usize = 5000;
const MAX_FIELD_LEN: usize = 1000;
const VALID_TYPES: &[&str] = &[
    "bug",
    "feature",
    "improvement",
    "question",
    "friction",
    "other",
];
const VALID_SEVERITIES: &[&str] = &["low", "medium", "high", "critical"];

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct SubmitFeedbackRequest {
    /// Type of feedback: bug, feature, improvement, question, friction, other
    pub feedback_type: String,
    /// Brief title/summary (max 200 chars)
    pub title: String,
    /// Detailed description (max 5000 chars)
    pub description: String,
    /// Reporter's email (optional)
    pub email: Option<String>,
    /// Severity for bugs: low, medium, high, critical
    pub severity: Option<String>,
    /// Base64 encoded screenshot image (optional)
    pub screenshot: Option<String>,
    // ── Friction logging fields (dogfooding) ──
    /// Page/route where friction occurred (e.g. "/crm/deals")
    #[ts(optional)]
    pub page_url: Option<String>,
    /// What the user was trying to do (max 1000 chars)
    #[ts(optional)]
    pub user_intent: Option<String>,
    /// What went wrong or felt slow/confusing (max 1000 chars)
    #[ts(optional)]
    pub friction_point: Option<String>,
    /// Expected behavior vs actual (max 1000 chars)
    #[ts(optional)]
    pub expected_behavior: Option<String>,
    /// Time spent blocked (seconds, self-reported)
    #[ts(optional)]
    pub time_lost_seconds: Option<i32>,
    /// Frustration level: 1 (minor) to 5 (show-stopper)
    #[ts(optional)]
    pub frustration_level: Option<i32>,
}

impl SubmitFeedbackRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.title.trim().is_empty() {
            return Err("Title is required".into());
        }
        if self.title.len() > MAX_TITLE_LEN {
            return Err(format!(
                "Title must be {} characters or fewer",
                MAX_TITLE_LEN
            ));
        }
        if self.description.trim().is_empty() {
            return Err("Description is required".into());
        }
        if self.description.len() > MAX_DESCRIPTION_LEN {
            return Err(format!(
                "Description must be {} characters or fewer",
                MAX_DESCRIPTION_LEN
            ));
        }
        if !VALID_TYPES.contains(&self.feedback_type.as_str()) {
            return Err(format!("Invalid feedback type: {}", self.feedback_type));
        }
        if let Some(ref sev) = self.severity {
            if !VALID_SEVERITIES.contains(&sev.as_str()) {
                return Err(format!("Invalid severity: {}", sev));
            }
        }
        if let Some(ref fl) = self.frustration_level {
            if !(1..=5).contains(fl) {
                return Err("Frustration level must be between 1 and 5".into());
            }
        }
        // Length limits on optional text fields
        for (field, name) in [
            (&self.user_intent, "User intent"),
            (&self.friction_point, "Friction point"),
            (&self.expected_behavior, "Expected behavior"),
        ] {
            if let Some(v) = field {
                if v.len() > MAX_FIELD_LEN {
                    return Err(format!(
                        "{} must be {} characters or fewer",
                        name, MAX_FIELD_LEN
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SubmitFeedbackResponse {
    pub task_id: String,
    pub message: String,
}

/// POST /api/feedback
///
/// Submit user feedback which creates a task in the Bug Reports project.
pub async fn submit_feedback(
    Extension(access_context): Extension<crate::middleware::access_control::AccessContext>,
    State(deployment): State<DeploymentImpl>,
    ResponseJson(req): ResponseJson<SubmitFeedbackRequest>,
) -> Result<ResponseJson<ApiResponse<SubmitFeedbackResponse>>, ApiError> {
    // Authentication is enforced by require_auth middleware (route is in protected_routes)
    let _user_id = &access_context.user_id;
    let pool = &deployment.db().pool;

    // Validate all fields
    req.validate().map_err(ApiError::BadRequest)?;

    // Map severity to priority
    let priority = match req.severity.as_deref() {
        Some("critical") => Priority::Critical,
        Some("high") => Priority::High,
        Some("low") => Priority::Low,
        _ => match req.frustration_level {
            Some(5) => Priority::Critical,
            Some(4) => Priority::High,
            Some(1) => Priority::Low,
            _ => Priority::Medium,
        },
    };

    // Build title with type prefix
    let type_prefix = match req.feedback_type.as_str() {
        "bug" => "[Bug]",
        "feature" => "[Feature Request]",
        "improvement" => "[Improvement]",
        "question" => "[Question]",
        "friction" => "[Friction]",
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
    // Append friction context
    if let Some(ref page) = req.page_url {
        full_description.push_str(&format!("\nPage: {}", page));
    }
    if let Some(ref intent) = req.user_intent {
        full_description.push_str(&format!("\nUser intent: {}", intent));
    }
    if let Some(ref friction) = req.friction_point {
        full_description.push_str(&format!("\nFriction point: {}", friction));
    }
    if let Some(ref expected) = req.expected_behavior {
        full_description.push_str(&format!("\nExpected: {}", expected));
    }
    if let Some(time) = req.time_lost_seconds {
        full_description.push_str(&format!("\nTime lost: {}s", time));
    }
    if let Some(level) = req.frustration_level {
        full_description.push_str(&format!("\nFrustration: {}/5", level));
    }

    // Build tags based on feedback type
    let mut tags = vec![req.feedback_type.clone(), "user-submitted".to_string()];
    if req.feedback_type == "friction" {
        tags.push("dogfood".to_string());
    }

    let task_id = DbUuid::new();
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
        collaborators: None,
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
                tracing::info!(
                    "Assigned dev agent '{}' to feedback task {}",
                    agent.short_name,
                    task_id_str
                );
            }
        }
        Ok(None) => {
            tracing::warn!("No active agents found for feedback task assignment");
        }
        Err(e) => {
            tracing::warn!("Failed to look up dev agents for feedback task: {e}");
        }
    }

    // Also create a DataSource so workflow triggers (Feedback Triage Pipeline) fire automatically
    let ds_metadata = json!({
        "feedback_type": req.feedback_type,
        "severity": req.severity,
        "email": req.email,
        "task_id": task_id_str,
        "page_url": req.page_url,
        "user_intent": req.user_intent,
        "friction_point": req.friction_point,
        "expected_behavior": req.expected_behavior,
        "time_lost_seconds": req.time_lost_seconds,
        "frustration_level": req.frustration_level,
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
            super::data_source_workflows::fire_triggers_for_data_source(
                trigger_pool,
                ds_id,
                trigger_dep,
            )
            .await;
        });
    }

    Ok(ResponseJson(ApiResponse::success(SubmitFeedbackResponse {
        task_id: task_id_str,
        message: "Thank you for your feedback! We'll review it shortly.".to_string(),
    })))
}
