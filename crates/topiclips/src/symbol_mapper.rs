//! Symbol Mapper - Maps topology events to artistic symbols
//!
//! Uses the symbol library to translate raw topology events into
//! rich visual metaphors for artistic interpretation.

use anyhow::Result;
use db::models::topiclip::TopiClipSymbol;
use regex::Regex;
use sqlx::SqlitePool;
use topsi::TopologyChange;

/// Symbol mapper that translates events to artistic symbols
pub struct SymbolMapper {
    pool: SqlitePool,
}

impl SymbolMapper {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Load symbols from database
    async fn load_symbols(&self) -> Result<Vec<TopiClipSymbol>> {
        let symbols = TopiClipSymbol::list_all(&self.pool).await?;
        Ok(symbols)
    }

    /// Map a topology event to its corresponding artistic symbol
    pub async fn map_event(&self, event: &TopologyChange) -> Result<Option<TopiClipSymbol>> {
        // Load symbols fresh (could be cached with RwLock if performance is an issue)
        let symbols = self.load_symbols().await?;

        // Get the event type string for matching
        let event_type = Self::get_event_type_static(event);

        // Try to find a matching symbol
        for symbol in &symbols {
            if Self::matches_pattern_static(&symbol.event_pattern, &event_type, event) {
                // Clone and customize the symbol with event-specific data
                let mut matched = symbol.clone();
                matched.prompt_template = Self::customize_prompt_static(&matched.prompt_template, event);
                return Ok(Some(matched));
            }
        }

        Ok(None)
    }

    /// Map multiple events and return their symbols
    pub async fn map_events(
        &self,
        events: &[TopologyChange],
    ) -> Result<Vec<(TopologyChange, Option<TopiClipSymbol>)>> {
        // Load symbols once for all events
        let symbols = self.load_symbols().await?;
        let mut results = Vec::new();

        for event in events {
            let event_type = Self::get_event_type_static(event);
            let mut matched_symbol = None;

            for symbol in &symbols {
                if Self::matches_pattern_static(&symbol.event_pattern, &event_type, event) {
                    let mut matched = symbol.clone();
                    matched.prompt_template = Self::customize_prompt_static(&matched.prompt_template, event);
                    matched_symbol = Some(matched);
                    break;
                }
            }

            results.push((event.clone(), matched_symbol));
        }
        Ok(results)
    }

    /// Get the event type string for an event (static version)
    fn get_event_type_static(event: &TopologyChange) -> String {
        match event {
            TopologyChange::NodeAdded { node_type, .. } => format!("NodeAdded:{}", node_type),
            TopologyChange::NodeRemoved { .. } => "NodeRemoved".to_string(),
            TopologyChange::NodeStatusChanged { new_status, .. } => {
                format!("NodeStatusChanged:{}", new_status)
            }
            TopologyChange::EdgeAdded { .. } => "EdgeAdded".to_string(),
            TopologyChange::EdgeRemoved { .. } => "EdgeRemoved".to_string(),
            TopologyChange::EdgeStatusChanged { new_status, .. } => {
                format!("EdgeStatusChanged:{}", new_status)
            }
            TopologyChange::ClusterFormed { .. } => "ClusterFormed".to_string(),
            TopologyChange::ClusterDissolved { .. } => "ClusterDissolved".to_string(),
            TopologyChange::RouteCreated { .. } => "RouteCreated".to_string(),
            TopologyChange::RouteCompleted { .. } => "RouteCompleted".to_string(),
            TopologyChange::RouteFailed { .. } => "RouteFailed".to_string(),
        }
    }

    /// Check if an event matches a pattern (static version)
    fn matches_pattern_static(pattern: &str, event_type: &str, event: &TopologyChange) -> bool {
        // First try exact match
        if pattern == event_type {
            return true;
        }

        // Try base event type match (without subtype)
        let base_type = event_type.split(':').next().unwrap_or(event_type);
        if pattern == base_type {
            return true;
        }

        // Special pattern matching for certain event types
        match event {
            TopologyChange::NodeStatusChanged { new_status, .. } => {
                if pattern == "bottleneck" && new_status == "degraded" {
                    return true;
                }
                if pattern == "HealthImproved" && new_status == "active" {
                    return true;
                }
            }
            _ => {}
        }

        // Try regex pattern matching
        if let Ok(re) = Regex::new(pattern) {
            return re.is_match(event_type);
        }

        false
    }

    /// Customize prompt template with event-specific data (static version)
    fn customize_prompt_static(template: &str, event: &TopologyChange) -> String {
        let mut prompt = template.to_string();

        // Replace placeholders with actual values
        match event {
            TopologyChange::ClusterFormed { node_count, name, .. } => {
                prompt = prompt.replace("{count}", &node_count.to_string());
                prompt = prompt.replace("{name}", name);
            }
            TopologyChange::RouteCreated {
                path_length, goal, ..
            } => {
                prompt = prompt.replace("{count}", &path_length.to_string());
                prompt = prompt.replace("{goal}", goal);
            }
            TopologyChange::NodeAdded { node_type, .. } => {
                prompt = prompt.replace("{type}", node_type);
            }
            TopologyChange::RouteFailed { reason, .. } => {
                prompt = prompt.replace("{reason}", reason);
            }
            _ => {}
        }

        prompt
    }

    /// Get all symbols for a given theme
    pub async fn get_symbols_for_theme(&self, theme: &str) -> Result<Vec<TopiClipSymbol>> {
        let symbols = self.load_symbols().await?;
        Ok(symbols
            .into_iter()
            .filter(|s| s.theme_affinity.as_ref().map(|t| t == theme).unwrap_or(false))
            .collect())
    }

    /// Combine multiple symbols into a cohesive prompt
    pub fn combine_symbol_prompts(&self, symbols: &[TopiClipSymbol]) -> String {
        if symbols.is_empty() {
            return "Abstract geometric forms float in ethereal void".to_string();
        }

        if symbols.len() == 1 {
            return symbols[0].prompt_template.clone();
        }

        // Combine prompts with transitions
        let mut combined = Vec::new();
        let transitions = [
            "Meanwhile, ",
            "In the distance, ",
            "Above them, ",
            "Below, ",
            "Beside this, ",
        ];

        for (i, symbol) in symbols.iter().enumerate() {
            if i == 0 {
                combined.push(symbol.prompt_template.clone());
            } else {
                let transition = transitions[i % transitions.len()];
                // Take first sentence of subsequent prompts
                let prompt_part = symbol
                    .prompt_template
                    .split('.')
                    .next()
                    .unwrap_or(&symbol.prompt_template);
                combined.push(format!("{}{}", transition, prompt_part.to_lowercase()));
            }
        }

        combined.join(". ")
    }

    /// Get motion types for symbols to inform animation
    pub fn get_motion_types(&self, symbols: &[TopiClipSymbol]) -> Vec<String> {
        symbols
            .iter()
            .filter_map(|s| s.motion_type.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        // Test exact match
        assert!(SymbolMapper::matches_pattern_static(
            "ClusterFormed",
            "ClusterFormed",
            &TopologyChange::ClusterFormed {
                cluster_id: uuid::Uuid::new_v4(),
                name: "test".to_string(),
                node_count: 3,
            }
        ));

        // Test base type match
        assert!(SymbolMapper::matches_pattern_static(
            "NodeStatusChanged",
            "NodeStatusChanged:degraded",
            &TopologyChange::NodeStatusChanged {
                node_id: uuid::Uuid::new_v4(),
                old_status: "active".to_string(),
                new_status: "degraded".to_string(),
            }
        ));
    }

    #[test]
    fn test_prompt_customization() {
        let template = "{count} luminous figures rise";
        let event = TopologyChange::ClusterFormed {
            cluster_id: uuid::Uuid::new_v4(),
            name: "Creative Team".to_string(),
            node_count: 5,
        };

        let result = SymbolMapper::customize_prompt_static(template, &event);
        assert_eq!(result, "5 luminous figures rise");
    }
}
