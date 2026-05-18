//! Orchestration Events for SSE streaming
//!
//! Events emitted during orchestration task execution for real-time UI updates.

use db::models::orchestration_task::OrchestrationTaskType;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Events emitted during orchestration task execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum OrchestrationEvent {
    /// Task has been queued for execution
    TaskQueued {
        task_id: String,
        task_type: OrchestrationTaskType,
        description: String,
        position: i32,
    },

    /// Task has started executing
    TaskStarted {
        task_id: String,
        task_type: OrchestrationTaskType,
    },

    /// Task is making progress (intermediate update)
    TaskProgress {
        task_id: String,
        /// Human-readable progress message
        progress: String,
        /// Optional percentage (0-100)
        #[serde(skip_serializing_if = "Option::is_none")]
        percent: Option<u8>,
    },

    /// Task completed successfully
    TaskCompleted {
        task_id: String,
        /// Task output/result
        output: String,
        /// Duration in milliseconds
        duration_ms: i64,
    },

    /// Task failed with an error
    TaskFailed {
        task_id: String,
        /// Error message
        error: String,
        /// Duration in milliseconds
        duration_ms: Option<i64>,
    },

    /// Task was manually killed
    TaskKilled {
        task_id: String,
        /// Duration in milliseconds before kill
        duration_ms: Option<i64>,
    },

    /// All tasks in the flow have completed
    FlowCompleted {
        flow_id: String,
        /// Total duration in milliseconds
        total_duration_ms: i64,
        /// Number of tasks that completed successfully
        completed_count: i32,
        /// Number of tasks that failed
        failed_count: i32,
    },

    /// Flow-level error occurred
    FlowError { flow_id: String, error: String },
}

impl OrchestrationEvent {
    /// Create a TaskQueued event
    pub fn task_queued(
        task_id: impl Into<String>,
        task_type: OrchestrationTaskType,
        description: impl Into<String>,
        position: i32,
    ) -> Self {
        Self::TaskQueued {
            task_id: task_id.into(),
            task_type,
            description: description.into(),
            position,
        }
    }

    /// Create a TaskStarted event
    pub fn task_started(task_id: impl Into<String>, task_type: OrchestrationTaskType) -> Self {
        Self::TaskStarted {
            task_id: task_id.into(),
            task_type,
        }
    }

    /// Create a TaskProgress event
    pub fn task_progress(
        task_id: impl Into<String>,
        progress: impl Into<String>,
        percent: Option<u8>,
    ) -> Self {
        Self::TaskProgress {
            task_id: task_id.into(),
            progress: progress.into(),
            percent,
        }
    }

    /// Create a TaskCompleted event
    pub fn task_completed(
        task_id: impl Into<String>,
        output: impl Into<String>,
        duration_ms: i64,
    ) -> Self {
        Self::TaskCompleted {
            task_id: task_id.into(),
            output: output.into(),
            duration_ms,
        }
    }

    /// Create a TaskFailed event
    pub fn task_failed(
        task_id: impl Into<String>,
        error: impl Into<String>,
        duration_ms: Option<i64>,
    ) -> Self {
        Self::TaskFailed {
            task_id: task_id.into(),
            error: error.into(),
            duration_ms,
        }
    }

    /// Create a TaskKilled event
    pub fn task_killed(task_id: impl Into<String>, duration_ms: Option<i64>) -> Self {
        Self::TaskKilled {
            task_id: task_id.into(),
            duration_ms,
        }
    }

    /// Create a FlowCompleted event
    pub fn flow_completed(
        flow_id: impl Into<String>,
        total_duration_ms: i64,
        completed_count: i32,
        failed_count: i32,
    ) -> Self {
        Self::FlowCompleted {
            flow_id: flow_id.into(),
            total_duration_ms,
            completed_count,
            failed_count,
        }
    }

    /// Create a FlowError event
    pub fn flow_error(flow_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::FlowError {
            flow_id: flow_id.into(),
            error: error.into(),
        }
    }

    /// Get the task_id if this is a task-level event
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::TaskQueued { task_id, .. }
            | Self::TaskStarted { task_id, .. }
            | Self::TaskProgress { task_id, .. }
            | Self::TaskCompleted { task_id, .. }
            | Self::TaskFailed { task_id, .. }
            | Self::TaskKilled { task_id, .. } => Some(task_id),
            Self::FlowCompleted { .. } | Self::FlowError { .. } => None,
        }
    }

    /// Check if this is a terminal event for a task
    pub fn is_task_terminal(&self) -> bool {
        matches!(
            self,
            Self::TaskCompleted { .. } | Self::TaskFailed { .. } | Self::TaskKilled { .. }
        )
    }

    /// Convert to SSE data format
    pub fn to_sse_data(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization_tagged() {
        let event = OrchestrationEvent::TaskStarted {
            task_id: "task-1".to_string(),
            task_type: OrchestrationTaskType::LocalBash,
        };
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"type\":\"task_started\""));
        assert!(json.contains("\"task_id\":\"task-1\""));
        assert!(json.contains("\"task_type\":\"local_bash\""));
    }

    #[test]
    fn test_event_serialization_with_optional() {
        let event = OrchestrationEvent::TaskProgress {
            task_id: "task-1".to_string(),
            progress: "Processing...".to_string(),
            percent: Some(50),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"percent\":50"));

        // Without percent
        let event_no_percent = OrchestrationEvent::TaskProgress {
            task_id: "task-1".to_string(),
            progress: "Working...".to_string(),
            percent: None,
        };
        let json2 = serde_json::to_string(&event_no_percent).unwrap();
        assert!(!json2.contains("percent"));
    }

    #[test]
    fn test_flow_completed_serialization() {
        let event = OrchestrationEvent::FlowCompleted {
            flow_id: "flow-123".to_string(),
            total_duration_ms: 5000,
            completed_count: 3,
            failed_count: 1,
        };
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"type\":\"flow_completed\""));
        assert!(json.contains("\"flow_id\":\"flow-123\""));
        assert!(json.contains("\"total_duration_ms\":5000"));
        assert!(json.contains("\"completed_count\":3"));
        assert!(json.contains("\"failed_count\":1"));
    }

    #[test]
    fn test_task_id_extraction() {
        let task_event = OrchestrationEvent::task_started("task-abc", OrchestrationTaskType::Dream);
        assert_eq!(task_event.task_id(), Some("task-abc"));

        let flow_event = OrchestrationEvent::flow_completed("flow-1", 1000, 2, 0);
        assert_eq!(flow_event.task_id(), None);
    }

    #[test]
    fn test_is_terminal() {
        assert!(OrchestrationEvent::task_completed("t", "done", 100).is_task_terminal());
        assert!(OrchestrationEvent::task_failed("t", "err", None).is_task_terminal());
        assert!(OrchestrationEvent::task_killed("t", None).is_task_terminal());

        assert!(
            !OrchestrationEvent::task_started("t", OrchestrationTaskType::LocalBash)
                .is_task_terminal()
        );
        assert!(!OrchestrationEvent::task_progress("t", "...", None).is_task_terminal());
    }

    #[test]
    fn test_builder_methods() {
        let event = OrchestrationEvent::task_queued(
            "id-1",
            OrchestrationTaskType::RemoteAgent,
            "Call external API",
            2,
        );

        if let OrchestrationEvent::TaskQueued {
            task_id,
            task_type,
            description,
            position,
        } = event
        {
            assert_eq!(task_id, "id-1");
            assert_eq!(task_type, OrchestrationTaskType::RemoteAgent);
            assert_eq!(description, "Call external API");
            assert_eq!(position, 2);
        } else {
            panic!("Expected TaskQueued variant");
        }
    }

    #[test]
    fn test_to_sse_data() {
        let event = OrchestrationEvent::task_progress("task-1", "Loading", Some(25));
        let sse_data = event.to_sse_data();

        assert!(sse_data.starts_with('{'));
        assert!(sse_data.ends_with('}'));
        assert!(sse_data.contains("task_progress"));
    }
}
