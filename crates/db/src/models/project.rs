use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Project not found")]
    ProjectNotFound,
    #[error("Project with git repository path already exists")]
    GitRepoPathExists,
    #[error("Failed to check existing git repository path: {0}")]
    GitRepoCheckFailed(String),
    #[error("Failed to create project: {0}")]
    CreateFailed(String),
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[sqlx(try_from = "String")]
    pub git_repo_path: PathBuf,
    pub setup_script: Option<String>,
    pub dev_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub copy_files: Option<String>,
    /// VIBE budget limit (1 VIBE = $0.01 USD), None means unlimited
    #[ts(type = "number | null")]
    pub vibe_budget_limit: Option<i64>,
    /// VIBE spent amount
    #[ts(type = "number")]
    pub vibe_spent_amount: i64,
    /// Organization this project belongs to
    pub organization_id: Option<Uuid>,
    /// Client this project is for (within the organization)
    pub client_id: Option<Uuid>,
    /// Folder this project is grouped under
    pub folder_id: Option<Uuid>,
    /// Aptos wallet address registered for on-chain deposits
    pub aptos_address: Option<String>,
    /// Whether this project has been funded with on-chain VIBE
    pub aptos_funded: bool,

    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateProject {
    pub name: String,
    pub git_repo_path: String,
    pub use_existing_repo: bool,
    pub setup_script: Option<String>,
    pub dev_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub copy_files: Option<String>,
    pub organization_id: Option<Uuid>,
    pub client_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateProject {
    pub name: Option<String>,
    pub git_repo_path: Option<String>,
    pub setup_script: Option<String>,
    pub dev_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub copy_files: Option<String>,
}

#[derive(Debug, Serialize, TS)]
pub struct SearchResult {
    pub path: String,
    pub is_file: bool,
    pub match_type: SearchMatchType,
}

#[derive(Debug, Clone, Serialize, TS)]
pub enum SearchMatchType {
    FileName,
    DirectoryName,
    FullPath,
}

impl Project {
    pub async fn count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!: i64" FROM projects"#)
            .fetch_one(pool)
            .await
    }

    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at
               FROM projects ORDER BY created_at DESC"#,
        )
        .fetch_all(pool)
        .await
    }

    /// Find the most actively used projects based on recent task activity
    pub async fn find_most_active(pool: &SqlitePool, limit: i32) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT p.id, p.name, p.git_repo_path, p.setup_script, p.dev_script, p.cleanup_script, p.copy_files,
                   p.vibe_budget_limit, COALESCE(p.vibe_spent_amount, 0) as vibe_spent_amount,
                   p.organization_id, p.client_id, p.folder_id,
                   p.aptos_address, COALESCE(p.aptos_funded, 0) as aptos_funded,
                   p.created_at, p.updated_at
            FROM projects p
            WHERE p.id IN (
                SELECT DISTINCT t.project_id
                FROM tasks t
                INNER JOIN task_attempts ta ON ta.task_id = t.id
                ORDER BY ta.updated_at DESC
            )
            LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at
               FROM projects WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_git_repo_path(
        pool: &SqlitePool,
        git_repo_path: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at
               FROM projects WHERE git_repo_path = ?"#,
        )
        .bind(git_repo_path)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_git_repo_path_excluding_id(
        pool: &SqlitePool,
        git_repo_path: &str,
        exclude_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at
               FROM projects WHERE git_repo_path = ? AND id != ?"#,
        )
        .bind(git_repo_path)
        .bind(exclude_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_name_case_insensitive(
        pool: &SqlitePool,
        name: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let row: Option<(Vec<u8>,)> =
            sqlx::query_as("SELECT id FROM projects WHERE LOWER(name) = LOWER(?) LIMIT 1")
                .bind(name)
                .fetch_optional(pool)
                .await?;

        if let Some((bytes,)) = row {
            if let Ok(uuid) = Uuid::from_slice(&bytes) {
                Project::find_by_id(pool, uuid).await
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    pub async fn create(
        pool: &SqlitePool,
        data: &CreateProject,
        project_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"INSERT INTO projects (id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files, organization_id, client_id, folder_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                         vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                         organization_id, client_id, folder_id,
                         aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                         created_at, updated_at"#,
        )
        .bind(project_id)
        .bind(&data.name)
        .bind(&data.git_repo_path)
        .bind(&data.setup_script)
        .bind(&data.dev_script)
        .bind(&data.cleanup_script)
        .bind(&data.copy_files)
        .bind(data.organization_id)
        .bind(data.client_id)
        .bind(data.folder_id)
        .fetch_one(pool)
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        name: String,
        git_repo_path: String,
        setup_script: Option<String>,
        dev_script: Option<String>,
        cleanup_script: Option<String>,
        copy_files: Option<String>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"UPDATE projects SET name = ?, git_repo_path = ?, setup_script = ?, dev_script = ?, cleanup_script = ?, copy_files = ?
               WHERE id = ?
               RETURNING id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                         vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                         organization_id, client_id, folder_id,
                         aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                         created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&git_repo_path)
        .bind(&setup_script)
        .bind(&dev_script)
        .bind(&cleanup_script)
        .bind(&copy_files)
        .bind(id)
        .fetch_one(pool)
        .await
    }

    /// Set VIBE budget limit for a project
    pub async fn set_vibe_budget(
        pool: &SqlitePool,
        project_id: Uuid,
        vibe_budget_limit: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE projects SET vibe_budget_limit = ?, updated_at = datetime('now', 'subsec') WHERE id = ?"#,
            vibe_budget_limit,
            project_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Adjust VIBE spent amount for a project
    pub async fn adjust_vibe_spent(
        pool: &SqlitePool,
        project_id: Uuid,
        delta: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE projects SET vibe_spent_amount = COALESCE(vibe_spent_amount, 0) + ?, updated_at = datetime('now', 'subsec') WHERE id = ?"#,
            delta,
            project_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Check if project has sufficient VIBE budget
    pub fn has_vibe_budget(&self, required_vibe: i64) -> bool {
        match self.vibe_budget_limit {
            Some(limit) => (limit - self.vibe_spent_amount) >= required_vibe,
            None => true, // No limit means unlimited
        }
    }

    /// Get remaining VIBE budget
    pub fn remaining_vibe(&self) -> Option<i64> {
        self.vibe_budget_limit.map(|limit| limit - self.vibe_spent_amount)
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM projects WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn exists(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query!(
            r#"
                SELECT COUNT(*) as "count!: i64"
                FROM projects
                WHERE id = $1
            "#,
            id
        )
        .fetch_one(pool)
        .await?;

        Ok(result.count > 0)
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at
               FROM projects WHERE organization_id = ? ORDER BY name ASC"#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_client(
        pool: &SqlitePool,
        client_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at
               FROM projects WHERE client_id = ? ORDER BY name ASC"#,
        )
        .bind(client_id)
        .fetch_all(pool)
        .await
    }

    /// Register an Aptos wallet address for on-chain deposits
    pub async fn set_aptos_wallet(
        pool: &SqlitePool,
        project_id: Uuid,
        aptos_address: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE projects SET aptos_address = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(aptos_address)
        .bind(project_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Set or clear the folder assignment for a project
    pub async fn set_folder(
        pool: &SqlitePool,
        project_id: Uuid,
        folder_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE projects SET folder_id = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(folder_id)
        .bind(project_id)
        .execute(pool)
        .await?;
        Ok(())
    }
}
