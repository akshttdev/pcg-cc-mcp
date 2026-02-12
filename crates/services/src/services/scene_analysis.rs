//! Deep Scene Analysis Engine
//!
//! Uses FFmpeg to analyze video content beyond composition:
//! - Brightness/exposure measurement per segment
//! - Motion intensity (frame difference energy)
//! - Scene complexity (spatial information)
//! - Content classification (high_energy, establishing, intimate, transition)
//! - Per-clip content map for intelligent shot selection

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Error)]
pub enum SceneAnalysisError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Content type classification for a video segment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    /// High energy crowd/performance moment
    HighEnergy,
    /// Wide establishing shot
    Establishing,
    /// Close-up or intimate detail
    Intimate,
    /// Camera movement or transition footage
    Transition,
    /// Static or low-action moment
    Ambient,
}

/// Analysis of a single segment within a clip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentAnalysis {
    pub timestamp: f64,
    pub duration: f64,
    pub brightness: f64,
    pub motion_intensity: f64,
    pub complexity: f64,
    pub energy_score: f64,
    pub content_type: ContentType,
}

/// Complete analysis for one clip
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipAnalysis {
    pub filename: String,
    pub path: PathBuf,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub segments: Vec<SegmentAnalysis>,
    pub overall_energy: f64,
    pub peak_energy_timestamp: f64,
    pub dominant_content_type: ContentType,
    pub usable: bool,
}

/// Result of analyzing an entire batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneAnalysisResult {
    pub batch_id: String,
    pub clips: Vec<ClipAnalysis>,
    pub total_clips: u32,
    pub total_usable: u32,
    pub processing_time_ms: u64,
}

pub struct SceneAnalysisEngine {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

impl SceneAnalysisEngine {
    pub fn new() -> Self {
        Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
            ffprobe_path: PathBuf::from("ffprobe"),
        }
    }

    /// Probe a video file for metadata
    pub async fn probe_clip(&self, path: &Path) -> Result<(f64, u32, u32, f64), SceneAnalysisError> {
        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_streams",
                "-show_format",
            ])
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let val: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| SceneAnalysisError::Parse(format!("ffprobe JSON: {}", e)))?;

        let duration = val["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let video_stream = val["streams"]
            .as_array()
            .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"));

        let (width, height, fps) = if let Some(vs) = video_stream {
            let w = vs["width"].as_u64().unwrap_or(1920) as u32;
            let h = vs["height"].as_u64().unwrap_or(1080) as u32;
            let fps_str = vs["r_frame_rate"].as_str().unwrap_or("30/1");
            let fps = if let Some((n, d)) = fps_str.split_once('/') {
                let num: f64 = n.parse().unwrap_or(30.0);
                let den: f64 = d.parse().unwrap_or(1.0);
                if den > 0.0 { num / den } else { 30.0 }
            } else {
                fps_str.parse().unwrap_or(30.0)
            };
            (w, h, fps)
        } else {
            (1920, 1080, 30.0)
        };

        Ok((duration, width, height, fps))
    }

    /// Analyze brightness and complexity at regular intervals using signalstats
    pub async fn analyze_clip_segments(
        &self,
        path: &Path,
        duration: f64,
        segment_interval: f64,
    ) -> Result<Vec<(f64, f64, f64)>, SceneAnalysisError> {
        // Use signalstats to get YAVG (brightness) and SI (spatial info/complexity)
        // Sample one frame at each interval point
        let mut results = Vec::new();
        let num_segments = ((duration / segment_interval).ceil() as usize).min(30);

        for i in 0..num_segments {
            let ts = i as f64 * segment_interval;
            if ts >= duration {
                break;
            }

            let output = Command::new(&self.ffmpeg_path)
                .args([
                    "-ss", &format!("{:.3}", ts),
                    "-i",
                ])
                .arg(path)
                .args([
                    "-vframes", "1",
                    "-vf", "signalstats=stat=tout+vrep+brng,metadata=print",
                    "-f", "null",
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            let stderr = String::from_utf8_lossy(&output.stderr);

            // Parse YAVG (average brightness 0-255) and SI (spatial information)
            let brightness = Self::parse_metadata_value(&stderr, "lavfi.signalstats.YAVG")
                .unwrap_or(128.0) / 255.0;
            let complexity = Self::parse_metadata_value(&stderr, "lavfi.signalstats.SATAVG")
                .unwrap_or(50.0) / 255.0;

            results.push((ts, brightness, complexity));
        }

        Ok(results)
    }

    /// Detect scene changes and motion intensity using the select filter
    pub async fn detect_scene_changes(
        &self,
        path: &Path,
        duration: f64,
    ) -> Result<Vec<(f64, f64)>, SceneAnalysisError> {
        // Use the select filter with scene detection to find high-motion moments
        let output = Command::new(&self.ffmpeg_path)
            .args(["-i"])
            .arg(path)
            .args([
                "-vf",
                "select='gte(scene,0.15)',metadata=print",
                "-vsync", "vfr",
                "-f", "null",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut scene_changes = Vec::new();

        for line in stderr.lines() {
            if line.contains("lavfi.scene_score") {
                if let Some(score) = Self::parse_inline_value(line) {
                    // Also try to find the timestamp
                    let ts = Self::parse_metadata_value(&stderr, "lavfi.select.pts_time")
                        .unwrap_or(0.0);
                    scene_changes.push((ts, score));
                }
            }
        }

        // If scene detection didn't produce results, estimate from duration
        if scene_changes.is_empty() {
            // Assume moderate motion throughout
            let interval = (duration / 10.0).max(1.0);
            for i in 0..10 {
                let ts = i as f64 * interval;
                if ts < duration {
                    scene_changes.push((ts, 0.3));
                }
            }
        }

        Ok(scene_changes)
    }

    /// Analyze a single clip fully
    pub async fn analyze_clip(
        &self,
        path: &Path,
        segment_interval: f64,
    ) -> Result<ClipAnalysis, SceneAnalysisError> {
        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Probe for metadata
        let (duration, width, height, fps) = self.probe_clip(path).await?;

        if duration < 0.5 {
            return Ok(ClipAnalysis {
                filename,
                path: path.to_path_buf(),
                duration,
                width,
                height,
                fps,
                segments: vec![],
                overall_energy: 0.0,
                peak_energy_timestamp: 0.0,
                dominant_content_type: ContentType::Ambient,
                usable: false,
            });
        }

        // Run brightness/complexity analysis
        let brightness_data = self.analyze_clip_segments(path, duration, segment_interval).await?;

        // Run scene change detection for motion intensity
        let scene_changes = self.detect_scene_changes(path, duration).await?;

        // Build segment analysis
        let mut segments = Vec::new();
        for (ts, brightness, complexity) in &brightness_data {
            // Find nearest scene change score for motion intensity
            let motion = scene_changes.iter()
                .filter(|(sct, _)| (*sct - ts).abs() < segment_interval)
                .map(|(_, score)| *score)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.2);

            // Energy = weighted combination
            let energy = motion * 0.4 + complexity * 0.3 + brightness * 0.3;

            // Classify content type
            let content_type = Self::classify_segment(energy, motion, *brightness, *complexity);

            segments.push(SegmentAnalysis {
                timestamp: *ts,
                duration: segment_interval.min(duration - ts),
                brightness: *brightness,
                motion_intensity: motion,
                complexity: *complexity,
                energy_score: energy,
                content_type,
            });
        }

        // Calculate overall metrics
        let overall_energy = if segments.is_empty() {
            0.0
        } else {
            segments.iter().map(|s| s.energy_score).sum::<f64>() / segments.len() as f64
        };

        let peak_segment = segments.iter()
            .max_by(|a, b| a.energy_score.partial_cmp(&b.energy_score).unwrap_or(std::cmp::Ordering::Equal));
        let peak_energy_timestamp = peak_segment.map(|s| s.timestamp).unwrap_or(0.0);

        let dominant_content_type = Self::dominant_type(&segments);

        Ok(ClipAnalysis {
            filename,
            path: path.to_path_buf(),
            duration,
            width,
            height,
            fps,
            segments,
            overall_energy,
            peak_energy_timestamp,
            dominant_content_type,
            usable: duration >= 2.0,
        })
    }

    fn classify_segment(energy: f64, motion: f64, brightness: f64, complexity: f64) -> ContentType {
        if energy > 0.65 && motion > 0.4 {
            ContentType::HighEnergy
        } else if brightness > 0.5 && complexity < 0.3 && motion < 0.2 {
            ContentType::Establishing
        } else if complexity > 0.5 && motion < 0.3 {
            ContentType::Intimate
        } else if motion > 0.5 && energy < 0.5 {
            ContentType::Transition
        } else {
            ContentType::Ambient
        }
    }

    fn dominant_type(segments: &[SegmentAnalysis]) -> ContentType {
        if segments.is_empty() {
            return ContentType::Ambient;
        }
        let mut counts = std::collections::HashMap::new();
        for s in segments {
            *counts.entry(&s.content_type).or_insert(0u32) += 1;
        }
        counts.into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(ct, _)| ct.clone())
            .unwrap_or(ContentType::Ambient)
    }

    fn parse_metadata_value(text: &str, key: &str) -> Option<f64> {
        for line in text.lines() {
            if line.contains(key) {
                if let Some(val_str) = line.split('=').last() {
                    if let Ok(v) = val_str.trim().parse::<f64>() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    fn parse_inline_value(line: &str) -> Option<f64> {
        line.split('=').last().and_then(|v| v.trim().parse::<f64>().ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_segment() {
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.8, 0.6, 0.5, 0.5),
            ContentType::HighEnergy
        );
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.2, 0.1, 0.7, 0.2),
            ContentType::Establishing
        );
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.3, 0.1, 0.3, 0.7),
            ContentType::Intimate
        );
    }

    #[test]
    fn test_dominant_type() {
        let segments = vec![
            SegmentAnalysis {
                timestamp: 0.0, duration: 3.0, brightness: 0.5,
                motion_intensity: 0.6, complexity: 0.5, energy_score: 0.8,
                content_type: ContentType::HighEnergy,
            },
            SegmentAnalysis {
                timestamp: 3.0, duration: 3.0, brightness: 0.5,
                motion_intensity: 0.5, complexity: 0.5, energy_score: 0.7,
                content_type: ContentType::HighEnergy,
            },
            SegmentAnalysis {
                timestamp: 6.0, duration: 3.0, brightness: 0.7,
                motion_intensity: 0.1, complexity: 0.2, energy_score: 0.2,
                content_type: ContentType::Establishing,
            },
        ];
        assert_eq!(SceneAnalysisEngine::dominant_type(&segments), ContentType::HighEnergy);
    }
}
