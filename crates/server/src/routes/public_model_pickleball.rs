//! Public POST endpoints for the Model Pickleball marketing site.
//!
//! These 8 routes back the 7 form components in `model-pickleball-web`. They
//! are intentionally unauthenticated — the site posts anonymously. Tenancy is
//! pinned to a single organization via the `MODEL_PICKLEBALL_ORGANIZATION_ID`
//! env var (a hyphenated UUID matching `organizations.id`).
//!
//! Spam/abuse controls (Turnstile, rate limiting) record the token + client
//! info but verification is intentionally deferred to a follow-up PR.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json as ResponseJson,
    routing::post,
    Router,
};
use db::db_uuid::DbUuid;
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use utils::response::ApiResponse;

use crate::{error::ApiError, DeploymentImpl};

// ---------------------------------------------------------------------------
// Common types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct SubmitResponse {
    pub ok: bool,
    pub id: String,
}

#[derive(Clone, Debug)]
struct ClientAudit {
    cf_turnstile_token: Option<String>,
    ip_address: Option<String>,
    user_agent: Option<String>,
}

fn client_audit(headers: &HeaderMap) -> ClientAudit {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };
    ClientAudit {
        cf_turnstile_token: header("cf-turnstile-token"),
        ip_address: header("x-forwarded-for")
            .or_else(|| header("x-real-ip"))
            .map(|s| s.split(',').next().unwrap_or("").trim().to_string()),
        user_agent: header("user-agent"),
    }
}

fn mp_organization_id() -> Result<DbUuid, ApiError> {
    let raw = std::env::var("MODEL_PICKLEBALL_ORGANIZATION_ID").map_err(|_| {
        ApiError::BadRequest(
            "MODEL_PICKLEBALL_ORGANIZATION_ID env var not configured on backend".into(),
        )
    })?;
    DbUuid::parse(&raw).map_err(|_| {
        ApiError::BadRequest("MODEL_PICKLEBALL_ORGANIZATION_ID is not a valid UUID".into())
    })
}

fn json_string(value: &Value) -> Result<String, ApiError> {
    serde_json::to_string(value)
        .map_err(|e| ApiError::BadRequest(format!("Failed to serialize payload: {}", e)))
}

fn require(field: &str, value: Option<&str>) -> Result<String, ApiError> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        Some(v) => Ok(v.to_string()),
        None => Err(ApiError::BadRequest(format!("{} is required", field))),
    }
}

fn pluck_str(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ---------------------------------------------------------------------------
// 1. Newsletter — POST /api/public/model-pickleball/newsletter
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct NewsletterBody {
    pub email: String,
}

async fn submit_newsletter(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    ResponseJson(body): ResponseJson<NewsletterBody>,
) -> Result<ResponseJson<ApiResponse<SubmitResponse>>, ApiError> {
    let email = require("email", Some(&body.email))?.to_lowercase();
    let org_id = mp_organization_id()?;
    let id = DbUuid::new();
    let pool = deployment.db().pool.clone();

    insert_newsletter(&pool, &id, &org_id, &email).await?;

    let _audit = client_audit(&headers); // reserved for future Turnstile verification

    Ok(ResponseJson(ApiResponse::success(SubmitResponse {
        ok: true,
        id: id.to_string(),
    })))
}

async fn insert_newsletter(
    pool: &SqlitePool,
    id: &DbUuid,
    org_id: &DbUuid,
    email: &str,
) -> Result<(), ApiError> {
    sqlx::query(
        r#"INSERT INTO newsletter_subscribers (id, organization_id, email, source)
           VALUES (?, ?, ?, 'model_pickleball')
           ON CONFLICT(organization_id, email) DO UPDATE SET deleted_at = NULL"#,
    )
    .bind(id)
    .bind(org_id)
    .bind(email)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Contact — POST /api/public/model-pickleball/contact
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ContactBody {
    pub first: String,
    pub last: String,
    pub company: Option<String>,
    pub email: String,
    pub phone: String,
    pub instagram: Option<String>,
    pub website: Option<String>,
    pub inquiry: String,
}

async fn submit_contact(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    ResponseJson(body): ResponseJson<ContactBody>,
) -> Result<ResponseJson<ApiResponse<SubmitResponse>>, ApiError> {
    let first = require("first", Some(&body.first))?;
    let last = require("last", Some(&body.last))?;
    let email = require("email", Some(&body.email))?.to_lowercase();
    let phone = require("phone", Some(&body.phone))?;
    let inquiry = require("inquiry", Some(&body.inquiry))?;

    let org_id = mp_organization_id()?;
    let id = DbUuid::new();
    let pool = deployment.db().pool.clone();
    let audit = client_audit(&headers);

    sqlx::query(
        r#"INSERT INTO mp_contact_submissions
           (id, organization_id, first_name, last_name, company, email, phone,
            instagram, website, inquiry, cf_turnstile_token, ip_address, user_agent)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&org_id)
    .bind(&first)
    .bind(&last)
    .bind(body.company.as_deref())
    .bind(&email)
    .bind(&phone)
    .bind(body.instagram.as_deref())
    .bind(body.website.as_deref())
    .bind(&inquiry)
    .bind(audit.cf_turnstile_token.as_deref())
    .bind(audit.ip_address.as_deref())
    .bind(audit.user_agent.as_deref())
    .execute(&pool)
    .await?;

    Ok(ResponseJson(ApiResponse::success(SubmitResponse {
        ok: true,
        id: id.to_string(),
    })))
}

// ---------------------------------------------------------------------------
// 3. Registrations — POST /registrations/{model|agency}
// ---------------------------------------------------------------------------

async fn submit_registration(
    pool: &SqlitePool,
    kind: &str,
    payload: Value,
    audit: &ClientAudit,
) -> Result<DbUuid, ApiError> {
    let email = require("email", pluck_str(&payload, "email").as_deref())?.to_lowercase();
    let phone = require("phone", pluck_str(&payload, "phone").as_deref())?;

    let org_id = mp_organization_id()?;
    let id = DbUuid::new();
    let payload_json = json_string(&payload)?;

    sqlx::query(
        r#"INSERT INTO mp_registrations
           (id, organization_id, kind, email, phone, payload,
            cf_turnstile_token, ip_address, user_agent)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&org_id)
    .bind(kind)
    .bind(&email)
    .bind(&phone)
    .bind(&payload_json)
    .bind(audit.cf_turnstile_token.as_deref())
    .bind(audit.ip_address.as_deref())
    .bind(audit.user_agent.as_deref())
    .execute(pool)
    .await?;

    Ok(id)
}

async fn submit_model_registration(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    ResponseJson(body): ResponseJson<Value>,
) -> Result<ResponseJson<ApiResponse<SubmitResponse>>, ApiError> {
    let audit = client_audit(&headers);
    let id = submit_registration(&deployment.db().pool, "model", body, &audit).await?;
    Ok(ResponseJson(ApiResponse::success(SubmitResponse {
        ok: true,
        id: id.to_string(),
    })))
}

async fn submit_agency_registration(
    State(deployment): State<DeploymentImpl>,
    headers: HeaderMap,
    ResponseJson(body): ResponseJson<Value>,
) -> Result<ResponseJson<ApiResponse<SubmitResponse>>, ApiError> {
    let audit = client_audit(&headers);
    let id = submit_registration(&deployment.db().pool, "agency", body, &audit).await?;
    Ok(ResponseJson(ApiResponse::success(SubmitResponse {
        ok: true,
        id: id.to_string(),
    })))
}

// ---------------------------------------------------------------------------
// 4. Inquiries — POST /inquiries/{brand|sponsor|media|venue}
// ---------------------------------------------------------------------------

async fn submit_inquiry(
    pool: &SqlitePool,
    kind: &str,
    payload: Value,
    audit: &ClientAudit,
) -> Result<DbUuid, ApiError> {
    let email = require("email", pluck_str(&payload, "email").as_deref())?.to_lowercase();
    let phone = pluck_str(&payload, "phone").filter(|s| !s.trim().is_empty());

    let org_id = mp_organization_id()?;
    let id = DbUuid::new();
    let payload_json = json_string(&payload)?;

    sqlx::query(
        r#"INSERT INTO mp_inquiries
           (id, organization_id, kind, email, phone, payload,
            cf_turnstile_token, ip_address, user_agent)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(&org_id)
    .bind(kind)
    .bind(&email)
    .bind(phone.as_deref())
    .bind(&payload_json)
    .bind(audit.cf_turnstile_token.as_deref())
    .bind(audit.ip_address.as_deref())
    .bind(audit.user_agent.as_deref())
    .execute(pool)
    .await?;

    Ok(id)
}

macro_rules! inquiry_handler {
    ($name:ident, $kind:expr) => {
        async fn $name(
            State(deployment): State<DeploymentImpl>,
            headers: HeaderMap,
            ResponseJson(body): ResponseJson<Value>,
        ) -> Result<ResponseJson<ApiResponse<SubmitResponse>>, ApiError> {
            let audit = client_audit(&headers);
            let id = submit_inquiry(&deployment.db().pool, $kind, body, &audit).await?;
            Ok(ResponseJson(ApiResponse::success(SubmitResponse {
                ok: true,
                id: id.to_string(),
            })))
        }
    };
}

inquiry_handler!(submit_brand_inquiry, "brand");
inquiry_handler!(submit_sponsor_inquiry, "sponsor");
inquiry_handler!(submit_media_inquiry, "media");
inquiry_handler!(submit_venue_inquiry, "venue");

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn public_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/public/model-pickleball",
        Router::new()
            .route("/newsletter", post(submit_newsletter))
            .route("/contact", post(submit_contact))
            .route("/registrations/model", post(submit_model_registration))
            .route("/registrations/agency", post(submit_agency_registration))
            .route("/inquiries/brand", post(submit_brand_inquiry))
            .route("/inquiries/sponsor", post(submit_sponsor_inquiry))
            .route("/inquiries/media", post(submit_media_inquiry))
            .route("/inquiries/venue", post(submit_venue_inquiry)),
    )
}

// Silence unused warning for StatusCode import in some builds.
#[allow(dead_code)]
const _UNUSED_STATUS: StatusCode = StatusCode::OK;
