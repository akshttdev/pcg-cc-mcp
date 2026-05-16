use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// A media asset attached to a social post or deliverable (distinct from the
/// video-studio DAM `media_assets` table, which lives in `media_asset.rs`).
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SocialMediaAsset {
    pub id: Uuid,
    pub project_id: Uuid,
    pub deliverable_id: Option<Uuid>,
    pub social_post_id: Option<Uuid>,
    pub filename: String,
    pub original_filename: Option<String>,
    pub storage_path: String,
    pub public_url: String,
    pub mime_type: String,
    pub file_size_bytes: Option<i64>,
    pub width_px: Option<i64>,
    pub height_px: Option<i64>,
    pub uploaded_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSocialMediaAsset {
    pub project_id: Uuid,
    pub deliverable_id: Option<Uuid>,
    pub social_post_id: Option<Uuid>,
    pub filename: String,
    pub original_filename: Option<String>,
    pub storage_path: String,
    pub public_url: String,
    pub mime_type: String,
    pub file_size_bytes: Option<i64>,
    pub width_px: Option<i64>,
    pub height_px: Option<i64>,
    pub uploaded_by: Option<String>,
}

impl SocialMediaAsset {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateSocialMediaAsset,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, SocialMediaAsset>(r#"
            INSERT INTO social_media_assets (id, project_id, deliverable_id, social_post_id, filename, original_filename, storage_path, public_url, mime_type, file_size_bytes, width_px, height_px, uploaded_by)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            RETURNING *
        "#)
        .bind(id)
        .bind(data.project_id)
        .bind(data.deliverable_id)
        .bind(data.social_post_id)
        .bind(&data.filename)
        .bind(&data.original_filename)
        .bind(&data.storage_path)
        .bind(&data.public_url)
        .bind(&data.mime_type)
        .bind(data.file_size_bytes)
        .bind(data.width_px)
        .bind(data.height_px)
        .bind(&data.uploaded_by)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SocialMediaAsset>(
            "SELECT * FROM social_media_assets WHERE project_id = ?1 ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_deliverable(
        pool: &SqlitePool,
        deliverable_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SocialMediaAsset>(
            "SELECT * FROM social_media_assets WHERE deliverable_id = ?1 ORDER BY created_at DESC",
        )
        .bind(deliverable_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_post(
        pool: &SqlitePool,
        social_post_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SocialMediaAsset>(
            "SELECT * FROM social_media_assets WHERE social_post_id = ?1 ORDER BY created_at DESC",
        )
        .bind(social_post_id)
        .fetch_all(pool)
        .await
    }
}
