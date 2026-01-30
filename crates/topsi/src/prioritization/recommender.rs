//! Priority Recommender
//!
//! Provides intelligent "what should I do next?" recommendations
//! based on Active Inference prioritization.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;
use uuid::Uuid;

use super::free_energy::PotentialAction;
use super::goals::{Goal, GoalType};
use super::priority_score::{PriorityCalculator, PriorityLevel, PriorityScore};

/// A recommendation for what to work on next
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    /// Recommended task ID
    pub task_id: Uuid,
    /// Task name
    pub task_name: String,
    /// Why this is recommended
    pub reason: String,
    /// Priority score details
    pub priority: PriorityScore,
    /// Estimated impact on goals
    pub impact: HashMap<Uuid, f64>,
    /// Suggested next actions after this task
    pub follow_ups: Vec<String>,
    /// Time context (e.g., "critical for Feb 10 deadline")
    pub time_context: Option<String>,
}

/// Batch of recommendations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationBatch {
    /// Top recommendations
    pub recommendations: Vec<Recommendation>,
    /// Summary of current state
    pub summary: String,
    /// Goals being addressed
    pub active_goals: Vec<GoalSummary>,
    /// Generated timestamp
    pub generated_at: DateTime<Utc>,
}

/// Brief goal summary
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GoalSummary {
    pub id: Uuid,
    pub name: String,
    pub progress: f64,
    pub urgency: f64,
    pub time_remaining: Option<String>,
}

/// The priority recommender
pub struct PriorityRecommender {
    calculator: PriorityCalculator,
    /// Maximum recommendations to return
    max_recommendations: usize,
    /// Include follow-up suggestions
    include_follow_ups: bool,
}

impl Default for PriorityRecommender {
    fn default() -> Self {
        Self {
            calculator: PriorityCalculator::new(),
            max_recommendations: 5,
            include_follow_ups: true,
        }
    }
}

impl PriorityRecommender {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_recommendations(mut self, max: usize) -> Self {
        self.max_recommendations = max;
        self
    }

    /// Generate recommendations for what to work on next
    pub fn recommend(
        &self,
        actions: &[PotentialAction],
        goals: &[Goal],
    ) -> RecommendationBatch {
        // Calculate priorities for all actions
        let ranked = self.calculator.rank_tasks(actions, goals);

        // Build recommendations from top priorities
        let recommendations: Vec<Recommendation> = ranked
            .into_iter()
            .take(self.max_recommendations)
            .map(|priority| self.build_recommendation(priority, actions, goals))
            .collect();

        // Build goal summaries
        let active_goals: Vec<GoalSummary> = goals
            .iter()
            .filter(|g| g.completion() < 1.0)
            .map(|g| GoalSummary {
                id: g.id,
                name: g.name.clone(),
                progress: g.completion(),
                urgency: g.urgency(),
                time_remaining: g.time_until_deadline().map(|d| format_duration(d)),
            })
            .collect();

        // Generate summary
        let summary = self.generate_summary(&recommendations, &active_goals);

        RecommendationBatch {
            recommendations,
            summary,
            active_goals,
            generated_at: Utc::now(),
        }
    }

    /// Build a single recommendation
    fn build_recommendation(
        &self,
        priority: PriorityScore,
        actions: &[PotentialAction],
        goals: &[Goal],
    ) -> Recommendation {
        let action = actions.iter().find(|a| a.id == priority.task_id);

        // Calculate impact on each goal
        let mut impact = HashMap::new();
        for goal in goals {
            if let Some(contribution) = priority.efe.goal_contributions.get(&goal.id) {
                impact.insert(goal.id, *contribution);
            }
        }

        // Generate reason
        let reason = self.generate_reason(&priority, goals);

        // Generate follow-ups
        let follow_ups = if self.include_follow_ups {
            self.suggest_follow_ups(&priority, action)
        } else {
            Vec::new()
        };

        // Time context from most urgent related goal
        let time_context = goals
            .iter()
            .filter(|g| impact.contains_key(&g.id))
            .filter(|g| g.deadline.is_some())
            .min_by(|a, b| a.urgency().partial_cmp(&b.urgency()).unwrap_or(std::cmp::Ordering::Equal))
            .and_then(|g| {
                g.deadline.map(|d| {
                    let days = (d - Utc::now()).num_days();
                    format!("Due in {} days ({})", days, d.format("%b %d"))
                })
            });

        Recommendation {
            task_id: priority.task_id,
            task_name: priority.task_name.clone(),
            reason,
            priority,
            impact,
            follow_ups,
            time_context,
        }
    }

    /// Generate human-readable reason for recommendation
    fn generate_reason(&self, priority: &PriorityScore, goals: &[Goal]) -> String {
        let level = &priority.level;
        let components = &priority.components;

        let mut parts = Vec::new();

        // Priority level context
        match level {
            PriorityLevel::Critical => parts.push("Critical priority".to_string()),
            PriorityLevel::High => parts.push("High priority".to_string()),
            _ => {}
        }

        // Urgency reason
        if components.urgency > 0.3 {
            parts.push("deadline approaching".to_string());
        }

        // Blocking reason
        if components.blocking > 0.1 {
            parts.push("unblocks dependent tasks".to_string());
        }

        // Data completeness reason
        if components.completeness < 0.5 {
            parts.push("data collection needed".to_string());
        }

        // Goal impact
        if !priority.efe.goal_contributions.is_empty() {
            let top_goal_id = priority.efe.goal_contributions
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(id, _)| *id);

            if let Some(goal_id) = top_goal_id {
                if let Some(goal) = goals.iter().find(|g| g.id == goal_id) {
                    parts.push(format!("advances '{}'", goal.name));
                }
            }
        }

        if parts.is_empty() {
            "Standard task for goal progress".to_string()
        } else {
            parts.join("; ")
        }
    }

    /// Suggest follow-up actions
    fn suggest_follow_ups(
        &self,
        priority: &PriorityScore,
        action: Option<&PotentialAction>,
    ) -> Vec<String> {
        let mut follow_ups = Vec::new();

        if priority.components.completeness < 0.7 {
            follow_ups.push("Complete data enrichment".to_string());
        }

        if priority.components.blocking > 0.1 {
            follow_ups.push("Review dependent tasks".to_string());
        }

        if let Some(action) = action {
            if action.data_completeness < 0.5 {
                follow_ups.push("Gather missing information".to_string());
            }
            if !action.downstream_tasks.is_empty() {
                follow_ups.push(format!("Check {} blocked tasks", action.downstream_tasks.len()));
            }
        }

        follow_ups
    }

    /// Generate batch summary
    fn generate_summary(
        &self,
        recommendations: &[Recommendation],
        active_goals: &[GoalSummary],
    ) -> String {
        let critical_count = recommendations
            .iter()
            .filter(|r| r.priority.level == PriorityLevel::Critical)
            .count();

        let high_count = recommendations
            .iter()
            .filter(|r| r.priority.level == PriorityLevel::High)
            .count();

        let urgent_goals: Vec<&GoalSummary> = active_goals
            .iter()
            .filter(|g| g.urgency > 0.7)
            .collect();

        let mut parts = Vec::new();

        if critical_count > 0 {
            parts.push(format!("{} critical task(s)", critical_count));
        }
        if high_count > 0 {
            parts.push(format!("{} high priority task(s)", high_count));
        }
        if !urgent_goals.is_empty() {
            let goal_names: Vec<&str> = urgent_goals.iter().map(|g| g.name.as_str()).collect();
            parts.push(format!("Urgent goals: {}", goal_names.join(", ")));
        }

        if parts.is_empty() {
            "All tasks at normal priority".to_string()
        } else {
            parts.join(". ")
        }
    }

    /// Quick "what's next?" for a single top recommendation
    pub fn whats_next(&self, actions: &[PotentialAction], goals: &[Goal]) -> Option<Recommendation> {
        self.recommend(actions, goals).recommendations.into_iter().next()
    }
}

/// Format a duration in human-readable form
fn format_duration(duration: Duration) -> String {
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;

    if days > 0 {
        format!("{} days", days)
    } else if hours > 0 {
        format!("{} hours", hours)
    } else {
        "less than an hour".to_string()
    }
}

/// Create a conference workflow goal with current progress
pub fn create_conference_goal(
    name: &str,
    deadline: DateTime<Utc>,
    speakers_current: i64,
    speakers_target: i64,
    sponsors_current: i64,
    sponsors_target: i64,
    articles_current: i64,
    articles_target: i64,
) -> Goal {
    let mut goal = Goal::new(
        &format!("{} Coverage", name),
        &format!("Complete coverage for {}", name),
        GoalType::ConferenceCoverage,
        0.9,
    ).with_deadline(deadline);

    goal.state.set_metric(
        "speakers",
        speakers_current as f64,
        speakers_target as f64,
        0.30,
    );
    goal.state.set_metric(
        "sponsors",
        sponsors_current as f64,
        sponsors_target as f64,
        0.25,
    );
    goal.state.set_metric(
        "articles",
        articles_current as f64,
        articles_target as f64,
        0.30,
    );
    goal.state.set_metric(
        "graphics",
        0.0,
        (speakers_target + sponsors_target) as f64 * 0.5,
        0.15,
    );

    goal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_generation() {
        let recommender = PriorityRecommender::new();

        let action = PotentialAction::new(Uuid::new_v4(), "Research Speaker");
        let goal = Goal::new("Conference", "Coverage", GoalType::ConferenceCoverage, 0.9)
            .with_deadline(Utc::now() + Duration::days(30));

        let batch = recommender.recommend(&[action], &[goal]);
        assert!(!batch.recommendations.is_empty());
        assert!(!batch.summary.is_empty());
    }

    #[test]
    fn test_conference_goal() {
        let goal = create_conference_goal(
            "iConnection",
            Utc::now() + Duration::days(30),
            46, 100,  // speakers
            57, 100,  // sponsors
            0, 20,    // articles
        );

        assert!(goal.completion() > 0.0);
        assert!(goal.completion() < 1.0);
    }
}
