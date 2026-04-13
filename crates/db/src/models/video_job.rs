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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_utils::{
        bootstrap_video_schema, create_test_avatar_profile, create_test_video_job, setup_test_pool,
    };

    #[tokio::test]
    async fn test_video_job_create_and_find() {
        let pool = setup_test_pool().await;
        bootstrap_video_schema(&pool).await;

        let avatar_id = create_test_avatar_profile(&pool).await;
        let input = CreateVideoJob {
            avatar_profile_id: avatar_id,
            script_text: "Hello world test script.".to_string(),
            background_url: None,
            segments_json: None,
        };

        let job = VideoJob::create(&pool, input, None)
            .await
            .expect("create should succeed");

        assert_eq!(job.avatar_profile_id, avatar_id);
        assert_eq!(job.script_text, "Hello world test script.");
        assert_eq!(job.status, "pending");
        assert_eq!(job.tts_status, "pending");
        assert_eq!(job.postprod_status, "pending");
        assert!(job.error_message.is_none());

        let found = VideoJob::find(&pool, job.id)
            .await
            .expect("find should succeed")
            .expect("job should exist");

        assert_eq!(found.id, job.id);
        assert_eq!(found.script_text, "Hello world test script.");
    }

    #[tokio::test]
    async fn test_video_job_update_status() {
        let pool = setup_test_pool().await;
        bootstrap_video_schema(&pool).await;

        let avatar_id = create_test_avatar_profile(&pool).await;
        let job_id = create_test_video_job(&pool, avatar_id).await;

        VideoJob::update_status(&pool, job_id, "failed", Some("test error"))
            .await
            .expect("update_status should succeed");

        let job = VideoJob::find(&pool, job_id)
            .await
            .expect("find should succeed")
            .expect("job should exist");

        assert_eq!(job.status, "failed");
        assert_eq!(job.error_message.as_deref(), Some("test error"));
    }

    #[tokio::test]
    async fn test_video_job_update_tts_done() {
        let pool = setup_test_pool().await;
        bootstrap_video_schema(&pool).await;

        let avatar_id = create_test_avatar_profile(&pool).await;
        let job_id = create_test_video_job(&pool, avatar_id).await;

        VideoJob::update_tts_done(&pool, job_id, "https://audio.example/test.mp3")
            .await
            .expect("update_tts_done should succeed");

        let job = VideoJob::find(&pool, job_id)
            .await
            .expect("find should succeed")
            .expect("job should exist");

        assert_eq!(job.tts_status, "done");
        assert_eq!(
            job.tts_audio_url.as_deref(),
            Some("https://audio.example/test.mp3")
        );
        assert_eq!(job.status, "avatar_generating");
    }

    #[tokio::test]
    async fn test_video_job_update_postprod_done() {
        let pool = setup_test_pool().await;
        bootstrap_video_schema(&pool).await;

        let avatar_id = create_test_avatar_profile(&pool).await;
        let job_id = create_test_video_job(&pool, avatar_id).await;

        VideoJob::update_postprod_done(
            &pool,
            job_id,
            "/tmp/output.mp4",
            "https://example.com/final/123",
        )
        .await
        .expect("update_postprod_done should succeed");

        let job = VideoJob::find(&pool, job_id)
            .await
            .expect("find should succeed")
            .expect("job should exist");

        assert_eq!(job.postprod_status, "done");
        assert_eq!(
            job.final_video_url.as_deref(),
            Some("https://example.com/final/123")
        );
        assert_eq!(job.postprod_video_path.as_deref(), Some("/tmp/output.mp4"));
        assert_eq!(job.status, "postprod_ready");
    }

    #[tokio::test]
    async fn test_video_job_update_tts_metadata() {
        let pool = setup_test_pool().await;
        bootstrap_video_schema(&pool).await;

        let avatar_id = create_test_avatar_profile(&pool).await;
        let job_id = create_test_video_job(&pool, avatar_id).await;

        let word_ts = r#"[{"word":"hello","start_time":0.1,"end_time":0.5}]"#;
        let cut_pts = "[5.0, 12.3]";

        VideoJob::update_tts_metadata(&pool, job_id, word_ts, cut_pts)
            .await
            .expect("update_tts_metadata should succeed");

        let job = VideoJob::find(&pool, job_id)
            .await
            .expect("find should succeed")
            .expect("job should exist");

        assert_eq!(job.word_timestamps_json.as_deref(), Some(word_ts));
        assert_eq!(job.cut_points_json.as_deref(), Some(cut_pts));
    }

    #[tokio::test]
    async fn test_video_job_list() {
        let pool = setup_test_pool().await;
        bootstrap_video_schema(&pool).await;

        let avatar_id = create_test_avatar_profile(&pool).await;
        let _job1 = create_test_video_job(&pool, avatar_id).await;
        let _job2 = create_test_video_job(&pool, avatar_id).await;

        // Create a second avatar with its own job — should not appear in filtered list
        let other_avatar_id = create_test_avatar_profile(&pool).await;
        let _other_job = create_test_video_job(&pool, other_avatar_id).await;

        let jobs = VideoJob::list(&pool, Some(avatar_id))
            .await
            .expect("list should succeed");

        assert_eq!(jobs.len(), 2);
        for job in &jobs {
            assert_eq!(job.avatar_profile_id, avatar_id);
        }
    }

    #[tokio::test]
    async fn test_video_job_delete() {
        let pool = setup_test_pool().await;
        bootstrap_video_schema(&pool).await;

        let avatar_id = create_test_avatar_profile(&pool).await;
        let job_id = create_test_video_job(&pool, avatar_id).await;

        // Confirm it exists first
        let found = VideoJob::find(&pool, job_id)
            .await
            .expect("find should succeed");
        assert!(found.is_some());

        let deleted = VideoJob::delete(&pool, job_id)
            .await
            .expect("delete should succeed");
        assert!(deleted);

        let after = VideoJob::find(&pool, job_id)
            .await
            .expect("find after delete should succeed");
        assert!(after.is_none());
    }
}
