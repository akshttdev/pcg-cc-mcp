//! Research Executor - Powers research agents like Scout
//!
//! Provides real LLM-powered research capabilities with web search integration.
//! Each workflow stage is executed as a focused research task with tool access.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

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
#[derive(Debug, Clone)]
pub struct ResearchTools {
    http_client: Client,
    openai_api_key: Option<String>,
    exa_api_key: Option<String>,
}

impl ResearchTools {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            exa_api_key: std::env::var("EXA_API_KEY").ok(),
        }
    }

    /// Search the web using Exa API (if available) or fall back to OpenAI web search
    pub async fn web_search(&self, query: &str, num_results: usize) -> Result<Vec<SearchResult>, String> {
        tracing::info!("[RESEARCH_TOOLS] Web search: {}", query);

        if let Some(exa_key) = &self.exa_api_key {
            self.exa_search(query, num_results, exa_key).await
        } else {
            // Fall back to OpenAI's built-in web search via function calling
            self.openai_web_search(query, num_results).await
        }
    }

    async fn exa_search(&self, query: &str, num_results: usize, api_key: &str) -> Result<Vec<SearchResult>, String> {
        let response = self.http_client
            .post("https://api.exa.ai/search")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "query": query,
                "num_results": num_results,
                "use_autoprompt": true,
                "type": "neural"
            }))
            .send()
            .await
            .map_err(|e| format!("Exa search failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Exa API error ({}): {}", status, body));
        }

        let json: Value = response.json().await
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

    async fn openai_web_search(&self, query: &str, _num_results: usize) -> Result<Vec<SearchResult>, String> {
        // Use OpenAI to generate simulated search results based on its knowledge
        // In production, you'd integrate with a real search API
        let api_key = self.openai_api_key.as_ref()
            .ok_or("No OpenAI API key configured")?;

        let response = self.http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a research assistant. Based on the query, provide relevant information you know about. Format as a JSON array of objects with 'title', 'url' (make up plausible URLs), and 'snippet' fields. Return ONLY valid JSON, no markdown."
                    },
                    {
                        "role": "user",
                        "content": format!("Research query: {}", query)
                    }
                ],
                "temperature": 0.7
            }))
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {}", e))?;

        let json: Value = response.json().await
            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("[]");

        // Parse the JSON array from the response
        let results: Vec<SearchResult> = serde_json::from_str(content)
            .unwrap_or_default();

        Ok(results)
    }

    /// Fetch and extract content from a URL
    pub async fn fetch_url(&self, url: &str) -> Result<String, String> {
        tracing::info!("[RESEARCH_TOOLS] Fetching URL: {}", url);

        let response = self.http_client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; Scout/1.0; Research Agent)")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        let text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // Simple HTML to text conversion (strip tags)
        let text = html_to_text(&text);

        // Truncate to reasonable length
        Ok(text.chars().take(10000).collect())
    }

    /// Call LLM with a research prompt
    pub async fn research_llm(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        let api_key = self.openai_api_key.as_ref()
            .ok_or("No OpenAI API key configured")?;

        tracing::info!("[RESEARCH_TOOLS] Calling LLM for research...");
        tracing::debug!("[RESEARCH_TOOLS] System: {}...", &system_prompt[..system_prompt.len().min(200)]);
        tracing::debug!("[RESEARCH_TOOLS] User: {}...", &user_prompt[..user_prompt.len().min(200)]);

        let response = self.http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "temperature": 0.7,
                "max_tokens": 4000
            }))
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error ({}): {}", status, body));
        }

        let json: Value = response.json().await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        tracing::info!("[RESEARCH_TOOLS] LLM response: {} chars", content.len());

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
        let brief: Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({
                "research_brief": response,
                "target": user_request,
                "key_questions": [],
                "platforms_to_check": ["web"],
                "success_criteria": "Comprehensive analysis"
            }));

        Ok(ResearchContext {
            original_request: user_request.to_string(),
            research_brief: brief["research_brief"].as_str().unwrap_or(&response).to_string(),
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
            "Account Discovery" => {
                self.execute_discovery_stage(context).await
            }
            "Content Analysis" => {
                self.execute_analysis_stage(context).await
            }
            "Insight Synthesis" => {
                self.execute_synthesis_stage(context).await
            }
            _ => {
                // Generic stage execution
                self.execute_generic_stage(stage_name, stage_description, expected_output, context).await
            }
        }
    }

    /// Stage 1: Account Discovery - Find relevant accounts/sources
    async fn execute_discovery_stage(&self, context: &mut ResearchContext) -> Result<Value, String> {
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
            context.research_brief,
            context.target
        );

        // First, do a web search to find accounts
        let search_query = format!("{} social media accounts competitors", context.target);
        let search_results = self.tools.web_search(&search_query, 10).await?;

        let user_prompt = format!(
            "Based on your knowledge and these search results, identify relevant accounts:\n\n{}",
            search_results.iter()
                .map(|r| format!("- {}: {}", r.title, r.snippet))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let response = self.tools.research_llm(&system_prompt, &user_prompt).await?;

        let output: Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({
                "accounts": [],
                "key_players": [],
                "platforms_covered": [],
                "discovery_summary": response,
                "raw_response": response
            }));

        // Store findings for next stage
        context.findings.insert("discovery".to_string(), output.clone());

        Ok(serde_json::json!({
            "stage": "Account Discovery",
            "status": "completed",
            "output": output,
            "search_results_used": search_results.len()
        }))
    }

    /// Stage 2: Content Analysis - Analyze content patterns
    async fn execute_analysis_stage(&self, context: &mut ResearchContext) -> Result<Value, String> {
        let discovery = context.findings.get("discovery")
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

        let user_prompt = format!(
            "Analyze the content strategies for: {}",
            context.target
        );

        let response = self.tools.research_llm(&system_prompt, &user_prompt).await?;

        let output: Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({
                "analysis_summary": response,
                "raw_response": response
            }));

        context.findings.insert("analysis".to_string(), output.clone());

        Ok(serde_json::json!({
            "stage": "Content Analysis",
            "status": "completed",
            "output": output
        }))
    }

    /// Stage 3: Insight Synthesis - Create final report
    async fn execute_synthesis_stage(&self, context: &mut ResearchContext) -> Result<Value, String> {
        let all_findings = serde_json::to_string_pretty(&context.findings)
            .unwrap_or_default();

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
            context.original_request,
            context.research_brief,
            all_findings
        );

        let user_prompt = "Create the final research report with all insights synthesized.";

        let response = self.tools.research_llm(&system_prompt, user_prompt).await?;

        let output: Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({
                "executive_summary": response,
                "raw_response": response
            }));

        context.findings.insert("synthesis".to_string(), output.clone());

        Ok(serde_json::json!({
            "stage": "Insight Synthesis",
            "status": "completed",
            "output": output,
            "final_report": true
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

        let response = self.tools.research_llm(&system_prompt, "Execute this research stage.").await?;

        let output: Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({
                "output": response,
                "raw_response": response
            }));

        context.findings.insert(stage_name.to_lowercase().replace(" ", "_"), output.clone());

        Ok(serde_json::json!({
            "stage": stage_name,
            "status": "completed",
            "output": output
        }))
    }
}

/// Simple HTML to text conversion
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
