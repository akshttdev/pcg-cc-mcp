use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseTrackingConfig {
    pub id: String,
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub keywords: String,
    pub entities: String,
    pub llm_enabled: bool,
    pub llm_model: Option<String>,
    pub notification_config: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdatePulseTrackingConfig {
    pub keywords: Option<serde_json::Value>,
    pub entities: Option<serde_json::Value>,
    pub llm_enabled: Option<bool>,
    pub llm_model: Option<String>,
    pub notification_config: Option<serde_json::Value>,
}

impl PulseTrackingConfig {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM pulse_tracking_configs WHERE project_id = ?")
            .bind(project_id)
            .fetch_optional(pool)
            .await
    }

    pub async fn upsert(
        pool: &SqlitePool,
        project_id: Uuid,
        organization_id: Option<Uuid>,
        data: &UpdatePulseTrackingConfig,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_project(pool, project_id).await?;

        if let Some(current) = existing {
            let keywords = data
                .keywords
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or(current.keywords);
            let entities = data
                .entities
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or(current.entities);
            let llm_enabled = data.llm_enabled.unwrap_or(current.llm_enabled);
            let llm_model = data.llm_model.as_deref().or(current.llm_model.as_deref());
            let notification_config = data
                .notification_config
                .as_ref()
                .map(|v| v.to_string())
                .or(current.notification_config);

            sqlx::query_as::<_, Self>(
                r#"UPDATE pulse_tracking_configs SET keywords = ?, entities = ?, llm_enabled = ?,
                   llm_model = ?, notification_config = ?, updated_at = datetime('now', 'subsec')
                   WHERE project_id = ? RETURNING *"#,
            )
            .bind(&keywords)
            .bind(&entities)
            .bind(llm_enabled)
            .bind(llm_model)
            .bind(&notification_config)
            .bind(project_id)
            .fetch_one(pool)
            .await
        } else {
            let id = Uuid::new_v4().to_string();
            let keywords = data
                .keywords
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "[]".to_string());
            let entities = data
                .entities
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_else(|| "{}".to_string());
            let llm_enabled = data.llm_enabled.unwrap_or(false);
            let llm_model = data.llm_model.as_deref().unwrap_or("llama3.2");
            let notification_config = data.notification_config.as_ref().map(|v| v.to_string());

            sqlx::query_as::<_, Self>(
                r#"INSERT INTO pulse_tracking_configs (id, project_id, organization_id, keywords, entities, llm_enabled, llm_model, notification_config)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                   RETURNING *"#,
            )
            .bind(&id)
            .bind(project_id)
            .bind(organization_id)
            .bind(&keywords)
            .bind(&entities)
            .bind(llm_enabled)
            .bind(llm_model)
            .bind(&notification_config)
            .fetch_one(pool)
            .await
        }
    }
}
