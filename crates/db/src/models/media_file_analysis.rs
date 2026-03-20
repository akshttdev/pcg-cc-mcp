use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Scene Analysis persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MediaFileSceneAnalysis {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub batch_id: Uuid,
    pub file_path: String,
    pub overall_energy: f64,
    pub peak_energy_timestamp: f64,
    pub dominant_content_type: String,
    pub usable: bool,
    /// JSON-serialized Vec<SceneSegment>
    pub segments: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl MediaFileSceneAnalysis {
    pub async fn upsert(pool: &SqlitePool, record: &MediaFileSceneAnalysis) -> sqlx::Result<()> {
        sqlx::query(
            r#"INSERT INTO media_file_scene_analyses
                (id, media_file_id, batch_id, file_path, overall_energy,
                 peak_energy_timestamp, dominant_content_type, usable, segments)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 overall_energy = excluded.overall_energy,
                 peak_energy_timestamp = excluded.peak_energy_timestamp,
                 dominant_content_type = excluded.dominant_content_type,
                 usable = excluded.usable,
                 segments = excluded.segments"#,
        )
        .bind(record.id)
        .bind(record.media_file_id)
        .bind(record.batch_id)
        .bind(&record.file_path)
        .bind(record.overall_energy)
        .bind(record.peak_energy_timestamp)
        .bind(&record.dominant_content_type)
        .bind(record.usable)
        .bind(&record.segments)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_media_file(
        pool: &SqlitePool,
        media_file_id: Uuid,
    ) -> sqlx::Result<Option<MediaFileSceneAnalysis>> {
        sqlx::query_as::<_, MediaFileSceneAnalysis>(
            r#"SELECT id, media_file_id, batch_id, file_path, overall_energy,
                      peak_energy_timestamp, dominant_content_type, usable, segments, created_at
               FROM media_file_scene_analyses
               WHERE media_file_id = ?"#,
        )
        .bind(media_file_id)
        .fetch_optional(pool)
        .await
    }
}

// ---------------------------------------------------------------------------
// Visual QC persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MediaFileVisualQc {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub batch_id: Uuid,
    pub file_path: String,
    pub best_in_point: f64,
    pub best_composition_score: f64,
    pub qc_passed: bool,
    pub summary: String,
    /// JSON-serialized Vec<FrameAnalysis>
    pub analyzed_frames: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl MediaFileVisualQc {
    pub async fn upsert(pool: &SqlitePool, record: &MediaFileVisualQc) -> sqlx::Result<()> {
        sqlx::query(
            r#"INSERT INTO media_file_visual_qcs
                (id, media_file_id, batch_id, file_path, best_in_point,
                 best_composition_score, qc_passed, summary, analyzed_frames)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 best_in_point = excluded.best_in_point,
                 best_composition_score = excluded.best_composition_score,
                 qc_passed = excluded.qc_passed,
                 summary = excluded.summary,
                 analyzed_frames = excluded.analyzed_frames"#,
        )
        .bind(record.id)
        .bind(record.media_file_id)
        .bind(record.batch_id)
        .bind(&record.file_path)
        .bind(record.best_in_point)
        .bind(record.best_composition_score)
        .bind(record.qc_passed)
        .bind(&record.summary)
        .bind(&record.analyzed_frames)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_media_file(
        pool: &SqlitePool,
        media_file_id: Uuid,
    ) -> sqlx::Result<Option<MediaFileVisualQc>> {
        sqlx::query_as::<_, MediaFileVisualQc>(
            r#"SELECT id, media_file_id, batch_id, file_path, best_in_point,
                      best_composition_score, qc_passed, summary, analyzed_frames, created_at
               FROM media_file_visual_qcs
               WHERE media_file_id = ?"#,
        )
        .bind(media_file_id)
        .fetch_optional(pool)
        .await
    }
}

// ---------------------------------------------------------------------------
// Beat Analysis persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MediaFileBeatAnalysis {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub batch_id: Uuid,
    pub file_path: String,
    pub bpm: f64,
    pub beat_interval: f64,
    pub total_beats: i64,
    pub beats_per_bar: i64,
    /// JSON-serialized Vec<BeatPoint>
    pub beats: String,
    /// JSON-serialized Vec<MusicStructureSection>
    pub sections: String,
    /// JSON-serialized Vec<MusicEnergyPoint>
    pub energy_curve: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl MediaFileBeatAnalysis {
    pub async fn upsert(pool: &SqlitePool, record: &MediaFileBeatAnalysis) -> sqlx::Result<()> {
        sqlx::query(
            r#"INSERT INTO media_file_beat_analyses
                (id, media_file_id, batch_id, file_path, bpm, beat_interval,
                 total_beats, beats_per_bar, beats, sections, energy_curve)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(id) DO UPDATE SET
                 bpm = excluded.bpm,
                 beat_interval = excluded.beat_interval,
                 total_beats = excluded.total_beats,
                 beats_per_bar = excluded.beats_per_bar,
                 beats = excluded.beats,
                 sections = excluded.sections,
                 energy_curve = excluded.energy_curve"#,
        )
        .bind(record.id)
        .bind(record.media_file_id)
        .bind(record.batch_id)
        .bind(&record.file_path)
        .bind(record.bpm)
        .bind(record.beat_interval)
        .bind(record.total_beats)
        .bind(record.beats_per_bar)
        .bind(&record.beats)
        .bind(&record.sections)
        .bind(&record.energy_curve)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_media_file(
        pool: &SqlitePool,
        media_file_id: Uuid,
    ) -> sqlx::Result<Option<MediaFileBeatAnalysis>> {
        sqlx::query_as::<_, MediaFileBeatAnalysis>(
            r#"SELECT id, media_file_id, batch_id, file_path, bpm, beat_interval,
                      total_beats, beats_per_bar, beats, sections, energy_curve, created_at
               FROM media_file_beat_analyses
               WHERE media_file_id = ?"#,
        )
        .bind(media_file_id)
        .fetch_optional(pool)
        .await
    }
}

// ---------------------------------------------------------------------------
// Media File Tags persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MediaFileTag {
    pub id: Uuid,
    pub media_file_id: Uuid,
    pub batch_id: Uuid,
    pub tag: String,
    pub source: String,
    pub confidence: f64,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

impl MediaFileTag {
    pub async fn insert(pool: &SqlitePool, record: &MediaFileTag) -> sqlx::Result<()> {
        sqlx::query(
            r#"INSERT INTO media_file_tags
                (id, media_file_id, batch_id, tag, source, confidence)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(record.id)
        .bind(record.media_file_id)
        .bind(record.batch_id)
        .bind(&record.tag)
        .bind(&record.source)
        .bind(record.confidence)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn find_by_media_file(
        pool: &SqlitePool,
        media_file_id: Uuid,
    ) -> sqlx::Result<Vec<MediaFileTag>> {
        sqlx::query_as::<_, MediaFileTag>(
            r#"SELECT id, media_file_id, batch_id, tag, source, confidence, created_at
               FROM media_file_tags
               WHERE media_file_id = ?"#,
        )
        .bind(media_file_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_tag(pool: &SqlitePool, tag: &str) -> sqlx::Result<Vec<MediaFileTag>> {
        sqlx::query_as::<_, MediaFileTag>(
            r#"SELECT id, media_file_id, batch_id, tag, source, confidence, created_at
               FROM media_file_tags
               WHERE tag = ?"#,
        )
        .bind(tag)
        .fetch_all(pool)
        .await
    }
}
