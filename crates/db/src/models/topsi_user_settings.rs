use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row, SqlitePool};
use ts_rs::TS;

/// Confirmation mode for Topsi tool execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationMode {
    /// Always ask for confirmation before executing
    AlwaysConfirm,
    /// Only confirm destructive (Red) tools; execute Yellow/Green immediately
    ConfirmDestructive,
    /// Execute all tools without confirmation
    Autonomous,
}

impl ConfirmationMode {
    /// Restrictiveness ordering (higher = more restrictive)
    pub fn restrictiveness(&self) -> u8 {
        match self {
            Self::Autonomous => 0,
            Self::ConfirmDestructive => 1,
            Self::AlwaysConfirm => 2,
        }
    }

    /// Return the more restrictive of two modes
    pub fn most_restrictive(a: &Self, b: &Self) -> Self {
        if a.restrictiveness() >= b.restrictiveness() {
            a.clone()
        } else {
            b.clone()
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "always_confirm" => Self::AlwaysConfirm,
            "autonomous" => Self::Autonomous,
            _ => Self::ConfirmDestructive,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlwaysConfirm => "always_confirm",
            Self::ConfirmDestructive => "confirm_destructive",
            Self::Autonomous => "autonomous",
        }
    }
}

/// Tool risk classification
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRisk {
    /// Read-only tools — never need confirmation
    Green,
    /// Create/update tools — confirmable via settings
    Yellow,
    /// Delete/bulk tools — confirm by default
    Red,
}

/// Classify a tool by its risk level
pub fn classify_tool_risk(tool_name: &str) -> ToolRisk {
    match tool_name {
        // Red: destructive / bulk operations
        "delete_task" | "bulk_update_tasks" => ToolRisk::Red,

        // Yellow: create / update single entities
        "create_project"
        | "update_project"
        | "create_task"
        | "update_task"
        | "start_task_execution"
        | "create_crm_contact"
        | "create_crm_deal"
        | "update_crm_deal"
        | "approve_staged_records"
        | "build_workflow" => ToolRisk::Yellow,

        // Green: everything else (reads, searches, topology queries)
        _ => ToolRisk::Green,
    }
}

/// Per-user settings for Topsi confirmation behavior
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TopsiUserSettings {
    pub user_id: String,
    pub default_confirmation_mode: String,
    pub per_tool_overrides: Option<String>,
    pub auto_approve_timeout_minutes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TopsiUserSettings {
    /// Get settings for a user, returning defaults if none saved
    pub async fn get_or_default(pool: &SqlitePool, user_id: &str) -> Self {
        let row = sqlx::query(
            "SELECT user_id, default_confirmation_mode, per_tool_overrides, auto_approve_timeout_minutes, created_at, updated_at FROM topsi_user_settings WHERE user_id = ?"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        match row {
            Some(r) => Self {
                user_id: r.get("user_id"),
                default_confirmation_mode: r.get("default_confirmation_mode"),
                per_tool_overrides: r.get("per_tool_overrides"),
                auto_approve_timeout_minutes: r.get("auto_approve_timeout_minutes"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            },
            None => Self {
                user_id: user_id.to_string(),
                default_confirmation_mode: "confirm_destructive".to_string(),
                per_tool_overrides: None,
                auto_approve_timeout_minutes: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        }
    }

    /// Upsert user settings
    pub async fn upsert(
        pool: &SqlitePool,
        user_id: &str,
        mode: &str,
        per_tool_overrides: Option<&str>,
        auto_approve_timeout_minutes: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO topsi_user_settings (user_id, default_confirmation_mode, per_tool_overrides, auto_approve_timeout_minutes, updated_at)
               VALUES (?, ?, ?, ?, datetime('now','subsec'))
               ON CONFLICT(user_id) DO UPDATE SET
                 default_confirmation_mode = excluded.default_confirmation_mode,
                 per_tool_overrides = excluded.per_tool_overrides,
                 auto_approve_timeout_minutes = excluded.auto_approve_timeout_minutes,
                 updated_at = excluded.updated_at"#,
        )
        .bind(user_id)
        .bind(mode)
        .bind(per_tool_overrides)
        .bind(auto_approve_timeout_minutes)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get the effective confirmation mode for a specific tool
    pub fn confirmation_mode_for_tool(&self, tool_name: &str) -> ConfirmationMode {
        // Check per-tool override first
        if let Some(ref overrides_json) = self.per_tool_overrides {
            if let Ok(overrides) = serde_json::from_str::<HashMap<String, String>>(overrides_json) {
                if let Some(mode_str) = overrides.get(tool_name) {
                    return ConfirmationMode::from_str(mode_str);
                }
            }
        }

        // Fall back to default mode
        let default_mode = ConfirmationMode::from_str(&self.default_confirmation_mode);
        let risk = classify_tool_risk(tool_name);

        match (&default_mode, &risk) {
            // Green tools never need confirmation regardless of mode
            (_, ToolRisk::Green) => ConfirmationMode::Autonomous,
            // Autonomous mode skips all confirmations
            (ConfirmationMode::Autonomous, _) => ConfirmationMode::Autonomous,
            // Always confirm = confirm everything Yellow and Red
            (ConfirmationMode::AlwaysConfirm, _) => ConfirmationMode::AlwaysConfirm,
            // Confirm destructive = only confirm Red tools
            (ConfirmationMode::ConfirmDestructive, ToolRisk::Red) => {
                ConfirmationMode::AlwaysConfirm
            }
            (ConfirmationMode::ConfirmDestructive, ToolRisk::Yellow) => {
                ConfirmationMode::Autonomous
            }
        }
    }

    /// Check if a tool requires confirmation given current settings
    pub fn requires_confirmation(&self, tool_name: &str) -> bool {
        matches!(
            self.confirmation_mode_for_tool(tool_name),
            ConfirmationMode::AlwaysConfirm
        )
    }
}
