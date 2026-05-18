//! Social post publish cron — runs every 15 minutes.
//!
//! Polls for posts in `scheduled` status where `scheduled_for <= now` and
//! `publish_attempt < 3`, then dispatches each to the appropriate platform
//! connector. On success: marks `published`. On failure: increments attempt
//! counter and marks `failed` after 3 consecutive failures.

use chrono::Utc;
use db::models::{social_account::SocialAccount, social_post::SocialPost};
use services::services::social::{get_connector, PublishContent};
use sqlx::SqlitePool;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

const PUBLISH_INTERVAL_SECS: u64 = 15 * 60; // 15 minutes

pub fn spawn_social_publish_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(PUBLISH_INTERVAL_SECS));
        ticker.tick().await; // skip immediate first tick — let server fully start
        loop {
            ticker.tick().await;
            if let Err(e) = run_publish_cycle(&pool).await {
                error!("Social publish cycle error: {e}");
            }
        }
    });
}

async fn run_publish_cycle(pool: &SqlitePool) -> Result<(), anyhow::Error> {
    let due_posts = SocialPost::find_due_for_publish(pool).await?;
    if due_posts.is_empty() {
        return Ok(());
    }
    info!(
        "Social publisher: {} post(s) due for publishing",
        due_posts.len()
    );

    for post in due_posts {
        // Skip posts that have no real social account linked and whose platforms field
        // contains only platform name strings (e.g. ["linkedin"]) rather than account UUIDs.
        // These are draft/planning entries that haven't been wired to an OAuth account yet —
        // burning publish attempts on them would flip them to 'failed' prematurely.
        let has_account = post.social_account_id.is_some() || {
            let platform_uuids: Result<Vec<uuid::Uuid>, _> = serde_json::from_str(&post.platforms);
            platform_uuids.is_ok()
        };
        if !has_account {
            warn!(
                "Skipping post {} — platforms field contains no account UUIDs and no social_account_id",
                post.id
            );
            continue;
        }

        // Lock the post into publishing state (optimistic — ignore if already grabbed)
        let lock_result = sqlx::query(
            "UPDATE social_posts SET status = 'publishing', updated_at = datetime('now','subsec') WHERE id = ?1 AND status = 'scheduled'"
        )
        .bind(post.id)
        .execute(pool)
        .await;

        match lock_result {
            Err(e) => {
                warn!("Could not lock post {} for publishing: {}", post.id, e);
                continue;
            }
            Ok(r) if r.rows_affected() == 0 => {
                continue;
            } // Another worker beat us
            Ok(_) => {}
        }

        let result = publish_post(pool, &post).await;

        match result {
            Ok((platform_post_id, platform_url)) => {
                SocialPost::mark_published(
                    pool,
                    post.id,
                    &platform_post_id,
                    platform_url.as_deref(),
                )
                .await?;

                // Log to social_publish_log
                let log_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO social_publish_log (id, post_id, account_id, attempt_number, status, platform_post_id, platform_url, attempted_at) VALUES (?1, ?2, ?3, ?4, 'published', ?5, ?6, datetime('now','subsec'))"
                )
                .bind(&log_id)
                .bind(post.id)
                .bind(post.social_account_id.map(|u| u.to_string()).unwrap_or_default())
                .bind(post.publish_attempt + 1)
                .bind(&platform_post_id)
                .bind(&platform_url)
                .execute(pool)
                .await;

                info!("Post {} published: {}", post.id, platform_post_id);
            }
            Err(e) => {
                let error_msg = e.to_string();
                warn!(
                    "Post {} publish failed (attempt {}): {}",
                    post.id,
                    post.publish_attempt + 1,
                    error_msg
                );

                SocialPost::increment_publish_attempt(pool, post.id).await?;

                let new_status = if post.publish_attempt + 1 >= 3 {
                    "failed"
                } else {
                    "scheduled"
                };
                sqlx::query(
                    "UPDATE social_posts SET status = ?2, publish_error = ?3, updated_at = datetime('now','subsec') WHERE id = ?1"
                )
                .bind(post.id)
                .bind(new_status)
                .bind(&error_msg)
                .execute(pool)
                .await?;

                // Log failure
                let log_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO social_publish_log (id, post_id, account_id, attempt_number, status, error_message, attempted_at) VALUES (?1, ?2, ?3, ?4, 'failed', ?5, datetime('now','subsec'))"
                )
                .bind(&log_id)
                .bind(post.id)
                .bind(post.social_account_id.map(|u| u.to_string()).unwrap_or_default())
                .bind(post.publish_attempt + 1)
                .bind(&error_msg)
                .execute(pool)
                .await;
            }
        }
    }

    Ok(())
}

async fn publish_post(
    pool: &SqlitePool,
    post: &SocialPost,
) -> Result<(String, Option<String>), anyhow::Error> {
    // Resolve which social account to use
    let account = if let Some(account_id) = post.social_account_id {
        SocialAccount::find_by_id(pool, account_id)
            .await
            .map_err(|e| anyhow::anyhow!("Account lookup failed: {}", e))?
    } else if post.project_id.is_some() {
        let project_uuid = post
            .project_id
            .as_ref()
            .map(|id| Uuid::from(id.clone()))
            .unwrap();
        let accounts = SocialAccount::find_by_project(pool, project_uuid)
            .await
            .map_err(|e| anyhow::anyhow!("Account list failed: {}", e))?;
        accounts
            .into_iter()
            .find(|a| a.status == "active")
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No active social account found for project {:?}",
                    post.project_id
                )
            })?
    } else if let Some(ref org_id) = post.organization_id {
        let accounts = SocialAccount::find_by_organization(pool, org_id)
            .await
            .map_err(|e| anyhow::anyhow!("Account list failed: {}", e))?;
        accounts
            .into_iter()
            .find(|a| a.status == "active")
            .ok_or_else(|| anyhow::anyhow!("No active social account found for org {}", org_id))?
    } else {
        return Err(anyhow::anyhow!("Post has no project_id or organization_id"));
    };

    let access_token = account
        .access_token
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("Account {} has no access token", account.id))?;

    // Check token expiry
    if let Some(exp) = account.token_expires_at {
        if exp < Utc::now() {
            return Err(anyhow::anyhow!("LinkedIn access token expired at {}", exp));
        }
    }

    // Parse platform from account
    let platform = account
        .platform
        .parse::<db::models::social_account::SocialPlatform>()
        .map_err(|_| anyhow::anyhow!("Unknown platform: {}", account.platform))?;

    let connector =
        get_connector(platform).map_err(|e| anyhow::anyhow!("No connector for platform: {}", e))?;

    // Build publish content from post fields
    let caption = post.caption.clone().unwrap_or_default();
    let media_urls: Vec<String> = post
        .media_urls
        .as_deref()
        .and_then(|m| serde_json::from_str(m).ok())
        .unwrap_or_default();
    let hashtags: Vec<String> = post
        .hashtags
        .as_deref()
        .and_then(|h| serde_json::from_str::<Vec<String>>(h).ok())
        .unwrap_or_default();
    let mentions: Vec<String> = post
        .mentions
        .as_deref()
        .and_then(|m| serde_json::from_str::<Vec<String>>(m).ok())
        .unwrap_or_default();

    let content = PublishContent {
        caption,
        media_urls,
        hashtags,
        mentions,
        link: None,
        scheduled_for: post.scheduled_for,
        platform_specific: post
            .platform_specific
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok()),
    };

    let result = connector
        .publish(access_token, &content)
        .await
        .map_err(|e| anyhow::anyhow!("Publish failed: {}", e))?;

    Ok((result.platform_post_id, result.platform_url))
}
