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
};
use deployment::Deployment;
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

/// Build routes for agent chat
pub fn routes() -> Router<DeploymentImpl> {
    Router::new()
        .route("/agents/{agent_id}/chat", post(agent_chat))
        .route("/agents/{agent_id}/chat/stream", post(agent_chat_stream))
        .route("/agents/{agent_id}/conversations", get(list_conversations))
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

    // Extract response content
    let content = match llm_response {
        LLMResponse::Text(text) => text,
        LLMResponse::ToolCalls(calls) => {
            // For now, just describe the tool calls
            format!(
                "I would like to use the following tools: {}",
                calls
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    };

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

    // Save assistant response to conversation
    AgentConversationMessage::add_assistant_message(
        pool,
        conversation.id,
        &content,
        model.as_deref(),
        provider.as_deref(),
        None, // TODO: Get actual token counts
        None,
        Some(latency_ms),
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to save response: {}", e)))?;

    Ok(Json(AgentChatResponse {
        content,
        conversation_id: conversation.id,
        agent_name: agent.short_name,
        agent_designation: agent.designation,
        input_tokens: None,
        output_tokens: None,
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
