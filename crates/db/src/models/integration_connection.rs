//! Unified OAuth credential store — used by every external provider.
//!
//! Tokens are stored as AES-256-GCM ciphertext (base64) in the
//! `*_ciphertext` columns. Encryption/decryption lives in
//! `services::oauth_crypto`; this model only handles persistence.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

/// All columns, with the three UUID-typed columns (`id`, `organization_id`,
/// `project_id`) converted from TEXT storage to 16-byte BLOBs so sqlx's
/// SQLite `Uuid` decoder accepts them. Reused by every find_* method so the
/// conversion stays in one place.
const SELECT_ALL_TEXT_TO_BLOB: &str = "\
    SELECT \
        unhex(replace(id, '-', ''))              AS id, \
        unhex(replace(organization_id, '-', '')) AS organization_id, \
        CASE WHEN project_id IS NULL THEN NULL \
             ELSE unhex(replace(project_id, '-', '')) END AS project_id, \
        provider, provider_account_id, display_name, avatar_url, \
        access_token_ciphertext, refresh_token_ciphertext, token_expires_at, \
        scopes, status, last_sync_at, last_error, metadata, created_at, updated_at \
    FROM integration_connections";

const SELECT_ALL_TEXT_TO_BLOB_WHERE_ORG_PROVIDER: &str = "SELECT \
        unhex(replace(id, '-', ''))              AS id, \
        unhex(replace(organization_id, '-', '')) AS organization_id, \
        CASE WHEN project_id IS NULL THEN NULL \
             ELSE unhex(replace(project_id, '-', '')) END AS project_id, \
        provider, provider_account_id, display_name, avatar_url, \
        access_token_ciphertext, refresh_token_ciphertext, token_expires_at, \
        scopes, status, last_sync_at, last_error, metadata, created_at, updated_at \
     FROM integration_connections \
     WHERE organization_id = ?1 AND provider = ?2 ORDER BY created_at DESC";

#[allow(dead_code)] // referenced by future find_by_org / find_expiring_within fixes
const _UNUSED_HINT: &str = SELECT_ALL_TEXT_TO_BLOB;

#[derive(Debug, Error)]
pub enum IntegrationConnectionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("integration connection not found")]
    NotFound,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IntegrationConnection {
    pub id: Uuid,
    pub organization_id: Uuid,
    #[ts(optional)]
    pub project_id: Option<Uuid>,
    pub provider: String,
    pub provider_account_id: String,
    #[ts(optional)]
    pub display_name: Option<String>,
    #[ts(optional)]
    pub avatar_url: Option<String>,

    // Tokens are NEVER serialized to API responses. Only the service layer
    // (which calls oauth_crypto::decrypt) ever touches these fields.
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub access_token_ciphertext: Option<String>,
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub refresh_token_ciphertext: Option<String>,

    #[ts(type = "Date | null", optional)]
    pub token_expires_at: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub scopes: Option<String>,
    pub status: String,
    #[ts(type = "Date | null", optional)]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub last_error: Option<String>,
    pub metadata: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateIntegrationConnection {
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,
    pub provider: String,
    pub provider_account_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token_ciphertext: Option<String>,
    pub refresh_token_ciphertext: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateTokens {
    pub access_token_ciphertext: Option<String>,
    pub refresh_token_ciphertext: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<String>,
}

impl IntegrationConnection {
    /// Insert a new connection or update tokens on an existing one keyed by
    /// (organization_id, provider, provider_account_id).
    pub async fn upsert(
        pool: &SqlitePool,
        c: CreateIntegrationConnection,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO integration_connections (
                id, organization_id, project_id, provider, provider_account_id,
                display_name, avatar_url, access_token_ciphertext, refresh_token_ciphertext,
                token_expires_at, scopes, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, COALESCE(?12, '{}'))
            ON CONFLICT(organization_id, provider, provider_account_id) DO UPDATE SET
                display_name             = excluded.display_name,
                avatar_url               = excluded.avatar_url,
                access_token_ciphertext  = excluded.access_token_ciphertext,
                -- Refresh tokens are sometimes only issued once — keep the existing one
                -- if the new exchange didn't return a fresh refresh_token.
                refresh_token_ciphertext = COALESCE(excluded.refresh_token_ciphertext, integration_connections.refresh_token_ciphertext),
                token_expires_at         = excluded.token_expires_at,
                scopes                   = excluded.scopes,
                status                   = 'active',
                last_error               = NULL,
                updated_at               = datetime('now','subsec')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(c.organization_id)
        .bind(c.project_id)
        .bind(&c.provider)
        .bind(&c.provider_account_id)
        .bind(&c.display_name)
        .bind(&c.avatar_url)
        .bind(&c.access_token_ciphertext)
        .bind(&c.refresh_token_ciphertext)
        .bind(c.token_expires_at)
        .bind(&c.scopes)
        .bind(&c.metadata)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        // Schema declares `id` as TEXT but every existing row was inserted via
        // `upsert` (line ~154) binding `Uuid` as 16-byte BLOB. SQLite's flexible
        // type affinity stored them as BLOB. The old TEXT_TO_BLOB query above
        // ran `unhex(replace(...))` assuming TEXT storage — it never matched a
        // real row. Plain SELECT + BLOB bind matches what's actually stored.
        sqlx::query_as::<_, Self>("SELECT * FROM integration_connections WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_org(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM integration_connections \
             WHERE organization_id = ?1 ORDER BY created_at DESC",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_org_and_provider(
        pool: &SqlitePool,
        organization_id: Uuid,
        provider: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        // The `id`, `organization_id`, `project_id` columns are TEXT (per
        // migration 20260328) but the struct field is `Uuid`, which sqlx's
        // SQLite decoder expects as 16-byte BLOB. Convert TEXT → BLOB at the
        // SELECT layer with `unhex(replace(<col>, '-', ''))` so the row decodes.
        sqlx::query_as::<_, Self>(SELECT_ALL_TEXT_TO_BLOB_WHERE_ORG_PROVIDER)
            .bind(organization_id.to_string())
            .bind(provider)
            .fetch_all(pool)
            .await
    }

    /// Active connections whose access token expires within `within` from now.
    /// Drives the background refresh worker.
    pub async fn find_expiring_within(
        pool: &SqlitePool,
        within: Duration,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let cutoff = Utc::now() + within;
        sqlx::query_as::<_, Self>(
            "SELECT * FROM integration_connections \
             WHERE status = 'active' \
               AND refresh_token_ciphertext IS NOT NULL \
               AND token_expires_at IS NOT NULL \
               AND token_expires_at <= ?1",
        )
        .bind(cutoff)
        .fetch_all(pool)
        .await
    }

    pub async fn update_tokens(
        pool: &SqlitePool,
        id: Uuid,
        upd: UpdateTokens,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            r#"
            UPDATE integration_connections SET
                access_token_ciphertext  = COALESCE(?1, access_token_ciphertext),
                refresh_token_ciphertext = COALESCE(?2, refresh_token_ciphertext),
                token_expires_at         = ?3,
                scopes                   = COALESCE(?4, scopes),
                status                   = 'active',
                last_error               = NULL,
                updated_at               = datetime('now','subsec')
            WHERE id = ?5
            RETURNING *
            "#,
        )
        .bind(&upd.access_token_ciphertext)
        .bind(&upd.refresh_token_ciphertext)
        .bind(upd.token_expires_at)
        .bind(&upd.scopes)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    pub async fn mark_status(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
        last_error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE integration_connections \
             SET status = ?1, last_error = ?2, updated_at = datetime('now','subsec') \
             WHERE id = ?3",
        )
        .bind(status)
        .bind(last_error)
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn touch_last_sync(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE integration_connections \
             SET last_sync_at = datetime('now','subsec'), updated_at = datetime('now','subsec') \
             WHERE id = ?1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM integration_connections WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// True when token_expires_at is within 5 minutes of now (or already past).
    pub fn needs_refresh(&self) -> bool {
        match self.token_expires_at {
            Some(exp) => exp <= Utc::now() + Duration::minutes(5),
            None => false,
        }
    }
}

// ─── Pending OAuth State (CSRF protection) ─────────────────────────────────

#[derive(Debug, Clone, FromRow)]
pub struct OAuthPendingState {
    pub state_token: String,
    pub provider: String,
    pub organization_id: Uuid,
    pub project_id: Option<Uuid>,
    pub redirect_after: Option<String>,
    pub metadata: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl OAuthPendingState {
    pub async fn create(
        pool: &SqlitePool,
        state_token: &str,
        provider: &str,
        organization_id: Uuid,
        project_id: Option<Uuid>,
        redirect_after: Option<&str>,
        metadata: Option<&str>,
        ttl: Duration,
    ) -> Result<(), sqlx::Error> {
        let expires_at = Utc::now() + ttl;
        sqlx::query(
            "INSERT INTO oauth_pending_states \
             (state_token, provider, organization_id, project_id, redirect_after, metadata, expires_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, COALESCE(?6, '{}'), ?7)",
        )
        .bind(state_token)
        .bind(provider)
        .bind(organization_id)
        .bind(project_id)
        .bind(redirect_after)
        .bind(metadata)
        .bind(expires_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Atomically look up and remove a pending state. Returns None if missing
    /// or expired — never returns a state that's already been consumed.
    pub async fn consume(
        pool: &SqlitePool,
        state_token: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let row: Option<Self> =
            sqlx::query_as::<_, Self>("SELECT * FROM oauth_pending_states WHERE state_token = ?1")
                .bind(state_token)
                .fetch_optional(pool)
                .await?;

        sqlx::query("DELETE FROM oauth_pending_states WHERE state_token = ?1")
            .bind(state_token)
            .execute(pool)
            .await?;

        Ok(row.filter(|r| r.expires_at > Utc::now()))
    }

    pub async fn cleanup_expired(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM oauth_pending_states WHERE expires_at < datetime('now','subsec')",
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }
}
