//! Dynamic Priority Scoring
//!
//! Combines Active Inference EFE with practical task attributes
//! to produce actionable priority scores.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use super::free_energy::{EFECalculator, ExpectedFreeEnergy, PotentialAction};
use super::goals::Goal;

/// Priority level (human-readable)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum PriorityLevel {
    Critical,
    High,
    Medium,
    Low,
    Backlog,
}

impl PriorityLevel {
    /// Convert from numeric score (0.0 to 1.0)
    pub fn from_score(score: f64) -> Self {
        match score {
            s if s >= 0.85 => PriorityLevel::Critical,
            s if s >= 0.70 => PriorityLevel::High,
            s if s >= 0.45 => PriorityLevel::Medium,
            s if s >= 0.25 => PriorityLevel::Low,
            _ => PriorityLevel::Backlog,
        }
    }

    /// Convert to numeric value
    pub fn to_value(&self) -> f64 {
        match self {
            PriorityLevel::Critical => 1.0,
            PriorityLevel::High => 0.75,
            PriorityLevel::Medium => 0.5,
            PriorityLevel::Low => 0.25,
            PriorityLevel::Backlog => 0.1,
        }
    }
}

/// Detailed priority score for a task
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PriorityScore {
    /// Task/action ID
    pub task_id: Uuid,
    /// Task name
    pub task_name: String,
    /// Numeric score (0.0 to 1.0)
    pub score: f64,
    /// Human-readable priority level
    pub level: PriorityLevel,
    /// Expected Free Energy breakdown
    pub efe: ExpectedFreeEnergy,
    /// Score components breakdown
    pub components: ScoreComponents,
    /// Reasoning for this score
    pub reasoning: String,
    /// Calculated at timestamp
    pub calculated_at: DateTime<Utc>,
}

/// Breakdown of score components
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScoreComponents {
    /// Information gain component
    pub epistemic: f64,
    /// Goal proximity component
    pub pragmatic: f64,
    /// Time urgency component
    pub urgency: f64,
    /// Dependency blocking component
    pub blocking: f64,
    /// Data completeness factor
    pub completeness: f64,
}

impl PriorityScore {
    /// Create a new priority score
    pub fn new(task_id: Uuid, task_name: &str, efe: ExpectedFreeEnergy) -> Self {
        let components = ScoreComponents {
            epistemic: efe.epistemic_value,
            pragmatic: efe.pragmatic_value,
            urgency: efe.urgency_factor - 1.0, // Normalize to 0-based
            blocking: efe.dependency_impact,
            completeness: 0.0, // Set externally
        };

        let score = efe.total.clamp(0.0, 1.0);
        let level = PriorityLevel::from_score(score);

        Self {
            task_id,
            task_name: task_name.to_string(),
            score,
            level,
            efe,
            components,
            reasoning: String::new(),
            calculated_at: Utc::now(),
        }
    }

    /// Generate reasoning explanation
    pub fn with_reasoning(mut self) -> Self {
        let mut reasons = Vec::new();

        if self.components.urgency > 0.3 {
            reasons.push("approaching deadline");
        }
        if self.components.blocking > 0.1 {
            reasons.push("blocks other tasks");
        }
        if self.components.epistemic > 0.2 {
            reasons.push("high information gain");
        }
        if self.components.pragmatic > 0.3 {
            reasons.push("advances key goals");
        }
        if self.components.completeness < 0.5 {
            reasons.push("incomplete data");
        }

        if reasons.is_empty() {
            self.reasoning = "Standard priority based on goal alignment".to_string();
        } else {
            self.reasoning = format!("Priority due to: {}", reasons.join(", "));
        }

        self
    }
}

/// Calculator for priority scores
pub struct PriorityCalculator {
    efe_calculator: EFECalculator,
    /// Boost for tasks with incomplete data (encourages research)
    research_boost: f64,
    /// Penalty for already-complete tasks
    completion_penalty: f64,
}

impl Default for PriorityCalculator {
    fn default() -> Self {
        Self {
            efe_calculator: EFECalculator::default(),
            research_boost: 0.1,
            completion_penalty: 0.5,
        }
    }
}

impl PriorityCalculator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate priority score for a single task
    pub fn calculate(
        &self,
        action: &PotentialAction,
        goals: &[Goal],
    ) -> PriorityScore {
        let efe = self.efe_calculator.calculate(action, goals);
        let mut priority = PriorityScore::new(action.id, &action.name, efe);

        // Apply research boost for incomplete data
        if action.data_completeness < 0.7 {
            let boost = (1.0 - action.data_completeness) * self.research_boost;
            priority.score = (priority.score + boost).min(1.0);
        }

        // Apply completion penalty
        if action.data_completeness > 0.95 {
            priority.score *= self.completion_penalty;
        }

        priority.components.completeness = action.data_completeness;
        priority.level = PriorityLevel::from_score(priority.score);
        priority.with_reasoning()
    }

    /// Calculate and rank all tasks
    pub fn rank_tasks(
        &self,
        actions: &[PotentialAction],
        goals: &[Goal],
    ) -> Vec<PriorityScore> {
        let mut scores: Vec<PriorityScore> = actions
            .iter()
            .map(|a| self.calculate(a, goals))
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
        });

        scores
    }

    /// Get the top N priority tasks
    pub fn top_priorities(
        &self,
        actions: &[PotentialAction],
        goals: &[Goal],
        n: usize,
    ) -> Vec<PriorityScore> {
        self.rank_tasks(actions, goals).into_iter().take(n).collect()
    }
}

/// Quick priority check for a task
pub fn quick_priority(
    task_name: &str,
    deadline_days: Option<i64>,
    data_completeness: f64,
    downstream_count: usize,
) -> PriorityLevel {
    let mut score = 0.5; // Base score

    // Urgency from deadline
    if let Some(days) = deadline_days {
        score += match days {
            d if d <= 1 => 0.4,
            d if d <= 7 => 0.25,
            d if d <= 14 => 0.15,
            d if d <= 30 => 0.05,
            _ => 0.0,
        };
    }

    // Boost for incomplete data (needs research)
    if data_completeness < 0.5 {
        score += 0.1;
    }

    // Blocking factor
    score += (downstream_count as f64 * 0.05).min(0.2);

    PriorityLevel::from_score(score.min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_level_from_score() {
        assert_eq!(PriorityLevel::from_score(0.9), PriorityLevel::Critical);
        assert_eq!(PriorityLevel::from_score(0.75), PriorityLevel::High);
        assert_eq!(PriorityLevel::from_score(0.5), PriorityLevel::Medium);
        assert_eq!(PriorityLevel::from_score(0.3), PriorityLevel::Low);
        assert_eq!(PriorityLevel::from_score(0.1), PriorityLevel::Backlog);
    }

    #[test]
    fn test_quick_priority() {
        let critical = quick_priority("Urgent task", Some(1), 0.3, 5);
        assert_eq!(critical, PriorityLevel::Critical);

        let low = quick_priority("Not urgent", Some(60), 0.9, 0);
        assert!(matches!(low, PriorityLevel::Medium | PriorityLevel::Low));
    }
}
