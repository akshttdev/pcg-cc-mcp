//! Unified Execution Engine
//!
//! Replaces the fragmented orchestration systems (WorkflowOrchestrator, GraphOrchestrator,
//! CoordinationManager, TaskExecutor) with a single coherent execution model.
//!
//! Architecture based on best practices from Manus AI and Google Antigravity:
//! - Router-Executor-Observer loop (Manus: analyze → plan → execute → observe)
//! - Artifact-based communication (Antigravity: plans, outputs, diffs)
//! - Event broadcast to all UI surfaces (Mission Control, Task Board, Chat)

mod artifact;
mod engine;
mod events;
mod research;
mod router;

pub use artifact::{Artifact, ArtifactStore, ArtifactType};
pub use engine::{ExecutionEngine, ExecutionRequest, ExecutionResult, ExecutionStatus, TaskCreator};
pub use events::{ExecutionEvent, EventBroadcaster};
pub use research::{ResearchContext, ResearchExecutor};
pub use router::{AgentMatch, ExecutionRouter};
