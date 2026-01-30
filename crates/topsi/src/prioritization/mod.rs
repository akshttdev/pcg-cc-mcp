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

pub mod goals;
pub mod free_energy;
pub mod priority_score;
pub mod recommender;

pub use goals::{Goal, GoalState, GoalType};
pub use free_energy::{ExpectedFreeEnergy, EFECalculator};
pub use priority_score::{PriorityScore, PriorityCalculator, PriorityLevel};
pub use recommender::{Recommendation, PriorityRecommender};
