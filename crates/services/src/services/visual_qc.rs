//! Visual QC Engine for Spectra (Master Cinematographer)
//!
//! Analyzes video frames using Vision APIs (provider-agnostic) to score composition,
//! select optimal in-points, and suggest reframes before assembly.

use std::path::{Path, PathBuf};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;

/// Error type for Visual QC operations
#[derive(Debug, Error)]
pub enum VisualQcError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Minimal clip representation for applying QC results
/// Compatible with any clip struct that has path, in_point, and hero_moment fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QcFootageClip {
    pub path: PathBuf,
    pub in_point: f64,
    pub hero_moment: Option<f64>,
}

/// Configuration for a Visual QC pass
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualQcConfig {
    /// Number of candidate frames to extract per clip
    pub candidates_per_clip: u32,
    /// Minimum composition score to pass QC (0.0-1.0)
    pub min_composition_score: f64,
    /// Width to scale extracted frames to (height auto from aspect)
    pub frame_width: u32,
    /// JPEG quality for extracted frames (1-31, lower = better)
    pub jpeg_quality: u32,
    /// Target aspect ratio (e.g. "16:9")
    pub target_aspect_ratio: Option<String>,
    /// Whether to suggest reframe crops
    pub suggest_reframe: bool,
    /// Maximum total frames across all clips (cost control)
    pub max_total_frames: u32,
}

impl Default for VisualQcConfig {
    fn default() -> Self {
        Self {
            candidates_per_clip: 5,
            min_composition_score: 0.6,
            frame_width: 1280,
            jpeg_quality: 5,
            target_aspect_ratio: None,
            suggest_reframe: true,
            max_total_frames: 50,
        }
    }
}

/// A single analyzed frame with vision scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedFrame {
    pub timestamp: f64,
    pub frame_path: PathBuf,
    pub composition_score: f64,
    pub subject_score: f64,
    pub thirds_score: f64,
    pub headroom_score: f64,
    pub exposure_score: f64,
    pub sharpness_score: f64,
    pub subject_region: Option<SubjectRegion>,
    pub suggested_crop: Option<CropRegion>,
    pub notes: String,
}

/// Normalized bounding box for a detected subject
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectRegion {
    /// Normalized 0.0-1.0 coordinates
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: String,
}

/// Suggested crop region for reframing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropRegion {
    /// Normalized 0.0-1.0 coordinates
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rationale: String,
}

/// QC result for a single clip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipQcResult {
    pub clip_path: PathBuf,
    pub frames_analyzed: u32,
    pub best_in_point: f64,
    pub best_composition_score: f64,
    pub recommended_crop: Option<CropRegion>,
    pub qc_passed: bool,
    pub summary: String,
}

/// Aggregate QC result for a batch of clips
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualQcResult {
    pub id: String,
    pub clips_analyzed: u32,
    pub clips_passed: u32,
    pub clips_failed: u32,
    pub clip_results: Vec<ClipQcResult>,
    pub average_composition_score: f64,
    pub processing_time_ms: u64,
}

/// Core engine for frame extraction and QC scoring
pub struct VisualQcEngine {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
    work_dir: PathBuf,
}

impl VisualQcEngine {
    pub fn new(ffmpeg_path: &Path, work_dir: &Path) -> Self {
        let ffprobe_path = ffmpeg_path
            .parent()
            .map(|p| p.join("ffprobe"))
            .unwrap_or_else(|| PathBuf::from("ffprobe"));
        Self {
            ffmpeg_path: ffmpeg_path.to_path_buf(),
            ffprobe_path,
            work_dir: work_dir.to_path_buf(),
        }
    }

    /// Extract N candidate frames at evenly-spaced timestamps from a clip
    pub async fn extract_candidate_frames(
        &self,
        clip_path: &Path,
        config: &VisualQcConfig,
    ) -> Result<Vec<(f64, PathBuf)>, VisualQcError> {
        let duration = self.probe_duration(clip_path).await?;
        if duration <= 0.0 {
            return Err(VisualQcError::InvalidFormat(
                "Clip has zero duration".to_string(),
            ));
        }

        let n = config.candidates_per_clip.max(1);
        let timestamps = Self::compute_timestamps(duration, n);

        // Create working directory for this clip
        let clip_name = clip_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "clip".to_string());
        let frames_dir = self.work_dir.join(&clip_name);
        tokio::fs::create_dir_all(&frames_dir).await?;

        let mut results = Vec::new();
        for (i, &ts) in timestamps.iter().enumerate() {
            let frame_path = frames_dir.join(format!("frame_{:04}.jpg", i));
            let output = Command::new(&self.ffmpeg_path)
                .args([
                    "-y",
                    "-ss",
                    &format!("{:.3}", ts),
                    "-i",
                    &clip_path.to_string_lossy(),
                    "-vframes",
                    "1",
                    "-vf",
                    &format!("scale={}:-1", config.frame_width),
                    "-q:v",
                    &config.jpeg_quality.to_string(),
                    &frame_path.to_string_lossy(),
                ])
                .output()
                .await?;

            if output.status.success() && frame_path.exists() {
                results.push((ts, frame_path));
            } else {
                tracing::warn!(
                    "Failed to extract frame at {:.2}s from {}: {}",
                    ts,
                    clip_path.display(),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        Ok(results)
    }

    /// Read a JPEG frame and return its base64 encoding
    pub async fn frame_to_base64(path: &Path) -> Result<String, VisualQcError> {
        let bytes = tokio::fs::read(path).await?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    /// Build the vision analysis prompt (provider-agnostic — just text)
    pub fn build_vision_prompt(target_aspect_ratio: Option<&str>) -> (String, String) {
        let system = "You are Spectra, a master cinematographer. Analyze the provided video frame for composition quality. Respond with valid JSON only — no markdown fences, no explanation.".to_string();

        let aspect_note = target_aspect_ratio
            .map(|ar| format!(" The target delivery aspect ratio is {}.", ar))
            .unwrap_or_default();

        let user = format!(
            "Score this frame on these axes (each 0.0-1.0):\n\
             - composition_score: overall composition quality\n\
             - subject_score: how well the main subject is framed\n\
             - thirds_score: rule-of-thirds alignment\n\
             - headroom_score: appropriate headroom above subjects (1.0 = perfect, 0.0 = heads cut off)\n\
             - exposure_score: exposure quality\n\
             - sharpness_score: perceived sharpness/focus\n\n\
             Also provide:\n\
             - subject_region: {{\"x\": 0-1, \"y\": 0-1, \"width\": 0-1, \"height\": 0-1, \"label\": \"...\"}} or null\n\
             - suggested_crop: {{\"x\": 0-1, \"y\": 0-1, \"width\": 0-1, \"height\": 0-1, \"rationale\": \"...\"}} or null\n\
             - notes: brief cinematographer notes{aspect_note}\n\n\
             Respond with a single JSON object."
        );

        (system, user)
    }

    /// Parse vision API JSON response into an AnalyzedFrame
    pub fn parse_vision_response(
        timestamp: f64,
        frame_path: &Path,
        response_text: &str,
    ) -> Result<AnalyzedFrame, VisualQcError> {
        // Strip markdown code fences if present
        let cleaned = response_text
            .trim()
            .strip_prefix("```json")
            .or_else(|| response_text.trim().strip_prefix("```"))
            .unwrap_or(response_text.trim());
        let cleaned = cleaned
            .strip_suffix("```")
            .unwrap_or(cleaned)
            .trim();

        let json: serde_json::Value = serde_json::from_str(cleaned).map_err(|e| {
            VisualQcError::InvalidFormat(format!(
                "Failed to parse vision response as JSON: {} — raw: {}",
                e,
                &response_text[..response_text.len().min(200)]
            ))
        })?;

        let f = |key: &str| -> f64 {
            json.get(key).and_then(|v| v.as_f64()).unwrap_or(0.0)
        };

        let subject_region = json.get("subject_region").and_then(|v| {
            if v.is_null() {
                return None;
            }
            Some(SubjectRegion {
                x: v.get("x")?.as_f64()?,
                y: v.get("y")?.as_f64()?,
                width: v.get("width")?.as_f64()?,
                height: v.get("height")?.as_f64()?,
                label: v.get("label").and_then(|l| l.as_str()).unwrap_or("").to_string(),
            })
        });

        let suggested_crop = json.get("suggested_crop").and_then(|v| {
            if v.is_null() {
                return None;
            }
            Some(CropRegion {
                x: v.get("x")?.as_f64()?,
                y: v.get("y")?.as_f64()?,
                width: v.get("width")?.as_f64()?,
                height: v.get("height")?.as_f64()?,
                rationale: v.get("rationale").and_then(|r| r.as_str()).unwrap_or("").to_string(),
            })
        });

        let notes = json
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(AnalyzedFrame {
            timestamp,
            frame_path: frame_path.to_path_buf(),
            composition_score: f("composition_score"),
            subject_score: f("subject_score"),
            thirds_score: f("thirds_score"),
            headroom_score: f("headroom_score"),
            exposure_score: f("exposure_score"),
            sharpness_score: f("sharpness_score"),
            subject_region,
            suggested_crop,
            notes,
        })
    }

    /// Select the best in-point from analyzed frames by composition score
    pub fn select_best_in_point(
        frames: &[AnalyzedFrame],
        config: &VisualQcConfig,
    ) -> Option<(f64, f64)> {
        frames
            .iter()
            .filter(|f| f.composition_score >= config.min_composition_score)
            .max_by(|a, b| {
                a.composition_score
                    .partial_cmp(&b.composition_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .or_else(|| {
                // Fallback: pick the best frame even if below threshold
                frames.iter().max_by(|a, b| {
                    a.composition_score
                        .partial_cmp(&b.composition_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            })
            .map(|f| (f.timestamp, f.composition_score))
    }

    /// Apply QC results to clips: set in_point and hero_moment
    pub fn apply_qc_to_clips(clips: &mut [QcFootageClip], qc_result: &VisualQcResult) {
        for clip_result in &qc_result.clip_results {
            if let Some(clip) = clips.iter_mut().find(|c| c.path == clip_result.clip_path) {
                if clip_result.qc_passed {
                    clip.in_point = clip_result.best_in_point;
                    clip.hero_moment = Some(clip_result.best_in_point);
                }
            }
        }
    }

    /// Assemble batch-level result from per-clip results
    pub fn assemble_batch_result(
        clip_results: Vec<ClipQcResult>,
        _config: &VisualQcConfig,
        time_ms: u64,
    ) -> VisualQcResult {
        let clips_analyzed = clip_results.len() as u32;
        let clips_passed = clip_results.iter().filter(|r| r.qc_passed).count() as u32;
        let clips_failed = clips_analyzed - clips_passed;
        let avg_score = if clips_analyzed > 0 {
            clip_results.iter().map(|r| r.best_composition_score).sum::<f64>() / clips_analyzed as f64
        } else {
            0.0
        };

        VisualQcResult {
            id: uuid::Uuid::new_v4().to_string(),
            clips_analyzed,
            clips_passed,
            clips_failed,
            clip_results,
            average_composition_score: avg_score,
            processing_time_ms: time_ms,
        }
    }

    /// Delete temporary frame JPEGs after QC
    pub async fn cleanup_frames(qc_result: &VisualQcResult) {
        for clip_result in &qc_result.clip_results {
            // Remove the clip frames directory (parent of individual frames)
            let frames_dir = clip_result
                .clip_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string());
            if let Some(dir_name) = frames_dir {
                // Best effort cleanup
                let _ = tokio::fs::remove_dir_all(&dir_name).await;
            }
        }
    }

    // ---- Private helpers ----

    /// Probe video duration via ffprobe
    async fn probe_duration(&self, path: &Path) -> Result<f64, VisualQcError> {
        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                &path.to_string_lossy(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            return Err(VisualQcError::FFmpeg(format!(
                "ffprobe failed for {}: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .trim()
            .parse::<f64>()
            .map_err(|e| VisualQcError::InvalidFormat(format!("Cannot parse duration: {}", e)))
    }

    /// Compute evenly-spaced timestamps for N frames within a duration,
    /// avoiding the very start (0s) and last 0.5s
    fn compute_timestamps(duration: f64, n: u32) -> Vec<f64> {
        let margin_start = (duration * 0.05).min(1.0);
        let margin_end = (duration * 0.05).min(0.5);
        let usable = duration - margin_start - margin_end;
        if usable <= 0.0 || n == 0 {
            return vec![duration / 2.0];
        }
        if n == 1 {
            return vec![margin_start + usable / 2.0];
        }
        let step = usable / (n - 1) as f64;
        (0..n).map(|i| margin_start + step * i as f64).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_timestamps_single() {
        let ts = VisualQcEngine::compute_timestamps(10.0, 1);
        assert_eq!(ts.len(), 1);
        // Should be near center
        assert!((ts[0] - 5.0).abs() < 1.0);
    }

    #[test]
    fn test_compute_timestamps_five() {
        let ts = VisualQcEngine::compute_timestamps(20.0, 5);
        assert_eq!(ts.len(), 5);
        // First should be near start margin, last near end
        assert!(ts[0] > 0.0);
        assert!(ts[4] < 20.0);
        // Should be evenly spaced
        let step = ts[1] - ts[0];
        for i in 1..ts.len() {
            assert!((ts[i] - ts[i - 1] - step).abs() < 0.001);
        }
    }

    #[test]
    fn test_parse_vision_response_basic() {
        let json = r#"{
            "composition_score": 0.85,
            "subject_score": 0.9,
            "thirds_score": 0.7,
            "headroom_score": 0.95,
            "exposure_score": 0.8,
            "sharpness_score": 0.75,
            "subject_region": {"x": 0.3, "y": 0.2, "width": 0.4, "height": 0.6, "label": "person"},
            "suggested_crop": null,
            "notes": "Well-composed frame with good headroom"
        }"#;

        let frame = VisualQcEngine::parse_vision_response(5.0, Path::new("/tmp/frame.jpg"), json)
            .unwrap();
        assert!((frame.composition_score - 0.85).abs() < 0.001);
        assert!((frame.subject_score - 0.9).abs() < 0.001);
        assert!(frame.subject_region.is_some());
        assert!(frame.suggested_crop.is_none());
        assert_eq!(frame.notes, "Well-composed frame with good headroom");
    }

    #[test]
    fn test_parse_vision_response_with_code_fences() {
        let json = "```json\n{\"composition_score\": 0.6, \"subject_score\": 0.5, \"thirds_score\": 0.4, \"headroom_score\": 0.3, \"exposure_score\": 0.7, \"sharpness_score\": 0.8, \"subject_region\": null, \"suggested_crop\": null, \"notes\": \"test\"}\n```";
        let frame =
            VisualQcEngine::parse_vision_response(2.0, Path::new("/tmp/f.jpg"), json).unwrap();
        assert!((frame.composition_score - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_select_best_in_point() {
        let frames = vec![
            AnalyzedFrame {
                timestamp: 1.0,
                frame_path: PathBuf::from("/tmp/a.jpg"),
                composition_score: 0.5,
                subject_score: 0.5,
                thirds_score: 0.5,
                headroom_score: 0.5,
                exposure_score: 0.5,
                sharpness_score: 0.5,
                subject_region: None,
                suggested_crop: None,
                notes: String::new(),
            },
            AnalyzedFrame {
                timestamp: 3.0,
                frame_path: PathBuf::from("/tmp/b.jpg"),
                composition_score: 0.9,
                subject_score: 0.8,
                thirds_score: 0.7,
                headroom_score: 0.9,
                exposure_score: 0.8,
                sharpness_score: 0.85,
                subject_region: None,
                suggested_crop: None,
                notes: String::new(),
            },
            AnalyzedFrame {
                timestamp: 5.0,
                frame_path: PathBuf::from("/tmp/c.jpg"),
                composition_score: 0.7,
                subject_score: 0.6,
                thirds_score: 0.8,
                headroom_score: 0.7,
                exposure_score: 0.7,
                sharpness_score: 0.7,
                subject_region: None,
                suggested_crop: None,
                notes: String::new(),
            },
        ];

        let config = VisualQcConfig::default();
        let (ts, score) = VisualQcEngine::select_best_in_point(&frames, &config).unwrap();
        assert!((ts - 3.0).abs() < 0.001);
        assert!((score - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_select_best_in_point_all_below_threshold() {
        let frames = vec![AnalyzedFrame {
            timestamp: 2.0,
            frame_path: PathBuf::from("/tmp/a.jpg"),
            composition_score: 0.3,
            subject_score: 0.3,
            thirds_score: 0.3,
            headroom_score: 0.3,
            exposure_score: 0.3,
            sharpness_score: 0.3,
            subject_region: None,
            suggested_crop: None,
            notes: String::new(),
        }];

        let config = VisualQcConfig::default(); // min 0.6
        let result = VisualQcEngine::select_best_in_point(&frames, &config);
        // Should still return the best available (fallback)
        assert!(result.is_some());
        assert!((result.unwrap().0 - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_assemble_batch_result() {
        let results = vec![
            ClipQcResult {
                clip_path: PathBuf::from("/tmp/a.mp4"),
                frames_analyzed: 5,
                best_in_point: 3.0,
                best_composition_score: 0.85,
                recommended_crop: None,
                qc_passed: true,
                summary: "Good".to_string(),
            },
            ClipQcResult {
                clip_path: PathBuf::from("/tmp/b.mp4"),
                frames_analyzed: 5,
                best_in_point: 1.0,
                best_composition_score: 0.45,
                recommended_crop: None,
                qc_passed: false,
                summary: "Below threshold".to_string(),
            },
        ];

        let config = VisualQcConfig::default();
        let batch = VisualQcEngine::assemble_batch_result(results, &config, 5000);
        assert_eq!(batch.clips_analyzed, 2);
        assert_eq!(batch.clips_passed, 1);
        assert_eq!(batch.clips_failed, 1);
        assert!((batch.average_composition_score - 0.65).abs() < 0.01);
        assert_eq!(batch.processing_time_ms, 5000);
    }
}
