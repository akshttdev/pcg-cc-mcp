use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
// IMPORTANT: Use DbUuid (not uuid::Uuid) for all SQLite-bound UUID fields.
// uuid::Uuid encodes as a 16-byte BLOB, but our tables store UUIDs as TEXT.
// DbUuid always encodes as TEXT and decodes both BLOB and TEXT transparently.
use crate::db_uuid::DbUuid;

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "actor_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    Human,
    Agent,
    Mcp,
    System,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ActivityLog {
    pub id: DbUuid,
    pub task_id: DbUuid,
    pub actor_id: DbUuid,
    pub actor_type: ActorType,
    pub action: String,
    pub previous_state: Option<String>, // JSON object
    pub new_state: Option<String>,      // JSON object
    pub metadata: Option<String>,       // JSON object
    pub timestamp: DateTime<Utc>,
    /// Display name of the actor (populated via JOIN, not stored in table)
    #[sqlx(default)]
    pub actor_name: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateActivityLog {
    pub task_id: String,
    pub actor_id: String,
    pub actor_type: ActorType,
    pub action: String,
    pub previous_state: Option<serde_json::Value>,
    pub new_state: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

impl ActivityLog {
    pub async fn find_by_task_id(
        pool: &SqlitePool,
        task_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let task_id_db = DbUuid::from_string(task_id);
        sqlx::query_as::<_, ActivityLog>(
            r#"SELECT al.id, al.task_id, al.actor_id, al.actor_type, al.action,
                      al.previous_state, al.new_state, al.metadata, al.timestamp,
                      COALESCE(u.full_name, u.username) AS actor_name
               FROM activity_logs al
               LEFT JOIN users u ON lower(hex(u.id)) = replace(al.actor_id, '-', '')
               WHERE al.task_id = $1
               ORDER BY al.timestamp DESC"#,
        )
        .bind(&task_id_db)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        let id_db = DbUuid::from_string(id);
        sqlx::query_as::<_, ActivityLog>(
            r#"SELECT al.id, al.task_id, al.actor_id, al.actor_type, al.action,
                      al.previous_state, al.new_state, al.metadata, al.timestamp,
                      COALESCE(u.full_name, u.username) AS actor_name
               FROM activity_logs al
               LEFT JOIN users u ON lower(hex(u.id)) = replace(al.actor_id, '-', '')
               WHERE al.id = $1"#,
        )
        .bind(&id_db)
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateActivityLog) -> Result<Self, sqlx::Error> {
        let id = DbUuid::new();
        let task_id = DbUuid::from_string(&data.task_id);
        let previous_state_json = data
            .previous_state
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());
        let new_state_json = data
            .new_state
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap());

        sqlx::query_as::<_, ActivityLog>(
            r#"INSERT INTO activity_logs (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id, task_id, actor_id, actor_type, action,
                         previous_state, new_state, metadata, timestamp"#,
        )
        .bind(&id)
        .bind(&task_id)
        .bind(&data.actor_id)
        .bind(&data.actor_type)
        .bind(&data.action)
        .bind(&previous_state_json)
        .bind(&new_state_json)
        .bind(&metadata_json)
        .fetch_one(pool)
        .await
    }

    /// Fetch recent activity across multiple tasks (for notification feed).
    /// Returns the most recent N activity log entries for tasks in the given project IDs.
    pub async fn find_recent_for_projects(
        pool: &SqlitePool,
        project_ids: &[DbUuid],
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        if project_ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: Vec<String> = project_ids.iter().map(|_| "?".to_string()).collect();
        let query_str = format!(
            r#"SELECT
                al.id, al.task_id, al.actor_id, al.actor_type, al.action,
                al.previous_state, al.new_state, al.metadata, al.timestamp,
                COALESCE(u.full_name, u.username) AS actor_name
            FROM activity_logs al
            JOIN tasks t ON al.task_id = t.id
            LEFT JOIN users u ON lower(hex(u.id)) = replace(al.actor_id, '-', '')
            WHERE t.project_id IN ({})
            ORDER BY al.timestamp DESC
            LIMIT ?"#,
            placeholders.join(", ")
        );

        let mut query = sqlx::query_as::<_, ActivityLog>(&query_str);
        for pid in project_ids {
            query = query.bind(pid);
        }
        query = query.bind(limit);
        query.fetch_all(pool).await
    }

    pub async fn delete(pool: &SqlitePool, id: &DbUuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM activity_logs WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
