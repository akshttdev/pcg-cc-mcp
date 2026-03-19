//! Executive tools and capabilities for Nora

mod execute;
mod execute_impl;
mod parse;
mod schemas;
#[cfg(test)]
mod tests;
pub mod types;
mod user_scoped;

// Re-export everything from types so external code using `crate::tools::*` still works
use std::{collections::HashMap, sync::Arc};

use services::services::{
    agent_channels::{AgentChannelService, ChannelOwner},
    media_pipeline::MediaPipelineService,
};
pub use types::*;

use crate::{
    executor::TaskExecutor,
    integrations::{CalendarService, DiscordService, EmailService},
};

/// Executive tools available to Nora
pub struct ExecutiveTools {
    pub(super) available_tools: HashMap<String, ToolDefinition>,
    // External service integrations
    pub(super) email_service: Option<EmailService>,
    pub(super) discord_service: Option<DiscordService>,
    pub(super) calendar_service: Option<CalendarService>,
    // Agent communication channels (OAuth-based, DB-backed)
    pub(super) agent_channel_service: Option<Arc<AgentChannelService>>,
    /// The agent identity to use as sender for channel operations
    pub(super) agent_owner: Option<ChannelOwner>,
    // Task execution
    pub(super) task_executor: Option<Arc<TaskExecutor>>,
    pub(super) media_pipeline: Option<MediaPipelineService>,
    pub(super) workflow_orchestrator: Option<Arc<crate::workflow::WorkflowOrchestrator>>,
    // Unified execution engine (replaces workflow_orchestrator for new architecture)
    pub(super) execution_engine: Option<Arc<crate::execution::ExecutionEngine>>,
}

#[allow(dead_code)]
impl ExecutiveTools {
    pub fn new() -> Self {
        let mut tools = Self {
            available_tools: HashMap::new(),
            email_service: EmailService::from_env().ok(),
            discord_service: DiscordService::from_env().ok(),
            calendar_service: CalendarService::from_env().ok(),
            agent_channel_service: None,
            agent_owner: None,
            task_executor: None,
            media_pipeline: None,
            workflow_orchestrator: None,
            execution_engine: None,
        };

        tools.initialize_tools();
        tools
    }

    /// Set the task executor for project management operations
    pub fn set_task_executor(&mut self, executor: Arc<TaskExecutor>) {
        self.task_executor = Some(executor);
    }

    pub fn set_media_pipeline(&mut self, pipeline: MediaPipelineService) {
        self.media_pipeline = Some(pipeline);
    }

    pub fn set_workflow_orchestrator(
        &mut self,
        orchestrator: Arc<crate::workflow::WorkflowOrchestrator>,
    ) {
        self.workflow_orchestrator = Some(orchestrator);
    }

    /// Set the unified execution engine (new architecture)
    pub fn set_execution_engine(&mut self, engine: Arc<crate::execution::ExecutionEngine>) {
        self.execution_engine = Some(engine);
    }

    /// Wire the agent communication channel service.
    /// Call this from `NoraAgent::with_database` once the pool is available.
    pub fn set_agent_channels(&mut self, service: Arc<AgentChannelService>, owner: ChannelOwner) {
        self.agent_channel_service = Some(service);
        self.agent_owner = Some(owner);
    }

    pub fn get_available_tools(&self) -> Vec<&ToolDefinition> {
        self.available_tools.values().collect()
    }

    pub fn get_tools_by_category(&self, category: &ToolCategory) -> Vec<&ToolDefinition> {
        self.available_tools
            .values()
            .filter(|tool| &tool.category == category)
            .collect()
    }
}
