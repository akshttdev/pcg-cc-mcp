use anyhow::Error;
use executors::{executors::BaseCodingAgent, profile::ExecutorProfileId};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub use v9::{
    AptosWalletConfig, EditorConfig, EditorType, GitHubConfig, NotificationConfig, SoundFile,
    ThemeMode, TrelloConfig, UiLanguage,
};

use crate::services::config::versions::v9;

/// Airtable integration configuration
#[derive(Clone, Debug, Serialize, Deserialize, TS, Default)]
pub struct AirtableConfig {
    /// Airtable Personal Access Token
    pub token: Option<String>,
    /// Cached Airtable user email for display
    pub user_email: Option<String>,
    /// Sync deliverables as comments on Airtable records
    #[serde(default = "default_true")]
    pub sync_deliverables_as_comments: bool,
    /// Auto-import new records when syncing
    #[serde(default)]
    pub auto_import_new_records: bool,
}

fn default_true() -> bool {
    true
}

impl AirtableConfig {
    /// Check if Airtable is configured with a valid token
    pub fn is_configured(&self) -> bool {
        self.token.is_some()
    }

    /// Get the token if configured
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct Config {
    pub config_version: String,
    pub theme: ThemeMode,
    pub executor_profile: ExecutorProfileId,
    pub disclaimer_acknowledged: bool,
    pub onboarding_acknowledged: bool,
    pub github_login_acknowledged: bool,
    pub telemetry_acknowledged: bool,
    pub notifications: NotificationConfig,
    pub editor: EditorConfig,
    pub github: GitHubConfig,
    pub analytics_enabled: Option<bool>,
    pub workspace_dir: Option<String>,
    pub last_app_version: Option<String>,
    pub show_release_notes: bool,
    #[serde(default)]
    pub language: UiLanguage,
    #[serde(default)]
    pub aptos_wallet: AptosWalletConfig,
    #[serde(default)]
    pub trello: TrelloConfig,
    #[serde(default)]
    pub airtable: AirtableConfig,
}

impl Config {
    pub fn from_previous_version(raw_config: &str) -> Result<Self, Error> {
        let old_config = match serde_json::from_str::<v9::Config>(raw_config) {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::error!("❌ Failed to parse config: {}", e);
                tracing::error!("   at line {}, column {}", e.line(), e.column());
                return Err(e.into());
            }
        };

        Ok(Self {
            config_version: "v10".to_string(),
            theme: old_config.theme,
            executor_profile: old_config.executor_profile,
            disclaimer_acknowledged: old_config.disclaimer_acknowledged,
            onboarding_acknowledged: old_config.onboarding_acknowledged,
            github_login_acknowledged: old_config.github_login_acknowledged,
            telemetry_acknowledged: old_config.telemetry_acknowledged,
            notifications: old_config.notifications,
            editor: old_config.editor,
            github: old_config.github,
            analytics_enabled: old_config.analytics_enabled,
            workspace_dir: old_config.workspace_dir,
            last_app_version: old_config.last_app_version,
            show_release_notes: old_config.show_release_notes,
            language: old_config.language,
            aptos_wallet: old_config.aptos_wallet,
            trello: old_config.trello,
            airtable: AirtableConfig::default(),
        })
    }
}

impl From<String> for Config {
    fn from(raw_config: String) -> Self {
        if let Ok(config) = serde_json::from_str::<Config>(&raw_config)
            && config.config_version == "v10"
        {
            return config;
        }

        match Self::from_previous_version(&raw_config) {
            Ok(config) => {
                tracing::info!("Config upgraded to v10");
                config
            }
            Err(e) => {
                tracing::warn!("Config migration failed: {}, using default", e);
                Self::default()
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            config_version: "v10".to_string(),
            theme: ThemeMode::System,
            executor_profile: ExecutorProfileId::new(BaseCodingAgent::ClaudeCode),
            disclaimer_acknowledged: false,
            onboarding_acknowledged: false,
            github_login_acknowledged: false,
            telemetry_acknowledged: false,
            notifications: NotificationConfig::default(),
            editor: EditorConfig::default(),
            github: GitHubConfig::default(),
            analytics_enabled: None,
            workspace_dir: None,
            last_app_version: None,
            show_release_notes: false,
            language: UiLanguage::default(),
            aptos_wallet: AptosWalletConfig::default(),
            trello: TrelloConfig::default(),
            airtable: AirtableConfig::default(),
        }
    }
}
