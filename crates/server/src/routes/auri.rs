//! Auri coding-agent execution routes.
//!
//! POST /auri/tasks/{id}/run  — trigger Auri to work on a task
//! GET  /auri/tasks/{id}/log  — stream execution log

use std::process::Stdio;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use db::models::integration_connection::IntegrationConnection;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::oauth_token_manager;
use sqlx::SqlitePool;
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{error::ApiError, helpers::billing::record_llm_vibe_usage, DeploymentImpl};

const AURI_VIBE_ESTIMATE: i64 = 500; // upfront estimate, reconciled after

fn claude_bin() -> String {
    std::env::var("CLAUDE_BIN")
        .unwrap_or_else(|_| "/home/pythia/.nvm/versions/node/v22.20.0/bin/claude".to_string())
}
fn gh_bin() -> String {
    std::env::var("GH_BIN").unwrap_or_else(|_| "/home/pythia/.local/bin/gh".to_string())
}
fn workspaces_dir() -> String {
    std::env::var("AURI_WORKSPACES_DIR")
        .unwrap_or_else(|_| "/home/pythia/auri-workspaces".to_string())
}

#[derive(Debug, Deserialize)]
pub struct RunPayload {
    pub github_repo: Option<String>,
    pub base_branch: Option<String>,
    pub prompt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RunResponse {
    pub task_id: String,
    pub status: String,
    pub message: String,
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/auri/tasks", get(list_tasks))
        .route("/auri/tasks/{id}/run", post(run_task))
        .route("/auri/tasks/{id}/log", get(get_log))
        .with_state(deployment.clone())
}

async fn list_tasks(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use sqlx::Row;
    let pool = &deployment.db().pool;
    let rows = sqlx::query(
        "SELECT id, title, status, github_repo, github_branch, auri_pr_url, created_at, updated_at \
         FROM tasks WHERE assigned_agent = 'Auri' ORDER BY updated_at DESC LIMIT 100"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let tasks: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<Option<String>, _>("id"),
                "title": r.get::<Option<String>, _>("title"),
                "status": r.get::<Option<String>, _>("status"),
                "github_repo": r.get::<Option<String>, _>("github_repo"),
                "github_branch": r.get::<Option<String>, _>("github_branch"),
                "auri_pr_url": r.get::<Option<String>, _>("auri_pr_url"),
                "created_at": r.get::<Option<String>, _>("created_at"),
                "updated_at": r.get::<Option<String>, _>("updated_at"),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({ "tasks": tasks })))
}

async fn run_task(
    Path(task_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<RunPayload>,
) -> Result<Json<RunResponse>, ApiError> {
    use sqlx::Row;
    let pool = deployment.db().pool.clone();

    // Load task (runtime query — new columns not in sqlx cache)
    let task_row = sqlx::query(
        "SELECT id, title, description, project_id, status, github_repo, github_branch, assigned_agent \
         FROM tasks WHERE id = ?"
    )
    .bind(&task_id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| ApiError::InternalError(e.to_string()))?
    .ok_or_else(|| ApiError::NotFound(format!("Task {} not found", task_id)))?;

    let task_title: String = task_row
        .get::<Option<String>, _>("title")
        .unwrap_or_default();
    let task_desc: String = task_row
        .get::<Option<String>, _>("description")
        .unwrap_or_default();
    let task_project_id: Option<String> = task_row.get("project_id");
    let task_github_repo: Option<String> = task_row.get("github_repo");
    let task_github_repo_display = task_github_repo.clone().unwrap_or_default();

    // Get github_repo — from payload or task
    let github_repo = payload.github_repo.or(task_github_repo).ok_or_else(|| {
        ApiError::BadRequest("github_repo is required (set on task or in payload)".into())
    })?;

    let base_branch = payload.base_branch.unwrap_or_else(|| "main".to_string());

    let prompt = payload.prompt.unwrap_or_else(|| {
        format!(
            "You are Auri, a senior developer working on the {} repository.\n\n\
             Task: {}\n\n\
             Description: {}\n\n\
             Instructions:\n\
             - Read relevant files to understand the codebase before making changes\n\
             - Implement the requested changes carefully\n\
             - Follow existing code patterns and conventions\n\
             - After all changes are made, run any relevant tests if a test command is available\n\
             - Do not commit or push — the system will handle git operations after you finish\n\
             - When done, summarize what you changed and why",
            github_repo, task_title, task_desc
        )
    });

    // Get org_id from project (optional — we fall back to any active GitHub connection)
    let proj_id_str = task_project_id.unwrap_or_default();
    let org_id_hex_opt: Option<String> = if !proj_id_str.is_empty() {
        sqlx::query("SELECT organization_id FROM projects WHERE id = ?")
            .bind(&proj_id_str)
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.get::<Option<String>, _>("organization_id"))
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    // Resolve GitHub token: prefer org-scoped connection, fall back to any active connection, then env var
    let github_token: String = {
        let mut token: Option<String> = None;

        // Try org-scoped connection first
        if let Some(ref org_hex) = org_id_hex_opt {
            if let Ok(org_uuid) = Uuid::parse_str(org_hex) {
                if let Ok(conns) =
                    IntegrationConnection::find_by_org_and_provider(&pool, org_uuid, "github").await
                {
                    if let Some(conn) = conns.into_iter().next() {
                        token = oauth_token_manager::decrypt_access(&conn)
                            .ok()
                            .filter(|t| !t.is_empty());
                    }
                }
            }
        }

        // Fall back to any active GitHub integration
        if token.as_deref().unwrap_or("").is_empty() {
            if let Ok(Some(conn)) = sqlx::query_as::<_, IntegrationConnection>(
                "SELECT * FROM integration_connections WHERE provider = 'github' AND status = 'active' LIMIT 1"
            )
            .fetch_optional(&pool)
            .await
            {
                token = oauth_token_manager::decrypt_access(&conn).ok().filter(|t| !t.is_empty());
            }
        }

        // Final fallback to env vars
        token
            .filter(|t| !t.is_empty())
            .or_else(|| std::env::var("GITHUB_PAT").ok().filter(|t| !t.is_empty()))
            .or_else(|| std::env::var("GH_TOKEN").ok().filter(|t| !t.is_empty()))
            .ok_or_else(|| {
                ApiError::BadRequest(
                    "No GitHub token available. Connect GitHub in Settings → Integrations.".into(),
                )
            })?
    };

    let org_id_hex = org_id_hex_opt.unwrap_or_default();

    // Mark task inprogress
    sqlx::query("UPDATE tasks SET status = 'inprogress', updated_at = datetime('now','subsec') WHERE id = ?")
        .bind(&task_id)
        .execute(&pool)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Save github_repo on task if not already set
    sqlx::query("UPDATE tasks SET github_repo = ?, updated_at = datetime('now','subsec') WHERE id = ? AND github_repo IS NULL")
        .bind(&github_repo)
        .bind(&task_id)
        .execute(&pool)
        .await.ok();

    // Record upfront VIBE estimate
    let vibe_tx_id = Uuid::new_v4().to_string();
    let vibe_desc = format!("Auri task: {}", task_title);
    let _ = sqlx::query(
        "INSERT INTO vibe_transactions \
         (id, source_type, source_id, amount_vibe, model, provider, description, task_id, created_at, updated_at) \
         VALUES (?, 'project', ?, ?, 'auri-claude-sonnet-4', 'anthropic', ?, ?, datetime('now','subsec'), datetime('now','subsec'))"
    )
    .bind(&vibe_tx_id)
    .bind(&org_id_hex)
    .bind(AURI_VIBE_ESTIMATE)
    .bind(&vibe_desc)
    .bind(&task_id)
    .execute(&pool)
    .await;

    // Spawn execution in background — return immediately
    let pool2 = pool.clone();
    let task_id2 = task_id.clone();
    tokio::spawn(async move {
        let result = execute_auri_task(
            &pool2,
            &task_id2,
            &github_repo,
            &base_branch,
            &github_token,
            &prompt,
        )
        .await;

        match result {
            Ok(pr_url) => {
                info!("[AURI] Task {} completed. PR: {}", task_id2, pr_url);
                let _ = sqlx::query("UPDATE tasks SET status='done', auri_pr_url=?, updated_at=datetime('now','subsec') WHERE id=?")
                    .bind(&pr_url)
                    .bind(&task_id2)
                    .execute(&pool2)
                    .await;
            }
            Err(e) => {
                error!("[AURI] Task {} failed: {}", task_id2, e);
                let _ = sqlx::query("UPDATE tasks SET status='cancelled', auri_log=?, updated_at=datetime('now','subsec') WHERE id=?")
                    .bind(format!("ERROR: {}", e))
                    .bind(&task_id2)
                    .execute(&pool2)
                    .await;
            }
        }
    });

    Ok(Json(RunResponse {
        task_id: task_id.clone(),
        status: "inprogress".into(),
        message: format!(
            "Auri is working on task '{}' in repo {}. Check the task for progress.",
            task_title, task_github_repo_display
        ),
    }))
}

async fn execute_auri_task(
    pool: &SqlitePool,
    task_id: &str,
    github_repo: &str,
    base_branch: &str,
    github_token: &str,
    prompt: &str,
) -> Result<String, String> {
    std::fs::create_dir_all(WORKSPACES_DIR)
        .map_err(|e| format!("Cannot create workspaces dir: {}", e))?;

    let workspace = format!("{}/{}", WORKSPACES_DIR, &task_id[..8]);
    let branch_name = format!("auri/{}", &task_id[..8]);

    // Build authenticated remote URL
    let remote_url = format!(
        "https://oauth2:{}@github.com/{}.git",
        github_token, github_repo
    );

    // Clone or update the repo
    if std::path::Path::new(&workspace).exists() {
        info!("[AURI] Updating existing workspace {}", workspace);
        let fetch = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&workspace)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .await
            .map_err(|e| format!("git fetch failed: {}", e))?;
        if !fetch.status.success() {
            warn!(
                "[AURI] git fetch: {}",
                String::from_utf8_lossy(&fetch.stderr)
            );
        }
    } else {
        info!("[AURI] Cloning {} to {}", github_repo, workspace);
        let clone = Command::new("git")
            .args(["clone", "--depth=50", &remote_url, &workspace])
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .await
            .map_err(|e| format!("git clone failed: {}", e))?;
        if !clone.status.success() {
            return Err(format!(
                "git clone failed: {}",
                String::from_utf8_lossy(&clone.stderr)
            ));
        }
    }

    // Configure git identity
    for (k, v) in [
        ("user.email", "auri@powerclubglobal.com"),
        ("user.name", "Auri (PCG)"),
    ] {
        Command::new("git")
            .args(["config", k, v])
            .current_dir(&workspace)
            .output()
            .await
            .ok();
    }
    // Set authenticated remote
    Command::new("git")
        .args(["remote", "set-url", "origin", &remote_url])
        .current_dir(&workspace)
        .output()
        .await
        .map_err(|e| format!("set remote url: {}", e))?;

    // Checkout base branch and create Auri's branch
    Command::new("git")
        .args(["checkout", base_branch])
        .current_dir(&workspace)
        .output()
        .await
        .ok();
    Command::new("git")
        .args(["pull", "origin", base_branch])
        .current_dir(&workspace)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .ok();
    Command::new("git")
        .args(["checkout", "-B", &branch_name])
        .current_dir(&workspace)
        .output()
        .await
        .map_err(|e| format!("branch create: {}", e))?;

    // Save branch to task
    sqlx::query("UPDATE tasks SET github_branch=?, updated_at=datetime('now','subsec') WHERE id=?")
        .bind(&branch_name)
        .bind(task_id)
        .execute(pool)
        .await
        .ok();

    info!("[AURI] Running Claude Code in {}", workspace);

    // Run Claude Code
    let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap_or_default();
    let output = Command::new(CLAUDE_BIN)
        .args(["--print", "--output-format", "json", "-p", prompt])
        .current_dir(&workspace)
        .env("ANTHROPIC_API_KEY", &api_key)
        .env("GH_TOKEN", github_token)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env(
            "HOME",
            std::env::var("HOME").unwrap_or_else(|_| "/home/pythia".into()),
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("claude --print failed to start: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Extract usage from JSON output if available
    let (result_text, input_tokens, output_tokens) =
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let text = parsed["result"].as_str().unwrap_or(&stdout).to_string();
            let inp = parsed["usage"]["input_tokens"].as_i64().unwrap_or(0);
            let out = parsed["usage"]["output_tokens"].as_i64().unwrap_or(0);
            (text, inp, out)
        } else {
            (stdout.clone(), 0i64, 0i64)
        };

    let log_content = format!(
        "=== Auri Output ===\n{}\n\n=== Stderr ===\n{}",
        result_text, stderr
    );

    // Save log to task
    let truncated_log: String = log_content.chars().take(50000).collect();
    sqlx::query("UPDATE tasks SET auri_log=?, updated_at=datetime('now','subsec') WHERE id=?")
        .bind(&truncated_log)
        .bind(task_id)
        .execute(pool)
        .await
        .ok();

    // Record actual VIBE usage if we have token counts
    if input_tokens > 0 || output_tokens > 0 {
        use sqlx::Row;
        let project_row = sqlx::query("SELECT project_id FROM tasks WHERE id=?")
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
        if let Some(row) = project_row {
            let proj_id: Option<String> = row.get("project_id");
            if let Some(pid) = proj_id {
                if let Ok(proj_uuid) = Uuid::parse_str(&pid) {
                    let _ = record_llm_vibe_usage(
                        pool,
                        proj_uuid,
                        "claude-sonnet-4-5",
                        input_tokens,
                        output_tokens,
                        Some(Uuid::parse_str(task_id).unwrap_or_default()),
                        None,
                        None,
                        "auri_task",
                    )
                    .await;
                }
            }
        }
    }

    if !output.status.success() && result_text.trim().is_empty() {
        return Err(format!("Claude Code exited with error:\n{}", stderr));
    }

    // Check for changes
    let diff = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&workspace)
        .output()
        .await
        .map_err(|e| format!("git status: {}", e))?;

    let has_changes = !String::from_utf8_lossy(&diff.stdout).trim().is_empty();

    if !has_changes {
        return Ok("(no file changes — task completed without code modifications)".into());
    }

    // Commit all changes
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(&workspace)
        .output()
        .await
        .map_err(|e| format!("git add: {}", e))?;

    let task_title_for_commit = {
        use sqlx::Row;
        sqlx::query("SELECT title FROM tasks WHERE id=?")
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.get::<Option<String>, _>("title"))
            .unwrap_or_else(|| "auri task".into())
    };
    let commit_msg = format!(
        "feat(auri): {}\n\nTask ID: {}\nAssigned by: Nora",
        task_title_for_commit, task_id
    );

    let commit = Command::new("git")
        .args(["commit", "-m", &commit_msg])
        .current_dir(&workspace)
        .output()
        .await
        .map_err(|e| format!("git commit: {}", e))?;

    if !commit.status.success() {
        return Err(format!(
            "git commit failed: {}",
            String::from_utf8_lossy(&commit.stderr)
        ));
    }

    // Push branch
    let push = Command::new("git")
        .args(["push", "-u", "origin", &branch_name, "--force"])
        .current_dir(&workspace)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .await
        .map_err(|e| format!("git push: {}", e))?;

    if !push.status.success() {
        return Err(format!(
            "git push failed: {}",
            String::from_utf8_lossy(&push.stderr)
        ));
    }

    // Create PR via gh CLI
    let pr_title = {
        use sqlx::Row;
        let t = sqlx::query("SELECT title FROM tasks WHERE id=?")
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .and_then(|r| r.get::<Option<String>, _>("title"))
            .unwrap_or_else(|| "coding task".into());
        format!("[Auri] {}", t)
    };

    let pr_body = format!(
        "## Summary\n\n{}\n\n---\n_Automated by Auri · Task ID: {}_",
        result_text.chars().take(2000).collect::<String>(),
        task_id
    );

    let pr = Command::new(GH_BIN)
        .args([
            "pr",
            "create",
            "--repo",
            github_repo,
            "--title",
            &pr_title,
            "--body",
            &pr_body,
            "--base",
            base_branch,
            "--head",
            &branch_name,
        ])
        .current_dir(&workspace)
        .env("GH_TOKEN", github_token)
        .output()
        .await
        .map_err(|e| format!("gh pr create: {}", e))?;

    let pr_url = if pr.status.success() {
        String::from_utf8_lossy(&pr.stdout).trim().to_string()
    } else {
        let err = String::from_utf8_lossy(&pr.stderr).to_string();
        // PR may already exist — extract URL from error
        if err.contains("https://github.com") {
            err.lines()
                .find(|l| l.contains("https://github.com"))
                .unwrap_or("PR creation failed")
                .trim()
                .to_string()
        } else {
            format!("Branch pushed but PR creation failed: {}", err)
        }
    };

    Ok(pr_url)
}

async fn get_log(
    Path(task_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use sqlx::Row;
    let pool = &deployment.db().pool;
    let row =
        sqlx::query("SELECT status, auri_log, auri_pr_url, github_branch FROM tasks WHERE id=?")
            .bind(&task_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
            .ok_or_else(|| ApiError::NotFound(task_id.clone()))?;

    Ok(Json(serde_json::json!({
        "task_id": task_id,
        "status": row.get::<Option<String>, _>("status"),
        "log": row.get::<Option<String>, _>("auri_log"),
        "pr_url": row.get::<Option<String>, _>("auri_pr_url"),
        "branch": row.get::<Option<String>, _>("github_branch"),
    })))
}
