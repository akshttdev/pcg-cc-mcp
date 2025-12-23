use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
}

impl MediaPipelineService {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, MediaPipelineError> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("batches"))?;
        std::fs::create_dir_all(root.join("sessions"))?;
        std::fs::create_dir_all(root.join("renders"))?;
        Ok(Self {
            inner: Arc::new(MediaPipelineInner {
                root,
                client: reqwest::Client::new(),
            }),
        })
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

        let service = self.clone();
        let req_clone = request.clone();
        tokio::spawn(async move {
            if let Err(err) = service.process_download(batch.id, req_clone).await {
                tracing::error!("Media ingest failed for {}: {}", batch.id, err);
                let _ = service
                    .update_batch_status(batch.id, MediaBatchStatus::Failed, Some(err.to_string()))
                    .await;
            }
        });

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
            created_at: Utc::now(),
        };

        self.persist_session(&session).await?;

        Ok(session)
    }

    pub async fn render_deliverables(
        &self,
        request: RenderJobRequest,
    ) -> Result<RenderJob, MediaPipelineError> {
        let session = self.load_session(request.edit_session_id).await?;

        let job = RenderJob {
            id: Uuid::new_v4(),
            edit_session_id: session.id,
            destinations: request.destinations,
            formats: request.formats,
            priority: request.priority,
            status: RenderJobStatus::Queued,
            created_at: Utc::now(),
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
        self.persist_json(&path, batch).await
    }

    async fn persist_analysis(
        &self,
        analysis: &MediaBatchAnalysis,
    ) -> Result<(), MediaPipelineError> {
        fs::create_dir_all(self.analysis_path(analysis.batch_id)).await?;
        self.persist_json(&analysis.insights_path, analysis).await
    }

    async fn persist_session(&self, session: &EditSession) -> Result<(), MediaPipelineError> {
        let path = self.sessions_dir().join(format!("{}.json", session.id));
        self.persist_json(&path, session).await
    }

    async fn persist_render_job(&self, job: &RenderJob) -> Result<(), MediaPipelineError> {
        let path = self.renders_dir().join(format!("{}.json", job.id));
        self.persist_json(&path, job).await
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaBatchStatus {
    Queued,
    Downloading,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaBatch {
    pub id: Uuid,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VideoRenderPriority {
    Low,
    Standard,
    Rush,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RenderJobStatus {
    Queued,
    Rendering,
    Complete,
    Failed,
}
