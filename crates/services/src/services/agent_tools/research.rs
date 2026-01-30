//! Research Tools Implementation
//!
//! Tools for web search, competitor analysis, and market research.
//! Used primarily by Astra and Scout agents.

use crate::services::agent_tools::{ToolResult, ServiceCredentials};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// Web Search Provider
// ============================================================================

/// Supported search providers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchProvider {
    Tavily,
    Serper,
    Brave,
    SerpApi,
}

/// Web search configuration
#[derive(Debug, Clone)]
pub struct WebSearchConfig {
    pub provider: SearchProvider,
    pub api_key: String,
    pub default_num_results: u32,
    pub include_images: bool,
    pub safe_search: bool,
}

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub published_date: Option<String>,
    pub source: Option<String>,
}

/// Web search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub total_results: Option<u64>,
    pub search_time_ms: u64,
}

/// Web search provider abstraction
#[derive(Clone)]
pub struct WebSearchProvider {
    config: WebSearchConfig,
    client: Client,
}

impl WebSearchProvider {
    pub fn new(config: WebSearchConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// Create from credentials
    pub fn from_credentials(provider: SearchProvider, credentials: &ServiceCredentials) -> Option<Self> {
        let api_key = credentials.api_key.clone()?;
        Some(Self::new(WebSearchConfig {
            provider,
            api_key,
            default_num_results: 10,
            include_images: false,
            safe_search: true,
        }))
    }

    /// Perform a web search
    pub async fn search(&self, query: &str, num_results: Option<u32>) -> Result<SearchResponse, SearchError> {
        let start = Instant::now();
        let num = num_results.unwrap_or(self.config.default_num_results);

        let results = match self.config.provider {
            SearchProvider::Tavily => self.search_tavily(query, num).await?,
            SearchProvider::Serper => self.search_serper(query, num).await?,
            SearchProvider::Brave => self.search_brave(query, num).await?,
            SearchProvider::SerpApi => self.search_serpapi(query, num).await?,
        };

        Ok(SearchResponse {
            query: query.to_string(),
            results,
            total_results: None,
            search_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn search_tavily(&self, query: &str, num_results: u32) -> Result<Vec<SearchResult>, SearchError> {
        #[derive(Serialize)]
        struct TavilyRequest {
            api_key: String,
            query: String,
            search_depth: String,
            max_results: u32,
            include_answer: bool,
        }

        #[derive(Deserialize)]
        struct TavilyResponse {
            results: Vec<TavilyResult>,
        }

        #[derive(Deserialize)]
        struct TavilyResult {
            title: String,
            url: String,
            content: String,
            published_date: Option<String>,
        }

        let response = self.client
            .post("https://api.tavily.com/search")
            .json(&TavilyRequest {
                api_key: self.config.api_key.clone(),
                query: query.to_string(),
                search_depth: "basic".to_string(),
                max_results: num_results,
                include_answer: false,
            })
            .send()
            .await
            .map_err(|e| SearchError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SearchError::ApiError(format!("Tavily API error: {}", response.status())));
        }

        let tavily_response: TavilyResponse = response.json().await
            .map_err(|e| SearchError::ParseError(e.to_string()))?;

        Ok(tavily_response.results.into_iter().map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.content,
            published_date: r.published_date,
            source: Some("tavily".to_string()),
        }).collect())
    }

    async fn search_serper(&self, query: &str, num_results: u32) -> Result<Vec<SearchResult>, SearchError> {
        #[derive(Serialize)]
        struct SerperRequest {
            q: String,
            num: u32,
        }

        #[derive(Deserialize)]
        struct SerperResponse {
            organic: Vec<SerperResult>,
        }

        #[derive(Deserialize)]
        struct SerperResult {
            title: String,
            link: String,
            snippet: String,
            date: Option<String>,
        }

        let response = self.client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&SerperRequest {
                q: query.to_string(),
                num: num_results,
            })
            .send()
            .await
            .map_err(|e| SearchError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SearchError::ApiError(format!("Serper API error: {}", response.status())));
        }

        let serper_response: SerperResponse = response.json().await
            .map_err(|e| SearchError::ParseError(e.to_string()))?;

        Ok(serper_response.organic.into_iter().map(|r| SearchResult {
            title: r.title,
            url: r.link,
            snippet: r.snippet,
            published_date: r.date,
            source: Some("serper".to_string()),
        }).collect())
    }

    async fn search_brave(&self, query: &str, num_results: u32) -> Result<Vec<SearchResult>, SearchError> {
        #[derive(Deserialize)]
        struct BraveResponse {
            web: BraveWebResults,
        }

        #[derive(Deserialize)]
        struct BraveWebResults {
            results: Vec<BraveResult>,
        }

        #[derive(Deserialize)]
        struct BraveResult {
            title: String,
            url: String,
            description: String,
            age: Option<String>,
        }

        let response = self.client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", &self.config.api_key)
            .query(&[
                ("q", query),
                ("count", &num_results.to_string()),
            ])
            .send()
            .await
            .map_err(|e| SearchError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SearchError::ApiError(format!("Brave API error: {}", response.status())));
        }

        let brave_response: BraveResponse = response.json().await
            .map_err(|e| SearchError::ParseError(e.to_string()))?;

        Ok(brave_response.web.results.into_iter().map(|r| SearchResult {
            title: r.title,
            url: r.url,
            snippet: r.description,
            published_date: r.age,
            source: Some("brave".to_string()),
        }).collect())
    }

    async fn search_serpapi(&self, query: &str, num_results: u32) -> Result<Vec<SearchResult>, SearchError> {
        #[derive(Deserialize)]
        struct SerpApiResponse {
            organic_results: Vec<SerpApiResult>,
        }

        #[derive(Deserialize)]
        struct SerpApiResult {
            title: String,
            link: String,
            snippet: String,
            date: Option<String>,
        }

        let response = self.client
            .get("https://serpapi.com/search")
            .query(&[
                ("api_key", self.config.api_key.as_str()),
                ("q", query),
                ("num", &num_results.to_string()),
                ("engine", "google"),
            ])
            .send()
            .await
            .map_err(|e| SearchError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SearchError::ApiError(format!("SerpAPI error: {}", response.status())));
        }

        let serpapi_response: SerpApiResponse = response.json().await
            .map_err(|e| SearchError::ParseError(e.to_string()))?;

        Ok(serpapi_response.organic_results.into_iter().map(|r| SearchResult {
            title: r.title,
            url: r.link,
            snippet: r.snippet,
            published_date: r.date,
            source: Some("serpapi".to_string()),
        }).collect())
    }
}

#[derive(Debug)]
pub enum SearchError {
    RequestFailed(String),
    ApiError(String),
    ParseError(String),
    RateLimited,
}

// ============================================================================
// Web Fetcher
// ============================================================================

/// Configuration for web fetching
#[derive(Debug, Clone)]
pub struct WebFetchConfig {
    pub timeout_seconds: u64,
    pub max_content_length: usize,
    pub user_agent: String,
}

impl Default for WebFetchConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_content_length: 1_000_000, // 1MB
            user_agent: "PCG-Agent/1.0".to_string(),
        }
    }
}

/// Fetched page content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedPage {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub content_type: String,
    pub metadata: PageMetadata,
    pub fetch_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub author: Option<String>,
    pub og_title: Option<String>,
    pub og_description: Option<String>,
    pub og_image: Option<String>,
}

/// Web content fetcher
pub struct WebFetcher {
    config: WebFetchConfig,
    client: Client,
}

impl WebFetcher {
    pub fn new(config: WebFetchConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .user_agent(&config.user_agent)
            .build()
            .expect("Failed to build HTTP client");

        Self { config, client }
    }

    /// Fetch and extract content from a URL
    pub async fn fetch(&self, url: &str) -> Result<FetchedPage, FetchError> {
        let start = Instant::now();

        let response = self.client
            .get(url)
            .send()
            .await
            .map_err(|e| FetchError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(FetchError::HttpError(response.status().as_u16()));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let html = response.text().await
            .map_err(|e| FetchError::ParseError(e.to_string()))?;

        if html.len() > self.config.max_content_length {
            return Err(FetchError::ContentTooLarge);
        }

        // Parse HTML and extract content
        let (title, content, metadata) = self.extract_content(&html);

        Ok(FetchedPage {
            url: url.to_string(),
            title,
            content,
            content_type,
            metadata,
            fetch_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn extract_content(&self, html: &str) -> (Option<String>, String, PageMetadata) {
        use scraper::{Html, Selector};

        let document = Html::parse_document(html);

        // Extract title
        let title = Selector::parse("title").ok()
            .and_then(|sel| document.select(&sel).next())
            .map(|el| el.text().collect::<String>().trim().to_string());

        // Extract metadata
        let get_meta = |name: &str| -> Option<String> {
            let selector = Selector::parse(&format!("meta[name='{}'], meta[property='{}']", name, name)).ok()?;
            document.select(&selector).next()
                .and_then(|el| el.value().attr("content"))
                .map(|s| s.to_string())
        };

        let metadata = PageMetadata {
            description: get_meta("description"),
            keywords: get_meta("keywords").map(|k| k.split(',').map(|s| s.trim().to_string()).collect()),
            author: get_meta("author"),
            og_title: get_meta("og:title"),
            og_description: get_meta("og:description"),
            og_image: get_meta("og:image"),
        };

        // Extract main content
        let content = self.extract_main_content(&document);

        (title, content, metadata)
    }

    fn extract_main_content(&self, document: &scraper::Html) -> String {
        use scraper::Selector;

        // Try common content selectors in order of preference
        let content_selectors = [
            "article",
            "main",
            "[role='main']",
            ".content",
            ".post-content",
            ".entry-content",
            "#content",
            "body",
        ];

        for selector_str in content_selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    // Extract text, removing script and style content
                    let text: String = element
                        .text()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ");

                    if text.len() > 100 {
                        return text;
                    }
                }
            }
        }

        // Fallback to body text
        document.root_element()
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for WebFetcher {
    fn default() -> Self {
        Self::new(WebFetchConfig::default())
    }
}

#[derive(Debug)]
pub enum FetchError {
    RequestFailed(String),
    HttpError(u16),
    ParseError(String),
    ContentTooLarge,
}

// ============================================================================
// Competitor Analyzer
// ============================================================================

/// Competitor analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorProfile {
    pub company_name: String,
    pub website_url: Option<String>,
    pub description: Option<String>,
    pub industry: Option<String>,
    pub positioning: Option<String>,
    pub value_proposition: Option<String>,
    pub target_audience: Option<String>,
    pub key_features: Vec<String>,
    pub pricing_model: Option<String>,
    pub social_presence: SocialPresence,
    pub website_analysis: Option<WebsiteAnalysis>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialPresence {
    pub twitter: Option<SocialAccount>,
    pub linkedin: Option<SocialAccount>,
    pub instagram: Option<SocialAccount>,
    pub facebook: Option<SocialAccount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialAccount {
    pub url: String,
    pub followers: Option<u64>,
    pub posts_count: Option<u64>,
    pub engagement_rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteAnalysis {
    pub tech_stack: Vec<String>,
    pub performance_score: Option<u32>,
    pub mobile_friendly: Option<bool>,
    pub ssl_enabled: bool,
    pub page_count_estimate: Option<u32>,
}

/// Competitor analyzer service
pub struct CompetitorAnalyzer {
    web_fetcher: WebFetcher,
    search_provider: Option<WebSearchProvider>,
}

impl CompetitorAnalyzer {
    pub fn new(search_provider: Option<WebSearchProvider>) -> Self {
        Self {
            web_fetcher: WebFetcher::default(),
            search_provider,
        }
    }

    /// Analyze a competitor
    pub async fn analyze(&self, company_name: &str, website_url: Option<&str>) -> Result<CompetitorProfile, AnalysisError> {
        let mut profile = CompetitorProfile {
            company_name: company_name.to_string(),
            website_url: website_url.map(|s| s.to_string()),
            description: None,
            industry: None,
            positioning: None,
            value_proposition: None,
            target_audience: None,
            key_features: vec![],
            pricing_model: None,
            social_presence: SocialPresence {
                twitter: None,
                linkedin: None,
                instagram: None,
                facebook: None,
            },
            website_analysis: None,
            strengths: vec![],
            weaknesses: vec![],
        };

        // Analyze website if URL provided
        if let Some(url) = website_url {
            if let Ok(page) = self.web_fetcher.fetch(url).await {
                profile.description = page.metadata.description.clone()
                    .or(page.metadata.og_description.clone());

                // Extract value proposition from homepage
                profile.value_proposition = self.extract_value_proposition(&page.content);

                profile.website_analysis = Some(WebsiteAnalysis {
                    tech_stack: self.detect_tech_stack(&page.content),
                    performance_score: None, // Would need Lighthouse
                    mobile_friendly: None,
                    ssl_enabled: url.starts_with("https"),
                    page_count_estimate: None,
                });
            }
        }

        // Search for additional information
        if let Some(ref search) = self.search_provider {
            if let Ok(results) = search.search(&format!("{} company", company_name), Some(5)).await {
                // Extract information from search results
                for result in results.results {
                    if result.url.contains("linkedin.com") {
                        profile.social_presence.linkedin = Some(SocialAccount {
                            url: result.url,
                            followers: None,
                            posts_count: None,
                            engagement_rate: None,
                        });
                    } else if result.url.contains("twitter.com") || result.url.contains("x.com") {
                        profile.social_presence.twitter = Some(SocialAccount {
                            url: result.url,
                            followers: None,
                            posts_count: None,
                            engagement_rate: None,
                        });
                    }
                }
            }
        }

        Ok(profile)
    }

    fn extract_value_proposition(&self, content: &str) -> Option<String> {
        // Simple heuristic: look for short, impactful sentences at the start
        let sentences: Vec<&str> = content.split(|c| c == '.' || c == '!' || c == '?')
            .map(|s| s.trim())
            .filter(|s| s.len() > 20 && s.len() < 200)
            .take(3)
            .collect();

        sentences.first().map(|s| s.to_string())
    }

    fn detect_tech_stack(&self, html: &str) -> Vec<String> {
        let mut tech = vec![];

        // Simple detection based on common patterns
        if html.contains("react") || html.contains("__NEXT_DATA__") {
            tech.push("React".to_string());
        }
        if html.contains("__NEXT_DATA__") {
            tech.push("Next.js".to_string());
        }
        if html.contains("vue") {
            tech.push("Vue.js".to_string());
        }
        if html.contains("angular") {
            tech.push("Angular".to_string());
        }
        if html.contains("webflow") {
            tech.push("Webflow".to_string());
        }
        if html.contains("shopify") {
            tech.push("Shopify".to_string());
        }
        if html.contains("wordpress") || html.contains("wp-content") {
            tech.push("WordPress".to_string());
        }
        if html.contains("tailwind") {
            tech.push("Tailwind CSS".to_string());
        }

        tech
    }
}

#[derive(Debug)]
pub enum AnalysisError {
    FetchFailed(String),
    ParseError(String),
}

// ============================================================================
// Research Tools (High-level interface)
// ============================================================================

/// High-level research tools interface
pub struct ResearchTools {
    pub search: Option<WebSearchProvider>,
    pub fetcher: WebFetcher,
    pub competitor_analyzer: CompetitorAnalyzer,
}

impl ResearchTools {
    pub fn new(search_credentials: Option<(SearchProvider, ServiceCredentials)>) -> Self {
        let search = search_credentials
            .and_then(|(provider, creds)| WebSearchProvider::from_credentials(provider, &creds));

        let competitor_analyzer = CompetitorAnalyzer::new(search.clone());

        Self {
            search,
            fetcher: WebFetcher::default(),
            competitor_analyzer,
        }
    }

    /// Execute a research tool by name
    pub async fn execute(&self, tool_name: &str, params: serde_json::Value) -> ToolResult {
        let start = Instant::now();

        let result = match tool_name {
            "web_search" => self.execute_web_search(params).await,
            "web_fetch" => self.execute_web_fetch(params).await,
            "analyze_competitor" => self.execute_analyze_competitor(params).await,
            _ => Err(format!("Unknown tool: {}", tool_name)),
        };

        match result {
            Ok(data) => ToolResult {
                success: true,
                tool_name: tool_name.to_string(),
                agent_name: "research".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data,
                error: None,
            },
            Err(error) => ToolResult {
                success: false,
                tool_name: tool_name.to_string(),
                agent_name: "research".to_string(),
                execution_time_ms: start.elapsed().as_millis() as u64,
                data: serde_json::json!({}),
                error: Some(error),
            },
        }
    }

    async fn execute_web_search(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let search = self.search.as_ref()
            .ok_or("Search provider not configured")?;

        let query = params.get("query")
            .and_then(|v| v.as_str())
            .ok_or("Missing query parameter")?;

        let num_results = params.get("num_results")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);

        let response = search.search(query, num_results).await
            .map_err(|e| format!("Search failed: {:?}", e))?;

        Ok(serde_json::to_value(response).unwrap())
    }

    async fn execute_web_fetch(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = params.get("url")
            .and_then(|v| v.as_str())
            .ok_or("Missing url parameter")?;

        let page = self.fetcher.fetch(url).await
            .map_err(|e| format!("Fetch failed: {:?}", e))?;

        Ok(serde_json::to_value(page).unwrap())
    }

    async fn execute_analyze_competitor(&self, params: serde_json::Value) -> Result<serde_json::Value, String> {
        let company_name = params.get("company_name")
            .and_then(|v| v.as_str())
            .ok_or("Missing company_name parameter")?;

        let website_url = params.get("website_url")
            .and_then(|v| v.as_str());

        let profile = self.competitor_analyzer.analyze(company_name, website_url).await
            .map_err(|e| format!("Analysis failed: {:?}", e))?;

        Ok(serde_json::to_value(profile).unwrap())
    }
}
