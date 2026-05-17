use std::sync::Arc;

use chrono::{DateTime, Datelike, Utc};
use nora::agent::{
    NoraRequest, NoraRequestType, RapidPlaybookRequest, RapidPlaybookResult, RequestPriority,
};
use rmcp::{
    handler::server::tool::{Parameters, ToolRouter},
    model::{
        CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo,
    },
    schemars, tool, tool_handler, tool_router, ErrorData, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

/// MCP request: aggregated AR/AP financial summary for an organization
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraFinancialSummaryRequest {
    #[schemars(description = "Organization ID whose financials to aggregate")]
    pub organization_id: String,
}

/// MCP request: look up a contact's QBO/AR status by name, company, or email
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraQboLookupContactRequest {
    #[schemars(description = "Organization ID to scope the search")]
    pub organization_id: String,

    #[schemars(
        description = "Free-text search: matches against email, full name, first+last, or company name (case-insensitive substring)"
    )]
    pub query: String,

    #[schemars(description = "Maximum number of matching contacts to return (default 5, max 25)")]
    pub limit: Option<i32>,
}

/// MCP request: list connected social accounts (YouTube, LinkedIn, etc.) for a project
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraListSocialAccountsRequest {
    #[schemars(description = "Project UUID whose connected social accounts to list")]
    pub project_id: String,
}

/// MCP request: create + publish a social post in one shot
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraPostToSocialRequest {
    #[schemars(description = "Project UUID")]
    pub project_id: String,

    #[schemars(
        description = "Social account UUIDs to publish to. Discover them with nora_list_social_accounts."
    )]
    pub account_ids: Vec<String>,

    #[schemars(description = "Post caption / text body")]
    pub caption: String,

    #[schemars(
        description = "Content type: post (default), story, reel, carousel, thread, video, article"
    )]
    pub content_type: Option<String>,

    #[schemars(
        description = "Media URLs (images, video) to attach. YouTube uploads currently buffer the entire file in memory — keep videos under ~100MB."
    )]
    pub media_urls: Option<Vec<String>>,

    #[schemars(description = "Hashtags (without the leading `#`)")]
    pub hashtags: Option<Vec<String>>,

    #[schemars(description = "Usernames to @-mention (without the leading `@`)")]
    pub mentions: Option<Vec<String>>,

    #[schemars(
        description = "Platform-specific extras as JSON (e.g. {\"tiktok\":{\"privacy_level\":\"PUBLIC_TO_EVERYONE\"}})"
    )]
    pub platform_specific: Option<Value>,
}

/// MCP request: trigger a QBO sync for an organization's active QuickBooks account
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct NoraTriggerQboSyncRequest {
    #[schemars(description = "Organization ID whose QuickBooks account to sync")]
    pub organization_id: String,

    #[schemars(
        description = "Optional explicit quickbooks_account_id. If omitted, the org's first active account is used."
    )]
    pub quickbooks_account_id: Option<String>,
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
        description = "Aggregate AR/AP financial summary for an organization: outstanding receivables, overdue receivables, payables, 30/60/90-day revenue, and open/overdue invoice counts. Read from the local `invoices` table — reflects the state of the last QBO sync. Use this to answer 'how are we doing on cash?' / 'what's overdue?' style questions."
    )]
    async fn nora_financial_summary(
        &self,
        Parameters(NoraFinancialSummaryRequest { organization_id }): Parameters<
            NoraFinancialSummaryRequest,
        >,
    ) -> Result<CallToolResult, ErrorData> {
        let Ok(org_uuid) = uuid::Uuid::parse_str(&organization_id) else {
            return Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": "Invalid organization_id" }).to_string(),
            )]));
        };

        match services::services::quickbooks::financial_summary(&self.pool, org_uuid).await {
            Ok(summary) => Ok(CallToolResult::success(vec![Content::text(
                json!({ "success": true, "summary": summary }).to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Look up CRM contacts in an organization by name, company, or email and return each one's open AR balance, overdue total, and open invoice count. Use this for 'what's the AR balance for Acme?' / 'does Jane Doe owe us anything?' style questions. Returns up to `limit` matches ranked by largest outstanding balance."
    )]
    async fn nora_qbo_lookup_contact(
        &self,
        Parameters(NoraQboLookupContactRequest {
            organization_id,
            query,
            limit,
        }): Parameters<NoraQboLookupContactRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": "query must be non-empty" }).to_string(),
            )]));
        }

        let Ok(org_uuid) = uuid::Uuid::parse_str(&organization_id) else {
            return Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": "Invalid organization_id" }).to_string(),
            )]));
        };

        let limit = limit.unwrap_or(5).clamp(1, 25);

        match lookup_contact_balances(&self.pool, org_uuid, trimmed, limit).await {
            Ok(matches) => Ok(CallToolResult::success(vec![Content::text(
                json!({
                    "success": true,
                    "query": trimmed,
                    "count": matches.len(),
                    "matches": matches,
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Trigger a QuickBooks Online sync for an organization. Pulls Customers, Invoices, and Payments since the last sync into the local CRM/invoices tables. Use sparingly — sync also runs on a background timer. Returns counts of rows touched."
    )]
    async fn nora_trigger_qbo_sync(
        &self,
        Parameters(NoraTriggerQboSyncRequest {
            organization_id,
            quickbooks_account_id,
        }): Parameters<NoraTriggerQboSyncRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        use db::models::quickbooks_account::QuickBooksAccount;

        let Ok(org_uuid) = uuid::Uuid::parse_str(&organization_id) else {
            return Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": "Invalid organization_id" }).to_string(),
            )]));
        };

        let account_id = match quickbooks_account_id {
            Some(raw) => match uuid::Uuid::parse_str(&raw) {
                Ok(u) => u,
                Err(_) => {
                    return Ok(CallToolResult::error(vec![Content::text(
                        json!({ "success": false, "error": "Invalid quickbooks_account_id" })
                            .to_string(),
                    )]));
                }
            },
            None => match QuickBooksAccount::find_by_organization(&self.pool, org_uuid).await {
                Ok(accounts) => {
                    let active = accounts
                        .into_iter()
                        .find(|a| a.sync_enabled == 1 && a.status == "active");
                    match active {
                        Some(a) => a.id,
                        None => {
                            return Ok(CallToolResult::error(vec![Content::text(
                                json!({
                                    "success": false,
                                    "error": "No active QuickBooks account connected for this organization",
                                })
                                .to_string(),
                            )]));
                        }
                    }
                }
                Err(e) => {
                    return Ok(CallToolResult::error(vec![Content::text(
                        json!({ "success": false, "error": e.to_string() }).to_string(),
                    )]));
                }
            },
        };

        match services::services::quickbooks::run_qbo_sync(&self.pool, account_id).await {
            Ok(stats) => Ok(CallToolResult::success(vec![Content::text(
                json!({
                    "success": true,
                    "quickbooks_account_id": account_id.to_string(),
                    "stats": stats,
                })
                .to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(
        description = "List social media accounts (YouTube, LinkedIn, Instagram, Twitter, TikTok, Threads, Facebook, Bluesky, Pinterest) connected to a project. Returns each account's UUID, platform, username, and status. The returned UUIDs are what you pass as `account_ids` to nora_post_to_social. Always call this BEFORE nora_post_to_social so you target real accounts."
    )]
    async fn nora_list_social_accounts(
        &self,
        Parameters(NoraListSocialAccountsRequest { project_id }): Parameters<
            NoraListSocialAccountsRequest,
        >,
    ) -> Result<CallToolResult, ErrorData> {
        use db::models::social_account::SocialAccount;

        let Ok(project_uuid) = uuid::Uuid::parse_str(&project_id) else {
            return Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": "Invalid project_id" }).to_string(),
            )]));
        };

        match SocialAccount::find_by_project(&self.pool, project_uuid).await {
            Ok(accounts) => {
                let summary: Vec<Value> = accounts
                    .iter()
                    .map(|a| {
                        json!({
                            "id": a.id.to_string(),
                            "platform": a.platform,
                            "username": a.username,
                            "display_name": a.display_name,
                            "status": a.status,
                            "profile_url": a.profile_url,
                            "follower_count": a.follower_count,
                        })
                    })
                    .collect();
                Ok(CallToolResult::success(vec![Content::text(
                    json!({
                        "success": true,
                        "count": summary.len(),
                        "accounts": summary,
                    })
                    .to_string(),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": e.to_string() }).to_string(),
            )])),
        }
    }

    #[tool(
        description = "Create and publish a social post in one shot to the listed connected accounts (LinkedIn and YouTube fully wired; other platforms vary). The post is routed through the platform connector with stored OAuth tokens; per-platform results — including platform_post_id and platform_url — come back in the response. Returns success=false if every target account failed. Use nora_list_social_accounts first to discover account UUIDs."
    )]
    async fn nora_post_to_social(
        &self,
        Parameters(NoraPostToSocialRequest {
            project_id,
            account_ids,
            caption,
            content_type,
            media_urls,
            hashtags,
            mentions,
            platform_specific,
        }): Parameters<NoraPostToSocialRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match publish_now(
            &self.pool,
            &project_id,
            &account_ids,
            caption,
            content_type.as_deref(),
            media_urls,
            hashtags,
            mentions,
            platform_specific,
        )
        .await
        {
            Ok(value) => Ok(CallToolResult::success(vec![Content::text(
                value.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                json!({ "success": false, "error": e }).to_string(),
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
            instructions: Some("Nora Executive Assistant MCP provides AI-powered executive functions. Available tools: 'nora_chat' (general conversation), 'nora_coordinate_tasks' (task management), 'nora_strategic_planning' (strategic analysis), 'nora_performance_analysis' (performance insights), 'nora_voice_synthesis' (British accent TTS), 'nora_coordination_stats' (system statistics), 'nora_schedule' (calendar lookup — what's on today / when am I next meeting <person>?), 'nora_financial_summary' (org AR/AP + revenue trend from QuickBooks-synced data), 'nora_qbo_lookup_contact' (find a contact's AR balance by name/company/email), 'nora_trigger_qbo_sync' (force a QuickBooks pull), 'nora_list_social_accounts' (discover connected social handles for a project), 'nora_post_to_social' (create + publish a post in one shot to YouTube/LinkedIn/etc.), 'nora_rapid_playbook' (rapid prototyping). Nora is a professional British executive assistant who can help with strategic planning, task coordination, performance analysis, decision support, scheduling, financial reporting, and direct social publishing.".to_string()),
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

/// Parse the optional content_type string into the DB enum, defaulting to Post.
fn parse_content_type(s: Option<&str>) -> db::models::social_post::ContentType {
    use db::models::social_post::ContentType;
    match s.map(|v| v.to_lowercase()).as_deref() {
        Some("story") => ContentType::Story,
        Some("reel") => ContentType::Reel,
        Some("carousel") => ContentType::Carousel,
        Some("thread") => ContentType::Thread,
        Some("video") => ContentType::Video,
        Some("article") => ContentType::Article,
        _ => ContentType::Post,
    }
}

/// Create a SocialPost then immediately route it through Publisher. Returns a
/// JSON value matching the shape Nora's chat layer expects, or an error string.
#[allow(clippy::too_many_arguments)]
async fn publish_now(
    pool: &SqlitePool,
    project_id: &str,
    account_ids: &[String],
    caption: String,
    content_type: Option<&str>,
    media_urls: Option<Vec<String>>,
    hashtags: Option<Vec<String>>,
    mentions: Option<Vec<String>>,
    platform_specific: Option<Value>,
) -> Result<Value, String> {
    use db::models::social_post::{CreateSocialPost, SocialPost};
    use services::services::social::Publisher;

    if account_ids.is_empty() {
        return Err("account_ids must contain at least one social account UUID".into());
    }

    let project_uuid =
        uuid::Uuid::parse_str(project_id).map_err(|_| "Invalid project_id".to_string())?;

    let account_uuids: Vec<uuid::Uuid> = account_ids
        .iter()
        .map(|s| uuid::Uuid::parse_str(s).map_err(|_| format!("Invalid account UUID: {s}")))
        .collect::<Result<_, _>>()?;

    let post = SocialPost::create(
        pool,
        CreateSocialPost {
            project_id: project_uuid,
            social_account_id: None,
            task_id: None,
            content_type: Some(parse_content_type(content_type)),
            caption: Some(caption),
            content_blocks: None,
            media_urls,
            hashtags,
            mentions,
            platforms: account_uuids,
            platform_specific,
            scheduled_for: None,
            category: None,
            is_evergreen: None,
            recycle_after_days: None,
            created_by_agent_id: None,
        },
    )
    .await
    .map_err(|e| format!("Failed to create social post: {e}"))?;

    let publisher = Publisher::new(pool.clone());
    match publisher.publish_post(post.id).await {
        Ok(results) => {
            let entries: Vec<Value> = results
                .iter()
                .map(|r| {
                    json!({
                        "platform": r.platform.to_string(),
                        "platform_post_id": r.platform_post_id,
                        "platform_url": r.platform_url,
                        "published_at": r.published_at.to_rfc3339(),
                    })
                })
                .collect();
            Ok(json!({
                "success": !entries.is_empty(),
                "post_id": post.id.to_string(),
                "results": entries,
            }))
        }
        Err(e) => Err(format!(
            "Post created (id={}) but publish failed: {e}",
            post.id
        )),
    }
}

/// One row from the contact-balance lookup: identity columns + AR aggregates.
type ContactBalanceRow = (
    String,         // id
    Option<String>, // full_name
    Option<String>, // first_name
    Option<String>, // last_name
    Option<String>, // email
    Option<String>, // company_name
    Option<f64>,    // ar_outstanding
    Option<f64>,    // ar_overdue
    Option<i64>,    // open_count
    Option<i64>,    // overdue_count
);

/// Find org contacts matching `query` (email / name / company, case-insensitive
/// substring) and aggregate their AR exposure from the local `invoices` table.
async fn lookup_contact_balances(
    pool: &SqlitePool,
    organization_id: uuid::Uuid,
    query: &str,
    limit: i32,
) -> Result<Vec<Value>, sqlx::Error> {
    let org_text = organization_id.to_string();
    let needle = format!("%{}%", query.to_lowercase());

    let rows: Vec<ContactBalanceRow> = sqlx::query_as(
        r#"
        SELECT
            c.id,
            c.full_name,
            c.first_name,
            c.last_name,
            c.email,
            c.company_name,
            COALESCE(SUM(CASE WHEN i.invoice_type = 'ar' AND i.status IN ('sent','viewed','partial','overdue') THEN i.amount_usd ELSE 0.0 END), 0.0) AS ar_outstanding,
            COALESCE(SUM(CASE WHEN i.invoice_type = 'ar' AND i.status = 'overdue' THEN i.amount_usd ELSE 0.0 END), 0.0) AS ar_overdue,
            COALESCE(SUM(CASE WHEN i.invoice_type = 'ar' AND i.status IN ('sent','viewed','partial','overdue') THEN 1 ELSE 0 END), 0) AS open_count,
            COALESCE(SUM(CASE WHEN i.invoice_type = 'ar' AND i.status = 'overdue' THEN 1 ELSE 0 END), 0) AS overdue_count
        FROM crm_contacts c
        LEFT JOIN invoices i ON i.crm_contact_id = c.id
        WHERE c.organization_id = ?1
        AND (
            LOWER(COALESCE(c.email, '')) LIKE ?2
            OR LOWER(COALESCE(c.full_name, '')) LIKE ?2
            OR LOWER(COALESCE(c.first_name, '') || ' ' || COALESCE(c.last_name, '')) LIKE ?2
            OR LOWER(COALESCE(c.company_name, '')) LIKE ?2
        )
        GROUP BY c.id
        ORDER BY ar_outstanding DESC, c.full_name ASC
        LIMIT ?3
        "#,
    )
    .bind(&org_text)
    .bind(&needle)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let matches = rows
        .into_iter()
        .map(|r| {
            json!({
                "contact_id": r.0,
                "full_name": r.1,
                "first_name": r.2,
                "last_name": r.3,
                "email": r.4,
                "company_name": r.5,
                "ar_outstanding_usd": r.6.unwrap_or(0.0),
                "ar_overdue_usd": r.7.unwrap_or(0.0),
                "open_invoice_count": r.8.unwrap_or(0),
                "overdue_invoice_count": r.9.unwrap_or(0),
            })
        })
        .collect();
    Ok(matches)
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
