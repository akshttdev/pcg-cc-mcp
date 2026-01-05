use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum EmailAccountError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Email account not found")]
    NotFound,
    #[error("Account already exists")]
    AlreadyExists,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum EmailProvider {
    Gmail,
    Zoho,
    Outlook,
    ImapCustom,
}

impl std::fmt::Display for EmailProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EmailProvider::Gmail => "gmail",
            EmailProvider::Zoho => "zoho",
            EmailProvider::Outlook => "outlook",
            EmailProvider::ImapCustom => "imap_custom",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for EmailProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gmail" => Ok(EmailProvider::Gmail),
            "zoho" => Ok(EmailProvider::Zoho),
            "outlook" => Ok(EmailProvider::Outlook),
            "imap_custom" | "custom" | "imap" => Ok(EmailProvider::ImapCustom),
            _ => Err(format!("Unknown email provider: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum EmailAccountType {
    Primary,
    Team,
    Notifications,
    Marketing,
    Support,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum EmailAccountStatus {
    Active,
    Inactive,
    Expired,
    Error,
    PendingAuth,
    Revoked,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailAccount {
    pub id: Uuid,
    pub project_id: Uuid,
    pub provider: String,
    pub account_type: String,
    pub email_address: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub imap_host: Option<String>,
    pub imap_port: Option<i32>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub use_ssl: Option<i32>,
    pub granted_scopes: Option<String>,
    pub storage_used_bytes: Option<i64>,
    pub storage_total_bytes: Option<i64>,
    pub unread_count: Option<i32>,
    pub metadata: Option<String>,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub sync_enabled: Option<i32>,
    pub sync_frequency_minutes: Option<i32>,
    pub auto_reply_enabled: Option<i32>,
    pub signature: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateEmailAccount {
    pub project_id: Uuid,
    pub provider: EmailProvider,
    pub account_type: Option<EmailAccountType>,
    pub email_address: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub imap_host: Option<String>,
    pub imap_port: Option<i32>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub use_ssl: Option<bool>,
    pub granted_scopes: Option<Vec<String>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateEmailAccount {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub imap_host: Option<String>,
    pub imap_port: Option<i32>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<i32>,
    pub use_ssl: Option<bool>,
    pub granted_scopes: Option<Vec<String>>,
    pub storage_used_bytes: Option<i64>,
    pub storage_total_bytes: Option<i64>,
    pub unread_count: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub status: Option<EmailAccountStatus>,
    pub last_error: Option<String>,
    pub sync_enabled: Option<bool>,
    pub sync_frequency_minutes: Option<i32>,
    pub auto_reply_enabled: Option<bool>,
    pub signature: Option<String>,
}

impl EmailAccount {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateEmailAccount,
    ) -> Result<Self, EmailAccountError> {
        let id = Uuid::new_v4();
        let provider = data.provider.to_string();
        let account_type = data
            .account_type
            .map(|t| format!("{:?}", t).to_lowercase())
            .unwrap_or_else(|| "primary".to_string());
        let metadata = data.metadata.map(|v| v.to_string());
        let granted_scopes = data.granted_scopes.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let use_ssl = data.use_ssl.map(|b| if b { 1 } else { 0 });

        let account = sqlx::query_as::<_, EmailAccount>(
            r#"
            INSERT INTO email_accounts (
                id, project_id, provider, account_type, email_address,
                display_name, avatar_url, access_token, refresh_token, token_expires_at,
                imap_host, imap_port, smtp_host, smtp_port, use_ssl,
                granted_scopes, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&provider)
        .bind(&account_type)
        .bind(&data.email_address)
        .bind(&data.display_name)
        .bind(&data.avatar_url)
        .bind(&data.access_token)
        .bind(&data.refresh_token)
        .bind(data.token_expires_at)
        .bind(&data.imap_host)
        .bind(data.imap_port)
        .bind(&data.smtp_host)
        .bind(data.smtp_port)
        .bind(use_ssl)
        .bind(granted_scopes)
        .bind(metadata)
        .fetch_one(pool)
        .await?;

        Ok(account)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, EmailAccountError> {
        sqlx::query_as::<_, EmailAccount>(
            r#"SELECT * FROM email_accounts WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(EmailAccountError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, EmailAccountError> {
        let accounts = sqlx::query_as::<_, EmailAccount>(
            r#"SELECT * FROM email_accounts WHERE project_id = ?1 ORDER BY provider, email_address"#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_by_provider(
        pool: &SqlitePool,
        project_id: Uuid,
        provider: EmailProvider,
    ) -> Result<Vec<Self>, EmailAccountError> {
        let provider_str = provider.to_string();
        let accounts = sqlx::query_as::<_, EmailAccount>(
            r#"SELECT * FROM email_accounts WHERE project_id = ?1 AND provider = ?2"#,
        )
        .bind(project_id)
        .bind(&provider_str)
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_by_email(
        pool: &SqlitePool,
        email_address: &str,
    ) -> Result<Vec<Self>, EmailAccountError> {
        let accounts = sqlx::query_as::<_, EmailAccount>(
            r#"SELECT * FROM email_accounts WHERE email_address = ?1"#,
        )
        .bind(email_address)
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, EmailAccountError> {
        let accounts = sqlx::query_as::<_, EmailAccount>(
            r#"SELECT * FROM email_accounts WHERE status = 'active' AND sync_enabled = 1"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_needs_sync(pool: &SqlitePool) -> Result<Vec<Self>, EmailAccountError> {
        let accounts = sqlx::query_as::<_, EmailAccount>(
            r#"
            SELECT * FROM email_accounts
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
        data: UpdateEmailAccount,
    ) -> Result<Self, EmailAccountError> {
        let status = data.status.map(|s| format!("{:?}", s).to_lowercase());
        let metadata = data.metadata.map(|v| v.to_string());
        let granted_scopes = data.granted_scopes.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let use_ssl = data.use_ssl.map(|b| if b { 1 } else { 0 });
        let sync_enabled = data.sync_enabled.map(|b| if b { 1 } else { 0 });
        let auto_reply_enabled = data.auto_reply_enabled.map(|b| if b { 1 } else { 0 });

        sqlx::query_as::<_, EmailAccount>(
            r#"
            UPDATE email_accounts SET
                display_name = COALESCE(?2, display_name),
                avatar_url = COALESCE(?3, avatar_url),
                access_token = COALESCE(?4, access_token),
                refresh_token = COALESCE(?5, refresh_token),
                token_expires_at = COALESCE(?6, token_expires_at),
                imap_host = COALESCE(?7, imap_host),
                imap_port = COALESCE(?8, imap_port),
                smtp_host = COALESCE(?9, smtp_host),
                smtp_port = COALESCE(?10, smtp_port),
                use_ssl = COALESCE(?11, use_ssl),
                granted_scopes = COALESCE(?12, granted_scopes),
                storage_used_bytes = COALESCE(?13, storage_used_bytes),
                storage_total_bytes = COALESCE(?14, storage_total_bytes),
                unread_count = COALESCE(?15, unread_count),
                metadata = COALESCE(?16, metadata),
                status = COALESCE(?17, status),
                last_error = ?18,
                sync_enabled = COALESCE(?19, sync_enabled),
                sync_frequency_minutes = COALESCE(?20, sync_frequency_minutes),
                auto_reply_enabled = COALESCE(?21, auto_reply_enabled),
                signature = COALESCE(?22, signature),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.display_name)
        .bind(&data.avatar_url)
        .bind(&data.access_token)
        .bind(&data.refresh_token)
        .bind(data.token_expires_at)
        .bind(&data.imap_host)
        .bind(data.imap_port)
        .bind(&data.smtp_host)
        .bind(data.smtp_port)
        .bind(use_ssl)
        .bind(granted_scopes)
        .bind(data.storage_used_bytes)
        .bind(data.storage_total_bytes)
        .bind(data.unread_count)
        .bind(metadata)
        .bind(&status)
        .bind(&data.last_error)
        .bind(sync_enabled)
        .bind(data.sync_frequency_minutes)
        .bind(auto_reply_enabled)
        .bind(&data.signature)
        .fetch_optional(pool)
        .await?
        .ok_or(EmailAccountError::NotFound)
    }

    pub async fn update_sync_status(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
        unread_count: Option<i32>,
    ) -> Result<(), EmailAccountError> {
        sqlx::query(
            r#"
            UPDATE email_accounts SET
                last_sync_at = datetime('now', 'subsec'),
                status = ?2,
                unread_count = COALESCE(?3, unread_count),
                last_error = NULL,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(unread_count)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_tokens(
        pool: &SqlitePool,
        id: Uuid,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), EmailAccountError> {
        sqlx::query(
            r#"
            UPDATE email_accounts SET
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

    pub async fn set_error(
        pool: &SqlitePool,
        id: Uuid,
        error: &str,
    ) -> Result<(), EmailAccountError> {
        sqlx::query(
            r#"
            UPDATE email_accounts SET
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

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), EmailAccountError> {
        let result = sqlx::query(r#"DELETE FROM email_accounts WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(EmailAccountError::NotFound);
        }

        Ok(())
    }

    /// Check if the token needs refreshing (expired or expires within 5 minutes)
    pub fn needs_token_refresh(&self) -> bool {
        match &self.token_expires_at {
            Some(expires) => {
                let buffer = chrono::Duration::minutes(5);
                *expires <= Utc::now() + buffer
            }
            None => false, // No expiry set, assume valid
        }
    }

    /// Get provider-specific OAuth scopes for Gmail
    pub fn gmail_scopes() -> Vec<&'static str> {
        vec![
            "https://www.googleapis.com/auth/gmail.readonly",
            "https://www.googleapis.com/auth/gmail.send",
            "https://www.googleapis.com/auth/gmail.modify",
            "https://www.googleapis.com/auth/userinfo.email",
            "https://www.googleapis.com/auth/userinfo.profile",
        ]
    }

    /// Get provider-specific OAuth scopes for Zoho Mail
    pub fn zoho_mail_scopes() -> Vec<&'static str> {
        vec![
            "ZohoMail.messages.READ",
            "ZohoMail.messages.CREATE",
            "ZohoMail.accounts.READ",
            "ZohoMail.folders.READ",
        ]
    }

    /// Get provider-specific OAuth scopes for Zoho CRM
    pub fn zoho_crm_scopes() -> Vec<&'static str> {
        vec![
            "ZohoCRM.modules.ALL",
            "ZohoCRM.settings.ALL",
            "ZohoCRM.users.READ",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_utils::{create_test_project, setup_test_pool};

    #[tokio::test]
    async fn create_and_query_email_accounts() {
        let pool = setup_test_pool().await;
        let project_id = create_test_project(&pool).await;

        let created = EmailAccount::create(
            &pool,
            CreateEmailAccount {
                project_id,
                provider: EmailProvider::Gmail,
                account_type: Some(EmailAccountType::Primary),
                email_address: "team@example.com".into(),
                display_name: Some("Team Account".into()),
                avatar_url: None,
                access_token: Some("token123".into()),
                refresh_token: Some("refresh123".into()),
                token_expires_at: None,
                imap_host: None,
                imap_port: None,
                smtp_host: None,
                smtp_port: None,
                use_ssl: None,
                granted_scopes: Some(vec!["gmail.readonly".into()]),
                metadata: None,
            },
        )
        .await
        .expect("failed to create email account");

        assert_eq!(created.provider, "gmail");
        assert_eq!(created.status, "active");
        assert_eq!(created.email_address, "team@example.com");

        let fetched = EmailAccount::find_by_id(&pool, created.id)
            .await
            .expect("account missing");
        assert_eq!(fetched.display_name.as_deref(), Some("Team Account"));

        let by_project = EmailAccount::find_by_project(&pool, project_id)
            .await
            .expect("project lookup failed");
        assert_eq!(by_project.len(), 1);

        let by_provider = EmailAccount::find_by_provider(&pool, project_id, EmailProvider::Gmail)
            .await
            .expect("provider lookup failed");
        assert_eq!(by_provider.len(), 1);
    }

    #[tokio::test]
    async fn create_zoho_account() {
        let pool = setup_test_pool().await;
        let project_id = create_test_project(&pool).await;

        let created = EmailAccount::create(
            &pool,
            CreateEmailAccount {
                project_id,
                provider: EmailProvider::Zoho,
                account_type: Some(EmailAccountType::Team),
                email_address: "ops@company.zoho.com".into(),
                display_name: Some("Operations Team".into()),
                avatar_url: None,
                access_token: Some("zoho_token".into()),
                refresh_token: Some("zoho_refresh".into()),
                token_expires_at: None,
                imap_host: None,
                imap_port: None,
                smtp_host: None,
                smtp_port: None,
                use_ssl: None,
                granted_scopes: Some(vec!["ZohoMail.messages.READ".into()]),
                metadata: None,
            },
        )
        .await
        .expect("failed to create zoho account");

        assert_eq!(created.provider, "zoho");
        assert_eq!(created.account_type, "team");
    }

    #[tokio::test]
    async fn update_and_delete_email_account() {
        let pool = setup_test_pool().await;
        let project_id = create_test_project(&pool).await;

        let account = EmailAccount::create(
            &pool,
            CreateEmailAccount {
                project_id,
                provider: EmailProvider::Gmail,
                account_type: None,
                email_address: "test@gmail.com".into(),
                display_name: Some("Test".into()),
                avatar_url: None,
                access_token: Some("initial".into()),
                refresh_token: None,
                token_expires_at: None,
                imap_host: None,
                imap_port: None,
                smtp_host: None,
                smtp_port: None,
                use_ssl: None,
                granted_scopes: None,
                metadata: None,
            },
        )
        .await
        .expect("failed to create");

        let updated = EmailAccount::update(
            &pool,
            account.id,
            UpdateEmailAccount {
                display_name: Some("Updated Name".into()),
                unread_count: Some(42),
                sync_frequency_minutes: Some(30),
                ..Default::default()
            },
        )
        .await
        .expect("update failed");

        assert_eq!(updated.display_name.as_deref(), Some("Updated Name"));
        assert_eq!(updated.unread_count, Some(42));
        assert_eq!(updated.sync_frequency_minutes, Some(30));

        EmailAccount::update_sync_status(&pool, account.id, "active", Some(10))
            .await
            .expect("sync update failed");

        let synced = EmailAccount::find_by_id(&pool, account.id)
            .await
            .expect("lookup failed");
        assert!(synced.last_sync_at.is_some());
        assert_eq!(synced.unread_count, Some(10));

        EmailAccount::delete(&pool, account.id)
            .await
            .expect("delete failed");

        let lookup = EmailAccount::find_by_id(&pool, account.id).await;
        assert!(matches!(lookup, Err(EmailAccountError::NotFound)));
    }
}
