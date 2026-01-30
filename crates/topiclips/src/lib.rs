//! # TopiClips - AI-generated artistic video clips from topology evolution
//!
//! "Beeple Everydays from Topsi" - Daily artistic interpretations of your
//! knowledge graph evolution through symbolism and surrealism.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
//! │  Topsi Topology │────▶│  Story Extractor │────▶│    Artistic     │
//! │    (Changes)    │     │   (Narrative)    │     │   Interpreter   │
//! └─────────────────┘     └──────────────────┘     └────────┬────────┘
//!                                                           │
//! ┌─────────────────┐     ┌──────────────────┐              │
//! │   ProjectAsset  │◀────│   Cinematics     │◀─────────────┘
//! │    (Output)     │     │    (ComfyUI)     │   (Creative Prompts)
//! └─────────────────┘     └──────────────────┘
//! ```

pub mod artistic_interpreter;
pub mod event_detector;
pub mod scheduler;
pub mod story_extractor;
pub mod symbol_mapper;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use cinematics::{CinematicsConfig, CinematicsService};
use db::models::topiclip::{
    CreateTopiClipCapturedEvent, CreateTopiClipSession, TopiClipCapturedEvent, TopiClipDailySchedule, TopiClipGalleryResponse, TopiClipSession,
    TopiClipSessionStatus, TopiClipTimelineEntry, TopiClipTriggerType,
    UpdateTopiClipSessionStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;
use topsi::TopologyChange;
use tracing::info;
use uuid::Uuid;

pub use artistic_interpreter::ArtisticInterpreter;
pub use event_detector::EventDetector;
pub use story_extractor::{NarrativeStory, StoryExtractor};
pub use symbol_mapper::SymbolMapper;

/// Configuration for TopiClips service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopiClipsConfig {
    /// LLM model to use for artistic interpretation
    pub llm_model: String,
    /// LLM API endpoint
    pub llm_endpoint: String,
    /// LLM API key (from environment)
    pub llm_api_key: Option<String>,
    /// Temperature for LLM generation (higher = more creative)
    pub interpretation_temperature: f64,
    /// Default significance threshold for event-driven triggers
    pub default_significance_threshold: f64,
    /// Cinematics configuration
    pub cinematics_config: CinematicsConfig,
}

impl Default for TopiClipsConfig {
    fn default() -> Self {
        Self {
            llm_model: std::env::var("TOPICLIPS_LLM_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".into()),
            llm_endpoint: std::env::var("TOPICLIPS_LLM_ENDPOINT")
                .unwrap_or_else(|_| "https://api.anthropic.com/v1/messages".into()),
            llm_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            interpretation_temperature: std::env::var("TOPICLIPS_TEMPERATURE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.8),
            default_significance_threshold: std::env::var("TOPICLIPS_SIGNIFICANCE_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.3),
            cinematics_config: CinematicsConfig::default(),
        }
    }
}

/// Main error types for TopiClips
#[derive(Debug, thiserror::Error)]
pub enum TopiClipsError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(Uuid),

    #[error("Story extraction failed: {0}")]
    StoryExtractionError(String),

    #[error("Artistic interpretation failed: {0}")]
    InterpretationError(String),

    #[error("Render failed: {0}")]
    RenderError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("No significant events found")]
    NoSignificantEvents,

    #[error("LLM error: {0}")]
    LLMError(String),
}

/// Trait for the TopiClips generation pipeline
#[async_trait]
pub trait TopiClipGenerator {
    /// Create a new TopiClip session
    async fn create_session(
        &self,
        project_id: Uuid,
        trigger_type: TopiClipTriggerType,
        period_start: Option<String>,
        period_end: Option<String>,
    ) -> Result<TopiClipSession>;

    /// Run the full generation pipeline for a session
    async fn generate(&self, session_id: Uuid) -> Result<TopiClipSession>;

    /// Get gallery view for a project
    async fn get_gallery(&self, project_id: Uuid) -> Result<TopiClipGalleryResponse>;

    /// Get timeline entry with events and assets
    async fn get_timeline_entry(&self, session_id: Uuid) -> Result<TopiClipTimelineEntry>;
}

/// Main TopiClips service
pub struct TopiClipsService {
    pool: SqlitePool,
    config: TopiClipsConfig,
    story_extractor: StoryExtractor,
    symbol_mapper: SymbolMapper,
    artistic_interpreter: ArtisticInterpreter,
    event_detector: EventDetector,
    cinematics: CinematicsService,
}

impl TopiClipsService {
    pub fn new(pool: SqlitePool, config: TopiClipsConfig) -> Self {
        let cinematics = CinematicsService::new(pool.clone(), config.cinematics_config.clone());
        let story_extractor = StoryExtractor::new(pool.clone());
        let symbol_mapper = SymbolMapper::new(pool.clone());
        let artistic_interpreter = ArtisticInterpreter::new(
            config.llm_endpoint.clone(),
            config.llm_api_key.clone(),
            config.llm_model.clone(),
            config.interpretation_temperature,
        );
        let event_detector = EventDetector::new(config.default_significance_threshold);

        Self {
            pool,
            config,
            story_extractor,
            symbol_mapper,
            artistic_interpreter,
            event_detector,
            cinematics,
        }
    }

    /// Create a session with automatic day numbering
    async fn create_session_internal(
        &self,
        project_id: Uuid,
        trigger_type: TopiClipTriggerType,
        period_start: Option<String>,
        period_end: Option<String>,
    ) -> Result<TopiClipSession> {
        // Get next day number
        let day_number = TopiClipSession::get_latest_day_number(&self.pool, project_id).await? + 1;

        // Generate title based on day number and trigger
        let title = match trigger_type {
            TopiClipTriggerType::Daily => format!("Day {} - Daily Reflection", day_number),
            TopiClipTriggerType::Event => format!("Day {} - Significant Moment", day_number),
            TopiClipTriggerType::Manual => format!("Day {} - Manual Creation", day_number),
        };

        let session = TopiClipSession::create(
            &self.pool,
            &CreateTopiClipSession {
                project_id,
                title,
                day_number,
                trigger_type,
                period_start,
                period_end,
            },
            Uuid::new_v4(),
        )
        .await?;

        Ok(session)
    }

    /// Analyze phase: extract story from topology changes
    async fn analyze_phase(&self, session: &TopiClipSession) -> Result<NarrativeStory> {
        // Update status to analyzing
        TopiClipSession::update_status(
            &self.pool,
            session.id,
            &UpdateTopiClipSessionStatus {
                status: TopiClipSessionStatus::Analyzing,
                ..default_status_update(TopiClipSessionStatus::Pending)
            },
        )
        .await?;

        // Extract story from topology changes
        let story = self
            .story_extractor
            .extract_story(
                session.project_id,
                session.period_start.as_deref(),
                session.period_end.as_deref(),
            )
            .await
            .map_err(|e| TopiClipsError::StoryExtractionError(e.to_string()))?;

        // Capture events with their significance scores
        for event in &story.events {
            let significance = self.event_detector.score_event(event);
            let symbol = self.symbol_mapper.map_event(event).await?;

            TopiClipCapturedEvent::create(
                &self.pool,
                &CreateTopiClipCapturedEvent {
                    session_id: session.id,
                    event_type: event.event_type(),
                    event_data: serde_json::to_value(event)?,
                    narrative_role: Some(story.get_narrative_role(event)),
                    significance_score: Some(significance),
                    assigned_symbol: symbol.as_ref().map(|s| s.symbol_name.clone()),
                    symbol_prompt: symbol.as_ref().map(|s| s.prompt_template.clone()),
                    affected_node_ids: event.affected_node_ids(),
                    affected_edge_ids: event.affected_edge_ids(),
                    occurred_at: Utc::now().to_rfc3339(),
                },
                Uuid::new_v4(),
            )
            .await?;
        }

        Ok(story)
    }

    /// Interpret phase: generate artistic prompts via LLM
    async fn interpret_phase(
        &self,
        session: &TopiClipSession,
        story: &NarrativeStory,
    ) -> Result<(String, String, Value)> {
        // Update status to interpreting
        TopiClipSession::update_status(
            &self.pool,
            session.id,
            &UpdateTopiClipSessionStatus {
                status: TopiClipSessionStatus::Interpreting,
                primary_theme: Some(story.primary_theme.clone()),
                emotional_arc: Some(story.emotional_arc.clone()),
                narrative_summary: Some(story.narrative_summary.clone()),
                events_analyzed: Some(story.events.len() as i64),
                significance_score: Some(story.overall_significance),
                ..default_status_update(TopiClipSessionStatus::Pending)
            },
        )
        .await?;

        // Get captured events with their symbols
        let events = TopiClipCapturedEvent::list_by_session(&self.pool, session.id).await?;

        // Generate artistic interpretation
        let interpretation = self
            .artistic_interpreter
            .interpret(story, &events)
            .await
            .map_err(|e| TopiClipsError::InterpretationError(e.to_string()))?;

        Ok((
            interpretation.artistic_prompt,
            interpretation.negative_prompt,
            interpretation.symbol_mapping,
        ))
    }

    /// Render phase: generate single high-quality TopiClip image via Cinematics/ComfyUI
    async fn render_phase(
        &self,
        session: &TopiClipSession,
        artistic_prompt: &str,
        negative_prompt: &str,
        symbol_mapping: Value,
    ) -> Result<TopiClipSession> {
        // Update status to rendering
        TopiClipSession::update_status(
            &self.pool,
            session.id,
            &UpdateTopiClipSessionStatus {
                status: TopiClipSessionStatus::Rendering,
                artistic_prompt: Some(artistic_prompt.to_string()),
                negative_prompt: Some(negative_prompt.to_string()),
                symbol_mapping: Some(symbol_mapping.clone()),
                ..default_status_update(TopiClipSessionStatus::Pending)
            },
        )
        .await?;

        // Generate filename prefix based on session
        let filename_prefix = format!("TopiClips_Day{:03}", session.day_number);

        // Render high-quality video using SDXL + Refiner + Wan2.2 pipeline
        let output_path = self
            .cinematics
            .generate_topiclip_video_hq(artistic_prompt, negative_prompt, &filename_prefix)
            .await
            .map_err(|e| TopiClipsError::RenderError(e.to_string()))?;

        // Store the output path as the asset ID
        let output_asset_ids: Vec<String> = vec![output_path];

        // Update to delivered
        let final_session = TopiClipSession::update_status(
            &self.pool,
            session.id,
            &UpdateTopiClipSessionStatus {
                status: TopiClipSessionStatus::Delivered,
                cinematic_brief_id: None, // Direct render, no brief created
                output_asset_ids: Some(output_asset_ids.clone()),
                llm_notes: Some("TopiClip rendered directly via ComfyUI".into()),
                ..default_status_update(TopiClipSessionStatus::Pending)
            },
        )
        .await?;

        // Update streak if this is a daily clip
        if session.trigger_type == TopiClipTriggerType::Daily {
            if let Some(schedule) =
                TopiClipDailySchedule::find_by_project(&self.pool, session.project_id).await?
            {
                let today = Utc::now().format("%Y-%m-%d").to_string();
                TopiClipDailySchedule::update_streak(&self.pool, schedule.id, &today).await?;
            }
        }

        info!(
            "TopiClip {} rendered {} assets",
            session.id,
            output_asset_ids.len()
        );

        Ok(final_session)
    }

    /// Handle errors during generation
    async fn handle_error(&self, session_id: Uuid, error: &str) -> Result<TopiClipSession> {
        let session = TopiClipSession::update_status(
            &self.pool,
            session_id,
            &UpdateTopiClipSessionStatus {
                status: TopiClipSessionStatus::Failed,
                error_message: Some(error.to_string()),
                ..default_status_update(TopiClipSessionStatus::Pending)
            },
        )
        .await?;
        Ok(session)
    }
}

#[async_trait]
impl TopiClipGenerator for TopiClipsService {
    async fn create_session(
        &self,
        project_id: Uuid,
        trigger_type: TopiClipTriggerType,
        period_start: Option<String>,
        period_end: Option<String>,
    ) -> Result<TopiClipSession> {
        self.create_session_internal(project_id, trigger_type, period_start, period_end)
            .await
    }

    async fn generate(&self, session_id: Uuid) -> Result<TopiClipSession> {
        let session = TopiClipSession::find_by_id(&self.pool, session_id)
            .await?
            .ok_or(TopiClipsError::SessionNotFound(session_id))?;

        // Check if session already has artistic prompt (e.g., Genesis/Day 0 sessions)
        // These sessions skip analysis and interpretation phases, going directly to rendering
        if let (Some(ref artistic_prompt), Some(ref negative_prompt)) =
            (&session.artistic_prompt, &session.negative_prompt)
        {
            // Skip to render phase for pre-populated sessions
            let symbol_mapping = session
                .symbol_mapping
                .as_ref()
                .map(|j| j.0.clone())
                .unwrap_or(serde_json::json!({"genesis": {"symbol": "Cosmic Seed"}}));

            return match self
                .render_phase(&session, artistic_prompt, negative_prompt, symbol_mapping)
                .await
            {
                Ok(final_session) => Ok(final_session),
                Err(e) => {
                    self.handle_error(session_id, &format!("Render failed: {}", e))
                        .await
                }
            };
        }

        // Phase 1: Analyze topology changes and extract story
        let story = match self.analyze_phase(&session).await {
            Ok(story) => story,
            Err(e) => {
                return self
                    .handle_error(session_id, &format!("Analysis failed: {}", e))
                    .await;
            }
        };

        // Check if we have enough significance to proceed
        if story.events.is_empty() {
            return self
                .handle_error(session_id, "No significant events found")
                .await;
        }

        // Phase 2: Generate artistic interpretation
        let (artistic_prompt, negative_prompt, symbol_mapping) =
            match self.interpret_phase(&session, &story).await {
                Ok(result) => result,
                Err(e) => {
                    return self
                        .handle_error(session_id, &format!("Interpretation failed: {}", e))
                        .await;
                }
            };

        // Phase 3: Render video
        match self
            .render_phase(&session, &artistic_prompt, &negative_prompt, symbol_mapping)
            .await
        {
            Ok(final_session) => Ok(final_session),
            Err(e) => {
                self.handle_error(session_id, &format!("Render failed: {}", e))
                    .await
            }
        }
    }

    async fn get_gallery(&self, project_id: Uuid) -> Result<TopiClipGalleryResponse> {
        let sessions = TopiClipSession::list_by_project(&self.pool, project_id).await?;
        let schedule = TopiClipDailySchedule::find_by_project(&self.pool, project_id).await?;

        let (current_streak, longest_streak, total_clips) = schedule
            .as_ref()
            .map(|s| {
                (
                    s.current_streak,
                    s.longest_streak,
                    s.total_clips_generated,
                )
            })
            .unwrap_or((0, 0, sessions.len() as i64));

        Ok(TopiClipGalleryResponse {
            sessions,
            schedule,
            current_streak,
            longest_streak,
            total_clips,
        })
    }

    async fn get_timeline_entry(&self, session_id: Uuid) -> Result<TopiClipTimelineEntry> {
        let session = TopiClipSession::find_by_id(&self.pool, session_id)
            .await?
            .ok_or(TopiClipsError::SessionNotFound(session_id))?;

        let events = TopiClipCapturedEvent::list_by_session(&self.pool, session_id).await?;

        // Extract asset URLs from output_asset_ids
        let asset_urls: Vec<String> = session
            .output_asset_ids
            .as_ref()
            .and_then(|json| json.0.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| format!("/api/assets/{}", s)))
                    .collect()
            })
            .unwrap_or_default();

        Ok(TopiClipTimelineEntry {
            session,
            events,
            asset_urls,
        })
    }
}

/// Extension trait for TopologyChange to get event type and affected IDs
trait TopologyChangeExt {
    fn event_type(&self) -> String;
    fn affected_node_ids(&self) -> Option<Vec<String>>;
    fn affected_edge_ids(&self) -> Option<Vec<String>>;
}

impl TopologyChangeExt for TopologyChange {
    fn event_type(&self) -> String {
        match self {
            TopologyChange::NodeAdded { .. } => "NodeAdded".to_string(),
            TopologyChange::NodeRemoved { .. } => "NodeRemoved".to_string(),
            TopologyChange::NodeStatusChanged { .. } => "NodeStatusChanged".to_string(),
            TopologyChange::EdgeAdded { .. } => "EdgeAdded".to_string(),
            TopologyChange::EdgeRemoved { .. } => "EdgeRemoved".to_string(),
            TopologyChange::EdgeStatusChanged { .. } => "EdgeStatusChanged".to_string(),
            TopologyChange::ClusterFormed { .. } => "ClusterFormed".to_string(),
            TopologyChange::ClusterDissolved { .. } => "ClusterDissolved".to_string(),
            TopologyChange::RouteCreated { .. } => "RouteCreated".to_string(),
            TopologyChange::RouteCompleted { .. } => "RouteCompleted".to_string(),
            TopologyChange::RouteFailed { .. } => "RouteFailed".to_string(),
        }
    }

    fn affected_node_ids(&self) -> Option<Vec<String>> {
        match self {
            TopologyChange::NodeAdded { node_id, .. } => Some(vec![node_id.to_string()]),
            TopologyChange::NodeRemoved { node_id } => Some(vec![node_id.to_string()]),
            TopologyChange::NodeStatusChanged { node_id, .. } => Some(vec![node_id.to_string()]),
            TopologyChange::EdgeAdded { from, to, .. } => {
                Some(vec![from.to_string(), to.to_string()])
            }
            TopologyChange::ClusterFormed { cluster_id, .. } => Some(vec![cluster_id.to_string()]),
            TopologyChange::ClusterDissolved { cluster_id } => Some(vec![cluster_id.to_string()]),
            _ => None,
        }
    }

    fn affected_edge_ids(&self) -> Option<Vec<String>> {
        match self {
            TopologyChange::EdgeAdded { edge_id, .. } => Some(vec![edge_id.to_string()]),
            TopologyChange::EdgeRemoved { edge_id } => Some(vec![edge_id.to_string()]),
            TopologyChange::EdgeStatusChanged { edge_id, .. } => Some(vec![edge_id.to_string()]),
            _ => None,
        }
    }
}

/// Create a default UpdateTopiClipSessionStatus
fn default_status_update(status: TopiClipSessionStatus) -> UpdateTopiClipSessionStatus {
    UpdateTopiClipSessionStatus {
        status,
        primary_theme: None,
        emotional_arc: None,
        narrative_summary: None,
        artistic_prompt: None,
        negative_prompt: None,
        symbol_mapping: None,
        cinematic_brief_id: None,
        output_asset_ids: None,
        llm_notes: None,
        error_message: None,
        events_analyzed: None,
        significance_score: None,
    }
}
