//! Bowser Bridge - Playwright Integration for JavaScript-rendered pages
//!
//! Provides a simplified interface to the BowserService for rendering
//! JavaScript-heavy conference websites.
//!
//! ## Implementation
//!
//! The bridge uses a Node.js script (`scripts/render-page.js`) with Playwright
//! to render JavaScript pages. This is simpler and more reliable than maintaining
//! a persistent browser connection.
//!
//! ## Requirements
//!
//! - Node.js 18+
//! - Playwright: `pnpm add -D playwright && npx playwright install chromium`

use services::services::bowser::BowserService;
use sqlx::SqlitePool;
use uuid::Uuid;
use db::DBService;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// A page rendered via Playwright/Bowser
#[derive(Debug, Clone)]
pub struct RenderedPage {
    /// Final URL after any redirects
    pub url: String,
    /// Rendered HTML content
    pub html: String,
    /// Extracted text content
    pub text: String,
    /// Page title
    pub title: Option<String>,
    /// Whether rendering was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// JSON output from the render-page.js script
#[derive(Debug, Deserialize)]
struct RenderScriptOutput {
    success: bool,
    url: String,
    html: String,
    title: Option<String>,
    error: Option<String>,
}

/// Bridge to Bowser service for JavaScript rendering
pub struct BowserBridge {
    bowser: Option<BowserService>,
    pool: SqlitePool,
    /// Path to the render-page.js script
    script_path: Option<String>,
}

impl BowserBridge {
    /// Create a new BowserBridge
    pub async fn new(pool: SqlitePool) -> Self {
        // Try to create BowserService - may fail if DB service not configured
        let bowser = match DBService::new().await {
            Ok(db_service) => Some(BowserService::new(db_service)),
            Err(e) => {
                tracing::warn!("[BOWSER_BRIDGE] Failed to create DBService: {}", e);
                None
            }
        };

        // Find the render-page.js script
        let script_path = Self::find_render_script();

        Self { bowser, pool, script_path }
    }

    /// Create a new BowserBridge with an existing DBService
    pub fn with_db_service(db_service: DBService, pool: SqlitePool) -> Self {
        Self {
            bowser: Some(BowserService::new(db_service)),
            pool,
            script_path: Self::find_render_script(),
        }
    }

    /// Find the render-page.js script in common locations
    fn find_render_script() -> Option<String> {
        let possible_paths = [
            "scripts/render-page.js",
            "./scripts/render-page.js",
            "../scripts/render-page.js",
            "../../scripts/render-page.js",
        ];

        for path in possible_paths {
            if std::path::Path::new(path).exists() {
                tracing::info!("[BOWSER_BRIDGE] Found render script at: {}", path);
                return Some(path.to_string());
            }
        }

        // Try from CARGO_MANIFEST_DIR if available
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let cargo_path = format!("{}/../../../scripts/render-page.js", manifest_dir);
            if std::path::Path::new(&cargo_path).exists() {
                tracing::info!("[BOWSER_BRIDGE] Found render script at: {}", cargo_path);
                return Some(cargo_path);
            }
        }

        tracing::warn!("[BOWSER_BRIDGE] render-page.js script not found");
        None
    }

    /// Check if Playwright/Bowser is available (can render JavaScript pages)
    pub async fn is_available(&self) -> bool {
        self.script_path.is_some() && Self::is_playwright_installed()
    }

    /// Check if Playwright is installed by trying to import it
    fn is_playwright_installed() -> bool {
        let output = Command::new("node")
            .args(["-e", "require('playwright')"])
            .output();

        matches!(output, Ok(o) if o.status.success())
    }

    /// Render a JavaScript page and return the final HTML
    ///
    /// This uses the Node.js script with Playwright to render the page.
    /// Falls back to an error if Playwright isn't installed.
    pub async fn render_page(&self, url: &str, _execution_id: Uuid) -> Result<RenderedPage, String> {
        let script_path = self.script_path.as_ref()
            .ok_or("Render script not found. Ensure scripts/render-page.js exists.")?;

        tracing::info!("[BOWSER_BRIDGE] Rendering JavaScript page: {}", url);

        // Call the Node.js script via subprocess
        let timeout_ms = 30000;
        let output = tokio::task::spawn_blocking({
            let script_path = script_path.clone();
            let url = url.to_string();
            move || {
                Command::new("node")
                    .args([&script_path, &url, &timeout_ms.to_string()])
                    .output()
            }
        })
        .await
        .map_err(|e| format!("Failed to spawn render task: {}", e))?
        .map_err(|e| format!("Failed to execute render script: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check if Playwright is not installed
            if stderr.contains("Cannot find package 'playwright'") || stderr.contains("Cannot find module 'playwright'") {
                return Err(
                    "Playwright not installed. Run: pnpm add -D playwright && npx playwright install chromium".to_string()
                );
            }

            // Try to parse the JSON error output
            if let Ok(result) = serde_json::from_slice::<RenderScriptOutput>(&output.stdout) {
                return Err(result.error.unwrap_or_else(|| "Unknown render error".to_string()));
            }

            return Err(format!("Render script failed: {}", stderr));
        }

        // Parse the JSON output
        let result: RenderScriptOutput = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("Failed to parse render output: {}", e))?;

        if !result.success {
            return Err(result.error.unwrap_or_else(|| "Render failed".to_string()));
        }

        tracing::info!("[BOWSER_BRIDGE] Successfully rendered: {} -> {} ({} chars)",
            url, result.url, result.html.len());

        // Convert HTML to text
        let text = html_to_text(&result.html);

        Ok(RenderedPage {
            url: result.url,
            html: result.html,
            text,
            title: result.title,
            success: true,
            error: None,
        })
    }

    /// Render page with retry on failure
    pub async fn render_page_with_retry(
        &self,
        url: &str,
        execution_id: Uuid,
        max_retries: u32,
    ) -> Result<RenderedPage, String> {
        let mut last_error = String::new();

        for attempt in 0..=max_retries {
            if attempt > 0 {
                tracing::info!("[BOWSER_BRIDGE] Retry {} for {}", attempt, url);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }

            match self.render_page(url, execution_id).await {
                Ok(page) => return Ok(page),
                Err(e) => {
                    last_error = e;
                    tracing::warn!("[BOWSER_BRIDGE] Attempt {} failed: {}", attempt + 1, last_error);
                }
            }
        }

        Err(format!("All {} attempts failed. Last error: {}", max_retries + 1, last_error))
    }
}

/// Extract text content from rendered HTML
fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script blocks
    if let Ok(re) = regex::Regex::new(r"(?i)<script[^>]*>[\s\S]*?</script>") {
        text = re.replace_all(&text, "").to_string();
    }

    // Remove style blocks
    if let Ok(re) = regex::Regex::new(r"(?i)<style[^>]*>[\s\S]*?</style>") {
        text = re.replace_all(&text, "").to_string();
    }

    // Replace common tags with whitespace
    text = text
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n\n")
        .replace("</div>", "\n")
        .replace("</li>", "\n");

    // Remove all remaining HTML tags
    if let Ok(re) = regex::Regex::new(r"<[^>]+>") {
        text = re.replace_all(&text, "").to_string();
    }

    // Decode HTML entities
    text = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    // Clean up whitespace
    if let Ok(re) = regex::Regex::new(r"\s+") {
        text = re.replace_all(&text, " ").to_string();
    }

    text.trim().to_string()
}

/// Extract title from HTML
fn extract_title(html: &str) -> Option<String> {
    if let Ok(re) = regex::Regex::new(r"<title[^>]*>([^<]+)</title>") {
        if let Some(cap) = re.captures(html) {
            if let Some(title) = cap.get(1) {
                return Some(title.as_str().trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_to_text() {
        let html = "<p>Hello <b>world</b></p><script>evil()</script>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert!(!text.contains("script"));
    }

    #[test]
    fn test_extract_title() {
        let html = "<html><head><title>Conference 2026</title></head></html>";
        assert_eq!(extract_title(html), Some("Conference 2026".to_string()));
    }
}
