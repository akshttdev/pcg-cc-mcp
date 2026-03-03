//! Universal Person Entity — API Routes
//!
//! Persons are the single canonical identity record that bridges `users`
//! (platform accounts) and `crm_contacts` (external contacts).
//! Every client, contractor, lead, team member, and partner is a Person.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::person::{
    CreatePerson, ListPersonsQuery, Person, PersonSocialProfile, PersonWithSocials,
    UpdatePerson, UpsertPersonSocialProfile,
};
use db::models::invoice::{CreateInvoice, Invoice, UpdateInvoice};

// ---------------------------------------------------------------------------
// Query extractors
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListPersonsQueryParams {
    pub person_type: Option<String>,
    pub financial_role: Option<String>,
    pub lifecycle_stage: Option<String>,
    pub organization_id: Option<Uuid>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ---------------------------------------------------------------------------
// Person CRUD
// ---------------------------------------------------------------------------

/// GET /api/persons
async fn list_persons(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<ListPersonsQueryParams>,
) -> Result<Json<ApiResponse<Vec<Person>>>, ApiError> {
    let pool = &deployment.db().pool;
    let query = ListPersonsQuery {
        person_type: params.person_type,
        financial_role: params.financial_role,
        lifecycle_stage: params.lifecycle_stage,
        organization_id: params.organization_id,
        query: params.q,
        limit: params.limit,
        offset: params.offset,
    };
    let persons = Person::list(pool, &query).await?;
    Ok(Json(ApiResponse::success(persons)))
}

/// GET /api/persons/:id
async fn get_person(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<PersonWithSocials>>, ApiError> {
    let pool = &deployment.db().pool;

    let person = Person::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Person {} not found", id)))?;

    let social_profiles = PersonSocialProfile::list_for_person(pool, id).await?;

    Ok(Json(ApiResponse::success(PersonWithSocials {
        person,
        social_profiles,
    })))
}

/// POST /api/persons
async fn create_person(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreatePerson>,
) -> Result<Json<ApiResponse<Person>>, ApiError> {
    let pool = &deployment.db().pool;
    let person = Person::create(pool, data).await?;
    Ok(Json(ApiResponse::success(person)))
}

/// PATCH /api/persons/:id
async fn update_person(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePerson>,
) -> Result<Json<ApiResponse<Person>>, ApiError> {
    let pool = &deployment.db().pool;

    let person = Person::update(pool, id, data)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Person {} not found", id)))?;

    Ok(Json(ApiResponse::success(person)))
}

/// DELETE /api/persons/:id
async fn delete_person(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = Person::delete(pool, id).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!("Person {} not found", id)));
    }
    Ok(Json(ApiResponse::success(true)))
}

// ---------------------------------------------------------------------------
// Social profiles
// ---------------------------------------------------------------------------

/// GET /api/persons/:id/social-profiles
async fn list_social_profiles(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<PersonSocialProfile>>>, ApiError> {
    let pool = &deployment.db().pool;
    let profiles = PersonSocialProfile::list_for_person(pool, id).await?;
    Ok(Json(ApiResponse::success(profiles)))
}

/// POST /api/persons/:id/social-profiles
/// Upserts (creates or updates) a social profile for the given platform.
async fn upsert_social_profile(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpsertPersonSocialProfile>,
) -> Result<Json<ApiResponse<PersonSocialProfile>>, ApiError> {
    let pool = &deployment.db().pool;

    // Ensure person exists
    if Person::find_by_id(pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Person {} not found", id)));
    }

    let profile = PersonSocialProfile::upsert(pool, id, data).await?;
    Ok(Json(ApiResponse::success(profile)))
}

/// DELETE /api/persons/:id/social-profiles/:platform
async fn delete_social_profile(
    State(deployment): State<DeploymentImpl>,
    Path((id, platform)): Path<(Uuid, String)>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = PersonSocialProfile::delete(pool, id, &platform).await?;
    if !deleted {
        return Err(ApiError::NotFound(format!(
            "No {} profile for person {}",
            platform, id
        )));
    }
    Ok(Json(ApiResponse::success(true)))
}

// ---------------------------------------------------------------------------
// Invoices
// ---------------------------------------------------------------------------

/// GET /api/persons/:id/invoices
async fn list_person_invoices(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<Invoice>>>, ApiError> {
    let pool = &deployment.db().pool;
    let invoices = Invoice::list_for_person(pool, id).await?;
    Ok(Json(ApiResponse::success(invoices)))
}

/// POST /api/invoices
async fn create_invoice(
    State(deployment): State<DeploymentImpl>,
    Json(data): Json<CreateInvoice>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
    let invoice = Invoice::create(pool, data).await?;
    Ok(Json(ApiResponse::success(invoice)))
}

/// GET /api/invoices/:id
async fn get_invoice(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
    let invoice = Invoice::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;
    Ok(Json(ApiResponse::success(invoice)))
}

/// PATCH /api/invoices/:id
async fn update_invoice(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateInvoice>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
    let invoice = Invoice::update(pool, id, data)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;
    Ok(Json(ApiResponse::success(invoice)))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Person CRUD
        .route("/persons", get(list_persons).post(create_person))
        .route(
            "/persons/{id}",
            get(get_person).patch(update_person).delete(delete_person),
        )
        // Social profiles
        .route(
            "/persons/{id}/social-profiles",
            get(list_social_profiles).post(upsert_social_profile),
        )
        .route(
            "/persons/{id}/social-profiles/{platform}",
            delete(delete_social_profile),
        )
        // Person invoices
        .route("/persons/{id}/invoices", get(list_person_invoices))
        // Invoice CRUD
        .route("/invoices", post(create_invoice))
        .route(
            "/invoices/{id}",
            get(get_invoice).patch(update_invoice),
        )
        .with_state(deployment.clone())
}
