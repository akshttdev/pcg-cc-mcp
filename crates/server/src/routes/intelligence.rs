//! Contact Intelligence — person research pipeline.
//!
//! Architecture:
//!   1. POST /api/persons/:id/research  → Nora receives the orchestration request
//!   2. Nora delegates to Scout (Social Intelligence) or Astra (Strategy Research)
//!   3. Agent performs web research via its tool suite
//!   4. Results are stored in persons.intelligence_* fields
//!   5. Social profiles, company contact methods, proposals, tasks, and
//!      business analysis files are all created/updated from the research output.

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
use db::db_uuid::DbUuid;


use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ResearchRequest {
    /// Optional project scope — if provided, results also go into that project's knowledge graph
    pub project_id: Option<Uuid>,

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

pub async fn trigger_research(
    State(d): State<DeploymentImpl>,
    Path(person_id): Path<Uuid>,
    Json(body): Json<ResearchRequest>,
) -> Result<Json<ApiResponse<ResearchJobResponse>>, ApiError> {
    let pool = &d.db().pool;


    let person = Person::find_by_id(pool, person_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Person not found".into()))?;


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
        .map(|pid| format!(" Project context: {}.", pid))

        .unwrap_or_default();

    let research_prompt = format!(
        "[INTELLIGENCE TASK — delegate to {}] \
         Research the following contact and return ONLY a valid JSON object (no markdown, no preamble): \
         Name: {}, Email: {}, Company: {}, Job Title: {}, Person type: {}. \
         Use web search to find their professional background, social media, and their company's \
         public contact information, website, Google My Business listing, and social media. \
         \
         Return this exact JSON structure: \
         {{ \
           \"summary\": \"1-2 sentence professional profile\", \
           \"social_profiles\": [{{\"platform\": \"linkedin\", \"handle\": \"...\", \"url\": \"...\", \"followers\": 0}}], \
           \"company_description\": \"brief company description\", \
           \"company_website\": \"https://...\", \
           \"company_phone\": \"+1...\", \
           \"company_email\": \"info@...\", \
           \"company_instagram\": \"@handle\", \
           \"company_linkedin\": \"url\", \
           \"company_twitter\": \"@handle\", \
           \"company_facebook\": \"url\", \
           \"gmb_rating\": 4.5, \
           \"gmb_review_count\": 42, \
           \"deal_potential\": \"high\", \
           \"recommended_approach\": \"1 sentence\", \
           \"confidence\": 0.8 \
         }}.{}",

        agent_pref,
        person.full_name,
        person.email.as_deref().unwrap_or("unknown"),
        person.company_name.as_deref().unwrap_or("unknown"),
        person.job_title.as_deref().unwrap_or("unknown"),

        person.person_type,
        project_context
    );

    // Fire async task — Nora orchestrates, Scout/Astra executes
    let pool_clone = pool.clone();
    let project_id = body.project_id;
    // TODO: full_name and use_direct were computed but never used downstream
    // let full_name = person.full_name.clone();
    // let use_direct = body.agent_preference.as_deref() == Some("direct");
    let person_clone = person.clone();

    tokio::spawn(async move {
        let result = run_research_via_nora(
            &pool_clone,
            person_clone,
            research_prompt,
            project_id,
        )
        .await;

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
    person: Person,
    prompt: String,
    project_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let person_id = person.id;
    let full_name = person.full_name.clone();

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
            return run_research_direct(pool, &person, project_id).await;
        }
    };

    // Use Nora agent directly from the locked instance
    let response = {
        let nora_guard = nora_instance.read().await;
        let Some(nora) = nora_guard.as_ref() else {
            drop(nora_guard);
            return run_research_direct(pool, &person, project_id).await;
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
        return run_research_direct(pool, &person, project_id).await;
    }

    // Parse Nora's response and write to person record
    let summary = extract_summary_from_response(&response.content);
    let confidence = extract_confidence_from_response(&response.content);

    write_intelligence_results(pool, person_id, &summary, confidence, &response.content, project_id, &full_name).await?;
    Ok(())
}

/// Fallback: direct Anthropic web search when Nora is not initialized.
/// Uses Scout's persona — social intelligence specialization.
async fn run_research_direct(
    pool: &sqlx::SqlitePool,
    person: &Person,

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
        "Research this contact for PCG: Name={}, Company={}, Email={}, Title={}. \
        Search the web for their LinkedIn, Instagram, and other social profiles. \
        Search for their company's website, Google My Business listing, phone, email, and social media accounts. \
        Return ONLY this JSON (no markdown): \
        {{\"summary\":\"...\", \
        \"social_profiles\":[{{\"platform\":\"linkedin\",\"handle\":\"...\",\"url\":\"...\",\"followers\":0}}], \
        \"company_description\":\"...\", \
        \"company_website\":\"...\", \
        \"company_phone\":\"...\", \
        \"company_email\":\"...\", \
        \"company_instagram\":\"...\", \
        \"company_linkedin\":\"...\", \
        \"company_twitter\":\"...\", \
        \"gmb_rating\":4.5, \
        \"gmb_review_count\":42, \
        \"deal_potential\":\"high\", \
        \"recommended_approach\":\"...\", \
        \"confidence\":0.8}}",
        person.full_name,
        person.company_name.as_deref().unwrap_or("unknown"),
        person.email.as_deref().unwrap_or("unknown"),
        person.job_title.as_deref().unwrap_or("unknown"),

    );

    let body = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 2048,

        "system": system,
        "tools": [{
            "type": "web_search_20250305",
            "name": "web_search",
            "max_uses": 5

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

    write_intelligence_results(pool, person.id, &summary, confidence, &response_text, project_id, &person.full_name).await?;
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1. Update person intelligence fields

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
        let source_id = person_id.to_string();
        let source_summary = Some(format!("Scout intelligence: {}", summary));

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
    // Try JSON block — look for "summary" anywhere in the JSON
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                    if let Some(s) = v.get("summary").and_then(|s| s.as_str()) {
                        return s.to_string();
                    }
                }
            }
        }
    }
    // Fall back to first 300 chars of text
    text.chars().take(300).collect()
}

fn extract_text_from_anthropic_response(response: &serde_json::Value) -> String {
    if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
        let mut parts = Vec::new();
        for block in content {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    parts.push(text.to_string());
                }
            }
        }
        if !parts.is_empty() {
            return parts.join("\n");
        }
    }
    // Check for error
    if let Some(err) = response.get("error") {
        return format!("API error: {}", err);
    }
    response.to_string()
}

fn extract_confidence_from_response(text: &str) -> f64 {
    // Try to parse JSON and extract confidence field
    let json_str = if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        &text[start..=end]
    } else {
        text
    };
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(c) = v.get("confidence").and_then(|c| c.as_f64()) {
            return c.clamp(0.0, 1.0);
        }
    }
    // Heuristic: longer responses tend to be more confident
    if text.len() > 500 { 0.7 } else if text.len() > 200 { 0.5 } else { 0.3 }
}

fn parse_research_json(text: &str) -> serde_json::Value {
    // Try direct parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return v;
    }
    // Try extracting JSON block
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                    return v;
                }
            }
        }
    }
    serde_json::Value::Object(Default::default())
}

async fn update_company_contact_methods(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    parsed: &serde_json::Value,
) {
    let contact_keys: &[(&str, &str, &str)] = &[
        ("company_phone", "phone", "Phone"),
        ("company_email", "email", "Email"),
        ("company_website", "website", "Website"),
        ("phone", "phone", "Phone"),
        ("email", "email", "Email"),
        ("website", "website", "Website"),
    ];
    for (json_key, method_type, label) in contact_keys {
        if let Some(val) = parsed.get(*json_key).and_then(|v| v.as_str()) {
            if !val.trim().is_empty() {
                let _ = sqlx::query(
                    "INSERT INTO company_contact_methods (id, company_id, method_type, label, value) \
                     VALUES (randomblob(16), ?, ?, ?, ?) ON CONFLICT DO NOTHING",
                )
                .bind(company_id)
                .bind(*method_type)
                .bind(*label)
                .bind(val.trim())
                .execute(pool)
                .await;
            }
        }
    }
}

// ── Company Intelligence ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CompanyResearchRequest {
    pub agent_preference: Option<String>,
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CompanyResearchJobResponse {
    pub company_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CompanyIntelligenceStatusResponse {
    pub company_id: Uuid,
    pub status: String,
    pub summary: Option<String>,
    pub confidence: f64,
    pub agent: Option<String>,
    pub last_run_at: Option<String>,
}

/// POST /api/companies/:id/research
pub async fn trigger_company_research(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<Uuid>,
    Json(body): Json<CompanyResearchRequest>,
) -> Result<Json<ApiResponse<CompanyResearchJobResponse>>, ApiError> {
    use db::models::company::Company;
    let pool = d.db().pool.clone();
    let company_db_id = DbUuid::from(company_id);

    let company = Company::find_by_id(&pool, &company_db_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;

    sqlx::query(
        "UPDATE companies SET intelligence_status = 'queued', updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(company_id)
    .execute(&pool)
    .await?;

    let company_name = company.name.clone();
    let project_id = body.project_id;
    let pool2 = pool.clone();

    tokio::spawn(async move {
        run_company_research(&pool2, company_id, &company_name, project_id).await;
    });

    Ok(Json(ApiResponse::success(CompanyResearchJobResponse {
        company_id,
        status: "queued".into(),
        message: format!("Research queued for {} — Astra will gather company intelligence.", company.name),
    })))
}

/// GET /api/companies/:id/intelligence-status
pub async fn get_company_intelligence_status(
    State(d): State<DeploymentImpl>,
    Path(company_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CompanyIntelligenceStatusResponse>>, ApiError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        intelligence_status: String,
        intelligence_summary: Option<String>,
        intelligence_confidence: Option<f64>,
        intelligence_agent: Option<String>,
        intelligence_last_run_at: Option<String>,
    }
    let pool = &d.db().pool;
    let row: Option<Row> = sqlx::query_as(
        "SELECT intelligence_status, intelligence_summary, intelligence_confidence, \
         intelligence_agent, intelligence_last_run_at FROM companies WHERE id = ?",
    )
    .bind(company_id)
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or_else(|| ApiError::NotFound("Company not found".into()))?;
    Ok(Json(ApiResponse::success(CompanyIntelligenceStatusResponse {
        company_id,
        status: row.intelligence_status,
        summary: row.intelligence_summary,
        confidence: row.intelligence_confidence.unwrap_or(0.0),
        agent: row.intelligence_agent,
        last_run_at: row.intelligence_last_run_at,
    })))
}

async fn run_company_research(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    company_name: &str,
    project_id: Option<Uuid>,
) {
    sqlx::query(
        "UPDATE companies SET intelligence_status = 'running', updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(company_id)
    .execute(pool)
    .await
    .ok();

    run_company_research_direct(pool, company_id, company_name, project_id).await;
}

pub async fn run_company_research_direct(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    company_name: &str,
    project_id: Option<Uuid>,
) {
    let client = reqwest::Client::new();
    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) => k,
        Err(_) => {
            write_company_intel_results(pool, company_id, "No API key configured", 0.0).await;
            return;
        }
    };

    let prompt = format!(
        "Research the company '{}'. Search the web for: \
        1) Company overview and background, \
        2) Website and contact information (phone, email, address), \
        3) Social media presence (Instagram, LinkedIn, Twitter, Facebook handles/URLs), \
        4) Google My Business listing (rating, number of reviews, address), \
        5) Leadership and key personnel, \
        6) Business model and services/products, \
        7) Recent news or developments, \
        8) Market opportunity and positioning. \
        Return ONLY a valid JSON object with keys: \
        {{\"summary\":\"...\", \"description\":\"...\", \"website\":\"...\", \
        \"phone\":\"...\", \"email\":\"...\", \"address\":\"...\", \
        \"instagram\":\"...\", \"linkedin\":\"...\", \"twitter\":\"...\", \"facebook\":\"...\", \
        \"gmb_rating\":4.5, \"gmb_review_count\":42, \
        \"industry\":\"...\", \"key_personnel\":[\"...\"], \
        \"business_model\":\"...\", \"market_opportunity\":\"...\", \
        \"recent_news\":[\"...\"], \"confidence\":0.8}}",
        company_name
    );

    let body = serde_json::json!({
        "model": "claude-opus-4-6",
        "max_tokens": 3000,
        "tools": [{"type": "web_search_20250305", "name": "web_search", "max_uses": 6}],
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "web-search-2025-03-05")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await;

    let (summary, parsed, confidence) = match resp {
        Ok(r) if r.status().is_success() => {
            let val: serde_json::Value = r.json().await.unwrap_or_default();
            let text = extract_text_from_anthropic_response(&val);
            let parsed = parse_research_json(&text);
            let conf = parsed.get("confidence").and_then(|c| c.as_f64()).unwrap_or_else(|| {
                if text.len() > 300 { 0.75 } else { 0.3 }
            }).clamp(0.0, 1.0);
            let summary = parsed.get("summary").and_then(|s| s.as_str()).unwrap_or(&text[..text.len().min(600)]).to_string();
            (summary, parsed, conf)
        }
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            tracing::error!("Company research API error {}: {}", status, &text[..text.len().min(200)]);
            (format!("Research unavailable for {}", company_name), serde_json::Value::Object(Default::default()), 0.1)
        }
        Err(e) => {
            tracing::error!("Company research HTTP error: {}", e);
            (format!("Research unavailable for {}", company_name), serde_json::Value::Object(Default::default()), 0.1)
        }
    };

    // Update company record with found data
    let website = parsed.get("website").and_then(|v| v.as_str()).unwrap_or_default();
    let description = parsed.get("description").and_then(|v| v.as_str()).unwrap_or_default();
    let industry = parsed.get("industry").and_then(|v| v.as_str()).unwrap_or_default();

    let _ = sqlx::query(
        "UPDATE companies SET \
         intelligence_status = 'done', \
         intelligence_summary = ?, \
         intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         intelligence_agent = 'astra', \
         website = COALESCE(NULLIF(?, ''), website), \
         description = COALESCE(NULLIF(?, ''), description), \
         industry = COALESCE(NULLIF(?, ''), industry), \
         updated_at = datetime('now','subsec') \
         WHERE id = ?",
    )
    .bind(&summary)
    .bind(confidence)
    .bind(website)
    .bind(description)
    .bind(industry)
    .bind(company_id)
    .execute(pool)
    .await;

    // Add contact methods found in research
    update_company_contact_methods(pool, company_id, &parsed).await;

    // Add social profiles as contact methods
    let social_keys: &[(&str, &str)] = &[
        ("instagram", "Instagram"),
        ("linkedin",  "LinkedIn"),
        ("twitter",   "Twitter"),
        ("facebook",  "Facebook"),
    ];
    for (key, label) in social_keys {
        if let Some(val) = parsed.get(*key).and_then(|v| v.as_str()) {
            if !val.trim().is_empty() {
                let _ = sqlx::query(
                    "INSERT INTO company_contact_methods (id, company_id, method_type, label, value) \
                     VALUES (randomblob(16), ?, ?, ?, ?) ON CONFLICT DO NOTHING",
                )
                .bind(company_id)
                .bind(*key)
                .bind(*label)
                .bind(val.trim())
                .execute(pool)
                .await;
            }
        }
    }

    // Register in knowledge graph if project_id provided
    if let Some(pid) = project_id {
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            pid,
            &KnowledgeSourceType::Entity,
            &company_id.to_string(),
            &format!("Company: {}", company_name),
            Some(&summary),
            confidence,
        )
        .await;
    }

    tracing::info!("Company research complete for {} (confidence: {:.0}%)", company_name, confidence * 100.0);
}

async fn write_company_intel_results(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    summary: &str,
    confidence: f64,
) {
    let _ = sqlx::query(
        "UPDATE companies SET \
         intelligence_status = 'done', \
         intelligence_summary = ?, \
         intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         updated_at = datetime('now','subsec') \
         WHERE id = ?",
    )
    .bind(summary)
    .bind(confidence)
    .bind(company_id)
    .execute(pool)
    .await;
}


// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/persons/{id}/research", post(trigger_research))
        .route("/persons/{id}/intelligence-status", get(get_intelligence_status))
        .route("/companies/{id}/intelligence-status", get(get_company_intelligence_status))

        .with_state(deployment.clone())
}

// ── Iterative research pass endpoints ────────────────────────────────────────

use db::models::person_research_pass::PersonResearchPass;
use db::models::business_report::BusinessReport;

#[derive(Debug, Deserialize)]
pub struct NextPassRequest {
    /// Override the auto-selected focus: 'identity' | 'market_position' | 'competitors' | 'target_clients' | 'deep_strategy' | 'custom'
    pub focus: Option<String>,
    pub custom_prompt: Option<String>,
    pub project_id: Option<Uuid>,
}

/// GET /api/persons/:id/research-passes — list all research passes in order
pub async fn list_research_passes(
    State(d): State<DeploymentImpl>,
    Path(person_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<PersonResearchPass>>>, ApiError> {
    let passes = PersonResearchPass::list_for_person(&d.db().pool, person_id).await?;
    Ok(Json(ApiResponse::success(passes)))
}

/// POST /api/persons/:id/research-passes/next
/// Triggers the next logical research pass, building on all prior passes.
pub async fn trigger_next_research_pass(
    State(d): State<DeploymentImpl>,
    Path(person_id): Path<Uuid>,
    Json(body): Json<NextPassRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    let pool = &d.db().pool;

    let person = Person::find_by_id(pool, person_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Person not found".into()))?;

    // Determine next pass number and auto-select focus
    let pass_number = PersonResearchPass::next_pass_number(pool, person_id).await;
    let focus = body.focus.clone().unwrap_or_else(|| auto_focus(pass_number));

    // Collect prior pass summaries for context
    let prior_passes = PersonResearchPass::list_for_person(pool, person_id).await?;
    let prior_context: String = prior_passes.iter()
        .filter(|p| p.status == "done")
        .map(|p| format!(
            "Pass {} ({}): {}",
            p.pass_number,
            p.research_focus,
            p.summary.as_deref().unwrap_or("(no summary)")
        ))
        .collect::<Vec<_>>()
        .join("\n\n");

    let pass = PersonResearchPass::create(pool, person_id, pass_number, &focus, body.custom_prompt.as_deref()).await?;
    let pass_id = pass.id;

    // Build research prompt incorporating all prior context
    let intel = person.intelligence_summary.as_deref().unwrap_or("").to_string();
    let name = person.full_name.clone();
    let company = person.company_name.clone().unwrap_or_else(|| "Unknown company".into());
    let project_id = body.project_id;
    let pool_clone = pool.clone();
    let pool_for_err = pool.clone();
    let focus_for_resp = focus.clone();

    tokio::spawn(async move {
        if let Err(e) = run_research_pass(
            pool_clone, pass_id, person_id, pass_number,
            &focus, &name, &company, &intel, &prior_context, project_id,
        ).await {
            tracing::error!("Research pass failed for {}: {}", pass_id, e);
            let _ = sqlx::query(
                "UPDATE person_research_passes SET status = 'failed', error = ?, completed_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(e.to_string())
            .bind(pass_id)
            .execute(&pool_for_err)
            .await;
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({
        "pass_id": pass_id,
        "pass_number": pass_number,
        "focus": focus_for_resp,
        "status": "queued"
    }))))
}

/// GET /api/persons/:id/reports
pub async fn list_person_reports(
    State(d): State<DeploymentImpl>,
    Path(person_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<BusinessReport>>>, ApiError> {
    let reports = BusinessReport::list_by_person(&d.db().pool, person_id).await?;
    Ok(Json(ApiResponse::success(reports)))
}

fn auto_focus(pass_number: i64) -> String {
    match pass_number {
        1 => "identity",
        2 => "market_position",
        3 => "competitors",
        4 => "target_clients",
        _ => "deep_strategy",
    }
    .to_string()
}

async fn run_research_pass(
    pool: sqlx::SqlitePool,
    pass_id: Uuid,
    person_id: Uuid,
    pass_number: i64,
    focus: &str,
    name: &str,
    company: &str,
    existing_intel: &str,
    prior_context: &str,
    project_id: Option<Uuid>,
) -> anyhow::Result<()> {
    use reqwest::Client;
    use serde_json::Value;

    // Mark running
    sqlx::query(
        "UPDATE person_research_passes SET status = 'running' WHERE id = ?",
    )
    .bind(pass_id)
    .execute(&pool)
    .await?;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

    let focus_instructions = match focus {
        "identity" => format!(
            "Research who {} at {} is: professional background, career history, social presence, \
             personal brand, public statements, media appearances.",
            name, company
        ),
        "market_position" => format!(
            "Research {}'s market position and business model: revenue, growth stage, \
             funding, industry standing, key partnerships, recent news.",
            company
        ),
        "competitors" => format!(
            "Identify {}'s top 5-8 direct competitors. For each: name, positioning, \
             strengths/weaknesses, how {} differentiates from them.",
            company, company
        ),
        "target_clients" => format!(
            "Research who {}'s ideal and current clients/customers are: industries, \
             company sizes, demographics, key use cases, testimonials or case studies.",
            company
        ),
        "deep_strategy" => format!(
            "Deep strategic analysis of {}: growth strategy, product roadmap signals, \
             hiring patterns, content strategy, event participation, PCG engagement opportunity.",
            company
        ),
        _ => format!("Research: {}", focus),
    };

    let prior_section = if prior_context.is_empty() {
        String::new()
    } else {
        format!("\n\nPRIOR RESEARCH (build on this, don't repeat):\n{}", prior_context)
    };

    let existing_section = if existing_intel.is_empty() {
        String::new()
    } else {
        format!("\n\nEXISTING INTELLIGENCE:\n{}", &existing_intel[..existing_intel.len().min(2000)])
    };

    let system = "You are an expert business intelligence researcher. Conduct thorough web research \
        and return structured findings. Always respond with valid JSON only.";

    let prompt = format!(
        "RESEARCH TASK — Pass #{pass_number} | Focus: {focus}\n\n\
        Subject: {name} ({company})\n\
        {existing_section}{prior_section}\n\n\
        FOCUS FOR THIS PASS:\n{focus_instructions}\n\n\
        Return JSON:\n\
        {{\n\
          \"summary\": \"3-5 sentence summary of findings for this pass\",\n\
          \"key_findings\": [\n\
            {{\"finding\": \"string\", \"confidence\": \"high|medium|low\", \"source\": \"where found or inferred\"}}\n\
          ],\n\
          \"search_queries\": [\"queries you would use to verify this\"],\n\
          \"updated_intelligence\": \"comprehensive updated intelligence summary incorporating all passes\",\n\
          \"confidence_score\": 0.0-1.0\n\
        }}"
    );

    let client = Client::new();
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "web-search-2025-03-05")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 4096,
            "system": system,
            "tools": [{"type": "web_search_20250305", "name": "web_search", "max_uses": 5}],
            "messages": [{"role": "user", "content": prompt}]
        }))
        .send()
        .await?;

    let body: Value = res.json().await?;

    // Extract text from response (may be in content array)
    let text = body["content"]
        .as_array()
        .and_then(|arr| arr.iter().find(|b| b["type"] == "text"))
        .and_then(|b| b["text"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No text in response: {:?}", body))?;

    let json_str = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let parsed: Value = serde_json::from_str(json_str)
        .map_err(|e| anyhow::anyhow!("JSON parse error: {e}\nRaw: {}", &json_str[..json_str.len().min(300)]))?;

    let summary = parsed["summary"].as_str().unwrap_or("").to_string();
    let updated_intel = parsed["updated_intelligence"].as_str().unwrap_or("").to_string();
    let confidence_score = parsed["confidence_score"].as_f64().unwrap_or(0.5);
    let key_findings = serde_json::to_string(&parsed["key_findings"]).unwrap_or_else(|_| "[]".into());
    let search_queries = serde_json::to_string(&parsed["search_queries"]).unwrap_or_else(|_| "[]".into());

    // Save pass results
    sqlx::query(
        "UPDATE person_research_passes SET
            status = 'done',
            summary = ?,
            raw_results = ?,
            key_findings = ?,
            search_queries = ?,
            confidence_delta = ?,
            agent_used = 'claude',
            completed_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(&summary)
    .bind(json_str)
    .bind(&key_findings)
    .bind(&search_queries)
    .bind(confidence_score)
    .bind(pass_id)
    .execute(&pool)
    .await?;

    // Update persons table with accumulated intelligence
    if !updated_intel.is_empty() {
        let new_confidence = (confidence_score as f64).min(1.0);
        let depth = match pass_number {
            1 => "shallow",
            2 | 3 => "moderate",
            _ => "deep",
        };
        sqlx::query(
            "UPDATE persons SET
                intelligence_summary = ?,
                intelligence_status = 'done',
                intelligence_last_run_at = datetime('now','subsec'),
                intelligence_confidence = ?,
                intelligence_agent = 'claude',
                research_pass_count = ?,
                research_depth = ?,
                updated_at = datetime('now','subsec')
             WHERE id = ?",
        )
        .bind(&updated_intel)
        .bind(new_confidence)
        .bind(pass_number)
        .bind(depth)
        .bind(person_id)
        .execute(&pool)
        .await?;
    }

    // Register each key finding in the project knowledge graph
    if let Some(pid) = project_id {
        let title = format!("Research Pass #{}: {} — {}", pass_number, focus, &name[..name.len().min(40)]);
        let _ = ProjectKnowledgeSource::upsert_source(
            &pool,
            pid,
            &KnowledgeSourceType::Entity,
            &pass_id.to_string(),
            &title,
            Some(&summary),
            confidence_score,
        )
        .await;
    }

    // Also register in the org-level knowledge graph (owner_type='organization')
    // Find org from person's associations
    #[derive(sqlx::FromRow)]
    struct OrgRow { org_id: Option<Uuid> }
    if let Ok(Some(row)) = sqlx::query_as::<_, OrgRow>(
        "SELECT poc.organization_id AS org_id FROM person_org_contacts poc WHERE poc.person_id = ? LIMIT 1",
    )
    .bind(person_id)
    .fetch_optional(&pool)
    .await {
        if let Some(org_id) = row.org_id {
            let title = format!("[Org KG] Research Pass #{}: {} — {}", pass_number, focus, &name[..name.len().min(40)]);
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO project_knowledge_sources
                 (id, owner_type, owner_id, source_type, source_id, source_title, source_summary,
                  coverage_score, auto_registered, is_active, created_at, updated_at)
                 VALUES (?, 'organization', ?, 'entity', ?, ?, ?, ?, 1, 1, datetime('now','subsec'), datetime('now','subsec'))",
            )
            .bind(Uuid::new_v4())
            .bind(org_id.to_string())
            .bind(pass_id.to_string())
            .bind(&title)
            .bind(&summary)
            .bind(confidence_score)
            .execute(&pool)
            .await;
        }
    }

    tracing::info!("Research pass #{} complete for person {} ({})", pass_number, person_id, focus);
    Ok(())
}

