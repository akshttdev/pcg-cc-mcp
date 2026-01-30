//! Conference Content Pipeline
//!
//! Handles intake and task generation for conference coverage workflows.
//! Creates a Board within an existing Project (e.g., PCG) for each conference.
//!
//! Standard conference package includes:
//! - Pre-Event: Speakers Article, Side Events Article, Press Release
//! - Post-Event: Recap Video, Highlight Reels, Speaker Clips, Speaker Articles

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use ts_rs::TS;
use uuid::Uuid;

use cinematics::CinematicsService;

use crate::{
    conference_workflow::{ConferenceWorkflowEngine, WorkflowConfig},
    execution::ExecutionEngine,
    executor::{TaskDefinition, TaskExecutor},
    NoraError, Result,
};
use db::models::{
    entity::{CreateEntity, Entity, EntityType},
    entity_appearance::{AppearanceType, EntityAppearance},
    project::Project,
    project_board::{CreateProjectBoard, ProjectBoard, ProjectBoardType},
    task::Priority,
    task_dependency::{CreateTaskDependency, DependencyType, TaskDependency},
};

/// Simple slugify function - converts string to lowercase kebab-case
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Role types for task assignment
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskRole {
    /// Research and information gathering
    Researcher,
    /// Content writing and copywriting
    Writer,
    /// Video editing and post-production
    Editor,
    /// Graphics and thumbnail creation
    Designer,
    /// Social media and distribution
    Publisher,
    /// Project oversight and coordination
    ProjectManager,
}

impl TaskRole {
    /// Get the role name as a string for assignment
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskRole::Researcher => "researcher",
            TaskRole::Writer => "writer",
            TaskRole::Editor => "editor",
            TaskRole::Designer => "designer",
            TaskRole::Publisher => "publisher",
            TaskRole::ProjectManager => "project_manager",
        }
    }
}

/// Conference details for intake
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceIntake {
    /// Conference name (e.g., "iConnection Conference 2026")
    pub name: String,
    /// Start date of the conference
    pub start_date: NaiveDate,
    /// End date of the conference
    pub end_date: NaiveDate,
    /// Location (city, venue)
    pub location: String,
    /// Conference website or info URL
    pub website: Option<String>,
    /// List of speakers (can be populated later via research)
    #[serde(default)]
    pub speakers: Vec<Speaker>,
    /// List of side events
    #[serde(default)]
    pub side_events: Vec<SideEvent>,
    /// Coverage package type
    #[serde(default)]
    pub package: ConferencePackage,
    /// Additional notes
    pub notes: Option<String>,
    /// Parent project ID (e.g., PCG project) - if None, will look up "PCG" project
    pub project_id: Option<Uuid>,
}

/// Speaker information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Speaker {
    pub name: String,
    pub title: Option<String>,
    pub company: Option<String>,
    pub topic: Option<String>,
    pub bio: Option<String>,
}

/// Side event information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SideEvent {
    pub name: String,
    pub date: Option<NaiveDate>,
    pub time: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
}

/// Coverage package types
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum ConferencePackage {
    /// Full coverage: all pre-event + post-event deliverables
    #[default]
    Standard,
    /// Pre-event content only
    PreEventOnly,
    /// Post-event content only
    PostEventOnly,
    /// Custom selection of deliverables
    Custom(Vec<String>),
}

/// Result of conference intake
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceIntakeResult {
    pub project_id: Uuid,
    pub project_name: String,
    pub board_id: Uuid,
    pub board_name: String,
    pub tasks_created: Vec<TaskSummary>,
    pub dependencies_created: usize,
    pub conference_metadata: serde_json::Value,
    /// ID of the automated workflow (if launched)
    pub workflow_id: Option<Uuid>,
}

/// Summary of created task
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    pub id: Uuid,
    pub title: String,
    pub due_date: Option<DateTime<Utc>>,
    pub phase: String,
    pub role: String,
}

/// Internal struct for tracking created tasks and their relationships
struct CreatedTask {
    id: Uuid,
    title: String,
    due_date: Option<DateTime<Utc>>,
    phase: String,
    role: TaskRole,
    task_type: TaskType,
    speaker_name: Option<String>,
}

/// Task types for dependency linking
#[derive(Debug, Clone, PartialEq)]
enum TaskType {
    Research,
    SpeakersArticle,
    SideEventsArticle,
    PressRelease,
    FootageIntake,
    RecapVideo,
    HighlightReels,
    SpeakerClip(String),    // Speaker name
    SpeakerArticle(String), // Speaker name
}

/// Conference Content Pipeline
pub struct ConferencePipeline {
    pool: SqlitePool,
    task_executor: TaskExecutor,
    execution_engine: Option<Arc<ExecutionEngine>>,
    /// CinematicsService for Maci (AI graphics via ComfyUI/SDXL)
    cinematics: Option<Arc<CinematicsService>>,
    /// Whether to automatically launch the workflow engine
    auto_launch_workflow: bool,
}

impl ConferencePipeline {
    pub fn new(pool: SqlitePool) -> Self {
        let task_executor = TaskExecutor::new(pool.clone());
        Self {
            pool,
            task_executor,
            execution_engine: None,
            cinematics: None,
            auto_launch_workflow: false,
        }
    }

    /// Create pipeline with execution engine for automated workflows
    pub fn with_execution_engine(pool: SqlitePool, execution_engine: Arc<ExecutionEngine>) -> Self {
        let task_executor = TaskExecutor::new(pool.clone());
        Self {
            pool,
            task_executor,
            execution_engine: Some(execution_engine),
            cinematics: None,
            auto_launch_workflow: true,
        }
    }

    /// Create pipeline with full services (execution engine + cinematics)
    pub fn with_services(
        pool: SqlitePool,
        execution_engine: Arc<ExecutionEngine>,
        cinematics: Option<Arc<CinematicsService>>,
    ) -> Self {
        let task_executor = TaskExecutor::new(pool.clone());
        Self {
            pool,
            task_executor,
            execution_engine: Some(execution_engine),
            cinematics,
            auto_launch_workflow: true,
        }
    }

    /// Set the CinematicsService for Maci graphics generation
    pub fn set_cinematics(&mut self, cinematics: Arc<CinematicsService>) {
        self.cinematics = Some(cinematics);
    }

    /// Enable or disable automatic workflow launching
    pub fn set_auto_launch_workflow(&mut self, enabled: bool) {
        self.auto_launch_workflow = enabled;
    }

    /// Intake a conference and create the board + tasks within the parent project
    pub async fn intake_conference(&self, intake: ConferenceIntake) -> Result<ConferenceIntakeResult> {
        tracing::info!(
            "[CONFERENCE_PIPELINE] Intaking conference: {} ({} - {})",
            intake.name,
            intake.start_date,
            intake.end_date
        );

        // 1. Find or resolve the parent project
        let project_id = match intake.project_id {
            Some(id) => id,
            None => self.find_pcg_project().await?,
        };

        let project = Project::find_by_id(&self.pool, project_id)
            .await
            .map_err(NoraError::DatabaseError)?
            .ok_or_else(|| NoraError::ConfigError(format!("Project {} not found", project_id)))?;

        tracing::info!(
            "[CONFERENCE_PIPELINE] Using parent project: {} ({})",
            project.name,
            project_id
        );

        // 2. Create a board for this conference within the project
        let board = self.create_conference_board(project_id, &intake).await?;
        let board_id = board.id;

        tracing::info!(
            "[CONFERENCE_PIPELINE] Created board: {} ({})",
            board.name,
            board_id
        );

        // 3. Generate tasks on the board
        let created_tasks = self.generate_conference_tasks(project_id, board_id, &intake).await?;

        tracing::info!(
            "[CONFERENCE_PIPELINE] Created {} tasks for conference",
            created_tasks.len()
        );

        // 4. Create task dependencies (RelatesTo)
        let dependencies_count = self.create_task_dependencies(project_id, &created_tasks).await?;

        tracing::info!(
            "[CONFERENCE_PIPELINE] Created {} task dependencies",
            dependencies_count
        );

        // 4.5. Pre-populate Entity records from intake speakers (so workflow doesn't hallucinate)
        let entities_created = self.create_speaker_entities(&intake, board_id).await?;
        if entities_created > 0 {
            tracing::info!(
                "[CONFERENCE_PIPELINE] Pre-populated {} speaker entities from intake",
                entities_created
            );
        }

        // 5. Build result
        let task_summaries: Vec<TaskSummary> = created_tasks
            .iter()
            .map(|t| TaskSummary {
                id: t.id,
                title: t.title.clone(),
                due_date: t.due_date,
                phase: t.phase.clone(),
                role: t.role.as_str().to_string(),
            })
            .collect();

        let metadata = serde_json::json!({
            "type": "conference",
            "name": intake.name,
            "start_date": intake.start_date.to_string(),
            "end_date": intake.end_date.to_string(),
            "location": intake.location,
            "website": intake.website,
            "speakers_count": intake.speakers.len(),
            "side_events_count": intake.side_events.len(),
            "package": format!("{:?}", intake.package),
        });

        // 6. Launch automated workflow (if enabled)
        let workflow_id = if self.auto_launch_workflow {
            if let Some(execution_engine) = &self.execution_engine {
                match self.launch_workflow_engine(&intake, board_id, execution_engine.clone()).await {
                    Ok(id) => {
                        tracing::info!(
                            "[CONFERENCE_PIPELINE] Launched workflow engine: {}",
                            id
                        );
                        Some(id)
                    }
                    Err(e) => {
                        tracing::warn!(
                            "[CONFERENCE_PIPELINE] Failed to launch workflow engine: {}",
                            e
                        );
                        None
                    }
                }
            } else {
                tracing::debug!(
                    "[CONFERENCE_PIPELINE] Workflow engine not available, skipping auto-launch"
                );
                None
            }
        } else {
            None
        };

        Ok(ConferenceIntakeResult {
            project_id,
            project_name: project.name,
            board_id,
            board_name: board.name,
            tasks_created: task_summaries,
            dependencies_created: dependencies_count,
            conference_metadata: metadata,
            workflow_id,
        })
    }

    /// Launch the automated conference workflow engine
    async fn launch_workflow_engine(
        &self,
        intake: &ConferenceIntake,
        board_id: Uuid,
        execution_engine: Arc<ExecutionEngine>,
    ) -> Result<Uuid> {
        // Pass CinematicsService (Maci) for AI graphics via ComfyUI/SDXL.
        // When cinematics is None, gradient backgrounds will be used as fallback.
        let workflow_engine = ConferenceWorkflowEngine::new(
            self.pool.clone(),
            execution_engine,
            self.cinematics.clone(),
        );

        // Initialize the workflow
        let workflow_id = workflow_engine.initialize(intake, board_id).await?;

        // Spawn the workflow to run in the background
        let pool = self.pool.clone();
        tokio::spawn(async move {
            tracing::info!(
                "[CONFERENCE_WORKFLOW] Starting automated workflow: {}",
                workflow_id
            );

            match workflow_engine.run_workflow(workflow_id).await {
                Ok(result) => {
                    tracing::info!(
                        "[CONFERENCE_WORKFLOW] Workflow completed successfully: {} entities, {} posts",
                        result.entities_created,
                        result.social_posts_scheduled
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "[CONFERENCE_WORKFLOW] Workflow failed: {}",
                        e
                    );

                    // Update workflow status to failed
                    if let Err(update_err) = db::models::conference_workflow::ConferenceWorkflow::update_status(
                        &pool,
                        workflow_id,
                        db::models::conference_workflow::WorkflowStatus::Failed,
                    ).await {
                        tracing::error!(
                            "[CONFERENCE_WORKFLOW] Failed to update workflow status: {}",
                            update_err
                        );
                    }
                }
            }
        });

        Ok(workflow_id)
    }

    /// Find the PCG project by name
    async fn find_pcg_project(&self) -> Result<Uuid> {
        // Try to find project named "PCG" or "PowerClub Global"
        for name in &["PCG", "PowerClub Global", "powerclub"] {
            if let Ok(Some(project)) = Project::find_by_name_case_insensitive(&self.pool, name).await {
                return Ok(project.id);
            }
        }

        Err(NoraError::ConfigError(
            "PCG project not found. Please create a project named 'PCG' first, or specify project_id.".to_string()
        ))
    }

    /// Create a board for the conference within the project
    async fn create_conference_board(&self, project_id: Uuid, intake: &ConferenceIntake) -> Result<ProjectBoard> {
        let slug = slugify(&intake.name);

        // Check if board already exists
        if let Ok(Some(existing)) = ProjectBoard::find_by_slug(&self.pool, project_id, &slug).await {
            tracing::info!(
                "[CONFERENCE_PIPELINE] Board '{}' already exists, using existing",
                intake.name
            );
            return Ok(existing);
        }

        // Create new board
        let create_board = CreateProjectBoard {
            project_id,
            name: intake.name.clone(),
            slug,
            board_type: ProjectBoardType::Custom,
            description: Some(format!(
                "Conference coverage for {} ({} - {}) in {}",
                intake.name, intake.start_date, intake.end_date, intake.location
            )),
            metadata: Some(serde_json::json!({
                "type": "conference",
                "start_date": intake.start_date.to_string(),
                "end_date": intake.end_date.to_string(),
                "location": intake.location,
            }).to_string()),
        };

        ProjectBoard::create(&self.pool, &create_board)
            .await
            .map_err(NoraError::DatabaseError)
    }

    /// Generate all conference tasks
    async fn generate_conference_tasks(
        &self,
        project_id: Uuid,
        board_id: Uuid,
        intake: &ConferenceIntake,
    ) -> Result<Vec<CreatedTask>> {
        let mut created_tasks = Vec::new();

        // Calculate key dates
        let start_dt = date_to_datetime(intake.start_date);
        let end_dt = date_to_datetime(intake.end_date);

        match &intake.package {
            ConferencePackage::Standard => {
                created_tasks.extend(
                    self.create_pre_event_tasks(project_id, board_id, intake, start_dt).await?
                );
                created_tasks.extend(
                    self.create_post_event_tasks(project_id, board_id, intake, end_dt).await?
                );
            }
            ConferencePackage::PreEventOnly => {
                created_tasks.extend(
                    self.create_pre_event_tasks(project_id, board_id, intake, start_dt).await?
                );
            }
            ConferencePackage::PostEventOnly => {
                created_tasks.extend(
                    self.create_post_event_tasks(project_id, board_id, intake, end_dt).await?
                );
            }
            ConferencePackage::Custom(_) => {
                // For now, treat custom as standard
                created_tasks.extend(
                    self.create_pre_event_tasks(project_id, board_id, intake, start_dt).await?
                );
                created_tasks.extend(
                    self.create_post_event_tasks(project_id, board_id, intake, end_dt).await?
                );
            }
        }

        Ok(created_tasks)
    }

    /// Create pre-event tasks
    async fn create_pre_event_tasks(
        &self,
        project_id: Uuid,
        board_id: Uuid,
        intake: &ConferenceIntake,
        start_dt: DateTime<Utc>,
    ) -> Result<Vec<CreatedTask>> {
        let mut created = Vec::new();
        let conference_slug = slugify(&intake.name);

        // Research task (5 days before) - Researcher role
        let research_task = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Research speakers for {}", intake.name),
                description: Some(format!(
                    "Research and compile comprehensive speaker list for {}.\n\n\
                    **IMPORTANT: You MUST create output files with your research findings.**\n\n\
                    ## Required Output Files\n\
                    1. `{}-speakers.md` - Main speaker research document with:\n\
                       - Conference overview and key themes\n\
                       - Featured speakers (5-10 highlighted with detailed bios)\n\
                       - Full speaker list in table format (Name, Title, Company, Topic)\n\
                       - Notable quotes or talking points\n\
                       - Sources and references\n\n\
                    2. `{}-conference-intel.md` - Conference intelligence with:\n\
                       - Event details (dates, venue, expected attendance)\n\
                       - Key sponsors and partners\n\
                       - Competing or related events\n\
                       - Industry trends and themes\n\n\
                    ## Research Sources\n\
                    - Conference website: {}\n\
                    - LinkedIn profiles of speakers\n\
                    - Press releases and announcements\n\
                    - Social media (Twitter/X, etc.)\n\n\
                    Location: {}\n\n\
                    **DO NOT complete this task without creating the required output files.**",
                    intake.name, conference_slug, conference_slug,
                    intake.website.as_deref().unwrap_or("Search for official website"),
                    intake.location
                )),
                priority: Some(Priority::High),
                tags: Some(vec!["pre-event".into(), "research".into(), conference_slug.clone()]),
                board_id: Some(board_id),
                due_date: Some(start_dt - Duration::days(5)),
                scheduled_start: Some(start_dt - Duration::days(6)),
                scheduled_end: Some(start_dt - Duration::days(5)),
                custom_properties: Some(serde_json::json!({
                    "phase": "pre-event",
                    "deliverable_type": "research",
                    "conference": intake.name,
                    "required_role": "researcher",
                    "workflow_stage_required": "intake",
                })),
                ..Default::default()
            },
            TaskRole::Researcher,
            TaskType::Research,
            None,
        ).await?;
        created.push(research_task);

        // Speakers Article (3 days before) - Writer role
        let speakers_article = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Write Speakers Article for {}", intake.name),
                description: Some(format!(
                    "Create comprehensive article highlighting key speakers at {}.\n\n\
                    **REQUIRED OUTPUT:** Create file `{}-speakers-article.md`\n\n\
                    ## Article Structure\n\
                    1. **Headline** - Compelling title for the article\n\
                    2. **Introduction** (100-150 words) - Set the scene for the conference\n\
                    3. **Featured Speakers** (400-600 words) - Deep dive on 3-5 key speakers with:\n\
                       - Brief bio and credentials\n\
                       - What they'll be speaking about\n\
                       - Why attendees should be excited\n\
                    4. **Full Speaker Lineup** - Table format with all confirmed speakers\n\
                    5. **Conference Details** - Dates, location, how to attend\n\
                    6. **Call to Action** - Encourage readers to register/attend\n\n\
                    ## Requirements\n\
                    - Word count: 800-1200 words\n\
                    - Professional tone suitable for publication\n\
                    - Include photo placeholders: [SPEAKER_PHOTO: Name]\n\
                    - Cite sources where applicable\n\n\
                    **DO NOT complete this task without creating the output file.**",
                    intake.name, conference_slug
                )),
                priority: Some(Priority::High),
                tags: Some(vec!["pre-event".into(), "content".into(), "article".into(), conference_slug.clone()]),
                board_id: Some(board_id),
                due_date: Some(start_dt - Duration::days(3)),
                scheduled_start: Some(start_dt - Duration::days(4)),
                scheduled_end: Some(start_dt - Duration::days(3)),
                custom_properties: Some(serde_json::json!({
                    "phase": "pre-event",
                    "deliverable_type": "article",
                    "article_type": "speakers",
                    "conference": intake.name,
                    "required_role": "writer",
                    "workflow_stage_required": "speaker_research",
                })),
                ..Default::default()
            },
            TaskRole::Writer,
            TaskType::SpeakersArticle,
            None,
        ).await?;
        created.push(speakers_article);

        // Side Events Article (2 days before) - Writer role
        let side_events_article = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Write Side Events Article for {}", intake.name),
                description: Some(format!(
                    "Create guide to side events and satellite activities around {}.\n\n\
                    **REQUIRED OUTPUT:** Create file `{}-side-events.md`\n\n\
                    ## Article Structure\n\
                    1. **Introduction** - Overview of the side event ecosystem\n\
                    2. **Official Side Events** - Events organized by the conference\n\
                    3. **Community Meetups** - Grassroots gatherings and networking\n\
                    4. **Networking Events** - Professional networking opportunities\n\
                    5. **After-Parties** - Evening social events\n\n\
                    ## For Each Event Include\n\
                    - Event name and organizer\n\
                    - Date and time\n\
                    - Location/venue\n\
                    - How to register/attend\n\
                    - Capacity/availability notes\n\n\
                    ## Requirements\n\
                    - Word count: 600-900 words\n\
                    - Include links where available\n\
                    - Organize chronologically or by category\n\n\
                    **DO NOT complete this task without creating the output file.**",
                    intake.name, conference_slug
                )),
                priority: Some(Priority::Medium),
                tags: Some(vec!["pre-event".into(), "content".into(), "article".into(), conference_slug.clone()]),
                board_id: Some(board_id),
                due_date: Some(start_dt - Duration::days(2)),
                scheduled_start: Some(start_dt - Duration::days(3)),
                scheduled_end: Some(start_dt - Duration::days(2)),
                custom_properties: Some(serde_json::json!({
                    "phase": "pre-event",
                    "deliverable_type": "article",
                    "article_type": "side_events",
                    "conference": intake.name,
                    "required_role": "writer",
                    "workflow_stage_required": "side_events",
                })),
                ..Default::default()
            },
            TaskRole::Writer,
            TaskType::SideEventsArticle,
            None,
        ).await?;
        created.push(side_events_article);

        // Press Release (1 day before) - Writer role
        let press_release = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Pre-Event Press Release for {}", intake.name),
                description: Some(format!(
                    "Create press release announcing PCG's coverage of {}.\n\n\
                    **REQUIRED OUTPUT:** Create file `{}-press-release.md`\n\n\
                    ## Press Release Structure\n\
                    1. **Headline** - Attention-grabbing announcement\n\
                    2. **Dateline** - Location, Date\n\
                    3. **Lead Paragraph** - Who, What, When, Where, Why\n\
                    4. **Body** - Details about coverage plans:\n\
                       - What: PCG covering the conference\n\
                       - When: {} to {}\n\
                       - Where: {}\n\
                       - Coverage scope: Articles, video content, interviews\n\
                    5. **Quote** - Statement from PCG representative\n\
                    6. **Boilerplate** - About PCG\n\
                    7. **Contact Information**\n\n\
                    ## Requirements\n\
                    - Word count: 400-600 words\n\
                    - Professional press release format\n\
                    - Include social media handles and hashtags\n\n\
                    **DO NOT complete this task without creating the output file.**",
                    intake.name, conference_slug, intake.start_date, intake.end_date, intake.location
                )),
                priority: Some(Priority::High),
                tags: Some(vec!["pre-event".into(), "content".into(), "press-release".into(), conference_slug.clone()]),
                board_id: Some(board_id),
                due_date: Some(start_dt - Duration::days(1)),
                scheduled_start: Some(start_dt - Duration::days(2)),
                scheduled_end: Some(start_dt - Duration::days(1)),
                custom_properties: Some(serde_json::json!({
                    "phase": "pre-event",
                    "deliverable_type": "press_release",
                    "conference": intake.name,
                    "required_role": "writer",
                    "workflow_stage_required": "research_complete",
                })),
                ..Default::default()
            },
            TaskRole::Writer,
            TaskType::PressRelease,
            None,
        ).await?;
        created.push(press_release);

        Ok(created)
    }

    /// Create post-event tasks
    async fn create_post_event_tasks(
        &self,
        project_id: Uuid,
        board_id: Uuid,
        intake: &ConferenceIntake,
        end_dt: DateTime<Utc>,
    ) -> Result<Vec<CreatedTask>> {
        let mut created = Vec::new();
        let conference_slug = slugify(&intake.name);

        // Footage intake (1 day after) - Editor role
        let footage_intake = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Intake raw footage from {}", intake.name),
                description: Some(format!(
                    "Collect and organize all raw footage from {}.\n\n\
                    Tasks:\n\
                    - Download from capture devices/cloud\n\
                    - Organize by: Date, Speaker, Event type\n\
                    - Create proxy files for editing\n\
                    - Log key moments and timestamps\n\
                    - Backup to archive storage",
                    intake.name
                )),
                priority: Some(Priority::Critical),
                tags: Some(vec!["post-event".into(), "footage".into(), "intake".into(), conference_slug.clone()]),
                board_id: Some(board_id),
                due_date: Some(end_dt + Duration::days(1)),
                scheduled_start: Some(end_dt),
                scheduled_end: Some(end_dt + Duration::days(1)),
                custom_properties: Some(serde_json::json!({
                    "phase": "post-event",
                    "deliverable_type": "footage_intake",
                    "conference": intake.name,
                    "required_role": "editor",
                })),
                ..Default::default()
            },
            TaskRole::Editor,
            TaskType::FootageIntake,
            None,
        ).await?;
        created.push(footage_intake);

        // Recap Video (3 days after) - Editor role
        let recap_video = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Create Recap Video for {}", intake.name),
                description: Some(format!(
                    "Produce comprehensive recap video for {}.\n\n\
                    Specifications:\n\
                    - Duration: 60-90 seconds\n\
                    - Formats: 16:9 (YouTube) + 9:16 (Reels/TikTok)\n\
                    - Include: Crowd shots, speaker highlights, venue atmosphere\n\
                    - Music: Licensed background track\n\
                    - Graphics: Conference branding, PCG watermark\n\
                    - Captions: Auto-generated, reviewed",
                    intake.name
                )),
                priority: Some(Priority::High),
                tags: Some(vec!["post-event".into(), "video".into(), "recap".into(), conference_slug.clone()]),
                board_id: Some(board_id),
                due_date: Some(end_dt + Duration::days(3)),
                scheduled_start: Some(end_dt + Duration::days(1)),
                scheduled_end: Some(end_dt + Duration::days(3)),
                custom_properties: Some(serde_json::json!({
                    "phase": "post-event",
                    "deliverable_type": "video",
                    "video_type": "recap",
                    "conference": intake.name,
                    "required_role": "editor",
                })),
                ..Default::default()
            },
            TaskRole::Editor,
            TaskType::RecapVideo,
            None,
        ).await?;
        created.push(recap_video);

        // Highlight Reels (5 days after) - Editor role
        let highlight_reels = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Create Highlight Reels for {}", intake.name),
                description: Some(format!(
                    "Produce thematic highlight reels from {}.\n\n\
                    Reels to create:\n\
                    1. Best speaker moments (15-30 sec)\n\
                    2. Networking/crowd energy (15-30 sec)\n\
                    3. Key announcements (15-30 sec)\n\n\
                    Formats: 9:16, 1:1, 16:9\n\
                    Each optimized for platform (IG Reels, TikTok, YouTube Shorts)",
                    intake.name
                )),
                priority: Some(Priority::High),
                tags: Some(vec!["post-event".into(), "video".into(), "highlights".into(), conference_slug.clone()]),
                board_id: Some(board_id),
                due_date: Some(end_dt + Duration::days(5)),
                scheduled_start: Some(end_dt + Duration::days(3)),
                scheduled_end: Some(end_dt + Duration::days(5)),
                custom_properties: Some(serde_json::json!({
                    "phase": "post-event",
                    "deliverable_type": "video",
                    "video_type": "highlight_reels",
                    "conference": intake.name,
                    "required_role": "editor",
                })),
                ..Default::default()
            },
            TaskRole::Editor,
            TaskType::HighlightReels,
            None,
        ).await?;
        created.push(highlight_reels);

        // Create speaker-specific tasks
        for (idx, speaker) in intake.speakers.iter().enumerate() {
            let speaker_tasks = self.create_speaker_tasks(
                project_id,
                board_id,
                intake,
                speaker,
                end_dt,
                idx,
            ).await?;
            created.extend(speaker_tasks);
        }

        Ok(created)
    }

    /// Create tasks for individual speaker content
    async fn create_speaker_tasks(
        &self,
        project_id: Uuid,
        board_id: Uuid,
        intake: &ConferenceIntake,
        speaker: &Speaker,
        end_dt: DateTime<Utc>,
        speaker_index: usize,
    ) -> Result<Vec<CreatedTask>> {
        let mut created = Vec::new();
        let conference_slug = slugify(&intake.name);
        let speaker_slug = slugify(&speaker.name);

        // Stagger speaker content by 1 day per speaker, starting 5 days after event
        let base_offset = 5 + speaker_index as i64;

        // Speaker Clip - Editor role
        let clip_task = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Create clip: {} at {}", speaker.name, intake.name),
                description: Some(format!(
                    "Extract and edit speaker clip for {}.\n\n\
                    Speaker: {}\n\
                    Title: {}\n\
                    Topic: {}\n\n\
                    Clip specs:\n\
                    - Duration: 60-120 seconds (best moment)\n\
                    - Include speaker intro graphic\n\
                    - Add captions\n\
                    - Formats: 16:9 + 9:16",
                    intake.name,
                    speaker.name,
                    speaker.title.as_deref().unwrap_or("N/A"),
                    speaker.topic.as_deref().unwrap_or("N/A"),
                )),
                priority: Some(Priority::Medium),
                tags: Some(vec![
                    "post-event".into(),
                    "video".into(),
                    "speaker-clip".into(),
                    conference_slug.clone(),
                    speaker_slug.clone(),
                ]),
                board_id: Some(board_id),
                due_date: Some(end_dt + Duration::days(base_offset)),
                scheduled_start: Some(end_dt + Duration::days(base_offset - 1)),
                scheduled_end: Some(end_dt + Duration::days(base_offset)),
                custom_properties: Some(serde_json::json!({
                    "phase": "post-event",
                    "deliverable_type": "video",
                    "video_type": "speaker_clip",
                    "speaker_name": speaker.name,
                    "conference": intake.name,
                    "required_role": "editor",
                })),
                ..Default::default()
            },
            TaskRole::Editor,
            TaskType::SpeakerClip(speaker.name.clone()),
            Some(speaker.name.clone()),
        ).await?;
        created.push(clip_task);

        // Speaker Article (1 day after clip) - Writer role
        let article_task = self.create_task(
            project_id,
            board_id,
            TaskDefinition {
                title: format!("Write article: {} at {}", speaker.name, intake.name),
                description: Some(format!(
                    "Create article accompanying {} speaker clip.\n\n\
                    Speaker: {}\n\
                    Title: {}\n\
                    Topic: {}\n\n\
                    Article structure:\n\
                    - Speaker introduction\n\
                    - Key takeaways from talk\n\
                    - Notable quotes\n\
                    - Link to full clip\n\n\
                    Word count: 400-600 words",
                    intake.name,
                    speaker.name,
                    speaker.title.as_deref().unwrap_or("N/A"),
                    speaker.topic.as_deref().unwrap_or("N/A"),
                )),
                priority: Some(Priority::Medium),
                tags: Some(vec![
                    "post-event".into(),
                    "content".into(),
                    "article".into(),
                    "speaker-article".into(),
                    conference_slug.clone(),
                    speaker_slug,
                ]),
                board_id: Some(board_id),
                due_date: Some(end_dt + Duration::days(base_offset + 1)),
                scheduled_start: Some(end_dt + Duration::days(base_offset)),
                scheduled_end: Some(end_dt + Duration::days(base_offset + 1)),
                custom_properties: Some(serde_json::json!({
                    "phase": "post-event",
                    "deliverable_type": "article",
                    "article_type": "speaker_article",
                    "speaker_name": speaker.name,
                    "conference": intake.name,
                    "required_role": "writer",
                })),
                ..Default::default()
            },
            TaskRole::Writer,
            TaskType::SpeakerArticle(speaker.name.clone()),
            Some(speaker.name.clone()),
        ).await?;
        created.push(article_task);

        Ok(created)
    }

    /// Create Entity records from intake speakers so workflow doesn't hallucinate
    async fn create_speaker_entities(
        &self,
        intake: &ConferenceIntake,
        board_id: Uuid,
    ) -> Result<usize> {
        if intake.speakers.is_empty() {
            return Ok(0);
        }

        let mut count = 0;

        for speaker in &intake.speakers {
            // Check if entity already exists
            if let Ok(Some(_existing)) = Entity::find_by_name(&self.pool, &speaker.name).await {
                tracing::debug!(
                    "[CONFERENCE_PIPELINE] Entity already exists for speaker: {}",
                    speaker.name
                );
                continue;
            }

            // Create new entity
            let create_entity = CreateEntity {
                entity_type: EntityType::Speaker,
                canonical_name: speaker.name.clone(),
                slug: None,
                external_ids: None,
                bio: speaker.bio.clone(),
                title: speaker.title.clone(),
                company: speaker.company.clone(),
                photo_url: None,
                social_profiles: None,
            };

            match Entity::create(&self.pool, &create_entity).await {
                Ok(entity) => {
                    // Link entity to this conference board
                    if let Err(e) = EntityAppearance::find_or_create(
                        &self.pool,
                        entity.id,
                        board_id,
                        AppearanceType::Speaker,
                    ).await {
                        tracing::warn!(
                            "[CONFERENCE_PIPELINE] Failed to create entity appearance: {}",
                            e
                        );
                    }

                    tracing::debug!(
                        "[CONFERENCE_PIPELINE] Created entity for speaker: {} ({})",
                        speaker.name,
                        entity.id
                    );
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "[CONFERENCE_PIPELINE] Failed to create entity for speaker {}: {}",
                        speaker.name,
                        e
                    );
                }
            }
        }

        Ok(count)
    }

    /// Create a single task and return tracking info
    async fn create_task(
        &self,
        project_id: Uuid,
        _board_id: Uuid,
        task_def: TaskDefinition,
        role: TaskRole,
        task_type: TaskType,
        speaker_name: Option<String>,
    ) -> Result<CreatedTask> {
        let due_date = task_def.due_date;
        let phase = task_def.custom_properties
            .as_ref()
            .and_then(|p| p.get("phase"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let task = self.task_executor.create_task(project_id, task_def).await?;

        Ok(CreatedTask {
            id: task.id,
            title: task.title,
            due_date,
            phase,
            role,
            task_type,
            speaker_name,
        })
    }

    /// Create dependencies between related tasks
    async fn create_task_dependencies(
        &self,
        project_id: Uuid,
        tasks: &[CreatedTask],
    ) -> Result<usize> {
        let mut count = 0;

        // Find research task
        let research_task = tasks.iter().find(|t| t.task_type == TaskType::Research);

        // Find speakers article
        let speakers_article = tasks.iter().find(|t| t.task_type == TaskType::SpeakersArticle);

        // Research -> Speakers Article (RelatesTo)
        if let (Some(research), Some(article)) = (research_task, speakers_article) {
            self.create_dependency(project_id, research.id, article.id, DependencyType::RelatesTo).await?;
            count += 1;
        }

        // Footage Intake -> Recap Video, Highlight Reels (RelatesTo)
        let footage_intake = tasks.iter().find(|t| t.task_type == TaskType::FootageIntake);
        let recap_video = tasks.iter().find(|t| t.task_type == TaskType::RecapVideo);
        let highlight_reels = tasks.iter().find(|t| t.task_type == TaskType::HighlightReels);

        if let Some(footage) = footage_intake {
            if let Some(recap) = recap_video {
                self.create_dependency(project_id, footage.id, recap.id, DependencyType::RelatesTo).await?;
                count += 1;
            }
            if let Some(highlights) = highlight_reels {
                self.create_dependency(project_id, footage.id, highlights.id, DependencyType::RelatesTo).await?;
                count += 1;
            }
        }

        // For each speaker: Clip -> Article (RelatesTo)
        let speaker_clips: Vec<_> = tasks.iter()
            .filter(|t| matches!(t.task_type, TaskType::SpeakerClip(_)))
            .collect();

        for clip in speaker_clips {
            if let Some(speaker_name) = &clip.speaker_name {
                // Find matching article
                let article = tasks.iter().find(|t| {
                    matches!(&t.task_type, TaskType::SpeakerArticle(name) if name == speaker_name)
                });

                if let Some(article) = article {
                    self.create_dependency(project_id, clip.id, article.id, DependencyType::RelatesTo).await?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Create a single dependency
    async fn create_dependency(
        &self,
        project_id: Uuid,
        source_id: Uuid,
        target_id: Uuid,
        dep_type: DependencyType,
    ) -> Result<TaskDependency> {
        let payload = CreateTaskDependency {
            project_id,
            source_task_id: source_id,
            target_task_id: target_id,
            dependency_type: dep_type,
        };

        TaskDependency::create(&self.pool, &payload)
            .await
            .map_err(NoraError::DatabaseError)
    }
}

/// Convert NaiveDate to DateTime<Utc> at start of day (9 AM)
fn date_to_datetime(date: NaiveDate) -> DateTime<Utc> {
    Utc.from_utc_datetime(&date.and_hms_opt(9, 0, 0).unwrap())
}

/// Keywords that indicate a conference intake request
pub const CONFERENCE_TRIGGER_KEYWORDS: &[&str] = &[
    "conference",
    "event",
    "summit",
    "coming up",
    "next week",
    "this week",
    "iconnection",
    "ethdenver",
    "devcon",
    "consensus",
    "token2049",
];

/// Check if a message appears to be about a conference
pub fn is_conference_related(message: &str) -> bool {
    let lower = message.to_lowercase();
    CONFERENCE_TRIGGER_KEYWORDS.iter().any(|kw| lower.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_to_datetime() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 20).unwrap();
        let dt = date_to_datetime(date);
        assert_eq!(dt.date_naive(), date);
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("iConnection Conference 2026"), "iconnection-conference-2026");
        assert_eq!(slugify("ETH Denver"), "eth-denver");
    }

    #[test]
    fn test_is_conference_related() {
        assert!(is_conference_related("We have iConnection coming up next week"));
        assert!(is_conference_related("There's a conference in Miami"));
        assert!(!is_conference_related("What's the weather like?"));
    }
}
