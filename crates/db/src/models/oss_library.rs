use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct OssLibrary {
    pub id: Uuid,
    pub name: String,
    pub github_owner: String,
    pub github_repo: String,
    pub tracked_version: Option<String>,
    pub latest_version: Option<String>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub check_interval_secs: i64,
    pub is_active: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, TS)]
#[ts(export)]
pub struct OssLibraryUpdate {
    pub id: Uuid,
    pub library_id: Uuid,
    pub version: String,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub significance: String,
    pub agent_recommendation: Option<String>,
    pub recommendation_status: String,
    pub agent_flow_id: Option<Uuid>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl OssLibrary {
    pub async fn list(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, github_owner, github_repo, tracked_version, latest_version,
                    last_checked_at, check_interval_secs, is_active, notes,
                    created_at, updated_at
             FROM oss_libraries ORDER BY name ASC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_due_for_check(pool: &SqlitePool) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, name, github_owner, github_repo, tracked_version, latest_version,
                    last_checked_at, check_interval_secs, is_active, notes,
                    created_at, updated_at
             FROM oss_libraries
             WHERE is_active = 1
               AND (last_checked_at IS NULL
                    OR datetime(last_checked_at, '+' || check_interval_secs || ' seconds') <= datetime('now'))
             ORDER BY last_checked_at ASC NULLS FIRST",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn get(pool: &SqlitePool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as(
            "SELECT id, name, github_owner, github_repo, tracked_version, latest_version,
                    last_checked_at, check_interval_secs, is_active, notes,
                    created_at, updated_at
             FROM oss_libraries WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }
}

impl OssLibraryUpdate {
    pub async fn list_for_library(pool: &SqlitePool, library_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, library_id, version, release_url, release_notes, significance,
                    agent_recommendation, recommendation_status, agent_flow_id,
                    published_at, created_at
             FROM oss_library_updates
             WHERE library_id = ?
             ORDER BY created_at DESC",
        )
        .bind(library_id)
        .fetch_all(pool)
        .await
    }

    pub async fn list_recent(pool: &SqlitePool, limit: i64) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as(
            "SELECT id, library_id, version, release_url, release_notes, significance,
                    agent_recommendation, recommendation_status, agent_flow_id,
                    published_at, created_at
             FROM oss_library_updates
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
