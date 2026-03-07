//! Contact Intelligence — person research pipeline.
//!
//! Architecture:
//!   1. POST /api/persons/:id/research  → Nora receives the orchestration request
//!   2. Nora delegates to Scout (Social Intelligence) or Astra (Strategy Research)
//!   3. Agent performs web research via its tool suite (incl. Exa neural search)
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

use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ResearchRequest {
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

    let pool_clone = pool.clone();
    let project_id = body.project_id;
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

    let nora_instance = match get_nora_instance().await {
        Ok(n) => n,
        Err(_) => {
            return run_research_direct(pool, &person, project_id).await;
        }
    };

    let response_text = {
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

        match tokio::time::timeout(
            std::time::Duration::from_secs(90),
            nora.process_request(nora_request),
        )
        .await
        {
            Ok(Ok(r)) => r.content,
            Ok(Err(e)) => {
                tracing::warn!("Nora error during person research ({}), using direct fallback: {}", person_id, e);
                drop(nora_guard);
                return run_research_direct(pool, &person, project_id).await;
            }
            Err(_) => {
                tracing::warn!("Nora timed out during person research ({}), using direct fallback", person_id);
                drop(nora_guard);
                return run_research_direct(pool, &person, project_id).await;
            }
        }
    };

    let parsed = parse_research_json(&response_text);
    let summary = parsed.get("summary").and_then(|s| s.as_str()).unwrap_or(&response_text[..response_text.len().min(500)]).to_string();
    let confidence = parsed.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.6).clamp(0.0, 1.0);

    write_intelligence_results(pool, &person, &parsed, &summary, confidence, &response_text, project_id).await?;
    Ok(())
}

/// Fallback: direct Anthropic API call with web search when Nora is unavailable or times out.
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
        Research contacts by searching for their professional background, social profiles, \
        and their company's public presence (website, Google My Business, social media, contact info). \
        Return ONLY a valid JSON object with no markdown or preamble.";

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

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error {}: {}", status, text).into());
    }

    let response: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parse: {}", e))?;
    let response_text = extract_text_from_anthropic_response(&response);

    let parsed = parse_research_json(&response_text);
    let summary = parsed.get("summary").and_then(|s| s.as_str()).unwrap_or(&response_text[..response_text.len().min(500)]).to_string();
    let confidence = parsed.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.6).clamp(0.0, 1.0);

    write_intelligence_results(pool, person, &parsed, &summary, confidence, &response_text, project_id).await?;
    Ok(())
}

// ── Post-research pipeline ────────────────────────────────────────────────────

async fn write_intelligence_results(
    pool: &sqlx::SqlitePool,
    person: &Person,
    parsed: &serde_json::Value,
    summary: &str,
    confidence: f64,
    raw: &str,
    project_id: Option<Uuid>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let person_id = person.id;

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

    // 2. Upsert social profiles found in research
    if let Some(profiles) = parsed.get("social_profiles").and_then(|p| p.as_array()) {
        for profile in profiles {
            let platform = profile.get("platform").and_then(|p| p.as_str()).unwrap_or_default();
            let handle = profile.get("handle").and_then(|h| h.as_str());
            let url = profile.get("url").and_then(|u| u.as_str());
            let followers = profile.get("followers").and_then(|f| f.as_i64());

            if !platform.is_empty() && (handle.is_some() || url.is_some()) {
                let _ = sqlx::query(
                    "INSERT INTO person_social_profiles \
                     (id, person_id, platform, handle, profile_url, follower_count, last_synced_at) \
                     VALUES (randomblob(16), ?, ?, ?, ?, ?, datetime('now','subsec')) \
                     ON CONFLICT(person_id, platform) DO UPDATE SET \
                       handle = excluded.handle, \
                       profile_url = excluded.profile_url, \
                       follower_count = excluded.follower_count, \
                       last_synced_at = excluded.last_synced_at, \
                       updated_at = datetime('now','subsec')",
                )
                .bind(person_id)
                .bind(platform)
                .bind(handle)
                .bind(url)
                .bind(followers)
                .execute(pool)
                .await;
            }
        }
    }

    // 3. Find or create the company, then update with research data
    let company_id = find_or_create_company_from_research(pool, person, parsed).await;

    // 4. Add person → company junction entry if company found
    if let Some(cid) = company_id {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO person_company_roles \
             (id, person_id, company_id, role, title, is_primary) \
             VALUES (randomblob(16), ?, ?, 'contact', ?, 1)",
        )
        .bind(person_id)
        .bind(cid)
        .bind(person.job_title.as_deref())
        .execute(pool)
        .await;

        // Add company contact methods found in research
        update_company_contact_methods(pool, cid, parsed).await;
    }

    // 5. Create CRM contact + deal in acquisition pipeline Lead stage
    ensure_crm_deal_in_pipeline(pool, person, company_id, parsed, project_id).await;

    // 5b. Also ensure proposal exists (separate proposals board)
    ensure_proposal_in_pipeline(pool, person, company_id, parsed, project_id).await;

    // 6. Register in knowledge graph
    if let Some(pid) = project_id {
        let source_id = person_id.to_string();
        let source_summary = Some(format!("Scout intelligence: {}", summary));
        let _ = ProjectKnowledgeSource::upsert_source(
            pool,
            pid,
            &KnowledgeSourceType::Entity,
            &source_id,
            &format!("Person: {}", person.full_name),
            source_summary.as_deref(),
            confidence,
        )
        .await;

        // 7. Create tracking task on project board
        create_research_task(pool, person, pid, summary, parsed).await;

        // 8. Generate business analysis file
        let _ = generate_business_analysis(pool, person, company_id, parsed, summary, pid).await;
    }

    tracing::info!(
        "Research complete for {} (confidence: {:.0}%){} proposal+task+artifacts created",
        person.full_name,
        confidence * 100.0,
        if project_id.is_some() { "," } else { " — no project_id, skipping" }
    );

    Ok(())
}

/// Find company by matching name, or create it if a company_website/description was found
async fn find_or_create_company_from_research(
    pool: &sqlx::SqlitePool,
    person: &Person,
    parsed: &serde_json::Value,
) -> Option<Uuid> {
    let company_name = person.company_name.as_deref().unwrap_or_default();
    if company_name.is_empty() {
        return None;
    }

    // 1. Try matching by name
    #[derive(sqlx::FromRow)]
    struct Row { id: Uuid }
    let existing: Option<Row> = sqlx::query_as(
        "SELECT id FROM companies WHERE lower(name) = lower(?)"
    )
    .bind(company_name)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some(row) = existing {
        // Update with any new data from research
        let website = parsed.get("company_website").and_then(|v| v.as_str()).unwrap_or_default();
        let description = parsed.get("company_description").and_then(|v| v.as_str()).unwrap_or_default();
        if !website.is_empty() || !description.is_empty() {
            let _ = sqlx::query(
                "UPDATE companies SET \
                 website = COALESCE(NULLIF(?, ''), website), \
                 description = COALESCE(NULLIF(?, ''), description), \
                 updated_at = datetime('now','subsec') \
                 WHERE id = ?",
            )
            .bind(website)
            .bind(description)
            .bind(row.id)
            .execute(pool)
            .await;
        }
        return Some(row.id);
    }

    // 2. Create company if we have meaningful data
    let website = parsed.get("company_website").and_then(|v| v.as_str()).unwrap_or_default();
    let description = parsed.get("company_description").and_then(|v| v.as_str()).unwrap_or_default();

    if description.len() > 20 || !website.is_empty() {
        let new_id = Uuid::new_v4();
        let slug = company_name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
            .collect::<String>();
        let slug = slug.trim_matches('-').to_string();

        let result = sqlx::query(
            "INSERT OR IGNORE INTO companies (id, name, slug, website, description) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(new_id)
        .bind(company_name)
        .bind(&slug)
        .bind(if website.is_empty() { None } else { Some(website) })
        .bind(if description.is_empty() { None } else { Some(description) })
        .execute(pool)
        .await;

        if result.is_ok() {
            tracing::info!("Created company '{}' from research", company_name);
            return Some(new_id);
        }
    }

    None
}

/// Upsert contact methods found in research to company_contact_methods
async fn update_company_contact_methods(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    parsed: &serde_json::Value,
) {
    let methods: &[(&str, &str, &str)] = &[
        ("phone",    "company_phone",     "General"),
        ("email",    "company_email",     "General"),
        ("website",  "company_website",   "Website"),
        ("instagram","company_instagram", "Instagram"),
        ("linkedin", "company_linkedin",  "LinkedIn"),
        ("twitter",  "company_twitter",   "Twitter"),
        ("facebook", "company_facebook",  "Facebook"),
    ];

    for (method_type, json_key, label) in methods {
        if let Some(value) = parsed.get(*json_key).and_then(|v| v.as_str()) {
            let value = value.trim();
            if value.is_empty() || value == "null" || value == "unknown" {
                continue;
            }
            let _ = sqlx::query(
                "INSERT INTO company_contact_methods (id, company_id, method_type, label, value) \
                 VALUES (randomblob(16), ?, ?, ?, ?) \
                 ON CONFLICT DO NOTHING",
            )
            .bind(company_id)
            .bind(method_type)
            .bind(label)
            .bind(value)
            .execute(pool)
            .await;
        }
    }
}

/// Create CRM contact + deal in the org's Acquisition pipeline "Lead" stage
async fn ensure_crm_deal_in_pipeline(
    pool: &sqlx::SqlitePool,
    person: &Person,
    company_id: Option<Uuid>,
    parsed: &serde_json::Value,
    project_id: Option<Uuid>,
) {
    let org_id = match person.organization_id {
        Some(id) => id,
        None => return,
    };

    // 1. Find or create crm_contact for this person
    #[derive(sqlx::FromRow)]
    struct IdRow { id: Uuid }

    let contact_id = {
        // First try to find by person_id
        let existing: Option<IdRow> = sqlx::query_as(
            "SELECT id FROM crm_contacts WHERE person_id = ? LIMIT 1"
        )
        .bind(person.id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        if let Some(row) = existing {
            row.id
        } else {
            // Create a new crm_contact
            let new_id = Uuid::new_v4();
            let company_name = person.company_name.as_deref()
                .or_else(|| company_id.map(|_| "")).unwrap_or_default();
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO crm_contacts \
                 (id, organization_id, full_name, email, company_name, job_title, person_id) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(new_id)
            .bind(org_id)
            .bind(&person.full_name)
            .bind(person.email.as_deref())
            .bind(company_name)
            .bind(person.job_title.as_deref())
            .bind(person.id)
            .execute(pool)
            .await;
            new_id
        }
    };

    // 2. Find the org's Acquisition pipeline
    let pipeline: Option<IdRow> = sqlx::query_as(
        "SELECT id FROM crm_pipelines \
         WHERE organization_id = ? AND pipeline_type = 'acquisition' \
         ORDER BY created_at ASC LIMIT 1"
    )
    .bind(org_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let Some(pipeline_row) = pipeline else { return };

    // 3. Get the first stage (Lead / position 0)
    let stage: Option<IdRow> = sqlx::query_as(
        "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? ORDER BY position ASC LIMIT 1"
    )
    .bind(pipeline_row.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    let Some(stage_row) = stage else { return };

    // 4. Check if deal already exists for this contact in this pipeline
    let existing_deal: Option<IdRow> = sqlx::query_as(
        "SELECT id FROM crm_deals WHERE crm_contact_id = ? AND crm_pipeline_id = ? LIMIT 1"
    )
    .bind(contact_id)
    .bind(pipeline_row.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if existing_deal.is_some() {
        return;
    }

    // 5. Calculate position in Lead stage
    let position: i32 = sqlx::query_as::<_, (i32,)>(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM crm_deals WHERE crm_stage_id = ?"
    )
    .bind(stage_row.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|(p,)| p)
    .unwrap_or(0);

    let deal_potential = parsed.get("deal_potential").and_then(|v| v.as_str()).unwrap_or("medium");
    let company_name = person.company_name.as_deref().unwrap_or("Unknown");
    let deal_name = format!("{} @ {}", person.full_name, company_name);
    let description = format!(
        "Research-identified lead. Deal potential: {}.",
        deal_potential
    );

    // 6. Insert the crm_deal
    let _ = sqlx::query(
        "INSERT INTO crm_deals \
         (id, organization_id, project_id, crm_contact_id, crm_pipeline_id, crm_stage_id, \
          position, name, description, stage, amount) \
         VALUES (randomblob(16), ?, ?, ?, ?, ?, ?, ?, ?, 'Lead', ?)",
    )
    .bind(org_id)
    .bind(project_id)
    .bind(contact_id)
    .bind(pipeline_row.id)
    .bind(stage_row.id)
    .bind(position)
    .bind(&deal_name)
    .bind(&description)
    .bind(company_id.map(|_| {
        parsed.get("estimated_value_usd").and_then(|v| v.as_f64()).unwrap_or(0.0)
    }))
    .execute(pool)
    .await;

    tracing::info!("Created CRM deal '{}' in Lead stage for {}", deal_name, person.full_name);
}

/// Create or find a proposal in the pipeline for this lead
async fn ensure_proposal_in_pipeline(
    pool: &sqlx::SqlitePool,
    person: &Person,
    company_id: Option<Uuid>,
    parsed: &serde_json::Value,
    project_id: Option<Uuid>,
) {
    // Check if a proposal already exists for this person
    let existing: Option<(Vec<u8>,)> = sqlx::query_as(
        "SELECT id FROM proposals WHERE lead_id = ? LIMIT 1"
    )
    .bind(person.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if existing.is_some() {
        return; // Already in pipeline
    }

    let deal_potential = parsed.get("deal_potential").and_then(|v| v.as_str()).unwrap_or("medium");
    let approach = parsed.get("recommended_approach").and_then(|v| v.as_str()).unwrap_or("");
    let company_name = person.company_name.as_deref().unwrap_or("Unknown Company");

    let title = format!("Business Opportunity — {} / {}", person.full_name, company_name);
    let description = format!(
        "Research-identified opportunity. Deal potential: {}. {}\n\nContact: {}{}",
        deal_potential,
        approach,
        person.full_name,
        person.email.as_ref().map(|e| format!(" <{}>", e)).unwrap_or_default(),
    );

    let proposal_id = Uuid::new_v4();
    let contact_ids = serde_json::json!([person.id.to_string()]);
    let _ = sqlx::query(
        "INSERT INTO proposals \
         (id, lead_id, company_id, project_id, organization_id, title, description, \
          deal_type, quote_amount_vibe, contact_ids) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'one-off', 0, ?)",
    )
    .bind(proposal_id)
    .bind(person.id)
    .bind(company_id)
    .bind(project_id)
    .bind(person.organization_id)
    .bind(&title)
    .bind(&description)
    .bind(contact_ids.to_string())
    .execute(pool)
    .await;

    tracing::info!("Created pipeline proposal for {}", person.full_name);
}

/// Create a tracking task on the project board
async fn create_research_task(
    pool: &sqlx::SqlitePool,
    person: &Person,
    project_id: Uuid,
    summary: &str,
    parsed: &serde_json::Value,
) {
    let deal_potential = parsed.get("deal_potential").and_then(|v| v.as_str()).unwrap_or("medium");
    let approach = parsed.get("recommended_approach").and_then(|v| v.as_str()).unwrap_or("");
    let company_name = person.company_name.as_deref().unwrap_or("Unknown Company");

    let title = format!("Follow up: {} @ {}", person.full_name, company_name);
    let description = format!(
        "Scout research complete.\n\nSummary: {}\n\nDeal potential: {}\nRecommended approach: {}\n\nContact: {}{}",
        summary,
        deal_potential,
        approach,
        person.full_name,
        person.email.as_ref().map(|e| format!(" <{}>", e)).unwrap_or_default(),
    );

    let _ = sqlx::query(
        "INSERT INTO tasks \
         (id, project_id, title, description, status, priority, created_by) \
         VALUES (randomblob(16), ?, ?, ?, 'todo', 'high', 'scout')",
    )
    .bind(project_id)
    .bind(&title)
    .bind(&description)
    .execute(pool)
    .await;

    tracing::info!("Created research task for {} on project {}", person.full_name, project_id);
}

/// Generate business analysis markdown and save as ProjectAsset
async fn generate_business_analysis(
    pool: &sqlx::SqlitePool,
    person: &Person,
    company_id: Option<Uuid>,
    parsed: &serde_json::Value,
    summary: &str,
    project_id: Uuid,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let company_name = person.company_name.as_deref().unwrap_or("Unknown Company");
    let deal_potential = parsed.get("deal_potential").and_then(|v| v.as_str()).unwrap_or("unknown");
    let approach = parsed.get("recommended_approach").and_then(|v| v.as_str()).unwrap_or("—");
    let company_desc = parsed.get("company_description").and_then(|v| v.as_str()).unwrap_or("—");
    let company_website = parsed.get("company_website").and_then(|v| v.as_str()).unwrap_or("—");
    let company_phone = parsed.get("company_phone").and_then(|v| v.as_str()).unwrap_or("—");
    let company_email = parsed.get("company_email").and_then(|v| v.as_str()).unwrap_or("—");
    let gmb_rating = parsed.get("gmb_rating").and_then(|v| v.as_f64()).map(|r| format!("{:.1} ⭐", r)).unwrap_or_else(|| "—".into());
    let gmb_reviews = parsed.get("gmb_review_count").and_then(|v| v.as_i64()).map(|r| r.to_string()).unwrap_or_else(|| "—".into());

    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let mut socials_md = String::new();
    if let Some(profiles) = parsed.get("social_profiles").and_then(|p| p.as_array()) {
        for p in profiles {
            let platform = p.get("platform").and_then(|x| x.as_str()).unwrap_or("unknown");
            let handle = p.get("handle").and_then(|x| x.as_str()).unwrap_or("—");
            let url = p.get("url").and_then(|x| x.as_str()).unwrap_or("");
            let followers = p.get("followers").and_then(|x| x.as_i64()).map(|f| format!(" ({} followers)", f)).unwrap_or_default();
            if !url.is_empty() {
                socials_md.push_str(&format!("- **{}**: [@{}]({}){}  \n", platform, handle, url, followers));
            } else {
                socials_md.push_str(&format!("- **{}**: @{}{}  \n", platform, handle, followers));
            }
        }
    }
    if socials_md.is_empty() {
        socials_md = "— No social profiles found  \n".into();
    }

    let company_id_str = company_id.map(|id| id.to_string()).unwrap_or_else(|| "—".into());

    let content = format!(
        "# Business Analysis — {name} / {company}\n\
        *Generated by Scout on {date} for PCG*\n\n\
        ---\n\n\
        ## Contact Profile\n\n\
        | Field | Value |\n\
        |-------|-------|\n\
        | **Name** | {name} |\n\
        | **Title** | {title} |\n\
        | **Company** | {company} |\n\
        | **Email** | {email} |\n\
        | **Phone** | {phone} |\n\
        | **Type** | {ptype} |\n\
        | **Lead Score** | {score} |\n\n\
        ## Intelligence Summary\n\n\
        {summary}\n\n\
        ## Social Profiles\n\n\
        {socials}\n\
        ## Company Intelligence\n\n\
        | Field | Value |\n\
        |-------|-------|\n\
        | **Company** | {company} |\n\
        | **ID** | {cid} |\n\
        | **Website** | {website} |\n\
        | **Phone** | {cphone} |\n\
        | **Email** | {cemail} |\n\
        | **Google Rating** | {gmb_rating} |\n\
        | **Google Reviews** | {gmb_reviews} |\n\n\
        ### Company Description\n\n\
        {company_desc}\n\n\
        ## Pipeline\n\n\
        | Field | Value |\n\
        |-------|-------|\n\
        | **Deal Potential** | {deal_potential} |\n\
        | **Recommended Approach** | {approach} |\n\
        | **Status** | Drafted — awaiting review |\n\n\
        ---\n\
        *This analysis was generated automatically by Scout. Verify before outreach.*\n",
        name = person.full_name,
        company = company_name,
        date = now,
        title = person.job_title.as_deref().unwrap_or("—"),
        email = person.email.as_deref().unwrap_or("—"),
        phone = person.phone.as_deref().unwrap_or("—"),
        ptype = person.person_type,
        score = person.lead_score,
        summary = summary,
        socials = socials_md,
        cid = company_id_str,
        website = company_website,
        cphone = company_phone,
        cemail = company_email,
        gmb_rating = gmb_rating,
        gmb_reviews = gmb_reviews,
        company_desc = company_desc,
        deal_potential = deal_potential,
        approach = approach,
    );

    // Write to dev_assets/artifacts/
    let filename = format!("analysis_{}.md", person.id);
    let artifacts_dir = utils::assets::asset_dir().join("artifacts");
    let _ = std::fs::create_dir_all(&artifacts_dir);
    let file_path = artifacts_dir.join(&filename);
    std::fs::write(&file_path, &content)?;

    // Register as ProjectAsset
    let storage_path = format!("artifacts/{}", filename);
    let byte_size = content.len() as i64;

    let _ = sqlx::query(
        "INSERT OR REPLACE INTO project_assets \
         (id, project_id, category, scope, name, storage_path, mime_type, byte_size, uploaded_by) \
         VALUES (randomblob(16), ?, 'intelligence', 'crm', ?, ?, 'text/markdown', ?, 'scout')",
    )
    .bind(project_id)
    .bind(format!("Business Analysis — {} / {}", person.full_name, company_name))
    .bind(&storage_path)
    .bind(byte_size)
    .execute(pool)
    .await;

    tracing::info!("Business analysis written to {}", storage_path);
    Ok(())
}

// ── Text / JSON extraction helpers ───────────────────────────────────────────

fn parse_research_json(text: &str) -> serde_json::Value {
    // Try direct parse
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        if v.is_object() {
            return v;
        }
    }
    // Find JSON block (handles markdown code fences or embedded JSON)
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if end > start {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                    if v.is_object() {
                        return v;
                    }
                }
            }
        }
    }
    serde_json::Value::Object(Default::default())
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

    let company = Company::find_by_id(&pool, company_id)
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
        .route("/companies/{id}/research", post(trigger_company_research))
        .route("/companies/{id}/intelligence-status", get(get_company_intelligence_status))
        .with_state(deployment.clone())
}
