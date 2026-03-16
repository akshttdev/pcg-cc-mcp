//! Nora chat and streaming chat handlers

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
            let home: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT home_project_id FROM users WHERE id = ?",
            )
            .bind(access_ctx.user_id.as_bytes().as_slice())
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
            home.and_then(|bytes| Uuid::from_slice(&bytes).ok())
        }
    };

    // VIBE Balance Check — uses real deposit ledger
    if !crate::helpers::vibe_check::is_vibe_bypass_active(&pool).await {
        if let Some(project_id) = billing_project_id {
            let total_deposited = VibeDeposit::total_deposited(&pool, project_id).await.unwrap_or(0);
            let total_withdrawn = VibeWithdrawal::total_withdrawn(&pool, project_id).await.unwrap_or(0);
            let total_spent = VibeTransaction::sum_by_source(&pool, VibeSourceType::Project, project_id, None)
                .await
                .map(|s| s.total_vibe)
                .unwrap_or(0);
            let balance = total_deposited - total_withdrawn - total_spent;
            if balance <= 0 {
                return Err(ApiError::PaymentRequired(
                    "Insufficient VIBE balance. Deposit VIBE tokens to your project to continue.".into(),
                ));
            }
        }
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
    let nora_request = NoraRequest {
        request_id: req_id,
        session_id: request.session_id,
        request_type: request
            .request_type
            .unwrap_or(NoraRequestType::TextInteraction),
        content: request.message.clone(),
        context: request.context,
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

    // Record VIBE cost
    if let Some(project_id) = billing_project_id {
        // Use actual tokens if available, otherwise estimate (2000 input, 500 output)
        let input_tokens = response.input_tokens.unwrap_or(2000);
        let output_tokens = response.output_tokens.unwrap_or(500);
        if input_tokens > 0 || output_tokens > 0 {
            let vibe_pricing = VibePricingService::new(pool.clone());
            match vibe_pricing.record_llm_usage(
                VibeSourceType::Project, project_id,
                "claude-sonnet-4-20250514",
                input_tokens, output_tokens,
                None, None, None,
            ).await {
                Ok(tx) => {
                    let _ = Project::adjust_vibe_spent(&pool, &project_id.to_string(), tx.amount_vibe).await;
                    tracing::info!("[VIBE] Nora recorded {} VIBE for project {}", tx.amount_vibe, project_id);
                }
                Err(e) => tracing::error!("[VIBE] Failed to record Nora usage: {}", e),
            }
        }
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
            // Use Nora's well-known agent ID from the agents table
            let nora_agent_id = Uuid::parse_str("0907dc4f-3f7f-4c40-93cf-f36a833eaa78")
                .unwrap_or_else(|_| Uuid::new_v4());
            match AgentConversation::get_or_create(&pool_conv, nora_agent_id, &session_id, None).await {
                Ok(conversation) => {
                    if let Err(e) = AgentConversationMessage::add_user_message(
                        &pool_conv, conversation.id, &user_msg,
                    ).await {
                        tracing::warn!("Failed to persist Nora user message: {}", e);
                    }
                    if let Err(e) = AgentConversationMessage::add_assistant_message(
                        &pool_conv, conversation.id, &assistant_msg,
                        Some("claude-sonnet-4-20250514"), Some("anthropic"),
                        resp_input, resp_output, None,
                    ).await {
                        tracing::warn!("Failed to persist Nora assistant message: {}", e);
                    }
                }
                Err(e) => tracing::warn!("Failed to get/create Nora conversation: {}", e),
            }
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
    axum::response::sse::Sse<impl futures::stream::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>,
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
            let home: Option<Vec<u8>> = sqlx::query_scalar(
                "SELECT home_project_id FROM users WHERE id = ?",
            )
            .bind(access_ctx.user_id.as_bytes().as_slice())
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();
            home.and_then(|bytes| Uuid::from_slice(&bytes).ok())
        }
    };

    // VIBE Balance Check — uses real deposit ledger
    if !crate::helpers::vibe_check::is_vibe_bypass_active(&pool).await {
        if let Some(project_id) = billing_project_id {
            let total_deposited = VibeDeposit::total_deposited(&pool, project_id).await.unwrap_or(0);
            let total_withdrawn = VibeWithdrawal::total_withdrawn(&pool, project_id).await.unwrap_or(0);
            let total_spent = VibeTransaction::sum_by_source(&pool, VibeSourceType::Project, project_id, None)
                .await
                .map(|s| s.total_vibe)
                .unwrap_or(0);
            let balance = total_deposited - total_withdrawn - total_spent;
            if balance <= 0 {
                return Err(ApiError::PaymentRequired(
                    "Insufficient VIBE balance. Deposit VIBE tokens to your project to continue.".into(),
                ));
            }
        }
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
