use std::{env, path::PathBuf};

use directories::ProjectDirs;
use rust_embed::RustEmbed;

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const ASSET_DIR_ENV: &str = "PCG_ASSET_DIR";
const TOPOS_DIR_ENV: &str = "TOPOS_DIR";

/// Standard project filing structure within topos
/// Each project folder should have this structure:
/// ```
/// ProjectName/
/// ├── GitHub/
/// │   └── repo-name/           # Git repository
/// ├── Assets/                  # Project assets (images, docs, etc.)
/// ├── session-logs/            # Claude Code session logs
/// ├── artifacts/               # Generated artifacts from agents
/// ├── Executive Assets/        # Executive-level documents
/// └── .topos.json              # Project metadata (optional)
/// ```
pub mod topos_structure {
    pub const GITHUB_DIR: &str = "GitHub";
    pub const ASSETS_DIR: &str = "Assets";
    pub const SESSION_LOGS_DIR: &str = "session-logs";
    pub const ARTIFACTS_DIR: &str = "artifacts";
    pub const EXECUTIVE_ASSETS_DIR: &str = "Executive Assets";
    pub const TOPOS_CONFIG_FILE: &str = ".topos.json";
}

/// Get the topos directory path from environment variable or default
pub fn topos_dir() -> Option<PathBuf> {
    if let Ok(topos_path) = env::var(TOPOS_DIR_ENV) {
        let path = PathBuf::from(topos_path);
        if path.exists() && path.is_dir() {
            return Some(path);
        }
        tracing::warn!("TOPOS_DIR '{}' does not exist or is not a directory", path.display());
    }
    None
}

/// Get session logs directory for a specific project within topos
pub fn project_session_logs_dir(project_name: &str) -> Option<PathBuf> {
    topos_dir().map(|topos| {
        topos.join(project_name).join(topos_structure::SESSION_LOGS_DIR)
    })
}

/// Get artifacts directory for a specific project within topos
pub fn project_artifacts_dir(project_name: &str) -> Option<PathBuf> {
    topos_dir().map(|topos| {
        topos.join(project_name).join(topos_structure::ARTIFACTS_DIR)
    })
}

/// Ensure standard project directories exist
pub fn ensure_project_structure(project_path: &PathBuf) -> std::io::Result<()> {
    use topos_structure::*;

    let dirs = [
        GITHUB_DIR,
        ASSETS_DIR,
        SESSION_LOGS_DIR,
        ARTIFACTS_DIR,
        EXECUTIVE_ASSETS_DIR,
    ];

    for dir in dirs {
        let dir_path = project_path.join(dir);
        if !dir_path.exists() {
            std::fs::create_dir_all(&dir_path)?;
            tracing::info!("Created project directory: {}", dir_path.display());
        }
    }

    Ok(())
}

pub fn asset_dir() -> PathBuf {
    let path = if let Ok(custom_dir) = env::var(ASSET_DIR_ENV) {
        PathBuf::from(custom_dir)
    } else if cfg!(debug_assertions) {
        PathBuf::from(PROJECT_ROOT).join("../../dev_assets")
    } else {
        ProjectDirs::from("ai", "bmorphism", "duck-kanban")
            .expect("OS didn't give us a home directory")
            .data_dir()
            .to_path_buf()
    };

    if !path.exists() {
        std::fs::create_dir_all(&path).expect("Failed to create asset directory");
    }

    path
    // ✔ macOS → ~/Library/Application Support/MyApp
    // ✔ Linux → ~/.local/share/myapp   (respects XDG_DATA_HOME)
    // ✔ Windows → %APPDATA%\Example\MyApp
}

pub fn config_path() -> std::path::PathBuf {
    asset_dir().join("config.json")
}

pub fn profiles_path() -> std::path::PathBuf {
    asset_dir().join("profiles.json")
}

#[derive(RustEmbed)]
#[folder = "../../assets/sounds"]
pub struct SoundAssets;

#[derive(RustEmbed)]
#[folder = "../../assets/scripts"]
pub struct ScriptAssets;
