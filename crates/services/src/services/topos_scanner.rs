//! Topos Directory Scanner Service
//!
//! Discovers and manages projects from a structured topos directory.
//! The topos directory follows a standard structure:
//!
//! ```
//! /path/to/topos/
//! ├── ProjectName/
//! │   ├── GitHub/
//! │   │   └── repo-name/           # Git repository
//! │   ├── Assets/                  # Project assets
//! │   ├── session-logs/            # Claude Code session logs
//! │   ├── artifacts/               # Generated artifacts
//! │   ├── Executive Assets/        # Executive documents
//! │   └── .topos.json              # Project metadata (optional)
//! └── AnotherProject/
//!     └── ...
//! ```

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use ts_rs::TS;
use utils::assets::{topos_dir, topos_structure, ensure_project_structure};

#[derive(Debug, Error)]
pub enum ToposScannerError {
    #[error("TOPOS_DIR not configured")]
    NotConfigured,
    #[error("Topos directory does not exist: {0}")]
    DirectoryNotFound(PathBuf),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Information about a discovered project in the topos
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DiscoveredProject {
    /// Name of the project (directory name in topos)
    pub name: String,
    /// Full path to the project folder
    pub project_path: PathBuf,
    /// Path to the GitHub/git repository (if found)
    pub git_repo_path: Option<PathBuf>,
    /// Whether this project has the standard filing structure
    pub has_standard_structure: bool,
    /// Last modified timestamp
    pub last_modified: Option<u64>,
    /// Optional metadata from .topos.json
    pub metadata: Option<ToposProjectMetadata>,
}

/// Optional metadata that can be stored in .topos.json
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export)]
pub struct ToposProjectMetadata {
    /// Display name (if different from folder name)
    pub display_name: Option<String>,
    /// Project description
    pub description: Option<String>,
    /// Default setup script
    pub setup_script: Option<String>,
    /// Default dev script
    pub dev_script: Option<String>,
    /// Tags for categorization
    pub tags: Option<Vec<String>>,
    /// Whether to auto-import this project
    #[serde(default = "default_true")]
    pub auto_import: bool,
}

fn default_true() -> bool {
    true
}

/// Topos directory scanner for project discovery
#[derive(Clone)]
pub struct ToposScannerService;

impl Default for ToposScannerService {
    fn default() -> Self {
        Self::new()
    }
}

impl ToposScannerService {
    pub fn new() -> Self {
        Self
    }

    /// Check if topos directory is configured
    pub fn is_configured() -> bool {
        topos_dir().is_some()
    }

    /// Get the configured topos directory path
    pub fn get_topos_dir() -> Result<PathBuf, ToposScannerError> {
        topos_dir().ok_or(ToposScannerError::NotConfigured)
    }

    /// Scan the topos directory for projects
    pub async fn discover_projects(&self) -> Result<Vec<DiscoveredProject>, ToposScannerError> {
        let topos_path = Self::get_topos_dir()?;

        if !topos_path.exists() {
            return Err(ToposScannerError::DirectoryNotFound(topos_path));
        }

        tracing::info!("Scanning topos directory: {}", topos_path.display());

        let mut projects = Vec::new();

        // Read entries in the topos directory
        let entries = std::fs::read_dir(&topos_path)?;

        for entry in entries.flatten() {
            let path = entry.path();

            // Skip hidden files and non-directories
            if !path.is_dir() {
                continue;
            }

            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) if !n.starts_with('.') => n.to_string(),
                _ => continue,
            };

            // Check if this looks like a project directory
            if let Some(project) = self.analyze_project_directory(&path, &name).await {
                projects.push(project);
            }
        }

        // Sort by last modified (most recent first)
        projects.sort_by(|a, b| {
            b.last_modified.unwrap_or(0).cmp(&a.last_modified.unwrap_or(0))
        });

        tracing::info!("Discovered {} projects in topos", projects.len());

        Ok(projects)
    }

    /// Analyze a directory to determine if it's a valid project
    async fn analyze_project_directory(&self, path: &Path, name: &str) -> Option<DiscoveredProject> {
        // Get last modified time
        let last_modified = path.metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        // Check for GitHub directory with git repos
        let github_dir = path.join(topos_structure::GITHUB_DIR);
        let git_repo_path = if github_dir.exists() && github_dir.is_dir() {
            self.find_git_repo_in_dir(&github_dir)
        } else {
            // Also check if the project folder itself is a git repo
            if path.join(".git").exists() {
                Some(path.to_path_buf())
            } else {
                None
            }
        };

        // Check for standard structure
        let has_standard_structure = self.has_standard_structure(path);

        // Load optional metadata
        let metadata = self.load_project_metadata(path);

        // Only include if there's a git repo or standard structure
        if git_repo_path.is_some() || has_standard_structure {
            Some(DiscoveredProject {
                name: name.to_string(),
                project_path: path.to_path_buf(),
                git_repo_path,
                has_standard_structure,
                last_modified,
                metadata,
            })
        } else {
            None
        }
    }

    /// Find a git repository inside a directory
    fn find_git_repo_in_dir(&self, dir: &Path) -> Option<PathBuf> {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join(".git").exists() {
                    return Some(path);
                }
            }
        }
        None
    }

    /// Check if a project has the standard filing structure
    fn has_standard_structure(&self, path: &Path) -> bool {
        let required_dirs = [
            topos_structure::GITHUB_DIR,
            topos_structure::ASSETS_DIR,
        ];

        required_dirs.iter().any(|dir| path.join(dir).exists())
    }

    /// Load project metadata from .topos.json if it exists
    fn load_project_metadata(&self, path: &Path) -> Option<ToposProjectMetadata> {
        let metadata_path = path.join(topos_structure::TOPOS_CONFIG_FILE);
        if metadata_path.exists() {
            match std::fs::read_to_string(&metadata_path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(metadata) => return Some(metadata),
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {}", metadata_path.display(), e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}", metadata_path.display(), e);
                }
            }
        }
        None
    }

    /// Ensure a project has the standard filing structure
    pub fn ensure_project_structure(&self, project_path: &PathBuf) -> Result<(), ToposScannerError> {
        ensure_project_structure(project_path)?;
        Ok(())
    }

    /// Create session log file path for a project
    pub fn session_log_path(&self, project_name: &str, session_id: &str) -> Result<PathBuf, ToposScannerError> {
        let topos_path = Self::get_topos_dir()?;
        let logs_dir = topos_path
            .join(project_name)
            .join(topos_structure::SESSION_LOGS_DIR);

        // Ensure directory exists
        std::fs::create_dir_all(&logs_dir)?;

        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        Ok(logs_dir.join(format!("{}_{}.md", timestamp, session_id)))
    }

    /// Create artifact path for a project
    pub fn artifact_path(&self, project_name: &str, artifact_name: &str) -> Result<PathBuf, ToposScannerError> {
        let topos_path = Self::get_topos_dir()?;
        let artifacts_dir = topos_path
            .join(project_name)
            .join(topos_structure::ARTIFACTS_DIR);

        // Ensure directory exists
        std::fs::create_dir_all(&artifacts_dir)?;

        Ok(artifacts_dir.join(artifact_name))
    }

    /// Get the assets directory for a project
    pub fn assets_dir(&self, project_name: &str) -> Result<PathBuf, ToposScannerError> {
        let topos_path = Self::get_topos_dir()?;
        Ok(topos_path.join(project_name).join(topos_structure::ASSETS_DIR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_configured_without_env() {
        // Should return false if TOPOS_DIR is not set
        std::env::remove_var("TOPOS_DIR");
        assert!(!ToposScannerService::is_configured());
    }
}
