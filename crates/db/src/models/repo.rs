use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct Repo {
    pub id: Uuid,
    pub path: String,
    pub name: String,
    pub display_name: String,
    pub setup_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub archive_script: Option<String>,
    pub copy_files: Option<String>,
    pub parallel_setup_script: bool,
    pub dev_server_script: Option<String>,
    pub default_target_branch: Option<String>,
    pub default_working_dir: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateRepo {
    pub path: String,
    pub display_name: Option<String>,
    pub setup_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub archive_script: Option<String>,
    pub copy_files: Option<String>,
    pub dev_server_script: Option<String>,
    pub default_target_branch: Option<String>,
    pub default_working_dir: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
pub struct UpdateRepo {
    pub display_name: Option<String>,
    pub setup_script: Option<Option<String>>,
    pub cleanup_script: Option<Option<String>>,
    pub archive_script: Option<Option<String>>,
    pub copy_files: Option<Option<String>>,
    pub dev_server_script: Option<Option<String>>,
    pub default_target_branch: Option<Option<String>>,
    pub default_working_dir: Option<Option<String>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ProjectRepo {
    pub id: Uuid,
    pub project_id: Uuid,
    pub repo_id: Uuid,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct AttemptRepo {
    pub id: Uuid,
    pub task_attempt_id: Uuid,
    pub repo_id: Uuid,
    pub target_branch: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct RepoWithTargetBranch {
    #[serde(flatten)]
    pub repo: Repo,
    pub target_branch: String,
}

impl Repo {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Repo,
            r#"SELECT
                id as "id!: Uuid",
                path,
                name,
                display_name,
                setup_script,
                cleanup_script,
                archive_script,
                copy_files,
                parallel_setup_script as "parallel_setup_script!: bool",
                dev_server_script,
                default_target_branch,
                default_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM repos
            ORDER BY display_name ASC"#
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Repo,
            r#"SELECT
                id as "id!: Uuid",
                path,
                name,
                display_name,
                setup_script,
                cleanup_script,
                archive_script,
                copy_files,
                parallel_setup_script as "parallel_setup_script!: bool",
                dev_server_script,
                default_target_branch,
                default_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM repos
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_path(pool: &SqlitePool, path: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            Repo,
            r#"SELECT
                id as "id!: Uuid",
                path,
                name,
                display_name,
                setup_script,
                cleanup_script,
                archive_script,
                copy_files,
                parallel_setup_script as "parallel_setup_script!: bool",
                dev_server_script,
                default_target_branch,
                default_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM repos
            WHERE path = $1"#,
            path
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateRepo) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let name = data
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&data.path)
            .to_string();
        let display_name = data.display_name.as_deref().unwrap_or(&name);

        sqlx::query_as!(
            Repo,
            r#"INSERT INTO repos (id, path, name, display_name, setup_script, cleanup_script, archive_script, copy_files, dev_server_script, default_target_branch, default_working_dir)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING
                id as "id!: Uuid",
                path,
                name,
                display_name,
                setup_script,
                cleanup_script,
                archive_script,
                copy_files,
                parallel_setup_script as "parallel_setup_script!: bool",
                dev_server_script,
                default_target_branch,
                default_working_dir,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.path,
            name,
            display_name,
            data.setup_script,
            data.cleanup_script,
            data.archive_script,
            data.copy_files,
            data.dev_server_script,
            data.default_target_branch,
            data.default_working_dir
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM repos WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            Repo,
            r#"SELECT
                r.id as "id!: Uuid",
                r.path,
                r.name,
                r.display_name,
                r.setup_script,
                r.cleanup_script,
                r.archive_script,
                r.copy_files,
                r.parallel_setup_script as "parallel_setup_script!: bool",
                r.dev_server_script,
                r.default_target_branch,
                r.default_working_dir,
                r.created_at as "created_at!: DateTime<Utc>",
                r.updated_at as "updated_at!: DateTime<Utc>"
            FROM repos r
            JOIN project_repos pr ON pr.repo_id = r.id
            WHERE pr.project_id = $1
            ORDER BY pr.is_default DESC, r.display_name ASC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_task_attempt_id(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<Vec<RepoWithTargetBranch>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT
                r.id as "id!: Uuid",
                r.path,
                r.name,
                r.display_name,
                r.setup_script,
                r.cleanup_script,
                r.archive_script,
                r.copy_files,
                r.parallel_setup_script as "parallel_setup_script!: bool",
                r.dev_server_script,
                r.default_target_branch,
                r.default_working_dir,
                r.created_at as "created_at!: DateTime<Utc>",
                r.updated_at as "updated_at!: DateTime<Utc>",
                ar.target_branch
            FROM repos r
            JOIN attempt_repos ar ON ar.repo_id = r.id
            WHERE ar.task_attempt_id = $1"#,
            task_attempt_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| RepoWithTargetBranch {
                repo: Repo {
                    id: r.id,
                    path: r.path,
                    name: r.name,
                    display_name: r.display_name,
                    setup_script: r.setup_script,
                    cleanup_script: r.cleanup_script,
                    archive_script: r.archive_script,
                    copy_files: r.copy_files,
                    parallel_setup_script: r.parallel_setup_script,
                    dev_server_script: r.dev_server_script,
                    default_target_branch: r.default_target_branch,
                    default_working_dir: r.default_working_dir,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                },
                target_branch: r.target_branch,
            })
            .collect())
    }

    pub async fn link_to_project(
        pool: &SqlitePool,
        project_id: Uuid,
        repo_id: Uuid,
        is_default: bool,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"INSERT OR IGNORE INTO project_repos (id, project_id, repo_id, is_default)
            VALUES ($1, $2, $3, $4)"#,
            id,
            project_id,
            repo_id,
            is_default
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn link_to_attempt(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
        repo_id: Uuid,
        target_branch: &str,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query!(
            r#"INSERT OR IGNORE INTO attempt_repos (id, task_attempt_id, repo_id, target_branch)
            VALUES ($1, $2, $3, $4)"#,
            id,
            task_attempt_id,
            repo_id,
            target_branch
        )
        .execute(pool)
        .await?;
        Ok(())
    }
}
