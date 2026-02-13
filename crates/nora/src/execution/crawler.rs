//! Deep Website Crawler
//!
//! Crawls conference websites using BFS with link discovery, handles pagination,
//! and supports JavaScript-rendered pages via Bowser (Playwright).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::{HashSet, VecDeque};
use std::time::Duration;
use url::Url;
use uuid::Uuid;

use super::bowser_bridge::BowserBridge;

/// Configuration for website crawling
#[derive(Debug, Clone)]
pub struct CrawlConfig {
    /// Maximum number of pages to crawl (default: 50)
    pub max_pages: usize,
    /// Maximum depth from the starting URL (default: 3)
    pub max_depth: usize,
    /// Whether to use Bowser (Playwright) for JavaScript rendering (default: false)
    pub use_bowser: bool,
    /// Page fetch timeout in seconds (default: 15)
    pub page_timeout_secs: u64,
    /// URL patterns to include (e.g., "speaker", "sponsor", "agenda")
    pub include_patterns: Vec<String>,
    /// URL patterns to exclude (e.g., "blog", "news", "pdf")
    pub exclude_patterns: Vec<String>,
    /// Whether to respect robots.txt (default: true)
    pub respect_robots: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        Self {
            max_pages: 200,  // Increased to handle sites with pagination (e.g., 9 pages of speakers)
            max_depth: 4,    // Increased to follow pagination links
            use_bowser: false,
            page_timeout_secs: 15,
            include_patterns: vec![
                "speaker".to_string(),
                "sponsor".to_string(),
                "partner".to_string(),
                "agenda".to_string(),
                "schedule".to_string(),
                "program".to_string(),
                "exhibitor".to_string(),
                "team".to_string(),
                "about".to_string(),
                "page".to_string(),  // Pagination URLs like /speakers/page/2/
            ],
            exclude_patterns: vec![
                "blog".to_string(),
                "news".to_string(),
                ".pdf".to_string(),
                ".jpg".to_string(),
                ".png".to_string(),
                ".gif".to_string(),
                ".svg".to_string(),
                ".css".to_string(),
                ".js".to_string(),
                ".woff".to_string(),
                ".ttf".to_string(),
                "mailto:".to_string(),
                "tel:".to_string(),
                "javascript:".to_string(),
                "#".to_string(),
                "/feed".to_string(),
                "wp-json".to_string(),
                "wp-content".to_string(),
                "wp-includes".to_string(),
                "oembed".to_string(),
                "twitter.com".to_string(),
                "facebook.com".to_string(),
                "linkedin.com".to_string(),
                "instagram.com".to_string(),
                "youtube.com".to_string(),
                "x.com".to_string(),
            ],
            respect_robots: true,
        }
    }
}

/// Type of page detected during crawling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PageType {
    Homepage,
    Speakers,
    SpeakerProfile,
    Sponsors,
    Schedule,
    About,
    Exhibitors,
    Team,
    Other,
}

impl PageType {
    /// Detect page type from URL path and content
    pub fn detect(url: &str, title: Option<&str>) -> Self {
        let url_lower = url.to_lowercase();
        let title_lower = title.map(|t| t.to_lowercase()).unwrap_or_default();

        // Check URL patterns for speaker profile pages
        // Handles both /speaker/name-slug/ and /speakers/name-slug/ patterns
        if let Ok(parsed) = Url::parse(url) {
            let path = parsed.path().to_lowercase();

            // Check for individual speaker profile: /speakers/name-slug/ or /speaker/name-slug/
            // But NOT pagination like /speakers/page/2/
            if let Ok(re) = regex::Regex::new(r"/speakers?/([a-z][-a-z0-9]+)/?$") {
                if re.is_match(&path) {
                    // Make sure it's not a pagination URL
                    if !path.contains("/page/") {
                        return Self::SpeakerProfile;
                    }
                }
            }
        }

        // Speaker listing pages
        if url_lower.contains("/speakers") || url_lower.contains("/lineup") {
            return Self::Speakers;
        }
        if url_lower.contains("/sponsor") || url_lower.contains("/partner") {
            return Self::Sponsors;
        }
        if url_lower.contains("/schedule") || url_lower.contains("/agenda") || url_lower.contains("/program") {
            return Self::Schedule;
        }
        if url_lower.contains("/about") || url_lower.contains("/info") {
            return Self::About;
        }
        if url_lower.contains("/exhibitor") {
            return Self::Exhibitors;
        }
        if url_lower.contains("/team") || url_lower.contains("/organizer") {
            return Self::Team;
        }

        // Check title patterns
        if title_lower.contains("speaker") {
            return Self::Speakers;
        }
        if title_lower.contains("sponsor") || title_lower.contains("partner") {
            return Self::Sponsors;
        }

        // Check if it's the homepage
        if let Ok(parsed) = Url::parse(url) {
            if parsed.path() == "/" || parsed.path().is_empty() {
                return Self::Homepage;
            }
        }

        Self::Other
    }
}

/// How a page was fetched
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FetchMethod {
    StaticHttp,
    Bowser,
}

/// A crawled page with its content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawledPage {
    /// URL of the page
    pub url: String,
    /// Raw HTML content
    pub html: String,
    /// Extracted text content (HTML stripped)
    pub text: String,
    /// Page title from <title> tag
    pub title: Option<String>,
    /// Depth from the starting URL
    pub depth: usize,
    /// Detected page type
    pub page_type: PageType,
    /// Links discovered on this page
    pub discovered_links: Vec<String>,
    /// How this page was fetched
    pub fetched_via: FetchMethod,
    /// HTTP status code
    pub status_code: u16,
}

/// Deep website crawler with BFS link discovery
pub struct WebsiteCrawler {
    http_client: Client,
    pool: SqlitePool,
    bowser_bridge: Option<BowserBridge>,
}

impl WebsiteCrawler {
    pub fn new(pool: SqlitePool) -> Self {
        let http_client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; Scout/1.0; Research Agent)")
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_default();

        Self {
            http_client,
            pool,
            bowser_bridge: None,
        }
    }

    /// Create crawler with BowserBridge for JavaScript rendering
    pub async fn with_bowser(pool: SqlitePool) -> Self {
        let http_client = Client::builder()
            .user_agent("Mozilla/5.0 (compatible; Scout/1.0; Research Agent)")
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::limited(5))
            .build()
            .unwrap_or_default();

        let bridge = BowserBridge::new(pool.clone()).await;
        let has_playwright = bridge.is_available().await;

        if has_playwright {
            tracing::info!("[CRAWLER] BowserBridge with Playwright available for JS rendering");
        } else {
            tracing::info!("[CRAWLER] Playwright not available, will use static HTTP only");
        }

        Self {
            http_client,
            pool,
            bowser_bridge: if has_playwright { Some(bridge) } else { None },
        }
    }

    /// Crawl a website starting from base_url using BFS
    pub async fn crawl(&self, base_url: &str, config: &CrawlConfig) -> Result<Vec<CrawledPage>, String> {
        tracing::info!("[CRAWLER] Starting crawl of {} (max_pages={}, max_depth={})",
            base_url, config.max_pages, config.max_depth);

        let base = Url::parse(base_url).map_err(|e| format!("Invalid base URL: {}", e))?;
        let base_host = base.host_str().ok_or("URL has no host")?;
        // Normalize host for comparison (remove www. prefix)
        let base_host_normalized = base_host.strip_prefix("www.").unwrap_or(base_host);

        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        let mut pages: Vec<CrawledPage> = Vec::new();

        // Start with the base URL
        queue.push_back((base_url.to_string(), 0));

        while let Some((url, depth)) = queue.pop_front() {
            // Stop if we've reached max pages
            if pages.len() >= config.max_pages {
                tracing::info!("[CRAWLER] Reached max_pages limit ({})", config.max_pages);
                break;
            }

            // Stop if we've exceeded max depth
            if depth > config.max_depth {
                continue;
            }

            // Normalize URL for visited check
            let normalized = self.normalize_url(&url);
            if visited.contains(&normalized) {
                continue;
            }
            visited.insert(normalized.clone());

            // Check if URL should be excluded
            if self.should_exclude(&url, config) {
                tracing::debug!("[CRAWLER] Skipping excluded URL: {}", url);
                continue;
            }

            // Fetch the page
            match self.fetch_page(&url, config.use_bowser, config.page_timeout_secs).await {
                Ok(mut page) => {
                    page.depth = depth;

                    tracing::debug!("[CRAWLER] Fetched {} ({:?}, {} links)",
                        url, page.page_type, page.discovered_links.len());

                    // Extract links and queue them for crawling
                    for link in &page.discovered_links {
                        // Only follow links on the same host (normalized)
                        if let Ok(link_url) = Url::parse(link) {
                            let link_host = link_url.host_str().unwrap_or("");
                            let link_host_normalized = link_host.strip_prefix("www.").unwrap_or(link_host);

                            if link_host_normalized == base_host_normalized {
                                let link_normalized = self.normalize_url(link);
                                if !visited.contains(&link_normalized) {
                                    // Prioritize pages matching include patterns
                                    if self.matches_include_pattern(link, config) {
                                        queue.push_front((link.clone(), depth + 1));
                                    } else {
                                        queue.push_back((link.clone(), depth + 1));
                                    }
                                }
                            }
                        }
                    }

                    pages.push(page);
                }
                Err(e) => {
                    tracing::warn!("[CRAWLER] Failed to fetch {}: {}", url, e);
                }
            }
        }

        // Log summary by page type
        let speakers_count = pages.iter().filter(|p| matches!(p.page_type, PageType::Speakers | PageType::SpeakerProfile)).count();
        let sponsors_count = pages.iter().filter(|p| p.page_type == PageType::Sponsors).count();

        tracing::info!(
            "[CRAWLER] Crawl complete: {} pages total ({} speaker pages, {} sponsor pages)",
            pages.len(), speakers_count, sponsors_count
        );

        Ok(pages)
    }

    /// Fetch a single page (static HTTP first, Bowser fallback if enabled)
    async fn fetch_page(&self, url: &str, use_bowser: bool, timeout_secs: u64) -> Result<CrawledPage, String> {
        // First try static HTTP fetch
        match self.fetch_static(url, timeout_secs).await {
            Ok(page) => {
                // Check if page appears to need JavaScript
                if use_bowser && self.needs_javascript(&page.html) {
                    tracing::info!("[CRAWLER] Page {} may need JavaScript, trying Bowser", url);

                    if let Some(ref bridge) = self.bowser_bridge {
                        match bridge.render_page(url, Uuid::new_v4()).await {
                            Ok(rendered) => {
                                tracing::info!("[CRAWLER] Successfully rendered JS page: {}", url);

                                // Convert rendered page to CrawledPage
                                let title = rendered.title.clone();
                                let page_type = PageType::detect(url, title.as_deref());
                                let discovered_links = self.extract_links(&rendered.html, url);

                                return Ok(CrawledPage {
                                    url: rendered.url,
                                    html: rendered.html,
                                    text: rendered.text,
                                    title,
                                    depth: 0,
                                    page_type,
                                    discovered_links,
                                    fetched_via: FetchMethod::Bowser,
                                    status_code: 200,
                                });
                            }
                            Err(e) => {
                                tracing::warn!("[CRAWLER] Bowser render failed for {}: {}. Using static content.", url, e);
                            }
                        }
                    } else {
                        tracing::debug!("[CRAWLER] Bowser not available, using static content for {}", url);
                    }
                }

                Ok(page)
            }
            Err(e) => {
                // If static fetch failed and Bowser is available, try rendering
                if use_bowser {
                    if let Some(ref bridge) = self.bowser_bridge {
                        tracing::info!("[CRAWLER] Static fetch failed for {}, trying Bowser: {}", url, e);

                        match bridge.render_page(url, Uuid::new_v4()).await {
                            Ok(rendered) => {
                                let title = rendered.title.clone();
                                let page_type = PageType::detect(url, title.as_deref());
                                let discovered_links = self.extract_links(&rendered.html, url);

                                return Ok(CrawledPage {
                                    url: rendered.url,
                                    html: rendered.html,
                                    text: rendered.text,
                                    title,
                                    depth: 0,
                                    page_type,
                                    discovered_links,
                                    fetched_via: FetchMethod::Bowser,
                                    status_code: 200,
                                });
                            }
                            Err(bowser_err) => {
                                tracing::warn!("[CRAWLER] Both static and Bowser failed for {}: {}", url, bowser_err);
                            }
                        }
                    }
                }

                Err(e)
            }
        }
    }

    /// Fetch page using static HTTP
    async fn fetch_static(&self, url: &str, timeout_secs: u64) -> Result<CrawledPage, String> {
        let response = self.http_client
            .get(url)
            .timeout(Duration::from_secs(timeout_secs))
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let status_code = response.status().as_u16();

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let html = response.text().await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Extract title
        let title = self.extract_title(&html);

        // Convert HTML to text
        let text = html_to_text(&html);

        // Detect page type
        let page_type = PageType::detect(url, title.as_deref());

        // Extract links
        let discovered_links = self.extract_links(&html, url);

        Ok(CrawledPage {
            url: url.to_string(),
            html,
            text,
            title,
            depth: 0, // Will be set by caller
            page_type,
            discovered_links,
            fetched_via: FetchMethod::StaticHttp,
            status_code,
        })
    }

    /// Extract title from HTML
    fn extract_title(&self, html: &str) -> Option<String> {
        // Simple regex-based extraction
        if let Ok(re) = regex::Regex::new(r"<title[^>]*>([^<]+)</title>") {
            if let Some(cap) = re.captures(html) {
                if let Some(title) = cap.get(1) {
                    return Some(html_entity_decode(title.as_str().trim()));
                }
            }
        }
        None
    }

    /// Extract all links from HTML
    fn extract_links(&self, html: &str, base_url: &str) -> Vec<String> {
        let mut links = Vec::new();

        // Extract href attributes
        if let Ok(re) = regex::Regex::new(r#"href=["']([^"']+)["']"#) {
            let base = Url::parse(base_url).ok();

            for cap in re.captures_iter(html) {
                if let Some(href) = cap.get(1) {
                    let href = href.as_str();

                    // Skip empty, anchor-only, or javascript links
                    if href.is_empty() || href.starts_with('#') || href.starts_with("javascript:") {
                        continue;
                    }

                    // Resolve relative URLs
                    let absolute_url = if href.starts_with("http://") || href.starts_with("https://") {
                        href.to_string()
                    } else if href.starts_with("//") {
                        format!("https:{}", href)
                    } else if let Some(ref base) = base {
                        base.join(href).map(|u| u.to_string()).unwrap_or_default()
                    } else {
                        continue;
                    };

                    if !absolute_url.is_empty() && !links.contains(&absolute_url) {
                        links.push(absolute_url);
                    }
                }
            }
        }

        links
    }

    /// Normalize URL for deduplication
    fn normalize_url(&self, url: &str) -> String {
        if let Ok(mut parsed) = Url::parse(url) {
            // Remove fragment
            parsed.set_fragment(None);
            // Remove trailing slash from path (except root)
            let path = parsed.path().to_string();
            if path.len() > 1 && path.ends_with('/') {
                parsed.set_path(&path[..path.len()-1]);
            }
            parsed.to_string()
        } else {
            url.to_lowercase()
        }
    }

    /// Check if URL should be excluded based on config patterns
    fn should_exclude(&self, url: &str, config: &CrawlConfig) -> bool {
        let url_lower = url.to_lowercase();

        for pattern in &config.exclude_patterns {
            if url_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }

        false
    }

    /// Check if URL matches include patterns (for prioritization)
    fn matches_include_pattern(&self, url: &str, config: &CrawlConfig) -> bool {
        let url_lower = url.to_lowercase();

        for pattern in &config.include_patterns {
            if url_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }

        false
    }

    /// Check if page content suggests JavaScript is needed
    fn needs_javascript(&self, html: &str) -> bool {
        // Indicators that a page might need JavaScript to render content
        let js_indicators = [
            "data-react-",
            "data-vue-",
            "__NEXT_DATA__",
            "__NUXT__",
            "ng-app",
            "window.__INITIAL_STATE__",
            "Loading...",
            "<noscript>",
        ];

        let html_lower = html.to_lowercase();

        // Check for minimal content (likely JS-rendered)
        let text = html_to_text(html);
        if text.len() < 500 && html.len() > 10000 {
            return true;
        }

        for indicator in js_indicators {
            if html.contains(indicator) || html_lower.contains(&indicator.to_lowercase()) {
                return true;
            }
        }

        false
    }
}

/// Simple HTML to text conversion
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

    // Replace common tags with appropriate text
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
    text = html_entity_decode(&text);

    // Clean up whitespace
    if let Ok(re) = regex::Regex::new(r"\s+") {
        text = re.replace_all(&text, " ").to_string();
    }

    text.trim().to_string()
}

/// Decode common HTML entities
fn html_entity_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&#x2F;", "/")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_type_detection() {
        assert_eq!(PageType::detect("https://conf.com/speakers", None), PageType::Speakers);
        assert_eq!(PageType::detect("https://conf.com/speaker/john-doe", None), PageType::SpeakerProfile);
        assert_eq!(PageType::detect("https://conf.com/sponsors", None), PageType::Sponsors);
        assert_eq!(PageType::detect("https://conf.com/schedule", None), PageType::Schedule);
        assert_eq!(PageType::detect("https://conf.com/", None), PageType::Homepage);
    }

    #[test]
    fn test_html_entity_decode() {
        assert_eq!(html_entity_decode("Hello &amp; World"), "Hello & World");
        assert_eq!(html_entity_decode("&lt;tag&gt;"), "<tag>");
    }

    /// Integration test - crawl a real conference website
    /// Run with: cargo test -p nora test_crawl_real_website -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn test_crawl_real_website() {
        // Use a simple test database
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        let crawler = WebsiteCrawler::new(pool);
        let config = CrawlConfig {
            max_pages: 10,
            max_depth: 2,
            ..Default::default()
        };

        // Test with ETHDenver (a real conference website)
        let result = crawler.crawl("https://www.ethdenver.com", &config).await;

        match result {
            Ok(pages) => {
                println!("\n=== CRAWL RESULTS ===");
                println!("Total pages crawled: {}", pages.len());

                for page in &pages {
                    println!(
                        "\n[{:?}] {} (depth={}, links={})",
                        page.page_type,
                        page.url,
                        page.depth,
                        page.discovered_links.len()
                    );
                    if let Some(ref title) = page.title {
                        println!("  Title: {}", title);
                    }
                    println!("  Text length: {} chars", page.text.len());

                    // Show some discovered links (first 10 same-host links)
                    let same_host_links: Vec<_> = page.discovered_links
                        .iter()
                        .filter(|l| l.contains("ethdenver.com"))
                        .take(10)
                        .collect();
                    if !same_host_links.is_empty() {
                        println!("  Same-host links (first 10):");
                        for link in same_host_links {
                            println!("    - {}", link);
                        }
                    }
                }

                // Basic assertions
                assert!(!pages.is_empty(), "Should have crawled at least one page");

                // Check for homepage
                let has_homepage = pages.iter().any(|p| p.page_type == PageType::Homepage);
                println!("\nHas homepage: {}", has_homepage);
            }
            Err(e) => {
                println!("Crawl error: {}", e);
                // Don't fail the test if network is unavailable
            }
        }
    }

    /// Full extraction test - crawl and extract speakers/sponsors
    /// Run with: cargo test -p nora test_full_extraction -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn test_full_extraction() {
        use super::super::research::ResearchTools;

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        let crawler = WebsiteCrawler::new(pool);
        let config = CrawlConfig {
            max_pages: 100,  // Crawl more pages to get full data
            max_depth: 4,    // Include pagination
            ..Default::default()
        };

        println!("\n=== FULL EXTRACTION TEST ===");
        println!("Starting crawl with max_pages={}, max_depth={}...", config.max_pages, config.max_depth);

        let result = crawler.crawl("https://www.ethdenver.com", &config).await;

        match result {
            Ok(pages) => {
                println!("\n=== CRAWL SUMMARY ===");
                println!("Total pages crawled: {}", pages.len());

                // Count by page type
                let speakers_pages = pages.iter().filter(|p| matches!(p.page_type, PageType::Speakers)).count();
                let speaker_profiles = pages.iter().filter(|p| p.page_type == PageType::SpeakerProfile).count();
                let sponsor_pages = pages.iter().filter(|p| p.page_type == PageType::Sponsors).count();
                let homepage_count = pages.iter().filter(|p| p.page_type == PageType::Homepage).count();
                let other_count = pages.iter().filter(|p| p.page_type == PageType::Other).count();

                println!("  - Homepage: {}", homepage_count);
                println!("  - Speakers list pages: {}", speakers_pages);
                println!("  - Speaker profiles: {}", speaker_profiles);
                println!("  - Sponsor pages: {}", sponsor_pages);
                println!("  - Other: {}", other_count);

                // Run extraction
                let research_tools = ResearchTools::new();
                let speakers = research_tools.extract_speakers_comprehensive(&pages);
                let sponsors = research_tools.extract_sponsors_comprehensive(&pages);

                println!("\n=== EXTRACTION RESULTS ===");
                println!("Unique speakers extracted: {}", speakers.len());
                println!("Unique sponsors extracted: {}", sponsors.len());

                // List first 20 speakers
                println!("\nFirst 20 speakers:");
                for (i, speaker) in speakers.iter().take(20).enumerate() {
                    println!(
                        "  {}. {} | {} | {}",
                        i + 1,
                        speaker.name,
                        speaker.title.as_deref().unwrap_or("-"),
                        speaker.company.as_deref().unwrap_or("-")
                    );
                }
                if speakers.len() > 20 {
                    println!("  ... and {} more", speakers.len() - 20);
                }

                // List first 15 sponsors
                println!("\nFirst 15 sponsors:");
                for (i, sponsor) in sponsors.iter().take(15).enumerate() {
                    println!(
                        "  {}. {} | tier: {} | website: {}",
                        i + 1,
                        sponsor.name,
                        sponsor.tier.as_deref().unwrap_or("-"),
                        sponsor.website.as_deref().unwrap_or("-")
                    );
                }
                if sponsors.len() > 15 {
                    println!("  ... and {} more", sponsors.len() - 15);
                }
            }
            Err(e) => {
                println!("Crawl error: {}", e);
            }
        }
    }
}
