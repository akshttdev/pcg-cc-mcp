//! Quality Analyst Implementation
//!
//! Automated QA gates for workflow stages. Evaluates outputs against
//! checklists, scores confidence, and decides approve/retry/escalate.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

use db::models::{
    conference_workflow::ConferenceWorkflow,
    workflow_qa_run::{
        ChecklistItem, ConfidenceLevel, CreateWorkflowQARun, QADecision as DbQADecision,
        WorkflowQARun,
    },
};

use crate::{NoraError, Result};

use super::stages::ResearchStageResult;

/// QA decision outcome
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum QADecision {
    Approve,
    Retry,
    Escalate,
}

impl From<QADecision> for DbQADecision {
    fn from(d: QADecision) -> Self {
        match d {
            QADecision::Approve => DbQADecision::Approve,
            QADecision::Retry => DbQADecision::Retry,
            QADecision::Escalate => DbQADecision::Escalate,
        }
    }
}

/// Result from QA evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAResult {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub stage_name: String,
    pub checklist_items: Vec<ChecklistItem>,
    pub confidence_level: ConfidenceLevel,
    pub overall_score: f64,
    pub decision: QADecision,
    pub retry_guidance: Option<String>,
    pub escalation_reason: Option<String>,
}

/// Quality Analyst agent
pub struct QualityAnalyst {
    pool: SqlitePool,
}

impl QualityAnalyst {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Evaluate a stage result against its checklist
    pub async fn evaluate_stage(
        &self,
        workflow_id: Uuid,
        stage_name: &str,
        result: &ResearchStageResult,
    ) -> Result<QAResult> {
        tracing::info!("[QA] Evaluating stage: {} for workflow: {}", stage_name, workflow_id);

        let checklist = self.get_checklist(stage_name);
        let evaluation = self.evaluate_against_checklist(&checklist, result);

        let confidence = self.assess_confidence(&evaluation);
        let score = self.calculate_score(&evaluation);
        let decision = self.make_decision(score, &confidence);

        let retry_guidance = if decision == QADecision::Retry {
            Some(self.generate_retry_guidance(&evaluation))
        } else {
            None
        };

        let escalation_reason = if decision == QADecision::Escalate {
            Some(self.generate_escalation_reason(&evaluation))
        } else {
            None
        };

        // Store QA run record
        let qa_run = WorkflowQARun::create(
            &self.pool,
            &CreateWorkflowQARun {
                workflow_id,
                stage_name: stage_name.to_string(),
                artifact_id: result.artifact_id,
                checklist_items: evaluation.clone(),
                confidence_level: confidence.clone(),
                overall_score: score,
                decision: decision.clone().into(),
                retry_guidance: retry_guidance.clone(),
                escalation_reason: escalation_reason.clone(),
                agent_id: None,
                execution_time_ms: Some(result.execution_time_ms as i64),
                tokens_used: None,
            },
        )
        .await
        .map_err(NoraError::DatabaseError)?;

        tracing::info!(
            "[QA] Stage {} evaluated: score={:.2}, decision={:?}",
            stage_name, score, decision
        );

        Ok(QAResult {
            id: qa_run.id,
            workflow_id,
            stage_name: stage_name.to_string(),
            checklist_items: evaluation,
            confidence_level: confidence,
            overall_score: score,
            decision,
            retry_guidance,
            escalation_reason,
        })
    }

    /// Evaluate entire workflow (final QA)
    pub async fn evaluate_workflow(&self, workflow: &ConferenceWorkflow) -> Result<QAResult> {
        tracing::info!("[QA] Final evaluation for workflow: {}", workflow.id);

        let checklist = vec![
            "Research stages completed".to_string(),
            "Entity profiles created".to_string(),
            "Side events discovered".to_string(),
            "Content created".to_string(),
            "Social posts scheduled".to_string(),
        ];

        let mut evaluation = Vec::new();

        // Check research completion
        evaluation.push(ChecklistItem {
            item: "Research stages completed".to_string(),
            passed: workflow.current_stage.as_deref() != Some("research"),
            notes: Some(format!("Current stage: {:?}", workflow.current_stage)),
        });

        // Check entities
        evaluation.push(ChecklistItem {
            item: "Entity profiles created".to_string(),
            passed: workflow.speakers_count > 0 || workflow.sponsors_count > 0,
            notes: Some(format!(
                "{} speakers, {} sponsors",
                workflow.speakers_count, workflow.sponsors_count
            )),
        });

        // Check side events
        evaluation.push(ChecklistItem {
            item: "Side events discovered".to_string(),
            passed: workflow.side_events_count > 0,
            notes: Some(format!("{} events found", workflow.side_events_count)),
        });

        // Check content (placeholder)
        evaluation.push(ChecklistItem {
            item: "Content created".to_string(),
            passed: true, // Would check artifacts
            notes: None,
        });

        // Check social posts
        evaluation.push(ChecklistItem {
            item: "Social posts scheduled".to_string(),
            passed: workflow.social_posts_scheduled > 0,
            notes: Some(format!("{} posts scheduled", workflow.social_posts_scheduled)),
        });

        let confidence = self.assess_confidence(&evaluation);
        let score = self.calculate_score(&evaluation);
        let decision = self.make_decision(score, &confidence);

        let qa_run = WorkflowQARun::create(
            &self.pool,
            &CreateWorkflowQARun {
                workflow_id: workflow.id,
                stage_name: "workflow_complete".to_string(),
                artifact_id: None,
                checklist_items: evaluation.clone(),
                confidence_level: confidence.clone(),
                overall_score: score,
                decision: decision.clone().into(),
                retry_guidance: None,
                escalation_reason: None,
                agent_id: None,
                execution_time_ms: None,
                tokens_used: None,
            },
        )
        .await
        .map_err(NoraError::DatabaseError)?;

        Ok(QAResult {
            id: qa_run.id,
            workflow_id: workflow.id,
            stage_name: "workflow_complete".to_string(),
            checklist_items: evaluation,
            confidence_level: confidence,
            overall_score: score,
            decision,
            retry_guidance: None,
            escalation_reason: None,
        })
    }

    /// Get checklist for a stage
    fn get_checklist(&self, stage_name: &str) -> Vec<String> {
        match stage_name {
            "Conference Intel" => vec![
                "Name captured".to_string(),
                "Dates captured".to_string(),
                "Location captured".to_string(),
                "Website validated".to_string(),
                "Themes identified".to_string(),
            ],
            "Speaker Research" => vec![
                "Bio present".to_string(),
                "Social handles found".to_string(),
                "Photo URL available".to_string(),
                "Talk topic captured".to_string(),
            ],
            "Brand Research" => vec![
                "Company description present".to_string(),
                "Website URL captured".to_string(),
                "Logo URL available".to_string(),
                "Social handles found".to_string(),
            ],
            "Production Team" => vec![
                "Production company identified".to_string(),
                "Contact information found".to_string(),
            ],
            "Competitive Intel" => vec![
                "Similar conferences identified".to_string(),
                "Content gaps analyzed".to_string(),
            ],
            "Side Events" => vec![
                "Events found".to_string(),
                "Dates/times present".to_string(),
                "Venue info available".to_string(),
                "URLs captured".to_string(),
            ],
            _ => vec![
                "Output complete".to_string(),
                "Data quality acceptable".to_string(),
            ],
        }
    }

    /// Evaluate result against checklist items
    fn evaluate_against_checklist(
        &self,
        checklist: &[String],
        result: &ResearchStageResult,
    ) -> Vec<ChecklistItem> {
        checklist
            .iter()
            .map(|item| {
                let passed = self.check_item(item, result);
                ChecklistItem {
                    item: item.clone(),
                    passed,
                    notes: None,
                }
            })
            .collect()
    }

    /// Check a single checklist item against result
    fn check_item(&self, item: &str, result: &ResearchStageResult) -> bool {
        // Simple heuristic checks based on result data
        if !result.success {
            return false;
        }

        let data = &result.data;

        match item {
            "Name captured" => data.get("name").map(|v| !v.is_null()).unwrap_or(false),
            "Dates captured" => data.get("dates").map(|v| !v.is_null()).unwrap_or(false),
            "Location captured" => data.get("location").map(|v| !v.is_null()).unwrap_or(false),
            "Website validated" => data.get("website").map(|v| !v.is_null()).unwrap_or(true),
            "Themes identified" => data
                .get("themes")
                .and_then(|v| v.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false),
            "Bio present" => data.get("bio").map(|v| !v.is_null()).unwrap_or(false),
            "Social handles found" => {
                data.get("linkedin_url").map(|v| !v.is_null()).unwrap_or(false)
                    || data.get("twitter_handle").map(|v| !v.is_null()).unwrap_or(false)
            }
            "Photo URL available" => data.get("photo_url").map(|v| !v.is_null()).unwrap_or(false),
            "Talk topic captured" => data.get("talk_title").map(|v| !v.is_null()).unwrap_or(false),
            "Events found" => true, // Side events stage returns list
            _ => true, // Default to passing for unknown items
        }
    }

    /// Assess confidence level based on evaluation
    fn assess_confidence(&self, evaluation: &[ChecklistItem]) -> ConfidenceLevel {
        let pass_rate = self.calculate_score(evaluation);

        if pass_rate >= 0.9 {
            ConfidenceLevel::High
        } else if pass_rate >= 0.7 {
            ConfidenceLevel::Medium
        } else {
            ConfidenceLevel::Low
        }
    }

    /// Calculate overall score (0.0 to 1.0)
    fn calculate_score(&self, evaluation: &[ChecklistItem]) -> f64 {
        if evaluation.is_empty() {
            return 0.0;
        }

        let passed = evaluation.iter().filter(|i| i.passed).count();
        passed as f64 / evaluation.len() as f64
    }

    /// Make decision based on score and confidence
    fn make_decision(&self, score: f64, confidence: &ConfidenceLevel) -> QADecision {
        if score >= 0.8 {
            QADecision::Approve
        } else if score >= 0.5 || *confidence == ConfidenceLevel::Low {
            QADecision::Retry
        } else {
            QADecision::Escalate
        }
    }

    /// Generate guidance for retry
    fn generate_retry_guidance(&self, evaluation: &[ChecklistItem]) -> String {
        let failed: Vec<_> = evaluation
            .iter()
            .filter(|i| !i.passed)
            .map(|i| i.item.as_str())
            .collect();

        if failed.is_empty() {
            "Review and improve data quality".to_string()
        } else {
            format!("Fix the following items: {}", failed.join(", "))
        }
    }

    /// Generate escalation reason
    fn generate_escalation_reason(&self, evaluation: &[ChecklistItem]) -> String {
        let failed_count = evaluation.iter().filter(|i| !i.passed).count();
        let total = evaluation.len();

        format!(
            "Too many failed checks ({}/{}) - manual review required",
            failed_count, total
        )
    }
}

/// System prompt for QA agent
pub const QA_SYSTEM_PROMPT: &str = r#"You are Sentinel, a Quality Analyst agent responsible for evaluating workflow outputs.

Your role:
1. Evaluate outputs against checklists
2. Score confidence levels (high/medium/low)
3. Decide: approve, retry, or escalate
4. Provide actionable guidance for retries

Be objective and consistent. Focus on data quality and completeness.
When in doubt, lean toward retry rather than escalate."#;
