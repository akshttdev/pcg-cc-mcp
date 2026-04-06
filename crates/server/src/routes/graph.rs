//! Business Entity Graph — CRM topology visualization endpoints.
//!
//! Uses `entity_graph_nodes` and `entity_graph_edges` tables to represent
//! relationships between companies, contacts, proposals, orgs, and projects.

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use db::models::entity_graph::{company_subgraph, sync_company_graph, EntitySubgraph};
use deployment::Deployment;
use serde::Serialize;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub message: String,
}

/// GET /api/graph/company/:id
/// Returns a 1-hop subgraph centred on a company entity.
pub async fn get_company_graph(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<ApiResponse<EntitySubgraph>>, ApiError> {
    let pool = &d.db().pool;
    let subgraph = company_subgraph(pool, company_id).await?;
    Ok(Json(ApiResponse::success(subgraph)))
}

/// POST /api/graph/sync/company/:id
/// (Re)creates entity graph nodes+edges from live company data.
pub async fn sync_company_graph_handler(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<ApiResponse<SyncResponse>>, ApiError> {
    let pool = &d.db().pool;

    // Fetch company
    #[derive(sqlx::FromRow)]
    struct CompanyRow {
        id: Uuid,
        name: String,
        organization_id: Option<Uuid>,
    }
    let row: Option<CompanyRow> =
        sqlx::query_as("SELECT id, name, organization_id FROM companies WHERE id = ?")
            .bind(company_id)
            .fetch_optional(pool)
            .await?;

    let company = row.ok_or_else(|| ApiError::NotFound("Company not found".into()))?;

    sync_company_graph(pool, company.id, &company.name, company.organization_id).await?;

    Ok(Json(ApiResponse::success(SyncResponse {
        message: format!("Graph synced for company {}", company_id),
    })))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/graph/company/{id}", get(get_company_graph))
        .route("/graph/sync/company/{id}", post(sync_company_graph_handler))
        .with_state(deployment.clone())
}
