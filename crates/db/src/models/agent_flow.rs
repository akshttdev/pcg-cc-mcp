use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AgentFlowError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Agent flow not found")]
    NotFound,
    #[error("Invalid flow state transition: {0}")]
    InvalidTransition(String),
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "flow_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FlowType {
    ContentCreation,
    Research,
    Engagement,
    Scheduling,
    Campaign,
    Analysis,
    Monitoring,
    Custom,
}

impl std::fmt::Display for FlowType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowType::ContentCreation => write!(f, "content_creation"),
            FlowType::Research => write!(f, "research"),
            FlowType::Engagement => write!(f, "engagement"),
            FlowType::Scheduling => write!(f, "scheduling"),
            FlowType::Campaign => write!(f, "campaign"),
            FlowType::Analysis => write!(f, "analysis"),
            FlowType::Monitoring => write!(f, "monitoring"),
            FlowType::Custom => write!(f, "custom"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "flow_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FlowStatus {
    Planning,
    Executing,
    Verifying,
    Completed,
    Failed,
    Paused,
    AwaitingApproval,
}

impl std::fmt::Display for FlowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowStatus::Planning => write!(f, "planning"),
            FlowStatus::Executing => write!(f, "executing"),
            FlowStatus::Verifying => write!(f, "verifying"),
            FlowStatus::Completed => write!(f, "completed"),
            FlowStatus::Failed => write!(f, "failed"),
            FlowStatus::Paused => write!(f, "paused"),
            FlowStatus::AwaitingApproval => write!(f, "awaiting_approval"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "agent_phase", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum AgentPhase {
    Planning,
    Execution,
    Verification,
}

impl std::fmt::Display for AgentPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentPhase::Planning => write!(f, "planning"),
            AgentPhase::Execution => write!(f, "execution"),
            AgentPhase::Verification => write!(f, "verification"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentFlow {
    pub id: Uuid,
    pub task_id: Uuid,
    pub flow_type: FlowType,
    pub status: FlowStatus,

    // Phase agents (Planner → Executor → Verifier)
    pub planner_agent_id: Option<Uuid>,
    pub executor_agent_id: Option<Uuid>,
    pub verifier_agent_id: Option<Uuid>,

    pub current_phase: AgentPhase,

    // Phase timestamps
    pub planning_started_at: Option<DateTime<Utc>>,
    pub planning_completed_at: Option<DateTime<Utc>>,
    pub execution_started_at: Option<DateTime<Utc>>,
    pub execution_completed_at: Option<DateTime<Utc>>,
    pub verification_started_at: Option<DateTime<Utc>>,
    pub verification_completed_at: Option<DateTime<Utc>>,

    // Configuration
    #[sqlx(default)]
    pub flow_config: Option<String>, // JSON
    pub handoff_instructions: Option<String>,

    // Quality tracking
    pub verification_score: Option<f64>,
    pub human_approval_required: bool,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateAgentFlow {
    pub task_id: Uuid,
    pub flow_type: FlowType,
    pub planner_agent_id: Option<Uuid>,
    pub executor_agent_id: Option<Uuid>,
    pub verifier_agent_id: Option<Uuid>,
    pub flow_config: Option<Value>,
    pub human_approval_required: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateAgentFlow {
    pub status: Option<FlowStatus>,
    pub current_phase: Option<AgentPhase>,
    pub planner_agent_id: Option<Uuid>,
    pub executor_agent_id: Option<Uuid>,
    pub verifier_agent_id: Option<Uuid>,
    pub handoff_instructions: Option<String>,
    pub verification_score: Option<f64>,
    pub approved_by: Option<String>,
}

impl AgentFlow {
    /// Create a new agent flow
    pub async fn create(
        pool: &SqlitePool,
        data: CreateAgentFlow,
    ) -> Result<Self, AgentFlowError> {
        let id = Uuid::new_v4();
        let flow_type_str = data.flow_type.to_string();
        let flow_config_str = data.flow_config.map(|v| v.to_string());
        let human_approval = data.human_approval_required.unwrap_or(false);

        let flow = sqlx::query_as::<_, AgentFlow>(
            r#"
            INSERT INTO agent_flows (
                id, task_id, flow_type, status, current_phase,
                planner_agent_id, executor_agent_id, verifier_agent_id,
                flow_config, human_approval_required, planning_started_at
            )
            VALUES (?1, ?2, ?3, 'planning', 'planning', ?4, ?5, ?6, ?7, ?8, datetime('now', 'subsec'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.task_id)
        .bind(flow_type_str)
        .bind(data.planner_agent_id)
        .bind(data.executor_agent_id)
        .bind(data.verifier_agent_id)
        .bind(flow_config_str)
        .bind(human_approval)
        .fetch_one(pool)
        .await?;

        Ok(flow)
    }

    /// Find flow by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, AgentFlowError> {
        let flow = sqlx::query_as::<_, AgentFlow>(
            r#"SELECT * FROM agent_flows WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(flow)
    }

    /// Find all flows for a task
    pub async fn find_by_task(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Vec<Self>, AgentFlowError> {
        let flows = sqlx::query_as::<_, AgentFlow>(
            r#"
            SELECT * FROM agent_flows
            WHERE task_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(task_id)
        .fetch_all(pool)
        .await?;

        Ok(flows)
    }

    /// Find flows by status
    pub async fn find_by_status(
        pool: &SqlitePool,
        status: FlowStatus,
    ) -> Result<Vec<Self>, AgentFlowError> {
        let status_str = status.to_string();

        let flows = sqlx::query_as::<_, AgentFlow>(
            r#"
            SELECT * FROM agent_flows
            WHERE status = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(status_str)
        .fetch_all(pool)
        .await?;

        Ok(flows)
    }

    /// Find flows awaiting approval
    pub async fn find_awaiting_approval(
        pool: &SqlitePool,
    ) -> Result<Vec<Self>, AgentFlowError> {
        let flows = sqlx::query_as::<_, AgentFlow>(
            r#"
            SELECT * FROM agent_flows
            WHERE status = 'awaiting_approval'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(flows)
    }

    /// Transition to next phase
    pub async fn transition_to_phase(
        pool: &SqlitePool,
        id: Uuid,
        phase: AgentPhase,
    ) -> Result<Self, AgentFlowError> {
        let phase_str = phase.to_string();
        let status_str = match phase {
            AgentPhase::Planning => "planning",
            AgentPhase::Execution => "executing",
            AgentPhase::Verification => "verifying",
        };

        // Set the appropriate started_at timestamp
        let query = match phase {
            AgentPhase::Planning => {
                r#"
                UPDATE agent_flows
                SET current_phase = ?2, status = ?3,
                    planning_started_at = datetime('now', 'subsec'),
                    updated_at = datetime('now', 'subsec')
                WHERE id = ?1
                RETURNING *
                "#
            }
            AgentPhase::Execution => {
                r#"
                UPDATE agent_flows
                SET current_phase = ?2, status = ?3,
                    planning_completed_at = datetime('now', 'subsec'),
                    execution_started_at = datetime('now', 'subsec'),
                    updated_at = datetime('now', 'subsec')
                WHERE id = ?1
                RETURNING *
                "#
            }
            AgentPhase::Verification => {
                r#"
                UPDATE agent_flows
                SET current_phase = ?2, status = ?3,
                    execution_completed_at = datetime('now', 'subsec'),
                    verification_started_at = datetime('now', 'subsec'),
                    updated_at = datetime('now', 'subsec')
                WHERE id = ?1
                RETURNING *
                "#
            }
        };

        let flow = sqlx::query_as::<_, AgentFlow>(query)
            .bind(id)
            .bind(phase_str)
            .bind(status_str)
            .fetch_one(pool)
            .await?;

        Ok(flow)
    }

    /// Complete the flow
    pub async fn complete(
        pool: &SqlitePool,
        id: Uuid,
        verification_score: Option<f64>,
    ) -> Result<Self, AgentFlowError> {
        let flow = sqlx::query_as::<_, AgentFlow>(
            r#"
            UPDATE agent_flows
            SET status = 'completed',
                verification_completed_at = datetime('now', 'subsec'),
                verification_score = ?2,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(verification_score)
        .fetch_one(pool)
        .await?;

        Ok(flow)
    }

    /// Request approval
    pub async fn request_approval(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Self, AgentFlowError> {
        let flow = sqlx::query_as::<_, AgentFlow>(
            r#"
            UPDATE agent_flows
            SET status = 'awaiting_approval',
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(flow)
    }

    /// Approve the flow
    pub async fn approve(
        pool: &SqlitePool,
        id: Uuid,
        approved_by: &str,
    ) -> Result<Self, AgentFlowError> {
        let flow = sqlx::query_as::<_, AgentFlow>(
            r#"
            UPDATE agent_flows
            SET approved_by = ?2,
                approved_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(approved_by)
        .fetch_one(pool)
        .await?;

        Ok(flow)
    }

    /// Parse flow_config as JSON Value
    pub fn config_json(&self) -> Option<Value> {
        self.flow_config
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }
}
