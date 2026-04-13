//! Call handling: incoming calls, speech input, call status, and fallback.

use super::{
    audio::{build_audio_url, generate_and_cache_audio},
    nora_integration::process_with_nora,
    onboarding::{
        build_pcg_team_context, create_caller_account, get_fallback_project_id,
        lookup_pcg_team_member,
    },
    *,
};

// ---------------------------------------------------------------------------
// handle_incoming_call
// ---------------------------------------------------------------------------

/// Handle incoming call webhook from Twilio
///
/// POST /api/twilio/voice
pub async fn handle_incoming_call(
    State(deployment): State<DeploymentImpl>,
    Form(request): Form<TwilioCallRequest>,
) -> impl IntoResponse {
    info!(
        "Incoming Twilio call: {} from {}",
        request.call_sid, request.from
    );

    let handler = match get_twilio_handler().await {
        Some(h) => h,
        None => {
            error!("Twilio not configured - rejecting call");
            let twiml = TwimlBuilder::new()
                .say_british("I apologise, the phone system is not currently configured. Please try again later.")
                .hangup()
                .build();
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // Register the call with the in-memory handler
    if let Err(e) = handler.handle_incoming_call(request.clone()).await {
        error!("Error registering incoming call: {}", e);
    }

    let pool = &deployment.db().pool;

    // ------------------------------------------------------------------
    // 1. Check if this is a PCG team member (highest priority)
    // ------------------------------------------------------------------
    let twilio_caller_name = request
        .caller_name
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown Caller")
        .to_string();

    let pcg_member = lookup_pcg_team_member(pool, &request.from).await;

    // ------------------------------------------------------------------
    // 2. Branch: PCG team vs external caller
    // ------------------------------------------------------------------
    enum CallerBranch {
        PcgTeam {
            user_id: Uuid,
            full_name: String,
            is_admin: bool,
            project_id: Uuid,
            team_context_json: String,
            crm_contact_id: DbUuid,
        },
        External {
            contact: CrmContact,
            project_id: Uuid,
            caller_role: CallerRole,
            previous_calls: usize,
            previous_summaries: Vec<String>,
        },
    }

    let branch = if let Some((user_id, full_name, is_admin)) = pcg_member {
        // PCG team member calling
        let project_id = {
            let row: Option<(Vec<u8>,)> = sqlx::query_as(
                "SELECT project_id FROM project_members WHERE user_id = ? ORDER BY granted_at DESC LIMIT 1"
            )
            .bind(user_id.as_bytes().as_slice())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
            match row.and_then(|(b,)| Uuid::from_slice(&b).ok()) {
                Some(pid) => pid,
                None => get_fallback_project_id(pool)
                    .await
                    .unwrap_or_else(Uuid::new_v4),
            }
        };

        let team_context = build_pcg_team_context(pool, user_id).await;

        // Look up organization_id from the project
        let organization_id = {
            let row: Option<(Vec<u8>,)> =
                sqlx::query_as("SELECT organization_id FROM projects WHERE id = ?")
                    .bind(project_id.as_bytes().as_slice())
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten();
            row.and_then(|(b,)| Uuid::from_slice(&b).ok())
                .unwrap_or_else(Uuid::new_v4)
        };

        // Ensure CRM contact exists for PCG team member (for call_log FK)
        let crm_contact_id = {
            match CrmContact::find_by_phone_global(pool, &request.from).await {
                Ok(Some(c)) => c.id,
                _ => {
                    // Create internal CRM entry for the team member
                    let first = full_name
                        .split_whitespace()
                        .next()
                        .unwrap_or(&full_name)
                        .to_string();
                    let last = full_name.split_whitespace().nth(1).map(|s| s.to_string());
                    match CrmContact::create(
                        pool,
                        CreateCrmContact {
                            organization_id: DbUuid::from(organization_id),
                            client_id: None,

                            first_name: Some(first),
                            last_name: last,
                            email: None,
                            phone: Some(request.from.clone()),
                            mobile: None,
                            avatar_url: None,
                            company_name: Some("Power Club Global".to_string()),
                            job_title: if is_admin {
                                Some("Administrator".to_string())
                            } else {
                                Some("Team Member".to_string())
                            },
                            department: None,
                            linkedin_url: None,
                            twitter_handle: None,
                            website: None,
                            source: Some(ContactSource::Manual),
                            lifecycle_stage: Some(LifecycleStage::Customer),
                            tags: Some(vec!["pcg-team".to_string()]),
                            custom_fields: None,
                            zoho_contact_id: None,
                            gmail_contact_id: None,
                        },
                    )
                    .await
                    {
                        Ok(c) => c.id,
                        Err(_) => DbUuid::new(),
                    }
                }
            }
        };

        info!(
            "PCG team member calling: {} (admin={}), project={}",
            full_name, is_admin, project_id
        );
        CallerBranch::PcgTeam {
            user_id,
            full_name,
            is_admin,
            project_id,
            team_context_json: team_context,
            crm_contact_id,
        }
    } else {
        // External caller — CRM lookup
        let existing_contact = CrmContact::find_by_phone_global(pool, &request.from)
            .await
            .unwrap_or(None);

        if let Some(contact) = existing_contact {
            // Returning external client
            let prev_logs = CallLog::find_by_crm_contact(
                pool,
                Uuid::parse_str(&contact.id).unwrap_or(Uuid::nil()),
                3,
            )
            .await
            .unwrap_or_default();
            let project_id = match prev_logs.first().map(|l| l.project_id) {
                Some(pid) => {
                    // Verify project still exists (may have been deleted or DB restored)
                    let exists: bool =
                        sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
                            .bind(pid)
                            .fetch_one(pool)
                            .await
                            .unwrap_or(0i64)
                            > 0;
                    if exists {
                        pid
                    } else {
                        get_fallback_project_id(pool)
                            .await
                            .unwrap_or_else(Uuid::new_v4)
                    }
                }
                None => get_fallback_project_id(pool)
                    .await
                    .unwrap_or_else(Uuid::new_v4),
            };
            let call_count = prev_logs.len();
            let summaries: Vec<String> =
                prev_logs.iter().filter_map(|l| l.summary.clone()).collect();
            info!(
                "Returning client {} ({} previous calls)",
                request.from, call_count
            );
            CallerBranch::External {
                contact,
                project_id,
                caller_role: CallerRole::ReturningClient,
                previous_calls: call_count,
                previous_summaries: summaries,
            }
        } else {
            // New caller — create account + CRM
            let (_user_id, project_id) =
                create_caller_account(pool, &request.from, &twilio_caller_name)
                    .await
                    .unwrap_or_else(|e| {
                        error!("Failed to create caller account: {}", e);
                        (Uuid::new_v4(), Uuid::new_v4())
                    });

            // Look up organization_id from the project
            let organization_id = {
                let row: Option<(Vec<u8>,)> =
                    sqlx::query_as("SELECT organization_id FROM projects WHERE id = ?")
                        .bind(project_id.as_bytes().as_slice())
                        .fetch_optional(pool)
                        .await
                        .ok()
                        .flatten();
                row.and_then(|(b,)| Uuid::from_slice(&b).ok())
                    .unwrap_or_else(Uuid::new_v4)
            };

            let contact = match CrmContact::create(
                pool,
                CreateCrmContact {
                    organization_id: DbUuid::from(organization_id),
                    client_id: None,

                    first_name: Some(
                        twilio_caller_name
                            .split_whitespace()
                            .next()
                            .unwrap_or(&twilio_caller_name)
                            .to_string(),
                    ),
                    last_name: twilio_caller_name
                        .split_whitespace()
                        .nth(1)
                        .map(|s| s.to_string()),
                    email: None,
                    phone: Some(request.from.clone()),
                    mobile: None,
                    avatar_url: None,
                    company_name: None,
                    job_title: None,
                    department: None,
                    linkedin_url: None,
                    twitter_handle: None,
                    website: None,
                    source: Some(ContactSource::Manual),
                    lifecycle_stage: Some(LifecycleStage::Lead),
                    tags: None,
                    custom_fields: None,
                    zoho_contact_id: None,
                    gmail_contact_id: None,
                },
            )
            .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create CRM contact: {}", e);
                    let twiml = TwimlBuilder::new()
                        .say_british("I apologise, we're experiencing a technical issue. Please call back in a moment.")
                        .hangup()
                        .build();
                    return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
                }
            };

            info!(
                "New caller {} onboarded: contact={}, project={}",
                request.from, contact.id, project_id
            );
            CallerBranch::External {
                contact,
                project_id,
                caller_role: CallerRole::NewCaller,
                previous_calls: 0,
                previous_summaries: vec![],
            }
        }
    };

    // ------------------------------------------------------------------
    // 3. Unpack branch into unified variables
    // ------------------------------------------------------------------
    let (
        caller_name,
        caller_role,
        pcg_user_id,
        project_id,
        crm_contact_id,
        pcg_team_context_json,
        caller_profile_json,
    ) = match branch {
        CallerBranch::PcgTeam {
            user_id,
            full_name,
            is_admin,
            project_id,
            team_context_json,
            crm_contact_id,
        } => {
            let role = if is_admin {
                CallerRole::PcgAdmin
            } else {
                CallerRole::PcgTeam
            };
            let profile = json!({
                "caller_phone": request.from,
                "caller_name": full_name,
                "caller_role": if is_admin { "pcg_admin" } else { "pcg_team" },
                "is_pcg_team": true,
                "company": "Power Club Global",
            })
            .to_string();
            (
                full_name,
                role,
                Some(user_id),
                project_id,
                crm_contact_id,
                Some(team_context_json),
                profile,
            )
        }
        CallerBranch::External {
            contact,
            project_id,
            caller_role,
            previous_calls,
            previous_summaries,
        } => {
            let name = contact
                .first_name
                .as_deref()
                .map(|f| {
                    if let Some(l) = &contact.last_name {
                        format!("{} {}", f, l)
                    } else {
                        f.to_string()
                    }
                })
                .unwrap_or_else(|| twilio_caller_name.clone());
            let is_new = caller_role == CallerRole::NewCaller;
            let profile = json!({
                "caller_phone": request.from,
                "caller_name": name,
                "caller_role": if is_new { "new_caller" } else { "returning_client" },
                "is_pcg_team": false,
                "company": contact.company_name,
                "job_title": contact.job_title,
                "lifecycle_stage": contact.lifecycle_stage,
                "previous_calls": previous_calls,
                "sponsored_call": is_new,
                "previous_summaries": previous_summaries,
            })
            .to_string();
            (
                name,
                caller_role,
                None,
                project_id,
                contact.id,
                None,
                profile,
            )
        }
    };

    // ------------------------------------------------------------------
    // 4. Create CallLog
    // ------------------------------------------------------------------
    let call_log = match CallLog::create(
        pool,
        CreateCallLog {
            project_id,
            call_sid: request.call_sid.clone(),
            parent_call_sid: None,
            account_sid: None,
            from_number: request.from.clone(),
            to_number: request.to.clone(),
            from_formatted: None,
            to_formatted: None,
            caller_name: Some(caller_name.clone()),
            direction: CallDirection::Inbound,
            status: CallStatus::InProgress,
            answered_by: None,
            start_time: Some(Utc::now()),
        },
    )
    .await
    {
        Ok(log) => {
            let _ = CallLog::update(
                pool,
                log.id,
                UpdateCallLog {
                    crm_contact_id: Some(Uuid::parse_str(&crm_contact_id).unwrap_or(Uuid::nil())),
                    ..Default::default()
                },
            )
            .await;
            log
        }
        Err(e) => {
            error!("Failed to create call log: {}", e);
            let twiml = TwimlBuilder::new()
                .say_british("I apologise, we're experiencing a technical issue logging this call. Please try again.")
                .hangup()
                .build();
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // ------------------------------------------------------------------
    // 5. Get or create AgentConversation
    // ------------------------------------------------------------------
    let session_id = format!("twilio-{}", request.call_sid);

    let nora_agent_id: Uuid = {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT id FROM agents WHERE short_name = 'Nora' LIMIT 1")
                .fetch_optional(pool)
                .await
                .unwrap_or(None);
        row.and_then(|(s,)| Uuid::parse_str(&s).ok())
            .unwrap_or_else(Uuid::new_v4)
    };

    let conversation =
        match AgentConversation::get_or_create(pool, nora_agent_id, &session_id, Some(project_id))
            .await
        {
            Ok(conv) => conv,
            Err(e) => {
                error!("Failed to get/create AgentConversation: {}", e);
                let twiml = TwimlBuilder::new()
                    .say_british(
                        "I apologise, I cannot start a conversation right now. Please try again.",
                    )
                    .hangup()
                    .build();
                return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
            }
        };

    // ------------------------------------------------------------------
    // 6. Store in CALL_DB_CONTEXTS + register phone→call_sid mapping
    // ------------------------------------------------------------------
    {
        let mut map = CALL_DB_CONTEXTS.lock().await;
        map.insert(
            request.call_sid.clone(),
            CallDbContext {
                call_log_id: call_log.id,
                conversation_id: conversation.id,
                crm_contact_id,
                project_id,
                caller_role: caller_role.clone(),
                caller_phone: request.from.clone(),
                pcg_user_id,
                caller_profile_json: caller_profile_json.clone(),
                pcg_team_context_json: pcg_team_context_json.clone(),
                sms_queue: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            },
        );
    }
    {
        let mut phones = ACTIVE_CALL_PHONES.lock().await;
        phones.insert(request.from.clone(), request.call_sid.clone());
    }

    // ------------------------------------------------------------------
    // 7. Personalised greeting
    // ------------------------------------------------------------------
    let greeting = match &caller_role {
        CallerRole::PcgAdmin | CallerRole::PcgTeam => {
            let first = caller_name
                .split_whitespace()
                .next()
                .unwrap_or(&caller_name);
            format!("Hello {}! How can I help you today?", first)
        }
        CallerRole::ReturningClient => {
            let first = caller_name.split_whitespace().next().unwrap_or("there");
            format!(
                "Welcome back, {}! Lovely to hear from you again. How can I help you today?",
                first
            )
        }
        CallerRole::NewCaller => handler.config().greeting_message.clone(),
    };

    let audio_result = generate_and_cache_audio(&greeting, Some(request.call_sid.clone())).await;

    let speech_url = format!(
        "{}/api/twilio/speech?call_sid={}",
        handler.config().webhook_base_url,
        request.call_sid
    );

    let twiml = match audio_result {
        Ok(audio_id) => {
            let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
            info!("Generated greeting audio: {} -> {}", audio_id, audio_url);
            TwimlBuilder::greeting_with_audio_and_gather(
                &audio_url,
                &speech_url,
                &handler.config().speech_language,
            )
        }
        Err(e) => {
            warn!("NORA voice synthesis failed, falling back to Polly: {}", e);
            TwimlBuilder::greeting_with_gather(
                &greeting,
                &speech_url,
                &handler.config().speech_language,
            )
        }
    };

    info!("Generated TwiML for incoming call {}", request.call_sid);
    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}

// ---------------------------------------------------------------------------
// handle_speech_input
// ---------------------------------------------------------------------------

/// Handle speech input webhook from Twilio
///
/// POST /api/twilio/speech
pub async fn handle_speech_input(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<SpeechQueryParams>,
    Form(speech_result): Form<TwilioSpeechResult>,
) -> impl IntoResponse {
    let call_sid = params
        .call_sid
        .as_deref()
        .unwrap_or(&speech_result.call_sid)
        .to_string();

    info!(
        "Speech input for call {}: {:?}",
        call_sid, speech_result.speech_result
    );

    let handler = match get_twilio_handler().await {
        Some(h) => h,
        None => {
            let twiml = TwimlBuilder::new()
                .say_british("The system is currently unavailable.")
                .hangup()
                .build();
            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // Get the caller's speech text
    let speech_text = match &speech_result.speech_result {
        Some(text) if !text.trim().is_empty() => text.clone(),
        _ => {
            // No speech detected
            let prompt = "I didn't catch that. Could you please repeat?";
            let speech_url = format!(
                "{}/api/twilio/speech?call_sid={}",
                handler.config().webhook_base_url,
                call_sid
            );

            let twiml = match generate_and_cache_audio(prompt, Some(call_sid.clone())).await {
                Ok(audio_id) => {
                    let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
                    TwimlBuilder::new()
                        .gather_speech_with_audio(
                            &audio_url,
                            &speech_url,
                            10,
                            &handler.config().speech_language,
                            None,
                        )
                        .say_british("If you'd like to end the call, simply say goodbye.")
                        .redirect(&speech_url)
                        .build()
                }
                Err(_) => TwimlBuilder::new()
                    .gather_speech(
                        &speech_url,
                        10,
                        &handler.config().speech_language,
                        None,
                        Some(prompt),
                    )
                    .say_british("If you'd like to end the call, simply say goodbye.")
                    .redirect(&speech_url)
                    .build(),
            };

            return (StatusCode::OK, [("Content-Type", "application/xml")], twiml);
        }
    };

    // Get NORA session ID for this call
    let session_id = handler
        .get_session_id(&call_sid)
        .await
        .unwrap_or_else(|| format!("twilio-{}", Uuid::new_v4()));

    // Get conversation context from the in-memory call handler (turn history)
    let context = handler
        .get_call_state(&call_sid)
        .await
        .map(|state| state.get_conversation_context());

    // Retrieve CallDbContext for this call (if available)
    let db_ctx = {
        let map = CALL_DB_CONTEXTS.lock().await;
        map.get(call_sid.as_str()).cloned()
    };

    // Drain any SMS messages queued while this call was active
    let queued_sms: Vec<serde_json::Value> = if let Some(ref ctx) = db_ctx {
        let mut queue = ctx.sms_queue.lock().await;
        queue
            .drain(..)
            .map(|sms| {
                let mut entry = json!({
                    "from": sms.from,
                    "received_at": sms.received_at.to_rfc3339(),
                    "body": sms.body,
                });
                if let Some(content) = sms.ingested_content {
                    entry["ingested_content"] = json!(content);
                }
                entry
            })
            .collect()
    } else {
        vec![]
    };
    let sms_note = if !queued_sms.is_empty() {
        format!(
            "\n\nNOTE: The caller sent {} SMS message(s) during this call. Acknowledge them naturally and use their content in your response:\n{}",
            queued_sms.len(),
            serde_json::to_string_pretty(&queued_sms).unwrap_or_default()
        )
    } else {
        String::new()
    };

    // Build caller-aware phone context
    let phone_context = if let Some(ref ctx) = db_ctx {
        let caller_profile: serde_json::Value =
            serde_json::from_str(&ctx.caller_profile_json).unwrap_or_default();

        match &ctx.caller_role {
            CallerRole::PcgAdmin | CallerRole::PcgTeam => {
                // Full orchestration context for PCG team
                let team_data: serde_json::Value = ctx
                    .pcg_team_context_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                json!({
                    "source": "phone_call",
                    "caller_type": "pcg_team",
                    "caller": caller_profile,
                    "pcg_work": team_data,
                    "conversation_history": context.unwrap_or_default(),
                    "sms_received_during_call": queued_sms,
                    "instruction": format!("Keep responses to 2-3 SHORT sentences. Be direct and action-oriented. British English. When asked to create tasks or update projects, confirm what you will do.{}", sms_note)
                })
            }
            CallerRole::ReturningClient => {
                json!({
                    "source": "phone_call",
                    "caller_type": "returning_client",
                    "caller": caller_profile,
                    "conversation_history": context.unwrap_or_default(),
                    "sms_received_during_call": queued_sms,
                    "instruction": format!("Keep responses to 2-3 SHORT sentences. Be warm and professional. British English. Reference previous context where relevant.{}", sms_note)
                })
            }
            CallerRole::NewCaller => {
                json!({
                    "source": "phone_call",
                    "caller_type": "new_caller",
                    "caller": caller_profile,
                    "conversation_history": context.unwrap_or_default(),
                    "sms_received_during_call": queued_sms,
                    "instruction": format!("Keep responses to 2-3 SHORT sentences. Be warm and welcoming. British English. Help them understand what PCG can do for them.{}", sms_note)
                })
            }
        }
    } else {
        json!({
            "source": "phone_call",
            "instruction": "This is a phone call. Keep your response to 1-2 SHORT sentences only. Be conversational and natural. Use British English.",
            "conversation_history": context.unwrap_or_default()
        })
    };

    // Process through NORA to get response text
    let (nora_response_raw, input_tokens, output_tokens) = match process_with_nora(
        &speech_text,
        &session_id,
        Some(phone_context),
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            error!("Error processing with NORA: {}", e);
            ("I apologise, I'm having trouble processing your request. Could you please try again?".to_string(), 0i64, 0i64)
        }
    };
    // Strip markdown so neither ElevenLabs TTS nor Twilio <Say> reads symbols aloud
    let nora_response = strip_markdown_for_tts(&nora_response_raw);

    // Record VIBE usage for this phone turn (fire-and-forget)
    if let Some(ref ctx) = db_ctx {
        if input_tokens > 0 || output_tokens > 0 {
            let pool = deployment.db().pool.clone();
            let project_id = ctx.project_id;
            let (in_tok, out_tok) = (input_tokens, output_tokens);
            tokio::spawn(async move {
                let pricing = VibePricingService::new(pool.clone());
                if let Ok(tx) = pricing
                    .record_llm_usage(
                        VibeSourceType::Project,
                        project_id,
                        "claude-haiku-4-5-20251001",
                        in_tok,
                        out_tok,
                        None,
                        None,
                        None,
                    )
                    .await
                {
                    if let Err(e) = db::models::project::Project::adjust_vibe_spent(
                        &pool,
                        &project_id.to_string(),
                        tx.amount_vibe,
                    )
                    .await
                    {
                        tracing::warn!("[VIBE] Failed to adjust project vibe_spent: {e}");
                    }
                    tracing::info!(
                        "[VIBE] Phone turn: {} VIBE charged to project={}",
                        tx.amount_vibe,
                        project_id
                    );
                }
            });
        }
    }

    // Persist messages to DB (fire-and-forget — don't block the response)
    if let Some(ref ctx) = db_ctx {
        let pool = deployment.db().pool.clone();
        let conversation_id = ctx.conversation_id;
        let speech_clone = speech_text.clone();
        let response_clone = nora_response.clone();

        tokio::spawn(async move {
            if let Err(e) =
                AgentConversationMessage::add_user_message(&pool, conversation_id, &speech_clone)
                    .await
            {
                warn!("Failed to persist user message: {}", e);
            }
            if let Err(e) = AgentConversationMessage::add_assistant_message(
                &pool,
                conversation_id,
                &response_clone,
                Some("claude-sonnet-4-20250514"),
                Some("anthropic"),
                None,
                None,
                None,
            )
            .await
            {
                warn!("Failed to persist assistant message: {}", e);
            }
        });
    }

    // Record the conversation in the call handler
    if let Err(e) = handler
        .handle_speech_input(speech_result.clone(), nora_response.clone())
        .await
    {
        warn!("Error recording conversation: {}", e);
    }

    // Check for goodbye phrases
    let caller_text = speech_result
        .speech_result
        .as_deref()
        .unwrap_or("")
        .to_lowercase();

    let is_goodbye = caller_text.contains("goodbye")
        || caller_text.contains("bye")
        || caller_text.contains("thank you")
        || caller_text.contains("thanks")
        || caller_text.contains("that's all")
        || caller_text.contains("hang up")
        || caller_text.contains("end call");

    // Generate audio using NORA's voice engine
    let audio_result = generate_and_cache_audio(&nora_response, Some(call_sid.clone())).await;

    let speech_url = format!(
        "{}/api/twilio/speech?call_sid={}",
        handler.config().webhook_base_url,
        call_sid
    );

    let twiml = match audio_result {
        Ok(audio_id) => {
            let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
            info!(
                "Generated response audio: {} for call {}",
                audio_id, call_sid
            );

            if is_goodbye {
                TwimlBuilder::goodbye_with_audio(&audio_url)
            } else {
                TwimlBuilder::respond_with_audio_and_gather(
                    &audio_url,
                    &speech_url,
                    &handler.config().speech_language,
                )
            }
        }
        Err(e) => {
            warn!("NORA voice synthesis failed, falling back to Polly: {}", e);
            if is_goodbye {
                TwimlBuilder::goodbye(&nora_response)
            } else {
                TwimlBuilder::respond_and_gather(
                    &nora_response,
                    &speech_url,
                    &handler.config().speech_language,
                )
            }
        }
    };

    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}

// ---------------------------------------------------------------------------
// handle_call_status (formerly handle_status_callback)
// ---------------------------------------------------------------------------

/// Handle call status callback from Twilio
///
/// POST /api/twilio/status
///
/// Finalises the CallLog, archives the AgentConversation, updates CRM.
pub async fn handle_call_status(
    State(deployment): State<DeploymentImpl>,
    Form(status): Form<TwilioStatusCallback>,
) -> impl IntoResponse {
    info!(
        "Call status update: {} -> {}",
        status.call_sid, status.call_status
    );

    // Notify the in-memory handler
    if let Some(handler) = get_twilio_handler().await {
        if let Err(e) = handler
            .handle_status_update(&status.call_sid, &status.call_status, status.call_duration)
            .await
        {
            warn!("Error handling status update: {}", e);
        }
    }

    // Clean up cached audio
    if matches!(
        status.call_status.as_str(),
        "completed" | "failed" | "busy" | "no-answer"
    ) {
        let cache = get_audio_cache().await;
        cache.cleanup_call(&status.call_sid).await;
        info!("Cleaned up audio cache for call {}", status.call_sid);
    }

    // Only do DB finalisation on completed calls
    if status.call_status != "completed" {
        return StatusCode::OK;
    }

    let pool = &deployment.db().pool;

    // Retrieve and remove CallDbContext + clear phone mapping
    let db_ctx = {
        let mut map = CALL_DB_CONTEXTS.lock().await;
        map.remove(status.call_sid.as_str())
    };
    if let Some(ref ctx) = db_ctx {
        let mut phones = ACTIVE_CALL_PHONES.lock().await;
        phones.remove(&ctx.caller_phone);
    }

    let db_ctx = match db_ctx {
        Some(ctx) => ctx,
        None => {
            warn!("No CallDbContext for completed call {}", status.call_sid);
            return StatusCode::OK;
        }
    };

    // Load all messages to build transcript
    let messages = AgentConversationMessage::find_recent(pool, db_ctx.conversation_id, 200)
        .await
        .unwrap_or_default();

    let transcript = messages
        .iter()
        .map(|m| format!("[{}]: {}", m.role.to_uppercase(), m.content))
        .collect::<Vec<_>>()
        .join("\n");

    // Finalise CallLog
    let duration = status.call_duration.unwrap_or(0) as i32;
    if let Err(e) = CallLog::update(
        pool,
        db_ctx.call_log_id,
        UpdateCallLog {
            status: Some(CallStatus::Completed),
            end_time: Some(Utc::now()),
            duration_seconds: Some(duration),
            transcription: Some(transcript),
            transcription_status: Some("completed".to_string()),
            crm_contact_id: Some(Uuid::parse_str(&db_ctx.crm_contact_id).unwrap_or(Uuid::nil())),
            ..Default::default()
        },
    )
    .await
    {
        warn!("Failed to finalise call log {}: {}", db_ctx.call_log_id, e);
    }

    // Archive AgentConversation
    if let Err(e) =
        AgentConversation::update_status(pool, db_ctx.conversation_id, ConversationStatus::Archived)
            .await
    {
        warn!(
            "Failed to archive conversation {}: {}",
            db_ctx.conversation_id, e
        );
    }

    // Update CRM last_contacted_at
    if let Err(e) = CrmContact::record_contact_made(pool, &db_ctx.crm_contact_id).await {
        warn!(
            "Failed to update CRM contact {}: {}",
            db_ctx.crm_contact_id, e
        );
    }

    info!(
        "Call {} finalised: log={}, conversation={}, crm={}",
        status.call_sid, db_ctx.call_log_id, db_ctx.conversation_id, db_ctx.crm_contact_id
    );

    StatusCode::OK
}

/// Handle fallback webhook (called on errors)
///
/// POST /api/twilio/fallback
pub async fn handle_fallback(
    State(_state): State<DeploymentImpl>,
    Form(request): Form<TwilioCallRequest>,
) -> impl IntoResponse {
    error!("Twilio fallback triggered for call: {}", request.call_sid);

    // Try to use NORA voice for error message
    let error_message = "I apologise, we're experiencing technical difficulties. Please try your call again in a few minutes.";

    let twiml = match generate_and_cache_audio(error_message, Some(request.call_sid.clone())).await
    {
        Ok(audio_id) => {
            if let Some(handler) = get_twilio_handler().await {
                let audio_url = build_audio_url(&handler.config().webhook_base_url, &audio_id);
                TwimlBuilder::new()
                    .play(&audio_url, 1)
                    .pause(1)
                    .hangup()
                    .build()
            } else {
                TwimlBuilder::new()
                    .say_british(error_message)
                    .pause(1)
                    .hangup()
                    .build()
            }
        }
        Err(_) => TwimlBuilder::new()
            .say_british(error_message)
            .pause(1)
            .hangup()
            .build(),
    };

    (StatusCode::OK, [("Content-Type", "application/xml")], twiml)
}
