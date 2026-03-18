//! APN Marketplace — Service listings, consumer subscriptions, and service gateway
//!
//! Architecture:
//!   - Providers (APN nodes or orgs) register service listings
//!   - Consumers (e.g. Jungleverse) subscribe and get an API key
//!   - The gateway endpoint routes requests to the best available provider via NATS
//!   - VIBE is charged to the consumer subscription and credited to the provider node
//!
//! Public endpoints (API key auth):
//!   POST /marketplace/gateway           — route a service request (Jungleverse calls this)
//!   GET  /marketplace/listings          — browse available services
//!   GET  /marketplace/stats             — network-wide gateway stats
//!
//! Protected endpoints (JWT auth — PCG dashboard):
//!   POST /marketplace/listings          — provider registers a service
//!   PATCH /marketplace/listings/:id     — provider updates listing
//!   DELETE /marketplace/listings/:id    — provider deactivates listing
//!   POST /marketplace/subscriptions     — create a consumer subscription + issue API key
//!   GET  /marketplace/subscriptions     — list subscriptions (admin) or project subs
//!   GET  /marketplace/subscriptions/:id/requests — usage history
//!   POST /marketplace/subscriptions/:id/topup    — add VIBE budget

use std::time::Instant;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{delete, get, patch, post},
    Extension, Json, Router,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use db::models::{
    gateway_request::GatewayRequest,
    marketplace_listing::{CreateListing, MarketplaceListing, UpdateListing},
    marketplace_subscription::{
        CreateSubscription, MarketplaceSubscription, SubscriptionCreated, SubscriptionView,
    },
    peer_reward::{CreatePeerReward, PeerReward, RewardType},
};
use futures::StreamExt;
use utils::response::ApiResponse;

use crate::{error::ApiError, middleware::access_control::AccessContext, DeploymentImpl};

// ─── Gateway Request / Response ──────────────────────────────────────────────

/// Request body Jungleverse (or any consumer) sends to the gateway
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct GatewayServiceRequest {
    /// Service type to route to: 'compute', 'web-hosting', 'data-api', 'proxy', etc.
    pub service_type: String,
    /// Optional: pin to a specific listing (provider) by ID
    pub listing_id: Option<Uuid>,
    /// The actual payload to forward to the provider (service-specific JSON)
    pub payload: serde_json::Value,
    /// Caller's HTTP method (for proxy services)
    pub method: Option<String>,
    /// Caller's path (for proxy/hosting services)
    pub path: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct GatewayServiceResponse {
    pub request_id: Uuid,
    pub provider_node_id: String,
    pub status: String,
    pub response: serde_json::Value,
    pub vibe_charged: f64,
    pub response_ms: i64,
}

// ─── Query params ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListingsQuery {
    pub service_type: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct SubQuery {
    pub project_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct TopUpRequest {
    pub vibe_amount: f64,
}

// ─── Router ──────────────────────────────────────────────────────────────────

pub fn public_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/marketplace/listings", get(list_listings))
        .route("/marketplace/gateway", post(gateway))
        .route("/marketplace/stats", get(gateway_stats))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/marketplace/listings", post(create_listing))
        .route("/marketplace/listings/{id}", patch(update_listing))
        .route("/marketplace/listings/{id}", delete(deactivate_listing))
        .route("/marketplace/subscriptions", post(create_subscription))
        .route("/marketplace/subscriptions", get(list_subscriptions))
        .route("/marketplace/subscriptions/{id}", get(get_subscription))
        .route("/marketplace/subscriptions/{id}/topup", post(topup_subscription))
        .route(
            "/marketplace/subscriptions/{id}/requests",
            get(list_requests),
        )
}

// ─── Public: Browse listings ─────────────────────────────────────────────────

async fn list_listings(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<ListingsQuery>,
) -> Result<Json<ApiResponse<Vec<MarketplaceListing>>>, ApiError> {
    let pool = &deployment.db().pool;
    let listings =
        MarketplaceListing::list_active(pool, q.service_type.as_deref(), q.limit).await?;
    Ok(Json(ApiResponse::success(listings)))
}

// ─── Public: Gateway stats ───────────────────────────────────────────────────

async fn gateway_stats(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<db::models::gateway_request::GatewayStats>>, ApiError> {
    let pool = &deployment.db().pool;
    let stats = GatewayRequest::get_stats(pool).await?;
    Ok(Json(ApiResponse::success(stats)))
}

// ─── Public: Gateway (API key auth) ─────────────────────────────────────────

async fn gateway(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    Json(body): Json<GatewayServiceRequest>,
) -> Result<Json<ApiResponse<GatewayServiceResponse>>, ApiError> {
    // 1. Authenticate via API key in Authorization header
    let api_key = extract_api_key(&headers).ok_or_else(|| {
        ApiError::Unauthorized("Missing Authorization: Bearer <api_key>".to_string())
    })?;

    let pool = &deployment.db().pool;

    let subscription = MarketplaceSubscription::find_by_api_key(pool, &api_key)
        .await?
        .ok_or_else(|| ApiError::Unauthorized("Invalid or expired API key".to_string()))?;

    // 2. Check status and balance
    if !subscription.is_active() {
        return Err(ApiError::BadRequest(format!(
            "Subscription is {}",
            subscription.status
        )));
    }

    // 3. Find best available provider
    let listing = if let Some(lid) = body.listing_id {
        MarketplaceListing::find_by_id(pool, lid)
            .await?
            .ok_or_else(|| ApiError::NotFound("Listing not found".to_string()))?
    } else {
        MarketplaceListing::find_best_for_service(pool, &body.service_type)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "No active providers for service type '{}'",
                    body.service_type
                ))
            })?
    };

    let vibe_cost = listing.price_vibe.max(listing.min_vibe);

    if !crate::helpers::vibe_check::is_vibe_bypass_active(pool).await && !subscription.has_balance(vibe_cost) {
        return Err(ApiError::PaymentRequired(format!(
            "Insufficient VIBE balance. Need {:.4}, have {:.4}",
            vibe_cost,
            subscription.vibe_balance()
        )));
    }

    let request_bytes = serde_json::to_vec(&body.payload).unwrap_or_default().len() as i64;

    // 4. Log the request
    let gateway_req = GatewayRequest::create(
        pool,
        subscription.id,
        Some(listing.id),
        &body.service_type,
        body.method.as_deref().or(Some("POST")),
        body.path.as_deref(),
        request_bytes,
    )
    .await?;

    GatewayRequest::mark_routing(pool, gateway_req.id).await?;

    let start = Instant::now();

    // 5. Route request to provider
    let route_result = route_to_provider(&deployment, &listing, &body).await;

    let elapsed_ms = start.elapsed().as_millis() as i64;

    match route_result {
        Ok((response_payload, response_bytes)) => {
            // 6. Settle VIBE
            let vibe_reward = vibe_cost * 0.85; // 85% to provider, 15% platform fee

            GatewayRequest::mark_fulfilled(
                pool,
                gateway_req.id,
                &listing.provider_node_id.as_deref().unwrap_or("org_infra"),
                &listing.provider_wallet,
                listing.id,
                response_bytes,
                elapsed_ms,
                vibe_cost,
                vibe_reward,
            )
            .await?;

            // Debit consumer
            MarketplaceSubscription::record_spend(pool, subscription.id, vibe_cost).await?;

            // Credit provider node (if apn_node — records in peer_rewards for on-chain settlement)
            if listing.provider_type == "apn_node" {
                if let Some(ref node_id) = listing.provider_node_id {
                    credit_provider_node(pool, node_id, &listing.provider_wallet, vibe_reward)
                        .await;
                }
            }

            Ok(Json(ApiResponse::success(GatewayServiceResponse {
                request_id: gateway_req.id,
                provider_node_id: listing
                    .provider_node_id
                    .unwrap_or_else(|| "org_infra".to_string()),
                status: "fulfilled".to_string(),
                response: response_payload,
                vibe_charged: vibe_cost,
                response_ms: elapsed_ms,
            })))
        }

        Err(e) => {
            GatewayRequest::mark_failed(pool, gateway_req.id, &e.to_string()).await?;
            Err(ApiError::InternalError(format!("Provider error: {}", e)))
        }
    }
}

/// Route the actual request to the provider
async fn route_to_provider(
    deployment: &DeploymentImpl,
    listing: &MarketplaceListing,
    body: &GatewayServiceRequest,
) -> anyhow::Result<(serde_json::Value, i64)> {
    match listing.provider_type.as_str() {
        // APN node: route via NATS task broadcast and await response
        "apn_node" => route_via_nats(deployment, listing, body).await,
        // Org infrastructure: direct HTTP proxy
        "org_infra" => route_via_http(listing, body).await,
        other => anyhow::bail!("Unknown provider type: {}", other),
    }
}

/// Route via NATS task broadcast to APN nodes
async fn route_via_nats(
    _deployment: &DeploymentImpl,
    listing: &MarketplaceListing,
    body: &GatewayServiceRequest,
) -> anyhow::Result<(serde_json::Value, i64)> {
    let settings = crate::apn_data_service::APNDataServiceConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Config error: {}", e))?;

    let nats_client = async_nats::connect(&settings.nats_url)
        .await
        .map_err(|e| anyhow::anyhow!("NATS connect failed: {}", e))?;

    let task_id = Uuid::new_v4().to_string();
    let subject = format!("apn.service.{}", body.service_type);
    let reply_subject = format!("apn.service.reply.{}", task_id);

    // Subscribe to reply channel before publishing
    let mut reply_sub = nats_client
        .subscribe(reply_subject.clone())
        .await
        .map_err(|e| anyhow::anyhow!("Subscribe failed: {}", e))?;

    // Publish task to the service subject
    let task_payload = serde_json::json!({
        "task_id": task_id,
        "service_type": body.service_type,
        "listing_id": listing.id,
        "reply_to": reply_subject,
        "payload": body.payload,
        "method": body.method,
        "path": body.path,
    });

    nats_client
        .publish(subject, serde_json::to_vec(&task_payload)?.into())
        .await
        .map_err(|e| anyhow::anyhow!("Publish failed: {}", e))?;

    // Wait for response with timeout
    let timeout = tokio::time::Duration::from_secs(30);
    let response = tokio::time::timeout(timeout, reply_sub.next())
        .await
        .map_err(|_| anyhow::anyhow!("Provider timeout after 30s"))?
        .ok_or_else(|| anyhow::anyhow!("No response from provider"))?;

    let response_bytes = response.payload.len() as i64;
    let response_json: serde_json::Value = serde_json::from_slice(&response.payload)
        .unwrap_or(serde_json::json!({"raw": "binary response"}));

    Ok((response_json, response_bytes))
}

/// Route via direct HTTP to org-infra provider endpoint
async fn route_via_http(
    listing: &MarketplaceListing,
    body: &GatewayServiceRequest,
) -> anyhow::Result<(serde_json::Value, i64)> {
    let endpoint = listing
        .endpoint_url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Listing has no endpoint_url"))?;

    let path = body.path.as_deref().unwrap_or("/");
    let url = format!("{}{}", endpoint.trim_end_matches('/'), path);
    let method = body.method.as_deref().unwrap_or("POST");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = match method.to_uppercase().as_str() {
        "GET" => client.get(&url).send().await?,
        "POST" => client.post(&url).json(&body.payload).send().await?,
        "PUT" => client.put(&url).json(&body.payload).send().await?,
        "DELETE" => client.delete(&url).send().await?,
        _ => client.post(&url).json(&body.payload).send().await?,
    };

    let response_bytes = response.content_length().unwrap_or(0) as i64;
    let response_json: serde_json::Value = response.json().await.unwrap_or(serde_json::json!({}));

    Ok((response_json, response_bytes))
}

/// Credit the APN node provider with a service reward (recorded for on-chain settlement)
async fn credit_provider_node(pool: &sqlx::SqlitePool, node_id: &str, wallet: &str, amount: f64) {
    use db::models::peer_node::PeerNode;

    let peer = match PeerNode::find_by_node_id(pool, node_id).await {
        Ok(Some(p)) => p,
        _ => {
            tracing::warn!("Provider node {} not found in DB for reward credit", node_id);
            return;
        }
    };

    let reward_data = CreatePeerReward {
        peer_node_id: peer.id,
        contribution_id: None,
        reward_type: RewardType::Task,
        base_amount: (amount * 100_000_000.0) as i64, // VIBE base units
        multiplier: 1.0,
        description: Some(format!("Service fulfillment reward: {:.6} VIBE", amount)),
        metadata: Some(serde_json::json!({
            "reward_type": "service_fulfillment",
            "wallet": wallet,
        })),
    };

    if let Err(e) = PeerReward::create(pool, reward_data).await {
        tracing::error!("Failed to credit provider node {}: {}", node_id, e);
    }
}

// ─── Protected: Listings CRUD ────────────────────────────────────────────────

async fn create_listing(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Json(body): Json<CreateListing>,
) -> Result<Json<ApiResponse<MarketplaceListing>>, ApiError> {
    let pool = &deployment.db().pool;
    let listing = MarketplaceListing::create(pool, body).await?;
    Ok(Json(ApiResponse::success(listing)))
}

async fn update_listing(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Path(id): Path<String>,
    Json(body): Json<UpdateListing>,
) -> Result<Json<ApiResponse<MarketplaceListing>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    let listing = MarketplaceListing::update(pool, id_uuid, body)
        .await?
        .ok_or_else(|| ApiError::NotFound("Listing not found".to_string()))?;
    Ok(Json(ApiResponse::success(listing)))
}

async fn deactivate_listing(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<bool>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    let deleted = MarketplaceListing::delete(pool, id_uuid).await?;
    Ok(Json(ApiResponse::success(deleted)))
}

// ─── Protected: Subscriptions ────────────────────────────────────────────────

async fn create_subscription(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Json(body): Json<CreateSubscription>,
) -> Result<Json<ApiResponse<SubscriptionCreated>>, ApiError> {
    let pool = &deployment.db().pool;
    let result = MarketplaceSubscription::create(pool, body).await?;
    Ok(Json(ApiResponse::success(result)))
}

async fn list_subscriptions(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Query(q): Query<SubQuery>,
) -> Result<Json<ApiResponse<Vec<SubscriptionView>>>, ApiError> {
    let pool = &deployment.db().pool;
    let subs = if let Some(pid) = q.project_id {
        MarketplaceSubscription::list_for_project(pool, pid).await?
    } else {
        MarketplaceSubscription::list_all(pool, q.limit).await?
    };
    Ok(Json(ApiResponse::success(subs)))
}

async fn get_subscription(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<SubscriptionView>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    let sub = MarketplaceSubscription::find_by_id(pool, id_uuid)
        .await?
        .ok_or_else(|| ApiError::NotFound("Subscription not found".to_string()))?;
    Ok(Json(ApiResponse::success(sub.into())))
}

async fn topup_subscription(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Path(id): Path<String>,
    Json(body): Json<TopUpRequest>,
) -> Result<Json<ApiResponse<SubscriptionView>>, ApiError> {
    if body.vibe_amount <= 0.0 {
        return Err(ApiError::BadRequest(
            "vibe_amount must be positive".to_string(),
        ));
    }
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    MarketplaceSubscription::add_budget(pool, id_uuid, body.vibe_amount).await?;
    let sub = MarketplaceSubscription::find_by_id(pool, id_uuid)
        .await?
        .ok_or_else(|| ApiError::NotFound("Subscription not found".to_string()))?;
    Ok(Json(ApiResponse::success(sub.into())))
}

async fn list_requests(
    State(deployment): State<DeploymentImpl>,
    Extension(_access): Extension<AccessContext>,
    Path(id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<ApiResponse<Vec<GatewayRequest>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id_uuid = Uuid::parse_str(&id).map_err(|_| ApiError::BadRequest("Invalid ID".into()))?;
    let requests = GatewayRequest::list_for_subscription(pool, id_uuid, q.limit).await?;
    Ok(Json(ApiResponse::success(requests)))
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
}
