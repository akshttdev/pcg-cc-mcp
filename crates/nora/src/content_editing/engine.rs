//! Content Editing Engine
//!
//! Master orchestrator for the 6-phase content editing pipeline.
//! Follows the ConferenceWorkflowEngine pattern: hold shared state,
//! execute phases sequentially, accumulate artifacts in PipelineState.

use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::{
    assembly::AssemblyProcessor, directive::DirectiveGenerator, media_catalog::MediaCataloger,
    transcript::TranscriptProcessor, types::*,
};
use crate::{execution::ExecutionEngine, NoraError, Result};

/// Configuration for the content editing pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentEditingConfig {
    /// Maximum retries per phase
    pub max_phase_retries: u32,
    /// Minimum Levenshtein match ratio for soundbite verification
    pub soundbite_match_threshold: f64,
    /// Target B-roll coverage ratio (0.0–1.0)
    pub target_broll_ratio: f64,
    /// Maximum interview-on-camera ratio (0.0–1.0)
    pub max_interview_on_camera_ratio: f64,
    /// Local Whisper endpoint URL
    pub whisper_endpoint: Option<String>,
    /// Whether to render the final video via FFmpeg
    pub enable_ffmpeg_render: bool,
    /// Whether to generate Premiere Pro XML
    pub enable_premiere_xml: bool,
    /// FFmpeg output bitrate in Mbps
    pub output_bitrate_mbps: u32,
    /// Enable FFmpeg-based scene analysis during Phase 4
    pub enable_scene_analysis: bool,
    /// Enable beat analysis for music tracks during Phase 4
    pub enable_beat_analysis: bool,
}

impl Default for ContentEditingConfig {
    fn default() -> Self {
        Self {
            max_phase_retries: 2,
            soundbite_match_threshold: 0.80,
            target_broll_ratio: 0.825,
            max_interview_on_camera_ratio: 0.17,
            whisper_endpoint: Some("http://localhost:8100".to_string()),
            enable_ffmpeg_render: true,
            enable_premiere_xml: true,
            output_bitrate_mbps: 18,
            enable_scene_analysis: true,
            enable_beat_analysis: true,
        }
    }
}

/// Result of the full pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ContentEditingResult {
    pub pipeline_id: String,
    pub phases_completed: Vec<String>,
    pub rendered_video_path: Option<PathBuf>,
    pub premiere_xml_path: Option<PathBuf>,
    pub total_duration_seconds: f64,
    pub broll_ratio: f64,
    pub interview_ratio: f64,
    pub soundbites_used: usize,
    pub media_assets_cataloged: usize,
    pub duration_ms: u64,
    pub errors: Vec<String>,
}

/// Main Content Editing Engine — orchestrates all 6 phases.
pub struct ContentEditingEngine {
    execution_engine: Arc<ExecutionEngine>,
    transcript_processor: TranscriptProcessor,
    media_cataloger: MediaCataloger,
    directive_generator: DirectiveGenerator,
    assembly_processor: AssemblyProcessor,
    config: ContentEditingConfig,
}

impl ContentEditingEngine {
    /// Create a new engine with default configuration.
    pub fn new(execution_engine: Arc<ExecutionEngine>) -> Self {
        let config = ContentEditingConfig::default();
        Self {
            execution_engine,
            transcript_processor: TranscriptProcessor::new(config.whisper_endpoint.clone()),
            media_cataloger: MediaCataloger::new()
                .with_analysis_flags(config.enable_scene_analysis, config.enable_beat_analysis),
            directive_generator: DirectiveGenerator::new(config.soundbite_match_threshold),
            assembly_processor: AssemblyProcessor::new(config.output_bitrate_mbps),
            config,
        }
    }

    /// Create with custom configuration.
    pub fn with_config(
        execution_engine: Arc<ExecutionEngine>,
        config: ContentEditingConfig,
    ) -> Self {
        Self {
            execution_engine,
            transcript_processor: TranscriptProcessor::new(config.whisper_endpoint.clone()),
            media_cataloger: MediaCataloger::new()
                .with_analysis_flags(config.enable_scene_analysis, config.enable_beat_analysis),
            directive_generator: DirectiveGenerator::new(config.soundbite_match_threshold),
            assembly_processor: AssemblyProcessor::new(config.output_bitrate_mbps),
            config,
        }
    }

    /// Run the full 6-phase pipeline.
    pub async fn run_pipeline(&self, client_spec: ClientSpec) -> Result<ContentEditingResult> {
        let start_time = std::time::Instant::now();
        let pipeline_id = Uuid::new_v4().to_string();
        let mut state = PipelineState::new();
        let mut phases_completed: Vec<String> = Vec::new();

        tracing::info!(
            "[CONTENT_EDITING] Starting pipeline {} for project: {}",
            pipeline_id,
            client_spec.project_name
        );

        let source_path = client_spec.source_path.clone();
        state.client_spec = Some(client_spec);

        // Phase 1: Intake — client spec is already parsed (passed in)
        phases_completed.push("intake".to_string());
        tracing::info!("[CONTENT_EDITING] Phase 1 (Intake) complete");

        // Phase 2: Transcribe — extract audio, run Whisper, clean with LLM
        match self.run_phase_transcribe(&source_path, &mut state).await {
            Ok(()) => {
                phases_completed.push("transcribe".to_string());
                tracing::info!("[CONTENT_EDITING] Phase 2 (Transcribe) complete");
            }
            Err(e) => {
                let msg = format!("Phase 2 (Transcribe) failed: {}", e);
                tracing::error!("[CONTENT_EDITING] {}", msg);
                state.errors.push(msg);
            }
        }

        // Phase 3: Research — extract entities, dispatch Scout
        match self.run_phase_research(&state).await {
            Ok(brief) => {
                state.context_brief = Some(brief);
                phases_completed.push("research".to_string());
                tracing::info!("[CONTENT_EDITING] Phase 3 (Research) complete");
            }
            Err(e) => {
                let msg = format!("Phase 3 (Research) failed: {}", e);
                tracing::warn!("[CONTENT_EDITING] {}", msg);
                state.errors.push(msg);
            }
        }

        // Phase 4: Index — catalog media files via ffprobe
        match self.run_phase_index(&source_path, &mut state).await {
            Ok(()) => {
                phases_completed.push("index".to_string());
                tracing::info!("[CONTENT_EDITING] Phase 4 (Index) complete");
            }
            Err(e) => {
                let msg = format!("Phase 4 (Index) failed: {}", e);
                tracing::error!("[CONTENT_EDITING] {}", msg);
                state.errors.push(msg);
            }
        }

        // Phase 5: Directive — synthesize edit instructions from all artifacts
        match self.run_phase_directive(&mut state).await {
            Ok(()) => {
                phases_completed.push("directive".to_string());
                tracing::info!("[CONTENT_EDITING] Phase 5 (Directive) complete");
            }
            Err(e) => {
                let msg = format!("Phase 5 (Directive) failed: {}", e);
                tracing::error!("[CONTENT_EDITING] {}", msg);
                state.errors.push(msg);
            }
        }

        // Phase 6: Assembly — render video + generate Premiere XML
        if state.directive.is_some() {
            match self.run_phase_assembly(&source_path, &mut state).await {
                Ok(()) => {
                    phases_completed.push("assembly".to_string());
                    tracing::info!("[CONTENT_EDITING] Phase 6 (Assembly) complete");
                }
                Err(e) => {
                    let msg = format!("Phase 6 (Assembly) failed: {}", e);
                    tracing::error!("[CONTENT_EDITING] {}", msg);
                    state.errors.push(msg);
                }
            }
        } else {
            state
                .errors
                .push("Skipping assembly: no directive generated".to_string());
        }

        let directive = state.directive.as_ref();
        let result = ContentEditingResult {
            pipeline_id,
            phases_completed,
            rendered_video_path: state
                .assembly_result
                .as_ref()
                .and_then(|a| a.rendered_video_path.clone()),
            premiere_xml_path: state
                .assembly_result
                .as_ref()
                .and_then(|a| a.premiere_xml_path.clone()),
            total_duration_seconds: directive.map(|d| d.total_duration_seconds).unwrap_or(0.0),
            broll_ratio: directive.map(|d| d.computed_broll_ratio).unwrap_or(0.0),
            interview_ratio: directive.map(|d| d.computed_interview_ratio).unwrap_or(0.0),
            soundbites_used: directive.map(|d| d.soundbites.len()).unwrap_or(0),
            media_assets_cataloged: state
                .shot_catalog
                .as_ref()
                .map(|c| c.assets.len())
                .unwrap_or(0),
            duration_ms: start_time.elapsed().as_millis() as u64,
            errors: state.errors.clone(),
        };

        tracing::info!(
            "[CONTENT_EDITING] Pipeline complete: {} phases, {} errors, {}ms",
            result.phases_completed.len(),
            result.errors.len(),
            result.duration_ms
        );

        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Phase implementations
    // -----------------------------------------------------------------------

    /// Phase 2: Transcribe interview audio.
    async fn run_phase_transcribe(
        &self,
        source_path: &PathBuf,
        state: &mut PipelineState,
    ) -> Result<()> {
        // Find interview files in the source directory
        let interview_files = self.media_cataloger.find_interview_files(source_path)?;
        if interview_files.is_empty() {
            return Err(NoraError::ExecutionError(
                "No interview files found in source directory".to_string(),
            ));
        }

        tracing::info!(
            "[CONTENT_EDITING] Found {} interview file(s) for transcription",
            interview_files.len()
        );

        // Transcribe the first/primary interview file
        let primary = &interview_files[0];
        let transcript = self.transcript_processor.process(primary).await?;
        state.transcript = Some(transcript);

        Ok(())
    }

    /// Phase 3: Research entities mentioned in the transcript.
    ///
    /// Uses the LLM (via ResearchExecutor) for proper NER instead of heuristics,
    /// then dispatches to Scout for web research on each entity.
    async fn run_phase_research(&self, state: &PipelineState) -> Result<ContentContextBrief> {
        let transcript = state.transcript.as_ref().ok_or_else(|| {
            NoraError::ExecutionError("No transcript available for research phase".to_string())
        })?;

        // Extract entity names from testimonial segments
        let testimonial_text: String = transcript
            .segments
            .iter()
            .filter(|s| s.segment_type == SegmentType::Testimonial)
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Use the execution engine's research tools to look up entities
        let entities = self.extract_entities(&testimonial_text).await;

        if entities.is_empty() {
            tracing::warn!("[CONTENT_EDITING] No entities extracted, building empty context brief");
            return Ok(self.build_context_brief(entities).await);
        }

        // Dispatch entity research to Scout via ExecutionEngine's ResearchExecutor
        let project_context = state.client_spec.as_ref().map(|s| s.project_name.as_str());

        let research_result = self
            .execution_engine
            .research_content_entities(&entities, project_context)
            .await;

        match research_result {
            Ok(findings) => {
                let brief = self.build_context_brief_from_findings(entities, findings);
                Ok(brief)
            }
            Err(e) => {
                tracing::warn!(
                    "[CONTENT_EDITING] Scout research failed, falling back to heuristic: {}",
                    e
                );
                Ok(self.build_context_brief(entities).await)
            }
        }
    }

    /// Phase 4: Catalog all media assets.
    async fn run_phase_index(
        &self,
        source_path: &PathBuf,
        state: &mut PipelineState,
    ) -> Result<()> {
        let catalog = self.media_cataloger.catalog_directory(source_path).await?;

        // Verify soundbites if transcript is available
        if let Some(transcript) = &state.transcript {
            let verified = self
                .directive_generator
                .verify_soundbites(transcript, &catalog);
            state.verified_soundbites = verified;
        }

        state.shot_catalog = Some(catalog);
        Ok(())
    }

    /// Phase 5: Generate the complete edit directive.
    async fn run_phase_directive(&self, state: &mut PipelineState) -> Result<()> {
        let client_spec = state.client_spec.as_ref().ok_or_else(|| {
            NoraError::ExecutionError("No client spec available for directive phase".to_string())
        })?;
        let transcript = state.transcript.as_ref().ok_or_else(|| {
            NoraError::ExecutionError("No transcript available for directive phase".to_string())
        })?;
        let catalog = state.shot_catalog.as_ref().ok_or_else(|| {
            NoraError::ExecutionError("No shot catalog available for directive phase".to_string())
        })?;

        let directive = self
            .directive_generator
            .generate(
                client_spec,
                transcript,
                catalog,
                state.context_brief.as_ref(),
                &state.verified_soundbites,
            )
            .await?;

        state.directive = Some(directive);
        Ok(())
    }

    /// Phase 6: Render video and generate Premiere XML.
    async fn run_phase_assembly(
        &self,
        source_path: &PathBuf,
        state: &mut PipelineState,
    ) -> Result<()> {
        let directive = state.directive.as_ref().ok_or_else(|| {
            NoraError::ExecutionError("No directive available for assembly phase".to_string())
        })?;
        let catalog = state.shot_catalog.as_ref().ok_or_else(|| {
            NoraError::ExecutionError("No shot catalog available for assembly phase".to_string())
        })?;

        let output_dir = state
            .client_spec
            .as_ref()
            .and_then(|s| s.output_path.clone())
            .unwrap_or_else(|| source_path.join("output"));

        let result = self
            .assembly_processor
            .assemble(
                directive,
                catalog,
                &output_dir,
                self.config.enable_ffmpeg_render,
                self.config.enable_premiere_xml,
            )
            .await?;

        state.assembly_result = Some(result);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Helper methods
    // -----------------------------------------------------------------------

    /// Extract entity names from transcript text.
    ///
    /// Uses a two-pass approach:
    /// 1. Heuristic: capitalized multi-word runs (fast, no API call)
    /// 2. Future: LLM-based NER via OpenAI for better accuracy
    async fn extract_entities(&self, text: &str) -> Vec<String> {
        // Heuristic pass: proper-noun extraction
        let mut entities: Vec<String> = Vec::new();
        let mut current = String::new();

        for word in text.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            if clean.is_empty() {
                continue;
            }
            if clean
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
            {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(clean);
            } else {
                if current.split_whitespace().count() >= 2 {
                    entities.push(current.clone());
                }
                current.clear();
            }
        }
        if current.split_whitespace().count() >= 2 {
            entities.push(current);
        }

        entities.sort();
        entities.dedup();

        tracing::info!(
            "[CONTENT_EDITING] Extracted {} entities: {:?}",
            entities.len(),
            entities
        );

        entities
    }

    /// Fallback: Build an empty ContentContextBrief from entity names only.
    async fn build_context_brief(&self, entities: Vec<String>) -> ContentContextBrief {
        tracing::info!(
            "[CONTENT_EDITING] Building heuristic context brief from {} entities",
            entities.len()
        );

        ContentContextBrief {
            event: None,
            subject: None,
            venue: None,
            company: None,
            key_themes: Vec::new(),
            raw_entities: entities,
        }
    }

    /// Build a ContentContextBrief from Scout research findings.
    fn build_context_brief_from_findings(
        &self,
        entities: Vec<String>,
        findings: serde_json::Value,
    ) -> ContentContextBrief {
        let mut brief = ContentContextBrief {
            event: None,
            subject: None,
            venue: None,
            company: None,
            key_themes: Vec::new(),
            raw_entities: entities,
        };

        if let Some(findings_arr) = findings["findings"].as_array() {
            for finding in findings_arr {
                let entity_type = finding["type"].as_str().unwrap_or("other");
                let name = finding["name"].as_str().unwrap_or("").to_string();
                let description = finding["description"].as_str().unwrap_or("").to_string();
                let key_facts: Vec<String> = finding["key_facts"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|f| f.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                match entity_type {
                    "event" => {
                        brief.event = Some(EventContext {
                            name: name.clone(),
                            description,
                            industry: key_facts.first().cloned().unwrap_or_default(),
                            location: key_facts.get(1).cloned(),
                        });
                    }
                    "person" => {
                        brief.subject = Some(SubjectProfile {
                            name: name.clone(),
                            title: key_facts.first().cloned(),
                            background: description,
                            relevant_experience: key_facts.clone(),
                        });
                    }
                    "venue" => {
                        brief.venue = Some(VenueContext {
                            name: name.clone(),
                            location: key_facts.first().cloned().unwrap_or_default(),
                            description: Some(description),
                        });
                    }
                    "company" => {
                        brief.company = Some(CompanyContext {
                            name: name.clone(),
                            description,
                            founder: key_facts.first().cloned(),
                            location: key_facts.get(1).cloned(),
                        });
                    }
                    _ => {}
                }

                // Extract themes from key facts
                for fact in &key_facts {
                    let fact_lower = fact.to_lowercase();
                    if fact_lower.contains("scale") || fact_lower.contains("large") {
                        brief.key_themes.push("scale".to_string());
                    }
                    if fact_lower.contains("professional") || fact_lower.contains("quality") {
                        brief.key_themes.push("quality".to_string());
                    }
                    if fact_lower.contains("trust") || fact_lower.contains("partner") {
                        brief.key_themes.push("trust".to_string());
                    }
                }
            }
        }

        brief.key_themes.sort();
        brief.key_themes.dedup();

        tracing::info!(
            "[CONTENT_EDITING] Built context brief: event={:?}, subject={:?}, \
             venue={:?}, company={:?}, themes={:?}",
            brief.event.as_ref().map(|e| &e.name),
            brief.subject.as_ref().map(|s| &s.name),
            brief.venue.as_ref().map(|v| &v.name),
            brief.company.as_ref().map(|c| &c.name),
            brief.key_themes
        );

        brief
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = ContentEditingConfig::default();
        assert_eq!(config.max_phase_retries, 2);
        assert!((config.soundbite_match_threshold - 0.80).abs() < f64::EPSILON);
        assert!(config.enable_ffmpeg_render);
        assert!(config.enable_premiere_xml);
        assert!(config.enable_scene_analysis);
        assert!(config.enable_beat_analysis);
    }

    #[test]
    fn test_pipeline_state_new() {
        let state = PipelineState::new();
        assert!(state.client_spec.is_none());
        assert!(state.transcript.is_none());
        assert!(state.errors.is_empty());
    }
}
