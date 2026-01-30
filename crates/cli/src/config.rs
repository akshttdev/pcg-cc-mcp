//! Configuration management for PCG CLI
//!
//! Handles loading and saving configuration from ~/.pcg/config.toml

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Configuration for PCG CLI
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,

    #[serde(default)]
    pub session: SessionConfig,

    #[serde(default)]
    pub display: DisplayConfig,

    #[serde(default)]
    pub agents: AgentsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_server_url")]
    pub url: String,

    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_server_url() -> String {
    "http://localhost:3002".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            url: default_server_url(),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "default_true")]
    pub auto_create_tasks: bool,

    #[serde(default)]
    pub default_project: Option<String>,

    #[serde(default = "default_true")]
    pub auto_save_history: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            auto_create_tasks: true,
            default_project: None,
            auto_save_history: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_true")]
    pub show_cost_bar: bool,

    #[serde(default = "default_true")]
    pub markdown_rendering: bool,

    #[serde(default = "default_true")]
    pub syntax_highlighting: bool,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            show_cost_bar: true,
            markdown_rendering: true,
            syntax_highlighting: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    #[serde(default = "default_agent")]
    pub default: String,

    #[serde(default = "default_true")]
    pub duck_enabled: bool,

    #[serde(default = "default_true")]
    pub nora_enabled: bool,

    #[serde(default = "default_true")]
    pub scout_enabled: bool,
}

fn default_agent() -> String {
    "duck".to_string()
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            default: default_agent(),
            duck_enabled: true,
            nora_enabled: true,
            scout_enabled: true,
        }
    }
}

impl Config {
    /// Get the path to the config file
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".pcg")
            .join("config.toml")
    }

    /// Load configuration from file, or return defaults if not found
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get a configuration value by key path (e.g., "server.url")
    pub fn get(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();

        match parts.as_slice() {
            ["server", "url"] => Some(self.server.url.clone()),
            ["server", "api_key"] => self.server.api_key.clone(),
            ["session", "auto_create_tasks"] => Some(self.session.auto_create_tasks.to_string()),
            ["session", "default_project"] => self.session.default_project.clone(),
            ["display", "theme"] => Some(self.display.theme.clone()),
            ["display", "show_cost_bar"] => Some(self.display.show_cost_bar.to_string()),
            ["agents", "default"] => Some(self.agents.default.clone()),
            _ => None,
        }
    }

    /// Set a configuration value by key path
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();

        match parts.as_slice() {
            ["server", "url"] => self.server.url = value.to_string(),
            ["server", "api_key"] => self.server.api_key = Some(value.to_string()),
            ["session", "auto_create_tasks"] => {
                self.session.auto_create_tasks = value.parse().unwrap_or(true)
            }
            ["session", "default_project"] => {
                self.session.default_project = Some(value.to_string())
            }
            ["display", "theme"] => self.display.theme = value.to_string(),
            ["display", "show_cost_bar"] => {
                self.display.show_cost_bar = value.parse().unwrap_or(true)
            }
            ["agents", "default"] => self.agents.default = value.to_string(),
            _ => anyhow::bail!("Unknown configuration key: {}", key),
        }

        self.save()?;
        Ok(())
    }
}
