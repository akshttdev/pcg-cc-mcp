//! Admin prompt, user settings, and tool risk map handlers for Topsi

use super::*;

// ============================================================================
// Admin Prompt Management (production-safe, admin-only)
// ============================================================================

/// Response for admin prompt endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct AdminPromptResponse {
    pub prompt: Option<String>,
    pub mode: String,
    pub prompt_sudolang: Option<String>,
    pub autonomy_level: String,
}

/// Request to update admin prompt
#[derive(Debug, Deserialize)]
pub struct UpdateAdminPromptRequest {
    pub prompt: Option<String>,
    pub mode: Option<String>,
    pub prompt_sudolang: Option<String>,
    pub autonomy_level: Option<String>,
}

/// GET /topsi/admin/prompt — returns current system prompt settings
pub async fn get_admin_prompt(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
) -> Result<Json<AdminPromptResponse>, ApiError> {
    if !access_ctx.is_admin {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    let pool = &state.db().pool;
    let prompt = SystemSetting::get(pool, "topsi_system_prompt")
        .await
        .ok()
        .flatten();
    let mode = SystemSetting::get(pool, "topsi_prompt_mode")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "standard".to_string());
    let prompt_sudolang = SystemSetting::get(pool, "topsi_system_prompt_sudolang")
        .await
        .ok()
        .flatten();
    let autonomy_level = SystemSetting::get(pool, "topsi_autonomy_level")
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "supervised".to_string());

    Ok(Json(AdminPromptResponse {
        prompt,
        mode,
        prompt_sudolang,
        autonomy_level,
    }))
}

/// PUT /topsi/admin/prompt — update system prompt settings
pub async fn update_admin_prompt(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
    Json(request): Json<UpdateAdminPromptRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    if !access_ctx.is_admin {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    let pool = &state.db().pool;
    let user_id = access_ctx.user_id.to_string();

    if let Some(prompt) = &request.prompt {
        SystemSetting::set(pool, "topsi_system_prompt", prompt, Some(&user_id))
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to save prompt: {}", e)))?;
    }

    if let Some(mode) = &request.mode {
        if !matches!(mode.as_str(), "standard" | "sudolang") {
            return Err(ApiError::BadRequest(format!(
                "Invalid prompt mode: '{}'. Must be 'standard' or 'sudolang'",
                mode
            )));
        }
        SystemSetting::set(pool, "topsi_prompt_mode", mode, Some(&user_id))
            .await
            .map_err(|e| ApiError::InternalError(format!("Failed to save mode: {}", e)))?;
    }

    if let Some(sudolang) = &request.prompt_sudolang {
        SystemSetting::set(
            pool,
            "topsi_system_prompt_sudolang",
            sudolang,
            Some(&user_id),
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to save sudolang prompt: {}", e)))?;
    }

    if let Some(ref level) = request.autonomy_level {
        if !matches!(
            level.as_str(),
            "full" | "supervised" | "approval_required" | "manual"
        ) {
            return Err(ApiError::BadRequest(format!(
                "Invalid autonomy level: '{}'. Must be 'full', 'supervised', 'approval_required', or 'manual'",
                level
            )));
        }
        SystemSetting::set(pool, "topsi_autonomy_level", level, Some(&user_id))
            .await
            .map_err(|e| {
                ApiError::InternalError(format!("Failed to save autonomy level: {}", e))
            })?;

        // Update the running Topsi instance config
        let topsi_instance = TOPSI_INSTANCE.get();
        if let Some(instance_lock) = topsi_instance {
            let mut instance = instance_lock.write().await;
            if let Some(ref mut agent) = *instance {
                agent.config.autonomy_level = topsi::config::AutonomyLevel::from_str(level);
            }
        }
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

// ============================================================================
// Per-User Settings
// ============================================================================

/// Request to update user settings
#[derive(Debug, Deserialize)]
pub struct UpdateUserSettingsRequest {
    pub default_confirmation_mode: Option<String>,
    pub per_tool_overrides: Option<serde_json::Value>,
    pub auto_approve_timeout_minutes: Option<i64>,
}

/// GET /topsi/user-settings — returns current user's Topsi settings
pub async fn get_user_settings(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
) -> Result<Json<TopsiUserSettings>, ApiError> {
    let pool = &state.db().pool;
    let user_id = access_ctx.user_id.to_string();
    let settings = TopsiUserSettings::get_or_default(pool, &user_id).await;
    Ok(Json(settings))
}

/// PUT /topsi/user-settings — update current user's Topsi settings
pub async fn update_user_settings(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
    Json(request): Json<UpdateUserSettingsRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pool = &state.db().pool;
    let user_id = access_ctx.user_id.to_string();

    let mode = request
        .default_confirmation_mode
        .as_deref()
        .unwrap_or("confirm_destructive");
    if !matches!(
        mode,
        "always_confirm" | "confirm_destructive" | "autonomous"
    ) {
        return Err(ApiError::BadRequest(format!(
            "Invalid confirmation mode: '{}'. Must be 'always_confirm', 'confirm_destructive', or 'autonomous'",
            mode
        )));
    }

    // Validate per_tool_overrides shape: must be a flat object of string -> string
    if let Some(ref overrides) = request.per_tool_overrides {
        if let Some(obj) = overrides.as_object() {
            for (key, val) in obj {
                if !val.is_string() {
                    return Err(ApiError::BadRequest(format!(
                        "per_tool_overrides value for '{}' must be a string",
                        key
                    )));
                }
            }
        } else {
            return Err(ApiError::BadRequest(
                "per_tool_overrides must be a JSON object".to_string(),
            ));
        }
    }

    let overrides_json = request.per_tool_overrides.map(|v| v.to_string());

    TopsiUserSettings::upsert(
        pool,
        &user_id,
        mode,
        overrides_json.as_deref(),
        request.auto_approve_timeout_minutes,
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to save settings: {}", e)))?;

    Ok(Json(serde_json::json!({ "success": true })))
}

/// GET /topsi/tools — returns all Topsi tools grouped by risk level
pub async fn get_tool_risk_map() -> Json<serde_json::Value> {
    let tool_names = topsi::tools::get_tool_names();

    let mut red = Vec::new();
    let mut yellow = Vec::new();
    let mut green = Vec::new();

    for name in tool_names {
        let risk = classify_tool_risk(&name);
        match risk {
            db::models::topsi_user_settings::ToolRisk::Red => red.push(name),
            db::models::topsi_user_settings::ToolRisk::Yellow => yellow.push(name),
            db::models::topsi_user_settings::ToolRisk::Green => green.push(name),
        }
    }

    Json(serde_json::json!({
        "red": { "label": "Destructive", "tools": red },
        "yellow": { "label": "Create / Update", "tools": yellow },
        "green": { "label": "Read-only", "tools": green },
    }))
}
