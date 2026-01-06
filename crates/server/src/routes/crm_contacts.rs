//! CRM Contact Management Routes
//!
//! Handles contact creation, lead scoring, lifecycle management, and Zoho CRM sync.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{get, post, delete, patch},
    Json,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::crm_contact::{
    CrmContact, CreateCrmContact, UpdateCrmContact, ContactSearchParams, LifecycleStage
};

#[derive(Debug, Deserialize)]
pub struct ListContactsQuery {
    pub project_id: Uuid,
    pub lifecycle_stage: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchContactsQuery {
    pub project_id: Uuid,
    pub query: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub company_name: Option<String>,
    pub min_lead_score: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ContactStats {
    pub total: i64,
    pub by_stage: Vec<StageCount>,
    pub avg_lead_score: f64,
    pub needs_follow_up: i64,
}

#[derive(Debug, Serialize)]
pub struct StageCount {
    pub stage: String,
    pub count: i64,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLeadScoreRequest {
    pub score_delta: i32,
}

/// GET /crm/contacts - List contacts
async fn list_contacts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListContactsQuery>,
) -> Result<Json<ApiResponse<Vec<CrmContact>>>, ApiError> {
    let pool = &deployment.db().pool;

    let contacts = if let Some(stage_str) = query.lifecycle_stage {
        let stage: LifecycleStage = stage_str.parse()
            .map_err(|_| ApiError::BadRequest(format!("Invalid lifecycle stage: {}", stage_str)))?;
        CrmContact::find_by_lifecycle_stage(pool, query.project_id, stage).await?
    } else {
        CrmContact::find_by_project(pool, query.project_id, query.limit).await?
    };

    Ok(Json(ApiResponse::success(contacts)))
}

/// POST /crm/contacts - Create contact
async fn create_contact(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateCrmContact>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;

    // Check if contact with this email already exists
    if let Some(ref email) = data.email {
        if let Some(existing) = CrmContact::find_by_email(pool, data.project_id, email).await? {
            return Err(ApiError::Conflict(format!(
                "Contact with email {} already exists: {}",
                email, existing.id
            )));
        }
    }

    let contact = CrmContact::create(pool, data).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// GET /crm/contacts/search - Search contacts
async fn search_contacts(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<SearchContactsQuery>,
) -> Result<Json<ApiResponse<Vec<CrmContact>>>, ApiError> {
    let pool = &deployment.db().pool;

    let lifecycle_stage = query.lifecycle_stage
        .and_then(|s| s.parse::<LifecycleStage>().ok());

    let params = ContactSearchParams {
        project_id: query.project_id,
        query: query.query,
        lifecycle_stage,
        company_name: query.company_name,
        tags: None,
        min_lead_score: query.min_lead_score,
        limit: query.limit,
        offset: query.offset,
    };

    let contacts = CrmContact::search(pool, params).await?;
    Ok(Json(ApiResponse::success(contacts)))
}

/// GET /crm/contacts/stats/:project_id - Get contact statistics
async fn get_contact_stats(
    State(deployment): State<DeploymentImpl>,
    Path(project_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ContactStats>>, ApiError> {
    let pool = &deployment.db().pool;

    // Get counts by stage
    let stage_counts: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT lifecycle_stage, COUNT(*) as count
        FROM crm_contacts
        WHERE project_id = ?1
        GROUP BY lifecycle_stage
        ORDER BY count DESC
        "#
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let total: i64 = stage_counts.iter().map(|(_, c)| c).sum();

    let by_stage: Vec<StageCount> = stage_counts
        .into_iter()
        .map(|(stage, count)| StageCount { stage, count })
        .collect();

    // Get average lead score
    let avg_score: (f64,) = sqlx::query_as(
        r#"SELECT COALESCE(AVG(lead_score), 0) FROM crm_contacts WHERE project_id = ?1"#
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    // Get contacts that need follow-up (no activity in 7 days, not churned)
    let needs_follow_up: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM crm_contacts
        WHERE project_id = ?1
        AND lifecycle_stage != 'churned'
        AND (
            last_activity_at IS NULL
            OR datetime(last_activity_at, '+7 days') < datetime('now')
        )
        "#
    )
    .bind(project_id)
    .fetch_one(pool)
    .await?;

    Ok(Json(ApiResponse::success(ContactStats {
        total,
        by_stage,
        avg_lead_score: avg_score.0,
        needs_follow_up: needs_follow_up.0,
    })))
}

/// GET /crm/contacts/:id - Get single contact
async fn get_contact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;
    let contact = CrmContact::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// GET /crm/contacts/by-email/:email - Get contact by email
async fn get_contact_by_email(
    State(deployment): State<DeploymentImpl>,
    Path((project_id, email)): Path<(Uuid, String)>,
) -> Result<Json<ApiResponse<Option<CrmContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    let contact = CrmContact::find_by_email(pool, project_id, &email).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// PATCH /crm/contacts/:id - Update contact
async fn update_contact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(update): Json<UpdateCrmContact>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;
    let contact = CrmContact::update(pool, id, update).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// POST /crm/contacts/:id/activity - Record activity
async fn record_activity(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmContact::record_activity(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/contacts/:id/contacted - Record contact made
async fn record_contacted(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmContact::record_contact_made(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/contacts/:id/replied - Record reply received
async fn record_replied(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmContact::record_reply_received(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/contacts/:id/lead-score - Update lead score
async fn update_lead_score(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateLeadScoreRequest>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmContact::update_lead_score(pool, id, request.score_delta).await?;
    let contact = CrmContact::find_by_id(pool, id).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// DELETE /crm/contacts/:id - Delete contact
async fn delete_contact(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    CrmContact::delete(pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/crm/contacts", get(list_contacts))
        .route("/crm/contacts", post(create_contact))
        .route("/crm/contacts/search", get(search_contacts))
        .route("/crm/contacts/stats/{project_id}", get(get_contact_stats))
        .route("/crm/contacts/{id}", get(get_contact))
        .route("/crm/contacts/{id}", patch(update_contact))
        .route("/crm/contacts/{id}", delete(delete_contact))
        .route("/crm/contacts/{id}/activity", post(record_activity))
        .route("/crm/contacts/{id}/contacted", post(record_contacted))
        .route("/crm/contacts/{id}/replied", post(record_replied))
        .route("/crm/contacts/{id}/lead-score", post(update_lead_score))
        .route("/crm/contacts/by-email/{project_id}/{email}", get(get_contact_by_email))
}
