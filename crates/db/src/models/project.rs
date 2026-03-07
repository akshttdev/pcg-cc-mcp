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
    /// Folder this project is grouped under (deprecated — use parent_project_id)
    pub folder_id: Option<Uuid>,
    /// Parent project for nesting (max 3 levels deep). None = top-level.
    pub parent_project_id: Option<Uuid>,
    /// Sort order among siblings
    #[ts(type = "number")]
    pub sort_order: i32,
    /// Aptos wallet address registered for on-chain deposits
    pub aptos_address: Option<String>,
    /// Whether this project has been funded with on-chain VIBE
    pub aptos_funded: bool,

    #[ts(type = "Date")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "Date")]
    pub updated_at: DateTime<Utc>,
    /// Soft delete timestamp - if set, project is considered deleted
    #[ts(type = "Date | null")]
    pub deleted_at: Option<DateTime<Utc>>,
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
    pub parent_project_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateProject {
    #[ts(optional)]
    pub name: Option<String>,
    #[ts(optional)]
    pub git_repo_path: Option<String>,
    #[ts(optional)]
    pub setup_script: Option<String>,
    #[ts(optional)]
    pub dev_script: Option<String>,
    #[ts(optional)]
    pub cleanup_script: Option<String>,
    #[ts(optional)]
    pub copy_files: Option<String>,
    #[ts(optional)]
    pub organization_id: Option<String>,
    #[ts(optional)]
    pub client_id: Option<String>,
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
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM projects WHERE deleted_at IS NULL"
        )
        .fetch_one(pool)
        .await?;
        Ok(result.0)
    }

    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id, parent_project_id, sort_order,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at, deleted_at
               FROM projects WHERE deleted_at IS NULL ORDER BY created_at DESC"#,
        )
        .fetch_all(pool)
        .await
    }

    /// Find the most actively used projects based on recent task activity
    pub async fn find_most_active(pool: &SqlitePool, limit: i32) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT p.id, p.name, p.git_repo_path, p.setup_script, p.dev_script, p.cleanup_script, p.copy_files,
                   p.vibe_budget_limit, COALESCE(p.vibe_spent_amount, 0) as vibe_spent_amount,
                   p.organization_id, p.client_id, p.folder_id, p.parent_project_id, p.sort_order,
                   p.aptos_address, COALESCE(p.aptos_funded, 0) as aptos_funded,
                   p.created_at, p.updated_at, p.deleted_at
            FROM projects p
            WHERE p.deleted_at IS NULL AND p.id IN (
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
                      organization_id, client_id, folder_id, parent_project_id, sort_order,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at, deleted_at
               FROM projects WHERE id = ? AND deleted_at IS NULL"#,
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
                      organization_id, client_id, folder_id, parent_project_id, sort_order,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at, deleted_at
               FROM projects WHERE git_repo_path = ? AND deleted_at IS NULL"#,
        )
        .bind(git_repo_path)
        .fetch_optional(pool)
        .await
    }

    /// Check if a project exists with this git repo path, including soft-deleted projects.
    /// Used by topos sync to avoid recreating deleted projects.
    pub async fn exists_by_git_repo_path_including_deleted(
        pool: &SqlitePool,
        git_repo_path: &str,
    ) -> Result<bool, sqlx::Error> {
        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM projects WHERE git_repo_path = ? LIMIT 1"
        )
        .bind(git_repo_path)
        .fetch_optional(pool)
        .await?;
        Ok(result.is_some())
    }

    pub async fn find_by_git_repo_path_excluding_id(
        pool: &SqlitePool,
        git_repo_path: &str,
        exclude_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id, parent_project_id, sort_order,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at, deleted_at
               FROM projects WHERE git_repo_path = ? AND id != ? AND deleted_at IS NULL"#,
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
            sqlx::query_as("SELECT id FROM projects WHERE LOWER(name) = LOWER(?) AND deleted_at IS NULL LIMIT 1")
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
            r#"INSERT INTO projects (id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files, organization_id, client_id, folder_id, parent_project_id)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               RETURNING id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                         vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                         organization_id, client_id, folder_id, parent_project_id, sort_order,
                         aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                         created_at, updated_at, deleted_at"#,
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
        .bind(data.parent_project_id)
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
        organization_id: Option<Uuid>,
        client_id: Option<Uuid>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"UPDATE projects SET name = ?, git_repo_path = ?, setup_script = ?, dev_script = ?, cleanup_script = ?, copy_files = ?,
               organization_id = ?, client_id = ?
               WHERE id = ? AND deleted_at IS NULL
               RETURNING id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                         vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                         organization_id, client_id, folder_id, parent_project_id, sort_order,
                         aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                         created_at, updated_at, deleted_at"#,
        )
        .bind(&name)
        .bind(&git_repo_path)
        .bind(&setup_script)
        .bind(&dev_script)
        .bind(&cleanup_script)
        .bind(&copy_files)
        .bind(organization_id)
        .bind(client_id)
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

    /// Soft delete a project by setting deleted_at timestamp
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE projects SET deleted_at = datetime('now', 'subsec') WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn exists(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM projects WHERE id = ? AND deleted_at IS NULL"
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(result.0 > 0)
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id, parent_project_id, sort_order,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at, deleted_at
               FROM projects WHERE organization_id = ? AND deleted_at IS NULL ORDER BY name ASC"#,
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
                      organization_id, client_id, folder_id, parent_project_id, sort_order,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at, deleted_at
               FROM projects WHERE client_id = ? AND deleted_at IS NULL ORDER BY name ASC"#,
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

    /// Find direct children of a project
    pub async fn find_children(
        pool: &SqlitePool,
        parent_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Project>(
            r#"SELECT id, name, git_repo_path, setup_script, dev_script, cleanup_script, copy_files,
                      vibe_budget_limit, COALESCE(vibe_spent_amount, 0) as vibe_spent_amount,
                      organization_id, client_id, folder_id, parent_project_id, sort_order,
                      aptos_address, COALESCE(aptos_funded, 0) as aptos_funded,
                      created_at, updated_at, deleted_at
               FROM projects WHERE parent_project_id = ? AND deleted_at IS NULL ORDER BY sort_order ASC, name ASC"#,
        )
        .bind(parent_id)
        .fetch_all(pool)
        .await
    }

    /// Set or clear the parent project (reparenting). Validates max 3-level depth.
    pub async fn set_parent(
        pool: &SqlitePool,
        project_id: Uuid,
        parent_project_id: Option<Uuid>,
    ) -> Result<(), ProjectError> {
        if let Some(parent_id) = parent_project_id {
            // Prevent circular reference
            if parent_id == project_id {
                return Err(ProjectError::CreateFailed(
                    "Cannot set a project as its own parent".into(),
                ));
            }
            // Validate depth: walk up from proposed parent, ensure total depth <= 3
            if !Self::validate_depth(pool, project_id, parent_id).await? {
                return Err(ProjectError::CreateFailed(
                    "Maximum nesting depth of 3 levels exceeded".into(),
                ));
            }
        }
        sqlx::query(
            "UPDATE projects SET parent_project_id = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(parent_project_id)
        .bind(project_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update the sort order of a project among its siblings
    pub async fn reorder(
        pool: &SqlitePool,
        project_id: Uuid,
        sort_order: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE projects SET sort_order = ?, updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(sort_order)
        .bind(project_id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Validate that nesting project_id under proposed_parent_id keeps depth <= 3.
    /// Depth counts: level 1 = top-level, level 2 = child, level 3 = grandchild.
    async fn validate_depth(
        pool: &SqlitePool,
        project_id: Uuid,
        proposed_parent_id: Uuid,
    ) -> Result<bool, ProjectError> {
        // Count ancestors of proposed_parent (including itself) to get parent depth
        let mut ancestor_count = 1; // the proposed parent itself
        let mut current_id = proposed_parent_id;
        loop {
            let parent: Option<(Option<Vec<u8>>,)> = sqlx::query_as(
                "SELECT parent_project_id FROM projects WHERE id = ?",
            )
            .bind(current_id)
            .fetch_optional(pool)
            .await?;

            match parent {
                Some((Some(parent_bytes),)) => {
                    if let Ok(pid) = Uuid::from_slice(&parent_bytes) {
                        ancestor_count += 1;
                        if ancestor_count >= 3 {
                            return Ok(false); // Already at depth 3, can't nest further
                        }
                        current_id = pid;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        // Count max depth of descendants of project_id
        let max_descendant_depth = Self::max_descendant_depth(pool, project_id).await?;

        // Total depth = ancestors of parent + 1 (project itself) + descendant depth
        Ok(ancestor_count + 1 + max_descendant_depth <= 3)
    }

    /// Recursively find the maximum descendant depth of a project
    async fn max_descendant_depth(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<usize, ProjectError> {
        #[derive(sqlx::FromRow)]
        struct IdRow {
            id: Vec<u8>,
        }

        let children: Vec<IdRow> = sqlx::query_as(
            "SELECT id FROM projects WHERE parent_project_id = ?",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        if children.is_empty() {
            return Ok(0);
        }

        let mut max_depth = 0;
        for child in children {
            if let Ok(child_id) = Uuid::from_slice(&child.id) {
                let depth = Box::pin(Self::max_descendant_depth(pool, child_id)).await?;
                if depth + 1 > max_depth {
                    max_depth = depth + 1;
                }
            }
        }
        Ok(max_depth)
    }
}
