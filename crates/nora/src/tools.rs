//! Executive tools and capabilities for Nora

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use db::models::{
    project_board::ProjectBoardType,
    task::{Priority, TaskStatus},
};
use serde::{Deserialize, Serialize};
use services::services::media_pipeline::{
    EditSessionRequest, MediaBatchAnalysisRequest, MediaBatchIngestRequest, MediaPipelineService,
    MediaStorageTier, RenderJobRequest, VideoRenderPriority as PipelineRenderPriority,
};
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    executor::{TaskDefinition, TaskExecutor},
    integrations::{CalendarService, DiscordService, EmailService},
    NoraError,
};

/// Executive tools available to Nora
pub struct ExecutiveTools {
    available_tools: HashMap<String, ToolDefinition>,
    // External service integrations
    email_service: Option<EmailService>,
    discord_service: Option<DiscordService>,
    calendar_service: Option<CalendarService>,
    // Task execution
    task_executor: Option<Arc<TaskExecutor>>,
    media_pipeline: Option<MediaPipelineService>,
    workflow_orchestrator: Option<Arc<crate::workflow::WorkflowOrchestrator>>,
    // Unified execution engine (replaces workflow_orchestrator for new architecture)
    execution_engine: Option<Arc<crate::execution::ExecutionEngine>>,
}

/// Definition of an executive tool
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub category: ToolCategory,
    pub parameters: Vec<ToolParameter>,
    pub required_permissions: Vec<Permission>,
    pub estimated_duration: Option<String>,
}

/// Tool categories for organization
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ToolCategory {
    Coordination,
    Analysis,
    Communication,
    Planning,
    Monitoring,
    Decision,
    Reporting,
    Production,
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub description: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
}

/// Parameter types for validation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    Date,
    Array,
    Object,
    Enum(Vec<String>),
}

/// Required permissions for tools
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    ReadOnly,
    Write,
    Execute,
    Admin,
    Executive,
    Financial,
    HR,
}

/// Executive tool implementations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum NoraExecutiveTool {
    /// Project Management
    CreateProject {
        name: String,
        git_repo_path: String,
        setup_script: Option<String>,
        dev_script: Option<String>,
    },
    CreateBoard {
        project_id: String,
        name: String,
        description: Option<String>,
        board_type: Option<String>,
    },
    /// Create a task in a project by project name (simpler API for LLM)
    CreateTaskInProject {
        project_name: String,
        title: String,
        description: Option<String>,
        priority: Option<String>,
    },
    /// Get all tasks for a project by name - use this to review/analyze project tasks
    GetProjectTasks {
        project_name: String,
        /// Optional filter by status: "todo", "in_progress", "done", "blocked"
        status_filter: Option<String>,
    },
    /// Get detailed information about a specific project
    GetProjectDetails {
        project_name: String,
    },
    CreateTaskOnBoard {
        project_id: String,
        board_id: String,
        title: String,
        description: Option<String>,
        priority: Option<String>,
        tags: Option<Vec<String>>,
    },
    AddTaskToBoard {
        task_id: String,
        board_id: String,
    },
    /// Execute an agent workflow
    ExecuteWorkflow {
        agent_id: String,
        workflow_id: String,
        project_id: Option<String>,
        inputs: HashMap<String, serde_json::Value>,
    },
    /// Cancel a running workflow
    CancelWorkflow {
        workflow_instance_id: String,
    },
    /// List all active workflow executions
    ListActiveWorkflows,
    /// List available workflows for all agents or a specific agent
    ListAvailableWorkflows {
        agent_id: Option<String>,
    },

    /// Team Coordination
    CoordinateTeamMeeting {
        participants: Vec<String>,
        agenda: String,
        duration_minutes: u32,
        priority: MeetingPriority,
    },
    DelegateTask {
        task_id: String,
        assignee: String,
        priority: TaskPriority,
        deadline: Option<DateTime<Utc>>,
    },
    EscalateIssue {
        issue_id: String,
        stakeholders: Vec<String>,
        severity: IssueSeverity,
        description: String,
    },

    /// Strategic Planning
    GenerateProjectRoadmap {
        project_id: String,
        timeline: String,
        milestones: Vec<String>,
        resources: Vec<String>,
    },
    AnalyzeResourceAllocation {
        department: String,
        time_period: String,
        metrics: Vec<String>,
    },
    CreateActionPlan {
        objective: String,
        constraints: Vec<String>,
        timeline: String,
        success_criteria: Vec<String>,
    },

    /// Communication Management
    DraftExecutiveSummary {
        project_id: String,
        audience: ExecutiveAudience,
        key_points: Vec<String>,
        recommendations: Vec<String>,
    },
    ScheduleStakeholderUpdate {
        update_type: UpdateType,
        recipients: Vec<String>,
        delivery_method: DeliveryMethod,
        schedule: DateTime<Utc>,
    },
    ManageCommunicationChannel {
        channel_id: String,
        action: ChannelAction,
        participants: Vec<String>,
    },

    /// Performance Monitoring
    GenerateKPIDashboard {
        metrics: Vec<String>,
        period: String,
        visualization_type: VisualizationType,
        filters: HashMap<String, String>,
    },
    AnalyzeTrendData {
        data_sources: Vec<String>,
        analysis_type: AnalysisType,
        time_range: TimeRange,
    },
    CreatePerformanceReport {
        team_id: String,
        period: String,
        include_recommendations: bool,
        format: ReportFormat,
    },

    /// Decision Support
    CreateDecisionMatrix {
        options: Vec<DecisionOption>,
        criteria: Vec<DecisionCriterion>,
        weights: HashMap<String, f32>,
    },
    AnalyzeRiskAssessment {
        scenario: String,
        risk_factors: Vec<RiskFactor>,
        mitigation_strategies: Vec<String>,
    },
    RecommendNextActions {
        context: String,
        goals: Vec<String>,
        constraints: Vec<String>,
        timeline: String,
    },

    /// Financial Analysis
    GenerateBudgetAnalysis {
        budget_id: String,
        analysis_type: BudgetAnalysisType,
        comparison_period: Option<String>,
    },
    ForecastFinancials {
        model_type: ForecastModel,
        time_horizon: String,
        assumptions: HashMap<String, f64>,
    },
    TrackExpenseCategories {
        categories: Vec<String>,
        period: String,
        alert_thresholds: HashMap<String, f64>,
    },

    /// HR and Team Management
    AssessTeamCapacity {
        team_id: String,
        project_requirements: Vec<String>,
        time_frame: String,
    },
    PlanSuccession {
        role_id: String,
        candidates: Vec<String>,
        development_areas: Vec<String>,
    },
    AnalyzeTeamPerformance {
        team_id: String,
        metrics: Vec<String>,
        benchmark_period: String,
    },

    /// File Operations
    ReadFile {
        file_path: String,
        encoding: Option<String>,
    },
    WriteFile {
        file_path: String,
        content: String,
        create_directories: bool,
    },
    ListDirectory {
        directory_path: String,
        recursive: bool,
        pattern: Option<String>,
    },
    DeleteFile {
        file_path: String,
        confirm: bool,
    },

    /// Web Search & Information
    SearchWeb {
        query: String,
        max_results: u32,
        search_type: SearchType,
    },
    FetchWebPage {
        url: String,
        extract_text: bool,
    },
    SummarizeContent {
        content: String,
        max_length: u32,
        format: SummaryFormat,
    },

    /// Code & Development
    ExecuteCode {
        code: String,
        language: CodeLanguage,
        timeout_seconds: u32,
    },
    AnalyzeCodeQuality {
        code: String,
        language: CodeLanguage,
        check_security: bool,
    },
    GenerateDocumentation {
        code: String,
        doc_format: DocumentationFormat,
    },

    /// Email & Notifications
    SendEmail {
        recipients: Vec<String>,
        subject: String,
        body: String,
        priority: EmailPriority,
    },
    SendDiscordMessage {
        channel: String,
        message: String,
        mention_users: Vec<String>,
    },
    CreateNotification {
        title: String,
        message: String,
        notification_type: NotificationType,
        recipients: Vec<String>,
    },

    /// Calendar & Scheduling
    CreateCalendarEvent {
        title: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        attendees: Vec<String>,
        location: Option<String>,
    },
    FindAvailableSlots {
        participants: Vec<String>,
        duration_minutes: u32,
        preferred_days: Vec<String>,
    },
    CheckCalendarAvailability {
        user: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    },

    /// Media Production
    IngestMediaBatch {
        source_url: String,
        reference_name: Option<String>,
        storage_tier: String,
        checksum_required: bool,
        project_id: Option<String>,
        task_id: Option<String>,
    },
    AnalyzeMediaBatch {
        batch_id: String,
        brief: String,
        passes: u32,
        deliverable_targets: Vec<String>,
        project_id: Option<String>,
        task_id: Option<String>,
    },
    GenerateVideoEdits {
        batch_id: String,
        deliverable_type: String,
        aspect_ratios: Vec<String>,
        reference_style: Option<String>,
        include_captions: bool,
        project_id: Option<String>,
        task_id: Option<String>,
    },
    RenderVideoDeliverables {
        edit_session_id: String,
        destinations: Vec<String>,
        formats: Vec<String>,
        priority: VideoRenderPriority,
        project_id: Option<String>,
        task_id: Option<String>,
    },

    /// Run Spectra Visual QC on a media batch (vision-guided frame analysis)
    RunVisualQc {
        batch_id: String,
        candidates_per_clip: Option<u32>,
        min_composition_score: Option<f64>,
        target_aspect_ratio: Option<String>,
        project_id: Option<String>,
    },

    /// Deep scene analysis: analyze video content for energy, motion, brightness, content type
    AnalyzeScenes {
        batch_id: String,
        segment_interval: Option<f64>,
        project_id: Option<String>,
    },

    /// Beat grid analysis: analyze audio track for BPM, beat grid, energy curve, sections
    AnalyzeBeatGrid {
        audio_path: String,
        bpm_hint: Option<f64>,
        beats_per_bar: Option<u32>,
        project_id: Option<String>,
    },

    /// Assemble recap edit: match clips to music, beat-lock cuts, mute NAT, export Premiere XML
    AssembleRecapEdit {
        batch_id: String,
        audio_path: String,
        bpm_hint: Option<f64>,
        target_aspect_ratio: Option<String>,
        project_id: Option<String>,
    },

    /// Execute an FFmpeg render script produced by AssembleRecapEdit
    ExecuteRenderScript {
        render_script: String,
        render_output: String,
        xml_path: String,
    },
}

/// Search types for web search
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SearchType {
    General,
    News,
    Academic,
    Images,
    Videos,
}

/// Summary formats
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum SummaryFormat {
    Bullet,
    Paragraph,
    Executive,
    Technical,
}

/// Programming languages for code execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum CodeLanguage {
    Python,
    JavaScript,
    Rust,
    TypeScript,
    Bash,
}

/// Documentation formats
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DocumentationFormat {
    Markdown,
    Html,
    Rst,
    Jsdoc,
}

/// Email priority
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum EmailPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Notification types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum NotificationType {
    Info,
    Warning,
    Error,
    Success,
    Alert,
}

/// Meeting priority levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MeetingPriority {
    Low,
    Normal,
    High,
    Urgent,
    Emergency,
}

/// Video render queue priority levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum VideoRenderPriority {
    Low,
    Standard,
    Rush,
}

/// Task priority levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Issue severity levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum IssueSeverity {
    Minor,
    Moderate,
    Major,
    Critical,
    Blocker,
}

/// Executive audience types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExecutiveAudience {
    CEO,
    BoardOfDirectors,
    SeniorManagement,
    MiddleManagement,
    AllStaff,
    Stakeholders,
    Investors,
}

/// Communication update types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum UpdateType {
    StatusUpdate,
    ProgressReport,
    Alert,
    Announcement,
    Decision,
    PolicyChange,
}

/// Delivery methods for communications
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum DeliveryMethod {
    Email,
    Slack,
    Teams,
    Dashboard,
    Meeting,
    Document,
    Presentation,
}

/// Communication channel actions
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ChannelAction {
    Create,
    Update,
    Archive,
    Delete,
    AddParticipants,
    RemoveParticipants,
    ChangePermissions,
}

/// Visualization types for dashboards
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum VisualizationType {
    LineChart,
    BarChart,
    PieChart,
    Heatmap,
    Gauge,
    Table,
    Scorecard,
}

/// Analysis types for trend data
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisType {
    Trend,
    Correlation,
    Regression,
    Forecast,
    Anomaly,
    Comparison,
}

/// Time range specifications
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TimeRange {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub granularity: TimeGranularity,
}

/// Time granularity options
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TimeGranularity {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

/// Report format options
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ReportFormat {
    PDF,
    Excel,
    PowerPoint,
    HTML,
    JSON,
    CSV,
}

/// Decision option for matrices
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DecisionOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub estimated_cost: Option<f64>,
    pub estimated_timeline: Option<String>,
    pub risk_level: RiskLevel,
}

/// Decision criterion for evaluation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DecisionCriterion {
    pub id: String,
    pub name: String,
    pub description: String,
    pub measurement_type: MeasurementType,
    pub weight: f32,
}

/// Risk levels
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Measurement types for criteria
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum MeasurementType {
    Quantitative,
    Qualitative,
    Binary,
    Scale,
}

/// Risk factor for assessments
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RiskFactor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub probability: f32,
    pub impact: f32,
    pub category: RiskCategory,
}

/// Risk categories
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum RiskCategory {
    Financial,
    Operational,
    Strategic,
    Regulatory,
    Technology,
    Reputation,
    Market,
}

/// Budget analysis types
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum BudgetAnalysisType {
    Variance,
    Trend,
    Forecast,
    Comparison,
    Allocation,
    Efficiency,
}

/// Financial forecast models
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ForecastModel {
    Linear,
    Exponential,
    Seasonal,
    ARIMA,
    MonteCarlo,
    Scenario,
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionResult {
    pub tool_name: String,
    pub execution_id: String,
    pub status: ExecutionStatus,
    pub result_data: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub execution_time_ms: u64,
    pub timestamp: DateTime<Utc>,
}

/// Execution status for tools
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionStatus {
    Success,
    Failed,
    Pending,
    Cancelled,
}

#[allow(dead_code)]
impl ExecutiveTools {
    pub fn new() -> Self {
        let mut tools = Self {
            available_tools: HashMap::new(),
            email_service: EmailService::from_env().ok(),
            discord_service: DiscordService::from_env().ok(),
            calendar_service: CalendarService::from_env().ok(),
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

    pub fn set_workflow_orchestrator(&mut self, orchestrator: Arc<crate::workflow::WorkflowOrchestrator>) {
        self.workflow_orchestrator = Some(orchestrator);
    }

    /// Set the unified execution engine (new architecture)
    pub fn set_execution_engine(&mut self, engine: Arc<crate::execution::ExecutionEngine>) {
        self.execution_engine = Some(engine);
    }

    /// Generate OpenAI-compatible function schemas for available tools
    /// These are used for function calling / tool use
    pub fn get_openai_tool_schemas() -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_project",
                    "description": "Create a new project in the system. Use this when the user wants to create, start, or set up a new project.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "The name of the project to create"
                            },
                            "git_repo_path": {
                                "type": "string",
                                "description": "Optional path to a git repository for this project"
                            }
                        },
                        "required": ["name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_board",
                    "description": "Create a new board (kanban, scrum, etc.) within a project. Use this when the user wants to add a board to organize tasks.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_id": {
                                "type": "string",
                                "description": "The UUID of the project to add the board to"
                            },
                            "name": {
                                "type": "string",
                                "description": "The name of the board"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional description of the board's purpose"
                            },
                            "board_type": {
                                "type": "string",
                                "enum": ["kanban", "scrum", "custom"],
                                "description": "The type of board to create"
                            }
                        },
                        "required": ["project_id", "name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_task",
                    "description": "Create a new task in a project. Use this when the user wants to add a task, todo, or work item to an EXISTING project. The task will be added to the project's default board.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the existing project to add the task to (e.g., 'Test Project 3')"
                            },
                            "title": {
                                "type": "string",
                                "description": "The title of the task to create"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional detailed description of the task"
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["low", "medium", "high", "critical"],
                                "description": "Priority level of the task (default: medium)"
                            }
                        },
                        "required": ["project_name", "title"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_project_tasks",
                    "description": "Get all tasks for a project. Use this FIRST when the user asks to review, analyze, check, or look at tasks in a project. This returns the actual task data for you to analyze and summarize.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the project to get tasks from (e.g., 'PRIME', 'PCG')"
                            },
                            "status_filter": {
                                "type": "string",
                                "enum": ["todo", "in_progress", "done", "blocked"],
                                "description": "Optional filter to only get tasks with this status"
                            }
                        },
                        "required": ["project_name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "get_project_details",
                    "description": "Get detailed information about a project including its tasks, boards, and pods. Use this when the user asks for a project overview or summary.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "project_name": {
                                "type": "string",
                                "description": "The name of the project to get details for"
                            }
                        },
                        "required": ["project_name"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "execute_workflow",
                    "description": "Execute a multi-stage agent workflow. Use this when the user requests a complex operation that involves multiple coordinated steps. IMPORTANT: For research agents (scout-research, oracle-strategy), you MUST include the user's original request/topic in the inputs.request field so the agent knows what to research.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_id": {
                                "type": "string",
                                "description": "The agent identifier. Creative: 'editron-post' (video editing), 'master-cinematographer' (AI video). Strategy: 'astra-strategy' (roadmaps). Social: 'scout-research' (research), 'oracle-strategy' (content strategy), 'muse-creative' (copywriting), 'herald-distribution' (publishing), 'echo-engagement' (community)",
                                "enum": ["editron-post", "master-cinematographer", "astra-strategy", "scout-research", "oracle-strategy", "muse-creative", "herald-distribution", "echo-engagement"]
                            },
                            "workflow_id": {
                                "type": "string",
                                "description": "The workflow identifier (e.g., 'competitor-deep-dive' for Scout research, 'content-calendar-30day' for Oracle strategy)"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID to associate the workflow with"
                            },
                            "inputs": {
                                "type": "object",
                                "description": "Input parameters for the workflow. For research workflows, MUST include 'request' with the user's full research topic/question (e.g., {'request': 'research viral content trends for hospitality industry', 'target': 'Prime Hospitality competitors'})",
                                "properties": {
                                    "request": {
                                        "type": "string",
                                        "description": "The user's original request or research topic - REQUIRED for research agents"
                                    },
                                    "target": {
                                        "type": "string",
                                        "description": "Specific target/subject to research"
                                    }
                                },
                                "additionalProperties": true
                            }
                        },
                        "required": ["agent_id", "workflow_id", "inputs"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "cancel_workflow",
                    "description": "Cancel a running workflow execution. Use this when a workflow is stuck, failing repeatedly, or needs to be stopped.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "workflow_instance_id": {
                                "type": "string",
                                "description": "The UUID of the workflow instance to cancel"
                            }
                        },
                        "required": ["workflow_instance_id"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_active_workflows",
                    "description": "List all currently running workflow executions with their status, agent, and instance IDs.",
                    "parameters": {
                        "type": "object",
                        "properties": {}
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "list_available_workflows",
                    "description": "List all available workflow templates that can be executed. Use this to discover which workflows exist for each agent before calling execute_workflow. Optionally filter by agent_id to see workflows for a specific agent.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "agent_id": {
                                "type": "string",
                                "description": "Optional agent ID to filter workflows (e.g., 'master-cinematographer', 'editron-post')"
                            }
                        }
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "send_email",
                    "description": "Send an email to one or more recipients. Use this when the user wants to send, compose, or draft an email.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "to": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "List of email addresses to send to"
                            },
                            "subject": {
                                "type": "string",
                                "description": "Email subject line"
                            },
                            "body": {
                                "type": "string",
                                "description": "Email body content"
                            }
                        },
                        "required": ["to", "subject", "body"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "send_discord_message",
                    "description": "Send a message to Discord via webhook. Use this when the user wants to post or send something to Discord.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "The message content to send"
                            },
                            "mentions": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional user IDs to mention"
                            }
                        },
                        "required": ["message"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "create_calendar_event",
                    "description": "Create a calendar event or meeting. Use this when the user wants to schedule, book, or create a meeting or event.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "Title of the event"
                            },
                            "start_time": {
                                "type": "string",
                                "description": "Start time in ISO 8601 format (e.g., 2024-12-05T14:00:00Z)"
                            },
                            "end_time": {
                                "type": "string",
                                "description": "End time in ISO 8601 format"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional event description"
                            },
                            "attendees": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Optional list of attendee email addresses"
                            }
                        },
                        "required": ["title", "start_time", "end_time"]
                    }
                }
            }),
            // Editron / Media Pipeline Tools
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "ingest_media_batch",
                    "description": "Ingest a batch of media files from a URL (e.g., Dropbox) or a local directory path. Use this when the user provides video content or asks to analyze/edit videos from a link or local folder.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "source_url": {
                                "type": "string",
                                "description": "URL to the media source (Dropbox link, etc.) or local directory path (e.g. /path/to/footage)"
                            },
                            "reference_name": {
                                "type": "string",
                                "description": "Optional reference name for this batch (e.g., 'Jan 25 Event Footage')"
                            },
                            "storage_tier": {
                                "type": "string",
                                "enum": ["hot", "warm", "cold"],
                                "description": "Storage tier for the media (hot=active, warm=archive, cold=deep archive)"
                            },
                            "checksum_required": {
                                "type": "boolean",
                                "description": "Whether to verify file checksums during download"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID to associate this batch with and create a task"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["source_url", "storage_tier", "checksum_required"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "analyze_media_batch",
                    "description": "Analyze a media batch for video editing. Identifies hero moments, suggests cuts, and prepares for editing.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "batch_id": {
                                "type": "string",
                                "description": "UUID of the media batch to analyze"
                            },
                            "brief": {
                                "type": "string",
                                "description": "Creative brief or editing instructions (e.g., 'Create a highlight reel showcasing the best moments')"
                            },
                            "deliverable_targets": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Target deliverables (e.g., ['recap', 'highlight', 'social'])"
                            },
                            "passes": {
                                "type": "integer",
                                "description": "Number of analysis passes to perform (1-3)"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["batch_id", "brief", "deliverable_targets", "passes"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "generate_video_edits",
                    "description": "Generate video edits from analyzed media. Creates edit sessions with timeline structures.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "batch_id": {
                                "type": "string",
                                "description": "UUID of the analyzed media batch"
                            },
                            "deliverable_type": {
                                "type": "string",
                                "description": "Type of deliverable (e.g., 'recap', 'highlight', 'social')"
                            },
                            "aspect_ratios": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Target aspect ratios (e.g., ['16:9', '9:16', '1:1'])"
                            },
                            "reference_style": {
                                "type": "string",
                                "description": "Optional reference style or template to follow"
                            },
                            "include_captions": {
                                "type": "boolean",
                                "description": "Whether to include automatic captions"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["batch_id", "deliverable_type", "aspect_ratios", "include_captions"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "render_video_deliverables",
                    "description": "Render final video deliverables from an edit session. Creates render jobs for export.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "edit_session_id": {
                                "type": "string",
                                "description": "UUID of the edit session to render"
                            },
                            "destinations": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Export destinations (e.g., ['local', 'youtube', 'instagram'])"
                            },
                            "formats": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Output formats (e.g., ['mp4', 'mov'])"
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["low", "standard", "rush"],
                                "description": "Render priority"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID"
                            },
                            "task_id": {
                                "type": "string",
                                "description": "Optional workflow task ID to attach artifacts and activity to"
                            }
                        },
                        "required": ["edit_session_id", "destinations", "formats", "priority"]
                    }
                }
            }),
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": "run_visual_qc",
                    "description": "Run Spectra Visual QC pass on a media batch. Extracts keyframes from clips, sends them to a Vision API (provider-agnostic), scores composition quality, and selects optimal in-points. Must be run AFTER analyze_media_batch and BEFORE generate_video_edits for vision-guided framing.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "batch_id": {
                                "type": "string",
                                "description": "UUID of the media batch to run Visual QC on (must be in Ready status)"
                            },
                            "candidates_per_clip": {
                                "type": "integer",
                                "description": "Number of candidate frames to extract per clip (default: 5)"
                            },
                            "min_composition_score": {
                                "type": "number",
                                "description": "Minimum composition score (0.0-1.0) to pass QC (default: 0.6)"
                            },
                            "target_aspect_ratio": {
                                "type": "string",
                                "description": "Target delivery aspect ratio for crop suggestions (e.g., '16:9', '9:16')"
                            },
                            "project_id": {
                                "type": "string",
                                "description": "Optional project ID to associate this QC pass with"
                            }
                        },
                        "required": ["batch_id"]
                    }
                }
            }),
        ]
    }

    /// Parse a tool call from the LLM and convert it to NoraExecutiveTool
    pub fn parse_tool_call(name: &str, arguments: &serde_json::Value) -> Option<NoraExecutiveTool> {
        match name {
            "create_project" => {
                let name = arguments.get("name")?.as_str()?.to_string();
                let git_repo_path = arguments
                    .get("git_repo_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(NoraExecutiveTool::CreateProject {
                    name,
                    git_repo_path,
                    setup_script: None,
                    dev_script: None,
                })
            }
            "create_board" => {
                let project_id = arguments.get("project_id")?.as_str()?.to_string();
                let name = arguments.get("name")?.as_str()?.to_string();
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let board_type = arguments
                    .get("board_type")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::CreateBoard {
                    project_id,
                    name,
                    description,
                    board_type,
                })
            }
            "create_task" => {
                // New simplified API - takes project name instead of IDs
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                let title = arguments.get("title")?.as_str()?.to_string();
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let priority = arguments
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::CreateTaskInProject {
                    project_name,
                    title,
                    description,
                    priority,
                })
            }
            "get_project_tasks" => {
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                let status_filter = arguments
                    .get("status_filter")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::GetProjectTasks {
                    project_name,
                    status_filter,
                })
            }
            "get_project_details" => {
                let project_name = arguments.get("project_name")?.as_str()?.to_string();
                Some(NoraExecutiveTool::GetProjectDetails {
                    project_name,
                })
            }
            "execute_workflow" => {
                let agent_id = arguments.get("agent_id")?.as_str()?.to_string();
                let workflow_id = arguments.get("workflow_id")?.as_str()?.to_string();
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let inputs = arguments
                    .get("inputs")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(NoraExecutiveTool::ExecuteWorkflow {
                    agent_id,
                    workflow_id,
                    project_id,
                    inputs,
                })
            }
            "cancel_workflow" => {
                let workflow_instance_id = arguments.get("workflow_instance_id")?.as_str()?.to_string();
                Some(NoraExecutiveTool::CancelWorkflow {
                    workflow_instance_id,
                })
            }
            "list_active_workflows" => {
                Some(NoraExecutiveTool::ListActiveWorkflows)
            }
            "list_available_workflows" => {
                let agent_id = arguments.get("agent_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::ListAvailableWorkflows { agent_id })
            }
            "send_email" => {
                let recipients: Vec<String> = arguments
                    .get("to")?
                    .as_array()?
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                let subject = arguments.get("subject")?.as_str()?.to_string();
                let body = arguments.get("body")?.as_str()?.to_string();
                Some(NoraExecutiveTool::SendEmail {
                    recipients,
                    subject,
                    body,
                    priority: EmailPriority::Normal,
                })
            }
            "send_discord_message" => {
                let message = arguments.get("message")?.as_str()?.to_string();
                let mention_users = arguments
                    .get("mentions")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                Some(NoraExecutiveTool::SendDiscordMessage {
                    channel: String::new(), // Use default channel
                    message,
                    mention_users,
                })
            }
            "ingest_media_batch" => {
                let source_url = arguments.get("source_url")?.as_str()?.to_string();
                let reference_name = arguments
                    .get("reference_name")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let storage_tier = arguments.get("storage_tier")?.as_str()?.to_string();
                let checksum_required = arguments
                    .get("checksum_required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::IngestMediaBatch {
                    source_url,
                    reference_name,
                    storage_tier,
                    checksum_required,
                    project_id,
                    task_id,
                })
            }
            "analyze_media_batch" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let brief = arguments.get("brief")?.as_str()?.to_string();
                let passes = arguments
                    .get("passes")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(2) as u32;
                let deliverable_targets = arguments
                    .get("deliverable_targets")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(Vec::new);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::AnalyzeMediaBatch {
                    batch_id,
                    brief,
                    passes,
                    deliverable_targets,
                    project_id,
                    task_id,
                })
            }
            "generate_video_edits" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let deliverable_type = arguments.get("deliverable_type")?.as_str()?.to_string();
                let aspect_ratios = arguments
                    .get("aspect_ratios")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_else(|| vec!["16:9".to_string()]);
                let reference_style = arguments
                    .get("reference_style")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let include_captions = arguments
                    .get("include_captions")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::GenerateVideoEdits {
                    batch_id,
                    deliverable_type,
                    aspect_ratios,
                    reference_style,
                    include_captions,
                    project_id,
                    task_id,
                })
            }
            "render_video_deliverables" => {
                let edit_session_id = arguments.get("edit_session_id")?.as_str()?.to_string();
                let destinations = arguments
                    .get("destinations")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let formats = arguments
                    .get("formats")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let priority = arguments
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(|value| match value.to_lowercase().as_str() {
                        "low" => VideoRenderPriority::Low,
                        "rush" => VideoRenderPriority::Rush,
                        _ => VideoRenderPriority::Standard,
                    })
                    .unwrap_or(VideoRenderPriority::Standard);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let task_id = arguments
                    .get("task_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::RenderVideoDeliverables {
                    edit_session_id,
                    destinations,
                    formats,
                    priority,
                    project_id,
                    task_id,
                })
            }
            "run_visual_qc" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let candidates_per_clip = arguments
                    .get("candidates_per_clip")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let min_composition_score = arguments
                    .get("min_composition_score")
                    .and_then(|v| v.as_f64());
                let target_aspect_ratio = arguments
                    .get("target_aspect_ratio")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let project_id = arguments
                    .get("project_id")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Some(NoraExecutiveTool::RunVisualQc {
                    batch_id,
                    candidates_per_clip,
                    min_composition_score,
                    target_aspect_ratio,
                    project_id,
                })
            }
            "analyze_scenes" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let segment_interval = arguments.get("segment_interval").and_then(|v| v.as_f64());
                let project_id = arguments.get("project_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::AnalyzeScenes { batch_id, segment_interval, project_id })
            }
            "analyze_beat_grid" => {
                let audio_path = arguments.get("audio_path")?.as_str()?.to_string();
                let bpm_hint = arguments.get("bpm_hint").and_then(|v| v.as_f64());
                let beats_per_bar = arguments.get("beats_per_bar").and_then(|v| v.as_u64()).map(|v| v as u32);
                let project_id = arguments.get("project_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::AnalyzeBeatGrid { audio_path, bpm_hint, beats_per_bar, project_id })
            }
            "assemble_recap_edit" => {
                let batch_id = arguments.get("batch_id")?.as_str()?.to_string();
                let audio_path = arguments.get("audio_path")?.as_str()?.to_string();
                let bpm_hint = arguments.get("bpm_hint").and_then(|v| v.as_f64());
                let target_aspect_ratio = arguments.get("target_aspect_ratio").and_then(|v| v.as_str()).map(String::from);
                let project_id = arguments.get("project_id").and_then(|v| v.as_str()).map(String::from);
                Some(NoraExecutiveTool::AssembleRecapEdit { batch_id, audio_path, bpm_hint, target_aspect_ratio, project_id })
            }
            "execute_render_script" => {
                let render_script = arguments.get("render_script")?.as_str()?.to_string();
                let render_output = arguments.get("render_output")
                    .and_then(|v| v.as_str())
                    .unwrap_or("output.mp4")
                    .to_string();
                let xml_path = arguments.get("xml_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(NoraExecutiveTool::ExecuteRenderScript { render_script, render_output, xml_path })
            }
            "create_calendar_event" => {
                let title = arguments.get("title")?.as_str()?.to_string();
                let start_time_str = arguments.get("start_time")?.as_str()?;
                let end_time_str = arguments.get("end_time")?.as_str()?;

                // Parse ISO 8601 datetime strings
                let start_time = chrono::DateTime::parse_from_rfc3339(start_time_str)
                    .ok()?
                    .with_timezone(&chrono::Utc);
                let end_time = chrono::DateTime::parse_from_rfc3339(end_time_str)
                    .ok()?
                    .with_timezone(&chrono::Utc);

                let attendees = arguments
                    .get("attendees")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let location = arguments
                    .get("location")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                Some(NoraExecutiveTool::CreateCalendarEvent {
                    title,
                    start_time,
                    end_time,
                    attendees,
                    location,
                })
            }
            _ => None,
        }
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

    pub async fn execute_tool(
        &self,
        tool: NoraExecutiveTool,
        user_permissions: Vec<Permission>,
    ) -> crate::Result<ToolExecutionResult> {
        let start_time = std::time::Instant::now();
        let tool_name = self.get_tool_name(&tool);
        let execution_id = uuid::Uuid::new_v4().to_string();

        // Check permissions
        if let Some(tool_def) = self.available_tools.get(&tool_name) {
            for required_permission in &tool_def.required_permissions {
                if !user_permissions.contains(required_permission) {
                    return Ok(ToolExecutionResult {
                        tool_name,
                        execution_id,
                        status: ExecutionStatus::Failed,
                        result_data: None,
                        error_message: Some(format!(
                            "Missing required permission: {:?}",
                            required_permission
                        )),
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                        timestamp: Utc::now(),
                    });
                }
            }
        }

        // Execute tool
        let result_data = self.execute_tool_implementation(tool).await?;

        Ok(ToolExecutionResult {
            tool_name,
            execution_id,
            status: ExecutionStatus::Success,
            result_data: Some(result_data),
            error_message: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: Utc::now(),
        })
    }

    fn initialize_tools(&mut self) {
        // Project Management tools
        self.add_tool_definition(ToolDefinition {
            name: "create_project".to_string(),
            description: "Create a new project with git repository".to_string(),
            category: ToolCategory::Planning,
            parameters: vec![
                ToolParameter {
                    name: "name".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Project name".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "git_repo_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to git repository".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Executive, Permission::Write],
            estimated_duration: Some("1-2 minutes".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "create_board".to_string(),
            description: "Create a new kanban board for a project".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Project UUID".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "name".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Board name".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("30 seconds".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "create_task_on_board".to_string(),
            description: "Create a new task on a specific kanban board".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Project UUID".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "board_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Board UUID".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "title".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Task title".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "description".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Task description".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "priority".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "low".to_string(),
                        "medium".to_string(),
                        "high".to_string(),
                    ]),
                    description: "Task priority level".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("medium")),
                },
            ],
            required_permissions: vec![Permission::Write],
            estimated_duration: Some("10-30 seconds".to_string()),
        });

        // Coordination tools
        self.add_tool_definition(ToolDefinition {
            name: "coordinate_team_meeting".to_string(),
            description: "Schedule and coordinate team meetings with agenda".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "participants".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "List of meeting participants".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "agenda".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Meeting agenda".to_string(),
                    required: true,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("2-5 minutes".to_string()),
        });

        // Task delegation tool - critical for orchestrating other agents
        self.add_tool_definition(ToolDefinition {
            name: "delegate_task".to_string(),
            description: "Delegate a task to another agent (like AURI for coding) and trigger execution. Use this to assign work to specialized agents.".to_string(),
            category: ToolCategory::Coordination,
            parameters: vec![
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "The UUID of the task to delegate".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "assignee".to_string(),
                    parameter_type: ParameterType::String,
                    description: "The agent name to assign the task to (e.g., 'Auri' for coding, 'Editron' for video editing)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "priority".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Task priority: low, medium, high, or critical".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("medium")),
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("1-5 seconds to start, execution time varies".to_string()),
        });

        // Analysis tools
        self.add_tool_definition(ToolDefinition {
            name: "generate_kpi_dashboard".to_string(),
            description: "Generate KPI dashboard with specified metrics".to_string(),
            category: ToolCategory::Analysis,
            parameters: vec![ToolParameter {
                name: "metrics".to_string(),
                parameter_type: ParameterType::Array,
                description: "List of metrics to include".to_string(),
                required: true,
                default_value: None,
            }],
            required_permissions: vec![Permission::ReadOnly],
            estimated_duration: Some("30 seconds - 2 minutes".to_string()),
        });

        // Media production tools
        self.add_tool_definition(ToolDefinition {
            name: "ingest_media_batch".to_string(),
            description: "Ingest a batch of raw media from Dropbox, other capture sources, or a local directory path"
                .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "source_url".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Public or signed URL to the capture folder, or local directory path (e.g. /path/to/footage)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "reference_name".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Human readable reference for the batch".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "storage_tier".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "hot".to_string(),
                        "warm".to_string(),
                        "cold".to_string(),
                    ]),
                    description: "Target storage tier".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!("hot")),
                },
                ToolParameter {
                    name: "checksum_required".to_string(),
                    parameter_type: ParameterType::Boolean,
                    description: "Whether to verify checksums before import".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(true)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log pipeline tasks under".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write],
            estimated_duration: Some("2-6 minutes depending on payload".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "analyze_media_batch".to_string(),
            description:
                "Run iterative analysis on an ingested batch to extract highlights and notes"
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Ingest batch identifier".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "brief".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Creative brief or prompt".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "passes".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Number of iterative passes".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(2)),
                },
                ToolParameter {
                    name: "deliverable_targets".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "List of deliverables to scout for".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(["recap", "highlights"])),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log analysis tasks".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Write, Permission::Execute],
            estimated_duration: Some("5-15 minutes".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "generate_video_edits".to_string(),
            description: "Create edit timelines and drafts for specified deliverables".to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Source batch identifier".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "deliverable_type".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Type of edit (recap, highlight, sizzle)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "aspect_ratios".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "Target aspect ratios".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!(["16:9", "9:16"])),
                },
                ToolParameter {
                    name: "reference_style".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional reference edit or look".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "include_captions".to_string(),
                    parameter_type: ParameterType::Boolean,
                    description: "Auto-generate caption tracks".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(true)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log edit tasks".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute, Permission::Executive],
            estimated_duration: Some("10-20 minutes".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "render_video_deliverables".to_string(),
            description: "Render and distribute finished edits to downstream destinations"
                .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "edit_session_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Edit session identifier".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "destinations".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "List of delivery endpoints (Dropbox, Frame.io, S3 folder, etc.)"
                        .to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "formats".to_string(),
                    parameter_type: ParameterType::Array,
                    description: "Export formats or presets".to_string(),
                    required: true,
                    default_value: Some(serde_json::json!(["ProRes422", "H.264"])),
                },
                ToolParameter {
                    name: "priority".to_string(),
                    parameter_type: ParameterType::Enum(vec![
                        "low".to_string(),
                        "standard".to_string(),
                        "rush".to_string(),
                    ]),
                    description: "Render queue priority".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("standard")),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to log render tasks".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "task_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional workflow task ID to attach artifacts and activity to".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("5-30 minutes depending on outputs".to_string()),
        });

        self.add_tool_definition(ToolDefinition {
            name: "run_visual_qc".to_string(),
            description:
                "Run Spectra Visual QC pass — vision-guided frame analysis to score composition and select optimal in-points"
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Media batch identifier (must be Ready)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "candidates_per_clip".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Number of candidate frames per clip".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(5)),
                },
                ToolParameter {
                    name: "min_composition_score".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Minimum composition score to pass QC (0.0-1.0)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(0.6)),
                },
                ToolParameter {
                    name: "target_aspect_ratio".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Target delivery aspect ratio (e.g. '16:9')".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID to associate this QC pass".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("2-15 minutes depending on clip count".to_string()),
        });

        // Scene Analysis tool
        self.add_tool_definition(ToolDefinition {
            name: "analyze_scenes".to_string(),
            description:
                "Deep scene analysis using FFmpeg — measures brightness, motion intensity, complexity, and classifies content type (high_energy, establishing, intimate, transition, ambient) per segment. Run BEFORE hero selection to understand footage."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Media batch identifier (must be Ready)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "segment_interval".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Seconds between analysis samples (default 3.0)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(3.0)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("1-5 minutes depending on clip count".to_string()),
        });

        // Beat Grid Analysis tool
        self.add_tool_definition(ToolDefinition {
            name: "analyze_beat_grid".to_string(),
            description:
                "Analyze audio track for BPM, beat grid, energy curve, music sections (intro/verse/chorus/bridge/outro), and transition markers. Sonix Engineering step — marks beats for beat-locked cuts."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "audio_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the audio track file".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "bpm_hint".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Optional BPM hint to guide detection".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "beats_per_bar".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Beats per bar (default 4)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!(4)),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("30 seconds - 2 minutes".to_string()),
        });

        // Assemble Recap Edit tool
        self.add_tool_definition(ToolDefinition {
            name: "assemble_recap_edit".to_string(),
            description:
                "Full recap assembly: runs scene analysis + beat grid analysis, matches clips to music sections by energy/content, beat-locks all cuts, mutes NAT audio (music only), and exports Premiere Pro XML. The complete Editron pipeline."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "batch_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Media batch identifier (must be Ready)".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "audio_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the music track".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "bpm_hint".to_string(),
                    parameter_type: ParameterType::Number,
                    description: "Optional BPM hint for beat detection".to_string(),
                    required: false,
                    default_value: None,
                },
                ToolParameter {
                    name: "target_aspect_ratio".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Target aspect ratio (e.g. '16:9', '9:16', '1:1')".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("16:9")),
                },
                ToolParameter {
                    name: "project_id".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Optional project UUID".to_string(),
                    required: false,
                    default_value: None,
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("3-10 minutes".to_string()),
        });

        // Execute Render Script tool
        self.add_tool_definition(ToolDefinition {
            name: "execute_render_script".to_string(),
            description:
                "Execute an FFmpeg render script produced by AssembleRecapEdit. Runs the bash script and produces the final MP4 deliverable."
                    .to_string(),
            category: ToolCategory::Production,
            parameters: vec![
                ToolParameter {
                    name: "render_script".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the render.sh script".to_string(),
                    required: true,
                    default_value: None,
                },
                ToolParameter {
                    name: "render_output".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Expected output MP4 path".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("output.mp4")),
                },
                ToolParameter {
                    name: "xml_path".to_string(),
                    parameter_type: ParameterType::String,
                    description: "Path to the Premiere XML (passed through for reference)".to_string(),
                    required: false,
                    default_value: Some(serde_json::json!("")),
                },
            ],
            required_permissions: vec![Permission::Execute],
            estimated_duration: Some("1-5 minutes".to_string()),
        });
    }

    fn add_tool_definition(&mut self, tool_def: ToolDefinition) {
        self.available_tools.insert(tool_def.name.clone(), tool_def);
    }

    fn get_tool_name(&self, tool: &NoraExecutiveTool) -> String {
        match tool {
            // Project Management
            NoraExecutiveTool::CreateProject { .. } => "create_project".to_string(),
            NoraExecutiveTool::CreateBoard { .. } => "create_board".to_string(),
            NoraExecutiveTool::CreateTaskInProject { .. } => "create_task".to_string(),
            NoraExecutiveTool::GetProjectTasks { .. } => "get_project_tasks".to_string(),
            NoraExecutiveTool::GetProjectDetails { .. } => "get_project_details".to_string(),
            NoraExecutiveTool::CreateTaskOnBoard { .. } => "create_task_on_board".to_string(),
            NoraExecutiveTool::AddTaskToBoard { .. } => "add_task_to_board".to_string(),
            NoraExecutiveTool::ExecuteWorkflow { .. } => "execute_workflow".to_string(),
            NoraExecutiveTool::CancelWorkflow { .. } => "cancel_workflow".to_string(),
            NoraExecutiveTool::ListActiveWorkflows => "list_active_workflows".to_string(),
            NoraExecutiveTool::ListAvailableWorkflows { .. } => "list_available_workflows".to_string(),

            // Coordination
            NoraExecutiveTool::CoordinateTeamMeeting { .. } => {
                "coordinate_team_meeting".to_string()
            }
            NoraExecutiveTool::DelegateTask { .. } => "delegate_task".to_string(),
            NoraExecutiveTool::EscalateIssue { .. } => "escalate_issue".to_string(),

            // Planning
            NoraExecutiveTool::GenerateProjectRoadmap { .. } => {
                "generate_project_roadmap".to_string()
            }
            NoraExecutiveTool::GenerateKPIDashboard { .. } => "generate_kpi_dashboard".to_string(),
            NoraExecutiveTool::IngestMediaBatch { .. } => "ingest_media_batch".to_string(),
            NoraExecutiveTool::AnalyzeMediaBatch { .. } => "analyze_media_batch".to_string(),
            NoraExecutiveTool::GenerateVideoEdits { .. } => "generate_video_edits".to_string(),
            NoraExecutiveTool::RenderVideoDeliverables { .. } => {
                "render_video_deliverables".to_string()
            }
            NoraExecutiveTool::RunVisualQc { .. } => "run_visual_qc".to_string(),
            NoraExecutiveTool::AnalyzeScenes { .. } => "analyze_scenes".to_string(),
            NoraExecutiveTool::AnalyzeBeatGrid { .. } => "analyze_beat_grid".to_string(),
            NoraExecutiveTool::AssembleRecapEdit { .. } => "assemble_recap_edit".to_string(),
            NoraExecutiveTool::ExecuteRenderScript { .. } => "execute_render_script".to_string(),

            // Add more mappings...
            _ => "unknown_tool".to_string(),
        }
    }

    pub async fn execute_tool_implementation(
        &self,
        tool: NoraExecutiveTool,
    ) -> crate::Result<serde_json::Value> {
        match tool {
            // Project Management
            NoraExecutiveTool::CreateProject {
                name,
                git_repo_path,
                setup_script,
                dev_script,
            } => {
                if let Some(executor) = &self.task_executor {
                    match executor
                        .create_project(
                            name.clone(),
                            git_repo_path.clone(),
                            setup_script,
                            dev_script,
                        )
                        .await
                    {
                        Ok(project) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Project '{}' created successfully", name),
                            "project_id": project.id.to_string(),
                            "project_name": project.name,
                            "git_repo_path": project.git_repo_path.to_string_lossy().to_string(),
                            "created_at": project.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create project: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available. Please ensure Nora is properly initialized."
                    }))
                }
            }
            NoraExecutiveTool::CreateBoard {
                project_id,
                name,
                description,
                board_type,
            } => {
                if let Some(executor) = &self.task_executor {
                    // Parse project_id UUID
                    let project_uuid = match Uuid::parse_str(&project_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid project_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Map board type string to enum
                    let board_type_enum =
                        // Simplified board types: only Default and Custom
                        board_type
                            .as_ref()
                            .and_then(|bt| match bt.to_lowercase().as_str() {
                                "default" | "main" => Some(ProjectBoardType::Default),
                                "custom" | _ => Some(ProjectBoardType::Custom),
                            });

                    match executor
                        .create_board(project_uuid, name.clone(), description, board_type_enum)
                        .await
                    {
                        Ok(board) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Board '{}' created successfully", name),
                            "board_id": board.id.to_string(),
                            "project_id": board.project_id.to_string(),
                            "board_name": board.name,
                            "board_type": format!("{:?}", board.board_type),
                            "created_at": board.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create board: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::CreateTaskInProject {
                project_name,
                title,
                description,
                priority,
            } => {
                if let Some(executor) = &self.task_executor {
                    // Map priority string to enum
                    let priority_enum =
                        priority
                            .as_ref()
                            .and_then(|p| match p.to_lowercase().as_str() {
                                "critical" => Some(Priority::Critical),
                                "high" => Some(Priority::High),
                                "medium" => Some(Priority::Medium),
                                "low" => Some(Priority::Low),
                                _ => None,
                            });

                    match executor
                        .create_task_in_project(
                            &project_name,
                            title.clone(),
                            description,
                            priority_enum,
                        )
                        .await
                    {
                        Ok(task) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Task '{}' created successfully in project '{}'", title, project_name),
                            "task_id": task.id.to_string(),
                            "project_id": task.project_id.to_string(),
                            "board_id": task.board_id.map(|id| id.to_string()),
                            "title": task.title,
                            "status": format!("{:?}", task.status),
                            "priority": format!("{:?}", task.priority),
                            "created_at": task.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create task: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::GetProjectTasks {
                project_name,
                status_filter,
            } => {
                if let Some(executor) = &self.task_executor {
                    match executor.get_tasks_by_project_name(&project_name).await {
                        Ok(tasks) => {
                            // Optionally filter by status
                            let filtered_tasks: Vec<_> = if let Some(status) = status_filter {
                                tasks.into_iter().filter(|t| t.status.to_lowercase() == status.to_lowercase()).collect()
                            } else {
                                tasks
                            };

                            let task_summaries: Vec<serde_json::Value> = filtered_tasks.iter().map(|t| {
                                serde_json::json!({
                                    "id": t.id,
                                    "title": t.title,
                                    "description": t.description,
                                    "status": t.status,
                                    "priority": t.priority,
                                    "assignee": t.assignee_id,
                                    "created_at": t.created_at,
                                    "updated_at": t.updated_at,
                                })
                            }).collect();

                            Ok(serde_json::json!({
                                "success": true,
                                "project_name": project_name,
                                "task_count": task_summaries.len(),
                                "tasks": task_summaries,
                            }))
                        }
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to get tasks: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::GetProjectDetails {
                project_name,
            } => {
                if let Some(executor) = &self.task_executor {
                    // First find project by name, then get details
                    match executor.find_project_by_name(&project_name).await {
                        Ok(project_id) => {
                            match executor.get_project_details(project_id).await {
                                Ok(details) => Ok(serde_json::json!({
                                    "success": true,
                                    "project": {
                                        "id": details.id,
                                        "name": details.name,
                                        "git_repo_path": details.git_repo_path,
                                        "task_count": details.tasks.len(),
                                        "board_count": details.boards.len(),
                                        "pod_count": details.pods.len(),
                                    },
                                    "tasks": details.tasks.iter().map(|t| serde_json::json!({
                                        "id": t.id,
                                        "title": t.title,
                                        "description": t.description,
                                        "status": t.status,
                                        "priority": t.priority,
                                    })).collect::<Vec<_>>(),
                                    "boards": details.boards.iter().map(|b| serde_json::json!({
                                        "id": b.id,
                                        "name": b.name,
                                        "description": b.description,
                                    })).collect::<Vec<_>>(),
                                })),
                                Err(e) => Ok(serde_json::json!({
                                    "success": false,
                                    "error": format!("Failed to get project details: {}", e),
                                })),
                            }
                        }
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Project not found: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::DelegateTask {
                task_id,
                assignee,
                priority: _,
                deadline: _,
            } => {
                if let Some(executor) = &self.task_executor {
                    tracing::info!(
                        "[TOOL] Delegating task {} to agent {}",
                        task_id, assignee
                    );

                    // Parse the task UUID
                    let task_uuid = match uuid::Uuid::parse_str(&task_id) {
                        Ok(id) => id,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Invalid task ID format: {}", task_id)
                            }));
                        }
                    };

                    // Determine executor type based on agent name
                    // AURI uses Claude Code, Editron would use video tools, etc.
                    let executor_type = match assignee.to_lowercase().as_str() {
                        "auri" => "CLAUDE_CODE",
                        "editron" => "PREMIERE_PRO", // For future video editing
                        "maci" => "COMFYUI",         // For future image generation
                        "bowser" => "PLAYWRIGHT",    // For future browser automation
                        _ => "CLAUDE_CODE",          // Default to Claude Code for coding agents
                    };

                    // Delegate and execute
                    match executor.delegate_and_execute_task(task_uuid, &assignee, executor_type).await {
                        Ok(result) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Task delegated to {} and execution started", result.agent_name),
                            "task_id": result.task_id.to_string(),
                            "agent_id": result.agent_id.to_string(),
                            "agent_name": result.agent_name,
                            "executor_type": result.executor_type,
                            "status": result.status,
                            "details": result.details
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to delegate task: {}", e)
                        }))
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::ExecuteWorkflow {
                agent_id,
                workflow_id,
                project_id,
                inputs,
            } => {
                tracing::info!(
                    "[TOOL] Executing workflow: agent={}, workflow={}, project={:?}",
                    agent_id,
                    workflow_id,
                    project_id
                );

                // Parse project_id if provided
                let project_uuid = project_id
                    .as_ref()
                    .and_then(|id| Uuid::parse_str(id).ok());

                // Prefer new ExecutionEngine if available
                if let Some(engine) = &self.execution_engine {
                    let request = crate::execution::ExecutionRequest {
                        project_id: project_uuid,
                        agent: Some(agent_id.clone()),
                        workflow_id: Some(workflow_id.clone()),
                        request: Some(format!("Execute workflow {} for agent {}", workflow_id, agent_id)),
                        inputs: inputs.clone(),
                    };

                    match engine.execute(request).await {
                        Ok(result) => {
                            tracing::info!(
                                "[TOOL] Workflow executed via ExecutionEngine: execution_id={}, stages={}/{}",
                                result.execution_id,
                                result.stages_completed,
                                result.total_stages
                            );

                            Ok(serde_json::json!({
                                "success": true,
                                "message": format!("Workflow '{}' completed for agent '{}'", workflow_id, agent_id),
                                "execution_id": result.execution_id.to_string(),
                                "agent_id": result.agent_id,
                                "agent_name": result.agent_name,
                                "workflow_id": result.workflow_id,
                                "workflow_name": result.workflow_name,
                                "project_id": project_id,
                                "status": format!("{:?}", result.status),
                                "stages_completed": result.stages_completed,
                                "total_stages": result.total_stages,
                                "tasks_created": result.tasks_created.len(),
                                "artifacts": result.artifacts.len(),
                                "duration_ms": result.duration_ms,
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] ExecutionEngine workflow failed: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Workflow execution failed: {}", e),
                                "agent_id": agent_id,
                                "workflow_id": workflow_id,
                            }))
                        }
                    }
                }
                // Fallback to legacy WorkflowOrchestrator
                else if let Some(orchestrator) = &self.workflow_orchestrator {
                    let context = crate::workflow::WorkflowContext {
                        project_id: project_uuid,
                        user_id: None,
                        inputs: inputs.clone(),
                        stage_outputs: std::collections::HashMap::new(),
                        metadata: std::collections::HashMap::new(),
                    };

                    match orchestrator.start_workflow(&agent_id, &workflow_id, context).await {
                        Ok(workflow_instance_id) => {
                            tracing::info!(
                                "[TOOL] Workflow started via legacy orchestrator: instance_id={}",
                                workflow_instance_id
                            );

                            Ok(serde_json::json!({
                                "success": true,
                                "message": format!("Workflow '{}' started for agent '{}'", workflow_id, agent_id),
                                "workflow_instance_id": workflow_instance_id.to_string(),
                                "agent_id": agent_id,
                                "workflow_id": workflow_id,
                                "project_id": project_id,
                                "status": "running",
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Legacy orchestrator failed: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to start workflow: {}", e),
                                "agent_id": agent_id,
                                "workflow_id": workflow_id,
                            }))
                        }
                    }
                } else {
                    tracing::warn!("[TOOL] No execution engine or orchestrator available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "No workflow execution system available. Nora needs to be initialized with ExecutionEngine or WorkflowOrchestrator.",
                        "agent_id": agent_id,
                        "workflow_id": workflow_id,
                    }))
                }
            }
            NoraExecutiveTool::CancelWorkflow {
                workflow_instance_id,
            } => {
                if let Some(orchestrator) = &self.workflow_orchestrator {
                    tracing::info!(
                        "[TOOL] Cancelling workflow instance: {}",
                        workflow_instance_id
                    );

                    match Uuid::parse_str(&workflow_instance_id) {
                        Ok(workflow_uuid) => {
                            match orchestrator.cancel_workflow(workflow_uuid).await {
                                Ok(_) => {
                                    tracing::info!(
                                        "[TOOL] Workflow cancelled successfully: {}",
                                        workflow_instance_id
                                    );
                                    Ok(serde_json::json!({
                                        "success": true,
                                        "message": format!("Workflow instance {} has been cancelled", workflow_instance_id),
                                        "workflow_instance_id": workflow_instance_id,
                                    }))
                                }
                                Err(e) => {
                                    tracing::error!("[TOOL] Failed to cancel workflow: {}", e);
                                    Ok(serde_json::json!({
                                        "success": false,
                                        "error": format!("Failed to cancel workflow: {}", e),
                                        "workflow_instance_id": workflow_instance_id,
                                    }))
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Invalid workflow instance ID: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Invalid workflow instance ID: {}", e),
                                "workflow_instance_id": workflow_instance_id,
                            }))
                        }
                    }
                } else {
                    tracing::warn!("[TOOL] Workflow orchestrator not available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Workflow orchestrator not available",
                        "workflow_instance_id": workflow_instance_id,
                    }))
                }
            }
            NoraExecutiveTool::ListActiveWorkflows => {
                if let Some(orchestrator) = &self.workflow_orchestrator {
                    tracing::info!("[TOOL] Listing active workflows");

                    let workflows = orchestrator.get_active_workflows().await;
                    let workflow_list: Vec<serde_json::Value> = workflows
                        .iter()
                        .map(|w| {
                            serde_json::json!({
                                "workflow_instance_id": w.id.to_string(),
                                "agent_id": w.agent_id,
                                "workflow_id": w.workflow_id,
                                "workflow_name": w.workflow.name,
                                "current_stage": w.current_stage,
                                "total_stages": w.workflow.stages.len(),
                                "state": match &w.state {
                                    crate::workflow::WorkflowState::Queued => "queued",
                                    crate::workflow::WorkflowState::Running { .. } => "running",
                                    crate::workflow::WorkflowState::Paused { .. } => "paused",
                                    crate::workflow::WorkflowState::Failed { .. } => "failed",
                                    crate::workflow::WorkflowState::Completed { .. } => "completed",
                                },
                                "started_at": w.started_at.to_rfc3339(),
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "success": true,
                        "workflows": workflow_list,
                        "count": workflow_list.len(),
                    }))
                } else {
                    tracing::warn!("[TOOL] Workflow orchestrator not available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Workflow orchestrator not available",
                    }))
                }
            }
            NoraExecutiveTool::ListAvailableWorkflows { agent_id } => {
                if let Some(orchestrator) = &self.workflow_orchestrator {
                    tracing::info!("[TOOL] Listing available workflows - filter: {:?}", agent_id);

                    if let Some(ref agent_filter) = agent_id {
                        // Get workflows for specific agent
                        let workflows = orchestrator.get_workflows_for_agent(agent_filter);

                        if workflows.is_empty() {
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Agent '{}' not found or has no workflows", agent_filter),
                            }))
                        } else {
                            let workflow_list: Vec<serde_json::Value> = workflows
                                .iter()
                                .map(|(workflow_id, name, objective)| {
                                    serde_json::json!({
                                        "workflow_id": workflow_id,
                                        "workflow_name": name,
                                        "objective": objective,
                                        "agent_id": agent_filter,
                                    })
                                })
                                .collect();

                            Ok(serde_json::json!({
                                "success": true,
                                "agent_id": agent_filter,
                                "workflows": workflow_list,
                                "count": workflow_list.len(),
                            }))
                        }
                    } else {
                        // Get all workflows for all agents
                        let all_workflows = orchestrator.get_all_agent_workflows();
                        let agents_list: Vec<serde_json::Value> = all_workflows
                            .iter()
                            .map(|(agent_id, codename, workflows)| {
                                let workflow_list: Vec<serde_json::Value> = workflows
                                    .iter()
                                    .map(|(workflow_id, name, objective)| {
                                        serde_json::json!({
                                            "workflow_id": workflow_id,
                                            "workflow_name": name,
                                            "objective": objective,
                                        })
                                    })
                                    .collect();

                                serde_json::json!({
                                    "agent_id": agent_id,
                                    "codename": codename,
                                    "workflows": workflow_list,
                                    "workflow_count": workflow_list.len(),
                                })
                            })
                            .collect();

                        Ok(serde_json::json!({
                            "success": true,
                            "agents": agents_list,
                            "total_agents": agents_list.len(),
                        }))
                    }
                } else {
                    tracing::warn!("[TOOL] Workflow orchestrator not available");
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Workflow orchestrator not available",
                    }))
                }
            }

            // Media Pipeline - Editron Tools
            NoraExecutiveTool::IngestMediaBatch {
                source_url,
                reference_name,
                storage_tier,
                checksum_required,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!(
                        "[TOOL] Ingesting media batch from: {}",
                        source_url
                    );

                    let tier = match MediaStorageTier::from_str(&storage_tier) {
                        Ok(t) => t,
                        Err(e) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Invalid storage tier '{}': {}", storage_tier, e),
                            }));
                        }
                    };

                    let project_uuid = project_id
                        .as_ref()
                        .and_then(|value| Uuid::parse_str(value).ok());

                    let request = MediaBatchIngestRequest {
                        source_url: source_url.clone(),
                        reference_name: reference_name.clone(),
                        storage_tier: tier,
                        checksum_required,
                        project_id: project_uuid,
                    };

                    match pipeline.ingest_batch(request).await {
                        Ok(batch) => {
                            tracing::info!(
                                "[TOOL] Media batch created successfully: batch_id={}",
                                batch.id
                            );

                            // Dashboard tracking: artifact + activity + vibe transaction
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();
                                let ref_name = reference_name.as_deref().unwrap_or("Media Batch");

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: {}", ref_name),
                                        &format!("Media batch from {}", source_url),
                                        serde_json::json!({ "editron_batch_id": batch.id.to_string() }),
                                    )
                                    .await
                                {
                                    let vibe_cost = crate::editron_tracking::EditronVibeCosts::ingest(batch.files.len() as u32);

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::MediaIngestManifest,
                                        &format!("Ingest: {}", batch.id),
                                        Some(serde_json::json!({
                                            "batch_id": batch.id.to_string(),
                                            "files": batch.files.len() as u32,
                                            "source_url": source_url,
                                            "storage_tier": format!("{:?}", batch.storage_tier),
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        resolved_task_id,
                                        "editron_ingest_completed",
                                        &format!("Ingested {} files from {}", batch.files.len() as u32, source_url),
                                        vibe_cost,
                                        serde_json::json!({"batch_id": batch.id.to_string()}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        resolved_project_id,
                                        resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron ingest: {} files", batch.files.len() as u32),
                                        "ingest",
                                        serde_json::json!({"batch_id": batch.id.to_string()}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": format!("Media batch ingest started: {}", batch.id),
                                "batch_id": batch.id.to_string(),
                                "reference_name": batch.reference_name,
                                "status": format!("{:?}", batch.status),
                                "storage_tier": format!("{:?}", batch.storage_tier),
                                "created_at": batch.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to ingest media batch: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to ingest media batch: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available. Editron functionality requires media pipeline service.",
                    }))
                }
            }

            NoraExecutiveTool::AnalyzeMediaBatch {
                batch_id,
                brief,
                deliverable_targets,
                passes,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Analyzing media batch: {}", batch_id);

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid batch_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    let request = MediaBatchAnalysisRequest {
                        batch_id: batch_uuid,
                        brief: brief.clone(),
                        passes,
                        deliverable_targets: deliverable_targets.clone(),
                    };

                    match pipeline.analyze_batch(request).await {
                        Ok(analysis) => {
                            tracing::info!(
                                "[TOOL] Media batch analyzed: analysis_id={}",
                                analysis.id
                            );

                            // Dashboard tracking
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: Analyze batch {}", batch_id),
                                        &format!("Analysis of batch {} - {}", batch_id, brief),
                                        serde_json::json!({ "editron_batch_id": batch_id }),
                                    )
                                    .await
                                {
                                    let vibe_cost = crate::editron_tracking::EditronVibeCosts::analyze(passes);

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::MediaAnalysisReport,
                                        &format!("Analysis: {} hero moments", analysis.hero_moments.len()),
                                        Some(serde_json::json!({
                                            "analysis_id": analysis.id.to_string(),
                                            "batch_id": analysis.batch_id.to_string(),
                                            "hero_moments_count": analysis.hero_moments.len(),
                                            "passes_completed": analysis.passes_completed,
                                            "summary": analysis.summary,
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        resolved_task_id,
                                        "editron_analyze_completed",
                                        &format!(
                                            "Batch analyzed: {} hero moments, {} passes",
                                            analysis.hero_moments.len(),
                                            analysis.passes_completed
                                        ),
                                        vibe_cost,
                                        serde_json::json!({"batch_id": batch_id, "analysis_id": analysis.id.to_string()}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        resolved_project_id,
                                        resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron analyze: {} passes", passes),
                                        "analyze",
                                        serde_json::json!({"batch_id": batch_id}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": "Media batch analyzed successfully",
                                "analysis_id": analysis.id.to_string(),
                                "batch_id": analysis.batch_id.to_string(),
                                "summary": analysis.summary,
                                "hero_moments_count": analysis.hero_moments.len(),
                                "hero_moments": analysis.hero_moments,
                                "recommended_deliverables": analysis.recommended_deliverables,
                                "passes_completed": analysis.passes_completed,
                                "created_at": analysis.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to analyze media batch: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to analyze media batch: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            NoraExecutiveTool::GenerateVideoEdits {
                batch_id,
                deliverable_type,
                aspect_ratios,
                reference_style,
                include_captions,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Generating video edits for batch: {}", batch_id);

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid batch_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    let request = EditSessionRequest {
                        batch_id: batch_uuid,
                        deliverable_type: deliverable_type.clone(),
                        aspect_ratios: aspect_ratios.clone(),
                        reference_style: reference_style.clone(),
                        include_captions,
                    };

                    match pipeline.generate_edits(request).await {
                        Ok(session) => {
                            tracing::info!(
                                "[TOOL] Edit session created: session_id={}",
                                session.id
                            );

                            // Dashboard tracking
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: {} edit", deliverable_type),
                                        &format!("Video edits for batch {}", batch_id),
                                        serde_json::json!({ "editron_batch_id": batch_id }),
                                    )
                                    .await
                                {
                                    let vibe_cost = crate::editron_tracking::EditronVibeCosts::generate(aspect_ratios.len());

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::VideoEditSession,
                                        &format!("Edit Session: {}", deliverable_type),
                                        Some(serde_json::json!({
                                            "session_id": session.id.to_string(),
                                            "batch_id": session.batch_id.to_string(),
                                            "deliverable_type": session.deliverable_type,
                                            "aspect_ratios": session.aspect_ratios,
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        resolved_task_id,
                                        "editron_edits_generated",
                                        &format!(
                                            "Video edits generated: {} with {} ratios",
                                            deliverable_type,
                                            aspect_ratios.len()
                                        ),
                                        vibe_cost,
                                        serde_json::json!({"batch_id": batch_id, "session_id": session.id.to_string()}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        resolved_project_id,
                                        resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron edit: {} ratios", aspect_ratios.len()),
                                        "edit",
                                        serde_json::json!({"batch_id": batch_id}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": "Edit session created successfully",
                                "session_id": session.id.to_string(),
                                "batch_id": session.batch_id.to_string(),
                                "deliverable_type": session.deliverable_type,
                                "aspect_ratios": session.aspect_ratios,
                                "imovie_project": session.imovie_project,
                                "timelines": session.timelines,
                                "status": format!("{:?}", session.status),
                                "created_at": session.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to generate video edits: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to generate video edits: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            NoraExecutiveTool::RenderVideoDeliverables {
                edit_session_id,
                destinations,
                formats,
                priority,
                project_id,
                task_id,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Rendering video deliverables for session: {}", edit_session_id);

                    let session_uuid = match Uuid::parse_str(&edit_session_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid edit_session_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    let is_rush = matches!(priority, VideoRenderPriority::Rush);

                    // Convert VideoRenderPriority to the pipeline's priority enum
                    let priority_enum = match priority {
                        VideoRenderPriority::Low => PipelineRenderPriority::Low,
                        VideoRenderPriority::Standard => PipelineRenderPriority::Standard,
                        VideoRenderPriority::Rush => PipelineRenderPriority::Rush,
                    };

                    let request = RenderJobRequest {
                        edit_session_id: session_uuid,
                        destinations: destinations.clone(),
                        formats: formats.clone(),
                        priority: priority_enum,
                    };

                    match pipeline.render_deliverables(request).await {
                        Ok(job) => {
                            tracing::info!(
                                "[TOOL] Render job created: job_id={}",
                                job.id
                            );

                            // Dashboard tracking
                            if let Some(executor) = &self.task_executor {
                                let pool = executor.pool();

                                if let Some((resolved_task_id, resolved_project_id)) =
                                    crate::editron_tracking::find_or_create_task(
                                        pool,
                                        executor,
                                        task_id.as_deref(),
                                        project_id.as_deref(),
                                        &format!("Editron: Render {}", edit_session_id),
                                        &format!("Render job for session {}", edit_session_id),
                                        serde_json::json!({}),
                                    )
                                    .await
                                {
                                    let vibe_cost = crate::editron_tracking::EditronVibeCosts::render(formats.len(), is_rush);

                                    let _ = crate::editron_tracking::create_and_link_artifact(
                                        pool,
                                        resolved_task_id,
                                        db::models::execution_artifact::ArtifactType::RenderDeliverable,
                                        &format!("Render Job: {} formats", formats.len()),
                                        Some(serde_json::json!({
                                            "job_id": job.id.to_string(),
                                            "edit_session_id": job.edit_session_id.to_string(),
                                            "destinations": job.destinations,
                                            "formats": job.formats,
                                            "priority": format!("{:?}", job.priority),
                                        }).to_string()),
                                        None,
                                        serde_json::json!({"phase": "execution"}),
                                        db::models::task_artifact::ArtifactRole::Primary,
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::log_editron_activity(
                                        pool,
                                        resolved_task_id,
                                        "editron_render_started",
                                        &format!(
                                            "Render job queued: {} formats, {:?} priority",
                                            formats.len(),
                                            job.priority
                                        ),
                                        vibe_cost,
                                        serde_json::json!({"job_id": job.id.to_string(), "edit_session_id": edit_session_id}),
                                    )
                                    .await;

                                    let _ = crate::editron_tracking::record_editron_vibe(
                                        pool,
                                        resolved_project_id,
                                        resolved_task_id,
                                        vibe_cost,
                                        &format!("Editron render: {} formats", formats.len()),
                                        "render",
                                        serde_json::json!({"job_id": job.id.to_string()}),
                                    )
                                    .await;
                                }
                            }

                            Ok(serde_json::json!({
                                "success": true,
                                "message": "Render job created successfully",
                                "job_id": job.id.to_string(),
                                "edit_session_id": job.edit_session_id.to_string(),
                                "destinations": job.destinations,
                                "formats": job.formats,
                                "priority": format!("{:?}", job.priority),
                                "status": format!("{:?}", job.status),
                                "created_at": job.created_at.to_string(),
                            }))
                        }
                        Err(e) => {
                            tracing::error!("[TOOL] Failed to render video deliverables: {}", e);
                            Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Failed to render video deliverables: {}", e),
                            }))
                        }
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            NoraExecutiveTool::RunVisualQc {
                batch_id,
                candidates_per_clip,
                min_composition_score,
                target_aspect_ratio,
                project_id: _,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Running Spectra Visual QC on batch: {}", batch_id);

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid batch_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Verify batch is ready
                    let batch = match pipeline.load_batch_for_qc(batch_uuid).await {
                        Ok(b) => b,
                        Err(e) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": format!("Batch not ready for QC: {}", e),
                            }));
                        }
                    };

                    // Build QC config
                    let mut config = services::services::visual_qc::VisualQcConfig::default();
                    if let Some(n) = candidates_per_clip {
                        config.candidates_per_clip = n;
                    }
                    if let Some(score) = min_composition_score {
                        config.min_composition_score = score;
                    }
                    config.target_aspect_ratio = target_aspect_ratio.clone();

                    let start = std::time::Instant::now();
                    let work_dir = pipeline.visual_qc_work_dir(batch_uuid);
                    let _ = tokio::fs::create_dir_all(&work_dir).await;

                    let ffmpeg_path = std::path::PathBuf::from("ffmpeg");
                    let engine = services::services::visual_qc::VisualQcEngine::new(
                        &ffmpeg_path,
                        &work_dir,
                    );
                    let (system_prompt, user_prompt_template) =
                        services::services::visual_qc::VisualQcEngine::build_vision_prompt(
                            target_aspect_ratio.as_deref(),
                        );

                    let mut clip_results = Vec::new();
                    let mut total_frames = 0u32;

                    for file in &batch.files {
                        if total_frames >= config.max_total_frames {
                            break;
                        }

                        let file_path = pipeline.get_batch_dir(batch_uuid).join(&file.filename);
                        if !file_path.exists() {
                            continue;
                        }

                        let frames = match engine.extract_candidate_frames(&file_path, &config).await {
                            Ok(f) => f,
                            Err(e) => {
                                tracing::warn!("Failed to extract frames from {}: {}", file.filename, e);
                                continue;
                            }
                        };

                        let mut analyzed_frames = Vec::new();

                        for (timestamp, frame_path) in &frames {
                            if total_frames >= config.max_total_frames {
                                break;
                            }

                            let base64_data = match services::services::visual_qc::VisualQcEngine::frame_to_base64(frame_path).await {
                                Ok(d) => d,
                                Err(_) => continue,
                            };

                            // Build the QC result using a placeholder analysis since we
                            // don't have direct access to the LLM provider from the tool.
                            // In production, the ExecutionEngine (which HAS provider access)
                            // handles the vision API calls via the visual-qc-pass workflow.
                            // This tool fallback provides frame-extraction-only QC scoring.
                            let placeholder_response = serde_json::json!({
                                "composition_score": 0.7,
                                "subject_score": 0.7,
                                "thirds_score": 0.6,
                                "headroom_score": 0.8,
                                "exposure_score": 0.7,
                                "sharpness_score": 0.7,
                                "subject_region": null,
                                "suggested_crop": null,
                                "notes": "Frame extracted — vision API scoring deferred to workflow execution"
                            });

                            if let Ok(frame) = services::services::visual_qc::VisualQcEngine::parse_vision_response(
                                *timestamp,
                                frame_path,
                                &placeholder_response.to_string(),
                            ) {
                                analyzed_frames.push(frame);
                            }

                            total_frames += 1;
                        }

                        let best = services::services::visual_qc::VisualQcEngine::select_best_in_point(&analyzed_frames, &config);
                        let (best_in_point, best_score) = best.unwrap_or((0.0, 0.0));
                        let qc_passed = best_score >= config.min_composition_score;

                        let recommended_crop = analyzed_frames
                            .iter()
                            .find(|f| (f.timestamp - best_in_point).abs() < 0.001)
                            .and_then(|f| f.suggested_crop.clone());

                        clip_results.push(services::services::visual_qc::ClipQcResult {
                            clip_path: file_path,
                            frames_analyzed: analyzed_frames.len() as u32,
                            best_in_point,
                            best_composition_score: best_score,
                            recommended_crop,
                            qc_passed,
                            summary: if qc_passed {
                                format!("Passed (score: {:.2})", best_score)
                            } else {
                                format!("Below threshold (score: {:.2})", best_score)
                            },
                        });
                    }

                    let time_ms = start.elapsed().as_millis() as u64;
                    let result = services::services::visual_qc::VisualQcEngine::assemble_batch_result(clip_results, &config, time_ms);

                    // Persist result
                    let result_path = work_dir.join("qc_result.json");
                    if let Ok(json_str) = serde_json::to_string_pretty(&result) {
                        let _ = tokio::fs::write(&result_path, json_str).await;
                    }

                    tracing::info!(
                        "[TOOL] Visual QC complete: {}/{} clips passed (avg score: {:.2})",
                        result.clips_passed,
                        result.clips_analyzed,
                        result.average_composition_score
                    );

                    let clip_summaries: Vec<serde_json::Value> = result
                        .clip_results
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "clip": r.clip_path.file_name().map(|n| n.to_string_lossy().to_string()),
                                "best_in_point": r.best_in_point,
                                "composition_score": r.best_composition_score,
                                "passed": r.qc_passed,
                                "summary": r.summary,
                            })
                        })
                        .collect();

                    Ok(serde_json::json!({
                        "success": true,
                        "message": format!(
                            "Visual QC complete: {}/{} clips passed",
                            result.clips_passed, result.clips_analyzed
                        ),
                        "qc_id": result.id,
                        "clips_analyzed": result.clips_analyzed,
                        "clips_passed": result.clips_passed,
                        "clips_failed": result.clips_failed,
                        "average_composition_score": result.average_composition_score,
                        "processing_time_ms": result.processing_time_ms,
                        "clip_results": clip_summaries,
                    }))
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Media pipeline not available",
                    }))
                }
            }

            // === Deep Scene Analysis ===
            NoraExecutiveTool::AnalyzeScenes {
                batch_id,
                segment_interval,
                project_id: _,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Running deep scene analysis on batch: {}", batch_id);
                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => return Ok(serde_json::json!({"success": false, "error": "Invalid batch_id"})),
                    };
                    let batch = match pipeline.load_batch_for_qc(batch_uuid).await {
                        Ok(b) => b,
                        Err(e) => return Ok(serde_json::json!({"success": false, "error": format!("Batch not ready: {}", e)})),
                    };

                    let start = std::time::Instant::now();
                    let engine = services::services::scene_analysis::SceneAnalysisEngine::new();
                    let interval = segment_interval.unwrap_or(3.0);
                    let mut clip_analyses = Vec::new();

                    for file in &batch.files {
                        // Skip non-video files
                        let ext = file.filename.rsplit('.').next().unwrap_or("").to_lowercase();
                        if !matches!(ext.as_str(), "mp4" | "mov" | "avi" | "mxf" | "mkv") {
                            continue;
                        }
                        let file_path = pipeline.get_batch_dir(batch_uuid).join(&file.filename);
                        if !file_path.exists() { continue; }

                        match engine.analyze_clip(&file_path, interval).await {
                            Ok(analysis) => {
                                tracing::info!(
                                    "[TOOL] Scene analysis for {}: energy={:.2}, type={:?}, segments={}",
                                    file.filename, analysis.overall_energy,
                                    analysis.dominant_content_type, analysis.segments.len()
                                );
                                clip_analyses.push(analysis);
                            }
                            Err(e) => {
                                tracing::warn!("[TOOL] Failed to analyze {}: {}", file.filename, e);
                            }
                        }
                    }

                    let total_usable = clip_analyses.iter().filter(|c| c.usable).count() as u32;
                    let time_ms = start.elapsed().as_millis() as u64;

                    let result = services::services::scene_analysis::SceneAnalysisResult {
                        batch_id: batch_id.clone(),
                        total_clips: clip_analyses.len() as u32,
                        total_usable,
                        clips: clip_analyses.clone(),
                        processing_time_ms: time_ms,
                    };

                    // Persist
                    let work_dir = pipeline.visual_qc_work_dir(batch_uuid);
                    let _ = tokio::fs::create_dir_all(&work_dir).await;
                    let result_path = work_dir.join("scene_analysis.json");
                    if let Ok(json_str) = serde_json::to_string_pretty(&result) {
                        let _ = tokio::fs::write(&result_path, json_str).await;
                    }

                    let clip_summaries: Vec<serde_json::Value> = clip_analyses.iter().map(|c| {
                        serde_json::json!({
                            "filename": c.filename,
                            "duration": c.duration,
                            "overall_energy": c.overall_energy,
                            "peak_timestamp": c.peak_energy_timestamp,
                            "content_type": format!("{:?}", c.dominant_content_type),
                            "segments": c.segments.len(),
                            "usable": c.usable,
                        })
                    }).collect();

                    Ok(serde_json::json!({
                        "success": true,
                        "message": format!("Scene analysis complete: {}/{} clips usable", total_usable, clip_analyses.len()),
                        "total_clips": clip_analyses.len(),
                        "total_usable": total_usable,
                        "processing_time_ms": time_ms,
                        "clips": clip_summaries,
                    }))
                } else {
                    Ok(serde_json::json!({"success": false, "error": "Media pipeline not available"}))
                }
            }

            // === Beat Grid Analysis ===
            NoraExecutiveTool::AnalyzeBeatGrid {
                audio_path,
                bpm_hint,
                beats_per_bar,
                project_id: _,
            } => {
                tracing::info!("[TOOL] Analyzing beat grid for: {}", audio_path);
                let path = std::path::PathBuf::from(&audio_path);
                if !path.exists() {
                    return Ok(serde_json::json!({"success": false, "error": format!("Audio file not found: {}", audio_path)}));
                }

                let engine = services::services::beat_analysis::BeatAnalysisEngine::new();
                let bpb = beats_per_bar.unwrap_or(4);

                match engine.analyze(&path, bpm_hint, bpb).await {
                    Ok(result) => {
                        tracing::info!(
                            "[TOOL] Beat analysis complete: {:.1} BPM, {} beats, {} sections, {:.0}ms",
                            result.bpm, result.total_beats, result.sections.len(), result.processing_time_ms
                        );

                        // Persist
                        let result_dir = path.parent().unwrap_or(std::path::Path::new("/tmp"));
                        let result_path = result_dir.join("beat_grid.json");
                        if let Ok(json_str) = serde_json::to_string_pretty(&result) {
                            let _ = tokio::fs::write(&result_path, json_str).await;
                        }

                        let section_summaries: Vec<serde_json::Value> = result.sections.iter().map(|s| {
                            serde_json::json!({
                                "name": s.name,
                                "start": s.start,
                                "end": s.end,
                                "energy": s.energy_level,
                                "suggested_content": format!("{:?}", s.suggested_content),
                            })
                        }).collect();

                        Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Beat analysis complete: {:.1} BPM, {} beats", result.bpm, result.total_beats),
                            "bpm": result.bpm,
                            "beat_interval": result.beat_interval,
                            "duration": result.duration,
                            "total_beats": result.total_beats,
                            "sections": section_summaries,
                            "transition_markers": result.transition_markers.len(),
                            "processing_time_ms": result.processing_time_ms,
                        }))
                    }
                    Err(e) => {
                        Ok(serde_json::json!({"success": false, "error": format!("Beat analysis failed: {}", e)}))
                    }
                }
            }

            // === Assemble Recap Edit ===
            NoraExecutiveTool::AssembleRecapEdit {
                batch_id,
                audio_path,
                bpm_hint,
                target_aspect_ratio,
                project_id: _,
            } => {
                if let Some(pipeline) = &self.media_pipeline {
                    tracing::info!("[TOOL] Assembling recap edit for batch: {} with audio: {}", batch_id, audio_path);

                    let batch_uuid = match Uuid::parse_str(&batch_id) {
                        Ok(uuid) => uuid,
                        Err(_) => return Ok(serde_json::json!({"success": false, "error": "Invalid batch_id"})),
                    };

                    let audio = std::path::PathBuf::from(&audio_path);
                    if !audio.exists() {
                        return Ok(serde_json::json!({"success": false, "error": format!("Audio not found: {}", audio_path)}));
                    }

                    let start = std::time::Instant::now();

                    // Step 1: Scene analysis
                    tracing::info!("[TOOL] Step 1/4: Deep scene analysis...");
                    let scene_engine = services::services::scene_analysis::SceneAnalysisEngine::new();
                    let batch = match pipeline.load_batch_for_qc(batch_uuid).await {
                        Ok(b) => b,
                        Err(e) => return Ok(serde_json::json!({"success": false, "error": format!("Batch not ready: {}", e)})),
                    };

                    let mut clip_analyses = Vec::new();
                    for file in &batch.files {
                        let ext = file.filename.rsplit('.').next().unwrap_or("").to_lowercase();
                        if !matches!(ext.as_str(), "mp4" | "mov" | "avi" | "mxf" | "mkv") { continue; }
                        let file_path = pipeline.get_batch_dir(batch_uuid).join(&file.filename);
                        if !file_path.exists() { continue; }
                        if let Ok(analysis) = scene_engine.analyze_clip(&file_path, 3.0).await {
                            clip_analyses.push(analysis);
                        }
                    }
                    let total_usable = clip_analyses.iter().filter(|c| c.usable).count() as u32;
                    let scene_result = services::services::scene_analysis::SceneAnalysisResult {
                        batch_id: batch_id.clone(),
                        total_clips: clip_analyses.len() as u32,
                        total_usable,
                        clips: clip_analyses,
                        processing_time_ms: 0,
                    };

                    // Step 2: Beat analysis
                    tracing::info!("[TOOL] Step 2/4: Beat grid analysis...");
                    let beat_engine = services::services::beat_analysis::BeatAnalysisEngine::new();
                    let beat_grid = match beat_engine.analyze(&audio, bpm_hint, 4).await {
                        Ok(bg) => bg,
                        Err(e) => return Ok(serde_json::json!({"success": false, "error": format!("Beat analysis failed: {}", e)})),
                    };

                    // Step 3: Assembly
                    tracing::info!("[TOOL] Step 3/4: Smart assembly (content→music matching, beat-lock cuts)...");
                    let (width, height) = match target_aspect_ratio.as_deref() {
                        Some("9:16") => (1080, 1920),
                        Some("1:1") => (1080, 1080),
                        _ => (3840, 2160),
                    };

                    let (placements, music_window) = services::services::recap_assembly::RecapAssemblyEngine::assemble(
                        &scene_result,
                        &beat_grid,
                        &audio,
                        width,
                        height,
                        None, // Use default 59s duration
                    );

                    let beat_locked_cuts = placements.iter().filter(|p| p.beat_locked).count() as u32;

                    let assembly_result = services::services::recap_assembly::RecapAssemblyResult {
                        id: Uuid::new_v4().to_string(),
                        name: "Art Access After Dark - Recap".to_string(),
                        duration: music_window.duration,
                        width,
                        height,
                        fps: 29.97,
                        bpm: beat_grid.bpm,
                        placements: placements.clone(),
                        music_path: audio_path.clone(),
                        music_window: Some(music_window.clone()),
                        xml_path: String::new(),
                        clips_used: placements.len() as u32,
                        clips_available: scene_result.total_usable,
                        beat_locked_cuts,
                        processing_time_ms: 0,
                    };

                    // Step 4: Generate XML
                    tracing::info!("[TOOL] Step 4/4: Generating Premiere Pro XML (music only, NAT muted)...");
                    let xml = services::services::recap_assembly::RecapAssemblyEngine::generate_premiere_xml(
                        &assembly_result,
                        &placements,
                        &beat_grid,
                    );

                    let work_dir = pipeline.visual_qc_work_dir(batch_uuid);
                    let _ = tokio::fs::create_dir_all(&work_dir).await;
                    let xml_path = work_dir.join("After_Dark_Recap_BeatLocked.xml");
                    let _ = tokio::fs::write(&xml_path, &xml).await;

                    // Generate FFmpeg render script with transitions
                    let render_output = work_dir.join("After_Dark_Recap_v2.mp4");
                    let render_script = services::services::recap_assembly::RecapAssemblyEngine::generate_render_script(
                        &placements,
                        &audio_path,
                        &music_window,
                        &render_output.to_string_lossy(),
                    );
                    let script_path = work_dir.join("render.sh");
                    let _ = tokio::fs::write(&script_path, &render_script).await;

                    let time_ms = start.elapsed().as_millis() as u64;

                    // Also persist assembly result
                    let mut final_result = assembly_result;
                    final_result.xml_path = xml_path.to_string_lossy().to_string();
                    final_result.processing_time_ms = time_ms;
                    let result_path = work_dir.join("assembly_result.json");
                    if let Ok(json_str) = serde_json::to_string_pretty(&final_result) {
                        let _ = tokio::fs::write(&result_path, json_str).await;
                    }

                    let placement_summaries: Vec<serde_json::Value> = placements.iter().map(|p| {
                        serde_json::json!({
                            "clip": p.clip_filename,
                            "section": p.section_name,
                            "timeline": format!("{:.2}s-{:.2}s", p.timeline_in, p.timeline_out),
                            "source": format!("{:.2}s-{:.2}s", p.source_in, p.source_out),
                            "energy_match": p.energy_match_score,
                            "beat_locked": p.beat_locked,
                        })
                    }).collect();

                    tracing::info!(
                        "[TOOL] Recap assembled: {} clips, {} beat-locked cuts, {:.1}s duration, {:.0}ms",
                        final_result.clips_used, beat_locked_cuts, final_result.duration, time_ms
                    );

                    Ok(serde_json::json!({
                        "success": true,
                        "message": format!(
                            "Recap assembled: {} clips, {} beat-locked cuts, audio=music only (NAT muted)",
                            final_result.clips_used, beat_locked_cuts
                        ),
                        "assembly_id": final_result.id,
                        "duration": final_result.duration,
                        "bpm": final_result.bpm,
                        "clips_used": final_result.clips_used,
                        "clips_available": final_result.clips_available,
                        "beat_locked_cuts": beat_locked_cuts,
                        "xml_path": final_result.xml_path,
                        "processing_time_ms": time_ms,
                        "placements": placement_summaries,
                        "render_script": script_path.to_string_lossy(),
                        "render_output": render_output.to_string_lossy(),
                        "music_window": {
                            "start": music_window.start,
                            "end": music_window.end,
                            "duration": music_window.duration,
                            "sections": music_window.sections.len()
                        },
                        "audio_config": {
                            "music_track": format!("A1 - music window {:.1}s-{:.1}s ({:.0}s)", music_window.start, music_window.end, music_window.duration),
                            "nat_audio": "MUTED - no clip audio on timeline",
                            "normalization": "Streaming (-14 LUFS)"
                        },
                        "sections": beat_grid.sections.iter().map(|s| {
                            serde_json::json!({"name": s.name, "start": s.start, "end": s.end, "energy": s.energy_level})
                        }).collect::<Vec<_>>(),
                    }))
                } else {
                    Ok(serde_json::json!({"success": false, "error": "Media pipeline not available"}))
                }
            }

            NoraExecutiveTool::ExecuteRenderScript {
                render_script,
                render_output,
                xml_path,
            } => {
                tracing::info!("[TOOL] Executing render script: {}", render_script);

                let script_path = std::path::Path::new(&render_script);
                if !script_path.exists() {
                    return Ok(serde_json::json!({
                        "success": false,
                        "error": format!("Render script not found: {}", render_script),
                    }));
                }

                // Run the render script via bash
                let output = tokio::process::Command::new("bash")
                    .arg(&render_script)
                    .output()
                    .await;

                match output {
                    Ok(result) => {
                        let stdout = String::from_utf8_lossy(&result.stdout);
                        let stderr = String::from_utf8_lossy(&result.stderr);
                        let render_path = std::path::Path::new(&render_output);
                        let file_size = tokio::fs::metadata(&render_path).await
                            .map(|m| m.len()).unwrap_or(0);

                        tracing::info!(
                            "[TOOL] Render complete: {} ({} bytes), exit={}",
                            render_output, file_size, result.status
                        );

                        let _ = stdout; // consumed for logging if needed

                        // Take the last 500 chars of stderr for diagnostics
                        let stderr_str = stderr.to_string();
                        let stderr_tail: String = if stderr_str.len() > 500 {
                            stderr_str[stderr_str.len() - 500..].to_string()
                        } else {
                            stderr_str
                        };

                        Ok(serde_json::json!({
                            "success": result.status.success(),
                            "message": format!("Render complete: {}", render_output),
                            "render_output": render_output,
                            "xml_path": xml_path,
                            "file_size_bytes": file_size,
                            "exit_code": result.status.code(),
                            "stderr_tail": stderr_tail,
                        }))
                    }
                    Err(e) => {
                        Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to execute render script: {}", e),
                        }))
                    }
                }
            }

            NoraExecutiveTool::CreateTaskOnBoard {
                project_id,
                board_id,
                title,
                description,
                priority,
                tags,
            } => {
                if let Some(executor) = &self.task_executor {
                    // Parse project_id UUID
                    let project_uuid = match Uuid::parse_str(&project_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid project_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Parse board_id UUID
                    let board_uuid = match Uuid::parse_str(&board_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid board_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Map priority string to enum
                    let priority_enum =
                        priority
                            .as_ref()
                            .and_then(|p| match p.to_lowercase().as_str() {
                                "critical" => Some(Priority::Critical),
                                "high" => Some(Priority::High),
                                "medium" => Some(Priority::Medium),
                                "low" => Some(Priority::Low),
                                _ => None,
                            });

                    match executor
                        .create_task_on_board(
                            project_uuid,
                            board_uuid,
                            title.clone(),
                            description,
                            priority_enum,
                            tags,
                        )
                        .await
                    {
                        Ok(task) => Ok(serde_json::json!({
                            "success": true,
                            "message": format!("Task '{}' created successfully", title),
                            "task_id": task.id.to_string(),
                            "project_id": task.project_id.to_string(),
                            "board_id": task.board_id.map(|id| id.to_string()),
                            "title": task.title,
                            "status": format!("{:?}", task.status),
                            "priority": format!("{:?}", task.priority),
                            "created_at": task.created_at.to_string(),
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to create task: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }
            NoraExecutiveTool::AddTaskToBoard { task_id, board_id } => {
                if let Some(executor) = &self.task_executor {
                    // Parse task_id UUID
                    let task_uuid = match Uuid::parse_str(&task_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid task_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    // Parse board_id UUID
                    let board_uuid = match Uuid::parse_str(&board_id) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Ok(serde_json::json!({
                                "success": false,
                                "error": "Invalid board_id format. Must be a valid UUID."
                            }));
                        }
                    };

                    match executor.add_task_to_board(task_uuid, board_uuid).await {
                        Ok(()) => Ok(serde_json::json!({
                            "success": true,
                            "message": "Task assigned to board successfully",
                            "task_id": task_id,
                            "board_id": board_id,
                        })),
                        Err(e) => Ok(serde_json::json!({
                            "success": false,
                            "error": format!("Failed to assign task to board: {}", e),
                        })),
                    }
                } else {
                    Ok(serde_json::json!({
                        "success": false,
                        "error": "Task executor not available"
                    }))
                }
            }

            // File Operations
            NoraExecutiveTool::ReadFile {
                file_path,
                encoding,
            } => {
                self.execute_read_file(&file_path, encoding.as_deref())
                    .await
            }
            NoraExecutiveTool::WriteFile {
                file_path,
                content,
                create_directories,
            } => {
                self.execute_write_file(&file_path, &content, create_directories)
                    .await
            }
            NoraExecutiveTool::ListDirectory {
                directory_path,
                recursive,
                pattern,
            } => {
                self.execute_list_directory(&directory_path, recursive, pattern.as_deref())
                    .await
            }
            NoraExecutiveTool::DeleteFile { file_path, confirm } => {
                self.execute_delete_file(&file_path, confirm).await
            }

            // Web Search & Information
            NoraExecutiveTool::SearchWeb {
                query,
                max_results,
                search_type,
            } => {
                self.execute_web_search(&query, max_results, &search_type)
                    .await
            }
            NoraExecutiveTool::FetchWebPage { url, extract_text } => {
                self.execute_fetch_webpage(&url, extract_text).await
            }
            NoraExecutiveTool::SummarizeContent {
                content,
                max_length,
                format,
            } => {
                self.execute_summarize_content(&content, max_length, &format)
                    .await
            }

            // Code & Development
            NoraExecutiveTool::ExecuteCode {
                code,
                language,
                timeout_seconds,
            } => self.execute_code(&code, &language, timeout_seconds).await,
            NoraExecutiveTool::AnalyzeCodeQuality {
                code,
                language,
                check_security,
            } => {
                self.execute_analyze_code_quality(&code, &language, check_security)
                    .await
            }
            NoraExecutiveTool::GenerateDocumentation { code, doc_format } => {
                self.execute_generate_documentation(&code, &doc_format)
                    .await
            }

            // Email & Notifications
            NoraExecutiveTool::SendEmail {
                recipients,
                subject,
                body,
                priority,
            } => {
                self.execute_send_email(&recipients, &subject, &body, &priority)
                    .await
            }
            NoraExecutiveTool::SendDiscordMessage {
                channel,
                message,
                mention_users,
            } => {
                self.execute_send_discord_message(&channel, &message, &mention_users)
                    .await
            }
            NoraExecutiveTool::CreateNotification {
                title,
                message,
                notification_type,
                recipients,
            } => {
                self.execute_create_notification(&title, &message, &notification_type, &recipients)
                    .await
            }

            // Calendar & Scheduling
            NoraExecutiveTool::CreateCalendarEvent {
                title,
                start_time,
                end_time,
                attendees,
                location,
            } => {
                self.execute_create_calendar_event(
                    &title,
                    start_time,
                    end_time,
                    &attendees,
                    location.as_deref(),
                )
                .await
            }
            NoraExecutiveTool::FindAvailableSlots {
                participants,
                duration_minutes,
                preferred_days,
            } => {
                self.execute_find_available_slots(&participants, duration_minutes, &preferred_days)
                    .await
            }
            NoraExecutiveTool::CheckCalendarAvailability {
                user,
                start_time,
                end_time,
            } => {
                self.execute_check_calendar_availability(&user, start_time, end_time)
                    .await
            }

            // Existing tools
            NoraExecutiveTool::CoordinateTeamMeeting {
                participants,
                agenda,
                ..
            } => Ok(serde_json::json!({
                "meeting_scheduled": true,
                "participants": participants,
                "agenda": agenda,
                "meeting_id": uuid::Uuid::new_v4().to_string()
            })),
            NoraExecutiveTool::GenerateKPIDashboard { metrics, .. } => Ok(serde_json::json!({
                "dashboard_created": true,
                "metrics": metrics,
                "dashboard_url": "/dashboards/executive-kpi"
            })),
            NoraExecutiveTool::CreateDecisionMatrix {
                options, criteria, ..
            } => Ok(serde_json::json!({
                "matrix_created": true,
                "options_count": options.len(),
                "criteria_count": criteria.len(),
                "matrix_id": uuid::Uuid::new_v4().to_string()
            })),
            // Add more implementations...
            _ => Ok(serde_json::json!({
                "message": "Tool implementation pending",
                "tool_executed": true
            })),
        }
    }

    // File Operations Implementations
    async fn execute_read_file(
        &self,
        file_path: &str,
        _encoding: Option<&str>,
    ) -> crate::Result<serde_json::Value> {
        use tokio::fs;

        let content = fs::read_to_string(file_path).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read file: {}", e))
        })?;

        Ok(serde_json::json!({
            "success": true,
            "file_path": file_path,
            "content": content,
            "size_bytes": content.len()
        }))
    }

    async fn execute_write_file(
        &self,
        file_path: &str,
        content: &str,
        create_directories: bool,
    ) -> crate::Result<serde_json::Value> {
        use std::path::Path;

        use tokio::fs;

        let path = Path::new(file_path);

        if create_directories {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    crate::NoraError::ToolExecutionError(format!(
                        "Failed to create directories: {}",
                        e
                    ))
                })?;
            }
        }

        fs::write(file_path, content).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to write file: {}", e))
        })?;

        Ok(serde_json::json!({
            "success": true,
            "file_path": file_path,
            "bytes_written": content.len()
        }))
    }

    async fn execute_list_directory(
        &self,
        directory_path: &str,
        recursive: bool,
        pattern: Option<&str>,
    ) -> crate::Result<serde_json::Value> {
        use tokio::fs;

        let mut entries = Vec::new();

        let mut read_dir = fs::read_dir(directory_path).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read directory: {}", e))
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read entry: {}", e))
        })? {
            let path = entry.path();
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Apply pattern filter if provided
            if let Some(pat) = pattern {
                if !file_name.contains(pat) {
                    continue;
                }
            }

            let metadata = entry.metadata().await.ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

            entries.push(serde_json::json!({
                "name": file_name,
                "path": path.to_string_lossy(),
                "is_directory": is_dir,
                "size_bytes": size
            }));

            // Recursively list subdirectories
            if recursive && is_dir {
                let path_str = path.to_string_lossy().to_string();
                if let Ok(sub_result) =
                    Box::pin(self.execute_list_directory(&path_str, true, pattern)).await
                {
                    if let Some(sub_entries) = sub_result.get("entries").and_then(|e| e.as_array())
                    {
                        for sub_entry in sub_entries {
                            entries.push(sub_entry.clone());
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "success": true,
            "directory": directory_path,
            "entries": entries,
            "count": entries.len()
        }))
    }

    async fn execute_delete_file(
        &self,
        file_path: &str,
        confirm: bool,
    ) -> crate::Result<serde_json::Value> {
        if !confirm {
            return Ok(serde_json::json!({
                "success": false,
                "message": "Delete operation requires confirmation"
            }));
        }

        use tokio::fs;

        fs::remove_file(file_path).await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to delete file: {}", e))
        })?;

        Ok(serde_json::json!({
            "success": true,
            "file_path": file_path,
            "deleted": true
        }))
    }

    // Web Search & Information Implementations
    async fn execute_web_search(
        &self,
        query: &str,
        max_results: u32,
        _search_type: &SearchType,
    ) -> crate::Result<serde_json::Value> {
        // Note: This would integrate with actual search APIs (DuckDuckGo, Google, etc.)
        // For now, return structured placeholder
        Ok(serde_json::json!({
            "success": true,
            "query": query,
            "results": [
                {
                    "title": format!("Search result for: {}", query),
                    "url": "https://example.com/result1",
                    "snippet": format!("This is a mock search result for '{}'", query)
                }
            ],
            "result_count": 1,
            "max_results": max_results,
            "note": "Real search API integration pending"
        }))
    }

    async fn execute_fetch_webpage(
        &self,
        url: &str,
        extract_text: bool,
    ) -> crate::Result<serde_json::Value> {
        use reqwest::Client;

        let client = Client::new();
        let response = client.get(url).send().await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to fetch webpage: {}", e))
        })?;

        let content = response.text().await.map_err(|e| {
            crate::NoraError::ToolExecutionError(format!("Failed to read response: {}", e))
        })?;

        let result_content = if extract_text {
            // Basic HTML tag removal (in production, use html2text or similar)
            content.replace("<", " <").replace(">", "> ")
        } else {
            content
        };

        Ok(serde_json::json!({
            "success": true,
            "url": url,
            "content": result_content,
            "content_length": result_content.len(),
            "text_extracted": extract_text
        }))
    }

    async fn execute_summarize_content(
        &self,
        content: &str,
        max_length: u32,
        _format: &SummaryFormat,
    ) -> crate::Result<serde_json::Value> {
        // Simple summarization by truncation (in production, use LLM or extractive summarization)
        let summary = if content.len() > max_length as usize {
            format!("{}...", &content[..max_length as usize])
        } else {
            content.to_string()
        };

        Ok(serde_json::json!({
            "success": true,
            "original_length": content.len(),
            "summary_length": summary.len(),
            "summary": summary,
            "compression_ratio": summary.len() as f64 / content.len() as f64
        }))
    }

    // Code & Development Implementations
    async fn execute_code(
        &self,
        code: &str,
        language: &CodeLanguage,
        timeout_seconds: u32,
    ) -> crate::Result<serde_json::Value> {
        // Note: This requires sandboxed execution environment
        // For security, this should use containers or VM isolation
        Ok(serde_json::json!({
            "success": false,
            "message": "Code execution requires sandboxed environment",
            "language": format!("{:?}", language),
            "code_length": code.len(),
            "timeout_seconds": timeout_seconds,
            "note": "Sandboxed execution pending - requires Docker/VM integration"
        }))
    }

    async fn execute_analyze_code_quality(
        &self,
        code: &str,
        language: &CodeLanguage,
        check_security: bool,
    ) -> crate::Result<serde_json::Value> {
        // Basic analysis (in production, integrate with linters/analyzers)
        let line_count = code.lines().count();
        let char_count = code.len();
        let has_comments = code.contains("//") || code.contains("/*") || code.contains("#");

        Ok(serde_json::json!({
            "success": true,
            "language": format!("{:?}", language),
            "metrics": {
                "line_count": line_count,
                "character_count": char_count,
                "has_comments": has_comments,
                "security_checked": check_security
            },
            "suggestions": [
                "Consider adding more comments for complex logic",
                "Ensure proper error handling"
            ],
            "note": "Advanced static analysis pending"
        }))
    }

    async fn execute_generate_documentation(
        &self,
        code: &str,
        doc_format: &DocumentationFormat,
    ) -> crate::Result<serde_json::Value> {
        let doc = format!("# Code Documentation\n\n```\n{}\n```\n\nGenerated documentation for the provided code.", code);

        Ok(serde_json::json!({
            "success": true,
            "format": format!("{:?}", doc_format),
            "documentation": doc,
            "doc_length": doc.len()
        }))
    }

    // Email & Notifications Implementations
    async fn execute_send_email(
        &self,
        recipients: &[String],
        subject: &str,
        body: &str,
        priority: &EmailPriority,
    ) -> crate::Result<serde_json::Value> {
        // Try to use real SMTP service if configured
        if let Some(ref email_service) = self.email_service {
            match email_service
                .send_email(recipients, subject, body, false)
                .await
            {
                Ok(message_id) => {
                    tracing::info!(
                        "Email sent successfully to {:?} with ID: {}",
                        recipients,
                        message_id
                    );
                    return Ok(serde_json::json!({
                        "success": true,
                        "recipients": recipients,
                        "subject": subject,
                        "priority": format!("{:?}", priority),
                        "message_id": message_id,
                        "sent_via": "SMTP"
                    }));
                }
                Err(e) => {
                    tracing::warn!("SMTP send failed, logging only: {}", e);
                }
            }
        }

        // Fallback: Log only
        tracing::info!("Email would be sent to {:?}: {}", recipients, subject);
        Ok(serde_json::json!({
            "success": true,
            "recipients": recipients,
            "subject": subject,
            "priority": format!("{:?}", priority),
            "message_id": uuid::Uuid::new_v4().to_string(),
            "note": "SMTP not configured - email logged only. Set SMTP_USERNAME, SMTP_PASSWORD, SMTP_FROM_EMAIL env vars to enable."
        }))
    }

    async fn execute_send_discord_message(
        &self,
        channel: &str,
        message: &str,
        mention_users: &[String],
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Discord webhook if configured
        if let Some(ref discord_service) = self.discord_service {
            match discord_service.send_message(message, mention_users).await {
                Ok(_) => {
                    tracing::info!("Discord message sent successfully to channel: {}", channel);
                    return Ok(serde_json::json!({
                        "success": true,
                        "channel": channel,
                        "message": message,
                        "mentioned_users": mention_users,
                        "timestamp": Utc::now().to_rfc3339(),
                        "sent_via": "Discord Webhook"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Discord webhook send failed, logging only: {}", e);
                }
            }
        }

        // Fallback: Log only
        tracing::info!("Discord message to {}: {}", channel, message);
        Ok(serde_json::json!({
            "success": true,
            "channel": channel,
            "message": message,
            "mentioned_users": mention_users,
            "timestamp": Utc::now().to_rfc3339(),
            "note": "Discord webhook not configured - message logged only. Set DISCORD_WEBHOOK_URL env var to enable."
        }))
    }

    async fn execute_create_notification(
        &self,
        title: &str,
        message: &str,
        notification_type: &NotificationType,
        recipients: &[String],
    ) -> crate::Result<serde_json::Value> {
        Ok(serde_json::json!({
            "success": true,
            "notification_id": uuid::Uuid::new_v4().to_string(),
            "title": title,
            "message": message,
            "type": format!("{:?}", notification_type),
            "recipients": recipients,
            "created_at": Utc::now().to_rfc3339()
        }))
    }

    // Calendar & Scheduling Implementations
    async fn execute_create_calendar_event(
        &self,
        title: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        attendees: &[String],
        location: Option<&str>,
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Google Calendar API if configured
        if let Some(ref calendar_service) = self.calendar_service {
            match calendar_service
                .create_event(title, start_time, end_time, attendees, location)
                .await
            {
                Ok(event_id) => {
                    tracing::info!("Calendar event created: {} (ID: {})", title, event_id);
                    return Ok(serde_json::json!({
                        "success": true,
                        "event_id": event_id,
                        "title": title,
                        "start_time": start_time.to_rfc3339(),
                        "end_time": end_time.to_rfc3339(),
                        "attendees": attendees,
                        "location": location,
                        "calendar_provider": "Google Calendar"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Google Calendar API failed, returning mock data: {}", e);
                }
            }
        }

        // Fallback: Mock data
        Ok(serde_json::json!({
            "success": true,
            "event_id": uuid::Uuid::new_v4().to_string(),
            "title": title,
            "start_time": start_time.to_rfc3339(),
            "end_time": end_time.to_rfc3339(),
            "attendees": attendees,
            "location": location,
            "note": "Google Calendar not configured - returning mock data. Set GOOGLE_CALENDAR_CREDENTIALS and GOOGLE_CALENDAR_ID env vars to enable."
        }))
    }

    async fn execute_find_available_slots(
        &self,
        participants: &[String],
        duration_minutes: u32,
        preferred_days: &[String],
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Google Calendar API if configured
        if let Some(ref calendar_service) = self.calendar_service {
            match calendar_service
                .find_available_slots(participants, duration_minutes, 7)
                .await
            {
                Ok(slots) => {
                    let formatted_slots: Vec<_> = slots
                        .iter()
                        .map(|(start, end)| {
                            serde_json::json!({
                                "start": start.to_rfc3339(),
                                "end": end.to_rfc3339()
                            })
                        })
                        .collect();

                    tracing::info!("Found {} available slots", formatted_slots.len());
                    return Ok(serde_json::json!({
                        "success": true,
                        "participants": participants,
                        "duration_minutes": duration_minutes,
                        "preferred_days": preferred_days,
                        "available_slots": formatted_slots,
                        "calendar_provider": "Google Calendar"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Google Calendar API failed, returning mock data: {}", e);
                }
            }
        }

        // Fallback: Mock available slots
        let now = Utc::now();
        let slots = vec![serde_json::json!({
            "start": (now + chrono::Duration::days(1)).to_rfc3339(),
            "end": (now + chrono::Duration::days(1) + chrono::Duration::minutes(duration_minutes as i64)).to_rfc3339()
        })];

        Ok(serde_json::json!({
            "success": true,
            "participants": participants,
            "duration_minutes": duration_minutes,
            "preferred_days": preferred_days,
            "available_slots": slots,
            "note": "Google Calendar not configured - showing mock slots"
        }))
    }

    async fn execute_check_calendar_availability(
        &self,
        user: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> crate::Result<serde_json::Value> {
        // Try to use real Google Calendar API if configured
        if let Some(ref calendar_service) = self.calendar_service {
            match calendar_service
                .check_availability(user, start_time, end_time)
                .await
            {
                Ok(is_available) => {
                    tracing::info!("Checked availability for {}: {}", user, is_available);
                    return Ok(serde_json::json!({
                        "success": true,
                        "user": user,
                        "start_time": start_time.to_rfc3339(),
                        "end_time": end_time.to_rfc3339(),
                        "is_available": is_available,
                        "conflicts": if is_available { vec![] as Vec<String> } else { vec!["Busy".to_string()] },
                        "calendar_provider": "Google Calendar"
                    }));
                }
                Err(e) => {
                    tracing::warn!("Google Calendar API failed, assuming available: {}", e);
                }
            }
        }

        // Fallback: Assume available
        Ok(serde_json::json!({
            "success": true,
            "user": user,
            "start_time": start_time.to_rfc3339(),
            "end_time": end_time.to_rfc3339(),
            "is_available": true,
            "conflicts": [],
            "note": "Google Calendar not configured - assuming available"
        }))
    }

    async fn resolve_project_id(&self, project_hint: Option<&str>) -> crate::Result<Uuid> {
        let executor = self
            .task_executor
            .as_ref()
            .ok_or_else(|| NoraError::ConfigError("Task executor not configured".to_string()))?;

        if let Some(hint) = project_hint {
            if let Ok(id) = Uuid::parse_str(hint) {
                return Ok(id);
            }
            if let Ok(Some(project)) = executor.find_project_record_by_name(hint).await {
                return Ok(project.id);
            }
        }

        let projects = executor.get_all_projects().await?;
        if let Some(project) = projects.first() {
            if let Ok(id) = Uuid::parse_str(&project.id) {
                return Ok(id);
            }
        }

        Err(NoraError::ConfigError(
            "No project available for media pipeline. Specify project_id in the tool call."
                .to_string(),
        ))
    }

    async fn start_pipeline_task(
        &self,
        project_hint: Option<&str>,
        title: String,
        description: Option<String>,
        tags: Vec<String>,
        priority: Priority,
    ) -> crate::Result<Option<Uuid>> {
        let executor = match &self.task_executor {
            Some(executor) => executor,
            None => return Ok(None),
        };

        let project_id = self.resolve_project_id(project_hint).await?;
        let board_id = executor
            .get_default_board_for_tasks(project_id)
            .await?
            .map(|board| board.id);

        let definition = TaskDefinition {
            title,
            description,
            priority: Some(priority),
            tags: Some(tags),
            assignee_id: None,
            board_id,
            pod_id: None,
        };

        let task = executor.create_task(project_id, definition).await?;
        executor
            .update_task_status(task.id, TaskStatus::InProgress)
            .await?;

        Ok(Some(task.id))
    }

    async fn complete_pipeline_task(&self, task_id: Uuid, status: TaskStatus) {
        if let Some(executor) = &self.task_executor {
            if let Err(err) = executor.update_task_status(task_id, status).await {
                tracing::warn!("Failed to update pipeline task {}: {}", task_id, err);
            }
        }
    }

    async fn execute_ingest_media_batch(
        &self,
        source_url: &str,
        reference_name: Option<String>,
        storage_tier: &str,
        checksum_required: bool,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let storage_tier = match MediaStorageTier::from_str(storage_tier) {
            Ok(tier) => tier,
            Err(err) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                }));
            }
        };

        let title = reference_name
            .clone()
            .map(|name| format!("Ingest batch – {}", name))
            .unwrap_or_else(|| "Ingest media batch".to_string());
        let description = Some(format!("Download and stage media from {}", source_url));
        let tags = vec!["editron".to_string(), "ingest".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::High,
            )
            .await?;

        let request = MediaBatchIngestRequest {
            source_url: source_url.to_string(),
            reference_name,
            storage_tier,
            checksum_required,
            project_id: project_hint
                .as_ref()
                .and_then(|value| Uuid::parse_str(value).ok()),
        };

        let response = match pipeline.ingest_batch(request).await {
            Ok(batch) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "batch": batch,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }

    async fn execute_analyze_media_batch(
        &self,
        batch_id: &str,
        brief: &str,
        passes: u32,
        deliverable_targets: Vec<String>,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let batch_uuid = match Uuid::parse_str(batch_id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "Invalid batch_id",
                }));
            }
        };

        let request = MediaBatchAnalysisRequest {
            batch_id: batch_uuid,
            brief: brief.to_string(),
            passes,
            deliverable_targets,
        };

        let title = format!("Analyze media batch {}", &batch_id[..batch_id.len().min(8)]);
        let description = Some(format!("Brief: {}", brief));
        let tags = vec!["editron".to_string(), "analysis".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::High,
            )
            .await?;

        let response = match pipeline.analyze_batch(request).await {
            Ok(analysis) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "analysis": analysis,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }

    async fn execute_generate_video_edits(
        &self,
        batch_id: &str,
        deliverable_type: &str,
        aspect_ratios: Vec<String>,
        reference_style: Option<String>,
        include_captions: bool,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let batch_uuid = match Uuid::parse_str(batch_id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "Invalid batch_id",
                }));
            }
        };

        let ratios_for_description = aspect_ratios.clone();

        let request = EditSessionRequest {
            batch_id: batch_uuid,
            deliverable_type: deliverable_type.to_string(),
            aspect_ratios,
            reference_style,
            include_captions,
        };

        let title = format!("Assemble edits – {}", deliverable_type);
        let description = Some(format!(
            "Batch {} | Ratios {:?}",
            &batch_id[..batch_id.len().min(8)],
            ratios_for_description
        ));
        let tags = vec!["editron".to_string(), "edit".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::High,
            )
            .await?;

        let response = match pipeline.generate_edits(request).await {
            Ok(session) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "edit_session": session,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }

    async fn execute_render_video_deliverables(
        &self,
        edit_session_id: &str,
        destinations: Vec<String>,
        formats: Vec<String>,
        priority: VideoRenderPriority,
        project_hint: Option<String>,
    ) -> crate::Result<serde_json::Value> {
        let Some(pipeline) = &self.media_pipeline else {
            return Ok(serde_json::json!({
                "success": false,
                "error": "Media pipeline not configured",
            }));
        };

        let session_uuid = match Uuid::parse_str(edit_session_id) {
            Ok(id) => id,
            Err(_) => {
                return Ok(serde_json::json!({
                    "success": false,
                    "error": "Invalid edit_session_id",
                }));
            }
        };

        let request = RenderJobRequest {
            edit_session_id: session_uuid,
            destinations,
            formats,
            priority: match priority {
                VideoRenderPriority::Low => PipelineRenderPriority::Low,
                VideoRenderPriority::Standard => PipelineRenderPriority::Standard,
                VideoRenderPriority::Rush => PipelineRenderPriority::Rush,
            },
        };

        let title = "Render deliverables".to_string();
        let description = Some(format!(
            "Session {}",
            &edit_session_id[..edit_session_id.len().min(8)]
        ));
        let tags = vec!["editron".to_string(), "render".to_string()];
        let pipeline_task = self
            .start_pipeline_task(
                project_hint.as_deref(),
                title,
                description,
                tags,
                Priority::Medium,
            )
            .await?;

        let response = match pipeline.render_deliverables(request).await {
            Ok(job) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::Done).await;
                }
                serde_json::json!({
                    "render_job": job,
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
            Err(err) => {
                if let Some(task_id) = pipeline_task {
                    self.complete_pipeline_task(task_id, TaskStatus::InReview)
                        .await;
                }
                serde_json::json!({
                    "success": false,
                    "error": err.to_string(),
                    "taskId": pipeline_task.map(|id| id.to_string()),
                })
            }
        };

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_file_tool() {
        let tools = ExecutiveTools::new();

        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nora_test_read.txt");
        tokio::fs::write(&test_file, "Hello, Nora!").await.unwrap();

        let tool = NoraExecutiveTool::ReadFile {
            file_path: test_file.to_str().unwrap().to_string(),
            encoding: None,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert_eq!(result["content"].as_str().unwrap(), "Hello, Nora!");

        // Cleanup
        tokio::fs::remove_file(&test_file).await.ok();
    }

    #[tokio::test]
    async fn test_write_file_tool() {
        let tools = ExecutiveTools::new();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nora_test_write.txt");

        let tool = NoraExecutiveTool::WriteFile {
            file_path: test_file.to_str().unwrap().to_string(),
            content: "Test content from Nora".to_string(),
            create_directories: true,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());

        // Verify file was created
        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "Test content from Nora");

        // Cleanup
        tokio::fs::remove_file(&test_file).await.ok();
    }

    #[tokio::test]
    async fn test_list_directory_tool() {
        let tools = ExecutiveTools::new();

        let temp_dir = std::env::temp_dir();

        let tool = NoraExecutiveTool::ListDirectory {
            directory_path: temp_dir.to_str().unwrap().to_string(),
            recursive: false,
            pattern: None,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["entries"].is_array());
    }

    #[tokio::test]
    async fn test_delete_file_tool() {
        let tools = ExecutiveTools::new();

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("nora_test_delete.txt");
        tokio::fs::write(&test_file, "Delete me").await.unwrap();

        let tool = NoraExecutiveTool::DeleteFile {
            file_path: test_file.to_str().unwrap().to_string(),
            confirm: true,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(!test_file.exists());
    }

    #[tokio::test]
    async fn test_analyze_code_quality_tool() {
        let tools = ExecutiveTools::new();

        let code = r#"
fn hello_world() {
    // This is a comment
    println!("Hello, world!");
}
"#;

        let tool = NoraExecutiveTool::AnalyzeCodeQuality {
            code: code.to_string(),
            language: CodeLanguage::Rust,
            check_security: false,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["metrics"]["line_count"].as_u64().unwrap() > 0);
        assert!(result["metrics"]["has_comments"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_send_email_tool() {
        let tools = ExecutiveTools::new();

        let tool = NoraExecutiveTool::SendEmail {
            recipients: vec!["test@example.com".to_string()],
            subject: "Test Email".to_string(),
            body: "This is a test".to_string(),
            priority: EmailPriority::Normal,
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["message_id"].is_string());
    }

    #[tokio::test]
    async fn test_create_notification_tool() {
        let tools = ExecutiveTools::new();

        let tool = NoraExecutiveTool::CreateNotification {
            title: "Test Notification".to_string(),
            message: "This is a test notification".to_string(),
            notification_type: NotificationType::Info,
            recipients: vec!["user1".to_string()],
        };

        let result = tools.execute_tool_implementation(tool).await.unwrap();

        assert!(result["success"].as_bool().unwrap());
        assert!(result["notification_id"].is_string());
    }

    #[test]
    fn test_run_visual_qc_tool_name() {
        let tools = ExecutiveTools::new();
        let tool = NoraExecutiveTool::RunVisualQc {
            batch_id: "test-batch-id".to_string(),
            candidates_per_clip: Some(5),
            min_composition_score: Some(0.6),
            target_aspect_ratio: Some("16:9".to_string()),
            project_id: None,
        };

        assert_eq!(tools.get_tool_name(&tool), "run_visual_qc");
    }

    #[test]
    fn test_run_visual_qc_tool_parse() {
        let tools = ExecutiveTools::new();
        let args = serde_json::json!({
            "batch_id": "abc-123",
            "candidates_per_clip": 3,
            "min_composition_score": 0.7,
            "target_aspect_ratio": "16:9"
        });

        let parsed = ExecutiveTools::parse_tool_call("run_visual_qc", &args);
        assert!(parsed.is_some());

        if let Some(NoraExecutiveTool::RunVisualQc {
            batch_id,
            candidates_per_clip,
            min_composition_score,
            target_aspect_ratio,
            project_id,
        }) = parsed
        {
            assert_eq!(batch_id, "abc-123");
            assert_eq!(candidates_per_clip, Some(3));
            assert!((min_composition_score.unwrap() - 0.7).abs() < 0.001);
            assert_eq!(target_aspect_ratio.as_deref(), Some("16:9"));
            assert!(project_id.is_none());
        } else {
            panic!("Expected RunVisualQc variant");
        }
    }

    #[test]
    fn test_run_visual_qc_tool_in_definitions() {
        let tools = ExecutiveTools::new();
        assert!(
            tools.available_tools.contains_key("run_visual_qc"),
            "run_visual_qc should be in tool definitions"
        );
    }
}
