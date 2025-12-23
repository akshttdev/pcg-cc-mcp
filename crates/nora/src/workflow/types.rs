//! Core types for workflow orchestration

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;
use uuid::Uuid;

use crate::profiles::AgentWorkflow;

/// Represents a running instance of an agent workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    pub id: Uuid,
    pub agent_id: String,
    pub workflow_id: String,
    pub workflow: AgentWorkflow,
    pub current_stage: usize,
    pub state: WorkflowState,
    pub context: WorkflowContext,
    pub created_tasks: Vec<Uuid>,
    pub deliverables: Vec<Deliverable>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// State of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum WorkflowState {
    Queued,
    Running {
        stage: usize,
        stage_name: String,
        progress: f32,
    },
    Paused {
        reason: String,
        stage: usize,
    },
    Failed {
        error: String,
        stage: usize,
        stage_name: String,
    },
    Completed {
        total_stages: usize,
        execution_time_ms: u64,
    },
}

/// Context data passed through workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    pub project_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub stage_outputs: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, String>,
}

impl WorkflowContext {
    pub fn new() -> Self {
        Self {
            project_id: None,
            user_id: None,
            inputs: HashMap::new(),
            stage_outputs: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_input(mut self, key: String, value: serde_json::Value) -> Self {
        self.inputs.insert(key, value);
        self
    }

    pub fn with_project(mut self, project_id: Uuid) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn set_stage_output(&mut self, stage_name: String, output: serde_json::Value) {
        self.stage_outputs.insert(stage_name, output);
    }

    pub fn get_stage_output(&self, stage_name: &str) -> Option<&serde_json::Value> {
        self.stage_outputs.get(stage_name)
    }
}

impl Default for WorkflowContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a workflow stage execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStageResult {
    pub stage_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub task_id: Option<Uuid>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Final result of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResult {
    pub workflow_id: Uuid,
    pub agent_id: String,
    pub workflow_name: String,
    pub state: WorkflowState,
    pub created_tasks: Vec<Uuid>,
    pub deliverables: Vec<Deliverable>,
    pub execution_time_ms: u64,
}

/// Represents a deliverable produced by a workflow
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Deliverable {
    pub name: String,
    pub deliverable_type: String,
    pub url: Option<String>,
    pub file_path: Option<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

impl Deliverable {
    pub fn new(name: String, deliverable_type: String) -> Self {
        Self {
            name,
            deliverable_type,
            url: None,
            file_path: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn with_file_path(mut self, path: String) -> Self {
        self.file_path = Some(path);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}
