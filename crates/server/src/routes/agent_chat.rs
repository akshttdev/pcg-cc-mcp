//! Agent Chat Routes
//!
//! Direct chat endpoints for individual agents, enabling each agent to have
//! their own LLM configuration, conversation history, and personality.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::{sse::Event, Sse},
    routing::{get, post},
    Json, Router,
};
use db::models::{
    agent::Agent,
    agent_conversation::{AgentConversation, AgentConversationMessage},
    agent_flow_event::AgentFlowEvent,
    agent_wallet::{AgentWallet, AgentWalletTransaction, CreateWalletTransaction},
    project::Project,
    vibe_transaction::VibeSourceType,
};
use deployment::Deployment;
use services::services::vibe_pricing::VibePricingService;
use futures::stream::Stream;
use nora::{
    brain::{create_client_for_agent, ConversationMessage, LLMResponse},
    ProjectScopedContext,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

/// Request to chat with an agent
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentChatRequest {
    /// The message content
    pub message: String,
    /// Session identifier for conversation continuity
    pub session_id: String,
    /// Optional project context
    pub project_id: Option<Uuid>,
    /// Optional additional context
    pub context: Option<serde_json::Value>,
    /// Enable streaming response
    #[serde(default)]
    pub stream: bool,
}

/// Response from agent chat
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentChatResponse {
    /// The agent's response content
    pub content: String,
    /// Conversation ID for reference
    pub conversation_id: Uuid,
    /// Agent information
    pub agent_name: String,
    pub agent_designation: String,
    /// Token usage (if available)
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    /// Model used for this response
    pub model: Option<String>,
    pub provider: Option<String>,
    /// Response latency in milliseconds
    pub latency_ms: i64,
}

/// Query parameters for listing conversations
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListConversationsQuery {
    pub limit: Option<i64>,
    pub project_id: Option<Uuid>,
}

/// Conversation summary for listing
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConversationSummary {
    pub id: Uuid,
    pub title: Option<String>,
    pub status: String,
    pub message_count: i64,
    pub last_message_at: Option<String>,
    pub created_at: String,
}

/// Build protected routes for agent chat (write operations - require auth)
pub fn routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/agents/{agent_id}/chat", post(agent_chat))
        .route("/agents/{agent_id}/chat/stream", post(agent_chat_stream))
}

/// Build public routes for reading agent conversations (no auth required)
/// These are read-only endpoints for viewing conversation history,
/// important for team collaboration and task handoffs.
pub fn public_routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/agents/{agent_id}/conversations", get(list_conversations))
        .route(
            "/agents/{agent_id}/conversations/session/{session_id}",
            get(get_conversation_by_session),
        )
        .route(
            "/agents/{agent_id}/conversations/{conversation_id}",
            get(get_conversation),
        )
        .route(
            "/agents/{agent_id}/conversations/{conversation_id}/messages",
            get(get_conversation_messages),
        )
}

/// Chat with a specific agent
#[axum::debug_handler]
pub async fn agent_chat(
    State(state): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<AgentChatRequest>,
) -> Result<Json<AgentChatResponse>, ApiError> {
    let start = std::time::Instant::now();

    tracing::info!(
        "Agent chat request: agent={}, session={}, message_len={}",
        agent_id,
        request.session_id,
        request.message.len()
    );

    // Get database pool
    let pool = &state.db().pool;

    // Load agent from database
    let agent = Agent::find_by_id(pool, agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load agent: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", agent_id)))?;

    // Get or create conversation
    let conversation = AgentConversation::get_or_create(
        pool,
        agent_id,
        &request.session_id,
        request.project_id,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to get conversation: {}", e)))?;

    // Add user message to conversation
    AgentConversationMessage::add_user_message(pool, conversation.id, &request.message)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to save user message: {}", e)))?;

    // Load recent conversation history for context
    let history = AgentConversationMessage::find_recent(pool, conversation.id, 20)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load history: {}", e)))?;

    // Convert to LLM conversation messages
    let conversation_messages: Vec<ConversationMessage> = history
        .iter()
        .filter_map(|msg| {
            match msg.role.as_str() {
                "user" => Some(ConversationMessage::user(&msg.content)),
                "assistant" => Some(ConversationMessage::assistant(&msg.content)),
                _ => None, // Skip system and tool messages for now
            }
        })
        .collect();

    // Create LLM client for this agent
    let llm = create_client_for_agent(&agent);

    // Load project-scoped context if project is specified
    let project_context = if let Some(project_id) = request.project_id {
        let scope = ProjectScopedContext::new(project_id, pool.clone());
        match scope.build_context().await {
            Ok(ctx) => Some(ctx),
            Err(e) => {
                tracing::warn!("Failed to build project context: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Build context string with workflow results for conversational continuity
    let context = build_context_string_with_workflows(
        &request.context,
        request.project_id,
        project_context.as_deref(),
        agent_id,
        pool,
    ).await;

    // VIBE Balance Check (if project is specified)
    let vibe_pricing = VibePricingService::new(pool.clone());
    if let Some(project_id) = request.project_id {
        if let Ok(Some(project)) = Project::find_by_id(pool, project_id).await {
            // Check if project has budget and hasn't exceeded it
            if let Some(budget_limit) = project.vibe_budget_limit {
                let spent = project.vibe_spent_amount;
                // Estimate cost (rough: assume 2000 input tokens, 500 output for a typical chat)
                let estimate = vibe_pricing.estimate_cost(
                    agent.default_model.as_deref().unwrap_or("gpt-4o"),
                    2000,  // Estimated input tokens
                    500,   // Estimated output tokens
                ).await.ok();

                if let Some(est) = estimate {
                    let remaining = budget_limit - spent;
                    if remaining < est.cost_vibe {
                        tracing::warn!(
                            "[VIBE] Insufficient budget for project {}: remaining={}, estimated={}",
                            project_id, remaining, est.cost_vibe
                        );
                        return Err(ApiError::PaymentRequired(format!(
                            "Insufficient VIBE balance. Remaining: {} VIBE, Estimated cost: {} VIBE",
                            remaining, est.cost_vibe
                        )));
                    }
                }
            }
        }
    }

    // Generate response
    let llm_response = llm
        .generate_with_tools_and_history(
            "", // Use agent's default system prompt
            &request.message,
            &context,
            &[], // No tools for now - can add later
            &conversation_messages,
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("LLM error: {}", e)))?;

    // Extract response content and usage
    let (content, usage) = match llm_response {
        LLMResponse::Text { content: text, usage } => (text, usage),
        LLMResponse::ToolCalls { calls, usage } => {
            // For now, just describe the tool calls
            let text = format!(
                "I would like to use the following tools: {}",
                calls
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            (text, usage)
        }
    };

    // Extract token counts from usage
    let (input_tokens, output_tokens) = usage
        .as_ref()
        .map(|u| (u.input_tokens as i64, u.output_tokens as i64))
        .unwrap_or((0, 0));

    // Log token usage if available
    if let Some(u) = &usage {
        tracing::info!("[AGENT_CHAT] Token usage: {} input, {} output", u.input_tokens, u.output_tokens);
    }

    let latency_ms = start.elapsed().as_millis() as i64;

    // Get model/provider info from agent config
    let model = agent.default_model.clone();
    let provider = model.as_ref().map(|m| {
        if m.starts_with("claude") {
            "anthropic".to_string()
        } else {
            "openai".to_string()
        }
    });

    // Record VIBE usage (if project is specified and we have token counts)
    let mut vibe_earned: i64 = 0;
    if let Some(project_id) = request.project_id {
        if input_tokens > 0 || output_tokens > 0 {
            match vibe_pricing.record_llm_usage(
                VibeSourceType::Project,
                project_id,
                model.as_deref().unwrap_or("gpt-4o"),
                input_tokens,
                output_tokens,
                None,  // task_id
                None,  // task_attempt_id
                None,  // process_id
            ).await {
                Ok(tx) => {
                    vibe_earned = tx.amount_vibe;
                    tracing::info!(
                        "[VIBE] Recorded {} VIBE usage for project {} (tx: {})",
                        tx.amount_vibe, project_id, tx.id
                    );
                    // Update project spent amount
                    if let Err(e) = Project::adjust_vibe_spent(pool, project_id, tx.amount_vibe).await {
                        tracing::error!("[VIBE] Failed to update project spent amount: {}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("[VIBE] Failed to record usage: {}", e);
                }
            }
        }
    }

    // Credit agent's wallet with earned VIBE
    if vibe_earned > 0 {
        // Use agent's short_name as wallet profile_key
        let profile_key = agent.short_name.to_lowercase();

        match AgentWallet::find_by_profile_key(pool, &profile_key).await {
            Ok(Some(wallet)) => {
                // Create credit transaction for agent
                let tx_result = AgentWalletTransaction::create(
                    pool,
                    &CreateWalletTransaction {
                        wallet_id: wallet.id,
                        direction: "credit".to_string(),
                        amount: vibe_earned,
                        description: Some(format!("Earned from chat session {}", request.session_id)),
                        metadata: Some(serde_json::json!({
                            "input_tokens": input_tokens,
                            "output_tokens": output_tokens,
                            "model": model,
                            "project_id": request.project_id
                        }).to_string()),
                        task_id: None,
                        process_id: None,
                    },
                ).await;

                match tx_result {
                    Ok(tx) => {
                        tracing::info!(
                            "[VIBE] Credited {} VIBE to agent {} wallet (tx: {})",
                            vibe_earned, agent.short_name, tx.id
                        );
                    }
                    Err(e) => {
                        tracing::error!("[VIBE] Failed to credit agent wallet: {}", e);
                    }
                }
            }
            Ok(None) => {
                tracing::warn!(
                    "[VIBE] No wallet found for agent '{}' (profile_key: {}), skipping credit",
                    agent.short_name, profile_key
                );
            }
            Err(e) => {
                tracing::error!("[VIBE] Failed to lookup agent wallet: {}", e);
            }
        }
    }

    // Save assistant response to conversation
    AgentConversationMessage::add_assistant_message(
        pool,
        conversation.id,
        &content,
        model.as_deref(),
        provider.as_deref(),
        Some(input_tokens),
        Some(output_tokens),
        Some(latency_ms),
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to save response: {}", e)))?;

    Ok(Json(AgentChatResponse {
        content,
        conversation_id: conversation.id,
        agent_name: agent.short_name,
        agent_designation: agent.designation,
        input_tokens: Some(input_tokens),
        output_tokens: Some(output_tokens),
        model,
        provider,
        latency_ms,
    }))
}

/// Stream chat response from an agent
#[axum::debug_handler]
pub async fn agent_chat_stream(
    State(state): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
    Json(request): Json<AgentChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError> {
    use futures::stream::StreamExt;

    tracing::info!(
        "Agent chat stream request: agent={}, session={}",
        agent_id,
        request.session_id
    );

    // Get database pool
    let pool = state.db().pool.clone();

    // Load agent from database
    let agent = Agent::find_by_id(&pool, agent_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load agent: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("Agent {} not found", agent_id)))?;

    // Get or create conversation
    let conversation = AgentConversation::get_or_create(
        &pool,
        agent_id,
        &request.session_id,
        request.project_id,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to get conversation: {}", e)))?;

    // Add user message
    AgentConversationMessage::add_user_message(&pool, conversation.id, &request.message)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to save user message: {}", e)))?;

    // Create LLM client for this agent
    let llm = create_client_for_agent(&agent);

    // Load project-scoped context if project is specified
    let project_context = if let Some(project_id) = request.project_id {
        let scope = ProjectScopedContext::new(project_id, pool.clone());
        match scope.build_context().await {
            Ok(ctx) => Some(ctx),
            Err(e) => {
                tracing::warn!("Failed to build project context for stream: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Build context with workflow results for conversational continuity
    let context = build_context_string_with_workflows(
        &request.context,
        request.project_id,
        project_context.as_deref(),
        agent_id,
        &pool,
    ).await;

    // Get streaming response
    let stream_result = llm
        .generate_stream("", &request.message, &context)
        .await
        .map_err(|e| ApiError::InternalError(format!("LLM stream error: {}", e)))?;

    // Collect full response for saving
    let agent_name = agent.short_name.clone();
    let conv_id = conversation.id;
    let model = agent.default_model.clone();
    let provider = model.as_ref().map(|m| {
        if m.starts_with("claude") {
            "anthropic".to_string()
        } else {
            "openai".to_string()
        }
    });

    // Wrap in Arc for sharing
    let full_response = Arc::new(tokio::sync::Mutex::new(String::new()));
    let full_response_clone = full_response.clone();
    let pool_clone = pool.clone();

    // Create SSE stream
    let sse_stream = stream_result
        .map(move |chunk_result| {
            let full_response = full_response_clone.clone();

            match chunk_result {
                Ok(chunk) => {
                    // Accumulate the response
                    let rt = tokio::runtime::Handle::current();
                    rt.block_on(async {
                        let mut resp = full_response.lock().await;
                        resp.push_str(&chunk);
                    });

                    Ok(Event::default().data(chunk))
                }
                Err(e) => {
                    tracing::error!("Stream chunk error: {}", e);
                    Ok(Event::default().data(format!("[ERROR] {}", e)))
                }
            }
        })
        .chain(futures::stream::once(async move {
            // Save the complete response when stream ends
            let response_content = full_response.lock().await.clone();
            if !response_content.is_empty() {
                let _ = AgentConversationMessage::add_assistant_message(
                    &pool_clone,
                    conv_id,
                    &response_content,
                    model.as_deref(),
                    provider.as_deref(),
                    None,
                    None,
                    None,
                )
                .await;
            }

            Ok(Event::default().event("done").data(format!(
                r#"{{"conversation_id": "{}", "agent": "{}"}}"#,
                conv_id, agent_name
            )))
        }));

    Ok(Sse::new(sse_stream))
}

/// List conversations for an agent
pub async fn list_conversations(
    State(state): State<DeploymentImpl>,
    Path(agent_id): Path<Uuid>,
    Query(query): Query<ListConversationsQuery>,
) -> Result<Json<Vec<ConversationSummary>>, ApiError> {
    let pool = &state.db().pool;

    let limit = query.limit.unwrap_or(50);

    let conversations = if let Some(project_id) = query.project_id {
        // Filter by project
        AgentConversation::find_by_project(pool, project_id, limit).await
    } else {
        AgentConversation::find_by_agent(pool, agent_id, limit).await
    }
    .map_err(|e| ApiError::InternalError(format!("Failed to load conversations: {}", e)))?;

    let summaries: Vec<ConversationSummary> = conversations
        .into_iter()
        .map(|c| ConversationSummary {
            id: c.id,
            title: c.title,
            status: c.status,
            message_count: c.message_count,
            last_message_at: c.last_message_at.map(|dt| dt.to_rfc3339()),
            created_at: c.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(summaries))
}

/// Get a specific conversation
pub async fn get_conversation(
    State(state): State<DeploymentImpl>,
    Path((agent_id, conversation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<AgentConversation>, ApiError> {
    let pool = &state.db().pool;

    let conversation = AgentConversation::find_by_id(pool, conversation_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load conversation: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

    // Verify conversation belongs to this agent
    if conversation.agent_id != agent_id {
        return Err(ApiError::NotFound("Conversation not found".to_string()));
    }

    Ok(Json(conversation))
}

/// Get conversation by session ID (returns conversation with messages)
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConversationWithMessages {
    pub conversation: AgentConversation,
    pub messages: Vec<AgentConversationMessage>,
}

pub async fn get_conversation_by_session(
    State(state): State<DeploymentImpl>,
    Path((agent_id, session_id)): Path<(Uuid, String)>,
) -> Result<Json<Option<ConversationWithMessages>>, ApiError> {
    let pool = &state.db().pool;

    // Find conversation by agent and session
    let conversation = AgentConversation::find_by_agent_session(pool, agent_id, &session_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load conversation: {}", e)))?;

    match conversation {
        Some(conv) => {
            // Load messages for this conversation
            let messages = AgentConversationMessage::find_by_conversation(pool, conv.id, None)
                .await
                .map_err(|e| ApiError::InternalError(format!("Failed to load messages: {}", e)))?;

            Ok(Json(Some(ConversationWithMessages {
                conversation: conv,
                messages,
            })))
        }
        None => Ok(Json(None)),
    }
}

/// Get messages for a conversation
pub async fn get_conversation_messages(
    State(state): State<DeploymentImpl>,
    Path((agent_id, conversation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<AgentConversationMessage>>, ApiError> {
    let pool = &state.db().pool;

    // Verify conversation exists and belongs to agent
    let conversation = AgentConversation::find_by_id(pool, conversation_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load conversation: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("Conversation not found".to_string()))?;

    if conversation.agent_id != agent_id {
        return Err(ApiError::NotFound("Conversation not found".to_string()));
    }

    let messages = AgentConversationMessage::find_by_conversation(pool, conversation_id, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load messages: {}", e)))?;

    Ok(Json(messages))
}

/// Build context string from request context and project ID
fn build_context_string(
    context: &Option<serde_json::Value>,
    project_id: Option<Uuid>,
    project_context: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    // Include rich project context if available
    if let Some(proj_ctx) = project_context {
        parts.push(proj_ctx.to_string());
    } else if let Some(project_id) = project_id {
        // Fallback to just the project ID
        parts.push(format!("Project ID: {}", project_id));
    }

    // Include any additional context from the request
    if let Some(ctx) = context {
        if let Some(obj) = ctx.as_object() {
            for (key, value) in obj {
                parts.push(format!("{}: {}", key, value));
            }
        } else {
            parts.push(ctx.to_string());
        }
    }

    if parts.is_empty() {
        "No additional context provided.".to_string()
    } else {
        parts.join("\n")
    }
}

/// Build context string from request context, project ID, and workflow results
async fn build_context_string_with_workflows(
    context: &Option<serde_json::Value>,
    project_id: Option<Uuid>,
    project_context: Option<&str>,
    agent_id: Uuid,
    pool: &sqlx::SqlitePool,
) -> String {
    let mut parts = Vec::new();

    // Include rich project context if available
    if let Some(proj_ctx) = project_context {
        parts.push(proj_ctx.to_string());
    } else if let Some(project_id) = project_id {
        parts.push(format!("Project ID: {}", project_id));
    }

    // Include any additional context from the request
    if let Some(ctx) = context {
        if let Some(obj) = ctx.as_object() {
            for (key, value) in obj {
                parts.push(format!("{}: {}", key, value));
            }
        } else {
            parts.push(ctx.to_string());
        }
    }

    // Fetch recent workflow execution results for this agent
    if let Ok(workflow_context) = fetch_recent_workflow_results(agent_id, pool).await {
        if !workflow_context.is_empty() {
            parts.push(workflow_context);
        }
    }

    if parts.is_empty() {
        "No additional context provided.".to_string()
    } else {
        parts.join("\n\n")
    }
}

/// Fetch recent workflow execution results for an agent to provide conversational context
async fn fetch_recent_workflow_results(
    agent_id: Uuid,
    pool: &sqlx::SqlitePool,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // First, get the agent's short_name to match in event_data
    let agent_name: Option<String> = sqlx::query_scalar(
        r#"SELECT short_name FROM agents WHERE id = ?1"#
    )
    .bind(agent_id)
    .fetch_optional(pool)
    .await?;

    let agent_name = match agent_name {
        Some(name) => name,
        None => return Ok(String::new()),
    };

    // Query recent flow events that mention this agent in event_data
    // Since executor_agent_id may be NULL, we search by agent_name in the JSON
    let events: Vec<AgentFlowEvent> = sqlx::query_as(
        r#"
        SELECT * FROM agent_flow_events
        WHERE created_at > datetime('now', '-24 hours')
          AND event_type IN ('phase_completed', 'flow_completed')
          AND event_data LIKE ?1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(format!("%\"agent_name\":\"{}\"%", agent_name))
    .fetch_all(pool)
    .await?;

    if events.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();
    output.push_str("═══════════════════════════════════════════════════════════════\n");
    output.push_str("                    RECENT WORKFLOW RESULTS                     \n");
    output.push_str("═══════════════════════════════════════════════════════════════\n\n");

    // Group events by flow and format them
    let mut current_flow_id: Option<Uuid> = None;

    for event in events.iter().rev() {
        // Parse the event data
        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&event.event_data) {
            // New flow header
            if current_flow_id != Some(event.agent_flow_id) {
                current_flow_id = Some(event.agent_flow_id);
                output.push_str("───────────────────────────────────────────────────────────────\n");
                output.push_str(&format!("Workflow: {}\n", event.agent_flow_id));
                output.push_str("───────────────────────────────────────────────────────────────\n");
            }

            let event_type = event.event_type.to_string();

            if event_type == "phase_completed" {
                let phase = data.get("phase")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let duration = data.get("duration_ms")
                    .and_then(|v| v.as_u64())
                    .map(|ms| format!("{}ms", ms))
                    .unwrap_or_default();
                let agent_name = data.get("agent_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                output.push_str(&format!("\n▶ Stage: {} (by {})\n", phase, agent_name));
                if !duration.is_empty() {
                    output.push_str(&format!("  Duration: {}\n", duration));
                }

                // Include the full output - this is key for conversationality
                if let Some(stage_output) = data.get("output") {
                    output.push_str("  Output:\n");
                    let output_str = if let Some(s) = stage_output.as_str() {
                        s.to_string()
                    } else {
                        serde_json::to_string_pretty(stage_output).unwrap_or_default()
                    };

                    // Format each line with indentation
                    for line in output_str.lines() {
                        output.push_str(&format!("    {}\n", line));
                    }
                }
            } else if event_type == "flow_completed" {
                let total_artifacts = data.get("total_artifacts")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let score = data.get("verification_score")
                    .and_then(|v| v.as_f64());

                output.push_str("\n✓ WORKFLOW COMPLETED\n");
                output.push_str(&format!("  Artifacts produced: {}\n", total_artifacts));
                if let Some(s) = score {
                    output.push_str(&format!("  Verification score: {:.1}%\n", s * 100.0));
                }
            }
        }
    }

    output.push_str("\n═══════════════════════════════════════════════════════════════\n");
    output.push_str("You can reference any of the above results in this conversation.\n");
    output.push_str("═══════════════════════════════════════════════════════════════\n");

    Ok(output)
}
