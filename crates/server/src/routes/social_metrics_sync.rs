//! Social post metrics sync worker.
//!
//! Background loop that periodically fetches engagement metrics from social
//! platforms for published posts and writes them to:
//!   - `social_posts` (current totals)
//!   - `social_post_metrics_snapshots` (time-series history)
//!
//! Sync frequency varies by post age to balance API quota usage:
//!   - < 48 h since publish  → sync every run (every 30 min)
//!   - 2 – 14 days old       → sync when last update > 6 h ago
//!   - > 14 days old         → sync when last update > 24 h ago

use chrono::Utc;
use services::services::social::get_connector;
use sqlx::SqlitePool;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

const SYNC_INTERVAL_SECS: u64 = 30 * 60; // 30 minutes
const STARTUP_DELAY_SECS: u64 = 120; // 2 min after boot before first sync

// Age-window thresholds
const HOURS_FRESH: i64 = 48;
const HOURS_MEDIUM: i64 = 14 * 24; // 14 days

// Minimum gap between syncs per age group
const SYNC_GAP_MEDIUM_HOURS: i64 = 6;
const SYNC_GAP_OLD_HOURS: i64 = 24;

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn spawn_metrics_sync_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        // Wait for server to fully start before first sync
        tokio::time::sleep(Duration::from_secs(STARTUP_DELAY_SECS)).await;
        let mut ticker = interval(Duration::from_secs(SYNC_INTERVAL_SECS));
        loop {
            ticker.tick().await;
            if let Err(e) = run_metrics_sync(&pool).await {
                error!("Social metrics sync error: {e}");
            }
        }
    });
}

// ─── Internal row types ───────────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct SyncPost {
    // BLOB columns — sqlx uuid feature reads them as Uuid
    id: Uuid,
    platform_post_id: Option<String>,
    platforms: String,
    social_account_id: Option<Uuid>,
    published_at: Option<String>,
    updated_at: String,
}

#[derive(sqlx::FromRow)]
struct AccountTokenRow {
    platform: String,
    access_token: Option<String>,
}

// ─── Core sync logic ──────────────────────────────────────────────────────────

async fn run_metrics_sync(pool: &SqlitePool) -> Result<(), anyhow::Error> {
    let now = Utc::now();

    let posts: Vec<SyncPost> = sqlx::query_as(
        r#"SELECT sp.id, sp.platform_post_id, sp.platforms, sp.social_account_id,
                  sp.published_at, sp.updated_at
           FROM social_posts sp
           WHERE sp.status = 'published'
             AND sp.platform_post_id IS NOT NULL
             AND sp.platform_post_id != ''
           ORDER BY sp.published_at DESC
           LIMIT 200"#,
    )
    .fetch_all(pool)
    .await?;

    if posts.is_empty() {
        return Ok(());
    }

    let mut synced = 0u32;

    for post in posts {
        let platform_post_id = match &post.platform_post_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => continue,
        };

        // Parse published_at to decide sync frequency
        let age_hours = parse_age_hours(&post.published_at, now);
        let last_update_hours = parse_update_hours(&post.updated_at, now);

        let should_sync = match age_hours {
            h if h < HOURS_FRESH => true,
            h if h < HOURS_MEDIUM => last_update_hours >= SYNC_GAP_MEDIUM_HOURS,
            _ => last_update_hours >= SYNC_GAP_OLD_HOURS,
        };

        if !should_sync {
            continue;
        }

        // Look up the social account for this post to get its access token and platform.
        // social_accounts.id is BLOB — sqlx uuid feature handles Uuid↔BLOB automatically.
        let account_row: Option<AccountTokenRow> = if let Some(acct_id) = post.social_account_id {
            sqlx::query_as(
                "SELECT platform, access_token FROM social_accounts WHERE id = ? LIMIT 1",
            )
            .bind(acct_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
        } else {
            None
        };

        let (platform_str, access_token) = match account_row {
            Some(row) => match row.access_token {
                Some(tok) if !tok.is_empty() => (row.platform, tok),
                _ => {
                    warn!(
                        post_id = %post.id,
                        "Social account has no access token — skipping metrics sync"
                    );
                    continue;
                }
            },
            None => {
                // Try to infer platform from the platforms JSON array
                let platform = infer_platform_from_json(&post.platforms);
                match platform {
                    Some(p) => {
                        warn!(
                            post_id = %post.id,
                            platform = %p,
                            "No social_account_id on post — cannot fetch metrics without token"
                        );
                        continue;
                    }
                    None => continue,
                }
            }
        };

        let social_platform =
            match platform_str.parse::<db::models::social_account::SocialPlatform>() {
                Ok(p) => p,
                Err(_) => {
                    warn!(post_id = %post.id, platform = %platform_str, "Unknown platform");
                    continue;
                }
            };

        let connector = match get_connector(social_platform) {
            Ok(c) => c,
            Err(e) => {
                warn!(post_id = %post.id, error = %e, "No connector for platform");
                continue;
            }
        };

        let metrics = match connector
            .get_metrics(&access_token, &platform_post_id)
            .await
        {
            Ok(m) => m,
            Err(e) => {
                warn!(post_id = %post.id, error = %e, "Failed to fetch metrics");
                continue;
            }
        };

        let engagement_rate = if metrics.impressions > 0 {
            (metrics.likes + metrics.comments + metrics.shares) as f64 / metrics.impressions as f64
        } else {
            0.0
        };

        // Update social_posts with latest totals
        let update_result = sqlx::query(
            "UPDATE social_posts SET
                impressions = ?2,
                reach = ?3,
                likes = ?4,
                comments = ?5,
                shares = ?6,
                saves = ?7,
                clicks = ?8,
                engagement_rate = ?9,
                updated_at = datetime('now','subsec')
             WHERE id = ?1",
        )
        .bind(post.id)
        .bind(metrics.impressions)
        .bind(metrics.reach)
        .bind(metrics.likes)
        .bind(metrics.comments)
        .bind(metrics.shares)
        .bind(metrics.saves)
        .bind(metrics.clicks)
        .bind(engagement_rate)
        .execute(pool)
        .await;

        if let Err(e) = update_result {
            error!(post_id = %post.id, error = %e, "Failed to update social_posts metrics");
            continue;
        }

        // Insert snapshot row into social_post_metrics_snapshots
        let snapshot_id = Uuid::new_v4();
        let snapshot_result = sqlx::query(
            "INSERT INTO social_post_metrics_snapshots \
             (id, post_id, social_account_id, platform, captured_at, \
              impressions, reach, likes, comments, shares, saves, clicks, \
              video_views, engagement_rate) \
             VALUES (?1, ?2, ?3, ?4, datetime('now','subsec'), \
                     ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12)",
        )
        .bind(snapshot_id)
        .bind(post.id)
        .bind(post.social_account_id)
        .bind(&platform_str)
        .bind(metrics.impressions)
        .bind(metrics.reach)
        .bind(metrics.likes)
        .bind(metrics.comments)
        .bind(metrics.shares)
        .bind(metrics.saves)
        .bind(metrics.clicks)
        .bind(engagement_rate)
        .execute(pool)
        .await;

        if let Err(e) = snapshot_result {
            error!(post_id = %post.id, error = %e, "Failed to insert metrics snapshot");
        } else {
            synced += 1;
        }
    }

    if synced > 0 {
        info!("Social metrics sync: updated {synced} post(s)");
    }

    Ok(())
}

// ─── Date parsing helpers ─────────────────────────────────────────────────────

/// Parse a datetime string into a `DateTime<Utc>`. Handles both RFC 3339
/// (what sqlx stores) and SQLite's plain `YYYY-MM-DD HH:MM:SS` format.
fn parse_dt(s: &str) -> Option<chrono::DateTime<Utc>> {
    // Try RFC 3339 first (sqlx default)
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }
    // Fallback: SQLite plain datetime (no timezone — assume UTC)
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(ndt.and_utc());
    }
    // With subsecond precision
    if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(ndt.and_utc());
    }
    None
}

fn parse_age_hours(published_at: &Option<String>, now: chrono::DateTime<Utc>) -> i64 {
    published_at
        .as_deref()
        .and_then(parse_dt)
        .map(|dt| (now - dt).num_hours())
        .unwrap_or(i64::MAX) // Unknown age → treat as very old
}

fn parse_update_hours(updated_at: &str, now: chrono::DateTime<Utc>) -> i64 {
    parse_dt(updated_at)
        .map(|dt| (now - dt).num_hours())
        .unwrap_or(i64::MAX)
}

/// Try to read the first element of the platforms JSON array to infer platform name.
fn infer_platform_from_json(platforms_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(platforms_json)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .and_then(|arr| arr.into_iter().next())
        .and_then(|v| v.as_str().map(|s| s.to_string()))
}

// ─── Account follower snapshot worker ────────────────────────────────────────

const SNAPSHOT_INTERVAL_SECS: u64 = 86_400; // once per day
const SNAPSHOT_STARTUP_DELAY_SECS: u64 = 5 * 60; // 5 min startup delay

#[derive(sqlx::FromRow)]
struct AccountSnapshotRow {
    id: Uuid,
    platform: String,
    access_token: Option<String>,
    project_id: Option<String>,
    organization_id: Option<String>,
}

pub fn spawn_account_snapshot_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(SNAPSHOT_STARTUP_DELAY_SECS)).await;
        let mut ticker = interval(Duration::from_secs(SNAPSHOT_INTERVAL_SECS));
        loop {
            ticker.tick().await;
            if let Err(e) = run_account_snapshot(&pool).await {
                error!("Account snapshot error: {e}");
            }
        }
    });
}

async fn run_account_snapshot(pool: &SqlitePool) -> Result<(), anyhow::Error> {
    let accounts: Vec<AccountSnapshotRow> = sqlx::query_as(
        "SELECT id, platform, access_token, project_id, organization_id \
         FROM social_accounts \
         WHERE status = 'active' AND access_token IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    if accounts.is_empty() {
        return Ok(());
    }

    let mut snapped = 0u32;

    for account in accounts {
        let access_token = match &account.access_token {
            Some(tok) if !tok.is_empty() => tok.clone(),
            _ => continue,
        };

        let social_platform = match account
            .platform
            .parse::<db::models::social_account::SocialPlatform>()
        {
            Ok(p) => p,
            Err(_) => {
                warn!(
                    account_id = %account.id,
                    platform = %account.platform,
                    "Unknown platform — skipping snapshot"
                );
                continue;
            }
        };

        let connector = match get_connector(social_platform) {
            Ok(c) => c,
            Err(e) => {
                warn!(account_id = %account.id, error = %e, "No connector for platform");
                continue;
            }
        };

        let profile = match connector.get_profile(&access_token).await {
            Ok(p) => p,
            Err(e) => {
                warn!(account_id = %account.id, error = %e, "get_profile failed");
                continue;
            }
        };

        let snapshot_id = Uuid::new_v4();
        let project_id_str = account.project_id.as_deref().unwrap_or("");
        let org_id_str = account.organization_id.as_deref().unwrap_or("");

        let upsert = sqlx::query(
            r#"INSERT INTO social_account_snapshots
                   (id, account_id, project_id, organization_id, platform, snapshot_date,
                    follower_count, following_count, post_count)
               VALUES (?1, ?2, ?3, ?4, ?5, date('now'), ?6, ?7, ?8)
               ON CONFLICT(account_id, snapshot_date) DO UPDATE SET
                   follower_count  = excluded.follower_count,
                   following_count = excluded.following_count,
                   post_count      = excluded.post_count"#,
        )
        .bind(snapshot_id)
        .bind(account.id)
        .bind(project_id_str)
        .bind(org_id_str)
        .bind(&account.platform)
        .bind(profile.follower_count.unwrap_or(0))
        .bind(profile.following_count.unwrap_or(0))
        .bind(profile.post_count.unwrap_or(0))
        .execute(pool)
        .await;

        if let Err(e) = upsert {
            error!(account_id = %account.id, error = %e, "Failed to upsert snapshot");
            continue;
        }

        // Update live follower/post counts on the account row
        let update = sqlx::query(
            "UPDATE social_accounts \
             SET follower_count = ?2, post_count = ?3, updated_at = datetime('now','subsec') \
             WHERE id = ?1",
        )
        .bind(account.id)
        .bind(profile.follower_count.unwrap_or(0))
        .bind(profile.post_count.unwrap_or(0))
        .execute(pool)
        .await;

        if let Err(e) = update {
            error!(account_id = %account.id, error = %e, "Failed to update social_accounts");
        } else {
            snapped += 1;
        }
    }

    if snapped > 0 {
        info!("Account snapshot: captured {snapped} account(s)");
    }

    Ok(())
}
