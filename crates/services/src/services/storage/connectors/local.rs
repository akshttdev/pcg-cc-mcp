//! Local-folder Storage Connector
//!
//! Indexes a directory tree on the master node's local disk as if it were a
//! cloud drive. The `sync_root_path` on the `cloud_storage_accounts` row
//! points at the folder root (e.g. `E:/topos/Sirak Studios Dropbox/`); each
//! sync re-walks the tree and emits a `RemoteFileChange` per file/folder.
//!
//! No OAuth round-trip — the connector ignores the `access_token` argument
//! everywhere. The auth-related trait methods are stubbed so the same call
//! sites used by the OAuth connectors compile, but they're never reached
//! because the local provider's account creation route doesn't go through
//! the OAuth callback path.
//!
//! Cursor strategy: the cursor stores the wall-clock of the previous walk
//! start. The next sync still re-walks the full tree (cheap on local disk)
//! but emits a deletion for paths the previous walk saw and this one didn't.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::services::storage::{
    AccountInfo, OAuthTokens, RemoteFileChange, StorageConnector, StorageError, SyncBatch,
};

const PROVIDER: &str = "local";
/// Cap the per-sync entry count. 1.5 TB Dropbox clones can exceed 100k files;
/// 500k gives headroom without making a single sync run forever.
const MAX_ENTRIES_PER_SYNC: usize = 500_000;

pub struct LocalFolderConnector;

impl LocalFolderConnector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalFolderConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageConnector for LocalFolderConnector {
    fn provider(&self) -> &'static str {
        PROVIDER
    }

    async fn get_auth_url(
        &self,
        _redirect_uri: &str,
        _state: &str,
    ) -> Result<String, StorageError> {
        Err(StorageError::AuthError(
            "local-folder provider has no OAuth flow; use POST /storage/accounts/local instead"
                .into(),
        ))
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _redirect_uri: &str,
    ) -> Result<OAuthTokens, StorageError> {
        Err(StorageError::AuthError(
            "local-folder provider has no OAuth flow".into(),
        ))
    }

    async fn refresh_token(&self, _refresh_token: &str) -> Result<OAuthTokens, StorageError> {
        Err(StorageError::AuthError(
            "local-folder provider has no tokens to refresh".into(),
        ))
    }

    async fn get_account_info(&self, _access_token: &str) -> Result<AccountInfo, StorageError> {
        // No identity to fetch — the caller derives display_name from the
        // sync_root_path at account creation time.
        Err(StorageError::AuthError(
            "local-folder provider has no remote account; account_info is derived at \
             registration time"
                .into(),
        ))
    }

    async fn sync_changes(
        &self,
        _access_token: &str,
        cursor: Option<&str>,
        root_path: Option<&str>,
    ) -> Result<SyncBatch, StorageError> {
        let root = root_path
            .filter(|s| !s.is_empty())
            .ok_or_else(|| StorageError::ProviderError("sync_root_path is required".into()))?;
        let root_buf = PathBuf::from(root);
        if !root_buf.exists() {
            return Err(StorageError::ProviderError(format!(
                "sync_root_path does not exist: {root}"
            )));
        }

        let prev: LocalCursor = cursor
            .map(serde_json::from_str)
            .transpose()
            .unwrap_or_default()
            .unwrap_or_default();

        let started_at = Utc::now();
        let (changes, seen_ids) = walk_root(&root_buf, &prev.seen_ids)?;

        Ok(SyncBatch {
            changes,
            next_cursor: Some(serde_json::to_string(&LocalCursor { started_at, seen_ids }).map_err(
                |e| StorageError::ProviderError(format!("failed to encode cursor: {e}")),
            )?),
        })
    }
}

/// Persisted between syncs. `seen_ids` is the set of `remote_id`s emitted by
/// the previous walk; anything missing this time becomes a soft-delete.
#[derive(Debug, Default, Serialize, Deserialize)]
struct LocalCursor {
    started_at: DateTime<Utc>,
    #[serde(default)]
    seen_ids: HashSet<String>,
}

fn walk_root(
    root: &Path,
    prev_seen: &HashSet<String>,
) -> Result<(Vec<RemoteFileChange>, HashSet<String>), StorageError> {
    let mut changes: Vec<RemoteFileChange> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Exempt the root from the hidden filter so callers can point at, e.g.,
    // `.../tmp/.tmpXYZ/` or any path whose top-level segment happens to
    // start with a dot. The filter still applies to every child entry.
    for entry_result in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden(e.path()))
    {
        if seen.len() >= MAX_ENTRIES_PER_SYNC {
            tracing::warn!(
                "LocalFolderConnector: stopping at MAX_ENTRIES_PER_SYNC={MAX_ENTRIES_PER_SYNC} \
                 for root {root:?}"
            );
            break;
        }

        let entry = match entry_result {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("walkdir error under {root:?}: {e}");
                continue;
            }
        };

        // Skip the root itself; the walk's caller has it in `sync_root_path`.
        if entry.depth() == 0 {
            continue;
        }

        let rel_path = match entry.path().strip_prefix(root) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let path_str = rel_path.to_string_lossy().replace('\\', "/");
        let remote_id = path_str.clone();
        seen.insert(remote_id.clone());

        let is_folder = entry.file_type().is_dir();
        let name = entry
            .file_name()
            .to_string_lossy()
            .to_string();

        let (size_bytes, modified_at, mime_type) = if is_folder {
            (0, None, None)
        } else {
            let md = entry.metadata().ok();
            let size = md.as_ref().map(|m| m.len() as i64).unwrap_or(0);
            let modified = md
                .as_ref()
                .and_then(|m| m.modified().ok())
                .map(DateTime::<Utc>::from);
            let mime = mime_guess::from_path(entry.path())
                .first_raw()
                .map(|s| s.to_string());
            (size, modified, mime)
        };

        changes.push(RemoteFileChange {
            remote_id,
            name,
            path: path_str,
            mime_type,
            size_bytes,
            content_hash: None,
            web_url: None,
            deleted: false,
            is_folder,
            modified_at,
        });
    }

    // Anything we saw last time but didn't see this time → soft-delete.
    for missing in prev_seen.difference(&seen) {
        changes.push(RemoteFileChange {
            remote_id: missing.clone(),
            name: Path::new(missing)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default(),
            path: missing.clone(),
            mime_type: None,
            size_bytes: 0,
            content_hash: None,
            web_url: None,
            deleted: true,
            is_folder: false,
            modified_at: None,
        });
    }

    Ok((changes, seen))
}

/// Skip dotfiles/.git and the macOS/Dropbox metadata sidecars that aren't
/// real content for the KG.
fn is_hidden(path: &Path) -> bool {
    let name = match path.file_name() {
        Some(n) => n.to_string_lossy(),
        None => return false,
    };
    name.starts_with('.')
        || name == "desktop.ini"
        || name == "Thumbs.db"
        || name == ".DS_Store"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn walks_a_simple_tree() {
        let td = TempDir::new().unwrap();
        let root = td.path();
        fs::create_dir(root.join("Projects")).unwrap();
        fs::write(root.join("Projects/brief.pdf"), b"%PDF-1.4 fake").unwrap();
        fs::write(root.join("README.txt"), b"hello").unwrap();
        // Hidden + sidecars must be skipped.
        fs::write(root.join(".hidden"), b"x").unwrap();
        fs::write(root.join("Thumbs.db"), b"x").unwrap();

        let conn = LocalFolderConnector::new();
        let batch = conn
            .sync_changes("", None, Some(&root.to_string_lossy()))
            .await
            .unwrap();

        let names: Vec<&str> = batch.changes.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Projects"));
        assert!(names.contains(&"brief.pdf"));
        assert!(names.contains(&"README.txt"));
        assert!(!names.contains(&".hidden"));
        assert!(!names.contains(&"Thumbs.db"));

        let brief = batch
            .changes
            .iter()
            .find(|c| c.name == "brief.pdf")
            .unwrap();
        assert_eq!(brief.path, "Projects/brief.pdf");
        assert!(!brief.is_folder);
        assert!(brief.size_bytes > 0);
        assert!(brief.mime_type.is_some());
    }

    #[tokio::test]
    async fn second_walk_emits_deletions_for_missing_files() {
        let td = TempDir::new().unwrap();
        let root = td.path();
        fs::write(root.join("a.txt"), b"a").unwrap();
        fs::write(root.join("b.txt"), b"b").unwrap();

        let conn = LocalFolderConnector::new();
        let first = conn
            .sync_changes("", None, Some(&root.to_string_lossy()))
            .await
            .unwrap();
        assert_eq!(first.changes.iter().filter(|c| !c.deleted).count(), 2);

        // Delete b.txt, walk again with the prior cursor.
        fs::remove_file(root.join("b.txt")).unwrap();
        let second = conn
            .sync_changes(
                "",
                first.next_cursor.as_deref(),
                Some(&root.to_string_lossy()),
            )
            .await
            .unwrap();
        let deletion = second
            .changes
            .iter()
            .find(|c| c.deleted)
            .expect("expected a deletion entry");
        assert_eq!(deletion.path, "b.txt");
    }

    #[tokio::test]
    async fn errors_when_root_missing() {
        let conn = LocalFolderConnector::new();
        let res = conn
            .sync_changes("", None, Some("/nonexistent/path/that/should/never/exist"))
            .await;
        assert!(res.is_err());
    }
}
