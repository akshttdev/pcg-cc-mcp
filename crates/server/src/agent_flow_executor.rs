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
        crm_deal::CrmDeal,
        deck_document::{
            Color, CreateDeckDocument, CreateDeckSlide, DeckDocument, DeckSlide, Fill,
            SlideElement, UpdateDeckDocument, UpdateDeckSlide,
        },
    },
};
use serde_json::{json, Value};
use services::services::workflow_llm::{
    LLMResponse, ToolCallRequest, ToolDefinition, WorkflowLLMService,
};
use tokio_util::sync::CancellationToken;

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

    /// Level 4: Circuit breaker — returns true if we should pause processing.
    /// Opens (pauses) after 5+ failures in 5 minutes across all flows.
    /// Automatically closes after 2 minutes with no new failures.
    async fn is_circuit_open(&self) -> bool {
        // Count recent failures
        let failure_count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM agent_flows
               WHERE status = 'failed'
               AND updated_at > datetime('now', '-5 minutes')"#,
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if failure_count >= 5 {
            // Check if we've been in circuit-open state long enough to close
            let most_recent_failure: Option<String> = sqlx::query_scalar(
                r#"SELECT MAX(updated_at) FROM agent_flows
                   WHERE status = 'failed'
                   AND updated_at > datetime('now', '-5 minutes')"#,
            )
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();

            // If most recent failure was over 2 minutes ago, allow retry (half-open)
            if let Some(ts) = most_recent_failure {
                // Simple check: if the timestamp is more than 2 minutes old
                let two_min_ago: String =
                    sqlx::query_scalar("SELECT datetime('now', '-2 minutes')")
                        .fetch_one(&self.pool)
                        .await
                        .unwrap_or_default();

                if ts < two_min_ago {
                    tracing::info!(
                        "[AgentFlowEngine] Circuit breaker HALF-OPEN — allowing retry after 2 min cooldown"
                    );
                    return false;
                }
            }

            tracing::warn!(
                "[AgentFlowEngine] Circuit breaker triggered: {} failures in 5 minutes",
                failure_count
            );
            return true;
        }

        false
    }

    /// Process one tick: find actionable flows and dispatch them.
    async fn tick(&self) {
        // Level 4: Circuit breaker check — pause if too many recent failures
        if self.is_circuit_open().await {
            tracing::warn!("[AgentFlowEngine] Circuit breaker OPEN — skipping tick");
            return;
        }

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
                FlowStatus::Verifying => self.handle_verifying_flow(&flow).await,
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

        // Detect task_execution flows (created by Topsi for PCG agents)
        let is_task_flow = flow_config
            .get("task_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .is_some();

        // Build context: task-based or deal-based
        let context = if is_task_flow {
            self.load_task_context(&flow_config).await
        } else {
            // Build context from deal data (scoped to this agent + deal's current stage)
            let deal_stage: Option<String> = sqlx::query_scalar(
                "SELECT s.name FROM crm_deals d JOIN crm_pipeline_stages s ON d.crm_stage_id = s.id WHERE d.id = ?1",
            )
            .bind(deal_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten();
            let stage_ref = deal_stage.as_deref();
            self.load_deal_context(deal_id, Some(agent_name), stage_ref)
                .await
        };

        // Build system prompt based on agent name
        let system_prompt = build_agent_prompt(agent_name, &context);

        // Build tool definitions — task flows get task-aware tools
        let tools = build_agent_tools(is_task_flow);

        // Build messages — task flow uses task-centric prompt
        let user_prompt = if is_task_flow {
            format!(
                "Execute the following task assigned to you:\n\n{}\n\nTask context:\n{}",
                flow_config
                    .get("task_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Task"),
                context
            )
        } else {
            format!(
                "Execute your role for the deal: {}.\n\nDeal context:\n{}",
                flow_config
                    .get("deal_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown"),
                context
            )
        };

        let messages = vec![
            WorkflowLLMService::system_message(&system_prompt),
            WorkflowLLMService::user_message(&user_prompt),
        ];

        // Extract agent model hint for conversation persistence (optional override)
        let agent_model = flow_config
            .get("agent_model")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Call LLM with retry logic (pass flow.id for artifact saving in simulation)
        let result = self
            .call_llm_with_retry(flow, messages, &tools, &flow.id)
            .await;

        match result {
            Ok(output) => {
                // Check for structured response (from submit_response tool)
                if let Ok(structured) = serde_json::from_str::<Value>(&output) {
                    if let Some(status) = structured.get("status").and_then(|s| s.as_str()) {
                        if status == "needs_clarification" {
                            // Handle clarification request
                            self.request_clarification(flow, &structured).await;
                            return;
                        }
                    }
                }

                // Save artifact with the output
                let artifact_id = DbUuid::new();
                if let Err(e) = AgentFlowEvent::emit_artifact_created(
                    &self.pool,
                    &flow.id,
                    artifact_id.into(),
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
                    tracing::error!("[AgentFlowEngine] Failed to store output for flow {}: {}", flow.id, e);
                }

                // Persist the agent's work as a conversation message so it's
                // visible in conversation history and carries forward as memory.
                if is_task_flow {
                    let task_id_str = flow_config
                        .get("task_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let task_title = flow_config
                        .get("task_title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("task");
                    let executor_agent_id = flow.executor_agent_id.clone();

                    if !task_id_str.is_empty() {
                        if let Some(agent_uuid) = executor_agent_id {
                            // session keyed by task so future queries can find it
                            let session_id = format!("task:{}", task_id_str);
                            let _ = crate::helpers::conversations::persist_chat_exchange(
                                &self.pool,
                                agent_uuid.into(),
                                &session_id,
                                flow_config
                                    .get("project_id")
                                    .and_then(|v| v.as_str())
                                    .and_then(|s| uuid::Uuid::parse_str(s).ok()),
                                &format!("Execute task: {}", task_title),
                                &output,
                                agent_model.as_deref().or(Some("claude-sonnet-4-6")),
                                Some("anthropic"),
                                None,
                                None,
                                agent_name,
                            )
                            .await;
                        }
                    }
                }

                // Check if verification is needed (via env var or flow config)
                if self.should_verify(flow) {
                    // Transition to verification phase
                    if let Err(e) = AgentFlow::transition_to_phase(
                        &self.pool,
                        &flow.id,
                        AgentPhase::Verification,
                        Some("executing"),
                    )
                    .await
                    {
                        tracing::error!(
                            "[AgentFlowEngine] Failed to transition flow {} to verification: {}",
                            flow.id,
                            e
                        );
                        self.complete_flow(flow).await;
                    } else {
                        tracing::info!(
                            "[AgentFlowEngine] Flow {} execution complete, transitioning to verification",
                            flow.id
                        );
                    }
                } else {
                    // Skip verification (default for v1)
                    self.complete_flow(flow).await;
                    tracing::info!(
                        "[AgentFlowEngine] Flow {} completed successfully (agent: {})",
                        flow.id,
                        agent_name
                    );
                }

                // Post-completion actions
                if is_task_flow {
                    // Mark the originating task as done
                    let task_id_str = flow_config
                        .get("task_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !task_id_str.is_empty() {
                        if let Err(e) = sqlx::query(
                            "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec') WHERE id = ?",
                        )
                        .bind(task_id_str)
                        .execute(&self.pool)
                        .await
                        {
                            tracing::error!(
                                "[AgentFlowEngine] Failed to mark task {} done: {}",
                                task_id_str,
                                e
                            );
                        } else {
                            tracing::info!(
                                "[AgentFlowEngine] Task {} marked done via flow {}",
                                task_id_str,
                                flow.id
                            );
                        }
                    }
                } else if !deal_id.is_empty() {
                    // Chain next agent if chain_actions exist, otherwise auto-advance
                    let flow_config = self.parse_flow_config(flow);
                    if self.try_chain_next_agent(deal_id, &flow_config).await {
                        tracing::info!(
                            "[AgentFlowEngine] Chained next agent for deal {} (from flow {})",
                            deal_id,
                            flow.id
                        );
                    } else {
                        self.try_auto_advance_deal(deal_id).await;
                    }
                }
            }
            Err(e) => {
                self.fail_flow(flow, &e.to_string()).await;
                tracing::error!("[AgentFlowEngine] Flow {} failed: {}", flow.id, e);
                // For task flows, leave task in 'todo' for retry (log only)
                if is_task_flow {
                    let task_id_str = flow_config
                        .get("task_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !task_id_str.is_empty() {
                        tracing::warn!(
                            "[AgentFlowEngine] Flow {} failed — task {} remains in todo for retry",
                            flow.id,
                            task_id_str
                        );
                    }
                }
            }
        }
    }

    /// Load task context from flow_config for task_execution flows
    async fn load_task_context(&self, flow_config: &Value) -> String {
        let task_title = flow_config
            .get("task_title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Task");
        let task_description = flow_config
            .get("task_description")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let project_id = flow_config
            .get("project_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let personality = flow_config
            .get("personality")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let task_id = flow_config
            .get("task_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Load project name
        let project_name: Option<String> =
            sqlx::query_scalar("SELECT name FROM projects WHERE id = ?")
                .bind(project_id)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();

        format!(
            "Task: {}\nProject: {}\nTask ID: {}\nDescription: {}\n\nAgent Personality:\n{}",
            task_title,
            project_name.as_deref().unwrap_or("Unknown Project"),
            task_id,
            task_description,
            personality
        )
    }

    /// Load task context by task_id directly (for `get_task_context` tool calls)
    async fn load_task_context_by_id(&self, task_id: &str) -> String {
        #[derive(sqlx::FromRow)]
        struct TaskRow {
            title: String,
            description: Option<String>,
            status: Option<String>,
            project_name: Option<String>,
        }
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT t.title, t.description, t.status, p.name AS project_name \
             FROM tasks t LEFT JOIN projects p ON p.id = t.project_id \
             WHERE t.id = ? LIMIT 1",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        match row {
            Some(r) => format!(
                "Task ID: {}\nTitle: {}\nStatus: {}\nProject: {}\nDescription: {}",
                task_id,
                r.title,
                r.status.as_deref().unwrap_or("todo"),
                r.project_name.as_deref().unwrap_or("Unknown Project"),
                r.description.as_deref().unwrap_or("(no description)")
            ),
            None => format!("Task {} not found", task_id),
        }
    }

    /// Update task description/notes field
    async fn update_task_notes(&self, task_id: &str, notes: &str) -> String {
        let result = sqlx::query(
            "UPDATE tasks SET description = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(notes)
        .bind(task_id)
        .execute(&self.pool)
        .await;

        match result {
            Ok(r) if r.rows_affected() > 0 => format!("Task {} notes updated", task_id),
            Ok(_) => format!("Task {} not found", task_id),
            Err(e) => format!("Failed to update task: {}", e),
        }
    }

    /// Call LLM with retry using an optional preferred model for the first attempts.
    /// Falls back to claude-sonnet-4-6 on the last attempt if preferred model fails.
    async fn call_llm_with_retry_model(
        &self,
        flow: &AgentFlow,
        messages: Vec<Value>,
        tools: &[ToolDefinition],
        flow_id: &DbUuid,
        preferred_model: Option<&str>,
    ) -> anyhow::Result<String> {
        let max_retries = 3;
        // Use preferred model for first two attempts, fallback on third
        let m0 = preferred_model;
        let m1 = preferred_model;
        let m2 = Some("claude-sonnet-4-6");
        let models: [Option<&str>; 3] = [m0, m1, m2];

        let flow_deal_id = flow
            .flow_config
            .as_deref()
            .and_then(|c| serde_json::from_str::<Value>(c).ok())
            .and_then(|v| v.get("deal_id").and_then(|d| d.as_str().map(String::from)))
            .unwrap_or_default();

        for attempt in 0..max_retries {
            let model_hint = models.get(attempt).copied().flatten();

            if let Err(e) = sqlx::query(
                "UPDATE agent_flows SET retry_count = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
            )
            .bind(attempt as i32)
            .bind(&flow.id)
            .execute(&self.pool)
            .await
            {
                tracing::error!("[AgentFlowEngine] Failed to update retry count for flow {}: {}", flow.id, e);
            }

            match self
                .call_llm_once(messages.clone(), tools, model_hint, &flow_deal_id, flow_id)
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

                    if let Err(db_err) = sqlx::query(
                        "UPDATE agent_flows SET last_error = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
                    )
                    .bind(e.to_string())
                    .bind(&flow.id)
                    .execute(&self.pool)
                    .await
                    {
                        tracing::error!("[AgentFlowEngine] Failed to store error for flow {}: {}", flow.id, db_err);
                    }

                    if attempt == max_retries - 1 {
                        return Err(e);
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }

        anyhow::bail!("All retry attempts exhausted")
    }

    /// Call LLM with retry: attempt → same model → fallback model → fail
    async fn call_llm_with_retry(
        &self,
        flow: &AgentFlow,
        messages: Vec<Value>,
        tools: &[ToolDefinition],
        flow_id: &DbUuid,
    ) -> anyhow::Result<String> {
        let max_retries = 3;
        let models = [None, None, Some("claude-sonnet-4-6")]; // last attempt uses cheaper model

        // Extract deal_id from flow config for simulated mode
        let flow_deal_id = flow
            .flow_config
            .as_deref()
            .and_then(|c| serde_json::from_str::<Value>(c).ok())
            .and_then(|v| v.get("deal_id").and_then(|d| d.as_str().map(String::from)))
            .unwrap_or_default();

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
                tracing::error!("[AgentFlowEngine] Failed to update retry count for flow {}: {}", flow.id, e);
            }

            match self
                .call_llm_once(messages.clone(), tools, model_hint, &flow_deal_id, flow_id)
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
                        tracing::error!("[AgentFlowEngine] Failed to store error for flow {}: {}", flow.id, db_err);
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
        deal_id: &str,
        flow_id: &DbUuid,
    ) -> anyhow::Result<String> {
        // Simulation mode: return realistic agent responses without calling LLM
        if std::env::var("SIMULATE_LLM").unwrap_or_default() == "1" {
            return self
                .simulate_llm_response(&messages, deal_id, flow_id)
                .await;
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

                    // Execute tool calls — optionally batched for concurrency
                    let results = if Self::is_batched_execution_enabled() && calls.len() > 1 {
                        self.execute_tool_calls_batched(&calls).await
                    } else {
                        // Serial execution (original behavior)
                        let mut results = Vec::with_capacity(calls.len());
                        for call in &calls {
                            let result = self.execute_tool_call(call).await;
                            results.push((call.id.clone(), result));
                        }
                        results
                    };

                    // Process results and check for submit_response
                    for (tool_id, result) in results {
                        // Check for structured submit_response
                        if result.starts_with("__SUBMIT_RESPONSE__:") {
                            let response_json = &result["__SUBMIT_RESPONSE__:".len()..];
                            return Ok(response_json.to_string());
                        }

                        messages.push(WorkflowLLMService::tool_result_message(&tool_id, &result));
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
            // ── Deal tools ────────────────────────────────────────────────────
            "get_deal_context" => {
                let deal_id = call
                    .arguments
                    .get("deal_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.load_deal_context(deal_id, None, None).await
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
            // ── Task tools ────────────────────────────────────────────────────
            "get_task_context" => {
                let task_id = call
                    .arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.load_task_context_by_id(task_id).await
            }
            "update_task_notes" => {
                let task_id = call
                    .arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let notes = call
                    .arguments
                    .get("notes")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.update_task_notes(task_id, notes).await
            }
            // ── Shared tools ──────────────────────────────────────────────────
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
            "submit_response" => {
                // Parse structured response from agent
                let status = call
                    .arguments
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("success");
                let message = call
                    .arguments
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let clarification_question = call
                    .arguments
                    .get("clarification_question")
                    .and_then(|v| v.as_str());
                let clarification_context = call
                    .arguments
                    .get("clarification_context")
                    .and_then(|v| v.as_str());

                // Build structured response JSON
                let response = json!({
                    "status": status,
                    "message": message,
                    "clarification": clarification_question.map(|q| json!({
                        "question": q,
                        "context": clarification_context,
                        "required_fields": []
                    }))
                });

                // Return special prefix for status detection in call_llm_once
                format!("__SUBMIT_RESPONSE__:{}", response)
            }
            // ── Lux Creator Studio deck authoring tools ────────────────────
            "create_deck_document" => {
                let deal_id = call
                    .arguments
                    .get("deal_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.create_deck_document(deal_id).await
            }
            "append_slide" => self.append_slide(&call.arguments).await,
            "add_element" => self.add_element(&call.arguments).await,
            _ => {
                format!("Unknown tool: {}", call.name)
            }
        }
    }

    /// Execute tool calls with concurrency partitioning.
    /// Safe tools run in parallel (max batch_size), unsafe tools run serially.
    ///
    /// Returns a list of (tool_use_id, result) pairs in the same order as input calls.
    async fn execute_tool_calls_batched(&self, calls: &[ToolCallRequest]) -> Vec<(String, String)> {
        use crate::tool_partitioner::{default_tool_metadata, partition_tool_calls, ToolCall};

        if calls.is_empty() {
            return Vec::new();
        }

        // Convert ToolCallRequest to partitioner's ToolCall
        let tool_calls: Vec<ToolCall> = calls
            .iter()
            .map(|c| ToolCall::new(&c.id, &c.name, c.arguments.clone()))
            .collect();

        let metadata = default_tool_metadata();
        let batches = partition_tool_calls(tool_calls, &metadata);

        const MAX_CONCURRENT: usize = 5;
        let mut results = Vec::with_capacity(calls.len());

        for batch in batches {
            if batch.is_concurrency_safe && batch.calls.len() > 1 {
                // Run safe tools concurrently in chunks
                for chunk in batch.calls.chunks(MAX_CONCURRENT) {
                    let futures: Vec<_> = chunk
                        .iter()
                        .map(|call| async {
                            // Find the original ToolCallRequest
                            let original = calls.iter().find(|c| c.id == call.id);
                            let result = match original {
                                Some(orig) => self.execute_tool_call(orig).await,
                                None => format!("Tool call {} not found", call.id),
                            };
                            (call.id.clone(), result)
                        })
                        .collect();

                    let chunk_results = futures::future::join_all(futures).await;
                    results.extend(chunk_results);
                }
            } else {
                // Run unsafe tools (or single safe tool) serially
                for call in &batch.calls {
                    let original = calls.iter().find(|c| c.id == call.id);
                    let result = match original {
                        Some(orig) => self.execute_tool_call(orig).await,
                        None => format!("Tool call {} not found", call.id),
                    };
                    results.push((call.id.clone(), result));
                }
            }
        }

        results
    }

    /// Check if batched execution is enabled (controlled by env var).
    /// Returns true if BATCHED_TOOL_EXECUTION=1 or if unset (default on).
    fn is_batched_execution_enabled() -> bool {
        std::env::var("BATCHED_TOOL_EXECUTION")
            .map(|v| v != "0")
            .unwrap_or(true)
    }

    // ── Tool Implementations ────────────────────────────────────────────

    async fn load_deal_context(
        &self,
        deal_id: &str,
        agent_name: Option<&str>,
        stage_name: Option<&str>,
    ) -> String {
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

        let Some(d) = deal else {
            return json!({"error": "Deal not found"}).to_string();
        };

        // Load linked transcripts
        #[derive(sqlx::FromRow)]
        struct TranscriptRow {
            summary: Option<String>,
            transcript_text: Option<String>,
        }
        let transcripts: Vec<TranscriptRow> = sqlx::query_as(
            "SELECT summary, transcript_text FROM deal_transcripts WHERE deal_id = ?1 ORDER BY created_at DESC LIMIT 5",
        )
        .bind(deal_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let transcript_summaries: Vec<String> = transcripts
            .iter()
            .filter_map(|t| t.summary.clone().or(t.transcript_text.clone()))
            .collect();

        // Load linked data sources (via join table), filtered by agent + stage scope
        #[derive(sqlx::FromRow)]
        struct SourceRow {
            title: Option<String>,
            content: Option<String>,
        }
        let agent_filter = agent_name.unwrap_or("");
        let stage_filter = stage_name.unwrap_or("");
        let sources: Vec<SourceRow> = sqlx::query_as(
            r#"SELECT ds.title, SUBSTR(ds.content, 1, 10000) as content
               FROM deal_data_sources dds
               JOIN data_sources ds ON dds.data_source_id = ds.id
               WHERE dds.deal_id = ?1
                 AND (dds.relevant_agents IS NULL OR ?2 IN (SELECT value FROM json_each(dds.relevant_agents)))
                 AND (dds.relevant_stages IS NULL OR ?3 IN (SELECT value FROM json_each(dds.relevant_stages)))
               ORDER BY dds.created_at DESC LIMIT 5"#,
        )
        .bind(deal_id)
        .bind(agent_filter)
        .bind(stage_filter)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let source_items: Vec<Value> = sources
            .iter()
            .map(|s| {
                json!({
                    "title": s.title,
                    "content": s.content,
                })
            })
            .collect();

        json!({
            "name": d.name,
            "description": d.description,
            "stage": d.stage,
            "amount": d.amount,
            "currency": d.currency,
            "has_proposal": d.proposal_text.is_some(),
            "transcripts": transcript_summaries,
            "linked_sources": source_items,
        })
        .to_string()
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

    /// Update contact intelligence fields directly via the deal's linked contact.
    /// The Intel tab reads from `crm_contacts.intelligence_summary`, so Scout
    /// writes there — no person lookup needed.
    async fn update_deal_contact_intelligence(
        &self,
        deal_id: &str,
        summary: &str,
    ) -> Result<(), anyhow::Error> {
        let rows = sqlx::query(
            r#"UPDATE crm_contacts
               SET intelligence_summary = ?1,
                   intelligence_status = 'done',
                   intelligence_confidence = 0.75,
                   research_pass_count = COALESCE(research_pass_count, 0) + 1,
                   updated_at = datetime('now', 'subsec')
               WHERE id = (SELECT crm_contact_id FROM crm_deals WHERE id = ?2)"#,
        )
        .bind(summary)
        .bind(deal_id)
        .execute(&self.pool)
        .await?;

        if rows.rows_affected() > 0 {
            tracing::info!(
                "[AgentFlowEngine] Updated contact intelligence for deal={}",
                deal_id
            );
        } else {
            tracing::warn!(
                "[AgentFlowEngine] No contact linked to deal {} — cannot update intelligence",
                deal_id
            );
        }

        Ok(())
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

        let artifact_id = DbUuid::new();
        let artifact_id_str = artifact_id.to_string();
        match AgentFlowEvent::create(
            &self.pool,
            CreateFlowEvent {
                agent_flow_id: flow_id.clone(),
                event_type: FlowEventType::ArtifactCreated,
                event_data: FlowEventPayload::ArtifactCreated {
                    artifact_id: artifact_id.clone().into(),
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
                if let Err(e) = AgentFlowEvent::create(
                    &self.pool,
                    CreateFlowEvent {
                        agent_flow_id: flow_id.clone(),
                        event_type: FlowEventType::ArtifactUpdated,
                        event_data: FlowEventPayload::ArtifactUpdated {
                            artifact_id: artifact_id.into(),
                            changes: json!({"content": content}),
                        },
                    },
                )
                .await
                {
                    tracing::error!(
                        "[AgentFlowEngine] Failed to store artifact content event: {}",
                        e
                    );
                }
                json!({"success": true, "artifact_id": artifact_id_str}).to_string()
            }
            Err(e) => json!({"error": e.to_string()}).to_string(),
        }
    }

    // ── Lux deck authoring tools ────────────────────────────────────────
    // These tools let Lux compose a structured DeckDocument via the agent
    // flow tool loop. Every mutation bumps `deck.version` and flips
    // `last_edited_by = "lux"` so the studio UI can detect external changes.

    async fn create_deck_document(&self, deal_id_str: &str) -> String {
        let deal_id = match DbUuid::parse(deal_id_str) {
            Ok(id) => id,
            Err(_) => return json!({"error": "Invalid deal_id"}).to_string(),
        };

        let deck = match DeckDocument::create(
            &self.pool,
            CreateDeckDocument {
                deal_id: deal_id.to_uuid(),
                canvas: None,
                brand_token_version: None,
            },
        )
        .await
        {
            Ok(d) => d,
            Err(e) => {
                return json!({"error": format!("Failed to create deck: {e}")}).to_string();
            }
        };

        // Link the deck to the deal for downstream lookup. Failure here is
        // non-fatal — the deck row still exists and can be rediscovered via
        // DeckDocument::find_latest_for_deal.
        if let Err(e) = sqlx::query(
            "UPDATE crm_deals SET deck_document_id = ?1, updated_at = datetime('now','subsec') \
             WHERE id = ?2",
        )
        .bind(deck.id.to_string())
        .bind(deal_id.to_string())
        .execute(&self.pool)
        .await
        {
            tracing::warn!(
                "[AgentFlowEngine] create_deck_document: failed to link deal {}: {}",
                deal_id,
                e
            );
        }

        json!({
            "success": true,
            "deck_id": deck.id.to_string(),
            "version": deck.version,
            "canvas": serde_json::from_str::<serde_json::Value>(&deck.canvas_json).unwrap_or(Value::Null),
        })
        .to_string()
    }

    async fn append_slide(&self, args: &Value) -> String {
        let deck_id_str = args.get("deck_id").and_then(|v| v.as_str()).unwrap_or("");
        let deck_id = match DbUuid::parse(deck_id_str) {
            Ok(id) => id,
            Err(_) => return json!({"error": "Invalid deck_id"}).to_string(),
        };

        let slide_index = args
            .get("slide_index")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let name = args.get("name").and_then(|v| v.as_str()).map(String::from);
        let layout_hint = args
            .get("layout_hint")
            .and_then(|v| v.as_str())
            .map(String::from);
        let notes = args.get("notes").and_then(|v| v.as_str()).map(String::from);

        let background: Fill = match args.get("background") {
            Some(bg) => match serde_json::from_value(bg.clone()) {
                Ok(f) => f,
                Err(e) => {
                    return json!({"error": format!("Invalid background: {e}")}).to_string();
                }
            },
            None => Fill::Solid {
                color: Color {
                    r: 255.0,
                    g: 255.0,
                    b: 255.0,
                    a: 1.0,
                    token: None,
                },
            },
        };

        let slide = match DeckSlide::create(
            &self.pool,
            CreateDeckSlide {
                deck_id: deck_id.to_uuid(),
                slide_index,
                name,
                layout_hint,
                background,
                elements: vec![],
                notes,
                origin: Some("lux".to_string()),
            },
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                return json!({"error": format!("Failed to append slide: {e}")}).to_string();
            }
        };

        self.mark_deck_edited_by_lux(deck_id.to_uuid()).await;

        json!({
            "success": true,
            "slide_id": slide.id.to_string(),
            "slide_index": slide.slide_index,
        })
        .to_string()
    }

    async fn add_element(&self, args: &Value) -> String {
        let deck_id_str = args.get("deck_id").and_then(|v| v.as_str()).unwrap_or("");
        let slide_id_str = args.get("slide_id").and_then(|v| v.as_str()).unwrap_or("");
        let element_value = match args.get("element") {
            Some(v) => v.clone(),
            None => return json!({"error": "Missing 'element' argument"}).to_string(),
        };

        let deck_id = match DbUuid::parse(deck_id_str) {
            Ok(id) => id,
            Err(_) => return json!({"error": "Invalid deck_id"}).to_string(),
        };
        let slide_id = match DbUuid::parse(slide_id_str) {
            Ok(id) => id,
            Err(_) => return json!({"error": "Invalid slide_id"}).to_string(),
        };

        let element: SlideElement = match serde_json::from_value(element_value) {
            Ok(e) => e,
            Err(e) => return json!({"error": format!("Invalid element JSON: {e}")}).to_string(),
        };

        let slide = match DeckSlide::find_by_id(&self.pool, slide_id.to_uuid()).await {
            Ok(Some(s)) => s,
            Ok(None) => return json!({"error": "Slide not found"}).to_string(),
            Err(e) => {
                return json!({"error": format!("Failed to load slide: {e}")}).to_string();
            }
        };

        // Enforce that the slide belongs to the claimed deck — prevents a
        // malformed prompt from writing elements into the wrong deck.
        if slide.deck_id != deck_id.to_uuid() {
            return json!({"error": "Slide does not belong to deck_id"}).to_string();
        }

        let mut elements: Vec<SlideElement> = match slide.elements() {
            Ok(v) => v,
            Err(e) => {
                return json!({"error": format!("Failed to parse existing elements: {e}")})
                    .to_string();
            }
        };
        elements.push(element);
        let element_id = elements
            .last()
            .map(|el| el.id().to_string())
            .unwrap_or_default();

        if let Err(e) = DeckSlide::update(
            &self.pool,
            slide_id.to_uuid(),
            UpdateDeckSlide {
                elements: Some(elements),
                ..Default::default()
            },
        )
        .await
        {
            return json!({"error": format!("Failed to persist slide: {e}")}).to_string();
        }

        self.mark_deck_edited_by_lux(deck_id.to_uuid()).await;

        json!({
            "success": true,
            "element_id": element_id,
        })
        .to_string()
    }

    /// Bump `deck.version` and tag the deck as last-edited-by-lux. Failures
    /// are logged but not fatal — the preceding mutation already succeeded.
    async fn mark_deck_edited_by_lux(&self, deck_id: uuid::Uuid) {
        if let Err(e) = DeckDocument::update(
            &self.pool,
            deck_id,
            UpdateDeckDocument {
                last_edited_by: Some("lux".to_string()),
                ..Default::default()
            },
        )
        .await
        {
            tracing::warn!(
                "[AgentFlowEngine] mark_deck_edited_by_lux: failed on deck {}: {}",
                deck_id,
                e
            );
        }
    }

    // ── Flow Lifecycle ──────────────────────────────────────────────────

    /// Load StageConfig for a deal's current stage (for auto_complete_review checks)
    async fn load_stage_config_for_deal(
        &self,
        deal_id: &str,
    ) -> Option<crate::stage_transition::StageConfig> {
        let stage_config_json: Option<String> = sqlx::query_scalar(
            r#"SELECT ps.stage_config FROM crm_deals d
               JOIN crm_pipeline_stages ps ON d.crm_stage_id = ps.id
               WHERE d.id = ?1"#,
        )
        .bind(deal_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        stage_config_json.and_then(|json| serde_json::from_str(&json).ok())
    }

    /// Handle a clarification request from the agent.
    /// Transitions the flow to NeedsClarification and stores the request.
    async fn request_clarification(&self, flow: &AgentFlow, structured_response: &Value) {
        let clarification = structured_response.get("clarification");
        let message = structured_response
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Clarification needed");

        // Build clarification request JSON
        let clarification_json = clarification
            .map(|c| c.to_string())
            .unwrap_or_else(|| json!({"question": message}).to_string());

        // Emit FlowPaused event
        if let Err(e) = AgentFlowEvent::create(
            &self.pool,
            CreateFlowEvent {
                agent_flow_id: flow.id.clone(),
                event_type: FlowEventType::FlowPaused,
                event_data: FlowEventPayload::FlowPaused {
                    reason: Some("Clarification needed".to_string()),
                    paused_by: Some("agent".to_string()),
                },
            },
        )
        .await
        {
            tracing::warn!(
                "[AgentFlowEngine] Failed to emit FlowPaused for flow {}: {}",
                flow.id,
                e
            );
        }

        // Update flow status to needs_clarification
        if let Err(e) = sqlx::query(
            "UPDATE agent_flows SET status = 'needs_clarification', clarification_request = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
        )
        .bind(&clarification_json)
        .bind(&flow.id)
        .execute(&self.pool)
        .await
        {
            tracing::error!(
                "[AgentFlowEngine] Failed to transition flow {} to needs_clarification: {}",
                flow.id,
                e
            );
        } else {
            tracing::info!(
                "[AgentFlowEngine] Flow {} paused for clarification: {}",
                flow.id,
                message
            );

            // Slack: agent escalation. Best-effort — resolve org via the flow's task → project.
            let task_id_str = flow.task_id.as_str().to_string();
            if let Ok(Some(task)) =
                db::models::task::Task::find_by_id(&self.pool, &task_id_str).await
            {
                if let Ok(Some(project)) =
                    db::models::project::Project::find_by_id(&self.pool, &task.project_id).await
                {
                    if let Some(org_str) = project.organization_id {
                        let org_db = db::db_uuid::DbUuid::from_string(org_str.clone());
                        let payload = serde_json::json!({
                            "agent_name": flow
                                .planner_agent_id
                                .as_ref()
                                .map(|a| a.as_str().to_string())
                                .unwrap_or_else(|| "Agent".to_string()),
                            "task_title": task.title,
                            "blocked_reason": message,
                            "review_url": format!(
                                "/organizations/{}/projects/{}/tasks/{}",
                                org_str, project.id, task.id
                            ),
                        });
                        let _ = services::services::slack::dispatch_event(
                            &self.pool,
                            &org_db,
                            db::models::slack_channel_route::SlackEventType::AgentEscalation,
                            &payload,
                        )
                        .await;
                    }
                }
            }
        }
    }

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

        // Mark as completed — guard on status to prevent double-completion races
        match sqlx::query(
            "UPDATE agent_flows SET status = 'completed', execution_completed_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec') WHERE id = ?1 AND status IN ('planning', 'executing')",
        )
        .bind(&flow.id)
        .execute(&self.pool)
        .await
        {
            Ok(r) if r.rows_affected() == 0 => {
                tracing::warn!(
                    "[AgentFlowEngine] Flow {} already completed or not in a completable state — skipping",
                    flow.id
                );
            }
            Err(e) => {
                tracing::error!("[AgentFlowEngine] Failed to mark flow {} as completed: {}", flow.id, e);
            }
            _ => {}
        }

        // Handle review tasks based on stage config
        let deal_id = flow.crm_deal_id.as_deref().unwrap_or("");
        if !deal_id.is_empty() {
            let stage_config = self.load_stage_config_for_deal(deal_id).await;
            let auto_complete = stage_config
                .map(|c| c.auto_complete_review)
                .unwrap_or(false);

            if auto_complete {
                // Auto-complete review tasks (fully autonomous progression)
                let completed = sqlx::query(
                    "UPDATE tasks SET status = 'done', updated_at = datetime('now','subsec') WHERE crm_deal_id = ?1 AND status IN ('waiting', 'todo') AND deleted_at IS NULL",
                )
                .bind(deal_id)
                .execute(&self.pool)
                .await;

                match completed {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::info!(
                            "[AgentFlowEngine] Auto-completed {} review task(s) for deal {} (auto_complete_review=true)",
                            r.rows_affected(),
                            deal_id
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "[AgentFlowEngine] Failed to auto-complete tasks for deal {}: {}",
                            deal_id,
                            e
                        );
                    }
                    _ => {}
                }
            } else {
                // Default: promote "waiting" review tasks to "todo" for human review
                let promoted = sqlx::query(
                    "UPDATE tasks SET status = 'todo', updated_at = datetime('now','subsec') WHERE crm_deal_id = ?1 AND status = 'waiting' AND deleted_at IS NULL",
                )
                .bind(deal_id)
                .execute(&self.pool)
                .await;

                match promoted {
                    Ok(r) if r.rows_affected() > 0 => {
                        tracing::info!(
                            "[AgentFlowEngine] Promoted {} waiting review task(s) to todo for deal {}",
                            r.rows_affected(),
                            deal_id
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            "[AgentFlowEngine] Failed to promote waiting tasks for deal {}: {}",
                            deal_id,
                            e
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    async fn fail_flow(&self, flow: &AgentFlow, error: &str) {
        let deal_id = flow.crm_deal_id.as_deref().unwrap_or("");

        // Level 3: Try fallback agent if configured and not already a fallback attempt
        let flow_config = self.parse_flow_config(flow);
        let is_fallback_attempt = flow_config
            .get("is_fallback_attempt")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !is_fallback_attempt && !deal_id.is_empty() {
            if let Some(stage_config) = self.load_stage_config_for_deal(deal_id).await {
                if let Some(ref fallback_agent) = stage_config.fallback_agent {
                    tracing::info!(
                        "[AgentFlowEngine] Flow {} failed, attempting fallback agent: {}",
                        flow.id,
                        fallback_agent
                    );

                    // Try to schedule fallback agent
                    if self
                        .try_schedule_fallback_agent(deal_id, fallback_agent, error)
                        .await
                    {
                        // Mark original flow as failed but don't escalate
                        if let Err(e) = sqlx::query(
                            "UPDATE agent_flows SET status = 'failed', last_error = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
                        )
                        .bind(format!("Failed, fallback scheduled: {}", error))
                        .bind(&flow.id)
                        .execute(&self.pool)
                        .await
                        {
                            tracing::error!("[AgentFlowEngine] Failed to mark flow {} as failed: {}", flow.id, e);
                        }
                        return;
                    }
                }
            }
        }

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

        // Mark as failed — guard on status to avoid overwriting a completed flow
        if let Err(e) = sqlx::query(
            "UPDATE agent_flows SET status = 'failed', last_error = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2 AND status IN ('planning', 'executing')",
        )
        .bind(error)
        .bind(&flow.id)
        .execute(&self.pool)
        .await
        {
            tracing::error!("[AgentFlowEngine] Failed to mark flow {} as failed: {}", flow.id, e);
        }

        // Level 5: Human escalation for repeated failures
        if !deal_id.is_empty() {
            self.try_escalate_to_human(deal_id, flow, error).await;
        }
    }

    /// Level 3: Try to schedule a fallback agent for failed flow
    async fn try_schedule_fallback_agent(
        &self,
        deal_id: &str,
        fallback_agent: &str,
        original_error: &str,
    ) -> bool {
        let deal_uuid = DbUuid::from_string(deal_id.to_string());
        let deal = match db::models::crm_deal::CrmDeal::find_by_id(&self.pool, &deal_uuid).await {
            Ok(d) => d,
            Err(_) => return false,
        };

        let flow_type = crate::stage_transition::agent_default_flow_type(fallback_agent);

        match crate::stage_transition::schedule_agent_flow(
            &self.pool,
            &deal,
            fallback_agent,
            &flow_type,
            0, // No cancel window for fallback
        )
        .await
        {
            Ok((new_flow_id, _)) => {
                // Mark as fallback attempt so we don't infinite loop
                if let Err(e) = sqlx::query(
                    "UPDATE agent_flows SET flow_config = json_set(COALESCE(flow_config, '{}'), '$.is_fallback_attempt', true), \
                     flow_config = json_set(flow_config, '$.original_error', ?1) WHERE id = ?2",
                )
                .bind(original_error)
                .bind(&new_flow_id)
                .execute(&self.pool)
                .await
                {
                    tracing::error!(
                        "[AgentFlowEngine] Failed to mark flow {} as fallback: {}",
                        new_flow_id,
                        e
                    );
                }

                tracing::info!(
                    "[AgentFlowEngine] Scheduled fallback agent {} for deal {} (flow {})",
                    fallback_agent,
                    deal_id,
                    new_flow_id
                );
                true
            }
            Err(e) => {
                tracing::error!("[AgentFlowEngine] Failed to schedule fallback agent: {}", e);
                false
            }
        }
    }

    /// Level 5: Create escalation task for human intervention after repeated failures
    async fn try_escalate_to_human(&self, deal_id: &str, flow: &AgentFlow, error: &str) {
        // Count recent failures for this deal (last 10 minutes)
        let failure_count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM agent_flows
               WHERE crm_deal_id = ?1
               AND status = 'failed'
               AND updated_at > datetime('now', '-10 minutes')"#,
        )
        .bind(deal_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // Escalate after 3+ failures
        if failure_count >= 3 {
            tracing::warn!(
                "[AgentFlowEngine] {} failures in 10 minutes for deal {} — escalating to human",
                failure_count,
                deal_id
            );

            let flow_config = self.parse_flow_config(flow);
            let agent_name = flow_config
                .get("agent_name")
                .and_then(|v| v.as_str())
                .unwrap_or("agent");

            // Create escalation task
            let task_id = DbUuid::new();
            if let Err(e) = sqlx::query(
                r#"INSERT INTO tasks (id, title, description, status, crm_deal_id, created_by, created_at, updated_at)
                   VALUES (?1, ?2, ?3, 'todo', ?4, 'system', datetime('now','subsec'), datetime('now','subsec'))"#,
            )
            .bind(task_id.to_string())
            .bind(format!("[ESCALATION] {} agent failed repeatedly", agent_name))
            .bind(format!(
                "The {} agent has failed {} times in the last 10 minutes.\n\nLatest error:\n{}\n\nManual intervention required.",
                agent_name, failure_count, error
            ))
            .bind(deal_id)
            .execute(&self.pool)
            .await
            {
                tracing::error!(
                    "[AgentFlowEngine] Failed to create escalation task for deal {}: {}",
                    deal_id,
                    e
                );
            } else {
                tracing::info!(
                    "[AgentFlowEngine] Created escalation task {} for deal {}",
                    task_id,
                    deal_id
                );
            }
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

    /// Verifying flows: run verification agent to validate execution output
    async fn handle_verifying_flow(&self, flow: &AgentFlow) {
        tracing::info!(
            "[AgentFlowEngine] Starting verification for flow {} (deal: {:?})",
            flow.id,
            flow.crm_deal_id
        );

        let flow_config = self.parse_flow_config(flow);
        let execution_output = flow_config
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let agent_name = flow_config
            .get("agent_name")
            .and_then(|v| v.as_str())
            .unwrap_or("agent");

        // Build verification prompt
        let system_prompt = format!(
            "You are a verification agent. Your job is to review the output from the {} agent \
             and determine if it meets quality standards.\n\n\
             Review the following output and use the submit_verification tool to provide your assessment.",
            agent_name
        );

        let user_prompt = format!(
            "Please verify this output:\n\n---\n{}\n---\n\n\
             Evaluate: completeness, accuracy, relevance, and quality.\n\
             Provide a score from 0.0 to 1.0 and decide if it passes (score >= 0.7) or needs retry.",
            execution_output.chars().take(4000).collect::<String>()
        );

        let messages = vec![
            WorkflowLLMService::system_message(&system_prompt),
            WorkflowLLMService::user_message(&user_prompt),
        ];

        let tools = vec![ToolDefinition {
            name: "submit_verification".to_string(),
            description: "Submit verification result with score and pass/fail decision".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "score": {
                        "type": "number",
                        "minimum": 0.0,
                        "maximum": 1.0,
                        "description": "Quality score from 0.0 to 1.0"
                    },
                    "passed": {
                        "type": "boolean",
                        "description": "Whether the output passes verification"
                    },
                    "feedback": {
                        "type": "string",
                        "description": "Feedback for improvement if not passed"
                    }
                },
                "required": ["score", "passed"]
            }),
        }];

        // Call LLM for verification
        match self
            .call_llm_with_retry(flow, messages, &tools, &flow.id)
            .await
        {
            Ok(output) => {
                // Parse verification result
                let (score, passed, feedback) = self.parse_verification_result(&output);

                if passed {
                    // Update verification score and complete
                    if let Err(e) = sqlx::query(
                        "UPDATE agent_flows SET verification_score = ?1, updated_at = datetime('now', 'subsec') WHERE id = ?2",
                    )
                    .bind(score)
                    .bind(&flow.id)
                    .execute(&self.pool)
                    .await
                    {
                        tracing::error!(
                            "[AgentFlowEngine] Failed to store verification score for flow {}: {}",
                            flow.id,
                            e
                        );
                    }

                    self.complete_flow(flow).await;
                    tracing::info!(
                        "[AgentFlowEngine] Flow {} verified successfully (score: {:.2})",
                        flow.id,
                        score
                    );
                } else {
                    // Retry if under max retries
                    if flow.retry_count < 2 {
                        tracing::info!(
                            "[AgentFlowEngine] Flow {} failed verification (score: {:.2}), retry #{} with feedback",
                            flow.id,
                            score,
                            flow.retry_count + 1
                        );

                        // Store feedback and transition back to executing
                        if let Err(e) = sqlx::query(
                            "UPDATE agent_flows SET status = 'executing', current_phase = 'execution', \
                             retry_count = retry_count + 1, \
                             flow_config = json_set(COALESCE(flow_config, '{}'), '$.verification_feedback', ?1), \
                             updated_at = datetime('now', 'subsec') WHERE id = ?2",
                        )
                        .bind(&feedback)
                        .bind(&flow.id)
                        .execute(&self.pool)
                        .await
                        {
                            tracing::error!(
                                "[AgentFlowEngine] Failed to transition flow {} back to executing: {}",
                                flow.id,
                                e
                            );
                        }
                    } else {
                        tracing::warn!(
                            "[AgentFlowEngine] Flow {} failed verification after {} retries",
                            flow.id,
                            flow.retry_count
                        );
                        self.fail_flow(flow, &format!("Failed verification: {}", feedback))
                            .await;
                    }
                }
            }
            Err(e) => {
                // Verification LLM call failed — complete anyway (non-critical)
                tracing::warn!(
                    "[AgentFlowEngine] Verification LLM call failed for flow {}: {} — completing without verification",
                    flow.id,
                    e
                );
                self.complete_flow(flow).await;
            }
        }
    }

    /// Parse verification tool call result
    fn parse_verification_result(&self, output: &str) -> (f64, bool, String) {
        // Try to parse as JSON from submit_verification tool
        if let Ok(parsed) = serde_json::from_str::<Value>(output) {
            let score = parsed.get("score").and_then(|v| v.as_f64()).unwrap_or(0.5);
            let passed = parsed
                .get("passed")
                .and_then(|v| v.as_bool())
                .unwrap_or(score >= 0.7);
            let feedback = parsed
                .get("feedback")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            return (score, passed, feedback);
        }

        // Default: pass with medium score
        (0.75, true, String::new())
    }

    fn parse_flow_config(&self, flow: &AgentFlow) -> Value {
        flow.flow_config
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_else(|| json!({}))
    }

    /// Determine if a flow should go through verification phase.
    /// Enabled via:
    /// - ENABLE_FLOW_VERIFICATION=1 env var (global)
    /// - flow_config.require_verification: true (per-flow)
    /// - verifier_agent_id being set (explicit verifier)
    fn should_verify(&self, flow: &AgentFlow) -> bool {
        // Check if verifier agent is explicitly assigned
        if flow.verifier_agent_id.is_some() {
            return true;
        }

        // Check flow config
        let config = self.parse_flow_config(flow);
        if config
            .get("require_verification")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return true;
        }

        // Check global env var
        std::env::var("ENABLE_FLOW_VERIFICATION")
            .map(|v| v == "1")
            .unwrap_or(false)
    }

    /// Check if the completed flow has chained agent actions queued.
    /// If so, schedule the next agent and pass remaining chain to it.
    /// Returns true if a chained agent was scheduled (caller should NOT auto-advance).
    async fn try_chain_next_agent(&self, deal_id: &str, flow_config: &Value) -> bool {
        let chain_actions = match flow_config.get("chain_actions").and_then(|v| v.as_array()) {
            Some(actions) if !actions.is_empty() => actions,
            _ => return false,
        };

        let next = &chain_actions[0];
        let agent = match next.get("agent").and_then(|v| v.as_str()) {
            Some(a) => a,
            None => return false,
        };
        let flow_type = next
            .get("flow_type")
            .and_then(|v| v.as_str())
            .unwrap_or("custom");

        // Load the deal for schedule_agent_flow
        let deal_uuid = DbUuid::from_string(deal_id.to_string());
        let deal = match CrmDeal::find_by_id(&self.pool, &deal_uuid).await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("[AgentFlowEngine] Chain: deal {} not found: {}", deal_id, e);
                return false;
            }
        };

        // Schedule the next agent with default 30s cancel window
        match crate::stage_transition::schedule_agent_flow(&self.pool, &deal, agent, flow_type, 30)
            .await
        {
            Ok((new_flow_id, _)) => {
                // Pass remaining chain to the new flow
                let remaining: Vec<&Value> = chain_actions.iter().skip(1).collect();
                if !remaining.is_empty() {
                    let remaining_json =
                        serde_json::to_string(&remaining).unwrap_or_else(|_| "[]".to_string());
                    if let Err(e) = sqlx::query(
                        "UPDATE agent_flows SET flow_config = json_set(COALESCE(flow_config, '{}'), '$.chain_actions', json(?1)), updated_at = datetime('now', 'subsec') WHERE id = ?2",
                    )
                    .bind(&remaining_json)
                    .bind(&new_flow_id)
                    .execute(&self.pool)
                    .await
                    {
                        tracing::error!(
                            "[AgentFlowEngine] Failed to set chain_actions on chained flow {}: {}",
                            new_flow_id,
                            e
                        );
                    }
                }

                tracing::info!(
                    "[AgentFlowEngine] Chained {} agent (flow {}) for deal {} ({} remaining in chain)",
                    agent,
                    new_flow_id,
                    deal_id,
                    remaining.len()
                );
                true
            }
            Err(e) => {
                tracing::error!(
                    "[AgentFlowEngine] Failed to schedule chained {} agent for deal {}: {}",
                    agent,
                    deal_id,
                    e
                );
                false
            }
        }
    }
}

// ── Auto-Advance ────────────────────────────────────────────────────────────

impl AgentFlowExecutor {
    /// After an agent flow completes, check if the deal should auto-advance
    /// to the next pipeline stage. This happens when:
    /// 1. The current stage has an agent assigned (agent-owned stage)
    /// 2. All agent flows for this deal in the current stage are completed
    /// 3. The review task (if any) is done
    async fn try_auto_advance_deal(&self, deal_id: &str) {
        tracing::info!(
            "[AgentFlowEngine] Checking auto-advance for deal {}",
            deal_id
        );

        // Load the deal
        let deal = match sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
            "SELECT id, crm_stage_id, crm_pipeline_id FROM crm_deals WHERE id = ?1",
        )
        .bind(deal_id)
        .fetch_optional(&self.pool)
        .await
        {
            Ok(Some(d)) => d,
            Ok(None) => {
                tracing::warn!(
                    "[AgentFlowEngine] Auto-advance: deal {} not found in DB",
                    deal_id
                );
                return;
            }
            Err(e) => {
                tracing::error!(
                    "[AgentFlowEngine] Auto-advance: failed to load deal {}: {}",
                    deal_id,
                    e
                );
                return;
            }
        };

        let (_, stage_id, pipeline_id) = deal;
        let (Some(stage_id), Some(pipeline_id)) = (stage_id, pipeline_id) else {
            tracing::warn!(
                "[AgentFlowEngine] Auto-advance: deal {} has no stage or pipeline",
                deal_id
            );
            return;
        };

        // Check if the current stage has an agent assigned (agent-owned)
        let stage_config: Option<String> =
            sqlx::query_scalar("SELECT stage_config FROM crm_pipeline_stages WHERE id = ?1")
                .bind(&stage_id)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();

        let is_agent_stage = stage_config
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .map(|c| c.get("assigned_agent").and_then(|a| a.as_str()).is_some())
            .unwrap_or(false);

        if !is_agent_stage {
            tracing::info!(
                "[AgentFlowEngine] Auto-advance: stage {} is not agent-owned (no assigned_agent in config), skipping. Config: {:?}",
                stage_id,
                stage_config.as_deref().unwrap_or("null")
            );
            return;
        }

        // Check if any pending agent flows remain for this deal
        let pending_flows: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_flows WHERE crm_deal_id = ?1 AND status IN ('planning', 'executing')",
        )
        .bind(deal_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if pending_flows > 0 {
            tracing::info!(
                "[AgentFlowEngine] Auto-advance: deal {} has {} pending flows, waiting",
                deal_id,
                pending_flows
            );
            return;
        }

        // Check if deal-linked tasks are completed
        // Review tasks are created on stage entry — operator must complete them before auto-advance
        let pending_deal_tasks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ?1 AND status NOT IN ('cancelled', 'done') AND deleted_at IS NULL",
        )
        .bind(deal_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if pending_deal_tasks > 0 {
            tracing::info!(
                "[AgentFlowEngine] Auto-advance: deal {} has {} pending task(s) — operator must complete before advancing",
                deal_id,
                pending_deal_tasks
            );
            return;
        }

        // Find next stage in the pipeline
        let current_position: Option<i32> =
            sqlx::query_scalar("SELECT position FROM crm_pipeline_stages WHERE id = ?1")
                .bind(&stage_id)
                .fetch_optional(&self.pool)
                .await
                .ok()
                .flatten();

        let Some(pos) = current_position else {
            tracing::warn!(
                "[AgentFlowEngine] Auto-advance: stage {} has no position",
                stage_id
            );
            return;
        };

        let next_stage: Option<(String, String)> = sqlx::query_as(
            "SELECT id, name FROM crm_pipeline_stages WHERE pipeline_id = ?1 AND position > ?2 ORDER BY position ASC LIMIT 1",
        )
        .bind(&pipeline_id)
        .bind(pos)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        let Some((next_stage_id, next_stage_name)) = next_stage else {
            tracing::info!(
                "[AgentFlowEngine] Auto-advance: deal {} is in the last stage (pos {}), nothing to advance to",
                deal_id,
                pos
            );
            return;
        };

        // Auto-advance the deal
        tracing::info!(
            "[AgentFlowEngine] Auto-advancing deal {} to stage {} ({})",
            deal_id,
            next_stage_name,
            next_stage_id
        );

        if let Err(e) = sqlx::query(
            "UPDATE crm_deals SET crm_stage_id = ?1, stage = ?2, updated_at = datetime('now','subsec') WHERE id = ?3",
        )
        .bind(&next_stage_id)
        .bind(&next_stage_name)
        .bind(deal_id)
        .execute(&self.pool)
        .await
        {
            tracing::error!("[AgentFlowEngine] Failed to auto-advance deal {}: {}", deal_id, e);
            return;
        }

        // Run transition processor for the new stage (triggers next agent, creates review tasks, etc.)
        let deal_uuid = match db::db_uuid::DbUuid::parse(deal_id) {
            Ok(u) => u,
            Err(_) => return,
        };
        if let Ok(deal) = db::models::crm_deal::CrmDeal::find_by_id(&self.pool, &deal_uuid).await {
            let next_stage_uuid = db::db_uuid::DbUuid::from_string(next_stage_id.clone());
            if let Ok(to_stage) =
                db::models::crm_pipeline::CrmPipelineStage::find_by_id(&self.pool, &next_stage_uuid)
                    .await
            {
                let from_stage_uuid = db::db_uuid::DbUuid::from_string(stage_id);
                let from_stage = db::models::crm_pipeline::CrmPipelineStage::find_by_id(
                    &self.pool,
                    &from_stage_uuid,
                )
                .await
                .ok();
                let result = crate::stage_transition::process_transition(
                    &self.pool,
                    &deal,
                    from_stage.as_ref(),
                    &to_stage,
                )
                .await;
                tracing::info!(
                    "[AgentFlowEngine] Transition result for deal {} → {}: {:?}",
                    deal_id,
                    next_stage_name,
                    result.actions_taken
                );
            }
        }
    }
}

// ── LLM Simulation ──────────────────────────────────────────────────────────

impl AgentFlowExecutor {
    /// Simulate a realistic LLM response based on the agent name extracted from messages.
    /// Executes real tool calls (get_deal_context, update_deal_field, save_artifact)
    /// so the pipeline state actually advances — just skips the LLM API call.
    async fn simulate_llm_response(
        &self,
        messages: &[Value],
        deal_id_override: &str,
        flow_id: &DbUuid,
    ) -> anyhow::Result<String> {
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

        // Use deal_id from flow_config (passed via call chain), fallback to text extraction
        let deal_id = if !deal_id_override.is_empty() {
            deal_id_override
        } else {
            user_text
                .split_whitespace()
                .find(|w| w.len() == 36 && w.contains('-'))
                .or_else(|| {
                    user_text.split("deal_id").nth(1).and_then(|s| {
                        s.split_whitespace()
                            .next()
                            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-'))
                    })
                })
                .unwrap_or("")
        };

        tracing::info!(
            "[AgentFlowEngine] SIMULATE_LLM: agent={}, deal_id={}",
            agent_name,
            deal_id
        );

        // Load real deal context for realistic output (simulated — no agent/stage scope)
        let context = if !deal_id.is_empty() {
            self.load_deal_context(deal_id, None, None).await
        } else {
            "No deal context available".to_string()
        };

        // Simulate a brief processing delay
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let flow_id_str = flow_id.to_string();

        // Execute real tool calls based on agent role + return summary
        match agent_name {
            "scout" => {
                let report = "[Simulated Scout Output]\n\n\
                    ## Contact Intelligence Report\n\n\
                    Contact appears to be a decision-maker at a mid-size company.\n\
                    Key talking points: digital transformation, operational efficiency, \
                    and competitive positioning. Company is in a growth phase with \
                    potential for strategic partnerships.\n\n\
                    ### Key Findings\n\
                    - Contact profile analyzed\n\
                    - Company overview compiled\n\
                    - 4 key talking points identified\n\
                    - 3 potential pain points flagged"
                    .to_string();

                if !deal_id.is_empty() {
                    self.update_deal_field(
                        deal_id,
                        "description",
                        &format!("[Scout Research — Simulated]\n\n{}", report),
                    )
                    .await;

                    if let Err(e) = self
                        .update_deal_contact_intelligence(deal_id, &report)
                        .await
                    {
                        tracing::error!(
                            "[AgentFlowEngine] Simulated scout: contact intel error: {}",
                            e
                        );
                    }
                }

                self.save_artifact(&flow_id_str, "Contact Intelligence Report", &report)
                    .await;

                Ok(report)
            }
            "astra" => {
                let report = "[Simulated Astra Output]\n\n\
                    ## Business Analysis Report\n\n\
                    ### Pain Points\n\
                    - Manual processes causing operational bottlenecks\n\
                    - Lack of integrated data across departments\n\n\
                    ### Recommended Services\n\
                    - Workflow automation + AI integration\n\
                    - Data pipeline consolidation\n\n\
                    ### Scope & Timeline\n\
                    - Engagement: 3-6 months\n\
                    - Risk: low (proven approach, clear ROI)\n\n\
                    Ready for proposal generation."
                    .to_string();

                if !deal_id.is_empty() {
                    self.update_deal_field(
                        deal_id,
                        "description",
                        &format!("[Astra Analysis — Simulated]\n\n{}", report),
                    )
                    .await;
                }

                self.save_artifact(&flow_id_str, "Business Analysis Report", &report)
                    .await;

                Ok(report)
            }
            "cash" => {
                let proposal = "[Simulated Proposal — Cash]\n\n\
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
                    Start: 2 weeks from approval"
                    .to_string();

                if !deal_id.is_empty() {
                    self.update_deal_field(deal_id, "proposal_text", &proposal)
                        .await;
                }

                self.save_artifact(&flow_id_str, "Proposal Document", &proposal)
                    .await;

                Ok(proposal)
            }
            "lux" => {
                let deck = "[Simulated Lux Output]\n\n\
                    ## Pitch Deck Outline\n\n\
                    1. **Title**: Value proposition — transforming operations through AI\n\
                    2. **Problem/Opportunity**: Manual processes, data silos, competitive pressure\n\
                    3. **Solution approach**: Phased automation + AI agent integration\n\
                    4. **Deliverables & timeline**: 5 months, 4 work phases\n\
                    5. **Investment & ROI**: $45,000 — projected 3x return in Year 1"
                    .to_string();

                if !deal_id.is_empty() {
                    self.update_deal_field(deal_id, "deck_url", "/api/decks/simulated-deck.pdf")
                        .await;
                }

                self.save_artifact(&flow_id_str, "Pitch Deck Outline", &deck)
                    .await;

                Ok(deck)
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
    let tool_instructions = "\n\n## Available Tools\n\
        You have the following tools — use them to complete your task:\n\n\
        1. **get_deal_context**: Retrieve full deal details (contact info, company, stage, prior research).\n\
           Always call this first to get up-to-date context before generating output.\n\
        2. **update_deal_field**: Write results to the deal. Fields: description, proposal_text, deck_url, custom_fields.\n\
           Use this to persist your analysis — don't just return text.\n\
        3. **save_artifact**: Save a detailed document (research report, proposal, deck outline) as a named artifact.\n\
           Use this for longer outputs that should be reviewable.\n\n\
        ## Workflow\n\
        1. Call `get_deal_context` to load the deal\n\
        2. Do your analysis\n\
        3. Call `update_deal_field` to persist key results on the deal\n\
        4. Call `save_artifact` to save the full report/document\n\
        5. Return a brief summary of what you did\n";

    match agent_name {
        "scout" => format!(
            "You are Scout, Social Intelligence Analyst for Power Club Global.\n\
             Your job is to gather intelligence about a deal's contact and their company.\n\n\
             Research the contact and provide:\n\
             1. Professional profile summary (background, role, achievements)\n\
             2. Company overview and market position\n\
             3. Key talking points for a business meeting\n\
             4. Potential pain points and opportunities for PCG\n\n\
             Save your research via `update_deal_field` (field: description) and `save_artifact`.\n\
             {tool_instructions}\n\
             Deal context: {deal_context}"
        ),
        "astra" => format!(
            "You are Astra, Business Intelligence Analyst for Power Club Global.\n\
             Your job is to analyze business opportunities and produce structured reports.\n\n\
             Based on prior research (call `get_deal_context` first), provide:\n\
             1. Business pain point analysis — what problems does the prospect face?\n\
             2. Recommended services and solutions PCG can offer\n\
             3. Estimated project scope, timeline, and budget range\n\
             4. Risk assessment and competitive considerations\n\n\
             Save your analysis via `save_artifact` with title 'Business Analysis Report'.\n\
             {tool_instructions}\n\
             Deal context: {deal_context}"
        ),
        "cash" => format!(
            "You are Cash, Proposal Strategist for Power Club Global.\n\
             Your job is to create compelling, professional proposals.\n\n\
             Based on the business analysis (call `get_deal_context` first), generate:\n\
             1. Executive summary — the hook\n\
             2. Scope of work with specific deliverables\n\
             3. Pricing breakdown with estimated investment\n\
             4. Timeline with milestones and checkpoints\n\n\
             Save the proposal via `update_deal_field` (field: proposal_text) AND `save_artifact`.\n\
             {tool_instructions}\n\
             Deal context: {deal_context}"
        ),
        "lux" => format!(
            "You are Lux, Creative Director for Power Club Global.\n\
             Your job is to create polished pitch deck outlines.\n\n\
             Based on the proposal (call `get_deal_context` first), create:\n\
             1. Title slide — value proposition in one line\n\
             2. Problem/opportunity — what the client faces\n\
             3. Solution and approach — how PCG solves it\n\
             4. Deliverables and timeline — what they get and when\n\
             5. Investment and ROI — pricing framed as value\n\n\
             Save the deck outline via `update_deal_field` (field: deck_url placeholder) AND `save_artifact`.\n\
             {tool_instructions}\n\
             Deal context: {deal_context}"
        ),
        _ => format!(
            "You are an AI assistant helping with a CRM deal for Power Club Global.\n\
             Analyze the deal context and provide helpful insights.\n\
             {tool_instructions}\n\
             Deal context: {deal_context}"
        ),
    }
}

// ── Tool Definitions ─────────────────────────────────────────────────────────

fn build_agent_tools(is_task_flow: bool) -> Vec<ToolDefinition> {
    let save_artifact = ToolDefinition {
        name: "save_artifact".to_string(),
        description: "Save a research artifact, output document, or deliverable".to_string(),
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
    };

    // Structured response tool for all agents
    let submit_response = ToolDefinition {
        name: "submit_response".to_string(),
        description: "Submit your final structured response. Use this to complete your task with a structured output.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["success", "partial", "needs_clarification", "failed"],
                    "description": "Status of your task completion"
                },
                "message": {
                    "type": "string",
                    "description": "Summary of what you accomplished or why clarification is needed"
                },
                "clarification_question": {
                    "type": "string",
                    "description": "If status is needs_clarification, the question to ask the user"
                },
                "clarification_context": {
                    "type": "string",
                    "description": "Additional context for the clarification request"
                }
            },
            "required": ["status", "message"]
        }),
    };

    if is_task_flow {
        vec![
            ToolDefinition {
                name: "get_task_context".to_string(),
                description: "Get full details about the current task including project info"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The task ID to retrieve context for"
                        }
                    },
                    "required": ["task_id"]
                }),
            },
            ToolDefinition {
                name: "update_task_notes".to_string(),
                description: "Update the task description/notes with your findings or output"
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "task_id": {
                            "type": "string",
                            "description": "The task ID to update"
                        },
                        "notes": {
                            "type": "string",
                            "description": "The notes or output to write to the task"
                        }
                    },
                    "required": ["task_id", "notes"]
                }),
            },
            save_artifact,
            submit_response.clone(),
        ]
    } else {
        vec![
            ToolDefinition {
                name: "get_deal_context".to_string(),
                description: "Get full context about the current deal including contact and company info".to_string(),
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
                description: "Update a field on the deal (description, proposal_text, deck_url, custom_fields)".to_string(),
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
            // ── Lux Creator Studio deck authoring tools ────────────────────
            ToolDefinition {
                name: "create_deck_document".to_string(),
                description:
                    "Create a new structured deck document for a deal and link it. Returns the \
                     new deck_id. Canvas defaults to 1920x1080 at 72dpi. Call this once at the \
                     start of a deck generation run, then use append_slide + add_element to fill it."
                        .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "deal_id": {
                            "type": "string",
                            "description": "The deal ID this deck belongs to"
                        }
                    },
                    "required": ["deal_id"]
                }),
            },
            ToolDefinition {
                name: "append_slide".to_string(),
                description: "Append a slide to a deck. Creates an empty slide at `slide_index`; add \
                     elements to it with `add_element`. Returns the new slide_id."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "deck_id": {
                            "type": "string",
                            "description": "The deck to append to"
                        },
                        "slide_index": {
                            "type": "integer",
                            "description": "Zero-based position of this slide (Cover=0, Agenda=1, ...)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Short slide name shown in the slide navigator (e.g. 'Cover', 'Agenda', 'Solution')"
                        },
                        "layout_hint": {
                            "type": "string",
                            "enum": ["title", "content", "split", "cover", "closer"],
                            "description": "Optional layout family for the renderer"
                        },
                        "background": {
                            "type": "object",
                            "description": "Optional Fill: { kind: 'solid' | 'linear-gradient' | 'none', ... }. Defaults to solid white."
                        },
                        "notes": {
                            "type": "string",
                            "description": "Speaker notes for this slide"
                        }
                    },
                    "required": ["deck_id", "slide_index"]
                }),
            },
            ToolDefinition {
                name: "add_element".to_string(),
                description: "Append a SlideElement (text, image, shape, or group) to a slide. The \
                     element JSON follows the SlideElement discriminated union: every element \
                     has `type` ('text'|'image'|'shape'|'group'), `id` (uuid), `bbox` (x,y,w,h), \
                     `z` (integer z-index), and `origin` ('lux'). Text adds `text`, `font_family`, \
                     `font_size`, `color`, `align`. Image adds `asset_id?`, `asset_status` \
                     ('pending'|'resolved'|'failed'), `fit`. Shape adds `shape`, `fill`. Group \
                     adds `children` (nested SlideElement array). Use token_refs to reference \
                     design tokens like color.brand.primary."
                    .to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "deck_id": { "type": "string" },
                        "slide_id": { "type": "string" },
                        "element": {
                            "type": "object",
                            "description": "Full SlideElement JSON. Must include type, id, bbox, z, origin, and type-specific fields."
                        }
                    },
                    "required": ["deck_id", "slide_id", "element"]
                }),
            },
            save_artifact,
            submit_response,
        ]
    }
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

        // On startup, mark any flows that were left in `running`/`executing`/`planning`
        // state from a previous server boot as failed — they have no live executor.
        match sqlx::query(
            "UPDATE agent_flows \
             SET status = 'failed', last_error = 'Engine restarted — flow orphaned' \
             WHERE status IN ('running', 'executing', 'planning') \
               AND updated_at < datetime('now', '-5 minutes')",
        )
        .execute(&self.pool)
        .await
        {
            Ok(r) if r.rows_affected() > 0 => {
                tracing::warn!(
                    "[AgentFlowEngine] Cleaned up {} orphaned flow(s) from previous boot",
                    r.rows_affected()
                );
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("[AgentFlowEngine] Orphan cleanup failed: {}", e);
            }
        }

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
