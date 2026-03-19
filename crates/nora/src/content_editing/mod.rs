//! Content Editing Pipeline
//!
//! A 6-phase multi-agent pipeline for producing testimonial videos where
//! content is fully understood before any editing decisions are made.
//!
//! Pipeline: Nora (intake) → Sonix (transcribe) → Scout (research)
//!         → Editron (index) → Nora (directive) → Editron (assemble)
//!
//! # Status: Active
//!
//! Whisper transcription: Wired to local Whisper endpoint (API + CLI fallback)
//! Scout research: Dispatches to ExecutionEngine.research_content_entities()
//! FFmpeg assembly: Renders via shell script with eq=brightness transitions
//!
//! Remaining integration: Connect to render queue for async job processing

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod assembly;
pub mod directive;
pub mod engine;
pub mod media_catalog;
pub mod transcript;
pub mod types;

pub use engine::{ContentEditingConfig, ContentEditingEngine, ContentEditingResult};
pub use types::{
    ActAssignment,
    ActLabel,
    AssemblyResult,
    BRollClip,
    BeatPoint,
    ClientSpec,
    // Deep analysis types
    ClipSceneAnalysis,
    ClipVisualQc,
    ContentContextBrief,
    EditDirective,
    EnergyLevel,
    FrameAnalysis,
    LipSyncPoint,
    MediaAsset,
    MediaType,
    MusicBeatAnalysis,
    MusicBehavior,
    MusicEnergyPoint,
    MusicStructureSection,
    PipelineState,
    Resolution,
    SceneContentType,
    SceneSegment,
    SegmentType,
    SentenceBoundary,
    ShotCatalog,
    StoryArcBeat,
    StructuredTranscript,
    TranscriptSegment,
    VerifiedSoundbite,
    VisualCropRegion,
};
