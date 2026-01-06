use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ExecutionHandoffError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Handoff not found")]
    NotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "actor_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    Agent,
    Human,
    System,
}

impl std::fmt::Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorType::Agent => write!(f, "agent"),
            ActorType::Human => write!(f, "human"),
            ActorType::System => write!(f, "system"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "handoff_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum HandoffType {
    Takeover,
    Return,
    Escalation,
    Delegation,
    Assistance,
    ReviewRequest,
}

impl std::fmt::Display for HandoffType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandoffType::Takeover => write!(f, "takeover"),
            HandoffType::Return => write!(f, "return"),
            HandoffType::Escalation => write!(f, "escalation"),
            HandoffType::Delegation => write!(f, "delegation"),
            HandoffType::Assistance => write!(f, "assistance"),
            HandoffType::ReviewRequest => write!(f, "review_request"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ExecutionHandoff {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub from_actor_type: ActorType,
    pub from_actor_id: String,
    pub from_actor_name: Option<String>,
    pub to_actor_type: ActorType,
    pub to_actor_id: String,
    pub to_actor_name: Option<String>,
    pub handoff_type: HandoffType,
    pub reason: Option<String>,
    #[sqlx(default)]
    pub context_snapshot: Option<String>, // JSON
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateExecutionHandoff {
    pub execution_process_id: Uuid,
    pub from_actor_type: ActorType,
    pub from_actor_id: String,
    pub from_actor_name: Option<String>,
    pub to_actor_type: ActorType,
    pub to_actor_id: String,
    pub to_actor_name: Option<String>,
    pub handoff_type: HandoffType,
    pub reason: Option<String>,
    pub context_snapshot: Option<Value>,
}

impl ExecutionHandoff {
    /// Create a new handoff record
    pub async fn create(
        pool: &SqlitePool,
        data: CreateExecutionHandoff,
    ) -> Result<Self, ExecutionHandoffError> {
        let id = Uuid::new_v4();
        let from_actor_type_str = data.from_actor_type.to_string();
        let to_actor_type_str = data.to_actor_type.to_string();
        let handoff_type_str = data.handoff_type.to_string();
        let context_str = data.context_snapshot.map(|v| v.to_string());

        let handoff = sqlx::query_as::<_, ExecutionHandoff>(
            r#"
            INSERT INTO execution_handoffs (
                id, execution_process_id,
                from_actor_type, from_actor_id, from_actor_name,
                to_actor_type, to_actor_id, to_actor_name,
                handoff_type, reason, context_snapshot
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(from_actor_type_str)
        .bind(&data.from_actor_id)
        .bind(&data.from_actor_name)
        .bind(to_actor_type_str)
        .bind(&data.to_actor_id)
        .bind(&data.to_actor_name)
        .bind(handoff_type_str)
        .bind(&data.reason)
        .bind(context_str)
        .fetch_one(pool)
        .await?;

        Ok(handoff)
    }

    /// Find handoff by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, ExecutionHandoffError> {
        let handoff = sqlx::query_as::<_, ExecutionHandoff>(
            r#"SELECT * FROM execution_handoffs WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(handoff)
    }

    /// Find all handoffs for an execution
    pub async fn find_by_execution(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ExecutionHandoffError> {
        let handoffs = sqlx::query_as::<_, ExecutionHandoff>(
            r#"
            SELECT * FROM execution_handoffs
            WHERE execution_process_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(handoffs)
    }

    /// Find latest handoff for an execution
    pub async fn find_latest(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Option<Self>, ExecutionHandoffError> {
        let handoff = sqlx::query_as::<_, ExecutionHandoff>(
            r#"
            SELECT * FROM execution_handoffs
            WHERE execution_process_id = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(execution_process_id)
        .fetch_optional(pool)
        .await?;

        Ok(handoff)
    }

    /// Find handoffs to a specific actor
    pub async fn find_to_actor(
        pool: &SqlitePool,
        actor_type: ActorType,
        actor_id: &str,
    ) -> Result<Vec<Self>, ExecutionHandoffError> {
        let actor_type_str = actor_type.to_string();
        let handoffs = sqlx::query_as::<_, ExecutionHandoff>(
            r#"
            SELECT * FROM execution_handoffs
            WHERE to_actor_type = ?1 AND to_actor_id = ?2
            ORDER BY created_at DESC
            "#,
        )
        .bind(actor_type_str)
        .bind(actor_id)
        .fetch_all(pool)
        .await?;

        Ok(handoffs)
    }

    /// Parse context_snapshot as JSON Value
    pub fn context_snapshot_json(&self) -> Option<Value> {
        self.context_snapshot
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Check if this is a human takeover
    pub fn is_human_takeover(&self) -> bool {
        self.to_actor_type == ActorType::Human && self.handoff_type == HandoffType::Takeover
    }

    /// Check if this returns control to agent
    pub fn is_agent_return(&self) -> bool {
        self.to_actor_type == ActorType::Agent && self.handoff_type == HandoffType::Return
    }
}
