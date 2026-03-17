use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use deployment::Deployment;
use nora::agent::{NoraRequest, NoraRequestType, RequestPriority};
use serde::{Deserialize, Serialize};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, routes::nora::get_nora_instance, DeploymentImpl};
use db::db_uuid::DbUuid;
use db::models::company::{Company, CreateCompany, UpdateCompany};
use db::models::proposal::Proposal;
use db::models::person_association::{CompanyContactMethod, CreateCompanyContactMethod};

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
    Path(id): Path<Uuid>,
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
    Path(id): Path<Uuid>,
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
    Path(id): Path<Uuid>,
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
    Path(id): Path<Uuid>,
    Query(q): Query<ListCompanyProposalsQuery>,
) -> Result<Json<ApiResponse<Vec<Proposal>>>, ApiError> {
    let pool = &deployment.db().pool;
    let proposals = Proposal::list_by_company(pool, id, q.limit).await?;
    Ok(Json(ApiResponse::success(proposals)))
}

/// GET /companies/:id/persons — persons (contacts) at this company (via junction table)
async fn list_company_persons(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<db::models::person::Person>>>, ApiError> {
    let pool = &deployment.db().pool;
    let persons = sqlx::query_as::<_, db::models::person::Person>(
        r#"SELECT p.* FROM persons p
           JOIN person_company_roles pcr ON pcr.person_id = p.id
           WHERE pcr.company_id = ?
           ORDER BY pcr.is_primary DESC, p.full_name ASC"#,
    )
    .bind(id.to_string())
    .fetch_all(pool)
    .await?;
    Ok(Json(ApiResponse::success(persons)))
}

// ---------------------------------------------------------------------------
// Company contact methods
// ---------------------------------------------------------------------------

/// GET /companies/:id/contact-methods
async fn list_contact_methods(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<CompanyContactMethod>>>, ApiError> {
    let pool = &deployment.db().pool;
    let methods = CompanyContactMethod::list_for_company(pool, id).await?;
    Ok(Json(ApiResponse::success(methods)))
}

/// POST /companies/:id/contact-methods
async fn add_contact_method(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateCompanyContactMethod>,
) -> Result<Json<ApiResponse<CompanyContactMethod>>, ApiError> {
    let pool = &deployment.db().pool;
    let method = CompanyContactMethod::create(pool, id, data).await?;
    Ok(Json(ApiResponse::success(method)))
}

/// DELETE /companies/:id/contact-methods/:method_id
async fn delete_contact_method(
    State(deployment): State<DeploymentImpl>,
    Path((_id, method_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;
    let deleted = CompanyContactMethod::delete(pool, method_id).await?;
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
    Path(company_id): Path<Uuid>,
) -> Result<Json<ApiResponse<CompanyResearchResponse>>, ApiError> {
    let pool = &deployment.db().pool;
    let db_company_id = DbUuid::from(company_id);
    let company = Company::find_by_id(pool, &db_company_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Company not found".into()))?;

    // Mark as running immediately
    sqlx::query(
        "UPDATE companies SET intelligence_status = 'running', updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(company_id)
    .execute(pool)
    .await?;

    let name = company.name.clone();
    let website = company.website.clone();
    let pool_clone = pool.clone();

    tokio::spawn(async move {
        let result = run_company_research(&pool_clone, company_id, &name, website.as_deref()).await;
        if let Err(e) = result {
            tracing::error!("Company research failed for {}: {}", name, e);
            let _ = sqlx::query(
                "UPDATE companies SET intelligence_status = 'failed', updated_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(company_id)
            .execute(&pool_clone)
            .await;
        }
    });

    Ok(Json(ApiResponse::success(CompanyResearchResponse {
        company_id,
        status: "running".into(),
        message: format!("Research started for {} — Scout is gathering brand intelligence.", company.name),
    })))
}

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
    let nora_result = (|| async {
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
        .ok()?.ok()
    })()
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

    tracing::info!("Company research complete for {} (confidence: {:.0}%)", name, confidence * 100.0);
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
        website_ctx = website.map(|w| format!(" (website: {})", w)).unwrap_or_default(),
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

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/companies", get(list_companies).post(create_company))
        .route(
            "/companies/{id}",
            get(get_company).patch(update_company).delete(delete_company),
        )
        .route("/companies/{id}/research", post(trigger_company_research))
        .route("/companies/{id}/proposals", get(list_company_proposals))
        .route("/companies/{id}/persons", get(list_company_persons))
        .route(
            "/companies/{id}/contact-methods",
            get(list_contact_methods).post(add_contact_method),
        )
        .route(
            "/companies/{id}/contact-methods/{method_id}",
            delete(delete_contact_method),
        )
}
