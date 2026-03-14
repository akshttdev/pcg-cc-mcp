//! Sync management routes — CRUD for sync folders, device registration,
//! sync state reporting, and conflict resolution.
//!
//! All routes scoped to organization. Endpoints:
//!
//! Folders:
//!   GET    /organizations/:org_id/sync/folders          — list sync folders
//!   POST   /organizations/:org_id/sync/folders          — create folder
//!   PUT    /sync/folders/:id                            — update folder
//!   DELETE /sync/folders/:id                            — archive folder
//!
//! Devices:
//!   GET    /organizations/:org_id/sync/devices          — list org devices
//!   POST   /sync/devices/register                       — register device
//!   POST   /sync/devices/:id/heartbeat                  — device heartbeat
//!   PUT    /sync/devices/:id/status                     — update sync status
//!   DELETE /sync/devices/:id                            — deactivate device
//!
//! Sync State:
//!   GET    /sync/devices/:id/state                      — device file state
//!   POST   /sync/devices/:id/state                      — upsert file sync state
//!   GET    /sync/devices/:id/conflicts                  — list conflicts
//!   POST   /sync/state/:id/resolve                      — resolve conflict
//!   GET    /sync/devices/:id/pending                    — files needing sync
//!   GET    /sync/devices/:id/summary                    — sync summary stats
//!
//! Subscriptions:
//!   GET    /sync/devices/:id/subscriptions              — folder subscriptions
//!   POST   /sync/devices/:id/subscriptions              — subscribe to folder
//!   DELETE /sync/subscriptions/:id                      — unsubscribe
//!
//! Org Summary:
//!   GET    /organizations/:org_id/sync/summary          — org-wide sync stats

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post, put},
};
use db::models::{
    sync_folder::{CreateSyncFolder, SyncFolder, UpdateSyncFolder},
    sync_device::{RegisterDevice, SyncDevice, UpdateDeviceStatus},
    sync_state::{SyncState, UpsertSyncState, ResolveConflict},
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// ── Subscription types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSubscription {
    pub sync_folder_id: Uuid,
    pub is_enabled: Option<bool>,
    pub max_file_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncSubscription {
    pub id: Uuid,
    pub device_id: Uuid,
    pub sync_folder_id: Uuid,
    pub is_enabled: bool,
    pub selective_paths: Option<String>,
    pub max_file_size: Option<i64>,
    pub created_at: String,
}

// ── Folder routes ──────────────────────────────────────────────────────────

async fn list_folders(
    State(dep): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<SyncFolder>>>, ApiError> {
    let folders = SyncFolder::find_by_organization(&dep.pool, org_id).await?;
    Ok(Json(ApiResponse::success(folders)))
}

async fn create_folder(
    State(dep): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
    Json(mut input): Json<CreateSyncFolder>,
) -> Result<Json<ApiResponse<SyncFolder>>, ApiError> {
    input.organization_id = org_id;
    let folder = SyncFolder::create(&dep.pool, input).await?;
    Ok(Json(ApiResponse::success(folder)))
}

async fn update_folder(
    State(dep): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateSyncFolder>,
) -> Result<Json<ApiResponse<Option<SyncFolder>>>, ApiError> {
    let folder = SyncFolder::update(&dep.pool, id, input).await?;
    Ok(Json(ApiResponse::success(folder)))
}

async fn delete_folder(
    State(dep): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    SyncFolder::archive(&dep.pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ── Device routes ──────────────────────────────────────────────────────────

async fn list_devices(
    State(dep): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<SyncDevice>>>, ApiError> {
    let devices = SyncDevice::find_by_organization(&dep.pool, org_id).await?;
    Ok(Json(ApiResponse::success(devices)))
}

async fn register_device(
    State(dep): State<DeploymentImpl>,
    Json(input): Json<RegisterDevice>,
) -> Result<Json<ApiResponse<SyncDevice>>, ApiError> {
    let device = SyncDevice::register(&dep.pool, input).await?;
    Ok(Json(ApiResponse::success(device)))
}

async fn heartbeat_device(
    State(dep): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    SyncDevice::heartbeat(&dep.pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

async fn update_device_status(
    State(dep): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateDeviceStatus>,
) -> Result<Json<ApiResponse<Option<SyncDevice>>>, ApiError> {
    let device = SyncDevice::update_status(&dep.pool, id, input).await?;
    Ok(Json(ApiResponse::success(device)))
}

async fn deactivate_device(
    State(dep): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    SyncDevice::deactivate(&dep.pool, id).await?;
    Ok(Json(ApiResponse::success(())))
}

// ── Sync state routes ──────────────────────────────────────────────────────

async fn get_device_state(
    State(dep): State<DeploymentImpl>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<SyncState>>>, ApiError> {
    let state = SyncState::find_by_device(&dep.pool, device_id).await?;
    Ok(Json(ApiResponse::success(state)))
}

async fn upsert_state(
    State(dep): State<DeploymentImpl>,
    Path(device_id): Path<Uuid>,
    Json(mut input): Json<UpsertSyncState>,
) -> Result<Json<ApiResponse<SyncState>>, ApiError> {
    input.device_id = device_id;
    let state = SyncState::upsert(&dep.pool, input).await?;
    Ok(Json(ApiResponse::success(state)))
}

async fn get_conflicts(
    State(dep): State<DeploymentImpl>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<SyncState>>>, ApiError> {
    let conflicts = SyncState::find_conflicts(&dep.pool, device_id).await?;
    Ok(Json(ApiResponse::success(conflicts)))
}

async fn get_pending(
    State(dep): State<DeploymentImpl>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<SyncState>>>, ApiError> {
    let pending = SyncState::find_pending(&dep.pool, device_id).await?;
    Ok(Json(ApiResponse::success(pending)))
}

async fn resolve_conflict(
    State(dep): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(input): Json<ResolveConflict>,
) -> Result<Json<ApiResponse<Option<SyncState>>>, ApiError> {
    let state = SyncState::resolve_conflict(&dep.pool, id, &input.resolution).await?;
    Ok(Json(ApiResponse::success(state)))
}

async fn device_summary(
    State(dep): State<DeploymentImpl>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<ApiResponse<db::models::sync_state::SyncSummary>>, ApiError> {
    let summary = SyncState::summary_for_device(&dep.pool, device_id).await?;
    Ok(Json(ApiResponse::success(summary)))
}

// ── Subscription routes ────────────────────────────────────────────────────

async fn list_subscriptions(
    State(dep): State<DeploymentImpl>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<SyncSubscription>>>, ApiError> {
    let device_bytes = device_id.as_bytes().to_vec();
    let rows = sqlx::query_as::<_, (Vec<u8>, Vec<u8>, Vec<u8>, bool, Option<String>, Option<i64>, String)>(
        r#"SELECT id, device_id, sync_folder_id, is_enabled, selective_paths, max_file_size,
                  datetime(created_at) as created_at
           FROM sync_subscriptions WHERE device_id = ?"#,
    )
    .bind(&device_bytes)
    .fetch_all(&dep.pool)
    .await?;

    let subs: Vec<SyncSubscription> = rows.into_iter().map(|r| {
        SyncSubscription {
            id: Uuid::from_slice(&r.0).unwrap_or_default(),
            device_id: Uuid::from_slice(&r.1).unwrap_or_default(),
            sync_folder_id: Uuid::from_slice(&r.2).unwrap_or_default(),
            is_enabled: r.3,
            selective_paths: r.4,
            max_file_size: r.5,
            created_at: r.6,
        }
    }).collect();

    Ok(Json(ApiResponse::success(subs)))
}

async fn create_subscription(
    State(dep): State<DeploymentImpl>,
    Path(device_id): Path<Uuid>,
    Json(input): Json<CreateSubscription>,
) -> Result<Json<ApiResponse<SyncSubscription>>, ApiError> {
    let id = Uuid::new_v4();
    let id_bytes = id.as_bytes().to_vec();
    let device_bytes = device_id.as_bytes().to_vec();
    let folder_bytes = input.sync_folder_id.as_bytes().to_vec();
    let is_enabled = input.is_enabled.unwrap_or(true);

    sqlx::query(
        r#"INSERT INTO sync_subscriptions (id, device_id, sync_folder_id, is_enabled, max_file_size)
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT(device_id, sync_folder_id) DO UPDATE SET
             is_enabled = excluded.is_enabled,
             max_file_size = excluded.max_file_size"#,
    )
    .bind(&id_bytes)
    .bind(&device_bytes)
    .bind(&folder_bytes)
    .bind(is_enabled)
    .bind(input.max_file_size)
    .execute(&dep.pool)
    .await?;

    Ok(Json(ApiResponse::success(SyncSubscription {
        id,
        device_id,
        sync_folder_id: input.sync_folder_id,
        is_enabled,
        selective_paths: None,
        max_file_size: input.max_file_size,
        created_at: chrono::Utc::now().to_rfc3339(),
    })))
}

async fn delete_subscription(
    State(dep): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let id_bytes = id.as_bytes().to_vec();
    sqlx::query("DELETE FROM sync_subscriptions WHERE id = ?")
        .bind(&id_bytes)
        .execute(&dep.pool)
        .await?;
    Ok(Json(ApiResponse::success(())))
}

// ── Org summary ────────────────────────────────────────────────────────────

async fn org_sync_summary(
    State(dep): State<DeploymentImpl>,
    Path(org_id): Path<Uuid>,
) -> Result<Json<ApiResponse<OrgSyncOverview>>, ApiError> {
    let folders = SyncFolder::find_by_organization(&dep.pool, org_id).await?;
    let devices = SyncDevice::find_by_organization(&dep.pool, org_id).await?;
    let summary = SyncState::summary_for_org(&dep.pool, org_id).await?;

    Ok(Json(ApiResponse::success(OrgSyncOverview {
        folder_count: folders.len() as i64,
        device_count: devices.len() as i64,
        active_devices: devices.iter().filter(|d| {
            d.last_seen_at.map(|t| {
                chrono::Utc::now().signed_duration_since(t).num_minutes() < 5
            }).unwrap_or(false)
        }).count() as i64,
        sync_summary: summary,
    })))
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct OrgSyncOverview {
    pub folder_count: i64,
    pub device_count: i64,
    pub active_devices: i64,
    pub sync_summary: db::models::sync_state::SyncSummary,
}

// ── Router ─────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        // Org-scoped
        .route("/organizations/{org_id}/sync/folders", get(list_folders).post(create_folder))
        .route("/organizations/{org_id}/sync/devices", get(list_devices))
        .route("/organizations/{org_id}/sync/summary", get(org_sync_summary))
        // Folder management
        .route("/sync/folders/{id}", put(update_folder).delete(delete_folder))
        // Device management
        .route("/sync/devices/register", post(register_device))
        .route("/sync/devices/{id}/heartbeat", post(heartbeat_device))
        .route("/sync/devices/{id}/status", put(update_device_status))
        .route("/sync/devices/{id}", delete(deactivate_device))
        // Sync state
        .route("/sync/devices/{id}/state", get(get_device_state).post(upsert_state))
        .route("/sync/devices/{id}/conflicts", get(get_conflicts))
        .route("/sync/devices/{id}/pending", get(get_pending))
        .route("/sync/devices/{id}/summary", get(device_summary))
        .route("/sync/state/{id}/resolve", post(resolve_conflict))
        // Subscriptions
        .route("/sync/devices/{id}/subscriptions", get(list_subscriptions).post(create_subscription))
        .route("/sync/subscriptions/{id}", delete(delete_subscription))
        .with_state(deployment.clone())
}
