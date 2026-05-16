use std::sync::Arc;

use chrono::{DateTime, Datelike, Utc};
use nora::agent::{
    NoraRequest, NoraRequestType, RapidPlaybookRequest, RapidPlaybookResult, RequestPriority,
};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::tool::{Parameters, ToolRouter},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use crate::routes::nora::NoraManager;

/// MCP request for chatting with Nora
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraChatRequest {
    #[schemars(description = "The message to send to Nora")]
    pub message: String,

    #[schemars(description = "Type of executive request")]
    pub request_type: Option<String>,

    #[schemars(description = "Priority level of the request")]
    pub priority: Option<String>,

    #[schemars(description = "Whether to include voice response")]
    pub voice_enabled: Option<bool>,

    #[schemars(description = "Session ID for conversation continuity")]
    pub session_id: Option<String>,
}

/// MCP request for task coordination
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraCoordinateTasksRequest {
    #[schemars(description = "List of tasks to coordinate")]
    pub tasks: Vec<TaskCoordinationInfo>,

    #[schemars(description = "Type of coordination requested")]
    pub coordination_type: String,

    #[schemars(description = "Additional context for task coordination")]
    pub context: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, schemars::JsonSchema)]
pub struct TaskCoordinationInfo {
    #[schemars(description = "Task ID")]
    pub id: String,

    #[schemars(description = "Task title")]
    pub title: String,

    #[schemars(description = "Task priority")]
    pub priority: String,

    #[schemars(description = "Who the task is assigned to")]
    pub assigned_to: Option<String>,

    #[schemars(description = "Task deadline")]
    pub deadline: Option<DateTime<Utc>>,

    #[schemars(description = "Task dependencies")]
    pub dependencies: Option<Vec<String>>,
}

/// MCP request for strategic planning
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraStrategicPlanningRequest {
    #[schemars(description = "Scope of strategic planning")]
    pub planning_scope: String,

    #[schemars(description = "Strategic objectives to address")]
    pub objectives: Vec<String>,

    #[schemars(description = "Planning constraints and limitations")]
    pub constraints: Option<Value>,

    #[schemars(description = "Current organizational situation")]
    pub current_situation: Option<String>,

    #[schemars(description = "Success metrics")]
    pub success_metrics: Option<Vec<String>>,
}

/// MCP request for performance analysis
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraPerformanceAnalysisRequest {
    #[schemars(description = "Type of performance analysis")]
    pub analysis_type: String,

    #[schemars(description = "Performance metrics to analyze")]
    pub metrics: Value,

    #[schemars(description = "Time period for analysis")]
    pub time_period: Option<String>,

    #[schemars(description = "Baseline for performance comparison")]
    pub comparison_baseline: Option<String>,

    #[schemars(description = "Specific areas to focus analysis on")]
    pub focus_areas: Option<Vec<String>>,
}

/// MCP request for approval
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraApprovalRequest {
    #[schemars(description = "Description of the action requiring approval")]
    pub action_description: String,

    #[schemars(description = "ID or role of the required approver")]
    pub required_approver: String,

    #[schemars(description = "Urgency level of the approval request")]
    pub urgency: Option<String>,

    #[schemars(description = "Additional context for the approval request")]
    pub context: Option<Value>,

    #[schemars(description = "Deadline for approval")]
    pub deadline: Option<DateTime<Utc>>,
}

/// MCP request for executive alert
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraExecutiveAlertRequest {
    #[schemars(description = "Alert message content")]
    pub message: String,

    #[schemars(description = "Alert severity level")]
    pub severity: String,

    #[schemars(description = "Whether the alert requires immediate action")]
    pub requires_action: Option<bool>,

    #[schemars(description = "Source of the alert")]
    pub source: String,

    #[schemars(description = "Additional context for the alert")]
    pub context: Option<Value>,
}

/// MCP request for voice synthesis
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraVoiceSynthesisRequest {
    #[schemars(description = "Text to synthesize into speech")]
    pub text: String,

    #[schemars(description = "Voice synthesis settings")]
    pub voice_settings: Option<VoiceSettings>,

    #[schemars(description = "Format for audio output")]
    pub output_format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct VoiceSettings {
    #[schemars(description = "Speech speed (0.5-2.0)")]
    pub speed: Option<f32>,

    #[schemars(description = "Speech volume (0.0-1.0)")]
    pub volume: Option<f32>,

    #[schemars(description = "Speech pitch (0.5-2.0)")]
    pub pitch: Option<f32>,

    #[schemars(description = "British accent strength (0.0-1.0)")]
    pub accent_strength: Option<f32>,
}

/// MCP request for coordination stats
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraCoordinationStatsRequest {
    #[schemars(description = "Include individual agent statistics")]
    pub include_agents: Option<bool>,

    #[schemars(description = "Include recent coordination events")]
    pub include_events: Option<bool>,

    #[schemars(description = "Time range for statistics")]
    pub time_range: Option<String>,
}

/// MCP request for "what's on my schedule"
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraScheduleRequest {
    #[schemars(description = "Organization ID whose calendar to query")]
    pub organization_id: String,

    #[schemars(
        description = "Window: 'today', 'tomorrow', 'this_week', 'next_7_days'. Defaults to 'today'."
    )]
    pub window: Option<String>,

    #[schemars(description = "Filter to events with this CRM contact id (matched attendee)")]
    pub crm_contact_id: Option<String>,

    #[schemars(description = "Maximum number of events to return (default 25, max 100)")]
    pub limit: Option<i32>,
}

/// MCP request for rapid prototyping playbook
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraRapidPlaybookRequest {
    #[schemars(description = "Name of the project or initiative")]
    pub project_name: String,

    #[schemars(description = "Objectives to accomplish in this prototype")]
    pub objectives: Option<Vec<String>>,

    #[schemars(description = "Optional repo or workspace hint")]
    pub repo_hint: Option<String>,

    #[schemars(description = "Additional notes or context")]
    pub notes: Option<String>,
}

/// Nora MCP Server
#[derive(Clone)]
pub struct NoraServer {
    pub pool: SqlitePool,
    pub nora_manager: Arc<NoraManager>,
    tool_router: ToolRouter<NoraServer>,
}

impl NoraServer {
    pub fn new(pool: SqlitePool, nora_manager: Arc<NoraManager>) -> Self {
        Self {
            pool,
            nora_manager,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl NoraServer {
    #[tool(
        description = "Chat with Nora, the British AI Executive Assistant. She can help with strategic planning, task coordination, performance analysis, and executive decision support."
    )]
    async fn nora_chat(
        &self,
        Parameters(NoraChatRequest {
            message,
            request_type,
            priority,
            voice_enabled,
            session_id,
        }): Parameters<NoraChatRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let req_type = match request_type.as_deref() {
            Some("TaskCoordination") => NoraRequestType::TaskCoordination,
            Some("StrategyPlanning") => NoraRequestType::StrategyPlanning,
            Some("PerformanceAnalysis") => NoraRequestType::PerformanceAnalysis,
            Some("CommunicationManagement") => NoraRequestType::CommunicationManagement,
            Some("DecisionSupport") => NoraRequestType::DecisionSupport,
            Some("ProactiveNotification") => NoraRequestType::ProactiveNotification,
            _ => NoraRequestType::TextInteraction,
        };

        let req_priority = match priority.as_deref() {
            Some("Low") => RequestPriority::Low,
            Some("High") => RequestPriority::High,
            Some("Urgent") => RequestPriority::Urgent,
            Some("Executive") => RequestPriority::Executive,
            _ => RequestPriority::Normal,
        };

        let request = NoraRequest {
            request_id: format!("mcp-{}", uuid::Uuid::new_v4()),
            session_id: session_id
                .unwrap_or_else(|| format!("mcp-{}", chrono::Utc::now().timestamp())),
            request_type: req_type,
            content: message,
            context: None,
            voice_enabled: voice_enabled.unwrap_or(false),
            priority: req_priority,
            timestamp: chrono::Utc::now(),
        };

        match self.nora_manager.process_request(request).await {
            Ok(response) => {
                let mut result = json!({
                    "response": response.content,
                    "response_type": format!("{:?}", response.response_type),
                    "processing_time_ms": response.processing_time_ms,
                    "actions": response.actions,
                    "follow_up_suggestions": response.follow_up_suggestions,
                    "context_updates": response.context_updates
                });

                if let Some(voice_response) = response.voice_response {
                    result["voice_response"] = json!({
                        "audio_base64": voice_response,
                        "format": "wav"
                    });
                }

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| "Response serialization failed".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = json!({
                    "success": false,
                    "error": "Failed to process Nora request",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Request processing error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Coordinate tasks across teams and agents with Nora's executive oversight. Helps prioritize, allocate resources, and resolve conflicts."
    )]
    async fn nora_coordinate_tasks(
        &self,
        Parameters(NoraCoordinateTasksRequest {
            tasks,
            coordination_type,
            context,
        }): Parameters<NoraCoordinateTasksRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let request_content = json!({
            "coordination_request": {
                "type": coordination_type,
                "tasks": tasks,
                "context": context
            }
        });

        let request = NoraRequest {
            request_id: format!("mcp-coord-{}", uuid::Uuid::new_v4()),
            session_id: format!("mcp-coord-{}", chrono::Utc::now().timestamp()),
            request_type: NoraRequestType::TaskCoordination,
            content: format!(
                "Coordinate {} tasks using {} strategy",
                tasks.len(),
                coordination_type
            ),
            context: Some(request_content),
            voice_enabled: false,
            priority: RequestPriority::Executive,
            timestamp: chrono::Utc::now(),
        };

        match self.nora_manager.process_request(request).await {
            Ok(response) => {
                let result = json!({
                    "coordination_result": response.content,
                    "executive_actions": response.actions,
                    "recommendations": response.follow_up_suggestions,
                    "processing_time_ms": response.processing_time_ms
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| "Coordination result serialization failed".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = json!({
                    "success": false,
                    "error": "Failed to coordinate tasks",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Task coordination error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Engage Nora in strategic planning sessions. She can analyze scenarios, provide recommendations, and help develop executive-level strategies."
    )]
    async fn nora_strategic_planning(
        &self,
        Parameters(NoraStrategicPlanningRequest {
            planning_scope,
            objectives,
            constraints,
            current_situation,
            success_metrics,
        }): Parameters<NoraStrategicPlanningRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let planning_context = json!({
            "planning_request": {
                "scope": planning_scope,
                "objectives": objectives,
                "constraints": constraints,
                "current_situation": current_situation,
                "success_metrics": success_metrics
            }
        });

        let request = NoraRequest {
            request_id: format!("mcp-strategy-{}", uuid::Uuid::new_v4()),
            session_id: format!("mcp-strategy-{}", chrono::Utc::now().timestamp()),
            request_type: NoraRequestType::StrategyPlanning,
            content: format!(
                "Strategic planning session for {} scope with {} objectives",
                planning_scope,
                objectives.len()
            ),
            context: Some(planning_context),
            voice_enabled: false,
            priority: RequestPriority::Executive,
            timestamp: chrono::Utc::now(),
        };

        match self.nora_manager.process_request(request).await {
            Ok(response) => {
                let result = json!({
                    "strategic_plan": response.content,
                    "executive_actions": response.actions,
                    "implementation_suggestions": response.follow_up_suggestions,
                    "context_updates": response.context_updates,
                    "processing_time_ms": response.processing_time_ms
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| "Strategic plan serialization failed".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = json!({
                    "success": false,
                    "error": "Failed to create strategic plan",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Strategic planning error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Request performance analysis from Nora. She can analyze team performance, project metrics, and provide executive insights."
    )]
    async fn nora_performance_analysis(
        &self,
        Parameters(NoraPerformanceAnalysisRequest {
            analysis_type,
            metrics,
            time_period,
            comparison_baseline,
            focus_areas,
        }): Parameters<NoraPerformanceAnalysisRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let analysis_context = json!({
            "analysis_request": {
                "type": analysis_type,
                "metrics": metrics,
                "time_period": time_period,
                "comparison_baseline": comparison_baseline,
                "focus_areas": focus_areas
            }
        });

        let request = NoraRequest {
            request_id: format!("mcp-analysis-{}", uuid::Uuid::new_v4()),
            session_id: format!("mcp-analysis-{}", chrono::Utc::now().timestamp()),
            request_type: NoraRequestType::PerformanceAnalysis,
            content: format!(
                "Performance analysis for {} over {}",
                analysis_type,
                time_period.as_deref().unwrap_or("current period")
            ),
            context: Some(analysis_context),
            voice_enabled: false,
            priority: RequestPriority::High,
            timestamp: chrono::Utc::now(),
        };

        match self.nora_manager.process_request(request).await {
            Ok(response) => {
                let result = json!({
                    "analysis_result": response.content,
                    "performance_insights": response.actions,
                    "recommendations": response.follow_up_suggestions,
                    "context_updates": response.context_updates,
                    "processing_time_ms": response.processing_time_ms
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| {
                        "Performance analysis serialization failed".to_string()
                    }),
                )]))
            }
            Err(e) => {
                let error_response = json!({
                    "success": false,
                    "error": "Failed to complete performance analysis",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Performance analysis error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Generate British executive assistant voice audio from text using Nora's voice engine."
    )]
    async fn nora_voice_synthesis(
        &self,
        Parameters(NoraVoiceSynthesisRequest {
            text,
            voice_settings,
            output_format,
        }): Parameters<NoraVoiceSynthesisRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let synthesis_context = json!({
            "voice_synthesis": {
                "text": text,
                "settings": voice_settings,
                "output_format": output_format.unwrap_or_else(|| "base64".to_string())
            }
        });

        let request = NoraRequest {
            request_id: format!("mcp-voice-{}", uuid::Uuid::new_v4()),
            session_id: format!("mcp-voice-{}", chrono::Utc::now().timestamp()),
            request_type: NoraRequestType::TextInteraction,
            content: text.clone(),
            context: Some(synthesis_context),
            voice_enabled: true,
            priority: RequestPriority::Normal,
            timestamp: chrono::Utc::now(),
        };

        match self.nora_manager.process_request(request).await {
            Ok(response) => {
                if let Some(voice_response) = response.voice_response {
                    let result = json!({
                        "success": true,
                        "audio_base64": voice_response,
                        "format": "wav",
                        "text": text,
                        "processing_time_ms": response.processing_time_ms
                    });

                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| {
                            "Voice synthesis result serialization failed".to_string()
                        }),
                    )]))
                } else {
                    let error_response = json!({
                        "success": false,
                        "error": "Voice synthesis was requested but no audio was generated"
                    });
                    Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&error_response)
                            .unwrap_or_else(|_| "Voice synthesis failed".to_string()),
                    )]))
                }
            }
            Err(e) => {
                let error_response = json!({
                    "success": false,
                    "error": "Failed to synthesize voice",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Voice synthesis error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Retrieve coordination system statistics and agent performance metrics from Nora's management system."
    )]
    async fn nora_coordination_stats(
        &self,
        Parameters(NoraCoordinationStatsRequest {
            include_agents,
            include_events,
            time_range,
        }): Parameters<NoraCoordinationStatsRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match self.nora_manager.get_coordination_stats().await {
            Ok(stats) => {
                let mut result = json!({
                    "success": true,
                    "stats": stats,
                    "time_range": time_range.unwrap_or_else(|| "24h".to_string())
                });

                if include_agents.unwrap_or(true) {
                    match self.nora_manager.get_all_agents().await {
                        Ok(agents) => {
                            result["agents"] = json!(agents);
                        }
                        Err(e) => {
                            result["agents_error"] =
                                json!(format!("Failed to fetch agents: {}", e));
                        }
                    }
                }

                if include_events.unwrap_or(false) {
                    // Events would be retrieved from coordination manager
                    result["events"] = json!("Events retrieval not implemented yet");
                }

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| "Coordination stats serialization failed".to_string()),
                )]))
            }
            Err(e) => {
                let error_response = json!({
                    "success": false,
                    "error": "Failed to retrieve coordination statistics",
                    "details": e.to_string()
                });
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&error_response)
                        .unwrap_or_else(|_| "Coordination stats error".to_string()),
                )]))
            }
        }
    }

    #[tool(
        description = "Look up the user's schedule from their connected calendars. Returns normalized events from `calendar_events` (synced from Google Calendar / Outlook every 15 min). Supports windows like 'today' / 'tomorrow' / 'this_week' / 'next_7_days', and an optional crm_contact_id filter to answer questions like 'when am I next meeting Acme?'."
    )]
    async fn nora_schedule(
        &self,
        Parameters(NoraScheduleRequest {
            organization_id,
            window,
            crm_contact_id,
            limit,
        }): Parameters<NoraScheduleRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        use db::{db_uuid::DbUuid, models::calendar_event::CalendarEvent};

        let limit = limit.unwrap_or(25).clamp(1, 100);

        // crm_contact_id branch ignores the time window — useful for
        // "when am I next meeting <person>?" type queries.
        if let Some(contact_id) = crm_contact_id.filter(|s| !s.is_empty()) {
            let Ok(uuid) = DbUuid::parse(&contact_id) else {
                return Ok(CallToolResult::error(vec![Content::text(
                    json!({ "success": false, "error": "Invalid crm_contact_id" }).to_string(),
                )]));
            };
            return match CalendarEvent::list_for_contact(&self.pool, &uuid, limit).await {
                Ok(events) => Ok(CallToolResult::success(vec![Content::text(
                    schedule_response(&events, "by_contact"),
                )])),
                Err(e) => Ok(CallToolResult::error(vec![Content::text(
                    json!({ "success": false, "error": e.to_string() }).to_string(),
                )])),
            };
        }

        let Ok(uuid) = DbUuid::parse(&organization_id) else {
            return Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": "Invalid organization_id" }).to_string(),
            )]));
        };

        let (from, to) = window_bounds(window.as_deref().unwrap_or("today"));
        match CalendarEvent::list_for_org(&self.pool, &uuid, Some(&from), Some(&to), limit).await {
            Ok(events) => Ok(CallToolResult::success(vec![Content::text(
                schedule_response(&events, "by_window"),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Run Nora's rapid prototyping playbook. She syncs live context, ensures the project exists, and produces an executive summary of next steps."
    )]
    async fn nora_rapid_playbook(
        &self,
        Parameters(NoraRapidPlaybookRequest {
            project_name,
            objectives,
            repo_hint,
            notes,
        }): Parameters<NoraRapidPlaybookRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let payload = RapidPlaybookRequest {
            project_name,
            objectives: objectives.unwrap_or_default(),
            repo_hint,
            notes,
        };

        match self.nora_manager.run_rapid_playbook(payload).await {
            Ok(result) => {
                let formatted = format_rapid_playbook(&result);
                Ok(CallToolResult::success(vec![Content::text(formatted)]))
            }
            Err(err) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Playbook failed: {}",
                err
            ))])),
        }
    }
}

fn format_rapid_playbook(result: &RapidPlaybookResult) -> String {
    let mut sections = Vec::new();
    sections.push("### Nora Rapid Prototyping Playbook".to_string());
    sections.push(format!(
        "Projects synced: {} | Created project: {}",
        result.projects_synced,
        if result.created_project { "yes" } else { "no" }
    ));
    if let Some(message) = &result.created_message {
        sections.push(format!("New project notes: {}", message));
    }
    sections.push("---".to_string());
    sections.push(result.summary.clone());
    sections.join("\n")
}

#[tool_handler]
impl ServerHandler for NoraServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "nora-executive-mcp".to_string(),
                version: "1.0.0".to_string(),
            },
            instructions: Some("Nora Executive Assistant MCP provides AI-powered executive functions. Available tools: 'nora_chat' (general conversation), 'nora_coordinate_tasks' (task management), 'nora_strategic_planning' (strategic analysis), 'nora_performance_analysis' (performance insights), 'nora_voice_synthesis' (British accent TTS), 'nora_coordination_stats' (system statistics), 'nora_schedule' (calendar lookup — what's on today / when am I next meeting <person>?), 'nora_rapid_playbook' (rapid prototyping). Nora is a professional British executive assistant who can help with strategic planning, task coordination, performance analysis, decision support, and scheduling.".to_string()),
        }
    }
}

/// Translate a friendly window keyword into ISO-8601 UTC `[from, to)` bounds.
/// Unknown keywords fall back to "today".
fn window_bounds(window: &str) -> (String, String) {
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let (start_date, end_date) = match window {
        "tomorrow" => {
            let d = today + chrono::Duration::days(1);
            (d, d + chrono::Duration::days(1))
        }
        "this_week" => {
            // ISO week starts Monday
            let weekday = today.weekday().num_days_from_monday() as i64;
            let monday = today - chrono::Duration::days(weekday);
            (monday, monday + chrono::Duration::days(7))
        }
        "next_7_days" => (today, today + chrono::Duration::days(7)),
        // "today" or anything else
        _ => (today, today + chrono::Duration::days(1)),
    };
    (
        format!("{start_date}T00:00:00Z"),
        format!("{end_date}T00:00:00Z"),
    )
}

/// Render a `CalendarEvent` slice into the JSON shape Nora's chat layer
/// expects: a tight summary plus the full rows for any follow-up logic.
fn schedule_response(events: &[db::models::calendar_event::CalendarEvent], mode: &str) -> String {
    let summary: Vec<Value> = events
        .iter()
        .map(|e| {
            json!({
                "id": e.id.as_str(),
                "title": e.title,
                "start_at": e.start_at,
                "end_at": e.end_at,
                "location": e.location,
                "meeting_url": e.meeting_url,
                "status": e.status,
                "provider": e.provider,
                "crm_contact_ids": serde_json::from_str::<Value>(&e.crm_contact_ids)
                    .unwrap_or_else(|_| json!([])),
            })
        })
        .collect();
    json!({
        "success": true,
        "mode": mode,
        "count": events.len(),
        "events": summary,
    })
    .to_string()
}
