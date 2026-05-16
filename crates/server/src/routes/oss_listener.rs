//! Open Source Library Listener
//!
//! Tracks GitHub releases for sovereign-stack dependencies. On every new
//! release an agent workflow is created and Nora generates an upgrade
//! recommendation stored back on the update record.
//!
//! Endpoints:
//!   GET  /oss-libraries                        — list all tracked libraries
//!   POST /oss-libraries                        — add a library to track
//!   PATCH /oss-libraries/:id                   — update tracking config
//!   DELETE /oss-libraries/:id                  — stop tracking
//!   POST /oss-libraries/:id/check              — trigger immediate GitHub check
//!   GET  /oss-libraries/:id/updates            — list updates for a library
//!   GET  /oss-updates/recent                   — latest updates across all libraries
//!   PATCH /oss-updates/:id/dismiss             — dismiss a recommendation

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use db::{
    db_uuid::DbUuid,
    models::oss_library::{OssLibrary, OssLibraryUpdate},
};
use deployment::Deployment;
use serde::Deserialize;
use ts_rs::TS;
use uuid::Uuid;

use crate::DeploymentImpl;

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateOssLibrary {
    pub name: String,
    pub github_owner: String,
    pub github_repo: String,
    pub notes: Option<String>,
    pub check_interval_secs: Option<i64>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct PatchOssLibrary {
    pub name: Option<String>,
    pub notes: Option<String>,
    pub check_interval_secs: Option<i64>,
    pub is_active: Option<bool>,
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn list_libraries(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let libs = OssLibrary::list(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(libs))
}

async fn create_library(
    State(deployment): State<DeploymentImpl>,
    Json(body): Json<CreateOssLibrary>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;
    let id = Uuid::new_v4();

    sqlx::query(
        "INSERT OR IGNORE INTO oss_libraries
         (id, name, github_owner, github_repo, notes, check_interval_secs)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(&body.name)
    .bind(&body.github_owner)
    .bind(&body.github_repo)
    .bind(&body.notes)
    .bind(body.check_interval_secs.unwrap_or(3600))
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let created = OssLibrary::get(pool, id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "Insert failed".into()))?;

    Ok((StatusCode::CREATED, Json(created)))
}

async fn patch_library(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
    Json(body): Json<PatchOssLibrary>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = &deployment.db().pool;
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid ID".to_string()))?
        .to_uuid();

    sqlx::query(
        "UPDATE oss_libraries
         SET name                = COALESCE(?, name),
             notes               = COALESCE(?, notes),
             check_interval_secs = COALESCE(?, check_interval_secs),
             is_active           = COALESCE(?, is_active),
             updated_at          = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(&body.name)
    .bind(&body.notes)
    .bind(body.check_interval_secs)
    .bind(body.is_active)
    .bind(id_uuid)
    .execute(pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updated = OssLibrary::get(pool, id_uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Library not found".into()))?;

    Ok(Json(updated))
}

async fn delete_library(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid ID".to_string()))?
        .to_uuid();
    let result = sqlx::query("DELETE FROM oss_libraries WHERE id = ?")
        .bind(id_uuid)
        .execute(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "Library not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Trigger an immediate check for a single library without waiting for the scheduler.
async fn check_library_now(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let pool = deployment.db().pool.clone();
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid ID".to_string()))?
        .to_uuid();

    let lib = OssLibrary::get(&pool, id_uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Library not found".into()))?;

    let lib_name = lib.name.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::routes::oss_listener_bg::check_and_process_library(&pool, &lib).await
        {
            tracing::error!(
                "[OSS_LISTENER] Manual check failed for '{}': {}",
                lib.name,
                e
            );
        }
    });

    Ok(Json(
        serde_json::json!({ "status": "check_queued", "library": lib_name }),
    ))
}

async fn list_updates_for_library(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid ID".to_string()))?
        .to_uuid();
    let updates = OssLibraryUpdate::list_for_library(&deployment.db().pool, id_uuid)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(updates))
}

async fn recent_updates(
    State(deployment): State<DeploymentImpl>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let updates = OssLibraryUpdate::list_recent(&deployment.db().pool, 50)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(updates))
}

async fn dismiss_update(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let id_uuid = DbUuid::parse(&id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid ID".to_string()))?
        .to_uuid();
    sqlx::query("UPDATE oss_library_updates SET recommendation_status = 'dismissed' WHERE id = ?")
        .bind(id_uuid)
        .execute(&deployment.db().pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(serde_json::json!({ "status": "dismissed" })))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/oss-libraries", get(list_libraries).post(create_library))
        .route(
            "/oss-libraries/{id}",
            patch(patch_library).delete(delete_library),
        )
        .route("/oss-libraries/{id}/check", post(check_library_now))
        .route("/oss-libraries/{id}/updates", get(list_updates_for_library))
        .route("/oss-updates/recent", get(recent_updates))
        .route("/oss-updates/{id}/dismiss", patch(dismiss_update))
}
