//! SMS handling: incoming SMS, thread buffering, and SMS-during-call queuing.

use super::{
    media::{MediaResult, fetch_and_describe_media, ingest_sms_content},
    nora_integration::process_sms_with_nora,
    *,
};

/// POST /twilio/sms — Handle incoming SMS messages
pub async fn handle_incoming_sms(
    State(deployment): State<DeploymentImpl>,
    Form(request): Form<TwilioSmsRequest>,
) -> impl IntoResponse {
    info!(
        "Incoming SMS: {} from {} (sid={})",
        &request.body[..request.body.len().min(80)],
        request.from,
        request.message_sid
    );

    let pool = &deployment.db().pool;

    // ── Check if sender is on an active call ─────────────────────────────────
    let active_call_sid = {
        let phones = ACTIVE_CALL_PHONES.lock().await;
        phones.get(&request.from).cloned()
    };

    if let Some(call_sid) = active_call_sid {
        // Caller is mid-call — ingest and queue the SMS for Nora to use
        info!(
            "SMS from {} received during active call {} — queuing for Nora",
            request.from, call_sid
        );

        let sms_queue = {
            let map = CALL_DB_CONTEXTS.lock().await;
            map.get(&call_sid).map(|ctx| ctx.sms_queue.clone())
        };

        if let Some(queue) = sms_queue {
            // Spawn content ingestion so we don't block Twilio's webhook timeout

            let body = request.body.clone();
            let from = request.from.clone();
            tokio::spawn(async move {
                let ingested = ingest_sms_content(&body).await;
                let mut q = queue.lock().await;
                q.push(InCallSms {
                    body,
                    from,
                    received_at: chrono::Utc::now(),
                    ingested_content: ingested,
                });
            });
            let twiml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Response>\
                <Message>Got it — I'll bring that into our conversation now.</Message>\
                </Response>";
            return (
                StatusCode::OK,
                [("Content-Type", "application/xml")],
                twiml.to_string(),
            );
        }
    }

    // ── No active call — SMS thread buffering + Nora orchestration ───────────
    // Collect MMS media attachments (provider sends MediaUrl0..N as form fields)
    let media_attachments: Vec<(String, Option<String>)> = [
        (
            request.media_url_0.clone(),
            request.media_content_type_0.clone(),
        ),
        (
            request.media_url_1.clone(),
            request.media_content_type_1.clone(),
        ),
        (
            request.media_url_2.clone(),
            request.media_content_type_2.clone(),
        ),
    ]
    .into_iter()
    .filter_map(|(url, ct)| url.map(|u| (u, ct)))
    .collect();

    if !media_attachments.is_empty() {
        info!(
            "SMS from {} includes {} media attachment(s)",
            request.from,
            media_attachments.len()
        );
    }

    // Resolve sender identity (persons > CRM > pcg_team)
    let person_context = lookup_sms_sender_context(pool, &request.from).await;
    // TODO: caller_name was extracted but never used — person_context is still used below
    // let caller_name = person_context
    //     .as_ref()
    //     .and_then(|c| c.get("name").and_then(|v| v.as_str()))
    //     .unwrap_or("there")
    //     .to_string();

    // Buffer message + media URLs immediately — image fetch happens inside the debounce
    // spawn so we don't block the webhook response (SignalWire times out after ~15s)
    let msg_count = {
        let mut buffer = SMS_THREAD_BUFFER.lock().await;
        let entry = buffer
            .entry(request.from.clone())
            .or_insert_with(|| SmsThread {
                messages: Vec::new(),
                pending_media: Vec::new(),
                last_received: SystemTime::now(),
                person_context: person_context.clone(),
            });
        entry.messages.push(request.body.clone());
        // Accumulate media from all messages in the thread
        entry.pending_media.extend(media_attachments);
        entry.last_received = SystemTime::now();
        if entry.person_context.is_none() && person_context.is_some() {
            entry.person_context = person_context.clone();
        }
        entry.messages.len()
    };

    info!(
        "SMS thread from {}: {} message(s) buffered",
        request.from, msg_count
    );

    // Spawn debounced processor — each message spawns one; only the "last" one processes
    let from_clone = request.from.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(SMS_THREAD_WINDOW_SECS)).await;

        // Drain buffer only if we're still the most recent processor
        let thread = {
            let mut buffer = SMS_THREAD_BUFFER.lock().await;
            let should_process = buffer
                .get(&from_clone)
                .map(|t| {
                    t.last_received.elapsed().unwrap_or_default().as_secs()
                        >= SMS_THREAD_WINDOW_SECS - 5
                })
                .unwrap_or(false);
            if should_process {
                buffer.remove(&from_clone)
            } else {
                None
            }
        };

        if let Some(thread) = thread {
            // Fetch and process all media attachments (async, outside webhook handler)
            let media_result = if !thread.pending_media.is_empty() {
                info!(
                    "Fetching {} media attachment(s) for SMS thread from {}",
                    thread.pending_media.len(),
                    from_clone
                );
                fetch_and_describe_media(thread.pending_media).await
            } else {
                MediaResult {
                    text_content: None,
                    image_description: None,
                }
            };

            // Build combined message — start with buffered SMS bodies
            let mut combined = thread
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| format!("[{}] {}", i + 1, m))
                .collect::<Vec<_>>()
                .join("\n\n");

            // Append text content from text/plain media (SignalWire sends message body this way)
            if let Some(text) = media_result.text_content {
                info!(
                    "Recovered text from media attachment ({} chars): {:?}",
                    text.len(),
                    &text[..text.len().min(80)]
                );
                combined = if combined.trim().is_empty() {
                    text
                } else {
                    format!("{}\n\n{}", combined, text)
                };
            }

            // Append image description
            if let Some(desc) = media_result.image_description {
                info!(
                    "Image described ({} chars), appending to thread",
                    desc.len()
                );
                combined = format!("{}\n\n[Attached image: {}]", combined, desc);
            }

            let sender_label = thread
                .person_context
                .as_ref()
                .and_then(|c| c.get("name").and_then(|v| v.as_str()))
                .map(|n| format!("{} ({})", n, from_clone))
                .unwrap_or_else(|| from_clone.clone());

            let nora_content = format!(
                "[SMS THREAD from {} — {} message(s)]\n\n{}\n\n\
                [Channel: SMS. Reply in plain text only, no markdown. \
                Be concise — under 300 characters where possible.]",
                sender_label,
                thread.messages.len(),
                combined
            );

            let reply = match process_sms_with_nora(
                &nora_content,
                &from_clone,
                thread.person_context,
            )
            .await
            {
                Ok(text) => truncate_for_sms(&text, 320),
                Err(e) => {
                    error!("SMS Nora processing failed for {}: {}", from_clone, e);
                    "I hit a snag processing your request — please try again shortly.".to_string()
                }
            };

            if let Err(e) = send_outbound_sms(&from_clone, &reply).await {
                error!("Failed to send outbound SMS to {}: {}", from_clone, e);
            }
        }
    });

    // Return empty TwiML immediately — Nora replies via outbound SMS only
    (
        StatusCode::OK,
        [("Content-Type", "application/xml")],
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Response/>".to_string(),
    )
}
