//! Research Executor - Powers research agents like Scout
//!
//! Provides real LLM-powered research capabilities with web search integration.
//! Each workflow stage is executed as a focused research task with tool access.
//!
//! LLM calls are routed through PCG Router via WorkflowLLMService for centralized
//! API key management and cost tracking.

use std::{collections::HashMap, sync::Arc};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use services::services::workflow_llm::WorkflowLLMService;
use sqlx::SqlitePool;
use uuid::Uuid;

/// Structured data extracted from a scraped web page
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScrapedPage {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub og_image: Option<String>,
    pub favicon: Option<String>,
    pub text: String,
    pub images: Vec<String>,
    pub logos: Vec<String>,
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub social_links: HashMap<String, String>,
    pub brand_colors: Vec<String>,
}

/// Research context passed between stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchContext {
    /// Original user request
    pub original_request: String,
    /// Enhanced research brief from Nora
    pub research_brief: String,
    /// Project context
    pub project_name: Option<String>,
    /// Target topic/subject
    pub target: String,
    /// Accumulated findings from previous stages
    pub findings: HashMap<String, Value>,
}

/// Tools available to research agents
#[derive(Clone)]
pub struct ResearchTools {
    http_client: Client,
    exa_api_key: Option<String>,
    /// Database pool for routing LLM calls through PCG Router.
    pool: Option<Arc<SqlitePool>>,
}

impl std::fmt::Debug for ResearchTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResearchTools")
            .field("exa_api_key", &self.exa_api_key.is_some())
            .field("pool", &self.pool.is_some())
            .finish()
    }
}

impl ResearchTools {
    /// Create ResearchTools with PCG Router support.
    ///
    /// LLM calls will be routed through WorkflowLLMService using the provided
    /// database pool for API key lookup and cost tracking.
    pub fn with_pool(pool: Arc<SqlitePool>) -> Self {
        let exa_api_key = std::env::var("EXA_API_KEY").ok();
        if exa_api_key.is_some() {
            tracing::info!("[RESEARCH_TOOLS] Exa neural search enabled");
        } else {
            tracing::info!(
                "[RESEARCH_TOOLS] Exa API key not found, will fall back to PCG Router LLM"
            );
        }
        Self {
            http_client: Client::new(),
            exa_api_key,
            pool: Some(pool),
        }
    }

    /// Create ResearchTools without database pool (legacy mode).
    ///
    /// LLM calls will fail with an error since no pool is available for routing.
    /// Use `with_pool()` for production code.
    pub fn new() -> Self {
        let exa_api_key = std::env::var("EXA_API_KEY").ok();
        if exa_api_key.is_some() {
            tracing::info!("[RESEARCH_TOOLS] Exa neural search enabled");
        } else {
            tracing::info!("[RESEARCH_TOOLS] Exa API key not found, LLM fallback requires pool");
        }
        Self {
            http_client: Client::new(),
            exa_api_key,
            pool: None,
        }
    }

    /// Search the web using Exa API (if available) or fall back to LLM-based search
    pub async fn web_search(
        &self,
        query: &str,
        num_results: usize,
    ) -> Result<Vec<SearchResult>, String> {
        tracing::info!("[RESEARCH_TOOLS] Web search: {}", query);

        if let Some(exa_key) = &self.exa_api_key {
            self.exa_search(query, num_results, exa_key).await
        } else {
            // Fall back to PCG Router LLM-based search
            self.llm_web_search(query, num_results).await
        }
    }

    async fn exa_search(
        &self,
        query: &str,
        num_results: usize,
        api_key: &str,
    ) -> Result<Vec<SearchResult>, String> {
        let response = self
            .http_client
            .post("https://api.exa.ai/search")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "query": query,
                "num_results": num_results,
                "use_autoprompt": true,
                "type": "neural",
                "contents": {
                    "text": true
                }
            }))
            .send()
            .await
            .map_err(|e| format!("Exa search failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Exa API error ({}): {}", status, body));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Exa response: {}", e))?;

        let results = json["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|r| SearchResult {
                        title: r["title"].as_str().unwrap_or("").to_string(),
                        url: r["url"].as_str().unwrap_or("").to_string(),
                        snippet: r["text"].as_str().unwrap_or("").to_string(),
                        published_date: r["publishedDate"].as_str().map(String::from),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn llm_web_search(
        &self,
        query: &str,
        _num_results: usize,
    ) -> Result<Vec<SearchResult>, String> {
        // Use PCG Router to generate simulated search results based on LLM knowledge
        let pool = self
            .pool
            .as_ref()
            .ok_or("No database pool configured for LLM routing")?;

        let system_prompt = "You are a research assistant. Based on the query, provide relevant information you know about. Format as a JSON array of objects with 'title', 'url' (make up plausible URLs), and 'snippet' fields. Return ONLY valid JSON, no markdown.";
        let user_prompt = format!("Research query: {}", query);

        let messages = vec![
            WorkflowLLMService::system_message(system_prompt),
            WorkflowLLMService::user_message(&user_prompt),
        ];

        let (content, metadata) = WorkflowLLMService::completion(
            pool,
            messages,
            None, // Use default model from PCG Router
            Some(2000),
            Some(0.7),
        )
        .await
        .map_err(|e| format!("PCG Router LLM request failed: {}", e))?;

        tracing::info!(
            "[RESEARCH_TOOLS] LLM search via PCG Router: model={}, provider={}, cost={}µ",
            metadata.model_used,
            metadata.provider,
            metadata.estimated_cost_micros.unwrap_or(0)
        );

        // Parse the JSON array from the response
        let results: Vec<SearchResult> = serde_json::from_str(&content).unwrap_or_default();

        Ok(results)
    }

    /// Scrape a URL and return structured assets (images, logos, contact info, social links, colors).
    /// Tries Playwright for JS-heavy sites, falls back to static HTTP.
    pub async fn scrape_url(&self, url: &str) -> Result<ScrapedPage, String> {
        tracing::info!("[RESEARCH_TOOLS] Scraping URL: {}", url);

        // Try static fetch first
        let html = match self
            .http_client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (compatible; Scout/1.0; Research Agent)",
            )
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
        {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(e) => return Err(format!("Failed to fetch {}: {}", url, e)),
        };

        // Detect if the page needs JS rendering (empty body or SPA markers)
        let needs_js = html.len() < 2000
            || html.contains("__NEXT_DATA__")
            || html.contains("data-reactroot")
            || html.contains("ng-app")
            || (html.contains("<noscript>") && html.len() < 5000);

        let html = if needs_js {
            tracing::info!("[RESEARCH_TOOLS] JS rendering needed for {}", url);
            // Try render-page.js Playwright script
            let script_path = [
                "/home/pythia/pcg-cc-mcp/scripts/render-page.js",
                "scripts/render-page.js",
                "./scripts/render-page.js",
            ]
            .iter()
            .find(|p| std::path::Path::new(*p).exists())
            .map(|p| p.to_string());

            let playwright_ok = tokio::task::spawn_blocking(|| {
                std::process::Command::new("node")
                    .args(["-e", "require('playwright')"])
                    .output()
            })
            .await
            .ok()
            .and_then(|r| r.ok())
            .map(|o| o.status.success())
            .unwrap_or(false);

            if let (Some(script), true) = (script_path, playwright_ok) {
                let url_owned = url.to_string();
                let out = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("node")
                        .args([&script, &url_owned, "30000"])
                        .output()
                })
                .await
                .ok()
                .and_then(|r| r.ok());
                if let Some(out) = out {
                    if out.status.success() {
                        serde_json::from_slice::<Value>(&out.stdout)
                            .ok()
                            .and_then(|v| {
                                v.get("html")
                                    .and_then(|h| h.as_str())
                                    .map(|s| s.to_string())
                            })
                            .unwrap_or(html)
                    } else {
                        html
                    }
                } else {
                    html
                }
            } else {
                html
            }
        } else {
            html
        };

        Ok(extract_page_assets(&html, url))
    }

    /// Fetch and extract content from a URL
    #[allow(dead_code)]
    pub async fn fetch_url(&self, url: &str) -> Result<String, String> {
        tracing::info!("[RESEARCH_TOOLS] Fetching URL: {}", url);

        let response = self
            .http_client
            .get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (compatible; Scout/1.0; Research Agent)",
            )
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // Simple HTML to text conversion (strip tags)
        let text = html_to_text(&text);

        // Truncate to reasonable length
        Ok(text.chars().take(10000).collect())
    }

    /// Call LLM with a research prompt via PCG Router
    pub async fn research_llm(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let pool = self
            .pool
            .as_ref()
            .ok_or("No database pool configured for LLM routing")?;

        tracing::info!("[RESEARCH_TOOLS] Calling LLM for research via PCG Router...");
        tracing::debug!(
            "[RESEARCH_TOOLS] System: {}...",
            &system_prompt[..system_prompt.len().min(200)]
        );
        tracing::debug!(
            "[RESEARCH_TOOLS] User: {}...",
            &user_prompt[..user_prompt.len().min(200)]
        );

        let messages = vec![
            WorkflowLLMService::system_message(system_prompt),
            WorkflowLLMService::user_message(user_prompt),
        ];

        let (content, metadata) = WorkflowLLMService::completion(
            pool,
            messages,
            None, // Use default model from PCG Router
            Some(4000),
            Some(0.7),
        )
        .await
        .map_err(|e| format!("PCG Router LLM request failed: {}", e))?;

        tracing::info!(
            "[RESEARCH_TOOLS] LLM response: {} chars, model={}, provider={}, cost={}µ",
            content.len(),
            metadata.model_used,
            metadata.provider,
            metadata.estimated_cost_micros.unwrap_or(0)
        );

        Ok(content)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub published_date: Option<String>,
}

/// Research Executor - Runs research workflow stages
pub struct ResearchExecutor {
    tools: ResearchTools,
}

impl ResearchExecutor {
    /// Create a ResearchExecutor with PCG Router support.
    ///
    /// LLM calls will be routed through WorkflowLLMService using the provided
    /// database pool for API key lookup and cost tracking.
    pub fn with_pool(pool: Arc<SqlitePool>) -> Self {
        Self {
            tools: ResearchTools::with_pool(pool),
        }
    }

    /// Create a ResearchExecutor without database pool (legacy mode).
    ///
    /// LLM calls will fail since no pool is available for routing.
    /// Use `with_pool()` for production code.
    pub fn new() -> Self {
        Self {
            tools: ResearchTools::new(),
        }
    }

    /// Enhance user request into a detailed research brief
    pub async fn create_research_brief(
        &self,
        user_request: &str,
        project_context: Option<&str>,
    ) -> Result<ResearchContext, String> {
        let system_prompt = r#"You are Nora, an executive assistant preparing a research brief for Scout, a social intelligence analyst.

Your job is to transform the user's request into a structured research brief that Scout can execute.

Output a JSON object with these fields:
- research_brief: A detailed description of what to research (2-3 paragraphs)
- target: The main subject/topic to research (short phrase)
- key_questions: Array of 3-5 specific questions to answer
- platforms_to_check: Array of platforms/sources to investigate
- success_criteria: What constitutes successful research

Return ONLY valid JSON, no markdown formatting."#;

        let user_prompt = format!(
            "User request: {}\n\nProject context: {}",
            user_request,
            project_context.unwrap_or("General research")
        );

        let response = self.tools.research_llm(system_prompt, &user_prompt).await?;

        // Parse the JSON response
        let brief: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
            serde_json::json!({
                "research_brief": response,
                "target": user_request,
                "key_questions": [],
                "platforms_to_check": ["web"],
                "success_criteria": "Comprehensive analysis"
            })
        });

        Ok(ResearchContext {
            original_request: user_request.to_string(),
            research_brief: brief["research_brief"]
                .as_str()
                .unwrap_or(&response)
                .to_string(),
            project_name: project_context.map(String::from),
            target: brief["target"].as_str().unwrap_or(user_request).to_string(),
            findings: HashMap::new(),
        })
    }

    /// Execute a research stage
    pub async fn execute_stage(
        &self,
        execution_id: Uuid,
        stage_name: &str,
        stage_description: &str,
        expected_output: &str,
        context: &mut ResearchContext,
    ) -> Result<Value, String> {
        tracing::info!(
            "[RESEARCH_EXECUTOR] Executing stage '{}' for execution {}",
            stage_name,
            execution_id
        );

        match stage_name {
            "Account Discovery" => self.execute_discovery_stage(context).await,
            "Content Analysis" => self.execute_analysis_stage(context).await,
            "Insight Synthesis" => self.execute_synthesis_stage(context).await,
            "Website Scrape" | "Asset Collection" | "Brand Audit" => {
                self.execute_scrape_stage(context).await
            }
            _ => {
                // Generic stage execution
                self.execute_generic_stage(stage_name, stage_description, expected_output, context)
                    .await
            }
        }
    }

    /// Stage 1: Account Discovery - Find relevant accounts/sources
    async fn execute_discovery_stage(
        &self,
        context: &mut ResearchContext,
    ) -> Result<Value, String> {
        let system_prompt = format!(
            r#"You are Scout, a social intelligence analyst conducting the Account Discovery phase.

Research Brief: {}

Target: {}

Your task:
1. Identify key competitors, influencers, or relevant accounts related to this topic
2. List their platforms and handles
3. Note their approximate audience size and relevance

Output a JSON object with:
- accounts: Array of {{name, platform, handle, estimated_followers, relevance_score}}
- key_players: Top 5 most important accounts to analyze
- platforms_covered: Which platforms you found accounts on
- discovery_summary: Brief summary of what you found

Return ONLY valid JSON."#,
            context.research_brief, context.target
        );

        // First, do a web search to find accounts
        let search_query = format!("{} social media accounts competitors", context.target);
        let search_results = self.tools.web_search(&search_query, 10).await?;

        let user_prompt = format!(
            "Based on your knowledge and these search results, identify relevant accounts:\n\n{}",
            search_results
                .iter()
                .map(|r| format!("- {}: {}", r.title, r.snippet))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let response = self
            .tools
            .research_llm(&system_prompt, &user_prompt)
            .await?;

        let output: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
            serde_json::json!({
                "accounts": [],
                "key_players": [],
                "platforms_covered": [],
                "discovery_summary": response,
                "raw_response": response
            })
        });

        // Store findings for next stage
        context
            .findings
            .insert("discovery".to_string(), output.clone());

        Ok(serde_json::json!({
            "stage": "Account Discovery",
            "status": "completed",
            "output": output,
            "search_results_used": search_results.len()
        }))
    }

    /// Stage 2: Content Analysis - Analyze content patterns
    async fn execute_analysis_stage(&self, context: &mut ResearchContext) -> Result<Value, String> {
        let discovery = context
            .findings
            .get("discovery")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let system_prompt = format!(
            r#"You are Scout, a social intelligence analyst conducting the Content Analysis phase.

Research Brief: {}

Previous Discovery Findings:
{}

Your task:
1. Analyze content patterns from the discovered accounts
2. Identify posting frequency, content types, and engagement patterns
3. Note successful content themes and formats
4. Identify gaps and opportunities

Output a JSON object with:
- content_patterns: Array of {{pattern_name, description, frequency, effectiveness}}
- top_performing_content: Examples of high-engagement content
- posting_strategies: Common posting times, frequencies
- content_themes: Main topics/themes being covered
- engagement_insights: What drives engagement
- gaps_opportunities: Areas not being covered well
- analysis_summary: Overall analysis summary

Return ONLY valid JSON."#,
            context.research_brief,
            serde_json::to_string_pretty(&discovery).unwrap_or_default()
        );

        let user_prompt = format!("Analyze the content strategies for: {}", context.target);

        let response = self
            .tools
            .research_llm(&system_prompt, &user_prompt)
            .await?;

        let output: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
            serde_json::json!({
                "analysis_summary": response,
                "raw_response": response
            })
        });

        context
            .findings
            .insert("analysis".to_string(), output.clone());

        Ok(serde_json::json!({
            "stage": "Content Analysis",
            "status": "completed",
            "output": output
        }))
    }

    /// Stage 3: Insight Synthesis - Create final report
    async fn execute_synthesis_stage(
        &self,
        context: &mut ResearchContext,
    ) -> Result<Value, String> {
        let all_findings = serde_json::to_string_pretty(&context.findings).unwrap_or_default();

        let system_prompt = format!(
            r#"You are Scout, a social intelligence analyst creating the final Insight Synthesis report.

Original Request: {}

Research Brief: {}

All Findings:
{}

Your task:
Create a comprehensive research report with actionable insights.

Output a JSON object with:
- executive_summary: 2-3 paragraph summary for executives
- key_findings: Array of the most important discoveries
- recommendations: Array of actionable recommendations
- competitive_landscape: Overview of the competitive situation
- opportunities: Specific opportunities identified
- risks: Potential risks or threats noted
- next_steps: Suggested follow-up actions
- data_sources: What sources were used
- confidence_level: High/Medium/Low confidence in findings

Return ONLY valid JSON."#,
            context.original_request, context.research_brief, all_findings
        );

        let user_prompt = "Create the final research report with all insights synthesized.";

        let response = self.tools.research_llm(&system_prompt, user_prompt).await?;

        let output: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
            serde_json::json!({
                "executive_summary": response,
                "raw_response": response
            })
        });

        context
            .findings
            .insert("synthesis".to_string(), output.clone());

        Ok(serde_json::json!({
            "stage": "Insight Synthesis",
            "status": "completed",
            "output": output,
            "final_report": true
        }))
    }

    /// Website Scrape / Asset Collection / Brand Audit stage
    /// Looks for a `website_url` in context findings or extracts it from the research brief.
    async fn execute_scrape_stage(&self, context: &mut ResearchContext) -> Result<Value, String> {
        // Extract URL from context — check findings first, then target (if it looks like a URL)
        let url = context
            .findings
            .get("website_url")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                context
                    .findings
                    .get("discovery")
                    .and_then(|v| v.get("website"))
                    .and_then(|u| u.as_str())
                    .map(String::from)
            })
            .or_else(|| {
                // Try to extract URL from target or research_brief
                let re = regex::Regex::new(r"https?://[^\s<>]+").ok()?;
                let combined = format!("{} {}", context.target, context.research_brief);
                re.find(&combined).map(|m| m.as_str().to_string())
            });

        let url = match url {
            Some(u) => u,
            None => {
                // Fall back to a web search for the company's website
                let search_query = format!("{} official website", context.target);
                tracing::info!(
                    "[RESEARCH_EXECUTOR] No URL in context, searching for: {}",
                    search_query
                );
                let results = self.tools.web_search(&search_query, 3).await?;
                match results.first() {
                    Some(r) => r.url.clone(),
                    None => {
                        return Ok(serde_json::json!({
                            "stage": "Website Scrape",
                            "status": "skipped",
                            "reason": "No website URL found in research context",
                        }))
                    }
                }
            }
        };

        tracing::info!("[RESEARCH_EXECUTOR] Scraping website: {}", url);
        let scraped = self.tools.scrape_url(&url).await?;

        // Use LLM to synthesise brand intelligence from the scraped content
        let system_prompt = format!(
            r#"You are Scout, a brand intelligence analyst. You just scraped {}'s website.

Analyse the scraped data and extract key brand intelligence.

Return a JSON object with:
- brand_summary: 2-3 sentence description of the company/brand
- key_offerings: Array of main products or services
- tone_and_voice: Description of brand personality
- target_audience: Who this brand serves
- visual_identity: {{primary_color, secondary_colors, logo_found: bool, imagery_style}}
- contact_summary: Human-readable contact info found
- social_presence: Which platforms they're on
- data_quality: How complete/useful the scraped data was (0-100)
- recommendations: Array of follow-up research actions

Return ONLY valid JSON."#,
            context.target
        );

        let scraped_summary = format!(
            "Title: {}\nDescription: {}\nText excerpt: {}\nImages found: {}\nLogos found: {}\nEmails: {}\nPhones: {}\nSocial links: {}\nBrand colors: {}",
            scraped.title.as_deref().unwrap_or("N/A"),
            scraped.description.as_deref().unwrap_or("N/A"),
            scraped.text.chars().take(3000).collect::<String>(),
            scraped.images.len(),
            scraped.logos.len(),
            scraped.emails.join(", "),
            scraped.phones.join(", "),
            scraped.social_links.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<_>>().join(", "),
            scraped.brand_colors.join(", "),
        );

        let analysis = self
            .tools
            .research_llm(&system_prompt, &scraped_summary)
            .await
            .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
        let analysis_json: Value = serde_json::from_str(&analysis)
            .unwrap_or_else(|_| serde_json::json!({"raw": analysis}));

        let output = serde_json::json!({
            "scraped_url": url,
            "title": scraped.title,
            "description": scraped.description,
            "og_image": scraped.og_image,
            "favicon": scraped.favicon,
            "images": scraped.images,
            "logos": scraped.logos,
            "contact": {
                "emails": scraped.emails,
                "phones": scraped.phones,
            },
            "social_links": scraped.social_links,
            "brand_colors": scraped.brand_colors,
            "intelligence": analysis_json,
        });

        context
            .findings
            .insert("website_scrape".to_string(), output.clone());

        Ok(serde_json::json!({
            "stage": "Website Scrape",
            "status": "completed",
            "output": output,
        }))
    }

    /// Generic stage execution for custom stages
    async fn execute_generic_stage(
        &self,
        stage_name: &str,
        stage_description: &str,
        expected_output: &str,
        context: &mut ResearchContext,
    ) -> Result<Value, String> {
        let system_prompt = format!(
            r#"You are Scout, executing the "{}" stage of a research workflow.

Stage Description: {}
Expected Output: {}

Research Context:
- Target: {}
- Brief: {}
- Previous Findings: {}

Complete this stage and provide the expected output.

Return a JSON object with your findings."#,
            stage_name,
            stage_description,
            expected_output,
            context.target,
            context.research_brief,
            serde_json::to_string_pretty(&context.findings).unwrap_or_default()
        );

        let response = self
            .tools
            .research_llm(&system_prompt, "Execute this research stage.")
            .await?;

        let output: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
            serde_json::json!({
                "output": response,
                "raw_response": response
            })
        });

        context
            .findings
            .insert(stage_name.to_lowercase().replace(" ", "_"), output.clone());

        Ok(serde_json::json!({
            "stage": stage_name,
            "status": "completed",
            "output": output
        }))
    }
}

/// Extract structured brand assets from raw HTML
pub fn extract_page_assets(html: &str, url: &str) -> ScrapedPage {
    use std::collections::HashSet;

    let title = regex::Regex::new(r"(?i)<title[^>]*>([^<]+)</title>")
        .ok()
        .and_then(|re| re.captures(html))
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string());

    let description = {
        let r1 = regex::Regex::new(
            r#"(?i)<meta[^>]+name=["']description["'][^>]+content=["']([^"']+)["']"#,
        )
        .ok();
        let r2 = regex::Regex::new(
            r#"(?i)<meta[^>]+content=["']([^"']+)["'][^>]+name=["']description["']"#,
        )
        .ok();
        let r3 = regex::Regex::new(
            r#"(?i)<meta[^>]+property=["']og:description["'][^>]+content=["']([^"']+)["']"#,
        )
        .ok();
        [r1, r2, r3]
            .into_iter()
            .flatten()
            .find_map(|re| re.captures(html))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    };

    let og_image = {
        let r1 = regex::Regex::new(
            r#"(?i)<meta[^>]+property=["']og:image["'][^>]+content=["']([^"']+)["']"#,
        )
        .ok();
        let r2 = regex::Regex::new(
            r#"(?i)<meta[^>]+content=["']([^"']+)["'][^>]+property=["']og:image["']"#,
        )
        .ok();
        [r1, r2]
            .into_iter()
            .flatten()
            .find_map(|re| re.captures(html))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    };

    let favicon = {
        let r1 = regex::Regex::new(
            r#"(?i)<link[^>]+rel=["'][^"']*icon[^"']*["'][^>]+href=["']([^"']+)["']"#,
        )
        .ok();
        let r2 = regex::Regex::new(
            r#"(?i)<link[^>]+href=["']([^"']+)["'][^>]+rel=["'][^"']*icon[^"']*["']"#,
        )
        .ok();
        [r1, r2]
            .into_iter()
            .flatten()
            .find_map(|re| re.captures(html))
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
    };

    let text = html_to_text(html).chars().take(8000).collect::<String>();

    let images: Vec<String> = regex::Regex::new(r#"(?i)<img[^>]+src=["']([^"']+)["']"#)
        .map(|re| {
            re.captures_iter(html)
                .filter_map(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .filter(|s| !s.starts_with("data:"))
                .collect::<HashSet<_>>()
                .into_iter()
                .take(30)
                .collect()
        })
        .unwrap_or_default();

    let logos: Vec<String> = regex::Regex::new(r#"(?i)<img[^>]+(logo|brand)[^>]*>"#)
        .map(|re| {
            let src_re = regex::Regex::new(r#"(?i)src=["']([^"']+)["']"#).unwrap();
            re.find_iter(html)
                .filter_map(|m| src_re.captures(m.as_str()))
                .filter_map(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .filter(|s| !s.starts_with("data:"))
                .collect::<HashSet<_>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_default();

    let emails: Vec<String> =
        regex::Regex::new(r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b")
            .map(|re| {
                re.find_iter(&text)
                    .map(|m| m.as_str().to_string())
                    .filter(|e| {
                        !e.ends_with(".png") && !e.ends_with(".jpg") && !e.ends_with(".svg")
                    })
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .take(5)
                    .collect()
            })
            .unwrap_or_default();

    let phones: Vec<String> = regex::Regex::new(r"\(?\d{3}\)?[\s.\-]?\d{3}[\s.\-]?\d{4}")
        .map(|re| {
            re.find_iter(&text)
                .map(|m| m.as_str().trim().to_string())
                .collect::<HashSet<_>>()
                .into_iter()
                .take(5)
                .collect()
        })
        .unwrap_or_default();

    let all_links: Vec<String> = regex::Regex::new(r#"(?i)href=["']([^"']+)["']"#)
        .map(|re| {
            re.captures_iter(html)
                .filter_map(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .collect()
        })
        .unwrap_or_default();

    let mut social_links = HashMap::new();
    for (key, domain) in &[
        ("instagram", "instagram.com"),
        ("twitter", "twitter.com"),
        ("tiktok", "tiktok.com"),
        ("facebook", "facebook.com"),
        ("linkedin", "linkedin.com"),
        ("youtube", "youtube.com"),
        ("threads", "threads.net"),
    ] {
        if let Some(link) = all_links.iter().find(|l| l.contains(domain)) {
            social_links.insert(key.to_string(), link.clone());
        }
    }

    let brand_colors = {
        let mut style_content = String::new();
        if let Ok(re) = regex::Regex::new(r"(?is)<style[^>]*>(.*?)</style>") {
            for cap in re.captures_iter(html) {
                if let Some(m) = cap.get(1) {
                    style_content.push_str(m.as_str());
                }
            }
        }
        if let Ok(re) = regex::Regex::new(r#"(?i)style=["']([^"']+)["']"#) {
            for cap in re.captures_iter(html) {
                if let Some(m) = cap.get(1) {
                    style_content.push_str(m.as_str());
                }
            }
        }
        regex::Regex::new(r"#([0-9A-Fa-f]{6})\b")
            .map(|re| {
                let mut colors: Vec<String> = re
                    .find_iter(&style_content)
                    .map(|m| m.as_str().to_uppercase())
                    .filter(|c| {
                        !["#000000", "#FFFFFF", "#FAFAFA", "#F0F0F0", "#EEEEEE"]
                            .contains(&c.as_str())
                    })
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .take(10)
                    .collect();
                colors.sort();
                colors
            })
            .unwrap_or_default()
    };

    ScrapedPage {
        url: url.to_string(),
        title,
        description,
        og_image,
        favicon,
        text,
        images,
        logos,
        emails,
        phones,
        social_links,
        brand_colors,
    }
}

/// Simple HTML to text conversion
#[allow(dead_code)]
fn html_to_text(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script blocks
    if let Ok(re) = regex::Regex::new(r"<script[^>]*>[\s\S]*?</script>") {
        text = re.replace_all(&text, "").to_string();
    }

    // Remove style blocks
    if let Ok(re) = regex::Regex::new(r"<style[^>]*>[\s\S]*?</style>") {
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

impl Default for ResearchExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ResearchTools {
    fn default() -> Self {
        Self::new()
    }
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
        assert!(!text.contains("evil"));
    }
}
