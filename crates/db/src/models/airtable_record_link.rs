use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Origin of the task - where it was created
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum AirtableOrigin {
    /// Task was imported from Airtable
    Airtable,
    /// Task was created in PCG and pushed to Airtable
    Pcg,
}

impl AirtableOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            AirtableOrigin::Airtable => "airtable",
            AirtableOrigin::Pcg => "pcg",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pcg" => AirtableOrigin::Pcg,
            _ => AirtableOrigin::Airtable,
        }
    }
}

/// Sync status for an Airtable record link
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AirtableSyncStatus {
    /// Task and record are in sync
    Synced,
    /// Execution completed, needs to push results to Airtable
    PendingPush,
    /// Record updated on Airtable, needs to pull changes
    PendingPull,
    /// Last sync failed
    Error,
}

impl AirtableSyncStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AirtableSyncStatus::Synced => "synced",
            AirtableSyncStatus::PendingPush => "pending_push",
            AirtableSyncStatus::PendingPull => "pending_pull",
            AirtableSyncStatus::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending_push" => AirtableSyncStatus::PendingPush,
            "pending_pull" => AirtableSyncStatus::PendingPull,
            "error" => AirtableSyncStatus::Error,
            _ => AirtableSyncStatus::Synced,
        }
    }
}

/// Represents a link between a PCG task and an Airtable record
#[derive(Debug, Clone, Serialize, Deserialize, TS, FromRow)]
pub struct AirtableRecordLink {
    pub id: Uuid,
    pub task_id: Uuid,
    pub airtable_record_id: String,
    pub airtable_base_id: String,
    pub airtable_table_id: Option<String>,
    pub origin: String,
    pub sync_status: String,
    pub last_sync_error: Option<String>,
    pub airtable_record_url: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AirtableRecordLink {
    /// Get the origin as an enum
    pub fn origin_enum(&self) -> AirtableOrigin {
        AirtableOrigin::from_str(&self.origin)
    }

    /// Get the sync status as an enum
    pub fn sync_status_enum(&self) -> AirtableSyncStatus {
        AirtableSyncStatus::from_str(&self.sync_status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct CreateAirtableRecordLink {
    pub task_id: Uuid,
    pub airtable_record_id: String,
    pub airtable_base_id: String,
    pub airtable_table_id: Option<String>,
    pub origin: AirtableOrigin,
    pub airtable_record_url: Option<String>,
}

impl AirtableRecordLink {
    /// Find a task link by task ID
    pub async fn find_by_task_id(
        pool: &SqlitePool,
        task_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableRecordLink,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                airtable_record_id,
                airtable_base_id,
                airtable_table_id,
                origin,
                sync_status,
                last_sync_error,
                airtable_record_url,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_record_links
            WHERE task_id = ?"#,
            task_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find a task link by Airtable record ID
    pub async fn find_by_airtable_record_id(
        pool: &SqlitePool,
        record_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableRecordLink,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                airtable_record_id,
                airtable_base_id,
                airtable_table_id,
                origin,
                sync_status,
                last_sync_error,
                airtable_record_url,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_record_links
            WHERE airtable_record_id = ?"#,
            record_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find a task link by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableRecordLink,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                airtable_record_id,
                airtable_base_id,
                airtable_table_id,
                origin,
                sync_status,
                last_sync_error,
                airtable_record_url,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_record_links
            WHERE id = ?"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find all task links for a base
    pub async fn find_by_base_id(
        pool: &SqlitePool,
        base_id: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableRecordLink,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                airtable_record_id,
                airtable_base_id,
                airtable_table_id,
                origin,
                sync_status,
                last_sync_error,
                airtable_record_url,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_record_links
            WHERE airtable_base_id = ?
            ORDER BY created_at DESC"#,
            base_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find all task links with a specific sync status
    pub async fn find_by_sync_status(
        pool: &SqlitePool,
        status: AirtableSyncStatus,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let status_str = status.as_str();
        sqlx::query_as!(
            AirtableRecordLink,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                airtable_record_id,
                airtable_base_id,
                airtable_table_id,
                origin,
                sync_status,
                last_sync_error,
                airtable_record_url,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_record_links
            WHERE sync_status = ?
            ORDER BY created_at DESC"#,
            status_str
        )
        .fetch_all(pool)
        .await
    }

    /// Find all task links for Airtable-originated tasks with pending push status
    pub async fn find_pending_deliverable_sync(
        pool: &SqlitePool,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            AirtableRecordLink,
            r#"SELECT
                id as "id!: Uuid",
                task_id as "task_id!: Uuid",
                airtable_record_id,
                airtable_base_id,
                airtable_table_id,
                origin,
                sync_status,
                last_sync_error,
                airtable_record_url,
                last_synced_at as "last_synced_at?: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM airtable_record_links
            WHERE origin = 'airtable' AND sync_status = 'pending_push'
            ORDER BY created_at DESC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Create a new task link
    pub async fn create(
        pool: &SqlitePool,
        data: CreateAirtableRecordLink,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let origin_str = data.origin.as_str();
        let sync_status = AirtableSyncStatus::Synced.as_str();

        sqlx::query!(
            r#"INSERT INTO airtable_record_links (
                id, task_id, airtable_record_id, airtable_base_id, airtable_table_id,
                origin, sync_status, airtable_record_url, last_synced_at, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
            id,
            data.task_id,
            data.airtable_record_id,
            data.airtable_base_id,
            data.airtable_table_id,
            origin_str,
            sync_status,
            data.airtable_record_url,
            now,
            now,
            now
        )
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    /// Update sync status
    pub async fn update_sync_status(
        pool: &SqlitePool,
        id: Uuid,
        status: AirtableSyncStatus,
        error: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let status_str = status.as_str();
        let last_synced = if status == AirtableSyncStatus::Synced {
            Some(now)
        } else {
            None
        };

        sqlx::query!(
            r#"UPDATE airtable_record_links SET
                sync_status = ?,
                last_sync_error = ?,
                last_synced_at = COALESCE(?, last_synced_at),
                updated_at = ?
            WHERE id = ?"#,
            status_str,
            error,
            last_synced,
            now,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update table ID (when record moves in Airtable)
    pub async fn update_table_id(
        pool: &SqlitePool,
        id: Uuid,
        table_id: &str,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        sqlx::query!(
            "UPDATE airtable_record_links SET airtable_table_id = ?, updated_at = ? WHERE id = ?",
            table_id,
            now,
            id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark a task as needing to push deliverables
    pub async fn mark_pending_push(pool: &SqlitePool, task_id: Uuid) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let status = AirtableSyncStatus::PendingPush.as_str();
        sqlx::query!(
            "UPDATE airtable_record_links SET sync_status = ?, updated_at = ? WHERE task_id = ?",
            status,
            now,
            task_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a task link
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM airtable_record_links WHERE id = ?", id)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Delete by task ID
    pub async fn delete_by_task_id(pool: &SqlitePool, task_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM airtable_record_links WHERE task_id = ?", task_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
