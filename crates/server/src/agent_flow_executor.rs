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

        // For task flows, use the agent's preferred model as hint
        let agent_model = flow_config
            .get("agent_model")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Call LLM with retry logic (pass flow.id for artifact saving in simulation)
        let result = self
            .call_llm_with_retry_model(flow, messages, &tools, &flow.id, agent_model.as_deref())
            .await;

        match result {
            Ok(output) => {
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

                // Complete the flow (single-phase: skip verification)
                self.complete_flow(flow).await;

                tracing::info!(
                    "[AgentFlowEngine] Flow {} completed successfully (agent: {})",
                    flow.id,
                    agent_name
                );

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
            _ => {
                format!("Unknown tool: {}", call.name)
            }
        }
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

        // Promote "waiting" review tasks to "todo" now that the agent is done
        let deal_id = flow.crm_deal_id.as_deref().unwrap_or("");
        if !deal_id.is_empty() {
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
            save_artifact,
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
