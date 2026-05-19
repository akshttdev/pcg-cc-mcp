use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const HEYGEN_UPLOAD_URL: &str = "https://upload.heygen.com/v1/asset";
const HEYGEN_GENERATE_URL: &str = "https://api.heygen.com/v2/video/generate";
const HEYGEN_STATUS_URL: &str = "https://api.heygen.com/v1/video_status.get";

/// HeyGen API client with connection reuse.
pub struct HeyGenClient {
    client: Client,
    api_key: String,
}

impl HeyGenClient {
    /// Create a new HeyGen client with the given API key.
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            api_key: api_key.into(),
        })
    }

    /// Upload raw MP3 audio bytes to HeyGen's asset CDN.
    /// Returns the public CDN URL for use in video generation.
    pub async fn upload_audio(&self, audio_bytes: Vec<u8>) -> Result<String> {
        self.upload_audio_with_type(audio_bytes, "audio/mpeg").await
    }

    /// Upload WAV audio bytes to HeyGen's asset CDN.
    /// WAV (16kHz mono PCM) gives HeyGen's speech recognition the best chance
    /// of extracting word-level timing for expressive avatar lip sync.
    pub async fn upload_audio_wav(&self, wav_bytes: Vec<u8>) -> Result<String> {
        self.upload_audio_with_type(wav_bytes, "audio/x-wav").await
    }

    async fn upload_audio_with_type(
        &self,
        audio_bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<String> {
        let resp = self
            .client
            .post(HEYGEN_UPLOAD_URL)
            .header("X-Api-Key", &self.api_key)
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
        &self,
        avatar_id: &str,
        audio_url: &str,
        background_url: Option<&str>,
    ) -> Result<String> {
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

        let resp = self
            .client
            .post(HEYGEN_GENERATE_URL)
            .header("X-Api-Key", &self.api_key)
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

    /// Poll the status of a HeyGen video by `video_id`.
    pub async fn poll_status(&self, video_id: &str) -> Result<VideoStatus> {
        let url = format!("{}?video_id={}", HEYGEN_STATUS_URL, video_id);

        let resp = self
            .client
            .get(&url)
            .header("X-Api-Key", &self.api_key)
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStatus {
    pub status: String, // "pending" | "processing" | "completed" | "failed"
    pub video_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration: Option<f64>, // seconds
    pub error: Option<String>,
}

// ============================================================================
// Legacy standalone functions (preserved for backward compatibility)
// These create a new client per call - prefer using HeyGenClient directly
// ============================================================================

/// Upload raw audio bytes to HeyGen's asset CDN.
/// Returns the public CDN URL for use in video generation.
///
/// NOTE: Prefer using `HeyGenClient::upload_audio` for connection reuse.
pub async fn upload_audio(api_key: &str, audio_bytes: Vec<u8>) -> Result<String> {
    let client = HeyGenClient::new(api_key)?;
    client.upload_audio(audio_bytes).await
}

/// Upload WAV audio bytes to HeyGen's asset CDN.
/// WAV (16kHz mono PCM) gives HeyGen's speech recognition the best chance
/// of extracting word-level timing for expressive avatar lip sync.
///
/// NOTE: Prefer using `HeyGenClient::upload_audio_wav` for connection reuse.
pub async fn upload_audio_wav(api_key: &str, wav_bytes: Vec<u8>) -> Result<String> {
    let client = HeyGenClient::new(api_key)?;
    client.upload_audio_wav(wav_bytes).await
}

/// Generate a HeyGen talking-head video using a pre-rendered audio URL.
///
/// `avatar_id` — HeyGen avatar ID (stock or instant).
/// `audio_url`  — publicly accessible URL to the MP3 audio file.
///
/// Returns the HeyGen `video_id` for polling.
///
/// NOTE: Prefer using `HeyGenClient::generate_with_audio` for connection reuse.
pub async fn generate_with_audio(
    api_key: &str,
    avatar_id: &str,
    audio_url: &str,
    background_url: Option<&str>,
) -> Result<String> {
    let client = HeyGenClient::new(api_key)?;
    client
        .generate_with_audio(avatar_id, audio_url, background_url)
        .await
}

/// Poll the status of a HeyGen video by `video_id`.
///
/// NOTE: Prefer using `HeyGenClient::poll_status` for connection reuse.
pub async fn poll_status(api_key: &str, video_id: &str) -> Result<VideoStatus> {
    let client = HeyGenClient::new(api_key)?;
    client.poll_status(video_id).await
}
