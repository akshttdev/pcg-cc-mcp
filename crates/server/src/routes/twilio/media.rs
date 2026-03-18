//! Media handling: fetch MMS attachments, describe images, ingest SMS URLs.

use super::*;

/// Result of processing MMS media attachments.
pub(super) struct MediaResult {
    /// Text extracted from text/plain media attachments (the user's actual message body)
    pub text_content: Option<String>,
    /// Claude Vision description of any image attachments
    pub image_description: Option<String>,
}

/// Fetch and process all MMS media attachments from SignalWire/Twilio.
/// - text/plain → fetched and returned as the user's message text
/// - image/* → fetched, base64-encoded, described via Claude Vision
pub(super) async fn fetch_and_describe_media(media: Vec<(String, Option<String>)>) -> MediaResult {
    if media.is_empty() {
        return MediaResult { text_content: None, image_description: None };
    }

    let account_sid = match std::env::var("TWILIO_ACCOUNT_SID") {
        Ok(v) => v,
        Err(_) => return MediaResult { text_content: None, image_description: None },
    };
    let auth_token = match std::env::var("TWILIO_AUTH_TOKEN") {
        Ok(v) => v,
        Err(_) => return MediaResult { text_content: None, image_description: None },
    };
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("NORA_ANTHROPIC_API_KEY"))
        .ok();

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
    {
        Ok(c) => c,
        Err(_) => return MediaResult { text_content: None, image_description: None },
    };

    let mut image_blocks: Vec<serde_json::Value> = Vec::new();
    let mut text_parts: Vec<String> = Vec::new();

    for (url, content_type) in media.iter().take(5) {
        let mime = content_type.as_deref().unwrap_or("application/octet-stream");

        if mime.starts_with("text/plain") {
            // This is the user's message body sent as a media attachment by SignalWire
            match client.get(url).basic_auth(&account_sid, Some(&auth_token)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(text) = resp.text().await {
                        let trimmed = text.trim().to_string();
                        if !trimmed.is_empty() {
                            info!("Fetched text/plain media ({} chars): {:?}", trimmed.len(), &trimmed[..trimmed.len().min(100)]);
                            text_parts.push(trimmed);
                        }
                    }
                }
                Ok(resp) => warn!("Text media fetch {} returned HTTP {}", url, resp.status()),
                Err(e) => warn!("Text media fetch error for {}: {}", url, e),
            }
        } else if mime.starts_with("image/") {
            match client.get(url).basic_auth(&account_sid, Some(&auth_token)).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(bytes) = resp.bytes().await {
                        use base64::Engine;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        image_blocks.push(serde_json::json!({
                            "type": "image",
                            "source": { "type": "base64", "media_type": mime, "data": b64 }
                        }));
                    }
                }
                Ok(resp) => warn!("Image fetch {} returned HTTP {}", url, resp.status()),
                Err(e) => warn!("Image fetch error for {}: {}", url, e),
            }
        } else {
            info!("Skipping unsupported media type: {} ({})", url, mime);
        }
    }

    let text_content = if text_parts.is_empty() { None } else { Some(text_parts.join("\n")) };

    // Describe images with Claude Vision if we have any
    let image_description = if image_blocks.is_empty() || api_key.is_none() {
        None
    } else {
        let mut content = image_blocks;
        content.push(serde_json::json!({
            "type": "text",
            "text": "Describe what you see in these image(s) in detail. Include subject matter, \
                     any visible text or numbers, colours, composition, and any context useful \
                     for someone who hasn't seen the image. Be thorough but concise."
        }));

        let api_key = api_key.unwrap();
        let body = serde_json::json!({
            "model": "claude-sonnet-4-6",
            "max_tokens": 1024,
            "messages": [{ "role": "user", "content": content }]
        });

        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .ok();

        match resp {
            Some(r) => {
                let data: serde_json::Value = r.json().await.unwrap_or_default();
                data["content"][0]["text"].as_str().map(|s| s.to_string())
            }
            None => None,
        }
    };

    MediaResult { text_content, image_description }
}

/// Fetch and extract readable content from any URLs in an SMS body.
/// Returns a summarised string of ingested content, or None if no URLs found.
pub(super) async fn ingest_sms_content(body: &str) -> Option<String> {
    // Find URLs in the message
    let urls: Vec<&str> = body.split_whitespace()
        .filter(|w| w.starts_with("http://") || w.starts_with("https://"))
        .collect();

    if urls.is_empty() {
        return None;
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (compatible; NoraBot/1.0)")
        .build()
        .ok()?;

    let mut parts = Vec::new();

    for url in urls.iter().take(3) {
        match client.get(*url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let content_type = resp.headers()
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();

                if content_type.contains("text/html") || content_type.contains("text/plain") {
                    if let Ok(text) = resp.text().await {
                        let extracted = extract_text_from_html(&text);
                        // Truncate to 1500 chars per URL to keep context manageable
                        let snippet = if extracted.len() > 1500 {
                            format!("{}…", &extracted[..1500])
                        } else {
                            extracted
                        };
                        parts.push(format!("[Content from {}]:\n{}", url, snippet));
                    }
                } else {
                    parts.push(format!("[Link {} — content type: {}]", url, content_type));
                }
            }
            Ok(resp) => {
                parts.push(format!("[Link {} — HTTP {}]", url, resp.status()));
            }
            Err(e) => {
                parts.push(format!("[Link {} — fetch error: {}]", url, e));
            }
        }
    }

    if parts.is_empty() { None } else { Some(parts.join("\n\n")) }
}

/// Strip HTML tags and collapse whitespace to get readable text.
fn extract_text_from_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut tag_buf = String::new();

    for c in html.chars() {
        match c {
            '<' => {
                in_tag = true;
                tag_buf.clear();
            }
            '>' => {
                in_tag = false;
                let tag_lower = tag_buf.trim().to_lowercase();
                if tag_lower.starts_with("script") || tag_lower.starts_with("style") {
                    in_script_or_style = true;
                } else if tag_lower.starts_with("/script") || tag_lower.starts_with("/style") {
                    in_script_or_style = false;
                } else if tag_lower == "br" || tag_lower == "p" || tag_lower == "/p"
                    || tag_lower.starts_with("h") || tag_lower.starts_with("/h")
                {
                    out.push('\n');
                }
            }
            _ if in_tag => tag_buf.push(c),
            _ if in_script_or_style => {}
            _ => out.push(c),
        }
    }

    // Collapse whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
