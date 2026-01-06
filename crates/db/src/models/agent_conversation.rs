//! Agent conversation models and database operations
//!
//! Provides persistent storage for agent conversations, enabling agents to
//! maintain context across sessions and learn from past interactions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Conversation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ConversationStatus {
    Active,
    Archived,
    Expired,
}

impl Default for ConversationStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl std::fmt::Display for ConversationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversationStatus::Active => write!(f, "active"),
            ConversationStatus::Archived => write!(f, "archived"),
            ConversationStatus::Expired => write!(f, "expired"),
        }
    }
}

impl std::str::FromStr for ConversationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            "expired" => Ok(Self::Expired),
            _ => Err(format!("Unknown status: {}", s)),
        }
    }
}

/// Message role in conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageRole::User => write!(f, "user"),
            MessageRole::Assistant => write!(f, "assistant"),
            MessageRole::System => write!(f, "system"),
            MessageRole::Tool => write!(f, "tool"),
        }
    }
}

impl std::str::FromStr for MessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            "tool" => Ok(Self::Tool),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

/// Agent conversation entity
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AgentConversation {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub session_id: String,
    pub project_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub status: String,
    pub title: Option<String>,
    pub context_snapshot: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub message_count: i64,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
}

/// Conversation message entity
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AgentConversationMessage {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_arguments: Option<String>,
    pub tool_result: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub latency_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new conversation
#[derive(Debug, Clone)]
pub struct CreateConversation {
    pub agent_id: Uuid,
    pub session_id: String,
    pub project_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub title: Option<String>,
    pub context_snapshot: Option<String>,
}

/// Data for creating a new message
#[derive(Debug, Clone)]
pub struct CreateMessage {
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_arguments: Option<String>,
    pub tool_result: Option<String>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub latency_ms: Option<i64>,
}

impl AgentConversation {
    /// Create a new conversation
    pub async fn create(pool: &SqlitePool, data: CreateConversation) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO agent_conversations (
                id, agent_id, session_id, project_id, user_id,
                status, title, context_snapshot, created_at, updated_at,
                message_count, total_input_tokens, total_output_tokens
            )
            VALUES (?, ?, ?, ?, ?, 'active', ?, ?, ?, ?, 0, 0, 0)
            "#,
        )
        .bind(id)
        .bind(data.agent_id)
        .bind(&data.session_id)
        .bind(data.project_id)
        .bind(&data.user_id)
        .bind(&data.title)
        .bind(&data.context_snapshot)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Find conversation by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            r#"
            SELECT * FROM agent_conversations WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    /// Find conversation by agent and session
    pub async fn find_by_agent_session(
        pool: &SqlitePool,
        agent_id: Uuid,
        session_id: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            r#"
            SELECT * FROM agent_conversations
            WHERE agent_id = ? AND session_id = ? AND status = 'active'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(agent_id)
        .bind(session_id)
        .fetch_optional(pool)
        .await
    }

    /// Find all conversations for an agent
    pub async fn find_by_agent(
        pool: &SqlitePool,
        agent_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"
            SELECT * FROM agent_conversations
            WHERE agent_id = ?
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Find conversations by project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"
            SELECT * FROM agent_conversations
            WHERE project_id = ?
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Get or create a conversation for agent + session
    pub async fn get_or_create(
        pool: &SqlitePool,
        agent_id: Uuid,
        session_id: &str,
        project_id: Option<Uuid>,
    ) -> sqlx::Result<Self> {
        // Try to find existing conversation
        if let Some(existing) = Self::find_by_agent_session(pool, agent_id, session_id).await? {
            return Ok(existing);
        }

        // Create new conversation
        Self::create(
            pool,
            CreateConversation {
                agent_id,
                session_id: session_id.to_string(),
                project_id,
                user_id: None,
                title: None,
                context_snapshot: None,
            },
        )
        .await
    }

    /// Update conversation status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: ConversationStatus,
    ) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE agent_conversations
            SET status = ?, updated_at = datetime('now', 'subsec')
            WHERE id = ?
            "#,
        )
        .bind(status.to_string())
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update conversation title
    pub async fn update_title(pool: &SqlitePool, id: Uuid, title: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE agent_conversations
            SET title = ?, updated_at = datetime('now', 'subsec')
            WHERE id = ?
            "#,
        )
        .bind(title)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Archive old conversations (for cleanup)
    pub async fn archive_old(pool: &SqlitePool, days_old: i32) -> sqlx::Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE agent_conversations
            SET status = 'archived', updated_at = datetime('now', 'subsec')
            WHERE status = 'active'
            AND updated_at < datetime('now', ? || ' days')
            "#,
        )
        .bind(-days_old)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Delete a conversation and all its messages
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM agent_conversations WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl AgentConversationMessage {
    /// Add a message to a conversation
    pub async fn create(pool: &SqlitePool, data: CreateMessage) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO agent_conversation_messages (
                id, conversation_id, role, content,
                tool_call_id, tool_name, tool_arguments, tool_result,
                input_tokens, output_tokens, model, provider, latency_ms
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(data.conversation_id)
        .bind(data.role.to_string())
        .bind(&data.content)
        .bind(&data.tool_call_id)
        .bind(&data.tool_name)
        .bind(&data.tool_arguments)
        .bind(&data.tool_result)
        .bind(data.input_tokens)
        .bind(data.output_tokens)
        .bind(&data.model)
        .bind(&data.provider)
        .bind(data.latency_ms)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Find message by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM agent_conversation_messages WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Get messages for a conversation (with optional limit)
    pub async fn find_by_conversation(
        pool: &SqlitePool,
        conversation_id: Uuid,
        limit: Option<i64>,
    ) -> sqlx::Result<Vec<Self>> {
        if let Some(limit) = limit {
            sqlx::query_as(
                r#"
                SELECT * FROM agent_conversation_messages
                WHERE conversation_id = ?
                ORDER BY created_at ASC
                LIMIT ?
                "#,
            )
            .bind(conversation_id)
            .bind(limit)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as(
                r#"
                SELECT * FROM agent_conversation_messages
                WHERE conversation_id = ?
                ORDER BY created_at ASC
                "#,
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await
        }
    }

    /// Get recent messages for a conversation (newest first, for context building)
    pub async fn find_recent(
        pool: &SqlitePool,
        conversation_id: Uuid,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        // Get in reverse order then reverse in memory for chronological order
        let messages: Vec<Self> = sqlx::query_as(
            r#"
            SELECT * FROM agent_conversation_messages
            WHERE conversation_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(conversation_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        // Reverse to get chronological order
        Ok(messages.into_iter().rev().collect())
    }

    /// Add a user message (convenience method)
    pub async fn add_user_message(
        pool: &SqlitePool,
        conversation_id: Uuid,
        content: &str,
    ) -> sqlx::Result<Self> {
        Self::create(
            pool,
            CreateMessage {
                conversation_id,
                role: MessageRole::User,
                content: content.to_string(),
                tool_call_id: None,
                tool_name: None,
                tool_arguments: None,
                tool_result: None,
                input_tokens: None,
                output_tokens: None,
                model: None,
                provider: None,
                latency_ms: None,
            },
        )
        .await
    }

    /// Add an assistant message (convenience method)
    pub async fn add_assistant_message(
        pool: &SqlitePool,
        conversation_id: Uuid,
        content: &str,
        model: Option<&str>,
        provider: Option<&str>,
        input_tokens: Option<i64>,
        output_tokens: Option<i64>,
        latency_ms: Option<i64>,
    ) -> sqlx::Result<Self> {
        Self::create(
            pool,
            CreateMessage {
                conversation_id,
                role: MessageRole::Assistant,
                content: content.to_string(),
                tool_call_id: None,
                tool_name: None,
                tool_arguments: None,
                tool_result: None,
                input_tokens,
                output_tokens,
                model: model.map(|s| s.to_string()),
                provider: provider.map(|s| s.to_string()),
                latency_ms,
            },
        )
        .await
    }

    /// Add a tool result message (convenience method)
    pub async fn add_tool_result(
        pool: &SqlitePool,
        conversation_id: Uuid,
        tool_call_id: &str,
        tool_name: &str,
        result: &str,
    ) -> sqlx::Result<Self> {
        Self::create(
            pool,
            CreateMessage {
                conversation_id,
                role: MessageRole::Tool,
                content: result.to_string(),
                tool_call_id: Some(tool_call_id.to_string()),
                tool_name: Some(tool_name.to_string()),
                tool_arguments: None,
                tool_result: Some(result.to_string()),
                input_tokens: None,
                output_tokens: None,
                model: None,
                provider: None,
                latency_ms: None,
            },
        )
        .await
    }
}

/// Token usage summary for a conversation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ConversationTokenUsage {
    pub conversation_id: Uuid,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub message_count: i64,
}

impl AgentConversation {
    /// Get token usage summary for a conversation
    pub async fn get_token_usage(
        pool: &SqlitePool,
        id: Uuid,
    ) -> sqlx::Result<ConversationTokenUsage> {
        let conv = Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        let input = conv.total_input_tokens.unwrap_or(0);
        let output = conv.total_output_tokens.unwrap_or(0);

        Ok(ConversationTokenUsage {
            conversation_id: id,
            total_input_tokens: input,
            total_output_tokens: output,
            total_tokens: input + output,
            message_count: conv.message_count,
        })
    }
}
