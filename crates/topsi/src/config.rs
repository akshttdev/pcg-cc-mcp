//! Configuration for Topsi

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Autonomy level for Topsi operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    /// Full autonomy - can make all decisions
    Full,
    /// Supervised - reports decisions but can act
    Supervised,
    /// Approval required for major actions
    ApprovalRequired,
    /// Manual - requires explicit approval for all actions
    Manual,
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        Self::Supervised
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LLMConfig {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub api_key_env: Option<String>,
}

impl Default for LLMConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: "gpt-4o-mini".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            api_key_env: None,
        }
    }
}

/// Core configuration for Topsi
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TopsiConfig {
    /// Name of this Topsi instance
    pub name: String,
    /// System prompt for LLM interactions
    pub system_prompt: Option<String>,
    /// LLM configuration
    #[serde(default)]
    pub llm: LLMConfig,
    /// Autonomy level
    #[serde(default)]
    pub autonomy_level: AutonomyLevel,
    /// Whether to auto-refresh topology
    #[serde(default = "default_true")]
    pub topology_auto_refresh: bool,
    /// Refresh interval in seconds
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    /// Enabled tools
    #[serde(default)]
    pub enabled_tools: Vec<String>,
    /// Maximum nodes to include in context
    #[serde(default = "default_max_context_nodes")]
    pub max_context_nodes: usize,
    /// Whether to detect patterns automatically
    #[serde(default = "default_true")]
    pub auto_pattern_detection: bool,
    /// Whether to auto-reroute on failures
    #[serde(default = "default_true")]
    pub auto_reroute: bool,
}

fn default_true() -> bool {
    true
}

fn default_refresh_interval() -> u64 {
    60
}

fn default_max_context_nodes() -> usize {
    50
}

impl Default for TopsiConfig {
    fn default() -> Self {
        Self {
            name: "Topsi".to_string(),
            system_prompt: None,
            llm: LLMConfig::default(),
            autonomy_level: AutonomyLevel::default(),
            topology_auto_refresh: true,
            refresh_interval_secs: 60,
            enabled_tools: vec![
                "view_topology".to_string(),
                "find_path".to_string(),
                "detect_bottlenecks".to_string(),
                "detect_holes".to_string(),
                "list_clusters".to_string(),
                "form_team".to_string(),
                "dissolve_team".to_string(),
                "add_connection".to_string(),
                "remove_connection".to_string(),
                "reroute".to_string(),
                "create_task".to_string(),
                "route_task".to_string(),
                "delegate_via_route".to_string(),
                "execute_workflow".to_string(),
            ],
            max_context_nodes: 50,
            auto_pattern_detection: true,
            auto_reroute: true,
        }
    }
}

impl TopsiConfig {
    /// Create a minimal configuration
    pub fn minimal() -> Self {
        Self {
            name: "Topsi".to_string(),
            system_prompt: None,
            llm: LLMConfig::default(),
            autonomy_level: AutonomyLevel::Manual,
            topology_auto_refresh: false,
            refresh_interval_secs: 300,
            enabled_tools: vec![
                "view_topology".to_string(),
                "find_path".to_string(),
            ],
            max_context_nodes: 20,
            auto_pattern_detection: false,
            auto_reroute: false,
        }
    }

    /// Create a fully autonomous configuration
    pub fn autonomous() -> Self {
        Self {
            autonomy_level: AutonomyLevel::Full,
            ..Default::default()
        }
    }

    /// Check if a tool is enabled
    pub fn is_tool_enabled(&self, tool_name: &str) -> bool {
        self.enabled_tools.is_empty() || self.enabled_tools.iter().any(|t| t == tool_name)
    }
}
