//! QA Review Service — watcher-based agent review system.
//!
//! Instead of creating separate `[QA] Review PR #X` tasks, QA agents are registered
//! as watchers on the original task. When a dev execution completes and the task moves
//! to `inreview`, watching agents are triggered to execute against the **same task**.
//!
//! No extra tasks are created. All review activity lives in the collaborators trail.

use std::sync::Arc;

use db::models::{
    agent::Agent,
    agent_execution_config::AgentExecutionConfig,
    execution_artifact::ExecutionArtifact,
    execution_process::{ExecutionContext, ExecutionProcessRunReason},
    merge::Merge,
    project::Project,
    task::{
        Task, TaskCollaborator, TaskStatus, ACTOR_TYPE_AGENT_WATCHER, WATCHER_ACTION_QA_FAIL,
        WATCHER_ACTION_QA_NEEDS_CHANGES, WATCHER_ACTION_QA_PASS, WATCHER_ACTION_TRIGGERED,
        WATCHER_ACTION_WATCHING,
    },
    task_attempt::{CreateTaskAttempt, TaskAttempt},
};
use executors::{executors::BaseCodingAgent, profile::ExecutorProfileId};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{config::Config, container::ContainerService, git::GitService};

/// Maximum QA review iterations before escalating to human.
pub const MAX_QA_ITERATIONS: i64 = 2;

/// Info about a PR that was auto-created, passed to QA review and audit comment functions.
pub struct PrCreatedInfo {
    pub number: i64,
    pub url: String,
    pub repo_owner: String,
    pub repo_name: String,
}

/// Trigger all pending agent watchers on a task.
///
/// For each watcher with `last_action == "watching"`:
/// 1. Mark collaborator as "triggered"
/// 2. Look up agent + execution config → resolve executor profile
/// 3. Build QA review prompt (PR URL, completion criteria, JSON verdict schema)
/// 4. Create TaskAttempt on the **same task** (executor = QA agent's executor)
/// 5. Start execution with `run_reason = AgentReview`
pub async fn trigger_agent_watchers<C: ContainerService + Sync>(
    pool: &SqlitePool,
    container: &C,
    ctx: &ExecutionContext,
    pr: &PrCreatedInfo,
) {
    let pending = match Task::find_pending_agent_watchers(pool, &ctx.task.id).await {
        Ok(watchers) => watchers,
        Err(e) => {
            tracing::warn!(
                "Failed to find pending agent watchers for task {}: {e}",
                ctx.task.id
            );
            return;
        }
    };

    if pending.is_empty() {
        tracing::debug!("No pending agent watchers for task {}", ctx.task.id);
        return;
    }

    tracing::info!(
        "Triggering {} agent watcher(s) for task {} (PR #{})",
        pending.len(),
        ctx.task.id,
        pr.number
    );

    for watcher in &pending {
        if let Err(e) = trigger_single_watcher(pool, container, ctx, pr, watcher).await {
            tracing::error!(
                "Failed to trigger watcher {} for task {}: {e}",
                watcher.actor_id,
                ctx.task.id
            );
        }
    }
}

/// Spawn QA reviews for pending watchers without requiring an ExecutionContext.
///
/// This is the manual-trigger counterpart to [`trigger_agent_watchers`], used when
/// a task is moved to InReview via the UI rather than by a dev agent completing
/// execution. It looks up the task and base branch independently.
pub async fn spawn_watcher_reviews<C: ContainerService + Sync>(
    pool: &SqlitePool,
    container: &C,
    task_id: &str,
    pr: &PrCreatedInfo,
) {
    let task = match Task::find_by_id(pool, task_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            tracing::warn!("spawn_watcher_reviews: task {task_id} not found");
            return;
        }
        Err(e) => {
            tracing::warn!("spawn_watcher_reviews: failed to load task {task_id}: {e}");
            return;
        }
    };

    let pending = match Task::find_pending_agent_watchers(pool, task_id).await {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("spawn_watcher_reviews: failed to find watchers for {task_id}: {e}");
            return;
        }
    };

    if pending.is_empty() {
        tracing::debug!("spawn_watcher_reviews: no pending watchers for task {task_id}");
        return;
    }

    // Resolve base branch from the most recent task attempt (if any)
    let task_uuid = match Uuid::parse_str(task_id) {
        Ok(u) => u,
        Err(_) => return,
    };
    let base_branch = TaskAttempt::fetch_all(pool, Some(task_uuid))
        .await
        .ok()
        .and_then(|attempts| attempts.last().map(|a| a.base_branch.clone()))
        .unwrap_or_default();

    tracing::info!(
        "spawn_watcher_reviews: triggering {} watcher(s) for task {task_id} with PR #{}",
        pending.len(),
        pr.number
    );

    for watcher in &pending {
        if let Err(e) = spawn_single_watcher_review(
            pool,
            container,
            task_id,
            &task.title,
            task.completion_criteria.as_deref().unwrap_or_default(),
            &base_branch,
            pr,
            watcher,
        )
        .await
        {
            tracing::error!(
                "spawn_watcher_reviews: failed to trigger watcher {} for task {task_id}: {e}",
                watcher.actor_id
            );
        }
    }
}

/// Spawn a QA review for a single watcher without ExecutionContext.
async fn spawn_single_watcher_review<C: ContainerService + Sync>(
    pool: &SqlitePool,
    container: &C,
    task_id: &str,
    task_title: &str,
    completion_criteria: &str,
    base_branch: &str,
    pr: &PrCreatedInfo,
    watcher: &TaskCollaborator,
) -> Result<(), String> {
    let agent_id = &watcher.actor_id;

    // Mark as triggered
    Task::update_collaborator(
        pool,
        task_id,
        agent_id,
        ACTOR_TYPE_AGENT_WATCHER,
        WATCHER_ACTION_TRIGGERED,
    )
    .await
    .map_err(|e| format!("Failed to mark watcher as triggered: {e}"))?;

    // Look up agent and config
    let agent = Agent::find_by_id(pool, agent_id)
        .await
        .map_err(|e| format!("Failed to find agent {agent_id}: {e}"))?
        .ok_or_else(|| format!("Agent {agent_id} not found"))?;

    let config = AgentExecutionConfig::find_by_agent_id(pool, agent_id)
        .await
        .map_err(|e| format!("Failed to find config for agent {agent_id}: {e}"))?;

    // Resolve executor profile (default to CLAUDE_CODE)
    let executor_profile_id = if let Some(ref cfg) = config {
        if let Some(ref profile_str) = cfg.execution_profile_id {
            let parts: Vec<&str> = profile_str.splitn(2, ':').collect();
            let executor =
                std::str::FromStr::from_str(parts[0]).unwrap_or(BaseCodingAgent::ClaudeCode);
            let variant = parts.get(1).map(|s| s.to_string());
            ExecutorProfileId { executor, variant }
        } else {
            ExecutorProfileId::new(BaseCodingAgent::ClaudeCode)
        }
    } else {
        ExecutorProfileId::new(BaseCodingAgent::ClaudeCode)
    };

    let review_description = build_review_description(pr, task_title, task_id, completion_criteria);

    let task_uuid = Uuid::parse_str(task_id).map_err(|e| format!("Invalid task UUID: {e}"))?;

    let attempt = TaskAttempt::create(
        pool,
        &CreateTaskAttempt {
            executor: executor_profile_id.executor.clone(),
            base_branch: base_branch.to_string(),
        },
        task_uuid,
    )
    .await
    .map_err(|e| format!("Failed to create QA task attempt: {e}"))?;

    let process = container
        .start_attempt_with_reason(
            &attempt,
            executor_profile_id,
            Some(ExecutionProcessRunReason::AgentReview),
            Some(review_description.clone()),
        )
        .await
        .map_err(|e| format!("Failed to start QA execution: {e}"))?;

    // Store review instructions artifact
    if let Err(e) = ExecutionArtifact::create(
        pool,
        db::models::execution_artifact::CreateExecutionArtifact {
            execution_process_id: Some(process.id),
            artifact_type: db::models::execution_artifact::ArtifactType::ResearchReport,
            title: format!(
                "QA Review Instructions — Task {} PR #{}",
                task_id, pr.number
            ),
            content: Some(review_description),
            file_path: None,
            metadata: Some(serde_json::json!({
                "task_id": task_id,
                "pr_number": pr.number,
                "pr_url": pr.url,
                "agent_watcher_id": agent_id,
                "type": "qa_review_instructions",
            })),
            task_id: None,
            task_attempt_id: None,
        },
    )
    .await
    {
        tracing::warn!("Failed to store QA review instructions artifact: {e}");
    }

    tracing::info!(
        "Triggered agent watcher '{}' ({}) on task {} — attempt {}",
        agent.short_name,
        agent_id,
        task_id,
        attempt.id
    );

    Ok(())
}

/// Trigger a single agent watcher: mark as triggered, create attempt, start execution.
async fn trigger_single_watcher<C: ContainerService + Sync>(
    pool: &SqlitePool,
    container: &C,
    ctx: &ExecutionContext,
    pr: &PrCreatedInfo,
    watcher: &TaskCollaborator,
) -> Result<(), String> {
    let agent_id = &watcher.actor_id;

    // Mark as triggered
    Task::update_collaborator(
        pool,
        &ctx.task.id,
        agent_id,
        ACTOR_TYPE_AGENT_WATCHER,
        WATCHER_ACTION_TRIGGERED,
    )
    .await
    .map_err(|e| format!("Failed to mark watcher as triggered: {e}"))?;

    // Look up agent and config
    let agent = Agent::find_by_id(pool, agent_id)
        .await
        .map_err(|e| format!("Failed to find agent {agent_id}: {e}"))?
        .ok_or_else(|| format!("Agent {agent_id} not found"))?;

    let config = AgentExecutionConfig::find_by_agent_id(pool, agent_id)
        .await
        .map_err(|e| format!("Failed to find config for agent {agent_id}: {e}"))?;

    // Resolve executor profile (default to CLAUDE_CODE)
    let executor_profile_id = if let Some(ref cfg) = config {
        if let Some(ref profile_str) = cfg.execution_profile_id {
            let parts: Vec<&str> = profile_str.splitn(2, ':').collect();
            let executor =
                std::str::FromStr::from_str(parts[0]).unwrap_or(BaseCodingAgent::ClaudeCode);
            let variant = parts.get(1).map(|s| s.to_string());
            ExecutorProfileId { executor, variant }
        } else {
            ExecutorProfileId::new(BaseCodingAgent::ClaudeCode)
        }
    } else {
        ExecutorProfileId::new(BaseCodingAgent::ClaudeCode)
    };

    // Build the review description (structured QA prompt with PR URL, criteria, verdict schema)
    let completion_criteria = ctx.task.completion_criteria.clone().unwrap_or_default();
    let review_description =
        build_review_description(pr, &ctx.task.title, &ctx.task.id, &completion_criteria);

    // Resolve task UUID for attempt creation
    let task_uuid = Uuid::parse_str(&ctx.task.id).map_err(|e| format!("Invalid task UUID: {e}"))?;

    // ctx.task_attempt is always the dev attempt (not a QA attempt) — we reuse
    // its base_branch so the QA review targets the same integration point.
    let base_branch = ctx.task_attempt.base_branch.clone();

    // Create TaskAttempt on the SAME task (no separate QA task)
    let attempt = TaskAttempt::create(
        pool,
        &CreateTaskAttempt {
            executor: executor_profile_id.executor.clone(),
            base_branch,
        },
        task_uuid,
    )
    .await
    .map_err(|e| format!("Failed to create QA task attempt: {e}"))?;

    // Start execution with AgentReview run reason — this is critical for routing
    // completions to finalize_review() instead of finalize_task() in spawn_exit_monitor
    let process = container
        .start_attempt_with_reason(
            &attempt,
            executor_profile_id,
            Some(ExecutionProcessRunReason::AgentReview),
            Some(review_description.clone()),
        )
        .await
        .map_err(|e| format!("Failed to start QA execution: {e}"))?;

    // Store review instructions as an execution artifact linked to the process.
    // The agent can retrieve these via its execution_process_id to get the
    // structured review prompt with PR URL, criteria, and verdict schema.
    if let Err(e) = db::models::execution_artifact::ExecutionArtifact::create(
        pool,
        db::models::execution_artifact::CreateExecutionArtifact {
            execution_process_id: Some(process.id),
            artifact_type: db::models::execution_artifact::ArtifactType::ResearchReport,
            title: format!(
                "QA Review Instructions — Task {} PR #{}",
                ctx.task.id, pr.number
            ),
            content: Some(review_description),
            file_path: None,
            metadata: Some(serde_json::json!({
                "task_id": ctx.task.id,
                "pr_number": pr.number,
                "pr_url": pr.url,
                "agent_watcher_id": agent_id,
                "type": "qa_review_instructions",
            })),
            task_id: None,
            task_attempt_id: None,
        },
    )
    .await
    {
        tracing::warn!("Failed to store QA review instructions artifact: {e}");
    }

    tracing::info!(
        "Triggered agent watcher '{}' ({}) on task {} — attempt {}",
        agent.short_name,
        agent_id,
        ctx.task.id,
        attempt.id
    );

    Ok(())
}

/// Handle QA agent review completion. Called when an AgentReview execution finishes.
///
/// 1. Parse verdict from execution artifacts
/// 2. Post PR comment (emoji verdict, criteria checks, issues table)
/// 3. Update collaborator action: qa_pass / qa_needs_changes / qa_fail
/// 4. Handle verdict:
///    - "pass" → task stays InReview, human decides
///    - "needs_changes" + iteration < MAX → reset watcher, task → InProgress
///    - "fail" or max iterations → leave for human
pub async fn finalize_review(
    pool: &SqlitePool,
    config: &Arc<RwLock<Config>>,
    git: &GitService,
    ctx: &ExecutionContext,
) {
    use super::github_service::GitHubService;

    // Find the PR number from the Merge record for this task
    let pr_number = find_pr_number_for_task(pool, ctx).await;
    let pr_number = match pr_number {
        Some(n) => n,
        None => {
            tracing::warn!(
                "QA review finalize: no PR found for task {} — skipping",
                ctx.task.id
            );
            return;
        }
    };

    // Parse verdict from execution artifacts
    let verdict = extract_verdict_from_artifacts(pool, ctx).await;
    let verdict = match verdict {
        Some(v) => v,
        None => {
            tracing::warn!(
                "QA review finalize: no verdict JSON found in artifacts for task {}",
                ctx.task.id
            );
            return;
        }
    };

    let verdict_str = verdict["verdict"].as_str().unwrap_or("unknown");
    let iteration = count_agent_review_iterations(pool, ctx).await;

    // Build and post PR comment
    let comment = build_review_comment(&verdict, iteration, &ctx.task.id);

    // Post comment on PR
    let github_config = config.read().await.github.clone();
    if let Some(github_token) = github_config.token() {
        if let Ok(github_service) = GitHubService::new(&github_token) {
            if let Ok(Some(project)) = Project::find_by_id(pool, &ctx.task.project_id).await {
                if let Ok(repo_info) = git.get_github_repo_info(&project.git_repo_path) {
                    if let Err(e) = github_service
                        .add_pr_comment(&repo_info, pr_number, &comment)
                        .await
                    {
                        tracing::warn!("QA review: failed to post PR comment: {e}");
                    } else {
                        tracing::info!(
                            "QA review: posted review comment on PR #{} (verdict: {})",
                            pr_number,
                            verdict_str
                        );
                    }
                }
            }
        } else {
            tracing::debug!("QA review: failed to create GitHub service");
        }
    } else {
        tracing::debug!("QA review: no GitHub token configured — skipping PR comment");
    }

    // Determine which watcher agent posted this review (from the executor on the attempt)
    // The agent_id is the watcher's ID — find it from collaborators
    let watcher_agent_id = find_watcher_agent_for_attempt(pool, ctx).await;

    // Update collaborator action based on verdict
    let watcher_action = match verdict_str {
        "pass" => WATCHER_ACTION_QA_PASS,
        "needs_changes" => WATCHER_ACTION_QA_NEEDS_CHANGES,
        "fail" => WATCHER_ACTION_QA_FAIL,
        _ => WATCHER_ACTION_QA_FAIL,
    };

    if let Some(ref agent_id) = watcher_agent_id {
        if let Err(e) = Task::update_collaborator(
            pool,
            &ctx.task.id,
            agent_id,
            ACTOR_TYPE_AGENT_WATCHER,
            watcher_action,
        )
        .await
        {
            tracing::warn!("Failed to update watcher collaborator action: {e}");
        }
    }

    // Handle verdict outcome
    match verdict_str {
        "pass" => {
            tracing::info!(
                "QA passed for task {} — awaiting human approval",
                ctx.task.id
            );
        }
        "needs_changes" if iteration < MAX_QA_ITERATIONS => {
            // Reset watcher to "watching" so it re-triggers on next finalize_task
            if let Some(ref agent_id) = watcher_agent_id {
                let _ = Task::update_collaborator(
                    pool,
                    &ctx.task.id,
                    agent_id,
                    ACTOR_TYPE_AGENT_WATCHER,
                    WATCHER_ACTION_WATCHING,
                )
                .await;
            }

            // Send task back to InProgress for dev agent iteration
            if let Err(e) = Task::update_status(pool, &ctx.task.id, TaskStatus::InProgress).await {
                tracing::error!("QA review: failed to set task back to InProgress: {e}");
            }
            tracing::info!(
                "QA needs changes for task {} — iteration {}/{} (sending back to dev)",
                ctx.task.id,
                iteration,
                MAX_QA_ITERATIONS
            );
        }
        _ => {
            tracing::info!(
                "QA verdict '{}' for task {} (iteration {}/{}) — leaving for human review",
                verdict_str,
                ctx.task.id,
                iteration,
                MAX_QA_ITERATIONS
            );
        }
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

/// Build the QA review description/prompt for the watcher agent.
fn build_review_description(
    pr: &PrCreatedInfo,
    task_title: &str,
    task_id: &str,
    completion_criteria: &str,
) -> String {
    format!(
        "## QA Review for PR #{}\n\n\
         **PR URL**: {}\n\
         **Original Task**: {} ({})\n\n\
         ### Completion Criteria to Verify\n{}\n\n\
         ### Instructions\n\
         Review the PR diff and verify all completion criteria are met.\n\
         Output a JSON verdict with this schema:\n\
         ```json\n{{\n  \
         \"verdict\": \"pass | needs_changes | fail\",\n  \
         \"summary\": \"Brief assessment\",\n  \
         \"criteria_checks\": [{{ \"criterion\": \"...\", \"met\": true, \"notes\": \"...\" }}],\n  \
         \"issues\": [{{ \"file\": \"...\", \"line\": 0, \"severity\": \"error|warning\", \"description\": \"...\" }}],\n  \
         \"iteration\": 1\n}}\n```",
        pr.number,
        pr.url,
        task_title,
        task_id,
        if completion_criteria.is_empty() {
            "No specific criteria defined."
        } else {
            completion_criteria
        }
    )
}

/// Build a structured PR comment from a QA verdict.
fn build_review_comment(verdict: &serde_json::Value, iteration: i64, task_id: &str) -> String {
    let verdict_str = verdict["verdict"].as_str().unwrap_or("unknown");
    let summary = verdict["summary"]
        .as_str()
        .unwrap_or("No summary provided.");

    let verdict_emoji = match verdict_str {
        "pass" => "Pass",
        "needs_changes" => "Needs Changes",
        "fail" => "Fail",
        _ => "Unknown",
    };
    let verdict_icon = match verdict_str {
        "pass" => "\u{2705}",
        "needs_changes" => "\u{26a0}\u{fe0f}",
        "fail" => "\u{274c}",
        _ => "\u{2753}",
    };

    let mut comment = format!(
        "## QA Review — Iteration {}\n\n**Verdict**: {} {}\n\n",
        iteration, verdict_icon, verdict_emoji
    );

    // Criteria checks
    if let Some(checks) = verdict["criteria_checks"].as_array() {
        comment.push_str("### Completion Criteria\n");
        for check in checks {
            let met = check["met"].as_bool().unwrap_or(false);
            let criterion = check["criterion"].as_str().unwrap_or("?");
            let notes = check["notes"].as_str().unwrap_or("");
            let checkbox = if met { "[x]" } else { "[ ]" };
            comment.push_str(&format!("- {} {} — {}\n", checkbox, criterion, notes));
        }
        comment.push('\n');
    }

    // Issues table
    if let Some(issues) = verdict["issues"].as_array() {
        if !issues.is_empty() {
            comment.push_str(
                "### Issues\n| File | Line | Severity | Description |\n|---|---|---|---|\n",
            );
            for issue in issues {
                comment.push_str(&format!(
                    "| `{}` | {} | {} | {} |\n",
                    issue["file"].as_str().unwrap_or("?"),
                    issue["line"].as_i64().unwrap_or(0),
                    issue["severity"].as_str().unwrap_or("warning"),
                    issue["description"].as_str().unwrap_or("?"),
                ));
            }
            comment.push('\n');
        }
    }

    comment.push_str(&format!("### Summary\n{}\n\n", summary));
    comment.push_str(&format!(
        "---\n*ORCHA QA Agent • Task {} • Iteration {}/{}*",
        task_id, iteration, MAX_QA_ITERATIONS
    ));

    comment
}

/// Find the PR number associated with a task by looking up Merge records.
async fn find_pr_number_for_task(pool: &SqlitePool, ctx: &ExecutionContext) -> Option<i64> {
    // First try: look up from the current task's attempts
    let task_uuid = Uuid::parse_str(&ctx.task.id).ok()?;
    let attempts = TaskAttempt::fetch_all(pool, Some(task_uuid)).await.ok()?;

    for attempt in &attempts {
        if let Ok(Some(merge)) = Merge::find_latest_by_task_attempt_id(pool, attempt.id).await {
            if let db::models::merge::Merge::Pr(pr_merge) = merge {
                return Some(pr_merge.pr_info.number);
            }
        }
    }

    // Fallback: check custom_properties (legacy QA tasks stored pr_number there)
    ctx.task
        .custom_properties
        .as_ref()
        .and_then(|v| v["pr_number"].as_i64())
}

/// Extract the QA verdict JSON from execution artifacts.
fn parse_verdict_from_content(content: &str) -> Option<serde_json::Value> {
    // Try direct JSON parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
        if v.get("verdict").is_some() {
            return Some(v);
        }
    }
    // Try markdown code block extraction
    if let Some(start) = content.find("```json") {
        if let Some(end) = content[start + 7..].find("```") {
            let json_str = content[start + 7..start + 7 + end].trim();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                if v.get("verdict").is_some() {
                    return Some(v);
                }
            }
        }
    }
    None
}

async fn extract_verdict_from_artifacts(
    pool: &SqlitePool,
    ctx: &ExecutionContext,
) -> Option<serde_json::Value> {
    let artifacts = ExecutionArtifact::find_by_execution_process(pool, ctx.execution_process.id)
        .await
        .unwrap_or_default();

    artifacts
        .iter()
        .filter_map(|a| a.content.as_ref())
        .find_map(|content| parse_verdict_from_content(content))
}

/// Count how many AgentReview execution processes exist for this task.
async fn count_agent_review_iterations(pool: &SqlitePool, ctx: &ExecutionContext) -> i64 {
    let task_uuid = match Uuid::parse_str(&ctx.task.id) {
        Ok(u) => u,
        Err(_) => return 1,
    };

    let attempts = TaskAttempt::fetch_all(pool, Some(task_uuid))
        .await
        .unwrap_or_default();

    let mut count = 0i64;
    for attempt in &attempts {
        let processes = db::models::execution_process::ExecutionProcess::find_by_task_attempt_id(
            pool, attempt.id, false,
        )
        .await
        .unwrap_or_default();

        count += processes
            .iter()
            .filter(|p| p.run_reason == ExecutionProcessRunReason::AgentReview)
            .count() as i64;
    }

    count.max(1) // At least 1 if we're in finalize_review
}

/// Find which agent watcher corresponds to the current review attempt.
/// Matches the attempt's executor against agent_watcher collaborators.
async fn find_watcher_agent_for_attempt(
    pool: &SqlitePool,
    ctx: &ExecutionContext,
) -> Option<String> {
    let watchers = Task::find_agent_watchers(pool, &ctx.task.id).await.ok()?;

    // If there's only one agent watcher, it's unambiguous
    if watchers.len() == 1 {
        return Some(watchers[0].actor_id.clone());
    }

    // If multiple, find the one that was most recently "triggered"
    watchers
        .into_iter()
        .find(|w| w.last_action == WATCHER_ACTION_TRIGGERED)
        .map(|w| w.actor_id)
}

/// Post an audit trail comment on the PR identifying the dev agent and task.
/// Logs debug messages (not warnings) for missing token / service creation failure.
pub async fn post_dev_agent_pr_comment(
    config: &Arc<RwLock<Config>>,
    ctx: &ExecutionContext,
    pr: &PrCreatedInfo,
) {
    use super::github_service::GitHubService;

    let github_config = config.read().await.github.clone();
    let github_token = match github_config.token() {
        Some(t) => t,
        None => {
            tracing::debug!("Dev agent PR comment skipped: no GitHub token configured");
            return;
        }
    };

    let github_service = match GitHubService::new(&github_token) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("Dev agent PR comment skipped: failed to create GitHub service: {e}");
            return;
        }
    };

    let repo_info = super::github_service::GitHubRepoInfo {
        owner: pr.repo_owner.clone(),
        repo_name: pr.repo_name.clone(),
    };

    let comment = format!(
        "Created by **ORCHA Dev Agent** for task `{}`: {}\n\n---\n*Automated by ORCHA Platform*",
        ctx.task.id, ctx.task.title
    );

    if let Err(e) = github_service
        .add_pr_comment(&repo_info, pr.number, &comment)
        .await
    {
        tracing::warn!("Failed to post dev agent PR comment: {e}");
    }
}
