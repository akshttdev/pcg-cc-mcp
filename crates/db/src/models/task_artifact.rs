use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TaskArtifactError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Task-artifact link not found")]
    NotFound,
    #[error("Link already exists")]
    AlreadyExists,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "artifact_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ArtifactRole {
    Primary,
    Supporting,
    Verification,
    Reference,
}

impl std::fmt::Display for ArtifactRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactRole::Primary => write!(f, "primary"),
            ArtifactRole::Supporting => write!(f, "supporting"),
            ArtifactRole::Verification => write!(f, "verification"),
            ArtifactRole::Reference => write!(f, "reference"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TaskArtifact {
    pub task_id: Uuid,
    pub artifact_id: Uuid,
    pub artifact_role: ArtifactRole,
    pub display_order: i32,
    pub pinned: bool,
    pub added_at: DateTime<Utc>,
    pub added_by: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct LinkArtifactToTask {
    pub task_id: Uuid,
    pub artifact_id: Uuid,
    pub artifact_role: Option<ArtifactRole>,
    pub display_order: Option<i32>,
    pub pinned: Option<bool>,
    pub added_by: Option<String>,
}

impl TaskArtifact {
    /// Link an artifact to a task
    pub async fn link(
        pool: &SqlitePool,
        data: LinkArtifactToTask,
    ) -> Result<Self, TaskArtifactError> {
        let role = data.artifact_role.unwrap_or(ArtifactRole::Supporting);
        let role_str = role.to_string();
        let order = data.display_order.unwrap_or(0);
        let pinned = data.pinned.unwrap_or(false);

        let link = sqlx::query_as::<_, TaskArtifact>(
            r#"
            INSERT INTO task_artifacts (
                task_id, artifact_id, artifact_role, display_order, pinned, added_by
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING *
            "#,
        )
        .bind(data.task_id)
        .bind(data.artifact_id)
        .bind(role_str)
        .bind(order)
        .bind(pinned)
        .bind(&data.added_by)
        .fetch_one(pool)
        .await?;

        Ok(link)
    }

    /// Find all artifacts for a task
    pub async fn find_by_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, TaskArtifactError> {
        let links = sqlx::query_as::<_, TaskArtifact>(
            r#"
            SELECT * FROM task_artifacts
            WHERE task_id = ?1
            ORDER BY pinned DESC, display_order ASC
            "#,
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        Ok(links)
    }

    /// Find all tasks for an artifact
    pub async fn find_by_artifact(
        pool: &SqlitePool,
        artifact_id: Uuid,
    ) -> Result<Vec<Self>, TaskArtifactError> {
        let links = sqlx::query_as::<_, TaskArtifact>(
            r#"
            SELECT * FROM task_artifacts
            WHERE artifact_id = ?1
            ORDER BY added_at DESC
            "#,
        )
        .bind(artifact_id)
        .fetch_all(pool)
        .await?;

        Ok(links)
    }

    /// Find pinned artifacts for a task
    pub async fn find_pinned(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, TaskArtifactError> {
        let links = sqlx::query_as::<_, TaskArtifact>(
            r#"
            SELECT * FROM task_artifacts
            WHERE task_id = ?1 AND pinned = 1
            ORDER BY display_order ASC
            "#,
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        Ok(links)
    }

    /// Find artifacts by role
    pub async fn find_by_role(
        pool: &SqlitePool,
        task_id: Uuid,
        role: ArtifactRole,
    ) -> Result<Vec<Self>, TaskArtifactError> {
        let role_str = role.to_string();

        let links = sqlx::query_as::<_, TaskArtifact>(
            r#"
            SELECT * FROM task_artifacts
            WHERE task_id = ?1 AND artifact_role = ?2
            ORDER BY display_order ASC
            "#,
        )
        .bind(task_id)
        .bind(role_str)
        .fetch_all(pool)
        .await?;

        Ok(links)
    }

    /// Update artifact role
    pub async fn update_role(
        pool: &SqlitePool,
        task_id: Uuid,
        artifact_id: Uuid,
        role: ArtifactRole,
    ) -> Result<Self, TaskArtifactError> {
        let role_str = role.to_string();

        let link = sqlx::query_as::<_, TaskArtifact>(
            r#"
            UPDATE task_artifacts
            SET artifact_role = ?3
            WHERE task_id = ?1 AND artifact_id = ?2
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(artifact_id)
        .bind(role_str)
        .fetch_one(pool)
        .await?;

        Ok(link)
    }

    /// Toggle pin status
    pub async fn toggle_pin(
        pool: &SqlitePool,
        task_id: Uuid,
        artifact_id: Uuid,
    ) -> Result<Self, TaskArtifactError> {
        let link = sqlx::query_as::<_, TaskArtifact>(
            r#"
            UPDATE task_artifacts
            SET pinned = NOT pinned
            WHERE task_id = ?1 AND artifact_id = ?2
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(artifact_id)
        .fetch_one(pool)
        .await?;

        Ok(link)
    }

    /// Reorder artifacts
    pub async fn reorder(
        pool: &SqlitePool,
        task_id: Uuid,
        artifact_id: Uuid,
        new_order: i32,
    ) -> Result<Self, TaskArtifactError> {
        let link = sqlx::query_as::<_, TaskArtifact>(
            r#"
            UPDATE task_artifacts
            SET display_order = ?3
            WHERE task_id = ?1 AND artifact_id = ?2
            RETURNING *
            "#,
        )
        .bind(task_id)
        .bind(artifact_id)
        .bind(new_order)
        .fetch_one(pool)
        .await?;

        Ok(link)
    }

    /// Unlink an artifact from a task
    pub async fn unlink(
        pool: &SqlitePool,
        task_id: Uuid,
        artifact_id: Uuid,
    ) -> Result<bool, TaskArtifactError> {
        let result = sqlx::query(
            r#"DELETE FROM task_artifacts WHERE task_id = ?1 AND artifact_id = ?2"#,
        )
        .bind(task_id)
        .bind(artifact_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Unlink all artifacts from a task
    pub async fn unlink_all_from_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<u64, TaskArtifactError> {
        let result = sqlx::query(r#"DELETE FROM task_artifacts WHERE task_id = ?1"#)
            .bind(task_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}
