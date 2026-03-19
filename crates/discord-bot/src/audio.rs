//! Audio processing utilities: PCM accumulation, WAV encoding, STT, wake word detection, TTS

use std::{io::Cursor, sync::Arc};

use anyhow::{Context, Result};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use crate::session::ActiveAgent;

/// Encode 48kHz stereo i16 PCM as a WAV file in memory
pub fn encode_wav(stereo_samples: &[i16]) -> Result<Vec<u8>> {
    // Convert stereo to mono by averaging channels
    let mono: Vec<i16> = stereo_samples
        .chunks_exact(2)
        .map(|ch| {
            let l = ch[0] as i32;
            let r = ch[1] as i32;
            ((l + r) / 2) as i16
        })
        .collect();

    // Downsample from 48kHz to 16kHz (Whisper's preferred rate)
    // Simple decimation: take every 3rd sample (48000 / 16000 = 3)
    let downsampled: Vec<i16> = mono.iter().step_by(3).copied().collect();

    let mut wav_bytes = Vec::new();
    let cursor = Cursor::new(&mut wav_bytes);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::new(cursor, spec).context("Failed to create WAV writer")?;

    for sample in &downsampled {
        writer
            .write_sample(*sample)
            .context("Failed to write WAV sample")?;
    }
    writer.finalize().context("Failed to finalize WAV")?;

    Ok(wav_bytes)
}

/// Encode TTS audio (16kHz mono WAV) back to 48kHz stereo for Discord playback
pub fn upsample_to_discord(wav_bytes: &[u8]) -> Result<Vec<i16>> {
    let cursor = Cursor::new(wav_bytes);
    let mut reader = hound::WavReader::new(cursor).context("Failed to open TTS WAV")?;

    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to read TTS samples")?;

    // Upsample from source rate to 48kHz
    let src_rate = spec.sample_rate as f64;
    let dst_rate = 48000.0_f64;
    let ratio = dst_rate / src_rate;
    let dst_len = (samples.len() as f64 * ratio) as usize;

    let mut upsampled = Vec::with_capacity(dst_len * 2); // stereo

    for i in 0..dst_len {
        let src_idx = i as f64 / ratio;
        let lo = src_idx.floor() as usize;
        let hi = (lo + 1).min(samples.len() - 1);
        let frac = src_idx - lo as f64;

        let lo_val = samples[lo] as f64;
        let hi_val = samples[hi] as f64;
        let sample = (lo_val + (hi_val - lo_val) * frac) as i16;

        // Duplicate to stereo L + R
        upsampled.push(sample);
        upsampled.push(sample);
    }

    Ok(upsampled)
}

/// Transcribe WAV audio via OpenAI Whisper API or local Whisper server
pub async fn transcribe_wav(wav_bytes: Vec<u8>) -> Result<Option<String>> {
    // Require at least 0.5 seconds of audio (16kHz mono = 8000 samples = 16000 bytes + 44 header)
    if wav_bytes.len() < 16_044 {
        debug!("Audio too short to transcribe ({} bytes)", wav_bytes.len());
        return Ok(None);
    }

    // Try local Whisper server first
    if let Ok(whisper_url) = std::env::var("WHISPER_URL") {
        match transcribe_local(&whisper_url, wav_bytes.clone()).await {
            Ok(text) if !text.trim().is_empty() => return Ok(Some(text)),
            Ok(_) => {}
            Err(e) => warn!("Local Whisper failed: {}", e),
        }
    }

    // Fall back to OpenAI Whisper API
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        match transcribe_openai(&api_key, wav_bytes).await {
            Ok(text) if !text.trim().is_empty() => return Ok(Some(text)),
            Ok(_) => {}
            Err(e) => warn!("OpenAI Whisper failed: {}", e),
        }
    }

    warn!("No STT provider available — set WHISPER_URL or OPENAI_API_KEY");
    Ok(None)
}

async fn transcribe_local(whisper_url: &str, wav_bytes: Vec<u8>) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!("{}/inference", whisper_url.trim_end_matches('/'));

    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    let form = reqwest::multipart::Form::new().part("file", part);

    let resp = client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .context("Local Whisper request failed")?;

    let json: serde_json::Value = resp.json().await?;
    Ok(json["text"].as_str().unwrap_or("").to_string())
}

async fn transcribe_openai(api_key: &str, wav_bytes: Vec<u8>) -> Result<String> {
    let client = reqwest::Client::new();

    let part = reqwest::multipart::Part::bytes(wav_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    let form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", "whisper-1")
        .text("language", "en");

    let resp = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .context("OpenAI Whisper request failed")?;

    let json: serde_json::Value = resp.json().await?;
    Ok(json["text"].as_str().unwrap_or("").to_string())
}

/// Check if the transcript text addresses the given agent
/// Returns the addressed text (after the wake word) if detected
pub fn detect_wake_word(text: &str, agent: ActiveAgent) -> Option<String> {
    let lower = text.to_lowercase();

    for pattern in agent.wake_words() {
        if let Some(pos) = lower.find(pattern) {
            let after = pos + pattern.len();
            let remainder = if after < text.len() {
                text[after..]
                    .trim_start_matches(|c: char| !c.is_alphabetic() && !c.is_numeric())
                    .trim()
                    .to_string()
            } else {
                String::new()
            };

            return Some(remainder);
        }
    }
    None
}

/// Call the PCG server agent chat endpoint and return the response text
pub async fn call_agent(
    server_port: u16,
    agent: ActiveAgent,
    user_message: &str,
    project_id: &str,
    session_id: &str,
) -> Result<String> {
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{}/api", server_port);

    let voice_prefix = format!(
        "[DISCORD VOICE CALL — Speaking to {}. Voice only. 1-3 sentences max. No markdown. British English.]",
        agent.name()
    );
    let full_message = format!("{}\n\n{}", voice_prefix, user_message);

    let body = serde_json::json!({
        "message": full_message,
        "projectId": project_id,
        "sessionId": session_id,
    });

    let endpoint = match agent {
        ActiveAgent::Nora => format!("{}/nora/chat", base_url),
        ActiveAgent::Topsi => format!("{}/topsi/chat", base_url),
    };

    // Use the internal API key for service-to-service calls
    let api_key = std::env::var("ADMIN_API_KEY").unwrap_or_default();

    let resp = client
        .post(&endpoint)
        .header("X-Internal-Service", "discord-bot")
        .header("X-Admin-Key", &api_key)
        .json(&body)
        .send()
        .await
        .context("Agent call failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Agent returned {}: {}", status, text);
    }

    let json: serde_json::Value = resp.json().await?;
    let response_text = json["response"]
        .as_str()
        .or_else(|| json["message"].as_str())
        .or_else(|| json["content"].as_str())
        .unwrap_or("Understood.")
        .to_string();

    info!(
        "Agent {} responded: {}",
        agent.name(),
        &response_text[..response_text.len().min(80)]
    );
    Ok(response_text)
}

/// Synthesize text to WAV bytes via configured TTS provider
pub async fn synthesize_tts(text: &str) -> Result<Vec<u8>> {
    // Try ElevenLabs first
    if let Ok(api_key) = std::env::var("ELEVENLABS_API_KEY") {
        let voice_id = std::env::var("ELEVENLABS_VOICE_ID")
            .unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".to_string()); // Rachel voice

        match synthesize_elevenlabs(&api_key, &voice_id, text).await {
            Ok(wav) => return Ok(wav),
            Err(e) => warn!("ElevenLabs TTS failed: {}", e),
        }
    }

    // Fall back to OpenAI TTS
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        match synthesize_openai(&api_key, text).await {
            Ok(wav) => return Ok(wav),
            Err(e) => warn!("OpenAI TTS failed: {}", e),
        }
    }

    anyhow::bail!("No TTS provider available — set ELEVENLABS_API_KEY or OPENAI_API_KEY")
}

async fn synthesize_elevenlabs(api_key: &str, voice_id: &str, text: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let url = format!(
        "https://api.elevenlabs.io/v1/text-to-speech/{}/stream",
        voice_id
    );

    let body = serde_json::json!({
        "text": text,
        "model_id": "eleven_monolingual_v1",
        "voice_settings": {
            "stability": 0.5,
            "similarity_boost": 0.75
        },
        "output_format": "pcm_16000"
    });

    let resp = client
        .post(&url)
        .header("xi-api-key", api_key)
        .header("Accept", "audio/mpeg")
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("ElevenLabs returned {}", resp.status());
    }

    let audio_bytes = resp.bytes().await?.to_vec();

    // ElevenLabs pcm_16000 returns raw 16-bit LE PCM at 16kHz mono
    // Wrap in a WAV header so our upsample function can read it
    let mut wav_bytes = Vec::new();
    let cursor = Cursor::new(&mut wav_bytes);
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::new(cursor, spec)?;
    for chunk in audio_bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        writer.write_sample(sample)?;
    }
    writer.finalize()?;

    Ok(wav_bytes)
}

async fn synthesize_openai(api_key: &str, text: &str) -> Result<Vec<u8>> {
    let client = reqwest::Client::new();

    let body = serde_json::json!({
        "model": "tts-1",
        "input": text,
        "voice": "nova",
        "response_format": "wav",
    });

    let resp = client
        .post("https://api.openai.com/v1/audio/speech")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("OpenAI TTS returned {}", resp.status());
    }

    Ok(resp.bytes().await?.to_vec())
}

/// Synthesize TTS and play it back into the Discord voice channel via songbird.
///
/// Writes audio to a temp WAV file, loads it as a songbird Input, and plays it.
/// The temp file is cleaned up after a short delay.
pub async fn synthesize_and_play(call_lock: Arc<Mutex<songbird::Call>>, text: &str) -> Result<()> {
    let wav_bytes = synthesize_tts(text).await?;

    // Write to a temp file so songbird's symphonia decoder can read it
    let tmp_path = format!("/tmp/pcg_tts_{}.wav", uuid::Uuid::new_v4());
    tokio::fs::write(&tmp_path, &wav_bytes)
        .await
        .context("Failed to write TTS temp file")?;

    let input = songbird::input::File::new(tmp_path.clone());

    {
        let mut handler = call_lock.lock().await;
        handler.play_input(input.into());
    }

    // Clean up temp file after the audio has had time to play
    let path = tmp_path.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let _ = tokio::fs::remove_file(&path).await;
    });

    Ok(())
}
