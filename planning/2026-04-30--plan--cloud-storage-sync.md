## Cloud Storage Sync — OneDrive / Dropbox / Google Drive

**Date**: 2026-04-30
**Branch**: `tiktok_integration` (continuation; cloud storage layered on top of OAuth token manager)
**Worktree**: `C:/Users/mayan/Desktop/DEV/pcg-cc-mcp` (root)
**Base**: `main`

### Why
PCG actively uses OneDrive — every file there is a potential KG source / media item / deliverable. Without sync, agents are blind to PCG's content library. Dropbox already has a cursor-based ingest model in `dropbox_sources` but no OAuth or sync runtime. GDrive shares the Google OAuth app with Gmail / Calendar.

### Layering on existing infra
- **`integration_connections`** (already in tree, AES-GCM encrypted) — owns the OAuth credentials for **all** providers, social and storage alike.
- **`oauth_token_manager`** — already does store/refresh; we just add the new providers to its dispatch table.
- **NEW `cloud_storage_accounts`** — provider-specific *operational* state (sync cursor, sync_root_path, sync_interval_secs, total_files_synced, last_sync_at). FK back to `integration_connections.id` so tokens stay in the unified store.
- **`cloud_files`** (already in tree) — normalized destination row for every synced file. New `source_type='cloud_sync'` and `source_table='cloud_storage_accounts'` discriminators.
- **`data_sources`** (already in tree) — text-extractable files (PDF, docx, txt) get a row here once content extraction lands.

This avoids duplicating the credential boundary and keeps the encryption story clean.

### Migration shape
`20260430000000_cloud_storage_accounts.sql`:
```sql
CREATE TABLE cloud_storage_accounts (
    id                        TEXT PRIMARY KEY NOT NULL,
    organization_id           TEXT NOT NULL,
    project_id                TEXT,
    integration_connection_id TEXT REFERENCES integration_connections(id) ON DELETE SET NULL,
    provider                  TEXT NOT NULL CHECK (provider IN ('onedrive','dropbox','gdrive')),
    account_email             TEXT,
    display_name              TEXT,
    sync_cursor               TEXT,            -- delta token / cursor / pageToken
    sync_root_path            TEXT,            -- NULL = drive root
    auto_sync                 INTEGER NOT NULL DEFAULT 1,
    sync_interval_secs        INTEGER NOT NULL DEFAULT 900,
    last_sync_at              TEXT,
    last_error                TEXT,
    status                    TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active','syncing','expired','error','disconnected')),
    total_files_synced        INTEGER NOT NULL DEFAULT 0,
    metadata                  TEXT NOT NULL DEFAULT '{}',
    created_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(organization_id, provider, account_email)
);
CREATE INDEX idx_cloud_storage_accounts_org ON cloud_storage_accounts(organization_id);
CREATE INDEX idx_cloud_storage_accounts_provider ON cloud_storage_accounts(provider);
CREATE INDEX idx_cloud_storage_accounts_due ON cloud_storage_accounts(last_sync_at)
    WHERE auto_sync = 1 AND status = 'active';
```
The legacy `dropbox_sources` table is left in place — its rows can migrate forward in a later pass once the new flow is proven.

### Code shape
```
crates/services/src/services/storage/
├── mod.rs                  // StorageConnector trait, registry, errors
├── connectors/
│   ├── mod.rs
│   ├── onedrive.rs         // Microsoft Graph: /me/drive/root/delta
│   ├── dropbox.rs          // /2/files/list_folder + /continue
│   └── gdrive.rs           // /drive/v3/changes
├── sync_worker.rs          // 15-min ticker: pick due rows, run incremental sync
└── content_extractor.rs    // (P2-D, follow-up): PDF/Office/text → data_sources
```

### Trait shape
```rust
#[async_trait]
pub trait StorageConnector: Send + Sync {
    fn provider(&self) -> &'static str;
    async fn get_auth_url(&self, redirect_uri: &str, state: &str) -> Result<String, StorageError>;
    async fn exchange_code(&self, code: &str, redirect_uri: &str) -> Result<OAuthTokens, StorageError>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthTokens, StorageError>;
    async fn get_account_info(&self, access_token: &str) -> Result<AccountInfo, StorageError>;
    /// Run an incremental sync. cursor=None means "first crawl".
    /// Returns (changes, new_cursor).
    async fn sync_changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
        root_path: Option<&str>,
    ) -> Result<SyncBatch, StorageError>;
}
```

### Routes (`integrations.rs` parallel module `storage_integrations.rs`)
```
GET  /storage/connect/{provider}        // 302 to provider auth
GET  /storage/callback/{provider}       // exchange + persist + create cloud_storage_accounts row
GET  /storage/accounts                  // list for org
DELETE /storage/accounts/{id}           // revoke connection + delete row (cloud_files retained)
POST /storage/accounts/{id}/sync        // force incremental sync now (returns count)
PATCH /storage/accounts/{id}            // update auto_sync, sync_interval_secs, sync_root_path
```

### oauth_token_manager dispatcher
Add an arm for `provider in {"onedrive","dropbox","gdrive"}` that calls into `services::storage::get_connector(provider).refresh_token(...)`. Same shape as the social arm.

### Background sync worker
Mirrors `spawn_token_refresh_loop`. Every 15 min:
1. `SELECT id FROM cloud_storage_accounts WHERE auto_sync=1 AND status='active' AND (last_sync_at IS NULL OR last_sync_at < now - sync_interval_secs)`
2. For each: load token via `oauth_token_manager::get_access_token`, run `connector.sync_changes(...)`, upsert `cloud_files`, store new cursor.
3. On error: bump `status='error'`, write `last_error`.

### Out of scope this PR
- **P2-D content extraction** — file → text → `data_sources`. Lands as a follow-up; the sync worker just records `cloud_files` first. Adding extraction without it isn't blocking the OneDrive/Dropbox/GDrive connect-and-list UX.
- **Frontend settings page** — backend-first. UI reuses the existing integrations panel pattern.
- **Shared drives** (SharePoint / Team Drives) — single-drive only for v1.
- **Large-file Range fetching** — only metadata sync this pass; download-and-extract is the follow-up.

### Env vars (added to `.env.integration.example`)
```
MICROSOFT_CLIENT_ID=
MICROSOFT_CLIENT_SECRET=
MICROSOFT_REDIRECT_URI=  # optional override

DROPBOX_CLIENT_ID=
DROPBOX_CLIENT_SECRET=

# GOOGLE_* already exist for Gmail/Calendar — Drive reuses them
```

### Test plan
- [ ] `cargo check --workspace` clean
- [ ] `cargo sqlx prepare --workspace` regenerates with new queries
- [ ] Unit-mock each connector's auth URL builder + token exchange shape
- [ ] Round-trip OAuth manually for OneDrive once Azure app is registered
- [ ] Confirm sync worker upserts `cloud_files` and advances `sync_cursor` on second tick

### Open questions
- Where does file *content* live for the extractor? Likely re-uses the storage volume convention from `cloud_files.storage_volume`. Decided in P2-D.
- Should `cloud_storage_accounts` be project-scoped or org-scoped? Modeled as both: org-required, project optional.
