//! Sync Folders — org-scoped shared folders for local filesystem sync.
//!
//! Maps to Dropbox/OneDrive-style shared folders that sync to member devices.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncFolder {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub is_shared: bool,
    pub auto_sync: bool,
    pub max_depth: Option<i32>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSyncFolder {
    pub organization_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub path: Option<String>,
    pub description: Option<String>,
    pub is_shared: Option<bool>,
    pub auto_sync: Option<bool>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateSyncFolder {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_shared: Option<bool>,
    pub auto_sync: Option<bool>,
    pub max_depth: Option<i32>,
}

impl SyncFolder {
    pub async fn create(pool: &SqlitePool, input: CreateSyncFolder) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let id_bytes = id.as_bytes().to_vec();
        let org_bytes = input.organization_id.as_bytes().to_vec();
        let parent_bytes = input.parent_id.map(|u| u.as_bytes().to_vec());
        let user_bytes = input.created_by.map(|u| u.as_bytes().to_vec());
        let is_shared = input.is_shared.unwrap_or(true);
        let auto_sync = input.auto_sync.unwrap_or(true);

        // Build path from parent + name if not provided
        let path = if let Some(ref p) = input.path {
            p.clone()
        } else if let Some(ref parent_id) = input.parent_id {
            let parent = Self::find_by_id(pool, *parent_id).await?;
            match parent {
                Some(p) => format!("{}/{}", p.path, input.name),
                None => format!("/{}", input.name),
            }
        } else {
            format!("/{}", input.name)
        };

        sqlx::query(
            r#"INSERT INTO sync_folders
                (id, organization_id, parent_id, name, path, description,
                 is_shared, auto_sync, created_by)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id_bytes)
        .bind(&org_bytes)
        .bind(&parent_bytes)
        .bind(&input.name)
        .bind(&path)
        .bind(&input.description)
        .bind(is_shared)
        .bind(auto_sync)
        .bind(&user_bytes)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            "SELECT * FROM sync_folders WHERE id = ? AND archived_at IS NULL",
        )
        .bind(&id_bytes)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        org_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let org_bytes = org_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_folders
            WHERE organization_id = ? AND archived_at IS NULL
            ORDER BY path ASC"#,
        )
        .bind(&org_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn find_children(
        pool: &SqlitePool,
        parent_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let parent_bytes = parent_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_folders
            WHERE parent_id = ? AND archived_at IS NULL
            ORDER BY name ASC"#,
        )
        .bind(&parent_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn find_roots(
        pool: &SqlitePool,
        org_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let org_bytes = org_id.as_bytes().to_vec();
        sqlx::query_as::<_, Self>(
            r#"SELECT * FROM sync_folders
            WHERE organization_id = ? AND parent_id IS NULL AND archived_at IS NULL
            ORDER BY name ASC"#,
        )
        .bind(&org_bytes)
        .fetch_all(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateSyncFolder,
    ) -> Result<Option<Self>, sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        let mut sets = Vec::new();
        let mut binds: Vec<Option<String>> = Vec::new();

        if let Some(ref name) = input.name {
            sets.push("name = ?");
            binds.push(Some(name.clone()));
        }
        if let Some(ref desc) = input.description {
            sets.push("description = ?");
            binds.push(Some(desc.clone()));
        }
        if let Some(shared) = input.is_shared {
            sets.push("is_shared = ?");
            binds.push(Some(shared.to_string()));
        }
        if let Some(sync) = input.auto_sync {
            sets.push("auto_sync = ?");
            binds.push(Some(sync.to_string()));
        }

        if sets.is_empty() {
            return Self::find_by_id(pool, id).await;
        }

        sets.push("updated_at = datetime('now', 'subsec')");
        let query_str = format!(
            "UPDATE sync_folders SET {} WHERE id = ?",
            sets.join(", ")
        );

        let mut query = sqlx::query(&query_str);
        for val in &binds {
            query = query.bind(val);
        }
        query = query.bind(&id_bytes);
        query.execute(pool).await?;

        Self::find_by_id(pool, id).await
    }

    pub async fn archive(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        let id_bytes = id.as_bytes().to_vec();
        sqlx::query(
            "UPDATE sync_folders SET archived_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(&id_bytes)
        .execute(pool)
        .await?;
        Ok(())
    }
}
