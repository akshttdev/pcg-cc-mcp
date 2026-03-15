use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TaskTemplate {
    pub id: Uuid,
    pub project_id: Option<Uuid>, // None for global templates
    pub title: String,
    pub description: Option<String>,
    pub template_name: String,
    pub priority: Option<String>,
    pub completion_criteria: Option<String>,
    pub output_format: Option<String>,
    pub assigned_agent: Option<String>,
    pub tags: Option<String>,
    pub organization_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTaskTemplate {
    pub project_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub template_name: String,
    pub priority: Option<String>,
    pub completion_criteria: Option<String>,
    pub output_format: Option<String>,
    pub assigned_agent: Option<String>,
    pub tags: Option<Vec<String>>,
    pub organization_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateTaskTemplate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub template_name: Option<String>,
    pub priority: Option<String>,
    pub completion_criteria: Option<String>,
    pub output_format: Option<String>,
    pub assigned_agent: Option<String>,
    pub tags: Option<Vec<String>>,
}

const TEMPLATE_COLS: &str = "id, project_id, title, description, template_name, priority, completion_criteria, output_format, assigned_agent, tags, organization_id, created_at, updated_at";

impl TaskTemplate {
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, TaskTemplate>(
            &format!("SELECT {} FROM task_templates ORDER BY project_id IS NULL DESC, template_name ASC", TEMPLATE_COLS)
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_project_id(
        pool: &SqlitePool,
        project_id: Option<Uuid>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        if let Some(pid) = project_id {
            sqlx::query_as::<_, TaskTemplate>(
                &format!("SELECT {} FROM task_templates WHERE project_id = ?1 ORDER BY template_name ASC", TEMPLATE_COLS)
            )
            .bind(pid)
            .fetch_all(pool)
            .await
        } else {
            sqlx::query_as::<_, TaskTemplate>(
                &format!("SELECT {} FROM task_templates WHERE project_id IS NULL ORDER BY template_name ASC", TEMPLATE_COLS)
            )
            .fetch_all(pool)
            .await
        }
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, TaskTemplate>(
            &format!("SELECT {} FROM task_templates WHERE id = ?1", TEMPLATE_COLS)
        )
        .bind(id)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_organization(pool: &SqlitePool, organization_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, TaskTemplate>(
            &format!(
                "SELECT {} FROM task_templates WHERE organization_id = ?1 OR (project_id IS NULL AND organization_id IS NULL) ORDER BY template_name ASC",
                TEMPLATE_COLS
            )
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await
    }

    pub async fn create(pool: &SqlitePool, data: &CreateTaskTemplate) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let tags_json = data.tags.as_ref().map(|v| serde_json::to_string(v).unwrap_or_default());

        sqlx::query_as::<_, TaskTemplate>(
            &format!(
                "INSERT INTO task_templates (id, project_id, title, description, template_name, priority, completion_criteria, output_format, assigned_agent, tags, organization_id) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11) \
                 RETURNING {}", TEMPLATE_COLS
            )
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.template_name)
        .bind(&data.priority)
        .bind(&data.completion_criteria)
        .bind(&data.output_format)
        .bind(&data.assigned_agent)
        .bind(&tags_json)
        .bind(data.organization_id)
        .fetch_one(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateTaskTemplate,
    ) -> Result<Self, sqlx::Error> {
        let existing = Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        let title = data.title.as_ref().unwrap_or(&existing.title);
        let description = data.description.as_ref().or(existing.description.as_ref());
        let template_name = data.template_name.as_ref().unwrap_or(&existing.template_name);
        let priority = data.priority.as_ref().or(existing.priority.as_ref());
        let completion_criteria = data.completion_criteria.as_ref().or(existing.completion_criteria.as_ref());
        let output_format = data.output_format.as_ref().or(existing.output_format.as_ref());
        let assigned_agent = data.assigned_agent.as_ref().or(existing.assigned_agent.as_ref());
        let tags = data.tags.as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .or(existing.tags.clone());

        sqlx::query_as::<_, TaskTemplate>(
            &format!(
                "UPDATE task_templates SET title = ?2, description = ?3, template_name = ?4, \
                 priority = ?5, completion_criteria = ?6, output_format = ?7, assigned_agent = ?8, \
                 tags = ?9, updated_at = datetime('now', 'subsec') \
                 WHERE id = ?1 RETURNING {}", TEMPLATE_COLS
            )
        )
        .bind(id)
        .bind(title)
        .bind(description)
        .bind(template_name)
        .bind(priority)
        .bind(completion_criteria)
        .bind(output_format)
        .bind(assigned_agent)
        .bind(tags)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM task_templates WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}
