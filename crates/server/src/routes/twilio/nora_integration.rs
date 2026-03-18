//! NORA integration: process_with_nora() and process_sms_with_nora().

use super::*;

/// Process speech input by calling Anthropic directly — lean prompt, no context bloat
/// Returns (response_text, input_tokens, output_tokens)
pub(super) async fn process_with_nora(
    speech_text: &str,
    _session_id: &str,
    phone_context: Option<serde_json::Value>,
) -> Result<(String, i64, i64), String> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

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
    let mut messages = Vec::new();

    // For PCG team: inject their project/task context
    if caller_type == "pcg_team" {
        if let Some(ctx) = &phone_context {
            if let Some(work) = ctx.get("pcg_work") {
                let work_str = serde_json::to_string_pretty(work).unwrap_or_default();
                if !work_str.is_empty() && work_str != "null" {
                    messages.push(json!({
                        "role": "user",
                        "content": format!("[Your current PCG workspace:\n{}]", work_str)
                    }));
                    messages.push(json!({
                        "role": "assistant",
                        "content": "I have your workspace context — projects and active tasks loaded."
                    }));
                }
            }
        }
    }

    // Include prior turn history if available
    if let Some(ctx) = &phone_context {
        if let Some(history) = ctx.get("conversation_history").and_then(|h| h.as_str()) {
            if !history.is_empty() {
                messages.push(json!({
                    "role": "user",
                    "content": format!("[Previous conversation context:\n{}]", history)
                }));
                messages.push(json!({
                    "role": "assistant",
                    "content": "Understood, I have the conversation context."
                }));
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
                        let prev = c.get("previous_calls").and_then(|v| v.as_i64()).unwrap_or(0);
                        format!("[Returning client: {}, {} previous calls] ", name, prev)
                    }
                    _ => format!("[New caller: {}] ", name),
                }
            } else {
                String::new()
            }
        })
        .unwrap_or_default();

    messages.push(json!({
        "role": "user",
        "content": format!("{}{}", caller_note, speech_text)
    }));

    let body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 250,
        "system": system_prompt,
        "messages": messages
    });

    let client = reqwest::Client::new();
    let fut = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send();

    let resp = match timeout(LLM_TIMEOUT, fut).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return Err(format!("HTTP error: {}", e)),
        Err(_) => {
            warn!("LLM timeout after {:?}", LLM_TIMEOUT);
            return Ok(("I'm just pulling that information up — could you give me one moment?".to_string(), 0, 0));
        }
    };

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parse error: {}", e))?;

    let input_tokens = data["usage"]["input_tokens"].as_i64().unwrap_or(0);
    let output_tokens = data["usage"]["output_tokens"].as_i64().unwrap_or(0);
    let text = data["content"][0]["text"]
        .as_str()
        .unwrap_or("I'm sorry, I didn't quite catch that. Could you say that again?")
        .to_string();

    Ok((text, input_tokens, output_tokens))
}

/// Process an SMS through the real Nora agent (with full tool access),
/// falling back to a direct Claude API call if Nora is unavailable.
pub(super) async fn process_sms_with_nora(
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
            priority: if is_team { RequestPriority::High } else { RequestPriority::Normal },
            timestamp: chrono::Utc::now(),
        };

        match timeout(Duration::from_secs(55), nora.process_request(nora_req)).await {
            Ok(Ok(response)) => Some(response.content),
            Ok(Err(e)) => { error!("Nora agent error on SMS from {}: {}", from_number, e); None }
            Err(_) => { warn!("Nora agent timed out on SMS from {}", from_number); None }
        }
    }.await;

    if let Some(text) = nora_result {
        return Ok(text);
    }

    // ── Fallback: direct Claude API call ──────────────────────────────────────
    info!("Falling back to direct Claude API for SMS from {}", from_number);

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .map_err(|_| "ANTHROPIC_API_KEY not set".to_string())?;

    let system_prompt = if is_team { NORA_PCG_TEAM_SYSTEM } else { NORA_CLIENT_SYSTEM };
    let sms_instruction = "[SMS channel — reply as plain text, no markdown, \
        keep under 300 characters. British English.]";

    let body = json!({
        "model": "claude-sonnet-4-6",
        "max_tokens": 512,
        "system": format!("{}\n\n{}", system_prompt, sms_instruction),
        "messages": [{ "role": "user", "content": full_content }]
    });

    let client = reqwest::Client::new();
    let fut = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send();

    let resp = match timeout(Duration::from_secs(20), fut).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return Err(format!("HTTP error: {}", e)),
        Err(_) => return Ok("I'm just catching up — please send again in a moment.".into()),
    };

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parse: {}", e))?;
    let text = data["content"][0]["text"]
        .as_str()
        .unwrap_or("Sorry, I didn't quite catch that. Could you rephrase?")
        .to_string();

    Ok(text)
}
