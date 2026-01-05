use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AgentTaskPlanError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Task plan not found")]
    NotFound,
    #[error("Invalid plan JSON: {0}")]
    InvalidPlanJson(String),
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "plan_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Planning,
    Executing,
    Completed,
    Failed,
    Paused,
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanStatus::Planning => write!(f, "planning"),
            PlanStatus::Executing => write!(f, "executing"),
            PlanStatus::Completed => write!(f, "completed"),
            PlanStatus::Failed => write!(f, "failed"),
            PlanStatus::Paused => write!(f, "paused"),
        }
    }
}

/// A single step in an agent's task plan
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct PlanStep {
    pub index: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: StepStatus,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "lowercase")]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AgentTaskPlan {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub plan_json: String, // JSON array of PlanStep
    pub current_step: Option<i32>,
    pub total_steps: Option<i32>,
    pub status: PlanStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateAgentTaskPlan {
    pub execution_process_id: Uuid,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateAgentTaskPlan {
    pub current_step: Option<i32>,
    pub status: Option<PlanStatus>,
    pub steps: Option<Vec<PlanStep>>,
}

impl AgentTaskPlan {
    /// Create a new agent task plan
    pub async fn create(
        pool: &SqlitePool,
        data: CreateAgentTaskPlan,
    ) -> Result<Self, AgentTaskPlanError> {
        let id = Uuid::new_v4();
        let total_steps = data.steps.len() as i32;
        let plan_json = serde_json::to_string(&data.steps)
            .map_err(|e| AgentTaskPlanError::InvalidPlanJson(e.to_string()))?;

        let plan = sqlx::query_as::<_, AgentTaskPlan>(
            r#"
            INSERT INTO agent_task_plans (id, execution_process_id, plan_json, current_step, total_steps, status)
            VALUES (?1, ?2, ?3, 0, ?4, 'planning')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(&plan_json)
        .bind(total_steps)
        .fetch_one(pool)
        .await?;

        Ok(plan)
    }

    /// Find plan by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, AgentTaskPlanError> {
        let plan = sqlx::query_as::<_, AgentTaskPlan>(
            r#"SELECT * FROM agent_task_plans WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(plan)
    }

    /// Find plan by execution process ID
    pub async fn find_by_execution_process(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Option<Self>, AgentTaskPlanError> {
        let plan = sqlx::query_as::<_, AgentTaskPlan>(
            r#"
            SELECT * FROM agent_task_plans
            WHERE execution_process_id = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(execution_process_id)
        .fetch_optional(pool)
        .await?;

        Ok(plan)
    }

    /// Get all active plans (planning or executing)
    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, AgentTaskPlanError> {
        let plans = sqlx::query_as::<_, AgentTaskPlan>(
            r#"
            SELECT * FROM agent_task_plans
            WHERE status IN ('planning', 'executing')
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(plans)
    }

    /// Update plan status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: PlanStatus,
    ) -> Result<Self, AgentTaskPlanError> {
        let status_str = status.to_string();

        let plan = sqlx::query_as::<_, AgentTaskPlan>(
            r#"
            UPDATE agent_task_plans
            SET status = ?2, updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status_str)
        .fetch_one(pool)
        .await?;

        Ok(plan)
    }

    /// Update current step
    pub async fn update_current_step(
        pool: &SqlitePool,
        id: Uuid,
        step: i32,
    ) -> Result<Self, AgentTaskPlanError> {
        let plan = sqlx::query_as::<_, AgentTaskPlan>(
            r#"
            UPDATE agent_task_plans
            SET current_step = ?2, updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(step)
        .fetch_one(pool)
        .await?;

        Ok(plan)
    }

    /// Update plan steps
    pub async fn update_steps(
        pool: &SqlitePool,
        id: Uuid,
        steps: Vec<PlanStep>,
    ) -> Result<Self, AgentTaskPlanError> {
        let plan_json = serde_json::to_string(&steps)
            .map_err(|e| AgentTaskPlanError::InvalidPlanJson(e.to_string()))?;
        let total_steps = steps.len() as i32;

        let plan = sqlx::query_as::<_, AgentTaskPlan>(
            r#"
            UPDATE agent_task_plans
            SET plan_json = ?2, total_steps = ?3, updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&plan_json)
        .bind(total_steps)
        .fetch_one(pool)
        .await?;

        Ok(plan)
    }

    /// Parse the plan_json into steps
    pub fn steps(&self) -> Result<Vec<PlanStep>, AgentTaskPlanError> {
        serde_json::from_str(&self.plan_json)
            .map_err(|e| AgentTaskPlanError::InvalidPlanJson(e.to_string()))
    }

    /// Get progress as percentage
    pub fn progress_percent(&self) -> f32 {
        match (self.current_step, self.total_steps) {
            (Some(current), Some(total)) if total > 0 => (current as f32 / total as f32) * 100.0,
            _ => 0.0,
        }
    }
}
