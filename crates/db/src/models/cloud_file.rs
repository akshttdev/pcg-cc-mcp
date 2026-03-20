use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;

// ── CloudFile ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CloudFile {
    pub id: String,
    pub organization_id: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub file_name: String,
    pub file_path: String,
    pub storage_volume: String,
    pub content_hash: Option<String>,
    pub file_size_bytes: i64,
    pub mime_type: Option<String>,
    pub source_type: String,
    pub source_id: Option<String>,
    pub source_table: Option<String>,
    pub visibility: String,
    pub contributed_by: Option<String>,
    pub contributor_wallet: Option<String>,
    pub contributor_device: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCloudFile {
    pub organization_id: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub file_name: String,
    pub file_path: String,
    pub storage_volume: String,
    pub content_hash: Option<String>,
    pub file_size_bytes: i64,
    pub mime_type: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub source_table: Option<String>,
    pub visibility: Option<String>,
    pub contributed_by: Option<String>,
    pub contributor_wallet: Option<String>,
    pub contributor_device: Option<String>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCloudFile {
    pub file_name: Option<String>,
    pub visibility: Option<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
}

// ── CloudContribution ──────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CloudContribution {
    pub id: String,
    pub organization_id: String,
    pub cloud_file_id: String,
    pub user_id: Option<String>,
    pub wallet_address: Option<String>,
    pub device_id: Option<String>,
    pub contribution_type: String,
    pub channel: String,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub description: Option<String>,
    pub metadata: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCloudContribution {
    pub organization_id: String,
    pub cloud_file_id: String,
    pub user_id: Option<String>,
    pub wallet_address: Option<String>,
    pub device_id: Option<String>,
    pub contribution_type: Option<String>,
    pub channel: Option<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<String>,
}

// ── OrgCloudSettings ───────────────────────────────────────────────────────

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OrgCloudSettings {
    pub organization_id: String,
    pub storage_quota_bytes: i64,
    pub viewer_can_download: bool,
    pub member_can_upload: bool,
    pub auto_index_data_sources: bool,
    pub auto_index_artifacts: bool,
    pub auto_index_media: bool,
    pub nats_contribution_enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateOrgCloudSettings {
    pub storage_quota_bytes: Option<i64>,
    pub viewer_can_download: Option<bool>,
    pub member_can_upload: Option<bool>,
    pub auto_index_data_sources: Option<bool>,
    pub auto_index_artifacts: Option<bool>,
    pub auto_index_media: Option<bool>,
    pub nats_contribution_enabled: Option<bool>,
}

// ── Query params ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CloudBrowseParams {
    pub volume: Option<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub mime_type: Option<String>,
    pub search: Option<String>,
    pub visibility: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

// ── CloudFile CRUD ─────────────────────────────────────────────────────────

impl CloudFile {
    pub async fn create(pool: &SqlitePool, input: &CreateCloudFile) -> Result<Self, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let source_type = input.source_type.as_deref().unwrap_or("upload");
        let visibility = input.visibility.as_deref().unwrap_or("org");

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO cloud_files
                (id, organization_id, project_id, task_id, file_name, file_path,
                 storage_volume, content_hash, file_size_bytes, mime_type,
                 source_type, source_id, source_table, visibility,
                 contributed_by, contributor_wallet, contributor_device)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *"#,
        )
        .bind(&id)
        .bind(&input.organization_id)
        .bind(&input.project_id)
        .bind(&input.task_id)
        .bind(&input.file_name)
        .bind(&input.file_path)
        .bind(&input.storage_volume)
        .bind(&input.content_hash)
        .bind(input.file_size_bytes)
        .bind(&input.mime_type)
        .bind(source_type)
        .bind(&input.source_id)
        .bind(&input.source_table)
        .bind(visibility)
        .bind(&input.contributed_by)
        .bind(&input.contributor_wallet)
        .bind(&input.contributor_device)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM cloud_files WHERE id = ? AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_content_hash(
        pool: &SqlitePool,
        org_id: &str,
        hash: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM cloud_files WHERE organization_id = ? AND content_hash = ? AND deleted_at IS NULL",
        )
        .bind(org_id)
        .bind(hash)
        .fetch_optional(pool)
        .await
    }

    pub async fn browse(
        pool: &SqlitePool,
        org_id: &str,
        params: &CloudBrowseParams,
        visible_project_ids: Option<&[String]>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let per_page = params.per_page.unwrap_or(20).min(100);
        let offset = (params.page.unwrap_or(1) - 1).max(0) * per_page;

        let mut sql = String::from(
            "SELECT * FROM cloud_files WHERE organization_id = ? AND deleted_at IS NULL",
        );
        let mut binds: Vec<String> = vec![org_id.to_string()];

        if let Some(ref vol) = params.volume {
            sql.push_str(" AND storage_volume = ?");
            binds.push(vol.clone());
        }
        if let Some(ref pid) = params.project_id {
            sql.push_str(" AND project_id = ?");
            binds.push(pid.clone());
        }
        if let Some(ref tid) = params.task_id {
            sql.push_str(" AND task_id = ?");
            binds.push(tid.clone());
        }
        if let Some(ref vis) = params.visibility {
            sql.push_str(" AND visibility = ?");
            binds.push(vis.clone());
        }
        if let Some(ref search) = params.search {
            sql.push_str(" AND file_name LIKE ? ESCAPE '\\'");
            let escaped = search
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            binds.push(format!("%{escaped}%"));
        }
        if let Some(ref mime) = params.mime_type {
            sql.push_str(" AND mime_type LIKE ? ESCAPE '\\'");
            let escaped = mime
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            binds.push(format!("{escaped}%"));
        }

        // Scope to visible projects if provided (for non-admin members)
        if let Some(project_ids) = visible_project_ids {
            if !project_ids.is_empty() {
                let placeholders: Vec<&str> = project_ids.iter().map(|_| "?").collect();
                sql.push_str(&format!(
                    " AND (visibility = 'org' OR project_id IN ({}))",
                    placeholders.join(",")
                ));
                for pid in project_ids {
                    binds.push(pid.clone());
                }
            } else {
                sql.push_str(" AND visibility = 'org'");
            }
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, Self>(&sql);
        for b in &binds {
            query = query.bind(b);
        }
        query = query.bind(per_page).bind(offset);
        query.fetch_all(pool).await
    }

    pub async fn count(pool: &SqlitePool, org_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cloud_files WHERE organization_id = ? AND deleted_at IS NULL",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
    }

    pub async fn total_size(pool: &SqlitePool, org_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(file_size_bytes), 0) FROM cloud_files WHERE organization_id = ? AND deleted_at IS NULL",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
    }

    pub async fn soft_delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE cloud_files SET deleted_at = datetime('now'), updated_at = datetime('now') WHERE id = ?",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &str,
        input: &UpdateCloudFile,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut sets = vec!["updated_at = datetime('now')".to_string()];
        let mut binds: Vec<String> = vec![];

        if let Some(ref name) = input.file_name {
            sets.push("file_name = ?".to_string());
            binds.push(name.clone());
        }
        if let Some(ref vis) = input.visibility {
            sets.push("visibility = ?".to_string());
            binds.push(vis.clone());
        }
        if let Some(ref pid) = input.project_id {
            sets.push("project_id = ?".to_string());
            binds.push(pid.clone());
        }
        if let Some(ref tid) = input.task_id {
            sets.push("task_id = ?".to_string());
            binds.push(tid.clone());
        }

        let sql = format!(
            "UPDATE cloud_files SET {} WHERE id = ? AND deleted_at IS NULL RETURNING *",
            sets.join(", ")
        );
        binds.push(id.to_string());

        let mut query = sqlx::query_as::<_, Self>(&sql);
        for b in &binds {
            query = query.bind(b);
        }
        query.fetch_optional(pool).await
    }
}

// ── CloudContribution CRUD ─────────────────────────────────────────────────

impl CloudContribution {
    pub async fn create(
        pool: &SqlitePool,
        input: &CreateCloudContribution,
    ) -> Result<Self, sqlx::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let contribution_type = input.contribution_type.as_deref().unwrap_or("upload");
        let channel = input.channel.as_deref().unwrap_or("http");
        let metadata = input.metadata.as_deref().unwrap_or("{}");

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO cloud_contributions
                (id, organization_id, cloud_file_id, user_id, wallet_address,
                 device_id, contribution_type, channel, project_id, task_id,
                 description, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING *"#,
        )
        .bind(&id)
        .bind(&input.organization_id)
        .bind(&input.cloud_file_id)
        .bind(&input.user_id)
        .bind(&input.wallet_address)
        .bind(&input.device_id)
        .bind(contribution_type)
        .bind(channel)
        .bind(&input.project_id)
        .bind(&input.task_id)
        .bind(&input.description)
        .bind(metadata)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_org(
        pool: &SqlitePool,
        org_id: &str,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM cloud_contributions WHERE organization_id = ? ORDER BY created_at DESC LIMIT ?",
        )
        .bind(org_id)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn count_by_org(pool: &SqlitePool, org_id: &str) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM cloud_contributions WHERE organization_id = ?",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
    }
}

// ── OrgCloudSettings CRUD ──────────────────────────────────────────────────

impl OrgCloudSettings {
    pub async fn get_or_create(pool: &SqlitePool, org_id: &str) -> Result<Self, sqlx::Error> {
        // Try fetch first
        let existing =
            sqlx::query_as::<_, Self>("SELECT * FROM org_cloud_settings WHERE organization_id = ?")
                .bind(org_id)
                .fetch_optional(pool)
                .await?;

        if let Some(settings) = existing {
            return Ok(settings);
        }

        // Insert defaults
        sqlx::query_as::<_, Self>(
            "INSERT INTO org_cloud_settings (organization_id) VALUES (?) RETURNING *",
        )
        .bind(org_id)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        org_id: &str,
        input: &UpdateOrgCloudSettings,
    ) -> Result<Self, sqlx::Error> {
        // Ensure row exists
        let _ = Self::get_or_create(pool, org_id).await?;

        let mut sets = vec!["updated_at = datetime('now')".to_string()];

        // Build dynamic SET clause
        if let Some(quota) = input.storage_quota_bytes {
            sets.push(format!("storage_quota_bytes = {quota}"));
        }
        if let Some(v) = input.viewer_can_download {
            sets.push(format!("viewer_can_download = {}", if v { 1 } else { 0 }));
        }
        if let Some(v) = input.member_can_upload {
            sets.push(format!("member_can_upload = {}", if v { 1 } else { 0 }));
        }
        if let Some(v) = input.auto_index_data_sources {
            sets.push(format!(
                "auto_index_data_sources = {}",
                if v { 1 } else { 0 }
            ));
        }
        if let Some(v) = input.auto_index_artifacts {
            sets.push(format!("auto_index_artifacts = {}", if v { 1 } else { 0 }));
        }
        if let Some(v) = input.auto_index_media {
            sets.push(format!("auto_index_media = {}", if v { 1 } else { 0 }));
        }
        if let Some(v) = input.nats_contribution_enabled {
            sets.push(format!(
                "nats_contribution_enabled = {}",
                if v { 1 } else { 0 }
            ));
        }

        let sql = format!(
            "UPDATE org_cloud_settings SET {} WHERE organization_id = ? RETURNING *",
            sets.join(", ")
        );

        sqlx::query_as::<_, Self>(&sql)
            .bind(org_id)
            .fetch_one(pool)
            .await
    }
}
