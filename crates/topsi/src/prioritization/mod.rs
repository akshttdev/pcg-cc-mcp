//! Active Inference Prioritization Module
//!
//! Implements Free Energy minimization for task prioritization.
//! Helps identify the best use of time to reach goals by calculating
//! expected free energy for each potential action.
//!
//! Key concepts:
//! - Goals: Desired end states with value functions
//! - Expected Free Energy (EFE): epistemic_value + pragmatic_value
//! - Urgency: Deadline proximity multiplier
//! - Dependency Impact: How many downstream tasks are blocked

pub mod free_energy;
pub mod goals;
pub mod knowledge;
pub mod priority_score;
pub mod recommender;

pub use free_energy::{EFECalculator, ExpectedFreeEnergy};
pub use goals::{Goal, GoalState, GoalType};
pub use knowledge::enrich_actions_with_knowledge;
pub use priority_score::{PriorityCalculator, PriorityLevel, PriorityScore};
pub use recommender::{PriorityRecommender, Recommendation, RecommendationBatch};
