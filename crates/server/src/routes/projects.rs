use std::path::Path as StdPath;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware::from_fn_with_state,
    response::Json as ResponseJson,
    routing::{get, patch, post, put},
};
use db::models::{
    brand_profile::{BrandProfile, UpsertBrandProfile},
    project::{CreateProject, Project, ProjectError, SearchMatchType, SearchResult, UpdateProject},
    project_asset::{CreateProjectAsset, ProjectAsset, UpdateProjectAsset},
    project_board::ProjectBoard,
    project_pod::{CreateProjectPod, ProjectPod, UpdateProjectPod},
};
use deployment::Deployment;
use ignore::WalkBuilder;
use services::services::{
    file_ranker::FileRanker,
    file_search_cache::{CacheError, SearchMode, SearchQuery},
    git::GitBranch,
};
use utils::{path::expand_tilde, response::ApiResponse};
use uuid::Uuid;

use crate::{
    DeploymentImpl,
    error::ApiError,
    middleware::{
        access_control::{AccessContext, ProjectRole},
        load_project_middleware,
    },
};

pub async fn get_projects(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<Project>>>, ApiError> {
    // If admin, return all projects
    if access_context.is_admin {
        let projects = Project::find_all(&deployment.db().pool).await?;
        return Ok(ResponseJson(ApiResponse::success(projects)));
    }

    // For regular users, get only projects they have access to
    let user_id_bytes = access_context.user_id.as_bytes().to_vec();

    #[derive(sqlx::FromRow)]
    struct ProjectRow {
        id: String,
    }

    let project_ids: Vec<String> = sqlx::query_as::<_, ProjectRow>(
        "SELECT DISTINCT project_id as id FROM project_members WHERE user_id = ?",
    )
    .bind(&user_id_bytes)
    .fetch_all(&deployment.db().pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to fetch user projects: {}", e)))?
    .into_iter()
    .map(|row| row.id)
    .collect();

    if project_ids.is_empty() {
        return Ok(ResponseJson(ApiResponse::success(vec![])));
    }

    // Fetch all accessible projects
    let mut projects = Vec::new();
    for project_id in project_ids {
        if let Ok(uuid) = Uuid::parse_str(&project_id) {
            if let Ok(Some(project)) = Project::find_by_id(&deployment.db().pool, uuid).await {
                projects.push(project);
            }
        }
    }

    Ok(ResponseJson(ApiResponse::success(projects)))
}

pub async fn get_project(
    Extension(project): Extension<Project>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(project)))
}

pub async fn get_project_branches(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<GitBranch>>>, ApiError> {
    let branches = deployment.git().get_all_branches(&project.git_repo_path)?;
    Ok(ResponseJson(ApiResponse::success(branches)))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreatePodPayload {
    pub title: String,
    pub description: Option<String>,
    pub status: Option<String>,
    pub lead: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdatePodPayload {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub lead: Option<String>,
}

pub async fn list_project_pods(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ProjectPod>>>, ApiError> {
    let pods = ProjectPod::find_by_project(&deployment.db().pool, project.id).await?;
    Ok(ResponseJson(ApiResponse::success(pods)))
}

pub async fn create_project_pod(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreatePodPayload>,
) -> Result<ResponseJson<ApiResponse<ProjectPod>>, ApiError> {
    let pod = ProjectPod::create(
        &deployment.db().pool,
        Uuid::new_v4(),
        &CreateProjectPod {
            project_id: project.id,
            title: payload.title,
            description: payload.description,
            status: payload.status,
            lead: payload.lead,
        },
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(pod)))
}

pub async fn update_project_pod(
    Path(pod_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdatePodPayload>,
) -> Result<ResponseJson<ApiResponse<ProjectPod>>, ApiError> {
    let pod = ProjectPod::update(
        &deployment.db().pool,
        pod_id,
        &UpdateProjectPod {
            title: payload.title,
            description: payload.description,
            status: payload.status,
            lead: payload.lead,
        },
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(pod)))
}

pub async fn delete_project_pod(
    Path(pod_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let deleted = ProjectPod::delete(&deployment.db().pool, pod_id).await?;
    if deleted == 0 {
        return Err(ApiError::NotFound("Pod not found".to_string()));
    }

    Ok(ResponseJson(ApiResponse::success(())))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateAssetPayload {
    pub pod_id: Option<Uuid>,
    pub board_id: Option<Uuid>,
    pub category: Option<String>,
    pub scope: Option<String>,
    pub name: String,
    pub storage_path: String,
    pub checksum: Option<String>,
    pub byte_size: Option<i64>,
    pub mime_type: Option<String>,
    pub metadata: Option<String>,
    pub uploaded_by: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UpdateAssetPayload {
    pub pod_id: Option<Uuid>,
    pub board_id: Option<Uuid>,
    pub category: Option<String>,
    pub scope: Option<String>,
    pub name: Option<String>,
    pub storage_path: Option<String>,
    pub checksum: Option<String>,
    pub byte_size: Option<i64>,
    pub mime_type: Option<String>,
    pub metadata: Option<String>,
}

pub async fn list_project_assets(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Vec<ProjectAsset>>>, ApiError> {
    let assets = ProjectAsset::find_by_project(&deployment.db().pool, project.id).await?;

    if assets.is_empty() {
        return Err(ApiError::NotFound(
            "No assets found for the project".to_string(),
        ));
    }

    Ok(ResponseJson(ApiResponse::success(assets)))
}

pub async fn create_project_asset(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateAssetPayload>,
) -> Result<ResponseJson<ApiResponse<ProjectAsset>>, ApiError> {
    let CreateAssetPayload {
        pod_id,
        board_id,
        category,
        scope,
        name,
        storage_path,
        checksum,
        byte_size,
        mime_type,
        metadata,
        uploaded_by,
    } = payload;

    if let Some(board_id) = board_id {
        match ProjectBoard::find_by_id(&deployment.db().pool, board_id).await? {
            Some(board) if board.project_id == project.id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }

    let asset = ProjectAsset::create(
        &deployment.db().pool,
        Uuid::new_v4(),
        &CreateProjectAsset {
            project_id: project.id,
            pod_id,
            board_id,
            category,
            scope,
            name,
            storage_path,
            checksum,
            byte_size,
            mime_type,
            metadata,
            uploaded_by,
        },
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(asset)))
}

pub async fn update_project_asset(
    Path(asset_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateAssetPayload>,
) -> Result<ResponseJson<ApiResponse<ProjectAsset>>, ApiError> {
    let UpdateAssetPayload {
        pod_id,
        board_id,
        category,
        scope,
        name,
        storage_path,
        checksum,
        byte_size,
        mime_type,
        metadata,
    } = payload;

    let existing_asset = ProjectAsset::find_by_id(&deployment.db().pool, asset_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Asset not found".to_string()))?;

    if let Some(board_id) = board_id {
        match ProjectBoard::find_by_id(&deployment.db().pool, board_id).await? {
            Some(board) if board.project_id == existing_asset.project_id => {}
            Some(_) => {
                return Err(ApiError::BadRequest(
                    "Board does not belong to this project".to_string(),
                ));
            }
            None => return Err(ApiError::NotFound("Board not found".to_string())),
        }
    }

    let asset = ProjectAsset::update(
        &deployment.db().pool,
        asset_id,
        &UpdateProjectAsset {
            pod_id,
            board_id,
            category,
            scope,
            name,
            storage_path,
            checksum,
            byte_size,
            mime_type,
            metadata,
        },
    )
    .await?;

    Ok(ResponseJson(ApiResponse::success(asset)))
}

pub async fn delete_project_asset(
    Path(asset_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, ApiError> {
    let deleted = ProjectAsset::delete(&deployment.db().pool, asset_id).await?;
    if deleted == 0 {
        return Err(ApiError::NotFound("Asset not found".to_string()));
    }

    Ok(ResponseJson(ApiResponse::success(())))
}

pub async fn create_project(
    Extension(access_context): Extension<AccessContext>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<CreateProject>,
) -> Result<ResponseJson<ApiResponse<Project>>, ApiError> {
    // Only admins can create projects
    access_context.require_admin()?;

    let id = Uuid::new_v4();
    let CreateProject {
        name,
        git_repo_path,
        setup_script,
        dev_script,
        cleanup_script,
        copy_files,
        use_existing_repo,
    } = payload;
    tracing::debug!("Creating project '{}'", name);

    // Validate and setup git repository
    let path = std::path::absolute(expand_tilde(&git_repo_path))?;
    // Check if git repo path is already used by another project
    match Project::find_by_git_repo_path(&deployment.db().pool, path.to_string_lossy().as_ref())
        .await
    {
        Ok(Some(_)) => {
            return Ok(ResponseJson(ApiResponse::error(
                "A project with this git repository path already exists",
            )));
        }
        Ok(None) => {
            // Path is available, continue
        }
        Err(e) => {
            return Err(ProjectError::GitRepoCheckFailed(e.to_string()).into());
        }
    }

    if use_existing_repo {
        // For existing repos, validate that the path exists and is a git repository
        if !path.exists() {
            return Ok(ResponseJson(ApiResponse::error(
                "The specified path does not exist",
            )));
        }

        if !path.is_dir() {
            return Ok(ResponseJson(ApiResponse::error(
                "The specified path is not a directory",
            )));
        }

        if !path.join(".git").exists() {
            return Ok(ResponseJson(ApiResponse::error(
                "The specified directory is not a git repository",
            )));
        }

        // Ensure existing repo has a main branch if it's empty
        if let Err(e) = deployment.git().ensure_main_branch_exists(&path) {
            tracing::error!("Failed to ensure main branch exists: {}", e);
            return Ok(ResponseJson(ApiResponse::error(&format!(
                "Failed to ensure main branch exists: {}",
                e
            ))));
        }
    } else {
        // For new repos, create directory and initialize git

        // Create directory if it doesn't exist
        if !path.exists()
            && let Err(e) = std::fs::create_dir_all(&path)
        {
            tracing::error!("Failed to create directory: {}", e);
            return Ok(ResponseJson(ApiResponse::error(&format!(
                "Failed to create directory: {}",
                e
            ))));
        }

        // Check if it's already a git repo, if not initialize it
        if !path.join(".git").exists()
            && let Err(e) = deployment.git().initialize_repo_with_main_branch(&path)
        {
            tracing::error!("Failed to initialize git repository: {}", e);
            return Ok(ResponseJson(ApiResponse::error(&format!(
                "Failed to initialize git repository: {}",
                e
            ))));
        }
    }

    match Project::create(
        &deployment.db().pool,
        &CreateProject {
            name,
            git_repo_path: path.to_string_lossy().to_string(),
            use_existing_repo,
            setup_script,
            dev_script,
            cleanup_script,
            copy_files,
        },
        id,
    )
    .await
    {
        Ok(project) => {
            if let Err(e) =
                ProjectBoard::ensure_default_boards(&deployment.db().pool, project.id).await
            {
                tracing::error!(
                    "Failed to seed default boards for project {}: {}",
                    project.id,
                    e
                );
            }
            // Track project creation event
            deployment
                .track_if_analytics_allowed(
                    "project_created",
                    serde_json::json!({
                        "project_id": project.id.to_string(),
                        "use_existing_repo": use_existing_repo,
                        "has_setup_script": project.setup_script.is_some(),
                        "has_dev_script": project.dev_script.is_some(),
                        "source": "manual",
                    }),
                )
                .await;

            Ok(ResponseJson(ApiResponse::success(project)))
        }
        Err(e) => Err(ProjectError::CreateFailed(e.to_string()).into()),
    }
}

pub async fn update_project(
    Extension(access_context): Extension<AccessContext>,
    Extension(existing_project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpdateProject>,
) -> Result<ResponseJson<ApiResponse<Project>>, StatusCode> {
    // Only admins or project owners can update projects
    if !access_context.is_admin {
        match access_context
            .check_project_access(
                &deployment.db().pool,
                &existing_project.id.to_string(),
                ProjectRole::Owner,
            )
            .await
        {
            Ok(_) => {
                // User has owner access, allow update
            }
            Err(_) => {
                tracing::warn!(
                    "User {} denied update access to project {}",
                    access_context.user_id,
                    existing_project.id
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Destructure payload to handle field updates.
    // This allows us to treat `None` from the payload as an explicit `null` to clear a field,
    // as the frontend currently sends all fields on update.
    let UpdateProject {
        name,
        git_repo_path,
        setup_script,
        dev_script,
        cleanup_script,
        copy_files,
    } = payload;
    // If git_repo_path is being changed, check if the new path is already used by another project
    let git_repo_path = if let Some(new_git_repo_path) = git_repo_path.map(|s| expand_tilde(&s))
        && new_git_repo_path != existing_project.git_repo_path
    {
        match Project::find_by_git_repo_path_excluding_id(
            &deployment.db().pool,
            new_git_repo_path.to_string_lossy().as_ref(),
            existing_project.id,
        )
        .await
        {
            Ok(Some(_)) => {
                return Ok(ResponseJson(ApiResponse::error(
                    "A project with this git repository path already exists",
                )));
            }
            Ok(None) => new_git_repo_path,
            Err(e) => {
                tracing::error!("Failed to check for existing git repo path: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        existing_project.git_repo_path
    };

    match Project::update(
        &deployment.db().pool,
        existing_project.id,
        name.unwrap_or(existing_project.name),
        git_repo_path.to_string_lossy().to_string(),
        setup_script,
        dev_script,
        cleanup_script,
        copy_files,
    )
    .await
    {
        Ok(project) => Ok(ResponseJson(ApiResponse::success(project))),
        Err(e) => {
            tracing::error!("Failed to update project: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn delete_project(
    Extension(access_context): Extension<AccessContext>,
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<()>>, StatusCode> {
    // Only admins or project owners can delete projects
    if !access_context.is_admin {
        match access_context
            .check_project_access(
                &deployment.db().pool,
                &project.id.to_string(),
                ProjectRole::Owner,
            )
            .await
        {
            Ok(_) => {
                // User has owner access, allow delete
            }
            Err(_) => {
                tracing::warn!(
                    "User {} denied delete access to project {}",
                    access_context.user_id,
                    project.id
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    match Project::delete(&deployment.db().pool, project.id).await {
        Ok(rows_affected) => {
            if rows_affected == 0 {
                Err(StatusCode::NOT_FOUND)
            } else {
                Ok(ResponseJson(ApiResponse::success(())))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete project: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct OpenEditorRequest {
    editor_type: Option<String>,
}

pub async fn open_project_in_editor(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<Option<OpenEditorRequest>>,
) -> Result<ResponseJson<ApiResponse<()>>, StatusCode> {
    let path = project.git_repo_path.to_string_lossy();

    let editor_config = {
        let config = deployment.config().read().await;
        let editor_type_str = payload.as_ref().and_then(|req| req.editor_type.as_deref());
        config.editor.with_override(editor_type_str)
    };

    match editor_config.open_file(&path) {
        Ok(_) => {
            tracing::info!("Opened editor for project {} at path: {}", project.id, path);
            Ok(ResponseJson(ApiResponse::success(())))
        }
        Err(e) => {
            tracing::error!("Failed to open editor for project {}: {}", project.id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn search_project_files(
    State(deployment): State<DeploymentImpl>,
    Extension(project): Extension<Project>,
    Query(search_query): Query<SearchQuery>,
) -> Result<ResponseJson<ApiResponse<Vec<SearchResult>>>, StatusCode> {
    let query = search_query.q.trim();
    let mode = search_query.mode;

    if query.is_empty() {
        return Ok(ResponseJson(ApiResponse::error(
            "Query parameter 'q' is required and cannot be empty",
        )));
    }

    let repo_path = &project.git_repo_path;
    let file_search_cache = deployment.file_search_cache();

    // Try cache first
    match file_search_cache
        .search(repo_path, query, mode.clone())
        .await
    {
        Ok(results) => {
            tracing::debug!(
                "Cache hit for repo {:?}, query: {}, mode: {:?}",
                repo_path,
                query,
                mode
            );
            Ok(ResponseJson(ApiResponse::success(results)))
        }
        Err(CacheError::Miss) => {
            // Cache miss - fall back to filesystem search
            tracing::debug!(
                "Cache miss for repo {:?}, query: {}, mode: {:?}",
                repo_path,
                query,
                mode
            );
            match search_files_in_repo(&project.git_repo_path.to_string_lossy(), query, mode).await
            {
                Ok(results) => Ok(ResponseJson(ApiResponse::success(results))),
                Err(e) => {
                    tracing::error!("Failed to search files: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(CacheError::BuildError(e)) => {
            tracing::error!("Cache build error for repo {:?}: {}", repo_path, e);
            // Fall back to filesystem search
            match search_files_in_repo(&project.git_repo_path.to_string_lossy(), query, mode).await
            {
                Ok(results) => Ok(ResponseJson(ApiResponse::success(results))),
                Err(e) => {
                    tracing::error!("Failed to search files: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
    }
}

async fn search_files_in_repo(
    repo_path: &str,
    query: &str,
    mode: SearchMode,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
    let repo_path = StdPath::new(repo_path);

    if !repo_path.exists() {
        return Err("Repository path does not exist".into());
    }

    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    // Configure walker based on mode
    let walker = match mode {
        SearchMode::Settings => {
            // Settings mode: Include ignored files but exclude performance killers
            WalkBuilder::new(repo_path)
                .git_ignore(false) // Include ignored files like .env
                .git_global(false)
                .git_exclude(false)
                .hidden(false)
                .filter_entry(|entry| {
                    let name = entry.file_name().to_string_lossy();
                    // Always exclude .git directories and performance killers
                    name != ".git"
                        && name != "node_modules"
                        && name != "target"
                        && name != "dist"
                        && name != "build"
                })
                .build()
        }
        SearchMode::TaskForm => {
            // Task form mode: Respect gitignore (cleaner results)
            WalkBuilder::new(repo_path)
                .git_ignore(true) // Respect .gitignore
                .git_global(true) // Respect global .gitignore
                .git_exclude(true) // Respect .git/info/exclude
                .hidden(false) // Still show hidden files like .env (if not gitignored)
                .filter_entry(|entry| {
                    let name = entry.file_name().to_string_lossy();
                    name != ".git"
                })
                .build()
        }
    };

    for result in walker {
        let entry = result?;
        let path = entry.path();

        // Skip the root directory itself
        if path == repo_path {
            continue;
        }

        let relative_path = path.strip_prefix(repo_path)?;
        let relative_path_str = relative_path.to_string_lossy().to_lowercase();

        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // Check for matches
        if file_name.contains(&query_lower) {
            results.push(SearchResult {
                path: relative_path.to_string_lossy().to_string(),
                is_file: path.is_file(),
                match_type: SearchMatchType::FileName,
            });
        } else if relative_path_str.contains(&query_lower) {
            // Check if it's a directory name match or full path match
            let match_type = if path
                .parent()
                .and_then(|p| p.file_name())
                .map(|name| name.to_string_lossy().to_lowercase())
                .unwrap_or_default()
                .contains(&query_lower)
            {
                SearchMatchType::DirectoryName
            } else {
                SearchMatchType::FullPath
            };

            results.push(SearchResult {
                path: relative_path.to_string_lossy().to_string(),
                is_file: path.is_file(),
                match_type,
            });
        }
    }

    // Apply git history-based ranking
    let file_ranker = FileRanker::new();
    match file_ranker.get_stats(repo_path).await {
        Ok(stats) => {
            // Re-rank results using git history
            file_ranker.rerank(&mut results, &stats);
        }
        Err(e) => {
            tracing::warn!(
                "Failed to get git stats for ranking, using basic sort: {}",
                e
            );
            // Fallback to basic priority sorting
            results.sort_by(|a, b| {
                let priority = |match_type: &SearchMatchType| match match_type {
                    SearchMatchType::FileName => 0,
                    SearchMatchType::DirectoryName => 1,
                    SearchMatchType::FullPath => 2,
                };

                priority(&a.match_type)
                    .cmp(&priority(&b.match_type))
                    .then_with(|| a.path.cmp(&b.path))
            });
        }
    }

    // Limit to top 10 results
    results.truncate(10);

    Ok(results)
}

// ============================================================================
// Brand Profile Endpoints
// ============================================================================

pub async fn get_brand_profile(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
) -> Result<ResponseJson<ApiResponse<Option<BrandProfile>>>, ApiError> {
    let profile = BrandProfile::find_by_project(&deployment.db().pool, project.id).await?;
    Ok(ResponseJson(ApiResponse::success(profile)))
}

pub async fn upsert_brand_profile(
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<UpsertBrandProfile>,
) -> Result<ResponseJson<ApiResponse<BrandProfile>>, ApiError> {
    let profile = BrandProfile::upsert(&deployment.db().pool, project.id, &payload).await?;
    Ok(ResponseJson(ApiResponse::success(profile)))
}

// ============================================================================
// VIBE Budget Management
// ============================================================================

#[derive(Debug, serde::Deserialize)]
pub struct SetVibeBudgetRequest {
    /// Budget limit in VIBE (1 VIBE = $0.001 USD). None means unlimited.
    pub vibe_budget_limit: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
pub struct VibeBudgetResponse {
    pub vibe_budget_limit: Option<i64>,
    pub vibe_spent_amount: i64,
    pub vibe_remaining: Option<i64>,
}

/// Get the VIBE budget status for a project
pub async fn get_vibe_budget(
    Extension(project): Extension<Project>,
) -> Result<ResponseJson<ApiResponse<VibeBudgetResponse>>, ApiError> {
    Ok(ResponseJson(ApiResponse::success(VibeBudgetResponse {
        vibe_budget_limit: project.vibe_budget_limit,
        vibe_spent_amount: project.vibe_spent_amount,
        vibe_remaining: project.remaining_vibe(),
    })))
}

/// Set the VIBE budget limit for a project (admin/owner only)
pub async fn set_vibe_budget(
    Extension(access_context): Extension<AccessContext>,
    Extension(project): Extension<Project>,
    State(deployment): State<DeploymentImpl>,
    Json(payload): Json<SetVibeBudgetRequest>,
) -> Result<ResponseJson<ApiResponse<VibeBudgetResponse>>, ApiError> {
    // Only admins or project owners can set budget
    if !access_context.is_admin {
        access_context
            .check_project_access(
                &deployment.db().pool,
                &project.id.to_string(),
                ProjectRole::Owner,
            )
            .await
            .map_err(|_| ApiError::Forbidden("Only project owners can set VIBE budget".into()))?;
    }

    Project::set_vibe_budget(&deployment.db().pool, project.id, payload.vibe_budget_limit).await?;

    // Fetch updated project to return current state
    let updated_project = Project::find_by_id(&deployment.db().pool, project.id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Project not found".into()))?;

    Ok(ResponseJson(ApiResponse::success(VibeBudgetResponse {
        vibe_budget_limit: updated_project.vibe_budget_limit,
        vibe_spent_amount: updated_project.vibe_spent_amount,
        vibe_remaining: updated_project.remaining_vibe(),
    })))
}

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    let project_id_router = Router::new()
        .route(
            "/",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route("/branches", get(get_project_branches))
        .route("/search", get(search_project_files))
        .route("/open-editor", post(open_project_in_editor))
        .route("/pods", get(list_project_pods).post(create_project_pod))
        .route(
            "/pods/{pod_id}",
            patch(update_project_pod).delete(delete_project_pod),
        )
        .route(
            "/assets",
            get(list_project_assets).post(create_project_asset),
        )
        .route(
            "/assets/{asset_id}",
            patch(update_project_asset).delete(delete_project_asset),
        )
        .route(
            "/brand-profile",
            get(get_brand_profile).put(upsert_brand_profile),
        )
        .route(
            "/budget",
            get(get_vibe_budget).put(set_vibe_budget),
        )
        .merge(crate::routes::project_boards::router(deployment))
        .merge(crate::routes::project_controllers::router(deployment))
        .layer(from_fn_with_state(
            deployment.clone(),
            load_project_middleware,
        ));

    let projects_router = Router::new()
        .route("/", get(get_projects).post(create_project))
        .nest("/{id}", project_id_router)
        .layer(from_fn_with_state(
            deployment.clone(),
            crate::middleware::require_auth,
        ));

    Router::new().nest("/projects", projects_router)
}
