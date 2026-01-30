//! Workflow QA Run model for tracking quality assurance evaluations
//!
//! Each QA run evaluates a workflow stage against a checklist and provides
//! a decision (approve, retry, escalate) with guidance.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Confidence level of QA evaluation
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "confidence_level", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceLevel::High => write!(f, "high"),
            ConfidenceLevel::Medium => write!(f, "medium"),
            ConfidenceLevel::Low => write!(f, "low"),
        }
    }
}

/// QA decision outcome
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "qa_decision", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum QADecision {
    Approve,
    Retry,
    Escalate,
}

impl std::fmt::Display for QADecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QADecision::Approve => write!(f, "approve"),
            QADecision::Retry => write!(f, "retry"),
            QADecision::Escalate => write!(f, "escalate"),
        }
    }
}

/// Individual checklist item result
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub item: String,
    pub passed: bool,
    pub notes: Option<String>,
}

/// Full Workflow QA Run record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowQARun {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub stage_name: String,
    pub artifact_id: Option<Uuid>,
    pub checklist_items: String,  // JSON
    pub confidence_level: ConfidenceLevel,
    pub overall_score: f64,
    pub decision: QADecision,
    pub retry_guidance: Option<String>,
    pub escalation_reason: Option<String>,
    pub agent_id: Option<Uuid>,
    pub execution_time_ms: Option<i64>,
    pub tokens_used: Option<i64>,
    pub created_at: DateTime<Utc>,
}

impl WorkflowQARun {
    /// Parse checklist items from JSON
    pub fn checklist_items_parsed(&self) -> Option<Vec<ChecklistItem>> {
        serde_json::from_str(&self.checklist_items).ok()
    }

    /// Calculate pass rate from checklist
    pub fn pass_rate(&self) -> f64 {
        if let Some(items) = self.checklist_items_parsed() {
            if items.is_empty() {
                return 0.0;
            }
            let passed = items.iter().filter(|i| i.passed).count();
            passed as f64 / items.len() as f64
        } else {
            0.0
        }
    }
}

/// Create a new QA run
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowQARun {
    pub workflow_id: Uuid,
    pub stage_name: String,
    pub artifact_id: Option<Uuid>,
    pub checklist_items: Vec<ChecklistItem>,
    pub confidence_level: ConfidenceLevel,
    pub overall_score: f64,
    pub decision: QADecision,
    pub retry_guidance: Option<String>,
    pub escalation_reason: Option<String>,
    pub agent_id: Option<Uuid>,
    pub execution_time_ms: Option<i64>,
    pub tokens_used: Option<i64>,
}

/// QA run summary for lists
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowQARunBrief {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub stage_name: String,
    pub confidence_level: ConfidenceLevel,
    pub overall_score: f64,
    pub decision: QADecision,
    pub created_at: DateTime<Utc>,
}

impl From<WorkflowQARun> for WorkflowQARunBrief {
    fn from(run: WorkflowQARun) -> Self {
        Self {
            id: run.id,
            workflow_id: run.workflow_id,
            stage_name: run.stage_name,
            confidence_level: run.confidence_level,
            overall_score: run.overall_score,
            decision: run.decision,
            created_at: run.created_at,
        }
    }
}

impl WorkflowQARun {
    /// Find all QA runs
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WorkflowQARun,
            r#"SELECT
                id as "id!: Uuid",
                workflow_id as "workflow_id!: Uuid",
                stage_name,
                artifact_id as "artifact_id: Uuid",
                checklist_items,
                confidence_level as "confidence_level!: ConfidenceLevel",
                overall_score as "overall_score!: f64",
                decision as "decision!: QADecision",
                retry_guidance,
                escalation_reason,
                agent_id as "agent_id: Uuid",
                execution_time_ms,
                tokens_used,
                created_at as "created_at!: DateTime<Utc>"
            FROM workflow_qa_runs
            ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find QA runs by workflow
    pub async fn find_by_workflow(pool: &SqlitePool, workflow_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WorkflowQARun,
            r#"SELECT
                id as "id!: Uuid",
                workflow_id as "workflow_id!: Uuid",
                stage_name,
                artifact_id as "artifact_id: Uuid",
                checklist_items,
                confidence_level as "confidence_level!: ConfidenceLevel",
                overall_score as "overall_score!: f64",
                decision as "decision!: QADecision",
                retry_guidance,
                escalation_reason,
                agent_id as "agent_id: Uuid",
                execution_time_ms,
                tokens_used,
                created_at as "created_at!: DateTime<Utc>"
            FROM workflow_qa_runs
            WHERE workflow_id = $1
            ORDER BY created_at DESC"#,
            workflow_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find QA runs by workflow and stage
    pub async fn find_by_workflow_stage(pool: &SqlitePool, workflow_id: Uuid, stage_name: &str) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            WorkflowQARun,
            r#"SELECT
                id as "id!: Uuid",
                workflow_id as "workflow_id!: Uuid",
                stage_name,
                artifact_id as "artifact_id: Uuid",
                checklist_items,
                confidence_level as "confidence_level!: ConfidenceLevel",
                overall_score as "overall_score!: f64",
                decision as "decision!: QADecision",
                retry_guidance,
                escalation_reason,
                agent_id as "agent_id: Uuid",
                execution_time_ms,
                tokens_used,
                created_at as "created_at!: DateTime<Utc>"
            FROM workflow_qa_runs
            WHERE workflow_id = $1 AND stage_name = $2
            ORDER BY created_at DESC"#,
            workflow_id,
            stage_name
        )
        .fetch_all(pool)
        .await
    }

    /// Find latest QA run for a workflow stage
    pub async fn find_latest_for_stage(pool: &SqlitePool, workflow_id: Uuid, stage_name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            WorkflowQARun,
            r#"SELECT
                id as "id!: Uuid",
                workflow_id as "workflow_id!: Uuid",
                stage_name,
                artifact_id as "artifact_id: Uuid",
                checklist_items,
                confidence_level as "confidence_level!: ConfidenceLevel",
                overall_score as "overall_score!: f64",
                decision as "decision!: QADecision",
                retry_guidance,
                escalation_reason,
                agent_id as "agent_id: Uuid",
                execution_time_ms,
                tokens_used,
                created_at as "created_at!: DateTime<Utc>"
            FROM workflow_qa_runs
            WHERE workflow_id = $1 AND stage_name = $2
            ORDER BY created_at DESC
            LIMIT 1"#,
            workflow_id,
            stage_name
        )
        .fetch_optional(pool)
        .await
    }

    /// Find QA run by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            WorkflowQARun,
            r#"SELECT
                id as "id!: Uuid",
                workflow_id as "workflow_id!: Uuid",
                stage_name,
                artifact_id as "artifact_id: Uuid",
                checklist_items,
                confidence_level as "confidence_level!: ConfidenceLevel",
                overall_score as "overall_score!: f64",
                decision as "decision!: QADecision",
                retry_guidance,
                escalation_reason,
                agent_id as "agent_id: Uuid",
                execution_time_ms,
                tokens_used,
                created_at as "created_at!: DateTime<Utc>"
            FROM workflow_qa_runs
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new QA run
    pub async fn create(pool: &SqlitePool, data: &CreateWorkflowQARun) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let checklist_json = serde_json::to_string(&data.checklist_items).unwrap();
        let confidence_str = data.confidence_level.to_string();
        let decision_str = data.decision.to_string();

        sqlx::query_as!(
            WorkflowQARun,
            r#"INSERT INTO workflow_qa_runs (
                id, workflow_id, stage_name, artifact_id, checklist_items,
                confidence_level, overall_score, decision,
                retry_guidance, escalation_reason, agent_id,
                execution_time_ms, tokens_used
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
            )
            RETURNING
                id as "id!: Uuid",
                workflow_id as "workflow_id!: Uuid",
                stage_name,
                artifact_id as "artifact_id: Uuid",
                checklist_items,
                confidence_level as "confidence_level!: ConfidenceLevel",
                overall_score as "overall_score!: f64",
                decision as "decision!: QADecision",
                retry_guidance,
                escalation_reason,
                agent_id as "agent_id: Uuid",
                execution_time_ms,
                tokens_used,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.workflow_id,
            data.stage_name,
            data.artifact_id,
            checklist_json,
            confidence_str,
            data.overall_score,
            decision_str,
            data.retry_guidance,
            data.escalation_reason,
            data.agent_id,
            data.execution_time_ms,
            data.tokens_used
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a QA run
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM workflow_qa_runs WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Count failed QA runs for a workflow
    pub async fn count_failures(pool: &SqlitePool, workflow_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT COUNT(*) as "count!: i64" FROM workflow_qa_runs
            WHERE workflow_id = $1 AND decision != 'approve'"#,
            workflow_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.count)
    }

    /// Get average QA score for a workflow
    pub async fn average_score(pool: &SqlitePool, workflow_id: Uuid) -> Result<Option<f64>, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT AVG(overall_score) as "avg_score: f64" FROM workflow_qa_runs WHERE workflow_id = $1"#,
            workflow_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.avg_score)
    }
}
