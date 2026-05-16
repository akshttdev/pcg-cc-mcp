use std::{collections::HashMap, path::PathBuf};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use db::models::{
    avatar_profile::{AvatarProfile, CreateAvatarProfile, UpdateAvatarProfile},
    video_job::{CreateVideoJob, VideoJob},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::{error::ApiError, middleware::AccessContext, DeploymentImpl};

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
        // Cinematic film generator
        .route("/video-gen/birthday-film", post(birthday_film))
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
    let org_id = params.get("org_id").and_then(|s| Uuid::parse_str(s).ok());
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
    let _ctx = ctx;
    let created_by: Option<uuid::Uuid> = None;

    // Look up the avatar for its voice ID
    let avatar = AvatarProfile::find(&pool, body.avatar_profile_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", body.avatar_profile_id)))?;

    // Capture dimensions before body is consumed by VideoJob::create
    let width = body.width.unwrap_or(1280);
    let height = body.height.unwrap_or(720);

    let job = VideoJob::create(&pool, body, created_by)
        .await
        .map_err(ApiError::Database)?;

    // Spawn the async pipeline
    let job_id = job.id;
    let pool2 = pool.clone();
    let script = job.script_text.clone();
    let background_url = job.background_url.clone();

    tokio::spawn(async move {
        if let Err(e) = produce_job(
            &pool2,
            job_id,
            &avatar,
            &script,
            background_url.as_deref(),
            width,
            height,
        )
        .await
        {
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
// Birthday Film — ₿ODHI: THE SIGNAL
// ---------------------------------------------------------------------------

/// Sami Satoshi avatar profile UUID (active, HeyGen ID 07326f6a4ba54f5ab407f78fd9703cb2)
const SAMI_SATOSHI_AVATAR_ID: &str = "528c5bf7-364c-40fd-a593-86d90ca7acaa";

/// The full cinematic narration for "₿ODHI: THE SIGNAL"
const BODHI_SIGNAL_SCRIPT: &str = "\
Most people celebrate another year surviving the world.\n\
But some are built to redesign it.\n\n\
Empires aren't inherited.\n\
They're engineered.\n\
Line by line.\n\
Signal by signal.\n\
Node by node.\n\n\
While others consumed the future...\n\
One man chose to build it.\n\n\
The weight of vision is heavy.\n\
To dream beyond your time...\n\
is to walk alone, long before others understand.\n\n\
But history belongs to the people willing to carry the signal.\n\n\
PowerClub Global. Alpha Protocol. Omega Wireless.\n\n\
Happy birthday, Bodhi.\n\n\
The network is just beginning.";

#[derive(Debug, Serialize)]
struct BirthdayFilmResponse {
    job: VideoJob,
    shot_plan: Vec<ShotBrief>,
    message: String,
}

#[derive(Debug, Serialize)]
struct ShotBrief {
    index: u8,
    scene: &'static str,
    visual_prompt: &'static str,
    narration: &'static str,
    duration_sec: u8,
}

/// 12-shot vertical storyboard for "₿ODHI: THE SIGNAL"
fn bodhi_shot_plan() -> Vec<ShotBrief> {
    vec![
        ShotBrief {
            index: 1,
            scene: "The Awakening — Black void",
            visual_prompt: "Black screen, a single golden Omega symbol slowly materializing from darkness, low golden glow pulsing, dramatic cinematic atmosphere, 9:16 vertical, Blade Runner 2049 aesthetic, no text",
            narration: "Most people celebrate another year surviving the world.",
            duration_sec: 5,
        },
        ShotBrief {
            index: 2,
            scene: "Miami 3AM — Ocean seawall",
            visual_prompt: "Miami seawall at 3AM, massive waves crashing under a stormy sky, neon city reflections on wet pavement, cinematic rain, dramatic fog, golden and teal tones, 9:16 vertical portrait, ultra-cinematic",
            narration: "But some are built to redesign it.",
            duration_sec: 4,
        },
        ShotBrief {
            index: 3,
            scene: "The Builder — AI compute clusters",
            visual_prompt: "Massive server racks powering on in a dark cathedral room, blinking amber and blue indicator lights, rising steam, god rays cutting through darkness, gold metallic sheen, ultra-cinematic 9:16",
            narration: "Empires aren't inherited. They're engineered.",
            duration_sec: 5,
        },
        ShotBrief {
            index: 4,
            scene: "Alpha Protocol globe — mesh network",
            visual_prompt: "Holographic glowing Earth globe suspended in darkness, gold mesh network lines connecting cities worldwide, Alpha Protocol insignia, decentralized node pulses, cinematic 9:16 vertical, sci-fi luxury",
            narration: "Line by line. Signal by signal. Node by node.",
            duration_sec: 5,
        },
        ShotBrief {
            index: 5,
            scene: "Omega Node rack — gold glow",
            visual_prompt: "Close-up of futuristic Omega Wireless server rack glowing gold, sovereign compute hardware, LoRa antennas on dark rooftop, Miami city lights in background bokeh, 9:16 vertical cinematic",
            narration: "While others consumed the future...",
            duration_sec: 4,
        },
        ShotBrief {
            index: 6,
            scene: "Sprinter van — highway night",
            visual_prompt: "Blacked-out Mercedes Sprinter van with Omega gold logo, racing down a dark highway, tunnel light trails, motion blur, Miami neon reflections, rain-slicked roads, cinematic 9:16 portrait",
            narration: "One man chose to build it.",
            duration_sec: 4,
        },
        ShotBrief {
            index: 7,
            scene: "Motorcycle — tunnel rip",
            visual_prompt: "Supermoto motorcycle ripping through a neon-lit tunnel, first-person low angle, speed blur, golden sparks, cinematic slow motion, dramatic depth of field, 9:16 vertical Tron aesthetic",
            narration: "The weight of vision is heavy.",
            duration_sec: 4,
        },
        ShotBrief {
            index: 8,
            scene: "Eclipse GST — neon streets",
            visual_prompt: "1996 Mitsubishi Eclipse GST accelerating through rain-soaked neon streets at night, teal and purple reflections, cinematic car photography, motion blur, 9:16 vertical portrait ultra-luxury",
            narration: "To dream beyond your time...",
            duration_sec: 4,
        },
        ShotBrief {
            index: 9,
            scene: "Lone figure — rooftop ocean view",
            visual_prompt: "Lone figure in black robe standing on a glass rooftop overlooking Miami before sunrise, city glowing below, ocean horizon, spiritual solitude, cinematic wide angle 9:16 vertical",
            narration: "is to walk alone, long before others understand.",
            duration_sec: 5,
        },
        ShotBrief {
            index: 10,
            scene: "Final ascension — Omega constellation",
            visual_prompt: "Thousands of golden light nodes activating across a world map, forming the Omega Ω symbol constellation, digital gold rain transforming into stars, celestial AI halo, epic cinematic 9:16",
            narration: "But history belongs to the people willing to carry the signal.",
            duration_sec: 6,
        },
        ShotBrief {
            index: 11,
            scene: "Title card — Happy Birthday ₿odhi",
            visual_prompt: "Cinematic title card on deep black background: HAPPY BIRTHDAY ₿ODHI in bold golden Bitcoin-B typography, glowing halo effect, particle light trails, 9:16 vertical luxury brand aesthetic",
            narration: "Happy birthday, Bodhi.",
            duration_sec: 5,
        },
        ShotBrief {
            index: 12,
            scene: "Brand outro — The network begins",
            visual_prompt: "Three logos in golden light on black: Omega Wireless, Alpha Protocol, PowerClub Global. Tagline: BUILDING THE DECENTRALIZED FUTURE. Fade out to black, 9:16 cinematic brand card",
            narration: "The network is just beginning.",
            duration_sec: 5,
        },
    ]
}

/// POST /api/video-gen/birthday-film
/// Fires the full "₿ODHI: THE SIGNAL" vertical cinematic reel via Sami Satoshi + ElevenLabs → HeyGen.
/// Returns the video job + complete 12-shot production brief with ComfyUI/Midjourney prompts.
async fn birthday_film(
    State(deployment): State<DeploymentImpl>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<Json<BirthdayFilmResponse>, ApiError> {
    let pool = deployment.db().pool.clone();
    let _ctx = ctx;

    // Look up Sami Satoshi
    let avatar_id = Uuid::parse_str(SAMI_SATOSHI_AVATAR_ID)
        .map_err(|_| ApiError::InternalError("Invalid Sami Satoshi UUID".into()))?;
    let avatar = AvatarProfile::find(&pool, avatar_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("Sami Satoshi avatar not found".into()))?;

    // Create the video job — vertical 9:16 (720×1280)
    let job = VideoJob::create(
        &pool,
        CreateVideoJob {
            avatar_profile_id: avatar_id,
            script_text: BODHI_SIGNAL_SCRIPT.to_string(),
            background_url: None, // pure black — cinematic
            width: Some(720),
            height: Some(1280),
        },
        None,
    )
    .await
    .map_err(ApiError::Database)?;

    // Spawn HeyGen native TTS pipeline (vertical 720×1280) — bypasses ElevenLabs entirely
    let job_id = job.id;
    let pool2 = pool.clone();
    let script = job.script_text.clone();
    let heygen_avatar_id = avatar
        .heygen_avatar_id
        .clone()
        .ok_or_else(|| ApiError::InternalError("Sami Satoshi has no HeyGen avatar ID".into()))?;

    tokio::spawn(async move {
        if let Err(e) =
            produce_birthday_film_heygen_tts(&pool2, job_id, &heygen_avatar_id, &script).await
        {
            tracing::error!("₿ODHI birthday film failed for job {}: {}", job_id, e);
            let _ = VideoJob::update_status(&pool2, job_id, "failed", Some(&e.to_string())).await;
        }
    });

    Ok(Json(BirthdayFilmResponse {
        job,
        shot_plan: bodhi_shot_plan(),
        message: "₿ODHI: THE SIGNAL — vertical 9:16 cinematic reel is rendering. ElevenLabs narration → HeyGen vertical video. Poll /api/video-gen/jobs/{id} for status.".into(),
    }))
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
    width: u32,
    height: u32,
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
            "-y",
            "-i",
            audio_path.to_str().unwrap(),
            "-ar",
            "16000", // 16 kHz — optimal for speech recognition
            "-ac",
            "1", // mono
            "-f",
            "wav",
            wav_path.to_str().unwrap(),
        ])
        .output()
        .await?;

    if !ffmpeg_out.status.success() {
        let stderr = String::from_utf8_lossy(&ffmpeg_out.stderr);
        anyhow::bail!("ffmpeg conversion failed: {}", stderr);
    }

    let wav_bytes = fs::read(&wav_path).await?;
    tracing::info!(
        "video job {}: WAV {} bytes, uploading to HeyGen CDN",
        job_id,
        wav_bytes.len()
    );

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
    let heygen_video_id = video_gen::heygen::generate_with_audio(
        &hg_key,
        heygen_avatar_id,
        &audio_url,
        background_url,
        width,
        height,
    )
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
                    &video_url, // final_video_url = HeyGen CDN
                    &video_url, // raw_video_url = same
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

/// HeyGen-native TTS pipeline for the birthday film — no ElevenLabs dependency.
/// Uses HeyGen's built-in voice synthesis directly from the script text.
async fn produce_birthday_film_heygen_tts(
    pool: &sqlx::SqlitePool,
    job_id: Uuid,
    heygen_avatar_id: &str,
    script: &str,
) -> anyhow::Result<()> {
    let hg_key = std::env::var("HEYGEN_API_KEY")?;

    tracing::info!(
        "birthday film job {}: submitting to HeyGen native TTS (720×1280 vertical)",
        job_id
    );
    VideoJob::update_status(pool, job_id, "avatar_generating", None).await?;

    let heygen_video_id = video_gen::heygen::generate_with_text(
        &hg_key,
        heygen_avatar_id,
        script,
        Some("2lZkTvkzMXw1TFX4PKlb"), // Deep Authoritative — cinematic narrator
        None,                         // pure black background
        720,                          // vertical 9:16
        1280,
    )
    .await?;

    VideoJob::update_heygen_started(pool, job_id, &heygen_video_id).await?;
    tracing::info!(
        "birthday film job {}: HeyGen video_id = {}",
        job_id,
        heygen_video_id
    );

    // Poll until HeyGen completes (up to 20 min for longer scripts)
    let mut attempts = 0u32;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        attempts += 1;

        let status = video_gen::heygen::poll_status(&hg_key, &heygen_video_id).await?;
        tracing::debug!(
            "birthday film job {} poll {}: {}",
            job_id,
            attempts,
            status.status
        );

        match status.status.as_str() {
            "completed" => {
                let video_url = status
                    .video_url
                    .ok_or_else(|| anyhow::anyhow!("HeyGen completed but no video_url"))?;

                // Download and cache locally
                let vid_bytes = reqwest::get(&video_url).await?.bytes().await?;
                let vid_path = video_dir().join(format!("{}.mp4", job_id));
                if let Some(parent) = vid_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&vid_path, &vid_bytes).await?;

                VideoJob::update_completed(
                    pool,
                    job_id,
                    &video_url,
                    &video_url,
                    status.thumbnail_url.as_deref(),
                    status.duration,
                )
                .await?;

                tracing::info!("₿ODHI birthday film {} COMPLETE: {}", job_id, video_url);
                return Ok(());
            }
            "failed" => {
                let err = status.error.unwrap_or_else(|| "HeyGen failed".into());
                anyhow::bail!("HeyGen render failed: {}", err);
            }
            _ => {
                if attempts >= 60 {
                    anyhow::bail!("HeyGen timed out after 20 minutes");
                }
            }
        }
    }
}
