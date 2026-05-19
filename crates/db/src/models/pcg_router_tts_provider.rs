use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct PcgRouterTTSProvider {
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
    pub supported_voices: Option<String>,
    pub supported_formats: Option<String>,
    pub supports_streaming: bool,
    pub max_chars_per_request: Option<i64>,
    pub cost_per_1000_chars: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl PcgRouterTTSProvider {
    pub async fn list(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, provider_type, provider_base_url, api_key_env_var,
                    api_key_value, priority, is_enabled, is_local,
                    supported_voices, supported_formats, supports_streaming,
                    max_chars_per_request, cost_per_1000_chars,
                    created_at, updated_at
             FROM pcg_router_tts_providers
             ORDER BY priority ASC, name ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_enabled(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, provider_type, provider_base_url, api_key_env_var,
                    api_key_value, priority, is_enabled, is_local,
                    supported_voices, supported_formats, supports_streaming,
                    max_chars_per_request, cost_per_1000_chars,
                    created_at, updated_at
             FROM pcg_router_tts_providers
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
                    supported_voices, supported_formats, supports_streaming,
                    max_chars_per_request, cost_per_1000_chars,
                    created_at, updated_at
             FROM pcg_router_tts_providers
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
            "elevenlabs" => "https://api.elevenlabs.io",
            "openai" => "https://api.openai.com",
            "azure" => "https://eastus.tts.speech.microsoft.com",
            "chatterbox" => "http://localhost:8765",
            _ => "http://localhost:8765", // Local fallback
        }
    }

    /// Check if this provider supports a specific voice ID.
    pub fn supports_voice(&self, voice_id: &str) -> bool {
        let Some(voices_json) = &self.supported_voices else {
            return true; // No restrictions = supports all
        };
        match serde_json::from_str::<Vec<String>>(voices_json) {
            Ok(voices) => voices.is_empty() || voices.iter().any(|v| v == voice_id),
            Err(_) => true, // Invalid JSON = no restrictions
        }
    }

    /// Check if this provider supports a specific output format.
    pub fn supports_format(&self, format: &str) -> bool {
        let Some(formats_json) = &self.supported_formats else {
            return true;
        };
        match serde_json::from_str::<Vec<String>>(formats_json) {
            Ok(formats) => formats.is_empty() || formats.iter().any(|f| f == format),
            Err(_) => true,
        }
    }
}
