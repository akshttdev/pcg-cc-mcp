//! Agent Flow Orchestration Engine — Phase 1
//!
//! Background worker that polls for actionable agent flows every 15 seconds
//! and dispatches them through the planning → executing → verifying lifecycle.
//!
//! Disabled by default. Enable with `ENABLE_AGENT_FLOW_ENGINE=1`.

use db::models::agent_flow::{AgentFlow, AgentPhase, FlowStatus};
use tokio_util::sync::CancellationToken;

use crate::workers::BackgroundWorker;

/// Agent flow orchestration engine.
/// Polls for flows in actionable states and drives them forward.
pub struct AgentFlowExecutor {
    pool: sqlx::SqlitePool,
}

impl AgentFlowExecutor {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }

    /// Process one tick: find actionable flows and dispatch them.
    async fn tick(&self) {
        // Find flows in Planning status (need to start execution)
        let planning_flows = match AgentFlow::find_by_status(&self.pool, FlowStatus::Planning).await
        {
            Ok(flows) => flows,
            Err(e) => {
                tracing::error!("[AgentFlowEngine] Failed to query planning flows: {}", e);
                return;
            }
        };

        // Find flows in Executing status (check for completion)
        let executing_flows =
            match AgentFlow::find_by_status(&self.pool, FlowStatus::Executing).await {
                Ok(flows) => flows,
                Err(e) => {
                    tracing::error!("[AgentFlowEngine] Failed to query executing flows: {}", e);
                    return;
                }
            };

        // Find flows in Verifying status (check verification results)
        let verifying_flows =
            match AgentFlow::find_by_status(&self.pool, FlowStatus::Verifying).await {
                Ok(flows) => flows,
                Err(e) => {
                    tracing::error!("[AgentFlowEngine] Failed to query verifying flows: {}", e);
                    return;
                }
            };

        let total = planning_flows.len() + executing_flows.len() + verifying_flows.len();
        if total > 0 {
            tracing::info!(
                "[AgentFlowEngine] Tick: {} planning, {} executing, {} verifying",
                planning_flows.len(),
                executing_flows.len(),
                verifying_flows.len(),
            );
        }

        for flow in planning_flows {
            self.handle_planning_flow(&flow).await;
        }

        for flow in executing_flows {
            self.handle_executing_flow(&flow).await;
        }

        for flow in verifying_flows {
            self.handle_verifying_flow(&flow).await;
        }
    }

    /// Planning → Executing: validate plan, then dispatch to executor agent
    async fn handle_planning_flow(&self, flow: &AgentFlow) {
        tracing::info!(
            "[AgentFlowEngine] Processing planning flow {} (task {})",
            flow.id,
            flow.task_id
        );

        // Validate transition
        if !flow.status.can_transition_to(&FlowStatus::Executing) {
            tracing::warn!(
                "[AgentFlowEngine] Flow {} cannot transition from {} to executing",
                flow.id,
                flow.status
            );
            return;
        }

        // Phase 1: just log and transition. Phase 2 will add real agent dispatch.
        if let Err(e) = AgentFlow::transition_to_phase(
            &self.pool,
            flow.id,
            AgentPhase::Execution,
            Some("planning"),
        )
        .await
        {
            tracing::error!(
                "[AgentFlowEngine] Failed to transition flow {} to executing: {}",
                flow.id,
                e
            );
        }
    }

    /// Executing: check if the execution is complete
    async fn handle_executing_flow(&self, flow: &AgentFlow) {
        // Phase 1: check if execution_completed_at is set
        if flow.execution_completed_at.is_some() {
            tracing::info!(
                "[AgentFlowEngine] Flow {} execution complete, moving to verification",
                flow.id
            );
            if let Err(e) = AgentFlow::transition_to_phase(
                &self.pool,
                flow.id,
                AgentPhase::Verification,
                Some("executing"),
            )
            .await
            {
                tracing::error!(
                    "[AgentFlowEngine] Failed to transition flow {} to verifying: {}",
                    flow.id,
                    e
                );
            }
        }
    }

    /// Verifying: check if verification is done, then complete
    async fn handle_verifying_flow(&self, flow: &AgentFlow) {
        if flow.verification_completed_at.is_some() {
            let target_status = if flow.human_approval_required {
                "awaiting_approval"
            } else {
                "completed"
            };

            tracing::info!(
                "[AgentFlowEngine] Flow {} verification done → {}",
                flow.id,
                target_status
            );

            let res = sqlx::query(
                "UPDATE agent_flows SET status = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
            )
            .bind(target_status)
            .bind(flow.id)
            .execute(&self.pool)
            .await;

            if let Err(e) = res {
                tracing::error!(
                    "[AgentFlowEngine] Failed to complete flow {}: {}",
                    flow.id,
                    e
                );
            }
        }
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for AgentFlowExecutor {
    fn name(&self) -> &str {
        "agent_flow_executor"
    }

    async fn run(&self, shutdown: CancellationToken) {
        use std::time::Duration;

        use tokio::time::interval;

        let mut ticker = interval(Duration::from_secs(15));
        ticker.tick().await; // discard immediate first tick

        tracing::info!("[AgentFlowEngine] Started (polling every 15s)");

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.tick().await;
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[AgentFlowEngine] Shutting down");
                    break;
                }
            }
        }
    }
}
