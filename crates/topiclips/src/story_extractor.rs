//! Story Extractor - Extracts narrative themes from topology changes
//!
//! Queries topology changes and synthesizes them into narrative themes:
//! - Identifies significant events
//! - Determines narrative roles (Protagonist, Catalyst, Victim, Beneficiary)
//! - Computes primary theme and emotional arc

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use topsi::TopologyChange;
use uuid::Uuid;

/// Narrative roles for events in the story
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NarrativeRole {
    /// Main actor driving the story forward
    Protagonist,
    /// Event that triggers change
    Catalyst,
    /// Entity negatively affected
    Victim,
    /// Entity positively affected
    Beneficiary,
    /// Witness to the events
    Observer,
}

impl ToString for NarrativeRole {
    fn to_string(&self) -> String {
        match self {
            NarrativeRole::Protagonist => "protagonist".to_string(),
            NarrativeRole::Catalyst => "catalyst".to_string(),
            NarrativeRole::Victim => "victim".to_string(),
            NarrativeRole::Beneficiary => "beneficiary".to_string(),
            NarrativeRole::Observer => "observer".to_string(),
        }
    }
}

/// Primary theme of the story
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Growth,
    Struggle,
    Transformation,
    Connection,
    Loss,
}

impl ToString for Theme {
    fn to_string(&self) -> String {
        match self {
            Theme::Growth => "growth".to_string(),
            Theme::Struggle => "struggle".to_string(),
            Theme::Transformation => "transformation".to_string(),
            Theme::Connection => "connection".to_string(),
            Theme::Loss => "loss".to_string(),
        }
    }
}

/// Emotional arc of the story
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmotionalArc {
    Triumphant,
    Melancholic,
    Tense,
    Peaceful,
    Chaotic,
    Hopeful,
}

impl ToString for EmotionalArc {
    fn to_string(&self) -> String {
        match self {
            EmotionalArc::Triumphant => "triumphant".to_string(),
            EmotionalArc::Melancholic => "melancholic".to_string(),
            EmotionalArc::Tense => "tense".to_string(),
            EmotionalArc::Peaceful => "peaceful".to_string(),
            EmotionalArc::Chaotic => "chaotic".to_string(),
            EmotionalArc::Hopeful => "hopeful".to_string(),
        }
    }
}

/// Extracted narrative story from topology changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeStory {
    /// Events that make up the story
    pub events: Vec<TopologyChange>,
    /// Primary theme of the story
    pub primary_theme: String,
    /// Emotional arc
    pub emotional_arc: String,
    /// Human-readable narrative summary
    pub narrative_summary: String,
    /// Overall significance score (0.0-1.0)
    pub overall_significance: f64,
    /// Event role mappings
    pub event_roles: Vec<(usize, NarrativeRole)>,
}

impl NarrativeStory {
    /// Get the narrative role for a specific event
    pub fn get_narrative_role(&self, event: &TopologyChange) -> String {
        // Find the event in our list and return its role
        for (idx, e) in self.events.iter().enumerate() {
            if std::ptr::eq(e, event) {
                if let Some((_, role)) = self.event_roles.iter().find(|(i, _)| *i == idx) {
                    return role.to_string();
                }
            }
        }
        // Default role based on event type
        match event {
            TopologyChange::NodeAdded { .. } | TopologyChange::ClusterFormed { .. } => {
                "beneficiary".to_string()
            }
            TopologyChange::NodeRemoved { .. } | TopologyChange::ClusterDissolved { .. } => {
                "victim".to_string()
            }
            TopologyChange::RouteCreated { .. } | TopologyChange::RouteCompleted { .. } => {
                "protagonist".to_string()
            }
            TopologyChange::RouteFailed { .. } => "victim".to_string(),
            _ => "observer".to_string(),
        }
    }
}

/// Story extractor that queries topology and synthesizes narratives
pub struct StoryExtractor {
    pool: SqlitePool,
}

impl StoryExtractor {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Extract a narrative story from topology changes in the given time period
    pub async fn extract_story(
        &self,
        project_id: Uuid,
        period_start: Option<&str>,
        period_end: Option<&str>,
    ) -> Result<NarrativeStory> {
        // Determine time range - default to last 24 hours
        let end = period_end
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let start = period_start
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| end - Duration::hours(24));

        // Query topology changes from database
        let events = self
            .query_topology_changes(project_id, &start, &end)
            .await?;

        // If no events, create a minimal story
        if events.is_empty() {
            return Ok(NarrativeStory {
                events: vec![],
                primary_theme: Theme::Connection.to_string(), // Peaceful stability
                emotional_arc: EmotionalArc::Peaceful.to_string(),
                narrative_summary: "A quiet period of stability in the topology.".to_string(),
                overall_significance: 0.0,
                event_roles: vec![],
            });
        }

        // Analyze events to determine theme and arc
        let (theme, arc) = self.analyze_theme_and_arc(&events);
        let event_roles = self.assign_narrative_roles(&events);
        let significance = self.calculate_overall_significance(&events);
        let summary = self.generate_narrative_summary(&events, &theme, &arc);

        Ok(NarrativeStory {
            events,
            primary_theme: theme.to_string(),
            emotional_arc: arc.to_string(),
            narrative_summary: summary,
            overall_significance: significance,
            event_roles,
        })
    }

    /// Query topology changes from the database
    async fn query_topology_changes(
        &self,
        project_id: Uuid,
        start: &DateTime<Utc>,
        end: &DateTime<Utc>,
    ) -> Result<Vec<TopologyChange>> {
        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();

        // Query topology_nodes for changes
        let node_changes = sqlx::query!(
            r#"
            SELECT id, node_type, ref_id, status, created_at, updated_at
            FROM topology_nodes
            WHERE project_id = $1
              AND (created_at >= $2 OR updated_at >= $2)
              AND (created_at <= $3 OR updated_at <= $3)
            ORDER BY created_at ASC
            "#,
            project_id,
            start_str,
            end_str
        )
        .fetch_all(&self.pool)
        .await?;

        // Query topology_edges for changes
        let edge_changes = sqlx::query!(
            r#"
            SELECT id, from_node_id, to_node_id, edge_type, status, created_at
            FROM topology_edges
            WHERE project_id = $1
              AND created_at >= $2
              AND created_at <= $3
            ORDER BY created_at ASC
            "#,
            project_id,
            start_str,
            end_str
        )
        .fetch_all(&self.pool)
        .await?;

        // Query topology_clusters for changes
        let cluster_changes = sqlx::query!(
            r#"
            SELECT id, name, node_ids, is_active, formed_at, dissolved_at
            FROM topology_clusters
            WHERE project_id = $1
              AND (formed_at >= $2 OR dissolved_at >= $2)
              AND (formed_at <= $3 OR dissolved_at <= $3)
            ORDER BY formed_at ASC
            "#,
            project_id,
            start_str,
            end_str
        )
        .fetch_all(&self.pool)
        .await?;

        // Query topology_routes for changes
        let route_changes = sqlx::query!(
            r#"
            SELECT id, goal, path, status, created_at, completed_at
            FROM topology_routes
            WHERE project_id = $1
              AND created_at >= $2
              AND created_at <= $3
            ORDER BY created_at ASC
            "#,
            project_id,
            start_str,
            end_str
        )
        .fetch_all(&self.pool)
        .await?;

        // Convert to TopologyChange events
        let mut events = Vec::new();

        // Process node changes
        for node in node_changes {
            let node_id = Uuid::parse_str(&node.id)?;
            // If created within window, it's a new node
            if let Ok(created) = DateTime::parse_from_rfc3339(&node.created_at) {
                if created.with_timezone(&Utc) >= *start {
                    events.push(TopologyChange::NodeAdded {
                        node_id,
                        node_type: node.node_type,
                    });
                }
            }
        }

        // Process edge changes
        for edge in edge_changes {
            let edge_id = Uuid::parse_str(&edge.id)?;
            let from = Uuid::parse_str(&edge.from_node_id)?;
            let to = Uuid::parse_str(&edge.to_node_id)?;
            events.push(TopologyChange::EdgeAdded { edge_id, from, to });
        }

        // Process cluster changes
        for cluster in cluster_changes {
            let cluster_id = Uuid::parse_str(&cluster.id)?;
            // Parse node_ids JSON to count members
            let node_count: usize = serde_json::from_str::<Vec<String>>(&cluster.node_ids)
                .ok()
                .map(|v| v.len())
                .unwrap_or(0);

            if cluster.dissolved_at.is_some() {
                events.push(TopologyChange::ClusterDissolved { cluster_id });
            } else {
                events.push(TopologyChange::ClusterFormed {
                    cluster_id,
                    name: cluster.name.clone(),
                    node_count,
                });
            }
        }

        // Process route changes
        for route in route_changes {
            let route_id = Uuid::parse_str(&route.id)?;
            let path: Vec<String> = serde_json::from_str(&route.path).unwrap_or_default();

            match route.status.as_str() {
                "completed" => events.push(TopologyChange::RouteCompleted { route_id }),
                "failed" => events.push(TopologyChange::RouteFailed {
                    route_id,
                    reason: "Unknown".to_string(),
                }),
                _ => events.push(TopologyChange::RouteCreated {
                    route_id,
                    goal: route.goal,
                    path_length: path.len(),
                }),
            }
        }

        // Query issues that might indicate status changes
        let issues = sqlx::query!(
            r#"
            SELECT issue_type, severity, affected_nodes
            FROM topology_issues
            WHERE project_id = $1
              AND created_at >= $2
              AND created_at <= $3
              AND resolved_at IS NULL
            "#,
            project_id,
            start_str,
            end_str
        )
        .fetch_all(&self.pool)
        .await?;

        // Issues can indicate degraded status changes
        for issue in issues {
            if issue.issue_type == "bottleneck" || issue.issue_type == "degraded_path" {
                if let Some(affected) = issue.affected_nodes {
                    if let Ok(node_ids) = serde_json::from_str::<Vec<String>>(&affected) {
                        for node_id_str in node_ids.iter().take(1) {
                            if let Ok(node_id) = Uuid::parse_str(node_id_str) {
                                events.push(TopologyChange::NodeStatusChanged {
                                    node_id,
                                    old_status: "active".to_string(),
                                    new_status: "degraded".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(events)
    }

    /// Analyze events to determine the primary theme and emotional arc
    fn analyze_theme_and_arc(&self, events: &[TopologyChange]) -> (Theme, EmotionalArc) {
        let mut growth_score = 0;
        let mut loss_score = 0;
        let mut connection_score = 0;
        let mut struggle_score = 0;
        let mut transformation_score = 0;

        for event in events {
            match event {
                TopologyChange::NodeAdded { .. } => {
                    growth_score += 2;
                }
                TopologyChange::NodeRemoved { .. } => {
                    loss_score += 2;
                }
                TopologyChange::NodeStatusChanged { new_status, .. } => {
                    if new_status == "degraded" || new_status == "failed" {
                        struggle_score += 2;
                    } else {
                        transformation_score += 2;
                    }
                }
                TopologyChange::EdgeAdded { .. } => {
                    connection_score += 2;
                }
                TopologyChange::EdgeRemoved { .. } => {
                    loss_score += 1;
                    connection_score -= 1;
                }
                TopologyChange::EdgeStatusChanged { new_status, .. } => {
                    if new_status == "degraded" {
                        struggle_score += 1;
                    }
                }
                TopologyChange::ClusterFormed { node_count, .. } => {
                    connection_score += *node_count as i32;
                    growth_score += 1;
                }
                TopologyChange::ClusterDissolved { .. } => {
                    loss_score += 2;
                }
                TopologyChange::RouteCreated { path_length, .. } => {
                    connection_score += *path_length as i32 / 2;
                    growth_score += 1;
                }
                TopologyChange::RouteCompleted { .. } => {
                    transformation_score += 2;
                    growth_score += 1;
                }
                TopologyChange::RouteFailed { .. } => {
                    struggle_score += 2;
                }
            }
        }

        // Determine primary theme
        let theme = if struggle_score > growth_score
            && struggle_score > loss_score
            && struggle_score > connection_score
        {
            Theme::Struggle
        } else if loss_score > growth_score && loss_score > connection_score {
            Theme::Loss
        } else if connection_score > growth_score && connection_score > transformation_score {
            Theme::Connection
        } else if transformation_score > growth_score {
            Theme::Transformation
        } else {
            Theme::Growth
        };

        // Determine emotional arc
        let arc = match &theme {
            Theme::Growth => {
                if struggle_score > 0 {
                    EmotionalArc::Hopeful
                } else {
                    EmotionalArc::Triumphant
                }
            }
            Theme::Struggle => {
                if growth_score > loss_score {
                    EmotionalArc::Tense
                } else {
                    EmotionalArc::Chaotic
                }
            }
            Theme::Loss => {
                if transformation_score > 0 {
                    EmotionalArc::Melancholic
                } else {
                    EmotionalArc::Peaceful
                }
            }
            Theme::Connection => EmotionalArc::Peaceful,
            Theme::Transformation => {
                if struggle_score > 0 {
                    EmotionalArc::Hopeful
                } else {
                    EmotionalArc::Triumphant
                }
            }
        };

        (theme, arc)
    }

    /// Assign narrative roles to each event
    fn assign_narrative_roles(&self, events: &[TopologyChange]) -> Vec<(usize, NarrativeRole)> {
        let mut roles = Vec::new();
        let mut has_protagonist = false;

        for (idx, event) in events.iter().enumerate() {
            let role = match event {
                TopologyChange::RouteCreated { .. } | TopologyChange::RouteCompleted { .. } => {
                    if !has_protagonist {
                        has_protagonist = true;
                        NarrativeRole::Protagonist
                    } else {
                        NarrativeRole::Beneficiary
                    }
                }
                TopologyChange::ClusterFormed { .. } => {
                    if !has_protagonist {
                        has_protagonist = true;
                        NarrativeRole::Protagonist
                    } else {
                        NarrativeRole::Beneficiary
                    }
                }
                TopologyChange::NodeAdded { .. } | TopologyChange::EdgeAdded { .. } => {
                    NarrativeRole::Beneficiary
                }
                TopologyChange::NodeRemoved { .. }
                | TopologyChange::ClusterDissolved { .. }
                | TopologyChange::RouteFailed { .. } => NarrativeRole::Victim,
                TopologyChange::NodeStatusChanged { new_status, .. } => {
                    if new_status == "degraded" || new_status == "failed" {
                        NarrativeRole::Victim
                    } else {
                        NarrativeRole::Catalyst
                    }
                }
                TopologyChange::EdgeStatusChanged { .. } | TopologyChange::EdgeRemoved { .. } => {
                    NarrativeRole::Catalyst
                }
            };
            roles.push((idx, role));
        }

        roles
    }

    /// Calculate overall significance score
    fn calculate_overall_significance(&self, events: &[TopologyChange]) -> f64 {
        if events.is_empty() {
            return 0.0;
        }

        let total_weight: f64 = events
            .iter()
            .map(|e| match e {
                TopologyChange::ClusterFormed { node_count, .. } => 0.3 + (*node_count as f64 * 0.1),
                TopologyChange::ClusterDissolved { .. } => 0.4,
                TopologyChange::RouteCreated { path_length, .. } => {
                    0.2 + (*path_length as f64 * 0.05)
                }
                TopologyChange::RouteCompleted { .. } => 0.3,
                TopologyChange::RouteFailed { .. } => 0.5,
                TopologyChange::NodeStatusChanged { new_status, .. } => {
                    if new_status == "failed" {
                        0.6
                    } else {
                        0.3
                    }
                }
                TopologyChange::NodeAdded { .. } => 0.2,
                TopologyChange::NodeRemoved { .. } => 0.3,
                _ => 0.1,
            })
            .sum();

        // Normalize to 0-1 range, capping at 1.0
        (total_weight / events.len() as f64).min(1.0)
    }

    /// Generate a human-readable narrative summary
    fn generate_narrative_summary(
        &self,
        events: &[TopologyChange],
        theme: &Theme,
        arc: &EmotionalArc,
    ) -> String {
        let event_count = events.len();

        // Count event types
        let mut clusters_formed = 0;
        let mut clusters_dissolved = 0;
        let mut nodes_added = 0;
        let mut nodes_removed = 0;
        let mut routes_created = 0;
        let mut routes_completed = 0;
        let mut routes_failed = 0;

        for event in events {
            match event {
                TopologyChange::ClusterFormed { .. } => clusters_formed += 1,
                TopologyChange::ClusterDissolved { .. } => clusters_dissolved += 1,
                TopologyChange::NodeAdded { .. } => nodes_added += 1,
                TopologyChange::NodeRemoved { .. } => nodes_removed += 1,
                TopologyChange::RouteCreated { .. } => routes_created += 1,
                TopologyChange::RouteCompleted { .. } => routes_completed += 1,
                TopologyChange::RouteFailed { .. } => routes_failed += 1,
                _ => {}
            }
        }

        let mut parts = Vec::new();

        match theme {
            Theme::Growth => {
                parts.push(format!(
                    "A period of expansion with {} new developments",
                    event_count
                ));
                if nodes_added > 0 {
                    parts.push(format!("{} new entities emerged", nodes_added));
                }
                if clusters_formed > 0 {
                    parts.push(format!("{} new collaborations formed", clusters_formed));
                }
            }
            Theme::Struggle => {
                parts.push(format!(
                    "A challenging period marked by {} significant events",
                    event_count
                ));
                if routes_failed > 0 {
                    parts.push(format!("{} paths encountered obstacles", routes_failed));
                }
            }
            Theme::Transformation => {
                parts.push(format!(
                    "A transformative period with {} changes",
                    event_count
                ));
                if routes_completed > 0 {
                    parts.push(format!("{} journeys reached completion", routes_completed));
                }
            }
            Theme::Connection => {
                parts.push(format!(
                    "A period of unity with {} connections forming",
                    event_count
                ));
                if clusters_formed > 0 {
                    parts.push(format!(
                        "{} groups came together as one",
                        clusters_formed
                    ));
                }
            }
            Theme::Loss => {
                parts.push(format!(
                    "A reflective period with {} departures",
                    event_count
                ));
                if nodes_removed > 0 {
                    parts.push(format!("{} entities faded away", nodes_removed));
                }
                if clusters_dissolved > 0 {
                    parts.push(format!("{} groups dispersed", clusters_dissolved));
                }
            }
        }

        // Add emotional color
        let emotion_phrase = match arc {
            EmotionalArc::Triumphant => "culminating in triumph",
            EmotionalArc::Melancholic => "tinged with melancholy",
            EmotionalArc::Tense => "filled with tension",
            EmotionalArc::Peaceful => "in peaceful harmony",
            EmotionalArc::Chaotic => "amidst swirling chaos",
            EmotionalArc::Hopeful => "with hope on the horizon",
        };

        parts.push(emotion_phrase.to_string());

        parts.join(". ") + "."
    }
}
