//! TopsiAgent - Platform Agent with containerized access control
//!
//! Topsi is the platform orchestrator that manages all projects and users
//! with strict data isolation between clients.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::config::TopsiConfig;
use crate::meeting::{MeetingManager, MeetingTranscriptEntry, MeetingNotes, ActionItem, WakeWordResult};
use crate::topology::graph::TopologyGraph;
use crate::topology::voice::VoiceTopology;
use crate::tools::get_tool_schemas;
use crate::{DetectedIssue, Result, TopsiError, TopsiResponse, TopologySummary, ToolCallResult};

// Import Nora conversation types for agentic loop
use nora::brain::{ConversationMessage, ToolResult as NoraToolResult};

// Database models for querying real data
use db::models::project::Project;
use db::models::agent::Agent;
use db::models::task::Task;

// Import Nora's LLM infrastructure
use nora::brain::{
    infer_provider_from_model, LLMClient, LLMConfig as NoraLLMConfig, LLMProvider, LLMResponse,
};

/// Topsi's system prompt - defines its role as conversational task orchestrator
const TOPSI_SYSTEM_PROMPT: &str = r#"You are Topsi, the Topological Super Intelligence — the central orchestrator for the PowerClub Global platform.

## Your Role
You are a conversational project orchestrator. Users talk to you through chat, voice, and the virtual environment. When they have work to be done, you:
1. Understand their request
2. Break it into concrete tasks
3. Assign tasks to the right agents
4. Kick off execution
5. Monitor progress and report back
6. Handle issues, retries, and follow-ups

## How to Execute Work
- Use `create_task` with `agent_name` to create AND auto-execute in one step. Execution starts automatically when an agent is assigned.
- NEVER call `start_task_execution` in the same turn as `create_task` — `create_task` handles execution when `agent_name` is provided.
- Use `start_task_execution` ONLY to re-run or retry an existing task that was created without an agent.
- Use `get_task_status` to check on running tasks
- Use `list_tasks` to see the full picture
- Use `update_task` to mark work done or adjust priorities

## Agent Selection
- **claude** — Best for complex multi-file coding, architecture decisions, full-stack development
- **gemini** — Good for analysis, documentation, data processing
- **amp** — Fast iteration on focused coding tasks

## Task Decomposition
For complex requests like "build a website", break into phases:
1. Research & Planning tasks (gather requirements, analyze competitors)
2. Setup tasks (scaffold project, install dependencies)
3. Implementation tasks (build components, pages, features — can run in parallel)
4. Integration tasks (connect pieces, API routes)
5. Verification tasks (testing, build checks)
6. Deployment tasks (push to git, deploy to hosting)

## Execution Style
- ALWAYS execute immediately. Never ask "Would you like me to...?" or "Shall I proceed?" — just do it.
- When asked to do something: create the task, start execution, and confirm what you did in one step.
- If multiple steps are needed, do them all, then give a brief update.

## Conversation Style
- Be brief and conversational. 1-3 sentences for updates.
- Report progress naturally: "Done — kicked off the frontend build with Claude. I'll let you know when it's finished."
- When tasks complete, give a SHORT summary (1-2 sentences). The user can check details themselves or ask for more.
- If something fails, explain briefly and suggest next steps.
- Do NOT dump full task details, IDs, or JSON in your responses unless asked.
- Do NOT repeat back the task description you just created.

## Available Tools
### Topology tools
- `list_projects` - List accessible projects
- `list_nodes` - List nodes filtered by type or status
- `list_edges` - List connections between nodes
- `find_path` - Find optimal paths between nodes
- `detect_issues` - Identify topology problems
- `get_topology_summary` - Get overall topology statistics
- `create_cluster` - Form a team or cluster of nodes
- `verify_access` - Check user permissions

### Project & Task tools
- `create_project` - Create a new project
- `update_project` - Update project name or organization assignment
- `list_organizations` - List all organizations (to get IDs for update_project)
- `create_task` - Create a task in a project
- `list_tasks` - List tasks with filtering
- `update_task` - Update task properties
- `list_agents` - List available agents

### Execution tools
- `start_task_execution` - Spawn an agent to execute a task
- `get_task_status` - Check task execution status and logs

### Communication
- `respond_to_user` - IMPORTANT: Use this to deliver your response. Write your complete answer in the message parameter.

## How to Respond
ALWAYS use the `respond_to_user` tool to communicate with users. In the message parameter, write YOUR complete response:
- If asked "tell me a story" → write an actual story in the message
- If asked "who are you?" → write your introduction in the message
- If asked about the system → gather data with other tools, then use respond_to_user to explain
- NEVER just echo the user's request back - always provide your actual response content

## Example Interactions
User: "Build me a landing page for my new product"
→ create_task(agent_name="claude") → respond_to_user: "On it — Claude is building the landing page now."

User: "Research competitor activity with Scout"
→ create_task(agent_name="Scout") → respond_to_user: "Scout is researching competitor activity now. I'll update you when it's done."

User: "How are my tasks going?"
→ list_tasks → get_task_status for in-progress ones → respond_to_user: brief 2-3 line summary

User: "What projects do I have?"
→ list_projects → respond_to_user: short list with names"#;

/// Meeting mode system prompt - instructs Topsi to act as a passive observer
pub const MEETING_SYSTEM_PROMPT: &str = r#"You are Topsi, participating in a team meeting as a silent AI observer.

## Your Role in Meetings
- You are a PASSIVE OBSERVER. Never interject or speak unprompted.
- Only respond when directly addressed (someone says "Topsi" followed by a question or command).
- When addressed: answer concisely and helpfully, then immediately return to silent observation.

## What You Track
While silently observing, you maintain awareness of:
- **Topics discussed**: Main subjects and how they evolve
- **Decisions made**: Any agreed-upon conclusions or choices
- **Action items**: Tasks assigned with who is responsible and any deadlines
- **Open questions**: Unresolved issues or questions raised but not answered
- **Participants**: Who is speaking and their contributions

## When Addressed
- Answer the question or execute the command concisely
- Do not provide unnecessary context or over-explain
- If asked for a summary, provide a structured overview of the meeting so far
- If asked for your opinion, give a brief, considered response
- After responding, return to silent mode

## Meeting Notes Format
When asked to generate notes (at meeting end), use this structure:
- **Summary**: 2-3 sentence overview of the meeting
- **Topics Discussed**: Bullet list of main topics
- **Decisions Made**: Bullet list of decisions with context
- **Action Items**: Each with description, assignee (if mentioned), and deadline (if mentioned)
- **Open Questions**: Unresolved items that need follow-up
- **Participants**: List of identified speakers

## Important
- Keep responses SHORT when addressed mid-meeting (1-3 sentences)
- Be more thorough only when generating end-of-meeting notes
- Never fabricate information — only report what was actually said
- If you're unsure about something, say so rather than guessing"#;

pub mod access_control;
pub use access_control::{AccessControl, AccessScope, UserContext, ProjectAccess};

/// Bridge trait for task execution — implemented by the server deployment layer.
/// This keeps the topsi crate decoupled from deployment/executors internals.
#[async_trait::async_trait]
pub trait TaskExecutionBridge: Send + Sync {
    /// Start a task attempt by creating the attempt record and spawning the executor.
    /// Returns a JSON value with task_attempt_id, execution_process_id, and status.
    async fn start_task_attempt(
        &self,
        task_id: Uuid,
        executor_name: &str,
        base_branch: &str,
    ) -> std::result::Result<serde_json::Value, String>;
}

/// TopsiAgent - The Platform Intelligence Agent
///
/// Topsi serves as the central platform agent with:
/// - Master credential for admin users (full ecosystem visibility)
/// - Containerized access for regular users (project-scoped visibility)
/// - Strict client data isolation
/// - System-wide optimization capabilities
/// - LLM-powered intelligent conversations
pub struct TopsiAgent {
    /// Unique identifier for this Topsi instance
    pub id: Uuid,
    /// Configuration
    pub config: TopsiConfig,
    /// Access control manager
    pub access_control: Arc<AccessControl>,
    /// Database connection
    pub db: Option<SqlitePool>,
    /// Project topologies (indexed by project_id)
    topologies: Arc<RwLock<indexmap::IndexMap<Uuid, TopologyGraph>>>,
    /// Initialization timestamp
    pub initialized_at: DateTime<Utc>,
    /// Active status
    active: Arc<RwLock<bool>>,
    /// LLM client for intelligent conversations
    llm: Option<LLMClient>,
    /// Deployment bridge for triggering task execution
    execution_bridge: Option<Arc<dyn TaskExecutionBridge>>,
    /// Session-based conversation history for multi-turn context
    session_history: Arc<RwLock<HashMap<String, Vec<ConversationMessage>>>>,
    /// Meeting manager for active meeting sessions
    pub meeting_manager: Arc<MeetingManager>,
}

impl TopsiAgent {
    /// Create a new TopsiAgent with configuration
    pub async fn new(config: TopsiConfig) -> Result<Self> {
        tracing::info!("Initializing TopsiAgent: {}", config.name);

        let access_control = Arc::new(AccessControl::new());

        // Initialize LLM client if provider is configured
        let llm = if !config.llm.provider.is_empty() {
            let provider = infer_provider_from_model(&config.llm.model);
            let system_prompt = config
                .system_prompt
                .clone()
                .unwrap_or_else(|| TOPSI_SYSTEM_PROMPT.to_string());

            // Determine endpoint based on provider
            let endpoint = match provider {
                LLMProvider::Ollama => std::env::var("OLLAMA_ENDPOINT").ok().or_else(|| {
                    if std::net::TcpStream::connect("127.0.0.1:11434").is_ok() {
                        Some("http://127.0.0.1:11434/v1/chat/completions".to_string())
                    } else {
                        None
                    }
                }),
                _ => None,
            };

            let nora_config = NoraLLMConfig {
                provider,
                model: config.llm.model.clone(),
                temperature: config.llm.temperature,
                max_tokens: config.llm.max_tokens,
                system_prompt,
                endpoint,
            };

            tracing::info!(
                "Topsi LLM configured: provider={:?}, model={}",
                nora_config.provider,
                nora_config.model
            );

            Some(LLMClient::new(nora_config))
        } else {
            tracing::warn!("Topsi LLM not configured - chat will return stub responses");
            None
        };

        Ok(Self {
            id: Uuid::new_v4(),
            config,
            access_control,
            db: None,
            topologies: Arc::new(RwLock::new(indexmap::IndexMap::new())),
            initialized_at: Utc::now(),
            active: Arc::new(RwLock::new(false)),
            llm,
            execution_bridge: None,
            session_history: Arc::new(RwLock::new(HashMap::new())),
            meeting_manager: Arc::new(MeetingManager::new()),
        })
    }

    /// Attach database connection and sync access control
    pub async fn with_database(mut self, pool: SqlitePool) -> Self {
        // Sync access control from database so user permissions are loaded
        if let Err(e) = self.access_control.sync_from_database(&pool).await {
            tracing::error!("Failed to sync Topsi access control from database: {}", e);
        }
        self.db = Some(pool);
        self
    }

    /// Attach a task execution bridge for triggering agent execution
    pub fn with_execution_bridge(mut self, bridge: Arc<dyn TaskExecutionBridge>) -> Self {
        self.execution_bridge = Some(bridge);
        self
    }

    /// Set active state
    pub async fn set_active(&self, active: bool) -> Result<()> {
        let mut state = self.active.write().await;
        *state = active;
        tracing::info!("Topsi active state set to: {}", active);
        Ok(())
    }

    /// Check if Topsi is active
    pub async fn is_active(&self) -> bool {
        *self.active.read().await
    }

    /// Get uptime in milliseconds
    pub fn uptime_ms(&self) -> i64 {
        (Utc::now() - self.initialized_at).num_milliseconds()
    }

    /// Get the current access scope for a user
    pub async fn get_user_scope(&self, user_context: &UserContext) -> AccessScope {
        self.access_control.get_scope(user_context).await
    }

    /// Process a request with access control
    pub async fn process_request(
        &self,
        request: TopsiRequest,
        user_context: &UserContext,
        session_id: Option<&str>,
    ) -> Result<TopsiResponse> {
        // Verify user has appropriate access
        let scope = self.access_control.get_scope(user_context).await;

        // Log access for audit
        self.access_control.log_access(
            user_context,
            &format!("{:?}", request.request_type),
            true,
        ).await;

        match request.request_type {
            TopsiRequestType::Chat { message } => {
                self.handle_chat(&message, user_context, &scope, session_id).await
            }
            TopsiRequestType::GetTopology { project_id } => {
                self.handle_get_topology(project_id, user_context, &scope).await
            }
            TopsiRequestType::DetectIssues { project_id } => {
                self.handle_detect_issues(project_id, user_context, &scope).await
            }
            TopsiRequestType::ListProjects => {
                self.handle_list_projects(user_context, &scope).await
            }
            TopsiRequestType::ExecuteCommand { command } => {
                self.handle_command(&command, user_context, &scope).await
            }
            TopsiRequestType::GetRecommendations { project_id, max_count } => {
                self.handle_get_recommendations(project_id, max_count, user_context, &scope).await
            }
            TopsiRequestType::StartMeeting { project_id, title } => {
                self.handle_start_meeting(&project_id, title.as_deref(), user_context).await
            }
            TopsiRequestType::EndMeeting { session_id, generate_notes } => {
                self.handle_end_meeting(&session_id, generate_notes, user_context).await
            }
            TopsiRequestType::MeetingAudioChunk { session_id, audio_data, chunk_index, duration_ms } => {
                self.handle_meeting_audio_chunk(&session_id, &audio_data, chunk_index, duration_ms, user_context).await
            }
            TopsiRequestType::MeetingDirectAddress { session_id, message, transcript_context } => {
                self.handle_meeting_direct_address(&session_id, &message, transcript_context.as_deref(), user_context).await
            }
        }
    }

    /// Handle chat messages with LLM integration — agentic loop with multi-step reasoning
    async fn handle_chat(
        &self,
        message: &str,
        user_context: &UserContext,
        scope: &AccessScope,
        session_id: Option<&str>,
    ) -> Result<TopsiResponse> {
        // If LLM is not configured, return a helpful message
        let Some(llm) = &self.llm else {
            let scope_description = match scope {
                AccessScope::Admin => "full platform access".to_string(),
                AccessScope::Projects(ids) => format!("access to {} projects", ids.len()),
                AccessScope::SingleProject(id) => format!("access to project {}", id),
                AccessScope::None => "no access".to_string(),
            };
            return Ok(TopsiResponse {
                message: format!(
                    "Topsi is running without LLM. You have {}. Your message: {}",
                    scope_description, message
                ),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            });
        };

        // Build context string based on access scope
        let context = self.build_context_for_scope(scope, user_context).await;

        // Get tool schemas in OpenAI format
        let tools = get_tool_schemas();

        // Load conversation history for this session
        let history = if let Some(sid) = session_id {
            let store = self.session_history.read().await;
            let msgs = store.get(sid).cloned().unwrap_or_default();
            tracing::debug!("[TOPSI] Loaded {} history messages for session {}", msgs.len(), sid);
            msgs
        } else {
            vec![]
        };

        // Store the user's message in history
        if let Some(sid) = session_id {
            let mut store = self.session_history.write().await;
            let entry = store.entry(sid.to_string()).or_insert_with(Vec::new);
            entry.push(ConversationMessage::user(message));
            // Keep history bounded — last 40 messages (20 turns)
            if entry.len() > 40 {
                let drain_count = entry.len() - 40;
                entry.drain(..drain_count);
            }
        }

        tracing::debug!(
            "[TOPSI] Sending chat to LLM: {} chars, {} tools, {} history msgs",
            message.len(),
            tools.len(),
            history.len()
        );

        // === AGENTIC LOOP ===
        let mut all_tool_calls: Vec<ToolCallResult> = vec![];
        let mut total_input_tokens: i64 = 0;
        let mut total_output_tokens: i64 = 0;
        let mut final_message: Option<String> = None;

        // Initial LLM call with conversation history
        let mut response = llm
            .generate_with_tools_and_history(
                TOPSI_SYSTEM_PROMPT,
                message,
                &context,
                &tools,
                &history,
            )
            .await
            .map_err(|e| TopsiError::LLMError(format!("LLM request failed: {}", e)))?;

        // Token budget guard: stop when cumulative tokens exceed this threshold.
        // At Claude Sonnet 4 pricing (~$3/M in + $15/M out), 200k tokens ≈ $3.60 worst case.
        // This replaces the old hard iteration cap — the agent can reason as long as it needs
        // but won't run away on cost if something goes wrong.
        const MAX_TOTAL_TOKENS: i64 = 200_000;

        let mut iteration: u32 = 0;
        loop {
            match response {
                LLMResponse::Text { content, usage } => {
                    // Final text answer — return to user
                    if let Some(u) = &usage {
                        total_input_tokens += u.input_tokens as i64;
                        total_output_tokens += u.output_tokens as i64;
                    }
                    tracing::info!(
                        "[TOPSI] LLM returned text response after {} iterations ({} chars, {}+{} tokens)",
                        iteration,
                        content.len(),
                        total_input_tokens,
                        total_output_tokens,
                    );
                    final_message = Some(content);
                    break;
                }
                LLMResponse::ToolCalls { calls, usage } => {
                    if let Some(u) = &usage {
                        total_input_tokens += u.input_tokens as i64;
                        total_output_tokens += u.output_tokens as i64;
                    }
                    tracing::info!(
                        "[TOPSI] Iteration {}: LLM requested {} tool calls ({}+{} tokens cumulative)",
                        iteration,
                        calls.len(),
                        total_input_tokens,
                        total_output_tokens,
                    );

                    // Execute each tool call
                    let tool_results_json = self
                        .execute_tool_calls(&calls, user_context, scope)
                        .await;

                    // Build ToolCallResult records and NoraToolResult for feedback
                    let mut nora_results: Vec<NoraToolResult> = vec![];
                    for (call, result_json) in calls.iter().zip(tool_results_json.iter()) {
                        let success = !result_json
                            .get("error")
                            .map(|e| !e.is_null())
                            .unwrap_or(false);

                        all_tool_calls.push(ToolCallResult {
                            tool_name: call.name.clone(),
                            arguments: call.arguments.clone(),
                            result: result_json.clone(),
                            success,
                        });

                        nora_results.push(NoraToolResult {
                            tool_call_id: call.id.clone(),
                            success,
                            result: serde_json::to_string(result_json).unwrap_or_default(),
                        });
                    }

                    // Check if respond_to_user was called — extract message and return
                    if let Some(msg) = Self::extract_respond_to_user(&calls, &tool_results_json) {
                        tracing::info!(
                            "[TOPSI] respond_to_user found at iteration {}, returning",
                            iteration
                        );
                        final_message = Some(msg);
                        break;
                    }

                    // Token budget check — stop before the next LLM call would blow budget
                    if total_input_tokens + total_output_tokens >= MAX_TOTAL_TOKENS {
                        tracing::warn!(
                            "[TOPSI] Token budget exhausted after {} iterations ({}+{} = {} tokens)",
                            iteration,
                            total_input_tokens,
                            total_output_tokens,
                            total_input_tokens + total_output_tokens,
                        );
                        break;
                    }

                    // Feed results back to LLM for next reasoning step
                    response = llm
                        .continue_with_tool_results_and_history(
                            TOPSI_SYSTEM_PROMPT,
                            message,
                            &context,
                            &calls,
                            &nora_results,
                            &history,
                            &tools,
                        )
                        .await
                        .map_err(|e| {
                            TopsiError::LLMError(format!(
                                "LLM continuation failed at iteration {}: {}",
                                iteration, e
                            ))
                        })?;
                }
            }
            iteration += 1;
        }

        // Resolve the final message
        let final_msg = final_message.unwrap_or_else(|| {
            // Token budget exhausted — return best partial result we have
            tracing::warn!("[TOPSI] Agentic loop ended without final message ({}+{} tokens used)", total_input_tokens, total_output_tokens);
            all_tool_calls
            .iter()
            .find(|r| r.tool_name == "respond_to_user" && r.success)
            .and_then(|r| r.result.get("response"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                "I've been working on your request but reached my reasoning limit. Here's what I've done so far — please let me know if you'd like me to continue.".to_string()
            })
        });

        // Save assistant response to session history
        if let Some(sid) = session_id {
            let mut store = self.session_history.write().await;
            let entry = store.entry(sid.to_string()).or_insert_with(Vec::new);
            entry.push(ConversationMessage::assistant(&final_msg));
        }

        Ok(TopsiResponse {
            message: final_msg,
            tool_calls: all_tool_calls,
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: Some(total_input_tokens),
            output_tokens: Some(total_output_tokens),
        })
    }

    /// Extract respond_to_user message from tool calls if present
    fn extract_respond_to_user(
        calls: &[nora::brain::ToolCall],
        results: &[serde_json::Value],
    ) -> Option<String> {
        for (call, result) in calls.iter().zip(results.iter()) {
            if call.name == "respond_to_user" {
                if let Some(response) = result.get("response").and_then(|v| v.as_str()) {
                    return Some(response.to_string());
                }
            }
        }
        None
    }

    /// Build context string for LLM based on user's access scope
    async fn build_context_for_scope(&self, scope: &AccessScope, user_context: &UserContext) -> String {
        let mut context_parts = vec![];

        // Add user context
        context_parts.push(format!(
            "User: {} ({})",
            user_context.email.as_deref().unwrap_or("unknown"),
            if user_context.is_admin { "admin" } else { "user" }
        ));

        // Add scope information and real data
        if let Some(pool) = &self.db {
            match scope {
                AccessScope::Admin => {
                    context_parts.push("Access Level: Full platform access (admin)".to_string());

                    // Get real project data
                    if let Ok(projects) = Project::find_all(pool).await {
                        context_parts.push(format!("\n## Projects in System ({})", projects.len()));
                        for project in projects.iter().take(10) {
                            context_parts.push(format!(
                                "- {} (ID: {}): {}",
                                project.name,
                                project.id,
                                project.git_repo_path.display()
                            ));
                        }
                        if projects.len() > 10 {
                            context_parts.push(format!("... and {} more projects", projects.len() - 10));
                        }
                    }

                    // Get agent data
                    if let Ok(agents) = Agent::find_all(pool).await {
                        context_parts.push(format!("\n## Registered Agents ({})", agents.len()));
                        for agent in agents.iter() {
                            context_parts.push(format!(
                                "- {} ({}): {} - Status: {:?}",
                                agent.short_name,
                                agent.designation,
                                agent.description.as_deref().unwrap_or("No description"),
                                agent.status
                            ));
                        }
                    }

                    // Get task counts
                    if let Ok(count) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks")
                        .fetch_one(pool)
                        .await
                    {
                        context_parts.push(format!("\n## Task Statistics"));
                        context_parts.push(format!("Total tasks: {}", count));

                        // Get task counts by status
                        if let Ok(todo_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'todo'"
                        ).fetch_one(pool).await {
                            context_parts.push(format!("Todo: {}", todo_count));
                        }
                        if let Ok(in_progress_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'"
                        ).fetch_one(pool).await {
                            context_parts.push(format!("In Progress: {}", in_progress_count));
                        }
                        if let Ok(done_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'done'"
                        ).fetch_one(pool).await {
                            context_parts.push(format!("Done: {}", done_count));
                        }
                    }
                }
                AccessScope::Projects(ids) => {
                    context_parts.push(format!(
                        "Access Level: Project access ({} projects)",
                        ids.len()
                    ));

                    // Get details for accessible projects
                    context_parts.push("\n## Accessible Projects".to_string());
                    for project_id in ids.iter().take(10) {
                        if let Ok(Some(project)) = Project::find_by_id(pool, *project_id).await {
                            context_parts.push(format!(
                                "- {} (ID: {}): {}",
                                project.name,
                                project.id,
                                project.git_repo_path.display()
                            ));
                        }
                    }
                }
                AccessScope::SingleProject(id) => {
                    context_parts.push(format!("Access Level: Single project access"));
                    if let Ok(Some(project)) = Project::find_by_id(pool, *id).await {
                        context_parts.push(format!(
                            "\n## Current Project: {} (ID: {})",
                            project.name, project.id
                        ));
                        context_parts.push(format!("Path: {}", project.git_repo_path.display()));
                    }
                }
                AccessScope::None => {
                    context_parts.push("Access Level: No project access".to_string());
                }
            }
        } else {
            context_parts.push("Warning: Database not connected - limited data available".to_string());
        }

        // Add system stats
        context_parts.push(format!("\nTopsi uptime: {}ms", self.uptime_ms()));

        context_parts.join("\n")
    }

    /// Execute tool calls requested by the LLM
    async fn execute_tool_calls(
        &self,
        calls: &[nora::brain::ToolCall],
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Vec<serde_json::Value> {
        let mut results = Vec::new();

        for call in calls {
            tracing::debug!(
                "[TOPSI] Executing tool: {} with args: {}",
                call.name,
                call.arguments
            );

            let result = match call.name.as_str() {
                "list_projects" => self.tool_list_projects(&call.arguments, scope).await,
                "create_project" => self.tool_create_project(&call.arguments, user_context).await,
                "update_project" => self.tool_update_project(&call.arguments, user_context).await,
                "list_organizations" => self.tool_list_organizations().await,
                "list_nodes" => self.tool_list_nodes(&call.arguments, scope).await,
                "list_edges" => self.tool_list_edges(&call.arguments, scope).await,
                "find_path" => self.tool_find_path(&call.arguments, scope).await,
                "detect_issues" => self.tool_detect_issues(&call.arguments, scope).await,
                "get_topology_summary" => self.tool_get_topology_summary(&call.arguments, scope).await,
                "create_cluster" => self.tool_create_cluster(&call.arguments, user_context, scope).await,
                "verify_access" => self.tool_verify_access(&call.arguments, scope).await,
                "create_task" => self.tool_create_task(&call.arguments, user_context, scope).await,
                "list_agents" => self.tool_list_agents(&call.arguments).await,
                "start_task_execution" => self.tool_start_task_execution(&call.arguments, user_context, scope).await,
                "get_task_status" => self.tool_get_task_status(&call.arguments, scope).await,
                "update_task" => self.tool_update_task(&call.arguments, user_context, scope).await,
                "list_tasks" => self.tool_list_tasks(&call.arguments, user_context, scope).await,
                "respond_to_user" => self.tool_respond_to_user(&call.arguments).await,
                _ => Err(TopsiError::ToolError(format!(
                    "Unknown tool: {}",
                    call.name
                ))),
            };

            results.push(match result {
                Ok(value) => value,
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            });
        }

        results
    }

    // ==================== Tool Implementations ====================

    /// List all accessible projects
    async fn tool_list_projects(
        &self,
        _args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "projects": []
            }));
        };

        let projects = match scope {
            AccessScope::Admin => {
                // Admin sees all projects
                Project::find_all(pool).await.map_err(|e| TopsiError::DatabaseError(e))?
            }
            AccessScope::Projects(ids) => {
                // User sees only their projects
                let mut projects = Vec::new();
                for id in ids {
                    if let Ok(Some(p)) = Project::find_by_id(pool, *id).await {
                        projects.push(p);
                    }
                }
                projects
            }
            AccessScope::SingleProject(id) => {
                if let Ok(Some(p)) = Project::find_by_id(pool, *id).await {
                    vec![p]
                } else {
                    vec![]
                }
            }
            AccessScope::None => vec![],
        };

        let project_list: Vec<serde_json::Value> = projects
            .iter()
            .map(|p| {
                serde_json::json!({
                    "id": p.id.to_string(),
                    "name": p.name,
                    "path": p.git_repo_path.display().to_string(),
                    "vibe_spent": p.vibe_spent_amount,
                    "vibe_budget": p.vibe_budget_limit,
                    "created_at": p.created_at.to_rfc3339()
                })
            })
            .collect();

        Ok(serde_json::json!({
            "projects": project_list,
            "total": projects.len()
        }))
    }

    /// List nodes (projects, agents, tasks) based on type filter
    async fn tool_list_nodes(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let node_type = args.get("node_type").and_then(|v| v.as_str());
        let _status = args.get("status").and_then(|v| v.as_str());

        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "nodes": []
            }));
        };

        let mut nodes = Vec::new();

        // Get nodes based on type filter
        match node_type {
            Some("project") | None => {
                // Get projects based on scope
                let projects = match scope {
                    AccessScope::Admin => Project::find_all(pool).await.unwrap_or_default(),
                    AccessScope::Projects(ids) => {
                        let mut ps = Vec::new();
                        for id in ids {
                            if let Ok(Some(p)) = Project::find_by_id(pool, *id).await {
                                ps.push(p);
                            }
                        }
                        ps
                    }
                    _ => vec![],
                };

                for p in projects {
                    nodes.push(serde_json::json!({
                        "type": "project",
                        "id": p.id.to_string(),
                        "name": p.name,
                        "status": "active"
                    }));
                }
            }
            _ => {}
        }

        match node_type {
            Some("agent") | None => {
                // Agents are visible to all (platform-wide)
                if let Ok(agents) = Agent::find_all(pool).await {
                    for a in agents {
                        nodes.push(serde_json::json!({
                            "type": "agent",
                            "id": a.id.to_string(),
                            "name": a.short_name,
                            "designation": a.designation,
                            "status": format!("{:?}", a.status).to_lowercase(),
                            "capabilities": a.capabilities
                        }));
                    }
                }
            }
            _ => {}
        }

        match node_type {
            Some("task") | None => {
                // Get tasks based on accessible projects
                let project_ids: Vec<Uuid> = match scope {
                    AccessScope::Admin => {
                        Project::find_all(pool).await.unwrap_or_default().iter().map(|p| p.id).collect()
                    }
                    AccessScope::Projects(ids) => ids.iter().copied().collect(),
                    AccessScope::SingleProject(id) => vec![*id],
                    AccessScope::None => vec![],
                };

                for project_id in project_ids.iter().take(5) {
                    // Query tasks for this project
                    let tasks: Vec<Task> = sqlx::query_as(
                        "SELECT * FROM tasks WHERE project_id = ? ORDER BY created_at DESC LIMIT 20"
                    )
                    .bind(project_id)
                    .fetch_all(pool)
                    .await
                    .unwrap_or_default();

                    for t in tasks {
                        nodes.push(serde_json::json!({
                            "type": "task",
                            "id": t.id.to_string(),
                            "title": t.title,
                            "project_id": t.project_id.to_string(),
                            "status": format!("{:?}", t.status).to_lowercase(),
                            "priority": format!("{:?}", t.priority).to_lowercase()
                        }));
                    }
                }
            }
            _ => {}
        }

        Ok(serde_json::json!({
            "nodes": nodes,
            "total": nodes.len(),
            "filter": {
                "node_type": node_type
            }
        }))
    }

    async fn tool_list_edges(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let edge_type = args.get("edge_type").and_then(|v| v.as_str());

        // For now return relationship types available
        Ok(serde_json::json!({
            "edges": [],
            "edge_types_available": ["depends_on", "assigned_to", "contains", "communicates_with"],
            "total": 0,
            "filter": {
                "edge_type": edge_type
            },
            "scope": format!("{:?}", scope),
            "note": "Edge traversal requires topology graph to be built"
        }))
    }

    async fn tool_find_path(
        &self,
        args: &serde_json::Value,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let from_node = args.get("from_node_id").and_then(|v| v.as_str());
        let to_node = args.get("to_node_id").and_then(|v| v.as_str());

        Ok(serde_json::json!({
            "path": [],
            "from": from_node,
            "to": to_node,
            "found": false,
            "message": "Path finding requires topology graph - use list_nodes to discover available nodes first"
        }))
    }

    async fn tool_detect_issues(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let issue_types = args
            .get("issue_types")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
            });

        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "issues": []
            }));
        };

        let mut issues = Vec::new();

        // Check for tasks stuck in progress
        if issue_types.as_ref().map_or(true, |t| t.contains(&"stale")) {
            let stale_tasks: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress' AND updated_at < datetime('now', '-7 days')"
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);

            if stale_tasks > 0 {
                issues.push(serde_json::json!({
                    "type": "stale",
                    "severity": "warning",
                    "description": format!("{} tasks have been in progress for over 7 days", stale_tasks),
                    "affected_count": stale_tasks
                }));
            }
        }

        // Check for unassigned high-priority tasks
        if issue_types.as_ref().map_or(true, |t| t.contains(&"bottleneck")) {
            let unassigned_high: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE (priority = 'critical' OR priority = 'high') AND assigned_agent IS NULL AND status = 'todo'"
            )
            .fetch_one(pool)
            .await
            .unwrap_or(0);

            if unassigned_high > 0 {
                issues.push(serde_json::json!({
                    "type": "bottleneck",
                    "severity": "high",
                    "description": format!("{} high/critical priority tasks are unassigned", unassigned_high),
                    "affected_count": unassigned_high,
                    "suggestion": "Consider assigning these tasks to available agents"
                }));
            }
        }

        Ok(serde_json::json!({
            "issues": issues,
            "total": issues.len(),
            "filter": {
                "issue_types": issue_types
            },
            "scope": format!("{:?}", scope)
        }))
    }

    async fn tool_get_topology_summary(
        &self,
        _args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected"
            }));
        };

        // Get real counts from database
        let project_count: i64 = match scope {
            AccessScope::Admin => {
                sqlx::query_scalar("SELECT COUNT(*) FROM projects")
                    .fetch_one(pool)
                    .await
                    .unwrap_or(0)
            }
            AccessScope::Projects(ids) => ids.len() as i64,
            AccessScope::SingleProject(_) => 1,
            AccessScope::None => 0,
        };

        let agent_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let task_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(pool)
            .await
            .unwrap_or(0);

        let active_task_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE status = 'in_progress'"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let todo_task_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE status = 'todo'"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let done_task_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE status = 'done'"
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        // Calculate health score (simple heuristic)
        let health_score = if task_count > 0 {
            let completion_rate = done_task_count as f64 / task_count as f64;
            let active_rate = active_task_count as f64 / task_count.max(1) as f64;
            (completion_rate * 0.5 + (1.0 - active_rate.min(0.5)) * 0.5).min(1.0)
        } else {
            1.0
        };

        Ok(serde_json::json!({
            "projects": project_count,
            "agents": agent_count,
            "total_tasks": task_count,
            "tasks_by_status": {
                "todo": todo_task_count,
                "in_progress": active_task_count,
                "done": done_task_count
            },
            "health_score": format!("{:.2}", health_score),
            "scope": format!("{:?}", scope)
        }))
    }

    async fn tool_create_cluster(
        &self,
        args: &serde_json::Value,
        _user_context: &UserContext,
        _scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("unnamed");
        let node_ids = args.get("node_ids").and_then(|v| v.as_array());

        Ok(serde_json::json!({
            "created": false,
            "cluster_name": name,
            "node_count": node_ids.map(|arr| arr.len()).unwrap_or(0),
            "message": "Cluster creation not yet implemented - clusters require topology graph"
        }))
    }

    /// Generate a conversational response to the user
    /// Create a new project for the user
    async fn tool_create_project(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Err(TopsiError::ToolError("Database not connected".to_string()));
        };

        // Extract arguments
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing project name".to_string()))?;

        let mut path = args.get("path")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                // Default path: ~/projects/<sanitized-name>
                let sanitized = name.to_lowercase().replace(" ", "-");
                format!("~/projects/{}", sanitized)
            });

        // Create the project using the Project model
        use db::models::project::{Project, CreateProject};

        // Check if path already exists and make it unique if needed
        let base_path = path.clone();
        let mut attempt = 0;
        while let Ok(Some(_)) = Project::find_by_git_repo_path(pool, &path).await {
            attempt += 1;
            // Append a suffix to make it unique
            let sanitized_name = name.to_lowercase().replace(" ", "-");
            path = format!("~/projects/{}-{}", sanitized_name, attempt);
            if attempt > 10 {
                return Err(TopsiError::ToolError(
                    format!("Could not find unique path after {} attempts", attempt)
                ));
            }
        }

        if path != base_path {
            tracing::info!("Original path {} was taken, using {} instead", base_path, path);
        }

        let project_id = uuid::Uuid::new_v4();
        let create_project = CreateProject {
            name: name.to_string(),
            git_repo_path: path.clone(),
            setup_script: None,
            dev_script: None,
            cleanup_script: None,
            copy_files: None,
            use_existing_repo: false,
            organization_id: None,
            client_id: None,
            folder_id: None,
        };

        let project = Project::create(pool, &create_project, project_id)
            .await
            .map_err(|e| TopsiError::ToolError(format!("Failed to create project: {}", e)))?;

        // Ensure default board exists so tasks have somewhere to land
        use db::models::project_board::ProjectBoard;
        if let Err(e) = ProjectBoard::ensure_default_board(pool, project.id).await {
            tracing::error!("Failed to create default board for project {}: {}", project.id, e);
        }

        // Add the creator as project owner in project_members
        // Parse user_id string to UUID to get proper 16-byte blob encoding
        let user_uuid = uuid::Uuid::parse_str(&user_context.user_id)
            .map_err(|e| TopsiError::ToolError(format!("Invalid user ID '{}': {}", user_context.user_id, e)))?;
        let member_id = uuid::Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO project_members (id, project_id, user_id, role, granted_by)
               VALUES (?, ?, ?, ?, ?)"#
        )
        .bind(member_id.as_bytes().to_vec())
        .bind(project.id.as_bytes().to_vec())
        .bind(user_uuid.as_bytes().to_vec())
        .bind("owner")
        .bind(user_uuid.as_bytes().to_vec()) // granted_by is the user themselves
        .execute(pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Failed to add project member: {}", e)))?;

        tracing::info!("Created project '{}' (ID: {}) for user {}",
            project.name, project.id, user_context.user_id);

        Ok(serde_json::json!({
            "success": true,
            "project_id": project.id.to_string(),
            "name": project.name,
            "path": project.git_repo_path.display().to_string(),
            "message": format!("Project '{}' created successfully at {}",
                project.name,
                project.git_repo_path.display()
            )
        }))
    }

    /// List all organizations
    async fn tool_list_organizations(&self) -> std::result::Result<serde_json::Value, TopsiError> {
        let pool = self.db.as_ref().ok_or_else(|| TopsiError::ToolError("DB not available".into()))?;
        let rows = sqlx::query!(
            r#"SELECT hex(id) as id, name, slug, description FROM organizations WHERE deleted_at IS NULL ORDER BY name"#
        )
        .fetch_all(pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Failed to list organizations: {}", e)))?;

        let orgs: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({
            "id": r.id,
            "name": r.name,
            "slug": r.slug,
            "description": r.description
        })).collect();

        Ok(serde_json::json!({ "organizations": orgs, "count": orgs.len() }))
    }

    /// Update project metadata (name, organization assignment)
    async fn tool_update_project(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
    ) -> std::result::Result<serde_json::Value, TopsiError> {
        let pool = self.db.as_ref().ok_or_else(|| TopsiError::ToolError("DB not available".into()))?;

        let project_id_str = args["project_id"].as_str()
            .ok_or_else(|| TopsiError::ToolError("project_id required".into()))?;
        let project_uuid = uuid::Uuid::parse_str(project_id_str)
            .map_err(|e| TopsiError::ToolError(format!("Invalid project_id: {}", e)))?;

        // Verify user has access to this project
        let member_check = sqlx::query!(
            r#"SELECT role FROM project_members WHERE project_id = ? AND user_id = ?"#,
            project_uuid,
            user_context.user_id
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Access check failed: {}", e)))?;

        if member_check.is_none() && !user_context.is_admin {
            return Err(TopsiError::ToolError("Access denied: not a member of this project".into()));
        }

        // Build update dynamically based on provided fields
        let new_name = args["name"].as_str();
        let new_org_id = args["organization_id"].as_str();

        if new_name.is_none() && new_org_id.is_none() {
            return Err(TopsiError::ToolError("Provide at least one field to update: name or organization_id".into()));
        }

        // Parse org_id from hex string (Topsi returns hex from list_organizations)
        let org_uuid: Option<uuid::Uuid> = if let Some(org_str) = new_org_id {
            // Accept UUID format (with dashes) or raw hex string (32 chars, no dashes)
            if let Ok(u) = uuid::Uuid::parse_str(org_str) {
                Some(u)
            } else if org_str.len() == 32 {
                // Raw hex without dashes — insert dashes and parse
                let with_dashes = format!("{}-{}-{}-{}-{}",
                    &org_str[0..8], &org_str[8..12], &org_str[12..16],
                    &org_str[16..20], &org_str[20..32]);
                Some(uuid::Uuid::parse_str(&with_dashes)
                    .map_err(|_| TopsiError::ToolError(format!("Invalid organization_id: {}", org_str)))?)
            } else {
                return Err(TopsiError::ToolError(format!("organization_id must be a UUID or 32-char hex string, got: {}", org_str)));
            }
        } else {
            None
        };

        sqlx::query(r#"
            UPDATE projects
            SET name = COALESCE(?, name),
                organization_id = CASE WHEN ? = 1 THEN ? ELSE organization_id END,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?
        "#)
        .bind(new_name)
        .bind(org_uuid.is_some() as i32)
        .bind(org_uuid.map(|u| u.as_bytes().to_vec()))
        .bind(project_uuid.as_bytes().to_vec())
        .execute(pool)
        .await
        .map_err(|e| TopsiError::ToolError(format!("Failed to update project: {}", e)))?;

        tracing::info!("Updated project {} — name={:?}, org={:?}", project_id_str, new_name, new_org_id);

        Ok(serde_json::json!({
            "success": true,
            "project_id": project_id_str,
            "updated_name": new_name,
            "updated_organization_id": new_org_id,
            "message": "Project updated successfully"
        }))
    }

    /// Create a task in a project
    async fn tool_create_task(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Err(TopsiError::ToolError("Database not connected".to_string()));
        };

        // Extract or infer project_id
        let project_id = if let Some(pid_str) = args.get("project_id").and_then(|v| v.as_str()) {
            // Check if it's a placeholder string (LLMs sometimes use these)
            if pid_str.contains("<") || pid_str.contains(">") || pid_str == "null" || pid_str.is_empty() {
                // Treat as missing, will infer below
                None
            } else {
                // Try to parse as UUID
                uuid::Uuid::parse_str(pid_str).ok()
            }
        } else {
            None
        };

        let project_id = if let Some(pid) = project_id {
            pid
        } else {
            // No project_id provided - try to infer from user's accessible projects
            // IMPORTANT: Re-fetch from database to get freshly created projects
            use db::models::project::Project;

            let projects = if user_context.is_admin {
                // Admins can see all projects
                Project::find_all(pool).await.unwrap_or_default()
            } else {
                // Regular users - fetch their projects from project_members table
                // Parse user_id string to UUID for proper 16-byte blob encoding
                let user_uuid = uuid::Uuid::parse_str(&user_context.user_id).unwrap_or_default();
                let user_id_bytes = user_uuid.as_bytes().to_vec();
                let project_ids: Vec<String> = sqlx::query_scalar(
                    r#"SELECT DISTINCT project_id FROM project_members WHERE user_id = ?"#
                )
                .bind(&user_id_bytes)
                .fetch_all(pool)
                .await
                .unwrap_or_default();

                let mut projects = Vec::new();
                for pid_str in project_ids {
                    if let Ok(pid) = uuid::Uuid::parse_str(&pid_str) {
                        if let Ok(Some(p)) = Project::find_by_id(pool, pid).await {
                            projects.push(p);
                        }
                    }
                }
                projects
            };

            if projects.is_empty() {
                // No projects exist - auto-create one based on the task
                // Infer project name from task title
                let task_title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled Task");
                let project_name = if task_title.len() > 30 {
                    format!("{} Project", &task_title[..30])
                } else {
                    format!("{} Project", task_title)
                };

                use db::models::project::{Project, CreateProject};

                // Generate unique path for auto-created project
                let sanitized = project_name.to_lowercase().replace(" ", "-");
                let mut path = format!("~/projects/{}", sanitized);
                let mut attempt = 0;
                while let Ok(Some(_)) = Project::find_by_git_repo_path(pool, &path).await {
                    attempt += 1;
                    path = format!("~/projects/{}-{}", sanitized, attempt);
                    if attempt > 10 {
                        return Err(TopsiError::ToolError(
                            "Could not find unique path for auto-created project".to_string()
                        ));
                    }
                }

                let create_project = CreateProject {
                    name: project_name.clone(),
                    git_repo_path: path.clone(),
                    setup_script: None,
                    dev_script: None,
                    cleanup_script: None,
                    copy_files: None,
                    use_existing_repo: false,
                    organization_id: None,
                    client_id: None,
                    folder_id: None,
                };

                let new_project_id = uuid::Uuid::new_v4();
                match Project::create(pool, &create_project, new_project_id).await {
                    Ok(project) => {
                        // Add the creator as project owner in project_members
                        // Parse user_id string to UUID for proper 16-byte blob encoding
                        let auto_user_uuid = uuid::Uuid::parse_str(&user_context.user_id)
                            .map_err(|e| TopsiError::ToolError(format!("Invalid user ID: {}", e)))?;
                        let member_id = uuid::Uuid::new_v4();
                        if let Err(e) = sqlx::query(
                            r#"INSERT INTO project_members (id, project_id, user_id, role, granted_by)
                               VALUES (?, ?, ?, ?, ?)"#
                        )
                        .bind(member_id.as_bytes().to_vec())
                        .bind(project.id.as_bytes().to_vec())
                        .bind(auto_user_uuid.as_bytes().to_vec())
                        .bind("owner")
                        .bind(auto_user_uuid.as_bytes().to_vec())
                        .execute(pool)
                        .await {
                            tracing::error!("Failed to add project member for auto-created project: {}", e);
                            return Err(TopsiError::ToolError(
                                format!("Project created but failed to grant access: {}", e)
                            ));
                        }

                        tracing::info!("Auto-created project '{}' (ID: {}) for task '{}' by user {}",
                            project.name, project.id, task_title, user_context.user_id);
                        project.id
                    }
                    Err(e) => {
                        return Err(TopsiError::ToolError(
                            format!("No projects found and failed to auto-create project '{}': {}", project_name, e)
                        ));
                    }
                }
            } else if projects.len() == 1 {
                // Only one project - use it automatically
                projects[0].id
            } else {
                // Multiple projects - return helpful context for Topsi to decide
                let project_list: Vec<String> = projects.iter()
                    .map(|p| format!("  - {} (ID: {})", p.name, p.id))
                    .collect();

                return Err(TopsiError::ToolError(
                    format!(
                        "Multiple projects available. Please analyze which project this task belongs to and call create_task again with project_id, or create a new project if this is a new idea:\n\nAvailable projects:\n{}",
                        project_list.join("\n")
                    )
                ));
            }
        };

        let title = args.get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing title".to_string()))?;

        let description = args.get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing description".to_string()))?;

        let agent_name = args.get("agent_name").and_then(|v| v.as_str());

        // Verify access to project
        match scope {
            AccessScope::Admin => {}, // Admin can create in any project
            AccessScope::Projects(ids) => {
                if !ids.contains(&project_id) {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this project".to_string()
                    ));
                }
            },
            AccessScope::SingleProject(id) => {
                if *id != project_id {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this project".to_string()
                    ));
                }
            },
            AccessScope::None => {
                return Err(TopsiError::ToolError(
                    "You don't have permission to create tasks".to_string()
                ));
            }
        }

        // Create the task using the Task model
        use db::models::task::{Task, CreateTask};
        use db::models::project_board::ProjectBoard;

        // Look up the default board so the task appears on a board in the UI
        let default_board_id = ProjectBoard::ensure_default_board(pool, project_id)
            .await
            .ok()
            .map(|b| b.id);

        let create_task = CreateTask {
            project_id,
            pod_id: None,
            board_id: default_board_id,
            title: title.to_string(),
            description: Some(description.to_string()),
            parent_task_attempt: None,
            image_ids: None,
            priority: None,
            assignee_id: None,
            assigned_agent: agent_name.map(|s| s.to_string()),
            agent_id: None,
            assigned_mcps: None,
            created_by: user_context.user_id.clone(),
            requires_approval: None,
            parent_task_id: None,
            tags: None,
            due_date: None,
            custom_properties: None,
            scheduled_start: None,
            scheduled_end: None,
        };

        let task_id = uuid::Uuid::new_v4();
        let task = Task::create(pool, &create_task, task_id)
            .await
            .map_err(|e| TopsiError::ToolError(format!("Failed to create task: {}", e)))?;

        // Auto-execute: if an agent was assigned, automatically start task execution
        let auto_execute = args.get("auto_execute")
            .and_then(|v| v.as_bool())
            .unwrap_or(true); // Default to true when agent_name is present

        let execution_result = if agent_name.is_some() && auto_execute {
            if let Some(bridge) = &self.execution_bridge {
                let agent_lower = agent_name.unwrap().to_lowercase();
                let executor_name = match agent_lower.as_str() {
                    "claude" | "claude_code" => "CLAUDE_CODE",
                    "gemini" => "GEMINI",
                    "amp" => "AMP",
                    "codex" | "openai" => "CODEX",
                    _ => agent_name.unwrap(),
                };
                let base_branch = "main".to_string();

                tracing::info!(
                    "[TOPSI] Auto-executing task {} with agent {} (executor: {})",
                    task.id, agent_name.unwrap(), executor_name
                );

                match bridge.start_task_attempt(task.id, executor_name, &base_branch).await {
                    Ok(result) => {
                        tracing::info!("[TOPSI] Auto-execution started for task {}", task.id);
                        Some(result)
                    }
                    Err(e) => {
                        tracing::error!("[TOPSI] Auto-execution failed for task {}: {}", task.id, e);
                        Some(serde_json::json!({ "error": format!("Task created but execution failed: {}", e) }))
                    }
                }
            } else {
                Some(serde_json::json!({ "note": "Task created but execution bridge not available" }))
            }
        } else {
            None
        };

        let mut response = serde_json::json!({
            "success": true,
            "task_id": task.id.to_string(),
            "title": task.title,
            "status": format!("{:?}", task.status),
            "assigned_agent": task.assigned_agent,
            "project_id": task.project_id.to_string(),
            "message": format!("Task '{}' created successfully{}",
                task.title,
                agent_name.map(|a| format!(" and assigned to {}", a)).unwrap_or_default()
            )
        });

        if let Some(exec) = execution_result {
            response.as_object_mut().unwrap().insert("execution".to_string(), exec);
        }

        Ok(response)
    }

    /// List all available agents
    async fn tool_list_agents(
        &self,
        _args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "agents": []
            }));
        };

        // Get registered agents from the agent registry
        use services::services::agent_registry::AgentRegistryService;

        let agents = AgentRegistryService::get_active_agents(pool)
            .await
            .unwrap_or_default();

        let agent_list: Vec<serde_json::Value> = agents
            .iter()
            .map(|agent| {
                serde_json::json!({
                    "id": agent.id.to_string(),
                    "name": agent.short_name,
                    "designation": agent.designation,
                    "description": agent.description,
                    "capabilities": agent.capabilities,
                    "status": format!("{:?}", agent.status)
                })
            })
            .collect();

        Ok(serde_json::json!({
            "agents": agent_list,
            "total": agents.len()
        }))
    }

    /// Start executing a task by spawning a coding agent
    async fn tool_start_task_execution(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Err(TopsiError::ToolError("Database not connected".to_string()));
        };

        let Some(bridge) = &self.execution_bridge else {
            return Err(TopsiError::ToolError(
                "Task execution not available — execution bridge not configured".to_string(),
            ));
        };

        let task_id_str = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing task_id".to_string()))?;

        let task_id = Uuid::parse_str(task_id_str)
            .map_err(|e| TopsiError::ToolError(format!("Invalid task_id '{}': {}", task_id_str, e)))?;

        let agent_name = args
            .get("agent_name")
            .and_then(|v| v.as_str())
            .unwrap_or("claude");

        let _additional_prompt = args
            .get("additional_prompt")
            .and_then(|v| v.as_str());

        // Verify the task exists and user has access
        let task: Option<Task> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| TopsiError::DatabaseError(e))?;

        let task = task.ok_or_else(|| {
            TopsiError::ToolError(format!("Task {} not found", task_id))
        })?;

        // Verify project access
        match scope {
            AccessScope::Admin => {}
            AccessScope::Projects(ids) => {
                if !ids.contains(&task.project_id) {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::SingleProject(id) => {
                if *id != task.project_id {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::None => {
                return Err(TopsiError::ToolError(
                    "You don't have permission to execute tasks".to_string(),
                ));
            }
        }

        // Get the project to determine base branch
        let project = Project::find_by_id(pool, task.project_id)
            .await
            .map_err(|e| TopsiError::DatabaseError(e))?
            .ok_or_else(|| {
                TopsiError::ToolError(format!("Project {} not found", task.project_id))
            })?;

        // Use "main" as default base branch
        let base_branch = "main".to_string();

        // Map agent names to executor names
        let agent_lower = agent_name.to_lowercase();
        let executor_name = match agent_lower.as_str() {
            "claude" | "claude_code" => "CLAUDE_CODE",
            "gemini" => "GEMINI",
            "amp" => "AMP",
            "codex" | "openai" => "CODEX",
            _ => agent_name,
        };

        tracing::info!(
            "[TOPSI] Starting task execution: task={}, agent={}, executor={}",
            task_id,
            agent_name,
            executor_name
        );

        // Delegate to the execution bridge
        match bridge.start_task_attempt(task_id, executor_name, &base_branch).await {
            Ok(result) => {
                tracing::info!("[TOPSI] Task execution started successfully for task {}", task_id);
                Ok(result)
            }
            Err(e) => {
                tracing::error!("[TOPSI] Failed to start task execution: {}", e);
                Err(TopsiError::ToolError(format!(
                    "Failed to start task execution: {}",
                    e
                )))
            }
        }
    }

    /// Get the current status of a task including execution info and logs
    async fn tool_get_task_status(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Err(TopsiError::ToolError("Database not connected".to_string()));
        };

        let task_id_str = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing task_id".to_string()))?;

        let task_id = Uuid::parse_str(task_id_str)
            .map_err(|e| TopsiError::ToolError(format!("Invalid task_id: {}", e)))?;

        let include_logs = args
            .get("include_logs")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let log_lines = args
            .get("log_lines")
            .and_then(|v| v.as_i64())
            .unwrap_or(20) as i32;

        // Fetch the task
        let task: Option<Task> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| TopsiError::DatabaseError(e))?;

        let task = task.ok_or_else(|| {
            TopsiError::ToolError(format!("Task {} not found", task_id))
        })?;

        // Build task info
        let mut result = serde_json::json!({
            "task_id": task.id.to_string(),
            "title": task.title,
            "description": task.description,
            "status": format!("{:?}", task.status).to_lowercase(),
            "priority": format!("{:?}", task.priority).to_lowercase(),
            "assigned_agent": task.assigned_agent,
            "created_at": task.created_at.to_rfc3339(),
            "updated_at": task.updated_at.to_rfc3339(),
        });

        // Fetch latest task attempt
        let latest_attempt: Option<(String, String, String, String, String)> = sqlx::query_as(
            "SELECT id, executor, base_branch, created_at, updated_at FROM task_attempts WHERE task_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(task_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((attempt_id, executor, base_branch, attempt_created, attempt_updated)) =
            latest_attempt
        {
            result["latest_attempt"] = serde_json::json!({
                "attempt_id": attempt_id,
                "executor": executor,
                "base_branch": base_branch,
                "created_at": attempt_created,
                "updated_at": attempt_updated,
            });

            // Fetch latest execution process for this attempt
            let latest_process: Option<(String, String, Option<i32>, String, String)> = sqlx::query_as(
                "SELECT id, status, exit_code, created_at, updated_at FROM execution_processes WHERE task_attempt_id = ? ORDER BY created_at DESC LIMIT 1"
            )
            .bind(&attempt_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some((proc_id, proc_status, exit_code, proc_created, proc_updated)) =
                latest_process
            {
                result["latest_process"] = serde_json::json!({
                    "process_id": proc_id,
                    "status": proc_status,
                    "exit_code": exit_code,
                    "created_at": proc_created,
                    "updated_at": proc_updated,
                });

                // Fetch recent logs if requested
                if include_logs {
                    let logs: Vec<(String,)> = sqlx::query_as(
                        "SELECT logs FROM execution_process_logs WHERE execution_id = ? ORDER BY inserted_at DESC LIMIT ?"
                    )
                    .bind(&proc_id)
                    .bind(log_lines)
                    .fetch_all(pool)
                    .await
                    .unwrap_or_default();

                    if !logs.is_empty() {
                        let log_text: Vec<&str> = logs.iter().map(|(l,)| l.as_str()).collect();
                        result["recent_logs"] = serde_json::json!(log_text);
                    }
                }
            }
        } else {
            result["latest_attempt"] = serde_json::json!(null);
            result["note"] = serde_json::json!("No execution attempts yet");
        }

        Ok(result)
    }

    /// Update a task's properties
    async fn tool_update_task(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Err(TopsiError::ToolError("Database not connected".to_string()));
        };

        let task_id_str = args
            .get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TopsiError::ToolError("Missing task_id".to_string()))?;

        let task_id = Uuid::parse_str(task_id_str)
            .map_err(|e| TopsiError::ToolError(format!("Invalid task_id: {}", e)))?;

        // Fetch the task to verify it exists and check access
        let task: Option<Task> = sqlx::query_as("SELECT * FROM tasks WHERE id = ?")
            .bind(task_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| TopsiError::DatabaseError(e))?;

        let task = task.ok_or_else(|| {
            TopsiError::ToolError(format!("Task {} not found", task_id))
        })?;

        // Verify project access
        match scope {
            AccessScope::Admin => {}
            AccessScope::Projects(ids) => {
                if !ids.contains(&task.project_id) {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::SingleProject(id) => {
                if *id != task.project_id {
                    return Err(TopsiError::ToolError(
                        "You don't have access to this task's project".to_string(),
                    ));
                }
            }
            AccessScope::None => {
                return Err(TopsiError::ToolError(
                    "You don't have permission to update tasks".to_string(),
                ));
            }
        }

        // Build dynamic UPDATE query
        let mut updates = vec![];
        let mut values: Vec<String> = vec![];

        if let Some(status) = args.get("status").and_then(|v| v.as_str()) {
            updates.push("status = ?");
            values.push(status.to_string());
        }
        if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
            updates.push("title = ?");
            values.push(title.to_string());
        }
        if let Some(description) = args.get("description").and_then(|v| v.as_str()) {
            updates.push("description = ?");
            values.push(description.to_string());
        }
        if let Some(priority) = args.get("priority").and_then(|v| v.as_str()) {
            updates.push("priority = ?");
            values.push(priority.to_string());
        }
        if let Some(agent) = args.get("assigned_agent").and_then(|v| v.as_str()) {
            updates.push("assigned_agent = ?");
            values.push(agent.to_string());
        }

        if updates.is_empty() {
            return Ok(serde_json::json!({
                "success": false,
                "message": "No fields to update. Provide at least one of: status, title, description, priority, assigned_agent"
            }));
        }

        updates.push("updated_at = datetime('now')");

        let query = format!(
            "UPDATE tasks SET {} WHERE id = ?",
            updates.join(", ")
        );

        let mut q = sqlx::query(&query);
        for val in &values {
            q = q.bind(val);
        }
        q = q.bind(task_id);

        q.execute(pool)
            .await
            .map_err(|e| TopsiError::ToolError(format!("Failed to update task: {}", e)))?;

        tracing::info!("[TOPSI] Updated task {} with {} field changes", task_id, values.len());

        Ok(serde_json::json!({
            "success": true,
            "task_id": task_id.to_string(),
            "updated_fields": values.len(),
            "message": format!("Task '{}' updated successfully", task.title)
        }))
    }

    /// List tasks with filtering by status, agent, and project
    async fn tool_list_tasks(
        &self,
        args: &serde_json::Value,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Err(TopsiError::ToolError("Database not connected".to_string()));
        };

        let status_filter = args.get("status").and_then(|v| v.as_str());
        let agent_filter = args.get("assigned_agent").and_then(|v| v.as_str());
        let limit = args
            .get("limit")
            .and_then(|v| v.as_i64())
            .unwrap_or(20) as i32;

        // Determine which project IDs to query
        let project_id_filter = args.get("project_id").and_then(|v| v.as_str());

        let project_ids: Vec<Uuid> = if let Some(pid_str) = project_id_filter {
            if let Ok(pid) = Uuid::parse_str(pid_str) {
                vec![pid]
            } else {
                return Err(TopsiError::ToolError(format!("Invalid project_id: {}", pid_str)));
            }
        } else {
            // Infer from scope
            match scope {
                AccessScope::Admin => {
                    Project::find_all(pool)
                        .await
                        .unwrap_or_default()
                        .iter()
                        .map(|p| p.id)
                        .collect()
                }
                AccessScope::Projects(ids) => ids.iter().copied().collect(),
                AccessScope::SingleProject(id) => vec![*id],
                AccessScope::None => vec![],
            }
        };

        let mut all_tasks: Vec<serde_json::Value> = vec![];

        for pid in &project_ids {
            // Build query with filters
            let mut query = String::from(
                "SELECT id, title, status, priority, assigned_agent, created_at, updated_at FROM tasks WHERE project_id = ?"
            );
            let mut bind_values: Vec<String> = vec![pid.to_string()];

            if let Some(status) = status_filter {
                query.push_str(" AND status = ?");
                bind_values.push(status.to_string());
            }
            if let Some(agent) = agent_filter {
                query.push_str(" AND assigned_agent = ?");
                bind_values.push(agent.to_string());
            }

            query.push_str(" ORDER BY created_at DESC LIMIT ?");

            let mut q = sqlx::query_as::<_, (String, String, String, String, Option<String>, String, String)>(&query);
            for val in &bind_values {
                q = q.bind(val);
            }
            q = q.bind(limit);

            let tasks: Vec<(String, String, String, String, Option<String>, String, String)> = q
                .fetch_all(pool)
                .await
                .unwrap_or_default();

            for (id, title, status, priority, agent, created, updated) in tasks {
                all_tasks.push(serde_json::json!({
                    "id": id,
                    "title": title,
                    "status": status,
                    "priority": priority,
                    "assigned_agent": agent,
                    "project_id": pid.to_string(),
                    "created_at": created,
                    "updated_at": updated,
                }));
            }
        }

        Ok(serde_json::json!({
            "tasks": all_tasks,
            "total": all_tasks.len(),
            "filters": {
                "status": status_filter,
                "assigned_agent": agent_filter,
                "limit": limit,
            }
        }))
    }

    async fn tool_respond_to_user(
        &self,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let message = args.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("I'm Topsi, your topological super intelligence. How can I help you today?");

        Ok(serde_json::json!({
            "response": message,
            "spoken": true
        }))
    }

    async fn tool_verify_access(
        &self,
        args: &serde_json::Value,
        scope: &AccessScope,
    ) -> Result<serde_json::Value> {
        let user_id = args.get("user_id").and_then(|v| v.as_str());
        let resource_type = args.get("resource_type").and_then(|v| v.as_str());
        let resource_id = args.get("resource_id").and_then(|v| v.as_str());
        let action = args.get("action").and_then(|v| v.as_str());

        // Basic access check based on current scope
        let allowed = match scope {
            AccessScope::Admin => true,
            AccessScope::Projects(ids) => {
                if resource_type == Some("project") {
                    if let Some(rid) = resource_id {
                        if let Ok(uuid) = Uuid::parse_str(rid) {
                            ids.contains(&uuid)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    true // Allow other resources by default
                }
            }
            AccessScope::SingleProject(id) => {
                if resource_type == Some("project") {
                    resource_id == Some(&id.to_string())
                } else {
                    true
                }
            }
            AccessScope::None => false,
        };

        Ok(serde_json::json!({
            "user_id": user_id,
            "resource_type": resource_type,
            "resource_id": resource_id,
            "action": action,
            "allowed": allowed,
            "scope": format!("{:?}", scope)
        }))
    }

    /// Handle topology requests
    async fn handle_get_topology(
        &self,
        project_id: Option<Uuid>,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        // Verify access to the project
        if let Some(pid) = project_id {
            if !self.access_control.can_access_project(user_context, pid).await {
                return Err(TopsiError::TopologyError(
                    "Access denied to project".to_string(),
                ));
            }
        }

        // Get topology summary
        let summary = self.get_topology_summary(project_id, scope).await?;

        Ok(TopsiResponse {
            message: "Topology retrieved successfully".to_string(),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: Some(summary),
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle issue detection
    async fn handle_detect_issues(
        &self,
        project_id: Option<Uuid>,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        // Verify access
        if let Some(pid) = project_id {
            if !self.access_control.can_access_project(user_context, pid).await {
                return Err(TopsiError::TopologyError(
                    "Access denied to project".to_string(),
                ));
            }
        }

        // Detect issues (stub for now)
        let issues = self.detect_issues(project_id, scope).await?;

        Ok(TopsiResponse {
            message: format!("Detected {} issues", issues.len()),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues,
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle listing projects
    async fn handle_list_projects(
        &self,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        let project_count = match scope {
            AccessScope::Admin => {
                // Get all projects
                if let Some(pool) = &self.db {
                    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
                        .fetch_one(pool)
                        .await
                        .unwrap_or(0);
                    count as usize
                } else {
                    0
                }
            }
            AccessScope::Projects(ids) => ids.len(),
            AccessScope::SingleProject(_) => 1,
            AccessScope::None => 0,
        };

        Ok(TopsiResponse {
            message: format!("You have access to {} projects", project_count),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle command execution
    async fn handle_command(
        &self,
        command: &str,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        // Parse and execute command
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Ok(TopsiResponse {
                message: "No command provided".to_string(),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            });
        }

        match parts[0].to_lowercase().as_str() {
            "status" => {
                let is_admin = matches!(scope, AccessScope::Admin);
                Ok(TopsiResponse {
                    message: format!(
                        "Topsi Status:\n- Active: {}\n- Uptime: {}ms\n- Admin: {}",
                        self.is_active().await,
                        self.uptime_ms(),
                        is_admin
                    ),
                    tool_calls: vec![],
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: None,
                    output_tokens: None,
                })
            }
            "help" => {
                Ok(TopsiResponse {
                    message: "Available commands: status, help, topology, issues".to_string(),
                    tool_calls: vec![],
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: None,
                    output_tokens: None,
                })
            }
            _ => {
                Ok(TopsiResponse {
                    message: format!("Unknown command: {}", parts[0]),
                    tool_calls: vec![],
                    topology_changes: vec![],
                    topology_summary: None,
                    issues: vec![],
                    input_tokens: None,
                    output_tokens: None,
                })
            }
        }
    }

    /// Get topology summary for accessible projects
    async fn get_topology_summary(
        &self,
        project_id: Option<Uuid>,
        scope: &AccessScope,
    ) -> Result<TopologySummary> {
        // Return a basic summary for now
        // TODO: Integrate with actual topology data from database
        Ok(TopologySummary {
            node_count: 0,
            edge_count: 0,
            cluster_count: 0,
            active_routes: 0,
            unresolved_issues: 0,
            nodes_by_type: vec![],
            edges_by_type: vec![],
            health_score: 1.0,
        })
    }

    /// Detect issues in the topology
    async fn detect_issues(
        &self,
        project_id: Option<Uuid>,
        scope: &AccessScope,
    ) -> Result<Vec<DetectedIssue>> {
        // Return empty for now
        // TODO: Implement actual issue detection
        Ok(vec![])
    }

    /// Handle recommendations request — runs PriorityRecommender over project tasks
    async fn handle_get_recommendations(
        &self,
        project_id: Option<Uuid>,
        max_count: Option<usize>,
        user_context: &UserContext,
        scope: &AccessScope,
    ) -> Result<TopsiResponse> {
        use crate::prioritization::free_energy::{IntoPotentialAction, PotentialAction};
        use crate::prioritization::goals::{Goal, GoalType};
        use crate::prioritization::recommender::PriorityRecommender;

        // Verify access
        if let Some(pid) = project_id {
            if !self.access_control.can_access_project(user_context, pid).await {
                return Err(TopsiError::TopologyError("Access denied to project".to_string()));
            }
        }

        let Some(pool) = &self.db else {
            return Ok(TopsiResponse {
                message: "Database not connected — cannot generate recommendations".to_string(),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            });
        };

        // Determine which project IDs to query
        let project_ids: Vec<Uuid> = match project_id {
            Some(pid) => vec![pid],
            None => match scope {
                AccessScope::Admin => {
                    Project::find_all(pool)
                        .await
                        .unwrap_or_default()
                        .iter()
                        .map(|p| p.id)
                        .collect()
                }
                AccessScope::Projects(ids) => ids.iter().copied().collect(),
                AccessScope::SingleProject(id) => vec![*id],
                AccessScope::None => vec![],
            },
        };

        // Fetch open tasks across those projects
        let mut all_tasks: Vec<Task> = Vec::new();
        for pid in &project_ids {
            let tasks: Vec<Task> = sqlx::query_as(
                "SELECT * FROM tasks WHERE project_id = ? AND status IN ('todo', 'in_progress') ORDER BY created_at DESC LIMIT 50",
            )
            .bind(pid)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
            all_tasks.extend(tasks);
        }

        if all_tasks.is_empty() {
            return Ok(TopsiResponse {
                message: "No open tasks found for recommendations".to_string(),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            });
        }

        // Convert tasks to PotentialActions
        let actions: Vec<PotentialAction> = all_tasks.iter().map(|t| t.into_action()).collect();

        // Create a synthetic goal for general project progress
        let goal = Goal::new(
            "Project Progress",
            "Complete open tasks and move projects forward",
            GoalType::Custom,
            0.8,
        );

        // Run the recommender
        let max = max_count.unwrap_or(5);
        let recommender = PriorityRecommender::new().with_max_recommendations(max);
        let batch = recommender.recommend(&actions, &[goal]);

        // Serialize the batch as the response message (JSON)
        let batch_json = serde_json::to_string_pretty(&batch)
            .unwrap_or_else(|_| "Failed to serialize recommendations".to_string());

        Ok(TopsiResponse {
            message: batch_json,
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    // ========================================================================
    // Meeting Mode Handlers
    // ========================================================================

    /// Handle starting a new meeting session
    async fn handle_start_meeting(
        &self,
        project_id: &str,
        title: Option<&str>,
        user_context: &UserContext,
    ) -> Result<TopsiResponse> {
        let pool = self.db.as_ref().ok_or_else(|| {
            TopsiError::NotInitialized("Database not connected".to_string())
        })?;

        // Create DB record
        let session = db::models::meeting_session::MeetingSession::create(
            pool,
            db::models::meeting_session::CreateMeetingSession {
                project_id: project_id.to_string(),
                title: title.map(|t| t.to_string()),
                started_by: user_context.user_id.clone(),
            },
        )
        .await
        .map_err(|e| TopsiError::TopologyError(format!("Failed to create meeting session: {}", e)))?;

        // Track in-memory state
        self.meeting_manager
            .start_meeting(
                session.id.clone(),
                project_id.to_string(),
                user_context.user_id.clone(),
            )
            .await;

        // Create topology node for this meeting
        let topology_node_id = if let Ok(project_uuid) = Uuid::parse_str(project_id) {
            let mut topologies = self.topologies.write().await;
            let graph = topologies
                .entry(project_uuid)
                .or_insert_with(TopologyGraph::new);

            let meeting_uuid = VoiceTopology::add_meeting_to_project(
                graph,
                &session.id,
                &session.title,
                project_id,
                Some(project_uuid),
            );

            // Store topology_node_id on the DB record
            let node_id_str = meeting_uuid.to_string();
            let _ = db::models::meeting_session::MeetingSession::update(
                pool,
                &session.id,
                db::models::meeting_session::UpdateMeetingSession {
                    topology_node_id: Some(node_id_str.clone()),
                    ..Default::default()
                },
            )
            .await;

            Some(node_id_str)
        } else {
            None
        };

        tracing::info!(
            "[TOPSI] Meeting started: session={}, project={}, by={}, topology_node={:?}",
            session.id,
            project_id,
            user_context.user_id,
            topology_node_id
        );

        let response = serde_json::json!({
            "session_id": session.id,
            "title": session.title,
            "status": "active",
            "started_at": session.started_at,
            "topology_node_id": topology_node_id,
            "message": "Meeting started. Topsi is now listening as a silent observer."
        });

        Ok(TopsiResponse {
            message: response.to_string(),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle an audio chunk from a meeting — transcribe, detect wake word, store segment
    async fn handle_meeting_audio_chunk(
        &self,
        session_id: &str,
        audio_data: &str,
        chunk_index: u32,
        duration_ms: u32,
        user_context: &UserContext,
    ) -> Result<TopsiResponse> {
        let pool = self.db.as_ref().ok_or_else(|| {
            TopsiError::NotInitialized("Database not connected".to_string())
        })?;

        // Verify meeting exists and is active
        let meeting_state = self.meeting_manager.get_meeting(session_id).await;
        if meeting_state.is_none() {
            return Err(TopsiError::TopologyError(
                format!("No active meeting with session_id: {}", session_id),
            ));
        }
        let meeting_state = meeting_state.unwrap();

        // Calculate timestamps based on chunk index and duration
        let start_time_ms = chunk_index as i64 * duration_ms as i64;
        let end_time_ms = start_time_ms + duration_ms as i64;

        // The audio_data at this point is already transcribed text (the server layer
        // transcribes via VoiceEngine before calling the agent). This field is reused
        // to pass the transcription result through.
        let transcribed_text = audio_data.to_string();

        // Detect wake word
        let wake_result = MeetingManager::detect_wake_word(&transcribed_text);

        // Determine segment index
        let segment_index = chunk_index as i32;

        // Create transcript entry
        let entry = MeetingTranscriptEntry {
            speaker_label: None, // Set by diarization in Phase 2
            text: transcribed_text.to_string(),
            confidence: 0.0,
            start_time_ms,
            end_time_ms,
            is_topsi_addressed: wake_result.detected,
            segment_index,
        };

        // Store in memory
        self.meeting_manager
            .add_transcript_entry(session_id, entry)
            .await;

        // Store segment in DB
        let _ = db::models::meeting_session::MeetingSegment::create(
            pool,
            db::models::meeting_session::CreateMeetingSegment {
                meeting_session_id: session_id.to_string(),
                segment_index,
                speaker_label: None,
                text: transcribed_text.to_string(),
                confidence: None,
                start_time_ms,
                end_time_ms,
                is_topsi_addressed: wake_result.detected,
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to store meeting segment: {}", e);
        });

        // Build response
        let mut response = serde_json::json!({
            "session_id": session_id,
            "chunk_index": chunk_index,
            "text": transcribed_text,
            "is_topsi_addressed": wake_result.detected,
            "segment_index": segment_index,
        });

        // If Topsi was addressed, generate a response
        if wake_result.detected {
            if let Some(addressed_text) = &wake_result.addressed_text {
                let context = meeting_state.recent_context(20);
                let meeting_response = self
                    .handle_meeting_direct_address(
                        session_id,
                        addressed_text,
                        Some(&context),
                        user_context,
                    )
                    .await?;

                response["topsi_response"] = serde_json::Value::String(meeting_response.message.clone());
            }
        }

        Ok(TopsiResponse {
            message: response.to_string(),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle a direct address to Topsi during a meeting
    async fn handle_meeting_direct_address(
        &self,
        session_id: &str,
        message: &str,
        transcript_context: Option<&str>,
        _user_context: &UserContext,
    ) -> Result<TopsiResponse> {
        // Build context from recent transcript
        let context = if let Some(ctx) = transcript_context {
            ctx.to_string()
        } else if let Some(state) = self.meeting_manager.get_meeting(session_id).await {
            state.recent_context(20)
        } else {
            String::new()
        };

        // Use LLM to generate response if available
        let response_text = if let Some(llm) = &self.llm {
            let user_query = format!(
                "You were just addressed with: \"{}\"\n\nRespond concisely.",
                message
            );

            match llm
                .generate(MEETING_SYSTEM_PROMPT, &user_query, &context)
                .await
            {
                Ok(response) => response,
                Err(e) => {
                    tracing::error!("Meeting LLM response failed: {}", e);
                    format!(
                        "I heard your question: \"{}\". Let me know if you need me to elaborate.",
                        message
                    )
                }
            }
        } else {
            format!(
                "I heard: \"{}\". I'm tracking the meeting but my LLM isn't configured for responses.",
                message
            )
        };

        Ok(TopsiResponse {
            message: response_text,
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Handle ending a meeting session
    async fn handle_end_meeting(
        &self,
        session_id: &str,
        generate_notes: bool,
        _user_context: &UserContext,
    ) -> Result<TopsiResponse> {
        let pool = self.db.as_ref().ok_or_else(|| {
            TopsiError::NotInitialized("Database not connected".to_string())
        })?;

        // Get final state from memory
        let final_state = self.meeting_manager.end_meeting(session_id).await;

        let elapsed_seconds = if let Some(ref state) = final_state {
            (Utc::now() - state.started_at).num_seconds() as i32
        } else {
            0
        };

        // Generate notes if requested
        let notes = if generate_notes {
            if let Some(ref state) = final_state {
                Some(self.generate_meeting_notes(state).await?)
            } else {
                None
            }
        } else {
            None
        };

        let notes_json = notes
            .as_ref()
            .map(|n| serde_json::to_string(n).unwrap_or_default());

        // Build transcript JSON for storage
        let transcript_json = final_state
            .as_ref()
            .map(|s| serde_json::to_string(&s.transcript).unwrap_or_else(|_| "[]".to_string()));

        // Build participants JSON
        let participants_json = final_state.as_ref().map(|s| {
            let participants: Vec<String> = s.speakers.keys().cloned().collect();
            serde_json::to_string(&participants).unwrap_or_else(|_| "[]".to_string())
        });

        let participant_count = final_state
            .as_ref()
            .map(|s| s.speakers.len() as i32)
            .unwrap_or(0);

        // Fetch the DB record to get topology_node_id and project_id
        let db_session = db::models::meeting_session::MeetingSession::find_by_id(pool, session_id)
            .await
            .ok();

        // Update DB record
        let _ = db::models::meeting_session::MeetingSession::update(
            pool,
            session_id,
            db::models::meeting_session::UpdateMeetingSession {
                status: Some(db::models::meeting_session::MeetingStatus::Ended),
                ended_at: Some(Utc::now().to_rfc3339()),
                duration_seconds: Some(elapsed_seconds),
                participant_count: Some(participant_count),
                participants: participants_json,
                transcript: transcript_json,
                notes: notes_json.clone(),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| {
            tracing::error!("Failed to update meeting session: {}", e);
        });

        // Finalize topology node
        if let Some(ref session) = db_session {
            if let Some(ref topo_node_id) = session.topology_node_id {
                if let Ok(node_uuid) = Uuid::parse_str(topo_node_id) {
                    if let Ok(project_uuid) = Uuid::parse_str(&session.project_id) {
                        let mut topologies = self.topologies.write().await;
                        if let Some(graph) = topologies.get_mut(&project_uuid) {
                            VoiceTopology::end_meeting_node(graph, node_uuid);
                            tracing::info!(
                                "[TOPSI] Topology node finalized for meeting: {}",
                                session_id
                            );
                        }
                    }
                }
            }
        }

        tracing::info!(
            "[TOPSI] Meeting ended: session={}, duration={}s, segments={}",
            session_id,
            elapsed_seconds,
            final_state.as_ref().map(|s| s.transcript.len()).unwrap_or(0)
        );

        let mut response = serde_json::json!({
            "session_id": session_id,
            "status": "ended",
            "duration_seconds": elapsed_seconds,
            "participant_count": participant_count,
            "segment_count": final_state.as_ref().map(|s| s.transcript.len()).unwrap_or(0),
        });

        if let Some(notes) = notes {
            response["notes"] = serde_json::to_value(&notes).unwrap_or_default();
        }

        Ok(TopsiResponse {
            message: response.to_string(),
            tool_calls: vec![],
            topology_changes: vec![],
            topology_summary: None,
            issues: vec![],
            input_tokens: None,
            output_tokens: None,
        })
    }

    /// Generate structured meeting notes from transcript using LLM
    async fn generate_meeting_notes(
        &self,
        state: &crate::meeting::MeetingState,
    ) -> Result<MeetingNotes> {
        let transcript_text = state
            .transcript
            .iter()
            .map(|entry| {
                let speaker = entry.speaker_label.as_deref().unwrap_or("Unknown");
                format!("[{}]: {}", speaker, entry.text)
            })
            .collect::<Vec<_>>()
            .join("\n");

        if let Some(llm) = &self.llm {
            let user_query = r#"Generate structured meeting notes from the provided transcript. Respond ONLY with valid JSON in this exact format:
{
  "summary": "2-3 sentence overview",
  "topics": ["topic1", "topic2"],
  "decisions": ["decision1", "decision2"],
  "action_items": [{"description": "...", "assignee": "..." or null, "deadline": "..." or null, "priority": "high/medium/low" or null}],
  "open_questions": ["question1"],
  "participants": ["Speaker 1", "Speaker 2"]
}"#;

            match llm.generate(MEETING_SYSTEM_PROMPT, user_query, &transcript_text).await {
                Ok(response) => {
                    // Try to parse LLM output as MeetingNotes
                    if let Ok(notes) = serde_json::from_str::<MeetingNotes>(&response) {
                        return Ok(notes);
                    }
                    // Try to extract JSON from the response
                    if let Some(json_start) = response.find('{') {
                        if let Some(json_end) = response.rfind('}') {
                            let json_str = &response[json_start..=json_end];
                            if let Ok(notes) = serde_json::from_str::<MeetingNotes>(json_str) {
                                return Ok(notes);
                            }
                        }
                    }
                    tracing::warn!("Failed to parse LLM meeting notes response, using fallback");
                }
                Err(e) => {
                    tracing::error!("LLM meeting notes generation failed: {}", e);
                }
            }
        }

        // Fallback: generate basic notes from transcript
        let participants: Vec<String> = state.speakers.keys().cloned().collect();
        Ok(MeetingNotes {
            summary: format!(
                "Meeting lasted {} seconds with {} participants.",
                (Utc::now() - state.started_at).num_seconds(),
                participants.len()
            ),
            topics: vec!["See full transcript for details".to_string()],
            decisions: vec![],
            action_items: vec![],
            open_questions: vec![],
            participants,
        })
    }
}

/// Request types for Topsi
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum TopsiRequestType {
    /// Chat with Topsi
    Chat { message: String },
    /// Get topology for a project
    GetTopology { project_id: Option<Uuid> },
    /// Detect issues in topology
    DetectIssues { project_id: Option<Uuid> },
    /// List accessible projects
    ListProjects,
    /// Execute a command
    ExecuteCommand { command: String },
    /// Get prioritized recommendations for a project
    GetRecommendations {
        project_id: Option<Uuid>,
        max_count: Option<usize>,
    },
    /// Start a new meeting session
    StartMeeting {
        project_id: String,
        title: Option<String>,
    },
    /// End an active meeting session
    EndMeeting {
        session_id: String,
        generate_notes: bool,
    },
    /// Process an audio chunk from a meeting
    MeetingAudioChunk {
        session_id: String,
        audio_data: String,
        chunk_index: u32,
        duration_ms: u32,
    },
    /// Respond to a direct address during a meeting
    MeetingDirectAddress {
        session_id: String,
        message: String,
        transcript_context: Option<String>,
    },
}

/// A request to Topsi
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiRequest {
    /// Request ID
    pub id: Uuid,
    /// Request type
    #[serde(flatten)]
    pub request_type: TopsiRequestType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

impl TopsiRequest {
    pub fn new(request_type: TopsiRequestType) -> Self {
        Self {
            id: Uuid::new_v4(),
            request_type,
            timestamp: Utc::now(),
        }
    }
}
