#![allow(dead_code)]
//! MCP tools for social media operations.
//!
//! Lets an AI agent list connected social accounts, draft/schedule posts, and
//! publish them to a user's connected handles. Posts are routed through the
//! `Publisher` service, which dispatches to per-platform connectors using
//! stored OAuth tokens.

use db::models::{
    social_account::SocialAccount,
    social_post::{ContentType, CreateSocialPost, SocialPost},
};
use rmcp::{handler::server::tool::Parameters, model::CallToolResult, schemars, tool, ErrorData};
use serde::{Deserialize, Serialize};
use services::services::social::Publisher;
use uuid::Uuid;

use super::{helpers::*, TaskServer};

// ─── Request / Response types ───────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListSocialAccountsRequest {
    #[schemars(description = "Project UUID to list connected social accounts for")]
    pub project_id: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct SocialAccountSummary {
    pub id: String,
    pub platform: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub status: String,
    pub profile_url: Option<String>,
    pub follower_count: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListSocialPostsRequest {
    #[schemars(description = "Project UUID to list posts for")]
    pub project_id: String,
    #[schemars(
        description = "Optional status filter: draft, pending_review, approved, scheduled, publishing, published, failed, cancelled"
    )]
    pub status: Option<String>,
    #[schemars(description = "Max number of results to return (default: 50)")]
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetSocialPostRequest {
    #[schemars(description = "Post UUID")]
    pub post_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateSocialPostRequest {
    #[schemars(description = "Project UUID to create the post under")]
    pub project_id: String,
    #[schemars(
        description = "List of social account UUIDs to target. Use `list_social_accounts` to discover them."
    )]
    pub account_ids: Vec<String>,
    #[schemars(description = "Post caption / text body")]
    pub caption: Option<String>,
    #[schemars(
        description = "Content type: post, story, reel, carousel, thread, video, article (default: post)"
    )]
    pub content_type: Option<String>,
    #[schemars(description = "Media URLs (images, video) to attach")]
    pub media_urls: Option<Vec<String>>,
    #[schemars(description = "Hashtags (without the `#`)")]
    pub hashtags: Option<Vec<String>>,
    #[schemars(description = "Usernames to @-mention (without the `@`)")]
    pub mentions: Option<Vec<String>>,
    #[schemars(
        description = "ISO 8601 datetime to schedule the post for. Omit to leave as draft."
    )]
    pub scheduled_for: Option<String>,
    #[schemars(description = "Optional category label")]
    pub category: Option<String>,
    #[schemars(
        description = "Platform-specific extras as JSON (e.g. {\"tiktok\":{\"privacy_level\":\"PUBLIC_TO_EVERYONE\"}})"
    )]
    pub platform_specific: Option<serde_json::Value>,
    #[schemars(description = "Optional task UUID to associate this post with")]
    pub task_id: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PublishSocialPostRequest {
    #[schemars(description = "Post UUID to publish immediately, bypassing the scheduler")]
    pub post_id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PostToSocialRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,
    #[schemars(
        description = "List of social account UUIDs to publish to. Use `list_social_accounts` to discover them."
    )]
    pub account_ids: Vec<String>,
    #[schemars(description = "Post caption / text body")]
    pub caption: String,
    #[schemars(description = "Content type: post, story, reel, carousel, thread, video, article")]
    pub content_type: Option<String>,
    #[schemars(description = "Media URLs (images, video) to attach")]
    pub media_urls: Option<Vec<String>>,
    #[schemars(description = "Hashtags (without the `#`)")]
    pub hashtags: Option<Vec<String>>,
    #[schemars(description = "Usernames to @-mention (without the `@`)")]
    pub mentions: Option<Vec<String>>,
    #[schemars(
        description = "Platform-specific extras as JSON (e.g. {\"tiktok\":{\"privacy_level\":\"PUBLIC_TO_EVERYONE\"}})"
    )]
    pub platform_specific: Option<serde_json::Value>,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn parse_content_type(s: &str) -> Option<ContentType> {
    match s.to_lowercase().as_str() {
        "post" => Some(ContentType::Post),
        "story" => Some(ContentType::Story),
        "reel" => Some(ContentType::Reel),
        "carousel" => Some(ContentType::Carousel),
        "thread" => Some(ContentType::Thread),
        "video" => Some(ContentType::Video),
        "article" => Some(ContentType::Article),
        _ => None,
    }
}

fn parse_uuid_vec(ids: &[String], field: &str) -> Result<Vec<Uuid>, CallToolResult> {
    ids.iter()
        .map(|s| {
            Uuid::parse_str(s).map_err(|_| {
                error_result(
                    &format!("Invalid UUID in {field}"),
                    Some(&format!("`{s}` is not a valid UUID")),
                )
            })
        })
        .collect()
}

fn build_create_post(
    project_uuid: Uuid,
    account_uuids: Vec<Uuid>,
    caption: Option<String>,
    content_type: Option<String>,
    media_urls: Option<Vec<String>>,
    hashtags: Option<Vec<String>>,
    mentions: Option<Vec<String>>,
    scheduled_for: Option<&str>,
    category: Option<String>,
    platform_specific: Option<serde_json::Value>,
    task_id: Option<&str>,
) -> Result<CreateSocialPost, CallToolResult> {
    let content_type = content_type
        .as_deref()
        .and_then(parse_content_type)
        .or(Some(ContentType::Post));

    let task_uuid =
        match task_id {
            Some(t) => Some(Uuid::parse_str(t).map_err(|_| {
                error_result("Invalid task_id", Some("task_id must be a valid UUID"))
            })?),
            None => None,
        };

    let scheduled_for = scheduled_for.and_then(parse_iso_datetime);

    Ok(CreateSocialPost {
        project_id: project_uuid,
        social_account_id: None,
        task_id: task_uuid,
        content_type,
        caption,
        content_blocks: None,
        media_urls,
        hashtags,
        mentions,
        platforms: account_uuids.into_iter().map(|u| u.to_string()).collect(),
        platform_specific,
        status: None,
        scheduled_for,
        category,
        is_evergreen: None,
        recycle_after_days: None,
        created_by_agent_id: None,
        deliverable_id: None,
        assignee_id: None,
    })
}

fn social_post_to_json(post: &SocialPost) -> serde_json::Value {
    serde_json::json!({
        "id": post.id.to_string(),
        "project_id": post.project_id.to_string(),
        "status": post.status,
        "content_type": post.content_type,
        "caption": post.caption,
        "platforms": post.platforms,
        "scheduled_for": post.scheduled_for.map(|d| d.to_rfc3339()),
        "published_at": post.published_at.map(|d| d.to_rfc3339()),
        "platform_post_id": post.platform_post_id,
        "platform_url": post.platform_url,
        "publish_error": post.publish_error,
        "created_at": post.created_at.to_rfc3339(),
    })
}

// ─── Tool implementations ───────────────────────────────────────────────────

impl TaskServer {
    #[tool(
        description = "List social media accounts connected to a project (LinkedIn, Twitter, Instagram, TikTok, Threads, YouTube, Facebook, Bluesky, Pinterest). Returns each account's UUID, platform, username, and connection status. Use the returned UUIDs as `account_ids` for posting tools."
    )]
    pub(super) async fn list_social_accounts(
        &self,
        Parameters(req): Parameters<ListSocialAccountsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match SocialAccount::find_by_project(&self.pool, project_uuid).await {
            Ok(accounts) => {
                let summaries: Vec<SocialAccountSummary> = accounts
                    .into_iter()
                    .map(|a| SocialAccountSummary {
                        id: a.id.to_string(),
                        platform: a.platform,
                        username: a.username,
                        display_name: a.display_name,
                        status: a.status,
                        profile_url: a.profile_url,
                        follower_count: a.follower_count,
                    })
                    .collect();
                Ok(success_json(&serde_json::json!({
                    "success": true,
                    "count": summaries.len(),
                    "accounts": summaries,
                })))
            }
            Err(e) => Ok(error_result(
                "Failed to list social accounts",
                Some(&e.to_string()),
            )),
        }
    }

    #[tool(
        description = "List social media posts for a project, optionally filtered by status (draft, scheduled, published, failed, etc.)."
    )]
    pub(super) async fn list_social_posts(
        &self,
        Parameters(req): Parameters<ListSocialPostsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let limit = req.limit.or(Some(50));
        let posts = match SocialPost::find_by_project(&self.pool, project_uuid, limit).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(error_result(
                    "Failed to list social posts",
                    Some(&e.to_string()),
                ));
            }
        };

        let filtered: Vec<serde_json::Value> = posts
            .iter()
            .filter(|p| match &req.status {
                Some(s) => p.status.eq_ignore_ascii_case(s),
                None => true,
            })
            .map(social_post_to_json)
            .collect();

        Ok(success_json(&serde_json::json!({
            "success": true,
            "count": filtered.len(),
            "posts": filtered,
        })))
    }

    #[tool(description = "Get a single social post by its UUID.")]
    pub(super) async fn get_social_post(
        &self,
        Parameters(req): Parameters<GetSocialPostRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let post_uuid = match parse_uuid(&req.post_id, "post_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match SocialPost::find_by_id(&self.pool, post_uuid).await {
            Ok(post) => Ok(success_json(&serde_json::json!({
                "success": true,
                "post": social_post_to_json(&post),
            }))),
            Err(e) => Ok(error_result("Post not found", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Create a social media post (draft, or scheduled if `scheduled_for` is set). Targets one or more connected accounts via `account_ids`. Does NOT publish — use `publish_social_post` or `post_to_social` for that."
    )]
    pub(super) async fn create_social_post(
        &self,
        Parameters(req): Parameters<CreateSocialPostRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        if req.account_ids.is_empty() {
            return Ok(error_result(
                "account_ids must contain at least one social account UUID",
                None,
            ));
        }

        let account_uuids = match parse_uuid_vec(&req.account_ids, "account_ids") {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };

        let create = match build_create_post(
            project_uuid,
            account_uuids,
            req.caption,
            req.content_type,
            req.media_urls,
            req.hashtags,
            req.mentions,
            req.scheduled_for.as_deref(),
            req.category,
            req.platform_specific,
            req.task_id.as_deref(),
        ) {
            Ok(c) => c,
            Err(r) => return Ok(r),
        };

        match SocialPost::create(&self.pool, create).await {
            Ok(post) => Ok(success_json(&serde_json::json!({
                "success": true,
                "post_id": post.id.to_string(),
                "status": post.status,
                "message": "Social post created",
                "post": social_post_to_json(&post),
            }))),
            Err(e) => Ok(error_result(
                "Failed to create social post",
                Some(&e.to_string()),
            )),
        }
    }

    #[tool(
        description = "Publish an existing social post immediately, bypassing the scheduler. The post is routed through the appropriate connector(s) using stored OAuth tokens. Returns per-platform publish results."
    )]
    pub(super) async fn publish_social_post(
        &self,
        Parameters(req): Parameters<PublishSocialPostRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let post_uuid = match parse_uuid(&req.post_id, "post_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let publisher = Publisher::new(self.pool.clone());
        match publisher.publish_post(post_uuid).await {
            Ok(results) => {
                let entries: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "platform": r.platform.to_string(),
                            "platform_post_id": r.platform_post_id,
                            "platform_url": r.platform_url,
                            "published_at": r.published_at.to_rfc3339(),
                        })
                    })
                    .collect();
                Ok(success_json(&serde_json::json!({
                    "success": !entries.is_empty(),
                    "post_id": post_uuid.to_string(),
                    "results": entries,
                })))
            }
            Err(e) => Ok(error_result("Publish failed", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "One-shot: create a social post AND publish it immediately to the listed connected accounts. Use `list_social_accounts` to find the account UUIDs. Returns the new post ID plus per-platform publish results. Note: Bluesky, Pinterest, Facebook, and YouTube connectors are currently stubs and will report 'not yet implemented' at publish time."
    )]
    pub(super) async fn post_to_social(
        &self,
        Parameters(req): Parameters<PostToSocialRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        if req.account_ids.is_empty() {
            return Ok(error_result(
                "account_ids must contain at least one social account UUID",
                None,
            ));
        }

        let account_uuids = match parse_uuid_vec(&req.account_ids, "account_ids") {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };

        let create = match build_create_post(
            project_uuid,
            account_uuids,
            Some(req.caption),
            req.content_type,
            req.media_urls,
            req.hashtags,
            req.mentions,
            None,
            None,
            req.platform_specific,
            None,
        ) {
            Ok(c) => c,
            Err(r) => return Ok(r),
        };

        let post = match SocialPost::create(&self.pool, create).await {
            Ok(p) => p,
            Err(e) => {
                return Ok(error_result(
                    "Failed to create social post",
                    Some(&e.to_string()),
                ));
            }
        };

        let publisher = Publisher::new(self.pool.clone());
        match publisher.publish_post(post.id).await {
            Ok(results) => {
                let entries: Vec<serde_json::Value> = results
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "platform": r.platform.to_string(),
                            "platform_post_id": r.platform_post_id,
                            "platform_url": r.platform_url,
                            "published_at": r.published_at.to_rfc3339(),
                        })
                    })
                    .collect();
                Ok(success_json(&serde_json::json!({
                    "success": !entries.is_empty(),
                    "post_id": post.id.to_string(),
                    "results": entries,
                })))
            }
            Err(e) => Ok(error_result(
                "Post created but publish failed",
                Some(&format!("post_id={} error={}", post.id, e)),
            )),
        }
    }
}
