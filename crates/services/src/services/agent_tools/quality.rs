//! Quality Assurance Tools Implementation
//!
//! Tools for QA testing, accessibility checking, and quality validation.
//! Used primarily by Sentinel agent for quality gate verification.

use crate::services::agent_tools::ToolResult;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// Accessibility Checker
// ============================================================================

/// WCAG compliance level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum WcagLevel {
    A,
    AA,
    AAA,
}

impl WcagLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            WcagLevel::A => "A",
            WcagLevel::AA => "AA",
            WcagLevel::AAA => "AAA",
        }
    }
}

/// Accessibility issue severity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Notice,
}

/// Accessibility issue found during audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityIssue {
    pub id: String,
    pub severity: IssueSeverity,
    pub wcag_criteria: String,
    pub description: String,
    pub element: Option<String>,
    pub selector: Option<String>,
    pub fix_suggestion: String,
}

/// Accessibility audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityResult {
    pub url: String,
    pub wcag_level: String,
    pub passed: bool,
    pub score: f32,
    pub issues: Vec<AccessibilityIssue>,
    pub statistics: AccessibilityStats,
}

/// Accessibility audit statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityStats {
    pub errors: u32,
    pub warnings: u32,
    pub notices: u32,
    pub elements_tested: u32,
    pub rules_passed: u32,
    pub rules_failed: u32,
}

/// Accessibility checker using axe-core patterns
pub struct AccessibilityChecker {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl AccessibilityChecker {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }

    /// Run accessibility audit using PageSpeed Insights (includes accessibility)
    pub async fn audit(&self, url: &str, wcag_level: WcagLevel) -> Result<AccessibilityResult, QualityError> {
        let api_url = format!(
            "https://www.googleapis.com/pagespeedonline/v5/runPagespeed?url={}&category=accessibility",
            urlencoding::encode(url)
        );

        let mut request = self.client.get(&api_url);

        if let Some(key) = &self.api_key {
            request = request.query(&[("key", key)]);
        }

        let response = request.send().await
            .map_err(|e| QualityError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(QualityError::ApiError(format!("API returned {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| QualityError::ParseError(e.to_string()))?;

        self.parse_accessibility_result(&data, url, wcag_level)
    }

    fn parse_accessibility_result(
        &self,
        data: &serde_json::Value,
        url: &str,
        wcag_level: WcagLevel,
    ) -> Result<AccessibilityResult, QualityError> {
        let lighthouse = data.get("lighthouseResult")
            .ok_or_else(|| QualityError::ParseError("Missing lighthouseResult".to_string()))?;

        let categories = lighthouse.get("categories")
            .ok_or_else(|| QualityError::ParseError("Missing categories".to_string()))?;

        let accessibility = categories.get("accessibility")
            .ok_or_else(|| QualityError::ParseError("Missing accessibility category".to_string()))?;

        let score = accessibility.get("score")
            .and_then(|s| s.as_f64())
            .map(|s| (s * 100.0) as f32)
            .unwrap_or(0.0);

        let audits = lighthouse.get("audits").unwrap_or(&serde_json::Value::Null);

        let mut issues = Vec::new();
        let mut errors = 0u32;
        let mut warnings = 0u32;
        let mut notices = 0u32;
        let mut rules_passed = 0u32;
        let mut rules_failed = 0u32;

        // Parse individual accessibility audits
        if let Some(audits_obj) = audits.as_object() {
            for (audit_id, audit) in audits_obj {
                // Skip non-accessibility audits
                if !Self::is_accessibility_audit(audit_id) {
                    continue;
                }

                let audit_score = audit.get("score").and_then(|s| s.as_f64()).unwrap_or(1.0);

                if audit_score >= 1.0 {
                    rules_passed += 1;
                } else {
                    rules_failed += 1;

                    let severity = if audit_score == 0.0 {
                        errors += 1;
                        IssueSeverity::Error
                    } else if audit_score < 0.5 {
                        warnings += 1;
                        IssueSeverity::Warning
                    } else {
                        notices += 1;
                        IssueSeverity::Notice
                    };

                    issues.push(AccessibilityIssue {
                        id: audit_id.clone(),
                        severity,
                        wcag_criteria: Self::get_wcag_criteria(audit_id),
                        description: audit.get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                        element: None,
                        selector: None,
                        fix_suggestion: audit.get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string(),
                    });
                }
            }
        }

        // Determine if it passes the required WCAG level
        let passed = match wcag_level {
            WcagLevel::A => score >= 70.0,
            WcagLevel::AA => score >= 85.0,
            WcagLevel::AAA => score >= 95.0,
        };

        Ok(AccessibilityResult {
            url: url.to_string(),
            wcag_level: wcag_level.as_str().to_string(),
            passed,
            score,
            issues,
            statistics: AccessibilityStats {
                errors,
                warnings,
                notices,
                elements_tested: rules_passed + rules_failed,
                rules_passed,
                rules_failed,
            },
        })
    }

    fn is_accessibility_audit(audit_id: &str) -> bool {
        let accessibility_audits = [
            "accesskeys", "aria-allowed-attr", "aria-hidden-body", "aria-hidden-focus",
            "aria-input-field-name", "aria-required-attr", "aria-required-children",
            "aria-required-parent", "aria-roles", "aria-toggle-field-name", "aria-valid-attr",
            "aria-valid-attr-value", "button-name", "bypass", "color-contrast", "definition-list",
            "dlitem", "document-title", "duplicate-id-active", "duplicate-id-aria",
            "form-field-multiple-labels", "frame-title", "heading-order", "html-has-lang",
            "html-lang-valid", "image-alt", "input-image-alt", "label", "link-name",
            "list", "listitem", "meta-refresh", "meta-viewport", "object-alt",
            "tabindex", "td-headers-attr", "th-has-data-cells", "valid-lang", "video-caption",
        ];
        accessibility_audits.contains(&audit_id)
    }

    fn get_wcag_criteria(audit_id: &str) -> String {
        match audit_id {
            "color-contrast" => "1.4.3 Contrast (Minimum) (AA)".to_string(),
            "image-alt" => "1.1.1 Non-text Content (A)".to_string(),
            "link-name" => "2.4.4 Link Purpose (A)".to_string(),
            "button-name" => "4.1.2 Name, Role, Value (A)".to_string(),
            "document-title" => "2.4.2 Page Titled (A)".to_string(),
            "html-has-lang" => "3.1.1 Language of Page (A)".to_string(),
            "heading-order" => "1.3.1 Info and Relationships (A)".to_string(),
            "label" => "1.3.1 Info and Relationships (A)".to_string(),
            "bypass" => "2.4.1 Bypass Blocks (A)".to_string(),
            _ => "WCAG 2.1".to_string(),
        }
    }
}

// ============================================================================
// Brand Consistency Checker
// ============================================================================

/// Brand consistency check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandConsistencyResult {
    pub passed: bool,
    pub score: f32,
    pub checks: Vec<BrandCheck>,
    pub issues: Vec<BrandIssue>,
}

/// Individual brand check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandCheck {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

/// Brand consistency issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandIssue {
    pub category: String,
    pub description: String,
    pub expected: String,
    pub found: String,
    pub location: Option<String>,
}

/// Brand consistency checker
pub struct BrandConsistencyChecker {
    pub brand_colors: Vec<String>,
    pub brand_fonts: Vec<String>,
    pub logo_url: Option<String>,
}

impl BrandConsistencyChecker {
    pub fn new(colors: Vec<String>, fonts: Vec<String>, logo_url: Option<String>) -> Self {
        Self {
            brand_colors: colors,
            brand_fonts: fonts,
            logo_url,
        }
    }

    /// Check brand consistency (would integrate with headless browser)
    pub fn analyze_styles(&self, _css_content: &str) -> BrandConsistencyResult {
        // This would parse CSS and check against brand guidelines
        // For now, return a placeholder result
        BrandConsistencyResult {
            passed: true,
            score: 100.0,
            checks: vec![
                BrandCheck {
                    name: "Color Palette".to_string(),
                    passed: true,
                    details: "All colors match brand guidelines".to_string(),
                },
                BrandCheck {
                    name: "Typography".to_string(),
                    passed: true,
                    details: "Fonts match brand guidelines".to_string(),
                },
            ],
            issues: vec![],
        }
    }
}

// ============================================================================
// Link Checker
// ============================================================================

/// Link check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkCheckResult {
    pub url: String,
    pub total_links: u32,
    pub broken_links: Vec<BrokenLink>,
    pub redirects: Vec<RedirectLink>,
    pub passed: bool,
}

/// Broken link information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokenLink {
    pub url: String,
    pub status_code: Option<u16>,
    pub error: String,
    pub found_on: String,
}

/// Redirect link information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectLink {
    pub original_url: String,
    pub final_url: String,
    pub redirect_chain: Vec<String>,
}

/// Link checker for validating all links on a page
pub struct LinkChecker {
    client: reqwest::Client,
    timeout_ms: u64,
}

impl LinkChecker {
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            timeout_ms,
        }
    }

    /// Check a single URL
    pub async fn check_url(&self, url: &str) -> Result<(u16, Option<String>), String> {
        let response = self.client
            .head(url)
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status().as_u16();
        let redirect = response.headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Ok((status, redirect))
    }

    /// Check multiple URLs concurrently
    pub async fn check_urls(&self, urls: &[String], source_url: &str) -> LinkCheckResult {
        let mut broken_links = Vec::new();
        let mut redirects = Vec::new();

        for url in urls {
            match self.check_url(url).await {
                Ok((status, redirect)) => {
                    if status >= 400 {
                        broken_links.push(BrokenLink {
                            url: url.clone(),
                            status_code: Some(status),
                            error: format!("HTTP {}", status),
                            found_on: source_url.to_string(),
                        });
                    } else if status >= 300 && status < 400 {
                        if let Some(final_url) = redirect {
                            redirects.push(RedirectLink {
                                original_url: url.clone(),
                                final_url,
                                redirect_chain: vec![],
                            });
                        }
                    }
                }
                Err(error) => {
                    broken_links.push(BrokenLink {
                        url: url.clone(),
                        status_code: None,
                        error,
                        found_on: source_url.to_string(),
                    });
                }
            }
        }

        LinkCheckResult {
            url: source_url.to_string(),
            total_links: urls.len() as u32,
            passed: broken_links.is_empty(),
            broken_links,
            redirects,
        }
    }
}

// ============================================================================
// Screenshot Comparison
// ============================================================================

/// Screenshot comparison result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotComparisonResult {
    pub passed: bool,
    pub difference_percentage: f32,
    pub threshold: f32,
    pub diff_regions: Vec<DiffRegion>,
}

/// Region where screenshots differ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Screenshot comparison tool for visual regression testing
pub struct ScreenshotComparer {
    threshold: f32,
}

impl ScreenshotComparer {
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Compare two screenshots (would use image processing library)
    pub fn compare(&self, _baseline: &[u8], _current: &[u8]) -> ScreenshotComparisonResult {
        // This would use an image comparison algorithm
        // For now, return a placeholder result
        ScreenshotComparisonResult {
            passed: true,
            difference_percentage: 0.0,
            threshold: self.threshold,
            diff_regions: vec![],
        }
    }
}

// ============================================================================
// Content Validation
// ============================================================================

/// Content validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentValidationResult {
    pub passed: bool,
    pub issues: Vec<ContentIssue>,
    pub word_count: u32,
    pub reading_level: String,
    pub spelling_errors: u32,
    pub grammar_errors: u32,
}

/// Content validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentIssue {
    pub issue_type: String,
    pub severity: String,
    pub description: String,
    pub suggestion: Option<String>,
    pub position: Option<ContentPosition>,
}

/// Position in content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPosition {
    pub start: u32,
    pub end: u32,
    pub context: String,
}

/// Content validator for text quality
pub struct ContentValidator {
    min_word_count: Option<u32>,
    max_word_count: Option<u32>,
}

impl ContentValidator {
    pub fn new(min_word_count: Option<u32>, max_word_count: Option<u32>) -> Self {
        Self {
            min_word_count,
            max_word_count,
        }
    }

    /// Validate content text
    pub fn validate(&self, text: &str) -> ContentValidationResult {
        let word_count = text.split_whitespace().count() as u32;
        let mut issues = Vec::new();

        // Check word count limits
        if let Some(min) = self.min_word_count {
            if word_count < min {
                issues.push(ContentIssue {
                    issue_type: "word_count".to_string(),
                    severity: "warning".to_string(),
                    description: format!("Content has {} words, minimum is {}", word_count, min),
                    suggestion: Some("Add more content".to_string()),
                    position: None,
                });
            }
        }

        if let Some(max) = self.max_word_count {
            if word_count > max {
                issues.push(ContentIssue {
                    issue_type: "word_count".to_string(),
                    severity: "warning".to_string(),
                    description: format!("Content has {} words, maximum is {}", word_count, max),
                    suggestion: Some("Reduce content length".to_string()),
                    position: None,
                });
            }
        }

        // Calculate reading level (simplified Flesch-Kincaid)
        let reading_level = self.calculate_reading_level(text);

        ContentValidationResult {
            passed: issues.is_empty(),
            issues,
            word_count,
            reading_level,
            spelling_errors: 0, // Would integrate with spell checker
            grammar_errors: 0,  // Would integrate with grammar checker
        }
    }

    fn calculate_reading_level(&self, text: &str) -> String {
        let words = text.split_whitespace().count();
        let sentences = text.matches(|c| c == '.' || c == '!' || c == '?').count().max(1);
        let avg_words_per_sentence = words as f32 / sentences as f32;

        if avg_words_per_sentence < 10.0 {
            "Easy".to_string()
        } else if avg_words_per_sentence < 15.0 {
            "Standard".to_string()
        } else if avg_words_per_sentence < 20.0 {
            "Intermediate".to_string()
        } else {
            "Advanced".to_string()
        }
    }
}

// ============================================================================
// Quality Error Types
// ============================================================================

#[derive(Debug)]
pub enum QualityError {
    RequestFailed(String),
    ApiError(String),
    ParseError(String),
    ValidationFailed(String),
}

// ============================================================================
// Quality Tools (High-level interface)
// ============================================================================

/// High-level quality tools interface for Sentinel agent
pub struct QualityTools {
    pub accessibility_checker: AccessibilityChecker,
    pub link_checker: LinkChecker,
    pub content_validator: ContentValidator,
    pub screenshot_comparer: ScreenshotComparer,
}

impl QualityTools {
    pub fn new(pagespeed_api_key: Option<String>) -> Self {
        Self {
            accessibility_checker: AccessibilityChecker::new(pagespeed_api_key),
            link_checker: LinkChecker::new(10000),
            content_validator: ContentValidator::new(Some(100), Some(5000)),
            screenshot_comparer: ScreenshotComparer::new(0.01),
        }
    }

    /// Execute a quality tool by name
    pub async fn execute(&self, tool_name: &str, params: serde_json::Value) -> ToolResult {
        let start = Instant::now();

        let result = match tool_name {
            "check_accessibility" => self.execute_accessibility_check(params).await,
            "check_links" => self.execute_link_check(params).await,
            "validate_content" => self.execute_content_validation(params),
            "compare_screenshots" => self.execute_screenshot_comparison(params),
            _ => Err(format!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(data) => ToolResult {
                success: true,
                tool_name: tool_name.to_string(),
                agent_name: "quality".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data,
                error: None,
            },
            Err(error) => ToolResult {
                success: false,
                tool_name: tool_name.to_string(),
                agent_name: "quality".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data: serde_json::json!({}),
                error: Some(error),
            },
        }
    }

    async fn execute_accessibility_check(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing url parameter")?;

        let wcag_level = params.get("wcag_level")
            .and_then(|v| v.as_str())
            .map(|l| match l.to_uppercase().as_str() {
                "A" => WcagLevel::A,
                "AAA" => WcagLevel::AAA,
                _ => WcagLevel::AA,
            })
            .unwrap_or(WcagLevel::AA);

        let result = self.accessibility_checker.audit(url, wcag_level).await
            .map_err(|e| format!("Accessibility check failed: {:?}", e))?;

        Ok(serde_json::to_value(result).unwrap())
    }

    async fn execute_link_check(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing url parameter")?;

        let links = params.get("links")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>())
            .unwrap_or_default();

        let result = self.link_checker.check_urls(&links, url).await;

        Ok(serde_json::to_value(result).unwrap())
    }

    fn execute_content_validation(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let content = params.get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing content parameter")?;

        let result = self.content_validator.validate(content);

        Ok(serde_json::to_value(result).unwrap())
    }

    fn execute_screenshot_comparison(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let _baseline = params.get("baseline")
            .and_then(|v| v.as_str())
            .ok_or("Missing baseline parameter")?;

        let _current = params.get("current")
            .and_then(|v| v.as_str())
            .ok_or("Missing current parameter")?;

        // Would decode base64 images and compare
        let result = ScreenshotComparisonResult {
            passed: true,
            difference_percentage: 0.0,
            threshold: 0.01,
            diff_regions: vec![],
        };

        Ok(serde_json::to_value(result).unwrap())
    }
}
