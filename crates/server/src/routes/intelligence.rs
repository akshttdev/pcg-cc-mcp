//! Contact Intelligence — person research pipeline.
//!
//! Architecture:
//!   1. POST /api/persons/:id/research  → Nora receives the orchestration request
//!   2. Nora delegates to Scout (Social Intelligence) or Astra (Strategy Research)
//!   3. Agent performs web research via its tool suite
//!   4. Results are stored in persons.intelligence_* fields
//!   5. Results are registered in project_knowledge_sources as 'entity' type
//!      so the entire org knowledge graph stays current
//!
//! The endpoint is non-blocking — it sets intelligence_status='queued', fires
//! the Nora request asynchronously, and returns immediately with a job token.
//! The frontend polls GET /api/persons/:id/intelligence-status.

use axum::{
    Router,
    extract::{Path, State},
    routing::{get, post},
    Json,
};
use db::models::person::Person;
use db::models::project_knowledge_source::{KnowledgeSourceType, ProjectKnowledgeSource};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    DeploymentImpl,
    error::ApiError,
    routes::nora::get_nora_instance,
};

// Re-export the types we need from nora
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ResearchRequest {
    /// Optional project scope — if provided, results also go into that project's knowledge graph
    pub project_id: Option<Uuid>,
    /// Preferred agent: 'scout' (social intel) | 'astra' (market research) | auto
    pub agent_preference: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResearchJobResponse {
    pub person_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct IntelligenceStatusResponse {
    pub person_id: Uuid,
    pub status: String,
    pub summary: Option<String>,
    pub confidence: f64,
    pub agent: Option<String>,
    pub last_run_at: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/persons/:id/research
///
/// Triggers Nora to orchestrate contact intelligence gathering via Scout/Astra.
/// Non-blocking — sets status = 'queued', fires async task, returns job token.
pub async fn trigger_research(
    State(d): State<DeploymentImpl>,
    Path(person_id): Path<Uuid>,
    Json(body): Json<ResearchRequest>,
) -> Result<Json<ApiResponse<ResearchJobResponse>>, ApiError> {
    let pool = &d.db().pool;

    // Fetch the person
    let person = Person::find_by_id(pool, person_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Person not found".into()))?;

    // Mark as queued immediately
    sqlx::query(
        "UPDATE persons SET intelligence_status = 'queued', \
         intelligence_agent = ?, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(body.agent_preference.as_deref().unwrap_or("scout"))
    .bind(person_id)
    .execute(pool)
    .await?;

    // Build the research prompt — Nora will delegate to the right agent
    let agent_pref = body.agent_preference.as_deref().unwrap_or("Scout");
    let project_context = body.project_id
        .map(|pid| format!(" Store results in project knowledge graph (project_id={}).", pid))
        .unwrap_or_default();

    let research_prompt = format!(
        "[INTELLIGENCE TASK — delegate to {}] \
         Research the following contact and return structured intelligence: \
         Name: {}, Email: {}, Company: {}, Person type: {}. \
         Gather: current role/title, social media profiles (LinkedIn/Twitter/Instagram/GitHub), \
         company background, recent public activity, estimated influence and relevance to PCG. \
         Return a JSON object with keys: summary (1–2 sentence profile), \
         social_profiles (array of {{platform, handle, url, followers}}), \
         company_description, confidence (0.0–1.0). \
         Then update the person record with the structured data.{}",
        agent_pref,
        person.full_name,
        person.email.as_deref().unwrap_or("unknown"),
        person.company_name.as_deref().unwrap_or("unknown"),
        person.person_type,
        project_context
    );

    // Fire async task — Nora orchestrates, Scout/Astra executes
    let pool_clone = pool.clone();
    let project_id = body.project_id;
    let full_name = person.full_name.clone();
    let use_direct = body.agent_preference.as_deref() == Some("direct");

    tokio::spawn(async move {
        let result = if use_direct {
            run_research_direct(&pool_clone, person_id, &full_name, project_id).await
        } else {
            run_research_via_nora(
                &pool_clone,
                person_id,
                research_prompt,
                project_id,
                &full_name,
            )
            .await
        };
        if let Err(e) = result {
            tracing::error!("Intelligence research failed for person {}: {}", person_id, e);
            let _ = sqlx::query(
                "UPDATE persons SET intelligence_status = 'failed', updated_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(person_id)
            .execute(&pool_clone)
            .await;
        }
    });

    Ok(Json(ApiResponse::success(ResearchJobResponse {
        person_id,
        status: "queued".into(),
        message: format!(
            "Research queued for {} — {} will gather intelligence and update this profile.",
            person.full_name, agent_pref
        ),
    })))
}

/// GET /api/persons/:id/intelligence-status
pub async fn get_intelligence_status(
    State(d): State<DeploymentImpl>,
    Path(person_id): Path<Uuid>,
) -> Result<Json<ApiResponse<IntelligenceStatusResponse>>, ApiError> {
    let pool = &d.db().pool;

    #[derive(sqlx::FromRow)]
    struct Row {
        intelligence_status: String,
        intelligence_summary: Option<String>,
        intelligence_confidence: f64,
        intelligence_agent: Option<String>,
        intelligence_last_run_at: Option<String>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT intelligence_status, intelligence_summary, intelligence_confidence, \
         intelligence_agent, intelligence_last_run_at FROM persons WHERE id = ?",
    )
    .bind(person_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Person not found".into()))?;

    Ok(Json(ApiResponse::success(IntelligenceStatusResponse {
        person_id,
        status: row.intelligence_status,
        summary: row.intelligence_summary,
        confidence: row.intelligence_confidence,
        agent: row.intelligence_agent,
        last_run_at: row.intelligence_last_run_at,
    })))
}

// ── Core research execution ───────────────────────────────────────────────────

async fn run_research_via_nora(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    prompt: String,
    project_id: Option<Uuid>,
    full_name: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Mark as running
    sqlx::query(
        "UPDATE persons SET intelligence_status = 'running', updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(person_id)
    .execute(pool)
    .await?;

    // Get Nora instance — Nora orchestrates the delegation
    let nora_instance = match get_nora_instance().await {
        Ok(n) => n,
        Err(_) => {
            // Nora not initialized — run a direct research fallback
            return run_research_direct(pool, person_id, full_name, project_id).await;
        }
    };

    // Use Nora agent directly from the locked instance
    let response = {
        let nora_guard = nora_instance.read().await;
        let Some(nora) = nora_guard.as_ref() else {
            drop(nora_guard);
            return run_research_direct(pool, person_id, full_name, project_id).await;
        };

        let nora_request = NoraRequest {
            request_id: Uuid::new_v4().to_string(),
            session_id: format!("research-{}", person_id),
            request_type: NoraRequestType::TextInteraction,
            content: prompt,
            context: None,
            voice_enabled: false,
            priority: RequestPriority::High,
            timestamp: chrono::Utc::now(),
        };

        tokio::time::timeout(
            std::time::Duration::from_secs(60),
            nora.process_request(nora_request),
        )
        .await
        .map_err(|_| "Research timed out after 60s")?
        .map_err(|e| format!("Nora error: {}", e))?
    };

    // Detect Nora failure responses (quota exceeded, API errors) and fall back to direct
    let content_lower = response.content.to_lowercase();
    let is_failure_response = content_lower.contains("api quota")
        || content_lower.contains("quota limit")
        || content_lower.contains("quota exceeded")
        || content_lower.contains("quota limitation")
        || content_lower.contains("openai api quota")
        || (content_lower.contains("api quota") && content_lower.contains("exceeded"));

    if is_failure_response {
        tracing::warn!("Nora research hit quota error for person {}, falling back to direct Anthropic research", person_id);
        return run_research_direct(pool, person_id, full_name, project_id).await;
    }

    // Parse Nora's response and write to person record
    let summary = extract_summary_from_response(&response.content);
    let confidence = extract_confidence_from_response(&response.content);

    write_intelligence_results(pool, person_id, &summary, confidence, &response.content, project_id, full_name).await?;
    Ok(())
}

/// Fallback: direct Anthropic web search when Nora is not initialized.
/// Uses Scout's persona — social intelligence specialization.
async fn run_research_direct(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    full_name: &str,
    project_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use serde_json::json;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| "ANTHROPIC_API_KEY not set")?;

    let system = "You are Scout, Social Intelligence Analyst for Power Club Global. \
        Your specialty is finding and structuring online presence data about individuals. \
        When asked to research a person, use web search to gather their: \
        current role, company, LinkedIn/social profiles, recent activity, and public bio. \
        Always return valid JSON with keys: summary, social_profiles, company_description, confidence.";

    let prompt = format!(
        "Research {} for PCG contact intelligence. \
        Find their professional background, social media handles, company affiliation, \
        and any recent notable activity. Return structured JSON.",
        full_name
    );

    let body = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 1024,
        "system": system,
        "tools": [{
            "type": "web_search_20250305",
            "name": "web_search",
            "max_uses": 3
        }],
        "messages": [{"role": "user", "content": prompt}]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "web-search-2025-03-05")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    let response: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let response_text = extract_text_from_anthropic_response(&response);
    let summary = extract_summary_from_response(&response_text);
    let confidence = extract_confidence_from_response(&response_text);

    write_intelligence_results(pool, person_id, &summary, confidence, &response_text, project_id, full_name).await?;
    Ok(())
}

pub async fn write_intelligence_results(
    pool: &sqlx::SqlitePool,
    person_id: Uuid,
    summary: &str,
    confidence: f64,
    raw: &str,
    project_id: Option<Uuid>,
    full_name: &str,
) -> Result<(), sqlx::Error> {
    // Update person record
    sqlx::query(
        "UPDATE persons SET \
         intelligence_status = 'done', \
         intelligence_summary = ?, \
         intelligence_raw = ?, \
         intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         intelligence_agent = 'scout', \
         updated_at = datetime('now','subsec') \
         WHERE id = ?",
    )
    .bind(summary)
    .bind(raw)
    .bind(confidence)
    .bind(person_id)
    .execute(pool)
    .await?;

    // Register in knowledge graph if project_id provided
    if let Some(pid) = project_id {
        let source_id = format!("{}", person_id);
        let source_summary = Some(format!("Scout intelligence profile: {}", summary));
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            pid,
            &KnowledgeSourceType::Entity,
            &source_id,
            &format!("Person: {}", full_name),
            source_summary.as_deref(),
            confidence,
        )
        .await;
    }

    tracing::info!("Intelligence research complete for person {} (confidence: {:.0}%)", person_id, confidence * 100.0);
    Ok(())
}

// ── Text extraction helpers ───────────────────────────────────────────────────

fn extract_summary_from_response(text: &str) -> String {
    // Try direct JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(s) = v.get("summary").and_then(|s| s.as_str()) {
            return s.to_string();
        }
    }
    // Try JSON block — look for nested "summary" or "overview" anywhere in the JSON
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            let json_str = &text[start..=end];
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                // Try top-level "summary"
                if let Some(s) = v.get("summary").and_then(|s| s.as_str()) {
                    return s.to_string();
                }
                // Try nested patterns common in Scout responses
                for key in &["pcg_intelligence_report", "pcg_research_report", "pcg_contact_intelligence"] {
                    if let Some(nested) = v.get(key) {
                        if let Some(subj) = nested.get("subject") {
                            if let Some(s) = subj.get("summary").and_then(|s| s.as_str()) {
                                return s.to_string();
                            }
                        }
                        if let Some(s) = nested.get("summary") {
                            if let Some(ov) = s.get("overview").and_then(|o| o.as_str()) {
                                return ov.to_string();
                            }
                            if let Some(st) = s.as_str() {
                                return st.to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    // Strip markdown code fences and return first meaningful paragraph
    let clean: String = text.lines()
        .filter(|l| !l.trim_start().starts_with("```") && !l.trim().is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    clean.chars().take(600).collect()
}

fn extract_confidence_from_response(text: &str) -> f64 {
    // Try direct JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(c) = v.get("confidence").and_then(|c| c.as_f64()) {
            return c.clamp(0.0, 1.0);
        }
    }
    // Try JSON block extraction
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                if let Some(c) = v.get("confidence").and_then(|c| c.as_f64()) {
                    return c.clamp(0.0, 1.0);
                }
            }
        }
    }
    // Heuristic: if response contains substantive named data, assign higher confidence
    let lower = text.to_lowercase();
    let has_content = lower.contains("linkedin") || lower.contains("founded")
        || lower.contains("ceo") || lower.contains("revenue")
        || lower.contains("instagram") || lower.contains("twitter")
        || lower.contains("annual") || lower.contains("employees");
    let is_failure = lower.contains("insufficient data") || lower.contains("need more info")
        || lower.contains("could not find") || lower.contains("unable to verify")
        || lower.contains("no publicly") || lower.contains("i need a bit more");
    if is_failure {
        0.2
    } else if has_content {
        0.75
    } else {
        0.5
    }
}

fn extract_text_from_anthropic_response(response: &serde_json::Value) -> String {
    if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
        for block in content {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    return text.to_string();
                }
            }
        }
    }
    response.to_string()
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/persons/{id}/research", post(trigger_research))
        .route("/persons/{id}/intelligence-status", get(get_intelligence_status))
        .with_state(deployment.clone())
}
