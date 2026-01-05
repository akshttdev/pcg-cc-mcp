use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum AgentFlowEventError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Event not found")]
    NotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "flow_event_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum FlowEventType {
    PhaseStarted,
    PhaseCompleted,
    ArtifactCreated,
    ArtifactUpdated,
    ApprovalRequested,
    ApprovalDecision,
    WideResearchStarted,
    SubagentProgress,
    WideResearchCompleted,
    AgentHandoff,
    FlowPaused,
    FlowResumed,
    FlowFailed,
    FlowCompleted,
}

impl std::fmt::Display for FlowEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlowEventType::PhaseStarted => write!(f, "phase_started"),
            FlowEventType::PhaseCompleted => write!(f, "phase_completed"),
            FlowEventType::ArtifactCreated => write!(f, "artifact_created"),
            FlowEventType::ArtifactUpdated => write!(f, "artifact_updated"),
            FlowEventType::ApprovalRequested => write!(f, "approval_requested"),
            FlowEventType::ApprovalDecision => write!(f, "approval_decision"),
            FlowEventType::WideResearchStarted => write!(f, "wide_research_started"),
            FlowEventType::SubagentProgress => write!(f, "subagent_progress"),
            FlowEventType::WideResearchCompleted => write!(f, "wide_research_completed"),
            FlowEventType::AgentHandoff => write!(f, "agent_handoff"),
            FlowEventType::FlowPaused => write!(f, "flow_paused"),
            FlowEventType::FlowResumed => write!(f, "flow_resumed"),
            FlowEventType::FlowFailed => write!(f, "flow_failed"),
            FlowEventType::FlowCompleted => write!(f, "flow_completed"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentFlowEvent {
    pub id: Uuid,
    pub agent_flow_id: Uuid,
    pub event_type: FlowEventType,
    pub event_data: String, // JSON payload
    pub created_at: DateTime<Utc>,
}

// Event payload types for type-safe event data
#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(tag = "type")]
#[ts(export)]
pub enum FlowEventPayload {
    PhaseStarted {
        phase: String,
        agent_id: Option<Uuid>,
    },
    PhaseCompleted {
        phase: String,
        artifacts_produced: Vec<Uuid>,
    },
    ArtifactCreated {
        artifact_id: Uuid,
        artifact_type: String,
        title: String,
        phase: String,
    },
    ArtifactUpdated {
        artifact_id: Uuid,
        changes: Value,
    },
    ApprovalRequested {
        artifact_id: Uuid,
        requested_by_agent: Option<Uuid>,
        approval_type: String,
    },
    ApprovalDecision {
        artifact_id: Uuid,
        decision: String,
        reviewer_id: Option<String>,
        feedback: Option<String>,
    },
    WideResearchStarted {
        session_id: Uuid,
        total_subagents: i32,
    },
    SubagentProgress {
        session_id: Uuid,
        subagent_id: Uuid,
        subagent_index: i32,
        target_item: String,
        status: String,
        result_artifact_id: Option<Uuid>,
    },
    WideResearchCompleted {
        session_id: Uuid,
        aggregated_artifact_id: Uuid,
        total_completed: i32,
        total_failed: i32,
    },
    AgentHandoff {
        from_agent_id: Option<Uuid>,
        to_agent_id: Option<Uuid>,
        from_phase: String,
        to_phase: String,
        handoff_artifact_id: Option<Uuid>,
        instructions: Option<String>,
    },
    FlowPaused {
        reason: Option<String>,
        paused_by: Option<String>,
    },
    FlowResumed {
        resumed_by: Option<String>,
    },
    FlowFailed {
        error: String,
        phase: String,
    },
    FlowCompleted {
        verification_score: Option<f64>,
        total_artifacts: i32,
    },
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateFlowEvent {
    pub agent_flow_id: Uuid,
    pub event_type: FlowEventType,
    pub event_data: FlowEventPayload,
}

impl AgentFlowEvent {
    /// Create a new flow event
    pub async fn create(
        pool: &SqlitePool,
        data: CreateFlowEvent,
    ) -> Result<Self, AgentFlowEventError> {
        let id = Uuid::new_v4();
        let event_type_str = data.event_type.to_string();
        let event_data_str = serde_json::to_string(&data.event_data)
            .map_err(|e| AgentFlowEventError::Database(sqlx::Error::Decode(Box::new(e))))?;

        let event = sqlx::query_as::<_, AgentFlowEvent>(
            r#"
            INSERT INTO agent_flow_events (id, agent_flow_id, event_type, event_data)
            VALUES (?1, ?2, ?3, ?4)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.agent_flow_id)
        .bind(event_type_str)
        .bind(event_data_str)
        .fetch_one(pool)
        .await?;

        Ok(event)
    }

    /// Find events for a flow
    pub async fn find_by_flow(
        pool: &SqlitePool,
        agent_flow_id: Uuid,
    ) -> Result<Vec<Self>, AgentFlowEventError> {
        let events = sqlx::query_as::<_, AgentFlowEvent>(
            r#"
            SELECT * FROM agent_flow_events
            WHERE agent_flow_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(agent_flow_id)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// Find events for a flow since a timestamp (for polling/streaming)
    pub async fn find_since(
        pool: &SqlitePool,
        agent_flow_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<Vec<Self>, AgentFlowEventError> {
        let events = sqlx::query_as::<_, AgentFlowEvent>(
            r#"
            SELECT * FROM agent_flow_events
            WHERE agent_flow_id = ?1 AND created_at > ?2
            ORDER BY created_at ASC
            "#,
        )
        .bind(agent_flow_id)
        .bind(since)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// Find events by type
    pub async fn find_by_type(
        pool: &SqlitePool,
        agent_flow_id: Uuid,
        event_type: FlowEventType,
    ) -> Result<Vec<Self>, AgentFlowEventError> {
        let event_type_str = event_type.to_string();

        let events = sqlx::query_as::<_, AgentFlowEvent>(
            r#"
            SELECT * FROM agent_flow_events
            WHERE agent_flow_id = ?1 AND event_type = ?2
            ORDER BY created_at ASC
            "#,
        )
        .bind(agent_flow_id)
        .bind(event_type_str)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// Find latest events across all flows (for dashboard)
    pub async fn find_latest(
        pool: &SqlitePool,
        limit: i32,
    ) -> Result<Vec<Self>, AgentFlowEventError> {
        let events = sqlx::query_as::<_, AgentFlowEvent>(
            r#"
            SELECT * FROM agent_flow_events
            ORDER BY created_at DESC
            LIMIT ?1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// Parse event_data as typed payload
    pub fn payload(&self) -> Option<FlowEventPayload> {
        serde_json::from_str(&self.event_data).ok()
    }

    /// Parse event_data as generic JSON
    pub fn data_json(&self) -> Option<Value> {
        serde_json::from_str(&self.event_data).ok()
    }

    // Helper methods to emit common events

    /// Emit a phase started event
    pub async fn emit_phase_started(
        pool: &SqlitePool,
        flow_id: Uuid,
        phase: &str,
        agent_id: Option<Uuid>,
    ) -> Result<Self, AgentFlowEventError> {
        Self::create(
            pool,
            CreateFlowEvent {
                agent_flow_id: flow_id,
                event_type: FlowEventType::PhaseStarted,
                event_data: FlowEventPayload::PhaseStarted {
                    phase: phase.to_string(),
                    agent_id,
                },
            },
        )
        .await
    }

    /// Emit an artifact created event
    pub async fn emit_artifact_created(
        pool: &SqlitePool,
        flow_id: Uuid,
        artifact_id: Uuid,
        artifact_type: &str,
        title: &str,
        phase: &str,
    ) -> Result<Self, AgentFlowEventError> {
        Self::create(
            pool,
            CreateFlowEvent {
                agent_flow_id: flow_id,
                event_type: FlowEventType::ArtifactCreated,
                event_data: FlowEventPayload::ArtifactCreated {
                    artifact_id,
                    artifact_type: artifact_type.to_string(),
                    title: title.to_string(),
                    phase: phase.to_string(),
                },
            },
        )
        .await
    }

    /// Emit a subagent progress event
    pub async fn emit_subagent_progress(
        pool: &SqlitePool,
        flow_id: Uuid,
        session_id: Uuid,
        subagent_id: Uuid,
        subagent_index: i32,
        target_item: &str,
        status: &str,
        result_artifact_id: Option<Uuid>,
    ) -> Result<Self, AgentFlowEventError> {
        Self::create(
            pool,
            CreateFlowEvent {
                agent_flow_id: flow_id,
                event_type: FlowEventType::SubagentProgress,
                event_data: FlowEventPayload::SubagentProgress {
                    session_id,
                    subagent_id,
                    subagent_index,
                    target_item: target_item.to_string(),
                    status: status.to_string(),
                    result_artifact_id,
                },
            },
        )
        .await
    }
}
