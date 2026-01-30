//! Event Detector - Scores topology events for significance
//!
//! Determines which events are worth visualizing based on their
//! impact on the topology.

use topsi::TopologyChange;

/// Event detector that scores topology changes for significance
pub struct EventDetector {
    /// Minimum threshold for triggering clip generation
    pub threshold: f64,
}

impl EventDetector {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }

    /// Score a single event for significance (0.0 - 1.0)
    pub fn score_event(&self, event: &TopologyChange) -> f64 {
        match event {
            // Cluster events are highly significant
            TopologyChange::ClusterFormed { node_count, .. } => {
                // Base score + bonus for larger clusters
                0.6 + (*node_count as f64 * 0.05).min(0.3)
            }
            TopologyChange::ClusterDissolved { .. } => 0.7,

            // Route events indicate workflow activity
            TopologyChange::RouteCreated { path_length, .. } => {
                0.4 + (*path_length as f64 * 0.03).min(0.3)
            }
            TopologyChange::RouteCompleted { .. } => 0.5,
            TopologyChange::RouteFailed { .. } => 0.8,

            // Status changes indicate health issues
            TopologyChange::NodeStatusChanged { new_status, .. } => match new_status.as_str() {
                "failed" => 0.9,
                "degraded" => 0.7,
                "active" => 0.4, // Recovery is positive
                _ => 0.3,
            },
            TopologyChange::EdgeStatusChanged { new_status, .. } => match new_status.as_str() {
                "degraded" => 0.6,
                "inactive" => 0.5,
                _ => 0.3,
            },

            // Node additions/removals
            TopologyChange::NodeAdded { node_type, .. } => match node_type.as_str() {
                "agent" => 0.5,
                "workflow" => 0.4,
                _ => 0.3,
            },
            TopologyChange::NodeRemoved { .. } => 0.4,

            // Edge changes
            TopologyChange::EdgeAdded { .. } => 0.3,
            TopologyChange::EdgeRemoved { .. } => 0.35,
        }
    }

    /// Score a batch of events and return their aggregate significance
    pub fn score_batch(&self, events: &[TopologyChange]) -> f64 {
        if events.is_empty() {
            return 0.0;
        }

        // Calculate weighted average with diversity bonus
        let total_score: f64 = events.iter().map(|e| self.score_event(e)).sum();
        let avg_score = total_score / events.len() as f64;

        // Count unique event types for diversity bonus
        let mut event_types = events
            .iter()
            .map(|e| std::mem::discriminant(e))
            .collect::<Vec<_>>();
        event_types.sort_by_key(|d| format!("{:?}", d));
        event_types.dedup();
        let diversity = event_types.len() as f64 / 11.0; // 11 event types

        // Combine average score with diversity bonus
        (avg_score * 0.7 + diversity * 0.3).min(1.0)
    }

    /// Check if events meet significance threshold for auto-generation
    pub fn meets_threshold(&self, events: &[TopologyChange]) -> bool {
        self.score_batch(events) >= self.threshold
    }

    /// Filter events to only significant ones
    pub fn filter_significant<'a>(&self, events: &'a [TopologyChange]) -> Vec<&'a TopologyChange> {
        events
            .iter()
            .filter(|e| self.score_event(e) >= self.threshold)
            .collect()
    }

    /// Rank events by significance (most significant first)
    pub fn rank_events<'a>(&self, events: &'a [TopologyChange]) -> Vec<(f64, &'a TopologyChange)> {
        let mut ranked: Vec<_> = events.iter().map(|e| (self.score_event(e), e)).collect();
        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    /// Get top N most significant events
    pub fn top_events<'a>(
        &self,
        events: &'a [TopologyChange],
        n: usize,
    ) -> Vec<&'a TopologyChange> {
        self.rank_events(events)
            .into_iter()
            .take(n)
            .map(|(_, e)| e)
            .collect()
    }

    /// Categorize events by significance level
    pub fn categorize<'a>(&self, events: &'a [TopologyChange]) -> EventCategories<'a> {
        let mut critical = Vec::new();
        let mut high = Vec::new();
        let mut medium = Vec::new();
        let mut low = Vec::new();

        for event in events {
            let score = self.score_event(event);
            if score >= 0.8 {
                critical.push(event);
            } else if score >= 0.6 {
                high.push(event);
            } else if score >= 0.4 {
                medium.push(event);
            } else {
                low.push(event);
            }
        }

        EventCategories {
            critical,
            high,
            medium,
            low,
        }
    }
}

/// Categorized events by significance level
pub struct EventCategories<'a> {
    /// Critical events (0.8+) - must be visualized
    pub critical: Vec<&'a TopologyChange>,
    /// High significance (0.6-0.8)
    pub high: Vec<&'a TopologyChange>,
    /// Medium significance (0.4-0.6)
    pub medium: Vec<&'a TopologyChange>,
    /// Low significance (<0.4)
    pub low: Vec<&'a TopologyChange>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_score_cluster_formed() {
        let detector = EventDetector::new(0.3);
        let event = TopologyChange::ClusterFormed {
            cluster_id: Uuid::new_v4(),
            name: "Test Cluster".to_string(),
            node_count: 5,
        };
        let score = detector.score_event(&event);
        assert!(score >= 0.6);
        assert!(score <= 0.9);
    }

    #[test]
    fn test_score_route_failed() {
        let detector = EventDetector::new(0.3);
        let event = TopologyChange::RouteFailed {
            route_id: Uuid::new_v4(),
            reason: "Timeout".to_string(),
        };
        let score = detector.score_event(&event);
        assert_eq!(score, 0.8);
    }

    #[test]
    fn test_meets_threshold() {
        let detector = EventDetector::new(0.5);
        let events = vec![
            TopologyChange::NodeAdded {
                node_id: Uuid::new_v4(),
                node_type: "agent".to_string(),
            },
            TopologyChange::ClusterFormed {
                cluster_id: Uuid::new_v4(),
                name: "Team".to_string(),
                node_count: 3,
            },
        ];
        assert!(detector.meets_threshold(&events));
    }

    #[test]
    fn test_ranking() {
        let detector = EventDetector::new(0.3);
        let events = vec![
            TopologyChange::EdgeAdded {
                edge_id: Uuid::new_v4(),
                from: Uuid::new_v4(),
                to: Uuid::new_v4(),
            },
            TopologyChange::RouteFailed {
                route_id: Uuid::new_v4(),
                reason: "Error".to_string(),
            },
            TopologyChange::NodeAdded {
                node_id: Uuid::new_v4(),
                node_type: "task".to_string(),
            },
        ];

        let ranked = detector.rank_events(&events);
        // Route failed should be first (highest score)
        assert!(matches!(ranked[0].1, TopologyChange::RouteFailed { .. }));
    }
}
