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
use db::models::{
    avatar_profile::{AvatarProfile, CreateAvatarProfile, UpdateAvatarProfile},
    video_job::{CreateVideoJob, VideoJob},
};
use deployment::Deployment;
use serde::Deserialize;
use tokio::fs;
use uuid::Uuid;

use crate::{error::ApiError, middleware::AccessContext, routes::avatar_engine, DeploymentImpl};

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
        .route(
            "/video-gen/avatars/{id}/init-talking-head",
            post(init_talking_head),
        )
        .route("/video-gen/avatars/{id}/render-motion", post(render_motion))
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
        .route("/video-gen/final/{job_id}", get(serve_final_video))
        // Avatar engine asset serving (HeyGen / Hedra need fetchable URLs)
        .route(
            "/video-gen/avatars/{id}/reference.png",
            get(serve_reference),
        )
        .route("/video-gen/avatars/{id}/shots/{slot}", get(serve_shot))
        .route(
            "/video-gen/avatars/{id}/motion/{clip}",
            get(serve_motion_clip),
        )
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
    Extension(_ctx): Extension<AccessContext>,
    mut multipart: Multipart,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = &deployment.db().pool;

    // Confirm the avatar exists up-front.
    let _ = AvatarProfile::find(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;

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
    Extension(_ctx): Extension<AccessContext>,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = deployment.db().pool.clone();

    AvatarProfile::find(&pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;

    let reference_path = avatar_dir(id).join("reference.png");
    if !reference_path.exists() {
        return Err(ApiError::BadRequest(
            "avatar has no reference image — upload one first via /upload-reference".to_string(),
        ));
    }

    AvatarProfile::update_profile_status(&pool, id, "pending", None)
        .await
        .map_err(ApiError::Database)?;

    let cfg = avatar_engine::PipelineConfig {
        avatar_id: id,
        reference_path,
        shots_dir: avatar_dir(id).join("shots"),
        public_base_path: avatar_public_base(id),
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

/// POST /api/video-gen/avatars/:id/init-talking-head
///
/// Uploads the avatar's `front` shot (or `reference` if no profile yet) to
/// HeyGen's talking-photo endpoint and stamps the returned `talking_photo_id`
/// onto the avatar so subsequent `/video-gen/jobs` calls can render videos.
async fn init_talking_head(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Extension(_ctx): Extension<AccessContext>,
) -> Result<Json<AvatarProfile>, ApiError> {
    let pool = &deployment.db().pool;

    let _ = AvatarProfile::find(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;

    let front = avatar_dir(id).join("shots").join("front.png");
    let fallback = avatar_dir(id).join("reference.png");
    let source = if front.exists() {
        front
    } else if fallback.exists() {
        fallback
    } else {
        return Err(ApiError::BadRequest(
            "avatar has no front shot or reference image — upload + generate profile first"
                .to_string(),
        ));
    };

    let bytes = fs::read(&source)
        .await
        .map_err(|e| ApiError::InternalError(format!("read source image: {e}")))?;

    let api_key = std::env::var("HEYGEN_API_KEY")
        .map_err(|_| ApiError::InternalError("HEYGEN_API_KEY not set".to_string()))?;

    let talking_photo_id = heygen_talking_photo::upload(&api_key, bytes)
        .await
        .map_err(|e| ApiError::InternalError(format!("HeyGen upload failed: {e:#}")))?;

    let updated = AvatarProfile::update(
        pool,
        id,
        db::models::avatar_profile::UpdateAvatarProfile {
            name: None,
            slug: None,
            identity_doc: None,
            style_notes: None,
            heygen_avatar_id: Some(talking_photo_id),
            heygen_avatar_type: Some("talking_photo".to_string()),
            elevenlabs_voice_id: None,
            reference_image_url: None,
            thumbnail_url: None,
            default_background_url: None,
            status: None,
            bible_json: None,
        },
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;

    Ok(Json(updated))
}

mod heygen_talking_photo {
    use serde::Deserialize;

    /// POST raw image bytes to HeyGen's talking-photo upload endpoint.
    /// Returns the `talking_photo_id` that can be used in `/v2/video/generate`
    /// as `character.talking_photo_id`.
    pub async fn upload(api_key: &str, bytes: Vec<u8>) -> anyhow::Result<String> {
        let client = reqwest::Client::new();
        let resp = client
            .post("https://upload.heygen.com/v1/talking_photo")
            .header("X-Api-Key", api_key)
            .header("Content-Type", "image/png")
            .body(bytes)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HeyGen talking_photo upload {status}: {body}");
        }

        #[derive(Deserialize)]
        struct UploadResp {
            data: UploadData,
        }
        #[derive(Deserialize)]
        struct UploadData {
            talking_photo_id: String,
        }
        let data: UploadResp = resp.json().await?;
        Ok(data.data.talking_photo_id)
    }
}

/// GET /api/video-gen/avatars/:id/reference.png — public
async fn serve_reference(Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    let path = avatar_dir(id).join("reference.png");
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("reference for avatar {}", id)))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
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
    Extension(_ctx): Extension<AccessContext>,
    Json(body): Json<RenderMotionBody>,
) -> Result<Json<MotionClipResponse>, ApiError> {
    let pool = &deployment.db().pool;
    let avatar = AvatarProfile::find(pool, id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound(format!("avatar {}", id)))?;

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
async fn serve_motion_clip(Path((id, clip)): Path<(Uuid, String)>) -> Result<Response, ApiError> {
    // Defense in depth: only allow .mp4 files, no traversal
    if !clip.ends_with(".mp4") || clip.contains('/') || clip.contains("..") {
        return Err(ApiError::BadRequest("bad clip name".to_string()));
    }
    let path = avatar_dir(id).join("motion").join(&clip);
    let data = fs::read(&path)
        .await
        .map_err(|_| ApiError::NotFound(format!("motion {clip} for avatar {id}")))?;
    Ok(Response::builder()
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "public, max-age=86400")
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
            anyhow::bail!("ElevenLabs TTS {status}: {body}");
        }
        tracing::warn!("voice {voice_id} rejected by {model_id}, trying next");
    }
    anyhow::bail!("ElevenLabs TTS: voice {voice_id} accepted by no model")
}

/// GET /api/video-gen/avatars/:id/shots/:slot.png — public
async fn serve_shot(Path((id, slot)): Path<(Uuid, String)>) -> Result<Response, ApiError> {
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
        .header(header::CACHE_CONTROL, "public, max-age=86400")
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
    let segments_json = job.segments_json.clone();

    tokio::spawn(async move {
        if let Err(e) = produce_job(
            &pool2,
            job_id,
            &avatar,
            &script,
            background_url.as_deref(),
            segments_json.as_deref(),
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
                            .unwrap());
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
        .unwrap())
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
