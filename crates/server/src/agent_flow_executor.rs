//! Agent Flow Orchestration Engine
//!
//! Background worker that polls for actionable agent flows and dispatches them
//! through the LLM pipeline. Single-phase execute-only for v1 (skip planning/verification).
//!
//! Disabled by default. Enable with `ENABLE_AGENT_FLOW_ENGINE=1`.

use db::{
    db_uuid::DbUuid,
    models::{
        agent_flow::{AgentFlow, AgentPhase, FlowStatus},
        agent_flow_event::{AgentFlowEvent, CreateFlowEvent, FlowEventPayload, FlowEventType},
    },
};
use serde_json::{Value, json};
use services::services::workflow_llm::{
    LLMResponse, ToolCallRequest, ToolDefinition, WorkflowLLMService,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::workers::BackgroundWorker;

/// Runtime configuration for the agent flow executor.
pub struct AgentFlowExecutorConfig {
    /// Polling interval in seconds (default: 15)
    pub poll_interval_secs: u64,
    /// Maximum concurrent flows to process per tick (default: 5).
    /// NOTE: Not yet enforced — tick() currently processes all actionable flows.
    /// Will be used when LLM dispatch is added (pipeline-ops sprint W2).
    pub max_concurrent: usize,
}

impl AgentFlowExecutorConfig {
    /// Load configuration from environment variables.
    /// Falls back to sensible defaults if vars are not set.
    pub fn from_env() -> Self {
        Self {
            poll_interval_secs: std::env::var("AGENT_FLOW_POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15),
            max_concurrent: std::env::var("AGENT_FLOW_MAX_CONCURRENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
        }
    }
}

/// Agent flow orchestration engine.
pub struct AgentFlowExecutor {
    pool: sqlx::SqlitePool,
    config: AgentFlowExecutorConfig,
}

impl AgentFlowExecutor {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self {
            pool,
            config: AgentFlowExecutorConfig::from_env(),
        }
    }

    /// Process one tick: find actionable flows and dispatch them.
    async fn tick(&self) {
        tracing::info!("[AgentFlowEngine] Tick — polling for pending flows...");
        // Use find_pending_flows which respects cancel_deadline
        let flows = match AgentFlow::find_pending_flows(
            &self.pool,
            self.config.max_concurrent as i32,
        )
        .await
        {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("[AgentFlowEngine] Failed to query pending flows: {}", e);
                return;
            }
        };

        tracing::info!("[AgentFlowEngine] Found {} pending flows", flows.len());
        if !flows.is_empty() {
            tracing::info!("[AgentFlowEngine] Tick: {} actionable flow(s)", flows.len(),);
        }

        for flow in flows {
            match flow.status {
                FlowStatus::Planning => self.handle_planning_flow(&flow).await,
                FlowStatus::Executing => self.handle_executing_flow(&flow).await,
                _ => {}
            }
        }
    }

    /// Planning → Executing: transition and dispatch LLM call
    async fn handle_planning_flow(&self, flow: &AgentFlow) {
        tracing::info!(
            "[AgentFlowEngine] Starting execution for flow {} (deal: {:?})",
            flow.id,
            flow.crm_deal_id
        );

        if !flow.status.can_transition_to(&FlowStatus::Executing) {
            tracing::warn!(
                "[AgentFlowEngine] Flow {} cannot transition from {} to executing",
                flow.id,
                flow.status
            );
            return;
        }

        // Transition to Executing
        if let Err(e) = AgentFlow::transition_to_phase(
            &self.pool,
            &flow.id,
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
            return;
        }

        // Emit phase started event
        if let Err(e) = AgentFlowEvent::emit_phase_started(
            &self.pool,
            &flow.id,
            "execution",
            flow.executor_agent_id.as_ref(),
        )
        .await
        {
            tracing::warn!(
                "[AgentFlowEngine] Failed to emit phase_started for flow {}: {}",
                flow.id,
                e
            );
        }

        // Dispatch LLM execution
        self.execute_flow(flow).await;
    }

    /// Execute the flow: build prompt, call LLM, process response
    async fn execute_flow(&self, flow: &AgentFlow) {
        let flow_config = self.parse_flow_config(flow);
        let agent_name = flow_config
            .get("agent_name")
            .and_then(|v| v.as_str())
            .unwrap_or("assistant");
        let deal_id = flow_config
            .get("deal_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Build context from deal data
        let deal_context = self.load_deal_context(deal_id).await;

        // Build system prompt based on agent name
        let system_prompt = build_agent_prompt(agent_name, &deal_context);

        // Build tool definitions
        let tools = build_agent_tools();

        // Build messages
        let messages = vec![
            WorkflowLLMService::system_message(&system_prompt),
            WorkflowLLMService::user_message(&format!(
                "Execute your role for the deal: {}.\n\nDeal context:\n{}",
                flow_config
                    .get("deal_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown"),
                deal_context
            )),
        ];

        // Call LLM with retry logic
        let result = self.call_llm_with_retry(flow, messages, &tools).await;

        match result {
            Ok(output) => {
                // Save artifact with the output
                let artifact_id = Uuid::new_v4();
                if let Err(e) = AgentFlowEvent::emit_artifact_created(
                    &self.pool,
                    &flow.id,
                    artifact_id,
                    "agent_output",
                    &format!("{} output", agent_name),
                    "execution",
                )
                .await
                {
                    tracing::warn!(
                        "[AgentFlowEngine] Failed to emit artifact for flow {}: {}",
                        flow.id,
                        e
                    );
                }

                // Store the output in flow_config for retrieval
                if let Err(e) = sqlx::query(
                    "UPDATE agent_flows SET flow_config = json_set(COALESCE(flow_config, '{}'), '$.output', ?1), updated_at = datetime('now', 'subsec') WHERE id = ?2",
                )
                .bind(&output)
                .bind(&flow.id)
                .execute(&self.pool)
                .await
                {
                    tracing::warn!("[AgentFlowEngine] Failed to store output for flow {}: {}", flow.id, e);
                }

                // Complete the flow (single-phase: skip verification)
                self.complete_flow(flow).await;

                tracing::info!(
                    "[AgentFlowEngine] Flow {} completed successfully (agent: {})",
                    flow.id,
                    agent_name
                );
            }
            Err(e) => {
                self.fail_flow(flow, &e.to_string()).await;
                tracing::error!("[AgentFlowEngine] Flow {} failed: {}", flow.id, e);
            }
        }
    }

    /// Call LLM with retry: attempt → same model → fallback model → fail
    async fn call_llm_with_retry(
        &self,
        flow: &AgentFlow,
        messages: Vec<Value>,
        tools: &[ToolDefinition],
    ) -> anyhow::Result<String> {
        let max_retries = 3;
        let models = [None, None, Some("claude-sonnet-4-6-20250514")]; // last attempt uses cheaper model

        for attempt in 0..max_retries {
            let model_hint = models.get(attempt).copied().flatten();

            // Update retry count
            if let Err(e) = sqlx::query(
                "UPDATE agent_flows SET retry_count = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
            )
            .bind(attempt as i32)
            .bind(&flow.id)
            .execute(&self.pool)
            .await
            {
                tracing::warn!("[AgentFlowEngine] Failed to update retry count for flow {}: {}", flow.id, e);
            }

            match self
                .call_llm_once(messages.clone(), tools, model_hint)
                .await
            {
                Ok(output) => return Ok(output),
                Err(e) => {
                    tracing::warn!(
                        "[AgentFlowEngine] Flow {} attempt {}/{} failed: {}",
                        flow.id,
                        attempt + 1,
                        max_retries,
                        e
                    );

                    // Store error for observability
                    if let Err(db_err) = sqlx::query(
                        "UPDATE agent_flows SET last_error = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
                    )
                    .bind(e.to_string())
                    .bind(&flow.id)
                    .execute(&self.pool)
                    .await
                    {
                        tracing::warn!("[AgentFlowEngine] Failed to store error for flow {}: {}", flow.id, db_err);
                    }

                    if attempt == max_retries - 1 {
                        return Err(e);
                    }

                    // Brief delay before retry
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }

        anyhow::bail!("All retry attempts exhausted")
    }

    /// Single LLM call with optional tool-call loop (max 5 turns).
    /// When SIMULATE_LLM=1 is set, returns realistic simulated responses
    /// instead of calling the actual LLM API (useful for testing without credits).
    async fn call_llm_once(
        &self,
        mut messages: Vec<Value>,
        tools: &[ToolDefinition],
        model_hint: Option<&str>,
    ) -> anyhow::Result<String> {
        // Simulation mode: return realistic agent responses without calling LLM
        if std::env::var("SIMULATE_LLM").unwrap_or_default() == "1" {
            return self.simulate_llm_response(&messages).await;
        }

        let max_turns = 5;
        let mut last_tool_sig: Option<String> = None;

        for turn in 0..max_turns {
            let (response, meta) = WorkflowLLMService::completion_with_tools(
                &self.pool,
                messages.clone(),
                tools,
                model_hint,
                Some(4096),
                Some(0.7),
            )
            .await?;

            tracing::info!(
                "[AgentFlowEngine] LLM response via {} (turn {}/{})",
                meta.model_used,
                turn + 1,
                max_turns
            );

            match response {
                LLMResponse::Text { content, .. } => {
                    return Ok(content);
                }
                LLMResponse::ToolCalls { calls, .. } => {
                    // Dedup: break if LLM repeats the exact same tool call
                    let sig = calls
                        .iter()
                        .map(|c| format!("{}:{}", c.name, c.arguments))
                        .collect::<Vec<_>>()
                        .join("|");
                    if last_tool_sig.as_deref() == Some(&sig) {
                        tracing::warn!(
                            "[AgentFlowEngine] LLM repeated same tool call on turn {} — breaking loop",
                            turn + 1
                        );
                        anyhow::bail!("LLM stuck in tool-call loop (repeated same call)");
                    }
                    last_tool_sig = Some(sig);

                    // Add assistant tool-call message to conversation
                    messages.push(WorkflowLLMService::assistant_tool_calls_message(&calls));

                    // Execute each tool call and add results
                    for call in &calls {
                        let result = self.execute_tool_call(call).await;
                        messages.push(WorkflowLLMService::tool_result_message(&call.id, &result));
                    }
                    // Continue loop for next LLM turn
                }
            }
        }

        anyhow::bail!("Max tool-call turns ({}) exceeded", max_turns)
    }

    /// Execute a tool call from the LLM and return the result as a string
    async fn execute_tool_call(&self, call: &ToolCallRequest) -> String {
        tracing::info!(
            "[AgentFlowEngine] Executing tool: {} with args: {}",
            call.name,
            call.arguments
        );

        match call.name.as_str() {
            "get_deal_context" => {
                let deal_id = call
                    .arguments
                    .get("deal_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.load_deal_context(deal_id).await
            }
            "update_deal_field" => {
                let deal_id = call
                    .arguments
                    .get("deal_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let field = call
                    .arguments
                    .get("field")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let value = call
                    .arguments
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.update_deal_field(deal_id, field, value).await
            }
            "save_artifact" => {
                let title = call
                    .arguments
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Output");
                let content = call
                    .arguments
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let flow_id = call
                    .arguments
                    .get("flow_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.save_artifact(flow_id, title, content).await
            }
            _ => {
                format!("Unknown tool: {}", call.name)
            }
        }
    }

    // ── Tool Implementations ────────────────────────────────────────────

    async fn load_deal_context(&self, deal_id: &str) -> String {
        #[derive(sqlx::FromRow)]
        struct DealRow {
            name: String,
            description: Option<String>,
            stage: Option<String>,
            amount: Option<f64>,
            currency: String,
            proposal_text: Option<String>,
        }
        let deal = sqlx::query_as::<_, DealRow>(
            "SELECT name, description, stage, amount, currency, proposal_text FROM crm_deals WHERE id = ?1",
        )
        .bind(deal_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        match deal {
            Some(d) => json!({
                "name": d.name,
                "description": d.description,
                "stage": d.stage,
                "amount": d.amount,
                "currency": d.currency,
                "has_proposal": d.proposal_text.is_some(),
            })
            .to_string(),
            None => json!({"error": "Deal not found"}).to_string(),
        }
    }

    async fn update_deal_field(&self, deal_id: &str, field: &str, value: &str) -> String {
        // Match-based queries — no string interpolation in SQL
        let result = match field {
            "description" => {
                sqlx::query("UPDATE crm_deals SET description = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2")
                    .bind(value).bind(deal_id).execute(&self.pool).await
            }
            "proposal_text" => {
                sqlx::query("UPDATE crm_deals SET proposal_text = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2")
                    .bind(value).bind(deal_id).execute(&self.pool).await
            }
            "deck_url" => {
                sqlx::query("UPDATE crm_deals SET deck_url = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2")
                    .bind(value).bind(deal_id).execute(&self.pool).await
            }
            "custom_fields" => {
                sqlx::query("UPDATE crm_deals SET custom_fields = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2")
                    .bind(value).bind(deal_id).execute(&self.pool).await
            }
            _ => {
                return json!({"error": format!("Field '{}' is not updatable", field)}).to_string();
            }
        };
        match result {
            Ok(_) => json!({"success": true, "field": field}).to_string(),
            Err(e) => json!({"error": e.to_string()}).to_string(),
        }
    }

    async fn save_artifact(&self, flow_id_str: &str, title: &str, content: &str) -> String {
        // Input size limits
        const MAX_TITLE_LEN: usize = 500;
        const MAX_CONTENT_LEN: usize = 5 * 1024 * 1024; // 5MB
        if title.len() > MAX_TITLE_LEN {
            return json!({"error": format!("Title too long ({} > {} chars)", title.len(), MAX_TITLE_LEN)}).to_string();
        }
        if content.len() > MAX_CONTENT_LEN {
            return json!({"error": format!("Content too large ({} > {} bytes)", content.len(), MAX_CONTENT_LEN)}).to_string();
        }

        let flow_id = match DbUuid::parse(flow_id_str) {
            Ok(id) => id,
            Err(_) => {
                tracing::debug!(
                    "[AgentFlowEngine] Invalid flow_id in save_artifact: {}",
                    &flow_id_str[..flow_id_str.len().min(50)]
                );
                return json!({"error": "Invalid flow_id"}).to_string();
            }
        };

        let artifact_id = Uuid::new_v4();
        match AgentFlowEvent::create(
            &self.pool,
            CreateFlowEvent {
                agent_flow_id: flow_id.clone(),
                event_type: FlowEventType::ArtifactCreated,
                event_data: FlowEventPayload::ArtifactCreated {
                    artifact_id,
                    artifact_type: "text".to_string(),
                    title: title.to_string(),
                    phase: "execution".to_string(),
                },
            },
        )
        .await
        {
            Ok(_) => {
                // Also store content in a separate event for retrieval
                let _ = AgentFlowEvent::create(
                    &self.pool,
                    CreateFlowEvent {
                        agent_flow_id: flow_id.clone(),
                        event_type: FlowEventType::ArtifactUpdated,
                        event_data: FlowEventPayload::ArtifactUpdated {
                            artifact_id,
                            changes: json!({"content": content}),
                        },
                    },
                )
                .await;
                json!({"success": true, "artifact_id": artifact_id.to_string()}).to_string()
            }
            Err(e) => json!({"error": e.to_string()}).to_string(),
        }
    }

    // ── Flow Lifecycle ──────────────────────────────────────────────────

    async fn complete_flow(&self, flow: &AgentFlow) {
        // Emit completion event
        if let Err(e) = AgentFlowEvent::create(
            &self.pool,
            CreateFlowEvent {
                agent_flow_id: flow.id.clone(),
                event_type: FlowEventType::FlowCompleted,
                event_data: FlowEventPayload::FlowCompleted {
                    verification_score: None,
                    total_artifacts: 1,
                },
            },
        )
        .await
        {
            tracing::warn!(
                "[AgentFlowEngine] Failed to emit FlowCompleted for flow {}: {}",
                flow.id,
                e
            );
        }

        // Mark as completed
        if let Err(e) = sqlx::query(
            "UPDATE agent_flows SET status = 'completed', execution_completed_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec') WHERE id = ?1",
        )
        .bind(&flow.id)
        .execute(&self.pool)
        .await
        {
            tracing::error!("[AgentFlowEngine] Failed to mark flow {} as completed: {}", flow.id, e);
        }
    }

    async fn fail_flow(&self, flow: &AgentFlow, error: &str) {
        // Emit failure event
        if let Err(e) = AgentFlowEvent::create(
            &self.pool,
            CreateFlowEvent {
                agent_flow_id: flow.id.clone(),
                event_type: FlowEventType::FlowFailed,
                event_data: FlowEventPayload::FlowFailed {
                    error: error.to_string(),
                    phase: "execution".to_string(),
                },
            },
        )
        .await
        {
            tracing::warn!(
                "[AgentFlowEngine] Failed to emit FlowFailed for flow {}: {}",
                flow.id,
                e
            );
        }

        // Mark as failed
        if let Err(e) = sqlx::query(
            "UPDATE agent_flows SET status = 'failed', last_error = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
        )
        .bind(error)
        .bind(&flow.id)
        .execute(&self.pool)
        .await
        {
            tracing::error!("[AgentFlowEngine] Failed to mark flow {} as failed: {}", flow.id, e);
        }
    }

    /// Executing flows: check if already completed (legacy path)
    async fn handle_executing_flow(&self, flow: &AgentFlow) {
        if flow.execution_completed_at.is_some() {
            tracing::info!(
                "[AgentFlowEngine] Flow {} execution already completed, finalizing",
                flow.id
            );
            self.complete_flow(flow).await;
        }
    }

    fn parse_flow_config(&self, flow: &AgentFlow) -> Value {
        flow.flow_config
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| json!({}))
    }
}

// ── LLM Simulation ──────────────────────────────────────────────────────────

impl AgentFlowExecutor {
    /// Simulate a realistic LLM response based on the agent name extracted from messages.
    /// Executes real tool calls (get_deal_context, update_deal_field, save_artifact)
    /// so the pipeline state actually advances — just skips the LLM API call.
    async fn simulate_llm_response(&self, messages: &[Value]) -> anyhow::Result<String> {
        // Extract agent name from system prompt
        let system_text = messages
            .first()
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        let agent_name = if system_text.contains("Scout") {
            "scout"
        } else if system_text.contains("Astra") {
            "astra"
        } else if system_text.contains("Cash") {
            "cash"
        } else if system_text.contains("Lux") {
            "lux"
        } else {
            "assistant"
        };

        // Extract deal_id from user message
        let user_text = messages
            .iter()
            .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("user"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");

        // Try to extract deal_id from context (look for UUID pattern)
        let deal_id = user_text
            .split_whitespace()
            .find(|w| w.len() == 36 && w.contains('-'))
            .or_else(|| {
                // Fallback: look for deal_id in the message content
                user_text.split("deal_id").nth(1).and_then(|s| {
                    s.split_whitespace()
                        .next()
                        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-'))
                })
            })
            .unwrap_or("");

        tracing::info!(
            "[AgentFlowEngine] SIMULATE_LLM: agent={}, deal_id={}",
            agent_name,
            deal_id
        );

        // Load real deal context for realistic output
        let context = if !deal_id.is_empty() {
            self.load_deal_context(deal_id).await
        } else {
            "No deal context available".to_string()
        };

        // Simulate a brief processing delay
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // Execute real tool calls based on agent role + return summary
        match agent_name {
            "scout" => {
                // Scout: save research artifact
                if !deal_id.is_empty() {
                    let _ = self
                        .update_deal_field(
                            deal_id,
                            "description",
                            &format!(
                                "[Scout Research — Simulated]\n\n\
                             Contact appears to be a decision-maker at a mid-size company. \
                             Key talking points: digital transformation, operational efficiency, \
                             and competitive positioning. Company is in a growth phase with \
                             potential for strategic partnerships.\n\n\
                             Original context: {}",
                                context.chars().take(200).collect::<String>()
                            ),
                        )
                        .await;
                }
                Ok(format!(
                    "[Simulated Scout Output]\n\n\
                     ## Research Summary\n\
                     - Contact profile analyzed\n\
                     - Company overview compiled\n\
                     - 4 key talking points identified\n\
                     - 3 potential pain points flagged\n\n\
                     Deal context updated with research findings."
                ))
            }
            "astra" => Ok(format!(
                "[Simulated Astra Output]\n\n\
                     ## Business Analysis\n\
                     - Pain point: manual processes causing bottlenecks\n\
                     - Recommended: workflow automation + AI integration\n\
                     - Scope: 3-6 month engagement\n\
                     - Risk: low (proven approach, clear ROI)\n\n\
                     Ready for proposal generation."
            )),
            "cash" => {
                if !deal_id.is_empty() {
                    let _ = self
                        .update_deal_field(
                            deal_id,
                            "proposal_text",
                            "[Simulated Proposal — Cash]\n\n\
                         ## Executive Summary\n\
                         We propose a comprehensive digital transformation engagement.\n\n\
                         ## Scope of Work\n\
                         1. Process audit and optimization (Month 1)\n\
                         2. Workflow automation implementation (Month 2-3)\n\
                         3. AI agent integration (Month 3-4)\n\
                         4. Training and handoff (Month 5)\n\n\
                         ## Investment\n\
                         Total: $45,000 over 5 months\n\n\
                         ## Timeline\n\
                         Start: 2 weeks from approval",
                        )
                        .await;
                }
                Ok(format!(
                    "[Simulated Cash Output]\n\n\
                     Proposal generated and saved to deal.\n\
                     - 4 work phases defined\n\
                     - Pricing: $45,000\n\
                     - Timeline: 5 months"
                ))
            }
            "lux" => {
                if !deal_id.is_empty() {
                    let _ = self
                        .update_deal_field(deal_id, "deck_url", "/api/decks/simulated-deck.pdf")
                        .await;
                }
                Ok(format!(
                    "[Simulated Lux Output]\n\n\
                     Presentation deck outline created:\n\
                     1. Title: Value proposition\n\
                     2. Problem/Opportunity\n\
                     3. Solution approach\n\
                     4. Deliverables & timeline\n\
                     5. Investment & ROI\n\n\
                     Deck URL saved to deal."
                ))
            }
            _ => Ok(format!(
                "[Simulated Agent Output]\n\nAnalysis complete for deal. Context: {}",
                context.chars().take(100).collect::<String>()
            )),
        }
    }
}

// ── Agent Prompts (hardcoded for v1) ─────────────────────────────────────────

fn build_agent_prompt(agent_name: &str, deal_context: &str) -> String {
    match agent_name {
        "scout" => format!(
            "You are Scout, a research agent. Your job is to gather intelligence about a person and their company.\n\
             Research the contact associated with this deal and provide:\n\
             1. A professional profile summary\n\
             2. Company overview and market position\n\
             3. Key talking points for a business meeting\n\
             4. Potential pain points and opportunities\n\n\
             Deal context: {deal_context}"
        ),
        "astra" => format!(
            "You are Astra, a business analysis agent. Your job is to analyze business opportunities.\n\
             Based on the deal and research data, provide:\n\
             1. Business pain point analysis\n\
             2. Recommended services and solutions\n\
             3. Estimated project scope and timeline\n\
             4. Risk assessment\n\n\
             Deal context: {deal_context}"
        ),
        "cash" => format!(
            "You are Cash, a proposal generation agent. Your job is to create compelling proposals.\n\
             Based on the business analysis, generate a professional proposal including:\n\
             1. Executive summary\n\
             2. Scope of work with deliverables\n\
             3. Pricing breakdown with estimated value\n\
             4. Timeline and milestones\n\n\
             Deal context: {deal_context}"
        ),
        "lux" => format!(
            "You are Lux, a presentation deck generation agent. Your job is to create polished pitch decks.\n\
             Based on the proposal, create a presentation outline with:\n\
             1. Title slide with key value proposition\n\
             2. Problem/opportunity slides\n\
             3. Solution and approach\n\
             4. Deliverables and timeline\n\
             5. Investment and ROI\n\n\
             Deal context: {deal_context}"
        ),
        _ => format!(
            "You are an AI assistant helping with a CRM deal.\n\
             Analyze the deal context and provide helpful insights.\n\n\
             Deal context: {deal_context}"
        ),
    }
}

// ── Tool Definitions ─────────────────────────────────────────────────────────

fn build_agent_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "get_deal_context".to_string(),
            description:
                "Get full context about the current deal including contact and company info"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "deal_id": {
                        "type": "string",
                        "description": "The deal ID to retrieve context for"
                    }
                },
                "required": ["deal_id"]
            }),
        },
        ToolDefinition {
            name: "update_deal_field".to_string(),
            description:
                "Update a field on the deal (description, proposal_text, deck_url, custom_fields)"
                    .to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "deal_id": {
                        "type": "string",
                        "description": "The deal ID to update"
                    },
                    "field": {
                        "type": "string",
                        "enum": ["description", "proposal_text", "deck_url", "custom_fields"],
                        "description": "The field to update"
                    },
                    "value": {
                        "type": "string",
                        "description": "The new value for the field"
                    }
                },
                "required": ["deal_id", "field", "value"]
            }),
        },
        ToolDefinition {
            name: "save_artifact".to_string(),
            description: "Save a research artifact or output document".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "flow_id": {
                        "type": "string",
                        "description": "The agent flow ID"
                    },
                    "title": {
                        "type": "string",
                        "description": "Title for the artifact"
                    },
                    "content": {
                        "type": "string",
                        "description": "The artifact content"
                    }
                },
                "required": ["flow_id", "title", "content"]
            }),
        },
    ]
}

// ── Background Worker ────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl BackgroundWorker for AgentFlowExecutor {
    fn name(&self) -> &str {
        "agent_flow_executor"
    }

    async fn run(&self, shutdown: CancellationToken) {
        use std::time::Duration;

        use tokio::time::interval;

        let poll_secs = self.config.poll_interval_secs;
        let mut ticker = interval(Duration::from_secs(poll_secs));
        ticker.tick().await; // discard immediate first tick

        tracing::info!(
            "[AgentFlowEngine] Started (polling every {}s, max {} concurrent)",
            poll_secs,
            self.config.max_concurrent
        );

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
