//! Editron - Video Editing Service
//!
//! This module provides video editing capabilities through:
//! - FFmpeg for CLI-based video processing
//! - Adobe Premiere Pro automation via ExtendScript
//! - Adobe Media Encoder for rendering
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Editron Service                          │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
//! │  │   FFmpeg     │  │  Premiere    │  │  Media Encoder   │  │
//! │  │   Wrapper    │  │  Pro Bridge  │  │     Bridge       │  │
//! │  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘  │
//! │         │                 │                    │            │
//! │         └─────────────────┼────────────────────┘            │
//! │                           │                                 │
//! │                  ┌────────▼────────┐                       │
//! │                  │  EditronService │                       │
//! │                  └─────────────────┘                       │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod ffmpeg;
pub mod premiere;
pub mod encoder;
pub mod color_grading;
pub mod transitions;
pub mod audio;
pub mod proxy;
pub mod scene_detection;
pub mod music;
pub mod music_automation;
pub mod edit_assembly;
pub mod premiere_xml;
pub mod premiere_prproj;
pub mod artlist;
// visual_qc lives as a standalone module at services::services::visual_qc

use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

pub use ffmpeg::FFmpegClient;
pub use premiere::PremiereProBridge;
pub use encoder::MediaEncoderBridge;
pub use color_grading::{ColorGradingEngine, ColorGradePreset, LUT, ColorWheels, ColorCurves};
pub use transitions::{TransitionEngine, Transition, TransitionPreset, TransitionCategory, EasingCurve};
pub use audio::{AudioProcessingEngine, AudioProcessingPreset, LoudnessStandard, CompressionSettings};
pub use proxy::{ProxyWorkflowManager, ProxyPreset, ProxySettings, ProxyFile};
pub use scene_detection::{SceneDetectionEngine, Scene, SceneDetectionResult, DetectionMethod};
pub use music::{MusicLibrary, MusicTrack, MusicSearchCriteria, MusicMood, MusicGenre, MusicRecommendation, AudioAnalysis, MusicPlatform};
pub use artlist::{ArtlistClient, ArtlistConfig, ArtlistError};
pub use edit_assembly::{
    EditAssemblyEngine, AssembledEdit, FootageClip, MusicAnalysis as EditMusicAnalysis,
    MusicSection, SectionType, PacingStyle, TimelineClip, AudioClip, AssemblyConfig,
    TransitionStyle, EditMarker, MarkerType,
};
pub use premiere_xml::PremiereXmlExporter;
pub use premiere_prproj::{PrprojRecutEngine, PrprojClipEntry, PrprojRecutResult};
pub use super::visual_qc::{
    VisualQcEngine, VisualQcConfig, VisualQcResult, ClipQcResult, AnalyzedFrame,
    SubjectRegion, CropRegion,
};

#[derive(Debug, Error)]
pub enum EditronError {
    #[error("FFmpeg error: {0}")]
    FFmpeg(String),

    #[error("Premiere Pro error: {0}")]
    PremierePro(String),

    #[error("Media Encoder error: {0}")]
    MediaEncoder(String),

    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Process error: {0}")]
    Process(String),
}

pub type EditronResult<T> = Result<T, EditronError>;

/// Video codec options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoCodec {
    H264,
    H265,
    ProRes,
    ProRes422,
    ProRes4444,
    DNxHD,
    VP9,
    AV1,
}

impl VideoCodec {
    pub fn ffmpeg_codec(&self) -> &'static str {
        match self {
            VideoCodec::H264 => "libx264",
            VideoCodec::H265 => "libx265",
            VideoCodec::ProRes => "prores_ks",
            VideoCodec::ProRes422 => "prores_ks",
            VideoCodec::ProRes4444 => "prores_ks",
            VideoCodec::DNxHD => "dnxhd",
            VideoCodec::VP9 => "libvpx-vp9",
            VideoCodec::AV1 => "libaom-av1",
        }
    }

    pub fn prores_profile(&self) -> Option<&'static str> {
        match self {
            VideoCodec::ProRes => Some("3"),      // ProRes 422 HQ
            VideoCodec::ProRes422 => Some("2"),   // ProRes 422
            VideoCodec::ProRes4444 => Some("4"),  // ProRes 4444
            _ => None,
        }
    }
}

/// Audio codec options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioCodec {
    AAC,
    PCM,
    MP3,
    FLAC,
    Opus,
}

impl AudioCodec {
    pub fn ffmpeg_codec(&self) -> &'static str {
        match self {
            AudioCodec::AAC => "aac",
            AudioCodec::PCM => "pcm_s24le",
            AudioCodec::MP3 => "libmp3lame",
            AudioCodec::FLAC => "flac",
            AudioCodec::Opus => "libopus",
        }
    }
}

/// Export preset configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportPreset {
    pub name: String,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<f32>,
    pub bitrate: Option<String>,
    pub audio_bitrate: Option<String>,
    pub quality: Option<u8>,
}

impl ExportPreset {
    /// YouTube optimized preset
    pub fn youtube_1080p() -> Self {
        Self {
            name: "YouTube 1080p".to_string(),
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::AAC,
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(30.0),
            bitrate: Some("8M".to_string()),
            audio_bitrate: Some("192k".to_string()),
            quality: None,
        }
    }

    /// YouTube 4K preset
    pub fn youtube_4k() -> Self {
        Self {
            name: "YouTube 4K".to_string(),
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::AAC,
            width: Some(3840),
            height: Some(2160),
            frame_rate: Some(30.0),
            bitrate: Some("35M".to_string()),
            audio_bitrate: Some("256k".to_string()),
            quality: None,
        }
    }

    /// Instagram Reels preset
    pub fn instagram_reels() -> Self {
        Self {
            name: "Instagram Reels".to_string(),
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::AAC,
            width: Some(1080),
            height: Some(1920),
            frame_rate: Some(30.0),
            bitrate: Some("6M".to_string()),
            audio_bitrate: Some("128k".to_string()),
            quality: None,
        }
    }

    /// TikTok preset
    pub fn tiktok() -> Self {
        Self {
            name: "TikTok".to_string(),
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::AAC,
            width: Some(1080),
            height: Some(1920),
            frame_rate: Some(30.0),
            bitrate: Some("6M".to_string()),
            audio_bitrate: Some("128k".to_string()),
            quality: None,
        }
    }

    /// ProRes master preset
    pub fn prores_master() -> Self {
        Self {
            name: "ProRes Master".to_string(),
            video_codec: VideoCodec::ProRes422,
            audio_codec: AudioCodec::PCM,
            width: None,
            height: None,
            frame_rate: None,
            bitrate: None,
            audio_bitrate: None,
            quality: None,
        }
    }
}

/// Video metadata from probe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub path: PathBuf,
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f32,
    pub codec: String,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u32>,
    pub audio_sample_rate: Option<u32>,
    pub bitrate: Option<u64>,
    pub file_size: u64,
}

/// Edit operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EditOperation {
    /// Trim video to time range
    Trim {
        start_seconds: f64,
        end_seconds: f64,
    },
    /// Concatenate multiple clips
    Concat {
        clips: Vec<PathBuf>,
    },
    /// Add text overlay
    TextOverlay {
        text: String,
        position: TextPosition,
        font_size: u32,
        color: String,
        start_seconds: f64,
        duration_seconds: f64,
    },
    /// Add fade in/out
    Fade {
        fade_in_seconds: Option<f64>,
        fade_out_seconds: Option<f64>,
    },
    /// Scale/resize video
    Scale {
        width: u32,
        height: u32,
        maintain_aspect: bool,
    },
    /// Change speed
    Speed {
        factor: f32,
        maintain_pitch: bool,
    },
    /// Add audio track
    AddAudio {
        audio_path: PathBuf,
        start_seconds: f64,
        volume: f32,
    },
    /// Color correction
    ColorCorrect {
        brightness: f32,
        contrast: f32,
        saturation: f32,
        gamma: f32,
    },
}

/// Text position for overlays
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextPosition {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    Center,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Custom { x: u32, y: u32 },
}

/// The main Editron service
#[derive(Clone)]
pub struct EditronService {
    ffmpeg: Arc<FFmpegClient>,
    premiere: Arc<PremiereProBridge>,
    encoder: Arc<MediaEncoderBridge>,
    work_dir: PathBuf,
    presets: Arc<RwLock<Vec<ExportPreset>>>,
    // Advanced modules
    color_grading: Arc<ColorGradingEngine>,
    transitions: Arc<TransitionEngine>,
    audio: Arc<AudioProcessingEngine>,
    scene_detection: Arc<SceneDetectionEngine>,
    proxy_manager: Arc<RwLock<ProxyWorkflowManager>>,
    music_library: Arc<RwLock<MusicLibrary>>,
    // Music platform clients
    artlist_client: Arc<RwLock<Option<ArtlistClient>>>,
    // Visual QC engine (Spectra)
    visual_qc: Arc<VisualQcEngine>,
}

impl EditronService {
    pub async fn new<P: AsRef<Path>>(work_dir: P) -> EditronResult<Self> {
        let work_dir = work_dir.as_ref().to_path_buf();
        tokio::fs::create_dir_all(&work_dir).await?;

        let ffmpeg = Arc::new(FFmpegClient::new()?);
        let premiere = Arc::new(PremiereProBridge::new()?);
        let encoder = Arc::new(MediaEncoderBridge::new()?);

        let presets = vec![
            ExportPreset::youtube_1080p(),
            ExportPreset::youtube_4k(),
            ExportPreset::instagram_reels(),
            ExportPreset::tiktok(),
            ExportPreset::prores_master(),
        ];

        // Initialize advanced modules
        let lut_dir = work_dir.join("luts");
        tokio::fs::create_dir_all(&lut_dir).await?;
        let color_grading = Arc::new(ColorGradingEngine::new(&lut_dir));

        let transitions = Arc::new(TransitionEngine::new(24.0)); // Default 24fps
        let audio = Arc::new(AudioProcessingEngine::new());

        let ffmpeg_path = ffmpeg.ffmpeg_path().to_path_buf();
        let scene_detection = Arc::new(SceneDetectionEngine::new(&ffmpeg_path));

        let proxy_dir = work_dir.join("proxies");
        tokio::fs::create_dir_all(&proxy_dir).await?;
        let proxy_manager = Arc::new(RwLock::new(
            ProxyWorkflowManager::new(&proxy_dir, &ffmpeg_path)
        ));

        let music_dir = work_dir.join("music");
        tokio::fs::create_dir_all(&music_dir).await?;
        let music_library = Arc::new(RwLock::new(
            MusicLibrary::new(&music_dir, &ffmpeg_path)
        ));

        let visual_qc_dir = work_dir.join("visual_qc");
        tokio::fs::create_dir_all(&visual_qc_dir).await?;
        let visual_qc = Arc::new(VisualQcEngine::new(&ffmpeg_path, &visual_qc_dir));

        Ok(Self {
            ffmpeg,
            premiere,
            encoder,
            work_dir,
            presets: Arc::new(RwLock::new(presets)),
            color_grading,
            transitions,
            audio,
            scene_detection,
            proxy_manager,
            music_library,
            artlist_client: Arc::new(RwLock::new(None)),
            visual_qc,
        })
    }

    /// Probe video file for metadata
    pub async fn probe<P: AsRef<Path>>(&self, path: P) -> EditronResult<VideoMetadata> {
        self.ffmpeg.probe(path).await
    }

    /// Apply edit operations to video
    pub async fn apply_edits<P: AsRef<Path>>(
        &self,
        input: P,
        operations: Vec<EditOperation>,
        output: P,
        preset: Option<ExportPreset>,
    ) -> EditronResult<PathBuf> {
        self.ffmpeg.apply_edits(input, operations, output, preset).await
    }

    /// Quick trim video
    pub async fn trim<P: AsRef<Path>>(
        &self,
        input: P,
        start: f64,
        end: f64,
        output: P,
    ) -> EditronResult<PathBuf> {
        let ops = vec![EditOperation::Trim {
            start_seconds: start,
            end_seconds: end,
        }];
        self.apply_edits(input, ops, output, None).await
    }

    /// Concatenate multiple videos
    pub async fn concat<P: AsRef<Path>>(
        &self,
        inputs: Vec<P>,
        output: P,
        preset: Option<ExportPreset>,
    ) -> EditronResult<PathBuf> {
        self.ffmpeg.concat(inputs, output, preset).await
    }

    /// Export with preset
    pub async fn export<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        preset: ExportPreset,
    ) -> EditronResult<PathBuf> {
        self.ffmpeg.export(input, output, preset).await
    }

    /// Open project in Premiere Pro
    pub async fn open_in_premiere<P: AsRef<Path>>(&self, project: P) -> EditronResult<()> {
        self.premiere.open_project(project).await
    }

    /// Import media into Premiere Pro
    pub async fn import_to_premiere<P: AsRef<Path>>(&self, files: Vec<P>) -> EditronResult<()> {
        self.premiere.import_media(files).await
    }

    /// Export from Premiere Pro via Media Encoder
    pub async fn export_premiere_project<P: AsRef<Path>>(
        &self,
        project: P,
        preset_name: &str,
        output: P,
    ) -> EditronResult<PathBuf> {
        self.encoder.export(project, preset_name, output).await
    }

    /// Generate thumbnail from video
    pub async fn generate_thumbnail<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        time_seconds: f64,
    ) -> EditronResult<PathBuf> {
        self.ffmpeg.extract_frame(input, output, time_seconds).await
    }

    /// Extract audio from video
    pub async fn extract_audio<P: AsRef<Path>>(
        &self,
        input: P,
        output: P,
        codec: AudioCodec,
    ) -> EditronResult<PathBuf> {
        self.ffmpeg.extract_audio(input, output, codec).await
    }

    /// Get available presets
    pub async fn list_presets(&self) -> Vec<ExportPreset> {
        self.presets.read().await.clone()
    }

    /// Add custom preset
    pub async fn add_preset(&self, preset: ExportPreset) {
        self.presets.write().await.push(preset);
    }

    /// Get Premiere Pro status
    pub async fn premiere_status(&self) -> EditronResult<bool> {
        self.premiere.is_running().await
    }

    // ============ ADVANCED COLOR GRADING ============

    /// Get available color grade presets
    pub fn color_presets(&self) -> &[ColorGradePreset] {
        self.color_grading.presets()
    }

    /// Get a specific color preset
    pub fn get_color_preset(&self, name: &str) -> Option<&ColorGradePreset> {
        self.color_grading.get_preset(name)
    }

    /// Apply color grade to video
    pub async fn apply_color_grade<P: AsRef<Path>>(
        &self,
        input: P,
        preset: &ColorGradePreset,
        output: P,
    ) -> EditronResult<PathBuf> {
        let filter = self.color_grading.to_ffmpeg_filter(preset);
        self.ffmpeg.apply_filter(input, &filter, output).await
    }

    /// List available LUTs
    pub async fn list_luts(&self) -> EditronResult<Vec<LUT>> {
        self.color_grading.list_luts().await
    }

    // ============ TRANSITIONS ============

    /// Get available transition presets
    pub fn transition_presets(&self) -> &[TransitionPreset] {
        self.transitions.presets()
    }

    /// Get transitions by category
    pub fn transitions_by_category(&self, category: TransitionCategory) -> Vec<&TransitionPreset> {
        self.transitions.presets_by_category(category)
    }

    /// Search transition presets
    pub fn search_transitions(&self, query: &str) -> Vec<&TransitionPreset> {
        self.transitions.search_presets(query)
    }

    // ============ AUDIO PROCESSING ============

    /// Get audio processing presets
    pub fn audio_presets(&self) -> &[AudioProcessingPreset] {
        self.audio.presets()
    }

    /// Get specific audio preset
    pub fn get_audio_preset(&self, name: &str) -> Option<&AudioProcessingPreset> {
        self.audio.get_preset(name)
    }

    /// Apply audio processing to video/audio
    pub async fn apply_audio_processing<P: AsRef<Path>>(
        &self,
        input: P,
        preset: &AudioProcessingPreset,
        output: P,
    ) -> EditronResult<PathBuf> {
        let filter = self.audio.to_ffmpeg_filter(preset);
        self.ffmpeg.apply_audio_filter(input, &filter, output).await
    }

    /// Normalize audio loudness
    pub async fn normalize_loudness<P: AsRef<Path>>(
        &self,
        input: P,
        standard: LoudnessStandard,
        output: P,
    ) -> EditronResult<PathBuf> {
        let filter = format!(
            "loudnorm=I={}:TP={}:LRA=11",
            standard.target_lufs(),
            standard.true_peak_limit()
        );
        self.ffmpeg.apply_audio_filter(input, &filter, output).await
    }

    // ============ PROXY WORKFLOW ============

    /// Generate proxy for a video file
    pub async fn generate_proxy<P: AsRef<Path>>(
        &self,
        input: P,
        preset: ProxyPreset,
    ) -> EditronResult<ProxyFile> {
        self.proxy_manager.read().await.generate_proxy(input.as_ref(), preset).await
    }

    /// Generate proxies for multiple files
    pub async fn generate_proxies_batch(
        &self,
        files: Vec<PathBuf>,
        preset: ProxyPreset,
    ) -> Vec<EditronResult<ProxyFile>> {
        self.proxy_manager.read().await
            .generate_proxies_batch(files, preset, 4)
            .await
    }

    /// Get proxy storage summary
    pub async fn proxy_storage_summary(&self) -> (u64, u64, f32) {
        self.proxy_manager.read().await.storage_summary()
    }

    // ============ SCENE DETECTION ============

    /// Detect scenes in a video
    pub async fn detect_scenes<P: AsRef<Path>>(
        &self,
        input: P,
        method: DetectionMethod,
    ) -> EditronResult<SceneDetectionResult> {
        self.scene_detection.detect_scenes(input, method).await
    }

    /// Analyze motion in detected scenes
    pub async fn analyze_scene_motion<P: AsRef<Path>>(
        &self,
        input: P,
        scenes: &mut [Scene],
    ) -> EditronResult<()> {
        self.scene_detection.analyze_motion(input, scenes).await
    }

    /// Export scene detection to EDL
    pub fn export_scenes_edl(&self, result: &SceneDetectionResult, title: &str) -> String {
        self.scene_detection.export_edl(result, title)
    }

    /// Export scene markers for Premiere Pro
    pub fn export_premiere_markers(&self, result: &SceneDetectionResult) -> String {
        self.scene_detection.export_premiere_markers(result)
    }

    // ============ COMPREHENSIVE WORKFLOWS ============

    /// Full video processing pipeline
    pub async fn process_video<P: AsRef<Path>>(
        &self,
        input: P,
        color_preset: Option<&str>,
        audio_preset: Option<&str>,
        export_preset: Option<ExportPreset>,
        output: P,
    ) -> EditronResult<PathBuf> {
        let input = input.as_ref();
        let output = output.as_ref();

        // Build filter chain
        let mut video_filters = Vec::new();
        let mut audio_filters = Vec::new();

        // Add color grading if specified
        if let Some(preset_name) = color_preset {
            if let Some(preset) = self.get_color_preset(preset_name) {
                video_filters.push(self.color_grading.to_ffmpeg_filter(preset));
            }
        }

        // Add audio processing if specified
        if let Some(preset_name) = audio_preset {
            if let Some(preset) = self.get_audio_preset(preset_name) {
                audio_filters.push(self.audio.to_ffmpeg_filter(preset));
            }
        }

        // Apply combined filters
        self.ffmpeg.process_with_filters(
            input,
            if video_filters.is_empty() { None } else { Some(video_filters.join(",").as_str()) },
            if audio_filters.is_empty() { None } else { Some(audio_filters.join(",").as_str()) },
            output,
            export_preset,
        ).await
    }

    /// Prepare project for Premiere Pro editing
    pub async fn prepare_premiere_project<P: AsRef<Path>>(
        &self,
        media_files: Vec<P>,
        project_name: &str,
        generate_proxies: bool,
        detect_scenes: bool,
    ) -> EditronResult<PremiereProjectSetup> {
        let mut setup = PremiereProjectSetup {
            project_name: project_name.to_string(),
            media_files: vec![],
            proxies: vec![],
            scene_detection: None,
            scripts: vec![],
        };

        // Process each media file
        for file in media_files {
            let path = file.as_ref().to_path_buf();
            setup.media_files.push(path.clone());

            // Generate proxies if requested
            if generate_proxies {
                if let Ok(proxy) = self.generate_proxy(&path, ProxyPreset::HalfRes).await {
                    setup.proxies.push(proxy);
                }
            }
        }

        // Detect scenes if requested
        if detect_scenes && !setup.media_files.is_empty() {
            if let Ok(result) = self.detect_scenes(&setup.media_files[0], DetectionMethod::default()).await {
                setup.scene_detection = Some(result);
            }
        }

        // Generate Premiere Pro scripts
        if !setup.proxies.is_empty() {
            let proxy_script = self.proxy_manager.read().await
                .premiere_attach_proxies_script(&setup.proxies);
            setup.scripts.push(("attach_proxies.jsx".to_string(), proxy_script));
        }

        if let Some(ref scenes) = setup.scene_detection {
            let marker_script = self.export_premiere_markers(scenes);
            setup.scripts.push(("import_markers.jsx".to_string(), marker_script));
        }

        Ok(setup)
    }
}

/// Setup information for a Premiere Pro project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PremiereProjectSetup {
    pub project_name: String,
    pub media_files: Vec<PathBuf>,
    pub proxies: Vec<ProxyFile>,
    pub scene_detection: Option<SceneDetectionResult>,
    pub scripts: Vec<(String, String)>, // (filename, content)
    pub music_recommendations: Option<MusicRecommendation>,
}

impl EditronService {
    // ============ MUSIC SELECTION ============

    /// Get music recommendation for content type
    pub async fn recommend_music(
        &self,
        content_type: &str,
        duration: f64,
    ) -> MusicRecommendation {
        self.music_library.read().await
            .recommend_for_content(content_type, duration)
    }

    /// Analyze audio file for BPM, beats, etc.
    pub async fn analyze_music<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> EditronResult<AudioAnalysis> {
        self.music_library.read().await
            .analyze_audio(path).await
    }

    /// Register a downloaded music track
    pub async fn register_music_track(&self, track: MusicTrack) {
        self.music_library.write().await.register_track(track);
    }

    /// Search local music library
    pub async fn search_local_music(&self, criteria: &MusicSearchCriteria) -> Vec<MusicTrack> {
        self.music_library.read().await
            .search_local(criteria)
            .into_iter()
            .cloned()
            .collect()
    }

    /// Export license documentation
    pub async fn export_music_licenses(&self, project_name: &str) -> String {
        self.music_library.read().await
            .export_license_doc(project_name)
    }

    /// Get MotionArray search URL for criteria
    pub fn motionarray_search_url(&self, criteria: &MusicSearchCriteria) -> String {
        criteria.to_motionarray_url()
    }

    /// Suggest search terms based on content description
    pub fn suggest_music_terms(&self, description: &str) -> Vec<String> {
        music::suggest_search_terms(description)
    }

    // ============ EDIT ASSEMBLY ============

    /// Create a new edit assembly engine
    pub fn create_assembly_engine(&self) -> EditAssemblyEngine {
        EditAssemblyEngine::new()
    }

    /// Assemble an edit from footage and music
    pub async fn assemble_edit(
        &self,
        footage: Vec<FootageClip>,
        music_path: &Path,
        music_bpm: f32,
        config: AssemblyConfig,
    ) -> EditronResult<AssembledEdit> {
        // Get music duration
        let music_metadata = self.probe(music_path).await?;
        let music_duration = music_metadata.duration_seconds;

        // Create music analysis
        let music_analysis = EditAssemblyEngine::simple_music_analysis(
            music_path.to_path_buf(),
            music_duration,
            music_bpm,
        );

        // Create engine and add footage
        let mut engine = EditAssemblyEngine::new();
        engine.add_footage_batch(footage);
        engine.set_music(music_analysis);

        // Assemble the edit
        engine.assemble(&config)
    }

    /// Export assembled edit to native Premiere Pro .prproj file
    ///
    /// This is the most reliable export method. It modifies an existing .prproj
    /// by rearranging clips on V1 to match the assembled edit's timeline.
    /// The source .prproj must already have all clips imported in its bin.
    ///
    /// # Arguments
    /// * `source_prproj` - Path to the original .prproj with clips in the bin
    /// * `edit` - The assembled edit with timeline clip placements
    /// * `output_path` - Where to write the modified .prproj
    pub fn export_premiere_prproj(
        &self,
        source_prproj: &Path,
        edit: &AssembledEdit,
        output_path: &Path,
    ) -> EditronResult<PrprojRecutResult> {
        let mut engine = PrprojRecutEngine::load(source_prproj)?;

        // Convert AssembledEdit clips to PrprojClipEntries
        let edl: Vec<PrprojClipEntry> = edit.video_clips.iter().map(|clip| {
            let filename = clip.source.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            PrprojClipEntry {
                filename,
                source_in: clip.source_in,
                duration: clip.timeline_out - clip.timeline_in,
                label: Some(clip.label.clone().unwrap_or_default()),
            }
        }).collect();

        let mut result = engine.apply_edl(&edl)?;
        let file_size = engine.write(output_path)?;
        result.output_path = output_path.to_path_buf();
        result.file_size = file_size;

        Ok(result)
    }

    /// Export assembled edit to Premiere Pro XML
    pub fn export_premiere_xml(
        &self,
        edit: &AssembledEdit,
        output_path: &Path,
    ) -> EditronResult<PathBuf> {
        let exporter = PremiereXmlExporter::new(edit.frame_rate);
        exporter.export(edit, output_path)
            .map_err(|e| EditronError::Io(e))
    }

    /// Create footage clips from video files with metadata
    pub async fn create_footage_clips<P: AsRef<Path>>(
        &self,
        paths: Vec<P>,
    ) -> EditronResult<Vec<FootageClip>> {
        let mut clips = Vec::new();

        for path in paths {
            let path = path.as_ref();
            match self.probe(path).await {
                Ok(metadata) => {
                    let clip = FootageClip::new(path.to_path_buf(), &metadata);
                    clips.push(clip);
                }
                Err(e) => {
                    // Log error but continue with other files
                    eprintln!("Warning: Could not probe {}: {}", path.display(), e);
                }
            }
        }

        Ok(clips)
    }

    /// Full workflow: create edit and export to Premiere Pro
    pub async fn create_premiere_project(
        &self,
        project_name: &str,
        footage_paths: Vec<PathBuf>,
        music_path: &Path,
        music_bpm: f32,
        output_dir: &Path,
    ) -> EditronResult<PathBuf> {
        // Create footage clips with metadata
        let mut footage = self.create_footage_clips(footage_paths).await?;

        // Analyze footage for energy/motion (simplified - uses duration as proxy)
        for clip in &mut footage {
            // Estimate energy based on clip duration (shorter = higher energy content typically)
            let energy = if clip.duration < 5.0 { 0.8 }
                else if clip.duration < 15.0 { 0.6 }
                else { 0.4 };
            clip.energy_level = energy;
        }

        // Assembly configuration
        let config = AssemblyConfig {
            name: project_name.to_string(),
            target_duration: None, // Use music duration
            pacing: PacingStyle::Dynamic,
            sync_to_beats: true,
            sync_to_downbeats: true,
            allow_speed_ramping: false,
            max_speed: 1.5,
            min_clip_duration: 0.5,
            transition_style: TransitionStyle::Mixed,
            category_sequence: None,
            energy_matching: true,
        };

        // Assemble the edit
        let edit = self.assemble_edit(footage, music_path, music_bpm, config).await?;

        // Create output directory
        tokio::fs::create_dir_all(output_dir).await?;

        // Export to XML
        let xml_path = output_dir.join(format!("{}.xml", project_name));
        self.export_premiere_xml(&edit, &xml_path)?;

        // Also save the edit data as JSON for reference
        let json_path = output_dir.join(format!("{}_edit.json", project_name));
        let json = serde_json::to_string_pretty(&edit)
            .map_err(|e| EditronError::InvalidFormat(e.to_string()))?;
        tokio::fs::write(&json_path, json).await?;

        Ok(xml_path)
    }

    // ============ ARTLIST INTEGRATION ============

    /// Configure Artlist client with credentials
    pub async fn configure_artlist(&self, config: &ArtlistConfig) -> EditronResult<()> {
        if !config.is_configured() {
            // Disable Artlist if not configured
            let mut client_guard = self.artlist_client.write().await;
            *client_guard = None;
            return Ok(());
        }

        let client = ArtlistClient::from_config(config)?;

        // Verify credentials work
        client.verify_credentials().await?;

        let mut client_guard = self.artlist_client.write().await;
        *client_guard = Some(client);

        Ok(())
    }

    /// Check if Artlist is configured and available
    pub async fn artlist_available(&self) -> bool {
        self.artlist_client.read().await.is_some()
    }

    /// Search Artlist music library
    pub async fn search_artlist(
        &self,
        criteria: &MusicSearchCriteria,
        page: u32,
        per_page: u32,
    ) -> EditronResult<Vec<MusicTrack>> {
        let client_guard = self.artlist_client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| EditronError::Process("Artlist not configured".to_string()))?;

        let tracks = client.search_tracks(criteria, page, per_page).await?;
        Ok(tracks)
    }

    /// Get a specific track from Artlist
    pub async fn get_artlist_track(&self, track_id: &str) -> EditronResult<MusicTrack> {
        let client_guard = self.artlist_client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| EditronError::Process("Artlist not configured".to_string()))?;

        let track = client.get_track(track_id).await?;
        Ok(track)
    }

    /// Get download URL for an Artlist track
    pub async fn get_artlist_download_url(&self, track_id: &str) -> EditronResult<String> {
        let client_guard = self.artlist_client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| EditronError::Process("Artlist not configured".to_string()))?;

        let url = client.get_download_url(track_id).await?;
        Ok(url)
    }

    /// Download an Artlist track to local storage
    pub async fn download_artlist_track(
        &self,
        track_id: &str,
        filename: Option<&str>,
    ) -> EditronResult<PathBuf> {
        // Get the download URL
        let download_url = self.get_artlist_download_url(track_id).await?;

        // Get track info for default filename
        let track = self.get_artlist_track(track_id).await?;

        // Determine output path
        let music_dir = self.work_dir.join("music").join("artlist");
        tokio::fs::create_dir_all(&music_dir).await?;

        let safe_filename = filename
            .map(|f| f.to_string())
            .unwrap_or_else(|| {
                let safe_title = track.title.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                let safe_artist = track.artist.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_");
                format!("{} - {}.mp3", safe_artist, safe_title)
            });

        let output_path = music_dir.join(&safe_filename);

        // Download the file
        let client = reqwest::Client::new();
        let response = client
            .get(&download_url)
            .send()
            .await
            .map_err(|e| EditronError::Process(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(EditronError::Process(format!(
                "Download failed with status: {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| EditronError::Process(format!("Failed to read download: {}", e)))?;

        tokio::fs::write(&output_path, bytes).await?;

        // Register the track in the local library
        let mut local_track = track;
        local_track.local_path = Some(output_path.clone());
        local_track.license.download_date = Some(chrono::Utc::now());
        self.register_music_track(local_track).await;

        Ok(output_path)
    }

    /// Generate Artlist search URL for manual browsing
    pub fn artlist_search_url(&self, criteria: &MusicSearchCriteria) -> String {
        let mut params = vec![];

        // Add mood terms
        for mood in &criteria.moods {
            params.push(format!("moods={}", mood.artlist_term()));
        }

        // Add genre terms
        for genre in &criteria.genres {
            params.push(format!("genres={}", genre.artlist_term()));
        }

        // Add BPM range
        if let Some(min) = criteria.min_bpm {
            params.push(format!("bpm_from={}", min));
        }
        if let Some(max) = criteria.max_bpm {
            params.push(format!("bpm_to={}", max));
        }

        // Add duration
        if let Some(min) = criteria.min_duration {
            params.push(format!("duration_from={}", min as u32));
        }
        if let Some(max) = criteria.max_duration {
            params.push(format!("duration_to={}", max as u32));
        }

        // Instrumental filter
        if criteria.instrumental == Some(true) {
            params.push("vocals=no".to_string());
        }

        let query_string = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };

        format!("https://artlist.io/royalty-free-music{}", query_string)
    }

    /// Search across all configured music platforms
    pub async fn search_all_platforms(
        &self,
        criteria: &MusicSearchCriteria,
    ) -> EditronResult<Vec<MusicTrack>> {
        let mut all_tracks = Vec::new();

        // Search local library first
        let local_tracks = self.search_local_music(criteria).await;
        all_tracks.extend(local_tracks);

        // Search Artlist if available
        if self.artlist_available().await {
            match self.search_artlist(criteria, 1, 20).await {
                Ok(tracks) => all_tracks.extend(tracks),
                Err(e) => {
                    tracing::warn!("Artlist search failed: {}", e);
                }
            }
        }

        Ok(all_tracks)
    }

    // ============ VISUAL QC (SPECTRA) ============

    /// Extract candidate frames from a clip for visual QC analysis
    pub async fn extract_qc_frames(
        &self,
        clip_path: &Path,
        config: &VisualQcConfig,
    ) -> EditronResult<Vec<(f64, PathBuf)>> {
        self.visual_qc.extract_candidate_frames(clip_path, config).await
    }

    /// Read a frame JPEG and return base64 for vision API
    pub async fn get_frame_base64(&self, path: &Path) -> EditronResult<String> {
        VisualQcEngine::frame_to_base64(path).await
    }

    /// Apply visual QC results to a set of footage clips
    pub fn apply_visual_qc(&self, clips: &mut [FootageClip], qc_result: &VisualQcResult) {
        VisualQcEngine::apply_qc_to_clips(clips, qc_result);
    }

    /// Get a reference to the Visual QC engine
    pub fn visual_qc_engine(&self) -> &VisualQcEngine {
        &self.visual_qc
    }
}
