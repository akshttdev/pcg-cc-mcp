//! Orchestration State Store
//!
//! Thread-safe state management for orchestration flows.
//! Follows immutable update semantics similar to Zustand pattern.

use std::{collections::HashMap, sync::Arc};

use db::models::orchestration_task::{OrchestrationTask, OrchestrationTaskStatus};
use tokio::sync::RwLock;

/// State for a single orchestration flow
#[derive(Debug, Clone, Default)]
pub struct OrchestrationFlowState {
    /// Flow ID
    pub flow_id: String,
    /// All tasks in this flow, keyed by task ID
    pub tasks: HashMap<String, TaskState>,
    /// When the flow started (epoch ms)
    pub start_time_ms: Option<i64>,
    /// When the flow ended (epoch ms)
    pub end_time_ms: Option<i64>,
    /// Maximum concurrent tasks allowed
    pub max_concurrent: usize,
    /// Currently running task count
    pub running_count: usize,
}

/// Lightweight state for a single task (in-memory view)
#[derive(Debug, Clone)]
pub struct TaskState {
    pub id: String,
    pub status: OrchestrationTaskStatus,
    pub description: String,
    pub start_time_ms: Option<i64>,
    pub end_time_ms: Option<i64>,
    pub is_concurrency_safe: bool,
}

impl From<&OrchestrationTask> for TaskState {
    fn from(task: &OrchestrationTask) -> Self {
        Self {
            id: task.id.clone(),
            status: task.status,
            description: task.description.clone(),
            start_time_ms: task.start_time_ms,
            end_time_ms: task.end_time_ms,
            is_concurrency_safe: task.is_concurrency_safe,
        }
    }
}

impl OrchestrationFlowState {
    /// Create a new empty flow state
    pub fn new(flow_id: impl Into<String>, max_concurrent: usize) -> Self {
        Self {
            flow_id: flow_id.into(),
            tasks: HashMap::new(),
            start_time_ms: None,
            end_time_ms: None,
            max_concurrent,
            running_count: 0,
        }
    }

    /// Add or update a task
    pub fn upsert_task(&mut self, task: TaskState) {
        // Update running count if status changed
        if let Some(existing) = self.tasks.get(&task.id) {
            if existing.status == OrchestrationTaskStatus::Running
                && task.status != OrchestrationTaskStatus::Running
            {
                self.running_count = self.running_count.saturating_sub(1);
            } else if existing.status != OrchestrationTaskStatus::Running
                && task.status == OrchestrationTaskStatus::Running
            {
                self.running_count += 1;
            }
        } else if task.status == OrchestrationTaskStatus::Running {
            self.running_count += 1;
        }

        self.tasks.insert(task.id.clone(), task);
    }

    /// Update task status
    pub fn update_task_status(&mut self, task_id: &str, status: OrchestrationTaskStatus) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            // Update running count
            if task.status == OrchestrationTaskStatus::Running
                && status != OrchestrationTaskStatus::Running
            {
                self.running_count = self.running_count.saturating_sub(1);
            } else if task.status != OrchestrationTaskStatus::Running
                && status == OrchestrationTaskStatus::Running
            {
                self.running_count += 1;
            }

            task.status = status;

            // Set timestamps
            if status == OrchestrationTaskStatus::Running && task.start_time_ms.is_none() {
                task.start_time_ms = Some(chrono::Utc::now().timestamp_millis());
            }
            if status.is_terminal() {
                task.end_time_ms = Some(chrono::Utc::now().timestamp_millis());
            }
        }
    }

    /// Check if we can start more tasks (under concurrency limit)
    pub fn can_start_more(&self) -> bool {
        self.running_count < self.max_concurrent
    }

    /// Get pending tasks that can be started
    pub fn get_startable_tasks(&self) -> Vec<&TaskState> {
        if !self.can_start_more() {
            return Vec::new();
        }

        let slots_available = self.max_concurrent - self.running_count;

        self.tasks
            .values()
            .filter(|t| t.status == OrchestrationTaskStatus::Pending)
            .take(slots_available)
            .collect()
    }

    /// Check if all tasks are complete
    pub fn all_complete(&self) -> bool {
        !self.tasks.is_empty() && self.tasks.values().all(|t| t.status.is_terminal())
    }

    /// Count tasks by status
    pub fn count_by_status(&self, status: OrchestrationTaskStatus) -> usize {
        self.tasks.values().filter(|t| t.status == status).count()
    }

    /// Get total duration (if flow has ended)
    pub fn total_duration_ms(&self) -> Option<i64> {
        match (self.start_time_ms, self.end_time_ms) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}

/// Thread-safe wrapper around flow state
#[derive(Debug, Clone)]
pub struct OrchestrationStateStore {
    state: Arc<RwLock<OrchestrationFlowState>>,
}

impl OrchestrationStateStore {
    /// Create a new state store
    pub fn new(flow_id: impl Into<String>, max_concurrent: usize) -> Self {
        Self {
            state: Arc::new(RwLock::new(OrchestrationFlowState::new(
                flow_id,
                max_concurrent,
            ))),
        }
    }

    /// Create from existing state
    pub fn from_state(state: OrchestrationFlowState) -> Self {
        Self {
            state: Arc::new(RwLock::new(state)),
        }
    }

    /// Get a snapshot of the current state
    pub async fn get(&self) -> OrchestrationFlowState {
        self.state.read().await.clone()
    }

    /// Update state using a closure (immutable update pattern)
    pub async fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut OrchestrationFlowState),
    {
        let mut state = self.state.write().await;
        f(&mut state);
    }

    /// Add or update a task
    pub async fn upsert_task(&self, task: TaskState) {
        self.update(|state| state.upsert_task(task)).await;
    }

    /// Update task status
    pub async fn update_task_status(&self, task_id: &str, status: OrchestrationTaskStatus) {
        self.update(|state| state.update_task_status(task_id, status))
            .await;
    }

    /// Check if we can start more tasks
    pub async fn can_start_more(&self) -> bool {
        self.state.read().await.can_start_more()
    }

    /// Get startable task IDs
    pub async fn get_startable_task_ids(&self) -> Vec<String> {
        self.state
            .read()
            .await
            .get_startable_tasks()
            .iter()
            .map(|t| t.id.clone())
            .collect()
    }

    /// Check if all tasks are complete
    pub async fn all_complete(&self) -> bool {
        self.state.read().await.all_complete()
    }

    /// Mark flow as started
    pub async fn mark_started(&self) {
        self.update(|state| {
            state.start_time_ms = Some(chrono::Utc::now().timestamp_millis());
        })
        .await;
    }

    /// Mark flow as ended
    pub async fn mark_ended(&self) {
        self.update(|state| {
            state.end_time_ms = Some(chrono::Utc::now().timestamp_millis());
        })
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, status: OrchestrationTaskStatus, safe: bool) -> TaskState {
        TaskState {
            id: id.to_string(),
            status,
            description: format!("Task {}", id),
            start_time_ms: None,
            end_time_ms: None,
            is_concurrency_safe: safe,
        }
    }

    #[test]
    fn test_flow_state_new() {
        let state = OrchestrationFlowState::new("flow-1", 5);
        assert_eq!(state.flow_id, "flow-1");
        assert_eq!(state.max_concurrent, 5);
        assert_eq!(state.running_count, 0);
        assert!(state.tasks.is_empty());
    }

    #[test]
    fn test_upsert_task() {
        let mut state = OrchestrationFlowState::new("flow-1", 5);

        let task = make_task("t1", OrchestrationTaskStatus::Pending, true);
        state.upsert_task(task);

        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.running_count, 0);

        // Update to running
        let running_task = make_task("t1", OrchestrationTaskStatus::Running, true);
        state.upsert_task(running_task);

        assert_eq!(state.tasks.len(), 1);
        assert_eq!(state.running_count, 1);
    }

    #[test]
    fn test_update_task_status() {
        let mut state = OrchestrationFlowState::new("flow-1", 5);
        state.upsert_task(make_task("t1", OrchestrationTaskStatus::Pending, true));

        state.update_task_status("t1", OrchestrationTaskStatus::Running);
        assert_eq!(state.running_count, 1);

        state.update_task_status("t1", OrchestrationTaskStatus::Completed);
        assert_eq!(state.running_count, 0);
    }

    #[test]
    fn test_can_start_more() {
        let mut state = OrchestrationFlowState::new("flow-1", 2);

        assert!(state.can_start_more());

        state.upsert_task(make_task("t1", OrchestrationTaskStatus::Running, true));
        assert!(state.can_start_more());

        state.upsert_task(make_task("t2", OrchestrationTaskStatus::Running, true));
        assert!(!state.can_start_more());

        state.update_task_status("t1", OrchestrationTaskStatus::Completed);
        assert!(state.can_start_more());
    }

    #[test]
    fn test_get_startable_tasks() {
        let mut state = OrchestrationFlowState::new("flow-1", 2);

        state.upsert_task(make_task("t1", OrchestrationTaskStatus::Pending, true));
        state.upsert_task(make_task("t2", OrchestrationTaskStatus::Pending, true));
        state.upsert_task(make_task("t3", OrchestrationTaskStatus::Pending, true));

        let startable = state.get_startable_tasks();
        assert_eq!(startable.len(), 2); // Limited by max_concurrent

        state.upsert_task(make_task("t1", OrchestrationTaskStatus::Running, true));
        let startable = state.get_startable_tasks();
        assert_eq!(startable.len(), 1); // One slot remaining
    }

    #[test]
    fn test_all_complete() {
        let mut state = OrchestrationFlowState::new("flow-1", 5);

        // Empty is not complete
        assert!(!state.all_complete());

        state.upsert_task(make_task("t1", OrchestrationTaskStatus::Completed, true));
        assert!(state.all_complete());

        state.upsert_task(make_task("t2", OrchestrationTaskStatus::Running, true));
        assert!(!state.all_complete());

        state.update_task_status("t2", OrchestrationTaskStatus::Failed);
        assert!(state.all_complete()); // Failed is terminal
    }

    #[test]
    fn test_count_by_status() {
        let mut state = OrchestrationFlowState::new("flow-1", 5);

        state.upsert_task(make_task("t1", OrchestrationTaskStatus::Pending, true));
        state.upsert_task(make_task("t2", OrchestrationTaskStatus::Running, true));
        state.upsert_task(make_task("t3", OrchestrationTaskStatus::Completed, true));
        state.upsert_task(make_task("t4", OrchestrationTaskStatus::Completed, true));

        assert_eq!(state.count_by_status(OrchestrationTaskStatus::Pending), 1);
        assert_eq!(state.count_by_status(OrchestrationTaskStatus::Running), 1);
        assert_eq!(state.count_by_status(OrchestrationTaskStatus::Completed), 2);
        assert_eq!(state.count_by_status(OrchestrationTaskStatus::Failed), 0);
    }

    #[tokio::test]
    async fn test_store_async_operations() {
        let store = OrchestrationStateStore::new("flow-1", 3);

        store
            .upsert_task(make_task("t1", OrchestrationTaskStatus::Pending, true))
            .await;
        store
            .upsert_task(make_task("t2", OrchestrationTaskStatus::Pending, true))
            .await;

        assert!(store.can_start_more().await);

        let startable = store.get_startable_task_ids().await;
        assert_eq!(startable.len(), 2);

        store
            .update_task_status("t1", OrchestrationTaskStatus::Running)
            .await;
        store
            .update_task_status("t2", OrchestrationTaskStatus::Completed)
            .await;

        assert!(!store.all_complete().await);

        store
            .update_task_status("t1", OrchestrationTaskStatus::Completed)
            .await;
        assert!(store.all_complete().await);
    }

    #[tokio::test]
    async fn test_store_mark_times() {
        let store = OrchestrationStateStore::new("flow-1", 5);

        store.mark_started().await;
        let state = store.get().await;
        assert!(state.start_time_ms.is_some());
        assert!(state.end_time_ms.is_none());

        store.mark_ended().await;
        let state = store.get().await;
        assert!(state.end_time_ms.is_some());
        assert!(state.total_duration_ms().is_some());
    }
}
