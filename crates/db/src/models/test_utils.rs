use std::str::FromStr;

use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};
use uuid::Uuid;

use super::project::{CreateProject, Project};
use super::social_account::{CreateSocialAccount, SocialAccount, SocialPlatform};

pub(crate) async fn setup_test_pool() -> SqlitePool {
    let options = SqliteConnectOptions::from_str("sqlite::memory:?cache=shared")
        .expect("invalid sqlite config")
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("failed to open sqlite memory db");

    bootstrap_schema(&pool).await;

    pool
}

async fn bootstrap_schema(pool: &SqlitePool) {
    let statements = [
        r#"
        CREATE TABLE IF NOT EXISTS projects (
            id BLOB PRIMARY KEY,
            name TEXT NOT NULL,
            git_repo_path TEXT NOT NULL,
            setup_script TEXT,
            dev_script TEXT,
            cleanup_script TEXT,
            copy_files TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id BLOB PRIMARY KEY,
            project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            title TEXT,
            description TEXT,
            status TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id BLOB PRIMARY KEY,
            name TEXT
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS social_accounts (
            id BLOB PRIMARY KEY,
            project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            platform TEXT NOT NULL,
            account_type TEXT NOT NULL DEFAULT 'personal',
            platform_account_id TEXT NOT NULL,
            username TEXT,
            display_name TEXT,
            profile_url TEXT,
            avatar_url TEXT,
            access_token TEXT,
            refresh_token TEXT,
            token_expires_at TEXT,
            follower_count INTEGER,
            following_count INTEGER,
            post_count INTEGER,
            metadata TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            last_sync_at TEXT,
            last_error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS social_posts (
            id BLOB PRIMARY KEY,
            project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            social_account_id BLOB REFERENCES social_accounts(id) ON DELETE SET NULL,
            task_id BLOB REFERENCES tasks(id) ON DELETE SET NULL,
            content_type TEXT NOT NULL DEFAULT 'post',
            caption TEXT,
            content_blocks TEXT,
            media_urls TEXT,
            hashtags TEXT,
            mentions TEXT,
            platforms TEXT NOT NULL,
            platform_specific TEXT,
            status TEXT NOT NULL DEFAULT 'draft',
            scheduled_for TEXT,
            published_at TEXT,
            category TEXT,
            queue_position INTEGER,
            is_evergreen INTEGER NOT NULL DEFAULT 0,
            recycle_after_days INTEGER,
            last_recycled_at TEXT,
            created_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
            approved_by TEXT,
            approved_at TEXT,
            platform_post_id TEXT,
            platform_url TEXT,
            publish_error TEXT,
            impressions INTEGER DEFAULT 0,
            reach INTEGER DEFAULT 0,
            likes INTEGER DEFAULT 0,
            comments INTEGER DEFAULT 0,
            shares INTEGER DEFAULT 0,
            saves INTEGER DEFAULT 0,
            clicks INTEGER DEFAULT 0,
            engagement_rate REAL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS social_mentions (
            id BLOB PRIMARY KEY,
            social_account_id BLOB NOT NULL REFERENCES social_accounts(id) ON DELETE CASCADE,
            project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            mention_type TEXT NOT NULL,
            platform TEXT NOT NULL,
            platform_mention_id TEXT NOT NULL,
            author_username TEXT,
            author_display_name TEXT,
            author_avatar_url TEXT,
            author_follower_count INTEGER,
            author_is_verified INTEGER DEFAULT 0,
            content TEXT,
            media_urls TEXT,
            parent_post_id BLOB REFERENCES social_posts(id) ON DELETE SET NULL,
            parent_platform_id TEXT,
            status TEXT NOT NULL DEFAULT 'unread',
            sentiment TEXT,
            priority TEXT DEFAULT 'normal',
            replied_at TEXT,
            replied_by TEXT,
            reply_content TEXT,
            assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
            auto_response_sent INTEGER DEFAULT 0,
            received_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
        );
        "#,
        r#"
        CREATE TABLE IF NOT EXISTS email_accounts (
            id BLOB PRIMARY KEY,
            project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            provider TEXT NOT NULL,
            account_type TEXT NOT NULL DEFAULT 'primary',
            email_address TEXT NOT NULL,
            display_name TEXT,
            avatar_url TEXT,
            access_token TEXT,
            refresh_token TEXT,
            token_expires_at TEXT,
            imap_host TEXT,
            imap_port INTEGER,
            smtp_host TEXT,
            smtp_port INTEGER,
            use_ssl INTEGER DEFAULT 1,
            granted_scopes TEXT,
            storage_used_bytes INTEGER,
            storage_total_bytes INTEGER,
            unread_count INTEGER DEFAULT 0,
            metadata TEXT,
            status TEXT NOT NULL DEFAULT 'active',
            last_sync_at TEXT,
            last_error TEXT,
            sync_enabled INTEGER DEFAULT 1,
            sync_frequency_minutes INTEGER DEFAULT 15,
            auto_reply_enabled INTEGER DEFAULT 0,
            signature TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
            UNIQUE(project_id, provider, email_address)
        );
        "#,
    ];

    for statement in statements {
        sqlx::query(statement)
            .execute(pool)
            .await
            .expect("failed to bootstrap schema");
    }
}

pub(crate) async fn create_test_project(pool: &SqlitePool) -> Uuid {
    let project_id = Uuid::new_v4();
    let data = CreateProject {
        name: format!("Test Project {}", project_id),
        git_repo_path: format!("/tmp/{}", project_id),
        use_existing_repo: true,
        setup_script: None,
        dev_script: None,
        cleanup_script: None,
        copy_files: None,
    };

    Project::create(pool, &data, project_id)
        .await
        .expect("failed to create test project");

    project_id
}

pub(crate) async fn create_test_social_account(
    pool: &SqlitePool,
    project_id: Uuid,
) -> SocialAccount {
    SocialAccount::create(
        pool,
        CreateSocialAccount {
            project_id,
            platform: SocialPlatform::Instagram,
            account_type: None,
            platform_account_id: format!("acct-{}", Uuid::new_v4()),
            username: Some("test_handle".into()),
            display_name: Some("Test Handle".into()),
            profile_url: Some("https://social.example/test".into()),
            avatar_url: None,
            access_token: Some("token".into()),
            refresh_token: None,
            token_expires_at: None,
            metadata: None,
        },
    )
    .await
    .expect("failed to create test social account")
}
