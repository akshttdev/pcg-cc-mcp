//! Goal State Modeling
//!
//! Defines goals with value functions for Active Inference prioritization.
//! Goals represent desired end states that the system works toward.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;
use uuid::Uuid;

/// Type of goal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "snake_case")]
pub enum GoalType {
    /// Conference coverage goal (speakers researched, content created, etc.)
    ConferenceCoverage,
    /// Content production goal (articles, videos, graphics)
    ContentProduction,
    /// Research completion goal (data gathering)
    Research,
    /// Launch/release goal (shipping a feature/product)
    Launch,
    /// Revenue/business goal
    Business,
    /// Custom goal with user-defined metrics
    Custom,
}

/// Current state toward a goal
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GoalState {
    /// Current progress (0.0 to 1.0)
    pub progress: f64,
    /// Metrics contributing to progress
    pub metrics: HashMap<String, MetricValue>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// A metric value with current and target
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct MetricValue {
    pub current: f64,
    pub target: f64,
    pub weight: f64,
}

impl MetricValue {
    pub fn new(current: f64, target: f64, weight: f64) -> Self {
        Self { current, target, weight }
    }

    /// Progress toward target (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.target <= 0.0 {
            return 1.0;
        }
        (self.current / self.target).min(1.0)
    }
}

impl GoalState {
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            metrics: HashMap::new(),
            updated_at: Utc::now(),
        }
    }

    /// Calculate overall progress from metrics
    pub fn calculate_progress(&mut self) -> f64 {
        if self.metrics.is_empty() {
            return self.progress;
        }

        let total_weight: f64 = self.metrics.values().map(|m| m.weight).sum();
        if total_weight <= 0.0 {
            return 0.0;
        }

        let weighted_progress: f64 = self.metrics
            .values()
            .map(|m| m.progress() * m.weight)
            .sum();

        self.progress = weighted_progress / total_weight;
        self.updated_at = Utc::now();
        self.progress
    }

    /// Add or update a metric
    pub fn set_metric(&mut self, name: &str, current: f64, target: f64, weight: f64) {
        self.metrics.insert(name.to_string(), MetricValue::new(current, target, weight));
        self.calculate_progress();
    }
}

impl Default for GoalState {
    fn default() -> Self {
        Self::new()
    }
}

/// A goal representing a desired end state
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    /// Unique identifier
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// Description of success
    pub description: String,
    /// Type of goal
    pub goal_type: GoalType,
    /// Deadline (if any)
    pub deadline: Option<DateTime<Utc>>,
    /// Value/importance (0.0 to 1.0)
    pub value: f64,
    /// Current state toward the goal
    pub state: GoalState,
    /// Related entity IDs (projects, tasks, workflows)
    pub related_entities: Vec<Uuid>,
    /// Parent goal (for hierarchical goals)
    pub parent_goal_id: Option<Uuid>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
}

impl Goal {
    /// Create a new goal
    pub fn new(name: &str, description: &str, goal_type: GoalType, value: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            goal_type,
            deadline: None,
            value: value.clamp(0.0, 1.0),
            state: GoalState::new(),
            related_entities: Vec::new(),
            parent_goal_id: None,
            created_at: Utc::now(),
        }
    }

    /// Set a deadline
    pub fn with_deadline(mut self, deadline: DateTime<Utc>) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Add related entity
    pub fn with_entity(mut self, entity_id: Uuid) -> Self {
        self.related_entities.push(entity_id);
        self
    }

    /// Set parent goal
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_goal_id = Some(parent_id);
        self
    }

    /// Calculate urgency based on deadline proximity (0.0 to 1.0)
    /// Higher value = more urgent
    pub fn urgency(&self) -> f64 {
        let Some(deadline) = self.deadline else {
            return 0.5; // Default medium urgency for no deadline
        };

        let now = Utc::now();
        if deadline <= now {
            return 1.0; // Overdue = maximum urgency
        }

        let time_remaining = deadline - now;
        let days_remaining = time_remaining.num_hours() as f64 / 24.0;

        // Exponential urgency curve
        // 0 days = 1.0, 7 days = 0.7, 30 days = 0.4, 90 days = 0.2
        let urgency = (-days_remaining / 30.0).exp();
        urgency.clamp(0.1, 1.0)
    }

    /// Calculate time until deadline
    pub fn time_until_deadline(&self) -> Option<Duration> {
        self.deadline.map(|d| d - Utc::now())
    }

    /// Check if goal is overdue
    pub fn is_overdue(&self) -> bool {
        self.deadline.map(|d| d < Utc::now()).unwrap_or(false)
    }

    /// Calculate completion percentage
    pub fn completion(&self) -> f64 {
        self.state.progress
    }

    /// Calculate remaining work (1.0 - completion)
    pub fn remaining(&self) -> f64 {
        1.0 - self.state.progress
    }

    /// Priority score combining value, urgency, and remaining work
    pub fn priority_score(&self) -> f64 {
        let value_weight = 0.4;
        let urgency_weight = 0.3;
        let remaining_weight = 0.3;

        (self.value * value_weight)
            + (self.urgency() * urgency_weight)
            + (self.remaining() * remaining_weight)
    }
}

/// Create a conference coverage goal
pub fn conference_goal(
    conference_name: &str,
    deadline: DateTime<Utc>,
    target_speakers: i64,
    target_sponsors: i64,
    target_articles: i64,
) -> Goal {
    let mut goal = Goal::new(
        &format!("{} Coverage", conference_name),
        &format!("Complete coverage for {} including research, content, and media", conference_name),
        GoalType::ConferenceCoverage,
        0.9, // High value
    ).with_deadline(deadline);

    goal.state.set_metric("speakers_researched", 0.0, target_speakers as f64, 0.25);
    goal.state.set_metric("sponsors_researched", 0.0, target_sponsors as f64, 0.20);
    goal.state.set_metric("articles_written", 0.0, target_articles as f64, 0.30);
    goal.state.set_metric("graphics_created", 0.0, (target_speakers + target_sponsors) as f64 * 0.5, 0.15);
    goal.state.set_metric("social_posts_scheduled", 0.0, target_articles as f64 * 3.0, 0.10);

    goal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metric_progress() {
        let metric = MetricValue::new(50.0, 100.0, 1.0);
        assert!((metric.progress() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_goal_urgency() {
        let future = Utc::now() + Duration::days(30);
        let goal = Goal::new("Test", "Test goal", GoalType::Custom, 0.5)
            .with_deadline(future);

        let urgency = goal.urgency();
        assert!(urgency > 0.0 && urgency < 1.0);
    }

    #[test]
    fn test_goal_state_progress() {
        let mut state = GoalState::new();
        state.set_metric("a", 50.0, 100.0, 1.0);
        state.set_metric("b", 75.0, 100.0, 1.0);

        let progress = state.progress;
        assert!((progress - 0.625).abs() < 0.001); // (0.5 + 0.75) / 2
    }
}
