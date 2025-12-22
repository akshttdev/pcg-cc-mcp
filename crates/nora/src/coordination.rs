//! Multi-agent coordination capabilities for Nora

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use ts_rs::TS;
use uuid::Uuid;

use crate::NoraError;

/// Coordination events for multi-agent systems
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CoordinationEvent {
    /// Agent status update
    AgentStatusUpdate {
        agent_id: String,
        status: AgentStatus,
        capabilities: Vec<String>,
        timestamp: DateTime<Utc>,
    },
    /// Task handoff between agents
    TaskHandoff {
        from_agent: String,
        to_agent: String,
        task_id: String,
        context: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
    /// Conflict resolution request
    ConflictResolution {
        conflict_id: String,
        involved_agents: Vec<String>,
        description: String,
        priority: ConflictPriority,
        timestamp: DateTime<Utc>,
    },
    /// Human availability update
    HumanAvailabilityUpdate {
        user_id: String,
        availability: AvailabilityStatus,
        available_until: Option<DateTime<Utc>>,
        timestamp: DateTime<Utc>,
    },
    /// Approval request
    ApprovalRequest {
        request_id: String,
        requesting_agent: String,
        action_description: String,
        required_approver: String,
        urgency: ApprovalUrgency,
        timestamp: DateTime<Utc>,
    },
    /// Executive alert
    ExecutiveAlert {
        alert_id: String,
        source: String,
        message: String,
        severity: AlertSeverity,
        requires_action: bool,
        timestamp: DateTime<Utc>,
    },
    /// Directive issued to a specific agent from the global console
    AgentDirectiveIssued {
        agent_id: String,
        issued_by: String,
        content: String,
        priority: Option<String>,
        timestamp: DateTime<Utc>,
    },
}

/// Agent status types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AgentStatus {
    Active,
    Busy,
    Idle,
    Offline,
    Error,
    Maintenance,
}

/// Human availability status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AvailabilityStatus {
    Available,
    Busy,
    InMeeting,
    DoNotDisturb,
    Away,
    Offline,
}

/// Conflict resolution priority
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ConflictPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Approval urgency levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ApprovalUrgency {
    Low,
    Normal,
    High,
    Urgent,
    Emergency,
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Agent coordination state
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentCoordinationState {
    pub agent_id: String,
    pub agent_type: String,
    pub status: AgentStatus,
    pub capabilities: Vec<String>,
    pub current_tasks: Vec<String>,
    pub last_seen: DateTime<Utc>,
    pub performance_metrics: PerformanceMetrics,
}

/// Performance metrics for agents
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetrics {
    pub tasks_completed: u64,
    pub average_response_time_ms: f64,
    pub success_rate: f32,
    pub uptime_percentage: f32,
}

/// Coordination manager for multi-agent systems
#[derive(Debug)]
pub struct CoordinationManager {
    agents: Arc<RwLock<HashMap<String, AgentCoordinationState>>>,
    event_sender: broadcast::Sender<CoordinationEvent>,
    pending_approvals: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
    active_conflicts: Arc<RwLock<HashMap<String, ConflictResolution>>>,
}

/// Approval request structure
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub request_id: String,
    pub requesting_agent: String,
    pub action_description: String,
    pub required_approver: String,
    pub urgency: ApprovalUrgency,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub context: serde_json::Value,
}

/// Conflict resolution structure
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolution {
    pub conflict_id: String,
    pub involved_agents: Vec<String>,
    pub description: String,
    pub priority: ConflictPriority,
    pub created_at: DateTime<Utc>,
    pub resolution_strategy: Option<ResolutionStrategy>,
    pub status: ConflictStatus,
}

/// Resolution strategies for conflicts
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionStrategy {
    Escalate,
    Negotiate,
    Override,
    Queue,
    Reassign,
}

/// Conflict resolution status
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ConflictStatus {
    Pending,
    InProgress,
    Resolved,
    Escalated,
}

impl CoordinationManager {
    /// Create a new coordination manager
    pub async fn new() -> crate::Result<Self> {
        let (event_sender, _) = broadcast::channel(1000);

        Ok(Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            pending_approvals: Arc::new(RwLock::new(HashMap::new())),
            active_conflicts: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Register an agent with the coordination system
    pub async fn register_agent(&self, agent_state: AgentCoordinationState) -> crate::Result<()> {
        let mut agents = self.agents.write().await;
        agents.insert(agent_state.agent_id.clone(), agent_state.clone());

        // Broadcast agent registration
        let event = CoordinationEvent::AgentStatusUpdate {
            agent_id: agent_state.agent_id,
            status: agent_state.status,
            capabilities: agent_state.capabilities,
            timestamp: Utc::now(),
        };

        let _ = self.event_sender.send(event);

        Ok(())
    }

    /// Update agent status
    pub async fn update_agent_status(
        &self,
        agent_id: &str,
        status: AgentStatus,
    ) -> crate::Result<()> {
        let mut agents = self.agents.write().await;

        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = status.clone();
            agent.last_seen = Utc::now();

            // Broadcast status update
            let event = CoordinationEvent::AgentStatusUpdate {
                agent_id: agent_id.to_string(),
                status,
                capabilities: agent.capabilities.clone(),
                timestamp: Utc::now(),
            };

            let _ = self.event_sender.send(event);
        }

        Ok(())
    }

    /// Get all registered agents
    pub async fn get_all_agents(&self) -> crate::Result<Vec<AgentCoordinationState>> {
        let agents = self.agents.read().await;
        Ok(agents.values().cloned().collect())
    }

    /// Get agents by capability
    pub async fn get_agents_by_capability(
        &self,
        capability: &str,
    ) -> crate::Result<Vec<AgentCoordinationState>> {
        let agents = self.agents.read().await;
        Ok(agents
            .values()
            .filter(|agent| agent.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect())
    }

    /// Fetch a single agent snapshot by identifier
    pub async fn get_agent(&self, agent_id: &str) -> crate::Result<Option<AgentCoordinationState>> {
        let agents = self.agents.read().await;
        Ok(agents.get(agent_id).cloned())
    }

    /// Record a directive issued to an agent and broadcast it as an event
    pub async fn record_directive(
        &self,
        agent_id: &str,
        issued_by: &str,
        content: &str,
        priority: Option<String>,
    ) -> crate::Result<AgentCoordinationState> {
        let mut agents = self.agents.write().await;

        let agent = agents.get_mut(agent_id).ok_or_else(|| {
            NoraError::CoordinationError(format!("Agent {} not registered", agent_id))
        })?;

        agent.last_seen = Utc::now();
        agent.current_tasks.insert(0, content.to_string());
        if agent.current_tasks.len() > 8 {
            agent.current_tasks.pop();
        }

        let updated_agent = agent.clone();

        let event = CoordinationEvent::AgentDirectiveIssued {
            agent_id: agent_id.to_string(),
            issued_by: issued_by.to_string(),
            content: content.to_string(),
            priority,
            timestamp: Utc::now(),
        };
        let _ = self.event_sender.send(event);

        Ok(updated_agent)
    }

    /// Request task handoff between agents
    pub async fn request_task_handoff(
        &self,
        from_agent: String,
        to_agent: String,
        task_id: String,
        context: serde_json::Value,
    ) -> crate::Result<()> {
        let event = CoordinationEvent::TaskHandoff {
            from_agent,
            to_agent,
            task_id,
            context,
            timestamp: Utc::now(),
        };

        let _ = self.event_sender.send(event);

        Ok(())
    }

    /// Request approval for an action
    pub async fn request_approval(&self, request: ApprovalRequest) -> crate::Result<String> {
        let request_id = request.request_id.clone();

        // Store pending approval
        {
            let mut approvals = self.pending_approvals.write().await;
            approvals.insert(request_id.clone(), request.clone());
        }

        // Broadcast approval request
        let event = CoordinationEvent::ApprovalRequest {
            request_id: request.request_id,
            requesting_agent: request.requesting_agent,
            action_description: request.action_description,
            required_approver: request.required_approver,
            urgency: request.urgency,
            timestamp: Utc::now(),
        };

        let _ = self.event_sender.send(event);

        Ok(request_id)
    }

    /// Approve or deny a pending request
    pub async fn respond_to_approval(
        &self,
        request_id: &str,
        approved: bool,
        approver: &str,
    ) -> crate::Result<()> {
        let mut approvals = self.pending_approvals.write().await;

        if let Some(_request) = approvals.remove(request_id) {
            // Handle approval/denial logic here
            tracing::info!(
                "Approval request {} {} by {}",
                request_id,
                if approved { "approved" } else { "denied" },
                approver
            );

            // You could broadcast an approval response event here
            // let event = CoordinationEvent::ApprovalResponse { ... };
            // let _ = self.event_sender.send(event);
        }

        Ok(())
    }

    /// Report a conflict that needs resolution
    pub async fn report_conflict(&self, conflict: ConflictResolution) -> crate::Result<String> {
        let conflict_id = conflict.conflict_id.clone();

        // Store active conflict
        {
            let mut conflicts = self.active_conflicts.write().await;
            conflicts.insert(conflict_id.clone(), conflict.clone());
        }

        // Broadcast conflict
        let event = CoordinationEvent::ConflictResolution {
            conflict_id: conflict.conflict_id,
            involved_agents: conflict.involved_agents,
            description: conflict.description,
            priority: conflict.priority,
            timestamp: Utc::now(),
        };

        let _ = self.event_sender.send(event);

        Ok(conflict_id)
    }

    /// Send executive alert
    pub async fn send_executive_alert(
        &self,
        source: String,
        message: String,
        severity: AlertSeverity,
        requires_action: bool,
    ) -> crate::Result<()> {
        let event = CoordinationEvent::ExecutiveAlert {
            alert_id: Uuid::new_v4().to_string(),
            source,
            message,
            severity,
            requires_action,
            timestamp: Utc::now(),
        };

        let _ = self.event_sender.send(event);

        Ok(())
    }

    /// Emit a raw coordination event for external callers
    pub async fn emit_event(&self, event: CoordinationEvent) -> crate::Result<()> {
        let _ = self.event_sender.send(event);
        Ok(())
    }

    /// Subscribe to coordination events
    pub async fn subscribe_to_events(&self) -> broadcast::Receiver<CoordinationEvent> {
        self.event_sender.subscribe()
    }

    /// Get coordination statistics
    pub async fn get_coordination_stats(&self) -> crate::Result<CoordinationStats> {
        let agents = self.agents.read().await;
        let approvals = self.pending_approvals.read().await;
        let conflicts = self.active_conflicts.read().await;

        Ok(CoordinationStats {
            total_agents: agents.len(),
            active_agents: agents
                .values()
                .filter(|a| matches!(a.status, AgentStatus::Active))
                .count(),
            pending_approvals: approvals.len(),
            active_conflicts: conflicts.len(),
            average_response_time: agents
                .values()
                .map(|a| a.performance_metrics.average_response_time_ms)
                .sum::<f64>()
                / agents.len().max(1) as f64,
        })
    }
}

/// Coordination system statistics
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CoordinationStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub pending_approvals: usize,
    pub active_conflicts: usize,
    pub average_response_time: f64,
}
