//! Global Invoice routes — AR (accounts receivable) and AP (accounts payable).
//!
//! Invoices are always priced in USD; a VIBE equivalent is computed and stored.
//! 1 VIBE = $0.01 USD  (VIBE_USD_VALUE in model_pricing.rs)

use axum::{
    Router,
    extract::{Path, Query, State},
    routing::{delete, get, patch, post},
    Json,
};
use db::models::invoice::{CreateInvoice, Invoice, UpdateInvoice};
use deployment::Deployment;
use serde::Deserialize;
use utils::response::ApiResponse;
use uuid::Uuid;
use db::db_uuid::DbUuid;

use crate::{DeploymentImpl, error::ApiError};

const VIBE_PER_USD: f64 = 100.0; // 1 USD = 100 VIBE  (since 1 VIBE = $0.01)

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListInvoicesParams {
    pub invoice_type: Option<String>, // 'ar' | 'ap'
    pub status: Option<String>,
    pub person_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MoveStatusBody {
    pub status: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/invoices
async fn list_invoices(
    State(d): State<DeploymentImpl>,
    Query(p): Query<ListInvoicesParams>,
) -> Result<Json<ApiResponse<Vec<Invoice>>>, ApiError> {
    let pool = &d.db().pool;

    let mut qb = sqlx::QueryBuilder::new("SELECT * FROM invoices WHERE 1=1");
    if let Some(t) = &p.invoice_type {
        qb.push(" AND invoice_type = ").push_bind(t.clone());
    }
    if let Some(s) = &p.status {
        qb.push(" AND status = ").push_bind(s.clone());
    }
    if let Some(pid) = p.person_id {
        qb.push(" AND person_id = ").push_bind(pid);
    }
    if let Some(proj) = p.project_id {
        qb.push(" AND project_id = ").push_bind(proj);
    }
    qb.push(" ORDER BY created_at DESC LIMIT ").push_bind(p.limit.unwrap_or(200));

    let invoices = qb
        .build_query_as::<Invoice>()
        .fetch_all(pool)
        .await?;
    Ok(Json(ApiResponse::success(invoices)))
}

/// POST /api/invoices
async fn create_invoice(
    State(d): State<DeploymentImpl>,
    Json(mut body): Json<CreateInvoice>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    // Auto-compute VIBE from USD if not provided
    if body.amount_vibe.is_none() {
        if let Some(usd) = body.amount_usd {
            if usd > 0.0 {
                body.amount_vibe = Some((usd * VIBE_PER_USD).ceil() as i64);
            }
        }
    }
    let invoice = Invoice::create(&d.db().pool, body).await?;
    Ok(Json(ApiResponse::success(invoice)))
}

/// GET /api/invoices/:id
async fn get_invoice(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let id_uuid = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?.to_uuid();
    Invoice::find_by_id(&d.db().pool, id_uuid)
        .await?
        .map(|i| Json(ApiResponse::success(i)))
        .ok_or_else(|| ApiError::NotFound("Invoice not found".into()))
}

/// PATCH /api/invoices/:id
async fn update_invoice(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(mut body): Json<UpdateInvoice>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    // Auto-recompute VIBE if USD changed
    if let Some(usd) = body.amount_usd {
        if body.amount_vibe.is_none() {
            body.amount_vibe = Some((usd * VIBE_PER_USD).ceil() as i64);
        }
    }
    let id_uuid = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?.to_uuid();
    Invoice::update(&d.db().pool, id_uuid, body)
        .await?
        .map(|i| Json(ApiResponse::success(i)))
        .ok_or_else(|| ApiError::NotFound("Invoice not found".into()))
}

/// PATCH /api/invoices/:id/status
async fn move_invoice_status(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<MoveStatusBody>,
) -> Result<Json<ApiResponse<Invoice>>, ApiError> {
    let pool = &d.db().pool;
    let id_uuid = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?.to_uuid();

    // Set paid_at when marking paid or partial
    let sql = match body.status.as_str() {
        "paid" | "partial" =>
            "UPDATE invoices SET status = ?, paid_at = datetime('now','subsec'), updated_at = datetime('now','subsec') WHERE id = ?",
        _ =>
            "UPDATE invoices SET status = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    };
    sqlx::query(sql)
        .bind(&body.status)
        .bind(id_uuid)
        .execute(pool)
        .await?;

    Invoice::find_by_id(pool, id_uuid)
        .await?
        .map(|i| Json(ApiResponse::success(i)))
        .ok_or_else(|| ApiError::NotFound("Invoice not found".into()))
}

/// DELETE /api/invoices/:id
async fn delete_invoice(
    State(d): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id_uuid = DbUuid::parse(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?.to_uuid();
    let deleted = Invoice::delete(&d.db().pool, id_uuid).await?;
    if deleted {
        Ok(Json(ApiResponse::success(())))
    } else {
        Err(ApiError::NotFound("Invoice not found".into()))
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/invoices", get(list_invoices).post(create_invoice))
        .route(
            "/invoices/{id}",
            get(get_invoice).patch(update_invoice).delete(delete_invoice),
        )
        .route("/invoices/{id}/status", patch(move_invoice_status))
        .with_state(deployment.clone())
}
