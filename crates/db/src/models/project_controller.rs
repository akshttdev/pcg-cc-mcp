//! Project Controller models and database operations
//!
//! Provides AI controller configuration and conversation storage
//! scoped to individual projects.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Message role in project controller conversation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum ControllerMessageRole {
    User,
    Assistant,
    System,
}

impl std::fmt::Display for ControllerMessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControllerMessageRole::User => write!(f, "user"),
            ControllerMessageRole::Assistant => write!(f, "assistant"),
            ControllerMessageRole::System => write!(f, "system"),
        }
    }
}

impl std::str::FromStr for ControllerMessageRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

/// Project controller configuration
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectControllerConfig {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub personality: String,
    pub system_prompt: Option<String>,
    pub voice_id: Option<String>,
    pub avatar_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new controller config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateControllerConfig {
    pub project_id: String,
    pub name: Option<String>,
    pub personality: Option<String>,
    pub system_prompt: Option<String>,
    pub voice_id: Option<String>,
    pub avatar_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
}

/// Data for updating controller config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateControllerConfig {
    pub name: Option<String>,
    pub personality: Option<String>,
    pub system_prompt: Option<String>,
    pub voice_id: Option<String>,
    pub avatar_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i32>,
}

impl ProjectControllerConfig {
    /// Create a new controller config for a project
    pub async fn create(pool: &SqlitePool, data: CreateControllerConfig) -> sqlx::Result<Self> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO project_controller_config (
                id, project_id, name, personality, system_prompt,
                voice_id, avatar_url, model, temperature, max_tokens
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&data.project_id)
        .bind(data.name.as_deref().unwrap_or("Controller"))
        .bind(data.personality.as_deref().unwrap_or("professional"))
        .bind(&data.system_prompt)
        .bind(&data.voice_id)
        .bind(&data.avatar_url)
        .bind(data.model.as_deref().unwrap_or("gpt-4o-mini"))
        .bind(data.temperature.unwrap_or(0.7))
        .bind(data.max_tokens.unwrap_or(2048))
        .execute(pool)
        .await?;

        Self::find_by_id(pool, &id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Find controller config by ID
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM project_controller_config WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find controller config by project ID
    pub async fn find_by_project(pool: &SqlitePool, project_id: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM project_controller_config WHERE project_id = ?")
            .bind(project_id)
            .fetch_optional(pool)
            .await
    }

    /// Get or create controller config for a project
    pub async fn get_or_create(pool: &SqlitePool, project_id: &str) -> sqlx::Result<Self> {
        if let Some(existing) = Self::find_by_project(pool, project_id).await? {
            return Ok(existing);
        }

        Self::create(
            pool,
            CreateControllerConfig {
                project_id: project_id.to_string(),
                name: None,
                personality: None,
                system_prompt: None,
                voice_id: None,
                avatar_url: None,
                model: None,
                temperature: None,
                max_tokens: None,
            },
        )
        .await
    }

    /// Update controller config
    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        data: UpdateControllerConfig,
    ) -> sqlx::Result<Self> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)?;

        sqlx::query(
            r#"
            UPDATE project_controller_config
            SET name = ?, personality = ?, system_prompt = ?,
                voice_id = ?, avatar_url = ?, model = ?,
                temperature = ?, max_tokens = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(data.name.unwrap_or(existing.name))
        .bind(data.personality.unwrap_or(existing.personality))
        .bind(data.system_prompt.or(existing.system_prompt))
        .bind(data.voice_id.or(existing.voice_id))
        .bind(data.avatar_url.or(existing.avatar_url))
        .bind(data.model.or(existing.model))
        .bind(data.temperature.or(existing.temperature))
        .bind(data.max_tokens.or(existing.max_tokens))
        .bind(id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Delete controller config
    pub async fn delete(pool: &SqlitePool, id: &str) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM project_controller_config WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Project controller conversation
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectControllerConversation {
    pub id: String,
    pub project_id: String,
    pub user_id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Data for creating a new conversation
#[derive(Debug, Clone)]
pub struct CreateControllerConversation {
    pub project_id: String,
    pub user_id: String,
    pub title: Option<String>,
}

impl ProjectControllerConversation {
    /// Create a new conversation
    pub async fn create(
        pool: &SqlitePool,
        data: CreateControllerConversation,
    ) -> sqlx::Result<Self> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO project_controller_conversations (id, project_id, user_id, title)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&data.project_id)
        .bind(&data.user_id)
        .bind(&data.title)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, &id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Find conversation by ID
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM project_controller_conversations WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Find conversations for a project and user
    pub async fn find_by_project_user(
        pool: &SqlitePool,
        project_id: &str,
        user_id: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"
            SELECT * FROM project_controller_conversations
            WHERE project_id = ? AND user_id = ?
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(project_id)
        .bind(user_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    /// Find all conversations for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            r#"
            SELECT * FROM project_controller_conversations
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

    /// Get or create conversation for project + user
    pub async fn get_or_create(
        pool: &SqlitePool,
        project_id: &str,
        user_id: &str,
    ) -> sqlx::Result<Self> {
        // Get most recent conversation
        let existing = Self::find_by_project_user(pool, project_id, user_id, 1).await?;
        if let Some(conv) = existing.into_iter().next() {
            return Ok(conv);
        }

        Self::create(
            pool,
            CreateControllerConversation {
                project_id: project_id.to_string(),
                user_id: user_id.to_string(),
                title: None,
            },
        )
        .await
    }

    /// Update conversation title
    pub async fn update_title(pool: &SqlitePool, id: &str, title: &str) -> sqlx::Result<()> {
        sqlx::query(
            r#"
            UPDATE project_controller_conversations
            SET title = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(title)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Delete conversation and all messages
    pub async fn delete(pool: &SqlitePool, id: &str) -> sqlx::Result<()> {
        sqlx::query("DELETE FROM project_controller_conversations WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

/// Project controller message
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectControllerMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub tokens_used: Option<i32>,
    pub created_at: String,
}

/// Data for creating a new message
#[derive(Debug, Clone)]
pub struct CreateControllerMessage {
    pub conversation_id: String,
    pub role: ControllerMessageRole,
    pub content: String,
    pub tokens_used: Option<i32>,
}

impl ProjectControllerMessage {
    /// Create a new message
    pub async fn create(pool: &SqlitePool, data: CreateControllerMessage) -> sqlx::Result<Self> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO project_controller_messages (id, conversation_id, role, content, tokens_used)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&data.conversation_id)
        .bind(data.role.to_string())
        .bind(&data.content)
        .bind(data.tokens_used)
        .execute(pool)
        .await?;

        // Update conversation's updated_at
        sqlx::query(
            "UPDATE project_controller_conversations SET updated_at = datetime('now') WHERE id = ?",
        )
        .bind(&data.conversation_id)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, &id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// Find message by ID
    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM project_controller_messages WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    /// Get messages for a conversation
    pub async fn find_by_conversation(
        pool: &SqlitePool,
        conversation_id: &str,
        limit: Option<i64>,
    ) -> sqlx::Result<Vec<Self>> {
        if let Some(limit) = limit {
            sqlx::query_as(
                r#"
                SELECT * FROM project_controller_messages
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
                SELECT * FROM project_controller_messages
                WHERE conversation_id = ?
                ORDER BY created_at ASC
                "#,
            )
            .bind(conversation_id)
            .fetch_all(pool)
            .await
        }
    }

    /// Get recent messages (for context building)
    pub async fn find_recent(
        pool: &SqlitePool,
        conversation_id: &str,
        limit: i64,
    ) -> sqlx::Result<Vec<Self>> {
        let messages: Vec<Self> = sqlx::query_as(
            r#"
            SELECT * FROM project_controller_messages
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
        conversation_id: &str,
        content: &str,
    ) -> sqlx::Result<Self> {
        Self::create(
            pool,
            CreateControllerMessage {
                conversation_id: conversation_id.to_string(),
                role: ControllerMessageRole::User,
                content: content.to_string(),
                tokens_used: None,
            },
        )
        .await
    }

    /// Add an assistant message (convenience method)
    pub async fn add_assistant_message(
        pool: &SqlitePool,
        conversation_id: &str,
        content: &str,
        tokens_used: Option<i32>,
    ) -> sqlx::Result<Self> {
        Self::create(
            pool,
            CreateControllerMessage {
                conversation_id: conversation_id.to_string(),
                role: ControllerMessageRole::Assistant,
                content: content.to_string(),
                tokens_used,
            },
        )
        .await
    }
}
