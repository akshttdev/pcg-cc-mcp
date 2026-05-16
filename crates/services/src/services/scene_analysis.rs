//! Deep Scene Analysis Engine
//!
//! Uses FFmpeg to analyze video content beyond composition:
//! - Brightness/exposure measurement per segment
//! - Motion intensity via tblend frame differencing (replaces broken scene detection)
//! - Camera movement via hue variance analysis
//! - Scene complexity (spatial information)
//! - Content classification (high_energy, establishing, intimate, transition)
//! - Per-clip content map for intelligent shot selection
//! - Quality scoring (sharpness, exposure, stability)

use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use serde::{Deserialize, Serialize};
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
    /// Camera pan/tilt indicator from hue variance (0.0 = static, 1.0 = fast pan)
    pub camera_movement: f64,
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
    /// Parent directory name for venue area tracking
    pub source_folder: String,
    /// Combined quality metric: sharpness*0.4 + exposure_goodness*0.3 + stability*0.3
    pub quality_score: f64,
    /// Energy quartile 1-4 (assigned after all clips analyzed, 0 = unassigned)
    pub energy_quartile: u8,
    /// Timestamps of detected hard cuts (for source range avoidance)
    pub hard_cuts: Vec<f64>,
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

/// Motion measurement from a single frame (from tblend analysis)
#[derive(Debug, Clone)]
struct FrameMotion {
    timestamp: f64,
    motion: f64,
    hue_avg: f64,
}

#[derive(Clone)]
pub struct SceneAnalysisEngine {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

impl SceneAnalysisEngine {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            ffmpeg_path: PathBuf::from("ffmpeg"),
            ffprobe_path: PathBuf::from("ffprobe"),
        }
    }

    /// Probe a video file for metadata
    pub async fn probe_clip(
        &self,
        path: &Path,
    ) -> Result<(f64, u32, u32, f64), SceneAnalysisError> {
        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
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
                if den > 0.0 {
                    num / den
                } else {
                    30.0
                }
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
        // Use signalstats to get YAVG (brightness) and SATAVG (complexity)
        // Sample one frame at each interval point
        let mut results = Vec::new();
        let num_segments = ((duration / segment_interval).ceil() as usize).min(30);

        for i in 0..num_segments {
            let ts = i as f64 * segment_interval;
            if ts >= duration {
                break;
            }

            let output = Command::new(&self.ffmpeg_path)
                .args(["-ss", &format!("{:.3}", ts), "-i"])
                .arg(path)
                .args([
                    "-vframes",
                    "1",
                    "-vf",
                    "signalstats=stat=tout+vrep+brng,metadata=print",
                    "-f",
                    "null",
                    "-",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            let stderr = String::from_utf8_lossy(&output.stderr);

            // Parse YAVG (average brightness 0-255) and SATAVG (saturation/complexity)
            let brightness = Self::parse_metadata_value(&stderr, "lavfi.signalstats.YAVG")
                .unwrap_or(128.0)
                / 255.0;
            let complexity = Self::parse_metadata_value(&stderr, "lavfi.signalstats.SATAVG")
                .unwrap_or(50.0)
                / 255.0;

            results.push((ts, brightness, complexity));
        }

        Ok(results)
    }

    /// Measure actual pixel-level motion intensity using tblend frame differencing.
    ///
    /// Single-pass FFmpeg pipeline: downscale to 320x180 (36x compute reduction),
    /// sample at 2fps, compute frame differences via tblend, measure with signalstats.
    ///
    /// Returns per-frame motion measurements including YAVG (motion) and HUEAVG (hue).
    async fn measure_motion_intensity(
        &self,
        path: &Path,
        duration: f64,
    ) -> Result<Vec<FrameMotion>, SceneAnalysisError> {
        let output = Command::new(&self.ffmpeg_path)
            .args(["-i"])
            .arg(path)
            .args([
                "-vf",
                "fps=2,scale=320:180,tblend=all_mode=difference,signalstats=stat=tout+vrep+brng,metadata=print",
                "-f", "null",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut frames = Vec::new();
        let mut current_pts = 0.0;
        let mut current_yavg: Option<f64> = None;
        let mut current_hueavg: Option<f64> = None;

        for line in stderr.lines() {
            // Parse pts_time from metadata=print output
            if line.contains("pts_time:") {
                // Emit previous frame if we have data
                if let Some(yavg) = current_yavg.take() {
                    let hue = current_hueavg.take().unwrap_or(0.0);
                    let motion = (yavg / 40.0).clamp(0.0, 1.0);
                    frames.push(FrameMotion {
                        timestamp: current_pts,
                        motion,
                        hue_avg: hue,
                    });
                }
                // Parse new pts_time
                if let Some(pts_str) = line.split("pts_time:").nth(1) {
                    current_pts = pts_str
                        .trim()
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(current_pts);
                }
            }
            if line.contains("lavfi.signalstats.YAVG=") {
                current_yavg = line
                    .split('=')
                    .next_back()
                    .and_then(|v| v.trim().parse::<f64>().ok());
            }
            if line.contains("lavfi.signalstats.HUEAVG=") {
                current_hueavg = line
                    .split('=')
                    .next_back()
                    .and_then(|v| v.trim().parse::<f64>().ok());
            }
        }

        // Emit last frame
        if let Some(yavg) = current_yavg {
            let hue = current_hueavg.unwrap_or(0.0);
            let motion = (yavg / 40.0).clamp(0.0, 1.0);
            frames.push(FrameMotion {
                timestamp: current_pts,
                motion,
                hue_avg: hue,
            });
        }

        // If tblend produced no frames (very short clip), provide defaults
        if frames.is_empty() {
            let num = ((duration * 2.0).ceil() as usize).max(1);
            for i in 0..num {
                let ts = i as f64 * 0.5;
                if ts < duration {
                    frames.push(FrameMotion {
                        timestamp: ts,
                        motion: 0.2,
                        hue_avg: 0.0,
                    });
                }
            }
        }

        Ok(frames)
    }

    /// Detect hard cuts using scene detection filter (secondary signal).
    /// Used to avoid starting source ranges at jarring frame boundaries.
    /// Threshold lowered to 0.08 to catch more transitions.
    async fn detect_hard_cuts(
        &self,
        path: &Path,
        _duration: f64,
    ) -> Result<Vec<f64>, SceneAnalysisError> {
        let output = Command::new(&self.ffmpeg_path)
            .args(["-i"])
            .arg(path)
            .args([
                "-vf",
                "select='gte(scene,0.08)',metadata=print",
                "-vsync",
                "vfr",
                "-f",
                "null",
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut cuts = Vec::new();
        let mut current_pts: Option<f64> = None;

        for line in stderr.lines() {
            if line.contains("pts_time:") {
                if let Some(pts_str) = line.split("pts_time:").nth(1) {
                    current_pts = pts_str
                        .trim()
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse::<f64>().ok());
                }
            }
            if line.contains("lavfi.scene_score") {
                if let Some(ts) = current_pts.take() {
                    cuts.push(ts);
                }
            }
        }

        cuts.dedup_by(|a, b| (*a - *b).abs() < 0.1);
        Ok(cuts)
    }

    /// Analyze a single clip fully
    pub async fn analyze_clip(
        &self,
        path: &Path,
        segment_interval: f64,
    ) -> Result<ClipAnalysis, SceneAnalysisError> {
        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Source folder = parent directory name (for venue area tracking)
        let source_folder = path
            .parent()
            .and_then(|p| p.file_name())
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
                source_folder,
                quality_score: 0.0,
                energy_quartile: 0,
                hard_cuts: vec![],
            });
        }

        // Run brightness/complexity analysis
        let brightness_data = self
            .analyze_clip_segments(path, duration, segment_interval)
            .await?;

        // Run motion intensity analysis (replaces broken scene detection)
        let motion_data = self.measure_motion_intensity(path, duration).await?;

        // Run hard cut detection (secondary signal for source range avoidance)
        let hard_cuts = self.detect_hard_cuts(path, duration).await?;

        // Build segment analysis
        let mut segments = Vec::new();
        for (ts, brightness, complexity) in &brightness_data {
            let segment_end = *ts + segment_interval.min(duration - ts);

            // Find motion values within this segment's time range
            let segment_motions: Vec<&FrameMotion> = motion_data
                .iter()
                .filter(|m| m.timestamp >= *ts && m.timestamp < segment_end)
                .collect();

            // Average motion for this segment
            let motion = if segment_motions.is_empty() {
                // Use nearest frame as fallback
                motion_data
                    .iter()
                    .min_by(|a, b| {
                        (a.timestamp - ts)
                            .abs()
                            .partial_cmp(&(b.timestamp - ts).abs())
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|m| m.motion)
                    .unwrap_or(0.2)
            } else {
                segment_motions.iter().map(|m| m.motion).sum::<f64>() / segment_motions.len() as f64
            };

            // Camera movement: variance of HUEAVG within this segment
            // High hue variance = camera scanning across differently-colored areas
            let camera_movement = if segment_motions.len() >= 2 {
                let hue_values: Vec<f64> = segment_motions.iter().map(|m| m.hue_avg).collect();
                let mean_hue = hue_values.iter().sum::<f64>() / hue_values.len() as f64;
                let variance = hue_values
                    .iter()
                    .map(|h| (h - mean_hue).powi(2))
                    .sum::<f64>()
                    / hue_values.len() as f64;
                // Normalize: sqrt(variance)/60 (sensitive enough for pan detection)
                (variance.sqrt() / 60.0).clamp(0.0, 1.0)
            } else {
                0.0
            };

            // Energy = weighted combination
            let energy = motion * 0.5 + complexity * 0.3 + brightness * 0.2;

            // Classify content type (with camera_movement)
            let content_type =
                Self::classify_segment(energy, motion, *brightness, *complexity, camera_movement);

            segments.push(SegmentAnalysis {
                timestamp: *ts,
                duration: segment_interval.min(duration - ts),
                brightness: *brightness,
                motion_intensity: motion,
                complexity: *complexity,
                energy_score: energy,
                content_type,
                camera_movement,
            });
        }

        // Calculate overall metrics
        let overall_energy = if segments.is_empty() {
            0.0
        } else {
            segments.iter().map(|s| s.energy_score).sum::<f64>() / segments.len() as f64
        };

        let peak_segment = segments.iter().max_by(|a, b| {
            a.energy_score
                .partial_cmp(&b.energy_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let peak_energy_timestamp = peak_segment.map(|s| s.timestamp).unwrap_or(0.0);

        let dominant_content_type = Self::dominant_type(&segments);

        // Quality score: sharpness*0.4 + exposure_goodness*0.3 + stability*0.3
        let avg_complexity = if segments.is_empty() {
            0.5
        } else {
            segments.iter().map(|s| s.complexity).sum::<f64>() / segments.len() as f64
        };
        let avg_brightness = if segments.is_empty() {
            0.5
        } else {
            segments.iter().map(|s| s.brightness).sum::<f64>() / segments.len() as f64
        };

        let sharpness = avg_complexity; // SATAVG/255 serves as sharpness proxy
        let exposure_goodness = (1.0 - (avg_brightness - 0.5).abs() * 2.0).clamp(0.0, 1.0);

        let motion_values: Vec<f64> = segments.iter().map(|s| s.motion_intensity).collect();
        let motion_mean = if motion_values.is_empty() {
            0.0
        } else {
            motion_values.iter().sum::<f64>() / motion_values.len() as f64
        };
        let motion_variance = if motion_values.len() < 2 {
            0.0
        } else {
            motion_values
                .iter()
                .map(|m| (m - motion_mean).powi(2))
                .sum::<f64>()
                / motion_values.len() as f64
        };
        let stability = (1.0 - motion_variance).clamp(0.0, 1.0);

        let quality_score = sharpness * 0.4 + exposure_goodness * 0.3 + stability * 0.3;

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
            source_folder,
            quality_score,
            energy_quartile: 0, // assigned later by assign_energy_quartiles()
            hard_cuts,
        })
    }

    fn classify_segment(
        energy: f64,
        motion: f64,
        brightness: f64,
        complexity: f64,
        camera_movement: f64,
    ) -> ContentType {
        if energy > 0.55 && motion > 0.35 {
            ContentType::HighEnergy
        } else if brightness > 0.4 && complexity < 0.4 && motion < 0.25 {
            ContentType::Establishing
        } else if complexity > 0.4 && motion < 0.3 {
            ContentType::Intimate
        } else if motion > 0.4 && camera_movement > 0.3 {
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
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(ct, _)| ct.clone())
            .unwrap_or(ContentType::Ambient)
    }

    fn parse_metadata_value(text: &str, key: &str) -> Option<f64> {
        for line in text.lines() {
            if line.contains(key) {
                if let Some(val_str) = line.split('=').next_back() {
                    if let Ok(v) = val_str.trim().parse::<f64>() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    fn parse_inline_value(line: &str) -> Option<f64> {
        line.split('=')
            .next_back()
            .and_then(|v| v.trim().parse::<f64>().ok())
    }
}

/// Assign energy quartiles (1-4) to clips after all clips have been analyzed.
/// Sorts clips by overall_energy and assigns quartile labels.
pub fn assign_energy_quartiles(clips: &mut [ClipAnalysis]) {
    if clips.is_empty() {
        return;
    }

    // Sort indices by energy
    let mut indices: Vec<usize> = (0..clips.len()).collect();
    indices.sort_by(|&a, &b| {
        clips[a]
            .overall_energy
            .partial_cmp(&clips[b].overall_energy)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let n = indices.len();
    for (rank, &idx) in indices.iter().enumerate() {
        clips[idx].energy_quartile = match rank * 4 / n {
            0 => 1,
            1 => 2,
            2 => 3,
            _ => 4,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_segment() {
        // HighEnergy: energy > 0.55 AND motion > 0.35
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.8, 0.6, 0.5, 0.5, 0.0),
            ContentType::HighEnergy
        );
        // Establishing: brightness > 0.4 AND complexity < 0.4 AND motion < 0.25
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.2, 0.1, 0.7, 0.2, 0.0),
            ContentType::Establishing
        );
        // Intimate: complexity > 0.4 AND motion < 0.3
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.3, 0.1, 0.3, 0.7, 0.0),
            ContentType::Intimate
        );
        // Transition: motion > 0.4 AND camera_movement > 0.3
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.4, 0.5, 0.5, 0.5, 0.5),
            ContentType::Transition
        );
        // Ambient: catch-all
        assert_eq!(
            SceneAnalysisEngine::classify_segment(0.3, 0.3, 0.5, 0.3, 0.1),
            ContentType::Ambient
        );
    }

    #[test]
    fn test_dominant_type() {
        let segments = vec![
            SegmentAnalysis {
                timestamp: 0.0,
                duration: 3.0,
                brightness: 0.5,
                motion_intensity: 0.6,
                complexity: 0.5,
                energy_score: 0.8,
                content_type: ContentType::HighEnergy,
                camera_movement: 0.0,
            },
            SegmentAnalysis {
                timestamp: 3.0,
                duration: 3.0,
                brightness: 0.5,
                motion_intensity: 0.5,
                complexity: 0.5,
                energy_score: 0.7,
                content_type: ContentType::HighEnergy,
                camera_movement: 0.0,
            },
            SegmentAnalysis {
                timestamp: 6.0,
                duration: 3.0,
                brightness: 0.7,
                motion_intensity: 0.1,
                complexity: 0.2,
                energy_score: 0.2,
                content_type: ContentType::Establishing,
                camera_movement: 0.0,
            },
        ];
        assert_eq!(
            SceneAnalysisEngine::dominant_type(&segments),
            ContentType::HighEnergy
        );
    }

    #[test]
    fn test_assign_energy_quartiles() {
        let make_clip = |energy: f64| ClipAnalysis {
            filename: format!("clip_{:.0}.mp4", energy * 100.0),
            path: PathBuf::from("/test"),
            duration: 10.0,
            width: 1920,
            height: 1080,
            fps: 30.0,
            segments: vec![],
            overall_energy: energy,
            peak_energy_timestamp: 0.0,
            dominant_content_type: ContentType::Ambient,
            usable: true,
            source_folder: "test".to_string(),
            quality_score: 0.5,
            energy_quartile: 0,
            hard_cuts: vec![],
        };

        let mut clips = vec![
            make_clip(0.1),
            make_clip(0.3),
            make_clip(0.5),
            make_clip(0.7),
            make_clip(0.2),
            make_clip(0.6),
            make_clip(0.8),
            make_clip(0.4),
        ];

        assign_energy_quartiles(&mut clips);

        // Lowest energy clips should be Q1, highest Q4
        assert_eq!(clips[0].energy_quartile, 1); // energy 0.1
        assert_eq!(clips[6].energy_quartile, 4); // energy 0.8

        // All quartiles should be assigned (non-zero)
        for clip in &clips {
            assert!(clip.energy_quartile >= 1 && clip.energy_quartile <= 4);
        }
    }
}
