use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct PcgRouterModel {
    pub id: Uuid,
    pub name: String,
    pub model_id: String,
    pub provider: String,
    pub provider_base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    pub priority: i64,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub cost_per_million_input: i64,
    pub cost_per_million_output: i64,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PcgRouterModel {
    pub async fn list(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, model_id, provider, provider_base_url, api_key_env_var,
                    priority, context_window, max_output_tokens,
                    supports_tools, supports_vision,
                    cost_per_million_input, cost_per_million_output,
                    is_enabled, created_at, updated_at
             FROM pcg_router_models
             ORDER BY priority ASC, name ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_enabled(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, model_id, provider, provider_base_url, api_key_env_var,
                    priority, context_window, max_output_tokens,
                    supports_tools, supports_vision,
                    cost_per_million_input, cost_per_million_output,
                    is_enabled, created_at, updated_at
             FROM pcg_router_models
             WHERE is_enabled = 1
             ORDER BY priority ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get_by_model_id(pool: &SqlitePool, model_id: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "SELECT id, name, model_id, provider, provider_base_url, api_key_env_var,
                    priority, context_window, max_output_tokens,
                    supports_tools, supports_vision,
                    cost_per_million_input, cost_per_million_output,
                    is_enabled, created_at, updated_at
             FROM pcg_router_models
             WHERE model_id = ? AND is_enabled = 1
             LIMIT 1",
        )
        .bind(model_id)
        .fetch_optional(pool)
        .await
    }

    /// Resolve the API key from the environment variable specified on the model.
    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key_env_var.as_deref().and_then(|var| std::env::var(var).ok())
    }

    /// Base URL for the provider API.
    pub fn base_url(&self) -> &str {
        if let Some(url) = self.provider_base_url.as_deref() {
            return url;
        }
        match self.provider.as_str() {
            "anthropic" => "https://api.anthropic.com",
            "openai"    => "https://api.openai.com",
            "openrouter"=> "https://openrouter.ai/api",
            "gemini"    => "https://generativelanguage.googleapis.com/v1beta/openai",
            "mistral"   => "https://api.mistral.ai",
            "xai"       => "https://api.x.ai",
            "deepseek"  => "https://api.deepseek.com",
            "groq"      => "https://api.groq.com/openai",
            "cohere"    => "https://api.cohere.com/compatibility",
            // Qwen via Alibaba DashScope OpenAI-compat layer
            // base_url + /v1/chat/completions = correct endpoint
            "qwen"      => "https://dashscope.aliyuncs.com/compatible-mode",
            _           => "http://localhost:11434", // ollama default
        }
    }
}
