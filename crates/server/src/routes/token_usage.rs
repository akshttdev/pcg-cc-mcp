use axum::{
    Router,
    extract::{Path, Query, State},
    response::Json as ResponseJson,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use db::models::token_usage::{
    CreateTokenUsage, DailyTokenUsage, TokenUsage, TokenUsageByAgent, TokenUsageByProject,
    TokenUsageSummary,
};
use deployment::Deployment;
use serde::Deserialize;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

#[derive(Debug, Deserialize, TS)]
pub struct TokenUsageQuery {
    pub days: Option<i32>,
}

#[derive(Debug, Deserialize, TS)]
pub struct ProjectTokenQuery {
    pub project_id: Uuid,
    pub days: Option<i32>,
}

/// Get today's token usage summary
pub async fn get_today_usage(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenUsageSummary>>, ApiError> {
    let pool = &deployment.db().pool;
    let summary = TokenUsage::today_total(pool).await?;
    Ok(ResponseJson(ApiResponse::success(summary)))
}

/// Get daily token usage for the last N days
pub async fn get_daily_usage(
    Query(query): Query<TokenUsageQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<DailyTokenUsage>>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = query.days.unwrap_or(7);
    let usage = TokenUsage::daily_totals(pool, days).await?;
    Ok(ResponseJson(ApiResponse::success(usage)))
}

/// Get token usage by project
pub async fn get_usage_by_project(
    Query(query): Query<TokenUsageQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TokenUsageByProject>>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = query.days.unwrap_or(7);
    let since = Utc::now() - Duration::days(days as i64);
    let usage = TokenUsage::by_project(pool, since).await?;
    Ok(ResponseJson(ApiResponse::success(usage)))
}

/// Get token usage by agent
pub async fn get_usage_by_agent(
    Query(query): Query<TokenUsageQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<TokenUsageByAgent>>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = query.days.unwrap_or(7);
    let since = Utc::now() - Duration::days(days as i64);
    let usage = TokenUsage::by_agent(pool, since).await?;
    Ok(ResponseJson(ApiResponse::success(usage)))
}

/// Get token usage for a specific project
pub async fn get_project_usage(
    Path(project_id): Path<Uuid>,
    Query(query): Query<TokenUsageQuery>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenUsageSummary>>, ApiError> {
    let pool = &deployment.db().pool;
    let days = query.days.unwrap_or(7);
    let since = Utc::now() - Duration::days(days as i64);
    let usage = TokenUsage::sum_by_project(pool, project_id, since).await?;
    Ok(ResponseJson(ApiResponse::success(usage)))
}

/// Get token usage for a specific task attempt
pub async fn get_task_usage(
    Path(task_attempt_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<TokenUsageSummary>>, ApiError> {
    let pool = &deployment.db().pool;
    let usage = TokenUsage::sum_by_task(pool, task_attempt_id).await?;
    Ok(ResponseJson(ApiResponse::success(usage)))
}

/// Record new token usage
pub async fn record_usage(
    State(deployment): State<DeploymentImpl>,
    ResponseJson(data): ResponseJson<CreateTokenUsage>,
) -> Result<ResponseJson<ApiResponse<TokenUsage>>, ApiError> {
    let pool = &deployment.db().pool;
    let usage = TokenUsage::create(pool, data).await?;
    Ok(ResponseJson(ApiResponse::success(usage)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/token-usage/today", get(get_today_usage))
        .route("/token-usage/daily", get(get_daily_usage))
        .route("/token-usage/by-project", get(get_usage_by_project))
        .route("/token-usage/by-agent", get(get_usage_by_agent))
        .route("/token-usage/projects/{project_id}", get(get_project_usage))
        .route("/token-usage/tasks/{task_attempt_id}", get(get_task_usage))
        .route("/token-usage", post(record_usage))
}
