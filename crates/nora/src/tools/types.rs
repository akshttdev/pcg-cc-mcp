//! Type definitions for executive tools

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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
    /// Delete a project by name (permanently removes the project and all its tasks)
    DeleteProject {
        project_name: String,
    },
    /// Update a project's name or description
    UpdateProject {
        project_name: String,
        new_name: Option<String>,
        new_description: Option<String>,
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
    RenderPage {
        url: String,
        include_html: bool,
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
    /// Read Nora's inbox (or an org/project inbox when owner is specified)
    ReadInbox {
        limit: usize,
        /// Optional: "agent", "organization", "project" — defaults to Nora's own account
        owner_type: Option<String>,
        /// Optional: UUID of the owner (required when owner_type is set)
        owner_id: Option<String>,
    },
    /// Send an SMS from Nora's Twilio number
    SendSms {
        to: String,
        message: String,
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
        project_name: Option<String>,
    },

    /// Execute an FFmpeg render script produced by AssembleRecapEdit
    ExecuteRenderScript {
        render_script: String,
        render_output: String,
        xml_path: String,
    },

    /// Federated music search across all configured platforms
    SearchMusic {
        query: Option<String>,
        moods: Option<Vec<String>>,
        genres: Option<Vec<String>>,
        min_bpm: Option<u32>,
        max_bpm: Option<u32>,
        min_duration: Option<f64>,
        max_duration: Option<f64>,
        instrumental: Option<bool>,
        platforms: Option<Vec<String>>,
        page: Option<u32>,
        per_page: Option<u32>,
    },

    /// Download a music track by platform-prefixed ID
    DownloadMusicTrack {
        track_id: String,
        filename: Option<String>,
        output_dir: Option<String>,
    },

    /// Recommend music based on video content analysis or content type
    RecommendMusicForVideo {
        video_path: Option<String>,
        content_type: Option<String>,
        target_duration: Option<f64>,
        auto_search: Option<bool>,
    },

    /// Get preview/stream URL and basic info for a track
    PreviewMusicTrack {
        track_id: String,
    },

    /// Get full metadata for a specific track
    GetMusicTrackDetails {
        track_id: String,
    },

    /// Analyze a local audio file for BPM, energy, sections
    AnalyzeMusicTrack {
        audio_path: String,
        bpm_hint: Option<f64>,
    },

    // ── AI Image Generation (fal.ai) ────────────────────────────────────────

    /// Generate a photorealistic image via fal.ai FLUX Pro Ultra.
    /// Used by Editron for avatar portraits, character references, and visual assets.
    GenerateImage {
        /// Text prompt describing the image to generate
        prompt: String,
        /// Optional reference image URL or data URI for img2img / style transfer
        reference_image_url: Option<String>,
        /// Aspect ratio: "1:1", "3:4", "4:3", "16:9", "9:16"
        aspect_ratio: Option<String>,
        /// Strength of reference image influence (0.0-1.0). Only used with reference_image_url.
        reference_strength: Option<f64>,
        /// Enable raw photographic mode (less AI polish, more camera-like)
        raw_mode: Option<bool>,
        /// Fixed seed for reproducible results
        seed: Option<u64>,
        /// fal.ai model endpoint. Defaults to "fal-ai/flux-pro/v1.1-ultra"
        model: Option<String>,
        /// Output filename (saved to dev_assets/video_gen/portraits/)
        output_filename: Option<String>,
        /// Task to attach the generated image artifact to
        task_id: Option<String>,
        /// Project for VIBE billing
        project_id: Option<String>,
    },

    // ── Video Post-Processing ────────────────────────────────────────────────

    /// Apply a visual effect preset to a video file using FFmpeg.
    /// Editron's post-production finishing tool for HeyGen outputs.
    ApplyVideoEffect {
        /// Path to input video file, or video_job_id to look up automatically
        input_path: String,
        /// Effect preset to apply:
        /// "hologram_glitch" — RGB split + scanlines + pixel dissolve on outro
        /// "color_grade"     — Cinematic LUT + contrast enhancement
        /// "vignette"        — Dark studio vignette overlay
        /// "captions"        — Burn-in captions from SRT file
        effect: String,
        /// Output filename (saved alongside input if not specified)
        output_path: Option<String>,
        /// Effect-specific parameters as JSON (e.g. glitch start time, LUT name)
        effect_params: Option<serde_json::Value>,
        /// Task to attach the processed video artifact to
        task_id: Option<String>,
        /// Project for VIBE billing
        project_id: Option<String>,
    },

    /// Run the full post-production chain on a completed HeyGen video job.
    /// Fetches the raw video, applies effect chain, saves final deliverable.
    PostProcessVideoJob {
        /// video_jobs.id UUID
        video_job_id: String,
        /// Ordered list of effects to apply: ["hologram_glitch", "color_grade"]
        effects: Vec<String>,
        /// Effect parameters keyed by effect name
        effect_params: Option<serde_json::Value>,
        /// Task to attach the final deliverable artifact to
        task_id: Option<String>,
        /// Project for VIBE billing
        project_id: Option<String>,
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
