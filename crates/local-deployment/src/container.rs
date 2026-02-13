use std::{
    collections::{HashMap, HashSet},
    io,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use anyhow::anyhow;
use async_stream::try_stream;
use async_trait::async_trait;
use axum::response::sse::Event;
use command_group::AsyncGroupChild;
use db::{
    DBService,
    models::{
        activity::{ActivityLog, ActorType, CreateActivityLog},
        agent_flow::{AgentFlow, AgentPhase, CreateAgentFlow, FlowType},
        agent_flow_event::{AgentFlowEvent, CreateFlowEvent, FlowEventPayload, FlowEventType},
        execution_artifact::{ArtifactType, CreateExecutionArtifact},
        execution_process::{
            ExecutionContext, ExecutionProcess, ExecutionProcessRunReason, ExecutionProcessStatus,
        },
        execution_slot::ExecutionSlot,
        execution_summary::ExecutionSummary,
        executor_session::ExecutorSession,
        follow_up_draft::FollowUpDraft,
        image::TaskImage,
        merge::Merge,
        model_pricing::infer_provider,
        project::Project,
        task::{Task, TaskStatus},
        task_attempt::TaskAttempt,
        vibe_transaction::VibeTransaction,
    },
};
use deployment::DeploymentError;
use executors::{
    actions::{Executable, ExecutorAction},
    logs::{
        NormalizedEntryType,
        utils::{
            ConversationPatch,
            patch::{escape_json_pointer_segment, extract_normalized_entry_from_patch},
        },
    },
};
use futures::{FutureExt, StreamExt, TryStreamExt, stream::select};
use notify_debouncer_full::DebouncedEvent;
use serde_json::json;
use services::services::{
    analytics::AnalyticsContext,
    artifacts::ArtifactService,
    config::Config,
    container::{ContainerError, ContainerRef, ContainerService},
    execution_summary::{DiffStats, ExecutionSummaryService},
    filesystem_watcher,
    git::{Commit, DiffTarget, GitService},
    image::ImageService,
    notification::NotificationService,
    vibe_pricing::VibePricingService,
    worktree_manager::WorktreeManager,
};
use utils::diff::DiffChangeKind;
use tokio::{sync::RwLock, task::JoinHandle};
use tokio_util::io::ReaderStream;
use utils::{
    diff::create_unified_diff_hunk,
    log_msg::LogMsg,
    msg_store::MsgStore,
    text::{git_branch_id, short_uuid},
};
use uuid::Uuid;

use crate::command;

#[derive(Clone)]
pub struct LocalContainerService {
    db: DBService,
    child_store: Arc<RwLock<HashMap<Uuid, Arc<RwLock<AsyncGroupChild>>>>>,
    msg_stores: Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>>,
    config: Arc<RwLock<Config>>,
    git: GitService,
    image_service: ImageService,
    analytics: Option<AnalyticsContext>,
    /// Maps execution_process_id → agent_flow_id for workflow tracking
    flow_ids: Arc<RwLock<HashMap<Uuid, Uuid>>>,
}

impl LocalContainerService {
    // Max cumulative content bytes allowed per diff stream
    const MAX_CUMULATIVE_DIFF_BYTES: usize = 150 * 1024; // 150KB

    // Apply stream-level omit policy based on cumulative bytes.
    // If adding this diff's contents exceeds the cap, strip contents and set stats.
    fn apply_stream_omit_policy(
        &self,
        diff: &mut utils::diff::Diff,
        sent_bytes: &Arc<AtomicUsize>,
    ) {
        // Compute size of current diff payload
        let mut size = 0usize;
        if let Some(ref s) = diff.old_content {
            size += s.len();
        }
        if let Some(ref s) = diff.new_content {
            size += s.len();
        }

        if size == 0 {
            return; // nothing to account
        }

        let current = sent_bytes.load(Ordering::Relaxed);
        if current.saturating_add(size) > Self::MAX_CUMULATIVE_DIFF_BYTES {
            // We will omit content for this diff. If we still have both sides loaded
            // (i.e., not already omitted by file-size guards), compute stats for UI.
            if diff.additions.is_none() && diff.deletions.is_none() {
                let old = diff.old_content.as_deref().unwrap_or("");
                let new = diff.new_content.as_deref().unwrap_or("");
                let hunk = create_unified_diff_hunk(old, new);
                let mut add = 0usize;
                let mut del = 0usize;
                for line in hunk.lines() {
                    if let Some(first) = line.chars().next() {
                        if first == '+' {
                            add += 1;
                        } else if first == '-' {
                            del += 1;
                        }
                    }
                }
                diff.additions = Some(add);
                diff.deletions = Some(del);
            }

            diff.old_content = None;
            diff.new_content = None;
            diff.content_omitted = true;
        } else {
            // safe to include; account for it
            let _ = sent_bytes.fetch_add(size, Ordering::Relaxed);
        }
    }
    pub fn new(
        db: DBService,
        msg_stores: Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>>,
        config: Arc<RwLock<Config>>,
        git: GitService,
        image_service: ImageService,
        analytics: Option<AnalyticsContext>,
    ) -> Self {
        let child_store = Arc::new(RwLock::new(HashMap::new()));
        let flow_ids = Arc::new(RwLock::new(HashMap::new()));

        LocalContainerService {
            db,
            child_store,
            msg_stores,
            config,
            git,
            image_service,
            analytics,
            flow_ids,
        }
    }

    pub async fn get_child_from_store(&self, id: &Uuid) -> Option<Arc<RwLock<AsyncGroupChild>>> {
        let map = self.child_store.read().await;
        map.get(id).cloned()
    }

    pub async fn add_child_to_store(&self, id: Uuid, exec: AsyncGroupChild) {
        let mut map = self.child_store.write().await;
        map.insert(id, Arc::new(RwLock::new(exec)));
    }

    pub async fn remove_child_from_store(&self, id: &Uuid) {
        let mut map = self.child_store.write().await;
        map.remove(id);
    }

    /// A context is finalized when
    /// - The next action is None (no follow-up actions)
    /// - The run reason is not DevServer
    fn should_finalize(ctx: &ExecutionContext) -> bool {
        ctx.execution_process
            .executor_action()
            .unwrap()
            .next_action
            .is_none()
            && (!matches!(
                ctx.execution_process.run_reason,
                ExecutionProcessRunReason::DevServer
            ))
    }

    /// Finalize task execution by updating status to InReview and sending notifications
    async fn finalize_task(db: &DBService, config: &Arc<RwLock<Config>>, ctx: &ExecutionContext) {
        if let Err(e) = Task::update_status(&db.pool, ctx.task.id, TaskStatus::InReview).await {
            tracing::error!("Failed to update task status to InReview: {e}");
        }
        let notify_cfg = config.read().await.notifications.clone();
        NotificationService::notify_execution_halted(notify_cfg, ctx).await;
    }

    /// Generate an execution summary for a completed execution process
    async fn generate_execution_summary(
        &self,
        ctx: &ExecutionContext,
    ) -> Result<ExecutionSummary, anyhow::Error> {
        // Get diff stats from git
        let diff_stats = self.compute_diff_stats(ctx).await.unwrap_or_default();

        // Get executor name from the task attempt
        let executor_name = Some(ctx.task_attempt.executor.as_str());

        // Create the execution summary
        let summary = ExecutionSummaryService::create_summary(
            &self.db.pool,
            ctx.task_attempt.id,
            Some(ctx.execution_process.id),
            Some(&ctx.execution_process),
            diff_stats,
            executor_name,
        )
        .await?;

        tracing::info!(
            "Generated execution summary {} for attempt {}: {} files modified, status {:?}",
            summary.id,
            ctx.task_attempt.id,
            summary.files_modified + summary.files_created,
            summary.completion_status
        );

        Ok(summary)
    }

    /// Compute diff statistics for an execution context
    async fn compute_diff_stats(&self, ctx: &ExecutionContext) -> Result<DiffStats, anyhow::Error> {
        let worktree_dir = self.task_attempt_to_current_dir(&ctx.task_attempt);
        let project_repo_path = self.get_project_repo_path(&ctx.task_attempt).await?;

        // Get the base commit to compare against
        let task_branch = ctx.task_attempt.branch.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Task attempt {} has no branch", ctx.task_attempt.id)
        })?;

        let base_commit = self.git().get_base_commit(
            &project_repo_path,
            task_branch,
            &ctx.task_attempt.base_branch,
        )?;

        // Get diffs from worktree
        let diffs = self.git().get_diffs(
            DiffTarget::Worktree {
                worktree_path: &worktree_dir,
                base_commit: &base_commit,
            },
            None,
        )?;

        // Count file changes by type
        let mut stats = DiffStats::default();
        for diff in &diffs {
            match diff.change {
                DiffChangeKind::Added => stats.files_created += 1,
                DiffChangeKind::Deleted => stats.files_deleted += 1,
                DiffChangeKind::Modified | DiffChangeKind::Renamed | DiffChangeKind::Copied => {
                    stats.files_modified += 1
                }
                DiffChangeKind::PermissionChange => {} // Don't count permission changes
            }
            if let Some(adds) = diff.additions {
                stats.additions += adds as i32;
            }
            if let Some(dels) = diff.deletions {
                stats.deletions += dels as i32;
            }
        }

        Ok(stats)
    }

    /// Defensively check for externally deleted worktrees and mark them as deleted in the database
    async fn check_externally_deleted_worktrees(db: &DBService) -> Result<(), DeploymentError> {
        let active_attempts = TaskAttempt::find_by_worktree_deleted(&db.pool).await?;
        tracing::debug!(
            "Checking {} active worktrees for external deletion...",
            active_attempts.len()
        );
        for (attempt_id, worktree_path) in active_attempts {
            // Check if worktree directory exists
            if !std::path::Path::new(&worktree_path).exists() {
                // Worktree was deleted externally, mark as deleted in database
                if let Err(e) = TaskAttempt::mark_worktree_deleted(&db.pool, attempt_id).await {
                    tracing::error!(
                        "Failed to mark externally deleted worktree as deleted for attempt {}: {}",
                        attempt_id,
                        e
                    );
                } else {
                    tracing::info!(
                        "Marked externally deleted worktree as deleted for attempt {} (path: {})",
                        attempt_id,
                        worktree_path
                    );
                }
            }
        }
        Ok(())
    }

    /// Find and delete orphaned worktrees that don't correspond to any task attempts
    async fn cleanup_orphaned_worktrees(&self) {
        // Check if orphan cleanup is disabled via environment variable
        if std::env::var("DISABLE_WORKTREE_ORPHAN_CLEANUP").is_ok() {
            tracing::debug!(
                "Orphan worktree cleanup is disabled via DISABLE_WORKTREE_ORPHAN_CLEANUP environment variable"
            );
            return;
        }
        let worktree_base_dir = WorktreeManager::get_worktree_base_dir();
        if !worktree_base_dir.exists() {
            tracing::debug!(
                "Worktree base directory {} does not exist, skipping orphan cleanup",
                worktree_base_dir.display()
            );
            return;
        }
        let entries = match std::fs::read_dir(&worktree_base_dir) {
            Ok(entries) => entries,
            Err(e) => {
                tracing::error!(
                    "Failed to read worktree base directory {}: {}",
                    worktree_base_dir.display(),
                    e
                );
                return;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    tracing::warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };
            let path = entry.path();
            // Only process directories
            if !path.is_dir() {
                continue;
            }

            let worktree_path_str = path.to_string_lossy().to_string();
            if let Ok(false) =
                TaskAttempt::container_ref_exists(&self.db().pool, &worktree_path_str).await
            {
                // This is an orphaned worktree - delete it
                tracing::info!("Found orphaned worktree: {}", worktree_path_str);
                if let Err(e) = WorktreeManager::cleanup_worktree(&path, None).await {
                    tracing::error!(
                        "Failed to remove orphaned worktree {}: {}",
                        worktree_path_str,
                        e
                    );
                } else {
                    tracing::info!(
                        "Successfully removed orphaned worktree: {}",
                        worktree_path_str
                    );
                }
            }
        }
    }

    pub async fn cleanup_expired_attempt(
        db: &DBService,
        attempt_id: Uuid,
        worktree_path: PathBuf,
        git_repo_path: PathBuf,
    ) -> Result<(), DeploymentError> {
        WorktreeManager::cleanup_worktree(&worktree_path, Some(&git_repo_path)).await?;
        // Mark worktree as deleted in database after successful cleanup
        TaskAttempt::mark_worktree_deleted(&db.pool, attempt_id).await?;
        tracing::info!("Successfully marked worktree as deleted for attempt {attempt_id}",);
        Ok(())
    }

    pub async fn cleanup_expired_attempts(db: &DBService) -> Result<(), DeploymentError> {
        let expired_attempts = TaskAttempt::find_expired_for_cleanup(&db.pool).await?;
        if expired_attempts.is_empty() {
            tracing::debug!("No expired worktrees found");
            return Ok(());
        }
        tracing::info!(
            "Found {} expired worktrees to clean up",
            expired_attempts.len()
        );
        for (attempt_id, worktree_path, git_repo_path) in expired_attempts {
            Self::cleanup_expired_attempt(
                db,
                attempt_id,
                PathBuf::from(worktree_path),
                PathBuf::from(git_repo_path),
            )
            .await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to clean up expired attempt {attempt_id}: {e}",);
            });
        }
        Ok(())
    }

    pub async fn spawn_worktree_cleanup(&self) {
        let db = self.db.clone();
        let mut cleanup_interval = tokio::time::interval(tokio::time::Duration::from_secs(1800)); // 30 minutes
        self.cleanup_orphaned_worktrees().await;
        tokio::spawn(async move {
            loop {
                cleanup_interval.tick().await;
                tracing::info!("Starting periodic worktree cleanup...");
                Self::check_externally_deleted_worktrees(&db)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("Failed to check externally deleted worktrees: {}", e);
                    });
                Self::cleanup_expired_attempts(&db)
                    .await
                    .unwrap_or_else(|e| {
                        tracing::error!("Failed to clean up expired worktree attempts: {}", e)
                    });
            }
        });
    }

    /// Spawn a background task that polls the child process for completion and
    /// cleans up the execution entry when it exits.
    pub fn spawn_exit_monitor(
        &self,
        exec_id: &Uuid,
        exit_signal: Option<tokio::sync::oneshot::Receiver<()>>,
    ) -> JoinHandle<()> {
        let exec_id = *exec_id;
        let child_store = self.child_store.clone();
        let msg_stores = self.msg_stores.clone();
        let db = self.db.clone();
        let config = self.config.clone();
        let container = self.clone();
        let analytics = self.analytics.clone();

        let mut process_exit_rx = self.spawn_os_exit_watcher(exec_id);

        tokio::spawn(async move {
            let mut exit_signal_future = exit_signal
                .map(|rx| rx.map(|_| ()).boxed()) // wait for signal
                .unwrap_or_else(|| std::future::pending::<()>().boxed()); // no signal, stall forever

            let status_result: std::io::Result<std::process::ExitStatus>;

            // Wait for process to exit, or exit signal from executor
            tokio::select! {
                // Exit signal.
                // Some coding agent processes do not automatically exit after processing the user request; instead the executor
                // signals when processing has finished to gracefully kill the process.
                _ = &mut exit_signal_future => {
                    // Executor signaled completion: kill group and remember to force Completed(0)
                    if let Some(child_lock) = child_store.read().await.get(&exec_id).cloned() {
                        let mut child = child_lock.write().await ;
                        if let Err(err) = command::kill_process_group(&mut child).await {
                            tracing::error!("Failed to kill process group after exit signal: {} {}", exec_id, err);
                        }
                    }
                    status_result = Ok(std::process::ExitStatus::from_raw(0));
                }
                // Process exit
                exit_status_result = &mut process_exit_rx => {
                    status_result = exit_status_result.unwrap_or_else(|e| Err(std::io::Error::other(e)));
                }
            }

            let (exit_code, status) = match status_result {
                Ok(exit_status) => {
                    let code = exit_status.code().unwrap_or(-1) as i64;
                    let status = if exit_status.success() {
                        ExecutionProcessStatus::Completed
                    } else {
                        ExecutionProcessStatus::Failed
                    };
                    (Some(code), status)
                }
                Err(_) => (None, ExecutionProcessStatus::Failed),
            };

            if !ExecutionProcess::was_killed(&db.pool, exec_id).await
                && let Err(e) =
                    ExecutionProcess::update_completion(&db.pool, exec_id, status, exit_code).await
            {
                tracing::error!("Failed to update execution process completion: {}", e);
            }

            if let Ok(ctx) = ExecutionProcess::load_context(&db.pool, exec_id).await {
                // Update executor session summary if available
                if let Err(e) = container.update_executor_session_summary(&exec_id).await {
                    tracing::warn!("Failed to update executor session summary: {}", e);
                }

                if matches!(
                    ctx.execution_process.status,
                    ExecutionProcessStatus::Completed
                ) && exit_code == Some(0)
                {
                    // Commit changes (if any) and get feedback about whether changes were made
                    let changes_committed = match container.try_commit_changes(&ctx).await {
                        Ok(committed) => committed,
                        Err(e) => {
                            tracing::error!("Failed to commit changes after execution: {}", e);
                            // Treat commit failures as if changes were made to be safe
                            true
                        }
                    };

                    let should_start_next = if matches!(
                        ctx.execution_process.run_reason,
                        ExecutionProcessRunReason::CodingAgent
                    ) {
                        changes_committed
                    } else {
                        true
                    };

                    if should_start_next {
                        // If the process exited successfully, start the next action
                        if let Err(e) = container.try_start_next_action(&ctx).await {
                            tracing::error!("Failed to start next action after completion: {}", e);
                        }
                    } else {
                        tracing::info!(
                            "Skipping cleanup script for task attempt {} - no changes made by coding agent",
                            ctx.task_attempt.id
                        );

                        // Manually finalize task since we're bypassing normal execution flow
                        Self::finalize_task(&db, &config, &ctx).await;
                    }
                }

                if Self::should_finalize(&ctx) {
                    Self::finalize_task(&db, &config, &ctx).await;
                    // After finalization, check if a queued follow-up exists and start it
                    if let Err(e) = container.try_consume_queued_followup(&ctx).await {
                        tracing::error!(
                            "Failed to start queued follow-up for attempt {}: {}",
                            ctx.task_attempt.id,
                            e
                        );
                    }
                }

                // Generate execution summary for CodingAgent executions
                if matches!(
                    ctx.execution_process.run_reason,
                    ExecutionProcessRunReason::CodingAgent
                ) {
                    if let Err(e) = container.generate_execution_summary(&ctx).await {
                        tracing::warn!(
                            "Failed to generate execution summary for process {}: {}",
                            ctx.execution_process.id,
                            e
                        );
                    }

                    // Complete workflow flow (Part 2: Workflow Logs)
                    container.complete_classic_pipeline_flow(&ctx).await;

                    // Record VIBE cost (Part 1: VIBE Cost Tracking)
                    container.record_execution_vibe_cost(&ctx).await;

                    // Create execution artifacts (Part 3: Artifacts)
                    container.create_execution_artifacts(&ctx).await;

                    // Log activity: execution completed
                    let completion_status = match ctx.execution_process.status {
                        ExecutionProcessStatus::Completed => "completed",
                        ExecutionProcessStatus::Failed => "failed",
                        ExecutionProcessStatus::Killed => "killed",
                        _ => "unknown",
                    };
                    if let Err(e) = ActivityLog::create(
                        &db.pool,
                        &CreateActivityLog {
                            task_id: ctx.task.id,
                            actor_id: ctx.task_attempt.executor.clone(),
                            actor_type: ActorType::Agent,
                            action: "execution_completed".to_string(),
                            previous_state: None,
                            new_state: Some(json!({
                                "status": completion_status,
                                "exit_code": exit_code,
                            })),
                            metadata: Some(json!({
                                "execution_process_id": ctx.execution_process.id.to_string(),
                                "attempt_id": ctx.task_attempt.id.to_string(),
                                "executor": ctx.task_attempt.executor,
                            })),
                        },
                    )
                    .await
                    {
                        tracing::warn!("Failed to log execution_completed activity: {}", e);
                    }

                    // Update task collaborators
                    if let Err(e) = Task::update_collaborator(
                        &db.pool,
                        ctx.task.id,
                        &ctx.task_attempt.executor,
                        "agent",
                        completion_status,
                    )
                    .await
                    {
                        tracing::warn!("Failed to update task collaborators: {}", e);
                    }
                }

                // Fire analytics event when CodingAgent execution has finished
                if config.read().await.analytics_enabled == Some(true)
                    && matches!(
                        &ctx.execution_process.run_reason,
                        ExecutionProcessRunReason::CodingAgent
                    )
                    && let Some(analytics) = &analytics
                {
                    analytics.analytics_service.track_event(&analytics.user_id, "task_attempt_finished", Some(json!({
                        "task_id": ctx.task.id.to_string(),
                        "project_id": ctx.task.project_id.to_string(),
                        "attempt_id": ctx.task_attempt.id.to_string(),
                        "execution_success": matches!(ctx.execution_process.status, ExecutionProcessStatus::Completed),
                        "exit_code": ctx.execution_process.exit_code,
                    })));
                }
            }

            // Now that commit/next-action/finalization steps for this process are complete,
            // capture the HEAD OID as the definitive "after" state (best-effort).
            if let Ok(ctx) = ExecutionProcess::load_context(&db.pool, exec_id).await {
                let worktree_dir = container.task_attempt_to_current_dir(&ctx.task_attempt);
                if let Ok(head) = container.git().get_head_info(&worktree_dir)
                    && let Err(e) =
                        ExecutionProcess::update_after_head_commit(&db.pool, exec_id, &head.oid)
                            .await
                {
                    tracing::warn!("Failed to update after_head_commit for {}: {}", exec_id, e);
                }

                // Release execution slots for this task attempt when execution finalizes
                if Self::should_finalize(&ctx) {
                    if let Err(e) = ExecutionSlot::release_all_for_task_attempt(
                        &db.pool,
                        ctx.task_attempt.id,
                    )
                    .await
                    {
                        tracing::warn!(
                            "Failed to release slots for task attempt {}: {}",
                            ctx.task_attempt.id,
                            e
                        );
                    } else {
                        tracing::debug!(
                            "Released execution slots for task attempt {}",
                            ctx.task_attempt.id
                        );
                    }
                }
            }

            // Cleanup msg store
            if let Some(msg_arc) = msg_stores.write().await.remove(&exec_id) {
                msg_arc.push_finished();
                tokio::time::sleep(Duration::from_millis(50)).await; // Wait for the finish message to propogate
                match Arc::try_unwrap(msg_arc) {
                    Ok(inner) => drop(inner),
                    Err(arc) => tracing::error!(
                        "There are still {} strong Arcs to MsgStore for {}",
                        Arc::strong_count(&arc),
                        exec_id
                    ),
                }
            }

            // Cleanup child handle
            child_store.write().await.remove(&exec_id);
        })
    }

    pub fn spawn_os_exit_watcher(
        &self,
        exec_id: Uuid,
    ) -> tokio::sync::oneshot::Receiver<std::io::Result<std::process::ExitStatus>> {
        let (tx, rx) = tokio::sync::oneshot::channel::<std::io::Result<std::process::ExitStatus>>();
        let child_store = self.child_store.clone();
        tokio::spawn(async move {
            loop {
                let child_lock = {
                    let map = child_store.read().await;
                    map.get(&exec_id).cloned()
                };
                if let Some(child_lock) = child_lock {
                    let mut child_handler = child_lock.write().await;
                    match child_handler.try_wait() {
                        Ok(Some(status)) => {
                            let _ = tx.send(Ok(status));
                            break;
                        }
                        Ok(None) => {}
                        Err(e) => {
                            let _ = tx.send(Err(e));
                            break;
                        }
                    }
                } else {
                    let _ = tx.send(Err(io::Error::other(format!(
                        "Child handle missing for {exec_id}"
                    ))));
                    break;
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        });
        rx
    }

    pub fn dir_name_from_task_attempt(attempt_id: &Uuid, task_title: &str) -> String {
        let task_title_id = git_branch_id(task_title);
        format!("{}-{}", short_uuid(attempt_id), task_title_id)
    }

    pub fn git_branch_from_task_attempt(attempt_id: &Uuid, task_title: &str) -> String {
        let task_title_id = git_branch_id(task_title);
        format!("vk/{}-{}", short_uuid(attempt_id), task_title_id)
    }

    async fn track_child_msgs_in_store(&self, id: Uuid, child: &mut AsyncGroupChild) {
        let store = Arc::new(MsgStore::new());

        let out = child.inner().stdout.take().expect("no stdout");
        let err = child.inner().stderr.take().expect("no stderr");

        // Map stdout bytes -> LogMsg::Stdout
        let out = ReaderStream::new(out)
            .map_ok(|chunk| LogMsg::Stdout(String::from_utf8_lossy(&chunk).into_owned()));

        // Map stderr bytes -> LogMsg::Stderr
        let err = ReaderStream::new(err)
            .map_ok(|chunk| LogMsg::Stderr(String::from_utf8_lossy(&chunk).into_owned()));

        // If you have a JSON Patch source, map it to LogMsg::JsonPatch too, then select all three.

        // Merge and forward into the store
        let merged = select(out, err); // Stream<Item = Result<LogMsg, io::Error>>
        let debounced = utils::stream_ext::debounce_logs(merged);
        store.clone().spawn_forwarder(debounced);

        let mut map = self.msg_stores().write().await;
        map.insert(id, store);
    }

    /// Get the worktree path for a task attempt
    #[allow(dead_code)]
    async fn get_worktree_path(
        &self,
        task_attempt: &TaskAttempt,
    ) -> Result<PathBuf, ContainerError> {
        let container_ref = self.ensure_container_exists(task_attempt).await?;
        let worktree_dir = PathBuf::from(&container_ref);

        if !worktree_dir.exists() {
            return Err(ContainerError::Other(anyhow!(
                "Worktree directory not found"
            )));
        }

        Ok(worktree_dir)
    }

    /// Get the project repository path for a task attempt
    async fn get_project_repo_path(
        &self,
        task_attempt: &TaskAttempt,
    ) -> Result<PathBuf, ContainerError> {
        let project_repo_path = task_attempt
            .parent_task(&self.db().pool)
            .await?
            .ok_or(ContainerError::Other(anyhow!("Parent task not found")))?
            .parent_project(&self.db().pool)
            .await?
            .ok_or(ContainerError::Other(anyhow!("Parent project not found")))?
            .git_repo_path;

        Ok(project_repo_path)
    }

    /// Create a diff stream for merged attempts (never changes)
    fn create_merged_diff_stream(
        &self,
        project_repo_path: &Path,
        merge_commit_id: &str,
    ) -> Result<futures::stream::BoxStream<'static, Result<Event, std::io::Error>>, ContainerError>
    {
        let diffs = self.git().get_diffs(
            DiffTarget::Commit {
                repo_path: project_repo_path,
                commit_sha: merge_commit_id,
            },
            None,
        )?;

        let cum = Arc::new(AtomicUsize::new(0));
        let diffs: Vec<_> = diffs
            .into_iter()
            .map(|mut d| {
                self.apply_stream_omit_policy(&mut d, &cum);
                d
            })
            .collect();

        let stream = futures::stream::iter(diffs.into_iter().map(|diff| {
            let entry_index = GitService::diff_path(&diff);
            let patch =
                ConversationPatch::add_diff(escape_json_pointer_segment(&entry_index), diff);
            let event = LogMsg::JsonPatch(patch).to_sse_event();
            Ok::<_, std::io::Error>(event)
        }))
        .chain(futures::stream::once(async {
            Ok::<_, std::io::Error>(LogMsg::Finished.to_sse_event())
        }))
        .boxed();

        Ok(stream)
    }

    /// Create a live diff stream for ongoing attempts
    async fn create_live_diff_stream(
        &self,
        worktree_path: &Path,
        base_commit: &Commit,
    ) -> Result<futures::stream::BoxStream<'static, Result<Event, std::io::Error>>, ContainerError>
    {
        // Get initial snapshot
        let git_service = self.git().clone();
        let initial_diffs = git_service.get_diffs(
            DiffTarget::Worktree {
                worktree_path,
                base_commit,
            },
            None,
        )?;

        // cumulative counter for entire stream
        let cumulative = Arc::new(AtomicUsize::new(0));
        // track which file paths have been emitted with full content already
        let full_sent = Arc::new(std::sync::RwLock::new(HashSet::<String>::new()));
        let initial_diffs: Vec<_> = initial_diffs
            .into_iter()
            .map(|mut d| {
                self.apply_stream_omit_policy(&mut d, &cumulative);
                d
            })
            .collect();

        // Record which paths were sent with full content
        {
            let mut guard = full_sent.write().unwrap();
            for d in &initial_diffs {
                if !d.content_omitted {
                    let p = GitService::diff_path(d);
                    guard.insert(p);
                }
            }
        }

        let initial_stream = futures::stream::iter(initial_diffs.into_iter().map(|diff| {
            let entry_index = GitService::diff_path(&diff);
            let patch =
                ConversationPatch::add_diff(escape_json_pointer_segment(&entry_index), diff);
            let event = LogMsg::JsonPatch(patch).to_sse_event();
            Ok::<_, std::io::Error>(event)
        }))
        .boxed();

        // Create live update stream
        let worktree_path = worktree_path.to_path_buf();
        let base_commit = base_commit.clone();

        let live_stream = {
            let git_service = git_service.clone();
            let worktree_path_for_spawn = worktree_path.clone();
            let cumulative = Arc::clone(&cumulative);
            let full_sent = Arc::clone(&full_sent);
            try_stream! {
                // Move the expensive watcher setup to blocking thread to avoid blocking the async runtime
                let watcher_result = tokio::task::spawn_blocking(move || {
                    filesystem_watcher::async_watcher(worktree_path_for_spawn)
                })
                .await
                .map_err(|e| io::Error::other(format!("Failed to spawn watcher setup: {e}")))?;

                let (_debouncer, mut rx, canonical_worktree_path) = watcher_result
                    .map_err(|e| io::Error::other(e.to_string()))?;

                while let Some(result) = rx.next().await {
                    match result {
                        Ok(events) => {
                            let changed_paths = Self::extract_changed_paths(&events, &canonical_worktree_path, &worktree_path);

                            if !changed_paths.is_empty() {
                                for event in Self::process_file_changes(
                                    &git_service,
                                    &worktree_path,
                                    &base_commit,
                                    &changed_paths,
                                    &cumulative,
                                    &full_sent,
                                ).map_err(|e| {
                                    tracing::error!("Error processing file changes: {}", e);
                                    io::Error::other(e.to_string())
                                })? {
                                    yield event;
                                }
                            }
                        }
                        Err(errors) => {
                            let error_msg = errors.iter()
                                .map(|e| e.to_string())
                                .collect::<Vec<_>>()
                                .join("; ");
                            tracing::error!("Filesystem watcher error: {}", error_msg);
                            Err(io::Error::other(error_msg))?;
                        }
                    }
                }
            }
        }.boxed();

        // Ensure all initial diffs are emitted before live updates, to avoid
        // earlier files being abbreviated due to interleaving ordering.
        let combined_stream = initial_stream.chain(live_stream);
        Ok(combined_stream.boxed())
    }

    /// Extract changed file paths from filesystem events
    fn extract_changed_paths(
        events: &[DebouncedEvent],
        canonical_worktree_path: &Path,
        worktree_path: &Path,
    ) -> Vec<String> {
        events
            .iter()
            .flat_map(|event| &event.paths)
            .filter_map(|path| {
                path.strip_prefix(canonical_worktree_path)
                    .or_else(|_| path.strip_prefix(worktree_path))
                    .ok()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
            })
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Process file changes and generate diff events
    fn process_file_changes(
        git_service: &GitService,
        worktree_path: &Path,
        base_commit: &Commit,
        changed_paths: &[String],
        cumulative_bytes: &Arc<AtomicUsize>,
        full_sent_paths: &Arc<std::sync::RwLock<HashSet<String>>>,
    ) -> Result<Vec<Event>, ContainerError> {
        let path_filter: Vec<&str> = changed_paths.iter().map(|s| s.as_str()).collect();

        let current_diffs = git_service.get_diffs(
            DiffTarget::Worktree {
                worktree_path,
                base_commit,
            },
            Some(&path_filter),
        )?;

        let mut events = Vec::new();
        let mut files_with_diffs = HashSet::new();

        // Add/update files that have diffs
        for mut diff in current_diffs {
            let file_path = GitService::diff_path(&diff);
            files_with_diffs.insert(file_path.clone());
            // Apply stream-level omit policy (affects contents and stats)
            // Note: we can't call self methods from static fn; implement inline
            {
                // Compute size
                let mut size = 0usize;
                if let Some(ref s) = diff.old_content {
                    size += s.len();
                }
                if let Some(ref s) = diff.new_content {
                    size += s.len();
                }
                if size > 0 {
                    let current = cumulative_bytes.load(Ordering::Relaxed);
                    if current.saturating_add(size)
                        > LocalContainerService::MAX_CUMULATIVE_DIFF_BYTES
                    {
                        if diff.additions.is_none() && diff.deletions.is_none() {
                            let old = diff.old_content.as_deref().unwrap_or("");
                            let new = diff.new_content.as_deref().unwrap_or("");
                            let hunk = create_unified_diff_hunk(old, new);
                            let mut add = 0usize;
                            let mut del = 0usize;
                            for line in hunk.lines() {
                                if let Some(first) = line.chars().next() {
                                    if first == '+' {
                                        add += 1;
                                    } else if first == '-' {
                                        del += 1;
                                    }
                                }
                            }
                            diff.additions = Some(add);
                            diff.deletions = Some(del);
                        }
                        diff.old_content = None;
                        diff.new_content = None;
                        diff.content_omitted = true;
                    } else {
                        let _ = cumulative_bytes.fetch_add(size, Ordering::Relaxed);
                    }
                }
            }

            // If this diff would be omitted and we already sent a full-content
            // version of this path earlier in the stream, skip sending a
            // degrading replacement.
            if diff.content_omitted {
                if full_sent_paths.read().unwrap().contains(&file_path) {
                    continue;
                }
            } else {
                // Track that we have sent a full-content version
                {
                    let mut guard = full_sent_paths.write().unwrap();
                    guard.insert(file_path.clone());
                }
            }

            let patch = ConversationPatch::add_diff(escape_json_pointer_segment(&file_path), diff);
            let event = LogMsg::JsonPatch(patch).to_sse_event();
            events.push(event);
        }

        // Remove files that changed but no longer have diffs
        for changed_path in changed_paths {
            if !files_with_diffs.contains(changed_path) {
                let patch =
                    ConversationPatch::remove_diff(escape_json_pointer_segment(changed_path));
                let event = LogMsg::JsonPatch(patch).to_sse_event();
                events.push(event);
            }
        }

        Ok(events)
    }
}

#[async_trait]
impl ContainerService for LocalContainerService {
    fn msg_stores(&self) -> &Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>> {
        &self.msg_stores
    }

    fn db(&self) -> &DBService {
        &self.db
    }

    fn git(&self) -> &GitService {
        &self.git
    }

    fn task_attempt_to_current_dir(&self, task_attempt: &TaskAttempt) -> PathBuf {
        PathBuf::from(task_attempt.container_ref.clone().unwrap_or_default())
    }
    /// Create a container
    async fn create(&self, task_attempt: &TaskAttempt) -> Result<ContainerRef, ContainerError> {
        let task = task_attempt
            .parent_task(&self.db.pool)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let worktree_dir_name =
            LocalContainerService::dir_name_from_task_attempt(&task_attempt.id, &task.title);
        let worktree_path = WorktreeManager::get_worktree_base_dir().join(&worktree_dir_name);

        let git_branch_name =
            LocalContainerService::git_branch_from_task_attempt(&task_attempt.id, &task.title);

        let project = task
            .parent_project(&self.db.pool)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        WorktreeManager::create_worktree(
            &project.git_repo_path,
            &git_branch_name,
            &worktree_path,
            &task_attempt.base_branch,
            true, // create new branch
        )
        .await?;

        // Copy files specified in the project's copy_files field
        if let Some(copy_files) = &project.copy_files
            && !copy_files.trim().is_empty()
        {
            self.copy_project_files(&project.git_repo_path, &worktree_path, copy_files)
                .await
                .unwrap_or_else(|e| {
                    tracing::warn!("Failed to copy project files: {}", e);
                });
        }

        // Copy task images from cache to worktree
        if let Err(e) = self
            .image_service
            .copy_images_by_task_to_worktree(&worktree_path, task.id)
            .await
        {
            tracing::warn!("Failed to copy task images to worktree: {}", e);
        }

        // Update both container_ref and branch in the database
        TaskAttempt::update_container_ref(
            &self.db.pool,
            task_attempt.id,
            &worktree_path.to_string_lossy(),
        )
        .await?;

        TaskAttempt::update_branch(&self.db.pool, task_attempt.id, &git_branch_name).await?;

        Ok(worktree_path.to_string_lossy().to_string())
    }

    async fn delete_inner(&self, task_attempt: &TaskAttempt) -> Result<(), ContainerError> {
        // cleanup the container, here that means deleting the worktree
        let task = task_attempt
            .parent_task(&self.db.pool)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;
        let git_repo_path = match Project::find_by_id(&self.db.pool, task.project_id).await {
            Ok(Some(project)) => Some(project.git_repo_path.clone()),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to fetch project {}: {}", task.project_id, e);
                None
            }
        };
        WorktreeManager::cleanup_worktree(
            &PathBuf::from(task_attempt.container_ref.clone().unwrap_or_default()),
            git_repo_path.as_deref(),
        )
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to clean up worktree for task attempt {}: {}",
                task_attempt.id,
                e
            );
        });
        Ok(())
    }

    async fn ensure_container_exists(
        &self,
        task_attempt: &TaskAttempt,
    ) -> Result<ContainerRef, ContainerError> {
        // Get required context
        let task = task_attempt
            .parent_task(&self.db.pool)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let project = task
            .parent_project(&self.db.pool)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let container_ref = task_attempt.container_ref.as_ref().ok_or_else(|| {
            ContainerError::Other(anyhow!("Container ref not found for task attempt"))
        })?;
        let worktree_path = PathBuf::from(container_ref);

        let branch_name = task_attempt
            .branch
            .as_ref()
            .ok_or_else(|| ContainerError::Other(anyhow!("Branch not found for task attempt")))?;

        WorktreeManager::ensure_worktree_exists(
            &project.git_repo_path,
            branch_name,
            &worktree_path,
        )
        .await?;

        Ok(container_ref.to_string())
    }

    async fn is_container_clean(&self, task_attempt: &TaskAttempt) -> Result<bool, ContainerError> {
        if let Some(container_ref) = &task_attempt.container_ref {
            // If container_ref is set, check if the worktree exists
            let path = PathBuf::from(container_ref);
            if path.exists() {
                self.git().is_worktree_clean(&path).map_err(|e| e.into())
            } else {
                return Ok(true); // No worktree means it's clean
            }
        } else {
            return Ok(true); // No container_ref means no worktree, so it's clean
        }
    }

    async fn start_execution_inner(
        &self,
        task_attempt: &TaskAttempt,
        execution_process: &ExecutionProcess,
        executor_action: &ExecutorAction,
    ) -> Result<(), ContainerError> {
        // Get the worktree path
        let container_ref = task_attempt
            .container_ref
            .as_ref()
            .ok_or(ContainerError::Other(anyhow!(
                "Container ref not found for task attempt"
            )))?;
        let current_dir = PathBuf::from(container_ref);

        // Create the child and stream, add to execution tracker
        let mut spawned = executor_action.spawn(&current_dir).await?;

        self.track_child_msgs_in_store(execution_process.id, &mut spawned.child)
            .await;

        self.add_child_to_store(execution_process.id, spawned.child)
            .await;

        // Spawn unified exit monitor: watches OS exit and optional executor signal
        let _hn = self.spawn_exit_monitor(&execution_process.id, spawned.exit_signal);

        // Log activity: execution started
        if let Err(e) = ActivityLog::create(
            &self.db.pool,
            &CreateActivityLog {
                task_id: task_attempt.task_id,
                actor_id: task_attempt.executor.clone(),
                actor_type: ActorType::Agent,
                action: "execution_started".to_string(),
                previous_state: None,
                new_state: Some(json!({
                    "execution_process_id": execution_process.id.to_string(),
                    "attempt_id": task_attempt.id.to_string(),
                    "run_reason": format!("{:?}", execution_process.run_reason),
                })),
                metadata: Some(json!({
                    "executor": task_attempt.executor,
                    "branch": task_attempt.branch,
                })),
            },
        )
        .await
        {
            tracing::warn!("Failed to log execution_started activity: {}", e);
        }

        // Update task collaborators
        if let Err(e) = Task::update_collaborator(
            &self.db.pool,
            task_attempt.task_id,
            &task_attempt.executor,
            "agent",
            "execution_started",
        )
        .await
        {
            tracing::warn!("Failed to update task collaborators: {}", e);
        }

        // Create workflow flow for CodingAgent executions
        if matches!(
            execution_process.run_reason,
            ExecutionProcessRunReason::CodingAgent
        ) {
            if let Ok(ctx) = ExecutionProcess::load_context(&self.db.pool, execution_process.id).await {
                self.create_classic_pipeline_flow(&ctx).await;
            }
        }

        Ok(())
    }

    async fn stop_execution(
        &self,
        execution_process: &ExecutionProcess,
    ) -> Result<(), ContainerError> {
        let child = self
            .get_child_from_store(&execution_process.id)
            .await
            .ok_or_else(|| {
                ContainerError::Other(anyhow!("Child process not found for execution"))
            })?;
        ExecutionProcess::update_completion(
            &self.db.pool,
            execution_process.id,
            ExecutionProcessStatus::Killed,
            None,
        )
        .await?;

        // Kill the child process and remove from the store
        {
            let mut child_guard = child.write().await;
            if let Err(e) = command::kill_process_group(&mut child_guard).await {
                tracing::error!(
                    "Failed to stop execution process {}: {}",
                    execution_process.id,
                    e
                );
                return Err(e);
            }
        }
        self.remove_child_from_store(&execution_process.id).await;

        // Mark the process finished in the MsgStore
        if let Some(msg) = self.msg_stores.write().await.remove(&execution_process.id) {
            msg.push_finished();
        }

        // Update task status to InReview when execution is stopped
        if let Ok(ctx) = ExecutionProcess::load_context(&self.db.pool, execution_process.id).await
            && !matches!(
                ctx.execution_process.run_reason,
                ExecutionProcessRunReason::DevServer
            )
            && let Err(e) =
                Task::update_status(&self.db.pool, ctx.task.id, TaskStatus::InReview).await
        {
            tracing::error!("Failed to update task status to InReview: {e}");
        }

        tracing::debug!(
            "Execution process {} stopped successfully",
            execution_process.id
        );

        // Record after-head commit OID (best-effort)
        if let Ok(ctx) = ExecutionProcess::load_context(&self.db.pool, execution_process.id).await {
            let worktree = self.task_attempt_to_current_dir(&ctx.task_attempt);
            if let Ok(head) = self.git().get_head_info(&worktree) {
                let _ = ExecutionProcess::update_after_head_commit(
                    &self.db.pool,
                    execution_process.id,
                    &head.oid,
                )
                .await;
            }
        }

        Ok(())
    }

    async fn get_diff(
        &self,
        task_attempt: &TaskAttempt,
    ) -> Result<futures::stream::BoxStream<'static, Result<Event, std::io::Error>>, ContainerError>
    {
        let project_repo_path = self.get_project_repo_path(task_attempt).await?;
        let latest_merge =
            Merge::find_latest_by_task_attempt_id(&self.db.pool, task_attempt.id).await?;
        let task_branch = task_attempt
            .branch
            .clone()
            .ok_or(ContainerError::Other(anyhow!(
                "Task attempt {} does not have a branch",
                task_attempt.id
            )))?;

        let is_ahead = if let Ok((ahead, _)) = self.git().get_branch_status(
            &project_repo_path,
            &task_branch,
            &task_attempt.base_branch,
        ) {
            ahead > 0
        } else {
            false
        };

        // Show merged diff when no new work is on the branch or container
        if let Some(merge) = &latest_merge
            && let Some(commit) = merge.merge_commit()
            && self.is_container_clean(task_attempt).await?
            && !is_ahead
        {
            return self.create_merged_diff_stream(&project_repo_path, &commit);
        }

        // worktree is needed for non-merged diffs
        let container_ref = self.ensure_container_exists(task_attempt).await?;
        let worktree_path = PathBuf::from(container_ref);

        let base_commit = self.git().get_base_commit(
            &project_repo_path,
            &task_branch,
            &task_attempt.base_branch,
        )?;

        // Handle ongoing attempts (live streaming diff)
        self.create_live_diff_stream(&worktree_path, &base_commit)
            .await
    }

    async fn try_commit_changes(&self, ctx: &ExecutionContext) -> Result<bool, ContainerError> {
        if !matches!(
            ctx.execution_process.run_reason,
            ExecutionProcessRunReason::CodingAgent | ExecutionProcessRunReason::CleanupScript,
        ) {
            return Ok(false);
        }

        let message = match ctx.execution_process.run_reason {
            ExecutionProcessRunReason::CodingAgent => {
                // Try to retrieve the task summary from the executor session
                // otherwise fallback to default message
                match ExecutorSession::find_by_execution_process_id(
                    &self.db().pool,
                    ctx.execution_process.id,
                )
                .await
                {
                    Ok(Some(session)) if session.summary.is_some() => session.summary.unwrap(),
                    Ok(_) => {
                        tracing::debug!(
                            "No summary found for execution process {}, using default message",
                            ctx.execution_process.id
                        );
                        format!(
                            "Commit changes from coding agent for task attempt {}",
                            ctx.task_attempt.id
                        )
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to retrieve summary for execution process {}: {}",
                            ctx.execution_process.id,
                            e
                        );
                        format!(
                            "Commit changes from coding agent for task attempt {}",
                            ctx.task_attempt.id
                        )
                    }
                }
            }
            ExecutionProcessRunReason::CleanupScript => {
                format!(
                    "Cleanup script changes for task attempt {}",
                    ctx.task_attempt.id
                )
            }
            _ => Err(ContainerError::Other(anyhow::anyhow!(
                "Invalid run reason for commit"
            )))?,
        };

        let container_ref = ctx.task_attempt.container_ref.as_ref().ok_or_else(|| {
            ContainerError::Other(anyhow::anyhow!("Container reference not found"))
        })?;

        tracing::debug!(
            "Committing changes for task attempt {} at path {:?}: '{}'",
            ctx.task_attempt.id,
            &container_ref,
            message
        );

        let changes_committed = self.git().commit(Path::new(container_ref), &message)?;
        Ok(changes_committed)
    }

    /// Copy files from the original project directory to the worktree
    async fn copy_project_files(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        copy_files: &str,
    ) -> Result<(), ContainerError> {
        let files: Vec<&str> = copy_files
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        for file_path in files {
            let source_file = source_dir.join(file_path);
            let target_file = target_dir.join(file_path);

            // Create parent directories if needed
            if let Some(parent) = target_file.parent()
                && !parent.exists()
            {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ContainerError::Other(anyhow!("Failed to create directory {parent:?}: {e}"))
                })?;
            }

            // Copy the file
            if source_file.exists() {
                std::fs::copy(&source_file, &target_file).map_err(|e| {
                    ContainerError::Other(anyhow!(
                        "Failed to copy file {source_file:?} to {target_file:?}: {e}"
                    ))
                })?;
                tracing::info!("Copied file {:?} to worktree", file_path);
            } else {
                return Err(ContainerError::Other(anyhow!(
                    "File {source_file:?} does not exist in the project directory"
                )));
            }
        }
        Ok(())
    }
}

impl LocalContainerService {
    /// Extract the last assistant message from the MsgStore history
    fn extract_last_assistant_message(&self, exec_id: &Uuid) -> Option<String> {
        // Get the MsgStore for this execution
        let msg_stores = self.msg_stores.try_read().ok()?;
        let msg_store = msg_stores.get(exec_id)?;

        // Get the history and scan in reverse for the last assistant message
        let history = msg_store.get_history();

        for msg in history.iter().rev() {
            if let LogMsg::JsonPatch(patch) = msg {
                // Try to extract a NormalizedEntry from the patch
                if let Some((_, entry)) = extract_normalized_entry_from_patch(patch)
                    && matches!(entry.entry_type, NormalizedEntryType::AssistantMessage)
                {
                    let content = entry.content.trim();
                    if !content.is_empty() {
                        // Truncate to reasonable size (4KB as Oracle suggested)
                        const MAX_SUMMARY_LENGTH: usize = 4096;
                        if content.len() > MAX_SUMMARY_LENGTH {
                            return Some(format!("{}...", &content[..MAX_SUMMARY_LENGTH]));
                        }
                        return Some(content.to_string());
                    }
                }
            }
        }

        None
    }

    /// Update the executor session summary with the final assistant message
    async fn update_executor_session_summary(&self, exec_id: &Uuid) -> Result<(), anyhow::Error> {
        // Check if there's an executor session for this execution process
        let session =
            ExecutorSession::find_by_execution_process_id(&self.db.pool, *exec_id).await?;

        if let Some(session) = session {
            // Only update if summary is not already set
            if session.summary.is_none() {
                if let Some(summary) = self.extract_last_assistant_message(exec_id) {
                    ExecutorSession::update_summary(&self.db.pool, *exec_id, &summary).await?;
                } else {
                    tracing::debug!("No assistant message found for execution {}", exec_id);
                }
            }
        }

        Ok(())
    }

    /// If a queued follow-up draft exists for this attempt and nothing is running,
    /// start it immediately and clear the draft.
    async fn try_consume_queued_followup(
        &self,
        ctx: &ExecutionContext,
    ) -> Result<(), ContainerError> {
        // Only consider CodingAgent/cleanup chains; skip DevServer completions
        if matches!(
            ctx.execution_process.run_reason,
            ExecutionProcessRunReason::DevServer
        ) {
            return Ok(());
        }

        // If anything is running for this attempt, bail
        let procs =
            ExecutionProcess::find_by_task_attempt_id(&self.db.pool, ctx.task_attempt.id, false)
                .await?;
        if procs
            .iter()
            .any(|p| matches!(p.status, ExecutionProcessStatus::Running))
        {
            return Ok(());
        }

        // Load draft and ensure it's eligible
        let Some(draft) =
            FollowUpDraft::find_by_task_attempt_id(&self.db.pool, ctx.task_attempt.id).await?
        else {
            return Ok(());
        };

        if !draft.queued || draft.prompt.trim().is_empty() {
            return Ok(());
        }

        // Atomically acquire sending lock; if not acquired, someone else is sending.
        if !FollowUpDraft::try_mark_sending(&self.db.pool, ctx.task_attempt.id)
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }

        // Ensure worktree exists
        let container_ref = self.ensure_container_exists(&ctx.task_attempt).await?;

        // Get session id
        let Some(session_id) = ExecutionProcess::find_latest_session_id_by_task_attempt(
            &self.db.pool,
            ctx.task_attempt.id,
        )
        .await?
        else {
            tracing::warn!(
                "No session id found for attempt {}. Cannot start queued follow-up.",
                ctx.task_attempt.id
            );
            return Ok(());
        };

        // Get last coding agent process to inherit executor profile
        let Some(latest) = ExecutionProcess::find_latest_by_task_attempt_and_run_reason(
            &self.db.pool,
            ctx.task_attempt.id,
            &ExecutionProcessRunReason::CodingAgent,
        )
        .await?
        else {
            tracing::warn!(
                "No prior CodingAgent process for attempt {}. Cannot start queued follow-up.",
                ctx.task_attempt.id
            );
            return Ok(());
        };

        use executors::actions::ExecutorActionType;
        let initial_executor_profile_id = match &latest.executor_action()?.typ {
            ExecutorActionType::CodingAgentInitialRequest(req) => req.executor_profile_id.clone(),
            ExecutorActionType::CodingAgentFollowUpRequest(req) => req.executor_profile_id.clone(),
            _ => {
                tracing::warn!(
                    "Latest process for attempt {} is not a coding agent; skipping queued follow-up",
                    ctx.task_attempt.id
                );
                return Ok(());
            }
        };

        let executor_profile_id = executors::profile::ExecutorProfileId {
            executor: initial_executor_profile_id.executor,
            variant: draft.variant.clone(),
        };

        // Prepare cleanup action
        let cleanup_action = ctx
            .task
            .parent_project(&self.db.pool)
            .await?
            .and_then(|p| p.cleanup_script)
            .map(|script| {
                Box::new(executors::actions::ExecutorAction::new(
                    executors::actions::ExecutorActionType::ScriptRequest(
                        executors::actions::script::ScriptRequest {
                            script,
                            language: executors::actions::script::ScriptRequestLanguage::Bash,
                            context: executors::actions::script::ScriptContext::CleanupScript,
                        },
                    ),
                    None,
                ))
            });

        // Handle images: associate, copy to worktree, canonicalize prompt
        let mut prompt = draft.prompt.clone();
        if let Some(image_ids) = &draft.image_ids {
            // Associate to task
            let _ = TaskImage::associate_many_dedup(&self.db.pool, ctx.task.id, image_ids).await;

            // Copy to worktree and canonicalize
            let worktree_path = std::path::PathBuf::from(&container_ref);
            if let Err(e) = self
                .image_service
                .copy_images_by_ids_to_worktree(&worktree_path, image_ids)
                .await
            {
                tracing::warn!("Failed to copy images to worktree: {}", e);
            } else {
                prompt = ImageService::canonicalise_image_paths(&prompt, &worktree_path);
            }
        }

        let follow_up_request =
            executors::actions::coding_agent_follow_up::CodingAgentFollowUpRequest {
                prompt,
                session_id,
                executor_profile_id,
            };

        let follow_up_action = executors::actions::ExecutorAction::new(
            executors::actions::ExecutorActionType::CodingAgentFollowUpRequest(follow_up_request),
            cleanup_action,
        );

        // Start the execution
        let _ = self
            .start_execution(
                &ctx.task_attempt,
                &follow_up_action,
                &ExecutionProcessRunReason::CodingAgent,
            )
            .await?;

        // Clear the draft to reflect that it has been consumed
        let _ = FollowUpDraft::clear_after_send(&self.db.pool, ctx.task_attempt.id).await;

        Ok(())
    }

    // ==================== Part 1: VIBE Cost Tracking ====================

    /// Extract accumulated token counts from MsgStore history for an execution
    fn extract_token_counts(&self, exec_id: &Uuid) -> Option<(u64, u64)> {
        let msg_stores = self.msg_stores.try_read().ok()?;
        let msg_store = msg_stores.get(exec_id)?;
        let history = msg_store.get_history();

        let mut total_input: u64 = 0;
        let mut total_output: u64 = 0;
        let mut found = false;

        for msg in &history {
            if let LogMsg::TokenCount { input_tokens, output_tokens } = msg {
                total_input += input_tokens;
                total_output += output_tokens;
                found = true;
            }
        }

        if found { Some((total_input, total_output)) } else { None }
    }

    /// Estimate token counts from execution duration and model
    fn estimate_tokens_from_duration(duration_ms: i64, model: &str) -> (i64, i64) {
        // Heuristic: model-specific output tokens/sec
        let output_tokens_per_sec: f64 = if model.contains("claude") {
            80.0
        } else if model.contains("gemini") {
            100.0
        } else if model.contains("codex") || model.contains("gpt") {
            90.0
        } else {
            80.0 // conservative default
        };

        let duration_secs = duration_ms as f64 / 1000.0;
        let estimated_output = (duration_secs * output_tokens_per_sec) as i64;
        // Assume 3:1 input:output ratio
        let estimated_input = estimated_output * 3;
        (estimated_input, estimated_output)
    }

    /// Infer a model name for an executor
    fn infer_model_for_executor(executor_name: &str) -> &'static str {
        match executor_name {
            name if name.contains("claude") => "claude-sonnet-4-20250514",
            name if name.contains("codex") => "codex-mini-latest",
            name if name.contains("gemini") => "gemini-2.5-pro",
            _ => "claude-sonnet-4-20250514", // default fallback
        }
    }

    /// Record VIBE cost for a completed execution
    async fn record_execution_vibe_cost(&self, ctx: &ExecutionContext) {
        let exec_id = ctx.execution_process.id;

        // Try to get actual token counts from MsgStore
        let (input_tokens, output_tokens, source) = if let Some((input, output)) = self.extract_token_counts(&exec_id) {
            (input as i64, output as i64, "actual")
        } else {
            // Fallback: estimate from duration
            let duration_ms = ctx.execution_process.completed_at
                .map(|completed| {
                    (completed - ctx.execution_process.started_at).num_milliseconds()
                })
                .unwrap_or(60_000);
            let model = Self::infer_model_for_executor(&ctx.task_attempt.executor);
            let (input, output) = Self::estimate_tokens_from_duration(duration_ms, model);
            (input, output, "estimated")
        };

        if input_tokens == 0 && output_tokens == 0 {
            return;
        }

        let model = Self::infer_model_for_executor(&ctx.task_attempt.executor);
        let provider = infer_provider(model);

        // Try to find and update the pending zero-amount transaction
        match VibeTransaction::find_by_task_pending(&self.db.pool, ctx.task.id).await {
            Ok(Some(pending_tx)) => {
                // Use VibePricingService to calculate cost
                let pricing_service = VibePricingService::new(self.db.pool.clone());
                let cost_estimate = pricing_service.estimate_cost(model, input_tokens, output_tokens).await;

                let (amount_vibe, cost_cents) = match cost_estimate {
                    Ok(est) => (est.cost_vibe, Some(est.cost_cents)),
                    Err(_) => {
                        // Rough fallback: 1 VIBE per 1000 tokens
                        let total_tokens = input_tokens + output_tokens;
                        ((total_tokens / 1000).max(1), None)
                    }
                };

                let description = format!(
                    "LLM usage ({}): {} ({} in, {} out tokens)",
                    source, model, input_tokens, output_tokens
                );

                if let Err(e) = VibeTransaction::update_cost(
                    &self.db.pool,
                    pending_tx.id,
                    amount_vibe,
                    Some(input_tokens),
                    Some(output_tokens),
                    Some(model),
                    Some(provider),
                    cost_cents,
                    Some(ctx.execution_process.id),
                    Some(ctx.task_attempt.id),
                    Some(&description),
                ).await {
                    tracing::warn!("Failed to update VIBE transaction cost: {}", e);
                } else {
                    tracing::info!(
                        "Updated VIBE cost for task {}: {} VIBE ({} in, {} out tokens, {})",
                        ctx.task.id, amount_vibe, input_tokens, output_tokens, source
                    );
                }
            }
            Ok(None) => {
                // No pending transaction found; create a new one via VibePricingService
                let pricing_service = VibePricingService::new(self.db.pool.clone());
                // We need a source_id; use the task's project_id as a fallback
                if let Err(e) = pricing_service.record_llm_usage(
                    db::models::vibe_transaction::VibeSourceType::Project,
                    ctx.task.project_id,
                    model,
                    input_tokens,
                    output_tokens,
                    Some(ctx.task.id),
                    Some(ctx.task_attempt.id),
                    Some(ctx.execution_process.id),
                ).await {
                    tracing::warn!("Failed to create VIBE transaction: {}", e);
                }
            }
            Err(e) => {
                tracing::warn!("Failed to find pending VIBE transaction: {}", e);
            }
        }
    }

    // ==================== Part 2: Workflow Logs ====================

    /// Create an AgentFlow for a classic pipeline execution
    async fn create_classic_pipeline_flow(&self, ctx: &ExecutionContext) {
        let flow = AgentFlow::create(
            &self.db.pool,
            CreateAgentFlow {
                task_id: ctx.task.id,
                flow_type: FlowType::Custom,
                planner_agent_id: None,
                executor_agent_id: None,
                verifier_agent_id: None,
                flow_config: Some(serde_json::json!({
                    "pipeline": "classic",
                    "executor": ctx.task_attempt.executor,
                    "execution_process_id": ctx.execution_process.id.to_string(),
                })),
                human_approval_required: None,
            },
        ).await;

        match flow {
            Ok(flow) => {
                // Transition to Execution phase (classic pipeline skips Planning)
                if let Err(e) = AgentFlow::transition_to_phase(
                    &self.db.pool,
                    flow.id,
                    AgentPhase::Execution,
                ).await {
                    tracing::warn!("Failed to transition flow to execution phase: {}", e);
                }

                // Emit phase_started event
                if let Err(e) = AgentFlowEvent::emit_phase_started(
                    &self.db.pool,
                    flow.id,
                    "execution",
                    None,
                ).await {
                    tracing::warn!("Failed to emit phase_started event: {}", e);
                }

                // Store flow_id for retrieval at completion
                self.flow_ids.write().await.insert(ctx.execution_process.id, flow.id);

                tracing::info!(
                    "Created classic pipeline flow {} for task {}",
                    flow.id, ctx.task.id
                );
            }
            Err(e) => {
                tracing::warn!("Failed to create classic pipeline flow: {}", e);
            }
        }
    }

    /// Complete an AgentFlow for a classic pipeline execution
    async fn complete_classic_pipeline_flow(&self, ctx: &ExecutionContext) {
        let flow_id = {
            let map = self.flow_ids.read().await;
            map.get(&ctx.execution_process.id).copied()
        };

        let flow_id = match flow_id {
            Some(id) => id,
            None => {
                // Fallback: try to find by task_id
                match AgentFlow::find_by_task(&self.db.pool, ctx.task.id).await {
                    Ok(flows) => {
                        if let Some(flow) = flows.into_iter().find(|f| {
                            f.status != db::models::agent_flow::FlowStatus::Completed
                                && f.status != db::models::agent_flow::FlowStatus::Failed
                        }) {
                            flow.id
                        } else {
                            return;
                        }
                    }
                    Err(_) => return,
                }
            }
        };

        let is_success = matches!(
            ctx.execution_process.status,
            ExecutionProcessStatus::Completed
        );

        if is_success {
            // Emit phase_completed
            if let Err(e) = AgentFlowEvent::create(
                &self.db.pool,
                CreateFlowEvent {
                    agent_flow_id: flow_id,
                    event_type: FlowEventType::PhaseCompleted,
                    event_data: FlowEventPayload::PhaseCompleted {
                        phase: "execution".to_string(),
                        artifacts_produced: vec![],
                    },
                },
            ).await {
                tracing::warn!("Failed to emit phase_completed event: {}", e);
            }

            // Emit flow_completed
            if let Err(e) = AgentFlowEvent::create(
                &self.db.pool,
                CreateFlowEvent {
                    agent_flow_id: flow_id,
                    event_type: FlowEventType::FlowCompleted,
                    event_data: FlowEventPayload::FlowCompleted {
                        verification_score: None,
                        total_artifacts: 0,
                    },
                },
            ).await {
                tracing::warn!("Failed to emit flow_completed event: {}", e);
            }

            // Complete the flow
            if let Err(e) = AgentFlow::complete(&self.db.pool, flow_id, None).await {
                tracing::warn!("Failed to complete agent flow: {}", e);
            }
        } else {
            // Emit flow_failed
            let error_msg = format!(
                "Execution failed with status {:?}, exit_code {:?}",
                ctx.execution_process.status,
                ctx.execution_process.exit_code
            );
            if let Err(e) = AgentFlowEvent::create(
                &self.db.pool,
                CreateFlowEvent {
                    agent_flow_id: flow_id,
                    event_type: FlowEventType::FlowFailed,
                    event_data: FlowEventPayload::FlowFailed {
                        error: error_msg,
                        phase: "execution".to_string(),
                    },
                },
            ).await {
                tracing::warn!("Failed to emit flow_failed event: {}", e);
            }

            // Update flow status to failed
            let _ = sqlx::query(
                "UPDATE agent_flows SET status = 'failed', updated_at = datetime('now', 'subsec') WHERE id = ?1"
            )
            .bind(flow_id)
            .execute(&self.db.pool)
            .await;
        }

        // Cleanup flow_ids entry
        self.flow_ids.write().await.remove(&ctx.execution_process.id);
    }

    // ==================== Part 3: Execution Artifacts ====================

    /// Create execution artifacts from completed execution data
    async fn create_execution_artifacts(&self, ctx: &ExecutionContext) {
        let artifact_service = ArtifactService::new(self.db.clone());
        let exec_id = ctx.execution_process.id;

        // 1. DiffSummary artifact — from compute_diff_stats()
        if let Ok(diff_stats) = self.compute_diff_stats(ctx).await {
            if diff_stats.files_modified > 0 || diff_stats.files_created > 0 || diff_stats.files_deleted > 0 {
                let content = format!(
                    "Files modified: {}, Files created: {}, Files deleted: {}, Additions: +{}, Deletions: -{}",
                    diff_stats.files_modified, diff_stats.files_created, diff_stats.files_deleted,
                    diff_stats.additions, diff_stats.deletions
                );
                if let Err(e) = artifact_service.create_artifact(CreateExecutionArtifact {
                    execution_process_id: Some(exec_id),
                    artifact_type: ArtifactType::DiffSummary,
                    title: "Diff Summary".to_string(),
                    content: Some(content),
                    file_path: None,
                    metadata: Some(serde_json::json!({
                        "files_modified": diff_stats.files_modified,
                        "files_created": diff_stats.files_created,
                        "files_deleted": diff_stats.files_deleted,
                        "additions": diff_stats.additions,
                        "deletions": diff_stats.deletions,
                    })),
                }).await {
                    tracing::warn!("Failed to create DiffSummary artifact: {}", e);
                }
            }
        }

        // 2. ErrorReport artifact — on failure, extract last error from MsgStore
        if matches!(ctx.execution_process.status, ExecutionProcessStatus::Failed) {
            let error_content = self.extract_last_error_message(&exec_id)
                .unwrap_or_else(|| format!(
                    "Execution failed with exit code {:?}",
                    ctx.execution_process.exit_code
                ));
            if let Err(e) = artifact_service.store_error_report(
                exec_id,
                "Execution Error".to_string(),
                error_content,
                Some(serde_json::json!({
                    "exit_code": ctx.execution_process.exit_code,
                    "executor": ctx.task_attempt.executor,
                })),
            ).await {
                tracing::warn!("Failed to create ErrorReport artifact: {}", e);
            }
        }

        // 3. Checkpoint artifact — executor's final summary
        if let Some(summary) = self.extract_last_assistant_message(&exec_id) {
            if let Err(e) = artifact_service.create_artifact(CreateExecutionArtifact {
                execution_process_id: Some(exec_id),
                artifact_type: ArtifactType::Checkpoint,
                title: "Execution Summary".to_string(),
                content: Some(summary),
                file_path: None,
                metadata: Some(serde_json::json!({
                    "executor": ctx.task_attempt.executor,
                    "status": format!("{:?}", ctx.execution_process.status),
                })),
            }).await {
                tracing::warn!("Failed to create Checkpoint artifact: {}", e);
            }
        }
    }

    /// Extract the last error message from the MsgStore history
    fn extract_last_error_message(&self, exec_id: &Uuid) -> Option<String> {
        let msg_stores = self.msg_stores.try_read().ok()?;
        let msg_store = msg_stores.get(exec_id)?;
        let history = msg_store.get_history();

        for msg in history.iter().rev() {
            if let LogMsg::JsonPatch(patch) = msg {
                if let Some((_, entry)) = extract_normalized_entry_from_patch(patch)
                    && matches!(entry.entry_type, NormalizedEntryType::ErrorMessage)
                {
                    let content = entry.content.trim();
                    if !content.is_empty() {
                        return Some(content.to_string());
                    }
                }
            }
        }
        None
    }
}
