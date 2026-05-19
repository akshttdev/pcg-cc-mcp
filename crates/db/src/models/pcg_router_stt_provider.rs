use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct PcgRouterSTTProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String,
    pub provider_base_url: Option<String>,
    pub api_key_env_var: Option<String>,
    #[serde(skip_serializing)]
    pub api_key_value: Option<String>,
    pub priority: i64,
    pub is_enabled: bool,
    pub is_local: bool,
    pub supported_languages: Option<String>,
    pub supports_streaming: bool,
    pub supports_word_timestamps: bool,
    pub max_audio_duration_sec: Option<i64>,
    pub cost_per_minute: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PcgRouterSTTProvider {
    pub async fn list(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, provider_type, provider_base_url, api_key_env_var,
                    api_key_value, priority, is_enabled, is_local,
                    supported_languages, supports_streaming, supports_word_timestamps,
                    max_audio_duration_sec, cost_per_minute,
                    created_at, updated_at
             FROM pcg_router_stt_providers
             ORDER BY priority ASC, name ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_enabled(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, provider_type, provider_base_url, api_key_env_var,
                    api_key_value, priority, is_enabled, is_local,
                    supported_languages, supports_streaming, supports_word_timestamps,
                    max_audio_duration_sec, cost_per_minute,
                    created_at, updated_at
             FROM pcg_router_stt_providers
             WHERE is_enabled = 1
             ORDER BY priority ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get_by_provider_type(
        pool: &SqlitePool,
        provider_type: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "SELECT id, name, provider_type, provider_base_url, api_key_env_var,
                    api_key_value, priority, is_enabled, is_local,
                    supported_languages, supports_streaming, supports_word_timestamps,
                    max_audio_duration_sec, cost_per_minute,
                    created_at, updated_at
             FROM pcg_router_stt_providers
             WHERE provider_type = ? AND is_enabled = 1
             LIMIT 1",
        )
        .bind(provider_type)
        .fetch_optional(pool)
        .await
    }

    /// Resolve the API key: prefer stored value, then fall back to env var lookup.
    pub fn resolve_api_key(&self) -> Option<String> {
        // 1. Check for a directly stored API key value
        if let Some(ref key) = self.api_key_value {
            if !key.is_empty() {
                return Some(key.clone());
            }
        }
        // 2. Fall back to environment variable lookup
        self.api_key_env_var
            .as_deref()
            .and_then(|var| std::env::var(var).ok())
    }

    /// Base URL for the provider API.
    pub fn base_url(&self) -> &str {
        if let Some(url) = self.provider_base_url.as_deref() {
            return url;
        }
        match self.provider_type.as_str() {
            "whisper" => "https://api.openai.com",
            "azure" => "https://eastus.stt.speech.microsoft.com",
            "google" => "https://speech.googleapis.com",
            "local_whisper" => "http://localhost:8766",
            _ => "http://localhost:8766", // Local fallback
        }
    }

    /// Check if this provider supports a specific language.
    pub fn supports_language(&self, language: &str) -> bool {
        let Some(langs_json) = &self.supported_languages else {
            return true; // No restrictions = supports all
        };
        match serde_json::from_str::<Vec<String>>(langs_json) {
            Ok(langs) => langs.is_empty() || langs.iter().any(|l| l == language),
            Err(_) => true, // Invalid JSON = no restrictions
        }
    }
}
