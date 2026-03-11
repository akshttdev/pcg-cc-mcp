use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowTrigger {
    pub id: String,
    pub workflow_id: String,
    pub name: String,
    pub enabled: bool,
    pub trigger_type: String,
    pub filter_data_source_types: Option<String>,
    pub filter_organization_id: Option<String>,
    pub filter_project_id: Option<String>,
    pub filter_tags: Option<String>,
    pub model_override: Option<String>,
    pub auto_approve: bool,
    pub last_triggered_at: Option<String>,
    pub trigger_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateWorkflowTrigger {
    pub workflow_id: String,
    pub name: String,
    pub trigger_type: Option<String>,
    pub filter_data_source_types: Option<Vec<String>>,
    pub filter_organization_id: Option<String>,
    pub filter_project_id: Option<String>,
    pub filter_tags: Option<Vec<String>>,
    pub model_override: Option<String>,
    pub auto_approve: Option<bool>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateWorkflowTrigger {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub trigger_type: Option<String>,
    pub filter_data_source_types: Option<Vec<String>>,
    pub filter_organization_id: Option<String>,
    pub filter_project_id: Option<String>,
    pub filter_tags: Option<Vec<String>>,
    pub model_override: Option<String>,
    pub auto_approve: Option<bool>,
}

impl WorkflowTrigger {
    pub async fn create(pool: &SqlitePool, input: CreateWorkflowTrigger) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4().to_string();
        let trigger_type = input.trigger_type.unwrap_or_else(|| "data_source_created".to_string());
        let filter_ds_types = input.filter_data_source_types
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()));
        let filter_tags = input.filter_tags
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()));
        let auto_approve = input.auto_approve.unwrap_or(false);

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workflow_triggers
               (id, workflow_id, name, enabled, trigger_type,
                filter_data_source_types, filter_organization_id, filter_project_id,
                filter_tags, model_override, auto_approve)
               VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
               RETURNING *"#,
        )
        .bind(&id)
        .bind(&input.workflow_id)
        .bind(&input.name)
        .bind(&trigger_type)
        .bind(&filter_ds_types)
        .bind(&input.filter_organization_id)
        .bind(&input.filter_project_id)
        .bind(&filter_tags)
        .bind(&input.model_override)
        .bind(auto_approve)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM workflow_triggers WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_workflow(pool: &SqlitePool, workflow_id: &str) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_triggers WHERE workflow_id = ?1 ORDER BY created_at DESC",
        )
        .bind(workflow_id)
        .fetch_all(pool)
        .await
    }

    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_triggers ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_enabled(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM workflow_triggers WHERE enabled = 1 ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn update(pool: &SqlitePool, id: &str, input: UpdateWorkflowTrigger) -> Result<Option<Self>, sqlx::Error> {
        let mut sets = Vec::new();
        let mut binds: Vec<Option<String>> = Vec::new();

        if let Some(ref name) = input.name {
            sets.push("name = ?");
            binds.push(Some(name.clone()));
        }
        if let Some(enabled) = input.enabled {
            sets.push("enabled = ?");
            binds.push(Some(if enabled { "1".to_string() } else { "0".to_string() }));
        }
        if let Some(ref tt) = input.trigger_type {
            sets.push("trigger_type = ?");
            binds.push(Some(tt.clone()));
        }
        if let Some(ref types) = input.filter_data_source_types {
            sets.push("filter_data_source_types = ?");
            binds.push(Some(serde_json::to_string(types).unwrap_or_else(|_| "[]".to_string())));
        }
        if let Some(ref org) = input.filter_organization_id {
            sets.push("filter_organization_id = ?");
            binds.push(Some(org.clone()));
        }
        if let Some(ref proj) = input.filter_project_id {
            sets.push("filter_project_id = ?");
            binds.push(Some(proj.clone()));
        }
        if let Some(ref tags) = input.filter_tags {
            sets.push("filter_tags = ?");
            binds.push(Some(serde_json::to_string(tags).unwrap_or_else(|_| "[]".to_string())));
        }
        if let Some(ref model) = input.model_override {
            sets.push("model_override = ?");
            binds.push(Some(model.clone()));
        }
        if let Some(auto) = input.auto_approve {
            sets.push("auto_approve = ?");
            binds.push(Some(if auto { "1".to_string() } else { "0".to_string() }));
        }

        if sets.is_empty() {
            return Self::find_by_id(pool, id).await;
        }

        sets.push("updated_at = datetime('now', 'subsec')");
        let query_str = format!(
            "UPDATE workflow_triggers SET {} WHERE id = ?",
            sets.join(", ")
        );

        let mut query = sqlx::query(&query_str);
        for val in &binds {
            query = query.bind(val);
        }
        query = query.bind(id);
        query.execute(pool).await?;

        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM workflow_triggers WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn toggle(pool: &SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query("UPDATE workflow_triggers SET enabled = NOT enabled, updated_at = datetime('now', 'subsec') WHERE id = ?1")
            .bind(id)
            .execute(pool)
            .await?;
        Self::find_by_id(pool, id).await
    }

    pub async fn increment_trigger_count(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE workflow_triggers SET trigger_count = trigger_count + 1, last_triggered_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec') WHERE id = ?1",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find enabled triggers whose filters match the given data source attributes.
    pub async fn find_matching_triggers(
        pool: &SqlitePool,
        data_source_type: &str,
        organization_id: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let enabled = Self::find_enabled(pool).await?;
        let matching = enabled.into_iter().filter(|trigger| {
            // Check trigger_type — only data_source_created triggers fire on creation
            if trigger.trigger_type != "data_source_created" {
                return false;
            }

            // Check data_source_types filter
            if let Some(ref types_json) = trigger.filter_data_source_types {
                if let Ok(types) = serde_json::from_str::<Vec<String>>(types_json) {
                    if !types.is_empty() && !types.contains(&data_source_type.to_string()) {
                        return false;
                    }
                }
            }

            // Check organization filter
            if let Some(ref filter_org) = trigger.filter_organization_id {
                if !filter_org.is_empty() {
                    match organization_id {
                        Some(org) if org == filter_org => {}
                        _ => return false,
                    }
                }
            }

            // Check project filter
            if let Some(ref filter_proj) = trigger.filter_project_id {
                if !filter_proj.is_empty() {
                    match project_id {
                        Some(proj) if proj == filter_proj => {}
                        _ => return false,
                    }
                }
            }

            true
        }).collect();

        Ok(matching)
    }
}
