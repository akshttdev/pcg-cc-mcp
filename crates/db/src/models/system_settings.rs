use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};
use ts_rs::TS;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct SystemSetting {
    pub key: String,
    pub value: String,
    pub updated_by: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl SystemSetting {
    /// Get a single setting by key
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let row = sqlx::query("SELECT value FROM system_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;
        Ok(row.map(|r| r.get::<String, _>("value")))
    }

    /// Set a setting value (upsert)
    pub async fn set(
        pool: &SqlitePool,
        key: &str,
        value: &str,
        updated_by: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO system_settings (key, value, updated_by, updated_at)
               VALUES (?, ?, ?, datetime('now','subsec'))
               ON CONFLICT(key) DO UPDATE SET
                 value = excluded.value,
                 updated_by = excluded.updated_by,
                 updated_at = excluded.updated_at"#,
        )
        .bind(key)
        .bind(value)
        .bind(updated_by)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Check if VIBE bypass is enabled (only in debug builds)
    pub async fn is_vibe_bypass_enabled(pool: &SqlitePool) -> bool {
        if !cfg!(debug_assertions) {
            return false;
        }
        Self::get(pool, "vibe_check_bypass")
            .await
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(false)
    }

    /// Get all settings as a list
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<SystemSetting>, sqlx::Error> {
        sqlx::query_as::<_, SystemSetting>(
            "SELECT key, value, updated_by, updated_at FROM system_settings ORDER BY key",
        )
        .fetch_all(pool)
        .await
    }
}
