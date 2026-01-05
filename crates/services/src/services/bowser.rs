//! Bowser Browser Agent Service
//!
//! Provides browser automation capabilities with Playwright integration,
//! screenshot capture, visual diffs, and security-first URL allowlisting.

use db::{
    models::{
        browser_action::{ActionResult, ActionType, BrowserAction, BrowserActionError, CompleteAction, CreateBrowserAction},
        browser_allowlist::{BrowserAllowlist, BrowserAllowlistError, CreateBrowserAllowlist, PatternType},
        browser_screenshot::{AddVisualDiff, BrowserScreenshot, BrowserScreenshotError, CreateBrowserScreenshot},
        browser_session::{BrowserSession, BrowserSessionError, BrowserType, CreateBrowserSession, SessionStatus},
    },
    DBService,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BowserError {
    #[error(transparent)]
    SessionError(#[from] BrowserSessionError),
    #[error(transparent)]
    ScreenshotError(#[from] BrowserScreenshotError),
    #[error(transparent)]
    AllowlistError(#[from] BrowserAllowlistError),
    #[error(transparent)]
    ActionError(#[from] BrowserActionError),
    #[error("URL not allowed: {0}")]
    UrlBlocked(String),
    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),
    #[error("No active session for execution: {0}")]
    NoActiveSession(Uuid),
}

/// Response for session with its screenshots and actions
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct BrowserSessionDetails {
    pub session: BrowserSession,
    pub screenshots: Vec<BrowserScreenshot>,
    pub actions: Vec<BrowserAction>,
    pub action_counts: Vec<(String, i64)>,
}

/// Summary of browser activity for Mission Control
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct BowserSummary {
    pub active_sessions: usize,
    pub total_screenshots: usize,
    pub total_actions: usize,
    pub blocked_actions: usize,
}

/// Navigation request with URL validation
#[derive(Debug, Deserialize, TS)]
pub struct NavigateRequest {
    pub session_id: Uuid,
    pub url: String,
    pub wait_for: Option<String>, // 'load', 'domcontentloaded', 'networkidle'
}

/// Screenshot request with options
#[derive(Debug, Deserialize, TS)]
pub struct ScreenshotRequest {
    pub session_id: Uuid,
    pub full_page: bool,
    pub compare_to_baseline: bool,
}

/// Bowser Browser Agent Service
#[derive(Clone)]
pub struct BowserService {
    db: DBService,
}

impl BowserService {
    pub fn new(db: DBService) -> Self {
        Self { db }
    }

    // ========== Session Management ==========

    /// Start a new browser session
    pub async fn start_session(
        &self,
        execution_process_id: Uuid,
        browser_type: Option<BrowserType>,
        viewport: Option<(i32, i32)>,
        headless: Option<bool>,
    ) -> Result<BrowserSession, BowserError> {
        let (width, height) = viewport.unwrap_or((1280, 720));

        let session = BrowserSession::create(
            &self.db.pool,
            CreateBrowserSession {
                execution_process_id,
                browser_type: browser_type.unwrap_or_default(),
                viewport_width: width,
                viewport_height: height,
                headless: headless.unwrap_or(true),
            },
        )
        .await?;

        Ok(session)
    }

    /// Get session by ID
    pub async fn get_session(&self, id: Uuid) -> Result<BrowserSession, BowserError> {
        BrowserSession::find_by_id(&self.db.pool, id)
            .await?
            .ok_or(BowserError::SessionNotFound(id))
    }

    /// Get session for an execution process
    pub async fn get_session_for_execution(
        &self,
        execution_process_id: Uuid,
    ) -> Result<BrowserSession, BowserError> {
        BrowserSession::find_by_execution_process(&self.db.pool, execution_process_id)
            .await?
            .ok_or(BowserError::NoActiveSession(execution_process_id))
    }

    /// Get session with full details
    pub async fn get_session_details(&self, id: Uuid) -> Result<BrowserSessionDetails, BowserError> {
        let session = self.get_session(id).await?;
        let screenshots = BrowserScreenshot::find_by_session(&self.db.pool, id).await?;
        let actions = BrowserAction::find_by_session(&self.db.pool, id).await?;
        let action_counts = BrowserAction::count_by_type(&self.db.pool, id).await?;

        Ok(BrowserSessionDetails {
            session,
            screenshots,
            actions,
            action_counts,
        })
    }

    /// Get all active browser sessions
    pub async fn get_active_sessions(&self) -> Result<Vec<BrowserSession>, BowserError> {
        let sessions = BrowserSession::find_active(&self.db.pool).await?;
        Ok(sessions)
    }

    /// Close a browser session
    pub async fn close_session(&self, id: Uuid) -> Result<BrowserSession, BowserError> {
        let session = BrowserSession::close(&self.db.pool, id).await?;
        Ok(session)
    }

    /// Mark session as error
    pub async fn mark_session_error(
        &self,
        id: Uuid,
        error_message: String,
    ) -> Result<BrowserSession, BowserError> {
        let session = BrowserSession::update_status(
            &self.db.pool,
            id,
            SessionStatus::Error,
            Some(error_message),
        )
        .await?;
        Ok(session)
    }

    // ========== URL Allowlist ==========

    /// Check if URL is allowed for a project
    pub async fn is_url_allowed(&self, project_id: Uuid, url: &str) -> Result<bool, BowserError> {
        let entries = BrowserAllowlist::find_for_project(&self.db.pool, project_id).await?;
        Ok(BrowserAllowlist::is_url_allowed(&entries, url))
    }

    /// Validate URL and return error if blocked
    pub async fn validate_url(&self, project_id: Uuid, url: &str) -> Result<(), BowserError> {
        if !self.is_url_allowed(project_id, url).await? {
            return Err(BowserError::UrlBlocked(url.to_string()));
        }
        Ok(())
    }

    /// Add URL pattern to allowlist
    pub async fn add_to_allowlist(
        &self,
        project_id: Option<Uuid>,
        pattern: String,
        pattern_type: PatternType,
        description: Option<String>,
        is_global: bool,
        created_by: Option<String>,
    ) -> Result<BrowserAllowlist, BowserError> {
        let entry = BrowserAllowlist::create(
            &self.db.pool,
            CreateBrowserAllowlist {
                project_id,
                pattern,
                pattern_type,
                description,
                is_global,
                created_by,
            },
        )
        .await?;
        Ok(entry)
    }

    /// Get allowlist for a project
    pub async fn get_allowlist(&self, project_id: Uuid) -> Result<Vec<BrowserAllowlist>, BowserError> {
        let entries = BrowserAllowlist::find_for_project(&self.db.pool, project_id).await?;
        Ok(entries)
    }

    /// Remove from allowlist
    pub async fn remove_from_allowlist(&self, id: Uuid) -> Result<(), BowserError> {
        BrowserAllowlist::delete(&self.db.pool, id).await?;
        Ok(())
    }

    // ========== Navigation ==========

    /// Navigate to URL (with allowlist check)
    pub async fn navigate(
        &self,
        session_id: Uuid,
        project_id: Uuid,
        url: &str,
    ) -> Result<BrowserAction, BowserError> {
        // Create action record
        let action = BrowserAction::create(
            &self.db.pool,
            CreateBrowserAction {
                browser_session_id: session_id,
                action_type: ActionType::Navigate,
                target_selector: None,
                action_data: Some(serde_json::json!({ "url": url })),
            },
        )
        .await?;

        // Check allowlist
        if !self.is_url_allowed(project_id, url).await? {
            let _action = BrowserAction::complete(
                &self.db.pool,
                action.id,
                CompleteAction {
                    result: ActionResult::Blocked,
                    error_message: Some(format!("URL not in allowlist: {}", url)),
                    duration_ms: Some(0),
                    screenshot_id: None,
                },
            )
            .await?;
            return Err(BowserError::UrlBlocked(url.to_string()));
        }

        // Update session URL
        BrowserSession::update_url(&self.db.pool, session_id, url.to_string()).await?;

        Ok(action)
    }

    /// Complete a navigation action (called after Playwright navigates)
    pub async fn complete_navigation(
        &self,
        action_id: Uuid,
        success: bool,
        error_message: Option<String>,
        duration_ms: i32,
    ) -> Result<BrowserAction, BowserError> {
        let action = BrowserAction::complete(
            &self.db.pool,
            action_id,
            CompleteAction {
                result: if success { ActionResult::Success } else { ActionResult::Failed },
                error_message,
                duration_ms: Some(duration_ms),
                screenshot_id: None,
            },
        )
        .await?;
        Ok(action)
    }

    // ========== Screenshots ==========

    /// Capture and store a screenshot
    pub async fn capture_screenshot(
        &self,
        session_id: Uuid,
        url: String,
        page_title: Option<String>,
        screenshot_path: String,
        thumbnail_path: Option<String>,
        viewport: (i32, i32),
        full_page: bool,
        metadata: Option<Value>,
    ) -> Result<BrowserScreenshot, BowserError> {
        let screenshot = BrowserScreenshot::create(
            &self.db.pool,
            CreateBrowserScreenshot {
                browser_session_id: session_id,
                url,
                page_title,
                screenshot_path,
                thumbnail_path,
                viewport_width: viewport.0,
                viewport_height: viewport.1,
                full_page,
                metadata,
            },
        )
        .await?;

        // Record screenshot action
        let action = BrowserAction::create(
            &self.db.pool,
            CreateBrowserAction {
                browser_session_id: session_id,
                action_type: ActionType::Screenshot,
                target_selector: None,
                action_data: Some(serde_json::json!({
                    "full_page": full_page,
                    "path": &screenshot.screenshot_path
                })),
            },
        )
        .await?;

        BrowserAction::complete(
            &self.db.pool,
            action.id,
            CompleteAction {
                result: ActionResult::Success,
                error_message: None,
                duration_ms: None,
                screenshot_id: Some(screenshot.id),
            },
        )
        .await?;

        Ok(screenshot)
    }

    /// Get screenshots for a session
    pub async fn get_screenshots(&self, session_id: Uuid) -> Result<Vec<BrowserScreenshot>, BowserError> {
        let screenshots = BrowserScreenshot::find_by_session(&self.db.pool, session_id).await?;
        Ok(screenshots)
    }

    /// Add visual diff to screenshot
    pub async fn add_visual_diff(
        &self,
        screenshot_id: Uuid,
        baseline_id: Uuid,
        diff_path: String,
        diff_percentage: f64,
    ) -> Result<BrowserScreenshot, BowserError> {
        let screenshot = BrowserScreenshot::add_visual_diff(
            &self.db.pool,
            screenshot_id,
            AddVisualDiff {
                baseline_screenshot_id: baseline_id,
                diff_path,
                diff_percentage,
            },
        )
        .await?;
        Ok(screenshot)
    }

    /// Get screenshots with visual diffs
    pub async fn get_screenshots_with_diffs(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<BrowserScreenshot>, BowserError> {
        let screenshots = BrowserScreenshot::find_with_diffs(&self.db.pool, session_id).await?;
        Ok(screenshots)
    }

    // ========== Actions ==========

    /// Record a browser action
    pub async fn record_action(
        &self,
        session_id: Uuid,
        action_type: ActionType,
        target_selector: Option<String>,
        action_data: Option<Value>,
    ) -> Result<BrowserAction, BowserError> {
        let action = BrowserAction::create(
            &self.db.pool,
            CreateBrowserAction {
                browser_session_id: session_id,
                action_type,
                target_selector,
                action_data,
            },
        )
        .await?;
        Ok(action)
    }

    /// Complete a browser action
    pub async fn complete_action(
        &self,
        action_id: Uuid,
        result: ActionResult,
        error_message: Option<String>,
        duration_ms: Option<i32>,
        screenshot_id: Option<Uuid>,
    ) -> Result<BrowserAction, BowserError> {
        let action = BrowserAction::complete(
            &self.db.pool,
            action_id,
            CompleteAction {
                result,
                error_message,
                duration_ms,
                screenshot_id,
            },
        )
        .await?;
        Ok(action)
    }

    /// Get actions for a session
    pub async fn get_actions(&self, session_id: Uuid) -> Result<Vec<BrowserAction>, BowserError> {
        let actions = BrowserAction::find_by_session(&self.db.pool, session_id).await?;
        Ok(actions)
    }

    /// Get failed actions for a session
    pub async fn get_failed_actions(&self, session_id: Uuid) -> Result<Vec<BrowserAction>, BowserError> {
        let actions = BrowserAction::find_failed(&self.db.pool, session_id).await?;
        Ok(actions)
    }

    // ========== Summary ==========

    /// Get Bowser summary for Mission Control
    pub async fn get_summary(&self) -> Result<BowserSummary, BowserError> {
        let active_sessions = BrowserSession::find_active(&self.db.pool).await?;

        let mut total_screenshots = 0;
        let mut total_actions = 0;
        let mut blocked_actions = 0;

        for session in &active_sessions {
            let screenshots = BrowserScreenshot::find_by_session(&self.db.pool, session.id).await?;
            total_screenshots += screenshots.len();

            let actions = BrowserAction::find_by_session(&self.db.pool, session.id).await?;
            total_actions += actions.len();
            blocked_actions += actions.iter().filter(|a| a.was_blocked()).count();
        }

        Ok(BowserSummary {
            active_sessions: active_sessions.len(),
            total_screenshots,
            total_actions,
            blocked_actions,
        })
    }
}
