use std::{collections::HashMap, path::PathBuf};

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    routing::get,
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
        // Video Studio inventory
        .route("/video-gen/files", get(list_video_files))
        .route("/video-gen/overlays", get(list_overlay_episodes))
        .with_state(deployment.clone())
}

/// Public routes — no auth (audio/video file serving so HeyGen can fetch)
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

    #[allow(dead_code)]
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
    }
}
