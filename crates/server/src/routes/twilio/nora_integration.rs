//! NORA integration: process_with_nora() and process_sms_with_nora().

use services::services::workflow_llm::WorkflowLLMService;
use sqlx::SqlitePool;

use super::*;

/// Process speech input via PCG Router — lean prompt, no context bloat
/// Returns (response_text, input_tokens, output_tokens)
pub(super) async fn process_with_nora(
    pool: &SqlitePool,
    speech_text: &str,
    _session_id: &str,
    phone_context: Option<serde_json::Value>,
) -> Result<(String, i64, i64), String> {
    // Pick system prompt based on caller type
    let caller_type = phone_context
        .as_ref()
        .and_then(|ctx| ctx.get("caller_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let system_prompt = if caller_type == "pcg_team" {
        NORA_PCG_TEAM_SYSTEM
    } else {
        NORA_CLIENT_SYSTEM
    };

    // Build conversation messages
    let mut messages = vec![WorkflowLLMService::system_message(system_prompt)];

    // For PCG team: inject their project/task context
    if caller_type == "pcg_team" {
        if let Some(ctx) = &phone_context {
            if let Some(work) = ctx.get("pcg_work") {
                let work_str = serde_json::to_string_pretty(work).unwrap_or_default();
                if !work_str.is_empty() && work_str != "null" {
                    messages.push(WorkflowLLMService::user_message(&format!(
                        "[Your current PCG workspace:\n{}]",
                        work_str
                    )));
                    messages.push(WorkflowLLMService::assistant_message(
                        "I have your workspace context — projects and active tasks loaded.",
                    ));
                }
            }
        }
    }

    // Include prior turn history if available
    if let Some(ctx) = &phone_context {
        if let Some(history) = ctx.get("conversation_history").and_then(|h| h.as_str()) {
            if !history.is_empty() {
                messages.push(WorkflowLLMService::user_message(&format!(
                    "[Previous conversation context:\n{}]",
                    history
                )));
                messages.push(WorkflowLLMService::assistant_message(
                    "Understood, I have the conversation context.",
                ));
            }
        }
    }

    // Caller info prefix
    let caller_note = phone_context
        .as_ref()
        .and_then(|ctx| ctx.get("caller"))
        .map(|c| {
            let name = c.get("caller_name").and_then(|v| v.as_str()).unwrap_or("");
            let role = c.get("caller_role").and_then(|v| v.as_str()).unwrap_or("");
            if !name.is_empty() && name != "Unknown Caller" {
                match role {
                    "pcg_admin" => format!("[PCG Admin: {}] ", name),
                    "pcg_team" => format!("[PCG Team: {}] ", name),
                    "returning_client" => {
                        let prev = c
                            .get("previous_calls")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        format!("[Returning client: {}, {} previous calls] ", name, prev)
                    }
                    _ => format!("[New caller: {}] ", name),
                }
            } else {
                String::new()
            }
        })
        .unwrap_or_default();

    messages.push(WorkflowLLMService::user_message(&format!(
        "{}{}",
        caller_note, speech_text
    )));

    // Route through PCG Router with timeout
    let fut = WorkflowLLMService::completion(
        pool,
        messages,
        Some("claude-haiku-4-5-20251001"), // Fast model for phone calls
        Some(250),
        None,
    );

    let llm_t0 = std::time::Instant::now();
    let result = match timeout(LLM_TIMEOUT, fut).await {
        Ok(Ok((text, metadata))) => {
            crate::nora_metrics::record_voice_stage(
                "twilio",
                "llm",
                llm_t0.elapsed().as_secs_f64(),
            );
            let input_tokens = metadata.input_tokens.unwrap_or(0);
            let output_tokens = metadata.output_tokens.unwrap_or(0);
            (text, input_tokens, output_tokens)
        }
        Ok(Err(e)) => {
            crate::nora_metrics::record_voice_stage(
                "twilio",
                "llm_error",
                llm_t0.elapsed().as_secs_f64(),
            );
            warn!("[NORA_INTEGRATION] LLM error: {}", e);
            return Err(format!("LLM error: {}", e));
        }
        Err(_) => {
            crate::nora_metrics::record_voice_stage(
                "twilio",
                "llm_timeout",
                llm_t0.elapsed().as_secs_f64(),
            );
            warn!("LLM timeout after {:?}", LLM_TIMEOUT);
            return Ok((
                "I'm just pulling that information up — could you give me one moment?".to_string(),
                0,
                0,
            ));
        }
    };

    Ok(result)
}

/// Process an SMS through the real Nora agent (with full tool access),
/// falling back to PCG Router if Nora is unavailable.
pub(super) async fn process_sms_with_nora(
    pool: &SqlitePool,
    message: &str,
    from_number: &str,
    context: Option<serde_json::Value>,
) -> Result<String, String> {
    let caller_type = context
        .as_ref()
        .and_then(|ctx| ctx.get("caller_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("client");

    let is_team = caller_type == "pcg_team" || caller_type == "pcg_admin";

    let caller_note = context
        .as_ref()
        .and_then(|ctx| ctx.get("name"))
        .and_then(|v| v.as_str())
        .filter(|n| !n.is_empty())
        .map(|name| format!("[SMS from {} ({})] ", name, from_number))
        .unwrap_or_else(|| format!("[SMS from {}] ", from_number));

    let full_content = format!("{}{}", caller_note, message);

    // ── Route through the real Nora agent (has tools — can create tasks, etc.) ─
    let nora_result: Option<String> = async {
        let nora_arc = get_nora_instance().await.ok()?;
        let guard = nora_arc.read().await;
        let nora = guard.as_ref()?;

        // Always use TextInteraction — it calls process_text_with_tools which reads the
        // actual message content via LLM + tools. TaskCoordination ignores request.content
        // and returns a canned template response.
        let request_type = NoraRequestType::TextInteraction;

        let nora_req = NoraRequest {
            request_id: Uuid::new_v4().to_string(),
            session_id: format!("sms-{}", from_number),
            request_type,
            content: full_content.clone(),
            context: context.clone(),
            voice_enabled: false,
            priority: if is_team {
                RequestPriority::High
            } else {
                RequestPriority::Normal
            },
            timestamp: chrono::Utc::now(),
        };

        match timeout(Duration::from_secs(55), nora.process_request(nora_req)).await {
            Ok(Ok(response)) => Some(response.content),
            Ok(Err(e)) => {
                error!("Nora agent error on SMS from {}: {}", from_number, e);
                None
            }
            Err(_) => {
                warn!("Nora agent timed out on SMS from {}", from_number);
                None
            }
        }
    }
    .await;

    if let Some(text) = nora_result {
        return Ok(text);
    }

    // ── Fallback: PCG Router LLM call ─────────────────────────────────────────
    info!("Falling back to PCG Router for SMS from {}", from_number);

    let system_prompt = if is_team {
        NORA_PCG_TEAM_SYSTEM
    } else {
        NORA_CLIENT_SYSTEM
    };
    let sms_instruction = "[SMS channel — reply as plain text, no markdown, \
        keep under 300 characters. British English.]";

    let messages = vec![
        WorkflowLLMService::system_message(&format!("{}\n\n{}", system_prompt, sms_instruction)),
        WorkflowLLMService::user_message(&full_content),
    ];

    let fut =
        WorkflowLLMService::completion(pool, messages, Some("claude-sonnet-4-6"), Some(512), None);

    match timeout(Duration::from_secs(20), fut).await {
        Ok(Ok((text, _metadata))) => Ok(text),
        Ok(Err(e)) => {
            warn!("[NORA_INTEGRATION] SMS fallback LLM error: {}", e);
            Err(format!("LLM error: {}", e))
        }
        Err(_) => Ok("I'm just catching up — please send again in a moment.".into()),
    }
}
