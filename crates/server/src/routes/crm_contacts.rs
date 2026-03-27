//! CRM Contact Management Routes
//!
//! Handles contact creation, lead scoring, lifecycle management, and Zoho CRM sync.

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
};
use db::{
    db_uuid::DbUuid,
    models::{
        contact_association::{ContactCompanyRole, ContactOrgLink},
        contact_note::{ContactNote, CreateContactNote, UpdateContactNote},
        contact_social_profile::ContactSocialProfile,
        crm_contact::{
            ContactSearchParams, CreateCrmContact, CrmContact, LifecycleStage, UpdateCrmContact,
        },
        invoice::Invoice,
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

#[derive(Debug, Deserialize)]
pub struct ListContactsQuery {
    pub organization_id: Uuid,

    pub lifecycle_stage: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SearchContactsQuery {
    pub organization_id: Uuid,

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

/// Helper: load a contact and verify org membership
async fn require_contact_org_access(
    access: &AccessContext,
    pool: &sqlx::SqlitePool,
    contact_id: &DbUuid,
) -> Result<CrmContact, ApiError> {
    let contact = CrmContact::find_by_id(pool, contact_id).await?;
    if let Some(ref org_id) = contact.organization_id {
        access.require_org_membership(pool, org_id.as_str()).await?;
    }
    Ok(contact)
}

/// GET /crm/contacts - List contacts by organization
async fn list_contacts(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<ListContactsQuery>,
) -> Result<Json<ApiResponse<Vec<CrmContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context
        .require_org_membership(pool, &query.organization_id.to_string())
        .await?;

    let org_id = DbUuid::from(query.organization_id);
    let contacts = if let Some(stage_str) = query.lifecycle_stage {
        let stage: LifecycleStage = stage_str
            .parse()
            .map_err(|_| ApiError::BadRequest(format!("Invalid lifecycle stage: {}", stage_str)))?;
        let params = ContactSearchParams {
            organization_id: Some(org_id.clone()),
            client_id: None,
            query: None,
            lifecycle_stage: Some(stage),
            company_name: None,
            tags: None,
            min_lead_score: None,
            limit: query.limit,
            offset: None,
        };
        CrmContact::search(pool, params).await?
    } else {
        CrmContact::find_by_organization(pool, &org_id, query.limit).await?
    };

    Ok(Json(ApiResponse::success(contacts)))
}

/// POST /crm/contacts - Create contact
async fn create_contact(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateCrmContact>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context
        .require_org_membership(pool, data.organization_id.as_str())
        .await?;

    // Check if contact with this email already exists
    if let Some(ref email) = data.email {
        if let Some(existing) =
            CrmContact::find_by_email(pool, &data.organization_id, email).await?
        {
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
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<SearchContactsQuery>,
) -> Result<Json<ApiResponse<Vec<CrmContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context
        .require_org_membership(pool, &query.organization_id.to_string())
        .await?;

    let lifecycle_stage = query
        .lifecycle_stage
        .and_then(|s| s.parse::<LifecycleStage>().ok());

    let params = ContactSearchParams {
        organization_id: Some(DbUuid::from(query.organization_id)),
        client_id: None,

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

/// GET /crm/contacts/stats/:organization_id - Get contact statistics by org
async fn get_contact_stats(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(organization_id): Path<String>,
) -> Result<Json<ApiResponse<ContactStats>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context
        .require_org_membership(pool, &organization_id)
        .await?;
    let organization_id = DbUuid::from(organization_id);

    let stage_counts: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT COALESCE(lifecycle_stage, 'unknown') as stage, COUNT(*) as count
        FROM crm_contacts
        WHERE organization_id = ?1
        GROUP BY COALESCE(lifecycle_stage, 'unknown')
        ORDER BY count DESC
        "#,
    )
    .bind(organization_id.as_str())
    .fetch_all(pool)
    .await?;

    let total: i64 = stage_counts.iter().map(|(_, c)| c).sum();

    let by_stage: Vec<StageCount> = stage_counts
        .into_iter()
        .map(|(stage, count)| StageCount { stage, count })
        .collect();

    let avg_score: (f64,) = sqlx::query_as(
        r#"SELECT COALESCE(AVG(CAST(lead_score AS REAL)), 0.0) FROM crm_contacts WHERE organization_id = ?1"#
    )
    .bind(organization_id.as_str())
    .fetch_one(pool)
    .await?;

    let needs_follow_up: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM crm_contacts
        WHERE organization_id = ?1
        AND COALESCE(lifecycle_stage, 'lead') != 'churned'
        AND (
            last_activity_at IS NULL
            OR last_activity_at < datetime('now', '-7 days')
        )
        "#,
    )
    .bind(organization_id.as_str())
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
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    let contact = require_contact_org_access(&access_context, pool, &id).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// GET /crm/contacts/by-email/:organization_id/:email - Get contact by email (org-scoped)
async fn get_contact_by_email(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path((organization_id, email)): Path<(String, String)>,
) -> Result<Json<ApiResponse<Option<CrmContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    access_context
        .require_org_membership(pool, &organization_id)
        .await?;
    let organization_id = DbUuid::from(organization_id);
    let contact = CrmContact::find_by_email(pool, &organization_id, &email).await?;

    Ok(Json(ApiResponse::success(contact)))
}

/// PATCH /crm/contacts/:id - Update contact
async fn update_contact(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(update): Json<UpdateCrmContact>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    let contact = CrmContact::update(pool, &id, update).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// POST /crm/contacts/:id/activity - Record activity
async fn record_activity(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    CrmContact::record_activity(pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/contacts/:id/contacted - Record contact made
async fn record_contacted(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    CrmContact::record_contact_made(pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/contacts/:id/replied - Record reply received
async fn record_replied(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    CrmContact::record_reply_received(pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

/// POST /crm/contacts/:id/lead-score - Update lead score
async fn update_lead_score(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(request): Json<UpdateLeadScoreRequest>,
) -> Result<Json<ApiResponse<CrmContact>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    CrmContact::update_lead_score(pool, &id, request.score_delta).await?;
    let contact = CrmContact::find_by_id(pool, &id).await?;
    Ok(Json(ApiResponse::success(contact)))
}

/// DELETE /crm/contacts/:id - Delete contact
async fn delete_contact(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    CrmContact::delete(pool, &id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ---------------------------------------------------------------------------
// Sub-resources: Notes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListNotesQuery {
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNoteRequest {
    pub text: String,
    pub status: Option<String>,
}

/// GET /crm/contacts/:id/notes - List notes for a contact
async fn list_contact_notes(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Query(query): Query<ListNotesQuery>,
) -> Result<Json<ApiResponse<Vec<ContactNote>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    let notes = ContactNote::list_for_contact(pool, id.as_str(), query.status.as_deref()).await?;
    Ok(Json(ApiResponse::success(notes)))
}

/// POST /crm/contacts/:id/notes - Create a note for a contact
async fn create_contact_note(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<CreateNoteRequest>,
) -> Result<Json<ApiResponse<ContactNote>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;

    let input = CreateContactNote {
        crm_contact_id: id.to_string(),
        author_id: access_context.user_id.map(|u| *u.as_uuid()),
        text: body.text,
        status: body.status,
        proposal_id: None,
    };
    let note = ContactNote::create(pool, input).await?;
    Ok(Json(ApiResponse::success(note)))
}

/// PATCH /crm/contact-notes/:note_id - Update a note
async fn update_contact_note(
    Extension(_access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(note_id): Path<String>,
    Json(body): Json<UpdateContactNote>,
) -> Result<Json<ApiResponse<ContactNote>>, ApiError> {
    let pool = &deployment.db().pool;
    let note_uuid = Uuid::parse_str(&note_id)
        .map_err(|_| ApiError::BadRequest(format!("Invalid note ID: {}", note_id)))?;
    let note = ContactNote::update(pool, note_uuid, body)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Note {} not found", note_id)))?;
    Ok(Json(ApiResponse::success(note)))
}

/// DELETE /crm/contact-notes/:note_id - Delete a note
async fn delete_contact_note(
    Extension(_access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(note_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let note_uuid = Uuid::parse_str(&note_id)
        .map_err(|_| ApiError::BadRequest(format!("Invalid note ID: {}", note_id)))?;
    let deleted = ContactNote::delete(pool, note_uuid).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("Note {} not found", note_id)));
    }
    Ok(Json(ApiResponse::success(())))
}

// ---------------------------------------------------------------------------
// Sub-resources: Social Profiles
// ---------------------------------------------------------------------------

/// GET /crm/contacts/:id/social-profiles - List social profiles for a contact
async fn list_contact_social_profiles(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ContactSocialProfile>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    let profiles = ContactSocialProfile::list_for_contact(pool, &id).await?;
    Ok(Json(ApiResponse::success(profiles)))
}

// ---------------------------------------------------------------------------
// Sub-resources: Company Roles
// ---------------------------------------------------------------------------

/// GET /crm/contacts/:id/companies - List company roles for a contact
async fn list_contact_companies(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ContactCompanyRole>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    let roles = ContactCompanyRole::list_for_contact(pool, id.as_str()).await?;
    Ok(Json(ApiResponse::success(roles)))
}

// ---------------------------------------------------------------------------
// Sub-resources: Organization Links
// ---------------------------------------------------------------------------

/// GET /crm/contacts/:id/organizations - List org links for a contact
async fn list_contact_organizations(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<ContactOrgLink>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    let links = ContactOrgLink::list_for_contact(pool, id.as_str()).await?;
    Ok(Json(ApiResponse::success(links)))
}

// ---------------------------------------------------------------------------
// Sub-resources: Invoices
// ---------------------------------------------------------------------------

/// GET /crm/contacts/:id/invoices - List invoices for a contact
async fn list_contact_invoices(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Invoice>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    require_contact_org_access(&access_context, pool, &id).await?;
    let invoices: Vec<Invoice> =
        sqlx::query_as("SELECT * FROM invoices WHERE crm_contact_id = ?1 ORDER BY created_at DESC")
            .bind(id.as_str())
            .fetch_all(pool)
            .await?;
    Ok(Json(ApiResponse::success(invoices)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/crm/contacts", get(list_contacts))
        .route("/crm/contacts", post(create_contact))
        .route("/crm/contacts/search", get(search_contacts))
        .route(
            "/crm/contacts/stats/{organization_id}",
            get(get_contact_stats),
        )
        .route("/crm/contacts/{id}", get(get_contact))
        .route("/crm/contacts/{id}", patch(update_contact))
        .route("/crm/contacts/{id}", delete(delete_contact))
        .route("/crm/contacts/{id}/activity", post(record_activity))
        .route("/crm/contacts/{id}/contacted", post(record_contacted))
        .route("/crm/contacts/{id}/replied", post(record_replied))
        .route("/crm/contacts/{id}/lead-score", post(update_lead_score))
        .route(
            "/crm/contacts/by-email/{organization_id}/{email}",
            get(get_contact_by_email),
        )
        // Sub-resources: notes
        .route("/crm/contacts/{id}/notes", get(list_contact_notes))
        .route("/crm/contacts/{id}/notes", post(create_contact_note))
        .route("/crm/contact-notes/{note_id}", patch(update_contact_note))
        .route("/crm/contact-notes/{note_id}", delete(delete_contact_note))
        // Sub-resources: social profiles, companies, orgs, invoices
        .route(
            "/crm/contacts/{id}/social-profiles",
            get(list_contact_social_profiles),
        )
        .route("/crm/contacts/{id}/companies", get(list_contact_companies))
        .route(
            "/crm/contacts/{id}/organizations",
            get(list_contact_organizations),
        )
        .route("/crm/contacts/{id}/invoices", get(list_contact_invoices))
}
