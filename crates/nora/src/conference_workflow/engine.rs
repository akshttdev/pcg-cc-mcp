//! Conference Workflow Engine
//!
//! Main orchestrator for automated conference workflow pipelines.
//! Coordinates research stages, QA gates, content/graphics creation, and social scheduling.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use ts_rs::TS;
use uuid::Uuid;

use db::models::{
    conference_workflow::{ConferenceWorkflow, CreateConferenceWorkflow, WorkflowStatus},
    entity::Entity,
    entity_appearance::EntityAppearance,
    project_board::ProjectBoard,
    side_event::SideEvent,
    task::{Task, CreateTask, Priority, TaskStatus},
    workflow_qa_run::WorkflowQARun,
    workflow_artifact::{WorkflowArtifact, CreateWorkflowArtifact, ArtifactType},
};
use cinematics::CinematicsService;

use crate::{
    conference::{ConferenceIntake, Speaker},
    execution::ExecutionEngine,
    NoraError, Result,
};

use super::{
    qa::{QualityAnalyst, QADecision},
    parallel::ParallelOrchestrator,
    social::SocialPostCreator,
    stages::{StageExecutor, ResearchStageResult},
};

/// Configuration for the workflow engine
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfig {
    /// Maximum retries per stage
    pub max_stage_retries: u32,
    /// Parallelism limit for speaker/brand research
    pub parallelism_limit: usize,
    /// QA score threshold for auto-approval
    pub qa_approval_threshold: f64,
    /// Research freshness duration in days
    pub research_freshness_days: i64,
    /// Enable parallel content + graphics creation
    pub enable_parallel_creation: bool,
    /// Auto-schedule social posts
    pub auto_schedule_posts: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            max_stage_retries: 3,
            parallelism_limit: 5,
            qa_approval_threshold: 0.8,
            research_freshness_days: 30,
            enable_parallel_creation: true,
            auto_schedule_posts: true,
        }
    }
}

/// Result of workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowResult {
    pub workflow_id: Uuid,
    pub status: WorkflowStatus,
    pub stages_completed: Vec<String>,
    pub entities_created: usize,
    pub side_events_discovered: usize,
    pub social_posts_scheduled: usize,
    pub final_qa_score: Option<f64>,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// Research flow results aggregated from all stages
#[derive(Debug, Clone)]
pub struct ResearchFlowResult {
    pub conference_intel: Option<ResearchStageResult>,
    pub speaker_results: Vec<ResearchStageResult>,
    pub brand_results: Vec<ResearchStageResult>,
    pub production_team: Option<ResearchStageResult>,
    pub competitive_intel: Option<ResearchStageResult>,
    pub side_events: Vec<SideEvent>,
    pub entities: Vec<Entity>,
}

/// Main Conference Workflow Engine
pub struct ConferenceWorkflowEngine {
    pool: SqlitePool,
    execution_engine: Arc<ExecutionEngine>,
    stage_executor: StageExecutor,
    qa: QualityAnalyst,
    parallel: ParallelOrchestrator,
    social: SocialPostCreator,
    config: WorkflowConfig,
}

impl ConferenceWorkflowEngine {
    /// Create a new workflow engine
    pub fn new(
        pool: SqlitePool,
        execution_engine: Arc<ExecutionEngine>,
        cinematics: Option<Arc<CinematicsService>>,
    ) -> Self {
        let config = WorkflowConfig::default();
        Self {
            pool: pool.clone(),
            execution_engine: execution_engine.clone(),
            stage_executor: StageExecutor::new(pool.clone(), execution_engine.clone()),
            qa: QualityAnalyst::new(pool.clone()),
            parallel: ParallelOrchestrator::new(pool.clone(), execution_engine.clone(), cinematics),
            social: SocialPostCreator::new(pool.clone()),
            config,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        pool: SqlitePool,
        execution_engine: Arc<ExecutionEngine>,
        cinematics: Option<Arc<CinematicsService>>,
        config: WorkflowConfig,
    ) -> Self {
        Self {
            pool: pool.clone(),
            execution_engine: execution_engine.clone(),
            stage_executor: StageExecutor::new(pool.clone(), execution_engine.clone()),
            qa: QualityAnalyst::new(pool.clone()),
            parallel: ParallelOrchestrator::new(pool.clone(), execution_engine.clone(), cinematics),
            social: SocialPostCreator::new(pool.clone()),
            config,
        }
    }

    /// Initialize a workflow from conference intake
    pub async fn initialize(
        &self,
        intake: &ConferenceIntake,
        board_id: Uuid,
    ) -> Result<Uuid> {
        tracing::info!(
            "[WORKFLOW_ENGINE] Initializing workflow for: {}",
            intake.name
        );

        // Create workflow record
        let create = CreateConferenceWorkflow {
            conference_board_id: board_id,
            conference_name: intake.name.clone(),
            start_date: intake.start_date.to_string(),
            end_date: intake.end_date.to_string(),
            location: Some(intake.location.clone()),
            timezone: None,
            website: intake.website.clone(),
            target_platform_ids: None,
            config_overrides: None,
        };

        let workflow = ConferenceWorkflow::create(&self.pool, &create)
            .await
            .map_err(NoraError::DatabaseError)?;

        tracing::info!(
            "[WORKFLOW_ENGINE] Created workflow: {} ({})",
            workflow.conference_name,
            workflow.id
        );

        Ok(workflow.id)
    }

    /// Run the full automated workflow
    pub async fn run_workflow(&self, workflow_id: Uuid) -> Result<WorkflowResult> {
        let start_time = std::time::Instant::now();
        let mut errors = Vec::new();
        let mut stages_completed = Vec::new();

        tracing::info!("[WORKFLOW_ENGINE] Starting workflow: {}", workflow_id);

        // Load workflow
        let workflow = ConferenceWorkflow::find_by_id(&self.pool, workflow_id)
            .await
            .map_err(NoraError::DatabaseError)?
            .ok_or_else(|| NoraError::ConfigError(format!("Workflow {} not found", workflow_id)))?;

        // Update status to researching
        ConferenceWorkflow::update_status(&self.pool, workflow_id, WorkflowStatus::Researching)
            .await
            .map_err(NoraError::DatabaseError)?;

        // 1. Run research flow (6 stages with QA gates)
        let research_result = match self.run_research_flow(&workflow).await {
            Ok(result) => {
                stages_completed.extend(vec![
                    "conference_intel".to_string(),
                    "speaker_research".to_string(),
                    "brand_research".to_string(),
                    "production_team".to_string(),
                    "competitive_intel".to_string(),
                    "side_events".to_string(),
                ]);
                result
            }
            Err(e) => {
                errors.push(format!("Research flow failed: {}", e));
                ConferenceWorkflow::record_error(&self.pool, workflow_id, &e.to_string())
                    .await
                    .ok();
                ConferenceWorkflow::update_status(&self.pool, workflow_id, WorkflowStatus::Failed)
                    .await
                    .ok();

                return Ok(WorkflowResult {
                    workflow_id,
                    status: WorkflowStatus::Failed,
                    stages_completed,
                    entities_created: 0,
                    side_events_discovered: 0,
                    social_posts_scheduled: 0,
                    final_qa_score: None,
                    duration_ms: start_time.elapsed().as_millis() as u64,
                    errors,
                });
            }
        };

        // Update research complete status
        ConferenceWorkflow::update_status(&self.pool, workflow_id, WorkflowStatus::ResearchComplete)
            .await
            .map_err(NoraError::DatabaseError)?;

        // Update counts
        ConferenceWorkflow::update_counts(
            &self.pool,
            workflow_id,
            Some(research_result.entities.iter().filter(|e| e.entity_type == db::models::entity::EntityType::Speaker).count() as i64),
            Some(research_result.entities.iter().filter(|e| e.entity_type == db::models::entity::EntityType::Sponsor).count() as i64),
            Some(research_result.side_events.len() as i64),
        )
        .await
        .map_err(NoraError::DatabaseError)?;

        // 2. Parallel content + graphics creation
        if self.config.enable_parallel_creation {
            ConferenceWorkflow::update_status(&self.pool, workflow_id, WorkflowStatus::ContentCreation)
                .await
                .map_err(NoraError::DatabaseError)?;

            match self.parallel.run_parallel_creation(&workflow, &research_result).await {
                Ok((content_result, graphics_result)) => {
                    stages_completed.push("content_creation".to_string());
                    stages_completed.push("graphics_creation".to_string());

                    // Persist artifacts to database AND create tasks on the board
                    let artifacts_saved = self.persist_artifacts(
                        &workflow,
                        &content_result,
                        &graphics_result,
                    ).await;

                    match artifacts_saved {
                        Ok(count) => {
                            tracing::info!("[WORKFLOW_ENGINE] Persisted {} artifacts and created tasks", count);
                        }
                        Err(e) => {
                            errors.push(format!("Artifact persistence failed: {}", e));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("Parallel creation failed: {}", e));
                }
            }
        }

        // 3. Create and schedule social posts
        let posts_scheduled = if self.config.auto_schedule_posts {
            ConferenceWorkflow::update_status(&self.pool, workflow_id, WorkflowStatus::Scheduling)
                .await
                .map_err(NoraError::DatabaseError)?;

            match self.social.create_posts_for_workflow(&workflow, &research_result).await {
                Ok(posts) => {
                    stages_completed.push("social_scheduling".to_string());
                    ConferenceWorkflow::increment_posts_scheduled(&self.pool, workflow_id, posts.len() as i64)
                        .await
                        .ok();
                    posts.len()
                }
                Err(e) => {
                    errors.push(format!("Social scheduling failed: {}", e));
                    0
                }
            }
        } else {
            0
        };

        // 4. Final QA and completion
        let final_qa_score = match self.qa.evaluate_workflow(&workflow).await {
            Ok(qa_result) => {
                ConferenceWorkflow::update_qa_result(&self.pool, workflow_id, qa_result.overall_score, qa_result.id)
                    .await
                    .ok();
                Some(qa_result.overall_score)
            }
            Err(e) => {
                errors.push(format!("Final QA failed: {}", e));
                None
            }
        };

        // Mark completed
        let final_status = if errors.is_empty() {
            ConferenceWorkflow::mark_completed(&self.pool, workflow_id)
                .await
                .map_err(NoraError::DatabaseError)?;
            WorkflowStatus::Completed
        } else {
            WorkflowStatus::ResearchComplete // Partial success
        };

        tracing::info!(
            "[WORKFLOW_ENGINE] Workflow {} completed with status {:?}",
            workflow_id,
            final_status
        );

        Ok(WorkflowResult {
            workflow_id,
            status: final_status,
            stages_completed,
            entities_created: research_result.entities.len(),
            side_events_discovered: research_result.side_events.len(),
            social_posts_scheduled: posts_scheduled,
            final_qa_score,
            duration_ms: start_time.elapsed().as_millis() as u64,
            errors,
        })
    }

    /// Run the research flow with 6 stages and QA gates
    async fn run_research_flow(&self, workflow: &ConferenceWorkflow) -> Result<ResearchFlowResult> {
        // Pre-load existing entities for this board (may have been created from intake)
        let existing_entities = Entity::find_by_board(&self.pool, workflow.conference_board_id)
            .await
            .unwrap_or_default();

        if !existing_entities.is_empty() {
            tracing::info!(
                "[WORKFLOW_ENGINE] Pre-loaded {} existing entities for board {}",
                existing_entities.len(),
                workflow.conference_board_id
            );
        }

        let mut result = ResearchFlowResult {
            conference_intel: None,
            speaker_results: Vec::new(),
            brand_results: Vec::new(),
            production_team: None,
            competitive_intel: None,
            side_events: Vec::new(),
            entities: existing_entities,
        };

        // Stage 1: Conference Intel
        ConferenceWorkflow::update_stage(&self.pool, workflow.id, "conference_intel")
            .await
            .map_err(NoraError::DatabaseError)?;

        let intel_result = self.stage_executor.execute_conference_intel(workflow).await?;
        result.conference_intel = Some(intel_result.clone());

        // QA gate for conference intel
        let qa_result = self.qa.evaluate_stage(workflow.id, "Conference Intel", &intel_result).await?;
        if qa_result.decision == QADecision::Escalate {
            return Err(NoraError::ExecutionError(format!(
                "Conference intel QA escalated: {}",
                qa_result.escalation_reason.unwrap_or_default()
            )));
        }

        // Stage 2: Speaker Research (parallel)
        ConferenceWorkflow::update_stage(&self.pool, workflow.id, "speaker_research")
            .await
            .map_err(NoraError::DatabaseError)?;

        let speaker_results = self.stage_executor.execute_speaker_research(
            workflow,
            &intel_result,
            self.config.parallelism_limit,
        ).await?;

        for sr in &speaker_results {
            if let Some(entity) = &sr.entity {
                result.entities.push(entity.clone());
            }
        }
        result.speaker_results = speaker_results;

        // Stage 3: Brand Research (parallel)
        ConferenceWorkflow::update_stage(&self.pool, workflow.id, "brand_research")
            .await
            .map_err(NoraError::DatabaseError)?;

        let brand_results = self.stage_executor.execute_brand_research(
            workflow,
            &intel_result,
            self.config.parallelism_limit,
        ).await?;

        for br in &brand_results {
            if let Some(entity) = &br.entity {
                result.entities.push(entity.clone());
            }
        }
        result.brand_results = brand_results;

        // Stage 4: Production Team
        ConferenceWorkflow::update_stage(&self.pool, workflow.id, "production_team")
            .await
            .map_err(NoraError::DatabaseError)?;

        let prod_result = self.stage_executor.execute_production_team(workflow).await?;
        result.production_team = Some(prod_result);

        // Stage 5: Competitive Intel
        ConferenceWorkflow::update_stage(&self.pool, workflow.id, "competitive_intel")
            .await
            .map_err(NoraError::DatabaseError)?;

        let comp_result = self.stage_executor.execute_competitive_intel(workflow).await?;
        result.competitive_intel = Some(comp_result);

        // Stage 6: Side Events (parallel across platforms)
        ConferenceWorkflow::update_stage(&self.pool, workflow.id, "side_events")
            .await
            .map_err(NoraError::DatabaseError)?;

        let side_events = self.stage_executor.execute_side_events(workflow).await?;
        result.side_events = side_events;

        Ok(result)
    }

    /// Get workflow status
    pub async fn get_status(&self, workflow_id: Uuid) -> Result<Option<ConferenceWorkflow>> {
        ConferenceWorkflow::find_by_id(&self.pool, workflow_id)
            .await
            .map_err(NoraError::DatabaseError)
    }

    /// Pause a running workflow
    pub async fn pause_workflow(&self, workflow_id: Uuid) -> Result<()> {
        ConferenceWorkflow::update_status(&self.pool, workflow_id, WorkflowStatus::Paused)
            .await
            .map_err(NoraError::DatabaseError)
    }

    /// Resume a paused workflow
    pub async fn resume_workflow(&self, workflow_id: Uuid) -> Result<WorkflowResult> {
        let workflow = ConferenceWorkflow::find_by_id(&self.pool, workflow_id)
            .await
            .map_err(NoraError::DatabaseError)?
            .ok_or_else(|| NoraError::ConfigError(format!("Workflow {} not found", workflow_id)))?;

        if workflow.status != WorkflowStatus::Paused {
            return Err(NoraError::ConfigError(format!(
                "Workflow {} is not paused (status: {:?})",
                workflow_id, workflow.status
            )));
        }

        // Resume by running the workflow again
        self.run_workflow(workflow_id).await
    }

    /// Persist content and graphics artifacts to the database AND create tasks on the board
    async fn persist_artifacts(
        &self,
        workflow: &ConferenceWorkflow,
        content: &super::parallel::ContentResult,
        graphics: &super::parallel::GraphicsResult,
    ) -> Result<usize> {
        let mut count = 0;
        let workflow_id = workflow.id;

        // Get board info to create tasks
        let board = ProjectBoard::find_by_id(&self.pool, workflow.conference_board_id)
            .await
            .map_err(NoraError::DatabaseError)?
            .ok_or_else(|| NoraError::ConfigError(format!("Board {} not found", workflow.conference_board_id)))?;

        // Parse conference start date for scheduling
        let conference_start = chrono::NaiveDate::parse_from_str(&workflow.start_date, "%Y-%m-%d")
            .ok()
            .map(|d| d.and_hms_opt(10, 0, 0).unwrap())
            .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));

        // Calculate review deadline (1 day before conference)
        let review_due = conference_start.map(|dt| dt - chrono::Duration::days(1));
        // Calculate publish deadline (2 days before conference)
        let publish_due = conference_start.map(|dt| dt - chrono::Duration::days(2));

        // Persist articles and create review tasks
        for article in &content.articles {
            let metadata = serde_json::json!({
                "hashtags": article.hashtags,
                "agent_id": article.agent_id,
                "social_caption": article.social_caption,
            });

            // Create artifact record
            let artifact = WorkflowArtifact::create(
                &self.pool,
                CreateWorkflowArtifact {
                    workflow_id,
                    artifact_type: ArtifactType::Article,
                    title: article.title.clone(),
                    content: Some(article.body.clone()),
                    file_url: None,
                    metadata: Some(metadata.clone()),
                },
            )
            .await;

            match artifact {
                Ok(artifact) => {
                    count += 1;

                    // Create a review task on the board for this article
                    let task_title = format!("Review: {}", article.title);
                    let task_description = format!(
                        "Review and approve the generated {} article for {}.\n\n**Preview:**\n{}\n\n---\n*Artifact ID: {}*",
                        article.article_type,
                        workflow.conference_name,
                        article.body.chars().take(500).collect::<String>(),
                        artifact.id
                    );

                    let task_custom_props = serde_json::json!({
                        "artifact_id": artifact.id.to_string(),
                        "artifact_type": "article",
                        "article_type": article.article_type,
                        "workflow_id": workflow_id.to_string(),
                        "conference_name": workflow.conference_name,
                    });

                    match Task::create(
                        &self.pool,
                        &CreateTask {
                            project_id: board.project_id,
                            pod_id: None,
                            board_id: Some(board.id),
                            title: task_title,
                            description: Some(task_description),
                            parent_task_attempt: None,
                            image_ids: None,
                            priority: Some(Priority::High),
                            assignee_id: None,
                            assigned_agent: Some("muse-creative".to_string()),
                            agent_id: None,
                            assigned_mcps: None,
                            created_by: "nora-workflow".to_string(),
                            requires_approval: Some(true),
                            parent_task_id: None,
                            tags: Some(vec!["article".to_string(), "review".to_string(), workflow.conference_name.clone()]),
                            due_date: review_due,
                            custom_properties: Some(task_custom_props),
                            scheduled_start: publish_due,
                            scheduled_end: review_due,
                        },
                        Uuid::new_v4(),
                    )
                    .await
                    {
                        Ok(task) => {
                            tracing::info!(
                                "[WORKFLOW_ENGINE] Created review task {} for article: {}",
                                task.id,
                                article.title
                            );
                        }
                        Err(e) => {
                            tracing::warn!("[WORKFLOW_ENGINE] Failed to create task for article: {}", e);
                        }
                    }
                }
                Err(e) => tracing::warn!("[WORKFLOW_ENGINE] Failed to persist article: {}", e),
            }
        }

        // Persist thumbnails and create publish tasks
        for (article_type, thumbnail) in &graphics.thumbnails {
            let metadata = serde_json::json!({
                "article_type": article_type,
                "width": thumbnail.width,
                "height": thumbnail.height,
                "format": thumbnail.format,
            });

            let artifact = WorkflowArtifact::create(
                &self.pool,
                CreateWorkflowArtifact {
                    workflow_id,
                    artifact_type: ArtifactType::Thumbnail,
                    title: format!("{} Thumbnail", article_type),
                    content: None,
                    file_url: Some(thumbnail.url.clone()),
                    metadata: Some(metadata),
                },
            )
            .await;

            match artifact {
                Ok(artifact) => {
                    count += 1;

                    // Create publish task for thumbnail
                    let task_title = format!("Publish: {} Thumbnail", article_type);
                    let task_description = format!(
                        "Publish the {} thumbnail for {}.\n\n**Asset URL:** {}\n\n---\n*Artifact ID: {}*",
                        article_type,
                        workflow.conference_name,
                        thumbnail.url,
                        artifact.id
                    );

                    let task_custom_props = serde_json::json!({
                        "artifact_id": artifact.id.to_string(),
                        "artifact_type": "thumbnail",
                        "article_type": article_type,
                        "asset_url": thumbnail.url,
                        "workflow_id": workflow_id.to_string(),
                    });

                    match Task::create(
                        &self.pool,
                        &CreateTask {
                            project_id: board.project_id,
                            pod_id: None,
                            board_id: Some(board.id),
                            title: task_title,
                            description: Some(task_description),
                            parent_task_attempt: None,
                            image_ids: None,
                            priority: Some(Priority::Medium),
                            assignee_id: None,
                            assigned_agent: Some("graphics-coordinator".to_string()),
                            agent_id: None,
                            assigned_mcps: None,
                            created_by: "nora-workflow".to_string(),
                            requires_approval: Some(false),
                            parent_task_id: None,
                            tags: Some(vec!["thumbnail".to_string(), "graphics".to_string(), workflow.conference_name.clone()]),
                            due_date: publish_due,
                            custom_properties: Some(task_custom_props),
                            scheduled_start: publish_due,
                            scheduled_end: None,
                        },
                        Uuid::new_v4(),
                    )
                    .await
                    {
                        Ok(task) => {
                            tracing::info!(
                                "[WORKFLOW_ENGINE] Created publish task {} for thumbnail: {}",
                                task.id,
                                article_type
                            );
                        }
                        Err(e) => {
                            tracing::warn!("[WORKFLOW_ENGINE] Failed to create task for thumbnail: {}", e);
                        }
                    }
                }
                Err(e) => tracing::warn!("[WORKFLOW_ENGINE] Failed to persist thumbnail: {}", e),
            }
        }

        // Persist social graphics
        for graphic in &graphics.social_graphics {
            let metadata = serde_json::json!({
                "platform": graphic.platform,
                "aspect_ratio": graphic.aspect_ratio,
            });

            match WorkflowArtifact::create(
                &self.pool,
                CreateWorkflowArtifact {
                    workflow_id,
                    artifact_type: ArtifactType::SocialGraphic,
                    title: format!("{} Social Graphic", graphic.platform),
                    content: None,
                    file_url: Some(graphic.url.clone()),
                    metadata: Some(metadata),
                },
            )
            .await
            {
                Ok(_) => count += 1,
                Err(e) => tracing::warn!("[WORKFLOW_ENGINE] Failed to persist social graphic: {}", e),
            }
        }

        // Persist social captions and create post tasks
        for (idx, caption) in content.social_captions.iter().enumerate() {
            let artifact = WorkflowArtifact::create(
                &self.pool,
                CreateWorkflowArtifact {
                    workflow_id,
                    artifact_type: ArtifactType::SocialPost,
                    title: format!("Social Caption {}", idx + 1),
                    content: Some(caption.clone()),
                    file_url: None,
                    metadata: None,
                },
            )
            .await;

            match artifact {
                Ok(artifact) => {
                    count += 1;

                    // Create social post task
                    let caption_preview: String = caption.chars().take(100).collect();
                    let task_title = format!("Post: {}...", caption_preview);
                    let task_description = format!(
                        "Schedule and publish social post for {}.\n\n**Caption:**\n{}\n\n---\n*Artifact ID: {}*",
                        workflow.conference_name,
                        caption,
                        artifact.id
                    );

                    let task_custom_props = serde_json::json!({
                        "artifact_id": artifact.id.to_string(),
                        "artifact_type": "social_post",
                        "caption": caption,
                        "workflow_id": workflow_id.to_string(),
                    });

                    match Task::create(
                        &self.pool,
                        &CreateTask {
                            project_id: board.project_id,
                            pod_id: None,
                            board_id: Some(board.id),
                            title: task_title,
                            description: Some(task_description),
                            parent_task_attempt: None,
                            image_ids: None,
                            priority: Some(Priority::Medium),
                            assignee_id: None,
                            assigned_agent: Some("social-manager".to_string()),
                            agent_id: None,
                            assigned_mcps: None,
                            created_by: "nora-workflow".to_string(),
                            requires_approval: Some(false),
                            parent_task_id: None,
                            tags: Some(vec!["social".to_string(), "post".to_string(), workflow.conference_name.clone()]),
                            due_date: publish_due,
                            custom_properties: Some(task_custom_props),
                            scheduled_start: publish_due,
                            scheduled_end: None,
                        },
                        Uuid::new_v4(),
                    )
                    .await
                    {
                        Ok(task) => {
                            tracing::info!(
                                "[WORKFLOW_ENGINE] Created social post task {} for caption {}",
                                task.id,
                                idx + 1
                            );
                        }
                        Err(e) => {
                            tracing::warn!("[WORKFLOW_ENGINE] Failed to create task for social caption: {}", e);
                        }
                    }
                }
                Err(e) => tracing::warn!("[WORKFLOW_ENGINE] Failed to persist social caption: {}", e),
            }
        }

        tracing::info!(
            "[WORKFLOW_ENGINE] Persisted {} artifacts and created tasks for workflow {}",
            count,
            workflow_id
        );

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_config_defaults() {
        let config = WorkflowConfig::default();
        assert_eq!(config.max_stage_retries, 3);
        assert_eq!(config.parallelism_limit, 5);
        assert!(config.enable_parallel_creation);
        assert!(config.auto_schedule_posts);
    }
}
