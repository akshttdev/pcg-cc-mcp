//! Conference Workflow Engine
//!
//! Automated workflow orchestration for conference coverage, including:
//! - Deep multi-stage research with parallel execution
//! - Knowledge graph for entity reuse across conferences
//! - Quality Analyst agent for automated QA gates
//! - Parallel content + graphics workflows
//! - Social post scheduling
//!
//! # Status: Work In Progress
//!
//! This module has the architecture in place but execution logic is pending.
//! TODO(nora-v2): Wire research stages to actual LLM calls
//! TODO(nora-v2): Integrate with entity knowledge graph
//! TODO(nora-v2): Connect parallel orchestrator to execution engine

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

pub mod engine;
pub mod graphics;
pub mod parallel;
pub mod qa;
pub mod scrapers;
pub mod social;
pub mod stages;

pub use engine::{
    ConferenceWorkflowEngine, WorkflowConfig, WorkflowResult,
};
pub use graphics::{
    GraphicsComposer, ThumbnailComposition, CollectedAssets, ComposedThumbnail,
};
pub use parallel::ParallelOrchestrator;
pub use qa::{QualityAnalyst, QAResult, QADecision};
pub use social::SocialPostCreator;
pub use stages::{
    ResearchStage, ResearchStageResult, StageExecutor,
    conference_intel, speaker_research, brand_research,
    production_team, competitive_intel, side_events,
};
