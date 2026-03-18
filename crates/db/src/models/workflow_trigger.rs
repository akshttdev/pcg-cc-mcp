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
    // Webhook-specific
    pub webhook_secret: Option<String>,
    pub webhook_url: Option<String>,
    // Rate limiting
    pub cooldown_seconds: i64,
    // Retry config
    pub max_retries: i64,
    // Error tracking
    pub last_error: Option<String>,
    pub retry_count: i64,
    pub next_retry_at: Option<String>,
    // Metadata
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
    pub cooldown_seconds: Option<i64>,
    pub max_retries: Option<i64>,
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
    pub cooldown_seconds: Option<i64>,
    pub max_retries: Option<i64>,
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
        let cooldown_seconds = input.cooldown_seconds.unwrap_or(0);
        let max_retries = input.max_retries.unwrap_or(0);

        // Generate webhook secret and URL for webhook triggers
        let (webhook_secret, webhook_url) = if trigger_type == "webhook" {
            let secret = Uuid::new_v4().to_string().replace('-', "");
            let url = format!("/api/webhooks/triggers/{id}");
            (Some(secret), Some(url))
        } else {
            (None, None)
        };

        sqlx::query_as::<_, Self>(
            r#"INSERT INTO workflow_triggers
               (id, workflow_id, name, enabled, trigger_type,
                filter_data_source_types, filter_organization_id, filter_project_id,
                filter_tags, model_override, auto_approve,
                webhook_secret, webhook_url, cooldown_seconds, max_retries)
               VALUES (?1, ?2, ?3, 0, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
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
        .bind(&webhook_secret)
        .bind(&webhook_url)
        .bind(cooldown_seconds)
        .bind(max_retries)
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
        if let Some(cooldown) = input.cooldown_seconds {
            sets.push("cooldown_seconds = ?");
            binds.push(Some(cooldown.to_string()));
        }
        if let Some(retries) = input.max_retries {
            sets.push("max_retries = ?");
            binds.push(Some(retries.to_string()));
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

    /// Check if a trigger is within its cooldown period.
    /// Returns true if the trigger can fire (not in cooldown).
    pub fn is_past_cooldown(&self) -> bool {
        if self.cooldown_seconds <= 0 {
            return true;
        }
        match &self.last_triggered_at {
            Some(last) => {
                let now = chrono::Utc::now();
                if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(last, "%Y-%m-%d %H:%M:%S")
                    .or_else(|_| chrono::NaiveDateTime::parse_from_str(last, "%Y-%m-%dT%H:%M:%S%.f"))
                {
                    let last_utc = last_time.and_utc();
                    let elapsed = now.signed_duration_since(last_utc).num_seconds();
                    elapsed >= self.cooldown_seconds
                } else {
                    true
                }
            }
            None => true,
        }
    }

    /// Record an error on this trigger and optionally schedule a retry.
    pub async fn record_error(pool: &SqlitePool, id: &str, error: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE workflow_triggers
               SET last_error = ?2,
                   retry_count = retry_count + 1,
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?1"#,
        )
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Clear error state after a successful execution.
    pub async fn clear_error(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"UPDATE workflow_triggers
               SET last_error = NULL, retry_count = 0, next_retry_at = NULL,
                   updated_at = datetime('now', 'subsec')
               WHERE id = ?1"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Find enabled schedule triggers that are due to run.
    /// A schedule trigger is due if:
    /// - trigger_type = 'schedule'
    /// - enabled = true
    /// - last_triggered_at is null OR older than the schedule interval
    pub async fn find_due_schedules(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        // We support cron-like intervals stored in filter_tags as JSON: {"interval": "hourly|daily|weekly"}
        // For simplicity, check if last_triggered_at is older than the interval threshold
        let enabled = Self::find_enabled(pool).await?;
        let now = chrono::Utc::now();

        let due: Vec<Self> = enabled.into_iter().filter(|trigger| {
            if trigger.trigger_type != "schedule" {
                return false;
            }

            // Parse interval from filter_tags (reused field for schedule config)
            let interval_minutes = trigger.filter_tags.as_deref()
                .and_then(|tags| serde_json::from_str::<serde_json::Value>(tags).ok())
                .and_then(|v| v.get("interval").and_then(|i| i.as_str()).map(|s| s.to_string()))
                .map(|interval| match interval.as_str() {
                    "hourly" => 60,
                    "daily" => 1440,
                    "weekly" => 10080,
                    "every_5m" => 5,
                    "every_15m" => 15,
                    "every_30m" => 30,
                    _ => interval.parse::<i64>().unwrap_or(60), // default hourly
                })
                .unwrap_or(60);

            // Check if enough time has passed since last trigger
            match &trigger.last_triggered_at {
                Some(last) => {
                    if let Ok(last_time) = chrono::NaiveDateTime::parse_from_str(last, "%Y-%m-%d %H:%M:%S")
                        .or_else(|_| chrono::NaiveDateTime::parse_from_str(last, "%Y-%m-%dT%H:%M:%S%.f"))
                    {
                        let last_utc = last_time.and_utc();
                        let elapsed = now.signed_duration_since(last_utc).num_minutes();
                        elapsed >= interval_minutes
                    } else {
                        true // Can't parse, consider it due
                    }
                }
                None => true, // Never triggered, is due
            }
        }).collect();

        Ok(due)
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
            if let Some(ref types_json) = trigger.filter_data_source_types
                && let Ok(types) = serde_json::from_str::<Vec<String>>(types_json)
                && !types.is_empty() && !types.contains(&data_source_type.to_string())
            {
                return false;
            }

            // Check organization filter
            if let Some(ref filter_org) = trigger.filter_organization_id
                && !filter_org.is_empty()
            {
                match organization_id {
                    Some(org) if org == filter_org => {}
                    _ => return false,
                }
            }

            // Check project filter
            if let Some(ref filter_proj) = trigger.filter_project_id
                && !filter_proj.is_empty()
            {
                match project_id {
                    Some(proj) if proj == filter_proj => {}
                    _ => return false,
                }
            }

            true
        }).collect();

        Ok(matching)
    }
}
