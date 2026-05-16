//! Facebook Platform Connector
//!
//! Implements Facebook Page publishing via Graph API v19. Shares Meta App
//! credentials with Instagram/Threads but persists a **Page access token** —
//! `/feed`, `/photos`, `/videos`, and `/insights` all require it.
//!
//! OAuth flow inside `exchange_code`:
//!   1. code   → short-lived user token
//!   2. short  → long-lived user token (~60d)
//!   3. user   → `/me/accounts` → first manageable Page's `access_token`
//!
//! Page tokens derived from a long-lived user token are non-expiring while the
//! user's grant stays valid; we still mark `expires_at = now + 55d` so the
//! refresh worker stops pinging and the user re-authorizes cleanly.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use db::models::social_account::SocialPlatform;
use reqwest::{Client, Response};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::info;

use crate::services::social::{
    EngagementMetrics, OAuthTokens, PlatformConnector, PlatformLimits, PlatformMention,
    ProfileInfo, PublishContent, PublishResult, SocialError,
};

const META_AUTH_URL: &str = "https://www.facebook.com/v19.0/dialog/oauth";
const META_TOKEN_URL: &str = "https://graph.facebook.com/v19.0/oauth/access_token";
const GRAPH_API_BASE: &str = "https://graph.facebook.com/v19.0";

pub struct FacebookConnector {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl FacebookConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("FACEBOOK_APP_ID")
                .or_else(|_| std::env::var("META_CLIENT_ID"))
                .unwrap_or_default(),
            client_secret: std::env::var("FACEBOOK_APP_SECRET")
                .or_else(|_| std::env::var("META_CLIENT_SECRET"))
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct MetaTokenResponse {
    access_token: String,
    token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PagesResponse {
    data: Vec<PageData>,
}

#[derive(Debug, Deserialize)]
struct PageData {
    id: String,
    name: String,
    access_token: String,
    tasks: Option<Vec<String>>,
}

#[async_trait]
impl PlatformConnector for FacebookConnector {
    fn platform(&self) -> SocialPlatform {
        SocialPlatform::Facebook
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, SocialError> {
        let scopes = "pages_show_list,pages_read_engagement,pages_manage_posts,pages_manage_engagement,read_insights,public_profile";
        Ok(format!(
            "{}?client_id={}&redirect_uri={}&state={}&scope={}&response_type=code",
            META_AUTH_URL,
            urlencoding::encode(&self.client_id),
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
        // 1) code → short-lived user token
        let short = self
            .client
            .get(META_TOKEN_URL)
            .query(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("redirect_uri", redirect_uri),
                ("code", code),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        let short: MetaTokenResponse = parse_json(short, "token exchange").await?;

        // 2) short-lived → long-lived user token (~60d)
        let long = self
            .client
            .get(format!("{}/oauth/access_token", GRAPH_API_BASE))
            .query(&[
                ("grant_type", "fb_exchange_token"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("fb_exchange_token", short.access_token.as_str()),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        let long: MetaTokenResponse = parse_json(long, "long-lived token exchange").await?;

        // 3) Resolve the first manageable Page → Page access token.
        let pages_resp = self
            .client
            .get(format!("{}/me/accounts", GRAPH_API_BASE))
            .query(&[
                ("fields", "id,name,access_token,tasks"),
                ("access_token", long.access_token.as_str()),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        let pages: PagesResponse = parse_json(pages_resp, "list pages").await?;

        let page = pages
            .data
            .into_iter()
            .find(|p| {
                p.tasks
                    .as_ref()
                    .map(|tasks| tasks.iter().any(|t| t == "CREATE_CONTENT" || t == "MANAGE"))
                    .unwrap_or(true)
            })
            .ok_or_else(|| {
                SocialError::AuthError("No manageable Facebook Page found for this account".into())
            })?;

        info!(
            "Facebook connector resolved page {} ({}) for publishing",
            page.name, page.id
        );

        // Page tokens from a long-lived user token are effectively non-expiring,
        // but Meta caps the underlying grant at ~60 days. Track 55d so the
        // refresh worker doesn't churn against UnsupportedProvider.
        let expires_at = Some(Utc::now() + chrono::Duration::days(55));

        Ok(OAuthTokens {
            access_token: page.access_token,
            refresh_token: None,
            expires_at,
            token_type: long.token_type.unwrap_or_else(|| "Bearer".into()),
            scope: None,
        })
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, SocialError> {
        Err(SocialError::UnsupportedPlatform(
            "Facebook page tokens have no refresh flow — reconnect to extend".into(),
        ))
    }

    async fn get_profile(&self, access_token: &str) -> Result<ProfileInfo, SocialError> {
        // A Page access token identifies the Page itself when called on /me.
        let resp = self
            .client
            .get(format!("{}/me", GRAPH_API_BASE))
            .query(&[
                (
                    "fields",
                    "id,name,username,link,fan_count,picture.type(large),verification_status",
                ),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        let body: Value = parse_json(resp, "profile fetch").await?;

        let id = body["id"].as_str().unwrap_or_default().to_string();
        let name = body["name"].as_str().unwrap_or_default().to_string();
        let username = body["username"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.clone());
        let avatar_url = body["picture"]["data"]["url"].as_str().map(String::from);
        let profile_url = body["link"]
            .as_str()
            .map(String::from)
            .or_else(|| Some(format!("https://facebook.com/{}", id)));
        let follower_count = body["fan_count"].as_i64();
        let is_verified = body["verification_status"]
            .as_str()
            .map(|s| matches!(s, "blue_verified" | "gray_verified"))
            .unwrap_or(false);

        Ok(ProfileInfo {
            platform_account_id: id,
            username,
            display_name: Some(name),
            profile_url,
            avatar_url,
            follower_count,
            following_count: None,
            post_count: None,
            is_verified,
        })
    }

    async fn publish(
        &self,
        access_token: &str,
        content: &PublishContent,
    ) -> Result<PublishResult, SocialError> {
        self.validate_content(content)?;

        let message = compose_message(&content.caption, &content.hashtags);
        let page_id = resolve_page_id(&self.client, access_token).await?;

        let post_id = match content.media_urls.as_slice() {
            [] => {
                publish_text(
                    &self.client,
                    &page_id,
                    access_token,
                    &message,
                    content.link.as_deref(),
                )
                .await?
            }
            [single] if is_video(single) => {
                publish_video(&self.client, &page_id, access_token, single, &message).await?
            }
            [single] => {
                publish_single_photo(&self.client, &page_id, access_token, single, &message).await?
            }
            many => publish_carousel(&self.client, &page_id, access_token, many, &message).await?,
        };

        let url_id = post_id.rsplit('_').next().unwrap_or(post_id.as_str());
        let post_url = Some(format!("https://facebook.com/{}/posts/{}", page_id, url_id));

        Ok(PublishResult {
            platform: SocialPlatform::Facebook,
            platform_post_id: post_id,
            platform_url: post_url,
            published_at: Utc::now(),
        })
    }

    async fn get_metrics(
        &self,
        access_token: &str,
        platform_post_id: &str,
    ) -> Result<EngagementMetrics, SocialError> {
        let mut metrics = EngagementMetrics::default();

        let insights_resp = self
            .client
            .get(format!("{}/{}/insights", GRAPH_API_BASE, platform_post_id))
            .query(&[
                (
                    "metric",
                    "post_impressions,post_impressions_unique,post_clicks",
                ),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if insights_resp.status().is_success() {
            let body: Value = insights_resp
                .json()
                .await
                .map_err(|e| SocialError::PlatformError(e.to_string()))?;
            for item in body["data"].as_array().into_iter().flatten() {
                let name = item["name"].as_str().unwrap_or_default();
                let value = item["values"][0]["value"].as_i64().unwrap_or(0);
                match name {
                    "post_impressions" => metrics.impressions = value,
                    "post_impressions_unique" => metrics.reach = value,
                    "post_clicks" => metrics.clicks = value,
                    _ => {}
                }
            }
        }

        let counts_resp = self
            .client
            .get(format!("{}/{}", GRAPH_API_BASE, platform_post_id))
            .query(&[
                (
                    "fields",
                    "likes.summary(true),comments.summary(true),shares",
                ),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;

        if counts_resp.status().is_success() {
            let body: Value = counts_resp
                .json()
                .await
                .map_err(|e| SocialError::PlatformError(e.to_string()))?;
            metrics.likes = body["likes"]["summary"]["total_count"]
                .as_i64()
                .unwrap_or(0);
            metrics.comments = body["comments"]["summary"]["total_count"]
                .as_i64()
                .unwrap_or(0);
            metrics.shares = body["shares"]["count"].as_i64().unwrap_or(0);
        }

        Ok(metrics)
    }

    async fn fetch_mentions(
        &self,
        _access_token: &str,
        _since: Option<DateTime<Utc>>,
    ) -> Result<Vec<PlatformMention>, SocialError> {
        // Comments span every Page post; deferred until the mention sync job
        // can iterate the post list. Returning empty keeps the inbox loop quiet.
        Ok(vec![])
    }

    async fn reply_to_mention(
        &self,
        access_token: &str,
        mention_id: &str,
        content: &str,
    ) -> Result<String, SocialError> {
        let resp = self
            .client
            .post(format!("{}/{}/comments", GRAPH_API_BASE, mention_id))
            .form(&[("message", content), ("access_token", access_token)])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        let body: Value = parse_json(resp, "reply").await?;
        Ok(body["id"].as_str().unwrap_or_default().to_string())
    }

    fn validate_content(&self, content: &PublishContent) -> Result<(), SocialError> {
        let limits = self.get_limits();
        if content.caption.len() > limits.max_caption_length {
            return Err(SocialError::ValidationError(format!(
                "Caption exceeds {} character limit",
                limits.max_caption_length
            )));
        }
        if content.media_urls.len() > limits.max_images {
            return Err(SocialError::ValidationError(format!(
                "Too many media items (max {})",
                limits.max_images
            )));
        }
        Ok(())
    }

    fn get_limits(&self) -> PlatformLimits {
        PlatformLimits {
            max_caption_length: 63_206,
            max_hashtags: 30,
            max_mentions: 50,
            max_images: 10,
            max_video_length_seconds: 240 * 60,
            max_image_size_bytes: 4 * 1024 * 1024,
            max_video_size_bytes: 10 * 1024 * 1024 * 1024,
            supported_media_types: vec![
                "image/jpeg".into(),
                "image/png".into(),
                "image/gif".into(),
                "video/mp4".into(),
            ],
        }
    }
}

// ─── Publish helpers ───────────────────────────────────────────────────────

async fn publish_text(
    client: &Client,
    page_id: &str,
    access_token: &str,
    message: &str,
    link: Option<&str>,
) -> Result<String, SocialError> {
    let mut params = vec![("message", message), ("access_token", access_token)];
    if let Some(link) = link {
        params.push(("link", link));
    }
    let resp = client
        .post(format!("{}/{}/feed", GRAPH_API_BASE, page_id))
        .form(&params)
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;
    extract_post_id(resp, "feed").await
}

async fn publish_single_photo(
    client: &Client,
    page_id: &str,
    access_token: &str,
    media_url: &str,
    caption: &str,
) -> Result<String, SocialError> {
    let resp = client
        .post(format!("{}/{}/photos", GRAPH_API_BASE, page_id))
        .form(&[
            ("url", media_url),
            ("caption", caption),
            ("access_token", access_token),
        ])
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;
    extract_post_id(resp, "photos").await
}

async fn publish_video(
    client: &Client,
    page_id: &str,
    access_token: &str,
    media_url: &str,
    description: &str,
) -> Result<String, SocialError> {
    let resp = client
        .post(format!("{}/{}/videos", GRAPH_API_BASE, page_id))
        .form(&[
            ("file_url", media_url),
            ("description", description),
            ("access_token", access_token),
        ])
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;
    extract_post_id(resp, "videos").await
}

async fn publish_carousel(
    client: &Client,
    page_id: &str,
    access_token: &str,
    media_urls: &[String],
    message: &str,
) -> Result<String, SocialError> {
    let mut media_fbids = Vec::with_capacity(media_urls.len());
    for url in media_urls {
        if is_video(url) {
            return Err(SocialError::ValidationError(
                "Facebook multi-attachment posts support photos only".into(),
            ));
        }
        let resp = client
            .post(format!("{}/{}/photos", GRAPH_API_BASE, page_id))
            .form(&[
                ("url", url.as_str()),
                ("published", "false"),
                ("access_token", access_token),
            ])
            .send()
            .await
            .map_err(|e| SocialError::NetworkError(e.to_string()))?;
        media_fbids.push(extract_post_id(resp, "photos").await?);
    }

    let attached: Vec<Value> = media_fbids
        .iter()
        .map(|fbid| json!({ "media_fbid": fbid }))
        .collect();
    let attached_str =
        serde_json::to_string(&attached).map_err(|e| SocialError::PlatformError(e.to_string()))?;

    let resp = client
        .post(format!("{}/{}/feed", GRAPH_API_BASE, page_id))
        .form(&[
            ("message", message),
            ("attached_media", attached_str.as_str()),
            ("access_token", access_token),
        ])
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;
    extract_post_id(resp, "feed").await
}

// ─── Utility helpers ───────────────────────────────────────────────────────

fn compose_message(caption: &str, hashtags: &[String]) -> String {
    if hashtags.is_empty() {
        return caption.to_string();
    }
    let tags = hashtags
        .iter()
        .map(|h| format!("#{}", h.trim_start_matches('#')))
        .collect::<Vec<_>>()
        .join(" ");
    if caption.is_empty() {
        tags
    } else {
        format!("{}\n\n{}", caption, tags)
    }
}

fn is_video(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.ends_with(".mp4") || lower.ends_with(".mov") || lower.contains("/video")
}

async fn resolve_page_id(client: &Client, access_token: &str) -> Result<String, SocialError> {
    let resp = client
        .get(format!("{}/me", GRAPH_API_BASE))
        .query(&[("fields", "id"), ("access_token", access_token)])
        .send()
        .await
        .map_err(|e| SocialError::NetworkError(e.to_string()))?;
    let body: Value = parse_json(resp, "resolve page id").await?;
    body["id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| SocialError::PlatformError("No page id in /me response".into()))
}

async fn extract_post_id(resp: Response, context: &str) -> Result<String, SocialError> {
    let body: Value = parse_json(resp, context).await?;
    body["post_id"]
        .as_str()
        .or_else(|| body["id"].as_str())
        .map(String::from)
        .ok_or_else(|| SocialError::PlatformError(format!("Facebook {} returned no id", context)))
}

async fn parse_json<T: for<'de> Deserialize<'de>>(
    resp: Response,
    context: &str,
) -> Result<T, SocialError> {
    if !resp.status().is_success() {
        let status = resp.status();
        let err = resp.text().await.unwrap_or_default();
        return Err(SocialError::PlatformError(format!(
            "Facebook {} failed ({}): {}",
            context, status, err
        )));
    }
    resp.json::<T>()
        .await
        .map_err(|e| SocialError::PlatformError(format!("Facebook {} parse error: {}", context, e)))
}
