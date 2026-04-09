use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct VideoJob {
    pub id: Uuid,
    pub avatar_profile_id: Uuid,
    pub created_by: Option<Uuid>,
    pub script_text: String,
    pub background_url: Option<String>,
    // TTS
    pub tts_status: String,
    pub tts_audio_url: Option<String>,
    // HeyGen
    pub heygen_video_id: Option<String>,
    pub heygen_status: Option<String>,
    pub raw_video_url: Option<String>,
    // Output
    pub final_video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<f64>,
    // Post-production (overlay + B-roll assembly)
    pub postprod_status: String,
    pub postprod_video_path: Option<String>,
    // Intelligence metadata
    pub segments_json: Option<String>,
    pub word_timestamps_json: Option<String>,
    pub cut_points_json: Option<String>,
    // Overall
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateVideoJob {
    pub avatar_profile_id: Uuid,
    pub script_text: String,
    pub background_url: Option<String>,
    pub segments_json: Option<String>,
}

impl VideoJob {
    pub async fn create(
        pool: &SqlitePool,
        input: CreateVideoJob,
        created_by: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"INSERT INTO video_jobs
               (id, avatar_profile_id, created_by, script_text, background_url, segments_json)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(input.avatar_profile_id)
        .bind(created_by)
        .bind(&input.script_text)
        .bind(&input.background_url)
        .bind(&input.segments_json)
        .execute(pool)
        .await?;

        sqlx::query_as("SELECT * FROM video_jobs WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
    }

    pub async fn find(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM video_jobs WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn list(
        pool: &SqlitePool,
        avatar_id: Option<Uuid>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        match avatar_id {
            Some(aid) => {
                sqlx::query_as(
                    "SELECT * FROM video_jobs WHERE avatar_profile_id = ? ORDER BY created_at DESC",
                )
                .bind(aid)
                .fetch_all(pool)
                .await
            }
            None => {
                sqlx::query_as("SELECT * FROM video_jobs ORDER BY created_at DESC")
                    .fetch_all(pool)
                    .await
            }
        }
    }

    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE video_jobs SET status = ?, error_message = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(status)
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_tts_done(
        pool: &SqlitePool,
        id: Uuid,
        audio_url: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE video_jobs SET
               tts_status = 'done', tts_audio_url = ?,
               status = 'avatar_generating',
               updated_at = datetime('now','subsec')
               WHERE id = ?"#,
        )
        .bind(audio_url)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_heygen_started(
        pool: &SqlitePool,
        id: Uuid,
        heygen_video_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE video_jobs SET
               heygen_video_id = ?, heygen_status = 'processing',
               updated_at = datetime('now','subsec')
               WHERE id = ?"#,
        )
        .bind(heygen_video_id)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_completed(
        pool: &SqlitePool,
        id: Uuid,
        final_video_url: &str,
        raw_video_url: &str,
        thumbnail_url: Option<&str>,
        duration_seconds: Option<f64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE video_jobs SET
               heygen_status = 'completed',
               raw_video_url = ?,
               final_video_url = ?,
               thumbnail_url = ?,
               duration_seconds = ?,
               status = 'ready',
               updated_at = datetime('now','subsec')
               WHERE id = ?"#,
        )
        .bind(raw_video_url)
        .bind(final_video_url)
        .bind(thumbnail_url)
        .bind(duration_seconds)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_tts_metadata(
        pool: &SqlitePool,
        id: Uuid,
        word_timestamps_json: &str,
        cut_points_json: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE video_jobs SET word_timestamps_json = ?, cut_points_json = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(word_timestamps_json)
        .bind(cut_points_json)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_postprod_started(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE video_jobs SET postprod_status = 'processing', updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_postprod_done(
        pool: &SqlitePool,
        id: Uuid,
        output_path: &str,
        public_url: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE video_jobs SET
               postprod_status = 'done',
               postprod_video_path = ?,
               final_video_url = ?,
               status = 'postprod_ready',
               updated_at = datetime('now','subsec')
               WHERE id = ?"#,
        )
        .bind(output_path)
        .bind(public_url)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update_postprod_failed(
        pool: &SqlitePool,
        id: Uuid,
        error: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE video_jobs SET postprod_status = 'failed', error_message = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(error)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let r = sqlx::query("DELETE FROM video_jobs WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(r.rows_affected() > 0)
    }
}
