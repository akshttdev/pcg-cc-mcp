//! Chat and command execution handlers for Topsi

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

    let topsi_agent_id = topsi.id;

    // Get REAL user context from authentication
    let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|h| h.to_str().ok());
    let user_context = get_user_context_from_req(&state, auth_header, cookie_header).await;

    let session_id = request.session_id.clone();

    let topsi_request = TopsiRequest::new(TopsiRequestType::Chat {
        message: request.message.clone(),
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
        crate::helpers::billing::record_llm_vibe_usage(
            &pool,
            project_id,
            "claude-sonnet-4-20250514",
            response.input_tokens.unwrap_or(0),
            response.output_tokens.unwrap_or(0),
            None,
            None,
            None,
            "Topsi",
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
                Some("claude-sonnet-4-20250514"),
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
