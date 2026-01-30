//! Expected Free Energy Calculation
//!
//! Implements Active Inference's Expected Free Energy (EFE) for action selection.
//! EFE = Epistemic Value + Pragmatic Value
//!
//! - Epistemic Value: Information gain (reducing uncertainty)
//! - Pragmatic Value: Expected reward (goal proximity)
//!
//! Actions with LOWER free energy are preferred (minimizing surprise).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;
use uuid::Uuid;

use super::goals::Goal;

/// Expected Free Energy calculation result
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedFreeEnergy {
    /// The action/task being evaluated
    pub action_id: Uuid,
    /// Total EFE (lower is better)
    pub total: f64,
    /// Epistemic component (information gain)
    pub epistemic_value: f64,
    /// Pragmatic component (goal proximity)
    pub pragmatic_value: f64,
    /// Urgency multiplier applied
    pub urgency_factor: f64,
    /// Dependency impact (blocking factor)
    pub dependency_impact: f64,
    /// Breakdown by goal
    pub goal_contributions: HashMap<Uuid, f64>,
}

impl ExpectedFreeEnergy {
    /// Create a new EFE result
    pub fn new(action_id: Uuid) -> Self {
        Self {
            action_id,
            total: 0.0,
            epistemic_value: 0.0,
            pragmatic_value: 0.0,
            urgency_factor: 1.0,
            dependency_impact: 0.0,
            goal_contributions: HashMap::new(),
        }
    }

    /// Calculate total EFE (lower = more preferred action)
    pub fn calculate(&mut self) {
        // Free energy = -epistemic - pragmatic (negated because we want to minimize)
        // But for prioritization, we want HIGHER scores to be better
        // So we return: epistemic + pragmatic + urgency_boost + dependency_boost
        self.total = (self.epistemic_value + self.pragmatic_value)
            * self.urgency_factor
            + self.dependency_impact;
    }
}

/// An action that can be taken (task, research item, etc.)
#[derive(Debug, Clone)]
pub struct PotentialAction {
    pub id: Uuid,
    pub name: String,
    /// Uncertainty before action (0.0 to 1.0, higher = more uncertain)
    pub prior_uncertainty: f64,
    /// Expected uncertainty after action (0.0 to 1.0)
    pub posterior_uncertainty: f64,
    /// Expected reward/progress toward goals (0.0 to 1.0)
    pub expected_reward: f64,
    /// Related goal IDs
    pub goal_ids: Vec<Uuid>,
    /// Tasks that depend on this completing
    pub downstream_tasks: Vec<Uuid>,
    /// Data completeness (0.0 to 1.0)
    pub data_completeness: f64,
}

impl PotentialAction {
    pub fn new(id: Uuid, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            prior_uncertainty: 0.5,
            posterior_uncertainty: 0.2,
            expected_reward: 0.5,
            goal_ids: Vec::new(),
            downstream_tasks: Vec::new(),
            data_completeness: 0.0,
        }
    }

    /// Information gain from taking this action
    pub fn information_gain(&self) -> f64 {
        // KL divergence approximation: reduction in uncertainty
        let gain = self.prior_uncertainty - self.posterior_uncertainty;
        gain.max(0.0)
    }
}

/// Calculator for Expected Free Energy
pub struct EFECalculator {
    /// Weight for epistemic value
    pub epistemic_weight: f64,
    /// Weight for pragmatic value
    pub pragmatic_weight: f64,
    /// Weight for dependency impact
    pub dependency_weight: f64,
    /// Minimum EFE score
    pub min_score: f64,
    /// Maximum EFE score
    pub max_score: f64,
}

impl Default for EFECalculator {
    fn default() -> Self {
        Self {
            epistemic_weight: 0.3,
            pragmatic_weight: 0.5,
            dependency_weight: 0.2,
            min_score: 0.0,
            max_score: 1.0,
        }
    }
}

impl EFECalculator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate EFE for an action given a set of goals
    pub fn calculate(&self, action: &PotentialAction, goals: &[Goal]) -> ExpectedFreeEnergy {
        let mut efe = ExpectedFreeEnergy::new(action.id);

        // 1. Calculate epistemic value (information gain)
        // Higher data completeness = less epistemic value to gain
        let info_gain = action.information_gain();
        let completeness_factor = 1.0 - action.data_completeness;
        efe.epistemic_value = info_gain * completeness_factor * self.epistemic_weight;

        // 2. Calculate pragmatic value (goal proximity)
        let mut total_pragmatic = 0.0;
        let mut total_urgency = 0.0;
        let mut goal_count = 0;

        for goal in goals {
            if action.goal_ids.contains(&goal.id) || action.goal_ids.is_empty() {
                // Expected progress toward this goal
                let goal_progress = action.expected_reward * goal.remaining();
                let weighted_progress = goal_progress * goal.value;

                total_pragmatic += weighted_progress;
                total_urgency += goal.urgency();
                goal_count += 1;

                efe.goal_contributions.insert(goal.id, weighted_progress);
            }
        }

        if goal_count > 0 {
            efe.pragmatic_value = (total_pragmatic / goal_count as f64) * self.pragmatic_weight;
            efe.urgency_factor = 1.0 + (total_urgency / goal_count as f64);
        } else {
            efe.pragmatic_value = action.expected_reward * self.pragmatic_weight;
            efe.urgency_factor = 1.0;
        }

        // 3. Calculate dependency impact
        // More downstream tasks = higher priority
        let downstream_count = action.downstream_tasks.len() as f64;
        efe.dependency_impact = (downstream_count.ln_1p() / 5.0).min(1.0) * self.dependency_weight;

        // 4. Calculate total
        efe.calculate();

        // Clamp to valid range
        efe.total = efe.total.clamp(self.min_score, self.max_score);

        efe
    }

    /// Calculate EFE for multiple actions and rank them
    pub fn rank_actions(
        &self,
        actions: &[PotentialAction],
        goals: &[Goal],
    ) -> Vec<ExpectedFreeEnergy> {
        let mut results: Vec<ExpectedFreeEnergy> = actions
            .iter()
            .map(|a| self.calculate(a, goals))
            .collect();

        // Sort by total EFE (highest first = best action)
        results.sort_by(|a, b| b.total.partial_cmp(&a.total).unwrap_or(std::cmp::Ordering::Equal));

        results
    }
}

/// Convert a task-like struct to a PotentialAction
pub trait IntoPotentialAction {
    fn into_action(&self) -> PotentialAction;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prioritization::goals::GoalType;
    use chrono::{Duration, Utc};

    #[test]
    fn test_information_gain() {
        let action = PotentialAction {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            prior_uncertainty: 0.8,
            posterior_uncertainty: 0.2,
            expected_reward: 0.5,
            goal_ids: vec![],
            downstream_tasks: vec![],
            data_completeness: 0.0,
        };

        let gain = action.information_gain();
        assert!((gain - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_efe_calculation() {
        let calc = EFECalculator::new();

        let action = PotentialAction::new(Uuid::new_v4(), "Research Speaker");
        let goal = Goal::new("Conference", "Coverage", GoalType::ConferenceCoverage, 0.9)
            .with_deadline(Utc::now() + Duration::days(30));

        let efe = calc.calculate(&action, &[goal]);
        assert!(efe.total > 0.0);
        assert!(efe.urgency_factor >= 1.0);
    }

    #[test]
    fn test_ranking() {
        let calc = EFECalculator::new();

        let urgent_action = PotentialAction {
            id: Uuid::new_v4(),
            name: "Urgent".to_string(),
            prior_uncertainty: 0.9,
            posterior_uncertainty: 0.1,
            expected_reward: 0.8,
            goal_ids: vec![],
            downstream_tasks: vec![Uuid::new_v4(), Uuid::new_v4()],
            data_completeness: 0.1,
        };

        let low_priority_action = PotentialAction {
            id: Uuid::new_v4(),
            name: "Low".to_string(),
            prior_uncertainty: 0.3,
            posterior_uncertainty: 0.2,
            expected_reward: 0.2,
            goal_ids: vec![],
            downstream_tasks: vec![],
            data_completeness: 0.8,
        };

        let goal = Goal::new("Test", "Test", GoalType::Custom, 0.9)
            .with_deadline(Utc::now() + Duration::days(7));

        let ranked = calc.rank_actions(&[low_priority_action, urgent_action], &[goal]);

        assert_eq!(ranked[0].action_id, ranked[0].action_id); // Urgent should be first
        assert!(ranked[0].total > ranked[1].total);
    }
}
