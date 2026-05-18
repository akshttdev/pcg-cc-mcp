//! Internal bridge for the JS Discord/voice bot
//!
//! POST /internal/nora/chat  — call Nora with X-Admin-Key (no JWT required)
//! POST /internal/topsi/chat — call Topsi with X-Admin-Key (no JWT required)

use axum::{http::HeaderMap, routing::post, Json, Router};
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};
use serde::{Deserialize, Serialize};
use topsi::{TopsiRequest, TopsiRequestType, UserContext};
use uuid::Uuid;

use crate::{
    error::ApiError,
    routes::{nora::get_nora_instance, topsi::get_topsi_instance},
    DeploymentImpl,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BotChatRequest {
    message: String,
    session_id: String,
}

#[derive(Serialize)]
struct BotChatResponse {
    response: String,
}

fn validate_admin_key(headers: &HeaderMap) -> Result<(), ApiError> {
    let expected = std::env::var("ADMIN_API_KEY").unwrap_or_default();
    let provided = headers
        .get("x-admin-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if expected.is_empty() || provided != expected {
        return Err(ApiError::Unauthorized("Admin key required".into()));
    }
    Ok(())
}

async fn bot_chat_nora(
    headers: HeaderMap,
    Json(req): Json<BotChatRequest>,
) -> Result<Json<BotChatResponse>, ApiError> {
    validate_admin_key(&headers)?;

    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".into()))?;

    if !nora.is_active().await {
        return Err(ApiError::BadRequest("Nora is not active".into()));
    }

    let nora_req = NoraRequest {
        request_id: Uuid::new_v4().to_string(),
        session_id: req.session_id,
        request_type: NoraRequestType::TextInteraction,
        content: req.message,
        context: None,
        voice_enabled: false,
        priority: RequestPriority::High,
        timestamp: chrono::Utc::now(),
    };

    let resp = nora
        .process_request(nora_req)
        .await
        .map_err(|e| ApiError::InternalError(format!("Nora error: {}", e)))?;

    Ok(Json(BotChatResponse {
        response: resp.content,
    }))
}

async fn bot_chat_topsi(
    headers: HeaderMap,
    Json(req): Json<BotChatRequest>,
) -> Result<Json<BotChatResponse>, ApiError> {
    validate_admin_key(&headers)?;

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".into()))?;

    let user_ctx = UserContext::user("discord-bot").with_session(req.session_id.clone());

    let topsi_req = TopsiRequest::new(TopsiRequestType::Chat {
        message: req.message,
        model_id: None,
    });

    let resp = topsi
        .process_request(topsi_req, &user_ctx, Some(&req.session_id))
        .await
        .map_err(|e| ApiError::InternalError(format!("Topsi error: {}", e)))?;

    Ok(Json(BotChatResponse {
        response: resp.message,
    }))
}

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/internal/nora/chat", post(bot_chat_nora))
        .route("/internal/topsi/chat", post(bot_chat_topsi))
}
