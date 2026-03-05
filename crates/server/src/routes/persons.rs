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
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};
use db::models::person::{
    CreatePerson, ListPersonsQuery, Person, PersonSocialProfile, PersonWithSocials,
    UpdatePerson, UpsertPersonSocialProfile,
};
use db::models::invoice::{CreateInvoice, Invoice, UpdateInvoice};
use db::models::person_note::{CreatePersonNote, PersonNote, UpdatePersonNote};

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
    pub assigned_to: Option<Uuid>,
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ListNotesParams {
    pub status: Option<String>,
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
        assigned_to: params.assigned_to,
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
    Json(mut data): Json<UpdateInvoice>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
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
    Path(id): Path<Uuid>,
    Json(body): Json<MoveInvoiceStatusBody>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &deployment.db().pool;
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
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = Invoice::delete(pool, id).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound(format!("Invoice {} not found", id)))
    }
}

// ---------------------------------------------------------------------------
// Person Notes
// ---------------------------------------------------------------------------

/// GET /api/persons/:id/notes
async fn list_person_notes(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Query(p): Query<ListNotesParams>,
) -> Result<Json<ApiResponse<Vec<PersonNote>>>, ApiError> {
    let pool = &deployment.db().pool;
    let notes = PersonNote::list_for_person(pool, id, p.status.as_deref()).await?;
    Ok(Json(ApiResponse::success(notes)))
}

/// POST /api/persons/:id/notes
async fn create_person_note(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(mut data): Json<CreatePersonNote>,
) -> Result<Json<ApiResponse<PersonNote>>, ApiError> {
    let pool = &deployment.db().pool;
    data.person_id = id;
    let note = PersonNote::create(pool, data).await?;
    Ok(Json(ApiResponse::success(note)))
}

/// PATCH /api/person-notes/:id
async fn update_person_note(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdatePersonNote>,
) -> Result<Json<ApiResponse<PersonNote>>, ApiError> {
    let pool = &deployment.db().pool;
    PersonNote::update(pool, id, data)
        .await?
        .map(|n| Json(ApiResponse::success(n)))
        .ok_or_else(|| ApiError::NotFound(format!("Note {} not found", id)))
}

/// DELETE /api/person-notes/:id
async fn delete_person_note(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = PersonNote::delete(pool, id).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound(format!("Note {} not found", id)))
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Org Provisioning
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ProvisionOrgResponse {
    org_id: String,
    org_name: String,
    slug: String,
}

/// POST /api/persons/:id/provision-org
/// Create a "shadow" org for a person's company and link it via company_org_id.
async fn provision_org(
    State(deployment): State<DeploymentImpl>,
    Path(person_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ProvisionOrgResponse>>, ApiError> {
    let pool = &deployment.db().pool;

    let person = Person::find_by_id(pool, person_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Person {} not found", person_id)))?;

    if person.company_org_id.is_some() {
        return Err(ApiError::BadRequest(
            "This person already has a company org provisioned".into(),
        ));
    }

    let company_name = person
        .company_name
        .ok_or_else(|| ApiError::BadRequest("Person has no company_name set".into()))?;

    // Slugify: lowercase, replace non-alphanumeric with hyphens, collapse runs
    let slug = company_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    // Ensure slug is unique
    #[derive(sqlx::FromRow)]
    struct SlugCount { count: i64 }
    let existing: SlugCount = sqlx::query_as("SELECT COUNT(*) as count FROM organizations WHERE slug LIKE ?")
        .bind(format!("{}%", slug))
        .fetch_one(pool)
        .await?;
    let final_slug = if existing.count == 0 {
        slug.clone()
    } else {
        format!("{}-{}", slug, existing.count)
    };

    // Get admin user to be temporary owner
    #[derive(sqlx::FromRow)]
    struct AdminRow {
        #[sqlx(try_from = "Vec<u8>")]
        id: Uuid,
    }
    let admin = sqlx::query_as::<_, AdminRow>(
        "SELECT id FROM users WHERE is_admin = 1 ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::InternalError("No admin user found".into()))?;

    let org_id = Uuid::new_v4();
    let created_by_org_id = person.organization_id;

    sqlx::query(
        r#"INSERT INTO organizations (id, name, slug, owner_id, created_by_org_id)
           VALUES (?, ?, ?, ?, ?)"#,
    )
    .bind(org_id.as_bytes().as_slice())
    .bind(&company_name)
    .bind(&final_slug)
    .bind(admin.id.as_bytes().as_slice())
    .bind(created_by_org_id.as_ref().map(|u| u.as_bytes().to_vec()))
    .execute(pool)
    .await?;

    // Link person → company org
    sqlx::query("UPDATE persons SET company_org_id = ?, updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(org_id.as_bytes().as_slice())
        .bind(person_id.as_bytes().as_slice())
        .execute(pool)
        .await?;

    Ok(Json(ApiResponse::success(ProvisionOrgResponse {
        org_id: org_id.to_string(),
        org_name: company_name,
        slug: final_slug,
    })))
}

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
        // Person notes
        .route(
            "/persons/{id}/notes",
            get(list_person_notes).post(create_person_note),
        )
        .route(
            "/person-notes/{id}",
            patch(update_person_note).delete(delete_person_note),
        )
        // Org provisioning
        .route("/persons/{id}/provision-org", post(provision_org))
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
