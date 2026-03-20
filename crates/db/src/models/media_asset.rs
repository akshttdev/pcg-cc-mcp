use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// A media asset stored in the local DAM (Shade-equivalent).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: Uuid,
    pub project_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub filename: String,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub mime_type: String,
    pub duration_seconds: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    // AI metadata
    pub ai_description: Option<String>,
    pub shot_type: Option<String>,
    pub energy_level: f64,
    pub motion_intensity: f64,
    pub dominant_colors: String, // JSON array
    pub scene_tags: String,      // JSON array
    pub ai_confidence: f64,
    pub analysis_status: String, // pending|running|done|failed
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMediaAsset {
    pub project_id: Uuid,
    pub batch_id: Option<Uuid>,
    pub filename: String,
    pub file_path: String,
    pub file_size_bytes: Option<i64>,
    pub mime_type: Option<String>,
    pub duration_seconds: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

impl MediaAsset {
    pub async fn create(pool: &SqlitePool, input: CreateMediaAsset) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let mime = input.mime_type.unwrap_or_else(|| "video/mp4".into());
        let size = input.file_size_bytes.unwrap_or(0);

        sqlx::query(
            r#"INSERT INTO media_assets
               (id, project_id, batch_id, filename, file_path, file_size_bytes,
                mime_type, duration_seconds, width, height)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(input.project_id)
        .bind(input.batch_id)
        .bind(&input.filename)
        .bind(&input.file_path)
        .bind(size)
        .bind(&mime)
        .bind(input.duration_seconds)
        .bind(input.width)
        .bind(input.height)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM media_assets WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM media_assets WHERE project_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// FTS5 natural-language search within a project.
    pub async fn search(
        pool: &SqlitePool,
        project_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(
            r#"SELECT ma.* FROM media_assets ma
               JOIN media_assets_fts fts ON fts.asset_id = hex(ma.id)
               WHERE media_assets_fts MATCH ?1 AND ma.project_id = ?2
               ORDER BY rank LIMIT ?3"#,
        )
        .bind(query)
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    // TODO: refactor into struct
    #[allow(clippy::too_many_arguments)]
    pub async fn update_ai_metadata(
        pool: &SqlitePool,
        id: Uuid,
        description: Option<String>,
        shot_type: Option<String>,
        energy_level: f64,
        motion_intensity: f64,
        dominant_colors: String,
        scene_tags: String,
        confidence: f64,
        status: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE media_assets SET
               ai_description = ?, shot_type = ?, energy_level = ?,
               motion_intensity = ?, dominant_colors = ?, scene_tags = ?,
               ai_confidence = ?, analysis_status = ?,
               updated_at = datetime('now','subsec')
               WHERE id = ?"#,
        )
        .bind(description)
        .bind(shot_type)
        .bind(energy_level)
        .bind(motion_intensity)
        .bind(dominant_colors)
        .bind(scene_tags)
        .bind(confidence)
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM media_assets WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
