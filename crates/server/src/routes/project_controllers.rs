//! Project Controller API routes
//!
//! Handles project-specific AI controller configuration and chat functionality.

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use db::models::{
    project::Project,
    project_controller::{
        ProjectControllerConfig, ProjectControllerConversation,
        ProjectControllerMessage, UpdateControllerConfig,
    },
};
use deployment::Deployment;
use nora::brain::{
    ConversationMessage, LLMClient, LLMConfig, LLMResponse,
    infer_provider_from_model,
};
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateConfigPayload {
    pub name: Option<String>,
    pub personality: Option<String>,
    pub system_prompt: Option<String>,
    pub voice_id: Option<String>,
    pub avatar_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessagePayload {
    pub content: String,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: ProjectControllerMessage,
    pub conversation_id: String,
}

#[derive(Debug, Serialize)]
pub struct ConversationWithMessages {
    pub conversation: ProjectControllerConversation,
    pub messages: Vec<ProjectControllerMessage>,
}

// ============================================================================
// Controller Config Endpoints
// ============================================================================

/// Get or create the controller config for a project
async fn get_controller_config(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<ProjectControllerConfig>>, ApiError> {
    let config =
        ProjectControllerConfig::get_or_create(&deployment.db().pool, &project.id.to_string())
            .await?;
    Ok(Json(ApiResponse::success(config)))
}

/// Update the controller config for a project
async fn update_controller_config(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateConfigPayload>,
) -> Result<Json<ApiResponse<ProjectControllerConfig>>, ApiError> {
    // First ensure config exists
    let existing =
        ProjectControllerConfig::get_or_create(&deployment.db().pool, &project.id.to_string())
            .await?;

    let config = ProjectControllerConfig::update(
        &deployment.db().pool,
        &existing.id,
        UpdateControllerConfig {
            name: payload.name,
            personality: payload.personality,
            system_prompt: payload.system_prompt,
            voice_id: payload.voice_id,
            avatar_url: payload.avatar_url,
            model: payload.model,
            temperature: payload.temperature,
            max_tokens: payload.max_tokens,
        },
    )
    .await?;

    Ok(Json(ApiResponse::success(config)))
}

// ============================================================================
// Conversation Endpoints
// ============================================================================

/// List conversations for the current user in this project
async fn list_conversations(
    Extension(project): Extension<Project>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ProjectControllerConversation>>>, ApiError> {
    let user_id = access_context.user_id.to_string();
    let conversations = ProjectControllerConversation::find_by_project_user(
        &deployment.db().pool,
        &project.id.to_string(),
        &user_id,
        50,
    )
    .await?;

    Ok(Json(ApiResponse::success(conversations)))
}

/// Get a specific conversation with messages
async fn get_conversation(
    Extension(project): Extension<Project>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(conversation_id): Path<String>,
) -> Result<Json<ApiResponse<ConversationWithMessages>>, ApiError> {
    let user_id = access_context.user_id.to_string();
    let conversation =
        ProjectControllerConversation::find_by_id(&deployment.db().pool, &conversation_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

    // Verify access
    if conversation.project_id != project.id.to_string() {
        return Err(ApiError::BadRequest(
            "Conversation does not belong to this project".into(),
        ));
    }
    if conversation.user_id != user_id {
        return Err(ApiError::Forbidden(
            "You do not have access to this conversation".into(),
        ));
    }

    let messages =
        ProjectControllerMessage::find_by_conversation(&deployment.db().pool, &conversation_id, None)
            .await?;

    Ok(Json(ApiResponse::success(ConversationWithMessages {
        conversation,
        messages,
    })))
}

/// Delete a conversation
async fn delete_conversation(
    Extension(project): Extension<Project>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Path(conversation_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let user_id = access_context.user_id.to_string();
    let conversation =
        ProjectControllerConversation::find_by_id(&deployment.db().pool, &conversation_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

    // Verify access
    if conversation.project_id != project.id.to_string() {
        return Err(ApiError::BadRequest(
            "Conversation does not belong to this project".into(),
        ));
    }
    if conversation.user_id != user_id && !access_context.is_admin {
        return Err(ApiError::Forbidden(
            "You do not have access to delete this conversation".into(),
        ));
    }

    ProjectControllerConversation::delete(&deployment.db().pool, &conversation_id).await?;

    Ok(Json(ApiResponse::success(())))
}

// ============================================================================
// LLM Client Creation
// ============================================================================

/// Create an LLM client configured for a project controller
fn create_client_for_controller(config: &ProjectControllerConfig, project: &Project) -> LLMClient {
    // Get model name (default to gpt-4o-mini if not specified)
    let model = config.model.as_deref().unwrap_or("gpt-4o-mini");

    // Infer provider from model name
    let provider = infer_provider_from_model(model);

    // Use model name directly - providers handle normalization internally
    let normalized_model = model.to_string();

    // Build system prompt
    let base_system_prompt = format!(
        "You are {}, an AI assistant for the {} project. {}",
        config.name,
        project.name,
        match config.personality.as_str() {
            "professional" => "You are professional, efficient, and focused on delivering results.",
            "friendly" => "You are friendly, approachable, and helpful.",
            "creative" => "You are creative, innovative, and think outside the box.",
            "technical" => "You are technical, precise, and detail-oriented.",
            _ => "You are helpful and responsive to user needs.",
        }
    );

    // Append custom system prompt if provided
    let system_prompt = if let Some(custom_prompt) = &config.system_prompt {
        if !custom_prompt.is_empty() {
            format!("{}\n\n{}", base_system_prompt, custom_prompt)
        } else {
            base_system_prompt
        }
    } else {
        base_system_prompt
    };

    let temperature = config.temperature.unwrap_or(0.7) as f32;
    let max_tokens = config.max_tokens.unwrap_or(2048) as u32;

    tracing::info!(
        "Creating LLM client for project controller '{}': provider={:?}, model={}, temp={}",
        config.name,
        provider,
        normalized_model,
        temperature
    );

    let llm_config = LLMConfig {
        provider,
        model: normalized_model,
        temperature,
        max_tokens,
        system_prompt,
        endpoint: None,
    };

    LLMClient::new(llm_config)
}

// ============================================================================
// Chat Endpoint
// ============================================================================

/// Send a message to the project controller
async fn send_message(
    Extension(project): Extension<Project>,
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<SendMessagePayload>,
) -> Result<(StatusCode, Json<ApiResponse<ChatResponse>>), ApiError> {
    let start = std::time::Instant::now();
    let project_id = project.id.to_string();
    let user_id = access_context.user_id.to_string();

    // Get or create conversation
    let conversation = if let Some(conv_id) = payload.conversation_id {
        let conv = ProjectControllerConversation::find_by_id(&deployment.db().pool, &conv_id)
            .await?
            .ok_or_else(|| ApiError::NotFound("Conversation not found".into()))?;

        // Verify access
        if conv.project_id != project_id {
            return Err(ApiError::BadRequest(
                "Conversation does not belong to this project".into(),
            ));
        }
        if conv.user_id != user_id {
            return Err(ApiError::Forbidden(
                "You do not have access to this conversation".into(),
            ));
        }
        conv
    } else {
        ProjectControllerConversation::get_or_create(&deployment.db().pool, &project_id, &user_id)
            .await?
    };

    // Save user message
    ProjectControllerMessage::add_user_message(
        &deployment.db().pool,
        &conversation.id,
        &payload.content,
    )
    .await?;

    // Get controller config for response generation
    let config = ProjectControllerConfig::get_or_create(&deployment.db().pool, &project_id).await?;

    // Load conversation history for context
    let history = ProjectControllerMessage::find_by_conversation(
        &deployment.db().pool,
        &conversation.id,
        Some(20), // Last 20 messages for context
    )
    .await?;

    // Convert to LLM conversation messages (exclude the message we just added)
    let conversation_messages: Vec<ConversationMessage> = history
        .iter()
        .rev() // Messages come in DESC order, reverse for chronological
        .take(history.len().saturating_sub(1)) // Skip the latest (user message we just added)
        .filter_map(|msg| match msg.role.as_str() {
            "user" => Some(ConversationMessage::user(&msg.content)),
            "assistant" => Some(ConversationMessage::assistant(&msg.content)),
            _ => None,
        })
        .collect();

    // Create LLM client for this controller
    let llm = create_client_for_controller(&config, &project);

    // Generate response from LLM
    let response_content = if llm.is_ready() {
        // Build context with project information
        let context = format!(
            "Project: {}\nRepository: {}",
            project.name,
            project.git_repo_path.display()
        );

        match llm
            .generate_with_tools_and_history(
                "", // Use the system prompt from config
                &payload.content,
                &context,
                &[], // No tools for now
                &conversation_messages,
            )
            .await
        {
            Ok(LLMResponse::Text { content: text, .. }) => text,
            Ok(LLMResponse::ToolCalls { calls, .. }) => {
                format!(
                    "I'd like to help with that. I identified these actions: {}",
                    calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>().join(", ")
                )
            }
            Err(e) => {
                tracing::error!("LLM error for project controller: {}", e);
                format!(
                    "I apologize, but I encountered an issue processing your request. \
                     As {}, I'm here to help with the {} project. Could you please try again?",
                    config.name, project.name
                )
            }
        }
    } else {
        // Fallback if no API key configured
        tracing::warn!("No LLM API key configured for project controller");
        format!(
            "I understand you said: \"{}\". As {}, I'm here to help manage the {} project. \
             Please ensure an API key (OPENAI_API_KEY or ANTHROPIC_API_KEY) is configured \
             to enable full AI capabilities.",
            payload.content, config.name, project.name
        )
    };

    let latency_ms = start.elapsed().as_millis() as i64;
    tracing::info!(
        "Project controller '{}' responded in {}ms",
        config.name,
        latency_ms
    );

    // Save assistant response
    let assistant_message = ProjectControllerMessage::add_assistant_message(
        &deployment.db().pool,
        &conversation.id,
        &response_content,
        None, // tokens_used - can be extracted from LLM response in future
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse::success(ChatResponse {
            message: assistant_message,
            conversation_id: conversation.id,
        })),
    ))
}

// ============================================================================
// Router
// ============================================================================

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/controller",
            get(get_controller_config).put(update_controller_config),
        )
        .route(
            "/controller/conversations",
            get(list_conversations),
        )
        .route(
            "/controller/conversations/{conversation_id}",
            get(get_conversation).delete(delete_conversation),
        )
        .route("/controller/chat", post(send_message))
        .with_state(deployment.clone())
}
