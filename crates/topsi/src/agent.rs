//! TopsiAgent - Platform Agent with containerized access control
//!
//! Topsi is the platform orchestrator that manages all projects and users
//! with strict data isolation between clients.

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
// Database models for querying real data (topology/agent tools still use these directly)
use db::models::project::Project;
use db::models::{agent::Agent, task::Task};
// Import Nora's LLM infrastructure
use nora::brain::{
    infer_provider_from_model, LLMClient, LLMConfig as NoraLLMConfig, LLMProvider, LLMResponse,
};
// Import Nora conversation types for agentic loop
use nora::brain::{ConversationMessage, ToolResult as NoraToolResult};
use serde::{Deserialize, Serialize};
use services::services::agent_channels::{AgentChannelService, ChannelOwner};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    config::TopsiConfig,
    meeting::{MeetingManager, MeetingNotes, MeetingTranscriptEntry},
    tools::get_tool_schemas,
    topology::{graph::TopologyGraph, voice::VoiceTopology},
    DetectedIssue, Result, ToolCallResult, TopologySummary, TopsiError, TopsiResponse,
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

### Web access tools
- `search_web` - Search the internet in real-time (powered by Exa). Use when asked to find or research anything online.
- `fetch_web_page` - Fetch and read any URL. Use when asked to open or read a specific web page or link.
You HAVE full internet access. NEVER say you can't browse URLs or search the web — call the tools.

### CRM & Data tools
- `get_project_detail` - Get project details with task counts and metadata. Requires project_id.
- `list_crm_contacts` - List CRM contacts for an organization. Requires organization_id, supports search_query and lifecycle_stage filters.
- `list_crm_deals` - List CRM deals. Requires one of: organization_id, pipeline_id, or stage_id.
- `list_crm_pipelines` - List CRM pipelines and stages. Requires organization_id, optional pipeline_type filter.
- `list_workflow_definitions` - List saved workflow definitions. System workflows visible to all; org-owned filtered by access.
- `get_workflow_definition` - Get full workflow definition with steps. Requires workflow_id.
- `search_entities` - Cross-entity keyword search across projects, contacts, deals, and tasks. Requires query; organization_id needed for CRM entity results. Supports entity_types filter.
- `list_workflow_runs` - List recent workflow execution runs. Optional workflow_id or organization_id filter.
- `get_workflow_run_status` - Get detailed status of a workflow run including staged record counts. Requires run_id.
- `review_staged_data` - Show staged CRM/task records pending review. Requires run_id or organization_id.
- `approve_staged_records` - Approve valid staged records and reject duplicates for a workflow run. Requires run_id.
- `create_crm_contact` - Create a CRM contact. Requires organization_id. Optional: first_name, last_name, email, phone, company_name, job_title, linkedin_url, lifecycle_stage.
- `create_crm_deal` - Create a CRM deal. Requires organization_id and name. Optional: amount, currency, pipeline_id, stage_id, contact_id, description, expected_close_date.
- `update_crm_deal` - Update a CRM deal. Requires deal_id. Optional: name, amount, currency, stage_id, description, expected_close_date, lost_reason, win_reason.
- `build_workflow` - Delegate to the Workflow Builder specialist to create or modify a workflow. Provide user_request (what they want) and context (data you've gathered about their org, schemas, existing workflows). The specialist handles node graph generation.

### Communication
- `respond_to_user` - IMPORTANT: Use this to deliver your response. Write your complete answer in the message parameter.

## When to Use Tools vs Respond Directly

**Respond immediately with respond_to_user (NO tool calls needed):**
- Greetings, casual chat, "what's the vibe?", "how are you?", "what can you do?"
- Questions you can answer from general knowledge
- Follow-up on something already discussed in this session
- Anything where gathering live data would add no value

**Gather data first, then respond:**
- "How are my tasks going?" → list_tasks → respond_to_user
- "What projects do I have?" → list_projects → respond_to_user
- "Any issues?" → detect_issues → respond_to_user
- "Show my contacts" → list_crm_contacts → respond_to_user
- "What deals are in the pipeline?" → list_crm_deals → respond_to_user
- "What workflows do we have?" → list_workflow_definitions → respond_to_user
- "Find anything about Acme" → search_entities → respond_to_user
- "Show recent workflow runs" → list_workflow_runs → respond_to_user
- "How did that last run go?" → get_workflow_run_status → respond_to_user
- "What records are pending?" → review_staged_data → respond_to_user
- "Approve the staged records" → approve_staged_records → respond_to_user
- "Create a workflow that extracts contacts from emails" → (gather context with list_workflow_definitions) → build_workflow → respond_to_user
- Action requests → execute → respond_to_user

**Golden rule:** If you already have enough to give a good answer, call respond_to_user NOW. Don't keep calling tools hoping for better data — one or two tool calls is almost always enough.

## How to Respond
ALWAYS use the `respond_to_user` tool to communicate with users. In the message parameter, write YOUR complete response:
- If asked "tell me a story" → respond_to_user immediately with the story
- If asked "who are you?" → respond_to_user immediately with your intro
- If asked about the system → ONE tool call to gather data, then respond_to_user
- NEVER call the same tool twice in a row — if you got results, use them

## Example Interactions
User: "What's the vibe today?" / "How's it going?" / casual greeting
→ respond_to_user immediately: brief, energetic status from your knowledge

User: "Build me a landing page for my new product"
→ create_task(agent_name="claude") → respond_to_user: "On it — Claude is building the landing page now."

User: "Research competitor activity with Scout"
→ create_task(agent_name="Scout") → respond_to_user: "Scout is researching competitor activity now. I'll update you when it's done."

User: "How are my tasks going?"
→ list_tasks → respond_to_user: brief 2-3 line summary (do NOT call list_tasks again)

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
pub use access_control::{AccessControl, AccessScope, ProjectAccess, UserContext};

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
    /// Agent communication channels (email, future SMS/chat)
    pub channel_service: Option<Arc<AgentChannelService>>,
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
    /// Platform data service — owns all CRUD operations for platform entities
    platform_data: Option<crate::platform_data::PlatformDataService>,
}

/// Generate a human-readable description of what a tool call will do.
fn describe_tool_action(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "delete_task" => {
            let id = args
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("Delete task {}", id)
        }
        "bulk_update_tasks" => {
            let count = args
                .get("task_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            let fields: Vec<&str> = ["status", "priority", "assigned_agent"]
                .iter()
                .filter(|f| args.get(**f).is_some())
                .copied()
                .collect();
            format!(
                "Bulk update {} tasks (changing: {})",
                count,
                fields.join(", ")
            )
        }
        "create_task" => {
            let title = args
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("untitled");
            format!("Create task '{}'", title)
        }
        "create_project" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");
            format!("Create project '{}'", name)
        }
        "create_crm_contact" => {
            let name = args
                .get("first_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let last = args.get("last_name").and_then(|v| v.as_str()).unwrap_or("");
            format!("Create CRM contact '{} {}'", name, last)
        }
        "create_crm_deal" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unnamed");
            format!("Create CRM deal '{}'", name)
        }
        "approve_staged_records" => {
            let run_id = args
                .get("run_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("Approve staged records for workflow run {}", run_id)
        }
        _ => format!("Execute {} with args: {}", tool_name, args),
    }
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
            channel_service: None,
            topologies: Arc::new(RwLock::new(indexmap::IndexMap::new())),
            initialized_at: Utc::now(),
            active: Arc::new(RwLock::new(false)),
            llm,
            execution_bridge: None,
            session_history: Arc::new(RwLock::new(HashMap::new())),
            meeting_manager: Arc::new(MeetingManager::new()),
            platform_data: None,
        })
    }

    /// Attach database connection and sync access control
    pub async fn with_database(mut self, pool: SqlitePool) -> Self {
        // Sync access control from database so user permissions are loaded
        if let Err(e) = self.access_control.sync_from_database(&pool).await {
            tracing::error!("Failed to sync Topsi access control from database: {}", e);
        }
        // Wire agent communication channels (Topsi's own agent-scoped email)
        self.channel_service = Some(Arc::new(AgentChannelService::new(pool.clone())));
        // Initialize platform data service
        self.platform_data = Some(crate::platform_data::PlatformDataService::new(
            pool.clone(),
            None,
        ));
        self.db = Some(pool);
        self
    }

    /// Get the channel service with Topsi's agent identity as owner.
    pub fn channel_owner(&self) -> ChannelOwner {
        ChannelOwner::Agent(self.id)
    }

    /// Attach a task execution bridge for triggering agent execution
    pub fn with_execution_bridge(mut self, bridge: Arc<dyn TaskExecutionBridge>) -> Self {
        // Rebuild platform_data with the bridge if DB is already set
        if let Some(pool) = &self.db {
            self.platform_data = Some(crate::platform_data::PlatformDataService::new(
                pool.clone(),
                Some(bridge.clone()),
            ));
        }
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
        self.access_control
            .log_access(user_context, &format!("{:?}", request.request_type), true)
            .await;

        match request.request_type {
            TopsiRequestType::Chat { message } => {
                self.handle_chat(&message, user_context, &scope, session_id)
                    .await
            }
            TopsiRequestType::GetTopology { project_id } => {
                self.handle_get_topology(project_id, user_context, &scope)
                    .await
            }
            TopsiRequestType::DetectIssues { project_id } => {
                self.handle_detect_issues(project_id, user_context, &scope)
                    .await
            }
            TopsiRequestType::ListProjects => self.handle_list_projects(user_context, &scope).await,
            TopsiRequestType::ExecuteCommand { command } => {
                self.handle_command(&command, user_context, &scope).await
            }
            TopsiRequestType::GetRecommendations {
                project_id,
                max_count,
            } => {
                self.handle_get_recommendations(project_id, max_count, user_context, &scope)
                    .await
            }
            TopsiRequestType::StartMeeting { project_id, title } => {
                self.handle_start_meeting(&project_id, title.as_deref(), user_context)
                    .await
            }
            TopsiRequestType::EndMeeting {
                session_id,
                generate_notes,
            } => {
                self.handle_end_meeting(&session_id, generate_notes, user_context)
                    .await
            }
            TopsiRequestType::MeetingAudioChunk {
                session_id,
                audio_data,
                chunk_index,
                duration_ms,
            } => {
                self.handle_meeting_audio_chunk(
                    &session_id,
                    &audio_data,
                    chunk_index,
                    duration_ms,
                    user_context,
                )
                .await
            }
            TopsiRequestType::MeetingDirectAddress {
                session_id,
                message,
                transcript_context,
            } => {
                self.handle_meeting_direct_address(
                    &session_id,
                    &message,
                    transcript_context.as_deref(),
                    user_context,
                )
                .await
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
            tracing::debug!(
                "[TOPSI] Loaded {} history messages for session {}",
                msgs.len(),
                sid
            );
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

        // Load effective system prompt (custom from DB or default)
        let system_prompt = self.get_effective_system_prompt().await;

        // Initial LLM call with conversation history
        let mut response = llm
            .generate_with_tools_and_history(&system_prompt, message, &context, &tools, &history)
            .await
            .map_err(|e| TopsiError::LLMError(format!("LLM request failed: {}", e)))?;

        // Token budget guard: stop when cumulative tokens exceed this threshold.
        const MAX_TOTAL_TOKENS: i64 = 200_000;
        // Hard iteration cap — safety net only, system prompt should prevent loops
        const MAX_ITERATIONS: u32 = 25;
        // Repetition guard — if the exact same tools fire 4x in a row, something is stuck
        const MAX_REPEAT_ROUNDS: usize = 4;

        let mut iteration: u32 = 0;
        let mut last_tool_batch: Option<Vec<String>> = None;
        let mut repeat_count: usize = 0;
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
                    let tool_results_json =
                        self.execute_tool_calls(&calls, user_context, scope).await;

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

                    // Hard iteration cap
                    if iteration >= MAX_ITERATIONS {
                        tracing::warn!(
                            "[TOPSI] Hit max iterations ({}) — forcing stop",
                            MAX_ITERATIONS
                        );
                        break;
                    }

                    // Repetition guard — same tool batch called too many times in a row
                    let this_batch: Vec<String> = calls.iter().map(|c| c.name.clone()).collect();
                    if last_tool_batch.as_deref() == Some(this_batch.as_slice()) {
                        repeat_count += 1;
                        if repeat_count >= MAX_REPEAT_ROUNDS {
                            tracing::warn!(
                                "[TOPSI] Repetition guard triggered after {} identical rounds of {:?}",
                                repeat_count, this_batch
                            );
                            break;
                        }
                    } else {
                        repeat_count = 0;
                        last_tool_batch = Some(this_batch);
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
                            &system_prompt,
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

        // Resolve the final message — make one last LLM call to synthesise if loop broke early
        let final_msg = if let Some(msg) = final_message {
            msg
        } else {
            tracing::warn!(
                "[TOPSI] Loop ended early (iter={}, tokens={}+{}), synthesising answer",
                iteration,
                total_input_tokens,
                total_output_tokens
            );

            // Try one direct synthesis call (no tools) with what we've gathered
            let gathered: String = all_tool_calls
                .iter()
                .filter(|r| r.success)
                .take(4)
                .map(|r| format!("Tool '{}' returned: {}", r.tool_name, r.result))
                .collect::<Vec<_>>()
                .join("\n");
            let synthesis_context = format!(
                "You gathered this information:\n{}\n\nNow give a direct, conversational answer.",
                gathered
            );
            match llm
                .generate(&system_prompt, message, &synthesis_context)
                .await
            {
                Ok(content) => content,
                Err(_) => "Something went sideways — try asking again with a bit more context."
                    .to_string(),
            }
        };

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
    async fn build_context_for_scope(
        &self,
        scope: &AccessScope,
        user_context: &UserContext,
    ) -> String {
        let mut context_parts = vec![];

        // Add user context
        context_parts.push(format!(
            "User: {} ({})",
            user_context.email.as_deref().unwrap_or("unknown"),
            if user_context.is_admin {
                "admin"
            } else {
                "user"
            }
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
                            context_parts
                                .push(format!("... and {} more projects", projects.len() - 10));
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
                            "SELECT COUNT(*) FROM tasks WHERE status = 'todo'",
                        )
                        .fetch_one(pool)
                        .await
                        {
                            context_parts.push(format!("Todo: {}", todo_count));
                        }
                        if let Ok(in_progress_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'inprogress'",
                        )
                        .fetch_one(pool)
                        .await
                        {
                            context_parts.push(format!("In Progress: {}", in_progress_count));
                        }
                        if let Ok(done_count) = sqlx::query_scalar::<_, i64>(
                            "SELECT COUNT(*) FROM tasks WHERE status = 'done'",
                        )
                        .fetch_one(pool)
                        .await
                        {
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
                        if let Ok(Some(project)) =
                            Project::find_by_id(pool, &project_id.to_string()).await
                        {
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
                    if let Ok(Some(project)) = Project::find_by_id(pool, &id.to_string()).await {
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
            context_parts
                .push("Warning: Database not connected - limited data available".to_string());
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
        use db::models::topsi_user_settings::{
            classify_tool_risk, ConfirmationMode, ToolRisk, TopsiUserSettings,
        };

        // Load user confirmation settings once per batch
        let user_settings = if let Some(pool) = &self.db {
            Some(TopsiUserSettings::get_or_default(pool, &user_context.user_id).await)
        } else {
            None
        };

        // Instance-wide autonomy floor constrains user settings
        let instance_floor = self.config.autonomy_level.max_confirmation_mode();

        let mut results = Vec::new();

        for call in calls {
            tracing::debug!(
                "[TOPSI] Executing tool: {} with args: {}",
                call.name,
                call.arguments
            );

            // ── Confirmation gate ────────────────────────────────────────
            if let Some(ref settings) = user_settings {
                let risk = classify_tool_risk(&call.name);
                let user_mode = settings.confirmation_mode_for_tool(&call.name);
                let effective = ConfirmationMode::most_restrictive(&user_mode, &instance_floor);
                if risk != ToolRisk::Green && matches!(effective, ConfirmationMode::AlwaysConfirm) {
                    let action_desc = describe_tool_action(&call.name, &call.arguments);
                    results.push(serde_json::json!({
                        "pending_confirmation": true,
                        "tool_name": call.name,
                        "arguments": call.arguments,
                        "risk_level": format!("{:?}", risk),
                        "action": action_desc,
                        "message": format!(
                            "This action requires confirmation: {}. Please confirm to proceed.",
                            action_desc
                        )
                    }));
                    continue;
                }
            }

            let result = match call.name.as_str() {
                // ── Topology tools (stay in agent) ──────────────────────────
                "list_nodes" => self.tool_list_nodes(&call.arguments, scope).await,
                "list_edges" => self.tool_list_edges(&call.arguments, scope).await,
                "find_path" => self.tool_find_path(&call.arguments, scope).await,
                "detect_issues" => self.tool_detect_issues(&call.arguments, scope).await,
                "get_topology_summary" => {
                    self.tool_get_topology_summary(&call.arguments, scope).await
                }
                "create_cluster" => {
                    self.tool_create_cluster(&call.arguments, user_context, scope)
                        .await
                }
                "verify_access" => self.tool_verify_access(&call.arguments, scope).await,

                // ── Agent & utility tools (stay in agent) ───────────────────
                "list_agents" => self.tool_list_agents(&call.arguments).await,
                "respond_to_user" => self.tool_respond_to_user(&call.arguments).await,
                "search_web" => self.tool_search_web(&call.arguments).await,
                "fetch_web_page" => self.tool_fetch_web_page(&call.arguments).await,

                // ── Specialist delegation (stay in agent) ───────────────────
                "build_workflow" => {
                    self.tool_build_workflow(&call.arguments, user_context)
                        .await
                }

                // ── Platform data tools (delegated to PlatformDataService) ──
                "list_projects"
                | "create_project"
                | "update_project"
                | "list_organizations"
                | "get_project_detail"
                | "create_task"
                | "start_task_execution"
                | "get_task_status"
                | "update_task"
                | "list_tasks"
                | "delete_task"
                | "bulk_update_tasks"
                | "list_crm_contacts"
                | "list_crm_deals"
                | "list_crm_pipelines"
                | "create_crm_contact"
                | "create_crm_deal"
                | "update_crm_deal"
                | "list_workflow_definitions"
                | "get_workflow_definition"
                | "list_workflow_runs"
                | "get_workflow_run_status"
                | "review_staged_data"
                | "approve_staged_records"
                | "search_entities" => {
                    if let Some(pds) = &self.platform_data {
                        match call.name.as_str() {
                            "list_projects" => pds.list_projects(&call.arguments, scope).await,
                            "create_project" => {
                                pds.create_project(&call.arguments, user_context).await
                            }
                            "update_project" => {
                                pds.update_project(&call.arguments, user_context).await
                            }
                            "list_organizations" => pds.list_organizations().await,
                            "get_project_detail" => {
                                pds.get_project_detail(&call.arguments, scope).await
                            }
                            "create_task" => {
                                pds.create_task(&call.arguments, user_context, scope).await
                            }
                            "start_task_execution" => {
                                pds.start_task_execution(&call.arguments, user_context, scope)
                                    .await
                            }
                            "get_task_status" => pds.get_task_status(&call.arguments, scope).await,
                            "update_task" => {
                                pds.update_task(&call.arguments, user_context, scope).await
                            }
                            "list_tasks" => {
                                pds.list_tasks(&call.arguments, user_context, scope).await
                            }
                            "delete_task" => {
                                pds.delete_task(&call.arguments, user_context, scope).await
                            }
                            "bulk_update_tasks" => {
                                pds.bulk_update_tasks(&call.arguments, user_context, scope)
                                    .await
                            }
                            "list_crm_contacts" => {
                                pds.list_crm_contacts(&call.arguments, user_context, scope)
                                    .await
                            }
                            "list_crm_deals" => {
                                pds.list_crm_deals(&call.arguments, user_context, scope)
                                    .await
                            }
                            "list_crm_pipelines" => {
                                pds.list_crm_pipelines(&call.arguments, user_context, scope)
                                    .await
                            }
                            "create_crm_contact" => {
                                pds.create_crm_contact(&call.arguments, user_context).await
                            }
                            "create_crm_deal" => {
                                pds.create_crm_deal(&call.arguments, user_context).await
                            }
                            "update_crm_deal" => {
                                pds.update_crm_deal(&call.arguments, user_context).await
                            }
                            "list_workflow_definitions" => {
                                pds.list_workflow_definitions(&call.arguments, scope).await
                            }
                            "get_workflow_definition" => {
                                pds.get_workflow_definition(&call.arguments, scope).await
                            }
                            "list_workflow_runs" => {
                                pds.list_workflow_runs(&call.arguments, user_context, scope)
                                    .await
                            }
                            "get_workflow_run_status" => {
                                pds.get_workflow_run_status(&call.arguments, scope).await
                            }
                            "review_staged_data" => {
                                pds.review_staged_data(&call.arguments, user_context, scope)
                                    .await
                            }
                            "approve_staged_records" => {
                                pds.approve_staged_records(&call.arguments, user_context, scope)
                                    .await
                            }
                            "search_entities" => {
                                pds.search_entities(&call.arguments, user_context, scope)
                                    .await
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        Err(TopsiError::ToolError(
                            "Platform data service not initialized (no database)".to_string(),
                        ))
                    }
                }

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

    /// Resolve the effective system prompt. Checks system_settings for a custom prompt
    /// (key: "topsi_system_prompt"), falling back to the compiled-in default.
    /// Also checks "topsi_prompt_mode" for "sudolang" vs "standard" (default).
    async fn get_effective_system_prompt(&self) -> String {
        let Some(pool) = &self.db else {
            return TOPSI_SYSTEM_PROMPT.to_string();
        };

        // Check if there's a custom prompt stored
        let mode = db::models::system_settings::SystemSetting::get(pool, "topsi_prompt_mode")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "standard".to_string());

        let setting_key = if mode == "sudolang" {
            "topsi_system_prompt_sudolang"
        } else {
            "topsi_system_prompt"
        };

        db::models::system_settings::SystemSetting::get(pool, setting_key)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| TOPSI_SYSTEM_PROMPT.to_string())
    }

    // ==================== Tool Implementations ====================
    // Platform data tools (projects, tasks, CRM, workflows) are delegated to
    // PlatformDataService — see platform_data.rs. Only topology, agent, utility,
    // and specialist delegation tools remain here.

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
                            if let Ok(Some(p)) = Project::find_by_id(pool, &id.to_string()).await {
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
                    AccessScope::Admin => Project::find_all(pool)
                        .await
                        .unwrap_or_default()
                        .iter()
                        .filter_map(|p| Uuid::parse_str(&p.id).ok())
                        .collect(),
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
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>());

        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({
                "error": "Database not connected",
                "issues": []
            }));
        };

        let mut issues = Vec::new();

        // Check for tasks stuck in progress
        if issue_types.as_ref().is_none_or(|t| t.contains(&"stale")) {
            let stale_tasks: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM tasks WHERE status = 'inprogress' AND updated_at < datetime('now', '-7 days')"
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
        if issue_types
            .as_ref()
            .is_none_or(|t| t.contains(&"bottleneck"))
        {
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
            AccessScope::Admin => sqlx::query_scalar("SELECT COUNT(*) FROM projects")
                .fetch_one(pool)
                .await
                .unwrap_or(0),
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

        let active_task_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'inprogress'")
                .fetch_one(pool)
                .await
                .unwrap_or(0);

        let todo_task_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'todo'")
                .fetch_one(pool)
                .await
                .unwrap_or(0);

        let done_task_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE status = 'done'")
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
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unnamed");
        let node_ids = args.get("node_ids").and_then(|v| v.as_array());

        Ok(serde_json::json!({
            "created": false,
            "cluster_name": name,
            "node_count": node_ids.map(|arr| arr.len()).unwrap_or(0),
            "message": "Cluster creation not yet implemented - clusters require topology graph"
        }))
    }

    /// List all available agents
    async fn tool_list_agents(&self, _args: &serde_json::Value) -> Result<serde_json::Value> {
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

    async fn tool_respond_to_user(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("I'm Topsi, your topological super intelligence. How can I help you today?");

        Ok(serde_json::json!({
            "response": message,
            "spoken": true
        }))
    }

    async fn tool_search_web(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as u32;

        let api_key = std::env::var("EXA_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            return Ok(
                serde_json::json!({"success": false, "error": "EXA_API_KEY not configured"}),
            );
        }

        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.exa.ai/search")
            .header("x-api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({"query": query, "num_results": max_results, "use_autoprompt": true, "text": true}))
            .send()
            .await
            .map_err(|e| TopsiError::ToolError(format!("Search failed: {}", e)))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| TopsiError::ToolError(format!("Search parse failed: {}", e)))?;

        let results = data
            .get("results")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|r| {
                        serde_json::json!({
                            "title": r.get("title").and_then(|t| t.as_str()).unwrap_or(""),
                            "url": r.get("url").and_then(|u| u.as_str()).unwrap_or(""),
                            "snippet": r.get("text").and_then(|t| t.as_str()).unwrap_or(""),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(serde_json::json!({"success": true, "query": query, "results": results}))
    }

    async fn tool_fetch_web_page(&self, args: &serde_json::Value) -> Result<serde_json::Value> {
        let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| TopsiError::ToolError(format!("Fetch failed: {}", e)))?;
        let content = response
            .text()
            .await
            .map_err(|e| TopsiError::ToolError(format!("Read failed: {}", e)))?;

        // Basic tag strip
        let mut in_tag = false;
        let mut text = String::with_capacity(content.len());
        for c in content.chars() {
            match c {
                '<' => {
                    in_tag = true;
                    text.push(' ');
                }
                '>' => {
                    in_tag = false;
                }
                _ if !in_tag => text.push(c),
                _ => {}
            }
        }
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");

        Ok(
            serde_json::json!({"success": true, "url": url, "content": text, "content_length": text.len()}),
        )
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

    // ── Workflow builder delegation ─────────────────────────────────────────

    /// Delegate workflow creation/modification to the Workflow Builder specialist.
    /// Topsi passes the user's request and scoping info; the builder queries the DB
    /// directly for anything else it needs.
    async fn tool_build_workflow(
        &self,
        args: &serde_json::Value,
        _user_context: &UserContext,
    ) -> Result<serde_json::Value> {
        let Some(pool) = &self.db else {
            return Ok(serde_json::json!({"error": "Database not connected"}));
        };
        let Some(llm) = &self.llm else {
            return Ok(
                serde_json::json!({"error": "LLM not configured — cannot generate workflows"}),
            );
        };

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("create");
        let user_request = match args.get("user_request").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return Ok(serde_json::json!({"error": "user_request is required"})),
        };
        let context = args.get("context").and_then(|v| v.as_str()).unwrap_or("");
        let workflow_id = args.get("workflow_id").and_then(|v| v.as_str());
        let owner_id = args.get("owner_id").and_then(|v| v.as_str());

        match crate::workflow_builder::build_workflow(
            llm,
            pool,
            action,
            user_request,
            context,
            workflow_id,
            owner_id,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(e) => Ok(serde_json::json!({"error": e})),
        }
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
            if !self
                .access_control
                .can_access_project(user_context, pid)
                .await
            {
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
            if !self
                .access_control
                .can_access_project(user_context, pid)
                .await
            {
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
        _user_context: &UserContext,
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
        _user_context: &UserContext,
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
            "help" => Ok(TopsiResponse {
                message: "Available commands: status, help, topology, issues".to_string(),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            }),
            _ => Ok(TopsiResponse {
                message: format!("Unknown command: {}", parts[0]),
                tool_calls: vec![],
                topology_changes: vec![],
                topology_summary: None,
                issues: vec![],
                input_tokens: None,
                output_tokens: None,
            }),
        }
    }

    /// Get topology summary for accessible projects
    async fn get_topology_summary(
        &self,
        _project_id: Option<Uuid>,
        _scope: &AccessScope,
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
        _project_id: Option<Uuid>,
        _scope: &AccessScope,
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
        use crate::prioritization::{
            free_energy::{IntoPotentialAction, PotentialAction},
            goals::{Goal, GoalType},
            recommender::PriorityRecommender,
        };

        // Verify access
        if let Some(pid) = project_id {
            if !self
                .access_control
                .can_access_project(user_context, pid)
                .await
            {
                return Err(TopsiError::TopologyError(
                    "Access denied to project".to_string(),
                ));
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
                AccessScope::Admin => Project::find_all(pool)
                    .await
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|p| Uuid::parse_str(&p.id).ok())
                    .collect(),
                AccessScope::Projects(ids) => ids.iter().copied().collect(),
                AccessScope::SingleProject(id) => vec![*id],
                AccessScope::None => vec![],
            },
        };

        // Fetch open tasks across those projects
        let mut all_tasks: Vec<Task> = Vec::new();
        for pid in &project_ids {
            let tasks: Vec<Task> = sqlx::query_as(
                "SELECT * FROM tasks WHERE project_id = ? AND status IN ('todo', 'inprogress') ORDER BY created_at DESC LIMIT 50",
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
        let pool = self
            .db
            .as_ref()
            .ok_or_else(|| TopsiError::NotInitialized("Database not connected".to_string()))?;

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
        .map_err(|e| {
            TopsiError::TopologyError(format!("Failed to create meeting session: {}", e))
        })?;

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
        let pool = self
            .db
            .as_ref()
            .ok_or_else(|| TopsiError::NotInitialized("Database not connected".to_string()))?;

        // Verify meeting exists and is active
        let meeting_state = self.meeting_manager.get_meeting(session_id).await;
        if meeting_state.is_none() {
            return Err(TopsiError::TopologyError(format!(
                "No active meeting with session_id: {}",
                session_id
            )));
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

                response["topsi_response"] =
                    serde_json::Value::String(meeting_response.message.clone());
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
        let pool = self
            .db
            .as_ref()
            .ok_or_else(|| TopsiError::NotInitialized("Database not connected".to_string()))?;

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
            final_state
                .as_ref()
                .map(|s| s.transcript.len())
                .unwrap_or(0)
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

    /// Regenerate notes for an ended meeting by reading segments from DB
    pub async fn handle_regenerate_meeting_notes(
        &self,
        session_id: &str,
    ) -> Result<crate::meeting::MeetingNotes> {
        let pool = self
            .db
            .as_ref()
            .ok_or_else(|| TopsiError::NotInitialized("Database not connected".to_string()))?;

        let segments =
            db::models::meeting_session::MeetingSegment::find_by_session(pool, session_id)
                .await
                .map_err(|e| TopsiError::NotInitialized(format!("DB error: {e}")))?;

        if segments.is_empty() {
            return Err(TopsiError::NotInitialized(
                "No segments found for this session".to_string(),
            ));
        }

        let transcript_text = segments
            .iter()
            .map(|s| {
                let speaker = s.speaker_label.as_deref().unwrap_or("Speaker");
                format!("[{}]: {}", speaker, s.text)
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Build synthetic MeetingState so we can reuse generate_meeting_notes
        let mut state =
            crate::meeting::MeetingState::new(session_id.to_string(), String::new(), String::new());
        for seg in &segments {
            let entry = crate::meeting::MeetingTranscriptEntry {
                speaker_label: seg.speaker_label.clone(),
                text: seg.text.clone(),
                confidence: seg.confidence.unwrap_or(1.0),
                start_time_ms: seg.start_time_ms,
                end_time_ms: seg.end_time_ms,
                is_topsi_addressed: seg.is_topsi_addressed,
                segment_index: seg.segment_index,
            };
            state.transcript.push(entry);
            if let Some(ref label) = seg.speaker_label {
                state
                    .speakers
                    .entry(label.clone())
                    .or_insert_with(|| crate::meeting::SpeakerInfo::new(label.clone()));
            }
        }

        // Try LLM with raw transcript text first (avoids second scan of state.transcript)
        if let Some(llm) = &self.llm {
            let user_query = r#"Generate structured meeting notes from the provided transcript. Respond ONLY with valid JSON matching this exact schema (use camelCase keys):
{
  "summary": "2-3 sentence overview",
  "topics": ["topic1", "topic2"],
  "decisions": ["decision1"],
  "actionItems": [{"description": "task", "assignee": null, "deadline": null, "priority": "high"}],
  "openQuestions": ["unresolved question"],
  "participants": ["Speaker 1"]
}"#;
            match llm
                .generate(MEETING_SYSTEM_PROMPT, user_query, &transcript_text)
                .await
            {
                Ok(response) => {
                    if let Ok(notes) =
                        serde_json::from_str::<crate::meeting::MeetingNotes>(&response)
                    {
                        return Ok(notes);
                    }
                    if let Some(js) = response.find('{') {
                        if let Some(je) = response.rfind('}') {
                            if let Ok(notes) = serde_json::from_str::<crate::meeting::MeetingNotes>(
                                &response[js..=je],
                            ) {
                                return Ok(notes);
                            }
                        }
                    }
                    tracing::warn!(
                        "Failed to parse regenerated notes JSON; snippet: {}",
                        &response[..response.len().min(400)]
                    );
                }
                Err(e) => tracing::error!("LLM regen failed: {e}"),
            }
        }

        self.generate_meeting_notes(&state).await
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
            let user_query = r#"Generate structured meeting notes from the provided transcript. Respond ONLY with valid JSON matching this exact schema (use camelCase keys):
{
  "summary": "2-3 sentence overview",
  "topics": ["topic1", "topic2"],
  "decisions": ["decision1"],
  "actionItems": [{"description": "task", "assignee": null, "deadline": null, "priority": "high"}],
  "openQuestions": ["unresolved question"],
  "participants": ["Speaker 1"]
}"#;

            match llm
                .generate(MEETING_SYSTEM_PROMPT, user_query, &transcript_text)
                .await
            {
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
