use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Nora voice configuration stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NoraVoiceConfig {
    pub id: i64,
    pub config_json: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl NoraVoiceConfig {
    /// Get the singleton configuration
    pub async fn get(db: &sqlx::SqlitePool) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, NoraVoiceConfig>(
            "SELECT id, config_json, created_at, updated_at FROM nora_voice_config WHERE id = 1"
        )
        .fetch_optional(db)
        .await
    }

    /// Save or update the configuration
    pub async fn save(
        db: &sqlx::SqlitePool,
        config_json: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO nora_voice_config (id, config_json) VALUES (1, ?)
             ON CONFLICT(id) DO UPDATE SET config_json = excluded.config_json, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(config_json)
        .execute(db)
        .await?;
        Ok(())
    }

    /// Delete the configuration (reset to default)
    pub async fn delete(db: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM nora_voice_config WHERE id = 1")
            .execute(db)
            .await?;
        Ok(())
    }
}
