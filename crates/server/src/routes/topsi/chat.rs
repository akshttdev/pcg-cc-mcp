//! Chat and command execution handlers for Topsi

use db::models::media_asset::MediaAsset;

use super::*;

/// Chat with Topsi
pub async fn chat_with_topsi(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
    headers: axum::http::HeaderMap,
    Json(request): Json<TopsiChatRequest>,
) -> Result<Json<TopsiResponse>, ApiError> {
    tracing::info!("Received chat request: {:?}", request.message);

    let pool = state.db().pool.clone();

    // Resolve project to bill against
    let billing_project_id = match request.project_id {
        Some(pid) => Some(pid),
        None => {
            let home: Option<Vec<u8>> =
                sqlx::query_scalar("SELECT home_project_id FROM users WHERE id = ?")
                    .bind(access_ctx.user_id.as_str())
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten();
            home.and_then(|bytes| Uuid::from_slice(&bytes).ok())
        }
    };

    // VIBE Balance Check
    if let Some(project_id) = billing_project_id {
        crate::helpers::billing::ensure_vibe_balance(&pool, project_id).await?;
    }

    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    if !topsi.is_active().await {
        return Err(ApiError::BadRequest("Topsi is not active".to_string()));
    }

    // Use DB agent ID for FK-safe conversation persistence
    let topsi_agent_id = super::TOPSI_DB_AGENT_ID.get().copied().unwrap_or(topsi.id);

    // Get REAL user context from authentication
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session_id = request.session_id.clone();

    // Build attachment context if attachments provided
    let attachment_context = if let Some(ref attachment_ids) = request.attachment_ids {
        if !attachment_ids.is_empty() {
            match build_attachment_context(&pool, attachment_ids).await {
                Ok(ctx) => Some(ctx),
                Err(e) => {
                    tracing::warn!("Failed to build attachment context: {}", e);
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Merge attachment context into existing context
    let merged_context = match (request.context, attachment_context) {
        (Some(mut ctx), Some(attachments)) => {
            if let Some(obj) = ctx.as_object_mut() {
                obj.insert("attachments".to_string(), attachments);
            }
            Some(ctx)
        }
        (None, Some(attachments)) => Some(serde_json::json!({ "attachments": attachments })),
        (Some(ctx), None) => Some(ctx),
        (None, None) => None,
    };

    let topsi_request = TopsiRequest::new(TopsiRequestType::Chat {
        message: request.message.clone(),
        model_id: request.model_id.clone(),
        context: merged_context,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, Some(&session_id))
        .await
        .map_err(|e| {
            tracing::error!("Topsi processing error: {}", e);
            ApiError::InternalError(format!("Topsi processing failed: {}", e))
        })?;

    // Record VIBE cost
    if let Some(project_id) = billing_project_id {
        let vibe_spent = crate::helpers::billing::record_llm_vibe_usage(
            &pool,
            project_id,
            "claude-sonnet-4-6",
            response.input_tokens.unwrap_or(0),
            response.output_tokens.unwrap_or(0),
            None,
            None,
            None,
            "Topsi",
        )
        .await;
        let _ = db::models::workflow_interaction_log::log_cost(
            &pool,
            None,
            Some(&session_id),
            "Topsi",
            "claude-sonnet-4-6",
            response.input_tokens.unwrap_or(0),
            response.output_tokens.unwrap_or(0),
            vibe_spent as f64,
        )
        .await;
    }

    // Persist conversation (non-blocking)
    {
        let pool_conv = pool.clone();
        let sess = session_id.clone();
        let user_msg = request.message.clone();
        let assistant_msg = response.message.clone();
        let resp_input = response.input_tokens;
        let resp_output = response.output_tokens;
        tokio::spawn(async move {
            crate::helpers::conversations::persist_chat_exchange(
                &pool_conv,
                topsi_agent_id,
                &sess,
                None,
                &user_msg,
                &assistant_msg,
                Some("claude-sonnet-4-6"),
                Some("anthropic"),
                resp_input,
                resp_output,
                "Topsi",
            )
            .await;
        });
    }

    Ok(Json(response))
}

/// Execute a Topsi command
pub async fn execute_command(
    State(state): State<DeploymentImpl>,
    headers: axum::http::HeaderMap,
    Json(request): Json<CommandRequest>,
) -> Result<Json<TopsiResponse>, ApiError> {
    let topsi_instance = get_topsi_instance().await?;
    let instance = topsi_instance.read().await;
    let topsi = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Topsi not initialized".to_string()))?;

    // SECURITY: Extract real user from auth headers
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let topsi_request = TopsiRequest::new(TopsiRequestType::ExecuteCommand {
        command: request.command,
    });

    let response = topsi
        .process_request(topsi_request, &user_context, None)
        .await
        .map_err(|e| {
            tracing::error!("Command execution error: {}", e);
            ApiError::InternalError(format!("Command execution failed: {}", e))
        })?;

    Ok(Json(response))
}

/// Build structured context from media asset attachments.
/// For images, includes AI description and analysis.
/// For documents, includes metadata (actual text extraction would require additional tooling).
async fn build_attachment_context(
    pool: &sqlx::SqlitePool,
    attachment_ids: &[Uuid],
) -> Result<serde_json::Value, sqlx::Error> {
    let mut attachments = Vec::new();

    for id in attachment_ids {
        if let Some(asset) = MediaAsset::find_by_id(pool, *id).await? {
            let is_image = asset.mime_type.starts_with("image/");
            let is_document = asset.mime_type.starts_with("application/pdf")
                || asset.mime_type.starts_with("text/")
                || asset.mime_type.contains("document");

            let mut attachment_info = serde_json::json!({
                "id": asset.id.to_string(),
                "filename": asset.filename,
                "mimeType": asset.mime_type,
                "fileSize": asset.file_size_bytes,
            });

            if is_image {
                // Include vision AI analysis for images
                if let Some(desc) = &asset.ai_description {
                    attachment_info["description"] = serde_json::json!(desc);
                }
                if let Some(shot) = &asset.shot_type {
                    attachment_info["shotType"] = serde_json::json!(shot);
                }
                if !asset.scene_tags.is_empty() && asset.scene_tags != "[]" {
                    if let Ok(tags) = serde_json::from_str::<serde_json::Value>(&asset.scene_tags) {
                        attachment_info["tags"] = tags;
                    }
                }
                if !asset.dominant_colors.is_empty() && asset.dominant_colors != "[]" {
                    if let Ok(colors) =
                        serde_json::from_str::<serde_json::Value>(&asset.dominant_colors)
                    {
                        attachment_info["colors"] = colors;
                    }
                }
                attachment_info["type"] = serde_json::json!("image");
                attachment_info["analysisStatus"] = serde_json::json!(asset.analysis_status);
            } else if is_document {
                attachment_info["type"] = serde_json::json!("document");
                // Document text extraction could be added here in the future
                // For now, just include metadata
            } else {
                attachment_info["type"] = serde_json::json!("file");
            }

            attachments.push(attachment_info);
        }
    }

    Ok(serde_json::json!(attachments))
}
