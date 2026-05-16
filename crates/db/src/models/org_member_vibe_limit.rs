//! Per-user-within-org monthly VIBE spending cap.
//!
//! Org admins set these to prevent any single member from draining the
//! organization's `vibe_balance` pool. `monthly_limit_vibe = NULL` means
//! the user is uncapped within the org (the org's balance is still the
//! ultimate ceiling).
//!
//! `current_period_spent` is reset to 0 lazily by callers when
//! `period_start` rolls into a new month — no scheduled DB job.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrgMemberVibeLimit {
    pub organization_id: String,
    /// User id stored as BLOB (legacy `users.id` column).
    #[ts(type = "string")]
    pub user_id: Vec<u8>,
    /// NULL = uncapped within the org.
    pub monthly_limit_vibe: Option<i64>,
    pub current_period_spent: i64,
    pub period_start: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpsertOrgMemberVibeLimit {
    pub organization_id: String,
    pub user_id: String,
    /// NULL clears the cap (uncapped within the org).
    pub monthly_limit_vibe: Option<i64>,
}

impl OrgMemberVibeLimit {
    /// Fetch the limit row for a user within an org, if one exists.
    pub async fn find(
        pool: &SqlitePool,
        org_id: &str,
        user_id_str: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let user_uuid =
            uuid::Uuid::parse_str(user_id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        sqlx::query_as::<_, OrgMemberVibeLimit>(
            "SELECT organization_id, user_id, monthly_limit_vibe, current_period_spent,
                    period_start, created_at, updated_at
             FROM org_member_vibe_limits
             WHERE organization_id = ? AND user_id = ?",
        )
        .bind(org_id)
        .bind(user_uuid)
        .fetch_optional(pool)
        .await
    }

    /// Atomically check + bump `current_period_spent` against the user's cap.
    ///
    /// Rolls the period forward if `period_start < start-of-current-month`.
    /// Returns `Ok(true)` if the bump landed (within cap or uncapped),
    /// `Ok(false)` if the cap is exceeded.
    ///
    /// Callers that need a row to exist for tracking should call
    /// [`Self::ensure_row`] first.
    pub async fn try_bump_spent(
        pool: &SqlitePool,
        org_id: &str,
        user_id_str: &str,
        amount_vibe: i64,
    ) -> Result<bool, sqlx::Error> {
        let user_uuid =
            uuid::Uuid::parse_str(user_id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        // Roll period forward if a new month started.
        sqlx::query(
            "UPDATE org_member_vibe_limits
             SET current_period_spent = 0,
                 period_start = datetime('now','start of month'),
                 updated_at = datetime('now','subsec')
             WHERE organization_id = ? AND user_id = ?
               AND period_start < datetime('now','start of month')",
        )
        .bind(org_id)
        .bind(user_uuid)
        .execute(pool)
        .await?;

        // Atomic conditional bump.
        let result = sqlx::query(
            "UPDATE org_member_vibe_limits
             SET current_period_spent = current_period_spent + ?,
                 updated_at = datetime('now','subsec')
             WHERE organization_id = ? AND user_id = ?
               AND (monthly_limit_vibe IS NULL
                    OR current_period_spent + ? <= monthly_limit_vibe)",
        )
        .bind(amount_vibe)
        .bind(org_id)
        .bind(user_uuid)
        .bind(amount_vibe)
        .execute(pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Decrement `current_period_spent` (used for refunds). Never goes below 0.
    pub async fn decrement_spent(
        pool: &SqlitePool,
        org_id: &str,
        user_id_str: &str,
        amount_vibe: i64,
    ) -> Result<(), sqlx::Error> {
        let user_uuid =
            uuid::Uuid::parse_str(user_id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        sqlx::query(
            "UPDATE org_member_vibe_limits
             SET current_period_spent = MAX(0, current_period_spent - ?),
                 updated_at = datetime('now','subsec')
             WHERE organization_id = ? AND user_id = ?",
        )
        .bind(amount_vibe)
        .bind(org_id)
        .bind(user_uuid)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Insert an uncapped row if one doesn't already exist for this
    /// (org, user). Idempotent; no-op if a row is present.
    pub async fn ensure_row(
        pool: &SqlitePool,
        org_id: &str,
        user_id_str: &str,
    ) -> Result<(), sqlx::Error> {
        let user_uuid =
            uuid::Uuid::parse_str(user_id_str).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        sqlx::query(
            "INSERT OR IGNORE INTO org_member_vibe_limits
                (organization_id, user_id)
             VALUES (?, ?)",
        )
        .bind(org_id)
        .bind(user_uuid)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Upsert the cap (org admin operation). `monthly_limit_vibe = None`
    /// clears the cap.
    pub async fn upsert(
        pool: &SqlitePool,
        input: &UpsertOrgMemberVibeLimit,
    ) -> Result<Self, sqlx::Error> {
        let user_uuid =
            uuid::Uuid::parse_str(&input.user_id).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
        sqlx::query(
            "INSERT INTO org_member_vibe_limits
                (organization_id, user_id, monthly_limit_vibe)
             VALUES (?, ?, ?)
             ON CONFLICT(organization_id, user_id) DO UPDATE SET
                monthly_limit_vibe = excluded.monthly_limit_vibe,
                updated_at = datetime('now','subsec')",
        )
        .bind(&input.organization_id)
        .bind(user_uuid)
        .bind(input.monthly_limit_vibe)
        .execute(pool)
        .await?;
        Self::find(pool, &input.organization_id, &input.user_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// List all limit rows for an organization (admin view).
    pub async fn list_for_org(pool: &SqlitePool, org_id: &str) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, OrgMemberVibeLimit>(
            "SELECT organization_id, user_id, monthly_limit_vibe, current_period_spent,
                    period_start, created_at, updated_at
             FROM org_member_vibe_limits
             WHERE organization_id = ?
             ORDER BY created_at DESC",
        )
        .bind(org_id)
        .fetch_all(pool)
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    async fn setup() -> (SqlitePool, String, String) {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:?cache=shared")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE org_member_vibe_limits (
                organization_id          TEXT NOT NULL,
                user_id                  BLOB NOT NULL,
                monthly_limit_vibe       INTEGER,
                current_period_spent     INTEGER NOT NULL DEFAULT 0,
                period_start             TEXT NOT NULL DEFAULT (datetime('now','start of month')),
                created_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
                updated_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
                PRIMARY KEY (organization_id, user_id)
            );",
        )
        .execute(&pool)
        .await
        .unwrap();
        let org = "org-test".to_string();
        let user = uuid::Uuid::new_v4().to_string();
        (pool, org, user)
    }

    #[tokio::test]
    async fn no_row_means_uncapped() {
        let (pool, org, user) = setup().await;
        // try_bump on a missing row: SQLite UPDATE on no rows returns 0 affected.
        let ok = OrgMemberVibeLimit::try_bump_spent(&pool, &org, &user, 100)
            .await
            .unwrap();
        assert!(!ok, "no row should mean update affects 0 rows");
        // Caller is expected to interpret None as "uncapped" — we don't enforce a cap.
    }

    #[tokio::test]
    async fn ensure_row_makes_uncapped_then_try_bump_succeeds() {
        let (pool, org, user) = setup().await;
        OrgMemberVibeLimit::ensure_row(&pool, &org, &user)
            .await
            .unwrap();
        let ok = OrgMemberVibeLimit::try_bump_spent(&pool, &org, &user, 5000)
            .await
            .unwrap();
        assert!(ok);
        let row = OrgMemberVibeLimit::find(&pool, &org, &user)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.current_period_spent, 5000);
        assert!(row.monthly_limit_vibe.is_none()); // uncapped
    }

    #[tokio::test]
    async fn capped_user_blocks_when_cap_would_be_exceeded() {
        let (pool, org, user) = setup().await;
        OrgMemberVibeLimit::upsert(
            &pool,
            &UpsertOrgMemberVibeLimit {
                organization_id: org.clone(),
                user_id: user.clone(),
                monthly_limit_vibe: Some(100),
            },
        )
        .await
        .unwrap();
        // First charge within cap succeeds.
        let ok = OrgMemberVibeLimit::try_bump_spent(&pool, &org, &user, 80)
            .await
            .unwrap();
        assert!(ok);
        // Second charge would push spent to 180 > 100 — blocked.
        let blocked = OrgMemberVibeLimit::try_bump_spent(&pool, &org, &user, 30)
            .await
            .unwrap();
        assert!(!blocked);
        // Exactly fitting the cap is allowed.
        let exact = OrgMemberVibeLimit::try_bump_spent(&pool, &org, &user, 20)
            .await
            .unwrap();
        assert!(exact);
        let row = OrgMemberVibeLimit::find(&pool, &org, &user)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.current_period_spent, 100);
    }

    #[tokio::test]
    async fn decrement_clamps_at_zero() {
        let (pool, org, user) = setup().await;
        OrgMemberVibeLimit::upsert(
            &pool,
            &UpsertOrgMemberVibeLimit {
                organization_id: org.clone(),
                user_id: user.clone(),
                monthly_limit_vibe: Some(100),
            },
        )
        .await
        .unwrap();
        let _ = OrgMemberVibeLimit::try_bump_spent(&pool, &org, &user, 30).await;
        OrgMemberVibeLimit::decrement_spent(&pool, &org, &user, 100)
            .await
            .unwrap();
        let row = OrgMemberVibeLimit::find(&pool, &org, &user)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.current_period_spent, 0);
    }

    #[tokio::test]
    async fn upsert_clears_cap_when_passed_none() {
        let (pool, org, user) = setup().await;
        OrgMemberVibeLimit::upsert(
            &pool,
            &UpsertOrgMemberVibeLimit {
                organization_id: org.clone(),
                user_id: user.clone(),
                monthly_limit_vibe: Some(100),
            },
        )
        .await
        .unwrap();
        OrgMemberVibeLimit::upsert(
            &pool,
            &UpsertOrgMemberVibeLimit {
                organization_id: org.clone(),
                user_id: user.clone(),
                monthly_limit_vibe: None,
            },
        )
        .await
        .unwrap();
        let row = OrgMemberVibeLimit::find(&pool, &org, &user)
            .await
            .unwrap()
            .unwrap();
        assert!(row.monthly_limit_vibe.is_none());
    }

    #[allow(dead_code)]
    fn _silence_unused_import_warning() {
        // FromStr is used implicitly by Uuid parsing; this prevents the warning
        // when running the test suite with module-level imports.
        let _ = uuid::Uuid::from_str("00000000-0000-0000-0000-000000000000");
    }
}
