//! ORCHA Routing Layer
//!
//! Maps authenticated users to their Topsi instances based on device availability.
//! Implements federated routing where:
//! - Each user has a primary device hosting their Topsi database
//! - Secondary devices access the primary via APN
//! - Fallback to APN Cloud when primary is offline (e.g., Sirak's laptop)
//! - Multi-device orchestration for users with multiple compute nodes (e.g., admin)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use sqlx::SqlitePool;

// ============================================================================
// Configuration Structures
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct OrchaConfig {
    pub orcha: OrchaInfo,
    pub apn: ApnConfig,
    pub users: Vec<UserConfig>,
    pub devices: Vec<DeviceConfig>,
    pub routing: RoutingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrchaInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApnConfig {
    pub relay_url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserConfig {
    pub username: String,
    pub primary_device: String,
    #[serde(default)]
    pub secondary_devices: Vec<String>,
    pub fallback_device: Option<String>,
    pub topsi_db_path: String,
    pub cloud_backup_path: Option<String>,
    pub projects_path: String,
    pub uptime_guarantee: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceConfig {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub owner: String,
    pub apn_node_id: String,
    pub serves_data: bool,
    #[serde(default)]
    pub is_storage_provider: bool,
    #[serde(default)]
    pub hardware: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoutingConfig {
    pub default_strategy: String,
    pub fallback_strategy: String,
    pub multi_device_orchestration: bool,
}

// ============================================================================
// Routing Resolution
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct TopsiRoute {
    /// User being routed
    pub username: String,

    /// Primary device ID
    pub primary_device_id: String,

    /// Device currently serving (may be fallback)
    pub serving_device_id: String,

    /// Topsi database path
    pub topsi_db_path: PathBuf,

    /// Whether using fallback device
    pub using_fallback: bool,

    /// Secondary devices for multi-device orchestration
    pub secondary_devices: Vec<String>,

    /// Projects directory
    pub projects_path: PathBuf,
}

pub struct OrchaRouter {
    config: OrchaConfig,
    user_map: HashMap<String, UserConfig>,
    device_map: HashMap<String, DeviceConfig>,
}

impl OrchaRouter {
    /// Load ORCHA configuration from file
    pub fn from_file(path: &str) -> Result<Self> {
        let config_str = std::fs::read_to_string(path)
            .context("Failed to read ORCHA config file")?;

        let config: OrchaConfig = toml::from_str(&config_str)
            .context("Failed to parse ORCHA config")?;

        // Build lookup maps
        let user_map: HashMap<String, UserConfig> = config.users.iter()
            .map(|u| (u.username.clone(), u.clone()))
            .collect();

        let device_map: HashMap<String, DeviceConfig> = config.devices.iter()
            .map(|d| (d.id.clone(), d.clone()))
            .collect();

        Ok(Self {
            config,
            user_map,
            device_map,
        })
    }

    /// Route user to their Topsi instance
    pub async fn route_user(&self, username: &str, pool: &SqlitePool) -> Result<TopsiRoute> {
        let user_config = self.user_map.get(username)
            .context(format!("User '{}' not found in ORCHA config", username))?;

        // Check if primary device is online
        let primary_online = self.is_device_online(pool, &user_config.primary_device).await?;

        let (serving_device_id, topsi_db_path, using_fallback) = if primary_online {
            // Use primary device
            (
                user_config.primary_device.clone(),
                PathBuf::from(&user_config.topsi_db_path),
                false
            )
        } else if let Some(fallback_id) = &user_config.fallback_device {
            // Use fallback device (e.g., APN Cloud for Sirak)
            let fallback_online = self.is_device_online(pool, fallback_id).await?;

            if fallback_online {
                let fallback_path = user_config.cloud_backup_path.clone()
                    .unwrap_or_else(|| user_config.topsi_db_path.clone());

                tracing::info!(
                    "Routing user '{}' to fallback device '{}' (primary offline)",
                    username, fallback_id
                );

                (
                    fallback_id.clone(),
                    PathBuf::from(fallback_path),
                    true
                )
            } else {
                anyhow::bail!(
                    "Both primary and fallback devices offline for user '{}'",
                    username
                );
            }
        } else {
            anyhow::bail!(
                "Primary device '{}' offline and no fallback configured for user '{}'",
                user_config.primary_device, username
            );
        };

        Ok(TopsiRoute {
            username: username.to_string(),
            primary_device_id: user_config.primary_device.clone(),
            serving_device_id,
            topsi_db_path,
            using_fallback,
            secondary_devices: user_config.secondary_devices.clone(),
            projects_path: PathBuf::from(&user_config.projects_path),
        })
    }

    /// Check if a device is online
    async fn is_device_online(&self, pool: &SqlitePool, device_id: &str) -> Result<bool> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT is_online FROM device_registry WHERE id = ?"
        )
        .bind(device_id)
        .fetch_optional(pool)
        .await
        .context("Failed to query device status")?;

        Ok(result.map(|(online,)| online == 1).unwrap_or(false))
    }

    /// Get device configuration
    pub fn get_device(&self, device_id: &str) -> Option<&DeviceConfig> {
        self.device_map.get(device_id)
    }

    /// Get user configuration
    pub fn get_user(&self, username: &str) -> Option<&UserConfig> {
        self.user_map.get(username)
    }

    /// Get APN relay URL
    pub fn apn_relay_url(&self) -> &str {
        &self.config.apn.relay_url
    }
}

// ============================================================================
// Database Path Resolution
// ============================================================================

/// Resolve database path for authenticated user
pub fn resolve_db_path_for_user(username: &str, config_path: Option<&str>) -> Result<PathBuf> {
    let config_file = config_path.unwrap_or("orcha_config.toml");
    let router = OrchaRouter::from_file(config_file)?;

    let user_config = router.get_user(username)
        .context(format!("User '{}' not found in ORCHA config", username))?;

    Ok(PathBuf::from(&user_config.topsi_db_path))
}

/// Initialize per-user Topsi database if it doesn't exist
pub async fn ensure_user_topsi_db(db_path: &PathBuf) -> Result<SqlitePool> {
    // Create parent directory if needed
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .context("Failed to create Topsi database directory")?;
    }

    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    tracing::info!("Connecting to Topsi database: {}", db_url);

    let pool = SqlitePool::connect(&db_url)
        .await
        .context("Failed to connect to Topsi database")?;

    // Run migrations to ensure schema is up to date
    // Note: Migrations are run separately during database initialization
    // We don't run them here to avoid sqlx macro path resolution issues
    // The database should already be migrated via init_user_topsi_databases.sh

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let router = OrchaRouter::from_file("orcha_config.toml");
        assert!(router.is_ok(), "Failed to load ORCHA config");

        let router = router.unwrap();

        // Test user lookup
        assert!(router.get_user("admin").is_some());
        assert!(router.get_user("Sirak").is_some());
        assert!(router.get_user("Bonomotion").is_some());

        // Test device lookup
        assert!(router.get_device("pythia-master-node-001").is_some());
        assert!(router.get_device("space-terminal-001").is_some());
    }

    #[test]
    fn test_db_path_resolution() {
        let admin_path = resolve_db_path_for_user("admin", Some("orcha_config.toml"));
        assert!(admin_path.is_ok());

        let path = admin_path.unwrap();
        assert!(path.to_str().unwrap().contains("admin"));
    }
}
