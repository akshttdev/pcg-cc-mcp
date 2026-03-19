//! Topology overview, issues, and recommendations handlers for Topsi

use super::*;

/// Get topology overview (all accessible projects)
pub async fn get_topology_overview(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<TopologyOverviewResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetTopology { project_id: None });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get topology: {}", e)))?;

    // Build overview from response
    let summary = response.topology_summary.unwrap_or(TopologySummary {
        node_count: 0,
        edge_count: 0,
        cluster_count: 0,
        active_routes: 0,
        unresolved_issues: 0,
        nodes_by_type: Vec::new(),
        edges_by_type: Vec::new(),
        health_score: 1.0,
    });

    Ok(Json(TopologyOverviewResponse {
        summaries: Vec::new(), // Would need to return per-project summaries
        total_nodes: summary.node_count,
        total_edges: summary.edge_count,
        total_clusters: summary.cluster_count,
        system_health: Some(summary.health_score),
    }))
}

/// Get topology for a specific project
pub async fn get_project_topology(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<TopologyOverviewResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Verify access
    if !topsi
        .access_control
        .can_access_project(&user_context, project_id)
        .await
    {
        return Err(ApiError::Forbidden(format!(
            "Access denied to project {}",
            project_id
        )));
    }

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetTopology {
        project_id: Some(project_id),
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get topology: {}", e)))?;

    let summary = response.topology_summary.unwrap_or_default();

    Ok(Json(TopologyOverviewResponse {
        summaries: Vec::new(),
        total_nodes: summary.node_count,
        total_edges: summary.edge_count,
        total_clusters: summary.cluster_count,
        system_health: Some(summary.health_score),
    }))
}

/// Detect issues across accessible projects
pub async fn detect_issues(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<IssuesResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::DetectIssues { project_id: None });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to detect issues: {}", e)))?;

    let critical_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "critical")
        .count();
    let warning_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "warning")
        .count();

    Ok(Json(IssuesResponse {
        total_count: response.issues.len(),
        critical_count,
        warning_count,
        issues: response.issues,
    }))
}

/// Detect issues for a specific project
pub async fn detect_project_issues(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<Uuid>,
) -> Result<Json<IssuesResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Verify access
    if !topsi
        .access_control
        .can_access_project(&user_context, project_id)
        .await
    {
        return Err(ApiError::Forbidden(format!(
            "Access denied to project {}",
            project_id
        )));
    }

    let topsi_request = TopsiRequest::new(TopsiRequestType::DetectIssues {
        project_id: Some(project_id),
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to detect issues: {}", e)))?;

    let critical_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "critical")
        .count();
    let warning_count = response
        .issues
        .iter()
        .filter(|i| i.severity == "warning")
        .count();

    Ok(Json(IssuesResponse {
        total_count: response.issues.len(),
        critical_count,
        warning_count,
        issues: response.issues,
    }))
}

/// Get accessible projects for current user
pub async fn get_accessible_projects(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
) -> Result<Json<AccessibleProjectsResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let projects = topsi
        .access_control
        .get_accessible_projects(&user_context)
        .await;

    let access_level = if user_context.is_admin {
        "admin"
    } else if projects.is_empty() {
        "none"
    } else {
        "user"
    };

    Ok(Json(AccessibleProjectsResponse {
        projects,
        access_level: access_level.to_string(),
        is_admin: user_context.is_admin,
    }))
}

/// Get recommendations across all accessible projects
pub async fn get_recommendations(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Query(params): Query<RecommendationsQuery>,
) -> Result<Json<RecommendationBatch>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetRecommendations {
        project_id: None,
        max_count: params.max_count,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get recommendations: {}", e)))?;

    // Parse the JSON message back into a RecommendationBatch
    let batch: RecommendationBatch = serde_json::from_str(&response.message)
        .map_err(|e| ApiError::InternalError(format!("Failed to parse recommendations: {}", e)))?;

    Ok(Json(batch))
}

/// Get recommendations for a specific project
pub async fn get_project_recommendations(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Path(project_id): Path<Uuid>,
    Query(params): Query<RecommendationsQuery>,
) -> Result<Json<RecommendationBatch>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    // Verify access
    if !topsi
        .access_control
        .can_access_project(&user_context, project_id)
        .await
    {
        return Err(ApiError::Forbidden(format!(
            "Access denied to project {}",
            project_id
        )));
    }

    let topsi_request = TopsiRequest::new(TopsiRequestType::GetRecommendations {
        project_id: Some(project_id),
        max_count: params.max_count,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to get recommendations: {}", e)))?;

    let batch: RecommendationBatch = serde_json::from_str(&response.message)
        .map_err(|e| ApiError::InternalError(format!("Failed to parse recommendations: {}", e)))?;

    Ok(Json(batch))
}
