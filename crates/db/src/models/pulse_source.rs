use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PulseSourceType {
    Rss,
    Twitter,
    Youtube,
    Reddit,
    Web,
}

impl std::fmt::Display for PulseSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Rss => "rss",
            Self::Twitter => "twitter",
            Self::Youtube => "youtube",
            Self::Reddit => "reddit",
            Self::Web => "web",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseSource {
    pub id: String,
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub source_id: String,
    pub source_type: String,
    pub name: String,
    pub url: String,
    pub config: Option<String>,
    pub enabled: bool,
    pub status: String,
    pub last_fetch_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub collection_interval_secs: i64,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreatePulseSource {
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub source_id: String,
    pub source_type: String,
    pub name: String,
    pub url: String,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub collection_interval_secs: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdatePulseSource {
    pub name: Option<String>,
    pub url: Option<String>,
    pub config: Option<serde_json::Value>,
    pub enabled: Option<bool>,
    pub collection_interval_secs: Option<i64>,
    pub category: Option<String>,
}

impl PulseSource {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_sources WHERE project_id = ? ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM pulse_sources WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreatePulseSource) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let config_json = data.config.as_ref().map(|v| v.to_string());
        let enabled = data.enabled.unwrap_or(true);
        let interval = data.collection_interval_secs.unwrap_or(300);

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO pulse_sources (id, project_id, organization_id, source_id, source_type, name, url, config, enabled, collection_interval_secs, category)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(data.project_id)
        .bind(data.organization_id)
        .bind(&data.source_id)
        .bind(&data.source_type)
        .bind(&data.name)
        .bind(&data.url)
        .bind(&config_json)
        .bind(enabled)
        .bind(interval)
        .bind(&data.category)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        data: &UpdatePulseSource,
    ) -> Result<Self, sqlx::Error> {
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_deref().unwrap_or(&current.name);
        let url = data.url.as_deref().unwrap_or(&current.url);
        let config = data
            .config
            .as_ref()
            .map(|v| v.to_string())
            .or(current.config);
        let enabled = data.enabled.unwrap_or(current.enabled);
        let interval = data
            .collection_interval_secs
            .unwrap_or(current.collection_interval_secs);
        let category = data.category.as_deref().or(current.category.as_deref());

        sqlx::query_as::<_, Self>(
            r#"UPDATE pulse_sources SET name = ?, url = ?, config = ?, enabled = ?,
               collection_interval_secs = ?, category = ?, updated_at = datetime('now', 'subsec')
               WHERE id = ? RETURNING *"#,
        )
        .bind(name)
        .bind(url)
        .bind(&config)
        .bind(enabled)
        .bind(interval)
        .bind(category)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM pulse_sources WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
