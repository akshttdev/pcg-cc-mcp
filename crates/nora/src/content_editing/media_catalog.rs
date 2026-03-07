//! Media Cataloger — Phase 4
//!
//! Walks a source directory tree, runs `ffprobe` on each media file,
//! and classifies assets by type (Interview, Drone, Cinematic, BTS, etc.).

use std::path::{Path, PathBuf};

use serde_json::Value;
use tokio::process::Command;

use services::services::beat_analysis::BeatAnalysisEngine;
use services::services::scene_analysis::SceneAnalysisEngine;

use crate::{NoraError, Result};

use super::types::*;

/// Media file extensions to catalog.
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "mxf", "avi", "mkv", "m4v", "mpg", "mpeg", "wmv", "webm",
];
const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "aac", "flac", "ogg", "m4a", "aif", "aiff"];

/// Catalogs media files in a directory tree using `ffprobe`.
/// Optionally runs deep scene and beat analysis on discovered assets.
pub struct MediaCataloger {
    scene_engine: SceneAnalysisEngine,
    beat_engine: BeatAnalysisEngine,
    enable_scene_analysis: bool,
    enable_beat_analysis: bool,
}

impl MediaCataloger {
    pub fn new() -> Self {
        Self {
            scene_engine: SceneAnalysisEngine::new(),
            beat_engine: BeatAnalysisEngine::new(),
            enable_scene_analysis: true,
            enable_beat_analysis: true,
        }
    }

    pub fn with_analysis_flags(mut self, scene: bool, beat: bool) -> Self {
        self.enable_scene_analysis = scene;
        self.enable_beat_analysis = beat;
        self
    }

    /// Catalog all media files under the given root directory.
    pub async fn catalog_directory(&self, root: &Path) -> Result<ShotCatalog> {
        tracing::info!(
            "[MEDIA_CATALOG] Cataloging directory: {}",
            root.display()
        );

        let media_files = self.find_media_files(root)?;
        tracing::info!(
            "[MEDIA_CATALOG] Found {} media files",
            media_files.len()
        );

        let mut assets: Vec<MediaAsset> = Vec::new();
        let mut interview_indices: Vec<usize> = Vec::new();
        let mut broll_indices: Vec<usize> = Vec::new();
        let mut music_indices: Vec<usize> = Vec::new();
        let mut total_duration = 0.0;

        for path in &media_files {
            match self.probe_file(path).await {
                Ok(asset) => {
                    let idx = assets.len();
                    total_duration += asset.duration_seconds;

                    match asset.media_type {
                        MediaType::Interview => interview_indices.push(idx),
                        MediaType::Drone | MediaType::Cinematic | MediaType::BTS | MediaType::VerticalHighlight => {
                            broll_indices.push(idx)
                        }
                        MediaType::Music => music_indices.push(idx),
                        _ => {}
                    }

                    assets.push(asset);
                }
                Err(e) => {
                    tracing::warn!(
                        "[MEDIA_CATALOG] Failed to probe {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        tracing::info!(
            "[MEDIA_CATALOG] Cataloged {} assets: {} interview, {} B-roll, {} music",
            assets.len(),
            interview_indices.len(),
            broll_indices.len(),
            music_indices.len()
        );

        // === Phase 4a: Deep Scene Analysis (parallel per B-roll clip) ===
        if self.enable_scene_analysis && !broll_indices.is_empty() {
            tracing::info!(
                "[MEDIA_CATALOG] Running scene analysis on {} B-roll clips",
                broll_indices.len()
            );

            let mut handles = Vec::new();
            for &idx in &broll_indices {
                let path = assets[idx].path.clone();
                let engine = self.scene_engine.clone();
                handles.push((
                    idx,
                    tokio::spawn(async move { engine.analyze_clip(&path, 3.0).await }),
                ));
            }

            for (idx, handle) in handles {
                match handle.await {
                    Ok(Ok(clip_analysis)) => {
                        let analysis = convert_scene_analysis(&clip_analysis);
                        let scene_tags = generate_scene_tags(&analysis);
                        assets[idx].content_tags.extend(scene_tags);
                        assets[idx].content_tags.sort();
                        assets[idx].content_tags.dedup();
                        assets[idx].energy_level = energy_from_score(analysis.overall_energy);
                        assets[idx].scene_analysis = Some(analysis);
                    }
                    Ok(Err(e)) => tracing::warn!(
                        "[MEDIA_CATALOG] Scene analysis failed for {}: {}",
                        assets[idx].filename,
                        e
                    ),
                    Err(e) => tracing::warn!(
                        "[MEDIA_CATALOG] Scene analysis task panicked for {}: {}",
                        assets[idx].filename,
                        e
                    ),
                }
            }
        }

        // === Phase 4b: Beat Analysis (parallel per music track) ===
        let mut music_beat_grid = None;
        if self.enable_beat_analysis && !music_indices.is_empty() {
            tracing::info!(
                "[MEDIA_CATALOG] Running beat analysis on {} music tracks",
                music_indices.len()
            );

            let mut handles = Vec::new();
            for &idx in &music_indices {
                let path = assets[idx].path.clone();
                let engine = self.beat_engine.clone();
                handles.push((
                    idx,
                    tokio::spawn(async move { engine.analyze(&path, None, 4).await }),
                ));
            }

            for (idx, handle) in handles {
                match handle.await {
                    Ok(Ok(beat_result)) => {
                        let analysis = convert_beat_analysis(&beat_result);
                        if music_beat_grid.is_none() {
                            music_beat_grid = Some(analysis.clone());
                        }
                        assets[idx].beat_analysis = Some(analysis);
                    }
                    Ok(Err(e)) => tracing::warn!(
                        "[MEDIA_CATALOG] Beat analysis failed for {}: {}",
                        assets[idx].filename,
                        e
                    ),
                    Err(e) => tracing::warn!(
                        "[MEDIA_CATALOG] Beat analysis task panicked for {}: {}",
                        assets[idx].filename,
                        e
                    ),
                }
            }
        }

        Ok(ShotCatalog {
            assets,
            total_duration_seconds: total_duration,
            interview_assets: interview_indices,
            broll_assets: broll_indices,
            music_assets: music_indices,
            music_beat_grid,
        })
    }

    /// Find interview video files in the source directory (for Phase 2 transcription).
    pub fn find_interview_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let all_files = self.find_media_files(root)?;

        let interview_files: Vec<PathBuf> = all_files
            .into_iter()
            .filter(|p| {
                let path_str = p.to_string_lossy().to_lowercase();
                // Match interview / testimonial folders and video extensions
                (path_str.contains("testimonial") || path_str.contains("interview"))
                    && is_video_file(p)
            })
            .collect();

        Ok(interview_files)
    }

    /// Recursively find all media files under a directory.
    fn find_media_files(&self, root: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.walk_dir(root, &mut files)?;
        files.sort();
        Ok(files)
    }

    /// Recursive directory walker.
    fn walk_dir(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| NoraError::IoError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| NoraError::IoError(e))?;
            let path = entry.path();

            if path.is_dir() {
                // Skip reference/example folders
                let dir_name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                if dir_name.contains("example") || dir_name.contains("reference") {
                    tracing::debug!(
                        "[MEDIA_CATALOG] Skipping reference folder: {}",
                        path.display()
                    );
                    continue;
                }

                self.walk_dir(&path, files)?;
            } else if is_media_file(&path) {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Run `ffprobe` on a single file and return a MediaAsset.
    async fn probe_file(&self, path: &Path) -> Result<MediaAsset> {
        let output = Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_streams",
                "-show_format",
                path.to_str().unwrap_or(""),
            ])
            .output()
            .await
            .map_err(|e| NoraError::ExecutionError(format!("ffprobe failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(NoraError::ExecutionError(format!(
                "ffprobe error for {}: {}",
                path.display(),
                stderr
            )));
        }

        let json: Value = serde_json::from_slice(&output.stdout).map_err(|e| {
            NoraError::ExecutionError(format!("Failed to parse ffprobe JSON: {}", e))
        })?;

        let format = &json["format"];
        let duration = format["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let file_size = format["size"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        // Find video stream for dimensions/codec
        let (width, height, codec) = json["streams"]
            .as_array()
            .and_then(|streams| {
                streams.iter().find(|s| s["codec_type"].as_str() == Some("video"))
            })
            .map(|vs| {
                (
                    vs["width"].as_u64().unwrap_or(0) as u32,
                    vs["height"].as_u64().unwrap_or(0) as u32,
                    vs["codec_name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                )
            })
            .unwrap_or_else(|| {
                // Audio-only file
                let codec = json["streams"]
                    .as_array()
                    .and_then(|s| s.first())
                    .and_then(|s| s["codec_name"].as_str())
                    .unwrap_or("unknown")
                    .to_string();
                (0, 0, codec)
            });

        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        let parent_folder = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        let media_type = classify_media(path, &parent_folder, &filename, width, height);
        let camera = extract_camera_designation(&filename);
        let energy_level = classify_energy(path, &parent_folder, &filename, &media_type);
        let content_tags = generate_content_tags(&parent_folder, &filename, &media_type, width, height);

        // Extract fps from video stream
        let fps = json["streams"]
            .as_array()
            .and_then(|streams| {
                streams.iter().find(|s| s["codec_type"].as_str() == Some("video"))
            })
            .and_then(|vs| {
                // Parse "30/1" or "29.97" format
                vs["r_frame_rate"].as_str().and_then(|r| {
                    if let Some(slash) = r.find('/') {
                        let num: f64 = r[..slash].parse().ok()?;
                        let den: f64 = r[slash + 1..].parse().ok()?;
                        if den > 0.0 { Some(num / den) } else { None }
                    } else {
                        r.parse().ok()
                    }
                })
            })
            .unwrap_or(24.0);

        Ok(MediaAsset {
            filename,
            path: path.to_path_buf(),
            duration_seconds: duration,
            width,
            height,
            codec,
            media_type,
            camera,
            file_size_bytes: file_size,
            parent_folder,
            energy_level,
            content_tags,
            fps,
            scene_analysis: None,
            visual_qc: None,
            beat_analysis: None,
        })
    }
}

/// Classify a media file based on its path, parent folder, and filename.
fn classify_media(
    path: &Path,
    parent_folder: &str,
    filename: &str,
    width: u32,
    height: u32,
) -> MediaType {
    let path_lower = path.to_string_lossy().to_lowercase();
    let parent_lower = parent_folder.to_lowercase();
    let name_lower = filename.to_lowercase();

    // Audio files
    if is_audio_file(path) {
        return MediaType::Music;
    }

    // Reference / example folders
    if path_lower.contains("example") || path_lower.contains("reference") {
        return MediaType::Reference;
    }

    // Interview / testimonial
    if parent_lower.contains("testimonial") || parent_lower.contains("interview") {
        return MediaType::Interview;
    }

    // Drone footage (DJI prefix is common)
    if name_lower.contains("dji") || parent_lower.contains("drone") {
        return MediaType::Drone;
    }

    // Cinematic B-roll
    if parent_lower.contains("cinematic") {
        return MediaType::Cinematic;
    }

    // Behind the scenes / production team
    if parent_lower.contains("production") || parent_lower.contains("bts") || parent_lower.contains("behind") {
        return MediaType::BTS;
    }

    // Vertical highlights (MOT Day, etc.)
    if name_lower.contains("mot day") || parent_lower.contains("mot day") {
        return MediaType::VerticalHighlight;
    }

    // Vertical video detection (height > width)
    if height > width && width > 0 {
        return MediaType::VerticalHighlight;
    }

    // B-Roll folder — classify by filename patterns
    if parent_lower.contains("b-roll") || parent_lower.contains("broll") {
        if name_lower.contains("arv") || name_lower.contains("action") {
            return MediaType::EventAction;
        }
        if name_lower.contains("cinema") {
            return MediaType::Cinematic;
        }
        // DJI already caught above; remaining clips are general event action
        return MediaType::EventAction;
    }

    MediaType::Unknown
}

/// Classify energy level based on file metadata and naming patterns.
fn classify_energy(
    path: &Path,
    parent_folder: &str,
    filename: &str,
    media_type: &MediaType,
) -> EnergyLevel {
    let name_lower = filename.to_lowercase();
    let parent_lower = parent_folder.to_lowercase();

    // Drone footage is generally low-medium (establishing shots)
    if *media_type == MediaType::Drone {
        return EnergyLevel::Medium;
    }

    // Cinematic B-roll with higher frame rates (60fps) suggests action
    if parent_lower.contains("cinematic") {
        return EnergyLevel::Medium;
    }

    // ARv (Action Review) clips from B-Roll tend to be high energy
    if name_lower.contains("arv") {
        return EnergyLevel::High;
    }

    // BTS footage is typically medium
    if *media_type == MediaType::BTS {
        return EnergyLevel::Medium;
    }

    // Vertical highlights are usually high energy
    if *media_type == MediaType::VerticalHighlight {
        return EnergyLevel::High;
    }

    // Interview footage is low energy (talking head)
    if *media_type == MediaType::Interview {
        return EnergyLevel::Low;
    }

    // Keywords that suggest energy level
    if name_lower.contains("crowd") || name_lower.contains("keynote") || name_lower.contains("stage") {
        return EnergyLevel::High;
    }
    if name_lower.contains("setup") || name_lower.contains("lobby") || name_lower.contains("exterior") {
        return EnergyLevel::Low;
    }

    EnergyLevel::Medium
}

/// Generate content tags based on filename patterns and classification.
fn generate_content_tags(
    parent_folder: &str,
    filename: &str,
    media_type: &MediaType,
    width: u32,
    height: u32,
) -> Vec<String> {
    let mut tags = Vec::new();
    let name_lower = filename.to_lowercase();
    let parent_lower = parent_folder.to_lowercase();

    // Type-based tags
    match media_type {
        MediaType::Drone => tags.push("aerial".to_string()),
        MediaType::Cinematic => tags.push("cinematic".to_string()),
        MediaType::BTS => tags.push("behind-the-scenes".to_string()),
        MediaType::VerticalHighlight => tags.push("vertical".to_string()),
        MediaType::EventAction => tags.push("event".to_string()),
        MediaType::Interview => tags.push("interview".to_string()),
        _ => {}
    }

    // Resolution tags
    if width >= 3840 {
        tags.push("4k".to_string());
    } else if width >= 1920 {
        tags.push("hd".to_string());
    }

    // Vertical detection
    if height > width && width > 0 {
        tags.push("vertical".to_string());
    }

    // Content-specific tags from filenames
    if name_lower.contains("dji") {
        tags.push("drone".to_string());
    }
    if name_lower.contains("arv") {
        tags.push("action-review".to_string());
    }
    if parent_lower.contains("production") || parent_lower.contains("team") {
        tags.push("team".to_string());
    }
    if parent_lower.contains("mot day") || name_lower.contains("mot") {
        tags.push("highlight-reel".to_string());
    }

    tags.sort();
    tags.dedup();
    tags
}

/// Extract camera designation from filename (e.g., "CAM_A", "CAM_B").
fn extract_camera_designation(filename: &str) -> Option<String> {
    let upper = filename.to_uppercase();
    if upper.contains("CAM_A") || upper.contains("CAMA") || upper.contains("CAM A") {
        Some("CAM_A".to_string())
    } else if upper.contains("CAM_B") || upper.contains("CAMB") || upper.contains("CAM B") {
        Some("CAM_B".to_string())
    } else if upper.contains("CAM_C") || upper.contains("CAMC") || upper.contains("CAM C") {
        Some("CAM_C".to_string())
    } else {
        None
    }
}

/// Check if a path has a video file extension.
fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| VIDEO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check if a path has an audio file extension.
fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Check if a path is any media file (video or audio).
fn is_media_file(path: &Path) -> bool {
    is_video_file(path) || is_audio_file(path)
}

// ---------------------------------------------------------------------------
// Analysis conversion helpers
// ---------------------------------------------------------------------------

/// Convert SceneAnalysisEngine's ContentType to our SceneContentType.
fn convert_content_type(
    ct: &services::services::scene_analysis::ContentType,
) -> SceneContentType {
    match ct {
        services::services::scene_analysis::ContentType::HighEnergy => SceneContentType::HighEnergy,
        services::services::scene_analysis::ContentType::Establishing => {
            SceneContentType::Establishing
        }
        services::services::scene_analysis::ContentType::Intimate => SceneContentType::Intimate,
        services::services::scene_analysis::ContentType::Transition => SceneContentType::Transition,
        services::services::scene_analysis::ContentType::Ambient => SceneContentType::Ambient,
    }
}

/// Convert a ClipAnalysis from the scene analysis engine into our pipeline type.
fn convert_scene_analysis(
    clip: &services::services::scene_analysis::ClipAnalysis,
) -> ClipSceneAnalysis {
    ClipSceneAnalysis {
        segments: clip
            .segments
            .iter()
            .map(|s| SceneSegment {
                timestamp: s.timestamp,
                duration: s.duration,
                brightness: s.brightness,
                motion_intensity: s.motion_intensity,
                complexity: s.complexity,
                energy_score: s.energy_score,
                content_type: convert_content_type(&s.content_type),
            })
            .collect(),
        overall_energy: clip.overall_energy,
        peak_energy_timestamp: clip.peak_energy_timestamp,
        dominant_content_type: convert_content_type(&clip.dominant_content_type),
        usable: clip.usable,
    }
}

/// Convert a BeatGridResult from the beat analysis engine into our pipeline type.
fn convert_beat_analysis(
    beat: &services::services::beat_analysis::BeatGridResult,
) -> MusicBeatAnalysis {
    MusicBeatAnalysis {
        bpm: beat.bpm,
        beat_interval: beat.beat_interval,
        total_beats: beat.total_beats,
        beats_per_bar: beat.beats_per_bar,
        beats: beat
            .beats
            .iter()
            .map(|b| BeatPoint {
                timestamp: b.timestamp,
                beat_number: b.beat_number,
                bar_number: b.bar_number,
                beat_in_bar: b.beat_in_bar,
                is_downbeat: b.is_downbeat,
                energy_at_beat: b.energy_at_beat,
                is_strong_cut_point: b.is_strong_cut_point,
            })
            .collect(),
        sections: beat
            .sections
            .iter()
            .map(|s| MusicStructureSection {
                name: s.name.clone(),
                start: s.start,
                end: s.end,
                energy_level: s.energy_level,
                suggested_content: format!("{:?}", s.suggested_content),
            })
            .collect(),
        energy_curve: beat
            .energy_curve
            .iter()
            .map(|e| MusicEnergyPoint {
                timestamp: e.timestamp,
                normalized_energy: e.normalized_energy,
            })
            .collect(),
    }
}

/// Map a measured energy score to the heuristic EnergyLevel enum.
fn energy_from_score(energy: f64) -> EnergyLevel {
    if energy > 0.6 {
        EnergyLevel::High
    } else if energy > 0.3 {
        EnergyLevel::Medium
    } else {
        EnergyLevel::Low
    }
}

/// Generate semantic content tags from scene analysis results.
fn generate_scene_tags(analysis: &ClipSceneAnalysis) -> Vec<String> {
    let mut tags = Vec::new();

    // Content type tags
    match analysis.dominant_content_type {
        SceneContentType::HighEnergy => tags.push("high-energy".to_string()),
        SceneContentType::Establishing => {
            tags.push("establishing".to_string());
            tags.push("wide-shot".to_string());
        }
        SceneContentType::Intimate => {
            tags.push("intimate".to_string());
            tags.push("close-up".to_string());
        }
        SceneContentType::Transition => tags.push("transition".to_string()),
        SceneContentType::Ambient => tags.push("ambient".to_string()),
    }

    // Energy level tags
    if analysis.overall_energy > 0.7 {
        tags.push("peak-energy".to_string());
    } else if analysis.overall_energy > 0.5 {
        tags.push("energetic".to_string());
    } else if analysis.overall_energy > 0.3 {
        tags.push("building".to_string());
    } else {
        tags.push("calm".to_string());
    }

    // Motion intensity tags (average across segments)
    if !analysis.segments.is_empty() {
        let avg_motion = analysis
            .segments
            .iter()
            .map(|s| s.motion_intensity)
            .sum::<f64>()
            / analysis.segments.len() as f64;

        if avg_motion > 0.5 {
            tags.push("fast-motion".to_string());
        } else if avg_motion > 0.2 {
            tags.push("moderate-motion".to_string());
        } else {
            tags.push("static".to_string());
        }

        // Energy ramp detection
        if analysis.segments.len() >= 2 {
            let first_energy = analysis.segments.first().unwrap().energy_score;
            let last_energy = analysis.segments.last().unwrap().energy_score;
            if last_energy > first_energy * 1.5 {
                tags.push("energy-ramp-up".to_string());
            }
            if first_energy > last_energy * 1.5 {
                tags.push("energy-ramp-down".to_string());
            }
        }

        // Peak moment detection
        let peak = analysis
            .segments
            .iter()
            .map(|s| s.energy_score)
            .fold(0.0f64, f64::max);
        if peak > 0.7 {
            tags.push("has-peak-moment".to_string());
        }
    }

    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_interview() {
        let path = Path::new("/media/testimonial DAVE/clip001.mp4");
        let mt = classify_media(path, "testimonial DAVE", "clip001.mp4", 1920, 1080);
        assert_eq!(mt, MediaType::Interview);
    }

    #[test]
    fn test_classify_drone() {
        let path = Path::new("/media/B-Roll/DJI_0001.mp4");
        let mt = classify_media(path, "B-Roll", "DJI_0001.mp4", 3840, 2160);
        assert_eq!(mt, MediaType::Drone);
    }

    #[test]
    fn test_classify_arv_event_action() {
        let path = Path::new("/media/B-Roll/ARv1_7974.MP4");
        let mt = classify_media(path, "B-Roll", "ARv1_7974.MP4", 3840, 2160);
        assert_eq!(mt, MediaType::EventAction);
    }

    #[test]
    fn test_classify_vertical() {
        let path = Path::new("/media/clips/story.mp4");
        let mt = classify_media(path, "clips", "story.mp4", 1080, 1920);
        assert_eq!(mt, MediaType::VerticalHighlight);
    }

    #[test]
    fn test_classify_music() {
        let path = Path::new("/media/music/track.wav");
        let mt = classify_media(path, "music", "track.wav", 0, 0);
        assert_eq!(mt, MediaType::Music);
    }

    #[test]
    fn test_energy_classification() {
        let arv_energy = classify_energy(
            Path::new("/media/B-Roll/ARv1_7974.MP4"),
            "B-Roll", "ARv1_7974.MP4", &MediaType::EventAction
        );
        assert_eq!(arv_energy, EnergyLevel::High);

        let drone_energy = classify_energy(
            Path::new("/media/B-Roll/DJI_0001.mp4"),
            "B-Roll", "DJI_0001.mp4", &MediaType::Drone
        );
        assert_eq!(drone_energy, EnergyLevel::Medium);

        let interview_energy = classify_energy(
            Path::new("/media/testimonial DAVE/clip.mp4"),
            "testimonial DAVE", "clip.mp4", &MediaType::Interview
        );
        assert_eq!(interview_energy, EnergyLevel::Low);
    }

    #[test]
    fn test_content_tags() {
        let tags = generate_content_tags("B-Roll", "ARv1_7974.MP4", &MediaType::EventAction, 3840, 2160);
        assert!(tags.contains(&"4k".to_string()));
        assert!(tags.contains(&"event".to_string()));
        assert!(tags.contains(&"action-review".to_string()));
    }

    #[test]
    fn test_camera_designation() {
        assert_eq!(
            extract_camera_designation("DAVE_CAM_A_001.mp4"),
            Some("CAM_A".to_string())
        );
        assert_eq!(extract_camera_designation("random_clip.mp4"), None);
    }

    #[test]
    fn test_is_video_file() {
        assert!(is_video_file(Path::new("clip.mp4")));
        assert!(is_video_file(Path::new("clip.MOV")));
        assert!(!is_video_file(Path::new("track.wav")));
        assert!(!is_video_file(Path::new("notes.txt")));
    }
}
