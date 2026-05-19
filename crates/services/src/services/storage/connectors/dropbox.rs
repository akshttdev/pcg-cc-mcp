//! Dropbox Storage Connector
//!
//! Auth: Dropbox OAuth 2.0 with PKCE optional but supported. We use the
//! plain client_secret flow to match the existing pattern.
//! Sync: `/2/files/list_folder` + `/list_folder/continue`. The `cursor`
//! returned by either is opaque — we round-trip it as `sync_cursor`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::services::storage::{
    AccountInfo, OAuthTokens, RemoteFileChange, StorageConnector, StorageError, SyncBatch,
};

const AUTH_URL: &str = "https://www.dropbox.com/oauth2/authorize";
const TOKEN_URL: &str = "https://api.dropbox.com/oauth2/token";
const API_BASE: &str = "https://api.dropboxapi.com/2";
const CONTENT_BASE: &str = "https://content.dropboxapi.com/2";
const DEFAULT_SCOPES: &str = "files.metadata.read files.content.read account_info.read";

pub struct DropboxConnector {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl DropboxConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("DROPBOX_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("DROPBOX_CLIENT_SECRET").unwrap_or_default(),
        }
    }

    fn scopes() -> String {
        std::env::var("DROPBOX_OAUTH_SCOPES").unwrap_or_else(|_| DEFAULT_SCOPES.to_string())
    }
}

// ─── Token + account responses ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    token_type: String,
    scope: Option<String>,
}

fn tokens_from_response(t: TokenResponse) -> OAuthTokens {
    let expires_at = t
        .expires_in
        .map(|s| Utc::now() + chrono::Duration::seconds(s));
    OAuthTokens {
        access_token: t.access_token,
        refresh_token: t.refresh_token,
        expires_at,
        token_type: t.token_type,
        scope: t.scope,
    }
}

#[derive(Debug, Deserialize)]
struct CurrentAccount {
    account_id: String,
    name: AccountName,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AccountName {
    display_name: Option<String>,
}

// ─── List folder responses ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ListFolderResult {
    entries: Vec<Entry>,
    cursor: String,
    has_more: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = ".tag", rename_all = "lowercase")]
enum Entry {
    File(FileEntry),
    Folder(FolderEntry),
    Deleted(DeletedEntry),
}

#[derive(Debug, Deserialize)]
struct FileEntry {
    id: String,
    name: String,
    path_display: Option<String>,
    path_lower: Option<String>,
    size: i64,
    content_hash: Option<String>,
    server_modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct FolderEntry {
    id: String,
    name: String,
    path_display: Option<String>,
    path_lower: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeletedEntry {
    name: String,
    path_display: Option<String>,
    path_lower: Option<String>,
}

// ─── Connector impl ────────────────────────────────────────────────────────

#[async_trait]
impl StorageConnector for DropboxConnector {
    fn provider(&self) -> &'static str {
        "dropbox"
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, StorageError> {
        let scopes = Self::scopes();
        // token_access_type=offline → ensures we get a refresh_token.
        let url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&state={}\
             &token_access_type=offline&scope={}",
            AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(&scopes),
        );
        Ok(url)
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, StorageError> {
        let params = [
            ("code", code),
            ("grant_type", "authorization_code"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
        ];

        let resp = self
            .client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::AuthError(format!(
                "Dropbox token exchange failed: {body}"
            )));
        }

        let t: TokenResponse = resp
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;
        Ok(tokens_from_response(t))
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, StorageError> {
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
        ];

        let resp = self
            .client
            .post(TOKEN_URL)
            .form(&params)
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::AuthError(format!(
                "Dropbox token refresh failed: {body}"
            )));
        }

        let t: TokenResponse = resp
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;
        Ok(tokens_from_response(t))
    }

    async fn get_account_info(&self, access_token: &str) -> Result<AccountInfo, StorageError> {
        // POST with empty body — Dropbox idiom for /2/* endpoints with no params.
        let resp = self
            .client
            .post(format!("{API_BASE}/users/get_current_account"))
            .bearer_auth(access_token)
            .header("Content-Type", "application/json")
            .body("null")
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?
            .error_for_status()
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;

        let acct: CurrentAccount = resp
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;

        Ok(AccountInfo {
            account_id: acct.account_id,
            email: acct.email,
            display_name: acct.name.display_name,
            drive_id: None,
            extras: serde_json::json!({}),
        })
    }

    async fn sync_changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        root_path: Option<&str>,
    ) -> Result<SyncBatch, StorageError> {
        let mut all_changes: Vec<RemoteFileChange> = Vec::new();
        let mut current_cursor = cursor.map(|s| s.to_string()).filter(|s| !s.is_empty());

        let mut pages = 0usize;
        const MAX_PAGES: usize = 200;

        loop {
            pages += 1;
            if pages > MAX_PAGES {
                return Err(StorageError::ProviderError(
                    "Dropbox sync exceeded MAX_PAGES — partial sync".into(),
                ));
            }

            let (url, body) = match &current_cursor {
                Some(cur) => (
                    format!("{API_BASE}/files/list_folder/continue"),
                    serde_json::json!({ "cursor": cur }),
                ),
                None => {
                    // First crawl: list root recursively. Dropbox treats "" as root,
                    // not "/", and rejects "/" as a path.
                    let path = root_path
                        .map(|p| {
                            let trimmed = p.trim_end_matches('/');
                            if trimmed.is_empty() {
                                String::new()
                            } else if trimmed.starts_with('/') {
                                trimmed.to_string()
                            } else {
                                format!("/{trimmed}")
                            }
                        })
                        .unwrap_or_default();
                    (
                        format!("{API_BASE}/files/list_folder"),
                        serde_json::json!({
                            "path": path,
                            "recursive": true,
                            "include_deleted": true,
                            "include_non_downloadable_files": false,
                        }),
                    )
                }
            };

            let resp = self
                .client
                .post(&url)
                .bearer_auth(access_token)
                .json(&body)
                .send()
                .await
                .map_err(|e| StorageError::NetworkError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(StorageError::ProviderError(format!(
                    "Dropbox list_folder failed ({status}): {body}"
                )));
            }

            let page: ListFolderResult = resp
                .json()
                .await
                .map_err(|e| StorageError::ProviderError(e.to_string()))?;

            for entry in page.entries {
                all_changes.push(entry_to_change(entry));
            }
            current_cursor = Some(page.cursor);
            if !page.has_more {
                break;
            }
        }

        Ok(SyncBatch {
            changes: all_changes,
            next_cursor: current_cursor,
        })
    }

    /// Download a file's bytes by its Dropbox `id:<remote_id>` selector.
    /// Dropbox's download endpoint puts the API arg in a header instead of the
    /// body; the response body is the raw file content.
    async fn download_file(
        &self,
        access_token: &str,
        remote_id: &str,
    ) -> Result<Vec<u8>, StorageError> {
        let api_arg = serde_json::json!({ "path": format!("id:{remote_id}") }).to_string();
        let resp = self
            .client
            .post(format!("{CONTENT_BASE}/files/download"))
            .bearer_auth(access_token)
            .header("Dropbox-API-Arg", api_arg)
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::ProviderError(format!(
                "Dropbox download failed ({status}): {body}"
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?;
        Ok(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `.tag=file` entry should produce a non-deleted, non-folder change
    /// with size, hash, and a leading-slash-trimmed path.
    #[test]
    fn entry_file_round_trips_basic_fields() {
        let entry: Entry = serde_json::from_value(serde_json::json!({
            ".tag": "file",
            "id": "id:abc",
            "name": "brief.pdf",
            "path_display": "/Projects/Brand/brief.pdf",
            "size": 4096,
            "content_hash": "deadbeef",
            "server_modified": "2026-04-30T10:00:00Z"
        }))
        .unwrap();
        let change = entry_to_change(entry);
        assert_eq!(change.remote_id, "id:abc");
        assert_eq!(change.name, "brief.pdf");
        assert_eq!(change.path, "Projects/Brand/brief.pdf");
        assert_eq!(change.size_bytes, 4096);
        assert_eq!(change.content_hash.as_deref(), Some("deadbeef"));
        assert!(!change.deleted);
        assert!(!change.is_folder);
    }

    /// A `.tag=folder` should be flagged as is_folder=true.
    #[test]
    fn entry_folder_marked_is_folder() {
        let entry: Entry = serde_json::from_value(serde_json::json!({
            ".tag": "folder",
            "id": "id:f",
            "name": "Brand",
            "path_display": "/Projects/Brand"
        }))
        .unwrap();
        let change = entry_to_change(entry);
        assert!(change.is_folder);
        assert!(!change.deleted);
    }

    /// A `.tag=deleted` entry has no Dropbox file id — the path becomes the
    /// stable key (this is what the sync worker then uses to locate the row
    /// to soft-delete).
    #[test]
    fn entry_deleted_uses_path_as_remote_id() {
        let entry: Entry = serde_json::from_value(serde_json::json!({
            ".tag": "deleted",
            "name": "old.pdf",
            "path_display": "/Projects/Brand/old.pdf"
        }))
        .unwrap();
        let change = entry_to_change(entry);
        assert!(change.deleted);
        assert_eq!(change.remote_id, "Projects/Brand/old.pdf");
        assert_eq!(change.path, "Projects/Brand/old.pdf");
    }

    /// Dropbox falls back to `path_lower` when `path_display` is missing.
    #[test]
    fn entry_falls_back_to_path_lower() {
        let entry: Entry = serde_json::from_value(serde_json::json!({
            ".tag": "file",
            "id": "id:x",
            "name": "X.PDF",
            "path_lower": "/projects/brand/x.pdf",
            "size": 1
        }))
        .unwrap();
        let change = entry_to_change(entry);
        assert_eq!(change.path, "projects/brand/x.pdf");
    }

    /// Token response without expires_in (rare but allowed by Dropbox for
    /// long-lived tokens) should produce expires_at = None.
    #[test]
    fn token_response_no_expiry_is_none() {
        let raw: TokenResponse = serde_json::from_value(serde_json::json!({
            "access_token": "at",
            "token_type": "bearer"
        }))
        .unwrap();
        let tokens = tokens_from_response(raw);
        assert_eq!(tokens.access_token, "at");
        assert!(tokens.expires_at.is_none());
        assert!(tokens.refresh_token.is_none());
    }
}

fn entry_to_change(entry: Entry) -> RemoteFileChange {
    match entry {
        Entry::File(f) => {
            let path = f
                .path_display
                .or(f.path_lower)
                .unwrap_or_else(|| f.name.clone())
                .trim_start_matches('/')
                .to_string();
            RemoteFileChange {
                remote_id: f.id,
                name: f.name,
                path,
                mime_type: None, // Dropbox doesn't return mime — inferred later from extension
                size_bytes: f.size,
                content_hash: f.content_hash,
                web_url: None,
                deleted: false,
                is_folder: false,
                modified_at: f.server_modified,
            }
        }
        Entry::Folder(f) => {
            let path = f
                .path_display
                .or(f.path_lower)
                .unwrap_or_else(|| f.name.clone())
                .trim_start_matches('/')
                .to_string();
            RemoteFileChange {
                remote_id: f.id,
                name: f.name,
                path,
                mime_type: None,
                size_bytes: 0,
                content_hash: None,
                web_url: None,
                deleted: false,
                is_folder: true,
                modified_at: None,
            }
        }
        Entry::Deleted(d) => {
            // Dropbox doesn't return an id for deleted entries — use the path
            // as a stable key so the writer can locate the row to soft-delete.
            let path = d
                .path_display
                .or(d.path_lower)
                .unwrap_or_else(|| d.name.clone())
                .trim_start_matches('/')
                .to_string();
            RemoteFileChange {
                remote_id: path.clone(),
                name: d.name,
                path,
                mime_type: None,
                size_bytes: 0,
                content_hash: None,
                web_url: None,
                deleted: true,
                is_folder: false,
                modified_at: None,
            }
        }
    }
}
