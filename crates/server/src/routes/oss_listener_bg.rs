//! OSS Listener background worker.
//!
//! Polls GitHub Releases API for each tracked library. When a new release is
//! found an agent_flow workflow is created and Nora generates an upgrade
//! recommendation that is written back to oss_library_updates.

use chrono;
use db::models::oss_library::OssLibrary;
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};
use serde::Deserialize;
use sqlx::SqlitePool;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

// ── GitHub release payload (only fields we need) ──────────────────────────────

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    prerelease: bool,
    draft: bool,
    published_at: Option<String>,
}

// ── Spawn ─────────────────────────────────────────────────────────────────────

pub fn spawn_oss_listener(pool: SqlitePool) {
    tokio::spawn(async move {
        // Stagger 5 minutes after startup so the server is fully initialised
        tokio::time::sleep(Duration::from_secs(300)).await;

        let mut ticker = interval(Duration::from_secs(3600));
        ticker.tick().await; // discard the immediate first tick
        loop {
            ticker.tick().await;
            if let Err(e) = run_checks(&pool).await {
                error!("[OSS_LISTENER] Check cycle error: {}", e);
            }
        }
    });
}

async fn run_checks(pool: &SqlitePool) -> anyhow::Result<()> {
    let due = OssLibrary::list_due_for_check(pool).await?;
    if due.is_empty() {
        return Ok(());
    }
    info!("[OSS_LISTENER] Checking {} libraries", due.len());

    for lib in &due {
        if let Err(e) = check_and_process_library(pool, lib).await {
            error!("[OSS_LISTENER] Failed for '{}': {}", lib.name, e);
        }
    }
    Ok(())
}

/// Public so the manual-check endpoint can call it.
pub async fn check_and_process_library(pool: &SqlitePool, lib: &OssLibrary) -> anyhow::Result<()> {
    // Update last_checked_at immediately so we don't re-check on crash
    sqlx::query(
        "UPDATE oss_libraries SET last_checked_at = datetime('now','subsec'), updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(lib.id)
    .execute(pool)
    .await?;

    let release = fetch_latest_release(&lib.github_owner, &lib.github_repo).await?;

    if release.prerelease || release.draft {
        info!(
            "[OSS_LISTENER] '{}' latest is pre-release/draft ({}), skipping",
            lib.name, release.tag_name
        );
        return Ok(());
    }

    // Persist latest_version on library record
    sqlx::query(
        "UPDATE oss_libraries SET latest_version = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(&release.tag_name)
    .bind(lib.id)
    .execute(pool)
    .await?;

    // Already processed this version?
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM oss_library_updates WHERE library_id = ? AND version = ?",
    )
    .bind(lib.id)
    .bind(&release.tag_name)
    .fetch_one(pool)
    .await?;

    if exists > 0 {
        info!(
            "[OSS_LISTENER] '{}' {} already recorded",
            lib.name, release.tag_name
        );
        return Ok(());
    }

    info!(
        "[OSS_LISTENER] New release: {} {}",
        lib.name, release.tag_name
    );

    let significance = classify_significance(lib.tracked_version.as_deref(), &release.tag_name);
    let update_id = Uuid::new_v4();

    sqlx::query(
        "INSERT OR IGNORE INTO oss_library_updates
         (id, library_id, version, release_url, release_notes, significance, published_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(update_id)
    .bind(lib.id)
    .bind(&release.tag_name)
    .bind(&release.html_url)
    .bind(&release.body)
    .bind(&significance)
    .bind(&release.published_at)
    .execute(pool)
    .await?;

    // Kick off recommendation workflow
    let pool_clone = pool.clone();
    let lib_name = lib.name.clone();
    let lib_notes = lib.notes.clone();
    let tag = release.tag_name.clone();
    let release_notes = release.body.clone();
    let release_url = release.html_url.clone();

    tokio::spawn(async move {
        if let Err(e) = generate_recommendation(
            &pool_clone,
            update_id,
            &lib_name,
            lib_notes.as_deref(),
            &tag,
            release_notes.as_deref(),
            &release_url,
            &significance,
        )
        .await
        {
            error!(
                "[OSS_LISTENER] Recommendation failed for '{}' {}: {}",
                lib_name, tag, e
            );
            let _ = sqlx::query(
                "UPDATE oss_library_updates SET recommendation_status = 'failed' WHERE id = ?",
            )
            .bind(update_id)
            .execute(&pool_clone)
            .await;
        }
    });

    Ok(())
}

// ── GitHub API ────────────────────────────────────────────────────────────────

async fn fetch_latest_release(owner: &str, repo: &str) -> anyhow::Result<GithubRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        owner, repo
    );
    let token = std::env::var("GITHUB_TOKEN").ok();

    let mut req = reqwest::Client::new()
        .get(&url)
        .header("User-Agent", "PCG-Sovereign-Stack/1.0")
        .header("Accept", "application/vnd.github+json");

    if let Some(t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().await?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("GitHub API {} for {}/{}: {}", status, owner, repo, body);
    }

    Ok(resp.json::<GithubRelease>().await?)
}

// ── Semver significance ───────────────────────────────────────────────────────

fn classify_significance(previous: Option<&str>, current: &str) -> String {
    let strip = |v: &str| v.trim_start_matches('v').to_string();
    let parse = |v: &str| -> (u64, u64, u64) {
        let parts: Vec<&str> = v.splitn(3, '.').collect();
        let n = |i: usize| parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
        (n(0), n(1), n(2))
    };

    let Some(prev) = previous else {
        return "minor".into();
    };
    let (p_maj, p_min, _) = parse(&strip(prev));
    let (c_maj, c_min, _) = parse(&strip(current));

    if c_maj > p_maj {
        "major".into()
    } else if c_min > p_min {
        "minor".into()
    } else {
        "patch".into()
    }
}

// ── Nora recommendation workflow ──────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
async fn generate_recommendation(
    pool: &SqlitePool,
    update_id: Uuid,
    lib_name: &str,
    lib_notes: Option<&str>,
    version: &str,
    release_notes: Option<&str>,
    release_url: &str,
    significance: &str,
) -> anyhow::Result<()> {
    // Mark generating + create agent_flow record
    let flow_id = Uuid::new_v4();

    sqlx::query(
        "UPDATE oss_library_updates
         SET recommendation_status = 'generating', agent_flow_id = ?
         WHERE id = ?",
    )
    .bind(flow_id)
    .bind(update_id)
    .execute(pool)
    .await?;

    // Create an agent_flow so this shows up as a tracked workflow
    sqlx::query(
        "INSERT OR IGNORE INTO agent_flows
         (id, task_id, flow_type, status, created_at, updated_at)
         VALUES (?, NULL, 'monitoring', 'executing', datetime('now','subsec'), datetime('now','subsec'))",
    )
    .bind(flow_id)
    .execute(pool)
    .await?;

    let prompt = build_recommendation_prompt(
        lib_name,
        lib_notes,
        version,
        release_notes,
        release_url,
        significance,
    );

    let recommendation = run_nora_or_fallback(pool, flow_id, &prompt, lib_name, version).await?;

    // Store result
    sqlx::query(
        "UPDATE oss_library_updates
         SET agent_recommendation = ?, recommendation_status = 'done'
         WHERE id = ?",
    )
    .bind(&recommendation)
    .bind(update_id)
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE agent_flows
         SET status = 'completed', updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(flow_id)
    .execute(pool)
    .await?;

    // Advance tracked_version so we don't re-process
    sqlx::query(
        "UPDATE oss_libraries
         SET tracked_version = ?, updated_at = datetime('now','subsec')
         WHERE id = (SELECT library_id FROM oss_library_updates WHERE id = ?)",
    )
    .bind(version)
    .bind(update_id)
    .execute(pool)
    .await?;

    info!(
        "[OSS_LISTENER] Recommendation complete for {} {}",
        lib_name, version
    );
    Ok(())
}

fn build_recommendation_prompt(
    lib_name: &str,
    lib_notes: Option<&str>,
    version: &str,
    release_notes: Option<&str>,
    release_url: &str,
    significance: &str,
) -> String {
    let context = lib_notes.map(|n| format!(" ({})", n)).unwrap_or_default();
    let notes = release_notes.unwrap_or("No release notes provided.");

    format!(
        "You are a senior Rust engineer reviewing an open-source library update for the PCG Sovereign Stack.\n\
         \n\
         Library: {lib_name}{context}\n\
         New Version: {version} ({significance} release)\n\
         Release URL: {release_url}\n\
         Release Notes:\n{notes}\n\
         \n\
         Based on this release:\n\
         1. Summarise the key changes that are relevant to a production Rust/axum/sqlx application.\n\
         2. Identify any breaking changes, deprecations, or security fixes.\n\
         3. Provide specific, actionable code change recommendations for upgrading in our stack.\n\
         4. Rate upgrade urgency: Critical / High / Medium / Low and explain why.\n\
         \n\
         Be concise and technical. Format as markdown with clear headings.",
        lib_name = lib_name,
        context = context,
        version = version,
        significance = significance,
        release_url = release_url,
        notes = notes,
    )
}

async fn run_nora_or_fallback(
    pool: &SqlitePool,
    flow_id: Uuid,
    prompt: &str,
    lib_name: &str,
    version: &str,
) -> anyhow::Result<String> {
    use crate::routes::nora::get_nora_instance;

    if let Ok(nora_arc) = get_nora_instance().await {
        let nora_guard = nora_arc.read().await;
        if let Some(nora) = nora_guard.as_ref() {
            let req = NoraRequest {
                request_id: Uuid::new_v4().to_string(),
                session_id: format!("oss-{}", flow_id),
                request_type: NoraRequestType::TextInteraction,
                content: prompt.to_string(),
                context: None,
                voice_enabled: false,
                priority: RequestPriority::Normal,
                timestamp: chrono::Utc::now(),
            };

            let timeout = tokio::time::Duration::from_secs(90);
            match tokio::time::timeout(timeout, nora.process_request(req)).await {
                Ok(Ok(resp)) => return Ok(resp.content),
                Ok(Err(e)) => warn!(
                    "[OSS_LISTENER] Nora error for {} {}: {}",
                    lib_name, version, e
                ),
                Err(_) => warn!("[OSS_LISTENER] Nora timeout for {} {}", lib_name, version),
            }
        }
    }

    // Fallback: direct Anthropic call
    run_direct_anthropic(pool, prompt).await
}

async fn run_direct_anthropic(pool: &SqlitePool, prompt: &str) -> anyhow::Result<String> {
    use services::services::workflow_llm::WorkflowLLMService;

    let messages = vec![WorkflowLLMService::user_message(prompt)];

    let (text, metadata) =
        WorkflowLLMService::completion(pool, messages, Some("claude-sonnet-4-6"), Some(2048), None)
            .await?;

    info!(
        "[OSS_LISTENER] Routed to {} ({})",
        metadata.model_used, metadata.provider
    );

    Ok(text)
}
