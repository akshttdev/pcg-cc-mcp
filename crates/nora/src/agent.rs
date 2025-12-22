//! Core Nora agent implementation

use std::{
    fs,
    io,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    brain::LLMClient,
    coordination::CoordinationManager,
    executor::{TaskDefinition, TaskExecutor},
    memory::{
        BudgetStatus, ConversationMemory, ExecutiveContext, ExecutivePriority, Milestone,
        MilestoneStatus, PriorityImpact, PriorityStatus, PriorityUrgency, ProjectContext,
        ProjectStatus,
    },
    personality::BritishPersonality,
    tools::ExecutiveTools,
    voice::VoiceEngine,
    NoraConfig, NoraError, Result,
};
use db::models::{project::{CreateProject, Project}, task::Priority};
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::SqlitePool;

/// Main Nora agent structure
pub struct NoraAgent {
    pub id: Uuid,
    pub config: NoraConfig,
    pub voice_engine: Arc<VoiceEngine>,
    pub coordination_manager: Arc<CoordinationManager>,
    pub memory: Arc<RwLock<ConversationMemory>>,
    pub personality: BritishPersonality,
    pub executive_tools: ExecutiveTools,
    pub context: Arc<RwLock<ExecutiveContext>>,
    pub llm: Option<Arc<LLMClient>>,
    pub is_active: Arc<RwLock<bool>>,
    pub pool: Option<SqlitePool>,
    pub executor: Option<Arc<TaskExecutor>>,
}

/// Request to Nora for processing
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraRequest {
    pub request_id: String,
    pub session_id: String,
    pub request_type: NoraRequestType,
    pub content: String,
    pub context: Option<serde_json::Value>,
    pub voice_enabled: bool,
    pub priority: RequestPriority,
    pub timestamp: DateTime<Utc>,
}

/// Types of requests Nora can handle
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum NoraRequestType {
    /// Voice interaction (speech-to-text processed)
    VoiceInteraction,
    /// Text-based interaction
    TextInteraction,
    /// Executive task coordination
    TaskCoordination,
    /// Strategic planning request
    StrategyPlanning,
    /// Performance analysis
    PerformanceAnalysis,
    /// Communication management
    CommunicationManagement,
    /// Decision support
    DecisionSupport,
    /// Proactive notification/alert
    ProactiveNotification,
}

/// Request priority levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RequestPriority {
    Low,
    Normal,
    High,
    Urgent,
    Executive,
}

/// Nora's response structure
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct NoraResponse {
    pub response_id: String,
    pub request_id: String,
    pub session_id: String,
    pub response_type: NoraResponseType,
    pub content: String,
    pub actions: Vec<ExecutiveAction>,
    pub voice_response: Option<String>, // Base64 encoded audio
    pub follow_up_suggestions: Vec<String>,
    pub context_updates: Vec<ContextUpdate>,
    pub timestamp: DateTime<Utc>,
    pub processing_time_ms: u64,
}

/// Types of responses from Nora
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum NoraResponseType {
    DirectResponse,
    TaskDelegation,
    StrategyRecommendation,
    PerformanceInsight,
    DecisionSupport,
    CoordinationAction,
    ProactiveAlert,
}

/// Executive actions Nora can perform
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExecutiveAction {
    pub action_id: String,
    pub action_type: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub requires_approval: bool,
    pub estimated_duration: Option<String>,
    pub assigned_to: Option<String>,
}

/// Context updates from Nora's analysis
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContextUpdate {
    pub update_type: String,
    pub key: String,
    pub value: serde_json::Value,
    pub confidence: f32,
    pub source: String,
}

impl NoraAgent {
    /// Create a new Nora agent instance
    pub async fn new(config: NoraConfig) -> Result<Self> {
        let id = Uuid::new_v4();

        tracing::info!("Initializing Nora agent with ID: {}", id);

        // Initialize voice engine
        let voice_engine = Arc::new(VoiceEngine::new(config.voice.clone()).await?);

        // Initialize coordination manager
        let coordination_manager = Arc::new(CoordinationManager::new().await?);

        // Initialize memory
        let memory = Arc::new(RwLock::new(ConversationMemory::new()));

        // Initialize personality
        let personality = BritishPersonality::new(config.personality.clone());

        // Initialize executive tools
        let executive_tools = ExecutiveTools::new();

        // Initialize executive context
        let context = Arc::new(RwLock::new(ExecutiveContext::new()));
        {
            let mut ctx = context.write().await;
            if ctx.active_projects.is_empty() {
                ctx.active_projects = Self::default_projects();
            }
            if ctx.current_priorities.is_empty() {
                ctx.current_priorities = Self::default_priorities();
            }
        }

        let llm = config
            .llm
            .clone()
            .map(LLMClient::new)
            .and_then(|client| {
                if client.is_ready() {
                    Some(Arc::new(client))
                } else {
                    tracing::warn!("LLM configuration detected but OPENAI_API_KEY is missing; falling back to deterministic responses");
                    None
                }
            });

        // Start as active
        let is_active = Arc::new(RwLock::new(true));

        Ok(Self {
            id,
            config,
            voice_engine,
            coordination_manager,
            memory,
            personality,
            executive_tools,
            context,
            is_active,
            llm,
            pool: None,
            executor: None,
        })
    }

    /// Set the database pool and initialize the task executor
    pub fn with_database(mut self, pool: SqlitePool) -> Self {
        let executor = TaskExecutor::new(pool.clone());
        self.pool = Some(pool);
        self.executor = Some(Arc::new(executor));
        self
    }

    /// Process a request from user or system
    pub async fn process_request(&self, request: NoraRequest) -> Result<NoraResponse> {
        let start_time = std::time::Instant::now();

        tracing::info!(
            "Processing request {} of type {:?}",
            request.request_id,
            request.request_type
        );

        // Update executive context
        self.update_context_from_request(&request).await?;

        // Process based on request type
        let (response_content, actions, response_type) = match request.request_type {
            NoraRequestType::VoiceInteraction => (
                self.process_voice_interaction(&request).await?,
                Vec::new(),
                NoraResponseType::DirectResponse,
            ),
            NoraRequestType::TextInteraction => (
                self.process_text_interaction(&request).await?,
                Vec::new(),
                NoraResponseType::DirectResponse,
            ),
            NoraRequestType::TaskCoordination => {
                let (content, actions) = self.process_task_coordination(&request).await?;
                (content, actions, NoraResponseType::CoordinationAction)
            }
            NoraRequestType::StrategyPlanning => {
                let (content, actions) = self.process_strategy_planning(&request).await?;
                (content, actions, NoraResponseType::StrategyRecommendation)
            }
            NoraRequestType::PerformanceAnalysis => (
                self.process_performance_analysis(&request).await?,
                Vec::new(),
                NoraResponseType::PerformanceInsight,
            ),
            NoraRequestType::CommunicationManagement => {
                let (content, actions) =
                    self.process_communication_management(&request).await?;
                (content, actions, NoraResponseType::DirectResponse)
            }
            NoraRequestType::DecisionSupport => (
                self.process_decision_support(&request).await?,
                Vec::new(),
                NoraResponseType::DecisionSupport,
            ),
            NoraRequestType::ProactiveNotification => (
                self.process_proactive_notification(&request).await?,
                Vec::new(),
                NoraResponseType::ProactiveAlert,
            ),
        };

        // Personality layer disabled - causes repetitive broken phrases
        // response_content = self.personality.apply_personality_to_response(&response_content, &request);

        // Generate voice response if enabled
        let voice_response = if request.voice_enabled {
            Some(self.generate_voice_response(&response_content).await?)
        } else {
            None
        };

        // Generate follow-up suggestions
        let follow_up_suggestions = self
            .generate_follow_up_suggestions(&request, &response_content)
            .await?;

        // Update conversation memory
        self.update_conversation_memory(&request, &response_content)
            .await?;

        let processing_time = start_time.elapsed().as_millis() as u64;

        // Extract context updates before consuming response_content
        let context_updates = self
            .extract_context_updates(&request, &response_content)
            .await;

        Ok(NoraResponse {
            response_id: Uuid::new_v4().to_string(),
            request_id: request.request_id.clone(),
            session_id: request.session_id.clone(),
            response_type,
            content: response_content,
            actions,
            voice_response,
            follow_up_suggestions,
            context_updates,
            timestamp: Utc::now(),
            processing_time_ms: processing_time,
        })
    }

    /// Check if Nora is currently active
    pub async fn is_active(&self) -> bool {
        *self.is_active.read().await
    }

    /// Activate or deactivate Nora
    pub async fn set_active(&self, active: bool) -> Result<()> {
        *self.is_active.write().await = active;

        if active {
            tracing::info!("Nora activated");
        } else {
            tracing::info!("Nora deactivated");
        }

        Ok(())
    }

    /// Get LLM cache statistics
    pub fn get_cache_stats(&self) -> Option<crate::cache::CacheStats> {
        self.llm.as_ref().map(|llm| llm.get_cache_stats())
    }

    // Private helper methods

    async fn update_context_from_request(&self, request: &NoraRequest) -> Result<()> {
        let mut context = self.context.write().await;
        context.update_from_request(request).await?;
        Ok(())
    }

    async fn process_voice_interaction(&self, request: &NoraRequest) -> Result<String> {
        let original = request.content.trim();
        let lowered = original.to_lowercase();

        let response = if lowered.contains("hello") || lowered.contains("hi") {
            "Hello! Lovely to hear your voice. How can I help you today?".to_string()
        } else if lowered.contains("project") || lowered.contains("roadmap") {
            self.describe_roadmap().await
        } else if lowered.contains("capabilities") {
            "Great question! I'm your executive assistant - I handle strategic planning, team coordination, and performance analysis. I'm brilliant at multi-agent coordination and decision support. What would you like to know more about?".to_string()
        } else {
            self.generate_llm_response(request, original).await
        };

        Ok(response)
    }

    async fn process_text_interaction(&self, request: &NoraRequest) -> Result<String> {
        let original = request.content.trim();
        let lowered = original.to_lowercase();

        // Check if user is confirming a pending action
        if self.is_confirmation(&lowered).await {
            if let Some(response) = self.handle_confirmation().await? {
                return Ok(response);
            }
        }

        // Only use simple pattern matching for very short messages (greetings)
        // For anything longer or more complex, use the LLM
        let is_short_message = original.split_whitespace().count() <= 5;

        let response = if is_short_message && (lowered.starts_with("hello")
            || lowered.starts_with("hi ")
            || lowered.starts_with("hi,")
            || lowered == "hi"
            || lowered.starts_with("good morning")
            || lowered.starts_with("good afternoon")
            || lowered.starts_with("good evening"))
        {
            "Hello there! Lovely to meet you. How can I help today?".to_string()
        } else if is_short_message && lowered.contains("thank") {
            "You're very welcome! Always happy to help. Just give me a shout when you need anything.".to_string()
        } else {
            // Use LLM for all other requests (including complex questions about projects)
            let llm_response = self.generate_llm_response(request, original).await;

            // After LLM response, check if we should execute tasks
            self.extract_and_execute_tasks(original, &llm_response).await?;

            llm_response
        };

        Ok(response)
    }

    async fn process_task_coordination(
        &self,
        _request: &NoraRequest,
    ) -> Result<(String, Vec<ExecutiveAction>)> {
        let ctx = self.context.read().await;

        let mut response_parts =
            vec!["I've reviewed our current task landscape and project portfolio.".to_string()];

        // Analyze active projects
        let active_count = ctx
            .active_projects
            .iter()
            .filter(|p| matches!(p.status, ProjectStatus::InProgress))
            .count();

        if active_count > 0 {
            response_parts.push(format!(
                "We have {} active projects requiring coordination.",
                active_count
            ));
        }

        // Check priorities
        let high_priority_count = ctx
            .current_priorities
            .iter()
            .filter(|p| matches!(p.urgency, PriorityUrgency::High | PriorityUrgency::Critical))
            .count();

        if high_priority_count > 0 {
            response_parts.push(format!(
                "{} high-priority items need immediate attention.",
                high_priority_count
            ));
        }

        response_parts
            .push("Would you like me to deep-dive into any specific initiative?".to_string());

        let response = response_parts.join(" ");
        let actions = vec![];

        Ok((response, actions))
    }

    async fn process_strategy_planning(
        &self,
        request: &NoraRequest,
    ) -> Result<(String, Vec<ExecutiveAction>)> {
        // Use LLM for strategic planning if available
        if self.llm.is_some() {
            let strategic_prompt = format!(
                "Provide strategic planning analysis and recommendations for: {}",
                request.content
            );

            let response = self.generate_llm_response(request, &strategic_prompt).await;

            // Generate strategic actions
            let actions = vec![ExecutiveAction {
                action_id: Uuid::new_v4().to_string(),
                action_type: "StrategicReview".to_string(),
                description: "Schedule strategic review session with key stakeholders".to_string(),
                parameters: serde_json::json!({
                    "duration": "2 hours",
                    "participants": ["Executive Team", "Project Leads"]
                }),
                requires_approval: true,
                estimated_duration: Some("2 hours".to_string()),
                assigned_to: Some("Strategy Team".to_string()),
            }];

            Ok((response, actions))
        } else {
            let response = "Strategic analysis in progress. I recommend scheduling a comprehensive review session to align on priorities and resource allocation.".to_string();
            Ok((response, vec![]))
        }
    }

    async fn process_performance_analysis(&self, request: &NoraRequest) -> Result<String> {
        let ctx = self.context.read().await;

        let mut insights = vec!["**Performance Analysis Summary**".to_string()];

        // Analyze project progress
        let total_projects = ctx.active_projects.len();
        if total_projects > 0 {
            let avg_progress: f64 = ctx
                .active_projects
                .iter()
                .map(|p| p.progress_percentage as f64)
                .sum::<f64>()
                / total_projects as f64;

            insights.push(format!(
                "Portfolio Progress: {:.1}% average across {} projects",
                avg_progress, total_projects
            ));

            // Identify at-risk projects
            let at_risk: Vec<_> = ctx
                .active_projects
                .iter()
                .filter(|p| matches!(p.status, ProjectStatus::AtRisk))
                .collect();

            if !at_risk.is_empty() {
                insights.push(format!(
                    "⚠️  {} projects flagged as at-risk requiring intervention",
                    at_risk.len()
                ));
            }

            // Budget analysis
            let total_allocated: f64 = ctx
                .active_projects
                .iter()
                .map(|p| p.budget_status.allocated)
                .sum();
            let total_spent: f64 = ctx
                .active_projects
                .iter()
                .map(|p| p.budget_status.spent)
                .sum();
            let utilization = if total_allocated > 0.0 {
                (total_spent / total_allocated) * 100.0
            } else {
                0.0
            };

            insights.push(format!(
                "Budget Utilization: {:.1}% (£{:.0}K spent of £{:.0}K allocated)",
                utilization,
                total_spent / 1000.0,
                total_allocated / 1000.0
            ));
        }

        // Use LLM for deeper analysis if available
        if let Some(llm) = &self.llm {
            let context_snapshot = self.build_context_snapshot(request).await;
            if let Ok(llm_insights) = llm.generate(
                "Provide executive-level performance insights and recommendations based on the portfolio data.",
                &request.content,
                &context_snapshot
            ).await {
                insights.push(String::new()); // blank line
                insights.push(llm_insights);
            }
        }

        Ok(insights.join("\n"))
    }

    async fn process_communication_management(
        &self,
        _request: &NoraRequest,
    ) -> Result<(String, Vec<ExecutiveAction>)> {
        // Process communication management
        let response = "Communication management processed.".to_string();
        let actions = vec![];
        Ok((response, actions))
    }

    async fn process_decision_support(&self, _request: &NoraRequest) -> Result<String> {
        // Process decision support requests
        Ok("Decision support analysis completed.".to_string())
    }

    async fn process_proactive_notification(&self, _request: &NoraRequest) -> Result<String> {
        // Process proactive notifications
        Ok("Proactive notification processed.".to_string())
    }

    async fn generate_voice_response(&self, content: &str) -> Result<String> {
        self.voice_engine
            .synthesize_speech(content)
            .await
            .map_err(NoraError::VoiceEngineError)
    }

    async fn generate_follow_up_suggestions(
        &self,
        _request: &NoraRequest,
        _response: &str,
    ) -> Result<Vec<String>> {
        // Suggestions disabled per user request
        Ok(vec![])
    }

    async fn update_conversation_memory(
        &self,
        request: &NoraRequest,
        response: &str,
    ) -> Result<()> {
        let mut memory = self.memory.write().await;
        memory.add_interaction(request, response).await?;
        Ok(())
    }

    pub async fn seed_projects(&self, projects: Vec<ProjectContext>) -> Result<()> {
        let mut context = self.context.write().await;
        context.active_projects = projects;
        context.last_updated = Utc::now();
        Ok(())
    }

    async fn describe_roadmap(&self) -> String {
        let ctx = self.context.read().await;
        if ctx.active_projects.is_empty() {
            return "I don’t have any projects recorded on the roadmap yet. Would you like me to register them?".to_string();
        }

        let mut lines = Vec::new();
        lines.push("Here's the current roadmap:".to_string());
        for project in &ctx.active_projects {
            lines.push(format!(
                "• {} – {} ({}% complete)",
                project.name,
                Self::humanise_status(&project.status),
                project.progress_percentage.round()
            ));
        }
        lines.push("Would you like a deeper dive into any particular initiative?".to_string());
        lines.join("\n")
    }

    async fn build_context_snapshot(&self, request: &NoraRequest) -> String {
        let mut sections = Vec::new();

        // Try to fetch real-time data from database
        if let Some(executor) = &self.executor {
            tracing::info!("Executor available, building context for: {}", request.content);
            // Check if a specific project is mentioned
            if let Some(project_name) = self.extract_project_name(&request.content) {
                tracing::info!("Extracted project name: {}", project_name);
                // Fetch specific project data
                match executor.find_project_by_name(&project_name).await {
                    Ok(project_id) => {
                        match executor.get_project_details(project_id).await {
                            Ok(details) => {
                                sections.push(format!("**LIVE DATA FOR {} PROJECT:**", details.name.to_uppercase()));
                                sections.push(format!("Repository: {}", details.git_repo_path));

                                // Get project stats
                                if let Ok(stats) = executor.get_project_stats(project_id).await {
                                    sections.push(format!(
                                        "Tasks: {} total ({} completed, {} in progress, {} blocked)",
                                        stats.total_tasks, stats.completed_tasks,
                                        stats.in_progress_tasks, stats.blocked_tasks
                                    ));
                                }

                                // List all tasks
                                if !details.tasks.is_empty() {
                                    sections.push(format!("\nCurrent tasks ({}):", details.tasks.len()));
                                    for (i, task) in details.tasks.iter().take(20).enumerate() {
                                        sections.push(format!(
                                            "  {}. [{}] {} - {}",
                                            i + 1,
                                            task.status,
                                            task.title,
                                            task.description.as_deref().unwrap_or("No description")
                                        ));
                                    }
                                } else {
                                    sections.push("No tasks found in database for this project.".to_string());
                                }

                                // List boards
                                if !details.boards.is_empty() {
                                    let board_names: Vec<String> = details.boards.iter().map(|b| b.name.clone()).collect();
                                    sections.push(format!("\nBoards: {}", board_names.join(", ")));
                                }

                                // List pods
                                if !details.pods.is_empty() {
                                    let pod_names: Vec<String> = details.pods.iter().map(|p| p.name.clone()).collect();
                                    sections.push(format!("Pods: {}", pod_names.join(", ")));
                                }

                                sections.push("".to_string()); // Empty line separator
                            }
                            Err(e) => {
                                tracing::warn!("Failed to fetch project details: {}", e);
                            }
                        }
                    }
                    Err(_) => {
                        // Project not found in database, continue with static context
                    }
                }
            } else if request.content.to_lowercase().contains("task") {
                // No specific project, but asking about tasks - show all tasks across all projects
                tracing::info!("No specific project found, but request contains 'task' - fetching all tasks");
                match executor.get_all_tasks().await {
                    Ok(tasks) => {
                        if !tasks.is_empty() {
                            sections.push("**LIVE DATA - ALL TASKS ACROSS ECOSYSTEM:**".to_string());
                            sections.push(format!("Total tasks in database: {}", tasks.len()));
                            sections.push("\nRecent tasks:".to_string());
                            for (i, task) in tasks.iter().take(30).enumerate() {
                                sections.push(format!(
                                    "  {}. [{}] {} - {}",
                                    i + 1,
                                    task.status,
                                    task.title,
                                    task.description.as_deref().unwrap_or("No description")
                                ));
                            }
                            sections.push("".to_string());
                        } else {
                            sections.push("**LIVE DATA:** No tasks found in database.".to_string());
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to fetch all tasks: {}", e);
                    }
                }
            }
        }

        let ctx = self.context.read().await;

        if !ctx.active_projects.is_empty() {
            sections.push("Active projects (static context):".to_string());
            for project in &ctx.active_projects {
                sections.push(format!(
                    "- {} (status: {}, progress: {}%)",
                    project.name,
                    Self::humanise_status(&project.status),
                    project.progress_percentage.round()
                ));
            }
        }

        if !ctx.current_priorities.is_empty() {
            sections.push("Priority focus areas:".to_string());
            for priority in &ctx.current_priorities {
                sections.push(format!(
                    "- {} ({} / status: {})",
                    priority.title,
                    match priority.urgency {
                        PriorityUrgency::Low => "low urgency",
                        PriorityUrgency::Medium => "medium urgency",
                        PriorityUrgency::High => "high urgency",
                        PriorityUrgency::Critical => "critical",
                    },
                    Self::humanise_priority_status(&priority.status)
                ));
            }
        }

        drop(ctx);

        let memory = self.memory.read().await;
        let recent = memory.recent_interactions(3);
        if !recent.is_empty() {
            sections.push("Recent dialogue snippets:".to_string());
            for interaction in recent {
                sections.push(format!(
                    "- User: {} | Nora: {}",
                    interaction.user_input, interaction.nora_response
                ));
            }
        }

        sections.push(format!(
            "Request type: {:?}, priority: {:?}",
            request.request_type, request.priority
        ));
        sections.push(format!("Timestamp: {}", Utc::now().to_rfc3339()));

        sections.join("\n")
    }

    async fn generate_llm_response(&self, request: &NoraRequest, user_text: &str) -> String {
        if let Some(llm) = &self.llm {
            let context_snapshot = self.build_context_snapshot(request).await;
            let system_prompt = self.system_prompt();
            match llm
                .generate(&system_prompt, user_text, &context_snapshot)
                .await
            {
                Ok(answer) if !answer.trim().is_empty() => answer.trim().to_string(),
                Ok(_) => self
                    .generate_rule_based_response(user_text)
                    .await
                    .unwrap_or_else(Self::default_follow_up),
                Err(err) => {
                    tracing::warn!("LLM generation failed: {}", err);
                    self.generate_rule_based_response(user_text)
                        .await
                        .unwrap_or_else(Self::default_follow_up)
                }
            }
        } else {
            self.generate_rule_based_response(user_text)
                .await
                .unwrap_or_else(Self::default_follow_up)
        }
    }

    fn system_prompt(&self) -> String {
        self.config
            .llm
            .as_ref()
            .map(|cfg| cfg.system_prompt.clone())
            .unwrap_or_else(|| {
                "You are Nora, an operational super-intelligence and executive assistant for PowerClub Global.

You have LIVE DATABASE ACCESS to the entire PCG ecosystem. When the user asks about a specific project, you are provided with REAL-TIME data including:
- All current tasks with their status, priority, and descriptions
- Project statistics (total, completed, in progress, blocked tasks)
- Boards and pods associated with the project
- Full repository information

CAPABILITIES:
- Query and report on any project's current state
- Create new tasks when requested or needed
- Update task statuses
- Analyze project progress and identify blockers
- Suggest improvements based on actual data
- Track changes and objectives across all projects

When presented with LIVE DATA, use it as your primary source of truth. Answer questions with specific, data-driven insights based on the actual current state of projects.

Provide concise, insight-driven British executive responses. Surface actionable next steps and be proactive about identifying risks or opportunities.".to_string()
            })
    }

    fn default_follow_up() -> String {
        "Right, I understand what you're getting at. Let me analyse this properly and get you some actionable recommendations. Would you like me to dig deeper into this for you?".to_string()
    }

    async fn stage_virtual_project(&self, project_name: &str) -> String {
        let mut context = self.context.write().await;
        if context
            .active_projects
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(project_name))
        {
            return format!(
                "{} is already staged. Point me at the pods or deliverables you want me to move next.",
                project_name
            );
        }

        let project_id = Uuid::new_v4().to_string();
        let kickoff = Milestone {
            id: format!("{}-kickoff", project_id),
            name: "Kickoff packet".to_string(),
            due_date: Utc::now() + Duration::days(7),
            status: MilestoneStatus::NotStarted,
            completion_percentage: 0.0,
        };

        context.active_projects.push(ProjectContext {
            project_id: project_id.clone(),
            name: project_name.to_string(),
            description: format!(
                "Autogenerated by Nora on {}",
                Utc::now().format("%d %b %Y %H:%M UTC")
            ),
            status: ProjectStatus::Planning,
            progress_percentage: 0.0,
            team_members: vec!["NORA Automation".to_string()],
            budget_status: BudgetStatus {
                allocated: 0.0,
                spent: 0.0,
                remaining: 0.0,
                burn_rate: 0.0,
                forecast_completion: 0.0,
            },
            key_milestones: vec![kickoff],
            risks: Vec::new(),
        });

        format!(
            "Project {} is staged in my working memory. Hand me repos or specs and I'll sync the real boards when they're ready.",
            project_name
        )
    }

    async fn bootstrap_project(&self, project_name: &str) -> Result<String> {
        let executor = self
            .executor
            .as_ref()
            .ok_or_else(|| NoraError::ExecutionError("Task executor not initialised".to_string()))?;

        if let Some(existing) = executor
            .find_project_record_by_name(project_name)
            .await?
        {
            self.add_project_to_context(&existing).await;
            return Ok(format!(
                "{} already exists in the command centre. I can dive straight into its boards, pods, or tasks whenever you’re ready.",
                existing.name
            ));
        }

        let slug = Self::slugify_name(project_name);
        let workspace_root = Self::detect_workspace_root();
        let repo_path = workspace_root.join(&slug);
        let repo_existed = repo_path.exists();
        fs::create_dir_all(&repo_path)?;

        if !repo_path.join(".git").exists() {
            if let Err(err) = Self::initialise_git_repo(&repo_path) {
                tracing::warn!(
                    "Failed to initialise git repo at {}: {}",
                    repo_path.display(),
                    err
                );
            }
        }

        let payload = CreateProject {
            name: project_name.to_string(),
            git_repo_path: repo_path.to_string_lossy().to_string(),
            use_existing_repo: repo_existed,
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
        };

        let project = executor.create_project_entry(payload).await?;
        let boards = executor.ensure_default_boards(project.id).await?;
        let pods = executor.seed_default_pods(project.id).await?;
        let default_board = executor.get_default_board_for_tasks(project.id).await?;
        let board_name = default_board
            .as_ref()
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "the intake board".to_string());
        let board_id = default_board.as_ref().map(|b| b.id);

        let kickoff_tasks = if let Some(board_id) = board_id {
            executor
                .create_tasks_batch(
                    project.id,
                    Self::spinup_task_templates(project_name, board_id),
                )
                .await?
        } else {
            Vec::new()
        };

        self.add_project_to_context(&project).await;

        Ok(format!(
            "{} is now live at {}. I stood up {} boards, {} pods, and {} starter tasks on {}. Point me at specs or repos and I’ll keep the pipeline moving.",
            project.name,
            repo_path.display(),
            boards.len(),
            pods.len(),
            kickoff_tasks.len(),
            board_name
        ))
    }

    async fn add_project_to_context(&self, project: &Project) {
        let mut context = self.context.write().await;
        let entry = Self::map_project_to_context(project);
        context
            .active_projects
            .retain(|existing| existing.project_id != entry.project_id);
        context.active_projects.push(entry);
        context.last_updated = Utc::now();
    }

    async fn resolve_project_for_task(
        &self,
        project_hint: Option<&str>,
    ) -> Option<(Uuid, String)> {
        if let Some(executor) = &self.executor {
            if let Some(hint) = project_hint {
                if let Ok(Some(project)) = executor.find_project_record_by_name(hint).await {
                    return Some((project.id, project.name));
                }
            }
        }

        let context_hint = {
            let context = self.context.read().await;
            context.active_projects.first().cloned()
        };

        if let Some(project_ctx) = context_hint {
            if let Ok(uuid) = Uuid::parse_str(&project_ctx.project_id) {
                return Some((uuid, project_ctx.name));
            }

            if let Some(executor) = &self.executor {
                if let Ok(project_id) = executor.find_project_by_name(&project_ctx.name).await {
                    return Some((project_id, project_ctx.name));
                }
            }
        }

        None
    }

    async fn try_create_task_record(
        &self,
        task_name: &str,
        project_hint: Option<&str>,
        utterance: &str,
    ) -> Option<String> {
        let executor = self.executor.as_ref()?;
        let (project_id, project_label) =
            self.resolve_project_for_task(project_hint).await?;
        let board = match executor.get_default_board_for_tasks(project_id).await {
            Ok(board) => board,
            Err(err) => {
                tracing::error!("Failed to look up default board: {}", err);
                None
            }
        };

        let board_name = board
            .as_ref()
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "the intake board".to_string());
        let definition = TaskDefinition {
            title: task_name.to_string(),
            description: Some("Captured via Nora directive".to_string()),
            priority: Some(Self::infer_priority_from_text(utterance)),
            tags: Some(vec!["nora".to_string(), "direct".to_string()]),
            assignee_id: None,
            board_id: board.as_ref().map(|b| b.id),
            pod_id: None,
        };

        match executor.create_task(project_id, definition).await {
            Ok(task) => Some(format!(
                "Task '{}' is live on {} for {} ({}). Let me know when to assign owners or flesh out scope.",
                task.title,
                board_name,
                project_label,
                task.id
            )),
            Err(err) => {
                tracing::error!("Failed to create task from directive: {}", err);
                None
            }
        }
    }

    fn infer_priority_from_text(text: &str) -> Priority {
        let lowered = text.to_lowercase();
        if lowered.contains("critical") || lowered.contains("urgent") || lowered.contains("asap") {
            Priority::Critical
        } else if lowered.contains("eventually") || lowered.contains("whenever") {
            Priority::Low
        } else if lowered.contains("plan") || lowered.contains("research") {
            Priority::Medium
        } else {
            Priority::High
        }
    }

    fn detect_workspace_root() -> PathBuf {
        for key in ["NORA_PROJECTS_ROOT", "PCG_PROJECTS_ROOT", "WORKSPACE_DIR"] {
            if let Ok(path) = std::env::var(key) {
                if !path.trim().is_empty() {
                    let buf = PathBuf::from(path);
                    if buf.exists() {
                        return buf;
                    }
                }
            }
        }

        match std::env::current_dir() {
            Ok(dir) => {
                if dir
                    .file_name()
                    .map(|n| n == "pcg-cc-mcp")
                    .unwrap_or(false)
                {
                    dir.parent().map(Path::to_path_buf).unwrap_or(dir)
                } else {
                    dir
                }
            }
            Err(_) => PathBuf::from("."),
        }
    }

    fn slugify_name(input: &str) -> String {
        let mut slug = String::with_capacity(input.len());
        let mut prev_dash = false;
        for ch in input.chars() {
            if ch.is_ascii_alphanumeric() {
                slug.push(ch.to_ascii_lowercase());
                prev_dash = false;
            } else if !prev_dash {
                slug.push('-');
                prev_dash = true;
            }
        }
        let cleaned = slug.trim_matches('-').to_string();
        if cleaned.is_empty() {
            "new-project".to_string()
        } else {
            cleaned
        }
    }

    fn initialise_git_repo(path: &Path) -> io::Result<()> {
        let status = Command::new("git").arg("init").current_dir(path).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "git init exited with an error",
            ))
        }
    }

    fn spinup_task_templates(name: &str, board_id: Uuid) -> Vec<TaskDefinition> {
        vec![
            TaskDefinition {
                title: format!("Author {} briefing + success criteria", name),
                description: Some("Summarise objectives, KPIs, and counterparties".to_string()),
                priority: Some(Priority::High),
                tags: Some(vec!["nora".to_string(), "spinup".to_string()]),
                assignee_id: None,
                board_id: Some(board_id),
                pod_id: None,
            },
            TaskDefinition {
                title: format!("Stand up repo + automation for {}", name),
                description: Some("Scaffold README, tooling, and CI hooks".to_string()),
                priority: Some(Priority::Medium),
                tags: Some(vec!["nora".to_string(), "infra".to_string()]),
                assignee_id: None,
                board_id: Some(board_id),
                pod_id: None,
            },
            TaskDefinition {
                title: format!("Collect research + asset tracker for {}", name),
                description: Some("Ingest docs, references, and blockers into the board".to_string()),
                priority: Some(Priority::Medium),
                tags: Some(vec!["nora".to_string(), "research".to_string()]),
                assignee_id: None,
                board_id: Some(board_id),
                pod_id: None,
            },
        ]
    }

    fn map_project_to_context(project: &Project) -> ProjectContext {
        ProjectContext {
            project_id: project.id.to_string(),
            name: project.name.clone(),
            description: format!("Repository: {}", project.git_repo_path.to_string_lossy()),
            status: ProjectStatus::InProgress,
            progress_percentage: 0.0,
            team_members: Vec::new(),
            budget_status: BudgetStatus {
                allocated: 0.0,
                spent: 0.0,
                remaining: 0.0,
                burn_rate: 0.0,
                forecast_completion: 0.0,
            },
            key_milestones: Vec::new(),
            risks: Vec::new(),
        }
    }

    async fn generate_rule_based_response(&self, user_text: &str) -> Option<String> {
        static PROJECT_CREATE_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r#"(?ix)(?:start|create|spin\s+up|launch|open)\s+(?:a\s+)?(?:new\s+)?project(?:\s+(?:called|named))?\s+"?([a-z0-9 _\-]+)"?)"#
            )
            .expect("valid project regex")
        });
        static TASK_CREATE_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r#"(?ix)(?:create|add|open)\s+(?:a\s+)?(?:new\s+)?task(?:\s+(?:called|named))?\s+"?([a-z0-9 _\-]+)"?(?:\s+for\s+(?:project\s+)??"?([a-z0-9 _\-]+)"?)?)"#
            )
            .expect("valid task regex")
        });
        let utterance = user_text.to_lowercase();

        if let Some(caps) = PROJECT_CREATE_REGEX.captures(&utterance) {
            let name_raw = caps
                .get(1)
                .map(|m| m.as_str().trim())
                .filter(|s| !s.is_empty())?;
            let project_name = title_case(name_raw);
            if self.executor.is_some() {
                match self.bootstrap_project(&project_name).await {
                    Ok(message) => return Some(message),
                    Err(err) => {
                        tracing::error!(
                            "Failed to create project {} via directive: {}",
                            project_name,
                            err
                        );
                        let fallback = self.stage_virtual_project(&project_name).await;
                        return Some(format!(
                            "I couldn’t bring {} online in the live system ({}). I’ve staged it virtually instead so we don’t lose the intent.\n{}",
                            project_name,
                            err,
                            fallback
                        ));
                    }
                }
            }

            return Some(self.stage_virtual_project(&project_name).await);
        }

        if let Some(caps) = TASK_CREATE_REGEX.captures(&utterance) {
            let task_name = caps
                .get(1)
                .map(|m| m.as_str().trim())
                .filter(|s| !s.is_empty())
                .map(title_case)?;
            let project_hint = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string());

            if let Some(message) = self
                .try_create_task_record(&task_name, project_hint.as_deref(), user_text)
                .await
            {
                return Some(message);
            }

            let context = self.context.read().await;
            let target_project = project_hint
                .as_ref()
                .and_then(|hint| {
                    context
                        .active_projects
                        .iter()
                        .find(|p| p.name.eq_ignore_ascii_case(hint))
                })
                .or_else(|| context.active_projects.first());

            if let Some(project) = target_project {
                return Some(format!(
                    "Queued task '{}' under {}. I can flesh out acceptance criteria or drop it straight into MCP when you're ready.",
                    task_name, project.name
                ));
            }

            return Some(format!(
                "I can schedule '{}' as soon as you tell me which project or board should own it.",
                task_name
            ));
        }

        let context = self.context.read().await;

        if utterance.contains("project") || utterance.contains("pipeline") || utterance.contains("roadmap") {
            let projects = &context.active_projects;
            if projects.is_empty() {
                return Some(
                    "I’m not currently tracking any active initiatives. Ask me to sync a project and I’ll keep it on radar for you.".to_string(),
                );
            }

            let mut lines = Vec::new();
            lines.push(format!("I’m monitoring {} active initiatives:", projects.len()));
            for project in projects.iter().take(5) {
                let milestone_summary = project
                    .key_milestones
                    .iter()
                    .find(|m| matches!(m.status, MilestoneStatus::InProgress | MilestoneStatus::NotStarted))
                    .map(|m| format!("next milestone '{}' due {}", m.name, m.due_date.format("%b %d")))
                    .unwrap_or_else(|| "no upcoming milestones logged".to_string());

                lines.push(format!(
                    "- {} ({:?}) · {:.0}% complete · {} specialists assigned · {}.",
                    project.name,
                    project.status,
                    project.progress_percentage,
                    project.team_members.len(),
                    milestone_summary
                ));
            }

            if projects.len() > 5 {
                lines.push(format!(
                    "…plus {} additional initiatives ready for deeper review.",
                    projects.len() - 5
                ));
            }

            lines.push("Tell me which thread you’d like to dive into and I can pull boards, pods, or task stats on demand.".to_string());
            return Some(lines.join("\n"));
        }

        if utterance.contains("capab") || utterance.contains("ability") || utterance.contains("what can you do") {
            return Some(
                "Here’s what I’m cleared to do for you right now:\n- Summarise active projects and surface risks ahead of reviews\n- Coordinate human + AI agents via the MCP tools (create/update tasks, monitor pods, run diffs)\n- Assemble exec-ready briefs, retros, or go/no-go packets from repository data\n- Watch budget burn, milestones, and release health so you get early warning\nJust give me a directive and I’ll move the pieces.".to_string(),
            );
        }

        if utterance.starts_with("hello") || utterance.starts_with("hi") || utterance.contains("good morning") {
            return Some("Hello there. I’m synced with the coordination feeds and ready whenever you are.".to_string());
        }

        None
    }

    fn default_projects() -> Vec<ProjectContext> {
        let now = Utc::now();
        vec![
            ProjectContext {
                project_id: "pcg-dashboard-mcp".to_string(),
                name: "PCG Dashboard MCP".to_string(),
                description: "Executive control centre combining multi-agent orchestration and performance telemetry.".to_string(),
                status: ProjectStatus::InProgress,
                progress_percentage: 72.0,
                team_members: vec!["Platform".to_string(), "Data Ops".to_string()],
                budget_status: BudgetStatus {
                    allocated: 1_000_000.0,
                    spent: 620_000.0,
                    remaining: 380_000.0,
                    burn_rate: 1.12,
                    forecast_completion: 0.9,
                },
                key_milestones: vec![Milestone {
                    id: "mcp-milestone-1".to_string(),
                    name: "Voice + Orchestration GA".to_string(),
                    due_date: now + Duration::weeks(4),
                    status: MilestoneStatus::InProgress,
                    completion_percentage: 65.0,
                }],
                risks: vec![],
            },
            ProjectContext {
                project_id: "powerclub-global".to_string(),
                name: "PowerClub Global Coordination".to_string(),
                description: "Enterprise operations programme covering global club launches.".to_string(),
                status: ProjectStatus::OnHold,
                progress_percentage: 48.0,
                team_members: vec!["Operations".to_string(), "Finance".to_string()],
                budget_status: BudgetStatus {
                    allocated: 2_500_000.0,
                    spent: 1_150_000.0,
                    remaining: 1_350_000.0,
                    burn_rate: 0.95,
                    forecast_completion: 0.8,
                },
                key_milestones: vec![Milestone {
                    id: "powerclub-q4".to_string(),
                    name: "Q4 Expansion Blueprint".to_string(),
                    due_date: now + Duration::weeks(8),
                    status: MilestoneStatus::NotStarted,
                    completion_percentage: 0.0,
                }],
                risks: vec![],
            },
            ProjectContext {
                project_id: "experience-the-game".to_string(),
                name: "Experience the Game".to_string(),
                description: "Immersive fan engagement platform spanning live events and digital twins.".to_string(),
                status: ProjectStatus::InProgress,
                progress_percentage: 61.0,
                team_members: vec!["Product".to_string(), "Marketing".to_string()],
                budget_status: BudgetStatus {
                    allocated: 1_800_000.0,
                    spent: 1_020_000.0,
                    remaining: 780_000.0,
                    burn_rate: 1.08,
                    forecast_completion: 0.88,
                },
                key_milestones: vec![Milestone {
                    id: "etg-beta".to_string(),
                    name: "Beta cohort launch".to_string(),
                    due_date: now + Duration::weeks(6),
                    status: MilestoneStatus::InProgress,
                    completion_percentage: 55.0,
                }],
                risks: vec![],
            },
            ProjectContext {
                project_id: "chimia-dao".to_string(),
                name: "Chimia DAO".to_string(),
                description: "Decentralised governance framework for sustainability investments.".to_string(),
                status: ProjectStatus::Planning,
                progress_percentage: 32.0,
                team_members: vec!["Research".to_string(), "Legal".to_string()],
                budget_status: BudgetStatus {
                    allocated: 950_000.0,
                    spent: 220_000.0,
                    remaining: 730_000.0,
                    burn_rate: 0.65,
                    forecast_completion: 0.75,
                },
                key_milestones: vec![Milestone {
                    id: "dao-charter".to_string(),
                    name: "Charter ratification".to_string(),
                    due_date: now + Duration::weeks(10),
                    status: MilestoneStatus::NotStarted,
                    completion_percentage: 0.0,
                }],
                risks: vec![],
            },
        ]
    }

    fn default_priorities() -> Vec<ExecutivePriority> {
        vec![
            ExecutivePriority {
                id: "priority-voice".to_string(),
                title: "Voice concierge launch".to_string(),
                description: "Stabilise Nora's voice concierge and integrate orchestration hooks."
                    .to_string(),
                urgency: PriorityUrgency::High,
                impact: PriorityImpact::Strategic,
                owner: "Innovation Office".to_string(),
                target_date: Some(Utc::now() + Duration::weeks(4)),
                status: PriorityStatus::InProgress,
            },
            ExecutivePriority {
                id: "priority-roadmap".to_string(),
                title: "Roadmap transparency".to_string(),
                description: "Publish near-real time roadmap summaries for stakeholders."
                    .to_string(),
                urgency: PriorityUrgency::Medium,
                impact: PriorityImpact::High,
                owner: "Strategy Team".to_string(),
                target_date: Some(Utc::now() + Duration::weeks(6)),
                status: PriorityStatus::Planned,
            },
        ]
    }

    fn humanise_status(status: &ProjectStatus) -> String {
        match status {
            ProjectStatus::Planning => "planning",
            ProjectStatus::InProgress => "in progress",
            ProjectStatus::OnHold => "on hold",
            ProjectStatus::AtRisk => "at risk",
            ProjectStatus::Completed => "completed",
            ProjectStatus::Cancelled => "cancelled",
        }
        .to_string()
    }

    fn humanise_priority_status(status: &PriorityStatus) -> String {
        match status {
            PriorityStatus::Planned => "planned",
            PriorityStatus::InProgress => "in progress",
            PriorityStatus::OnTrack => "on track",
            PriorityStatus::AtRisk => "at risk",
            PriorityStatus::Delayed => "delayed",
            PriorityStatus::Completed => "completed",
        }
        .to_string()
    }

    async fn extract_context_updates(
        &self,
        request: &NoraRequest,
        response: &str,
    ) -> Vec<ContextUpdate> {
        let mut updates = Vec::new();

        // Extract insights from response
        if response.contains("project") || response.contains("initiative") {
            updates.push(ContextUpdate {
                update_type: "ProjectMention".to_string(),
                key: "recent_project_discussion".to_string(),
                value: serde_json::json!({
                    "request_type": format!("{:?}", request.request_type),
                    "timestamp": Utc::now().to_rfc3339()
                }),
                confidence: 0.8,
                source: "response_analysis".to_string(),
            });
        }

        if response.contains("priority") || response.contains("urgent") {
            updates.push(ContextUpdate {
                update_type: "PriorityShift".to_string(),
                key: "priority_discussion".to_string(),
                value: serde_json::json!({
                    "priority_level": format!("{:?}", request.priority),
                    "timestamp": Utc::now().to_rfc3339()
                }),
                confidence: 0.7,
                source: "priority_analysis".to_string(),
            });
        }

        updates
    }

    /// Check if a message is a confirmation
    async fn is_confirmation(&self, lowered: &str) -> bool {
        // Check if there's a pending action first
        let memory = self.memory.read().await;
        if memory.get_pending_action().is_none() {
            return false;
        }

        // Common confirmation phrases (verbal and text)
        lowered == "yes"
            || lowered == "yeah"
            || lowered == "yep"
            || lowered == "sure"
            || lowered == "ok"
            || lowered == "okay"
            || lowered == "confirm"
            || lowered == "confirmed"
            || lowered == "approve"
            || lowered == "approved"
            || lowered == "go ahead"
            || lowered == "do it"
            || lowered == "please do"
            || lowered.starts_with("yes,")
            || lowered.starts_with("yeah,")
            || lowered.starts_with("sure,")
    }

    /// Handle a confirmation from the user
    async fn handle_confirmation(&self) -> Result<Option<String>> {
        let mut memory = self.memory.write().await;
        let pending_action = match memory.clear_pending_action() {
            Some(action) => action,
            None => return Ok(None),
        };

        // Execute the pending action
        match pending_action.action_type {
            crate::memory::PendingActionType::CreateTasks => {
                if let (Some(executor), Some(project_id)) =
                    (self.executor.as_ref(), pending_action.project_id)
                {
                    let task_defs: Vec<TaskDefinition> = pending_action
                        .tasks
                        .iter()
                        .map(|t| TaskDefinition {
                            title: t.title.clone(),
                            description: t.description.clone(),
                            priority: t.priority.as_ref().and_then(|p| match p.as_str() {
                                "high" => Some(Priority::High),
                                "medium" => Some(Priority::Medium),
                                "low" => Some(Priority::Low),
                                _ => None,
                            }),
                            tags: t.tags.clone(),
                            assignee_id: None,
                            board_id: None,
                            pod_id: None,
                        })
                        .collect();

                    let created_tasks = executor.create_tasks_batch(project_id, task_defs).await?;

                    let task_list = created_tasks
                        .iter()
                        .map(|t| format!("- {}", t.title))
                        .collect::<Vec<_>>()
                        .join("\n");

                    Ok(Some(format!(
                        "Excellent! I've created {} tasks for {}:\n\n{}\n\nAll tasks are now on the board and ready to go.",
                        created_tasks.len(),
                        pending_action.project_name.unwrap_or_else(|| "the project".to_string()),
                        task_list
                    )))
                } else {
                    Ok(Some(
                        "I'm sorry, but I'm unable to execute tasks at the moment. The database connection isn't available.".to_string(),
                    ))
                }
            }
            _ => Ok(Some("Action confirmed!".to_string())),
        }
    }

    /// Extract tasks from user request and LLM response, then execute or confirm
    async fn extract_and_execute_tasks(
        &self,
        user_input: &str,
        llm_response: &str,
    ) -> Result<()> {
        use crate::memory::{PendingAction, PendingActionType};

        // Check if executor is available
        if self.executor.is_none() {
            return Ok(());
        }

        let executor = self.executor.as_ref().unwrap();
        let lowered_input = user_input.to_lowercase();
        let lowered_response = llm_response.to_lowercase();

        // Detect if this is a direct task creation order
        let is_direct_order = lowered_input.contains("create task")
            || lowered_input.contains("add task")
            || lowered_input.contains("make task")
            || lowered_input.starts_with("create")
            || lowered_input.starts_with("add");

        // Detect if tasks are mentioned in the response
        let mentions_tasks = lowered_response.contains("task")
            || lowered_response.contains("will create")
            || lowered_response.contains("i'll create");

        if !mentions_tasks {
            return Ok(());
        }

        // Try to extract project name from user input
        let project_name = self.extract_project_name(user_input);

        // If project name found, try to resolve it
        if let Some(proj_name) = project_name {
            match executor.find_project_by_name(&proj_name).await {
                Ok(project_id) => {
                    // Extract task information from LLM response
                    let tasks = self.extract_tasks_from_response(llm_response);

                    if !tasks.is_empty() {
                        if is_direct_order {
                            // Direct order - execute immediately
                            let task_defs: Vec<TaskDefinition> = tasks
                                .iter()
                                .map(|t| TaskDefinition {
                                    title: t.title.clone(),
                                    description: t.description.clone(),
                                    priority: t.priority.as_ref().and_then(|p| match p.as_str() {
                                        "high" => Some(Priority::High),
                                        "medium" => Some(Priority::Medium),
                                        "low" => Some(Priority::Low),
                                        _ => None,
                                    }),
                                    tags: t.tags.clone(),
                                    assignee_id: None,
                                    board_id: None,
                                    pod_id: None,
                                })
                                .collect();

                            executor.create_tasks_batch(project_id, task_defs).await?;
                            tracing::info!(
                                "Autonomously created {} tasks for project {}",
                                tasks.len(),
                                proj_name
                            );
                        } else {
                            // Not a direct order - store for confirmation
                            let mut memory = self.memory.write().await;
                            memory.set_pending_action(PendingAction {
                                action_id: Uuid::new_v4().to_string(),
                                action_type: PendingActionType::CreateTasks,
                                project_name: Some(proj_name),
                                project_id: Some(project_id),
                                tasks,
                                created_at: Utc::now(),
                            });
                        }
                    }
                }
                Err(_) => {
                    tracing::warn!("Could not find project: {}", proj_name);
                }
            }
        }

        Ok(())
    }

    /// Extract project name from user input
    fn extract_project_name(&self, input: &str) -> Option<String> {
        let lowered = input.to_lowercase();

        // Common patterns: "for X project", "in X", "X project"
        if let Some(pos) = lowered.find(" for ") {
            let after = &input[pos + 5..];
            let words: Vec<&str> = after.split_whitespace().take(3).collect();
            if !words.is_empty() {
                let project = words.join(" ").trim_end_matches(" project").to_string();
                return Some(project);
            }
        }

        if let Some(pos) = lowered.find(" in ") {
            let after = &input[pos + 4..];
            let words: Vec<&str> = after.split_whitespace().take(3).collect();
            if !words.is_empty() {
                let project = words.join(" ").trim_end_matches(" project").to_string();
                return Some(project);
            }
        }

        // Look for project names at the start
        let words: Vec<&str> = input.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if word.to_lowercase() == "project" && i > 0 {
                return Some(words[i - 1].to_string());
            }
        }

        None
    }

    /// Extract task information from LLM response
    fn extract_tasks_from_response(&self, response: &str) -> Vec<crate::memory::PendingTask> {
        use crate::memory::PendingTask;

        let mut tasks = Vec::new();

        // Look for markdown bullet points or numbered lists
        for line in response.lines() {
            let trimmed = line.trim();

            // Match patterns like "- Task name" or "1. Task name"
            if trimmed.starts_with('-') || trimmed.starts_with('*') {
                let title = trimmed
                    .trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim()
                    .to_string();
                if !title.is_empty() && !title.to_lowercase().contains("task") {
                    tasks.push(PendingTask {
                        title,
                        description: None,
                        priority: Some("medium".to_string()),
                        tags: None,
                    });
                }
            } else if let Some(pos) = trimmed.find(". ") {
                if pos > 0 && pos < 3 {
                    if trimmed[..pos].chars().all(|c| c.is_ascii_digit()) {
                        let title = trimmed[pos + 2..].trim().to_string();
                        if !title.is_empty() && !title.to_lowercase().contains("task") {
                            tasks.push(PendingTask {
                                title,
                                description: None,
                                priority: Some("medium".to_string()),
                                tags: None,
                            });
                        }
                    }
                }
            }
        }

        tasks
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
