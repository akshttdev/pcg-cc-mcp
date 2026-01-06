use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ApprovalGateError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Approval gate not found")]
    NotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "gate_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum GateType {
    PreExecution,
    PostPlan,
    PreCommit,
    PostExecution,
    Custom,
}

impl std::fmt::Display for GateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateType::PreExecution => write!(f, "pre_execution"),
            GateType::PostPlan => write!(f, "post_plan"),
            GateType::PreCommit => write!(f, "pre_commit"),
            GateType::PostExecution => write!(f, "post_execution"),
            GateType::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "pending_gate_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PendingGateStatus {
    Pending,
    Approved,
    Rejected,
    Bypassed,
}

impl std::fmt::Display for PendingGateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PendingGateStatus::Pending => write!(f, "pending"),
            PendingGateStatus::Approved => write!(f, "approved"),
            PendingGateStatus::Rejected => write!(f, "rejected"),
            PendingGateStatus::Bypassed => write!(f, "bypassed"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "approval_decision", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    Approved,
    Rejected,
    Abstained,
}

impl std::fmt::Display for ApprovalDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalDecision::Approved => write!(f, "approved"),
            ApprovalDecision::Rejected => write!(f, "rejected"),
            ApprovalDecision::Abstained => write!(f, "abstained"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ApprovalGate {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub name: String,
    pub gate_type: GateType,
    #[sqlx(default)]
    pub required_approvers: String, // JSON array
    pub min_approvals: i32,
    #[sqlx(default)]
    pub conditions: Option<String>, // JSON
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct PendingGate {
    pub id: Uuid,
    pub approval_gate_id: Uuid,
    pub execution_process_id: Uuid,
    pub status: PendingGateStatus,
    #[sqlx(default)]
    pub trigger_context: Option<String>, // JSON
    pub approval_count: i32,
    pub rejection_count: i32,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct GateApproval {
    pub id: Uuid,
    pub approval_gate_id: Uuid,
    pub execution_process_id: Uuid,
    pub approver_id: String,
    pub approver_name: Option<String>,
    pub decision: ApprovalDecision,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateApprovalGate {
    pub project_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub name: String,
    pub gate_type: GateType,
    pub required_approvers: Vec<String>,
    #[serde(default = "default_min_approvals")]
    pub min_approvals: i32,
    pub conditions: Option<Value>,
}

fn default_min_approvals() -> i32 {
    1
}

#[derive(Debug, Deserialize, TS)]
pub struct CreatePendingGate {
    pub approval_gate_id: Uuid,
    pub execution_process_id: Uuid,
    pub trigger_context: Option<Value>,
}

#[derive(Debug, Deserialize, TS)]
pub struct SubmitApproval {
    pub approver_id: String,
    pub approver_name: Option<String>,
    pub decision: ApprovalDecision,
    pub comment: Option<String>,
}

impl ApprovalGate {
    /// Create a new approval gate
    pub async fn create(
        pool: &SqlitePool,
        data: CreateApprovalGate,
    ) -> Result<Self, ApprovalGateError> {
        let id = Uuid::new_v4();
        let gate_type_str = data.gate_type.to_string();
        let required_approvers_str = serde_json::to_string(&data.required_approvers)
            .unwrap_or_else(|_| "[]".to_string());
        let conditions_str = data.conditions.map(|v| v.to_string());

        let gate = sqlx::query_as::<_, ApprovalGate>(
            r#"
            INSERT INTO approval_gates (
                id, project_id, task_id, name, gate_type,
                required_approvers, min_approvals, conditions
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(data.task_id)
        .bind(&data.name)
        .bind(gate_type_str)
        .bind(required_approvers_str)
        .bind(data.min_approvals)
        .bind(conditions_str)
        .fetch_one(pool)
        .await?;

        Ok(gate)
    }

    /// Find by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, ApprovalGateError> {
        let gate = sqlx::query_as::<_, ApprovalGate>(
            r#"SELECT * FROM approval_gates WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(gate)
    }

    /// Find all gates for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, ApprovalGateError> {
        let gates = sqlx::query_as::<_, ApprovalGate>(
            r#"
            SELECT * FROM approval_gates
            WHERE project_id = ?1 AND is_active = 1
            ORDER BY created_at ASC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(gates)
    }

    /// Find gates for a task
    pub async fn find_by_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, ApprovalGateError> {
        let gates = sqlx::query_as::<_, ApprovalGate>(
            r#"
            SELECT * FROM approval_gates
            WHERE task_id = ?1 AND is_active = 1
            ORDER BY created_at ASC
            "#,
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        Ok(gates)
    }

    /// Delete a gate
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), ApprovalGateError> {
        sqlx::query(r#"DELETE FROM approval_gates WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Parse required_approvers as Vec<String>
    pub fn required_approvers_list(&self) -> Vec<String> {
        serde_json::from_str(&self.required_approvers).unwrap_or_default()
    }

    /// Parse conditions as JSON Value
    pub fn conditions_json(&self) -> Option<Value> {
        self.conditions
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }
}

impl PendingGate {
    /// Create a pending gate instance
    pub async fn create(
        pool: &SqlitePool,
        data: CreatePendingGate,
    ) -> Result<Self, ApprovalGateError> {
        let id = Uuid::new_v4();
        let context_str = data.trigger_context.map(|v| v.to_string());

        let pending = sqlx::query_as::<_, PendingGate>(
            r#"
            INSERT INTO pending_gates (id, approval_gate_id, execution_process_id, trigger_context)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.approval_gate_id)
        .bind(data.execution_process_id)
        .bind(context_str)
        .fetch_one(pool)
        .await?;

        Ok(pending)
    }

    /// Find by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, ApprovalGateError> {
        let pending = sqlx::query_as::<_, PendingGate>(
            r#"SELECT * FROM pending_gates WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(pending)
    }

    /// Find pending gates for an execution
    pub async fn find_by_execution(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ApprovalGateError> {
        let pending = sqlx::query_as::<_, PendingGate>(
            r#"
            SELECT * FROM pending_gates
            WHERE execution_process_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(pending)
    }

    /// Find all pending (unresolved) gates
    pub async fn find_all_pending(pool: &SqlitePool) -> Result<Vec<Self>, ApprovalGateError> {
        let pending = sqlx::query_as::<_, PendingGate>(
            r#"
            SELECT * FROM pending_gates
            WHERE status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(pending)
    }

    /// Update approval counts
    pub async fn update_counts(
        pool: &SqlitePool,
        id: Uuid,
        approval_count: i32,
        rejection_count: i32,
    ) -> Result<Self, ApprovalGateError> {
        let pending = sqlx::query_as::<_, PendingGate>(
            r#"
            UPDATE pending_gates
            SET approval_count = ?2, rejection_count = ?3
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(approval_count)
        .bind(rejection_count)
        .fetch_one(pool)
        .await?;

        Ok(pending)
    }

    /// Resolve a pending gate
    pub async fn resolve(
        pool: &SqlitePool,
        id: Uuid,
        status: PendingGateStatus,
    ) -> Result<Self, ApprovalGateError> {
        let status_str = status.to_string();

        let pending = sqlx::query_as::<_, PendingGate>(
            r#"
            UPDATE pending_gates
            SET status = ?2, resolved_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status_str)
        .fetch_one(pool)
        .await?;

        Ok(pending)
    }
}

impl GateApproval {
    /// Submit an approval
    pub async fn create(
        pool: &SqlitePool,
        _pending_gate_id: Uuid,
        execution_process_id: Uuid,
        approval_gate_id: Uuid,
        data: SubmitApproval,
    ) -> Result<Self, ApprovalGateError> {
        let id = Uuid::new_v4();
        let decision_str = data.decision.to_string();

        let approval = sqlx::query_as::<_, GateApproval>(
            r#"
            INSERT INTO gate_approvals (
                id, approval_gate_id, execution_process_id,
                approver_id, approver_name, decision, comment
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(approval_gate_id)
        .bind(execution_process_id)
        .bind(&data.approver_id)
        .bind(&data.approver_name)
        .bind(decision_str)
        .bind(&data.comment)
        .fetch_one(pool)
        .await?;

        Ok(approval)
    }

    /// Find approvals for a pending gate
    pub async fn find_for_pending_gate(
        pool: &SqlitePool,
        approval_gate_id: Uuid,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ApprovalGateError> {
        let approvals = sqlx::query_as::<_, GateApproval>(
            r#"
            SELECT * FROM gate_approvals
            WHERE approval_gate_id = ?1 AND execution_process_id = ?2
            ORDER BY created_at ASC
            "#,
        )
        .bind(approval_gate_id)
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(approvals)
    }
}
