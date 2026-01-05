//! Autonomy Service
//!
//! Manages autonomy modes, checkpoints, and approval gates for configurable
//! human-agent collaboration levels.

use db::{
    models::{
        approval_gate::{
            ApprovalDecision, ApprovalGate, ApprovalGateError, CreateApprovalGate,
            CreatePendingGate, GateApproval, PendingGate, PendingGateStatus, SubmitApproval,
        },
        checkpoint_definition::{
            CheckpointDefinition, CheckpointDefinitionError,
            CreateCheckpointDefinition, UpdateCheckpointDefinition,
        },
        execution_checkpoint::{
            CreateExecutionCheckpoint, ExecutionCheckpoint,
            ExecutionCheckpointError, ReviewCheckpoint,
        },
    },
    DBService,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AutonomyError {
    #[error(transparent)]
    CheckpointDefinitionError(#[from] CheckpointDefinitionError),
    #[error(transparent)]
    ExecutionCheckpointError(#[from] ExecutionCheckpointError),
    #[error(transparent)]
    ApprovalGateError(#[from] ApprovalGateError),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Checkpoint not found")]
    CheckpointNotFound,
    #[error("Gate not found")]
    GateNotFound,
    #[error("Pending gate not found")]
    PendingGateNotFound,
}

/// Autonomy mode for tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyMode {
    AgentDriven,
    AgentAssisted,
    ReviewDriven,
}

impl std::fmt::Display for AutonomyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutonomyMode::AgentDriven => write!(f, "agent_driven"),
            AutonomyMode::AgentAssisted => write!(f, "agent_assisted"),
            AutonomyMode::ReviewDriven => write!(f, "review_driven"),
        }
    }
}

impl Default for AutonomyMode {
    fn default() -> Self {
        AutonomyMode::AgentAssisted
    }
}

/// Summary of pending approvals
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct PendingApprovalsSummary {
    pub pending_checkpoints: usize,
    pub pending_gates: usize,
    pub checkpoints: Vec<ExecutionCheckpoint>,
    pub gates: Vec<PendingGateWithDetails>,
}

/// Pending gate with its definition
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct PendingGateWithDetails {
    pub pending_gate: PendingGate,
    pub gate_definition: ApprovalGate,
    pub approvals: Vec<GateApproval>,
}

/// Autonomy Service
#[derive(Clone)]
pub struct AutonomyService {
    db: DBService,
}

impl AutonomyService {
    pub fn new(db: DBService) -> Self {
        Self { db }
    }

    // ========== Autonomy Mode ==========

    /// Get autonomy mode for a task
    pub async fn get_task_autonomy_mode(&self, task_id: Uuid) -> Result<AutonomyMode, AutonomyError> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"SELECT autonomy_mode FROM tasks WHERE id = ?1"#,
        )
        .bind(task_id)
        .fetch_optional(&self.db.pool)
        .await
        .map_err(|e| AutonomyError::DatabaseError(e.to_string()))?;

        match row {
            Some((mode,)) => match mode.as_str() {
                "agent_driven" => Ok(AutonomyMode::AgentDriven),
                "agent_assisted" => Ok(AutonomyMode::AgentAssisted),
                "review_driven" => Ok(AutonomyMode::ReviewDriven),
                _ => Ok(AutonomyMode::AgentAssisted),
            },
            None => Ok(AutonomyMode::AgentAssisted),
        }
    }

    /// Set autonomy mode for a task
    pub async fn set_task_autonomy_mode(
        &self,
        task_id: Uuid,
        mode: AutonomyMode,
    ) -> Result<(), AutonomyError> {
        let mode_str = mode.to_string();

        sqlx::query(r#"UPDATE tasks SET autonomy_mode = ?2 WHERE id = ?1"#)
            .bind(task_id)
            .bind(mode_str)
            .execute(&self.db.pool)
            .await
            .map_err(|e| AutonomyError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    // ========== Checkpoint Definitions ==========

    /// Create a checkpoint definition
    pub async fn create_checkpoint_definition(
        &self,
        data: CreateCheckpointDefinition,
    ) -> Result<CheckpointDefinition, AutonomyError> {
        let def = CheckpointDefinition::create(&self.db.pool, data).await?;
        Ok(def)
    }

    /// Get checkpoint definitions for a project
    pub async fn get_checkpoint_definitions(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<CheckpointDefinition>, AutonomyError> {
        let defs = CheckpointDefinition::find_for_project(&self.db.pool, project_id).await?;
        Ok(defs)
    }

    /// Update a checkpoint definition
    pub async fn update_checkpoint_definition(
        &self,
        id: Uuid,
        data: UpdateCheckpointDefinition,
    ) -> Result<CheckpointDefinition, AutonomyError> {
        let def = CheckpointDefinition::update(&self.db.pool, id, data).await?;
        Ok(def)
    }

    /// Delete a checkpoint definition
    pub async fn delete_checkpoint_definition(&self, id: Uuid) -> Result<(), AutonomyError> {
        CheckpointDefinition::delete(&self.db.pool, id).await?;
        Ok(())
    }

    // ========== Execution Checkpoints ==========

    /// Trigger a checkpoint
    pub async fn trigger_checkpoint(
        &self,
        execution_process_id: Uuid,
        checkpoint_definition_id: Option<Uuid>,
        checkpoint_data: Value,
        trigger_reason: Option<String>,
        auto_approve_minutes: Option<i32>,
    ) -> Result<ExecutionCheckpoint, AutonomyError> {
        let expires_at = auto_approve_minutes.map(|mins| {
            chrono::Utc::now() + chrono::Duration::minutes(mins as i64)
        });

        let checkpoint = ExecutionCheckpoint::create(
            &self.db.pool,
            CreateExecutionCheckpoint {
                execution_process_id,
                checkpoint_definition_id,
                checkpoint_data,
                trigger_reason,
                expires_at,
            },
        )
        .await?;

        Ok(checkpoint)
    }

    /// Get checkpoints for an execution
    pub async fn get_execution_checkpoints(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<ExecutionCheckpoint>, AutonomyError> {
        let checkpoints =
            ExecutionCheckpoint::find_by_execution(&self.db.pool, execution_process_id).await?;
        Ok(checkpoints)
    }

    /// Get pending checkpoints for an execution
    pub async fn get_pending_checkpoints(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<ExecutionCheckpoint>, AutonomyError> {
        let checkpoints =
            ExecutionCheckpoint::find_pending(&self.db.pool, execution_process_id).await?;
        Ok(checkpoints)
    }

    /// Review a checkpoint
    pub async fn review_checkpoint(
        &self,
        checkpoint_id: Uuid,
        review: ReviewCheckpoint,
    ) -> Result<ExecutionCheckpoint, AutonomyError> {
        let checkpoint = ExecutionCheckpoint::review(&self.db.pool, checkpoint_id, review).await?;
        Ok(checkpoint)
    }

    /// Skip a checkpoint
    pub async fn skip_checkpoint(
        &self,
        checkpoint_id: Uuid,
        reason: Option<String>,
    ) -> Result<ExecutionCheckpoint, AutonomyError> {
        let checkpoint = ExecutionCheckpoint::skip(&self.db.pool, checkpoint_id, reason).await?;
        Ok(checkpoint)
    }

    /// Auto-approve expired checkpoints
    pub async fn auto_approve_expired_checkpoints(&self) -> Result<u64, AutonomyError> {
        let count = ExecutionCheckpoint::auto_approve_expired(&self.db.pool).await?;
        Ok(count)
    }

    // ========== Approval Gates ==========

    /// Create an approval gate
    pub async fn create_approval_gate(
        &self,
        data: CreateApprovalGate,
    ) -> Result<ApprovalGate, AutonomyError> {
        let gate = ApprovalGate::create(&self.db.pool, data).await?;
        Ok(gate)
    }

    /// Get approval gates for a project
    pub async fn get_project_gates(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<ApprovalGate>, AutonomyError> {
        let gates = ApprovalGate::find_by_project(&self.db.pool, project_id).await?;
        Ok(gates)
    }

    /// Get approval gates for a task
    pub async fn get_task_gates(&self, task_id: Uuid) -> Result<Vec<ApprovalGate>, AutonomyError> {
        let gates = ApprovalGate::find_by_task(&self.db.pool, task_id).await?;
        Ok(gates)
    }

    /// Delete an approval gate
    pub async fn delete_approval_gate(&self, id: Uuid) -> Result<(), AutonomyError> {
        ApprovalGate::delete(&self.db.pool, id).await?;
        Ok(())
    }

    // ========== Pending Gates ==========

    /// Trigger a gate for an execution
    pub async fn trigger_gate(
        &self,
        approval_gate_id: Uuid,
        execution_process_id: Uuid,
        trigger_context: Option<Value>,
    ) -> Result<PendingGate, AutonomyError> {
        let pending = PendingGate::create(
            &self.db.pool,
            CreatePendingGate {
                approval_gate_id,
                execution_process_id,
                trigger_context,
            },
        )
        .await?;

        Ok(pending)
    }

    /// Get pending gates for an execution
    pub async fn get_pending_gates(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<PendingGate>, AutonomyError> {
        let gates = PendingGate::find_by_execution(&self.db.pool, execution_process_id).await?;
        Ok(gates)
    }

    /// Submit an approval for a pending gate
    pub async fn submit_gate_approval(
        &self,
        pending_gate_id: Uuid,
        data: SubmitApproval,
    ) -> Result<GateApproval, AutonomyError> {
        // Get the pending gate
        let pending = PendingGate::find_by_id(&self.db.pool, pending_gate_id)
            .await?
            .ok_or(AutonomyError::PendingGateNotFound)?;

        // Get the gate definition
        let gate = ApprovalGate::find_by_id(&self.db.pool, pending.approval_gate_id)
            .await?
            .ok_or(AutonomyError::GateNotFound)?;

        // Create the approval record
        let approval = GateApproval::create(
            &self.db.pool,
            pending_gate_id,
            pending.execution_process_id,
            pending.approval_gate_id,
            data,
        )
        .await?;

        // Update counts and check if gate is resolved
        let approvals = GateApproval::find_for_pending_gate(
            &self.db.pool,
            pending.approval_gate_id,
            pending.execution_process_id,
        )
        .await?;

        let approval_count = approvals
            .iter()
            .filter(|a| a.decision == ApprovalDecision::Approved)
            .count() as i32;
        let rejection_count = approvals
            .iter()
            .filter(|a| a.decision == ApprovalDecision::Rejected)
            .count() as i32;

        PendingGate::update_counts(&self.db.pool, pending_gate_id, approval_count, rejection_count)
            .await?;

        // Resolve if enough approvals or any rejection
        if approval_count >= gate.min_approvals {
            PendingGate::resolve(&self.db.pool, pending_gate_id, PendingGateStatus::Approved)
                .await?;
        } else if rejection_count > 0 {
            PendingGate::resolve(&self.db.pool, pending_gate_id, PendingGateStatus::Rejected)
                .await?;
        }

        Ok(approval)
    }

    /// Bypass a pending gate
    pub async fn bypass_gate(&self, pending_gate_id: Uuid) -> Result<PendingGate, AutonomyError> {
        let pending =
            PendingGate::resolve(&self.db.pool, pending_gate_id, PendingGateStatus::Bypassed)
                .await?;
        Ok(pending)
    }

    // ========== Summary ==========

    /// Get all pending approvals summary
    pub async fn get_pending_approvals_summary(&self) -> Result<PendingApprovalsSummary, AutonomyError> {
        // Get all pending checkpoints
        let checkpoints = ExecutionCheckpoint::find_all_pending(&self.db.pool).await?;

        // Get all pending gates with details
        let pending_gates = PendingGate::find_all_pending(&self.db.pool).await?;
        let mut gates_with_details = Vec::new();

        for pending in pending_gates {
            if let Some(gate_def) = ApprovalGate::find_by_id(&self.db.pool, pending.approval_gate_id).await? {
                let approvals = GateApproval::find_for_pending_gate(
                    &self.db.pool,
                    pending.approval_gate_id,
                    pending.execution_process_id,
                )
                .await?;

                gates_with_details.push(PendingGateWithDetails {
                    pending_gate: pending,
                    gate_definition: gate_def,
                    approvals,
                });
            }
        }

        Ok(PendingApprovalsSummary {
            pending_checkpoints: checkpoints.len(),
            pending_gates: gates_with_details.len(),
            checkpoints,
            gates: gates_with_details,
        })
    }

    /// Check if execution should proceed based on autonomy mode and pending items
    pub async fn can_execution_proceed(
        &self,
        execution_process_id: Uuid,
    ) -> Result<bool, AutonomyError> {
        // Check for pending checkpoints
        let checkpoints = self.get_pending_checkpoints(execution_process_id).await?;
        if !checkpoints.is_empty() {
            return Ok(false);
        }

        // Check for pending gates
        let gates = self.get_pending_gates(execution_process_id).await?;
        let has_blocking_gate = gates
            .iter()
            .any(|g| g.status == PendingGateStatus::Pending || g.status == PendingGateStatus::Rejected);

        Ok(!has_blocking_gate)
    }
}
