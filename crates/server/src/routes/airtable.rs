use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use db::models::{
    airtable_base::{AirtableBase, CreateAirtableBase, UpdateAirtableBase},
    airtable_record_link::{
        AirtableOrigin, AirtableRecordLink, AirtableSyncStatus, CreateAirtableRecordLink,
    },
    task::{CreateTask, Priority, Task},
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use services::services::{
    airtable_service::{
        build_record_url, get_record_name, AirtableBaseInfo, AirtableRecord, AirtableService,
        AirtableTable,
    },
    config::save_config_to_file,
};
use tracing::{error, info};
use ts_rs::TS;
use utils::{assets::config_path, response::ApiResponse};
use uuid::Uuid;

use crate::DeploymentImpl;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        // Config/Auth endpoints
        .route("/airtable/verify", post(verify_credentials))
        .route("/airtable/bases", get(list_user_bases))
        // Base connection management
        .route(
            "/airtable/connections",
            get(list_connections).post(create_connection),
        )
        .route(
            "/airtable/connections/{id}",
            get(get_connection)
                .delete(delete_connection)
                .patch(update_connection),
        )
        .route("/airtable/connections/{id}/tables", get(get_base_tables))
        .route("/airtable/connections/{id}/records", get(get_table_records))
        // Sync operations
        .route(
            "/airtable/connections/{id}/import",
            post(import_records_from_table),
        )
        // Task-level operations
        .route("/airtable/tasks/{task_id}/link", get(get_task_link))
        .route("/airtable/tasks/{task_id}/push", post(push_task_to_airtable))
        .route(
            "/airtable/tasks/{task_id}/sync-deliverables",
            post(sync_deliverables_to_airtable),
        )
}

// Request/Response types

#[derive(Debug, Deserialize, TS)]
pub struct AirtableVerifyRequest {
    pub token: String,
}

#[derive(Debug, Serialize, TS)]
pub struct AirtableVerifyResponse {
    pub user_email: Option<String>,
    pub valid: bool,
}

#[derive(Debug, Deserialize, TS)]
pub struct AirtableProjectQuery {
    pub project_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
pub struct AirtableTableQuery {
    pub table_id: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct AirtableImportRequest {
    pub table_id: String,
    pub project_id: Uuid,
    pub board_id: Option<Uuid>,
    /// The field name to use as task title (defaults to first text field)
    pub title_field: Option<String>,
    /// The field name to use as task description (optional)
    pub description_field: Option<String>,
}

#[derive(Debug, Serialize, TS)]
pub struct AirtableImportResult {
    pub imported_count: usize,
    pub skipped_count: usize,
    pub tasks: Vec<AirtableTaskWithLink>,
}

#[derive(Debug, Serialize, TS)]
pub struct AirtableTaskWithLink {
    pub task: Task,
    pub airtable_link: AirtableRecordLink,
}

#[derive(Debug, Deserialize, TS)]
pub struct AirtablePushTaskRequest {
    pub table_id: String,
    pub base_id: String,
    /// Field name to store the task title
    pub title_field: Option<String>,
    /// Field name to store the task description
    pub description_field: Option<String>,
}

#[derive(Debug, Serialize, TS)]
pub struct AirtableConnectionWithBase {
    pub connection: AirtableBase,
    pub base_info: Option<AirtableBaseInfo>,
}

// Handler implementations

/// Verify Airtable Personal Access Token and optionally save it to config
async fn verify_credentials(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<AirtableVerifyRequest>,
) -> Result<Json<ApiResponse<AirtableVerifyResponse>>, StatusCode> {
    let service = match AirtableService::new(&payload.token) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Invalid token: {}",
                e
            ))));
        }
    };

    match service.verify_credentials().await {
        Ok(user_info) => {
            // Store token in config
            {
                let mut config = deployment.config().write().await;
                config.airtable.token = Some(payload.token);
                config.airtable.user_email = user_info.email.clone();

                // Save to file
                if let Err(e) = save_config_to_file(&config, &config_path()).await {
                    error!("Failed to save Airtable config: {}", e);
                }
            }

            info!(
                "Airtable credentials verified for user: {:?}",
                user_info.email
            );
            Ok(Json(ApiResponse::success(AirtableVerifyResponse {
                user_email: user_info.email,
                valid: true,
            })))
        }
        Err(e) => Ok(Json(ApiResponse::error(&format!(
            "Verification failed: {}",
            e
        )))),
    }
}

/// List all Airtable bases for the authenticated user
async fn list_user_bases(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<AirtableBaseInfo>>>, StatusCode> {
    let config = deployment.config().read().await;

    if !config.airtable.is_configured() {
        return Ok(Json(ApiResponse::error(
            "Airtable not configured. Please add your Personal Access Token in settings.",
        )));
    }

    let service = match AirtableService::new(config.airtable.token.as_ref().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to create Airtable service: {}",
                e
            ))));
        }
    };

    match service.list_my_bases().await {
        Ok(bases) => Ok(Json(ApiResponse::success(bases))),
        Err(e) => Ok(Json(ApiResponse::error(&format!(
            "Failed to list bases: {}",
            e
        )))),
    }
}

/// List all Airtable base connections, optionally filtered by project
async fn list_connections(
    State(deployment): State<DeploymentImpl>,
    Query(query): Query<AirtableProjectQuery>,
) -> Result<Json<ApiResponse<Vec<AirtableBase>>>, StatusCode> {
    let pool = &deployment.db().pool;

    let connections = if let Some(project_id) = query.project_id {
        AirtableBase::find_by_project_id(pool, project_id).await
    } else {
        AirtableBase::list(pool).await
    };

    match connections {
        Ok(bases) => Ok(Json(ApiResponse::success(bases))),
        Err(e) => {
            error!("Failed to list Airtable connections: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a single Airtable base connection by ID
async fn get_connection(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AirtableConnectionWithBase>>, StatusCode> {
    let pool = &deployment.db().pool;
    let config = deployment.config().read().await;

    let connection = match AirtableBase::find_by_id(pool, id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(Json(ApiResponse::error("Connection not found"))),
        Err(e) => {
            error!("Failed to get Airtable connection: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Try to fetch base info from Airtable API
    let base_info = if config.airtable.is_configured() {
        if let Ok(service) = AirtableService::new(config.airtable.token.as_ref().unwrap()) {
            // Get bases and find the matching one
            if let Ok(bases) = service.list_my_bases().await {
                bases
                    .into_iter()
                    .find(|b| b.id == connection.airtable_base_id)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(ApiResponse::success(AirtableConnectionWithBase {
        connection,
        base_info,
    })))
}

/// Create a new Airtable base connection
async fn create_connection(
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAirtableBase>,
) -> Result<Json<ApiResponse<AirtableBase>>, StatusCode> {
    let pool = &deployment.db().pool;
    let config = deployment.config().read().await;

    // Optionally fetch base name from Airtable if not provided
    let mut create_data = payload;
    if create_data.airtable_base_name.is_none() && config.airtable.is_configured() {
        if let Ok(service) = AirtableService::new(config.airtable.token.as_ref().unwrap()) {
            if let Ok(bases) = service.list_my_bases().await {
                if let Some(base) = bases
                    .into_iter()
                    .find(|b| b.id == create_data.airtable_base_id)
                {
                    create_data.airtable_base_name = Some(base.name);
                }
            }
        }
    }

    match AirtableBase::create(pool, create_data).await {
        Ok(base) => {
            info!(
                "Created Airtable connection {} for project {}",
                base.id, base.project_id
            );
            Ok(Json(ApiResponse::success(base)))
        }
        Err(e) => {
            error!("Failed to create Airtable connection: {}", e);
            Ok(Json(ApiResponse::error(&format!(
                "Failed to create connection: {}",
                e
            ))))
        }
    }
}

/// Update an Airtable base connection
async fn update_connection(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAirtableBase>,
) -> Result<Json<ApiResponse<AirtableBase>>, StatusCode> {
    let pool = &deployment.db().pool;

    match AirtableBase::update(pool, id, payload).await {
        Ok(base) => {
            info!("Updated Airtable connection {}", id);
            Ok(Json(ApiResponse::success(base)))
        }
        Err(e) => {
            error!("Failed to update Airtable connection: {}", e);
            Ok(Json(ApiResponse::error(&format!(
                "Failed to update connection: {}",
                e
            ))))
        }
    }
}

/// Delete an Airtable base connection
async fn delete_connection(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let pool = &deployment.db().pool;

    match AirtableBase::delete(pool, id).await {
        Ok(()) => {
            info!("Deleted Airtable connection {}", id);
            Ok(Json(ApiResponse::success(())))
        }
        Err(e) => {
            error!("Failed to delete Airtable connection: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get tables in a connected Airtable base
async fn get_base_tables(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<AirtableTable>>>, StatusCode> {
    let pool = &deployment.db().pool;
    let config = deployment.config().read().await;

    if !config.airtable.is_configured() {
        return Ok(Json(ApiResponse::error("Airtable not configured")));
    }

    let connection = match AirtableBase::find_by_id(pool, id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(Json(ApiResponse::error("Connection not found"))),
        Err(e) => {
            error!("Failed to get connection: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let service = match AirtableService::new(config.airtable.token.as_ref().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to create service: {}",
                e
            ))));
        }
    };

    match service.get_base_tables(&connection.airtable_base_id).await {
        Ok(tables) => Ok(Json(ApiResponse::success(tables))),
        Err(e) => Ok(Json(ApiResponse::error(&format!(
            "Failed to get tables: {}",
            e
        )))),
    }
}

/// Get records from a table in a connected Airtable base
async fn get_table_records(
    State(deployment): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Query(query): Query<AirtableTableQuery>,
) -> Result<Json<ApiResponse<Vec<AirtableRecord>>>, StatusCode> {
    let pool = &deployment.db().pool;
    let config = deployment.config().read().await;

    if !config.airtable.is_configured() {
        return Ok(Json(ApiResponse::error("Airtable not configured")));
    }

    let connection = match AirtableBase::find_by_id(pool, id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(Json(ApiResponse::error("Connection not found"))),
        Err(e) => {
            error!("Failed to get connection: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let table_id = match query.table_id {
        Some(id) => id,
        None => {
            return Ok(Json(ApiResponse::error(
                "table_id query parameter is required",
            )));
        }
    };

    let service = match AirtableService::new(config.airtable.token.as_ref().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to create service: {}",
                e
            ))));
        }
    };

    match service
        .get_table_records(&connection.airtable_base_id, &table_id)
        .await
    {
        Ok(records) => Ok(Json(ApiResponse::success(records))),
        Err(e) => Ok(Json(ApiResponse::error(&format!(
            "Failed to get records: {}",
            e
        )))),
    }
}

/// Import records from an Airtable table as PCG tasks
async fn import_records_from_table(
    State(deployment): State<DeploymentImpl>,
    Path(connection_id): Path<Uuid>,
    Json(payload): Json<AirtableImportRequest>,
) -> Result<Json<ApiResponse<AirtableImportResult>>, StatusCode> {
    let pool = &deployment.db().pool;
    let config = deployment.config().read().await;

    if !config.airtable.is_configured() {
        return Ok(Json(ApiResponse::error("Airtable not configured")));
    }

    let connection = match AirtableBase::find_by_id(pool, connection_id).await {
        Ok(Some(c)) => c,
        Ok(None) => return Ok(Json(ApiResponse::error("Connection not found"))),
        Err(e) => {
            error!("Failed to get connection: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let service = match AirtableService::new(config.airtable.token.as_ref().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to create service: {}",
                e
            ))));
        }
    };

    // Get table info to find the primary field
    let tables = match service.get_base_tables(&connection.airtable_base_id).await {
        Ok(t) => t,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to get table info: {}",
                e
            ))));
        }
    };

    let table = tables.iter().find(|t| t.id == payload.table_id);
    let primary_field_name = table
        .and_then(|t| t.fields.first())
        .map(|f| f.name.clone())
        .unwrap_or_else(|| "Name".to_string());

    let title_field = payload
        .title_field
        .clone()
        .unwrap_or_else(|| primary_field_name.clone());
    let description_field = payload.description_field.clone();

    // Get records from the table
    let records = match service
        .get_table_records(&connection.airtable_base_id, &payload.table_id)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to get Airtable records: {}",
                e
            ))));
        }
    };

    let mut imported_tasks = Vec::new();
    let mut skipped = 0;

    for record in records {
        // Check if already imported
        if AirtableRecordLink::find_by_airtable_record_id(pool, &record.id)
            .await
            .ok()
            .flatten()
            .is_some()
        {
            skipped += 1;
            continue;
        }

        // Extract title from the specified field
        let title = get_record_name(&record, &title_field)
            .unwrap_or_else(|| format!("Record {}", &record.id[..8]));

        // Extract description if field specified
        let description = description_field
            .as_ref()
            .and_then(|f| record.fields.get(f))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Create PCG task from Airtable record
        let task_id = Uuid::new_v4();
        let create_task = CreateTask {
            project_id: payload.project_id,
            pod_id: None,
            board_id: payload.board_id,
            title,
            description,
            parent_task_attempt: None,
            image_ids: None,
            priority: Some(Priority::Medium),
            assignee_id: None,
            assigned_agent: None,
            agent_id: None,
            assigned_mcps: None,
            created_by: "airtable_import".to_string(),
            requires_approval: Some(false),
            parent_task_id: None,
            tags: None,
            due_date: None,
            custom_properties: None,
            scheduled_start: None,
            scheduled_end: None,
        };

        let task = match Task::create(pool, &create_task, task_id).await {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to create task for record {}: {}", record.id, e);
                continue;
            }
        };

        // Build record URL
        let record_url = build_record_url(
            &connection.airtable_base_id,
            &payload.table_id,
            &record.id,
        );

        // Create the link
        let link = match AirtableRecordLink::create(
            pool,
            CreateAirtableRecordLink {
                task_id: task.id,
                airtable_record_id: record.id.clone(),
                airtable_base_id: connection.airtable_base_id.clone(),
                airtable_table_id: Some(payload.table_id.clone()),
                origin: AirtableOrigin::Airtable,
                airtable_record_url: Some(record_url),
            },
        )
        .await
        {
            Ok(l) => l,
            Err(e) => {
                error!("Failed to create task link for record {}: {}", record.id, e);
                // Delete the task we just created since we couldn't link it
                let _ = Task::delete(pool, task.id).await;
                continue;
            }
        };

        imported_tasks.push(AirtableTaskWithLink {
            task,
            airtable_link: link,
        });
    }

    // Mark connection as synced
    let _ = AirtableBase::mark_synced(pool, connection_id).await;

    info!(
        "Imported {} tasks from Airtable table {}, skipped {}",
        imported_tasks.len(),
        payload.table_id,
        skipped
    );

    Ok(Json(ApiResponse::success(AirtableImportResult {
        imported_count: imported_tasks.len(),
        skipped_count: skipped,
        tasks: imported_tasks,
    })))
}

/// Get the Airtable link for a task
async fn get_task_link(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Option<AirtableRecordLink>>>, StatusCode> {
    let pool = &deployment.db().pool;

    match AirtableRecordLink::find_by_task_id(pool, task_id).await {
        Ok(link) => Ok(Json(ApiResponse::success(link))),
        Err(e) => {
            error!("Failed to get task link: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Push a PCG task to Airtable as a new record
async fn push_task_to_airtable(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
    Json(payload): Json<AirtablePushTaskRequest>,
) -> Result<Json<ApiResponse<AirtableRecordLink>>, StatusCode> {
    let pool = &deployment.db().pool;
    let config = deployment.config().read().await;

    if !config.airtable.is_configured() {
        return Ok(Json(ApiResponse::error("Airtable not configured")));
    }

    // Check if already linked
    if AirtableRecordLink::find_by_task_id(pool, task_id)
        .await
        .ok()
        .flatten()
        .is_some()
    {
        return Ok(Json(ApiResponse::error(
            "Task is already linked to an Airtable record",
        )));
    }

    // Get the PCG task
    let task = match Task::find_by_id(pool, task_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Ok(Json(ApiResponse::error("Task not found"))),
        Err(e) => {
            error!("Failed to get task: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let service = match AirtableService::new(config.airtable.token.as_ref().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to create service: {}",
                e
            ))));
        }
    };

    // Build record fields
    let title_field = payload.title_field.unwrap_or_else(|| "Name".to_string());
    let mut fields = serde_json::json!({
        title_field: task.title
    });

    if let Some(desc_field) = &payload.description_field {
        if let Some(desc) = &task.description {
            fields[desc_field] = serde_json::Value::String(desc.clone());
        }
    }

    // Create record on Airtable
    let record = match service
        .create_record(&payload.base_id, &payload.table_id, fields)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to create Airtable record: {}",
                e
            ))));
        }
    };

    // Build record URL
    let record_url = build_record_url(&payload.base_id, &payload.table_id, &record.id);

    // Create the link
    let link = match AirtableRecordLink::create(
        pool,
        CreateAirtableRecordLink {
            task_id: task.id,
            airtable_record_id: record.id,
            airtable_base_id: payload.base_id,
            airtable_table_id: Some(payload.table_id),
            origin: AirtableOrigin::Pcg,
            airtable_record_url: Some(record_url),
        },
    )
    .await
    {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to create task link: {}", e);
            return Ok(Json(ApiResponse::error(&format!(
                "Record created but failed to link: {}",
                e
            ))));
        }
    };

    info!(
        "Pushed task {} to Airtable as record {}",
        task_id, link.airtable_record_id
    );
    Ok(Json(ApiResponse::success(link)))
}

/// Sync execution deliverables to Airtable as a comment or field update
async fn sync_deliverables_to_airtable(
    State(deployment): State<DeploymentImpl>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<ApiResponse<AirtableRecordLink>>, StatusCode> {
    let pool = &deployment.db().pool;
    let config = deployment.config().read().await;

    if !config.airtable.is_configured() {
        return Ok(Json(ApiResponse::error("Airtable not configured")));
    }

    // Get the task link
    let link = match AirtableRecordLink::find_by_task_id(pool, task_id).await {
        Ok(Some(l)) => l,
        Ok(None) => return Ok(Json(ApiResponse::error("Task is not linked to Airtable"))),
        Err(e) => {
            error!("Failed to get task link: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Only sync tasks that originated from Airtable (per user requirements)
    if link.origin_enum() != AirtableOrigin::Airtable {
        return Ok(Json(ApiResponse::error(
            "Only Airtable-originated tasks can be synced back automatically",
        )));
    }

    // Get the task for status info
    let task = match Task::find_by_id(pool, task_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Ok(Json(ApiResponse::error("Task not found"))),
        Err(e) => {
            error!("Failed to get task: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let service = match AirtableService::new(config.airtable.token.as_ref().unwrap()) {
        Ok(s) => s,
        Err(e) => {
            return Ok(Json(ApiResponse::error(&format!(
                "Failed to create service: {}",
                e
            ))));
        }
    };

    // Build the comment with execution summary
    let comment = format!(
        "**PCG Dashboard Update**\n\n\
        **Status:** {:?}\n\
        **Task ID:** {}\n\n\
        _Synced from PCG Dashboard at {}_",
        task.status,
        task_id,
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    );

    // Try to add a comment to the record
    match service
        .add_record_comment(&link.airtable_base_id, &link.airtable_record_id, &comment)
        .await
    {
        Ok(_) => {
            // Update sync status
            let _ = AirtableRecordLink::update_sync_status(
                pool,
                link.id,
                AirtableSyncStatus::Synced,
                None,
            )
            .await;

            // Get updated link
            let updated_link = AirtableRecordLink::find_by_id(pool, link.id)
                .await
                .ok()
                .flatten()
                .unwrap_or(link);

            info!("Synced deliverables for task {} to Airtable", task_id);
            Ok(Json(ApiResponse::success(updated_link)))
        }
        Err(e) => {
            // Update sync status with error
            let _ = AirtableRecordLink::update_sync_status(
                pool,
                link.id,
                AirtableSyncStatus::Error,
                Some(&e.to_string()),
            )
            .await;

            Ok(Json(ApiResponse::error(&format!(
                "Failed to sync to Airtable: {}",
                e
            ))))
        }
    }
}
