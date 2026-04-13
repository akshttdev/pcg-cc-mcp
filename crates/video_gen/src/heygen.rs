use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Upload raw audio bytes to HeyGen's asset CDN.
/// Returns the public CDN URL for use in video generation.
pub async fn upload_audio(api_key: &str, audio_bytes: Vec<u8>) -> Result<String> {
    upload_audio_with_type(api_key, audio_bytes, "audio/mpeg").await
}

/// Upload WAV audio bytes to HeyGen's asset CDN.
/// WAV (16kHz mono PCM) gives HeyGen's speech recognition the best chance
/// of extracting word-level timing for expressive avatar lip sync.
pub async fn upload_audio_wav(api_key: &str, wav_bytes: Vec<u8>) -> Result<String> {
    upload_audio_with_type(api_key, wav_bytes, "audio/x-wav").await
}

async fn upload_audio_with_type(
    api_key: &str,
    audio_bytes: Vec<u8>,
    content_type: &str,
) -> Result<String> {
    let client = reqwest::Client::new();

    let resp = client
        .post("https://upload.heygen.com/v1/asset")
        .header("X-Api-Key", api_key)
        .header("Content-Type", content_type)
        .body(audio_bytes)
        .send()
        .await
        .context("HeyGen asset upload request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("HeyGen asset upload error {}: {}", status, text);
    }

    #[derive(Deserialize)]
    struct UploadResp {
        data: UploadData,
    }
    #[derive(Deserialize)]
    struct UploadData {
        url: String,
    }

    let parsed: UploadResp = resp
        .json()
        .await
        .context("parsing HeyGen upload response")?;
    Ok(parsed.data.url)
}

/// Generate a HeyGen talking-head video using a pre-rendered audio URL.
///
/// `avatar_id` — HeyGen avatar ID (stock or instant).
/// `audio_url`  — publicly accessible URL to the MP3 audio file.
///
/// Returns the HeyGen `video_id` for polling.
pub async fn generate_with_audio(
    api_key: &str,
    avatar_id: &str,
    audio_url: &str,
    background_url: Option<&str>,
) -> Result<String> {
    let client = reqwest::Client::new();

    // Build background section
    let background = match background_url {
        Some(url) => serde_json::json!({
            "type": "image",
            "url": url
        }),
        None => serde_json::json!({
            "type": "color",
            "value": "#000000"
        }),
    };

    let body = serde_json::json!({
        "video_inputs": [
            {
                "character": {
                    "type": "avatar",
                    "avatar_id": avatar_id,
                    "avatar_style": "normal"
                },
                "voice": {
                    "type": "audio",
                    "audio_url": audio_url
                },
                "background": background
            }
        ],
        "dimension": {
            "width": 1280,
            "height": 720
        }
    });

    let resp = client
        .post("https://api.heygen.com/v2/video/generate")
        .header("X-Api-Key", api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("HeyGen generate request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("HeyGen generate error {}: {}", status, text);
    }

    #[derive(Deserialize)]
    struct GenerateResp {
        data: GenerateData,
    }
    #[derive(Deserialize)]
    struct GenerateData {
        video_id: String,
    }

    let parsed: GenerateResp = resp
        .json()
        .await
        .context("parsing HeyGen generate response")?;
    Ok(parsed.data.video_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStatus {
    pub status: String, // "pending" | "processing" | "completed" | "failed"
    pub video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration: Option<f64>, // seconds
    pub error: Option<String>,
}

/// Poll the status of a HeyGen video by `video_id`.
pub async fn poll_status(api_key: &str, video_id: &str) -> Result<VideoStatus> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.heygen.com/v1/video_status.get?video_id={}",
        video_id
    );

    let resp = client
        .get(&url)
        .header("X-Api-Key", api_key)
        .send()
        .await
        .context("HeyGen status request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("HeyGen status error {}: {}", status, text);
    }

    #[derive(Deserialize)]
    struct StatusResp {
        data: StatusData,
    }
    #[derive(Deserialize)]
    struct StatusData {
        status: String,
        video_url: Option<String>,
        thumbnail_url: Option<String>,
        duration: Option<f64>,
        error: Option<String>,
    }

    let parsed: StatusResp = resp
        .json()
        .await
        .context("parsing HeyGen status response")?;
    Ok(VideoStatus {
        status: parsed.data.status,
        video_url: parsed.data.video_url,
        thumbnail_url: parsed.data.thumbnail_url,
        duration: parsed.data.duration,
        error: parsed.data.error,
    })
}
