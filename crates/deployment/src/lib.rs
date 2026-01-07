use std::{collections::HashMap, sync::Arc};

use anyhow::Error as AnyhowError;
use async_trait::async_trait;
use axum::response::sse::Event;
#[cfg(feature = "postgres")]
use db::PgDBService;
use db::{
    DBService,
    models::{
        execution_process::{ExecutionProcess, ExecutionProcessRunReason, ExecutionProcessStatus},
        project::{CreateProject, Project},
        task::{Task, TaskStatus},
        task_attempt::{TaskAttempt, TaskAttemptError},
    },
};
use executors::executors::ExecutorError;
use futures::{StreamExt, TryStreamExt};
use git2::Error as Git2Error;
use serde_json::Value;
use services::services::{
    analytics::AnalyticsService,
    approvals::Approvals,
    auth::{AuthError, AuthService},
    config::{Config, ConfigError},
    container::{ContainerError, ContainerService},
    events::{EventError, EventService},
    file_search_cache::FileSearchCache,
    filesystem::{FilesystemError, FilesystemService},
    filesystem_watcher::FilesystemWatcherError,
    git::{GitService, GitServiceError},
    image::{ImageError, ImageService},
    media_pipeline::{MediaPipelineError, MediaPipelineService},
    pr_monitor::PrMonitorService,
    sentry::SentryService,
    topos_scanner::{ToposScannerService, DiscoveredProject},
    worktree_manager::WorktreeError,
};
use sqlx::{Error as SqlxError, types::Uuid};
#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres};
use thiserror::Error;
use tokio::sync::RwLock;
use utils::msg_store::MsgStore;

#[derive(Debug, Error)]
pub enum DeploymentError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error(transparent)]
    Git2(#[from] Git2Error),
    #[error(transparent)]
    GitServiceError(#[from] GitServiceError),
    #[error(transparent)]
    FilesystemWatcherError(#[from] FilesystemWatcherError),
    #[error(transparent)]
    TaskAttempt(#[from] TaskAttemptError),
    #[error(transparent)]
    Container(#[from] ContainerError),
    #[error(transparent)]
    Executor(#[from] ExecutorError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Image(#[from] ImageError),
    #[error(transparent)]
    Filesystem(#[from] FilesystemError),
    #[error(transparent)]
    Worktree(#[from] WorktreeError),
    #[error(transparent)]
    Event(#[from] EventError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    MediaPipeline(#[from] MediaPipelineError),
    #[error(transparent)]
    Other(#[from] AnyhowError),
}

#[async_trait]
pub trait Deployment: Clone + Send + Sync + 'static {
    async fn new() -> Result<Self, DeploymentError>;

    fn user_id(&self) -> &str;

    fn shared_types() -> Vec<String>;

    fn config(&self) -> &Arc<RwLock<Config>>;

    fn sentry(&self) -> &SentryService;

    fn db(&self) -> &DBService;

    #[cfg(feature = "postgres")]
    fn pg_pool(&self) -> Option<&Pool<Postgres>> {
        None // Default implementation returns None
    }

    fn analytics(&self) -> &Option<AnalyticsService>;

    fn container(&self) -> &impl ContainerService;

    fn auth(&self) -> &AuthService;

    fn git(&self) -> &GitService;

    fn image(&self) -> &ImageService;

    fn filesystem(&self) -> &FilesystemService;

    fn msg_stores(&self) -> &Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>>;

    fn events(&self) -> &EventService;

    fn file_search_cache(&self) -> &Arc<FileSearchCache>;

    fn approvals(&self) -> &Approvals;

    fn media_pipeline(&self) -> &MediaPipelineService;

    async fn update_sentry_scope(&self) -> Result<(), DeploymentError> {
        let user_id = self.user_id();
        let config = self.config().read().await;
        let username = config.github.username.as_deref();
        let email = config.github.primary_email.as_deref();

        self.sentry().update_scope(user_id, username, email).await;

        Ok(())
    }

    async fn spawn_pr_monitor_service(&self) -> tokio::task::JoinHandle<()> {
        let db = self.db().clone();
        let config = self.config().clone();
        PrMonitorService::spawn(db, config).await
    }

    async fn track_if_analytics_allowed(&self, event_name: &str, properties: Value) {
        let analytics_enabled = self.config().read().await.analytics_enabled;
        // Only skip tracking if user explicitly opted out (Some(false))
        // Send for None (undecided) and Some(true) (opted in)
        if analytics_enabled != Some(false)
            && let Some(analytics) = self.analytics()
        {
            analytics.track_event(self.user_id(), event_name, Some(properties.clone()));
        }
    }

    /// Cleanup executions marked as running in the db, call at startup
    async fn cleanup_orphan_executions(&self) -> Result<(), DeploymentError> {
        let running_processes = ExecutionProcess::find_running(&self.db().pool).await?;
        for process in running_processes {
            tracing::info!(
                "Found orphaned execution process {} for task attempt {}",
                process.id,
                process.task_attempt_id
            );
            // Update the execution process status first
            if let Err(e) = ExecutionProcess::update_completion(
                &self.db().pool,
                process.id,
                ExecutionProcessStatus::Failed,
                None, // No exit code for orphaned processes
            )
            .await
            {
                tracing::error!(
                    "Failed to update orphaned execution process {} status: {}",
                    process.id,
                    e
                );
                continue;
            }
            // Capture after-head commit OID (best-effort)
            if let Ok(Some(task_attempt)) =
                TaskAttempt::find_by_id(&self.db().pool, process.task_attempt_id).await
                && let Some(container_ref) = task_attempt.container_ref
            {
                let wt = std::path::PathBuf::from(container_ref);
                if let Ok(head) = self.git().get_head_info(&wt) {
                    let _ = ExecutionProcess::update_after_head_commit(
                        &self.db().pool,
                        process.id,
                        &head.oid,
                    )
                    .await;
                }
            }
            // Process marked as failed
            tracing::info!("Marked orphaned execution process {} as failed", process.id);
            // Update task status to InReview for coding agent and setup script failures
            if matches!(
                process.run_reason,
                ExecutionProcessRunReason::CodingAgent
                    | ExecutionProcessRunReason::SetupScript
                    | ExecutionProcessRunReason::CleanupScript
            ) && let Ok(Some(task_attempt)) =
                TaskAttempt::find_by_id(&self.db().pool, process.task_attempt_id).await
                && let Ok(Some(task)) = task_attempt.parent_task(&self.db().pool).await
                && let Err(e) =
                    Task::update_status(&self.db().pool, task.id, TaskStatus::InReview).await
            {
                tracing::error!(
                    "Failed to update task status to InReview for orphaned attempt: {}",
                    e
                );
            }
        }
        Ok(())
    }

    /// Backfill before_head_commit for legacy execution processes.
    /// Rules:
    /// - If a process has after_head_commit and missing before_head_commit,
    ///   then set before_head_commit to the previous process's after_head_commit.
    /// - If there is no previous process, set before_head_commit to the base branch commit.
    async fn backfill_before_head_commits(&self) -> Result<(), DeploymentError> {
        let pool = &self.db().pool;
        let rows = ExecutionProcess::list_missing_before_context(pool).await?;
        for row in rows {
            // Skip if no after commit at all (shouldn't happen due to WHERE)
            // Prefer previous process after-commit if present
            let mut before = row.prev_after_head_commit.clone();

            // Fallback to base branch commit OID
            if before.is_none() {
                let repo_path =
                    std::path::Path::new(row.git_repo_path.as_deref().unwrap_or_default());
                match self
                    .git()
                    .get_branch_oid(repo_path, row.base_branch.as_str())
                {
                    Ok(oid) => before = Some(oid),
                    Err(e) => {
                        tracing::warn!(
                            "Backfill: Failed to resolve base branch OID for attempt {} (branch {}): {}",
                            row.task_attempt_id,
                            row.base_branch,
                            e
                        );
                    }
                }
            }

            if let Some(before_oid) = before
                && let Err(e) =
                    ExecutionProcess::update_before_head_commit(pool, row.id, &before_oid).await
            {
                tracing::warn!(
                    "Backfill: Failed to update before_head_commit for process {}: {}",
                    row.id,
                    e
                );
            }
        }

        Ok(())
    }

    /// Trigger background auto-setup of default projects for new users
    async fn trigger_auto_project_setup(&self) {
        // soft timeout to give the filesystem search a chance to complete
        let soft_timeout_ms = 2_000;
        // hard timeout to ensure the background task doesn't run indefinitely
        let hard_timeout_ms = 2_300;
        let project_count = Project::count(&self.db().pool).await.unwrap_or(0);

        // Only proceed if no projects exist
        if project_count == 0 {
            // Discover local git repositories
            if let Ok(repos) = self
                .filesystem()
                .list_common_git_repos(soft_timeout_ms, hard_timeout_ms, Some(4))
                .await
            {
                // Take first 3 repositories and create projects
                for repo in repos.into_iter().take(3) {
                    // Generate clean project name from path
                    let project_name = repo.name;

                    let create_data = CreateProject {
                        name: project_name,
                        git_repo_path: repo.path.to_string_lossy().to_string(),
                        use_existing_repo: true,
                        setup_script: None,
                        dev_script: None,
                        cleanup_script: None,
                        copy_files: None,
                    };
                    // Ensure existing repo has a main branch if it's empty
                    if let Err(e) = self.git().ensure_main_branch_exists(&repo.path) {
                        tracing::error!("Failed to ensure main branch exists: {}", e);
                        continue;
                    }

                    // Create project (ignore individual failures)
                    let project_id = Uuid::new_v4();
                    match Project::create(&self.db().pool, &create_data, project_id).await {
                        Ok(project) => {
                            tracing::info!(
                                "Auto-created project '{}' from {}",
                                create_data.name,
                                create_data.git_repo_path
                            );

                            // Track project creation event
                            self.track_if_analytics_allowed(
                                "project_created",
                                serde_json::json!({
                                    "project_id": project.id.to_string(),
                                    "use_existing_repo": create_data.use_existing_repo,
                                    "has_setup_script": create_data.setup_script.is_some(),
                                    "has_dev_script": create_data.dev_script.is_some(),
                                    "source": "auto_setup",
                                }),
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to auto-create project '{}': {}",
                                create_data.name,
                                e
                            );
                        }
                    }
                }
            }
        }
    }

    async fn stream_events(
        &self,
    ) -> futures::stream::BoxStream<'static, Result<Event, std::io::Error>> {
        self.events()
            .msg_store()
            .history_plus_stream()
            .map_ok(|m| m.to_sse_event())
            .boxed()
    }

    /// Sync projects from the topos directory on startup
    /// This runs every time the server starts and ensures projects from topos are loaded
    async fn sync_from_topos(&self) {
        if !ToposScannerService::is_configured() {
            tracing::debug!("TOPOS_DIR not configured, skipping topos sync");
            return;
        }

        let scanner = ToposScannerService::new();

        match scanner.discover_projects().await {
            Ok(projects) => {
                tracing::info!("Discovered {} projects in topos directory", projects.len());

                for project in projects {
                    self.sync_topos_project(project).await;
                }
            }
            Err(e) => {
                tracing::warn!("Failed to discover topos projects: {}", e);
            }
        }
    }

    /// Sync a single discovered project from topos
    async fn sync_topos_project(&self, discovered: DiscoveredProject) {
        let scanner = ToposScannerService::new();

        // Skip if no git repo found
        let git_repo_path = match &discovered.git_repo_path {
            Some(path) => path,
            None => {
                tracing::debug!(
                    "Skipping topos project '{}' - no git repository found",
                    discovered.name
                );
                return;
            }
        };

        let repo_path_str = git_repo_path.to_string_lossy().to_string();

        // Check if project already exists with this repo path
        match Project::find_by_git_repo_path(&self.db().pool, &repo_path_str).await {
            Ok(Some(existing)) => {
                tracing::debug!(
                    "Topos project '{}' already exists in database (id: {})",
                    discovered.name,
                    existing.id
                );
                return;
            }
            Ok(None) => {
                // Project doesn't exist, create it
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to check for existing project '{}': {}",
                    discovered.name,
                    e
                );
                return;
            }
        }

        // Ensure main branch exists
        if let Err(e) = self.git().ensure_main_branch_exists(git_repo_path) {
            tracing::warn!(
                "Failed to ensure main branch for '{}': {}",
                discovered.name,
                e
            );
            return;
        }

        // Determine scripts from metadata
        let (setup_script, dev_script) = match &discovered.metadata {
            Some(meta) => (meta.setup_script.clone(), meta.dev_script.clone()),
            None => (None, None),
        };

        // Create the project
        let create_data = CreateProject {
            name: discovered.name.clone(),
            git_repo_path: repo_path_str,
            use_existing_repo: true,
            setup_script,
            dev_script,
            cleanup_script: None,
            copy_files: None,
        };

        let project_id = Uuid::new_v4();
        match Project::create(&self.db().pool, &create_data, project_id).await {
            Ok(project) => {
                tracing::info!(
                    "Created project '{}' from topos (id: {})",
                    discovered.name,
                    project.id
                );

                // Ensure project has standard topos directory structure
                if let Err(e) = scanner.ensure_project_structure(&discovered.project_path) {
                    tracing::warn!(
                        "Failed to ensure topos structure for '{}': {}",
                        discovered.name,
                        e
                    );
                }

                // Track analytics
                self.track_if_analytics_allowed(
                    "project_created",
                    serde_json::json!({
                        "project_id": project.id.to_string(),
                        "source": "topos_sync",
                        "has_metadata": discovered.metadata.is_some(),
                    }),
                )
                .await;
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to create topos project '{}': {}",
                    discovered.name,
                    e
                );
            }
        }
    }
}
