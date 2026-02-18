use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PulseAlertRule {
    pub id: String,
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub conditions: String,
    pub actions: String,
    pub priority: String,
    pub enabled: bool,
    pub trigger_count: i64,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreatePulseAlertRule {
    pub project_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub name: String,
    pub conditions: serde_json::Value,
    pub actions: serde_json::Value,
    pub priority: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdatePulseAlertRule {
    pub name: Option<String>,
    pub conditions: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
    pub priority: Option<String>,
    pub enabled: Option<bool>,
}

impl PulseAlertRule {
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM pulse_alert_rules WHERE project_id = ? ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM pulse_alert_rules WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreatePulseAlertRule,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let priority = data.priority.as_deref().unwrap_or("normal");
        let enabled = data.enabled.unwrap_or(true);

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO pulse_alert_rules (id, project_id, organization_id, name, conditions, actions, priority, enabled)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(data.project_id)
        .bind(data.organization_id)
        .bind(&data.name)
        .bind(data.conditions.to_string())
        .bind(data.actions.to_string())
        .bind(priority)
        .bind(enabled)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        data: &UpdatePulseAlertRule,
    ) -> Result<Self, sqlx::Error> {
        let current = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let name = data.name.as_deref().unwrap_or(&current.name);
        let conditions = data
            .conditions
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or(current.conditions);
        let actions = data
            .actions
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or(current.actions);
        let priority = data.priority.as_deref().unwrap_or(&current.priority);
        let enabled = data.enabled.unwrap_or(current.enabled);

        sqlx::query_as::<_, Self>(
            r#"UPDATE pulse_alert_rules SET name = ?, conditions = ?, actions = ?, priority = ?, enabled = ?,
               updated_at = datetime('now', 'subsec')
               WHERE id = ? RETURNING *"#,
        )
        .bind(name)
        .bind(&conditions)
        .bind(&actions)
        .bind(priority)
        .bind(enabled)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn increment_trigger_count(
        pool: &SqlitePool,
        id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE pulse_alert_rules SET trigger_count = trigger_count + 1,
               last_triggered_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec')
               WHERE id = ?"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM pulse_alert_rules WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
