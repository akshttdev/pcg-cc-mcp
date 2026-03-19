//! Core data types for the content editing pipeline.
//!
//! Defines structures for every phase: intake, transcription, research,
//! shot cataloging, directive synthesis, and assembly.

use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ---------------------------------------------------------------------------
// Phase 1 – Client Spec (parsed from RTF / brief)
// ---------------------------------------------------------------------------

/// Parsed client specification for a testimonial video project.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClientSpec {
    pub project_name: String,
    pub max_duration_seconds: u32,
    pub resolution: Resolution,
    pub aspect_ratio: String,
    pub style: String,
    pub broll_coverage_min: f64,
    pub broll_coverage_max: f64,
    pub interview_on_camera_min: f64,
    pub interview_on_camera_max: f64,
    pub music_behavior: MusicBehavior,
    pub source_path: PathBuf,
    pub output_path: Option<PathBuf>,
    pub notes: Vec<String>,
    /// Creative tone direction (e.g. "Professional but not stiff, Confident, Fun not salesy")
    pub tone: Option<String>,
    /// Target audience segments (e.g. ["Event organizers", "Conference producers"])
    pub target_audience: Vec<String>,
    /// URL of reference video for pacing/style
    pub reference_video_url: Option<String>,
    /// Suggested story arc beats from the client brief
    pub story_arc: Vec<StoryArcBeat>,
    /// Key content themes to emphasize (e.g. ["scale", "credibility", "trust"])
    pub content_themes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MusicBehavior {
    /// Volume in dB when dialogue is present (e.g. -18)
    pub ducked_db: f64,
    /// Volume in dB during montage / no-dialogue sections (e.g. 0 or -6)
    pub full_db: f64,
    /// Whether to fade in at the start
    pub fade_in: bool,
    /// Whether to fade out at the end
    pub fade_out: bool,
}

/// A beat in the client's suggested story arc.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StoryArcBeat {
    pub order: u32,
    pub description: String,
    /// Content type hint: "soundbite", "montage", "transition"
    pub beat_type: String,
}

// ---------------------------------------------------------------------------
// Phase 2 – Transcript
// ---------------------------------------------------------------------------

/// Type of segment identified in the transcript.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SegmentType {
    PreInterview,
    Testimonial,
    Transition,
    Unusable,
}

/// A single time-stamped segment of the cleaned transcript.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub speaker: String,
    pub text: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub segment_type: SegmentType,
    /// Word-level timestamps when available
    pub words: Vec<WordTiming>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WordTiming {
    pub word: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

/// A detected sentence boundary within the transcript.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SentenceBoundary {
    /// Full sentence text
    pub text: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    /// Index of the first word in this sentence (within the segment's words)
    pub start_word_idx: usize,
    /// Index of the last word (inclusive)
    pub end_word_idx: usize,
    /// Which transcript segment this sentence belongs to
    pub segment_index: usize,
}

/// Full structured transcript produced by Phase 2.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct StructuredTranscript {
    pub source_file: String,
    pub total_duration_seconds: f64,
    pub segments: Vec<TranscriptSegment>,
    pub speakers: Vec<String>,
    /// Index of first testimonial segment (banter/testimonial boundary)
    pub testimonial_start_index: Option<usize>,
    /// Sentence boundaries detected from word-level timestamps
    pub sentences: Vec<SentenceBoundary>,
}

// ---------------------------------------------------------------------------
// Phase 3 – Content Context Brief (research output)
// ---------------------------------------------------------------------------

/// Contextual research brief produced by Scout.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentContextBrief {
    pub event: Option<EventContext>,
    pub subject: Option<SubjectProfile>,
    pub venue: Option<VenueContext>,
    pub company: Option<CompanyContext>,
    pub key_themes: Vec<String>,
    pub raw_entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EventContext {
    pub name: String,
    pub description: String,
    pub industry: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SubjectProfile {
    pub name: String,
    pub title: Option<String>,
    pub background: String,
    pub relevant_experience: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VenueContext {
    pub name: String,
    pub location: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CompanyContext {
    pub name: String,
    pub description: String,
    pub founder: Option<String>,
    pub location: Option<String>,
}

// ---------------------------------------------------------------------------
// Phase 4 – Media Catalog & Verified Soundbites
// ---------------------------------------------------------------------------

/// Classification of a media asset by content type.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum MediaType {
    Interview,
    Drone,
    Cinematic,
    BTS,
    VerticalHighlight,
    /// Action/event footage (ARv clips, keynote, crowd)
    EventAction,
    Music,
    Reference,
    Unknown,
}

/// Energy level of a B-roll clip, used for content-aware assignment.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, PartialOrd)]
#[serde(rename_all = "camelCase")]
pub enum EnergyLevel {
    Low,
    Medium,
    High,
}

// ---------------------------------------------------------------------------
// Deep Analysis Types (populated by Phase 4 analysis engines)
// ---------------------------------------------------------------------------

/// Content type classification for a video segment (from SceneAnalysisEngine).
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SceneContentType {
    HighEnergy,
    Establishing,
    Intimate,
    Transition,
    Ambient,
}

/// A single analyzed segment within a clip.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SceneSegment {
    pub timestamp: f64,
    pub duration: f64,
    pub brightness: f64,
    pub motion_intensity: f64,
    pub complexity: f64,
    pub energy_score: f64,
    pub content_type: SceneContentType,
}

/// Complete scene analysis for one clip.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClipSceneAnalysis {
    pub segments: Vec<SceneSegment>,
    pub overall_energy: f64,
    pub peak_energy_timestamp: f64,
    pub dominant_content_type: SceneContentType,
    pub usable: bool,
}

/// A single analyzed frame with composition scores (from VisualQcEngine).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FrameAnalysis {
    pub timestamp: f64,
    pub composition_score: f64,
    pub subject_score: f64,
    pub thirds_score: f64,
    pub headroom_score: f64,
    pub exposure_score: f64,
    pub sharpness_score: f64,
    pub subject_label: Option<String>,
    pub notes: String,
}

/// Suggested crop region for reframing a clip.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VisualCropRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rationale: String,
}

/// Visual QC result for one clip.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ClipVisualQc {
    pub best_in_point: f64,
    pub best_composition_score: f64,
    pub qc_passed: bool,
    pub recommended_crop: Option<VisualCropRegion>,
    pub summary: String,
    pub analyzed_frames: Vec<FrameAnalysis>,
}

/// A single beat point on the music grid (from BeatAnalysisEngine).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BeatPoint {
    pub timestamp: f64,
    pub beat_number: u32,
    pub bar_number: u32,
    pub beat_in_bar: u32,
    pub is_downbeat: bool,
    pub energy_at_beat: f64,
    pub is_strong_cut_point: bool,
}

/// A structural section of a music track.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MusicStructureSection {
    pub name: String,
    pub start: f64,
    pub end: f64,
    pub energy_level: f64,
    pub suggested_content: String,
}

/// A single energy measurement at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MusicEnergyPoint {
    pub timestamp: f64,
    pub normalized_energy: f64,
}

/// Complete beat/music analysis for one track.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MusicBeatAnalysis {
    pub bpm: f64,
    pub beat_interval: f64,
    pub total_beats: u32,
    pub beats_per_bar: u32,
    pub beats: Vec<BeatPoint>,
    pub sections: Vec<MusicStructureSection>,
    pub energy_curve: Vec<MusicEnergyPoint>,
}

/// A single cataloged media file with technical metadata.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub filename: String,
    pub path: PathBuf,
    pub duration_seconds: f64,
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub media_type: MediaType,
    /// Camera designation parsed from filename, e.g. "CAM_A"
    pub camera: Option<String>,
    /// File size in bytes
    pub file_size_bytes: u64,
    /// Parent folder name for classification hints
    pub parent_folder: String,
    /// Energy level for content-aware B-roll assignment
    pub energy_level: EnergyLevel,
    /// Content tags for semantic matching (e.g. ["crowd", "stage", "LED screens"])
    pub content_tags: Vec<String>,
    /// Frames per second
    pub fps: f64,
    // --- Analysis results (populated by Phase 4 deep analysis) ---
    /// Scene analysis from SceneAnalysisEngine
    pub scene_analysis: Option<ClipSceneAnalysis>,
    /// Visual QC from VisualQcEngine
    pub visual_qc: Option<ClipVisualQc>,
    /// Beat analysis from BeatAnalysisEngine (for music tracks)
    pub beat_analysis: Option<MusicBeatAnalysis>,
}

/// The full shot catalog produced by Phase 4.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ShotCatalog {
    pub assets: Vec<MediaAsset>,
    pub total_duration_seconds: f64,
    pub interview_assets: Vec<usize>,
    pub broll_assets: Vec<usize>,
    pub music_assets: Vec<usize>,
    /// Beat analysis for the selected music track
    pub music_beat_grid: Option<MusicBeatAnalysis>,
}

/// A soundbite verified against the actual transcript.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedSoundbite {
    pub id: String,
    pub text: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub duration_seconds: f64,
    pub speaker: String,
    pub act: ActLabel,
    pub emotional_beat: bool,
    /// Levenshtein match ratio against transcript (0.0–1.0)
    pub match_confidence: f64,
    /// Whether this soundbite starts at a sentence boundary
    pub starts_at_sentence: bool,
    /// Whether this soundbite ends at a sentence boundary
    pub ends_at_sentence: bool,
    /// LLM-assigned quality score (0.0–1.0) based on content relevance,
    /// emotional impact, and narrative value
    pub quality_score: f64,
    /// Content themes this soundbite addresses (e.g. ["scale", "trust"])
    pub themes: Vec<String>,
}

/// A verified point where on-camera video must be lip-synced with audio.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LipSyncPoint {
    /// Soundbite ID this sync point belongs to
    pub soundbite_id: String,
    /// Source timecode in the interview file where audio starts
    pub audio_source_timecode: f64,
    /// Source timecode in the interview file where video should start
    /// MUST equal audio_source_timecode for correct lip sync
    pub video_source_timecode: f64,
    /// Duration of the on-camera segment
    pub duration_seconds: f64,
    /// Position on the output timeline
    pub timeline_position_seconds: f64,
}

/// 5-act narrative structure for testimonial videos.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ActLabel {
    /// Opening – establish context
    Act1Intro,
    /// Challenge – what problem existed
    Act2Challenge,
    /// Solution – how the company solved it
    Act3Solution,
    /// Results – measurable outcomes
    Act4Results,
    /// Close – final endorsement
    Act5Close,
}

// ---------------------------------------------------------------------------
// Phase 5 – Edit Directive
// ---------------------------------------------------------------------------

/// Complete edit instructions synthesized from all prior artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct EditDirective {
    pub project_name: String,
    pub total_duration_seconds: f64,
    pub soundbites: Vec<VerifiedSoundbite>,
    pub acts: Vec<ActAssignment>,
    pub music_track: Option<MusicTrackDirective>,
    pub computed_broll_ratio: f64,
    pub computed_interview_ratio: f64,
    pub vertical_clip_treatment: VerticalTreatment,
    pub output_spec: OutputSpec,
    /// Verified lip sync points for on-camera clips
    pub lip_sync_points: Vec<LipSyncPoint>,
    /// Transition duration in seconds between clips (default 0.5)
    pub transition_duration_seconds: f64,
}

/// Per-act assignment of soundbites and B-roll.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ActAssignment {
    pub act: ActLabel,
    pub duration_target_seconds: f64,
    pub soundbite_ids: Vec<String>,
    pub broll_clips: Vec<BRollClip>,
    /// Whether the interview subject appears on camera in this act
    pub interview_on_camera: bool,
}

/// A B-roll clip reference with in/out points.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct BRollClip {
    pub asset_index: usize,
    pub filename: String,
    pub in_point_seconds: f64,
    pub out_point_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MusicTrackDirective {
    pub asset_index: usize,
    pub filename: String,
    pub ducked_db: f64,
    pub full_db: f64,
    /// Regions where music should be at full volume (no dialogue)
    pub full_regions: Vec<TimeRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TimeRegion {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VerticalTreatment {
    /// Blur radius for pillarbox fill
    pub blur_radius: u32,
    /// Scale factor for the foreground (centered video)
    pub fg_scale_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct OutputSpec {
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub bitrate_mbps: u32,
}

// ---------------------------------------------------------------------------
// Phase 6 – Assembly result
// ---------------------------------------------------------------------------

/// Result of the assembly phase.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyResult {
    pub rendered_video_path: Option<PathBuf>,
    pub premiere_xml_path: Option<PathBuf>,
    pub duration_seconds: f64,
    pub file_size_bytes: u64,
}

// ---------------------------------------------------------------------------
// Pipeline state – flows through the engine
// ---------------------------------------------------------------------------

/// Accumulated state of the content editing pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub client_spec: Option<ClientSpec>,
    pub transcript: Option<StructuredTranscript>,
    pub context_brief: Option<ContentContextBrief>,
    pub shot_catalog: Option<ShotCatalog>,
    pub verified_soundbites: Vec<VerifiedSoundbite>,
    pub directive: Option<EditDirective>,
    pub assembly_result: Option<AssemblyResult>,
    pub lip_sync_points: Vec<LipSyncPoint>,
    pub errors: Vec<String>,
    pub phase_artifacts: HashMap<String, serde_json::Value>,
}

impl PipelineState {
    pub fn new() -> Self {
        Self {
            client_spec: None,
            transcript: None,
            context_brief: None,
            shot_catalog: None,
            verified_soundbites: Vec::new(),
            directive: None,
            assembly_result: None,
            lip_sync_points: Vec::new(),
            errors: Vec::new(),
            phase_artifacts: HashMap::new(),
        }
    }
}
