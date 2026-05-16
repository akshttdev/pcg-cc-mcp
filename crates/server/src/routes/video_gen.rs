use std::{collections::HashMap, path::PathBuf};

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use base64::Engine as _;
use db::{
    db_uuid::DbUuid,
    models::{
        avatar_profile::{AvatarProfile, CreateAvatarProfile, UpdateAvatarProfile},
        video_job::{CreateVideoJob, VideoJob},
    },
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

use crate::{
    error::ApiError,
    helpers::avatar_access::{require_avatar_org_access, require_video_job_org_access},
    middleware::AccessContext,
    routes::avatar_engine,
    DeploymentImpl,
};
use crate::{error::ApiError, middleware::AccessContext, DeploymentImpl};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
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

fn avatar_dir(avatar_id: Uuid) -> PathBuf {
    PathBuf::from("dev_assets/avatars").join(avatar_id.to_string())
}

fn avatar_public_base(avatar_id: Uuid) -> String {
    format!("/api/video-gen/avatars/{avatar_id}")
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
        // Avatar Profile Engine
        .route(
            "/video-gen/avatars/{id}/upload-reference",
            post(upload_reference).layer(DefaultBodyLimit::max(20 * 1024 * 1024)), // 20MB
        )
        .route(
            "/video-gen/avatars/{id}/generate-profile",
            post(generate_profile),
        )
        .route("/video-gen/avatars/{id}/render-motion", post(render_motion))
        // Avatar engine asset serving — auth-required to prevent UUID-guessing
        // leaks across orgs. HeyGen / external consumers are gone, frontend
        // sends cookies, so authenticating these is feasible.
        .route(
            "/video-gen/avatars/{id}/reference.png",
            get(serve_reference),
        )
        .route("/video-gen/avatars/{id}/shots/{slot}", get(serve_shot))
        .route(
            "/video-gen/avatars/{id}/motion/{clip}",
            get(serve_motion_clip),
        )
        // Video jobs
        .route("/video-gen/jobs", get(list_jobs).post(create_job))
        .route("/video-gen/jobs/{id}", get(get_job).delete(delete_job))
        // Video Studio inventory
        .route("/video-gen/files", get(list_video_files))
        .route("/video-gen/overlays", get(list_overlay_episodes))
        // Cinematic film generator
        .route("/video-gen/birthday-film", post(birthday_film))
        .with_state(deployment.clone())
}

/// Public routes — no auth. Legacy VideoJob serve endpoints only.
pub fn public_router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/video-gen/audio/{job_id}", get(serve_audio))
        .route("/video-gen/video/{job_id}", get(serve_video))
        .route("/video-gen/final/{job_id}", get(serve_final_video))
        .with_state(deployment.clone())
}

// ---------------------------------------------------------------------------
// Avatar Profile Handlers
// ---------------------------------------------------------------------------

async fn list_avatars(
    State(deployment): State<DeploymentImpl>,
    Extension(ctx): Extension<AccessContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<AvatarProfile>>, ApiError> {
    let pool = &deployment.db().pool;
    // Per rust-standards: parse via DbUuid, then convert to uuid::Uuid for the
    // model layer (which still uses uuid::Uuid until Phase 2.2 follow-up).
    let org_id = params
        .get("org_id")
        .and_then(|s| DbUuid::parse(s).ok())
        .and_then(|d| Uuid::parse_str(d.as_str()).ok());

    match org_id {
        Some(oid) => {
            ctx.require_org_membership(pool, &oid.to_string()).await?;
        }
        None if !ctx.is_admin => {
            return Err(ApiError::BadRequest(
                "org_id query parameter required".to_string(),
            ));
        }
        None => {} // admin may list across orgs
    }

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

    // Access control: caller must be a member of the target org (or admin).
    match body.organization_id {
        Some(oid) => {
            ctx.require_org_membership(pool, &oid.to_string()).await?;
        }
        None if !ctx.is_admin => {
            return Err(ApiError::BadRequest("organization_id required".to_string()));
        }
        None => {} // admin may create un-scoped avatars
    }

    // Wire created_by from auth context so per-user billing attribution works.
    // ctx.user_id is a DbUuid (validated by auth middleware); convert to
    // uuid::Uuid until the model migrates to DbUuid in the Phase 2.2 follow-up.
    let created_by = Uuid::parse_str(ctx.user_id.as_str()).ok();
    let avatar = AvatarProfile::create(pool, body, created_by)
        .await
        .map_err(ApiError::Database)?;
    Ok((StatusCode::CREATED, Json(avatar)))
}

async fn get_avatar(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = &deployment.db().pool;
    let avatar = require_avatar_org_access(&ctx, pool, id).await?;
    Ok(Json(avatar))
}

async fn update_avatar(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
    Json(body): Json<UpdateAvatarProfile>,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = &deployment.db().pool;
    require_avatar_org_access(&ctx, pool, id).await?;
    let avatar = AvatarProfile::update(pool, id, body)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;
    Ok(Json(avatar))
}

async fn delete_avatar(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    require_avatar_org_access(&ctx, pool, id).await?;
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
// Avatar Profile Engine — upload reference + trigger generation
// ---------------------------------------------------------------------------

/// POST /api/video-gen/avatars/:id/upload-reference
///
/// Accepts multipart form-data with a single `image` field. Persists the bytes
/// to `dev_assets/avatars/<id>/reference.png` and stamps the URL on the avatar
/// record.
async fn upload_reference(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
    mut multipart: Multipart,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = &deployment.db().pool;

    // Access control + existence check in one call.
    require_avatar_org_access(&ctx, pool, id).await?;

    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::BadRequest(format!("multipart: {e}")))?
    {
        if field.name() == Some("image") {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("read image bytes: {e}")))?;
            bytes = Some(data.to_vec());
            break;
        }
    }
    let bytes = bytes.ok_or_else(|| ApiError::BadRequest("missing `image` field".to_string()))?;

    // Field-level size cap (the 20MB body limit is a backstop).
    const MAX_REFERENCE_BYTES: usize = 10 * 1024 * 1024;
    if bytes.len() > MAX_REFERENCE_BYTES {
        return Err(ApiError::BadRequest(format!(
            "image too large ({} bytes, max 10 MB)",
            bytes.len()
        )));
    }

    // Magic-byte sniff — reject anything that isn't a real PNG or JPEG.
    // The bytes get base64-fed into gpt-image-1 and served back to clients, so
    // a sniff vs claim mismatch is a real risk (SVG-with-JS, EXE renamed .png).
    let kind = infer::get(&bytes)
        .ok_or_else(|| ApiError::BadRequest("could not determine image type".to_string()))?;
    let mime = kind.mime_type();
    if mime != "image/png" && mime != "image/jpeg" {
        return Err(ApiError::BadRequest(format!(
            "unsupported image type {mime} — must be PNG or JPEG"
        )));
    }

    let dir = avatar_dir(id);
    fs::create_dir_all(&dir)
        .await
        .map_err(|e| ApiError::InternalError(format!("mkdir: {e}")))?;
    let reference_path = dir.join("reference.png");
    fs::write(&reference_path, &bytes)
        .await
        .map_err(|e| ApiError::InternalError(format!("write reference: {e}")))?;

    let url = format!("{}/reference.png", avatar_public_base(id));
    AvatarProfile::set_reference_image(pool, id, &url)
        .await
        .map_err(ApiError::Database)?;

    let refreshed = AvatarProfile::find(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;
    Ok(Json(refreshed))
}

/// POST /api/video-gen/avatars/:id/generate-profile
///
/// Spawns the async pipeline. Returns immediately with `profile_status = pending`.
/// Clients poll the avatar GET endpoint to watch status flip to
/// `generating` → `ready` (or `failed`).
async fn generate_profile(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = deployment.db().pool.clone();

    let avatar = require_avatar_org_access(&ctx, &pool, id).await?;

    let reference_path = avatar_dir(id).join("reference.png");
    if !reference_path.exists() {
        return Err(ApiError::BadRequest(
            "avatar has no reference image — upload one first via /upload-reference".to_string(),
        ));
    }

    // Pre-debit VIBE estimate. On insufficient balance or per-user cap exceeded
    // this returns 402 / 403 before we spawn anything.
    let org_id = avatar.organization_id.ok_or_else(|| {
        ApiError::BadRequest("avatar has no organization_id; cannot bill for generation".into())
    })?;
    let estimated_vibe = services::services::avatar_pricing::profile_pipeline_vibe_cost();
    let charge_id = crate::helpers::billing::ensure_org_vibe_balance_and_debit(
        &pool,
        &org_id.to_string(),
        ctx.user_id.as_str(),
        estimated_vibe,
        Some(id),
        "avatar profile pipeline (16 shots + bible)",
    )
    .await?;

    // Atomic claim: only proceed if no other pipeline is in flight for this
    // avatar. Prevents two concurrent POSTs from each spawning a pipeline and
    // clobbering each other's shots on disk.
    let claimed = AvatarProfile::try_claim_profile_generation(&pool, id)
        .await
        .map_err(ApiError::Database)?;
    if !claimed {
        // Roll back the pre-debit since the pipeline isn't going to run.
        let _ = crate::helpers::billing::refund_org_vibe_charge(&pool, charge_id).await;
        return Err(ApiError::Conflict(
            "avatar profile generation already in progress".to_string(),
        ));
    }

    let cfg = avatar_engine::PipelineConfig {
        avatar_id: id,
        reference_path,
        shots_dir: avatar_dir(id).join("shots"),
        public_base_path: avatar_public_base(id),
        charge_id: Some(charge_id),
    };

    let pipeline_pool = pool.clone();
    tokio::spawn(async move {
        avatar_engine::run_pipeline(pipeline_pool, cfg).await;
    });

    let refreshed = AvatarProfile::find(&pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;
    Ok(Json(refreshed))
}

/// GET /api/video-gen/avatars/:id/reference.png — auth-required
///
/// Path keeps the historical `.png` suffix for backwards compatibility, but the
/// actual file may be PNG or JPEG (validated at upload). We sniff bytes to emit
/// the correct `Content-Type` header.
///
/// Access control: caller must be a member of the avatar's org (or admin).
async fn serve_reference(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    require_avatar_org_access(&ctx, pool, id).await?;
    let path = avatar_dir(id).join("reference.png");
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("reference for avatar {}", id)))?;
    let content_type = infer::get(&data)
        .map(|k| k.mime_type())
        .unwrap_or("image/png");
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .body(Body::from(data))
        .unwrap_or_else(|_| Response::new(Body::empty())))
}

// ---------------------------------------------------------------------------
// Full-body motion via fal.ai OmniHuman (image + audio → person speaking + moving)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct RenderMotionBody {
    script_text: String,
    #[serde(default)]
    source_slot: Option<String>,
}

#[derive(serde::Serialize)]
struct MotionClipResponse {
    motion_url: String,
    duration_seconds: Option<f64>,
    source_slot: String,
    elapsed_ms: u128,
}

/// POST /api/video-gen/avatars/:id/render-motion
///
/// Takes a short script + which identity-sheet slot to use as the source frame
/// (defaults to `full_body_front`). Pipeline:
///   1. ElevenLabs TTS → MP3 → WAV
///   2. base64 the source PNG + the WAV
///   3. POST both to fal.ai `fal-ai/bytedance/omnihuman` (sync endpoint)
///   4. Download the resulting MP4 → save under
///      `dev_assets/avatars/<id>/motion/<slug>-<unix>.mp4`
///   5. Return the public URL
///
/// Blocking call; OmniHuman generation typically takes 60-180s.
async fn render_motion(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
    Json(body): Json<RenderMotionBody>,
) -> Result<Json<MotionClipResponse>, ApiError> {
    let pool = &deployment.db().pool;
    let avatar = require_avatar_org_access(&ctx, pool, id).await?;

    if body.script_text.trim().is_empty() {
        return Err(ApiError::BadRequest("script_text is empty".to_string()));
    }

    let source_slot = body
        .source_slot
        .clone()
        .unwrap_or_else(|| "full_body_front".to_string());
    let source_path = avatar_dir(id)
        .join("shots")
        .join(format!("{source_slot}.png"));
    if !source_path.exists() {
        return Err(ApiError::BadRequest(format!(
            "no shot {source_slot} on disk — generate the profile first"
        )));
    }

    let fal_key = std::env::var("FAL_API_KEY")
        .map_err(|_| ApiError::InternalError("FAL_API_KEY not set".to_string()))?;
    let elevenlabs_key = std::env::var("ELEVENLABS_API_KEY")
        .map_err(|_| ApiError::InternalError("ELEVENLABS_API_KEY not set".to_string()))?;

    let started = std::time::Instant::now();

    // 1. ElevenLabs TTS
    let tts_bytes = elevenlabs_tts(
        &elevenlabs_key,
        &avatar.elevenlabs_voice_id,
        &body.script_text,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("TTS failed: {e:#}")))?;

    // Save MP3 to a temp file then convert to WAV (HeyGen/OmniHuman want PCM)
    let unix = chrono::Utc::now().timestamp();
    let motion_dir = avatar_dir(id).join("motion");
    fs::create_dir_all(&motion_dir)
        .await
        .map_err(|e| ApiError::InternalError(format!("mkdir motion: {e}")))?;
    let mp3_path = motion_dir.join(format!("{source_slot}-{unix}.mp3"));
    let wav_path = motion_dir.join(format!("{source_slot}-{unix}.wav"));
    fs::write(&mp3_path, &tts_bytes)
        .await
        .map_err(|e| ApiError::InternalError(format!("write mp3: {e}")))?;
    let ffmpeg = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            mp3_path.to_str().unwrap_or(""),
            "-ar",
            "44100",
            "-ac",
            "1",
            "-f",
            "wav",
            wav_path.to_str().unwrap_or(""),
        ])
        .output()
        .await
        .map_err(|e| ApiError::InternalError(format!("ffmpeg: {e}")))?;
    if !ffmpeg.status.success() {
        return Err(ApiError::InternalError(format!(
            "ffmpeg mp3→wav failed: {}",
            String::from_utf8_lossy(&ffmpeg.stderr)
        )));
    }

    // 2. Read both files, upload to fal.ai's storage so OmniHuman can fetch them
    let img_bytes = fs::read(&source_path)
        .await
        .map_err(|e| ApiError::InternalError(format!("read source: {e}")))?;
    let wav_bytes = fs::read(&wav_path)
        .await
        .map_err(|e| ApiError::InternalError(format!("read wav: {e}")))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| ApiError::InternalError(format!("http client: {e}")))?;

    let image_url = fal_storage_upload(
        &client,
        &fal_key,
        &format!("avatar-{id}-{source_slot}.png"),
        "image/png",
        img_bytes,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("fal upload image: {e:#}")))?;

    let audio_url = fal_storage_upload(
        &client,
        &fal_key,
        &format!("avatar-{id}-{unix}.wav"),
        "audio/wav",
        wav_bytes.clone(),
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("fal upload audio: {e:#}")))?;

    tracing::info!(
        "motion render avatar={} slot={} uploaded → OmniHuman ({} bytes audio, image={}, audio={})",
        id,
        source_slot,
        wav_bytes.len(),
        image_url,
        audio_url
    );

    // 3. Submit to fal.ai OmniHuman via queue API (more reliable than sync for long jobs)
    let queue_resp = client
        .post("https://queue.fal.run/fal-ai/bytedance/omnihuman")
        .header("Authorization", format!("Key {fal_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "image_url": image_url,
            "audio_url": audio_url,
        }))
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("OmniHuman submit: {e}")))?;

    let submit_status = queue_resp.status();
    let submit_text = queue_resp
        .text()
        .await
        .map_err(|e| ApiError::InternalError(format!("read submit: {e}")))?;
    if !submit_status.is_success() {
        return Err(ApiError::InternalError(format!(
            "OmniHuman submit {submit_status}: {}",
            &submit_text[..submit_text.len().min(500)]
        )));
    }

    #[derive(Deserialize)]
    struct QueueSubmit {
        request_id: String,
        status_url: String,
        response_url: String,
    }
    let submit: QueueSubmit = serde_json::from_str(&submit_text).map_err(|e| {
        ApiError::InternalError(format!("parse queue submit: {e}; body={submit_text}"))
    })?;
    tracing::info!(
        "OmniHuman queued: request_id={} status_url={}",
        submit.request_id,
        submit.status_url
    );
    let status_url = submit.status_url;
    let result_url = submit.response_url;

    // Poll status — total budget ~10 min
    let mut attempt = 0u32;
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        attempt += 1;
        let sresp = client
            .get(&status_url)
            .header("Authorization", format!("Key {fal_key}"))
            .send()
            .await
            .map_err(|e| ApiError::InternalError(format!("poll: {e}")))?;
        let stext = sresp
            .text()
            .await
            .map_err(|e| ApiError::InternalError(format!("poll body: {e}")))?;
        let sjson: serde_json::Value = match serde_json::from_str(&stext) {
            Ok(v) => v,
            Err(_) => {
                // Transient non-JSON response (e.g. CDN edge error) — wait + retry
                tracing::warn!(
                    "OmniHuman poll {attempt}: non-JSON body, retrying. body[..200]={}",
                    &stext[..stext.len().min(200)]
                );
                continue;
            }
        };
        match sjson["status"].as_str() {
            Some("COMPLETED") => {
                tracing::info!("OmniHuman queue COMPLETED after {attempt} polls");
                break;
            }
            Some("IN_QUEUE") | Some("IN_PROGRESS") => {
                if attempt >= 120 {
                    return Err(ApiError::InternalError(
                        "OmniHuman timed out after 10 min".to_string(),
                    ));
                }
                continue;
            }
            Some(other) => {
                return Err(ApiError::InternalError(format!(
                    "OmniHuman queue status: {other} (full: {sjson})"
                )));
            }
            None => {
                return Err(ApiError::InternalError(format!(
                    "OmniHuman queue no status (full: {sjson})"
                )));
            }
        }
    }

    // Fetch the completed result
    let resp = client
        .get(&result_url)
        .header("Authorization", format!("Key {fal_key}"))
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("result fetch: {e}")))?;
    let text = resp
        .text()
        .await
        .map_err(|e| ApiError::InternalError(format!("read resp: {e}")))?;

    #[derive(Deserialize)]
    struct OmniResp {
        video: OmniVideo,
    }
    #[derive(Deserialize)]
    struct OmniVideo {
        url: String,
        #[serde(default)]
        duration: Option<f64>,
    }
    let parsed: OmniResp = serde_json::from_str(&text).map_err(|e| {
        ApiError::InternalError(format!(
            "parse OmniHuman resp: {e}; body={}",
            &text[..text.len().min(500)]
        ))
    })?;

    // 4. Download the MP4
    let mp4_bytes = client
        .get(&parsed.video.url)
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("download mp4: {e}")))?
        .bytes()
        .await
        .map_err(|e| ApiError::InternalError(format!("read mp4: {e}")))?;

    let mp4_filename = format!("{source_slot}-{unix}.mp4");
    let mp4_path = motion_dir.join(&mp4_filename);
    fs::write(&mp4_path, &mp4_bytes)
        .await
        .map_err(|e| ApiError::InternalError(format!("write mp4: {e}")))?;

    let public_url = format!("{}/motion/{}", avatar_public_base(id), mp4_filename);
    let elapsed_ms = started.elapsed().as_millis();
    tracing::info!(
        "motion render avatar={} slot={} done in {}ms ({} KB)",
        id,
        source_slot,
        elapsed_ms,
        mp4_bytes.len() / 1024
    );

    Ok(Json(MotionClipResponse {
        motion_url: public_url,
        duration_seconds: parsed.video.duration,
        source_slot,
        elapsed_ms,
    }))
}

/// GET /api/video-gen/avatars/:id/motion/<clip> — public
async fn serve_motion_clip(
    State(deployment): State<DeploymentImpl>,
    Path((id, clip)): Path<(Uuid, String)>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    require_avatar_org_access(&ctx, pool, id).await?;

    // Strict whitelist: alnum + underscore/dash, then literal `.mp4`. Rejects
    // backslash, NUL, leading dots, anything with a slash or `..`, etc.
    static MOTION_CLIP_RE: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"^[A-Za-z0-9_\-]+\.mp4$").unwrap());
    if !MOTION_CLIP_RE.is_match(&clip) {
        return Err(ApiError::BadRequest("invalid clip name".to_string()));
    }
    let path = avatar_dir(id).join("motion").join(&clip);
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("motion {clip} for avatar {id}")))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .body(Body::from(data))
        .unwrap_or_else(|_| Response::new(Body::empty())))
}

/// Upload bytes to fal.ai's private storage. Returns a fal-hosted public URL
/// that can be passed to any fal model as `image_url` / `audio_url`.
///
/// Two-step flow per fal docs:
///   1. POST /storage/upload/initiate {file_name, content_type}
///      → {upload_url, file_url}
///   2. PUT bytes to upload_url
///   3. Return file_url
async fn fal_storage_upload(
    client: &reqwest::Client,
    fal_key: &str,
    file_name: &str,
    content_type: &str,
    bytes: Vec<u8>,
) -> anyhow::Result<String> {
    #[derive(Deserialize)]
    struct InitiateResp {
        upload_url: String,
        file_url: String,
    }

    let initiate: InitiateResp = client
        .post("https://rest.alpha.fal.ai/storage/upload/initiate")
        .header("Authorization", format!("Key {fal_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "file_name": file_name,
            "content_type": content_type,
        }))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let put = client
        .put(&initiate.upload_url)
        .header("Content-Type", content_type)
        .body(bytes)
        .send()
        .await?;
    if !put.status().is_success() {
        let status = put.status();
        let body = put.text().await.unwrap_or_default();
        anyhow::bail!("fal storage PUT {status}: {body}");
    }

    Ok(initiate.file_url)
}

/// Minimal ElevenLabs TTS — returns MP3 bytes. Distinct from the timestamped
/// flavor used by produce_job; OmniHuman doesn't need word alignment.
///
/// Tries `eleven_turbo_v2_5` first (best for instant voice clones), falls
/// back to `eleven_multilingual_v2` if the turbo model rejects the voice.
async fn elevenlabs_tts(api_key: &str, voice_id: &str, text: &str) -> anyhow::Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{voice_id}");

    for model_id in [
        "eleven_turbo_v2_5",
        "eleven_multilingual_v2",
        "eleven_flash_v2_5",
    ] {
        let resp = client
            .post(&url)
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "text": text,
                "model_id": model_id,
            }))
            .send()
            .await?;
        if resp.status().is_success() {
            return Ok(resp.bytes().await?.to_vec());
        }
        // Only retry on 400 voice_not_fine_tuned; bail on auth / quota / network.
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if status.as_u16() != 400 || !body.contains("voice_not_fine_tuned") {
            // Scrubbed: upstream body can echo headers / request IDs that aid
            // enumeration. Log full detail, surface only status to the caller.
            tracing::error!(target: "avatar_engine", %status, body = %body, "elevenlabs tts failed");
            anyhow::bail!("tts failed (status {status})");
        }
        tracing::warn!("voice {voice_id} rejected by {model_id}, trying next");
    }
    anyhow::bail!("tts failed: voice {voice_id} not supported by any model")
}

/// GET /api/video-gen/avatars/:id/shots/:slot.png — auth-required
async fn serve_shot(
    State(deployment): State<DeploymentImpl>,
    Path((id, slot)): Path<(Uuid, String)>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<Response, ApiError> {
    let pool = &deployment.db().pool;
    require_avatar_org_access(&ctx, pool, id).await?;

    // Slot must match the canonical taxonomy to avoid path traversal.
    let allowed = avatar_engine::SHOT_SLOTS.iter().any(|s| s.key == slot);
    if !allowed {
        return Err(ApiError::BadRequest(format!("unknown slot {slot}")));
    }
    let path = avatar_dir(id).join("shots").join(format!("{slot}.png"));
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("shot {slot} for avatar {id}")))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .body(Body::from(data))
        .unwrap_or_else(|_| Response::new(Body::empty())))
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
    Extension(ctx): Extension<AccessContext>,
    Query(params): Query<ListJobsParams>,
) -> Result<Json<Vec<VideoJob>>, ApiError> {
    let pool = &deployment.db().pool;

    // Scope by avatar (which scopes by org). Non-admins must specify an
    // avatar_id they have access to; admins may list across the system.
    match params.avatar_id {
        Some(avatar_id) => {
            require_avatar_org_access(&ctx, pool, avatar_id).await?;
        }
        None if !ctx.is_admin => {
            return Err(ApiError::BadRequest(
                "avatar_id query parameter required".to_string(),
            ));
        }
        None => {} // admin
    }

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

    // Access control: caller must have access to the avatar this job uses.
    // Returns the avatar so we can reuse it for voice/identity below.
    let avatar = require_avatar_org_access(&ctx, &pool, body.avatar_profile_id).await?;

    // Wire created_by from auth context so per-user billing attribution works.
    let created_by = Uuid::parse_str(ctx.user_id.as_str()).ok();

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
    let segments_json = job.segments_json.clone();

    tokio::spawn(async move {
        if let Err(e) = produce_job(
            &pool2,
            job_id,
            &avatar,
            &script,
            background_url.as_deref(),
            segments_json.as_deref(),
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
    Extension(ctx): Extension<AccessContext>,
) -> Result<Json<VideoJob>, ApiError> {
    let pool = &deployment.db().pool;
    let job = require_video_job_org_access(&ctx, pool, id).await?;
    Ok(Json(job))
}

async fn delete_job(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(ctx): Extension<AccessContext>,
) -> Result<StatusCode, ApiError> {
    let pool = &deployment.db().pool;
    require_video_job_org_access(&ctx, pool, id).await?;
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
        .unwrap_or_else(|_| Response::new(Body::empty())))
}

async fn serve_video(Path(job_id): Path<String>) -> Result<Response, ApiError> {
    let path = video_dir().join(format!("{}.mp4", job_id));
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("video {}", job_id)))?;

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "video/mp4")
        .body(Body::from(data))
        .unwrap_or_else(|_| Response::new(Body::empty())))
}

async fn serve_final_video(
    Path(job_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let path = video_dir().join(format!("{}_final.mp4", job_id));

    let meta = tokio::fs::metadata(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("final video {}", job_id)))?;
    let file_size = meta.len();

    let disposition = format!("inline; filename=\"{}_techbrief.mp4\"", &job_id[..8]);

    // Parse Range header for browser video seeking support
    if let Some(range_hdr) = headers.get(header::RANGE) {
        if let Ok(range_str) = range_hdr.to_str() {
            if let Some(range_val) = range_str.strip_prefix("bytes=") {
                let parts: Vec<&str> = range_val.splitn(2, '-').collect();
                if parts.len() == 2 {
                    let start: u64 = parts[0].parse().unwrap_or(0);
                    let end: u64 = parts[1].parse().unwrap_or(file_size - 1).min(file_size - 1);
                    if start <= end && start < file_size {
                        let length = end - start + 1;
                        use tokio::io::{AsyncReadExt, AsyncSeekExt};
                        let mut file = tokio::fs::File::open(&path)
                            .await
                            .map_err(|_| ApiError::NotFound(format!("final video {}", job_id)))?;
                        file.seek(std::io::SeekFrom::Start(start)).await.ok();
                        let mut buf = vec![0u8; length as usize];
                        let _ = file.read_exact(&mut buf).await;
                        return Ok(Response::builder()
                            .status(StatusCode::PARTIAL_CONTENT)
                            .header(header::CONTENT_TYPE, "video/mp4")
                            .header(header::CONTENT_DISPOSITION, disposition)
                            .header(header::ACCEPT_RANGES, "bytes")
                            .header(header::CONTENT_LENGTH, length)
                            .header(
                                header::CONTENT_RANGE,
                                format!("bytes {}-{}/{}", start, end, file_size),
                            )
                            .body(Body::from(buf))
                            .unwrap_or_else(|_| Response::new(Body::empty())));
                    }
                }
            }
        }
    }

    // Full file response
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("final video {}", job_id)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_LENGTH, file_size)
        .body(Body::from(data))
        .unwrap_or_else(|_| Response::new(Body::empty())))
}

// ---------------------------------------------------------------------------
// Async pipeline
// ---------------------------------------------------------------------------

pub(crate) async fn produce_job(
    pool: &sqlx::SqlitePool,
    job_id: Uuid,
    avatar: &AvatarProfile,
    script: &str,
    background_url: Option<&str>,
    segments_json: Option<&str>,
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    let el_key = std::env::var("ELEVENLABS_API_KEY")?;
    let hg_key = std::env::var("HEYGEN_API_KEY")?;

    // Step 1: TTS with word-level timestamps
    tracing::info!("video job {}: generating TTS with timestamps", job_id);
    VideoJob::update_status(pool, job_id, "tts_generating", None).await?;

    let tts = tts::generate_tts(&el_key, &avatar.elevenlabs_voice_id, script).await?;

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
    let audio_path_str = audio_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("audio path is not valid utf-8"))?;
    let wav_path_str = wav_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("wav path is not valid utf-8"))?;
    let ffmpeg_out = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            audio_path_str,
            audio_path.to_str().unwrap(),
            "-ar",
            "16000", // 16 kHz — optimal for speech recognition
            "-ac",
            "1", // mono
            "-f",
            "wav",
            wav_path_str,
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
    let audio_url = heygen::upload_audio_wav(&hg_key, wav_bytes).await?;

    // Compute smart cut points from word alignment + segment metadata
    let cut_points = find_cut_points(&tts.word_alignment, segments_json);
    let cut_points_json = serde_json::to_string(&cut_points).unwrap_or_else(|_| "[]".to_string());
    let word_timestamps_json = serde_json::to_string(
        &tts.word_alignment
            .iter()
            .map(|w| serde_json::json!({"word": w.word, "start": w.start_time, "end": w.end_time}))
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".to_string());
    let _ =
        VideoJob::update_tts_metadata(pool, job_id, &word_timestamps_json, &cut_points_json).await;
    tracing::info!("video job {}: cut points = {:?}", job_id, cut_points);

    VideoJob::update_tts_done(pool, job_id, &audio_url).await?;
    tracing::info!("video job {}: TTS done, audio at {}", job_id, audio_url);

    // Step 2: HeyGen generate
    let heygen_avatar_id = avatar
        .heygen_avatar_id
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("avatar has no heygen_avatar_id set"))?;

    tracing::info!("video job {}: submitting to HeyGen", job_id);
    let avatar_type = avatar.heygen_avatar_type.as_str();
    let heygen_video_id = heygen::generate_with_audio(
        &hg_key,
        heygen_avatar_id,
        avatar_type,
        &audio_url,
        background_url,
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

        let status = heygen::poll_status(&hg_key, &heygen_video_id).await?;
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

                // Kick off post-production pipeline (non-blocking)
                let pool_pp = pool.clone();
                let vid_path_pp = vid_path.clone();
                let cuts_pp = cut_points_json.clone();
                let segs_pp = segments_json.map(|s| s.to_string());
                tokio::spawn(async move {
                    run_post_production(
                        &pool_pp,
                        job_id,
                        &vid_path_pp,
                        &cuts_pp,
                        segs_pp.as_deref(),
                    )
                    .await;
                });

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

// ---------------------------------------------------------------------------
// Post-production pipeline
// ---------------------------------------------------------------------------

/// Runs the generic Tech Briefing post-production pipeline on a completed
/// HeyGen raw video. Adds B-roll placeholder backgrounds, overlay graphics,
/// PiP circles, thumbnail, logo, and outro via FFmpeg.
///
/// Non-blocking — spawned as a tokio task after `produce_job` completes.
async fn run_post_production(
    pool: &sqlx::SqlitePool,
    job_id: Uuid,
    raw_path: &std::path::Path,
    cut_points_json: &str,
    segments_json: Option<&str>,
) {
    tracing::info!("video job {}: starting post-production", job_id);

    if let Err(e) = VideoJob::update_postprod_started(pool, job_id).await {
        tracing::error!(
            "video job {}: failed to mark postprod started: {}",
            job_id,
            e
        );
        return;
    }

    // Locate pipeline script relative to the working directory
    let script = std::path::Path::new("dev_assets/video_gen/build_techbrief_generic.py");
    if !script.exists() {
        tracing::error!(
            "video job {}: post-production script not found at {:?}",
            job_id,
            script
        );
        let _ = VideoJob::update_postprod_failed(pool, job_id, "pipeline script not found").await;
        return;
    }

    let mut cmd = tokio::process::Command::new("python3");
    cmd.arg(script)
        .arg("--raw")
        .arg(raw_path)
        .arg("--job-id")
        .arg(job_id.to_string())
        .arg("--cuts")
        .arg(cut_points_json);

    if let Some(segs) = segments_json {
        cmd.arg("--segments").arg(segs);
    }

    let output = cmd.output().await;

    match output {
        Ok(out) if out.status.success() => {
            let final_path = video_dir()
                .join(format!("{}_final.mp4", job_id))
                .to_string_lossy()
                .to_string();
            let public_url = format!("{}/api/video-gen/final/{}", app_base_url(), job_id);
            tracing::info!(
                "video job {}: post-production done → {}",
                job_id,
                final_path
            );
            let _ = VideoJob::update_postprod_done(pool, job_id, &final_path, &public_url).await;
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let err = format!(
                "post-prod failed (exit {}): {}",
                out.status,
                &stderr[stderr.len().saturating_sub(500)..]
            );
            tracing::error!("video job {}: {}", job_id, err);
            let _ = VideoJob::update_postprod_failed(pool, job_id, &err).await;
        }
        Err(e) => {
            let err = format!("failed to spawn post-prod: {}", e);
            tracing::error!("video job {}: {}", job_id, err);
            let _ = VideoJob::update_postprod_failed(pool, job_id, &err).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Smart cut detection
// ---------------------------------------------------------------------------

/// Given word-level TTS timestamps and optional segments JSON, returns a list
/// of cut timecodes (seconds) corresponding to natural speech pauses near each
/// segment boundary.
///
/// If no segments metadata is provided, falls back to proportional 40/40/20 split.
fn find_cut_points(word_alignment: &[tts::WordAlignment], segments_json: Option<&str>) -> Vec<f64> {
    if word_alignment.is_empty() {
        return vec![];
    }

    let total_dur = word_alignment.last().map(|w| w.end_time).unwrap_or(0.0);

    if total_dur < 1.0 {
        return vec![];
    }

    // Determine target cut times from segments metadata or fall back to proportional split
    let target_times: Vec<f64> = if let Some(json) = segments_json {
        if let Ok(segs) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
            // Accumulate word counts per segment to estimate boundary timestamps
            let total_words = word_alignment.len() as f64;
            let mut word_cursor = 0usize;
            let mut times = Vec::new();

            for (i, seg) in segs.iter().enumerate() {
                if i == segs.len() - 1 {
                    break; // no cut after last segment
                }
                let approx_words = seg
                    .get("approx_words")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(total_words / segs.len() as f64);
                word_cursor += approx_words as usize;
                let idx = word_cursor.min(word_alignment.len() - 1);
                times.push(word_alignment[idx].start_time);
            }
            times
        } else {
            // Fallback: 3-cut proportional
            vec![total_dur * 0.40, total_dur * 0.70]
        }
    } else {
        // No segments: 2-cut 40/80 split
        vec![total_dur * 0.40, total_dur * 0.80]
    };

    // Snap each target time to the nearest natural pause (largest inter-word gap within ±3s)
    target_times
        .into_iter()
        .map(|target| snap_to_pause(word_alignment, target, 3.0))
        .collect()
}

/// Find the midpoint of the largest inter-word gap within `window_secs` of `target`.
fn snap_to_pause(words: &[tts::WordAlignment], target: f64, window_secs: f64) -> f64 {
    let best = words
        .windows(2)
        .filter_map(|pair| {
            let gap = pair[1].start_time - pair[0].end_time;
            let mid = (pair[0].end_time + pair[1].start_time) / 2.0;
            if (mid - target).abs() <= window_secs && gap > 0.0 {
                Some((gap, mid))
            } else {
                None
            }
        })
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    best.map(|(_, mid)| (mid * 1000.0).round() / 1000.0)
        .unwrap_or((target * 1000.0).round() / 1000.0)
}

// ---------------------------------------------------------------------------
// TTS integration (ElevenLabs)
// ---------------------------------------------------------------------------

mod tts {
    use serde::Deserialize;

    pub struct TtsResult {
        pub audio_bytes: Vec<u8>,
        pub word_alignment: Vec<WordAlignment>,
    }

    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    pub struct WordAlignment {
        pub word: String,
        pub start_time: f64,
        pub end_time: f64,
    }

    /// Generate TTS audio with word-level timestamps from ElevenLabs.
    pub async fn generate_tts(
        api_key: &str,
        voice_id: &str,
        text: &str,
    ) -> anyhow::Result<TtsResult> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.elevenlabs.io/v1/text-to-speech/{}/with-timestamps",
            voice_id
        );

        #[derive(serde::Serialize)]
        struct Req<'a> {
            text: &'a str,
            model_id: &'a str,
        }

        let resp = client
            .post(&url)
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&Req {
                text,
                model_id: "eleven_multilingual_v2",
            })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs TTS error {}: {}", status, body);
        }

        #[derive(Deserialize)]
        struct ElevenLabsResp {
            audio_base64: String,
            alignment: Option<AlignmentData>,
        }

        #[derive(Deserialize)]
        struct AlignmentData {
            characters: Vec<String>,
            character_start_times_seconds: Vec<f64>,
            character_end_times_seconds: Vec<f64>,
        }

        let data: ElevenLabsResp = resp.json().await?;
        let audio_bytes = base64_decode(&data.audio_base64)?;

        // Convert character-level alignment to approximate word-level
        let word_alignment = if let Some(align) = data.alignment {
            build_word_alignment(
                &align.characters,
                &align.character_start_times_seconds,
                &align.character_end_times_seconds,
            )
        } else {
            vec![]
        };

        Ok(TtsResult {
            audio_bytes,
            word_alignment,
        })
    }

    fn base64_decode(s: &str) -> anyhow::Result<Vec<u8>> {
        use std::io::Read;
        let mut decoder = base64::read::DecoderReader::new(
            s.as_bytes(),
            &base64::engine::general_purpose::STANDARD,
        );
        let mut buf = Vec::new();
        decoder.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn build_word_alignment(chars: &[String], starts: &[f64], ends: &[f64]) -> Vec<WordAlignment> {
        let mut words = Vec::new();
        let mut current_word = String::new();
        let mut word_start = 0.0f64;
        let mut last_end = 0.0f64;

        for ((ch, &start), &end) in chars.iter().zip(starts.iter()).zip(ends.iter()) {
            if ch == " " || ch == "\n" {
                if !current_word.is_empty() {
                    words.push(WordAlignment {
                        word: current_word.clone(),
                        start_time: word_start,
                        end_time: last_end,
                    });
                    current_word.clear();
                }
            } else {
                if current_word.is_empty() {
                    word_start = start;
                }
                current_word.push_str(ch);
                last_end = end;
            }
        }
        if !current_word.is_empty() {
            words.push(WordAlignment {
                word: current_word,
                start_time: word_start,
                end_time: last_end,
            });
        }
        words
    }
}

// ---------------------------------------------------------------------------
// HeyGen integration
// ---------------------------------------------------------------------------

mod heygen {
    use serde::{Deserialize, Serialize};

    pub struct HeyGenStatus {
        pub status: String,
        pub video_url: Option<String>,
        pub thumbnail_url: Option<String>,
        pub duration: Option<f64>,
        pub error: Option<String>,
    }

    /// Upload a WAV file to HeyGen's asset CDN and return the public URL.
    /// HeyGen's /v1/asset endpoint expects a raw binary body with Content-Type header
    /// (not multipart form-data).
    pub async fn upload_audio_wav(api_key: &str, wav_bytes: Vec<u8>) -> anyhow::Result<String> {
        let client = reqwest::Client::new();

        let resp = client
            .post("https://upload.heygen.com/v1/asset")
            .header("X-Api-Key", api_key)
            .header("Content-Type", "audio/x-wav")
            .body(wav_bytes)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HeyGen upload error {}: {}", status, body);
        }

        #[derive(Deserialize)]
        struct UploadResp {
            data: UploadData,
        }
        #[derive(Deserialize)]
        struct UploadData {
            url: String,
        }

        let data: UploadResp = resp.json().await?;
        Ok(data.data.url)
    }

    /// Submit a video generation job to HeyGen and return the video_id.
    /// `avatar_type` is either "avatar" or "talking_photo" (default: "avatar").
    pub async fn generate_with_audio(
        api_key: &str,
        avatar_id: &str,
        avatar_type: &str,
        audio_url: &str,
        background_url: Option<&str>,
    ) -> anyhow::Result<String> {
        let client = reqwest::Client::new();

        // Build character config dynamically: "avatar" uses avatar_id, "talking_photo" uses talking_photo_id
        let character = if avatar_type == "talking_photo" {
            serde_json::json!({
                "type": "talking_photo",
                "talking_photo_id": avatar_id
            })
        } else {
            serde_json::json!({
                "type": "avatar",
                "avatar_id": avatar_id
            })
        };

        let voice = serde_json::json!({
            "type": "audio",
            "audio_url": audio_url
        });

        let mut video_input = serde_json::json!({
            "character": character,
            "voice": voice
        });

        if let Some(url) = background_url {
            video_input["background"] = serde_json::json!({
                "type": "image",
                "url": url
            });
        }

        let body = serde_json::json!({
            "video_inputs": [video_input],
            "dimension": {"width": 720, "height": 1280}
        });

        let resp = client
            .post("https://api.heygen.com/v2/video/generate")
            .header("X-Api-Key", api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HeyGen generate error {}: {}", status, body);
        }

        #[derive(Deserialize)]
        struct GenerateResp {
            data: GenerateData,
        }
        #[derive(Deserialize)]
        struct GenerateData {
            video_id: String,
        }

        let data: GenerateResp = resp.json().await?;
        Ok(data.data.video_id)
    }

    /// Poll HeyGen for video status.
    pub async fn poll_status(api_key: &str, video_id: &str) -> anyhow::Result<HeyGenStatus> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!(
                "https://api.heygen.com/v1/video_status.get?video_id={}",
                video_id
            ))
            .header("X-Api-Key", api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HeyGen poll error {}: {}", status, body);
        }

        #[derive(Deserialize)]
        struct PollResp {
            data: PollData,
        }
        #[derive(Deserialize)]
        struct PollData {
            status: String,
            video_url: Option<String>,
            thumbnail_url: Option<String>,
            duration: Option<f64>,
            error: Option<String>,
        }

        let data: PollResp = resp.json().await?;
        Ok(HeyGenStatus {
            status: data.data.status,
            video_url: data.data.video_url,
            thumbnail_url: data.data.thumbnail_url,
            duration: data.data.duration,
            error: data.data.error,
        })
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Video Studio inventory: /video-gen/files and /video-gen/overlays
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct VideoFile {
    filename: String,
    size_bytes: u64,
    stream_url: String,
}

#[derive(Debug, Serialize)]
struct OverlayFile {
    filename: String,
    size_bytes: u64,
    download_url: String,
}

#[derive(Debug, Serialize)]
struct OverlayEpisode {
    episode: String,
    files: Vec<OverlayFile>,
}

/// GET /video-gen/files — lists final-cut MP4s under dev_assets/video_gen/video/
async fn list_video_files() -> Result<Json<Vec<VideoFile>>, ApiError> {
    let dir = video_dir();
    let mut out = Vec::new();
    let mut read_dir = match fs::read_dir(&dir).await {
        Ok(d) => d,
        Err(_) => return Ok(Json(out)),
    };
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        let is_video = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e.to_lowercase().as_str(), "mp4" | "mov" | "webm"))
            .unwrap_or(false);
        if !is_video {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let size_bytes = entry.metadata().await.map(|m| m.len()).unwrap_or(0);
        let job_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let stream_url = format!("/api/video-gen/video/{}", job_stem);
        out.push(VideoFile {
            filename,
            size_bytes,
            stream_url,
        });
    }
    out.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(out))
}

/// GET /video-gen/overlays — lists overlay PNG episodes under pipeline/video/overlays/
async fn list_overlay_episodes() -> Result<Json<Vec<OverlayEpisode>>, ApiError> {
    let root = PathBuf::from("pipeline/video/overlays");
    let mut out = Vec::new();
    let mut read_dir = match fs::read_dir(&root).await {
        Ok(d) => d,
        Err(_) => return Ok(Json(out)),
    };
    while let Ok(Some(ep_entry)) = read_dir.next_entry().await {
        let ep_path = ep_entry.path();
        if !ep_path.is_dir() {
            continue;
        }
        let episode = ep_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        if episode.is_empty() {
            continue;
        }
        let mut files = Vec::new();
        let mut ep_read = match fs::read_dir(&ep_path).await {
            Ok(d) => d,
            Err(_) => continue,
        };
        while let Ok(Some(f_entry)) = ep_read.next_entry().await {
            let f_path = f_entry.path();
            let is_png = f_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("png"))
                .unwrap_or(false);
            if !is_png {
                continue;
            }
            let filename = f_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let size_bytes = f_entry.metadata().await.map(|m| m.len()).unwrap_or(0);
            let download_url = format!("/api/video-gen/overlays/{}/{}", episode, filename);
            files.push(OverlayFile {
                filename,
                size_bytes,
                download_url,
            });
        }
        files.sort_by(|a, b| a.filename.cmp(&b.filename));
        if !files.is_empty() {
            out.push(OverlayEpisode { episode, files });
        }
    }
    out.sort_by(|a, b| a.episode.cmp(&b.episode));
    Ok(Json(out))
}

#[cfg(test)]
mod tests {
    use super::{find_cut_points, snap_to_pause, tts::WordAlignment};

    fn w(word: &str, start: f64, end: f64) -> WordAlignment {
        WordAlignment {
            word: word.to_string(),
            start_time: start,
            end_time: end,
        }
    }

    // ---- snap_to_pause tests ------------------------------------------------

    #[test]
    fn test_snap_to_pause_finds_largest_gap() {
        // Gap1: world(2.0) - hello(1.0) = 1.0s gap, mid = 1.5
        // Gap2: test(3.05) - world(3.0) = 0.05s gap, mid = 3.025
        let words = vec![
            w("hello", 0.0, 1.0),
            w("world", 2.0, 3.0),
            w("test", 3.05, 4.0),
        ];

        // target=1.5, window=3.0 — both gaps are within 3.0s of 1.5
        // largest gap (1.0s) is at mid=1.5, should be chosen
        let result = snap_to_pause(&words, 1.5, 3.0);
        assert!(
            (result - 1.5).abs() < 0.001,
            "expected ~1.5, got {}",
            result
        );
    }

    #[test]
    fn test_snap_to_pause_falls_back_to_target() {
        // All words run together with tiny gaps nowhere near the target
        let words = vec![w("a", 0.0, 0.1), w("b", 0.11, 0.2), w("c", 0.21, 0.3)];

        // target = 50.0 — no words within ±3.0s of 50.0
        let result = snap_to_pause(&words, 50.0, 3.0);
        assert!(
            (result - 50.0).abs() < 0.001,
            "expected fallback to ~50.0, got {}",
            result
        );
    }

    #[test]
    fn test_snap_to_pause_outside_window() {
        // Gap1: mid between hello(1.0) and world(2.0) = 1.5, 3.5s from target=5.0
        // Gap2: mid between world(3.0) and end(100.0) = 51.5, far from target=5.0
        // window=3.0 → neither gap is within 3.0s of 5.0, should fall back to target
        let words = vec![
            w("hello", 0.0, 1.0),
            w("world", 2.0, 3.0),
            w("end", 100.0, 101.0),
        ];

        // target=5.0, window=3.0
        // gap mid=1.5 is 3.5 away (outside window)
        // gap mid=51.5 is 46.5 away (outside window)
        let result = snap_to_pause(&words, 5.0, 3.0);
        assert!(
            (result - 5.0).abs() < 0.001,
            "expected fallback to ~5.0, got {}",
            result
        );
    }

    // ---- find_cut_points tests ----------------------------------------------

    #[test]
    fn test_find_cut_points_empty() {
        let cuts = find_cut_points(&[], None);
        assert!(cuts.is_empty(), "empty word_alignment should yield no cuts");
    }

    #[test]
    fn test_find_cut_points_no_segments() {
        // 10 words evenly spaced 0–10s (each word 0.0-0.8s, gap 0.2s)
        let words: Vec<WordAlignment> = (0..10)
            .map(|i| {
                let start = i as f64;
                w(&format!("word{}", i), start, start + 0.8)
            })
            .collect();

        let cuts = find_cut_points(&words, None);

        // No segments → 2-cut 40/80 split; total_dur ≈ 9.8
        assert_eq!(cuts.len(), 2, "expected 2 cut points");

        let total_dur = 9.8f64;
        // Each cut should be close to 40% and 80% of total_dur (±3s window)
        assert!(
            cuts[0] > 0.0 && cuts[0] < total_dur,
            "first cut should be within audio range"
        );
        assert!(cuts[1] > cuts[0], "second cut should be after first");
    }

    #[test]
    fn test_find_cut_points_with_segments() {
        // 20 words: Intro(5) + AI(10) + Closing(5) — 1.0s per word
        let words: Vec<WordAlignment> = (0..20)
            .map(|i| {
                let start = i as f64;
                w(&format!("word{}", i), start, start + 0.9)
            })
            .collect();

        let segments_json = r#"[
            {"label":"Intro","approx_words":5,"broll_category":"pcg"},
            {"label":"AI","approx_words":10,"broll_category":"ai"},
            {"label":"Closing","approx_words":5,"broll_category":"pcg"}
        ]"#;

        let cuts = find_cut_points(&words, Some(segments_json));

        // 3 segments → 2 cut points (N-1)
        assert_eq!(cuts.len(), 2, "expected 2 cut points for 3 segments");

        // First cut should be after ~5 words (around t=5.0)
        // Second cut should be after ~15 words (around t=15.0)
        assert!(cuts[0] > 0.0, "first cut should be positive");
        assert!(cuts[1] > cuts[0], "second cut should be after first");
        assert!(cuts[1] < 20.0, "second cut should be within audio");
    }

    #[test]
    fn test_find_cut_points_with_segments_short_audio() {
        // total_dur < 1.0 should return empty vec
        let words = vec![w("hi", 0.0, 0.5)];
        let segments_json = r#"[{"label":"A","approx_words":1,"broll_category":"pcg"}]"#;
        let cuts = find_cut_points(&words, Some(segments_json));
        assert!(
            cuts.is_empty(),
            "short audio should yield no cuts, got {:?}",
            cuts
        );
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
