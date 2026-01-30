//! Ralph Wiggum Loop Execution Module
//!
//! Implements the Ralph Wiggum methodology for autonomous, iterative task execution.
//! Named after the Simpsons character for his persistent, undeterred approach.
//!
//! Key concepts:
//! - Loop execution until completion signal detected
//! - Backpressure validation between iterations
//! - Session persistence across iterations via spawn_follow_up
//! - Dual-gate completion detection (promise + exit signal)

pub mod backpressure;
pub mod completion;
pub mod orchestrator;
pub mod prompt;

pub use backpressure::{
    BackpressureConfig,
    BackpressureValidator,
    BackpressureValidationResult,
};
pub use completion::{CompletionConfig, CompletionDetector, CompletionStatus};
pub use orchestrator::{RalphOrchestrator, RalphConfig, RalphResult, RalphError};
pub use prompt::RalphPromptBuilder;
