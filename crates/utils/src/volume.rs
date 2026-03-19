//! Shared sovereign-stack volume path resolution.
//!
//! Both `org_cloud` routes and `org_cloud_indexer` need to map logical volume
//! names to physical filesystem paths. This module centralises that logic so
//! there is a single source of truth.

use std::path::PathBuf;

/// Known sovereign-stack volume names.
pub const VOLUME_SOVEREIGN_PERSONAL: &str = "sovereign_personal";
pub const VOLUME_SOVEREIGN_ORG: &str = "sovereign_org";
pub const VOLUME_MEDIA_PIPELINE: &str = "media_pipeline";
pub const VOLUME_SOVEREIGN: &str = "sovereign";
pub const VOLUME_DATA_SOURCES: &str = "data_sources";
pub const VOLUME_ARTIFACTS: &str = "artifacts";

/// Error returned when a volume name is unrecognised or a path is invalid.
#[derive(Debug, thiserror::Error)]
pub enum VolumePathError {
    #[error("Unknown volume: {0}")]
    UnknownVolume(String),

    #[error("Invalid file path")]
    InvalidPath,
}

/// Read the sovereign-stack environment variables (with defaults).
fn stack_env() -> (String, String, String) {
    let stack_root = std::env::var("SOVEREIGN_STACK_ROOT")
        .unwrap_or_else(|_| "E:/topos/sovereign_stack".to_string());
    let org_name =
        std::env::var("SOVEREIGN_STACK_ORG_NAME").unwrap_or_else(|_| "Sirak Studios".to_string());
    let storage_root = std::env::var("SOVEREIGN_STORAGE_ROOT")
        .unwrap_or_else(|_| "E:/topos/sovereign_storage".to_string());
    (stack_root, org_name, storage_root)
}

/// Return the base directory for a given volume name.
///
/// This is the pure "volume name → base path" mapping extracted from
/// `org_cloud.rs::resolve_volume_path` and `org_cloud_indexer.rs::get_org_volumes`.
pub fn volume_base_path(volume: &str) -> Result<PathBuf, VolumePathError> {
    let (stack_root, org_name, storage_root) = stack_env();

    match volume {
        VOLUME_SOVEREIGN_PERSONAL => Ok(PathBuf::from(&stack_root).join("Personal")),
        VOLUME_SOVEREIGN_ORG => Ok(PathBuf::from(&stack_root).join(&org_name)),
        VOLUME_MEDIA_PIPELINE => Ok(PathBuf::from(&stack_root)
            .join(&org_name)
            .join("Media Pipeline")),
        VOLUME_SOVEREIGN => Ok(PathBuf::from(storage_root)),
        VOLUME_DATA_SOURCES => Ok(crate::cache_dir().join("data_sources")),
        VOLUME_ARTIFACTS => Ok(crate::cache_dir().join("artifacts")),
        other => Err(VolumePathError::UnknownVolume(other.to_string())),
    }
}

/// Resolve a logical `(volume, file_path)` pair to an absolute filesystem path.
///
/// Validates against path-traversal attacks (reject `..`, null bytes, encoded
/// `%2e%2e` sequences) and, when both paths exist on disk, verifies via
/// canonicalisation that the resolved path is still under the volume base.
pub fn resolve_volume_path(volume: &str, file_path: &str) -> Result<PathBuf, VolumePathError> {
    // Prevent path traversal
    if file_path.contains("..")
        || file_path.contains('\0')
        || file_path.contains("%2e%2e")
        || file_path.contains("%2E%2E")
    {
        return Err(VolumePathError::InvalidPath);
    }

    let base = volume_base_path(volume)?;
    let resolved = base.join(file_path);

    // When both paths exist on disk, canonicalise and verify containment
    if let (Ok(canon_base), Ok(canon_resolved)) = (
        std::fs::canonicalize(&base),
        std::fs::canonicalize(&resolved),
    )
        && !canon_resolved.starts_with(&canon_base) {
            return Err(VolumePathError::InvalidPath);
        }

    Ok(resolved)
}

/// Convenience: list all sovereign-stack volume names and their base paths.
/// Useful for the indexer that needs to iterate over all volumes.
pub fn all_sovereign_volumes() -> Vec<(&'static str, PathBuf)> {
    let (stack_root, org_name, storage_root) = stack_env();
    let stack = PathBuf::from(&stack_root);
    let org_dir = stack.join(&org_name);

    vec![
        (VOLUME_SOVEREIGN_PERSONAL, stack.join("Personal")),
        (VOLUME_SOVEREIGN_ORG, org_dir.clone()),
        (VOLUME_SOVEREIGN, PathBuf::from(storage_root)),
        (VOLUME_MEDIA_PIPELINE, org_dir.join("Media Pipeline")),
    ]
}
