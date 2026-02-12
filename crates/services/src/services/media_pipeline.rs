use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use thiserror::Error;
use tokio::{
    fs,
    io::AsyncWriteExt,
    time::{Duration, sleep},
};
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum MediaPipelineError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Batch not found: {0}")]
    BatchNotFound(Uuid),
    #[error("Edit session not found: {0}")]
    EditSessionNotFound(Uuid),
    #[error("Render job not found: {0}")]
    RenderJobNotFound(Uuid),
    #[error("Batch {0} is not ready yet")]
    BatchNotReady(Uuid),
    #[error("Unsupported storage tier: {0}")]
    UnknownStorageTier(String),
}

#[derive(Clone)]
pub struct MediaPipelineService {
    inner: Arc<MediaPipelineInner>,
}

struct MediaPipelineInner {
    root: PathBuf,
    client: reqwest::Client,
    db_pool: Option<SqlitePool>,
}

impl MediaPipelineService {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, MediaPipelineError> {
        Self::new_with_options(root, None)
    }

    pub fn new_with_database<P: AsRef<Path>>(root: P, db_pool: SqlitePool) -> Result<Self, MediaPipelineError> {
        Self::new_with_options(root, Some(db_pool))
    }

    fn new_with_options<P: AsRef<Path>>(root: P, db_pool: Option<SqlitePool>) -> Result<Self, MediaPipelineError> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("batches"))?;
        std::fs::create_dir_all(root.join("sessions"))?;
        std::fs::create_dir_all(root.join("renders"))?;
        Ok(Self {
            inner: Arc::new(MediaPipelineInner {
                root,
                client: reqwest::Client::new(),
                db_pool,
            }),
        })
    }

    fn db_pool(&self) -> Option<&SqlitePool> {
        self.inner.db_pool.as_ref()
    }

    pub fn root(&self) -> PathBuf {
        self.inner.root.clone()
    }

    pub async fn ingest_batch(
        &self,
        request: MediaBatchIngestRequest,
    ) -> Result<MediaBatch, MediaPipelineError> {
        let now = Utc::now();
        let batch = MediaBatch {
            id: Uuid::new_v4(),
            project_id: request.project_id.clone(),
            reference_name: request.reference_name.clone(),
            source_url: request.source_url.clone(),
            storage_tier: request.storage_tier,
            checksum_required: request.checksum_required,
            status: MediaBatchStatus::Queued,
            created_at: now,
            updated_at: now,
            files: Vec::new(),
            last_error: None,
        };

        self.persist_batch(&batch).await?;

        if request.source_url.starts_with('/') || request.source_url.starts_with("file://") {
            // Local directory — process inline so batch is Ready before returning
            let batch_id = batch.id;
            match self.process_local_directory(batch_id, request).await {
                Ok(ready_batch) => return Ok(ready_batch),
                Err(err) => {
                    tracing::error!("Local ingest failed for {}: {}", batch_id, err);
                    let _ = self
                        .update_batch_status(
                            batch_id,
                            MediaBatchStatus::Failed,
                            Some(err.to_string()),
                        )
                        .await;
                    return Err(err);
                }
            }
        } else {
            // Remote URL — download as before
            let service = self.clone();
            let req_clone = request.clone();
            tokio::spawn(async move {
                if let Err(err) = service.process_download(batch.id, req_clone).await {
                    tracing::error!("Media ingest failed for {}: {}", batch.id, err);
                    let _ = service
                        .update_batch_status(
                            batch.id,
                            MediaBatchStatus::Failed,
                            Some(err.to_string()),
                        )
                        .await;
                }
            });
        }

        Ok(batch)
    }

    pub async fn analyze_batch(
        &self,
        request: MediaBatchAnalysisRequest,
    ) -> Result<MediaBatchAnalysis, MediaPipelineError> {
        let batch = self.load_batch(request.batch_id).await?;
        if batch.status != MediaBatchStatus::Ready {
            return Err(MediaPipelineError::BatchNotReady(batch.id));
        }

        let hero_moments: Vec<HeroMoment> = batch
            .files
            .iter()
            .enumerate()
            .map(|(idx, file)| HeroMoment {
                timestamp: format!("{:02}:{:02}", idx, (idx * 11) % 60),
                description: format!("Potential highlight spotted in {}", file.filename),
                confidence: (0.72 + (idx as f32 * 0.03)).min(0.98),
            })
            .collect();

        let analysis = MediaBatchAnalysis {
            id: Uuid::new_v4(),
            batch_id: batch.id,
            brief: request.brief.clone(),
            summary: format!(
                "Analyzed {} assets for brief '{}', identified {} hero moments",
                batch.files.len(),
                request.brief,
                hero_moments.len()
            ),
            hero_moments,
            recommended_deliverables: request.deliverable_targets.clone(),
            passes_completed: request.passes.max(1),
            created_at: Utc::now(),
            insights_path: self
                .analysis_path(batch.id)
                .join(format!("{}.json", Uuid::new_v4())),
        };

        self.persist_analysis(&analysis).await?;

        Ok(analysis)
    }

    pub async fn generate_edits(
        &self,
        request: EditSessionRequest,
    ) -> Result<EditSession, MediaPipelineError> {
        let batch = self.load_batch(request.batch_id).await?;
        if batch.status != MediaBatchStatus::Ready {
            return Err(MediaPipelineError::BatchNotReady(batch.id));
        }

        let EditSessionRequest {
            batch_id: _,
            deliverable_type,
            aspect_ratios,
            reference_style,
            include_captions,
        } = request;

        let timelines = aspect_ratios
            .iter()
            .map(|ratio| RenderTimeline {
                aspect_ratio: ratio.clone(),
                estimated_duration_seconds: 60 + ratio.len() as u32 * 5,
            })
            .collect();

        let now = Utc::now();
        let session = EditSession {
            id: Uuid::new_v4(),
            batch_id: batch.id,
            deliverable_type,
            aspect_ratios,
            include_captions,
            reference_style,
            imovie_project: format!("Editron_{}", &batch.id.to_string()[..8]),
            timelines,
            status: EditSessionStatus::Assembling,
            created_at: now,
            updated_at: now,
        };

        self.persist_session(&session).await?;

        Ok(session)
    }

    pub async fn render_deliverables(
        &self,
        request: RenderJobRequest,
    ) -> Result<RenderJob, MediaPipelineError> {
        let session = self.load_session(request.edit_session_id).await?;

        let now = Utc::now();
        let job = RenderJob {
            id: Uuid::new_v4(),
            edit_session_id: session.id,
            destinations: request.destinations,
            formats: request.formats,
            priority: request.priority,
            status: RenderJobStatus::Queued,
            created_at: now,
            updated_at: now,
        };

        self.persist_render_job(&job).await?;

        let service = self.clone();
        tokio::spawn(async move {
            let job_id = job.id;
            if let Err(err) = service.progress_render(job_id).await {
                tracing::error!("Render job {} failed: {}", job_id, err);
            }
        });

        Ok(job)
    }

    /// Load a media batch and verify it's ready for Visual QC
    pub async fn load_batch_for_qc(
        &self,
        batch_id: Uuid,
    ) -> Result<MediaBatch, MediaPipelineError> {
        let batch = self.load_batch(batch_id).await?;
        if batch.status != MediaBatchStatus::Ready {
            return Err(MediaPipelineError::BatchNotReady(batch.id));
        }
        Ok(batch)
    }

    /// Get the directory path for a batch (public accessor for QC engine)
    pub fn get_batch_dir(&self, batch_id: Uuid) -> PathBuf {
        self.batch_dir(batch_id)
    }

    /// Get a working directory for visual QC under the pipeline root
    pub fn visual_qc_work_dir(&self, batch_id: Uuid) -> PathBuf {
        self.inner.root.join("visual_qc").join(batch_id.to_string())
    }

    async fn process_download(
        &self,
        batch_id: Uuid,
        request: MediaBatchIngestRequest,
    ) -> Result<(), MediaPipelineError> {
        self.update_batch_status(batch_id, MediaBatchStatus::Downloading, None)
            .await?;

        let mut batch = self.load_batch(batch_id).await?;
        let dest_dir = self.batch_dir(batch_id);
        fs::create_dir_all(&dest_dir).await?;
        let dest_file = dest_dir.join("source.bin");

        match self
            .download_to_path(
                &Self::normalize_dropbox_url(&request.source_url),
                &dest_file,
                request.checksum_required,
            )
            .await
        {
            Ok((size, checksum)) => {
                batch.files = vec![MediaAsset {
                    filename: dest_file
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "source.bin".to_string()),
                    relative_path: dest_file
                        .strip_prefix(&dest_dir.parent().unwrap_or(&self.inner.root))
                        .unwrap_or(&dest_file)
                        .to_string_lossy()
                        .to_string(),
                    size_bytes: size,
                    checksum_sha256: checksum,
                }];
                batch.status = MediaBatchStatus::Ready;
                batch.updated_at = Utc::now();
                batch.last_error = None;
                if let Some(pool) = self.db_pool() {
                    self.persist_media_files(pool, batch.id, &batch.files).await?;
                }
                self.persist_batch(&batch).await?;
            }
            Err(err) => {
                tracing::error!("Download failed for batch {}: {}", batch_id, err);
                batch.status = MediaBatchStatus::Failed;
                batch.updated_at = Utc::now();
                batch.last_error = Some(err.to_string());
                self.persist_batch(&batch).await?;
                return Err(err);
            }
        }

        Ok(())
    }

    async fn process_local_directory(
        &self,
        batch_id: Uuid,
        request: MediaBatchIngestRequest,
    ) -> Result<MediaBatch, MediaPipelineError> {
        // Strip file:// prefix if present
        let dir_path = if let Some(stripped) = request.source_url.strip_prefix("file://") {
            PathBuf::from(stripped)
        } else {
            PathBuf::from(&request.source_url)
        };

        if !dir_path.exists() {
            return Err(MediaPipelineError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir_path.display()),
            )));
        }

        self.update_batch_status(batch_id, MediaBatchStatus::Downloading, None)
            .await?;

        let mut batch = self.load_batch(batch_id).await?;
        let batch_dir = self.batch_dir(batch_id);
        fs::create_dir_all(&batch_dir).await?;

        let mut assets = Vec::new();
        let media_extensions = [
            "mp4", "mov", "mxf", "avi", "mkv", "m4v", "mts", "mpg", "wmv",
        ];

        // Collect directories to scan (root + immediate subdirectories)
        let mut dirs_to_scan = vec![dir_path.clone()];
        let mut root_entries = fs::read_dir(&dir_path).await?;
        while let Some(entry) = root_entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                dirs_to_scan.push(path);
            }
        }

        // Scan all directories for media files
        for scan_dir in &dirs_to_scan {
            let mut entries = fs::read_dir(scan_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
                    .unwrap_or_default();

                if !media_extensions.contains(&ext.as_str()) {
                    continue;
                }

                let metadata = entry.metadata().await?;
                let filename = path
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Symlink into batch directory for uniform access
                let link_dest = batch_dir.join(&filename);
                if !link_dest.exists() {
                    tokio::fs::symlink(&path, &link_dest)
                        .await
                        .unwrap_or_else(|e| {
                            tracing::warn!("Symlink failed for {}: {}", filename, e)
                        });
                }

                // Compute checksum if requested
                let checksum = if request.checksum_required {
                    Self::compute_file_checksum(&path).await.ok()
                } else {
                    None
                };

                assets.push(MediaAsset {
                    filename: filename.clone(),
                    relative_path: format!("{}/{}", batch_id, filename),
                    size_bytes: metadata.len(),
                    checksum_sha256: checksum,
                });
            }
        }

        if assets.is_empty() {
            return Err(MediaPipelineError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No media files found in {}", dir_path.display()),
            )));
        }

        tracing::info!(
            "[MEDIA_PIPELINE] Local ingest: {} files ({} bytes total) from {} ({} dirs scanned)",
            assets.len(),
            assets.iter().map(|a| a.size_bytes).sum::<u64>(),
            dir_path.display(),
            dirs_to_scan.len()
        );

        batch.files = assets;
        batch.status = MediaBatchStatus::Ready;
        batch.updated_at = Utc::now();
        batch.last_error = None;

        if let Some(pool) = self.db_pool() {
            self.persist_media_files(pool, batch.id, &batch.files)
                .await?;
        }
        self.persist_batch(&batch).await?;

        Ok(batch)
    }

    async fn compute_file_checksum(path: &Path) -> Result<String, MediaPipelineError> {
        let data = fs::read(path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn progress_render(&self, job_id: Uuid) -> Result<(), MediaPipelineError> {
        self.update_render_status(job_id, RenderJobStatus::Rendering)
            .await?;
        sleep(Duration::from_secs(2)).await;
        self.update_render_status(job_id, RenderJobStatus::Complete)
            .await?;
        Ok(())
    }

    async fn download_to_path(
        &self,
        url: &str,
        dest: &Path,
        checksum: bool,
    ) -> Result<(u64, Option<String>), MediaPipelineError> {
        let response = self
            .inner
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?;

        let mut file = fs::File::create(dest).await?;
        let mut stream = response.bytes_stream();
        let mut total = 0u64;
        let mut hasher = checksum.then_some(Sha256::new());

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            total += chunk.len() as u64;
            if let Some(h) = hasher.as_mut() {
                h.update(&chunk);
            }
        }

        file.flush().await?;
        let checksum_value = hasher.map(|h| format!("{:x}", h.finalize()));
        Ok((total, checksum_value))
    }

    async fn persist_batch(&self, batch: &MediaBatch) -> Result<(), MediaPipelineError> {
        let path = self.batch_dir(batch.id).join("batch.json");
        self.persist_json(&path, batch).await?;
        if let Some(pool) = self.db_pool() {
            self.persist_batch_to_db(pool, batch).await?;
        }
        Ok(())
    }

    async fn persist_analysis(
        &self,
        analysis: &MediaBatchAnalysis,
    ) -> Result<(), MediaPipelineError> {
        fs::create_dir_all(self.analysis_path(analysis.batch_id)).await?;
        self.persist_json(&analysis.insights_path, analysis).await?;
        if let Some(pool) = self.db_pool() {
            self.persist_analysis_db(pool, analysis).await?;
        }
        Ok(())
    }

    async fn persist_session(&self, session: &EditSession) -> Result<(), MediaPipelineError> {
        let path = self.sessions_dir().join(format!("{}.json", session.id));
        self.persist_json(&path, session).await?;
        if let Some(pool) = self.db_pool() {
            self.persist_session_db(pool, session).await?;
        }
        Ok(())
    }

    async fn persist_render_job(&self, job: &RenderJob) -> Result<(), MediaPipelineError> {
        let path = self.renders_dir().join(format!("{}.json", job.id));
        self.persist_json(&path, job).await?;
        if let Some(pool) = self.db_pool() {
            self.persist_render_job_db(pool, job).await?;
        }
        Ok(())
    }

    async fn update_batch_status(
        &self,
        batch_id: Uuid,
        status: MediaBatchStatus,
        error: Option<String>,
    ) -> Result<(), MediaPipelineError> {
        let mut batch = self.load_batch(batch_id).await?;
        batch.status = status;
        batch.last_error = error;
        batch.updated_at = Utc::now();
        self.persist_batch(&batch).await
    }

    async fn update_render_status(
        &self,
        job_id: Uuid,
        status: RenderJobStatus,
    ) -> Result<(), MediaPipelineError> {
        let mut job = self.load_render_job(job_id).await?;
        job.status = status;
        job.updated_at = Utc::now();
        self.persist_render_job(&job).await
    }

    async fn load_batch(&self, batch_id: Uuid) -> Result<MediaBatch, MediaPipelineError> {
        let path = self.batch_dir(batch_id).join("batch.json");
        if !path.exists() {
            return Err(MediaPipelineError::BatchNotFound(batch_id));
        }
        let raw = fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&raw)?)
    }

    async fn load_session(&self, session_id: Uuid) -> Result<EditSession, MediaPipelineError> {
        let path = self.sessions_dir().join(format!("{}.json", session_id));
        if !path.exists() {
            return Err(MediaPipelineError::EditSessionNotFound(session_id));
        }
        let raw = fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&raw)?)
    }

    async fn load_render_job(&self, job_id: Uuid) -> Result<RenderJob, MediaPipelineError> {
        let path = self.renders_dir().join(format!("{}.json", job_id));
        if !path.exists() {
            return Err(MediaPipelineError::RenderJobNotFound(job_id));
        }
        let raw = fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&raw)?)
    }

    async fn persist_json<T: Serialize>(
        &self,
        path: &Path,
        value: &T,
    ) -> Result<(), MediaPipelineError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let raw = serde_json::to_string_pretty(value)?;
        fs::write(path, raw).await?;
        Ok(())
    }

    async fn persist_batch_to_db(
        &self,
        pool: &SqlitePool,
        batch: &MediaBatch,
    ) -> Result<(), MediaPipelineError> {
        let file_count = batch.files.len() as i64;
        let total_size: i64 = batch
            .files
            .iter()
            .map(|file| file.size_bytes as i64)
            .sum();
        let relative_paths: Vec<&str> = batch
            .files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();
        let metadata = json!({
            "referenceName": batch.reference_name,
            "relativePaths": relative_paths,
        })
        .to_string();
        let storage_tier = batch.storage_tier.as_str();
        let status = batch.status.as_str();

        sqlx::query!(
            r#"
            INSERT INTO media_batches (
                id, project_id, reference_name, source_url, storage_tier,
                checksum_required, status, file_count, total_size_bytes,
                last_error, metadata, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                project_id=excluded.project_id,
                reference_name=excluded.reference_name,
                source_url=excluded.source_url,
                storage_tier=excluded.storage_tier,
                checksum_required=excluded.checksum_required,
                status=excluded.status,
                file_count=excluded.file_count,
                total_size_bytes=excluded.total_size_bytes,
                last_error=excluded.last_error,
                metadata=excluded.metadata,
                updated_at=excluded.updated_at
            "#,
            batch.id,
            batch.project_id,
            batch.reference_name,
            batch.source_url,
            storage_tier,
            batch.checksum_required,
            status,
            file_count,
            total_size,
            batch.last_error,
            metadata,
            batch.created_at,
            batch.updated_at,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn persist_media_files(
        &self,
        pool: &SqlitePool,
        batch_id: Uuid,
        files: &[MediaAsset],
    ) -> Result<(), MediaPipelineError> {
        sqlx::query!("DELETE FROM media_files WHERE batch_id = ?", batch_id)
            .execute(pool)
            .await?;

        for asset in files {
            let metadata = json!({
                "relativePath": asset.relative_path,
            })
            .to_string();
            let file_id = Uuid::new_v4();
            let size_bytes = asset.size_bytes as i64;
            let created_at = Utc::now();

            sqlx::query!(
                r#"
                INSERT INTO media_files (
                    id, batch_id, filename, file_path, size_bytes, checksum_sha256,
                    duration_seconds, resolution, codec, fps, metadata, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                file_id,
                batch_id,
                asset.filename,
                asset.relative_path,
                size_bytes,
                asset.checksum_sha256,
                None::<f64>,
                None::<String>,
                None::<String>,
                None::<f64>,
                metadata,
                created_at,
            )
            .execute(pool)
            .await?;
        }

        Ok(())
    }

    async fn persist_analysis_db(
        &self,
        pool: &SqlitePool,
        analysis: &MediaBatchAnalysis,
    ) -> Result<(), MediaPipelineError> {
        let deliverable_targets = serde_json::to_string(&analysis.recommended_deliverables)?;
        let hero_moments = serde_json::to_string(&analysis.hero_moments)?;
        let insights = json!({
            "insightsPath": analysis.insights_path.to_string_lossy(),
        })
        .to_string();
        let passes_completed = analysis.passes_completed as i64;

        sqlx::query!(
            r#"
            INSERT INTO media_batch_analyses (
                id, batch_id, brief, summary, passes_completed,
                deliverable_targets, hero_moments, insights, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                brief=excluded.brief,
                summary=excluded.summary,
                passes_completed=excluded.passes_completed,
                deliverable_targets=excluded.deliverable_targets,
                hero_moments=excluded.hero_moments,
                insights=excluded.insights,
                created_at=excluded.created_at
            "#,
            analysis.id,
            analysis.batch_id,
            analysis.brief,
            analysis.summary,
            passes_completed,
            deliverable_targets,
            hero_moments,
            insights,
            analysis.created_at,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn persist_session_db(
        &self,
        pool: &SqlitePool,
        session: &EditSession,
    ) -> Result<(), MediaPipelineError> {
        let aspect_ratios = serde_json::to_string(&session.aspect_ratios)?;
        let timelines = serde_json::to_string(&session.timelines)?;
        let status = session.status.as_str();
        let created_at = session.created_at;
        let updated_at = session.updated_at;

        sqlx::query!(
            r#"
            INSERT INTO edit_sessions (
                id, batch_id, deliverable_type, aspect_ratios, reference_style,
                include_captions, imovie_project, status, timelines, metadata,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, '{}', ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                deliverable_type=excluded.deliverable_type,
                aspect_ratios=excluded.aspect_ratios,
                reference_style=excluded.reference_style,
                include_captions=excluded.include_captions,
                imovie_project=excluded.imovie_project,
                status=excluded.status,
                timelines=excluded.timelines,
                metadata=excluded.metadata,
                updated_at=excluded.updated_at
            "#,
            session.id,
            session.batch_id,
            session.deliverable_type,
            aspect_ratios,
            session.reference_style,
            session.include_captions,
            session.imovie_project,
            status,
            timelines,
            created_at,
            updated_at,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn persist_render_job_db(
        &self,
        pool: &SqlitePool,
        job: &RenderJob,
    ) -> Result<(), MediaPipelineError> {
        let destinations = serde_json::to_string(&job.destinations)?;
        let formats = serde_json::to_string(&job.formats)?;
        let priority = job.priority.as_str();
        let status = job.status.as_str();
        let created_at = job.created_at;
        let updated_at = job.updated_at;

        sqlx::query!(
            r#"
            INSERT INTO render_jobs (
                id, edit_session_id, destinations, formats, priority,
                status, progress_percent, last_error, output_urls, metadata,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, NULL, NULL, '[]', '{}', ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                destinations=excluded.destinations,
                formats=excluded.formats,
                priority=excluded.priority,
                status=excluded.status,
                updated_at=excluded.updated_at
            "#,
            job.id,
            job.edit_session_id,
            destinations,
            formats,
            priority,
            status,
            created_at,
            updated_at,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    fn batches_dir(&self) -> PathBuf {
        self.inner.root.join("batches")
    }

    fn sessions_dir(&self) -> PathBuf {
        self.inner.root.join("sessions")
    }

    fn renders_dir(&self) -> PathBuf {
        self.inner.root.join("renders")
    }

    fn batch_dir(&self, id: Uuid) -> PathBuf {
        self.batches_dir().join(id.to_string())
    }

    fn analysis_path(&self, batch_id: Uuid) -> PathBuf {
        self.batch_dir(batch_id).join("analysis")
    }

    fn normalize_dropbox_url(url: &str) -> String {
        if url.contains("dropbox.com") {
            if url.contains("?dl=") {
                return url.replace("?dl=0", "?dl=1");
            }
            return format!("{}?dl=1", url);
        }
        url.to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaBatchIngestRequest {
    pub source_url: String,
    pub reference_name: Option<String>,
    pub storage_tier: MediaStorageTier,
    pub checksum_required: bool,
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaStorageTier {
    Hot,
    Warm,
    Cold,
}

impl MediaStorageTier {
    pub fn from_str(value: &str) -> Result<Self, MediaPipelineError> {
        match value.to_lowercase().as_str() {
            "hot" => Ok(MediaStorageTier::Hot),
            "warm" => Ok(MediaStorageTier::Warm),
            "cold" => Ok(MediaStorageTier::Cold),
            other => Err(MediaPipelineError::UnknownStorageTier(other.to_string())),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MediaStorageTier::Hot => "hot",
            MediaStorageTier::Warm => "warm",
            MediaStorageTier::Cold => "cold",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaBatchStatus {
    Queued,
    Downloading,
    Ready,
    Analyzing,
    Analyzed,
    Failed,
}

impl MediaBatchStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaBatchStatus::Queued => "queued",
            MediaBatchStatus::Downloading => "downloading",
            MediaBatchStatus::Ready => "ready",
            MediaBatchStatus::Analyzing => "analyzing",
            MediaBatchStatus::Analyzed => "analyzed",
            MediaBatchStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaBatch {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub reference_name: Option<String>,
    pub source_url: String,
    pub storage_tier: MediaStorageTier,
    pub checksum_required: bool,
    pub status: MediaBatchStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub files: Vec<MediaAsset>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    pub filename: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub checksum_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaBatchAnalysisRequest {
    pub batch_id: Uuid,
    pub brief: String,
    pub passes: u32,
    pub deliverable_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaBatchAnalysis {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub brief: String,
    pub summary: String,
    pub hero_moments: Vec<HeroMoment>,
    pub recommended_deliverables: Vec<String>,
    pub passes_completed: u32,
    pub created_at: DateTime<Utc>,
    pub insights_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroMoment {
    pub timestamp: String,
    pub description: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSessionRequest {
    pub batch_id: Uuid,
    pub deliverable_type: String,
    pub aspect_ratios: Vec<String>,
    pub reference_style: Option<String>,
    pub include_captions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSession {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub deliverable_type: String,
    pub aspect_ratios: Vec<String>,
    pub include_captions: bool,
    pub reference_style: Option<String>,
    pub imovie_project: String,
    pub timelines: Vec<RenderTimeline>,
    pub status: EditSessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderTimeline {
    pub aspect_ratio: String,
    pub estimated_duration_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EditSessionStatus {
    Assembling,
    NeedsReview,
    Approved,
    Rendering,
    Complete,
    Failed,
}

impl EditSessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EditSessionStatus::Assembling => "assembling",
            EditSessionStatus::NeedsReview => "needsreview",
            EditSessionStatus::Approved => "approved",
            EditSessionStatus::Rendering => "rendering",
            EditSessionStatus::Complete => "complete",
            EditSessionStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJobRequest {
    pub edit_session_id: Uuid,
    pub destinations: Vec<String>,
    pub formats: Vec<String>,
    pub priority: VideoRenderPriority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderJob {
    pub id: Uuid,
    pub edit_session_id: Uuid,
    pub destinations: Vec<String>,
    pub formats: Vec<String>,
    pub priority: VideoRenderPriority,
    pub status: RenderJobStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VideoRenderPriority {
    Low,
    Standard,
    Rush,
}

impl VideoRenderPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            VideoRenderPriority::Low => "low",
            VideoRenderPriority::Standard => "standard",
            VideoRenderPriority::Rush => "rush",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RenderJobStatus {
    Queued,
    Rendering,
    Complete,
    Failed,
}

impl RenderJobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RenderJobStatus::Queued => "queued",
            RenderJobStatus::Rendering => "rendering",
            RenderJobStatus::Complete => "complete",
            RenderJobStatus::Failed => "failed",
        }
    }
}
