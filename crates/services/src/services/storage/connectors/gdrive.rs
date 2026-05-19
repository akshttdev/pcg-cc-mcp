//! Google Drive Storage Connector
//!
//! Auth: Google OAuth 2.0. Reuses the existing Google OAuth app (also used by
//! Gmail / Calendar) — adding the Drive scopes is a config-only change.
//! Sync: `/drive/v3/changes` with a `pageToken`. First-call cursor comes from
//! `/drive/v3/changes/startPageToken`. We intentionally use *changes* (not the
//! more verbose `/drive/v3/files` listing) so the cursor advances cleanly on
//! moves, renames, and trashing.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::services::storage::{
    AccountInfo, OAuthTokens, RemoteFileChange, StorageConnector, StorageError, SyncBatch,
};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const API_BASE: &str = "https://www.googleapis.com/drive/v3";
const DEFAULT_SCOPES: &str = "https://www.googleapis.com/auth/drive.readonly \
                              https://www.googleapis.com/auth/drive.metadata.readonly \
                              https://www.googleapis.com/auth/userinfo.email \
                              https://www.googleapis.com/auth/userinfo.profile";

pub struct GoogleDriveConnector {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl GoogleDriveConnector {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            client_id: std::env::var("GOOGLE_CLIENT_ID").unwrap_or_default(),
            client_secret: std::env::var("GOOGLE_CLIENT_SECRET").unwrap_or_default(),
        }
    }

    fn scopes() -> String {
        std::env::var("GDRIVE_OAUTH_SCOPES").unwrap_or_else(|_| DEFAULT_SCOPES.to_string())
    }
}

// ─── Token + identity responses ────────────────────────────────────────────

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

#[derive(Debug, Deserialize)]
struct AboutResponse {
    user: AboutUser,
}

#[derive(Debug, Deserialize)]
struct AboutUser {
    #[serde(rename = "permissionId")]
    permission_id: Option<String>,
    #[serde(rename = "emailAddress")]
    email_address: Option<String>,
    #[serde(rename = "displayName")]
    display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StartPageTokenResponse {
    #[serde(rename = "startPageToken")]
    start_page_token: String,
}

#[derive(Debug, Deserialize)]
struct ChangesResponse {
    changes: Vec<ChangeItem>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "newStartPageToken")]
    new_start_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChangeItem {
    #[serde(rename = "fileId")]
    file_id: Option<String>,
    removed: Option<bool>,
    file: Option<DriveFile>,
}

// Drive returns camelCase field names — the struct uses snake_case Rust idioms
// and serde renames each field on the way in. Without this, fields like
// `md5Checksum` silently deserialize to None, leaving content_hash empty for
// every file.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFile {
    id: String,
    name: String,
    mime_type: Option<String>,
    size: Option<String>,
    md5_checksum: Option<String>,
    sha1_checksum: Option<String>,
    sha256_checksum: Option<String>,
    web_view_link: Option<String>,
    modified_time: Option<DateTime<Utc>>,
    trashed: Option<bool>,
    /// Reserved for a future pass that resolves full paths via parent walks.
    /// Drive's changes feed only returns parent IDs, not names — building a
    /// proper path requires a follow-up `files/get` per parent.
    #[allow(dead_code)]
    parents: Option<Vec<String>>,
}

// ─── Connector impl ────────────────────────────────────────────────────────

#[async_trait]
impl StorageConnector for GoogleDriveConnector {
    fn provider(&self) -> &'static str {
        "gdrive"
    }

    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, StorageError> {
        let scopes = Self::scopes();
        // access_type=offline + prompt=consent → ensures we get a refresh_token
        // even if the user already authorized this app.
        let url = format!(
            "{}?client_id={}&response_type=code&redirect_uri={}&state={}\
             &scope={}&access_type=offline&prompt=consent&include_granted_scopes=true",
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
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
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
                "Google token exchange failed: {body}"
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
            ("refresh_token", refresh_token),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
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
                "Google token refresh failed: {body}"
            )));
        }

        let t: TokenResponse = resp
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;
        Ok(tokens_from_response(t))
    }

    async fn get_account_info(&self, access_token: &str) -> Result<AccountInfo, StorageError> {
        let about: AboutResponse = self
            .client
            .get(format!(
                "{API_BASE}/about?fields=user(permissionId,emailAddress,displayName)"
            ))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| StorageError::NetworkError(e.to_string()))?
            .error_for_status()
            .map_err(|e| StorageError::ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| StorageError::ProviderError(e.to_string()))?;

        let account_id = about
            .user
            .permission_id
            .clone()
            .or_else(|| about.user.email_address.clone())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(AccountInfo {
            account_id,
            email: about.user.email_address,
            display_name: about.user.display_name,
            drive_id: None,
            extras: serde_json::json!({}),
        })
    }

    async fn sync_changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        _root_path: Option<&str>,
    ) -> Result<SyncBatch, StorageError> {
        // Cursor flow: first call → fetch /changes/startPageToken to anchor,
        // then page through /changes from there. Subsequent calls reuse the
        // newStartPageToken stored as sync_cursor.
        let mut page_token = match cursor {
            Some(c) if !c.is_empty() => c.to_string(),
            _ => {
                let start: StartPageTokenResponse = self
                    .client
                    .get(format!("{API_BASE}/changes/startPageToken"))
                    .bearer_auth(access_token)
                    .send()
                    .await
                    .map_err(|e| StorageError::NetworkError(e.to_string()))?
                    .error_for_status()
                    .map_err(|e| StorageError::ProviderError(e.to_string()))?
                    .json()
                    .await
                    .map_err(|e| StorageError::ProviderError(e.to_string()))?;
                start.start_page_token
            }
        };

        let mut all_changes: Vec<RemoteFileChange> = Vec::new();
        let new_cursor: Option<String>;

        let mut pages = 0usize;
        const MAX_PAGES: usize = 200;

        loop {
            pages += 1;
            if pages > MAX_PAGES {
                return Err(StorageError::ProviderError(
                    "Google Drive sync exceeded MAX_PAGES — partial sync".into(),
                ));
            }

            let url = format!(
                "{API_BASE}/changes?pageToken={}&pageSize=1000\
                 &includeRemoved=true&restrictToMyDrive=true\
                 &fields=changes(fileId,removed,file(id,name,mimeType,size,md5Checksum,\
sha1Checksum,sha256Checksum,webViewLink,modifiedTime,trashed,parents)),\
nextPageToken,newStartPageToken",
                urlencoding::encode(&page_token)
            );

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
                    "Google Drive changes failed ({status}): {body}"
                )));
            }

            let page: ChangesResponse = resp
                .json()
                .await
                .map_err(|e| StorageError::ProviderError(e.to_string()))?;

            for change in page.changes {
                if let Some(c) = change_to_remote(change) {
                    all_changes.push(c);
                }
            }

            if let Some(next) = page.next_page_token {
                page_token = next;
                continue;
            }
            new_cursor = page.new_start_page_token.or(Some(page_token.clone()));
            break;
        }

        Ok(SyncBatch {
            changes: all_changes,
            next_cursor: new_cursor,
        })
    }

    /// Download a file's bytes by Drive file id. Native (binary) files use
    /// `alt=media`. Google-native docs (Docs/Sheets/Slides) would need
    /// `/export?mimeType=...` — out of scope for v1, the extractor skips them
    /// by mime type.
    async fn download_file(
        &self,
        access_token: &str,
        remote_id: &str,
    ) -> Result<Vec<u8>, StorageError> {
        let url = format!(
            "{API_BASE}/files/{}?alt=media",
            urlencoding::encode(remote_id)
        );
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
                "Google Drive download failed ({status}): {body}"
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

    /// Normal file change. Drive returns size as a STRING — we must parse it
    /// to i64 instead of letting serde fail on type mismatch.
    #[test]
    fn change_normal_file_parses_size_string() {
        let item: ChangeItem = serde_json::from_value(serde_json::json!({
            "fileId": "1abc",
            "removed": false,
            "file": {
                "id": "1abc",
                "name": "brief.pdf",
                "mimeType": "application/pdf",
                "size": "4096",
                "md5Checksum": "abcd",
                "webViewLink": "https://drive.google.com/file/d/1abc/view",
                "modifiedTime": "2026-04-30T10:00:00Z",
                "trashed": false
            }
        }))
        .unwrap();
        let change = change_to_remote(item).expect("should convert");
        assert_eq!(change.remote_id, "1abc");
        assert_eq!(change.name, "brief.pdf");
        assert_eq!(change.mime_type.as_deref(), Some("application/pdf"));
        assert_eq!(change.size_bytes, 4096);
        assert_eq!(change.content_hash.as_deref(), Some("abcd"));
        assert!(!change.deleted);
        assert!(!change.is_folder);
    }

    /// Drive's folder mime type marks `is_folder=true`. The writer skips
    /// folders so they don't get cloud_files rows.
    #[test]
    fn change_folder_mime_type_marked_is_folder() {
        let item: ChangeItem = serde_json::from_value(serde_json::json!({
            "fileId": "1F",
            "file": {
                "id": "1F",
                "name": "Brand",
                "mimeType": "application/vnd.google-apps.folder"
            }
        }))
        .unwrap();
        let change = change_to_remote(item).unwrap();
        assert!(change.is_folder);
    }

    /// `trashed=true` is also a deletion. ORCHA shouldn't treat trashed-but-
    /// not-removed as still-live.
    #[test]
    fn change_trashed_counts_as_deleted() {
        let item: ChangeItem = serde_json::from_value(serde_json::json!({
            "fileId": "1T",
            "file": {
                "id": "1T",
                "name": "trashed.pdf",
                "mimeType": "application/pdf",
                "trashed": true
            }
        }))
        .unwrap();
        let change = change_to_remote(item).unwrap();
        assert!(change.deleted);
    }

    /// `removed=true` with no `file` field — the file is hard-deleted and
    /// Drive doesn't echo metadata. We still need a soft-delete signal with
    /// the original fileId so the writer can locate the row.
    #[test]
    fn change_removed_with_no_file_emits_delete_signal() {
        let item: ChangeItem = serde_json::from_value(serde_json::json!({
            "fileId": "1R",
            "removed": true
        }))
        .unwrap();
        let change = change_to_remote(item).unwrap();
        assert!(change.deleted);
        assert_eq!(change.remote_id, "1R");
    }

    /// Hash precedence: SHA-256 > SHA-1 > MD5. Same logic as OneDrive but
    /// against Drive's field names.
    #[test]
    fn change_hash_precedence() {
        let only_md5: ChangeItem = serde_json::from_value(serde_json::json!({
            "fileId": "x",
            "file": { "id": "x", "name": "f", "md5Checksum": "m5" }
        }))
        .unwrap();
        assert_eq!(
            change_to_remote(only_md5).unwrap().content_hash.as_deref(),
            Some("m5")
        );

        let all: ChangeItem = serde_json::from_value(serde_json::json!({
            "fileId": "x",
            "file": {
                "id": "x", "name": "f",
                "md5Checksum": "m5",
                "sha1Checksum": "s1",
                "sha256Checksum": "s2"
            }
        }))
        .unwrap();
        assert_eq!(
            change_to_remote(all).unwrap().content_hash.as_deref(),
            Some("s2")
        );
    }

    /// Token response from Google must produce a future expiry.
    #[test]
    fn token_response_carries_expiry() {
        let raw: TokenResponse = serde_json::from_value(serde_json::json!({
            "access_token": "at",
            "refresh_token": "rt",
            "expires_in": 3600,
            "token_type": "Bearer",
            "scope": "https://www.googleapis.com/auth/drive.readonly"
        }))
        .unwrap();
        let tokens = tokens_from_response(raw);
        assert_eq!(tokens.access_token, "at");
        assert!(tokens.expires_at.unwrap() > Utc::now());
    }
}

fn change_to_remote(change: ChangeItem) -> Option<RemoteFileChange> {
    let removed = change.removed.unwrap_or(false);
    let file_id = change.file_id?;

    if let Some(f) = change.file {
        let trashed = f.trashed.unwrap_or(false);
        let deleted = removed || trashed;
        let is_folder = f
            .mime_type
            .as_deref()
            .map(|m| m == "application/vnd.google-apps.folder")
            .unwrap_or(false);
        let size_bytes = f
            .size
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0);
        let content_hash = f.sha256_checksum.or(f.sha1_checksum).or(f.md5_checksum);
        // Drive doesn't expose a tree path in the changes feed — the consumer
        // will look up the parent chain on first sight if they need a full path.
        // For now we use the file's `name` as the path, marked relative.
        Some(RemoteFileChange {
            remote_id: f.id,
            name: f.name.clone(),
            path: f.name,
            mime_type: f.mime_type,
            size_bytes,
            content_hash,
            web_url: f.web_view_link,
            deleted,
            is_folder,
            modified_at: f.modified_time,
        })
    } else if removed {
        Some(RemoteFileChange {
            remote_id: file_id.clone(),
            name: file_id.clone(),
            path: String::new(),
            mime_type: None,
            size_bytes: 0,
            content_hash: None,
            web_url: None,
            deleted: true,
            is_folder: false,
            modified_at: None,
        })
    } else {
        None
    }
}
