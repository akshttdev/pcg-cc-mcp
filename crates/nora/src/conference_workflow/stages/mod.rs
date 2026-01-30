//! Research Stages for Conference Workflow
//!
//! Six sequential stages that research different aspects of a conference:
//! 1. Conference Intel - Event understanding
//! 2. Speaker Research - Parallel speaker profiles
//! 3. Brand Research - Parallel sponsor/brand profiles
//! 4. Production Team - Production company research
//! 5. Competitive Intel - Competitor analysis
//! 6. Side Events - Lu.ma/Eventbrite/Partiful discovery

pub mod conference_intel;
pub mod speaker_research;
pub mod brand_research;
pub mod production_team;
pub mod competitive_intel;
pub mod side_events;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use ts_rs::TS;
use uuid::Uuid;

use db::models::{
    conference_workflow::ConferenceWorkflow,
    entity::Entity,
    side_event::SideEvent,
};

use crate::{
    execution::ExecutionEngine,
    NoraError, Result,
};

pub use conference_intel::ConferenceIntelStage;
pub use speaker_research::SpeakerResearchStage;
pub use brand_research::BrandResearchStage;
pub use production_team::ProductionTeamStage;
pub use competitive_intel::CompetitiveIntelStage;
pub use side_events::SideEventsStage;

/// Research stage names
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ResearchStage {
    ConferenceIntel,
    SpeakerResearch,
    BrandResearch,
    ProductionTeam,
    CompetitiveIntel,
    SideEvents,
}

impl ResearchStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResearchStage::ConferenceIntel => "conference_intel",
            ResearchStage::SpeakerResearch => "speaker_research",
            ResearchStage::BrandResearch => "brand_research",
            ResearchStage::ProductionTeam => "production_team",
            ResearchStage::CompetitiveIntel => "competitive_intel",
            ResearchStage::SideEvents => "side_events",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ResearchStage::ConferenceIntel => "Conference Intel",
            ResearchStage::SpeakerResearch => "Speaker Research",
            ResearchStage::BrandResearch => "Brand Research",
            ResearchStage::ProductionTeam => "Production Team",
            ResearchStage::CompetitiveIntel => "Competitive Intel",
            ResearchStage::SideEvents => "Side Events",
        }
    }

    pub fn order(&self) -> u32 {
        match self {
            ResearchStage::ConferenceIntel => 1,
            ResearchStage::SpeakerResearch => 2,
            ResearchStage::BrandResearch => 3,
            ResearchStage::ProductionTeam => 4,
            ResearchStage::CompetitiveIntel => 5,
            ResearchStage::SideEvents => 6,
        }
    }
}

/// Result from a research stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchStageResult {
    pub stage: ResearchStage,
    pub success: bool,
    pub entity: Option<Entity>,
    pub artifact_id: Option<Uuid>,
    pub summary: String,
    pub data: serde_json::Value,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub execution_time_ms: u64,
}

impl ResearchStageResult {
    pub fn new(stage: ResearchStage, started_at: DateTime<Utc>) -> Self {
        Self {
            stage,
            success: false,
            entity: None,
            artifact_id: None,
            summary: String::new(),
            data: serde_json::Value::Null,
            started_at,
            completed_at: Utc::now(),
            execution_time_ms: 0,
        }
    }

    pub fn complete(mut self, summary: String, data: serde_json::Value) -> Self {
        self.success = true;
        self.summary = summary;
        self.data = data;
        self.completed_at = Utc::now();
        self.execution_time_ms = (self.completed_at - self.started_at).num_milliseconds() as u64;
        self
    }

    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.entity = Some(entity);
        self
    }

    pub fn with_artifact(mut self, artifact_id: Uuid) -> Self {
        self.artifact_id = Some(artifact_id);
        self
    }

    pub fn fail(mut self, error: &str) -> Self {
        self.success = false;
        self.summary = error.to_string();
        self.completed_at = Utc::now();
        self.execution_time_ms = (self.completed_at - self.started_at).num_milliseconds() as u64;
        self
    }
}

/// Main stage executor that coordinates all research stages
pub struct StageExecutor {
    pool: SqlitePool,
    execution_engine: Arc<ExecutionEngine>,
    conference_intel: ConferenceIntelStage,
    speaker_research: SpeakerResearchStage,
    brand_research: BrandResearchStage,
    production_team: ProductionTeamStage,
    competitive_intel: CompetitiveIntelStage,
    side_events: SideEventsStage,
}

impl StageExecutor {
    pub fn new(pool: SqlitePool, execution_engine: Arc<ExecutionEngine>) -> Self {
        Self {
            pool: pool.clone(),
            execution_engine: execution_engine.clone(),
            conference_intel: ConferenceIntelStage::new(pool.clone(), execution_engine.clone()),
            speaker_research: SpeakerResearchStage::new(pool.clone(), execution_engine.clone()),
            brand_research: BrandResearchStage::new(pool.clone(), execution_engine.clone()),
            production_team: ProductionTeamStage::new(pool.clone(), execution_engine.clone()),
            competitive_intel: CompetitiveIntelStage::new(pool.clone(), execution_engine.clone()),
            side_events: SideEventsStage::new(pool.clone()),
        }
    }

    /// Execute conference intel stage
    pub async fn execute_conference_intel(
        &self,
        workflow: &ConferenceWorkflow,
    ) -> Result<ResearchStageResult> {
        tracing::info!(
            "[STAGE_EXECUTOR] Running conference intel for: {}",
            workflow.conference_name
        );
        self.conference_intel.execute(workflow).await
    }

    /// Execute speaker research stage (parallel)
    pub async fn execute_speaker_research(
        &self,
        workflow: &ConferenceWorkflow,
        intel_result: &ResearchStageResult,
        parallelism_limit: usize,
    ) -> Result<Vec<ResearchStageResult>> {
        tracing::info!(
            "[STAGE_EXECUTOR] Running speaker research for: {}",
            workflow.conference_name
        );
        self.speaker_research.execute(workflow, intel_result, parallelism_limit).await
    }

    /// Execute brand research stage (parallel)
    pub async fn execute_brand_research(
        &self,
        workflow: &ConferenceWorkflow,
        intel_result: &ResearchStageResult,
        parallelism_limit: usize,
    ) -> Result<Vec<ResearchStageResult>> {
        tracing::info!(
            "[STAGE_EXECUTOR] Running brand research for: {}",
            workflow.conference_name
        );
        self.brand_research.execute(workflow, intel_result, parallelism_limit).await
    }

    /// Execute production team stage
    pub async fn execute_production_team(
        &self,
        workflow: &ConferenceWorkflow,
    ) -> Result<ResearchStageResult> {
        tracing::info!(
            "[STAGE_EXECUTOR] Running production team research for: {}",
            workflow.conference_name
        );
        self.production_team.execute(workflow).await
    }

    /// Execute competitive intel stage
    pub async fn execute_competitive_intel(
        &self,
        workflow: &ConferenceWorkflow,
    ) -> Result<ResearchStageResult> {
        tracing::info!(
            "[STAGE_EXECUTOR] Running competitive intel for: {}",
            workflow.conference_name
        );
        self.competitive_intel.execute(workflow).await
    }

    /// Execute side events stage (parallel across platforms)
    pub async fn execute_side_events(
        &self,
        workflow: &ConferenceWorkflow,
    ) -> Result<Vec<SideEvent>> {
        tracing::info!(
            "[STAGE_EXECUTOR] Running side events discovery for: {}",
            workflow.conference_name
        );
        self.side_events.execute(workflow).await
    }
}
