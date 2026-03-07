use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
    Json, Router,
};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};
use db::models::company::{Company, CreateCompany, UpdateCompany};
use db::models::proposal::Proposal;
use db::models::person_association::{CompanyContactMethod, CreateCompanyContactMethod};

#[derive(Debug, Deserialize)]
pub struct ListCompaniesQuery {
    pub created_by_org_id: Option<Uuid>,
    pub has_platform_org: Option<bool>,
    pub limit: Option<i64>,
}

/// GET /companies — list companies (pipeline knowledge graph entities)
async fn list_companies(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<ListCompaniesQuery>,
) -> Result<Json<ApiResponse<Vec<Company>>>, ApiError> {
    let pool = &deployment.db().pool;
    let companies = Company::list(pool, q.created_by_org_id, q.has_platform_org, q.limit).await?;
    Ok(Json(ApiResponse::success(companies)))
}

/// POST /companies — create company
async fn create_company(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateCompany>,
) -> Result<Json<ApiResponse<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let company = Company::create(pool, body).await?;
    Ok(Json(ApiResponse::success(company)))
}

/// GET /companies/:id — get company
async fn get_company(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let company = Company::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;
    Ok(Json(ApiResponse::success(company)))
}

/// PATCH /companies/:id — update company
async fn update_company(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateCompany>,
) -> Result<Json<ApiResponse<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let company = Company::update(pool, id, body)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;
    Ok(Json(ApiResponse::success(company)))
}

/// DELETE /companies/:id
async fn delete_company(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = Company::delete(pool, id).await?;
    if !deleted {
        return Err(ApiError::NotFound("Company not found".into()));
    }
    Ok(Json(ApiResponse::success(())))
}

#[derive(Debug, Deserialize)]
pub struct ListCompanyProposalsQuery {
    pub limit: Option<i64>,
}

/// GET /companies/:id/proposals — all pipeline proposals for this company
async fn list_company_proposals(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Query(q): Query<ListCompanyProposalsQuery>,
) -> Result<Json<ApiResponse<Vec<Proposal>>>, ApiError> {
    let pool = &deployment.db().pool;
    let proposals = Proposal::list_by_company(pool, id, q.limit).await?;
    Ok(Json(ApiResponse::success(proposals)))
}

/// GET /companies/:id/persons — persons (contacts) at this company (via junction table)
async fn list_company_persons(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<db::models::person::Person>>>, ApiError> {
    let pool = &deployment.db().pool;
    let persons = sqlx::query_as::<_, db::models::person::Person>(
        r#"SELECT p.* FROM persons p
           JOIN person_company_roles pcr ON pcr.person_id = p.id
           WHERE pcr.company_id = ?
           ORDER BY pcr.is_primary DESC, p.full_name ASC"#,
    )
    .bind(id.as_bytes().as_slice())
    .fetch_all(pool)
    .await?;
    Ok(Json(ApiResponse::success(persons)))
}

// ---------------------------------------------------------------------------
// Company contact methods
// ---------------------------------------------------------------------------

/// GET /companies/:id/contact-methods
async fn list_contact_methods(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<CompanyContactMethod>>>, ApiError> {
    let pool = &deployment.db().pool;
    let methods = CompanyContactMethod::list_for_company(pool, id).await?;
    Ok(Json(ApiResponse::success(methods)))
}

/// POST /companies/:id/contact-methods
async fn add_contact_method(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateCompanyContactMethod>,
) -> Result<Json<ApiResponse<CompanyContactMethod>>, ApiError> {
    let pool = &deployment.db().pool;
    let method = CompanyContactMethod::create(pool, id, data).await?;
    Ok(Json(ApiResponse::success(method)))
}

/// DELETE /companies/:id/contact-methods/:method_id
async fn delete_contact_method(
    State(deployment): State<DeploymentImpl>,
    Path((_id, method_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = CompanyContactMethod::delete(pool, method_id).await?;
    if !deleted {
        return Err(ApiError::NotFound("Contact method not found".into()));
    }
    Ok(Json(ApiResponse::success(())))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/companies", get(list_companies).post(create_company))
        .route(
            "/companies/{id}",
            get(get_company).patch(update_company).delete(delete_company),
        )
        .route("/companies/{id}/proposals", get(list_company_proposals))
        .route("/companies/{id}/persons", get(list_company_persons))
        .route(
            "/companies/{id}/contact-methods",
            get(list_contact_methods).post(add_contact_method),
        )
        .route(
            "/companies/{id}/contact-methods/{method_id}",
            delete(delete_contact_method),
        )
}
