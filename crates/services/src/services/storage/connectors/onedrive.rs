//! OneDrive Storage Connector (Microsoft Graph)
//!
//! Auth: Microsoft identity platform v2.0 OAuth.
//! Sync: `/me/drive/root/delta` returns the full file tree on first call,
//! then a `@odata.deltaLink` we use as the cursor for incremental syncs.
//!
//! Required scopes (env: `MICROSOFT_OAUTH_SCOPES` or default below):
//!   - `Files.Read.All`        Read all files the user has access to
//!   - `offline_access`        Issue a refresh_token
//!   - `User.Read`             Identify the connecting user

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::services::storage::{
    AccountInfo, OAuthTokens, RemoteFileChange, StorageConnector, StorageError, SyncBatch,
};

const AUTH_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
const TOKEN_URL: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";
const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
const DEFAULT_SCOPES: &str = "Files.Read.All offline_access User.Read";

pub struct OneDriveConnector {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl OneDriveConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("MICROSOFT_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("MICROSOFT_CLIENT_SECRET").unwrap_or_default(),
        }
    }

    fn scopes() -> String {
        std::env::var("MICROSOFT_OAUTH_SCOPES").unwrap_or_else(|_| DEFAULT_SCOPES.to_string())
    }
}

// ─── Token responses ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    token_type: String,
    scope: Option<String>,
}

fn tokens_from_response(t: TokenResponse) -> OAuthTokens {
    OAuthTokens {
        access_token: t.access_token,
        refresh_token: t.refresh_token,
        expires_at: Some(Utc::now() + chrono::Duration::seconds(t.expires_in)),
        token_type: t.token_type,
        scope: t.scope,
    }
}

// ─── Graph payloads ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GraphUser {
    id: String,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
    mail: Option<String>,
    #[serde(rename = "userPrincipalName")]
    user_principal_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GraphDrive {
    id: String,
    #[serde(rename = "driveType")]
    drive_type: Option<String>,
    owner: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct DeltaResponse {
    value: Vec<DeltaItem>,
    #[serde(rename = "@odata.nextLink")]
    next_link: Option<String>,
    #[serde(rename = "@odata.deltaLink")]
    delta_link: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeltaItem {
    id: String,
    name: Option<String>,
    size: Option<i64>,
    #[serde(rename = "webUrl")]
    web_url: Option<String>,
    #[serde(rename = "lastModifiedDateTime")]
    last_modified: Option<DateTime<Utc>>,
    file: Option<FileFacet>,
    folder: Option<serde_json::Value>,
    #[serde(rename = "parentReference")]
    parent_reference: Option<ParentReference>,
    deleted: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct FileFacet {
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    hashes: Option<FileHashes>,
}

#[derive(Debug, Deserialize)]
struct FileHashes {
    #[serde(rename = "quickXorHash")]
    quick_xor_hash: Option<String>,
    #[serde(rename = "sha256Hash")]
    sha256_hash: Option<String>,
    #[serde(rename = "sha1Hash")]
    sha1_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ParentReference {
    path: Option<String>,
}

// ─── Connector impl ────────────────────────────────────────────────────────

#[async_trait]
impl StorageConnector for OneDriveConnector {
    fn provider(&self) -> &'static str {
        "onedrive"
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, StorageError> {
        let scopes = Self::scopes();
        let url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&response_mode=query\
             &scope={}&state={}",
            AUTH_URL,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scopes),
            urlencoding::encode(state),
        );
        Ok(url)
    }

    async fn exchange_code(
        &self,
        code: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokens, StorageError> {
        let scopes = Self::scopes();
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
            ("scope", scopes.as_str()),
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
                "OneDrive token exchange failed: {body}"
            )));
        }

        let t: TokenResponse = resp
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;
        Ok(tokens_from_response(t))
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, StorageError> {
        let scopes = Self::scopes();
        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
            ("scope", scopes.as_str()),
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
                "OneDrive token refresh failed: {body}"
            )));
        }

        let t: TokenResponse = resp
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;
        Ok(tokens_from_response(t))
    }

    async fn get_account_info(&self, access_token: &str) -> Result<AccountInfo, StorageError> {
        // /me — identity
        let user: GraphUser = self
            .client
            .get(format!("{GRAPH_BASE}/me"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?
            .error_for_status()
            .map_err(|e| StorageError::ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;

        // /me/drive — primary drive metadata
        let drive: GraphDrive = self
            .client
            .get(format!("{GRAPH_BASE}/me/drive"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?
            .error_for_status()
            .map_err(|e| StorageError::ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;

        let email = user.mail.or(user.user_principal_name);
        let extras = serde_json::json!({
            "drive_type": drive.drive_type,
            "owner": drive.owner,
        });

        Ok(AccountInfo {
            account_id: user.id,
            email,
            display_name: user.display_name,
            drive_id: Some(drive.id),
            extras,
        })
    }

    async fn sync_changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        root_path: Option<&str>,
    ) -> Result<SyncBatch, StorageError> {
        // First call: cursor=None → start at /root/delta. Subsequent calls: use the
        // stored deltaLink (which contains a `token` query param) directly.
        let initial_url = match cursor {
            Some(link) if !link.is_empty() => link.to_string(),
            _ => match root_path {
                Some(path) if !path.is_empty() => {
                    let trimmed = path.trim_matches('/');
                    format!(
                        "{GRAPH_BASE}/me/drive/root:/{}:/delta",
                        urlencoding::encode(trimmed)
                    )
                }
                _ => format!("{GRAPH_BASE}/me/drive/root/delta"),
            },
        };

        let mut next_url = Some(initial_url);
        let mut all_changes: Vec<RemoteFileChange> = Vec::new();
        let mut final_delta: Option<String> = None;

        // Walk @odata.nextLink pages until we get a deltaLink (end-of-changes).
        // Cap pages defensively so a runaway sync can't pin the worker.
        let mut pages = 0usize;
        const MAX_PAGES: usize = 200;
        while let Some(url) = next_url.take() {
            pages += 1;
            if pages > MAX_PAGES {
                return Err(StorageError::ProviderError(
                    "OneDrive delta exceeded MAX_PAGES — partial sync".into(),
                ));
            }

            let resp = self
                .client
                .get(&url)
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| StorageError::NetworkError(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(StorageError::ProviderError(format!(
                    "OneDrive delta failed ({status}): {body}"
                )));
            }

            let page: DeltaResponse = resp
                .json()
                .await
                .map_err(|e| StorageError::ProviderError(e.to_string()))?;

            for item in page.value {
                if let Some(c) = into_change(item) {
                    all_changes.push(c);
                }
            }

            if let Some(link) = page.delta_link {
                final_delta = Some(link);
                break;
            }
            next_url = page.next_link;
        }

        Ok(SyncBatch {
            changes: all_changes,
            next_cursor: final_delta.or_else(|| Some(String::new())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A normal file change in a nested folder. Parent path comes back from
    /// Graph as `/drive/root:/Projects/Brand` — the prefix should be stripped
    /// and the file name appended.
    #[test]
    fn into_change_normal_file_in_nested_folder() {
        let item: DeltaItem = serde_json::from_value(serde_json::json!({
            "id": "01ABC",
            "name": "brief.pdf",
            "size": 12345,
            "webUrl": "https://example.com/brief.pdf",
            "lastModifiedDateTime": "2026-04-30T10:00:00Z",
            "parentReference": { "path": "/drive/root:/Projects/Brand" },
            "file": {
                "mimeType": "application/pdf",
                "hashes": { "sha256Hash": "deadbeef" }
            }
        }))
        .unwrap();
        let change = into_change(item).expect("should convert");
        assert_eq!(change.remote_id, "01ABC");
        assert_eq!(change.name, "brief.pdf");
        assert_eq!(change.path, "Projects/Brand/brief.pdf");
        assert_eq!(change.mime_type.as_deref(), Some("application/pdf"));
        assert_eq!(change.size_bytes, 12345);
        assert_eq!(change.content_hash.as_deref(), Some("deadbeef"));
        assert!(!change.deleted);
        assert!(!change.is_folder);
    }

    /// A folder entry (no `file` facet, has `folder` facet) should be flagged
    /// as is_folder=true so the writer skips it.
    #[test]
    fn into_change_folder_marked_is_folder() {
        let item: DeltaItem = serde_json::from_value(serde_json::json!({
            "id": "01F",
            "name": "Brand",
            "parentReference": { "path": "/drive/root:" },
            "folder": { "childCount": 5 }
        }))
        .unwrap();
        let change = into_change(item).unwrap();
        assert!(change.is_folder);
        assert!(!change.deleted);
    }

    /// A delta entry with `deleted` set is a soft-delete signal — the path
    /// may or may not be present, but `deleted` must round-trip.
    #[test]
    fn into_change_deleted_round_trips_flag() {
        let item: DeltaItem = serde_json::from_value(serde_json::json!({
            "id": "01D",
            "name": "old.pdf",
            "deleted": { "state": "deleted" }
        }))
        .unwrap();
        let change = into_change(item).unwrap();
        assert!(change.deleted);
        assert_eq!(change.remote_id, "01D");
    }

    /// SHA-256 wins, then SHA-1, then quickXorHash. Tests the precedence so a
    /// future field rename doesn't silently regress to the weakest hash.
    #[test]
    fn into_change_hash_precedence_sha256_then_sha1_then_quickxor() {
        let only_quick: DeltaItem = serde_json::from_value(serde_json::json!({
            "id": "x",
            "name": "f",
            "file": { "hashes": { "quickXorHash": "qx" } }
        }))
        .unwrap();
        assert_eq!(
            into_change(only_quick).unwrap().content_hash.as_deref(),
            Some("qx")
        );

        let sha1_and_quick: DeltaItem = serde_json::from_value(serde_json::json!({
            "id": "x",
            "name": "f",
            "file": { "hashes": { "sha1Hash": "s1", "quickXorHash": "qx" } }
        }))
        .unwrap();
        assert_eq!(
            into_change(sha1_and_quick).unwrap().content_hash.as_deref(),
            Some("s1")
        );

        let all_three: DeltaItem = serde_json::from_value(serde_json::json!({
            "id": "x",
            "name": "f",
            "file": { "hashes": { "sha256Hash": "s2", "sha1Hash": "s1", "quickXorHash": "qx" } }
        }))
        .unwrap();
        assert_eq!(
            into_change(all_three).unwrap().content_hash.as_deref(),
            Some("s2")
        );
    }

    /// Token response must fold into OAuthTokens with a future expires_at.
    #[test]
    fn token_response_carries_expiry_into_future() {
        let raw: TokenResponse = serde_json::from_value(serde_json::json!({
            "access_token": "at",
            "refresh_token": "rt",
            "expires_in": 3600,
            "token_type": "Bearer",
            "scope": "Files.Read.All offline_access"
        }))
        .unwrap();
        let tokens = tokens_from_response(raw);
        assert_eq!(tokens.access_token, "at");
        assert_eq!(tokens.refresh_token.as_deref(), Some("rt"));
        assert!(tokens.expires_at.unwrap() > Utc::now());
    }
}

fn into_change(item: DeltaItem) -> Option<RemoteFileChange> {
    let name = item.name.unwrap_or_else(|| item.id.clone());
    let parent_path = item
        .parent_reference
        .as_ref()
        .and_then(|p| p.path.as_deref())
        .and_then(|p| {
            // Graph returns "/drive/root:/Folder/Sub" — strip the prefix.
            p.split_once(":/").map(|(_, rest)| rest).or(Some(p))
        })
        .unwrap_or("")
        .trim_matches('/')
        .to_string();

    let path = if parent_path.is_empty() {
        name.clone()
    } else {
        format!("{parent_path}/{name}")
    };

    let deleted = item.deleted.is_some();
    let is_folder = item.folder.is_some();
    let (mime_type, content_hash) = match item.file {
        Some(f) => (
            f.mime_type,
            f.hashes
                .and_then(|h| h.sha256_hash.or(h.sha1_hash).or(h.quick_xor_hash)),
        ),
        None => (None, None),
    };

    Some(RemoteFileChange {
        remote_id: item.id,
        name,
        path,
        mime_type,
        size_bytes: item.size.unwrap_or(0),
        content_hash,
        web_url: item.web_url,
        deleted,
        is_folder,
        modified_at: item.last_modified,
    })
}
