use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use db::{
    db_uuid::DbUuid,
    models::{
        company::{Company, CreateCompany, UpdateCompany},
        contact_association::{CompanyContactMethod, CreateCompanyContactMethod},
        proposal::Proposal,
    },
};
use deployment::Deployment;
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{
    error::ApiError, helpers::uuid_params::parse_db_uuid_param, routes::nora::get_nora_instance,
    DeploymentImpl,
};

#[derive(Debug, Deserialize)]
pub struct ListCompaniesQuery {
    pub created_by_org_id: Option<DbUuid>,
    pub has_platform_org: Option<bool>,
    pub limit: Option<i64>,
}

/// GET /companies — list companies (pipeline knowledge graph entities)
async fn list_companies(
    State(deployment): State<DeploymentImpl>,
    Query(q): Query<ListCompaniesQuery>,
) -> Result<Json<ApiResponse<Vec<Company>>>, ApiError> {
    let pool = &deployment.db().pool;
    let companies = Company::list(pool, q.created_by_org_id, q.has_platform_org, q.limit).await?;
    Ok(Json(ApiResponse::success(companies)))
}

/// POST /companies — create company
async fn create_company(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateCompany>,
) -> Result<Json<ApiResponse<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let company = Company::create(pool, body).await?;
    Ok(Json(ApiResponse::success(company)))
}

/// GET /companies/:id — get company
async fn get_company(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    let company = Company::find_by_id(pool, &id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;
    Ok(Json(ApiResponse::success(company)))
}

/// PATCH /companies/:id — update company
async fn update_company(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<UpdateCompany>,
) -> Result<Json<ApiResponse<Company>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    let company = Company::update(pool, &id, body)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;
    Ok(Json(ApiResponse::success(company)))
}

/// DELETE /companies/:id
async fn delete_company(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = DbUuid::from(id);
    let deleted = Company::delete(pool, &id).await?;
    if !deleted {
        return Err(ApiError::NotFound("Company not found".into()));
    }
    Ok(Json(ApiResponse::success(())))
}

#[derive(Debug, Deserialize)]
pub struct ListCompanyProposalsQuery {
    pub limit: Option<i64>,
}

/// GET /companies/:id/proposals — all pipeline proposals for this company
async fn list_company_proposals(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Query(q): Query<ListCompanyProposalsQuery>,
) -> Result<Json<ApiResponse<Vec<Proposal>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "company ID")?;
    let proposals = Proposal::list_by_company(pool, id.to_uuid(), q.limit).await?;
    Ok(Json(ApiResponse::success(proposals)))
}

/// GET /companies/:id/contacts — contacts at this company
async fn list_company_contacts(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<db::models::crm_contact::CrmContact>>>, ApiError> {
    let pool = &deployment.db().pool;
    let contacts = sqlx::query_as::<_, db::models::crm_contact::CrmContact>(
        r#"SELECT * FROM crm_contacts
           WHERE company_id = ?
           ORDER BY full_name ASC"#,
    )
    .bind(&id)
    .fetch_all(pool)
    .await?;
    Ok(Json(ApiResponse::success(contacts)))
}

// ---------------------------------------------------------------------------
// Company contact methods
// ---------------------------------------------------------------------------

/// GET /companies/:id/contact-methods
async fn list_contact_methods(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<CompanyContactMethod>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "company ID")?;
    let methods = CompanyContactMethod::list_for_company(pool, id.to_uuid()).await?;
    Ok(Json(ApiResponse::success(methods)))
}

/// POST /companies/:id/contact-methods
async fn add_contact_method(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(data): Json<CreateCompanyContactMethod>,
) -> Result<Json<ApiResponse<CompanyContactMethod>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "company ID")?;
    let method = CompanyContactMethod::create(pool, id.to_uuid(), data).await?;
    Ok(Json(ApiResponse::success(method)))
}

/// DELETE /companies/:id/contact-methods/:method_id
async fn delete_contact_method(
    State(deployment): State<DeploymentImpl>,
    Path((_id, method_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let method_id = parse_db_uuid_param(&method_id, "contact method ID")?;
    let deleted = CompanyContactMethod::delete(pool, method_id.to_uuid()).await?;
    if !deleted {
        return Err(ApiError::NotFound("Contact method not found".into()));
    }
    Ok(Json(ApiResponse::success(())))
}

// ── Company intelligence research ────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CompanyResearchResponse {
    company_id: Uuid,
    status: String,
    message: String,
}

/// POST /companies/:id/research — trigger Scout to research this company
async fn trigger_company_research(
    State(deployment): State<DeploymentImpl>,
    Path(company_id): Path<String>,
) -> Result<Json<ApiResponse<CompanyResearchResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let db_company_id = parse_db_uuid_param(&company_id, "company ID")?;
    let company_id = db_company_id.to_uuid();
    let company = Company::find_by_id(pool, &db_company_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;

    // Mark as running — use string binding for TEXT-stored IDs
    sqlx::query(
        "UPDATE companies SET intelligence_status = 'running', updated_at = datetime('now','subsec') WHERE CAST(id AS TEXT) = ?",
    )
    .bind(company_id.to_string())
    .execute(pool)
    .await?;

    let name = company.name.clone();
    let pool_clone = pool.clone();

    tokio::spawn(async move {
        // Use direct research with OpenAI-first fallback
        crate::routes::intelligence::run_company_research_direct(
            &pool_clone,
            company_id,
            &name,
            None,
        )
        .await;
    });

    Ok(Json(ApiResponse::success(CompanyResearchResponse {
        company_id,
        status: "running".into(),
        message: format!(
            "Research started for {} — Scout is gathering intelligence.",
            company.name
        ),
    })))
}

// NOTE: Replaced by intelligence::run_company_research_direct() in sloperation317 port.
// Keeping for reference — remove after confirming direct version covers all cases.
#[allow(dead_code)]
async fn run_company_research(
    pool: &sqlx::SqlitePool,
    company_id: Uuid,
    name: &str,
    website: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let prompt = format!(
        "[INTELLIGENCE TASK — delegate to Scout] \
         Research the following company for Power Club Global's CRM: \
         Company: {name}, Website: {website}. \
         Gather: company overview, industry, founding year, founder/CEO, employee count, \
         social media presence (Instagram/LinkedIn/Twitter), recent news, \
         key clients or notable work, brand positioning, and relevance as a PCG client. \
         Return a JSON object with keys: summary (2–4 sentences), \
         industry, founded_year, founder_name, employee_count, \
         social_profiles (array), key_clients (array), brand_positioning, confidence (0.0–1.0).",
        name = name,
        website = website.unwrap_or("unknown"),
    );

    // Try Nora first
    let nora_result = async {
        let nora_instance = get_nora_instance().await.ok()?;
        let nora_guard = nora_instance.read().await;
        let nora = nora_guard.as_ref()?;
        let req = NoraRequest {
            request_id: Uuid::new_v4().to_string(),
            session_id: format!("company-research-{}", company_id),
            request_type: NoraRequestType::TextInteraction,
            content: prompt.clone(),
            context: None,
            voice_enabled: false,
            priority: RequestPriority::High,
            timestamp: chrono::Utc::now(),
        };
        tokio::time::timeout(
            std::time::Duration::from_secs(90),
            nora.process_request(req),
        )
        .await
        .ok()?
        .ok()
    }
    .await;

    let raw = if let Some(resp) = nora_result {
        resp.content
    } else {
        // Direct Anthropic fallback
        run_company_research_direct(name, website).await?
    };

    // Extract summary + confidence
    let summary = extract_summary(&raw);
    let confidence = extract_confidence(&raw);

    sqlx::query(
        "UPDATE companies SET intelligence_status = 'done', \
         intelligence_summary = ?, intelligence_raw = ?, intelligence_confidence = ?, \
         intelligence_last_run_at = datetime('now','subsec'), \
         updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(&summary)
    .bind(&raw)
    .bind(confidence)
    .bind(company_id)
    .execute(pool)
    .await?;

    tracing::info!(
        "Company research complete for {} (confidence: {:.0}%)",
        name,
        confidence * 100.0
    );
    Ok(())
}

async fn run_company_research_direct(
    name: &str,
    website: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use serde_json::json;

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| "ANTHROPIC_API_KEY not set")?;

    let prompt = format!(
        "Research the company '{name}'{website_ctx} for a CRM profile. \
         Find: what they do, industry, founding year, founder/CEO name, \
         approximate employee count, social media accounts, notable clients or work, \
         brand positioning, and any recent news. \
         Return structured JSON with: summary, industry, founded_year, \
         founder_name, employee_count, social_profiles, key_clients, brand_positioning, confidence.",
        name = name,
        website_ctx = website
            .map(|w| format!(" (website: {})", w))
            .unwrap_or_default(),
    );

    let body = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 2048,
        "system": "You are Scout, Brand Intelligence Analyst for Power Club Global. Research companies and return comprehensive JSON profiles.",
        "tools": [{"type": "web_search_20250305", "name": "web_search", "max_uses": 5}],
        "messages": [{"role": "user", "content": prompt}]
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", "web-search-2025-03-05")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    Ok(extract_text_from_response(&resp))
}

fn extract_summary(text: &str) -> String {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                if let Some(s) = v.get("summary").and_then(|s| s.as_str()) {
                    return s.to_string();
                }
            }
        }
    }
    text.lines()
        .filter(|l| !l.trim_start().starts_with("```") && !l.trim().is_empty())
        .take(5)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(500)
        .collect()
}

fn extract_confidence(text: &str) -> f64 {
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                if let Some(c) = v.get("confidence").and_then(|c| c.as_f64()) {
                    return c.clamp(0.0, 1.0);
                }
            }
        }
    }
    0.6
}

fn extract_text_from_response(resp: &serde_json::Value) -> String {
    if let Some(content) = resp.get("content").and_then(|c| c.as_array()) {
        for block in content {
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    return text.to_string();
                }
            }
        }
    }
    resp.to_string()
}

// ── Company Brand Profile ────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CompanyBrandProfile {
    pub id: DbUuid,
    pub company_id: DbUuid,
    pub tagline: Option<String>,
    pub primary_color: String,
    pub secondary_color: String,
    pub accent_color: Option<String>,
    pub typography_heading: Option<String>,
    pub typography_body: Option<String>,
    pub logo_url: Option<String>,
    pub industry: Option<String>,
    pub market_position: Option<String>,
    pub unique_value_proposition: Option<String>,
    pub mission_statement: Option<String>,
    pub vision_statement: Option<String>,
    pub brand_values: Option<String>,
    pub brand_voice: Option<String>,
    pub brand_archetype: Option<String>,
    pub target_audience: Option<String>,
    pub icp_description: Option<String>,
    pub competitor_brands: Option<String>,
    pub differentiators: Option<String>,
    pub content_pillars: Option<String>,
    pub content_tone: Option<String>,
    pub website_url: Option<String>,
    pub social_instagram: Option<String>,
    pub social_twitter: Option<String>,
    pub social_linkedin: Option<String>,
    pub research_status: String,
    pub research_summary: Option<String>,
    pub founder_name: Option<String>,
    pub founding_year: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// GET /api/companies/:id/brand-profile
async fn get_company_brand_profile(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Option<CompanyBrandProfile>>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "company ID")?;
    let profile = sqlx::query_as::<_, CompanyBrandProfile>(
        "SELECT * FROM company_brand_profiles WHERE company_id = ?",
    )
    .bind(&id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;
    Ok(Json(ApiResponse::success(profile)))
}

/// PUT /api/companies/:id/brand-profile — upsert
async fn upsert_company_brand_profile(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<CompanyBrandProfile>>, ApiError> {
    let pool = &deployment.db().pool;
    let id = parse_db_uuid_param(&id, "company ID")?;

    // Upsert: insert or update on conflict
    sqlx::query(
        "INSERT INTO company_brand_profiles (id, company_id, tagline, primary_color, secondary_color, accent_color, typography_heading, typography_body, logo_url, industry, market_position, unique_value_proposition, mission_statement, vision_statement, brand_values, brand_voice, brand_archetype, target_audience, icp_description, competitor_brands, differentiators, content_pillars, content_tone, website_url, social_instagram, social_twitter, social_linkedin, founder_name, founding_year)
         VALUES (randomblob(16), ?, ?, COALESCE(?, '#2563EB'), COALESCE(?, '#EC4899'), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(company_id) DO UPDATE SET
           tagline = COALESCE(excluded.tagline, tagline),
           primary_color = COALESCE(NULLIF(excluded.primary_color, '#2563EB'), primary_color),
           secondary_color = COALESCE(NULLIF(excluded.secondary_color, '#EC4899'), secondary_color),
           accent_color = COALESCE(excluded.accent_color, accent_color),
           typography_heading = COALESCE(excluded.typography_heading, typography_heading),
           typography_body = COALESCE(excluded.typography_body, typography_body),
           logo_url = COALESCE(excluded.logo_url, logo_url),
           industry = COALESCE(excluded.industry, industry),
           market_position = COALESCE(excluded.market_position, market_position),
           unique_value_proposition = COALESCE(excluded.unique_value_proposition, unique_value_proposition),
           mission_statement = COALESCE(excluded.mission_statement, mission_statement),
           vision_statement = COALESCE(excluded.vision_statement, vision_statement),
           brand_values = COALESCE(excluded.brand_values, brand_values),
           brand_voice = COALESCE(excluded.brand_voice, brand_voice),
           brand_archetype = COALESCE(excluded.brand_archetype, brand_archetype),
           target_audience = COALESCE(excluded.target_audience, target_audience),
           icp_description = COALESCE(excluded.icp_description, icp_description),
           competitor_brands = COALESCE(excluded.competitor_brands, competitor_brands),
           differentiators = COALESCE(excluded.differentiators, differentiators),
           content_pillars = COALESCE(excluded.content_pillars, content_pillars),
           content_tone = COALESCE(excluded.content_tone, content_tone),
           website_url = COALESCE(excluded.website_url, website_url),
           social_instagram = COALESCE(excluded.social_instagram, social_instagram),
           social_twitter = COALESCE(excluded.social_twitter, social_twitter),
           social_linkedin = COALESCE(excluded.social_linkedin, social_linkedin),
           founder_name = COALESCE(excluded.founder_name, founder_name),
           founding_year = COALESCE(excluded.founding_year, founding_year),
           updated_at = datetime('now','subsec')",
    )
    .bind(&id)
    .bind(body["tagline"].as_str())
    .bind(body["primary_color"].as_str())
    .bind(body["secondary_color"].as_str())
    .bind(body["accent_color"].as_str())
    .bind(body["typography_heading"].as_str())
    .bind(body["typography_body"].as_str())
    .bind(body["logo_url"].as_str())
    .bind(body["industry"].as_str())
    .bind(body["market_position"].as_str())
    .bind(body["unique_value_proposition"].as_str())
    .bind(body["mission_statement"].as_str())
    .bind(body["vision_statement"].as_str())
    .bind(body["brand_values"].as_str())
    .bind(body["brand_voice"].as_str())
    .bind(body["brand_archetype"].as_str())
    .bind(body["target_audience"].as_str())
    .bind(body["icp_description"].as_str())
    .bind(body["competitor_brands"].as_str())
    .bind(body["differentiators"].as_str())
    .bind(body["content_pillars"].as_str())
    .bind(body["content_tone"].as_str())
    .bind(body["website_url"].as_str())
    .bind(body["social_instagram"].as_str())
    .bind(body["social_twitter"].as_str())
    .bind(body["social_linkedin"].as_str())
    .bind(body["founder_name"].as_str())
    .bind(body["founding_year"].as_str())
    .execute(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;

    let profile = sqlx::query_as::<_, CompanyBrandProfile>(
        "SELECT * FROM company_brand_profiles WHERE company_id = ?",
    )
    .bind(&id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::BadRequest(format!("DB error: {}", e)))?;

    Ok(Json(ApiResponse::success(profile)))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/companies", get(list_companies).post(create_company))
        .route(
            "/companies/{id}",
            get(get_company)
                .patch(update_company)
                .delete(delete_company),
        )
        .route("/companies/{id}/research", post(trigger_company_research))
        .route("/companies/{id}/proposals", get(list_company_proposals))
        .route("/companies/{id}/contacts", get(list_company_contacts))
        .route(
            "/companies/{id}/contact-methods",
            get(list_contact_methods).post(add_contact_method),
        )
        .route(
            "/companies/{id}/contact-methods/{method_id}",
            delete(delete_contact_method),
        )
        .route(
            "/companies/{id}/brand-profile",
            get(get_company_brand_profile).put(upsert_company_brand_profile),
        )
}
