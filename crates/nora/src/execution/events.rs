//! Event Broadcaster - Unified event stream for all UI surfaces
//!
//! Broadcasts execution events to:
//! - Mission Control (SSE stream)
//! - Task Board (WebSocket)
//! - Nora Chat (inline updates)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use ts_rs::TS;
use uuid::Uuid;

/// Events emitted during execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExecutionEvent {
    /// Execution started
    Started {
        execution_id: Uuid,
        agent_id: String,
        agent_name: String,
        workflow_name: String,
        project_id: Option<Uuid>,
        total_stages: u32,
        timestamp: DateTime<Utc>,
    },

    /// Stage started
    StageStarted {
        execution_id: Uuid,
        agent_id: String,
        agent_name: String,
        stage_index: u32,
        stage_name: String,
        total_stages: u32,
        timestamp: DateTime<Utc>,
    },

    /// Stage completed successfully
    StageCompleted {
        execution_id: Uuid,
        agent_id: String,
        stage_index: u32,
        stage_name: String,
        duration_ms: u64,
        artifact_count: u32,
        timestamp: DateTime<Utc>,
    },

    /// Stage failed
    StageFailed {
        execution_id: Uuid,
        agent_id: String,
        stage_index: u32,
        stage_name: String,
        error: String,
        timestamp: DateTime<Utc>,
    },

    /// Execution completed successfully
    Completed {
        execution_id: Uuid,
        agent_id: String,
        agent_name: String,
        workflow_name: String,
        total_stages: u32,
        duration_ms: u64,
        artifact_count: u32,
        timestamp: DateTime<Utc>,
    },

    /// Execution failed
    Failed {
        execution_id: Uuid,
        agent_id: String,
        agent_name: String,
        workflow_name: String,
        failed_stage: u32,
        error: String,
        timestamp: DateTime<Utc>,
    },

    /// Task created on board
    TaskCreated {
        execution_id: Uuid,
        task_id: Uuid,
        project_id: Uuid,
        title: String,
        agent_id: String,
        timestamp: DateTime<Utc>,
    },

    /// Artifact produced
    ArtifactProduced {
        execution_id: Uuid,
        artifact_id: Uuid,
        artifact_type: String,
        title: String,
        stage_name: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Agent status change
    AgentStatus {
        agent_id: String,
        agent_name: String,
        status: AgentStatusType,
        current_task: Option<String>,
        timestamp: DateTime<Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatusType {
    Idle,
    Planning,
    Executing,
    Verifying,
    Blocked,
    Error,
}

/// Broadcasts events to all subscribers
pub struct EventBroadcaster {
    sender: broadcast::Sender<ExecutionEvent>,
}

impl EventBroadcaster {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    /// Broadcast an event to all subscribers
    pub fn broadcast(&self, event: ExecutionEvent) {
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<ExecutionEvent> {
        self.sender.subscribe()
    }

    // Convenience methods for common events

    pub fn execution_started(
        &self,
        execution_id: Uuid,
        agent_id: &str,
        agent_name: &str,
        workflow_name: &str,
        project_id: Option<Uuid>,
        total_stages: u32,
    ) {
        self.broadcast(ExecutionEvent::Started {
            execution_id,
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            workflow_name: workflow_name.to_string(),
            project_id,
            total_stages,
            timestamp: Utc::now(),
        });
    }

    pub fn stage_started(
        &self,
        execution_id: Uuid,
        agent_id: &str,
        agent_name: &str,
        stage_index: u32,
        stage_name: &str,
        total_stages: u32,
    ) {
        self.broadcast(ExecutionEvent::StageStarted {
            execution_id,
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            stage_index,
            stage_name: stage_name.to_string(),
            total_stages,
            timestamp: Utc::now(),
        });
    }

    pub fn stage_completed(
        &self,
        execution_id: Uuid,
        agent_id: &str,
        stage_index: u32,
        stage_name: &str,
        duration_ms: u64,
        artifact_count: u32,
    ) {
        self.broadcast(ExecutionEvent::StageCompleted {
            execution_id,
            agent_id: agent_id.to_string(),
            stage_index,
            stage_name: stage_name.to_string(),
            duration_ms,
            artifact_count,
            timestamp: Utc::now(),
        });
    }

    pub fn stage_failed(
        &self,
        execution_id: Uuid,
        agent_id: &str,
        stage_index: u32,
        stage_name: &str,
        error: &str,
    ) {
        self.broadcast(ExecutionEvent::StageFailed {
            execution_id,
            agent_id: agent_id.to_string(),
            stage_index,
            stage_name: stage_name.to_string(),
            error: error.to_string(),
            timestamp: Utc::now(),
        });
    }

    pub fn execution_completed(
        &self,
        execution_id: Uuid,
        agent_id: &str,
        agent_name: &str,
        workflow_name: &str,
        total_stages: u32,
        duration_ms: u64,
        artifact_count: u32,
    ) {
        self.broadcast(ExecutionEvent::Completed {
            execution_id,
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            workflow_name: workflow_name.to_string(),
            total_stages,
            duration_ms,
            artifact_count,
            timestamp: Utc::now(),
        });
    }

    pub fn execution_failed(
        &self,
        execution_id: Uuid,
        agent_id: &str,
        agent_name: &str,
        workflow_name: &str,
        failed_stage: u32,
        error: &str,
    ) {
        self.broadcast(ExecutionEvent::Failed {
            execution_id,
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            workflow_name: workflow_name.to_string(),
            failed_stage,
            error: error.to_string(),
            timestamp: Utc::now(),
        });
    }

    pub fn task_created(
        &self,
        execution_id: Uuid,
        task_id: Uuid,
        project_id: Uuid,
        title: &str,
        agent_id: &str,
    ) {
        self.broadcast(ExecutionEvent::TaskCreated {
            execution_id,
            task_id,
            project_id,
            title: title.to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
        });
    }

    pub fn agent_status(
        &self,
        agent_id: &str,
        agent_name: &str,
        status: AgentStatusType,
        current_task: Option<&str>,
    ) {
        self.broadcast(ExecutionEvent::AgentStatus {
            agent_id: agent_id.to_string(),
            agent_name: agent_name.to_string(),
            status,
            current_task: current_task.map(String::from),
            timestamp: Utc::now(),
        });
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
