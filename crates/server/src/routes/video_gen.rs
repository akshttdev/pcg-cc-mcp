use axum::{
    Extension, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::Response,
    routing::{delete, get, post},
    Json,
};
use db::models::{
    avatar_profile::{AvatarProfile, CreateAvatarProfile, UpdateAvatarProfile},
    video_job::{CreateVideoJob, VideoJob},
};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf};
use tokio::fs;
use uuid::Uuid;

use deployment::Deployment;

use crate::{DeploymentImpl, error::ApiError, middleware::AccessContext};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn app_base_url() -> String {
    std::env::var("APP_BASE_URL")
        .unwrap_or_else(|_| "https://dashboard.powerclubglobal.com".to_string())
}

fn audio_dir() -> PathBuf {
    PathBuf::from("dev_assets/video_gen/audio")
}

fn video_dir() -> PathBuf {
    PathBuf::from("dev_assets/video_gen/video")
}

// ---------------------------------------------------------------------------
// Routers
// ---------------------------------------------------------------------------

/// Protected routes — require JWT auth
pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Avatar profiles
        .route("/video-gen/avatars", get(list_avatars).post(create_avatar))
        .route(
            "/video-gen/avatars/{id}",
            get(get_avatar).patch(update_avatar).delete(delete_avatar),
        )
        // Video jobs
        .route("/video-gen/jobs", get(list_jobs).post(create_job))
        .route("/video-gen/jobs/{id}", get(get_job).delete(delete_job))
        .with_state(deployment.clone())
}

/// Public routes — no auth (audio/video file serving so HeyGen can fetch)
pub fn public_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/video-gen/audio/{job_id}", get(serve_audio))
        .route("/video-gen/video/{job_id}", get(serve_video))
        .with_state(deployment.clone())
}

// ---------------------------------------------------------------------------
// Avatar Profile Handlers
// ---------------------------------------------------------------------------

async fn list_avatars(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<AvatarProfile>>, ApiError> {
    let pool = &deployment.db().pool;
    let org_id = params
        .get("org_id")
        .and_then(|s| Uuid::parse_str(s).ok());
    let avatars = AvatarProfile::list(pool, org_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(avatars))
}

async fn create_avatar(
    State(deployment): State<DeploymentImpl>,
    Extension(ctx): Extension<AccessContext>,
    Json(body): Json<CreateAvatarProfile>,
) -> Result<(StatusCode, Json<AvatarProfile>), ApiError> {
    let pool = &deployment.db().pool;
    let _ctx = ctx; // user is authenticated; skip FK binding until blob/uuid alignment resolved
    let created_by: Option<uuid::Uuid> = None;
    let avatar = AvatarProfile::create(pool, body, created_by)
        .await
        .map_err(ApiError::Database)?;
    Ok((StatusCode::CREATED, Json(avatar)))
}

async fn get_avatar(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = &deployment.db().pool;
    let avatar = AvatarProfile::find(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;
    Ok(Json(avatar))
}

async fn update_avatar(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateAvatarProfile>,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = &deployment.db().pool;
    let avatar = AvatarProfile::update(pool, id, body)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;
    Ok(Json(avatar))
}

async fn delete_avatar(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = AvatarProfile::delete(pool, id)
        .await
        .map_err(ApiError::Database)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("avatar {}", id)))
    }
}

// ---------------------------------------------------------------------------
// Video Job Handlers
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ListJobsParams {
    avatar_id: Option<Uuid>,
}

async fn list_jobs(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<ListJobsParams>,
) -> Result<Json<Vec<VideoJob>>, ApiError> {
    let pool = &deployment.db().pool;
    let jobs = VideoJob::list(pool, params.avatar_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(jobs))
}

async fn create_job(
    State(deployment): State<DeploymentImpl>,
    Extension(ctx): Extension<AccessContext>,
    Json(body): Json<CreateVideoJob>,
) -> Result<(StatusCode, Json<VideoJob>), ApiError> {
    let pool = deployment.db().pool.clone();
    let _ctx = ctx; // user is authenticated; skip FK binding until blob/uuid alignment resolved
    let created_by: Option<uuid::Uuid> = None;

    // Look up the avatar for its voice ID
    let avatar = AvatarProfile::find(&pool, body.avatar_profile_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", body.avatar_profile_id)))?;

    let job = VideoJob::create(&pool, body, created_by)
        .await
        .map_err(ApiError::Database)?;

    // Spawn the async pipeline
    let job_id = job.id;
    let pool2 = pool.clone();
    let script = job.script_text.clone();
    let background_url = job.background_url.clone();

    tokio::spawn(async move {
        if let Err(e) = produce_job(&pool2, job_id, &avatar, &script, background_url.as_deref()).await {
            tracing::error!("video pipeline failed for job {}: {}", job_id, e);
            let _ = VideoJob::update_status(&pool2, job_id, "failed", Some(&e.to_string())).await;
        }
    });

    Ok((StatusCode::CREATED, Json(job)))
}

async fn get_job(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<VideoJob>, ApiError> {
    let pool = &deployment.db().pool;
    let job = VideoJob::find(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("job {}", id)))?;
    Ok(Json(job))
}

async fn delete_job(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = VideoJob::delete(pool, id)
        .await
        .map_err(ApiError::Database)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("job {}", id)))
    }
}

// ---------------------------------------------------------------------------
// File Serving (public — HeyGen needs to fetch audio)
// ---------------------------------------------------------------------------

async fn serve_audio(Path(job_id): Path<String>) -> Result<Response, ApiError> {
    let path = audio_dir().join(format!("{}.mp3", job_id));
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("audio {}", job_id)))?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "audio/mpeg")
        .body(Body::from(data))
        .unwrap())
}

async fn serve_video(Path(job_id): Path<String>) -> Result<Response, ApiError> {
    let path = video_dir().join(format!("{}.mp4", job_id));
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("video {}", job_id)))?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "video/mp4")
        .body(Body::from(data))
        .unwrap())
}

// ---------------------------------------------------------------------------
// Async pipeline
// ---------------------------------------------------------------------------

async fn produce_job(
    pool: &sqlx::SqlitePool,
    job_id: Uuid,
    avatar: &AvatarProfile,
    script: &str,
    background_url: Option<&str>,
) -> anyhow::Result<()> {
    let el_key = std::env::var("ELEVENLABS_API_KEY")?;
    let hg_key = std::env::var("HEYGEN_API_KEY")?;

    // Step 1: TTS with word-level timestamps
    tracing::info!("video job {}: generating TTS with timestamps", job_id);
    VideoJob::update_status(pool, job_id, "tts_generating", None).await?;

    let tts = video_gen::tts::generate_tts(&el_key, &avatar.elevenlabs_voice_id, script).await?;

    tracing::info!(
        "video job {}: {} words aligned over {} bytes",
        job_id,
        tts.word_alignment.len(),
        tts.audio_bytes.len()
    );

    // Persist MP3 to disk
    let audio_path = audio_dir().join(format!("{}.mp3", job_id));
    if let Some(parent) = audio_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&audio_path, &tts.audio_bytes).await?;

    // Convert MP3 → WAV — HeyGen's speech recognition extracts word timing
    // much more reliably from uncompressed PCM, enabling the expressive avatar.
    let wav_path = audio_dir().join(format!("{}.wav", job_id));
    let ffmpeg_out = tokio::process::Command::new("ffmpeg")
        .args([
            "-y", "-i",
            audio_path.to_str().unwrap(),
            "-ar", "16000",   // 16 kHz — optimal for speech recognition
            "-ac", "1",       // mono
            "-f", "wav",
            wav_path.to_str().unwrap(),
        ])
        .output()
        .await?;

    if !ffmpeg_out.status.success() {
        let stderr = String::from_utf8_lossy(&ffmpeg_out.stderr);
        anyhow::bail!("ffmpeg conversion failed: {}", stderr);
    }

    let wav_bytes = fs::read(&wav_path).await?;
    tracing::info!("video job {}: WAV {} bytes, uploading to HeyGen CDN", job_id, wav_bytes.len());

    // Upload WAV to HeyGen CDN
    let audio_url = video_gen::heygen::upload_audio_wav(&hg_key, wav_bytes).await?;

    VideoJob::update_tts_done(pool, job_id, &audio_url).await?;
    tracing::info!("video job {}: TTS done, audio at {}", job_id, audio_url);

    // Step 2: HeyGen generate
    let heygen_avatar_id = avatar
        .heygen_avatar_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("avatar has no heygen_avatar_id set"))?;

    tracing::info!("video job {}: submitting to HeyGen", job_id);
    let heygen_video_id =
        video_gen::heygen::generate_with_audio(&hg_key, heygen_avatar_id, &audio_url, background_url)
            .await?;

    VideoJob::update_heygen_started(pool, job_id, &heygen_video_id).await?;
    tracing::info!(
        "video job {}: HeyGen video_id = {}",
        job_id,
        heygen_video_id
    );

    // Step 3: Poll until done (up to 15 min)
    let mut attempts = 0u32;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        attempts += 1;

        let status = video_gen::heygen::poll_status(&hg_key, &heygen_video_id).await?;
        tracing::debug!("video job {} poll {}: {}", job_id, attempts, status.status);

        match status.status.as_str() {
            "completed" => {
                let video_url = status
                    .video_url
                    .ok_or_else(|| anyhow::anyhow!("HeyGen completed but no video_url"))?;

                // Download video to local disk (backup)
                let vid_bytes = reqwest::get(&video_url).await?.bytes().await?;
                let vid_path = video_dir().join(format!("{}.mp4", job_id));
                if let Some(parent) = vid_path.parent() {
                    fs::create_dir_all(parent).await?;
                }
                fs::write(&vid_path, &vid_bytes).await?;

                // Use HeyGen CDN URL as the canonical final_video_url (publicly accessible)
                VideoJob::update_completed(
                    pool,
                    job_id,
                    &video_url,  // final_video_url = HeyGen CDN
                    &video_url,  // raw_video_url = same
                    status.thumbnail_url.as_deref(),
                    status.duration,
                )
                .await?;

                tracing::info!("video job {} complete: {}", job_id, video_url);
                return Ok(());
            }
            "failed" => {
                let err = status.error.unwrap_or_else(|| "HeyGen failed".into());
                anyhow::bail!("HeyGen render failed: {}", err);
            }
            _ => {
                if attempts >= 45 {
                    anyhow::bail!("HeyGen timed out after 15 minutes");
                }
                // still processing
            }
        }
    }
}
