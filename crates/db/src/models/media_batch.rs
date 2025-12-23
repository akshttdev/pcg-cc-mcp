use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{types::Json, FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Status of a media batch ingestion
#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "media_batch_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MediaBatchStatus {
    Queued,
    Downloading,
    Ready,
    Analyzing,
    Analyzed,
    Failed,
}

impl Default for MediaBatchStatus {
    fn default() -> Self {
        Self::Queued
    }
}

/// Storage tier for media files
#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "media_storage_tier", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MediaStorageTier {
    Hot,
    Warm,
    Cold,
}

impl Default for MediaStorageTier {
    fn default() -> Self {
        Self::Hot
    }
}

/// Main media batch record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MediaBatch {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub reference_name: Option<String>,
    pub source_url: String,
    pub storage_tier: MediaStorageTier,
    pub checksum_required: bool,
    pub status: MediaBatchStatus,
    pub file_count: i64,
    pub total_size_bytes: i64,
    pub last_error: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Json<Value>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

/// Individual media file within a batch
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MediaFile {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub filename: String,
    pub file_path: String,
    pub size_bytes: i64,
    pub checksum_sha256: Option<String>,
    pub duration_seconds: Option<f64>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub fps: Option<f64>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Json<Value>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

/// Analysis results for a media batch
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct MediaBatchAnalysis {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub brief: String,
    pub summary: String,
    pub passes_completed: i64,
    #[ts(type = "string[]")]
    pub deliverable_targets: Json<Value>,
    #[ts(type = "Record<string, unknown>")]
    pub hero_moments: Json<Value>,
    #[ts(type = "Record<string, unknown>")]
    pub insights: Json<Value>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
}

/// Edit session for video editing
#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "edit_session_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum EditSessionStatus {
    Assembling,
    NeedsReview,
    Approved,
    Rendering,
    Complete,
    Failed,
}

impl Default for EditSessionStatus {
    fn default() -> Self {
        Self::Assembling
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct EditSession {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub deliverable_type: String,
    #[ts(type = "string[]")]
    pub aspect_ratios: Json<Value>,
    pub reference_style: Option<String>,
    pub include_captions: bool,
    pub imovie_project: String,
    pub status: EditSessionStatus,
    #[ts(type = "Record<string, unknown>")]
    pub timelines: Json<Value>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Json<Value>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

/// Render priority
#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "render_priority", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RenderPriority {
    Low,
    Standard,
    Rush,
}

impl Default for RenderPriority {
    fn default() -> Self {
        Self::Standard
    }
}

/// Render job status
#[derive(Debug, Clone, Type, Serialize, Deserialize, TS, PartialEq)]
#[sqlx(type_name = "render_job_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum RenderJobStatus {
    Queued,
    Rendering,
    Complete,
    Failed,
}

impl Default for RenderJobStatus {
    fn default() -> Self {
        Self::Queued
    }
}

/// Render job for exporting videos
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct RenderJob {
    pub id: Uuid,
    pub edit_session_id: Uuid,
    #[ts(type = "string[]")]
    pub destinations: Json<Value>,
    #[ts(type = "string[]")]
    pub formats: Json<Value>,
    pub priority: RenderPriority,
    pub status: RenderJobStatus,
    pub progress_percent: Option<f64>,
    pub last_error: Option<String>,
    #[ts(type = "string[]")]
    pub output_urls: Json<Value>,
    #[ts(type = "Record<string, unknown>")]
    pub metadata: Json<Value>,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

/// Create a new media batch
#[derive(Debug, Deserialize, TS)]
pub struct CreateMediaBatch {
    pub project_id: Option<Uuid>,
    pub reference_name: Option<String>,
    pub source_url: String,
    pub storage_tier: MediaStorageTier,
    #[serde(default = "default_checksum")]
    pub checksum_required: bool,
}

fn default_checksum() -> bool {
    true
}

impl MediaBatch {
    /// Create a new media batch
    pub async fn create(
        pool: &SqlitePool,
        create: CreateMediaBatch,
    ) -> Result<Self, sqlx::Error> {
        let now = Utc::now();
        let batch = MediaBatch {
            id: Uuid::new_v4(),
            project_id: create.project_id,
            reference_name: create.reference_name,
            source_url: create.source_url,
            storage_tier: create.storage_tier,
            checksum_required: create.checksum_required,
            status: MediaBatchStatus::Queued,
            file_count: 0,
            total_size_bytes: 0,
            last_error: None,
            metadata: Json(serde_json::json!({})),
            created_at: now,
            updated_at: now,
        };

        sqlx::query!(
            r#"
            INSERT INTO media_batches (
                id, project_id, reference_name, source_url, storage_tier,
                checksum_required, status, file_count, total_size_bytes,
                last_error, metadata, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            batch.id,
            batch.project_id,
            batch.reference_name,
            batch.source_url,
            batch.storage_tier,
            batch.checksum_required,
            batch.status,
            batch.file_count,
            batch.total_size_bytes,
            batch.last_error,
            batch.metadata,
            batch.created_at,
            batch.updated_at,
        )
        .execute(pool)
        .await?;

        Ok(batch)
    }

    /// Find a media batch by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            MediaBatch,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id?: Uuid",
                reference_name,
                source_url,
                storage_tier as "storage_tier!: MediaStorageTier",
                checksum_required,
                status as "status!: MediaBatchStatus",
                file_count,
                total_size_bytes,
                last_error,
                metadata as "metadata!: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM media_batches WHERE id = ?"#,
            id
        )
        .fetch_one(pool)
        .await
    }

    /// Find all media batches for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            MediaBatch,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id?: Uuid",
                reference_name,
                source_url,
                storage_tier as "storage_tier!: MediaStorageTier",
                checksum_required,
                status as "status!: MediaBatchStatus",
                file_count,
                total_size_bytes,
                last_error,
                metadata as "metadata!: Json<Value>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM media_batches WHERE project_id = ? ORDER BY created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Update batch status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: MediaBatchStatus,
        error: Option<String>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query!(
            r#"UPDATE media_batches SET status = ?, last_error = ?, updated_at = ? WHERE id = ?"#,
            status,
            error,
            now,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}

impl MediaFile {
    /// Create a new media file
    pub async fn create(
        pool: &SqlitePool,
        batch_id: Uuid,
        filename: String,
        file_path: String,
        size_bytes: i64,
        checksum: Option<String>,
    ) -> Result<Self, sqlx::Error> {
        let file = MediaFile {
            id: Uuid::new_v4(),
            batch_id,
            filename,
            file_path,
            size_bytes,
            checksum_sha256: checksum,
            duration_seconds: None,
            resolution: None,
            codec: None,
            fps: None,
            metadata: Json(serde_json::json!({})),
            created_at: Utc::now(),
        };

        sqlx::query!(
            r#"
            INSERT INTO media_files (
                id, batch_id, filename, file_path, size_bytes, checksum_sha256,
                duration_seconds, resolution, codec, fps, metadata, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            file.id,
            file.batch_id,
            file.filename,
            file.file_path,
            file.size_bytes,
            file.checksum_sha256,
            file.duration_seconds,
            file.resolution,
            file.codec,
            file.fps,
            file.metadata,
            file.created_at,
        )
        .execute(pool)
        .await?;

        Ok(file)
    }

    /// Find all files in a batch
    pub async fn find_by_batch(
        pool: &SqlitePool,
        batch_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            MediaFile,
            r#"SELECT
                id as "id!: Uuid",
                batch_id as "batch_id!: Uuid",
                filename,
                file_path,
                size_bytes,
                checksum_sha256,
                duration_seconds,
                resolution,
                codec,
                fps,
                metadata as "metadata!: Json<Value>",
                created_at as "created_at!: DateTime<Utc>"
            FROM media_files WHERE batch_id = ?"#,
            batch_id
        )
        .fetch_all(pool)
        .await
    }
}
