//! GitHub repository ↔ project link.
//!
//! Stores the *identity* of a linked GitHub repository. Auth tokens live on
//! `integration_connections` (provider='github'). Webhook + commit sync use
//! this table to route inbound events to the correct project.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum GitHubRepoLinkError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("github repo link not found")]
    NotFound,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GitHubRepoLink {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub project_id: Uuid,
    #[ts(optional)]
    pub integration_connection_id: Option<Uuid>,
    pub github_repo_id: i64,
    pub owner: String,
    pub repo_name: String,
    pub full_name: String,
    pub default_branch: String,
    #[ts(optional)]
    pub clone_url: Option<String>,
    #[ts(optional)]
    pub ssh_url: Option<String>,
    pub private: bool,
    #[ts(skip)]
    #[serde(skip_serializing)]
    pub webhook_secret: Option<String>,
    #[ts(type = "Date | null", optional)]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[ts(optional)]
    pub last_synced_commit_sha: Option<String>,
    #[ts(optional)]
    pub last_error: Option<String>,
    pub metadata: String,
    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[ts(export)]
pub struct CreateGitHubRepoLink {
    pub organization_id: Uuid,
    pub project_id: Uuid,
    pub integration_connection_id: Option<Uuid>,
    pub github_repo_id: i64,
    pub owner: String,
    pub repo_name: String,
    pub full_name: String,
    pub default_branch: Option<String>,
    pub clone_url: Option<String>,
    pub ssh_url: Option<String>,
    pub private: Option<bool>,
    pub metadata: Option<String>,
}

impl GitHubRepoLink {
    /// Insert or update by (project_id, github_repo_id). Tokens are kept on the
    /// separate IntegrationConnection row — this method only touches identity.
    pub async fn upsert(pool: &SqlitePool, c: CreateGitHubRepoLink) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let default_branch = c.default_branch.unwrap_or_else(|| "main".into());
        let private = c.private.unwrap_or(false);
        let metadata = c.metadata.unwrap_or_else(|| "{}".into());

        sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO github_repo_links (
                id, organization_id, project_id, integration_connection_id,
                github_repo_id, owner, repo_name, full_name, default_branch,
                clone_url, ssh_url, private, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(project_id, github_repo_id) DO UPDATE SET
                integration_connection_id = excluded.integration_connection_id,
                owner                     = excluded.owner,
                repo_name                 = excluded.repo_name,
                full_name                 = excluded.full_name,
                default_branch            = excluded.default_branch,
                clone_url                 = excluded.clone_url,
                ssh_url                   = excluded.ssh_url,
                private                   = excluded.private,
                metadata                  = excluded.metadata,
                updated_at                = datetime('now','subsec')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(c.organization_id)
        .bind(c.project_id)
        .bind(c.integration_connection_id)
        .bind(c.github_repo_id)
        .bind(&c.owner)
        .bind(&c.repo_name)
        .bind(&c.full_name)
        .bind(&default_branch)
        .bind(&c.clone_url)
        .bind(&c.ssh_url)
        .bind(private)
        .bind(&metadata)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM github_repo_links WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM github_repo_links \
             WHERE project_id = ?1 \
             ORDER BY created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_org(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM github_repo_links \
             WHERE organization_id = ?1 \
             ORDER BY created_at DESC",
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    /// Find by GitHub's "owner/repo" key — used by webhook routing.
    pub async fn find_by_full_name(
        pool: &SqlitePool,
        full_name: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM github_repo_links \
             WHERE full_name = ?1",
        )
        .bind(full_name)
        .fetch_all(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM github_repo_links WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn touch_sync(
        pool: &SqlitePool,
        id: Uuid,
        latest_commit_sha: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE github_repo_links SET \
                last_sync_at = datetime('now','subsec'), \
                last_synced_commit_sha = COALESCE(?2, last_synced_commit_sha), \
                last_error = NULL, \
                updated_at = datetime('now','subsec') \
             WHERE id = ?1",
        )
        .bind(id)
        .bind(latest_commit_sha)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn record_error(pool: &SqlitePool, id: Uuid, err: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE github_repo_links SET \
                last_error = ?2, \
                updated_at = datetime('now','subsec') \
             WHERE id = ?1",
        )
        .bind(id)
        .bind(err)
        .execute(pool)
        .await?;
        Ok(())
    }
}
