use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SocialAccountError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Social account not found")]
    NotFound,
    #[error("Account already exists")]
    AlreadyExists,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SocialPlatform {
    Instagram,
    LinkedIn,
    Twitter,
    TikTok,
    YouTube,
    Facebook,
    Threads,
    Bluesky,
    Pinterest,
}

impl std::fmt::Display for SocialPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SocialPlatform::Instagram => "instagram",
            SocialPlatform::LinkedIn => "linkedin",
            SocialPlatform::Twitter => "twitter",
            SocialPlatform::TikTok => "tiktok",
            SocialPlatform::YouTube => "youtube",
            SocialPlatform::Facebook => "facebook",
            SocialPlatform::Threads => "threads",
            SocialPlatform::Bluesky => "bluesky",
            SocialPlatform::Pinterest => "pinterest",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for SocialPlatform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "instagram" => Ok(SocialPlatform::Instagram),
            "linkedin" => Ok(SocialPlatform::LinkedIn),
            "twitter" | "x" => Ok(SocialPlatform::Twitter),
            "tiktok" => Ok(SocialPlatform::TikTok),
            "youtube" => Ok(SocialPlatform::YouTube),
            "facebook" => Ok(SocialPlatform::Facebook),
            "threads" => Ok(SocialPlatform::Threads),
            "bluesky" => Ok(SocialPlatform::Bluesky),
            "pinterest" => Ok(SocialPlatform::Pinterest),
            _ => Err(format!("Unknown platform: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    Personal,
    Business,
    Creator,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    Active,
    Inactive,
    Expired,
    Error,
    PendingAuth,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SocialAccount {
    pub id: Uuid,
    pub project_id: Uuid,
    pub platform: String,
    pub account_type: String,
    pub platform_account_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub profile_url: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub follower_count: Option<i64>,
    pub following_count: Option<i64>,
    pub post_count: Option<i64>,
    pub metadata: Option<String>,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSocialAccount {
    pub project_id: Uuid,
    pub platform: SocialPlatform,
    pub account_type: Option<AccountType>,
    pub platform_account_id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub profile_url: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateSocialAccount {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub profile_url: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub follower_count: Option<i32>,
    pub following_count: Option<i32>,
    pub post_count: Option<i32>,
    pub metadata: Option<serde_json::Value>,
    pub status: Option<AccountStatus>,
    pub last_error: Option<String>,
}

impl SocialAccount {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateSocialAccount,
    ) -> Result<Self, SocialAccountError> {
        let id = Uuid::new_v4();
        let platform = data.platform.to_string();
        let account_type = data
            .account_type
            .map(|t| format!("{:?}", t).to_lowercase())
            .unwrap_or_else(|| "personal".to_string());
        let metadata = data.metadata.map(|v| v.to_string());

        let account = sqlx::query_as::<_, SocialAccount>(
            r#"
            INSERT INTO social_accounts (
                id, project_id, platform, account_type, platform_account_id,
                username, display_name, profile_url, avatar_url,
                access_token, refresh_token, token_expires_at, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&platform)
        .bind(&account_type)
        .bind(&data.platform_account_id)
        .bind(&data.username)
        .bind(&data.display_name)
        .bind(&data.profile_url)
        .bind(&data.avatar_url)
        .bind(&data.access_token)
        .bind(&data.refresh_token)
        .bind(data.token_expires_at)
        .bind(metadata)
        .fetch_one(pool)
        .await?;

        Ok(account)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, SocialAccountError> {
        sqlx::query_as::<_, SocialAccount>(
            r#"SELECT * FROM social_accounts WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(SocialAccountError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, SocialAccountError> {
        let accounts = sqlx::query_as::<_, SocialAccount>(
            r#"SELECT * FROM social_accounts WHERE project_id = ?1 ORDER BY platform, username"#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_by_platform(
        pool: &SqlitePool,
        project_id: Uuid,
        platform: SocialPlatform,
    ) -> Result<Vec<Self>, SocialAccountError> {
        let platform_str = platform.to_string();
        let accounts = sqlx::query_as::<_, SocialAccount>(
            r#"SELECT * FROM social_accounts WHERE project_id = ?1 AND platform = ?2"#,
        )
        .bind(project_id)
        .bind(&platform_str)
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, SocialAccountError> {
        let accounts = sqlx::query_as::<_, SocialAccount>(
            r#"SELECT * FROM social_accounts WHERE status = 'active'"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(accounts)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateSocialAccount,
    ) -> Result<Self, SocialAccountError> {
        let status = data.status.map(|s| format!("{:?}", s).to_lowercase());
        let metadata = data.metadata.map(|v| v.to_string());

        sqlx::query_as::<_, SocialAccount>(
            r#"
            UPDATE social_accounts SET
                username = COALESCE(?2, username),
                display_name = COALESCE(?3, display_name),
                profile_url = COALESCE(?4, profile_url),
                avatar_url = COALESCE(?5, avatar_url),
                access_token = COALESCE(?6, access_token),
                refresh_token = COALESCE(?7, refresh_token),
                token_expires_at = COALESCE(?8, token_expires_at),
                follower_count = COALESCE(?9, follower_count),
                following_count = COALESCE(?10, following_count),
                post_count = COALESCE(?11, post_count),
                metadata = COALESCE(?12, metadata),
                status = COALESCE(?13, status),
                last_error = ?14,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.username)
        .bind(&data.display_name)
        .bind(&data.profile_url)
        .bind(&data.avatar_url)
        .bind(&data.access_token)
        .bind(&data.refresh_token)
        .bind(data.token_expires_at)
        .bind(data.follower_count)
        .bind(data.following_count)
        .bind(data.post_count)
        .bind(metadata)
        .bind(&status)
        .bind(&data.last_error)
        .fetch_optional(pool)
        .await?
        .ok_or(SocialAccountError::NotFound)
    }

    pub async fn update_sync_status(
        pool: &SqlitePool,
        id: Uuid,
        status: &str,
        sync_time: Option<DateTime<Utc>>,
    ) -> Result<(), SocialAccountError> {
        sqlx::query(
            r#"
            UPDATE social_accounts SET
                last_sync_at = COALESCE(?2, datetime('now', 'subsec')),
                status = ?3,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(sync_time)
        .bind(status)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), SocialAccountError> {
        let result = sqlx::query(r#"DELETE FROM social_accounts WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(SocialAccountError::NotFound);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_utils::{create_test_project, setup_test_pool};
    use chrono::Utc;

    #[tokio::test]
    async fn create_and_query_accounts() {
        let pool = setup_test_pool().await;
        let project_id = create_test_project(&pool).await;

        let created = SocialAccount::create(
            &pool,
            CreateSocialAccount {
                project_id,
                platform: SocialPlatform::LinkedIn,
                account_type: Some(AccountType::Business),
                platform_account_id: "acc_123".into(),
                username: Some("primehospitality".into()),
                display_name: Some("Prime Hospitality".into()),
                profile_url: Some("https://linkedin.com/company/prime".into()),
                avatar_url: None,
                access_token: Some("token".into()),
                refresh_token: None,
                token_expires_at: None,
                metadata: None,
            },
        )
        .await
        .expect("failed to create social account");

        assert_eq!(created.platform, "linkedin");
        assert_eq!(created.status, "active");

        let fetched = SocialAccount::find_by_id(&pool, created.id)
            .await
            .expect("account missing");
        assert_eq!(fetched.display_name.as_deref(), Some("Prime Hospitality"));

        let by_project = SocialAccount::find_by_project(&pool, project_id)
            .await
            .expect("project lookup failed");
        assert_eq!(by_project.len(), 1);

        let by_platform = SocialAccount::find_by_platform(&pool, project_id, SocialPlatform::LinkedIn)
            .await
            .expect("platform lookup failed");
        assert_eq!(by_platform.len(), 1);

        let active = SocialAccount::find_active(&pool)
            .await
            .expect("active lookup failed");
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn update_sync_and_delete_account() {
        let pool = setup_test_pool().await;
        let project_id = create_test_project(&pool).await;

        let account = SocialAccount::create(
            &pool,
            CreateSocialAccount {
                project_id,
                platform: SocialPlatform::Instagram,
                account_type: Some(AccountType::Creator),
                platform_account_id: "acct_to_update".into(),
                username: Some("social_original".into()),
                display_name: Some("Social Original".into()),
                profile_url: Some("https://instagram.com/social".into()),
                avatar_url: None,
                access_token: Some("initial".into()),
                refresh_token: Some("refresh".into()),
                token_expires_at: None,
                metadata: None,
            },
        )
        .await
        .expect("failed to create social account");

        let updated = SocialAccount::update(
            &pool,
            account.id,
            UpdateSocialAccount {
                username: Some("social_new".into()),
                follower_count: Some(4_200),
                status: Some(AccountStatus::Inactive),
                last_error: Some("token expired".into()),
                ..Default::default()
            },
        )
        .await
        .expect("update failed");

        assert_eq!(updated.username.as_deref(), Some("social_new"));
        assert_eq!(updated.status, "inactive");
        assert_eq!(updated.follower_count, Some(4_200));
        assert_eq!(updated.last_error.as_deref(), Some("token expired"));

        SocialAccount::update_sync_status(&pool, account.id, "error", Some(Utc::now()))
            .await
            .expect("sync update failed");

        let synced = SocialAccount::find_by_id(&pool, account.id)
            .await
            .expect("lookup failed");
        assert_eq!(synced.status, "error");
        assert!(synced.last_sync_at.is_some());

        SocialAccount::delete(&pool, account.id)
            .await
            .expect("delete failed");

        let lookup = SocialAccount::find_by_id(&pool, account.id).await;
        assert!(matches!(lookup, Err(SocialAccountError::NotFound)));
    }
}
