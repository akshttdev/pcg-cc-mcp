//! Context management for agents
//!
//! Provides project-scoped context isolation for agent conversations.

mod project_scope;

pub use project_scope::{
    ProjectContextSummary, ProjectScopeBuilder, ProjectScopeError, ProjectScopedContext,
    TaskSummary, TokenUsageSummary,
};
