//! Artifact Store - Stage output communication
//!
//! Artifacts are the "paper trail" for agent execution (Antigravity pattern).
//! They enable:
//! - Stage-to-stage data passing (fixing the missing output chaining)
//! - Audit trail for all agent actions
//! - UI visibility into execution progress

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

/// Types of artifacts produced during execution
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    /// Execution plan from Router phase
    Plan,
    /// Output from a stage execution
    StageOutput,
    /// File diff produced by agent
    Diff,
    /// Screenshot or visual capture
    Screenshot,
    /// Log/trace of execution
    ExecutionLog,
    /// Error report
    Error,
    /// Final deliverable
    Deliverable,
    /// Intermediate data for next stage
    StageData,
}

/// An artifact produced during execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub stage_index: Option<u32>,
    pub stage_name: Option<String>,
    pub artifact_type: ArtifactType,
    pub title: String,
    pub content: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub agent_id: Option<String>,
}

impl Artifact {
    pub fn new(
        execution_id: Uuid,
        artifact_type: ArtifactType,
        title: impl Into<String>,
        content: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            execution_id,
            stage_index: None,
            stage_name: None,
            artifact_type,
            title: title.into(),
            content,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            agent_id: None,
        }
    }

    pub fn with_stage(mut self, index: u32, name: impl Into<String>) -> Self {
        self.stage_index = Some(index);
        self.stage_name = Some(name.into());
        self
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Store for artifacts during execution
///
/// Provides:
/// - In-memory storage during execution
/// - Stage output retrieval for chaining
/// - Query by execution, stage, or type
#[derive(Debug, Default)]
pub struct ArtifactStore {
    /// Artifacts indexed by execution_id
    by_execution: RwLock<HashMap<Uuid, Vec<Artifact>>>,
    /// Quick lookup of stage outputs for chaining
    stage_outputs: RwLock<HashMap<(Uuid, u32), serde_json::Value>>,
}

impl ArtifactStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            by_execution: RwLock::new(HashMap::new()),
            stage_outputs: RwLock::new(HashMap::new()),
        })
    }

    /// Store a new artifact
    pub async fn store(&self, artifact: Artifact) {
        // Store stage output for chaining if applicable
        if artifact.artifact_type == ArtifactType::StageOutput ||
           artifact.artifact_type == ArtifactType::StageData {
            if let Some(stage_index) = artifact.stage_index {
                let mut outputs = self.stage_outputs.write().await;
                outputs.insert((artifact.execution_id, stage_index), artifact.content.clone());
            }
        }

        // Store the artifact
        let mut by_exec = self.by_execution.write().await;
        by_exec
            .entry(artifact.execution_id)
            .or_insert_with(Vec::new)
            .push(artifact);
    }

    /// Get output from a previous stage (for chaining)
    pub async fn get_stage_output(&self, execution_id: Uuid, stage_index: u32) -> Option<serde_json::Value> {
        let outputs = self.stage_outputs.read().await;
        outputs.get(&(execution_id, stage_index)).cloned()
    }

    /// Get all outputs from previous stages
    pub async fn get_all_stage_outputs(&self, execution_id: Uuid) -> HashMap<u32, serde_json::Value> {
        let outputs = self.stage_outputs.read().await;
        outputs
            .iter()
            .filter(|((exec_id, _), _)| *exec_id == execution_id)
            .map(|((_, stage), value)| (*stage, value.clone()))
            .collect()
    }

    /// Get all artifacts for an execution
    pub async fn get_by_execution(&self, execution_id: Uuid) -> Vec<Artifact> {
        let by_exec = self.by_execution.read().await;
        by_exec.get(&execution_id).cloned().unwrap_or_default()
    }

    /// Get artifacts of a specific type for an execution
    pub async fn get_by_type(&self, execution_id: Uuid, artifact_type: ArtifactType) -> Vec<Artifact> {
        self.get_by_execution(execution_id)
            .await
            .into_iter()
            .filter(|a| a.artifact_type == artifact_type)
            .collect()
    }

    /// Get the latest artifact of a type
    pub async fn get_latest(&self, execution_id: Uuid, artifact_type: ArtifactType) -> Option<Artifact> {
        self.get_by_type(execution_id, artifact_type)
            .await
            .into_iter()
            .max_by_key(|a| a.created_at)
    }

    /// Clean up artifacts for completed executions
    pub async fn cleanup(&self, execution_id: Uuid) {
        let mut by_exec = self.by_execution.write().await;
        by_exec.remove(&execution_id);

        let mut outputs = self.stage_outputs.write().await;
        outputs.retain(|(exec_id, _), _| *exec_id != execution_id);
    }

    /// Get count of artifacts for an execution
    pub async fn count(&self, execution_id: Uuid) -> usize {
        let by_exec = self.by_execution.read().await;
        by_exec.get(&execution_id).map(|v| v.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stage_output_chaining() {
        let store = ArtifactStore::new();
        let exec_id = Uuid::new_v4();

        // Stage 0 produces output
        let artifact = Artifact::new(
            exec_id,
            ArtifactType::StageOutput,
            "Stage 0 Output",
            serde_json::json!({"batch_id": "abc123"}),
        ).with_stage(0, "Ingest");

        store.store(artifact).await;

        // Stage 1 can retrieve it
        let output = store.get_stage_output(exec_id, 0).await;
        assert!(output.is_some());
        assert_eq!(output.unwrap()["batch_id"], "abc123");
    }
}
