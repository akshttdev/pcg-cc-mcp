use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BrowserSessionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Browser session not found")]
    NotFound,
    #[error("Invalid browser type: {0}")]
    InvalidBrowserType(String),
    #[error("Invalid session status: {0}")]
    InvalidStatus(String),
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "browser_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum BrowserType {
    Chromium,
    Firefox,
    Webkit,
}

impl std::fmt::Display for BrowserType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrowserType::Chromium => write!(f, "chromium"),
            BrowserType::Firefox => write!(f, "firefox"),
            BrowserType::Webkit => write!(f, "webkit"),
        }
    }
}

impl Default for BrowserType {
    fn default() -> Self {
        BrowserType::Chromium
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "session_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Starting,
    Active,
    Idle,
    Closed,
    Error,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Starting => write!(f, "starting"),
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Idle => write!(f, "idle"),
            SessionStatus::Closed => write!(f, "closed"),
            SessionStatus::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct BrowserSession {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub browser_type: BrowserType,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub headless: bool,
    pub status: SessionStatus,
    pub current_url: Option<String>,
    pub error_message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateBrowserSession {
    pub execution_process_id: Uuid,
    #[serde(default)]
    pub browser_type: BrowserType,
    #[serde(default = "default_viewport_width")]
    pub viewport_width: i32,
    #[serde(default = "default_viewport_height")]
    pub viewport_height: i32,
    #[serde(default = "default_headless")]
    pub headless: bool,
}

fn default_viewport_width() -> i32 { 1280 }
fn default_viewport_height() -> i32 { 720 }
fn default_headless() -> bool { true }

impl BrowserSession {
    /// Create a new browser session
    pub async fn create(
        pool: &SqlitePool,
        data: CreateBrowserSession,
    ) -> Result<Self, BrowserSessionError> {
        let id = Uuid::new_v4();
        let browser_type_str = data.browser_type.to_string();

        let session = sqlx::query_as::<_, BrowserSession>(
            r#"
            INSERT INTO browser_sessions (id, execution_process_id, browser_type, viewport_width, viewport_height, headless)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(browser_type_str)
        .bind(data.viewport_width)
        .bind(data.viewport_height)
        .bind(data.headless)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    /// Find session by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, BrowserSessionError> {
        let session = sqlx::query_as::<_, BrowserSession>(
            r#"SELECT * FROM browser_sessions WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Find session by execution process ID
    pub async fn find_by_execution_process(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Option<Self>, BrowserSessionError> {
        let session = sqlx::query_as::<_, BrowserSession>(
            r#"
            SELECT * FROM browser_sessions
            WHERE execution_process_id = ?1
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(execution_process_id)
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Get all active browser sessions
    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, BrowserSessionError> {
        let sessions = sqlx::query_as::<_, BrowserSession>(
            r#"
            SELECT * FROM browser_sessions
            WHERE status IN ('starting', 'active', 'idle')
            ORDER BY started_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    /// Update session status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: SessionStatus,
        error_message: Option<String>,
    ) -> Result<Self, BrowserSessionError> {
        let status_str = status.to_string();
        let closed_at = if status == SessionStatus::Closed || status == SessionStatus::Error {
            Some("datetime('now', 'subsec')")
        } else {
            None
        };

        let session = if closed_at.is_some() {
            sqlx::query_as::<_, BrowserSession>(
                r#"
                UPDATE browser_sessions
                SET status = ?2, error_message = ?3, closed_at = datetime('now', 'subsec')
                WHERE id = ?1
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(status_str)
            .bind(error_message)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_as::<_, BrowserSession>(
                r#"
                UPDATE browser_sessions
                SET status = ?2, error_message = ?3
                WHERE id = ?1
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(status_str)
            .bind(error_message)
            .fetch_one(pool)
            .await?
        };

        Ok(session)
    }

    /// Update current URL
    pub async fn update_url(
        pool: &SqlitePool,
        id: Uuid,
        url: String,
    ) -> Result<Self, BrowserSessionError> {
        let session = sqlx::query_as::<_, BrowserSession>(
            r#"
            UPDATE browser_sessions
            SET current_url = ?2, status = 'active'
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(url)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    /// Close a browser session
    pub async fn close(pool: &SqlitePool, id: Uuid) -> Result<Self, BrowserSessionError> {
        Self::update_status(pool, id, SessionStatus::Closed, None).await
    }
}
