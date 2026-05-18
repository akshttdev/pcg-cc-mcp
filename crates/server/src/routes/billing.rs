//! Billing routes (Stripe).
//!
//! - `POST /billing/checkout` — create a Stripe Checkout session for a plan.
//! - `POST /billing/portal`   — create a Stripe Billing Portal session.
//! - `GET  /billing/subscription?organization_id=...` — current plan + usage.
//! - `POST /webhooks/stripe`  — process Stripe webhook events.
//!
//! All routes degrade cleanly when `STRIPE_SECRET_KEY` / price-id env vars
//! aren't set: the request fails with `NotConfigured` instead of crashing.

use std::str::FromStr;

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use db::{
    db_uuid::DbUuid,
    models::{
        org_subscription::{OrgSubscription, PlanTier},
        user::Organization,
    },
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::billing::{
    create_billing_portal_session, create_checkout_session, ensure_customer, handle_event,
    usage::{usage_status, UsageStatus},
    webhook::verify_signature,
};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ─── Request / response shapes ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub organization_id: Uuid,
    pub plan: String,
    pub success_url: Option<String>,
    pub cancel_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub session_id: String,
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct PortalRequest {
    pub organization_id: Uuid,
    pub return_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PortalResponse {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionDto {
    pub plan: String,
    pub status: String,
    pub seat_limit: i64,
    pub action_limit: i64,
    pub allow_overage: bool,
    pub current_period_start: Option<String>,
    pub current_period_end: Option<String>,
    pub cancel_at_period_end: bool,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub usage: UsageStatus,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

async fn checkout(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CheckoutRequest>,
) -> Result<Json<ApiResponse<CheckoutResponse>>, ApiError> {
    let plan = PlanTier::from_str(&req.plan)
        .map_err(|e| ApiError::BadRequest(format!("invalid plan: {e}")))?;
    if matches!(plan, PlanTier::Free) {
        return Err(ApiError::BadRequest(
            "free plan does not require checkout".into(),
        ));
    }

    let pool = &deployment.db().pool;
    let org_uuid = DbUuid::from_string(req.organization_id.to_string());

    // Make sure we have a subscription row before talking to Stripe so the
    // customer-id round-trip has somewhere to land.
    let sub = OrgSubscription::ensure_free(pool, &org_uuid)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let org = Organization::find_by_id(pool, &req.organization_id.to_string())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("organization not found".into()))?;

    let customer_id = ensure_customer(pool, &sub, &org.name, None)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let success_url = req
        .success_url
        .unwrap_or_else(|| "/billing/success".to_string());
    let cancel_url = req
        .cancel_url
        .unwrap_or_else(|| "/billing/cancel".to_string());

    let session = create_checkout_session(
        &customer_id,
        org_uuid.as_str(),
        plan,
        &success_url,
        &cancel_url,
    )
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(CheckoutResponse {
        session_id: session.id,
        url: session.url,
    })))
}

async fn portal(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<PortalRequest>,
) -> Result<Json<ApiResponse<PortalResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_uuid = DbUuid::from_string(req.organization_id.to_string());

    let sub = OrgSubscription::ensure_free(pool, &org_uuid)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let customer_id = sub.stripe_customer_id.ok_or_else(|| {
        ApiError::BadRequest(
            "organization has no Stripe customer; run /billing/checkout first".into(),
        )
    })?;

    let return_url = req
        .return_url
        .unwrap_or_else(|| "/settings/integrations".to_string());

    let session = create_billing_portal_session(&customer_id, &return_url)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(PortalResponse {
        url: session.url,
    })))
}

async fn get_subscription(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<OrgQuery>,
) -> Result<Json<ApiResponse<SubscriptionDto>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_uuid = DbUuid::from_string(q.organization_id.to_string());
    let sub = OrgSubscription::ensure_free(pool, &org_uuid)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let usage = usage_status(pool, &org_uuid)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(SubscriptionDto {
        plan: sub.plan,
        status: sub.status,
        seat_limit: sub.seat_limit,
        action_limit: sub.action_limit,
        allow_overage: sub.allow_overage == 1,
        current_period_start: sub.current_period_start,
        current_period_end: sub.current_period_end,
        cancel_at_period_end: sub.cancel_at_period_end == 1,
        stripe_customer_id: sub.stripe_customer_id,
        stripe_subscription_id: sub.stripe_subscription_id,
        usage,
    })))
}

async fn stripe_webhook(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .map_err(|_| ApiError::InternalError("STRIPE_WEBHOOK_SECRET not configured".into()))?;
    let sig = headers
        .get("stripe-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::BadRequest("missing Stripe-Signature header".into()))?;

    verify_signature(sig, &body, &secret)
        .map_err(|e| ApiError::BadRequest(format!("signature verification failed: {e}")))?;

    handle_event(&deployment.db().pool, &body)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok(Json(ApiResponse::success(())))
}

// ─── Router ─────────────────────────────────────────────────────────────────

/// Auth-required billing routes (user session expected). Mount inside
/// `protected_routes`.
pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/billing/checkout", post(checkout))
        .route("/billing/portal", post(portal))
        .route("/billing/subscription", get(get_subscription))
}

/// Public Stripe webhook endpoint. Mount in the un-authenticated section
/// next to other webhook handlers — Stripe POSTs without a session cookie,
/// authentication is HMAC-based via `Stripe-Signature`.
pub fn webhook_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/webhooks/stripe", post(stripe_webhook))
}
