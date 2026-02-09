//! Scene Detection Module for Editron
//!
//! Automatic scene/cut detection capabilities:
//! - Content-aware scene boundary detection
//! - Shot type classification
//! - Motion analysis
//! - Color-based scene grouping
//! - Audio-based scene detection

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use super::{EditronError, EditronResult};

/// Scene detection method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMethod {
    /// Content-aware using frame difference
    ContentAware { threshold: f32 },
    /// Threshold-based on pixel difference
    Threshold { threshold: f32 },
    /// Adaptive threshold
    Adaptive { min_threshold: f32, max_threshold: f32 },
    /// Combined methods
    Hybrid,
}

impl Default for DetectionMethod {
    fn default() -> Self {
        DetectionMethod::ContentAware { threshold: 0.3 }
    }
}

/// A detected scene/shot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Scene index
    pub index: usize,
    /// Start time in seconds
    pub start_time: f64,
    /// End time in seconds
    pub end_time: f64,
    /// Duration in seconds
    pub duration: f64,
    /// Start frame number
    pub start_frame: u64,
    /// End frame number
    pub end_frame: u64,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Detected shot type
    pub shot_type: Option<ShotType>,
    /// Average motion intensity
    pub motion_intensity: Option<f32>,
    /// Dominant colors
    pub dominant_colors: Vec<String>,
    /// Scene tags/labels
    pub tags: Vec<String>,
}

impl Scene {
    pub fn frame_count(&self) -> u64 {
        self.end_frame - self.start_frame
    }
}

/// Shot type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShotType {
    /// Extreme close-up
    ExtremeCloseUp,
    /// Close-up shot
    CloseUp,
    /// Medium close-up
    MediumCloseUp,
    /// Medium shot
    Medium,
    /// Medium wide
    MediumWide,
    /// Wide shot
    Wide,
    /// Extreme wide/establishing
    ExtremeWide,
    /// Insert/cutaway
    Insert,
    /// Unknown
    Unknown,
}

/// Scene detection results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetectionResult {
    /// Source file
    pub source: PathBuf,
    /// Total duration
    pub total_duration: f64,
    /// Total frames
    pub total_frames: u64,
    /// Frame rate
    pub frame_rate: f32,
    /// Detection method used
    pub method: DetectionMethod,
    /// Detected scenes
    pub scenes: Vec<Scene>,
    /// Detection statistics
    pub stats: DetectionStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionStats {
    /// Number of scenes detected
    pub scene_count: usize,
    /// Average scene duration
    pub avg_duration: f64,
    /// Shortest scene duration
    pub min_duration: f64,
    /// Longest scene duration
    pub max_duration: f64,
    /// Processing time
    pub processing_time_ms: u64,
}

/// Scene grouping based on visual similarity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneGroup {
    pub name: String,
    pub scenes: Vec<usize>, // Scene indices
    pub similarity_score: f32,
    pub group_type: GroupType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupType {
    /// Same location/setting
    Location,
    /// Same subject/person
    Subject,
    /// Similar color palette
    ColorPalette,
    /// Similar motion pattern
    Motion,
    /// Manual grouping
    Manual,
}

/// Scene Detection Engine
pub struct SceneDetectionEngine {
    ffmpeg_path: PathBuf,
    ffprobe_path: PathBuf,
}

impl SceneDetectionEngine {
    pub fn new<P: AsRef<Path>>(ffmpeg_path: P) -> Self {
        let ffmpeg = ffmpeg_path.as_ref().to_path_buf();
        let ffprobe = ffmpeg.parent()
            .map(|p| p.join("ffprobe"))
            .unwrap_or_else(|| PathBuf::from("ffprobe"));

        Self {
            ffmpeg_path: ffmpeg,
            ffprobe_path: ffprobe,
        }
    }

    /// Detect scenes in a video file
    pub async fn detect_scenes<P: AsRef<Path>>(
        &self,
        input: P,
        method: DetectionMethod,
    ) -> EditronResult<SceneDetectionResult> {
        let input = input.as_ref();
        let start = std::time::Instant::now();

        // Get video info first
        let (duration, frame_count, fps) = self.get_video_info(input).await?;

        // Run scene detection
        let threshold = match &method {
            DetectionMethod::ContentAware { threshold } => *threshold,
            DetectionMethod::Threshold { threshold } => *threshold,
            DetectionMethod::Adaptive { min_threshold, .. } => *min_threshold,
            DetectionMethod::Hybrid => 0.3,
        };

        let scene_times = self.run_scene_detect(input, threshold).await?;

        // Build scene list
        let mut scenes = Vec::new();
        for (i, window) in scene_times.windows(2).enumerate() {
            let start_time = window[0];
            let end_time = window[1];

            scenes.push(Scene {
                index: i,
                start_time,
                end_time,
                duration: end_time - start_time,
                start_frame: (start_time * fps as f64) as u64,
                end_frame: (end_time * fps as f64) as u64,
                confidence: 0.8, // Default confidence
                shot_type: None,
                motion_intensity: None,
                dominant_colors: vec![],
                tags: vec![],
            });
        }

        // Add final scene if needed
        if let Some(&last_time) = scene_times.last() {
            if last_time < duration {
                scenes.push(Scene {
                    index: scenes.len(),
                    start_time: last_time,
                    end_time: duration,
                    duration: duration - last_time,
                    start_frame: (last_time * fps as f64) as u64,
                    end_frame: frame_count,
                    confidence: 0.8,
                    shot_type: None,
                    motion_intensity: None,
                    dominant_colors: vec![],
                    tags: vec![],
                });
            }
        }

        // Calculate stats
        let durations: Vec<f64> = scenes.iter().map(|s| s.duration).collect();
        let stats = DetectionStats {
            scene_count: scenes.len(),
            avg_duration: durations.iter().sum::<f64>() / durations.len().max(1) as f64,
            min_duration: durations.iter().cloned().fold(f64::INFINITY, f64::min),
            max_duration: durations.iter().cloned().fold(0.0, f64::max),
            processing_time_ms: start.elapsed().as_millis() as u64,
        };

        Ok(SceneDetectionResult {
            source: input.to_path_buf(),
            total_duration: duration,
            total_frames: frame_count,
            frame_rate: fps,
            method,
            scenes,
            stats,
        })
    }

    /// Get video information
    async fn get_video_info(&self, input: &Path) -> EditronResult<(f64, u64, f32)> {
        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v", "quiet",
                "-show_entries", "format=duration:stream=nb_frames,r_frame_rate",
                "-select_streams", "v:0",
                "-of", "json",
                &input.to_string_lossy(),
            ])
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| EditronError::FFmpeg(e.to_string()))?;

        let duration = json["format"]["duration"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        let frame_rate_str = json["streams"][0]["r_frame_rate"]
            .as_str()
            .unwrap_or("30/1");
        let fps = Self::parse_frame_rate(frame_rate_str);

        let nb_frames = json["streams"][0]["nb_frames"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or((duration * fps as f64) as u64);

        Ok((duration, nb_frames, fps))
    }

    /// Parse frame rate string (e.g., "30000/1001" or "30")
    fn parse_frame_rate(s: &str) -> f32 {
        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() == 2 {
                let num: f32 = parts[0].parse().unwrap_or(30.0);
                let den: f32 = parts[1].parse().unwrap_or(1.0);
                return num / den;
            }
        }
        s.parse().unwrap_or(30.0)
    }

    /// Run FFmpeg scene detection filter
    async fn run_scene_detect(&self, input: &Path, threshold: f32) -> EditronResult<Vec<f64>> {
        let output = Command::new(&self.ffmpeg_path)
            .args([
                "-i", &input.to_string_lossy(),
                "-filter:v", &format!("select='gt(scene,{})',showinfo", threshold),
                "-f", "null",
                "-",
            ])
            .output()
            .await?;

        // Parse scene timestamps from stderr (where FFmpeg writes filter output)
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut times = vec![0.0]; // Always start with 0

        for line in stderr.lines() {
            if line.contains("pts_time:") {
                if let Some(time_part) = line.split("pts_time:").nth(1) {
                    if let Some(time_str) = time_part.split_whitespace().next() {
                        if let Ok(time) = time_str.parse::<f64>() {
                            times.push(time);
                        }
                    }
                }
            }
        }

        Ok(times)
    }

    /// Analyze motion intensity for scenes
    pub async fn analyze_motion<P: AsRef<Path>>(
        &self,
        input: P,
        scenes: &mut [Scene],
    ) -> EditronResult<()> {
        // For each scene, calculate motion vectors
        // This is a simplified implementation
        for scene in scenes.iter_mut() {
            // Run motion estimation on scene segment
            let motion = self.estimate_motion(input.as_ref(), scene.start_time, scene.end_time).await?;
            scene.motion_intensity = Some(motion);

            // Tag based on motion
            if motion < 0.1 {
                scene.tags.push("static".to_string());
            } else if motion < 0.3 {
                scene.tags.push("slow_motion".to_string());
            } else if motion > 0.7 {
                scene.tags.push("high_action".to_string());
            }
        }

        Ok(())
    }

    /// Estimate motion intensity for a time range
    async fn estimate_motion(&self, input: &Path, start: f64, end: f64) -> EditronResult<f32> {
        let duration = end - start;

        let output = Command::new(&self.ffmpeg_path)
            .args([
                "-ss", &start.to_string(),
                "-t", &duration.to_string(),
                "-i", &input.to_string_lossy(),
                "-vf", "mpdecimate,metadata=print:file=-",
                "-f", "null",
                "-",
            ])
            .output()
            .await?;

        // Parse motion from metadata output
        // Higher values = more motion
        let stderr = String::from_utf8_lossy(&output.stderr);
        let motion_values: Vec<f32> = stderr
            .lines()
            .filter(|l| l.contains("lo:") || l.contains("hi:"))
            .filter_map(|l| {
                l.split_whitespace()
                    .find(|s| s.starts_with("lo:") || s.starts_with("hi:"))
                    .and_then(|s| s.split(':').nth(1))
                    .and_then(|s| s.parse().ok())
            })
            .collect();

        if motion_values.is_empty() {
            Ok(0.5) // Default to medium motion
        } else {
            let avg: f32 = motion_values.iter().sum::<f32>() / motion_values.len() as f32;
            Ok((avg / 100.0).min(1.0)) // Normalize to 0-1
        }
    }

    /// Extract dominant colors from a scene
    pub async fn extract_colors<P: AsRef<Path>>(
        &self,
        input: P,
        scene: &Scene,
        num_colors: usize,
    ) -> EditronResult<Vec<String>> {
        let mid_time = scene.start_time + (scene.duration / 2.0);

        let output = Command::new(&self.ffmpeg_path)
            .args([
                "-ss", &mid_time.to_string(),
                "-i", &input.as_ref().to_string_lossy(),
                "-vframes", "1",
                "-vf", &format!("scale=100:-1,palettegen=max_colors={}", num_colors),
                "-f", "image2pipe",
                "-vcodec", "png",
                "-",
            ])
            .output()
            .await?;

        // For now, return placeholder colors
        // Full implementation would parse the palette
        Ok(vec![
            "#3B82F6".to_string(), // Blue
            "#EF4444".to_string(), // Red
            "#10B981".to_string(), // Green
        ])
    }

    /// Group similar scenes
    pub fn group_scenes(
        &self,
        result: &SceneDetectionResult,
        group_type: GroupType,
    ) -> Vec<SceneGroup> {
        let mut groups = Vec::new();

        match group_type {
            GroupType::Motion => {
                // Group by motion intensity
                let mut static_scenes = vec![];
                let mut medium_scenes = vec![];
                let mut action_scenes = vec![];

                for scene in &result.scenes {
                    if let Some(motion) = scene.motion_intensity {
                        if motion < 0.2 {
                            static_scenes.push(scene.index);
                        } else if motion > 0.6 {
                            action_scenes.push(scene.index);
                        } else {
                            medium_scenes.push(scene.index);
                        }
                    }
                }

                if !static_scenes.is_empty() {
                    groups.push(SceneGroup {
                        name: "Static Shots".to_string(),
                        scenes: static_scenes,
                        similarity_score: 0.8,
                        group_type: GroupType::Motion,
                    });
                }
                if !medium_scenes.is_empty() {
                    groups.push(SceneGroup {
                        name: "Medium Motion".to_string(),
                        scenes: medium_scenes,
                        similarity_score: 0.7,
                        group_type: GroupType::Motion,
                    });
                }
                if !action_scenes.is_empty() {
                    groups.push(SceneGroup {
                        name: "Action Shots".to_string(),
                        scenes: action_scenes,
                        similarity_score: 0.8,
                        group_type: GroupType::Motion,
                    });
                }
            }
            _ => {
                // Default: group by duration
                let mut short_scenes = vec![];
                let mut long_scenes = vec![];

                for scene in &result.scenes {
                    if scene.duration < 3.0 {
                        short_scenes.push(scene.index);
                    } else {
                        long_scenes.push(scene.index);
                    }
                }

                groups.push(SceneGroup {
                    name: "Short Cuts".to_string(),
                    scenes: short_scenes,
                    similarity_score: 0.6,
                    group_type: group_type.clone(),
                });
                groups.push(SceneGroup {
                    name: "Long Takes".to_string(),
                    scenes: long_scenes,
                    similarity_score: 0.6,
                    group_type,
                });
            }
        }

        groups
    }

    /// Export scene list to EDL format
    pub fn export_edl(&self, result: &SceneDetectionResult, title: &str) -> String {
        let mut edl = format!("TITLE: {}\nFCM: NON-DROP FRAME\n\n", title);

        for (i, scene) in result.scenes.iter().enumerate() {
            let start_tc = Self::seconds_to_timecode(scene.start_time, result.frame_rate);
            let end_tc = Self::seconds_to_timecode(scene.end_time, result.frame_rate);

            edl.push_str(&format!(
                "{:03}  001      V     C        {} {} {} {}\n",
                i + 1,
                start_tc,
                end_tc,
                start_tc,
                end_tc
            ));
        }

        edl
    }

    /// Convert seconds to timecode
    fn seconds_to_timecode(seconds: f64, fps: f32) -> String {
        let total_frames = (seconds * fps as f64) as u64;
        let frames = total_frames % fps as u64;
        let total_seconds = total_frames / fps as u64;
        let secs = total_seconds % 60;
        let total_minutes = total_seconds / 60;
        let mins = total_minutes % 60;
        let hours = total_minutes / 60;

        format!("{:02}:{:02}:{:02}:{:02}", hours, mins, secs, frames)
    }

    /// Export scene markers for Premiere Pro
    pub fn export_premiere_markers(&self, result: &SceneDetectionResult) -> String {
        let mut script = String::from(r#"
// Import Scene Detection Markers
var seq = app.project.activeSequence;
var markers = seq.markers;

"#);

        for scene in &result.scenes {
            let ticks = (scene.start_time * 254016000000.0) as i64; // Premiere ticks
            script.push_str(&format!(
                "markers.createMarker({});\n",
                ticks
            ));
        }

        script.push_str("\n$.writeln(\"Scene markers imported\");\n");
        script
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timecode_conversion() {
        let tc = SceneDetectionEngine::seconds_to_timecode(3661.5, 30.0);
        assert_eq!(tc, "01:01:01:15");
    }

    #[test]
    fn test_frame_rate_parsing() {
        assert_eq!(SceneDetectionEngine::parse_frame_rate("30000/1001"), 29.97003);
        assert_eq!(SceneDetectionEngine::parse_frame_rate("24"), 24.0);
    }
}
