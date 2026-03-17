//! Audio generation, caching, and serving for Twilio TTS.

use super::*;

/// Generate audio using NORA's voice engine and cache it
pub(super) async fn generate_and_cache_audio(
    text: &str,
    call_sid: Option<String>,
) -> Result<String, String> {
    // Get NORA instance
    let nora_instance = get_nora_instance()
        .await
        .map_err(|e| format!("NORA not available: {}", e))?;

    let instance = nora_instance.read().await;
    let nora = instance
        .as_ref()
        .ok_or_else(|| "NORA not initialized".to_string())?;

    // Synthesize speech using NORA's voice engine with timeout
    let truncated_text = truncate_for_log(text, 50);
    info!("Synthesizing speech with NORA voice engine: '{}'", truncated_text);

    // Strip markdown before TTS so symbols like * aren't read aloud
    let clean_text = strip_markdown_for_tts(text);

    // Apply timeout to TTS generation
    let tts_future = nora.voice_engine.synthesize_speech_with_format(&clean_text);
    let (audio_base64, audio_format) = match timeout(TTS_TIMEOUT, tts_future).await {
        Ok(Ok(result)) => result,
        Ok(Err(e)) => return Err(format!("TTS synthesis failed: {}", e)),
        Err(_) => return Err(format!("TTS timeout after {:?}", TTS_TIMEOUT)),
    };

    // Cache the audio using the actual format returned by the TTS provider
    let cache = get_audio_cache().await;
    let audio_id = cache
        .store(&audio_base64, audio_format, &clean_text, call_sid)
        .await?;

    Ok(audio_id)
}

/// Build the full audio URL for Twilio to fetch
pub(super) fn build_audio_url(webhook_base_url: &str, audio_id: &str) -> String {
    format!("{}/api/twilio/audio/{}", webhook_base_url, audio_id)
}

/// Serve cached audio to Twilio
///
/// GET /api/twilio/audio/:audio_id
///
/// Returns the audio file for Twilio's <Play> element to fetch
pub async fn serve_audio(
    State(_state): State<DeploymentImpl>,
    Path(audio_id): Path<String>,
) -> impl IntoResponse {
    info!("Serving audio: {}", audio_id);

    let cache = get_audio_cache().await;

    match cache.get(&audio_id).await {
        Some(cached) => {
            let content_type = cached.content_type();
            info!(
                "Serving {} bytes of {} audio for id {}",
                cached.audio_bytes.len(),
                content_type,
                audio_id
            );

            (
                StatusCode::OK,
                [("Content-Type", content_type)],
                cached.audio_bytes,
            )
        }
        None => {
            warn!("Audio not found in cache: {}", audio_id);
            (
                StatusCode::NOT_FOUND,
                [("Content-Type", "text/plain")],
                b"Audio not found".to_vec(),
            )
        }
    }
}
