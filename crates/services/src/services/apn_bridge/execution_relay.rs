//! Execution Relay - Streams execution logs and progress from remote nodes
//!
//! Handles bidirectional communication during remote task execution,
//! including log streaming, progress updates, and result collection.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;
use chrono::Utc;

use super::types::*;

/// Execution relay service for remote task monitoring
pub struct ExecutionRelay {
    /// Active execution streams
    active_executions: Arc<RwLock<HashMap<Uuid, ActiveExecution>>>,
    /// Log broadcaster for each execution
    log_channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<ExecutionLogChunk>>>>,
    /// Progress broadcaster for each execution
    progress_channels: Arc<RwLock<HashMap<Uuid, broadcast::Sender<ExecutionProgress>>>>,
}

struct ActiveExecution {
    task_id: Uuid,
    execution_process_id: Uuid,
    executor_node: String,
    started_at: chrono::DateTime<Utc>,
    last_progress: Option<ExecutionProgress>,
    log_count: usize,
}

impl ExecutionRelay {
    pub fn new() -> Self {
        Self {
            active_executions: Arc::new(RwLock::new(HashMap::new())),
            log_channels: Arc::new(RwLock::new(HashMap::new())),
            progress_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start tracking an execution
    pub async fn start_execution(
        &self,
        task_id: Uuid,
        execution_process_id: Uuid,
        executor_node: String,
    ) -> anyhow::Result<()> {
        let execution = ActiveExecution {
            task_id,
            execution_process_id,
            executor_node,
            started_at: Utc::now(),
            last_progress: None,
            log_count: 0,
        };

        // Create broadcast channels
        let (log_tx, _) = broadcast::channel(1024);
        let (progress_tx, _) = broadcast::channel(256);

        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_process_id, execution);
        }

        {
            let mut log_channels = self.log_channels.write().await;
            log_channels.insert(execution_process_id, log_tx);
        }

        {
            let mut progress_channels = self.progress_channels.write().await;
            progress_channels.insert(execution_process_id, progress_tx);
        }

        tracing::info!(
            "Started tracking execution {} for task {}",
            execution_process_id,
            task_id
        );

        Ok(())
    }

    /// Stop tracking an execution
    pub async fn stop_execution(&self, execution_process_id: Uuid) {
        {
            let mut executions = self.active_executions.write().await;
            executions.remove(&execution_process_id);
        }

        {
            let mut log_channels = self.log_channels.write().await;
            log_channels.remove(&execution_process_id);
        }

        {
            let mut progress_channels = self.progress_channels.write().await;
            progress_channels.remove(&execution_process_id);
        }

        tracing::info!("Stopped tracking execution {}", execution_process_id);
    }

    /// Subscribe to logs for an execution
    pub async fn subscribe_logs(&self, execution_process_id: Uuid) -> Option<broadcast::Receiver<ExecutionLogChunk>> {
        let log_channels = self.log_channels.read().await;
        log_channels.get(&execution_process_id).map(|tx| tx.subscribe())
    }

    /// Subscribe to progress for an execution
    pub async fn subscribe_progress(&self, execution_process_id: Uuid) -> Option<broadcast::Receiver<ExecutionProgress>> {
        let progress_channels = self.progress_channels.read().await;
        progress_channels.get(&execution_process_id).map(|tx| tx.subscribe())
    }

    /// Relay a log chunk from a remote execution
    pub async fn relay_log(&self, log: ExecutionLogChunk) -> anyhow::Result<()> {
        let log_channels = self.log_channels.read().await;

        if let Some(tx) = log_channels.get(&log.execution_process_id) {
            // Ignore send errors (no subscribers)
            let _ = tx.send(log.clone());
        }

        // Update execution stats
        {
            let mut executions = self.active_executions.write().await;
            if let Some(exec) = executions.get_mut(&log.execution_process_id) {
                exec.log_count += 1;
            }
        }

        Ok(())
    }

    /// Relay a progress update from a remote execution
    pub async fn relay_progress(&self, progress: ExecutionProgress) -> anyhow::Result<()> {
        let progress_channels = self.progress_channels.read().await;

        if let Some(tx) = progress_channels.get(&progress.execution_process_id) {
            let _ = tx.send(progress.clone());
        }

        // Update execution state
        {
            let mut executions = self.active_executions.write().await;
            if let Some(exec) = executions.get_mut(&progress.execution_process_id) {
                exec.last_progress = Some(progress);
            }
        }

        Ok(())
    }

    /// Get active execution count
    pub async fn active_count(&self) -> usize {
        let executions = self.active_executions.read().await;
        executions.len()
    }

    /// Check if an execution is being tracked
    pub async fn is_tracking(&self, execution_process_id: Uuid) -> bool {
        let executions = self.active_executions.read().await;
        executions.contains_key(&execution_process_id)
    }

    /// Get execution info
    pub async fn get_execution_info(&self, execution_process_id: Uuid) -> Option<ExecutionInfo> {
        let executions = self.active_executions.read().await;
        executions.get(&execution_process_id).map(|e| ExecutionInfo {
            task_id: e.task_id,
            execution_process_id: e.execution_process_id,
            executor_node: e.executor_node.clone(),
            started_at: e.started_at,
            log_count: e.log_count,
            last_stage: e.last_progress.as_ref().map(|p| p.stage.clone()),
            progress_percent: e.last_progress.as_ref().map(|p| p.progress_percent),
        })
    }
}

/// Public execution info
#[derive(Debug, Clone)]
pub struct ExecutionInfo {
    pub task_id: Uuid,
    pub execution_process_id: Uuid,
    pub executor_node: String,
    pub started_at: chrono::DateTime<Utc>,
    pub log_count: usize,
    pub last_stage: Option<ExecutionStage>,
    pub progress_percent: Option<u8>,
}

impl Default for ExecutionRelay {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execution_tracking() {
        let relay = ExecutionRelay::new();

        let task_id = Uuid::new_v4();
        let exec_id = Uuid::new_v4();

        relay.start_execution(task_id, exec_id, "test_node".to_string()).await.unwrap();

        assert!(relay.is_tracking(exec_id).await);
        assert_eq!(relay.active_count().await, 1);

        relay.stop_execution(exec_id).await;

        assert!(!relay.is_tracking(exec_id).await);
        assert_eq!(relay.active_count().await, 0);
    }

    #[tokio::test]
    async fn test_log_relay() {
        let relay = ExecutionRelay::new();

        let task_id = Uuid::new_v4();
        let exec_id = Uuid::new_v4();

        relay.start_execution(task_id, exec_id, "test_node".to_string()).await.unwrap();

        // Subscribe to logs
        let mut log_rx = relay.subscribe_logs(exec_id).await.unwrap();

        // Relay a log
        let log = ExecutionLogChunk {
            execution_process_id: exec_id,
            log_type: LogType::Stdout,
            content: "Hello, world!".to_string(),
            timestamp: Utc::now(),
        };

        relay.relay_log(log.clone()).await.unwrap();

        // Check we received it
        let received = log_rx.try_recv().unwrap();
        assert_eq!(received.content, "Hello, world!");
    }
}
