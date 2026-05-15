//! PCG Router — sovereign OpenRouter-compatible LLM routing layer.
//!
//! Routes model requests to the appropriate provider based on priority, cost,
//! and availability. Exposes an OpenRouter/OpenAI-compatible `/chat/completions`
//! endpoint so any code that talks to OpenRouter can be pointed here instead.
//!
//! Endpoints:
//!   GET  /pcg-router/models            — list all registered models
//!   POST /pcg-router/v1/chat/completions — route a chat completion request
//!   POST /pcg-router/models            — register a new model
//!   PATCH /pcg-router/models/:id       — update a model (priority, enabled, etc.)
//!   DELETE /pcg-router/models/:id      — remove a model

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use db::models::pcg_router_model::PcgRouterModel;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ts_rs::TS;
use uuid::Uuid;

use crate::DeploymentImpl;

// ── Request / Response types ──────────────────────────────────────────────────

/// OpenAI-compatible chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value,
}

/// OpenAI-compatible chat completion request
#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub stream: Option<bool>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

/// Register or update a model
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertRouterModel {
    pub name: String,
    pub model_id: String,
    pub provider: String,
    pub provider_base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub priority: Option<i64>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_tools: Option<bool>,
    pub supports_vision: Option<bool>,
    pub cost_per_million_input: Option<i64>,
    pub cost_per_million_output: Option<i64>,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct PatchRouterModel {
    pub name: Option<String>,
    pub priority: Option<i64>,
    pub is_enabled: Option<bool>,
    pub provider_base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub api_key_value: Option<String>,
}

/// Set API key for a provider (updates all models of that provider)
#[derive(Debug, Deserialize)]
pub struct SetProviderKeyRequest {
    pub provider: String,
    pub api_key: String,
}

/// Provider key status (returned by GET /pcg-router/provider-keys)
#[derive(Debug, Serialize)]
pub struct ProviderKeyStatus {
    pub provider: String,
    pub has_key: bool,
    pub model_count: usize,
    pub enabled_count: usize,
    pub env_var: Option<String>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn list_models(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let models = PcgRouterModel::list(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(models))
}

async fn create_model(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<UpsertRouterModel>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT OR IGNORE INTO pcg_router_models
         (id, name, model_id, provider, provider_base_url, api_key_env_var,
          priority, context_window, max_output_tokens,
          supports_tools, supports_vision,
          cost_per_million_input, cost_per_million_output, is_enabled)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&body.name)
    .bind(&body.model_id)
    .bind(&body.provider)
    .bind(&body.provider_base_url)
    .bind(&body.api_key_env_var)
    .bind(body.priority.unwrap_or(10))
    .bind(body.context_window)
    .bind(body.max_output_tokens)
    .bind(body.supports_tools.unwrap_or(true))
    .bind(body.supports_vision.unwrap_or(false))
    .bind(body.cost_per_million_input.unwrap_or(0))
    .bind(body.cost_per_million_output.unwrap_or(0))
    .bind(body.is_enabled.unwrap_or(true))
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let created: PcgRouterModel = sqlx::query_as(
        "SELECT id, name, model_id, provider, provider_base_url, api_key_env_var,
                api_key_value, priority, context_window, max_output_tokens,
                supports_tools, supports_vision,
                cost_per_million_input, cost_per_million_output,
                is_enabled, created_at, updated_at
         FROM pcg_router_models WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, Json(created)))
}

async fn patch_model(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchRouterModel>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;

    sqlx::query(
        "UPDATE pcg_router_models
         SET name              = COALESCE(?, name),
             priority          = COALESCE(?, priority),
             is_enabled        = COALESCE(?, is_enabled),
             provider_base_url = COALESCE(?, provider_base_url),
             api_key_env_var   = COALESCE(?, api_key_env_var),
             api_key_value     = COALESCE(?, api_key_value),
             updated_at        = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(&body.name)
    .bind(body.priority)
    .bind(body.is_enabled)
    .bind(&body.provider_base_url)
    .bind(&body.api_key_env_var)
    .bind(&body.api_key_value)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated: PcgRouterModel = sqlx::query_as(
        "SELECT id, name, model_id, provider, provider_base_url, api_key_env_var,
                api_key_value, priority, context_window, max_output_tokens,
                supports_tools, supports_vision,
                cost_per_million_input, cost_per_million_output,
                is_enabled, created_at, updated_at
         FROM pcg_router_models WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(updated))
}

async fn delete_model(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM pcg_router_models WHERE id = ?")
        .bind(id)
        .execute(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Model not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ── Shared routing logic (used by both HTTP endpoint and workflow engine) ────

/// Metadata about a routed completion request.
#[derive(Debug, Clone, Serialize)]
pub struct RoutingMetadata {
    pub model_used: String,
    pub provider: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub estimated_cost_micros: Option<i64>,
}

/// Route a chat completion through the PCG Router model registry.
///
/// This is the core routing function shared by the HTTP endpoint and internal
/// callers like the workflow engine. Returns the OpenAI-format response and
/// routing metadata, or an error if all models fail.
pub async fn route_completion(
    pool: &sqlx::SqlitePool,
    messages: Vec<ChatMessage>,
    model_hint: Option<&str>,
    max_tokens: Option<i64>,
    temperature: Option<f64>,
) -> Result<(Value, RoutingMetadata), anyhow::Error> {
    // Resolve candidate models: prefer requested model, fall back by priority
    let candidates = if let Some(model_id) = model_hint {
        let mut specific = PcgRouterModel::get_by_model_id(pool, model_id)
            .await?
            .map(|m| vec![m])
            .unwrap_or_default();
        if specific.is_empty() {
            specific = PcgRouterModel::list_enabled(pool).await?;
        }
        specific
    } else {
        PcgRouterModel::list_enabled(pool).await?
    };

    if candidates.is_empty() {
        anyhow::bail!("No enabled models in PCG Router");
    }

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()?;

    let request = ChatCompletionRequest {
        model: model_hint.map(|s| s.to_string()),
        messages,
        temperature,
        max_tokens,
        stream: Some(false),
        extra: Default::default(),
    };

    for model in &candidates {
        let Some(api_key) = model.resolve_api_key() else {
            tracing::warn!(
                "[PCG_ROUTER] No API key for model '{}', skipping",
                model.name
            );
            continue;
        };

        let result = forward_to_provider(&http, model, &request, &api_key).await;
        match result {
            Ok(resp) => {
                tracing::info!(
                    "[PCG_ROUTER] Routed to '{}' ({})",
                    model.name,
                    model.provider
                );

                // Extract token usage from response
                let input_tokens = resp["usage"]["prompt_tokens"].as_i64();
                let output_tokens = resp["usage"]["completion_tokens"].as_i64();

                // Estimate cost in microdollars (millionths of a dollar)
                let estimated_cost_micros = match (input_tokens, output_tokens) {
                    (Some(inp), Some(out)) => {
                        let input_cost =
                            (inp as f64 / 1_000_000.0) * model.cost_per_million_input as f64;
                        let output_cost =
                            (out as f64 / 1_000_000.0) * model.cost_per_million_output as f64;
                        Some(((input_cost + output_cost) * 1_000_000.0) as i64)
                    }
                    _ => None,
                };

                let metadata = RoutingMetadata {
                    model_used: model.model_id.clone(),
                    provider: model.provider.clone(),
                    input_tokens,
                    output_tokens,
                    estimated_cost_micros,
                };

                return Ok((resp, metadata));
            }
            Err(e) => {
                tracing::warn!(
                    "[PCG_ROUTER] '{}' failed: {}, trying next model",
                    model.name,
                    e
                );
            }
        }
    }

    anyhow::bail!("All PCG Router models failed")
}

/// POST /pcg-router/v1/chat/completions
///
/// OpenRouter-compatible endpoint. Thin wrapper around `route_completion`.
async fn chat_completions(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<ChatCompletionRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;

    let (resp, _metadata) = route_completion(
        pool,
        body.messages,
        body.model.as_deref(),
        body.max_tokens,
        body.temperature,
    )
    .await
    .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(resp))
}

async fn forward_to_provider(
    http: &reqwest::Client,
    model: &PcgRouterModel,
    body: &ChatCompletionRequest,
    api_key: &str,
) -> anyhow::Result<Value> {
    match model.provider.as_str() {
        "anthropic" => forward_to_anthropic(http, model, body, api_key).await,
        "openai" | "openrouter" | "mistral" | "xai" | "deepseek" | "groq" | "cohere" | "qwen" => {
            forward_openai_compat(http, model, body, api_key).await
        }
        "gemini" => forward_to_gemini(http, model, body, api_key).await,
        other => anyhow::bail!("Unknown provider: {}", other),
    }
}

async fn forward_to_anthropic(
    http: &reqwest::Client,
    model: &PcgRouterModel,
    body: &ChatCompletionRequest,
    api_key: &str,
) -> anyhow::Result<Value> {
    // Convert OpenAI messages to Anthropic format
    let mut system_parts: Vec<String> = vec![];
    let mut messages: Vec<Value> = vec![];

    for msg in &body.messages {
        if msg.role == "system" {
            let text = match &msg.content {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            system_parts.push(text);
        } else {
            messages.push(json!({ "role": msg.role, "content": msg.content }));
        }
    }

    let mut payload = json!({
        "model": model.model_id,
        "max_tokens": body.max_tokens.unwrap_or(model.max_output_tokens.unwrap_or(4096)),
        "messages": messages,
    });
    if !system_parts.is_empty() {
        payload["system"] = json!(system_parts.join("\n\n"));
    }
    if let Some(temp) = body.temperature {
        payload["temperature"] = json!(temp);
    }

    let url = format!("{}/v1/messages", model.base_url());
    let resp = http
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Anthropic {}: {}", status, text);
    }

    let anthropic_resp: Value = resp.json().await?;

    // Normalise to OpenAI format
    let content = anthropic_resp["content"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|c| c["text"].as_str())
        .unwrap_or("")
        .to_string();

    Ok(json!({
        "id": anthropic_resp["id"],
        "object": "chat.completion",
        "model": model.model_id,
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": content },
            "finish_reason": anthropic_resp["stop_reason"]
        }],
        "usage": {
            "prompt_tokens": anthropic_resp["usage"]["input_tokens"],
            "completion_tokens": anthropic_resp["usage"]["output_tokens"]
        }
    }))
}

async fn forward_openai_compat(
    http: &reqwest::Client,
    model: &PcgRouterModel,
    body: &ChatCompletionRequest,
    api_key: &str,
) -> anyhow::Result<Value> {
    let payload = json!({
        "model": model.model_id,
        "messages": body.messages,
        "temperature": body.temperature,
        "max_tokens": body.max_tokens.unwrap_or(model.max_output_tokens.unwrap_or(4096)),
        "stream": false,
    });

    let url = format!("{}/v1/chat/completions", model.base_url());
    let resp = http
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Provider {}: {}", status, text);
    }

    Ok(resp.json().await?)
}

/// Gemini OpenAI-compatible endpoint — same as openai_compat but the path
/// is `/chat/completions` (no `/v1` prefix) because Google's compat layer
/// already embeds the version in the base URL.
async fn forward_to_gemini(
    http: &reqwest::Client,
    model: &PcgRouterModel,
    body: &ChatCompletionRequest,
    api_key: &str,
) -> anyhow::Result<Value> {
    let payload = json!({
        "model": model.model_id,
        "messages": body.messages,
        "temperature": body.temperature,
        "max_tokens": body.max_tokens.unwrap_or(model.max_output_tokens.unwrap_or(4096)),
        "stream": false,
    });

    let url = format!("{}/chat/completions", model.base_url());
    let resp = http
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Gemini {}: {}", status, text);
    }

    Ok(resp.json().await?)
}

// ── Provider key management ──────────────────────────────────────────────────

/// GET /pcg-router/provider-keys — list all providers and their key status
async fn list_provider_keys(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let models = PcgRouterModel::list(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Group by provider
    let mut providers: std::collections::BTreeMap<String, ProviderKeyStatus> =
        std::collections::BTreeMap::new();

    for model in &models {
        let entry = providers
            .entry(model.provider.clone())
            .or_insert_with(|| ProviderKeyStatus {
                provider: model.provider.clone(),
                has_key: false,
                model_count: 0,
                enabled_count: 0,
                env_var: model.api_key_env_var.clone(),
            });
        entry.model_count += 1;
        if model.is_enabled {
            entry.enabled_count += 1;
        }
        // Check if any model in this provider has a resolvable key
        if model.resolve_api_key().is_some() {
            entry.has_key = true;
        }
    }

    let statuses: Vec<ProviderKeyStatus> = providers.into_values().collect();
    Ok(Json(statuses))
}

/// POST /pcg-router/provider-keys — set API key for all models of a provider
async fn set_provider_key(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<SetProviderKeyRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;

    // If key is empty, clear it (set to NULL)
    let key_value: Option<&str> = if body.api_key.is_empty() {
        None
    } else {
        Some(&body.api_key)
    };

    let result = sqlx::query(
        "UPDATE pcg_router_models
         SET api_key_value = ?,
             updated_at = datetime('now','subsec')
         WHERE provider = ?",
    )
    .bind(key_value)
    .bind(&body.provider)
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(json!({
        "provider": body.provider,
        "models_updated": result.rows_affected(),
        "has_key": key_value.is_some(),
    })))
}

/// DELETE /pcg-router/provider-keys/:provider — clear API key for a provider
async fn delete_provider_key(
    State(deployment): State<DeploymentImpl>,
    Path(provider): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;

    sqlx::query(
        "UPDATE pcg_router_models
         SET api_key_value = NULL,
             updated_at = datetime('now','subsec')
         WHERE provider = ?",
    )
    .bind(&provider)
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/pcg-router/models", get(list_models).post(create_model))
        .route(
            "/pcg-router/models/{id}",
            patch(patch_model).delete(delete_model),
        )
        .route(
            "/pcg-router/provider-keys",
            get(list_provider_keys).post(set_provider_key),
        )
        .route(
            "/pcg-router/provider-keys/{provider}",
            delete(delete_provider_key),
        )
        .route("/pcg-router/v1/chat/completions", post(chat_completions))
}
