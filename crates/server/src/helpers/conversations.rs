//! Centralized conversation persistence helpers.
//!
//! The pattern of `AgentConversation::get_or_create` + two `add_*_message`
//! calls was duplicated in topsi/chat, nora/chat, nora/voice, agent_chat,
//! and twilio handlers. This module provides a single fire-and-forget helper.

use db::models::agent_conversation::{AgentConversation, AgentConversationMessage};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Persist a user + assistant chat exchange in one call.
///
/// This is designed to be called from a `tokio::spawn` block so that
/// conversation persistence never blocks the HTTP response.
pub async fn persist_chat_exchange(
    pool: &SqlitePool,
    agent_id: Uuid,
    session_id: &str,
    project_id: Option<Uuid>,
    user_msg: &str,
    assistant_msg: &str,
    model: Option<&str>,
    provider: Option<&str>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    label: &str,
) {
    match AgentConversation::get_or_create(pool, agent_id, session_id, project_id).await {
        Ok(conversation) => {
            if let Err(e) =
                AgentConversationMessage::add_user_message(pool, conversation.id, user_msg).await
            {
                tracing::warn!("Failed to persist {} user message: {}", label, e);
            }
            if let Err(e) = AgentConversationMessage::add_assistant_message(
                pool,
                conversation.id,
                assistant_msg,
                model,
                provider,
                input_tokens,
                output_tokens,
                None,
            )
            .await
            {
                tracing::warn!("Failed to persist {} assistant message: {}", label, e);
            }
        }
        Err(e) => tracing::warn!("Failed to get/create {} conversation: {}", label, e),
    }
}
