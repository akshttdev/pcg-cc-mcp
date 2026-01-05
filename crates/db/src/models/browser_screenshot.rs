use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BrowserScreenshotError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Screenshot not found")]
    NotFound,
    #[error("Failed to save screenshot: {0}")]
    SaveFailed(String),
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct BrowserScreenshot {
    pub id: Uuid,
    pub browser_session_id: Uuid,
    pub url: String,
    pub page_title: Option<String>,
    pub screenshot_path: String,
    pub thumbnail_path: Option<String>,
    pub baseline_screenshot_id: Option<Uuid>,
    pub diff_path: Option<String>,
    pub diff_percentage: Option<f64>,
    pub viewport_width: i32,
    pub viewport_height: i32,
    pub full_page: bool,
    #[sqlx(default)]
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateBrowserScreenshot {
    pub browser_session_id: Uuid,
    pub url: String,
    pub page_title: Option<String>,
    pub screenshot_path: String,
    pub thumbnail_path: Option<String>,
    pub viewport_width: i32,
    pub viewport_height: i32,
    #[serde(default)]
    pub full_page: bool,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, TS)]
pub struct AddVisualDiff {
    pub baseline_screenshot_id: Uuid,
    pub diff_path: String,
    pub diff_percentage: f64,
}

impl BrowserScreenshot {
    /// Create a new screenshot record
    pub async fn create(
        pool: &SqlitePool,
        data: CreateBrowserScreenshot,
    ) -> Result<Self, BrowserScreenshotError> {
        let id = Uuid::new_v4();
        let metadata_str = data.metadata.map(|v| v.to_string());

        let screenshot = sqlx::query_as::<_, BrowserScreenshot>(
            r#"
            INSERT INTO browser_screenshots (
                id, browser_session_id, url, page_title, screenshot_path, thumbnail_path,
                viewport_width, viewport_height, full_page, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.browser_session_id)
        .bind(&data.url)
        .bind(&data.page_title)
        .bind(&data.screenshot_path)
        .bind(&data.thumbnail_path)
        .bind(data.viewport_width)
        .bind(data.viewport_height)
        .bind(data.full_page)
        .bind(metadata_str)
        .fetch_one(pool)
        .await?;

        Ok(screenshot)
    }

    /// Find screenshot by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, BrowserScreenshotError> {
        let screenshot = sqlx::query_as::<_, BrowserScreenshot>(
            r#"SELECT * FROM browser_screenshots WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(screenshot)
    }

    /// Find all screenshots for a browser session
    pub async fn find_by_session(
        pool: &SqlitePool,
        browser_session_id: Uuid,
    ) -> Result<Vec<Self>, BrowserScreenshotError> {
        let screenshots = sqlx::query_as::<_, BrowserScreenshot>(
            r#"
            SELECT * FROM browser_screenshots
            WHERE browser_session_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(browser_session_id)
        .fetch_all(pool)
        .await?;

        Ok(screenshots)
    }

    /// Find screenshots by URL (for baseline comparison)
    pub async fn find_by_url(
        pool: &SqlitePool,
        browser_session_id: Uuid,
        url: &str,
    ) -> Result<Vec<Self>, BrowserScreenshotError> {
        let screenshots = sqlx::query_as::<_, BrowserScreenshot>(
            r#"
            SELECT * FROM browser_screenshots
            WHERE browser_session_id = ?1 AND url = ?2
            ORDER BY created_at DESC
            "#,
        )
        .bind(browser_session_id)
        .bind(url)
        .fetch_all(pool)
        .await?;

        Ok(screenshots)
    }

    /// Get latest screenshot for a URL (baseline for diff)
    pub async fn find_latest_for_url(
        pool: &SqlitePool,
        url: &str,
    ) -> Result<Option<Self>, BrowserScreenshotError> {
        let screenshot = sqlx::query_as::<_, BrowserScreenshot>(
            r#"
            SELECT * FROM browser_screenshots
            WHERE url = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(url)
        .fetch_optional(pool)
        .await?;

        Ok(screenshot)
    }

    /// Add visual diff data to a screenshot
    pub async fn add_visual_diff(
        pool: &SqlitePool,
        id: Uuid,
        diff: AddVisualDiff,
    ) -> Result<Self, BrowserScreenshotError> {
        let screenshot = sqlx::query_as::<_, BrowserScreenshot>(
            r#"
            UPDATE browser_screenshots
            SET baseline_screenshot_id = ?2, diff_path = ?3, diff_percentage = ?4
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(diff.baseline_screenshot_id)
        .bind(&diff.diff_path)
        .bind(diff.diff_percentage)
        .fetch_one(pool)
        .await?;

        Ok(screenshot)
    }

    /// Get screenshots with visual diffs
    pub async fn find_with_diffs(
        pool: &SqlitePool,
        browser_session_id: Uuid,
    ) -> Result<Vec<Self>, BrowserScreenshotError> {
        let screenshots = sqlx::query_as::<_, BrowserScreenshot>(
            r#"
            SELECT * FROM browser_screenshots
            WHERE browser_session_id = ?1 AND diff_percentage IS NOT NULL
            ORDER BY created_at DESC
            "#,
        )
        .bind(browser_session_id)
        .fetch_all(pool)
        .await?;

        Ok(screenshots)
    }

    /// Parse metadata as JSON Value
    pub fn metadata_json(&self) -> Option<Value> {
        self.metadata
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Check if diff exceeds threshold
    pub fn diff_exceeds_threshold(&self, threshold: f64) -> bool {
        self.diff_percentage.map(|p| p > threshold).unwrap_or(false)
    }
}
