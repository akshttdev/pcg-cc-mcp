//! Universal Person Entity — API Routes
//!
//! Persons are the single canonical identity record that bridges `users`
//! (platform accounts) and `crm_contacts` (external contacts).
//! Every client, contractor, lead, team member, and partner is a Person.

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch},
    Json,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;
use db::db_uuid::DbUuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::person::{
    CreatePerson, ListPersonsQuery, Person, PersonSocialProfile, PersonWithSocials,
    UpdatePerson, UpsertPersonSocialProfile,
};
use db::models::invoice::{CreateInvoice, Invoice, UpdateInvoice};
use db::models::person_note::{CreatePersonNote, PersonNote, UpdatePersonNote};
use db::models::person_association::{
    PersonCompanyRole, PersonOrgContact, UpsertPersonCompanyRole, PatchPersonCompanyRole,
    UpsertPersonOrgContact,
};

const VIBE_PER_USD: f64 = 100.0; // 1 USD = 100 VIBE (1 VIBE = $0.01)

// ---------------------------------------------------------------------------
// Query extractors
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListInvoicesParams {
    pub invoice_type: Option<String>,
    pub status: Option<String>,
    pub person_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MoveInvoiceStatusBody {
    pub status: String,
}

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
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<PersonWithSocials>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();

    let person = Person::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Person {} not found", id)))?;

    let social_profiles = PersonSocialProfile::list_for_person(pool, id).await?;
    let company_roles = PersonCompanyRole::list_for_person(pool, id).await?;
    let org_contacts = PersonOrgContact::list_for_person(pool, id).await?;

    Ok(Json(ApiResponse::success(PersonWithSocials {
        person,
        social_profiles,
        company_roles,
        org_contacts,
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
    Path(id): Path<String>,
    Json(data): Json<UpdatePerson>,
) -> Result<Json<ApiResponse<Person>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();

    let person = Person::update(pool, id, data)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Person {} not found", id)))?;

    Ok(Json(ApiResponse::success(person)))
}

/// DELETE /api/persons/:id
async fn delete_person(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
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
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<PersonSocialProfile>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let profiles = PersonSocialProfile::list_for_person(pool, id).await?;
    Ok(Json(ApiResponse::success(profiles)))
}

/// POST /api/persons/:id/social-profiles
/// Upserts (creates or updates) a social profile for the given platform.
async fn upsert_social_profile(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<UpsertPersonSocialProfile>,
) -> Result<Json<ApiResponse<PersonSocialProfile>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();

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
    Path((id, platform)): Path<(String, String)>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
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
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Invoice>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let invoices = Invoice::list_for_person(pool, id).await?;
    Ok(Json(ApiResponse::success(invoices)))
}

/// GET /api/invoices?invoice_type=ar&status=pending&person_id=...&project_id=...&limit=200
async fn list_invoices(
    State(deployment): State<DeploymentImpl>,
    Query(p): Query<ListInvoicesParams>,
) -> Result<Json<ApiResponse<Vec<Invoice>>>, ApiError> {
    let pool = &deployment.db().pool;
    let mut qb = sqlx::QueryBuilder::new("SELECT * FROM invoices WHERE 1=1");
    if let Some(t) = &p.invoice_type { qb.push(" AND invoice_type = ").push_bind(t.clone()); }
    if let Some(s) = &p.status { qb.push(" AND status = ").push_bind(s.clone()); }
    if let Some(pid) = p.person_id { qb.push(" AND person_id = ").push_bind(pid); }
    if let Some(proj) = p.project_id { qb.push(" AND project_id = ").push_bind(proj); }
    qb.push(" ORDER BY created_at DESC LIMIT ").push_bind(p.limit.unwrap_or(200));
    let invoices = qb.build_query_as::<Invoice>().fetch_all(pool).await?;
    Ok(Json(ApiResponse::success(invoices)))
}

/// POST /api/invoices
async fn create_invoice(
    State(deployment): State<DeploymentImpl>,
    Json(mut data): Json<CreateInvoice>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
    // Auto-compute VIBE from USD
    if data.amount_vibe.is_none() {
        if let Some(usd) = data.amount_usd {
            if usd > 0.0 {
                data.amount_vibe = Some((usd * VIBE_PER_USD).ceil() as i64);
            }
        }
    }
    let invoice = Invoice::create(pool, data).await?;
    Ok(Json(ApiResponse::success(invoice)))
}

/// GET /api/invoices/:id
async fn get_invoice(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let invoice = Invoice::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;
    Ok(Json(ApiResponse::success(invoice)))
}

/// PATCH /api/invoices/:id
async fn update_invoice(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(mut data): Json<UpdateInvoice>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    // Auto-recompute VIBE if USD changed
    if data.amount_vibe.is_none() {
        if let Some(usd) = data.amount_usd {
            data.amount_vibe = Some((usd * VIBE_PER_USD).ceil() as i64);
        }
    }
    let invoice = Invoice::update(pool, id, data)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))?;
    Ok(Json(ApiResponse::success(invoice)))
}

/// PATCH /api/invoices/:id/status
async fn move_invoice_status(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<MoveInvoiceStatusBody>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let sql = match body.status.as_str() {
        "paid" | "partial" =>
            "UPDATE invoices SET status = ?, paid_at = datetime('now','subsec'), updated_at = datetime('now','subsec') WHERE id = ?",
        _ =>
            "UPDATE invoices SET status = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    };
    sqlx::query(sql).bind(&body.status).bind(id).execute(pool).await?;
    Invoice::find_by_id(pool, id)
        .await?
        .map(|i| Json(ApiResponse::success(i)))
        .ok_or_else(|| ApiError::NotFound(format!("Invoice {} not found", id)))
}

/// DELETE /api/invoices/:id
async fn delete_invoice(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let deleted = Invoice::delete(pool, id).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound(format!("Invoice {} not found", id)))
    }
}

// ---------------------------------------------------------------------------
// Company associations
// ---------------------------------------------------------------------------

/// GET /api/persons/:id/companies
async fn list_person_companies(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<PersonCompanyRole>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let roles = PersonCompanyRole::list_for_person(pool, id).await?;
    Ok(Json(ApiResponse::success(roles)))
}

/// POST /api/persons/:id/companies
async fn add_person_company(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<UpsertPersonCompanyRole>,
) -> Result<Json<ApiResponse<PersonCompanyRole>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    if Person::find_by_id(pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Person {} not found", id)));
    }
    let role = PersonCompanyRole::upsert(pool, id, data).await?;
    Ok(Json(ApiResponse::success(role)))
}

/// PATCH /api/persons/:id/companies/:company_id
async fn patch_person_company(
    State(deployment): State<DeploymentImpl>,
    Path((id, company_id)): Path<(String, String)>,
    Json(data): Json<PatchPersonCompanyRole>,
) -> Result<Json<ApiResponse<PersonCompanyRole>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let company_id = DbUuid::parse(&company_id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", company_id)))?.to_uuid();
    let role = PersonCompanyRole::patch(pool, id, company_id, data)
        .await?
        .ok_or_else(|| ApiError::NotFound("Association not found".into()))?;
    Ok(Json(ApiResponse::success(role)))
}

/// DELETE /api/persons/:id/companies/:company_id
async fn delete_person_company(
    State(deployment): State<DeploymentImpl>,
    Path((id, company_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let company_id = DbUuid::parse(&company_id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", company_id)))?.to_uuid();
    let deleted = PersonCompanyRole::delete(pool, id, company_id).await?;
    if !deleted {
        return Err(ApiError::NotFound("Association not found".into()));
    }
    Ok(Json(ApiResponse::success(())))
}

// ---------------------------------------------------------------------------
// Organization associations
// ---------------------------------------------------------------------------

/// GET /api/persons/:id/organizations
async fn list_person_orgs(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<PersonOrgContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let orgs = PersonOrgContact::list_for_person(pool, id).await?;
    Ok(Json(ApiResponse::success(orgs)))
}

/// POST /api/persons/:id/organizations
async fn add_person_org(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<UpsertPersonOrgContact>,
) -> Result<Json<ApiResponse<PersonOrgContact>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    if Person::find_by_id(pool, id).await?.is_none() {
        return Err(ApiError::NotFound(format!("Person {} not found", id)));
    }
    let org = PersonOrgContact::upsert(pool, id, data).await?;
    Ok(Json(ApiResponse::success(org)))
}

/// DELETE /api/persons/:id/organizations/:org_id
async fn delete_person_org(
    State(deployment): State<DeploymentImpl>,
    Path((id, org_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let org_id = DbUuid::parse(&org_id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", org_id)))?.to_uuid();
    let deleted = PersonOrgContact::delete(pool, id, org_id).await?;
    if !deleted {
        return Err(ApiError::NotFound("Association not found".into()));
    }
    Ok(Json(ApiResponse::success(())))
}

// ---------------------------------------------------------------------------
// Person Notes
// ---------------------------------------------------------------------------

pub async fn list_person_notes(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<PersonNote>>>, ApiError> {
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    let notes = PersonNote::list_for_person(&deployment.db().pool, id, None).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(notes)))
}

pub async fn create_person_note(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(mut body): Json<CreatePersonNote>,
) -> Result<Json<ApiResponse<PersonNote>>, ApiError> {
    let id = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", id)))?.to_uuid();
    body.person_id = id;
    let note = PersonNote::create(&deployment.db().pool, body).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(note)))
}

pub async fn update_person_note(
    Path(note_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<UpdatePersonNote>,
) -> Result<Json<ApiResponse<PersonNote>>, ApiError> {
    let note_id = DbUuid::parse(&note_id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", note_id)))?.to_uuid();
    let note = PersonNote::update(&deployment.db().pool, note_id, body).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Note not found".into()))?;
    Ok(Json(ApiResponse::success(note)))
}

pub async fn delete_person_note(
    Path(note_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let note_id = DbUuid::parse(&note_id).map_err(|_| ApiError::BadRequest(format!("Invalid UUID: {}", note_id)))?.to_uuid();
    PersonNote::delete(&deployment.db().pool, note_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok(Json(ApiResponse::success(())))
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
        // Company associations
        .route("/persons/{id}/companies", get(list_person_companies).post(add_person_company))
        .route(
            "/persons/{id}/companies/{company_id}",
            patch(patch_person_company).delete(delete_person_company),
        )
        // Org associations
        .route("/persons/{id}/organizations", get(list_person_orgs).post(add_person_org))
        .route("/persons/{id}/organizations/{org_id}", delete(delete_person_org))
        // Person notes
        .route("/persons/{id}/notes", get(list_person_notes).post(create_person_note))
        .route("/person-notes/{note_id}", patch(update_person_note).delete(delete_person_note))
        // Person invoices
        .route("/persons/{id}/invoices", get(list_person_invoices))
        // Invoice CRUD
        .route("/invoices", get(list_invoices).post(create_invoice))
        .route(
            "/invoices/{id}",
            get(get_invoice).patch(update_invoice).delete(delete_invoice),
        )
        .route("/invoices/{id}/status", patch(move_invoice_status))
        .with_state(deployment.clone())
}
