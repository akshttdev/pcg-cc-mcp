//! Execution Control Service
//!
//! Manages pause/resume, context injection, and handoffs between humans and agents.
//! Enables true human-agent collaboration during task execution.

use db::{
    models::{
        context_injection::{ContextInjection, ContextInjectionError, CreateContextInjection, InjectionType},
        execution_handoff::{ActorType, CreateExecutionHandoff, ExecutionHandoff, ExecutionHandoffError, HandoffType},
        execution_pause_history::{ExecutionPauseHistory, ExecutionPauseHistoryError},
    },
    DBService,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ExecutionControlError {
    #[error(transparent)]
    InjectionError(#[from] ContextInjectionError),
    #[error(transparent)]
    HandoffError(#[from] ExecutionHandoffError),
    #[error(transparent)]
    PauseHistoryError(#[from] ExecutionPauseHistoryError),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Execution not found: {0}")]
    ExecutionNotFound(Uuid),
    #[error("Execution already paused")]
    AlreadyPaused,
    #[error("Execution not paused")]
    NotPaused,
    #[error("Execution under human control")]
    UnderHumanControl,
}

/// Control state for an execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "snake_case")]
pub enum ControlState {
    Running,
    Paused,
    HumanTakeover,
    AwaitingInput,
}

impl std::fmt::Display for ControlState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControlState::Running => write!(f, "running"),
            ControlState::Paused => write!(f, "paused"),
            ControlState::HumanTakeover => write!(f, "human_takeover"),
            ControlState::AwaitingInput => write!(f, "awaiting_input"),
        }
    }
}

/// Summary of collaboration state for an execution
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct CollaborationState {
    pub execution_process_id: Uuid,
    pub control_state: ControlState,
    pub current_controller: Option<ActorInfo>,
    pub pending_injections: Vec<ContextInjection>,
    pub recent_handoffs: Vec<ExecutionHandoff>,
    pub pause_history: Vec<ExecutionPauseHistory>,
}

/// Actor information
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ActorInfo {
    pub actor_type: ActorType,
    pub actor_id: String,
    pub actor_name: Option<String>,
}

/// Request for pausing an execution
#[derive(Debug, Deserialize, TS)]
pub struct PauseRequest {
    pub execution_process_id: Uuid,
    pub reason: Option<String>,
    pub initiated_by: String,
    pub initiated_by_name: Option<String>,
}

/// Request for resuming an execution
#[derive(Debug, Deserialize, TS)]
pub struct ResumeRequest {
    pub execution_process_id: Uuid,
    pub initiated_by: String,
    pub initiated_by_name: Option<String>,
}

/// Request for human takeover
#[derive(Debug, Deserialize, TS)]
pub struct TakeoverRequest {
    pub execution_process_id: Uuid,
    pub human_id: String,
    pub human_name: Option<String>,
    pub reason: Option<String>,
}

/// Request to return control to agent
#[derive(Debug, Deserialize, TS)]
pub struct ReturnControlRequest {
    pub execution_process_id: Uuid,
    pub human_id: String,
    pub human_name: Option<String>,
    pub to_agent_id: String,
    pub to_agent_name: Option<String>,
    pub context_notes: Option<String>,
}

/// Execution Control Service
#[derive(Clone)]
pub struct ExecutionControlService {
    db: DBService,
}

impl ExecutionControlService {
    pub fn new(db: DBService) -> Self {
        Self { db }
    }

    // ========== Control State ==========

    /// Get current control state of an execution
    pub async fn get_control_state(
        &self,
        execution_process_id: Uuid,
    ) -> Result<ControlState, ExecutionControlError> {
        let row: Option<(String,)> = sqlx::query_as(
            r#"SELECT control_state FROM execution_processes WHERE id = ?1"#,
        )
        .bind(execution_process_id)
        .fetch_optional(&self.db.pool)
        .await
        .map_err(|e| ExecutionControlError::DatabaseError(e.to_string()))?;

        match row {
            Some((state,)) => match state.as_str() {
                "running" => Ok(ControlState::Running),
                "paused" => Ok(ControlState::Paused),
                "human_takeover" => Ok(ControlState::HumanTakeover),
                "awaiting_input" => Ok(ControlState::AwaitingInput),
                _ => Ok(ControlState::Running),
            },
            None => Err(ExecutionControlError::ExecutionNotFound(execution_process_id)),
        }
    }

    /// Set control state of an execution
    async fn set_control_state(
        &self,
        execution_process_id: Uuid,
        state: ControlState,
        reason: Option<String>,
    ) -> Result<(), ExecutionControlError> {
        let state_str = state.to_string();

        if state == ControlState::Paused {
            sqlx::query(
                r#"
                UPDATE execution_processes
                SET control_state = ?2, paused_at = datetime('now', 'subsec'), pause_reason = ?3
                WHERE id = ?1
                "#,
            )
            .bind(execution_process_id)
            .bind(&state_str)
            .bind(reason)
            .execute(&self.db.pool)
            .await
            .map_err(|e| ExecutionControlError::DatabaseError(e.to_string()))?;
        } else {
            sqlx::query(
                r#"
                UPDATE execution_processes
                SET control_state = ?2, paused_at = NULL, pause_reason = NULL
                WHERE id = ?1
                "#,
            )
            .bind(execution_process_id)
            .bind(&state_str)
            .execute(&self.db.pool)
            .await
            .map_err(|e| ExecutionControlError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    // ========== Pause/Resume ==========

    /// Pause an execution
    pub async fn pause(&self, req: PauseRequest) -> Result<ExecutionPauseHistory, ExecutionControlError> {
        let current = self.get_control_state(req.execution_process_id).await?;

        if current == ControlState::Paused {
            return Err(ExecutionControlError::AlreadyPaused);
        }

        if current == ControlState::HumanTakeover {
            return Err(ExecutionControlError::UnderHumanControl);
        }

        // Update control state
        self.set_control_state(req.execution_process_id, ControlState::Paused, req.reason.clone())
            .await?;

        // Record in pause history
        let entry = ExecutionPauseHistory::record_pause(
            &self.db.pool,
            req.execution_process_id,
            req.reason,
            req.initiated_by,
            req.initiated_by_name,
        )
        .await?;

        Ok(entry)
    }

    /// Resume an execution
    pub async fn resume(&self, req: ResumeRequest) -> Result<ExecutionPauseHistory, ExecutionControlError> {
        let current = self.get_control_state(req.execution_process_id).await?;

        if current == ControlState::Running {
            return Err(ExecutionControlError::NotPaused);
        }

        if current == ControlState::HumanTakeover {
            return Err(ExecutionControlError::UnderHumanControl);
        }

        // Update control state
        self.set_control_state(req.execution_process_id, ControlState::Running, None)
            .await?;

        // Record in pause history
        let entry = ExecutionPauseHistory::record_resume(
            &self.db.pool,
            req.execution_process_id,
            None,
            req.initiated_by,
            req.initiated_by_name,
        )
        .await?;

        Ok(entry)
    }

    /// Get pause history for an execution
    pub async fn get_pause_history(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<ExecutionPauseHistory>, ExecutionControlError> {
        let entries = ExecutionPauseHistory::find_by_execution(&self.db.pool, execution_process_id).await?;
        Ok(entries)
    }

    // ========== Human Takeover ==========

    /// Human takes over control from agent
    pub async fn human_takeover(&self, req: TakeoverRequest) -> Result<ExecutionHandoff, ExecutionControlError> {
        let current = self.get_control_state(req.execution_process_id).await?;

        if current == ControlState::HumanTakeover {
            return Err(ExecutionControlError::UnderHumanControl);
        }

        // Get current controller (from latest handoff or default to system)
        let latest_handoff = ExecutionHandoff::find_latest(&self.db.pool, req.execution_process_id).await?;
        let (from_type, from_id, from_name) = match latest_handoff {
            Some(h) => (h.to_actor_type, h.to_actor_id, h.to_actor_name),
            None => (ActorType::System, "system".to_string(), Some("System".to_string())),
        };

        // Update control state
        self.set_control_state(req.execution_process_id, ControlState::HumanTakeover, req.reason.clone())
            .await?;

        // Create handoff record
        let handoff = ExecutionHandoff::create(
            &self.db.pool,
            CreateExecutionHandoff {
                execution_process_id: req.execution_process_id,
                from_actor_type: from_type,
                from_actor_id: from_id,
                from_actor_name: from_name,
                to_actor_type: ActorType::Human,
                to_actor_id: req.human_id,
                to_actor_name: req.human_name,
                handoff_type: HandoffType::Takeover,
                reason: req.reason,
                context_snapshot: None,
            },
        )
        .await?;

        Ok(handoff)
    }

    /// Return control from human to agent
    pub async fn return_control(&self, req: ReturnControlRequest) -> Result<ExecutionHandoff, ExecutionControlError> {
        let current = self.get_control_state(req.execution_process_id).await?;

        if current != ControlState::HumanTakeover {
            return Err(ExecutionControlError::NotPaused);
        }

        // Update control state to running
        self.set_control_state(req.execution_process_id, ControlState::Running, None)
            .await?;

        // Build context snapshot with notes if provided
        let context = req.context_notes.map(|notes| {
            serde_json::json!({
                "human_notes": notes,
                "returned_at": chrono::Utc::now().to_rfc3339(),
            })
        });

        // Create handoff record
        let handoff = ExecutionHandoff::create(
            &self.db.pool,
            CreateExecutionHandoff {
                execution_process_id: req.execution_process_id,
                from_actor_type: ActorType::Human,
                from_actor_id: req.human_id,
                from_actor_name: req.human_name,
                to_actor_type: ActorType::Agent,
                to_actor_id: req.to_agent_id,
                to_actor_name: req.to_agent_name,
                handoff_type: HandoffType::Return,
                reason: Some("Human completed work".to_string()),
                context_snapshot: context,
            },
        )
        .await?;

        Ok(handoff)
    }

    /// Get handoffs for an execution
    pub async fn get_handoffs(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<ExecutionHandoff>, ExecutionControlError> {
        let handoffs = ExecutionHandoff::find_by_execution(&self.db.pool, execution_process_id).await?;
        Ok(handoffs)
    }

    // ========== Context Injections ==========

    /// Inject context/note into an execution
    pub async fn inject_context(
        &self,
        execution_process_id: Uuid,
        injector_id: String,
        injector_name: Option<String>,
        injection_type: InjectionType,
        content: String,
        metadata: Option<Value>,
    ) -> Result<ContextInjection, ExecutionControlError> {
        let injection = ContextInjection::create(
            &self.db.pool,
            CreateContextInjection {
                execution_process_id,
                injector_id,
                injector_name,
                injection_type,
                content,
                metadata,
            },
        )
        .await?;

        Ok(injection)
    }

    /// Get all context injections for an execution
    pub async fn get_injections(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<ContextInjection>, ExecutionControlError> {
        let injections = ContextInjection::find_by_execution(&self.db.pool, execution_process_id).await?;
        Ok(injections)
    }

    /// Get unacknowledged injections for an execution
    pub async fn get_pending_injections(
        &self,
        execution_process_id: Uuid,
    ) -> Result<Vec<ContextInjection>, ExecutionControlError> {
        let injections = ContextInjection::find_unacknowledged(&self.db.pool, execution_process_id).await?;
        Ok(injections)
    }

    /// Acknowledge an injection
    pub async fn acknowledge_injection(
        &self,
        injection_id: Uuid,
    ) -> Result<ContextInjection, ExecutionControlError> {
        let injection = ContextInjection::acknowledge(&self.db.pool, injection_id).await?;
        Ok(injection)
    }

    /// Acknowledge all injections for an execution
    pub async fn acknowledge_all_injections(
        &self,
        execution_process_id: Uuid,
    ) -> Result<u64, ExecutionControlError> {
        let count = ContextInjection::acknowledge_all(&self.db.pool, execution_process_id).await?;
        Ok(count)
    }

    // ========== Full State ==========

    /// Get full collaboration state for an execution
    pub async fn get_collaboration_state(
        &self,
        execution_process_id: Uuid,
    ) -> Result<CollaborationState, ExecutionControlError> {
        let control_state = self.get_control_state(execution_process_id).await?;

        // Get current controller from latest handoff
        let latest_handoff = ExecutionHandoff::find_latest(&self.db.pool, execution_process_id).await?;
        let current_controller = latest_handoff.map(|h| ActorInfo {
            actor_type: h.to_actor_type,
            actor_id: h.to_actor_id,
            actor_name: h.to_actor_name,
        });

        // Get pending injections
        let pending_injections = self.get_pending_injections(execution_process_id).await?;

        // Get recent handoffs (last 10)
        let all_handoffs = self.get_handoffs(execution_process_id).await?;
        let recent_handoffs: Vec<_> = all_handoffs.into_iter().rev().take(10).collect();

        // Get pause history
        let pause_history = self.get_pause_history(execution_process_id).await?;

        Ok(CollaborationState {
            execution_process_id,
            control_state,
            current_controller,
            pending_injections,
            recent_handoffs,
            pause_history,
        })
    }
}
