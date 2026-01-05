use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use db::models::execution_summary::{ExecutionSummary, UpdateExecutionSummaryFeedback};
use deployment::Deployment;
use serde::Deserialize;
use uuid::Uuid;

use crate::DeploymentImpl;

pub fn routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/task-attempts/{attempt_id}/summary", get(get_summary_by_attempt))
        .route("/execution-summaries/{id}", get(get_summary))
        .route("/execution-summaries/{id}/feedback", post(update_feedback))
}

/// Get execution summary by ID
async fn get_summary(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let summary = ExecutionSummary::find_by_id(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match summary {
        Some(s) => Ok(Json(s)),
        None => Err((StatusCode::NOT_FOUND, "Execution summary not found".to_string())),
    }
}

/// Get the latest execution summary for a task attempt
async fn get_summary_by_attempt(
    State(deployment): State<DeploymentImpl>,
    Path(attempt_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let summary = ExecutionSummary::find_by_task_attempt_id(&deployment.db().pool, attempt_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match summary {
        Some(s) => Ok(Json(s)),
        None => Err((StatusCode::NOT_FOUND, "No execution summary found for this attempt".to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct FeedbackRequest {
    human_rating: Option<i32>,
    human_notes: Option<String>,
    is_reference_example: Option<bool>,
}

/// Update human feedback on an execution summary
async fn update_feedback(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(request): Json<FeedbackRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // First check if the summary exists
    let summary = ExecutionSummary::find_by_id(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if summary.is_none() {
        return Err((StatusCode::NOT_FOUND, "Execution summary not found".to_string()));
    }

    // Update the feedback
    let feedback = UpdateExecutionSummaryFeedback {
        human_rating: request.human_rating,
        human_notes: request.human_notes,
        is_reference_example: request.is_reference_example,
    };

    ExecutionSummary::update_feedback(&deployment.db().pool, id, feedback)
        .await
        .map_err(|e| match e {
            db::models::execution_summary::ExecutionSummaryError::InvalidRating => {
                (StatusCode::BAD_REQUEST, "Rating must be between 1 and 5".to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        })?;

    // Return the updated summary
    let updated = ExecutionSummary::find_by_id(&deployment.db().pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Execution summary not found".to_string()))?;

    Ok(Json(updated))
}
