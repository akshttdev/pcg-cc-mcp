//! Nora chat and streaming chat handlers

use db::models::media_asset::MediaAsset;

use super::*;

/// Chat with Nora
pub async fn chat_with_nora(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
    request_id: Option<axum::extract::Extension<crate::middleware::RequestId>>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<NoraResponse>, ApiError> {
    let _timer = crate::nora_metrics::start_request_timer("chat");

    // Apply rate limiting
    let rate_limiter = get_chat_rate_limiter().await;
    if !rate_limiter.try_consume().await {
        tracing::warn!("Chat rate limit exceeded");
        return Err(ApiError::TooManyRequests(
            "Rate limit exceeded. Please slow down your chat requests.".to_string(),
        ));
    }

    tracing::info!("Received chat request: {:?}", request.message);

    let pool = state.db().pool.clone();

    // Resolve project to bill against
    let billing_project_id = match request.project_id {
        Some(pid) => Some(pid),
        None => {
            // Fall back to user's home project
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

    // Get request ID from middleware or generate new one
    let req_id = request_id
        .map(|ext| ext.0.as_str().to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    tracing::info!("Nora instance found, checking if active...");
    if !nora.is_active().await {
        tracing::warn!("Nora is not active");
        crate::nora_metrics::record_request("chat", "normal");
        return Err(ApiError::BadRequest("Nora is not active".to_string()));
    }

    tracing::info!("Nora is active, creating request...");
    let persist_session_id = request.session_id.clone();
    let persist_user_msg = request.message.clone();

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
        (None, Some(attachments)) => Some(json!({ "attachments": attachments })),
        (Some(ctx), None) => Some(ctx),
        (None, None) => None,
    };

    let nora_request = NoraRequest {
        request_id: req_id,
        session_id: request.session_id,
        request_type: request
            .request_type
            .unwrap_or(NoraRequestType::TextInteraction),
        content: request.message.clone(),
        context: merged_context,
        voice_enabled: request.voice_enabled,
        priority: request.priority.unwrap_or(RequestPriority::Normal),
        timestamp: chrono::Utc::now(),
    };

    let priority_str = match nora_request.priority {
        RequestPriority::Low => "low",
        RequestPriority::Normal => "normal",
        RequestPriority::High => "high",
        RequestPriority::Urgent => "urgent",
        RequestPriority::Executive => "executive",
    };

    tracing::info!("Processing request with content: {}", request.message);
    let response = nora.process_request(nora_request).await.map_err(|e| {
        tracing::error!("Nora processing error: {}", e);
        crate::nora_metrics::record_request("chat", priority_str);
        ApiError::InternalError(format!("Nora processing failed: {}", e))
    })?;

    crate::nora_metrics::record_request("chat", priority_str);
    tracing::info!("Request processed successfully");

    // Record VIBE cost (estimate tokens if not available)
    if let Some(project_id) = billing_project_id {
        crate::helpers::billing::record_llm_vibe_usage(
            &pool,
            project_id,
            "claude-sonnet-4-20250514",
            response.input_tokens.unwrap_or(2000),
            response.output_tokens.unwrap_or(500),
            None,
            None,
            None,
            "Nora",
        )
        .await;
    }

    // Persist conversation (non-blocking — fire and forget)
    {
        let pool_conv = pool.clone();
        let session_id = persist_session_id;
        let user_msg = persist_user_msg;
        let assistant_msg = response.content.clone();
        let resp_input = response.input_tokens;
        let resp_output = response.output_tokens;
        tokio::spawn(async move {
            let nora_agent_id = Uuid::parse_str("0907dc4f-3f7f-4c40-93cf-f36a833eaa78")
                .unwrap_or_else(|_| Uuid::new_v4());
            crate::helpers::conversations::persist_chat_exchange(
                &pool_conv,
                nora_agent_id,
                &session_id,
                None,
                &user_msg,
                &assistant_msg,
                Some("claude-sonnet-4-20250514"),
                Some("anthropic"),
                resp_input,
                resp_output,
                "Nora",
            )
            .await;
        });
    }

    Ok(Json(response))
}

/// Chat with Nora using streaming (SSE)
pub async fn chat_with_nora_stream(
    State(state): State<DeploymentImpl>,
    axum::Extension(access_ctx): axum::Extension<AccessContext>,
    request_id: Option<axum::extract::Extension<crate::middleware::RequestId>>,
    Json(request): Json<ChatRequest>,
) -> Result<
    axum::response::sse::Sse<
        impl futures::stream::Stream<
            Item = Result<axum::response::sse::Event, std::convert::Infallible>,
        >,
    >,
    ApiError,
> {
    use futures::stream::StreamExt;

    let _timer = crate::nora_metrics::start_request_timer("chat_stream");

    // Apply rate limiting
    let rate_limiter = get_chat_rate_limiter().await;
    if !rate_limiter.try_consume().await {
        tracing::warn!("Chat stream rate limit exceeded");
        return Err(ApiError::TooManyRequests(
            "Rate limit exceeded. Please slow down your chat requests.".to_string(),
        ));
    }

    tracing::info!("Received streaming chat request: {:?}", request.message);

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

    // Get request ID from middleware or generate new one
    let _req_id = request_id
        .map(|ext| ext.0.as_str().to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let nora_instance = get_nora_instance().await?;
    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| ApiError::NotFound("Nora not initialized".to_string()))?;

    if !nora.is_active().await {
        tracing::warn!("Nora is not active");
        return Err(ApiError::BadRequest("Nora is not active".to_string()));
    }

    // Get the LLM client directly
    let llm_client = nora
        .llm
        .clone()
        .ok_or_else(|| ApiError::InternalError("LLM not available".to_string()))?;

    // Prepare context
    let context = request
        .context
        .map(|c| serde_json::to_string_pretty(&c).unwrap_or_default())
        .unwrap_or_default();

    let message = request.message.clone();

    // Create stream
    let llm_stream = llm_client
        .generate_stream("", &message, &context)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create stream: {}", e)))?;

    // Convert to SSE stream
    let sse_stream = llm_stream.map(|chunk_result| match chunk_result {
        Ok(chunk) => {
            tracing::debug!("Streaming chunk: {} chars", chunk.len());
            Ok(axum::response::sse::Event::default().data(chunk))
        }
        Err(e) => {
            tracing::error!("Stream error: {}", e);
            Ok(axum::response::sse::Event::default().data(format!("[ERROR]: {}", e)))
        }
    });

    crate::nora_metrics::record_request("chat_stream", "normal");

    Ok(axum::response::sse::Sse::new(sse_stream))
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

            let mut attachment_info = json!({
                "id": asset.id.to_string(),
                "filename": asset.filename,
                "mimeType": asset.mime_type,
                "fileSize": asset.file_size_bytes,
            });

            if is_image {
                // Include vision AI analysis for images
                if let Some(desc) = &asset.ai_description {
                    attachment_info["description"] = json!(desc);
                }
                if let Some(shot) = &asset.shot_type {
                    attachment_info["shotType"] = json!(shot);
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
                attachment_info["type"] = json!("image");
                attachment_info["analysisStatus"] = json!(asset.analysis_status);
            } else if is_document {
                attachment_info["type"] = json!("document");
                // Document text extraction could be added here in the future
                // For now, just include metadata
            } else {
                attachment_info["type"] = json!("file");
            }

            attachments.push(attachment_info);
        }
    }

    Ok(json!(attachments))
}
