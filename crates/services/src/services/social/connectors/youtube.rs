//! YouTube Platform Connector
//!
//! Connect-only: OAuth + channel identity. Video publishing (`publish`) still
//! requires YouTube's resumable upload protocol and is not yet implemented.
//! Reuses the Google Cloud OAuth client (same one as Gmail/Calendar/Drive).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::Client;
use serde::Deserialize;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

const YOUTUBE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const YOUTUBE_API_BASE: &str = "https://www.googleapis.com/youtube/v3";
const YOUTUBE_UPLOAD_BASE: &str = "https://www.googleapis.com/upload/youtube/v3/videos";
/// YouTube category 22 = "People & Blogs" — a safe generic default. Override
/// via `platform_specific.youtube.category_id` if a specific category matters.
const DEFAULT_CATEGORY_ID: &str = "22";
/// Safer default than `public` so a misconfigured caller can't accidentally
/// publish to the world. Caller can opt into public via platform_specific.
const DEFAULT_PRIVACY_STATUS: &str = "private";

pub struct YouTubeConnector {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl YouTubeConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("GOOGLE_CLIENT_ID")
                .or_else(|_| std::env::var("YOUTUBE_CLIENT_ID"))
                .unwrap_or_default(),
            client_secret: std::env::var("GOOGLE_CLIENT_SECRET")
                .or_else(|_| std::env::var("YOUTUBE_CLIENT_SECRET"))
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    expires_in: i64,
    refresh_token: Option<String>,
    scope: Option<String>,
    token_type: String,
}

fn tokens_from(t: GoogleTokenResponse) -> OAuthTokens {
    OAuthTokens {
        access_token: t.access_token,
        refresh_token: t.refresh_token,
        expires_at: Some(Utc::now() + chrono::Duration::seconds(t.expires_in)),
        token_type: t.token_type,
        scope: t.scope,
    }
}

#[derive(Debug, Deserialize)]
struct ChannelsResponse {
    #[serde(default)]
    items: Vec<ChannelItem>,
}

#[derive(Debug, Deserialize)]
struct ChannelItem {
    id: String,
    snippet: Option<ChannelSnippet>,
    statistics: Option<ChannelStatistics>,
}

#[derive(Debug, Deserialize)]
struct ChannelSnippet {
    title: Option<String>,
    #[serde(rename = "customUrl")]
    custom_url: Option<String>,
    thumbnails: Option<ChannelThumbnails>,
}

#[derive(Debug, Deserialize)]
struct ChannelThumbnails {
    default: Option<ChannelThumbnail>,
    high: Option<ChannelThumbnail>,
    medium: Option<ChannelThumbnail>,
}

#[derive(Debug, Deserialize)]
struct ChannelThumbnail {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChannelStatistics {
    #[serde(rename = "subscriberCount")]
    subscriber_count: Option<String>,
    #[serde(rename = "videoCount")]
    video_count: Option<String>,
}

#[derive(Debug, Deserialize)]
struct YouTubeVideoResource {
    id: String,
}

/// Fetch video bytes from a URL and return (bytes, content_type).
/// Buffers the entire video in memory — fine for short clips, but YouTube
/// allows up to 256 GB and this will OOM on anything that big. A future
/// pass should stream the source body through `reqwest::Body::wrap_stream`
/// into the resumable PUT, sending in chunks.
async fn fetch_video_bytes(client: &Client, url: &str) -> Result<(Vec<u8>, String), SocialError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(SocialError::PlatformError(format!(
            "Failed to fetch video from {url}: HTTP {}",
            resp.status()
        )));
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "video/mp4".into());
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?
        .to_vec();
    Ok((bytes, content_type))
}

/// Build the YouTube `videos.insert` request body from generic post content.
/// Reads `platform_specific.youtube.{title, privacy_status, category_id, tags}`
/// and falls back to sensible defaults.
fn build_video_metadata(content: &PublishContent) -> serde_json::Value {
    let extras = content
        .platform_specific
        .as_ref()
        .and_then(|v| v.get("youtube"));

    // YouTube title is required and capped at 100 chars. Prefer an explicit
    // override; otherwise derive from the first line of the caption.
    let title = extras
        .and_then(|y| y.get("title"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let first_line = content.caption.lines().next().unwrap_or("Untitled");
            first_line.chars().take(100).collect()
        });

    let mut description = content.caption.clone();
    if !content.hashtags.is_empty() {
        description.push_str("\n\n");
        description.push_str(
            &content
                .hashtags
                .iter()
                .map(|h| format!("#{h}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    let privacy_status = extras
        .and_then(|y| y.get("privacy_status"))
        .and_then(|p| p.as_str())
        .unwrap_or(DEFAULT_PRIVACY_STATUS);

    let category_id = extras
        .and_then(|y| y.get("category_id"))
        .and_then(|c| c.as_str())
        .unwrap_or(DEFAULT_CATEGORY_ID);

    let tags: Vec<String> = extras
        .and_then(|y| y.get("tags"))
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    serde_json::json!({
        "snippet": {
            "title": title,
            "description": description,
            "tags": tags,
            "categoryId": category_id,
        },
        "status": {
            "privacyStatus": privacy_status,
        }
    })
}

/// Initiate a resumable upload session. Returns the upload URL from the
/// `Location` header — bytes get PUT there in step 2.
async fn initiate_resumable_session(
    client: &Client,
    access_token: &str,
    metadata: &serde_json::Value,
    video_bytes: u64,
    content_type: &str,
) -> Result<String, SocialError> {
    let url = format!("{YOUTUBE_UPLOAD_BASE}?uploadType=resumable&part=snippet,status");
    let resp = client
        .post(&url)
        .bearer_auth(access_token)
        .header("X-Upload-Content-Length", video_bytes.to_string())
        .header("X-Upload-Content-Type", content_type)
        .json(metadata)
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(SocialError::PlatformError(format!(
            "YouTube resumable session init failed ({status}): {body}"
        )));
    }

    resp.headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            SocialError::PlatformError(
                "YouTube did not return a Location header for resumable upload".into(),
            )
        })
}

/// PUT the video bytes to the resumable upload URL in a single request.
/// YouTube also supports chunked uploads (Content-Range), but a single PUT
/// is the simplest correct implementation for now.
async fn upload_video_bytes(
    client: &Client,
    upload_url: &str,
    bytes: Vec<u8>,
    content_type: &str,
) -> Result<String, SocialError> {
    let resp = client
        .put(upload_url)
        .header(reqwest::header::CONTENT_TYPE, content_type)
        .header(reqwest::header::CONTENT_LENGTH, bytes.len().to_string())
        .body(bytes)
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(SocialError::PlatformError(format!(
            "YouTube video upload failed ({status}): {body}"
        )));
    }

    let video: YouTubeVideoResource = resp
        .json()
        .await
        .map_err(|e| SocialError::PlatformError(format!("Parse upload response: {e}")))?;
    Ok(video.id)
}

#[async_trait]
impl PlatformConnector for YouTubeConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::YouTube
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "https://www.googleapis.com/auth/youtube.upload \
                      https://www.googleapis.com/auth/youtube.readonly";
        Ok(format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&state={}&scope={}\
             &access_type=offline&prompt=consent",
            YOUTUBE_AUTH_URL,
            self.client_id,
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(scopes),
        ))
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
        _code_verifier: Option<&str>,
    ) -> Result<OAuthTokens, SocialError> {
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];
        let resp = self
            .client
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SocialError::AuthError(format!(
                "YouTube token exchange failed: {body}"
            )));
        }
        let t: GoogleTokenResponse = resp
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;
        Ok(tokens_from(t))
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
        ];
        let resp = self
            .client
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SocialError::AuthError(format!(
                "YouTube token refresh failed: {body}"
            )));
        }
        let t: GoogleTokenResponse = resp
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;
        Ok(tokens_from(t))
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        // /channels?mine=true returns the channel(s) owned by the authorising
        // user — usually one. Pick the first; if a user has multiple, they
        // can disambiguate from the YouTube account chooser at consent time.
        let resp = self
            .client
            .get(format!(
                "{YOUTUBE_API_BASE}/channels?part=snippet,statistics&mine=true"
            ))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SocialError::PlatformError(format!(
                "YouTube channels fetch failed: {body}"
            )));
        }
        let payload: ChannelsResponse = resp
            .json()
            .await
            .map_err(|e| SocialError::PlatformError(e.to_string()))?;
        let channel = payload.items.into_iter().next().ok_or_else(|| {
            SocialError::PlatformError(
                "YouTube returned no channel for the authenticated user".into(),
            )
        })?;

        let snippet = channel.snippet;
        let display_name = snippet.as_ref().and_then(|s| s.title.clone());
        let username = snippet
            .as_ref()
            .and_then(|s| s.custom_url.clone())
            .unwrap_or_else(|| channel.id.clone());
        let avatar_url = snippet
            .as_ref()
            .and_then(|s| s.thumbnails.as_ref())
            .and_then(|t| {
                t.high
                    .as_ref()
                    .or(t.medium.as_ref())
                    .or(t.default.as_ref())
            })
            .and_then(|t| t.url.clone());
        let profile_url = Some(format!("https://www.youtube.com/channel/{}", channel.id));
        let follower_count = channel
            .statistics
            .as_ref()
            .and_then(|s| s.subscriber_count.as_deref())
            .and_then(|s| s.parse::<i64>().ok());
        let post_count = channel
            .statistics
            .as_ref()
            .and_then(|s| s.video_count.as_deref())
            .and_then(|s| s.parse::<i64>().ok());

        Ok(ProfileInfo {
            platform_account_id: channel.id,
            username,
            display_name,
            profile_url,
            avatar_url,
            follower_count,
            following_count: None,
            post_count,
            is_verified: false,
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        self.validate_content(content)?;

        let video_url = content.media_urls.first().ok_or_else(|| {
            SocialError::ValidationError("YouTube requires a video URL".into())
        })?;

        let (video_bytes, content_type) = fetch_video_bytes(&self.client, video_url).await?;
        let metadata = build_video_metadata(content);
        let upload_url = initiate_resumable_session(
            &self.client,
            access_token,
            &metadata,
            video_bytes.len() as u64,
            &content_type,
        )
        .await?;
        let video_id =
            upload_video_bytes(&self.client, &upload_url, video_bytes, &content_type).await?;

        Ok(PublishResult {
            platform: SocialPlatform::YouTube,
            platform_post_id: video_id.clone(),
            platform_url: Some(format!("https://www.youtube.com/watch?v={video_id}")),
            published_at: Utc::now(),
        })
    }

    async fn get_metrics(
        &self,
        _access_token: &str,
        _platform_post_id: &str,
    ) -> Result<EngagementMetrics, SocialError> {
        Ok(EngagementMetrics::default())
    }

    async fn fetch_mentions(
        &self,
        _access_token: &str,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PlatformMention>, SocialError> {
        Ok(vec![])
    }

    async fn reply_to_mention(
        &self,
        _access_token: &str,
        _mention_id: &str,
        _content: &str,
    ) -> Result<String, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "YouTube comment reply not yet implemented".into(),
        ))
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        if content.media_urls.is_empty() {
            return Err(SocialError::ValidationError(
                "YouTube requires a video file".into(),
            ));
        }
        let limits = self.get_limits();
        if content.caption.len() > limits.max_caption_length {
            return Err(SocialError::ValidationError(format!(
                "Description exceeds {} character limit",
                limits.max_caption_length
            )));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 5_000, // description
            max_hashtags: 15,
            max_mentions: 0,
            max_images: 0,
            max_video_length_seconds: 12 * 60 * 60, // 12h verified accounts
            max_image_size_bytes: 0,
            max_video_size_bytes: 256 * 1024 * 1024 * 1024, // 256GB
            supported_media_types: vec![
                "video/mp4".into(),
                "video/quicktime".into(),
                "video/x-msvideo".into(),
                "video/webm".into(),
            ],
        }
    }
}
