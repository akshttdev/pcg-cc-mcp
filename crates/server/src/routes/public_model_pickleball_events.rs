//! Public GET endpoints for Model Pickleball tour data.
//!
//! Read-only, unauthenticated. The model-pickleball-web Astro site
//! fetches these at build time when PUBLIC_USE_API_EVENTS=true; the
//! loader otherwise uses src/lib/events.bootstrap.ts hardcoded data.
//!
//! Tenancy: scoped to MODEL_PICKLEBALL_ORGANIZATION_ID (same env var
//! the POST handlers use).

use axum::{Router, extract::State, response::Json as ResponseJson, routing::get};
use db::db_uuid::DbUuid;
use deployment::Deployment;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use utils::response::ApiResponse;

use crate::{DeploymentImpl, error::ApiError};

// ---------------------------------------------------------------------------
// Response shapes (must match TS adapters in
// model-pickleball-web/src/lib/events.ts — ApiEvent / ApiVenue / ApiSponsor)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ApiEvent {
    pub id: String,
    pub slug: String,
    pub city: String,
    pub starts_at: String,
    pub ends_at: Option<String>,
    pub status: String,
    pub hero_image: Option<String>,
    pub eventbrite_url: Option<String>,
    pub venue: Option<ApiVenue>,
}

#[derive(Debug, Serialize)]
pub struct ApiVenue {
    pub name: String,
    pub blurb: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiSponsor {
    pub name: String,
    pub logo_url: String,
    pub tier: String,
    pub event_id: Option<String>,
}

// ---------------------------------------------------------------------------
// DB row shapes
// ---------------------------------------------------------------------------

#[derive(Debug, FromRow)]
struct EventRow {
    id: DbUuid,
    slug: String,
    city: String,
    starts_at: String,
    ends_at: Option<String>,
    status: String,
    hero_image: Option<String>,
    eventbrite_url: Option<String>,
    venue_name: Option<String>,
    venue_blurb: Option<String>,
    venue_image: Option<String>,
}

#[derive(Debug, FromRow)]
struct SponsorRow {
    name: String,
    logo_url: String,
    tier: String,
    event_id: Option<DbUuid>,
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// GET /api/public/model-pickleball/events
// ---------------------------------------------------------------------------

async fn list_events(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ApiEvent>>>, ApiError> {
    let org_id = mp_organization_id()?;
    let rows = fetch_events(&deployment.db().pool, &org_id).await?;
    let body = rows.into_iter().map(event_row_to_api).collect();
    Ok(ResponseJson(ApiResponse::success(body)))
}

async fn fetch_events(pool: &SqlitePool, org_id: &DbUuid) -> Result<Vec<EventRow>, ApiError> {
    let rows = sqlx::query_as::<_, EventRow>(
        r#"SELECT
              e.id              AS id,
              e.slug            AS slug,
              e.city            AS city,
              e.starts_at       AS starts_at,
              e.ends_at         AS ends_at,
              e.status          AS status,
              e.hero_image      AS hero_image,
              e.eventbrite_url  AS eventbrite_url,
              v.name            AS venue_name,
              v.blurb           AS venue_blurb,
              v.image           AS venue_image
           FROM mp_events e
           LEFT JOIN mp_venues v ON v.id = e.venue_id AND v.deleted_at IS NULL
           WHERE e.organization_id = ?
             AND e.deleted_at IS NULL
             AND e.status IN ('announced', 'live')
           ORDER BY e.starts_at ASC"#,
    )
    .bind(org_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

fn event_row_to_api(row: EventRow) -> ApiEvent {
    let venue = match (&row.venue_name, &row.venue_blurb, &row.venue_image) {
        (None, None, None) => None,
        _ => Some(ApiVenue {
            name: row.venue_name.unwrap_or_default(),
            blurb: row.venue_blurb,
            image: row.venue_image,
        }),
    };
    ApiEvent {
        id: row.id.to_string(),
        slug: row.slug,
        city: row.city,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        status: row.status,
        hero_image: row.hero_image,
        eventbrite_url: row.eventbrite_url,
        venue,
    }
}

// ---------------------------------------------------------------------------
// GET /api/public/model-pickleball/sponsors
// ---------------------------------------------------------------------------

async fn list_sponsors(
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ApiSponsor>>>, ApiError> {
    let org_id = mp_organization_id()?;
    let pool = deployment.db().pool.clone();
    let rows = sqlx::query_as::<_, SponsorRow>(
        r#"SELECT name, logo_url, tier, event_id
           FROM mp_sponsors
           WHERE organization_id = ?
             AND deleted_at IS NULL
           ORDER BY
             CASE tier
               WHEN 'presenting' THEN 0
               WHEN 'venue'      THEN 1
               WHEN 'equipment'  THEN 2
               WHEN 'brand'      THEN 3
               WHEN 'associate'  THEN 4
               WHEN 'activation' THEN 5
               ELSE 99
             END,
             name ASC"#,
    )
    .bind(&org_id)
    .fetch_all(&pool)
    .await?;

    let body = rows
        .into_iter()
        .map(|r| ApiSponsor {
            name: r.name,
            logo_url: r.logo_url,
            tier: r.tier,
            event_id: r.event_id.map(|id| id.to_string()),
        })
        .collect();

    Ok(ResponseJson(ApiResponse::success(body)))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn public_router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().nest(
        "/public/model-pickleball",
        Router::new()
            .route("/events", get(list_events))
            .route("/sponsors", get(list_sponsors)),
    )
}
