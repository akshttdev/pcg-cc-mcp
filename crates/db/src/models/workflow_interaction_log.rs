use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

// ── Enums ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InteractionActorType {
    #[default]
    Human,
    Agent,
    System,
}

impl InteractionActorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
            Self::System => "system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InteractionActionType {
    #[default]
    Message,
    Response,
    ToolCall,
    ToolResult,
    TaskCreated,
    TaskUpdated,
    ArtifactCreated,
    CostRecorded,
    ExecutionStarted,
    ExecutionEnded,
    Error,
}

impl InteractionActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::Response => "response",
            Self::ToolCall => "tool_call",
            Self::ToolResult => "tool_result",
            Self::TaskCreated => "task_created",
            Self::TaskUpdated => "task_updated",
            Self::ArtifactCreated => "artifact_created",
            Self::CostRecorded => "cost_recorded",
            Self::ExecutionStarted => "execution_started",
            Self::ExecutionEnded => "execution_ended",
            Self::Error => "error",
        }
    }
}

// ── Struct ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WorkflowInteractionLog {
    pub id: String,
    pub task_id: Option<String>,
    pub task_attempt_id: Option<String>,
    pub workflow_run_id: Option<String>,
    pub session_id: Option<String>,
    pub actor_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub action_type: String,
    pub message: Option<String>,
    pub metadata: Option<String>, // JSON
    pub vibe_cost: Option<f64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model_used: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl WorkflowInteractionLog {
    pub fn metadata_json(&self) -> Option<Value> {
        self.metadata
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }
}

// ── Create ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct CreateWorkflowInteractionLog {
    pub task_id: Option<String>,
    pub task_attempt_id: Option<String>,
    pub workflow_run_id: Option<String>,
    pub session_id: Option<String>,
    pub actor_type: InteractionActorType,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub action_type: InteractionActionType,
    pub message: Option<String>,
    pub metadata: Option<Value>,
    pub vibe_cost: Option<f64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model_used: Option<String>,
}

impl CreateWorkflowInteractionLog {
    pub async fn insert(self, pool: &SqlitePool) -> Result<WorkflowInteractionLog, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let actor_type = self.actor_type.as_str().to_string();
        let action_type = self.action_type.as_str().to_string();
        let metadata_str = self.metadata.map(|v| v.to_string());

        sqlx::query_as::<_, WorkflowInteractionLog>(
            r#"
            INSERT INTO workflow_interaction_logs
                (id, task_id, task_attempt_id, workflow_run_id, session_id,
                 actor_type, actor_id, actor_name, action_type,
                 message, metadata, vibe_cost, input_tokens, output_tokens, model_used)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&self.task_id)
        .bind(&self.task_attempt_id)
        .bind(&self.workflow_run_id)
        .bind(&self.session_id)
        .bind(&actor_type)
        .bind(&self.actor_id)
        .bind(&self.actor_name)
        .bind(&action_type)
        .bind(&self.message)
        .bind(&metadata_str)
        .bind(self.vibe_cost)
        .bind(self.input_tokens)
        .bind(self.output_tokens)
        .bind(&self.model_used)
        .fetch_one(pool)
        .await
    }
}

// ── Queries ───────────────────────────────────────────────────────────────────

pub async fn find_by_task(
    pool: &SqlitePool,
    task_id: &str,
    limit: i64,
) -> Result<Vec<WorkflowInteractionLog>, sqlx::Error> {
    sqlx::query_as::<_, WorkflowInteractionLog>(
        r#"
        SELECT * FROM workflow_interaction_logs
        WHERE task_id = ?
        ORDER BY created_at ASC
        LIMIT ?
        "#,
    )
    .bind(task_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn find_by_session(
    pool: &SqlitePool,
    session_id: &str,
    limit: i64,
) -> Result<Vec<WorkflowInteractionLog>, sqlx::Error> {
    sqlx::query_as::<_, WorkflowInteractionLog>(
        r#"
        SELECT * FROM workflow_interaction_logs
        WHERE session_id = ?
        ORDER BY created_at ASC
        LIMIT ?
        "#,
    )
    .bind(session_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn find_by_task_attempt(
    pool: &SqlitePool,
    task_attempt_id: &str,
    limit: i64,
) -> Result<Vec<WorkflowInteractionLog>, sqlx::Error> {
    sqlx::query_as::<_, WorkflowInteractionLog>(
        r#"
        SELECT * FROM workflow_interaction_logs
        WHERE task_attempt_id = ?
        ORDER BY created_at ASC
        LIMIT ?
        "#,
    )
    .bind(task_attempt_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Convenience: log task creation event
pub async fn log_task_created(
    pool: &SqlitePool,
    task_id: &str,
    session_id: Option<&str>,
    actor_id: &str,
    actor_name: &str,
    actor_type: InteractionActorType,
    task_title: &str,
    agent_assigned: Option<&str>,
) -> Result<(), sqlx::Error> {
    let msg = match agent_assigned {
        Some(a) => format!("Task '{}' created and assigned to {}", task_title, a),
        None => format!("Task '{}' created", task_title),
    };
    let meta = agent_assigned
        .map(|a| serde_json::json!({ "assigned_agent": a, "task_title": task_title }));

    CreateWorkflowInteractionLog {
        task_id: Some(task_id.to_string()),
        session_id: session_id.map(|s| s.to_string()),
        actor_type,
        actor_id: Some(actor_id.to_string()),
        actor_name: Some(actor_name.to_string()),
        action_type: InteractionActionType::TaskCreated,
        message: Some(msg),
        metadata: meta,
        ..Default::default()
    }
    .insert(pool)
    .await
    .map(|_| ())
}

/// Convenience: log VIBE cost event
pub async fn log_cost(
    pool: &SqlitePool,
    task_id: Option<&str>,
    session_id: Option<&str>,
    actor_name: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    vibe_cost: f64,
) -> Result<(), sqlx::Error> {
    let msg = format!(
        "{} used {} ({} in / {} out) — {:.2} VIBE",
        actor_name, model, input_tokens, output_tokens, vibe_cost
    );
    CreateWorkflowInteractionLog {
        task_id: task_id.map(|s| s.to_string()),
        session_id: session_id.map(|s| s.to_string()),
        actor_type: InteractionActorType::Agent,
        actor_name: Some(actor_name.to_string()),
        action_type: InteractionActionType::CostRecorded,
        message: Some(msg),
        vibe_cost: Some(vibe_cost),
        input_tokens: Some(input_tokens),
        output_tokens: Some(output_tokens),
        model_used: Some(model.to_string()),
        ..Default::default()
    }
    .insert(pool)
    .await
    .map(|_| ())
}
