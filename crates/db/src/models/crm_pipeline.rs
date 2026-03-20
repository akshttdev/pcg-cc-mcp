use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;

use crate::db_uuid::DbUuid;

#[derive(Debug, Error)]
pub enum CrmPipelineError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Pipeline not found")]
    NotFound,
    #[error("Stage not found")]
    StageNotFound,
    #[error("Pipeline with this name already exists")]
    AlreadyExists,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PipelineType {
    Conferences,
    Clients,
    Sales,
    Delivery,
    Custom,
}

impl std::fmt::Display for PipelineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PipelineType::Conferences => "conferences",
            PipelineType::Clients => "clients",
            PipelineType::Sales => "sales",
            PipelineType::Delivery => "delivery",
            PipelineType::Custom => "custom",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for PipelineType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "conferences" => Ok(PipelineType::Conferences),
            "clients" => Ok(PipelineType::Clients),
            "sales" => Ok(PipelineType::Sales),
            "delivery" => Ok(PipelineType::Delivery),
            "custom" => Ok(PipelineType::Custom),
            _ => Err(format!("Unknown pipeline type: {s}")),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CrmPipeline {
    pub id: DbUuid,
    pub project_id: Option<DbUuid>,
    pub organization_id: Option<DbUuid>,
    pub name: String,
    pub description: Option<String>,
    pub pipeline_type: String,
    pub is_active: Option<i32>,
    pub is_default: Option<i32>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CrmPipelineStage {
    pub id: DbUuid,
    pub pipeline_id: DbUuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub position: i32,
    pub is_closed: Option<i32>,
    pub is_won: Option<i32>,
    pub probability: i32,
    pub auto_move_after_days: Option<i32>,
    pub notify_on_enter: Option<i32>,
    pub stage_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCrmPipeline {
    pub organization_id: Option<DbUuid>,
    pub client_id: Option<DbUuid>,
    pub name: String,
    pub description: Option<String>,
    pub pipeline_type: PipelineType,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCrmPipelineStage {
    pub pipeline_id: DbUuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub position: i32,
    pub is_closed: Option<bool>,
    pub is_won: Option<bool>,
    pub probability: i32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCrmPipeline {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCrmPipelineStage {
    pub name: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub position: Option<i32>,
    pub is_closed: Option<bool>,
    pub is_won: Option<bool>,
    pub probability: Option<i32>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CrmPipelineWithStages {
    #[serde(flatten)]
    pub pipeline: CrmPipeline,
    pub stages: Vec<CrmPipelineStage>,
}

impl CrmPipeline {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateCrmPipeline,
    ) -> Result<Self, CrmPipelineError> {
        let id = DbUuid::new();
        let pipeline_type = data.pipeline_type.to_string();

        let pipeline = sqlx::query_as::<_, CrmPipeline>(
            r#"
            INSERT INTO crm_pipelines (
                id, organization_id, project_id, client_id,
                name, description, pipeline_type, icon, color
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&data.organization_id)
        .bind(None::<DbUuid>)
        .bind(&data.client_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&pipeline_type)
        .bind(&data.icon)
        .bind(&data.color)
        .fetch_one(pool)
        .await?;

        Ok(pipeline)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &DbUuid) -> Result<Self, CrmPipelineError> {
        sqlx::query_as::<_, CrmPipeline>(r#"SELECT * FROM crm_pipelines WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(CrmPipelineError::NotFound)
    }

    /// Find pipelines by project (legacy/compatibility)
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: &DbUuid,
    ) -> Result<Vec<Self>, CrmPipelineError> {
        let pipelines = sqlx::query_as::<_, CrmPipeline>(
            r#"SELECT * FROM crm_pipelines WHERE project_id = ?1 AND is_active = 1 ORDER BY name"#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(pipelines)
    }

    /// List all active pipelines in an organization (direct org_id query)
    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        pipeline_type: Option<PipelineType>,
    ) -> Result<Vec<Self>, CrmPipelineError> {
        let pipelines = if let Some(pt) = pipeline_type {
            let pt_str = pt.to_string();
            sqlx::query_as::<_, CrmPipeline>(
                r#"SELECT * FROM crm_pipelines
                   WHERE organization_id = ?1 AND is_active = 1 AND pipeline_type = ?2
                   ORDER BY name"#,
            )
            .bind(organization_id)
            .bind(&pt_str)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, CrmPipeline>(
                r#"SELECT * FROM crm_pipelines
                   WHERE organization_id = ?1 AND is_active = 1
                   ORDER BY name"#,
            )
            .bind(organization_id)
            .fetch_all(pool)
            .await?
        };

        Ok(pipelines)
    }

    /// Find pipeline by type scoped to an organization
    pub async fn find_by_type_for_org(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        pipeline_type: PipelineType,
    ) -> Result<Option<Self>, CrmPipelineError> {
        let pipeline_type_str = pipeline_type.to_string();
        let pipeline = sqlx::query_as::<_, CrmPipeline>(
            r#"SELECT * FROM crm_pipelines WHERE organization_id = ?1 AND pipeline_type = ?2 AND is_active = 1"#,
        )
        .bind(organization_id)
        .bind(&pipeline_type_str)
        .fetch_optional(pool)
        .await?;

        Ok(pipeline)
    }

    /// Find pipeline by type scoped to a project (legacy/compatibility)
    pub async fn find_by_type(
        pool: &SqlitePool,
        project_id: &DbUuid,
        pipeline_type: PipelineType,
    ) -> Result<Option<Self>, CrmPipelineError> {
        let pipeline_type_str = pipeline_type.to_string();
        let pipeline = sqlx::query_as::<_, CrmPipeline>(
            r#"SELECT * FROM crm_pipelines WHERE project_id = ?1 AND pipeline_type = ?2 AND is_active = 1"#,
        )
        .bind(project_id)
        .bind(&pipeline_type_str)
        .fetch_optional(pool)
        .await?;

        Ok(pipeline)
    }

    pub async fn find_with_stages(
        pool: &SqlitePool,
        id: &DbUuid,
    ) -> Result<CrmPipelineWithStages, CrmPipelineError> {
        let pipeline = Self::find_by_id(pool, id).await?;
        let stages = CrmPipelineStage::find_by_pipeline(pool, id).await?;

        Ok(CrmPipelineWithStages { pipeline, stages })
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &DbUuid,
        data: UpdateCrmPipeline,
    ) -> Result<Self, CrmPipelineError> {
        let is_active = data.is_active.map(|b| if b { 1 } else { 0 });

        sqlx::query_as::<_, CrmPipeline>(
            r#"
            UPDATE crm_pipelines SET
                name = COALESCE(?2, name),
                description = COALESCE(?3, description),
                is_active = COALESCE(?4, is_active),
                icon = COALESCE(?5, icon),
                color = COALESCE(?6, color),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(is_active)
        .bind(&data.icon)
        .bind(&data.color)
        .fetch_optional(pool)
        .await?
        .ok_or(CrmPipelineError::NotFound)
    }

    pub async fn delete(pool: &SqlitePool, id: &DbUuid) -> Result<(), CrmPipelineError> {
        let result = sqlx::query(r#"DELETE FROM crm_pipelines WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CrmPipelineError::NotFound);
        }

        Ok(())
    }

    /// Ensure default pipelines exist for an organization
    pub async fn ensure_defaults_for_org(
        pool: &SqlitePool,
        organization_id: &DbUuid,
    ) -> Result<(), CrmPipelineError> {
        let existing = Self::find_by_organization(pool, organization_id, None).await?;

        if !existing.iter().any(|p| p.pipeline_type == "conferences") {
            Self::create_conferences_pipeline(pool, Some(organization_id)).await?;
        }
        if !existing.iter().any(|p| p.pipeline_type == "clients") {
            Self::create_clients_pipeline(pool, Some(organization_id)).await?;
        }
        if !existing.iter().any(|p| p.pipeline_type == "sales") {
            Self::create_sales_pipeline(pool, Some(organization_id)).await?;
        }
        if !existing.iter().any(|p| p.pipeline_type == "delivery") {
            Self::create_delivery_pipeline(pool, Some(organization_id)).await?;
        }

        Ok(())
    }

    /// Ensure default pipelines exist for a project (legacy/compatibility)
    pub async fn ensure_defaults(
        pool: &SqlitePool,
        project_id: &DbUuid,
    ) -> Result<(), CrmPipelineError> {
        if Self::find_by_type(pool, project_id, PipelineType::Conferences)
            .await?
            .is_none()
        {
            // For project-based defaults, we don't have an org_id easily, so pass None
            Self::create_conferences_pipeline(pool, None).await?;
        }

        if Self::find_by_type(pool, project_id, PipelineType::Clients)
            .await?
            .is_none()
        {
            Self::create_clients_pipeline(pool, None).await?;
        }

        if Self::find_by_type(pool, project_id, PipelineType::Sales)
            .await?
            .is_none()
        {
            Self::create_sales_pipeline(pool, None).await?;
        }

        if Self::find_by_type(pool, project_id, PipelineType::Delivery)
            .await?
            .is_none()
        {
            Self::create_delivery_pipeline(pool, None).await?;
        }

        Ok(())
    }

    async fn create_conferences_pipeline(
        pool: &SqlitePool,
        organization_id: Option<&DbUuid>,
    ) -> Result<Self, CrmPipelineError> {
        let pipeline = Self::create(
            pool,
            CreateCrmPipeline {
                organization_id: organization_id.cloned(),
                client_id: None,
                name: "Conferences".to_string(),
                description: Some("Track conference applications and attendance".to_string()),
                pipeline_type: PipelineType::Conferences,
                icon: Some("calendar".to_string()),
                color: Some("#8B5CF6".to_string()),
            },
        )
        .await?;

        // Create default stages for conferences
        let stages = vec![
            ("Researching", "#6B7280", 0, false, false, 5),
            ("Applied", "#3B82F6", 1, false, false, 15),
            ("In Discussion", "#8B5CF6", 2, false, false, 35),
            ("Confirmed", "#22C55E", 3, false, false, 90),
            ("Attended", "#22C55E", 4, true, true, 100),
            ("Declined", "#9CA3AF", 5, true, false, 0),
        ];

        for (name, color, position, is_closed, is_won, probability) in stages {
            CrmPipelineStage::create(
                pool,
                CreateCrmPipelineStage {
                    pipeline_id: pipeline.id.clone(),
                    name: name.to_string(),
                    description: None,
                    color: color.to_string(),
                    position,
                    is_closed: Some(is_closed),
                    is_won: Some(is_won),
                    probability,
                },
            )
            .await?;
        }

        Ok(pipeline)
    }

    async fn create_clients_pipeline(
        pool: &SqlitePool,
        organization_id: Option<&DbUuid>,
    ) -> Result<Self, CrmPipelineError> {
        let pipeline = Self::create(
            pool,
            CreateCrmPipeline {
                organization_id: organization_id.cloned(),
                client_id: None,
                name: "Clients".to_string(),
                description: Some("Track client acquisition pipeline".to_string()),
                pipeline_type: PipelineType::Clients,
                icon: Some("users".to_string()),
                color: Some("#3B82F6".to_string()),
            },
        )
        .await?;

        // Create default stages for clients (8-stage pipeline with human review gates)
        let stages = vec![
            ("Lead", "#6B7280", 0, false, false, 5),
            ("Business Analysis", "#3B82F6", 1, false, false, 20),
            ("Discovery", "#8B5CF6", 2, false, false, 40),
            ("Build Proposal", "#F59E0B", 3, false, false, 55),
            ("Polish", "#EC4899", 4, false, false, 70),
            ("Proposal Meeting", "#EF4444", 5, false, false, 85),
            ("Closed Won", "#22C55E", 6, true, true, 100),
            ("Closed Lost", "#9CA3AF", 7, true, false, 0),
        ];

        for (name, color, position, is_closed, is_won, probability) in stages {
            CrmPipelineStage::create(
                pool,
                CreateCrmPipelineStage {
                    pipeline_id: pipeline.id.clone(),
                    name: name.to_string(),
                    description: None,
                    color: color.to_string(),
                    position,
                    is_closed: Some(is_closed),
                    is_won: Some(is_won),
                    probability,
                },
            )
            .await?;
        }

        Ok(pipeline)
    }

    async fn create_sales_pipeline(
        pool: &SqlitePool,
        organization_id: Option<&DbUuid>,
    ) -> Result<Self, CrmPipelineError> {
        let pipeline = Self::create(
            pool,
            CreateCrmPipeline {
                organization_id: organization_id.cloned(),
                client_id: None,
                name: "Acquisition".to_string(),
                description: Some("8-stage client acquisition: Lead → Business Analysis → Discovery → Build Proposal → Polish → Proposal Meeting → Won/Lost".to_string()),
                pipeline_type: PipelineType::Sales,
                icon: Some("trending-up".to_string()),
                color: Some("#3B82F6".to_string()),
            },
        )
        .await?;

        let stages = vec![
            ("Lead", "#6B7280", 0, false, false, 5),
            ("Business Analysis", "#3B82F6", 1, false, false, 20),
            ("Discovery", "#8B5CF6", 2, false, false, 40),
            ("Build Proposal", "#F59E0B", 3, false, false, 55),
            ("Polish", "#EC4899", 4, false, false, 70),
            ("Proposal Meeting", "#EF4444", 5, false, false, 85),
            ("Closed Won", "#22C55E", 6, true, true, 100),
            ("Closed Lost", "#9CA3AF", 7, true, false, 0),
        ];

        for (name, color, position, is_closed, is_won, probability) in stages {
            CrmPipelineStage::create(
                pool,
                CreateCrmPipelineStage {
                    pipeline_id: pipeline.id.clone(),
                    name: name.to_string(),
                    description: None,
                    color: color.to_string(),
                    position,
                    is_closed: Some(is_closed),
                    is_won: Some(is_won),
                    probability,
                },
            )
            .await?;
        }

        Ok(pipeline)
    }

    async fn create_delivery_pipeline(
        pool: &SqlitePool,
        organization_id: Option<&DbUuid>,
    ) -> Result<Self, CrmPipelineError> {
        let pipeline = Self::create(
            pool,
            CreateCrmPipeline {
                organization_id: organization_id.cloned(),
                client_id: None,
                name: "Client Delivery".to_string(),
                description: Some("Client delivery pipeline: Onboarding → Brand Guide → Online Presence → Social Stack → Monthly Retainer".to_string()),
                pipeline_type: PipelineType::Delivery,
                icon: Some("package".to_string()),
                color: Some("#8B5CF6".to_string()),
            },
        )
        .await?;

        let stages = vec![
            ("Onboarding", "#3B82F6", 0, false, false, 5),
            ("Brand Guide", "#8B5CF6", 1, false, false, 20),
            ("Online Presence", "#F59E0B", 2, false, false, 40),
            ("Social Stack", "#EC4899", 3, false, false, 60),
            ("Monthly Retainer", "#14B8A6", 4, false, false, 80),
            ("Active Retainer", "#22C55E", 5, true, true, 100),
            ("Completed", "#9CA3AF", 6, true, true, 100),
        ];

        for (name, color, position, is_closed, is_won, probability) in stages {
            CrmPipelineStage::create(
                pool,
                CreateCrmPipelineStage {
                    pipeline_id: pipeline.id.clone(),
                    name: name.to_string(),
                    description: None,
                    color: color.to_string(),
                    position,
                    is_closed: Some(is_closed),
                    is_won: Some(is_won),
                    probability,
                },
            )
            .await?;
        }

        Ok(pipeline)
    }
}

impl CrmPipelineStage {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateCrmPipelineStage,
    ) -> Result<Self, CrmPipelineError> {
        let id = DbUuid::new();
        let is_closed = data.is_closed.map(|b| if b { 1 } else { 0 }).unwrap_or(0);
        let is_won = data.is_won.map(|b| if b { 1 } else { 0 }).unwrap_or(0);

        let stage = sqlx::query_as::<_, CrmPipelineStage>(
            r#"
            INSERT INTO crm_pipeline_stages (
                id, pipeline_id, name, description, color, position, is_closed, is_won, probability
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&data.pipeline_id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.color)
        .bind(data.position)
        .bind(is_closed)
        .bind(is_won)
        .bind(data.probability)
        .fetch_one(pool)
        .await?;

        Ok(stage)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: &DbUuid) -> Result<Self, CrmPipelineError> {
        sqlx::query_as::<_, CrmPipelineStage>(r#"SELECT * FROM crm_pipeline_stages WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(CrmPipelineError::StageNotFound)
    }

    pub async fn find_by_pipeline(
        pool: &SqlitePool,
        pipeline_id: &DbUuid,
    ) -> Result<Vec<Self>, CrmPipelineError> {
        let stages = sqlx::query_as::<_, CrmPipelineStage>(
            r#"SELECT * FROM crm_pipeline_stages WHERE pipeline_id = ?1 ORDER BY position"#,
        )
        .bind(pipeline_id)
        .fetch_all(pool)
        .await?;

        Ok(stages)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: &DbUuid,
        data: UpdateCrmPipelineStage,
    ) -> Result<Self, CrmPipelineError> {
        let is_closed = data.is_closed.map(|b| if b { 1 } else { 0 });
        let is_won = data.is_won.map(|b| if b { 1 } else { 0 });

        sqlx::query_as::<_, CrmPipelineStage>(
            r#"
            UPDATE crm_pipeline_stages SET
                name = COALESCE(?2, name),
                description = COALESCE(?3, description),
                color = COALESCE(?4, color),
                position = COALESCE(?5, position),
                is_closed = COALESCE(?6, is_closed),
                is_won = COALESCE(?7, is_won),
                probability = COALESCE(?8, probability),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(&data.color)
        .bind(data.position)
        .bind(is_closed)
        .bind(is_won)
        .bind(data.probability)
        .fetch_optional(pool)
        .await?
        .ok_or(CrmPipelineError::StageNotFound)
    }

    pub async fn delete(pool: &SqlitePool, id: &DbUuid) -> Result<(), CrmPipelineError> {
        let result = sqlx::query(r#"DELETE FROM crm_pipeline_stages WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CrmPipelineError::StageNotFound);
        }

        Ok(())
    }

    /// Reorder stages within a pipeline
    pub async fn reorder(
        pool: &SqlitePool,
        pipeline_id: &DbUuid,
        stage_ids: Vec<DbUuid>,
    ) -> Result<Vec<Self>, CrmPipelineError> {
        for (position, stage_id) in stage_ids.iter().enumerate() {
            sqlx::query(
                r#"
                UPDATE crm_pipeline_stages SET
                    position = ?2,
                    updated_at = datetime('now', 'subsec')
                WHERE id = ?1 AND pipeline_id = ?3
                "#,
            )
            .bind(stage_id)
            .bind(position as i32)
            .bind(pipeline_id)
            .execute(pool)
            .await?;
        }

        Self::find_by_pipeline(pool, pipeline_id).await
    }
}
