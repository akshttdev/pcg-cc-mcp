use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BrowserActionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Action not found")]
    NotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "action_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Navigate,
    Click,
    Type,
    Scroll,
    Screenshot,
    Wait,
    Select,
    Hover,
    PressKey,
    Evaluate,
    Upload,
    Download,
}

impl std::fmt::Display for ActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionType::Navigate => write!(f, "navigate"),
            ActionType::Click => write!(f, "click"),
            ActionType::Type => write!(f, "type"),
            ActionType::Scroll => write!(f, "scroll"),
            ActionType::Screenshot => write!(f, "screenshot"),
            ActionType::Wait => write!(f, "wait"),
            ActionType::Select => write!(f, "select"),
            ActionType::Hover => write!(f, "hover"),
            ActionType::PressKey => write!(f, "press_key"),
            ActionType::Evaluate => write!(f, "evaluate"),
            ActionType::Upload => write!(f, "upload"),
            ActionType::Download => write!(f, "download"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "action_result", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ActionResult {
    Success,
    Failed,
    Blocked,
    Timeout,
}

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActionResult::Success => write!(f, "success"),
            ActionResult::Failed => write!(f, "failed"),
            ActionResult::Blocked => write!(f, "blocked"),
            ActionResult::Timeout => write!(f, "timeout"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct BrowserAction {
    pub id: Uuid,
    pub browser_session_id: Uuid,
    pub action_type: ActionType,
    pub target_selector: Option<String>,
    #[sqlx(default)]
    pub action_data: Option<String>, // JSON
    pub result: Option<ActionResult>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub screenshot_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateBrowserAction {
    pub browser_session_id: Uuid,
    pub action_type: ActionType,
    pub target_selector: Option<String>,
    pub action_data: Option<Value>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CompleteAction {
    pub result: ActionResult,
    pub error_message: Option<String>,
    pub duration_ms: Option<i32>,
    pub screenshot_id: Option<Uuid>,
}

impl BrowserAction {
    /// Create a new action record (at start of action)
    pub async fn create(
        pool: &SqlitePool,
        data: CreateBrowserAction,
    ) -> Result<Self, BrowserActionError> {
        let id = Uuid::new_v4();
        let action_type_str = data.action_type.to_string();
        let action_data_str = data.action_data.map(|v| v.to_string());

        let action = sqlx::query_as::<_, BrowserAction>(
            r#"
            INSERT INTO browser_actions (id, browser_session_id, action_type, target_selector, action_data)
            VALUES (?1, ?2, ?3, ?4, ?5)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.browser_session_id)
        .bind(action_type_str)
        .bind(&data.target_selector)
        .bind(action_data_str)
        .fetch_one(pool)
        .await?;

        Ok(action)
    }

    /// Complete an action with result
    pub async fn complete(
        pool: &SqlitePool,
        id: Uuid,
        completion: CompleteAction,
    ) -> Result<Self, BrowserActionError> {
        let result_str = completion.result.to_string();

        let action = sqlx::query_as::<_, BrowserAction>(
            r#"
            UPDATE browser_actions
            SET result = ?2, error_message = ?3, duration_ms = ?4, screenshot_id = ?5
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(result_str)
        .bind(&completion.error_message)
        .bind(completion.duration_ms)
        .bind(completion.screenshot_id)
        .fetch_one(pool)
        .await?;

        Ok(action)
    }

    /// Find action by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, BrowserActionError> {
        let action = sqlx::query_as::<_, BrowserAction>(
            r#"SELECT * FROM browser_actions WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(action)
    }

    /// Find all actions for a session
    pub async fn find_by_session(
        pool: &SqlitePool,
        browser_session_id: Uuid,
    ) -> Result<Vec<Self>, BrowserActionError> {
        let actions = sqlx::query_as::<_, BrowserAction>(
            r#"
            SELECT * FROM browser_actions
            WHERE browser_session_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(browser_session_id)
        .fetch_all(pool)
        .await?;

        Ok(actions)
    }

    /// Find failed actions for a session
    pub async fn find_failed(
        pool: &SqlitePool,
        browser_session_id: Uuid,
    ) -> Result<Vec<Self>, BrowserActionError> {
        let actions = sqlx::query_as::<_, BrowserAction>(
            r#"
            SELECT * FROM browser_actions
            WHERE browser_session_id = ?1 AND result IN ('failed', 'blocked', 'timeout')
            ORDER BY created_at ASC
            "#,
        )
        .bind(browser_session_id)
        .fetch_all(pool)
        .await?;

        Ok(actions)
    }

    /// Get action count by type for a session
    pub async fn count_by_type(
        pool: &SqlitePool,
        browser_session_id: Uuid,
    ) -> Result<Vec<(String, i64)>, BrowserActionError> {
        let counts: Vec<(String, i64)> = sqlx::query_as(
            r#"
            SELECT action_type, COUNT(*) as count
            FROM browser_actions
            WHERE browser_session_id = ?1
            GROUP BY action_type
            ORDER BY count DESC
            "#,
        )
        .bind(browser_session_id)
        .fetch_all(pool)
        .await?;

        Ok(counts)
    }

    /// Parse action_data as JSON Value
    pub fn action_data_json(&self) -> Option<Value> {
        self.action_data
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Check if action was successful
    pub fn is_success(&self) -> bool {
        self.result.as_ref().map(|r| *r == ActionResult::Success).unwrap_or(false)
    }

    /// Check if action was blocked by allowlist
    pub fn was_blocked(&self) -> bool {
        self.result.as_ref().map(|r| *r == ActionResult::Blocked).unwrap_or(false)
    }
}
