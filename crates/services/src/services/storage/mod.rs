//! Cloud Storage Services
//!
//! Unified interface for OneDrive, Dropbox, and Google Drive:
//! - Per-provider OAuth (re-using `oauth_token_manager` for credential storage)
//! - Incremental sync: each provider exposes a delta/cursor mechanism, normalized
//!   into the same `SyncBatch` shape so the worker can process them uniformly
//! - Writes one `cloud_files` row per remote file; later passes will turn
//!   text-extractable rows into `data_sources` for the knowledge graph.

pub mod connectors;
pub mod sync_worker;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("provider not supported: {0}")]
    UnsupportedProvider(String),
    #[error("authentication failed: {0}")]
    AuthError(String),
    #[error("rate limited by provider")]
    RateLimited,
    #[error("provider API error: {0}")]
    ProviderError(String),
    #[error("network error: {0}")]
    NetworkError(String),
    #[error("invalid cursor: {0}")]
    InvalidCursor(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

/// OAuth tokens returned by a provider's token endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub token_type: String,
    pub scope: Option<String>,
}

/// Identity info pulled from a connected drive — used to label the connection
/// and key the `(org, provider, account_email)` UNIQUE constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Provider's stable ID for the account (Graph user id, Dropbox account_id, ...).
    pub account_id: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    /// Optional drive-level identifier (Graph drive_id, Dropbox root_namespace_id, ...).
    pub drive_id: Option<String>,
    /// Provider-specific extras for `cloud_storage_accounts.metadata` (JSON-encoded by caller).
    pub extras: serde_json::Value,
}

/// One change record from an incremental sync. Maps to a single row upsert
/// or soft-delete in `cloud_files`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileChange {
    /// Provider-side stable ID (Graph item id, Dropbox id, Drive file id).
    pub remote_id: String,
    pub name: String,
    /// Slash-separated path from the sync root, e.g. "Projects/Brand/2026/brief.pdf".
    pub path: String,
    pub mime_type: Option<String>,
    pub size_bytes: i64,
    pub content_hash: Option<String>,
    pub web_url: Option<String>,
    /// True when the change is a deletion. Drives soft-delete on the cloud_files row.
    pub deleted: bool,
    /// True when the entry is a folder rather than a file. Folders are tracked
    /// for path resolution but skipped by the cloud_files writer.
    pub is_folder: bool,
    pub modified_at: Option<DateTime<Utc>>,
}

/// Result of one incremental sync call.
#[derive(Debug, Clone)]
pub struct SyncBatch {
    pub changes: Vec<RemoteFileChange>,
    /// Updated cursor / delta token to persist for the next call.
    /// `None` means "leave the existing cursor in place" (rare — most providers
    /// always advance the cursor).
    pub next_cursor: Option<String>,
}

#[async_trait]
pub trait StorageConnector: Send + Sync {
    fn provider(&self) -> &'static str;

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, StorageError>;

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, StorageError>;

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, StorageError>;

    async fn get_account_info(&self, access_token: &str) -> Result<AccountInfo, StorageError>;

    /// Run an incremental sync. `cursor=None` means first crawl.
    async fn sync_changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        root_path: Option<&str>,
    ) -> Result<SyncBatch, StorageError>;
}

pub fn get_connector(provider: &str) -> Result<Box<dyn StorageConnector>, StorageError> {
    match provider {
        "onedrive" => Ok(Box::new(connectors::onedrive::OneDriveConnector::new())),
        "dropbox" => Ok(Box::new(connectors::dropbox::DropboxConnector::new())),
        "gdrive" => Ok(Box::new(connectors::gdrive::GoogleDriveConnector::new())),
        "local" => Ok(Box::new(connectors::local::LocalFolderConnector::new())),
        other => Err(StorageError::UnsupportedProvider(other.to_string())),
    }
}
