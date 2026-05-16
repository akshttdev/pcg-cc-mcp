//! Business Entity Graph — CRM topology visualization endpoints.
//!
//! Backs the `/organizations/:orgId/intelligence/topology` shell (Phase 3)
//! and, eventually, the `/interface` Jarvis surface (Phase 7).
//!
//! Uses `entity_graph_nodes` and `entity_graph_edges` tables. Stable
//! entities (org/company/person/proposal/project) are materialized here;
//! tasks/activities/research are queried live via perspective-specific
//! endpoints added in Phase 4.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Extension, Json, Router,
};
use db::models::entity_graph::{
    company_subgraph, global_subgraph, org_subgraph, subgraph_focused, sync_company_graph,
    sync_global_graph, EntitySubgraph, SyncGlobalStats,
};
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use db::models::entity_graph::{company_subgraph, sync_company_graph, EntitySubgraph};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    error::ApiError,
    graph_access::filter_visible_nodes,
    middleware::access_control::AccessContext,
    node_actions::{ActionHandler, NodeActionRegistry, NodeType},
    DeploymentImpl,
};
use crate::{DeploymentImpl, error::ApiError};
use crate::{error::ApiError, DeploymentImpl};

#[derive(Debug, Serialize)]
pub struct SyncResponse {
    pub message: String,
}

/// GET /api/graph/company/:id — 1-hop subgraph centred on a company.
pub async fn get_company_graph(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<ApiResponse<EntitySubgraph>>, ApiError> {
    let pool = &d.db().pool;
    let subgraph = company_subgraph(pool, company_id).await?;
    Ok(Json(ApiResponse::success(subgraph)))
}

/// POST /api/graph/sync/company/:id — rebuild graph rows for one company.
pub async fn sync_company_graph_handler(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<ApiResponse<SyncResponse>>, ApiError> {
    let pool = &d.db().pool;

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

/// GET /api/graph/global — all materialized nodes + edges, access-filtered.
pub async fn get_global_graph(
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<EntitySubgraph>>, ApiError> {
    access_context.require_active()?;
    let pool = &d.db().pool;

    let subgraph = global_subgraph(pool).await?;
    let filtered = filter_visible_nodes(&access_context, pool, subgraph).await?;

    Ok(Json(ApiResponse::success(filtered)))
}

/// GET /api/graph/org/:org_id — subgraph of one org, requires membership.
pub async fn get_org_graph(
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Path(org_id): Path<String>,
) -> Result<Json<ApiResponse<EntitySubgraph>>, ApiError> {
    let pool = &d.db().pool;
    access_context.require_org_membership(pool, &org_id).await?;

    let org_dbuuid = db::db_uuid::DbUuid::parse(&org_id)
        .map_err(|e| ApiError::BadRequest(format!("invalid org_id: {}", e)))?;
    let subgraph = org_subgraph(pool, &org_dbuuid).await?;

    Ok(Json(ApiResponse::success(subgraph)))
}

#[derive(Debug, Deserialize)]
pub struct SubgraphQuery {
    pub focus_type: String,
    pub focus_id: String,
    #[serde(default = "default_depth")]
    pub depth: u32,
}

fn default_depth() -> u32 {
    2
}

/// GET /api/graph/subgraph — BFS from a focus node, access-filtered.
pub async fn get_focused_subgraph(
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
    Query(q): Query<SubgraphQuery>,
) -> Result<Json<ApiResponse<EntitySubgraph>>, ApiError> {
    access_context.require_active()?;
    let pool = &d.db().pool;

    let node =
        db::models::entity_graph::EntityGraphNode::find_by_ref(pool, &q.focus_type, &q.focus_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("{}:{} not in graph", q.focus_type, q.focus_id))
            })?;

    let raw = subgraph_focused(pool, node.id, q.depth).await?;
    let filtered = filter_visible_nodes(&access_context, pool, raw).await?;

    Ok(Json(ApiResponse::success(filtered)))
}

/// POST /api/graph/sync/global — rebuild entire graph; admin only.
pub async fn sync_global_graph_handler(
    Extension(access_context): Extension<AccessContext>,
    State(d): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<SyncGlobalStats>>, ApiError> {
    access_context.require_admin()?;
    let pool = &d.db().pool;
    let stats = sync_global_graph(pool).await?;
    Ok(Json(ApiResponse::success(stats)))
}

#[derive(Debug, Deserialize)]
pub struct ActionsQuery {
    pub node_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ActionsResponse {
    pub actions: Vec<ActionHandler>,
}

/// GET /api/graph/actions?node_type= — registry entries for a node type,
/// so the UI only offers actions whose handlers actually exist.
pub async fn get_graph_actions(
    Extension(access_context): Extension<AccessContext>,
    Query(q): Query<ActionsQuery>,
) -> Result<Json<ApiResponse<ActionsResponse>>, ApiError> {
    access_context.require_active()?;
    let registry = NodeActionRegistry::new();

    let actions: Vec<ActionHandler> = if let Some(nt_str) = q.node_type.as_deref() {
        let nt = NodeType::parse(nt_str)
            .ok_or_else(|| ApiError::BadRequest(format!("unknown node_type: {}", nt_str)))?;
        registry.actions_for(nt).into_iter().cloned().collect()
    } else {
        // No node_type → return every registered action.
        [NodeType::Company, NodeType::Person, NodeType::Task]
            .iter()
            .flat_map(|nt| registry.actions_for(*nt).into_iter().cloned())
            .collect()
    };

    Ok(Json(ApiResponse::success(ActionsResponse { actions })))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/graph/company/{id}", get(get_company_graph))
        .route("/graph/sync/company/{id}", post(sync_company_graph_handler))
        .route("/graph/global", get(get_global_graph))
        .route("/graph/org/{org_id}", get(get_org_graph))
        .route("/graph/subgraph", get(get_focused_subgraph))
        .route("/graph/sync/global", post(sync_global_graph_handler))
        .route("/graph/actions", get(get_graph_actions))
        .with_state(deployment.clone())
}
