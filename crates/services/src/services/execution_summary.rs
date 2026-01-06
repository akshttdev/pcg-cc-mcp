use chrono::Utc;
use db::models::execution_summary::{CompletionStatus, CreateExecutionSummary, ExecutionSummary};
use db::models::execution_process::{ExecutionProcess, ExecutionProcessStatus};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Diff statistics for summary generation
#[derive(Debug, Default)]
pub struct DiffStats {
    pub files_modified: i32,
    pub files_created: i32,
    pub files_deleted: i32,
    pub additions: i32,
    pub deletions: i32,
}

/// Service for generating and managing execution summaries
pub struct ExecutionSummaryService;

impl ExecutionSummaryService {
    /// Generate a summary from execution context and diff stats
    pub fn generate_summary(
        task_attempt_id: Uuid,
        execution_process_id: Option<Uuid>,
        execution_process: Option<&ExecutionProcess>,
        diff_stats: DiffStats,
        executor_name: Option<&str>,
    ) -> CreateExecutionSummary {
        // Determine completion status from execution process
        let (completion_status, error_summary) = if let Some(ep) = execution_process {
            match ep.status {
                ExecutionProcessStatus::Completed => {
                    if ep.exit_code == Some(0) {
                        (CompletionStatus::Full, None)
                    } else {
                        (
                            CompletionStatus::Partial,
                            Some(format!("Exit code: {}", ep.exit_code.unwrap_or(-1))),
                        )
                    }
                }
                ExecutionProcessStatus::Failed => (
                    CompletionStatus::Failed,
                    Some(format!("Execution failed with exit code: {}", ep.exit_code.unwrap_or(-1))),
                ),
                ExecutionProcessStatus::Killed => (
                    CompletionStatus::Partial,
                    Some("Execution was stopped by user".to_string()),
                ),
                ExecutionProcessStatus::Running => (CompletionStatus::Full, None), // Shouldn't happen
            }
        } else {
            (CompletionStatus::Full, None)
        };

        // Calculate execution time
        let execution_time_ms = if let Some(ep) = execution_process {
            if let Some(completed) = ep.completed_at {
                (completed - ep.started_at).num_milliseconds()
            } else {
                (Utc::now() - ep.started_at).num_milliseconds()
            }
        } else {
            0
        };

        // Tools used - include the executor as a tool
        let tools_used = executor_name.map(|name| vec![name.to_string()]);

        CreateExecutionSummary {
            task_attempt_id,
            execution_process_id,
            files_modified: diff_stats.files_modified,
            files_created: diff_stats.files_created,
            files_deleted: diff_stats.files_deleted,
            commands_run: 0,  // Will be enhanced later with log parsing
            commands_failed: 0,
            tools_used,
            completion_status,
            blocker_summary: None,
            error_summary,
            execution_time_ms,
            workflow_tags: None,
        }
    }

    /// Create and save an execution summary
    pub async fn create_summary(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
        execution_process_id: Option<Uuid>,
        execution_process: Option<&ExecutionProcess>,
        diff_stats: DiffStats,
        executor_name: Option<&str>,
    ) -> Result<ExecutionSummary, db::models::execution_summary::ExecutionSummaryError> {
        let data = Self::generate_summary(
            task_attempt_id,
            execution_process_id,
            execution_process,
            diff_stats,
            executor_name,
        );
        ExecutionSummary::create(pool, data).await
    }

    /// Get the latest summary for a task attempt
    pub async fn get_latest_for_attempt(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<Option<ExecutionSummary>, db::models::execution_summary::ExecutionSummaryError> {
        ExecutionSummary::find_by_task_attempt_id(pool, task_attempt_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_summary_empty() {
        let summary = ExecutionSummaryService::generate_summary(
            Uuid::new_v4(),
            None,
            None,
            DiffStats::default(),
            None,
        );

        assert_eq!(summary.files_modified, 0);
        assert!(matches!(summary.completion_status, CompletionStatus::Full));
    }

    #[test]
    fn test_generate_summary_with_diff_stats() {
        let stats = DiffStats {
            files_modified: 5,
            files_created: 2,
            files_deleted: 1,
            additions: 100,
            deletions: 50,
        };

        let summary = ExecutionSummaryService::generate_summary(
            Uuid::new_v4(),
            None,
            None,
            stats,
            Some("Claude"),
        );

        assert_eq!(summary.files_modified, 5);
        assert_eq!(summary.files_created, 2);
        assert_eq!(summary.files_deleted, 1);
        assert_eq!(summary.tools_used, Some(vec!["Claude".to_string()]));
    }
}
