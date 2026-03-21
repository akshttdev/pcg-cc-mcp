//! Transcript Processor — Phase 2
//!
//! 1. Extract audio from video via FFmpeg
//! 2. Transcribe via local Whisper (file-based)
//! 3. Clean and structure via LLM (speaker tagging, segment classification)
//! 4. Verify soundbites against actual transcript text

use std::path::{Path, PathBuf};

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tokio::process::Command;

use super::types::*;
use crate::{NoraError, Result};

/// Handles audio extraction, Whisper transcription, and LLM-based cleaning.
pub struct TranscriptProcessor {
    whisper_endpoint: Option<String>,
    http_client: Client,
}

impl TranscriptProcessor {
    pub fn new(whisper_endpoint: Option<String>) -> Self {
        Self {
            whisper_endpoint,
            http_client: Client::new(),
        }
    }

    /// Full pipeline: extract audio → transcribe → clean → structure.
    pub async fn process(&self, video_path: &Path) -> Result<StructuredTranscript> {
        tracing::info!("[TRANSCRIPT] Processing: {}", video_path.display());

        // Step 1: Extract audio as WAV (16kHz mono for Whisper)
        let wav_path = self.extract_audio(video_path).await?;

        // Step 2: Transcribe with local Whisper
        let raw_transcript = self.transcribe_whisper(&wav_path).await?;

        // Step 3: Clean and structure with LLM
        let structured = self.clean_and_structure(raw_transcript, video_path).await?;

        // Clean up temp WAV
        let _ = tokio::fs::remove_file(&wav_path).await;

        Ok(structured)
    }

    /// Extract audio from video to a 16kHz mono WAV file.
    async fn extract_audio(&self, video_path: &Path) -> Result<PathBuf> {
        let wav_path = video_path.with_extension("_temp_audio.wav");

        tracing::info!(
            "[TRANSCRIPT] Extracting audio: {} -> {}",
            video_path.display(),
            wav_path.display()
        );

        let output = Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                video_path.to_str().unwrap_or(""),
                "-vn",
                "-acodec",
                "pcm_s16le",
                "-ar",
                "16000",
                "-ac",
                "1",
                wav_path.to_str().unwrap_or(""),
            ])
            .output()
            .await
            .map_err(|e| {
                NoraError::ExecutionError(format!("FFmpeg audio extraction failed: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NoraError::ExecutionError(format!(
                "FFmpeg audio extraction error: {}",
                stderr
            )));
        }

        Ok(wav_path)
    }

    /// Transcribe audio via local Whisper endpoint or CLI.
    async fn transcribe_whisper(&self, wav_path: &Path) -> Result<RawWhisperOutput> {
        if let Some(endpoint) = &self.whisper_endpoint {
            self.transcribe_whisper_api(wav_path, endpoint).await
        } else {
            self.transcribe_whisper_cli(wav_path).await
        }
    }

    /// Transcribe using the local Whisper HTTP server (same one used for Nora STT).
    async fn transcribe_whisper_api(
        &self,
        wav_path: &Path,
        endpoint: &str,
    ) -> Result<RawWhisperOutput> {
        tracing::info!("[TRANSCRIPT] Transcribing via Whisper API: {}", endpoint);

        let audio_bytes = tokio::fs::read(wav_path)
            .await
            .map_err(NoraError::IoError)?;

        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_bytes)
                    .file_name("audio.wav")
                    .mime_str("audio/wav")
                    .map_err(|e| {
                        NoraError::ExecutionError(format!("Failed to create multipart: {}", e))
                    })?,
            )
            .text("response_format", "verbose_json")
            .text("timestamp_granularities[]", "word");

        let url = format!("{}/v1/audio/transcriptions", endpoint.trim_end_matches('/'));
        let response = self
            .http_client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| NoraError::ExecutionError(format!("Whisper API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(NoraError::ExecutionError(format!(
                "Whisper API error ({}): {}",
                status, body
            )));
        }

        let json: Value = response.json().await.map_err(|e| {
            NoraError::ExecutionError(format!("Failed to parse Whisper response: {}", e))
        })?;

        Ok(parse_whisper_json(&json))
    }

    /// Fallback: Transcribe using the Whisper CLI.
    async fn transcribe_whisper_cli(&self, wav_path: &Path) -> Result<RawWhisperOutput> {
        tracing::info!("[TRANSCRIPT] Transcribing via Whisper CLI");

        let output_dir = wav_path.parent().unwrap_or(Path::new("/tmp"));
        let output = Command::new("whisper")
            .args([
                wav_path.to_str().unwrap_or(""),
                "--output_format",
                "json",
                "--word_timestamps",
                "True",
                "--output_dir",
                output_dir.to_str().unwrap_or("/tmp"),
            ])
            .output()
            .await
            .map_err(|e| NoraError::ExecutionError(format!("Whisper CLI failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NoraError::ExecutionError(format!(
                "Whisper CLI error: {}",
                stderr
            )));
        }

        // Read the JSON output file
        let json_path = wav_path.with_extension("json");
        let json_str = tokio::fs::read_to_string(&json_path).await.map_err(|e| {
            NoraError::ExecutionError(format!("Failed to read Whisper JSON output: {}", e))
        })?;

        let json: Value = serde_json::from_str(&json_str).map_err(|e| {
            NoraError::ExecutionError(format!("Failed to parse Whisper JSON: {}", e))
        })?;

        // Clean up the JSON file
        let _ = tokio::fs::remove_file(&json_path).await;

        Ok(parse_whisper_json(&json))
    }

    /// Detect sentence boundaries from word-level timestamps.
    /// Uses punctuation (.!?) and pause gaps (>0.3s) to find natural breaks.
    pub fn detect_sentence_boundaries(segments: &[TranscriptSegment]) -> Vec<SentenceBoundary> {
        let mut sentences = Vec::new();
        let sentence_end_chars = ['.', '!', '?'];

        for (seg_idx, segment) in segments.iter().enumerate() {
            if segment.words.is_empty() {
                // No word-level timestamps — treat the whole segment as one sentence
                sentences.push(SentenceBoundary {
                    text: segment.text.clone(),
                    start_seconds: segment.start_seconds,
                    end_seconds: segment.end_seconds,
                    start_word_idx: 0,
                    end_word_idx: 0,
                    segment_index: seg_idx,
                });
                continue;
            }

            let mut sentence_start_idx = 0;
            let mut sentence_words: Vec<String> = Vec::new();

            for (word_idx, word) in segment.words.iter().enumerate() {
                sentence_words.push(word.word.clone());

                let is_last_word = word_idx == segment.words.len() - 1;
                let trimmed = word.word.trim();
                let ends_sentence = trimmed.ends_with(|c: char| sentence_end_chars.contains(&c));

                // Check for pause gap to next word (>0.3s indicates natural break)
                let has_pause_gap = if !is_last_word {
                    let next_word = &segment.words[word_idx + 1];
                    (next_word.start_seconds - word.end_seconds) > 0.3
                } else {
                    false
                };

                if ends_sentence || has_pause_gap || is_last_word {
                    let text = sentence_words.join(" ").trim().to_string();
                    if !text.is_empty() {
                        sentences.push(SentenceBoundary {
                            text,
                            start_seconds: segment.words[sentence_start_idx].start_seconds,
                            end_seconds: word.end_seconds,
                            start_word_idx: sentence_start_idx,
                            end_word_idx: word_idx,
                            segment_index: seg_idx,
                        });
                    }
                    sentence_start_idx = word_idx + 1;
                    sentence_words.clear();
                }
            }
        }

        sentences
    }

    /// Verify that a soundbite text starts and ends at sentence boundaries.
    /// Returns (starts_at_boundary, ends_at_boundary).
    pub fn check_sentence_alignment(
        soundbite_text: &str,
        sentences: &[SentenceBoundary],
        start_seconds: f64,
        end_seconds: f64,
    ) -> (bool, bool) {
        let tolerance = 0.5; // 500ms tolerance for boundary matching

        let starts_at = sentences
            .iter()
            .any(|s| (s.start_seconds - start_seconds).abs() < tolerance);

        let ends_at = sentences
            .iter()
            .any(|s| (s.end_seconds - end_seconds).abs() < tolerance);

        (starts_at, ends_at)
    }

    /// Clean raw Whisper output using an LLM to tag speakers, classify segments.
    async fn clean_and_structure(
        &self,
        raw: RawWhisperOutput,
        source_file: &Path,
    ) -> Result<StructuredTranscript> {
        tracing::info!(
            "[TRANSCRIPT] Cleaning transcript: {} segments, {:.1}s total",
            raw.segments.len(),
            raw.duration_seconds
        );

        // Build LLM prompt for transcript cleaning
        let transcript_text: String = raw
            .segments
            .iter()
            .enumerate()
            .map(|(i, seg)| {
                format!(
                    "Line {}: [{:.1}s - {:.1}s] {}",
                    i + 1,
                    seg.start,
                    seg.end,
                    seg.text
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = r#"You are a professional video editor's transcript analyst.

Analyze this raw Whisper transcription and output a JSON object with:
1. "testimonial_start_line": the line number where the actual testimonial begins (after pre-interview banter)
2. "speakers": array of identified speakers with a short identifier
3. "segments": array of objects with:
   - "line": original line number
   - "speaker": speaker identifier
   - "segment_type": one of "PreInterview", "Testimonial", "Transition", "Unusable"
   - "text": cleaned text

Rules:
- Pre-interview banter includes greetings, mic checks, casual conversation before the formal interview
- Testimonial segments are the actual on-record responses
- Tag each line with the most likely speaker based on context
- Preserve exact timecodes from the original
- Return ONLY valid JSON, no markdown"#;

        let user_prompt = format!("Analyze this transcript:\n\n{}", transcript_text);

        // Call LLM for cleaning (using OpenAI API directly)
        let cleaned = self.call_cleaning_llm(system_prompt, &user_prompt).await;

        // Build structured transcript from raw + LLM annotations
        let segments = self.merge_raw_with_annotations(&raw, cleaned);
        let speakers: Vec<String> = segments
            .iter()
            .map(|s| s.speaker.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        let testimonial_start_index = segments
            .iter()
            .position(|s| s.segment_type == SegmentType::Testimonial);

        // Detect sentence boundaries from word-level timestamps
        let sentences = Self::detect_sentence_boundaries(&segments);
        tracing::info!(
            "[TRANSCRIPT] Detected {} sentence boundaries",
            sentences.len()
        );

        Ok(StructuredTranscript {
            source_file: source_file
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default(),
            total_duration_seconds: raw.duration_seconds,
            segments,
            speakers,
            testimonial_start_index,
            sentences,
        })
    }

    /// Call the LLM for transcript cleaning.
    async fn call_cleaning_llm(&self, system_prompt: &str, user_prompt: &str) -> Option<Value> {
        let api_key = match std::env::var("OPENAI_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                tracing::warn!("[TRANSCRIPT] No OpenAI API key, skipping LLM cleaning");
                return None;
            }
        };

        let response = self
            .http_client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": "gpt-4o",
                "messages": [
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_prompt}
                ],
                "temperature": 0.3,
                "max_tokens": 8000
            }))
            .send()
            .await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                let json: Value = resp.json().await.ok()?;
                let content = json["choices"][0]["message"]["content"].as_str()?;
                serde_json::from_str(content).ok()
            }
            Ok(resp) => {
                tracing::warn!("[TRANSCRIPT] LLM cleaning failed ({})", resp.status());
                None
            }
            Err(e) => {
                tracing::warn!("[TRANSCRIPT] LLM cleaning request failed: {}", e);
                None
            }
        }
    }

    /// Merge raw Whisper segments with LLM annotations.
    fn merge_raw_with_annotations(
        &self,
        raw: &RawWhisperOutput,
        annotations: Option<Value>,
    ) -> Vec<TranscriptSegment> {
        raw.segments
            .iter()
            .enumerate()
            .map(|(i, seg)| {
                let (speaker, segment_type) = if let Some(ref ann) = annotations {
                    let seg_ann = ann["segments"].as_array().and_then(|arr| {
                        arr.iter()
                            .find(|s| s["line"].as_u64() == Some((i + 1) as u64))
                    });

                    let speaker = seg_ann
                        .and_then(|s| s["speaker"].as_str())
                        .unwrap_or("Unknown")
                        .to_string();

                    let seg_type = seg_ann
                        .and_then(|s| s["segment_type"].as_str())
                        .map(|t| match t {
                            "PreInterview" => SegmentType::PreInterview,
                            "Testimonial" => SegmentType::Testimonial,
                            "Transition" => SegmentType::Transition,
                            _ => SegmentType::Unusable,
                        })
                        .unwrap_or(SegmentType::Unusable);

                    (speaker, seg_type)
                } else {
                    ("Unknown".to_string(), SegmentType::Testimonial)
                };

                let words = seg
                    .words
                    .iter()
                    .map(|w| WordTiming {
                        word: w.word.clone(),
                        start_seconds: w.start,
                        end_seconds: w.end,
                    })
                    .collect();

                TranscriptSegment {
                    speaker,
                    text: seg.text.clone(),
                    start_seconds: seg.start,
                    end_seconds: seg.end,
                    segment_type,
                    words,
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Internal types for raw Whisper output parsing
// ---------------------------------------------------------------------------

struct RawWhisperOutput {
    duration_seconds: f64,
    segments: Vec<RawSegment>,
}

struct RawSegment {
    text: String,
    start: f64,
    end: f64,
    words: Vec<RawWord>,
}

struct RawWord {
    word: String,
    start: f64,
    end: f64,
}

/// Parse Whisper's JSON output into our internal representation.
fn parse_whisper_json(json: &Value) -> RawWhisperOutput {
    let duration = json["duration"].as_f64().unwrap_or(0.0);

    let segments = json["segments"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|seg| {
                    let words = seg["words"]
                        .as_array()
                        .map(|warr| {
                            warr.iter()
                                .map(|w| RawWord {
                                    word: w["word"].as_str().unwrap_or("").to_string(),
                                    start: w["start"].as_f64().unwrap_or(0.0),
                                    end: w["end"].as_f64().unwrap_or(0.0),
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    RawSegment {
                        text: seg["text"].as_str().unwrap_or("").trim().to_string(),
                        start: seg["start"].as_f64().unwrap_or(0.0),
                        end: seg["end"].as_f64().unwrap_or(0.0),
                        words,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    RawWhisperOutput {
        duration_seconds: duration,
        segments,
    }
}

/// Compute the Levenshtein similarity ratio between two strings (0.0–1.0).
pub fn levenshtein_ratio(a: &str, b: &str) -> f64 {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();
    let a_chars: Vec<char> = a_lower.chars().collect();
    let b_chars: Vec<char> = b_lower.chars().collect();
    let len_a = a_chars.len();
    let len_b = b_chars.len();

    if len_a == 0 && len_b == 0 {
        return 1.0;
    }
    if len_a == 0 || len_b == 0 {
        return 0.0;
    }

    let mut matrix = vec![vec![0usize; len_b + 1]; len_a + 1];

    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    let distance = matrix[len_a][len_b];
    let max_len = len_a.max(len_b);
    1.0 - (distance as f64 / max_len as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_identical() {
        assert!((levenshtein_ratio("hello", "hello") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_levenshtein_similar() {
        let ratio = levenshtein_ratio(
            "they were professional from start to finish",
            "they were professional from start to finish",
        );
        assert!(ratio >= 0.99);
    }

    #[test]
    fn test_levenshtein_different() {
        let ratio = levenshtein_ratio("hello world", "completely different text");
        assert!(ratio < 0.5);
    }

    #[test]
    fn test_sentence_boundary_detection() {
        let segments = vec![TranscriptSegment {
            speaker: "Dave".to_string(),
            text: "My name is David Lewis. I have been in the entertainment space.".to_string(),
            start_seconds: 40.0,
            end_seconds: 55.0,
            segment_type: SegmentType::Testimonial,
            words: vec![
                WordTiming {
                    word: "My".to_string(),
                    start_seconds: 40.0,
                    end_seconds: 40.2,
                },
                WordTiming {
                    word: "name".to_string(),
                    start_seconds: 40.3,
                    end_seconds: 40.5,
                },
                WordTiming {
                    word: "is".to_string(),
                    start_seconds: 40.6,
                    end_seconds: 40.7,
                },
                WordTiming {
                    word: "David".to_string(),
                    start_seconds: 40.8,
                    end_seconds: 41.0,
                },
                WordTiming {
                    word: "Lewis.".to_string(),
                    start_seconds: 41.1,
                    end_seconds: 41.5,
                },
                WordTiming {
                    word: "I".to_string(),
                    start_seconds: 42.0,
                    end_seconds: 42.1,
                },
                WordTiming {
                    word: "have".to_string(),
                    start_seconds: 42.2,
                    end_seconds: 42.4,
                },
                WordTiming {
                    word: "been".to_string(),
                    start_seconds: 42.5,
                    end_seconds: 42.7,
                },
                WordTiming {
                    word: "in".to_string(),
                    start_seconds: 42.8,
                    end_seconds: 42.9,
                },
                WordTiming {
                    word: "the".to_string(),
                    start_seconds: 43.0,
                    end_seconds: 43.1,
                },
                WordTiming {
                    word: "entertainment".to_string(),
                    start_seconds: 43.2,
                    end_seconds: 43.8,
                },
                WordTiming {
                    word: "space.".to_string(),
                    start_seconds: 43.9,
                    end_seconds: 44.2,
                },
            ],
        }];

        let sentences = TranscriptProcessor::detect_sentence_boundaries(&segments);
        assert_eq!(sentences.len(), 2);
        assert!(sentences[0].text.contains("David Lewis."));
        assert!(sentences[1].text.contains("entertainment space."));
        assert!((sentences[0].end_seconds - 41.5).abs() < 0.01);
        assert!((sentences[1].start_seconds - 42.0).abs() < 0.01);
    }

    #[test]
    fn test_sentence_alignment_check() {
        let sentences = vec![
            SentenceBoundary {
                text: "My name is David Lewis.".to_string(),
                start_seconds: 40.0,
                end_seconds: 41.5,
                start_word_idx: 0,
                end_word_idx: 4,
                segment_index: 0,
            },
            SentenceBoundary {
                text: "I have been in the entertainment space.".to_string(),
                start_seconds: 42.0,
                end_seconds: 44.2,
                start_word_idx: 5,
                end_word_idx: 11,
                segment_index: 0,
            },
        ];

        // Aligned: starts at sentence 1, ends at sentence 2
        let (starts, ends) =
            TranscriptProcessor::check_sentence_alignment("full text", &sentences, 40.0, 44.2);
        assert!(starts);
        assert!(ends);

        // Misaligned: starts mid-sentence
        let (starts, ends) =
            TranscriptProcessor::check_sentence_alignment("mid text", &sentences, 41.0, 44.2);
        assert!(!starts);
        assert!(ends);
    }

    #[test]
    fn test_parse_whisper_json() {
        let json = serde_json::json!({
            "duration": 120.5,
            "segments": [
                {
                    "text": "Hello, this is a test.",
                    "start": 0.0,
                    "end": 3.5,
                    "words": [
                        {"word": "Hello,", "start": 0.0, "end": 0.5},
                        {"word": "this", "start": 0.6, "end": 0.8},
                    ]
                }
            ]
        });

        let output = parse_whisper_json(&json);
        assert!((output.duration_seconds - 120.5).abs() < f64::EPSILON);
        assert_eq!(output.segments.len(), 1);
        assert_eq!(output.segments[0].words.len(), 2);
    }
}
