mod creation;
mod execution;
mod follow_up;
mod git_ops;
mod streaming;

use std::path::PathBuf;

use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::StatusCode,
    middleware::from_fn_with_state,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Json as ResponseJson, Sse,
    },
    routing::{get, post},
    BoxError, Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use db::models::{
    execution_process::{ExecutionProcess, ExecutionProcessRunReason},
    follow_up_draft::FollowUpDraft,
    image::TaskImage,
    merge::{Merge, MergeStatus, PrMerge, PullRequestInfo},
    project::{Project, ProjectError},
    task::{Task, TaskRelationships, TaskStatus},
    task_attempt::{CreateTaskAttempt, TaskAttempt, TaskAttemptError},
};
use deployment::Deployment;
use executors::{
    actions::{
        coding_agent_follow_up::CodingAgentFollowUpRequest,
        script::{ScriptContext, ScriptRequest, ScriptRequestLanguage},
        ExecutorAction, ExecutorActionType,
    },
    executors::BaseCodingAgent,
    profile::ExecutorProfileId,
};
use futures_util::TryStreamExt;
use git2::BranchType;
use nora::coordination::{AgentStatus, CoordinationEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use services::services::{
    container::ContainerService,
    git::ConflictOp,
    github_service::{CreatePrRequest, GitHubService, GitHubServiceError},
    image::ImageService,
};
use sqlx::Error as SqlxError;
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    error::ApiError, middleware::load_task_attempt_middleware,
    routes::nora::emit_coordination_event, DeploymentImpl,
};

// --- Shared request/response types ---

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct RebaseTaskAttemptRequest {
    pub new_base_branch: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(tag = "type", rename_all = "snake_case")]
pub enum GitOperationError {
    MergeConflicts { message: String, op: ConflictOp },
    RebaseInProgress,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct ReplaceProcessRequest {
    /// Process to replace (delete this and later ones)
    pub process_id: Uuid,
    /// New prompt to use for the replacement follow-up
    pub prompt: String,
    /// Optional variant override
    pub variant: Option<String>,
    /// If true, allow resetting Git even when uncommitted changes exist
    pub force_when_dirty: Option<bool>,
    /// If false, skip performing the Git reset step (history drop still applies)
    pub perform_git_reset: Option<bool>,
}

#[derive(Debug, Serialize, TS)]
pub struct ReplaceProcessResult {
    pub deleted_count: i64,
    pub git_reset_needed: bool,
    pub git_reset_applied: bool,
    pub target_before_oid: Option<String>,
    pub new_execution_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
pub struct CreateGitHubPrRequest {
    pub title: String,
    pub body: Option<String>,
    pub base_branch: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FollowUpResponse {
    pub message: String,
    pub actual_attempt_id: Uuid,
    pub created_new_attempt: bool,
}

#[derive(Debug, Deserialize)]
pub struct TaskAttemptQuery {
    pub task_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ts_rs::TS)]
pub struct CreateTaskAttemptBody {
    pub task_id: Uuid,
    /// Executor profile specification
    pub executor_profile_id: ExecutorProfileId,
    pub base_branch: String,
}

impl CreateTaskAttemptBody {
    /// Get the executor profile ID
    pub fn get_executor_profile_id(&self) -> ExecutorProfileId {
        self.executor_profile_id.clone()
    }
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateFollowUpAttempt {
    pub prompt: String,
    pub variant: Option<String>,
    pub image_ids: Option<Vec<Uuid>>,
}

// Follow-up draft APIs and queueing
#[derive(Debug, Serialize, TS)]
pub struct FollowUpDraftResponse {
    pub task_attempt_id: Uuid,
    pub prompt: String,
    pub queued: bool,
    pub variant: Option<String>,
    pub image_ids: Option<Vec<Uuid>>, // attachments
    pub version: i64,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateFollowUpDraftRequest {
    pub prompt: Option<String>,
    // Present with null explicitly clears variant; absent leaves unchanged
    pub variant: Option<Option<String>>,
    pub image_ids: Option<Vec<Uuid>>, // send empty array to clear; omit to leave unchanged
    pub version: Option<i64>,         // optimistic concurrency
}

#[derive(Debug, Deserialize, TS)]
pub struct SetQueueRequest {
    pub queued: bool,
    pub expected_queued: Option<bool>,
    pub expected_version: Option<i64>,
}

#[derive(Debug, Serialize, TS)]
pub struct CommitInfo {
    pub sha: String,
    pub subject: String,
}

#[derive(Debug, Serialize, TS)]
pub struct CommitCompareResult {
    pub head_oid: String,
    pub target_oid: String,
    pub ahead_from_head: usize,
    pub behind_from_head: usize,
    pub is_linear: bool,
}

/// POST /task-attempts/:id/link-pr — register an externally-created PR against this attempt.
/// Creates a PR merge record without requiring a worktree or pushing branches.
#[derive(Debug, Deserialize, TS)]
pub struct LinkPrRequest {
    pub pr_number: i64,
    pub pr_url: String,
    pub target_branch: Option<String>,
}

/// POST /task-attempts/create-record — create a task attempt record without starting execution.
/// Used for linking external work (PRs, commits) to a task.
#[derive(Debug, Deserialize, TS)]
pub struct CreateTaskAttemptRecordBody {
    pub task_id: Uuid,
    pub executor: BaseCodingAgent,
    pub base_branch: String,
}

/// Response for create-record endpoint (avoids Uuid/TEXT decode issues)
#[derive(Debug, Serialize, TS)]
pub struct CreateRecordResponse {
    pub id: String,
    pub task_id: String,
    pub base_branch: String,
    pub executor: String,
}

#[derive(serde::Deserialize)]
pub struct OpenEditorRequest {
    editor_type: Option<String>,
    file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct BranchStatus {
    pub commits_behind: Option<usize>,
    pub commits_ahead: Option<usize>,
    pub has_uncommitted_changes: Option<bool>,
    pub head_oid: Option<String>,
    pub uncommitted_count: Option<usize>,
    pub untracked_count: Option<usize>,
    pub base_branch_name: String,
    pub remote_commits_behind: Option<usize>,
    pub remote_commits_ahead: Option<usize>,
    pub merges: Vec<Merge>,
    /// True if a `git rebase` is currently in progress in this worktree
    pub is_rebase_in_progress: bool,
    /// Current conflict operation if any
    pub conflict_op: Option<ConflictOp>,
    /// List of files currently in conflicted (unmerged) state
    pub conflicted_files: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct DeleteFileQuery {
    file_path: String,
}

// --- Shared helper functions ---

pub(crate) async fn publish_execution_events(
    executor_profile_id: &ExecutorProfileId,
    task_attempt: &TaskAttempt,
) {
    let agent_id = executor_profile_id.executor.to_string();
    let capabilities = capabilities_for_agent(&executor_profile_id.executor);

    emit_coordination_event(CoordinationEvent::AgentStatusUpdate {
        agent_id: agent_id.clone(),
        status: AgentStatus::Busy,
        capabilities: capabilities.clone(),
        timestamp: Utc::now(),
    })
    .await;

    emit_coordination_event(CoordinationEvent::TaskHandoff {
        from_agent: "NORA".to_string(),
        to_agent: agent_id,
        task_id: task_attempt.task_id.to_string(),
        context: json!({
            "taskAttemptId": task_attempt.id,
            "executor": executor_profile_id.executor,
            "projectId": task_attempt.task_id,
        }),
        timestamp: Utc::now(),
    })
    .await;
}

fn capabilities_for_agent(agent: &BaseCodingAgent) -> Vec<String> {
    match agent {
        BaseCodingAgent::ClaudeCode
        | BaseCodingAgent::Codex
        | BaseCodingAgent::Cursor
        | BaseCodingAgent::Opencode
        | BaseCodingAgent::QwenCode => vec!["coding".into(), "git".into()],
        BaseCodingAgent::Amp | BaseCodingAgent::Duck => vec!["automation".into(), "ops".into()],
        BaseCodingAgent::Gemini => vec!["analysis".into(), "multimodal".into()],
    }
}

// --- Router ---

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let task_attempt_id_router = Router::new()
        .route("/", get(creation::get_task_attempt))
        .route("/follow-up", post(follow_up::follow_up))
        .route(
            "/follow-up-draft",
            get(follow_up::get_follow_up_draft).put(follow_up::save_follow_up_draft),
        )
        .route(
            "/follow-up-draft/stream/ws",
            get(streaming::stream_follow_up_draft_ws),
        )
        .route(
            "/follow-up-draft/queue",
            post(follow_up::set_follow_up_queue),
        )
        .route("/replace-process", post(execution::replace_process))
        .route("/commit-info", get(git_ops::get_commit_info))
        .route("/commit-compare", get(git_ops::compare_commit_to_head))
        .route("/start-dev-server", post(execution::start_dev_server))
        .route(
            "/branch-status",
            get(git_ops::get_task_attempt_branch_status),
        )
        .route("/diff", get(git_ops::get_task_attempt_diff))
        .route("/merge", post(git_ops::merge_task_attempt))
        .route("/link-pr", post(git_ops::link_pr))
        .route("/push", post(git_ops::push_task_attempt_branch))
        .route("/rebase", post(git_ops::rebase_task_attempt))
        .route(
            "/conflicts/abort",
            post(git_ops::abort_conflicts_task_attempt),
        )
        .route("/pr", post(git_ops::create_github_pr))
        .route("/open-editor", post(execution::open_task_attempt_in_editor))
        .route("/delete-file", post(git_ops::delete_task_attempt_file))
        .route("/children", get(execution::get_task_attempt_children))
        .route("/stop", post(execution::stop_task_attempt_execution))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_task_attempt_middleware,
        ));

    let task_attempts_router = Router::new()
        .route(
            "/",
            get(creation::get_task_attempts).post(creation::create_task_attempt),
        )
        .route("/create-record", post(creation::create_task_attempt_record))
        .nest("/{id}", task_attempt_id_router);

    Router::new().nest("/task-attempts", task_attempts_router)
}
