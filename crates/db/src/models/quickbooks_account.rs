use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum QuickBooksAccountError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("QuickBooks account not found")]
    NotFound,
    #[error("Account already exists for this organization and realm")]
    AlreadyExists,
    #[error("Token expired")]
    TokenExpired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum QBEnvironment {
    Sandbox,
    Production,
}

impl std::fmt::Display for QBEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QBEnvironment::Sandbox => write!(f, "sandbox"),
            QBEnvironment::Production => write!(f, "production"),
        }
    }
}

impl std::str::FromStr for QBEnvironment {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sandbox" => Ok(QBEnvironment::Sandbox),
            "production" => Ok(QBEnvironment::Production),
            _ => Err(format!("Unknown QB environment: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum QBAccountStatus {
    Active,
    Inactive,
    Expired,
    Error,
    PendingAuth,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct QuickBooksAccount {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub realm_id: String,
    pub company_name: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub environment: String,
    pub sync_enabled: i32,
    pub sync_frequency_minutes: i32,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_invoices: i32,
    pub sync_customers: i32,
    pub sync_payments: i32,
    pub sync_expenses: i32,
    pub sync_time_tracking: i32,
    pub status: String,
    pub last_error: Option<String>,
    pub metadata: Option<String>,
    pub connected_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateQuickBooksAccount {
    pub organization_id: Uuid,
    pub realm_id: String,
    pub company_name: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub environment: Option<QBEnvironment>,
    pub connected_by: Option<String>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateQuickBooksAccount {
    pub company_name: Option<String>,
    pub sync_enabled: Option<bool>,
    pub sync_frequency_minutes: Option<i32>,
    pub sync_invoices: Option<bool>,
    pub sync_customers: Option<bool>,
    pub sync_payments: Option<bool>,
    pub sync_expenses: Option<bool>,
    pub sync_time_tracking: Option<bool>,
    pub status: Option<QBAccountStatus>,
    pub last_error: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Sync scope configuration for display
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct QBSyncScope {
    pub invoices: bool,
    pub customers: bool,
    pub payments: bool,
    pub expenses: bool,
    pub time_tracking: bool,
}

impl QuickBooksAccount {
    pub fn sync_scope(&self) -> QBSyncScope {
        QBSyncScope {
            invoices: self.sync_invoices == 1,
            customers: self.sync_customers == 1,
            payments: self.sync_payments == 1,
            expenses: self.sync_expenses == 1,
            time_tracking: self.sync_time_tracking == 1,
        }
    }

    /// QBO API base URL for this account's environment
    pub fn api_base_url(&self) -> String {
        if self.environment == "sandbox" {
            format!(
                "https://sandbox-quickbooks.api.intuit.com/v3/company/{}",
                self.realm_id
            )
        } else {
            format!(
                "https://quickbooks.api.intuit.com/v3/company/{}",
                self.realm_id
            )
        }
    }

    /// Check if the token needs refreshing (expired or expires within 5 minutes)
    pub fn needs_token_refresh(&self) -> bool {
        match &self.token_expires_at {
            Some(expires) => {
                let buffer = chrono::Duration::minutes(5);
                *expires <= Utc::now() + buffer
            }
            None => false,
        }
    }

    /// QuickBooks Online OAuth 2.0 scopes
    pub fn oauth_scopes() -> Vec<&'static str> {
        vec![
            "com.intuit.quickbooks.accounting",
        ]
    }

    pub async fn create(
        pool: &SqlitePool,
        data: CreateQuickBooksAccount,
    ) -> Result<Self, QuickBooksAccountError> {
        let id = Uuid::new_v4();
        let environment = data
            .environment
            .unwrap_or(QBEnvironment::Sandbox)
            .to_string();

        let account = sqlx::query_as::<_, QuickBooksAccount>(
            r#"
            INSERT INTO quickbooks_accounts (
                id, organization_id, realm_id, company_name,
                access_token, refresh_token, token_expires_at,
                environment, connected_by
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.organization_id)
        .bind(&data.realm_id)
        .bind(&data.company_name)
        .bind(&data.access_token)
        .bind(&data.refresh_token)
        .bind(data.token_expires_at)
        .bind(&environment)
        .bind(&data.connected_by)
        .fetch_one(pool)
        .await?;

        Ok(account)
    }

    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Self, QuickBooksAccountError> {
        sqlx::query_as::<_, QuickBooksAccount>(
            r#"SELECT * FROM quickbooks_accounts WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(QuickBooksAccountError::NotFound)
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, QuickBooksAccountError> {
        let accounts = sqlx::query_as::<_, QuickBooksAccount>(
            r#"SELECT * FROM quickbooks_accounts WHERE organization_id = ?1 ORDER BY created_at DESC"#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_by_realm(
        pool: &SqlitePool,
        organization_id: Uuid,
        realm_id: &str,
    ) -> Result<Option<Self>, QuickBooksAccountError> {
        let account = sqlx::query_as::<_, QuickBooksAccount>(
            r#"SELECT * FROM quickbooks_accounts WHERE organization_id = ?1 AND realm_id = ?2"#,
        )
        .bind(organization_id)
        .bind(realm_id)
        .fetch_optional(pool)
        .await?;

        Ok(account)
    }

    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, QuickBooksAccountError> {
        let accounts = sqlx::query_as::<_, QuickBooksAccount>(
            r#"SELECT * FROM quickbooks_accounts WHERE status = 'active' AND sync_enabled = 1"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_needs_sync(pool: &SqlitePool) -> Result<Vec<Self>, QuickBooksAccountError> {
        let accounts = sqlx::query_as::<_, QuickBooksAccount>(
            r#"
            SELECT * FROM quickbooks_accounts
            WHERE status = 'active'
            AND sync_enabled = 1
            AND (
                last_sync_at IS NULL
                OR datetime(last_sync_at, '+' || sync_frequency_minutes || ' minutes') < datetime('now')
            )
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateQuickBooksAccount,
    ) -> Result<Self, QuickBooksAccountError> {
        let status = data.status.map(|s| format!("{:?}", s).to_lowercase());
        let metadata = data.metadata.map(|v| v.to_string());
        let sync_enabled = data.sync_enabled.map(|b| if b { 1 } else { 0 });
        let sync_invoices = data.sync_invoices.map(|b| if b { 1 } else { 0 });
        let sync_customers = data.sync_customers.map(|b| if b { 1 } else { 0 });
        let sync_payments = data.sync_payments.map(|b| if b { 1 } else { 0 });
        let sync_expenses = data.sync_expenses.map(|b| if b { 1 } else { 0 });
        let sync_time_tracking = data.sync_time_tracking.map(|b| if b { 1 } else { 0 });

        sqlx::query_as::<_, QuickBooksAccount>(
            r#"
            UPDATE quickbooks_accounts SET
                company_name = COALESCE(?2, company_name),
                sync_enabled = COALESCE(?3, sync_enabled),
                sync_frequency_minutes = COALESCE(?4, sync_frequency_minutes),
                sync_invoices = COALESCE(?5, sync_invoices),
                sync_customers = COALESCE(?6, sync_customers),
                sync_payments = COALESCE(?7, sync_payments),
                sync_expenses = COALESCE(?8, sync_expenses),
                sync_time_tracking = COALESCE(?9, sync_time_tracking),
                status = COALESCE(?10, status),
                last_error = ?11,
                metadata = COALESCE(?12, metadata),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.company_name)
        .bind(sync_enabled)
        .bind(data.sync_frequency_minutes)
        .bind(sync_invoices)
        .bind(sync_customers)
        .bind(sync_payments)
        .bind(sync_expenses)
        .bind(sync_time_tracking)
        .bind(&status)
        .bind(&data.last_error)
        .bind(metadata)
        .fetch_optional(pool)
        .await?
        .ok_or(QuickBooksAccountError::NotFound)
    }

    pub async fn update_tokens(
        pool: &SqlitePool,
        id: Uuid,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), QuickBooksAccountError> {
        sqlx::query(
            r#"
            UPDATE quickbooks_accounts SET
                access_token = ?2,
                refresh_token = COALESCE(?3, refresh_token),
                token_expires_at = ?4,
                status = 'active',
                last_error = NULL,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(access_token)
        .bind(refresh_token)
        .bind(expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_sync_status(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
    ) -> Result<(), QuickBooksAccountError> {
        sqlx::query(
            r#"
            UPDATE quickbooks_accounts SET
                last_sync_at = datetime('now', 'subsec'),
                status = ?2,
                last_error = NULL,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(status)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn set_error(
        pool: &SqlitePool,
        id: Uuid,
        error: &str,
    ) -> Result<(), QuickBooksAccountError> {
        sqlx::query(
            r#"
            UPDATE quickbooks_accounts SET
                status = 'error',
                last_error = ?2,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), QuickBooksAccountError> {
        let result = sqlx::query(r#"DELETE FROM quickbooks_accounts WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(QuickBooksAccountError::NotFound);
        }

        Ok(())
    }
}

/// Mapping between PCG entities and QuickBooks Online entities
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct QuickBooksEntityMap {
    pub id: Uuid,
    pub quickbooks_account_id: Uuid,
    pub pcg_entity_type: String,
    pub pcg_entity_id: String,
    pub qbo_entity_type: String,
    pub qbo_entity_id: String,
    pub qbo_sync_token: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub sync_direction: String,
    pub sync_status: String,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl QuickBooksEntityMap {
    pub async fn create(
        pool: &SqlitePool,
        account_id: Uuid,
        pcg_entity_type: &str,
        pcg_entity_id: &str,
        qbo_entity_type: &str,
        qbo_entity_id: &str,
        sync_direction: &str,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, QuickBooksEntityMap>(
            r#"
            INSERT INTO quickbooks_entity_map (
                id, quickbooks_account_id, pcg_entity_type, pcg_entity_id,
                qbo_entity_type, qbo_entity_id, sync_direction
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(account_id)
        .bind(pcg_entity_type)
        .bind(pcg_entity_id)
        .bind(qbo_entity_type)
        .bind(qbo_entity_id)
        .bind(sync_direction)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_pcg_entity(
        pool: &SqlitePool,
        account_id: Uuid,
        pcg_entity_type: &str,
        pcg_entity_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, QuickBooksEntityMap>(
            r#"
            SELECT * FROM quickbooks_entity_map
            WHERE quickbooks_account_id = ?1 AND pcg_entity_type = ?2 AND pcg_entity_id = ?3
            "#,
        )
        .bind(account_id)
        .bind(pcg_entity_type)
        .bind(pcg_entity_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_qbo_entity(
        pool: &SqlitePool,
        account_id: Uuid,
        qbo_entity_type: &str,
        qbo_entity_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, QuickBooksEntityMap>(
            r#"
            SELECT * FROM quickbooks_entity_map
            WHERE quickbooks_account_id = ?1 AND qbo_entity_type = ?2 AND qbo_entity_id = ?3
            "#,
        )
        .bind(account_id)
        .bind(qbo_entity_type)
        .bind(qbo_entity_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn update_sync_status(
        pool: &SqlitePool,
        id: Uuid,
        sync_status: &str,
        qbo_sync_token: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE quickbooks_entity_map SET
                sync_status = ?2,
                qbo_sync_token = COALESCE(?3, qbo_sync_token),
                last_synced_at = datetime('now', 'subsec'),
                last_error = NULL,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(sync_status)
        .bind(qbo_sync_token)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn list_for_account(
        pool: &SqlitePool,
        account_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, QuickBooksEntityMap>(
            r#"SELECT * FROM quickbooks_entity_map WHERE quickbooks_account_id = ?1 ORDER BY created_at DESC"#,
        )
        .bind(account_id)
        .fetch_all(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(r#"DELETE FROM quickbooks_entity_map WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_utils::setup_test_pool;

    #[tokio::test]
    async fn create_and_query_quickbooks_account() {
        let pool = setup_test_pool().await;

        let org_id = Uuid::new_v4();
        let created = QuickBooksAccount::create(
            &pool,
            CreateQuickBooksAccount {
                organization_id: org_id,
                realm_id: "1234567890".into(),
                company_name: Some("Test Company".into()),
                access_token: Some("access_token_123".into()),
                refresh_token: Some("refresh_token_123".into()),
                token_expires_at: None,
                environment: Some(QBEnvironment::Sandbox),
                connected_by: Some("user_123".into()),
            },
        )
        .await
        .expect("failed to create QB account");

        assert_eq!(created.realm_id, "1234567890");
        assert_eq!(created.environment, "sandbox");
        assert_eq!(created.status, "active");
        assert_eq!(created.sync_enabled, 1);

        let fetched = QuickBooksAccount::find_by_id(&pool, created.id)
            .await
            .expect("account missing");
        assert_eq!(fetched.company_name.as_deref(), Some("Test Company"));

        let by_org = QuickBooksAccount::find_by_organization(&pool, org_id)
            .await
            .expect("org lookup failed");
        assert_eq!(by_org.len(), 1);

        let by_realm = QuickBooksAccount::find_by_realm(&pool, org_id, "1234567890")
            .await
            .expect("realm lookup failed");
        assert!(by_realm.is_some());

        assert_eq!(
            created.api_base_url(),
            "https://sandbox-quickbooks.api.intuit.com/v3/company/1234567890"
        );
    }

    #[tokio::test]
    async fn update_tokens_and_delete() {
        let pool = setup_test_pool().await;

        let org_id = Uuid::new_v4();
        let account = QuickBooksAccount::create(
            &pool,
            CreateQuickBooksAccount {
                organization_id: org_id,
                realm_id: "9876543210".into(),
                company_name: None,
                access_token: Some("old_token".into()),
                refresh_token: Some("old_refresh".into()),
                token_expires_at: None,
                environment: None,
                connected_by: None,
            },
        )
        .await
        .expect("failed to create");

        QuickBooksAccount::update_tokens(
            &pool,
            account.id,
            "new_access_token",
            Some("new_refresh_token"),
            None,
        )
        .await
        .expect("token update failed");

        let refreshed = QuickBooksAccount::find_by_id(&pool, account.id)
            .await
            .expect("lookup failed");
        assert_eq!(refreshed.access_token.as_deref(), Some("new_access_token"));
        assert_eq!(
            refreshed.refresh_token.as_deref(),
            Some("new_refresh_token")
        );

        QuickBooksAccount::set_error(&pool, account.id, "token revoked")
            .await
            .expect("set error failed");

        let errored = QuickBooksAccount::find_by_id(&pool, account.id)
            .await
            .expect("lookup failed");
        assert_eq!(errored.status, "error");
        assert_eq!(errored.last_error.as_deref(), Some("token revoked"));

        QuickBooksAccount::delete(&pool, account.id)
            .await
            .expect("delete failed");

        let lookup = QuickBooksAccount::find_by_id(&pool, account.id).await;
        assert!(matches!(lookup, Err(QuickBooksAccountError::NotFound)));
    }
}
