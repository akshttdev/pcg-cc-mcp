use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

use super::crm_pipeline::CrmPipelineStage;

#[derive(Debug, Error)]
pub enum CrmDealError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Deal not found")]
    NotFound,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CrmDeal {
    pub id: Uuid,
    pub project_id: Uuid,
    pub crm_contact_id: Option<Uuid>,
    pub crm_pipeline_id: Option<Uuid>,
    pub crm_stage_id: Option<Uuid>,
    pub position: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub amount: Option<f64>,
    pub currency: String,
    pub pipeline: String,
    pub stage: String,
    pub probability: i32,
    pub expected_close_date: Option<DateTime<Utc>>,
    pub actual_close_date: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub owner_user_id: Option<String>,
    pub assigned_agent_id: Option<Uuid>,
    pub zoho_deal_id: Option<String>,
    pub external_ids: Option<String>,
    pub tags: Option<String>,
    pub custom_fields: Option<String>,
    pub lost_reason: Option<String>,
    pub win_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCrmDeal {
    pub project_id: Uuid,
    pub crm_contact_id: Option<Uuid>,
    pub crm_pipeline_id: Option<Uuid>,
    pub crm_stage_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub expected_close_date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCrmDeal {
    pub crm_contact_id: Option<Uuid>,
    pub crm_pipeline_id: Option<Uuid>,
    pub crm_stage_id: Option<Uuid>,
    pub position: Option<i32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub expected_close_date: Option<String>,
    pub owner_user_id: Option<String>,
    pub assigned_agent_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub lost_reason: Option<String>,
    pub win_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveDealRequest {
    pub stage_id: Uuid,
    pub position: i32,
}

/// Deal with associated contact info for Kanban display
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CrmDealWithContact {
    #[serde(flatten)]
    pub deal: CrmDeal,
    pub contact_name: Option<String>,
    pub contact_email: Option<String>,
    pub contact_company: Option<String>,
    pub contact_avatar_url: Option<String>,
}

/// Kanban board data structure - deals grouped by stage
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct KanbanBoardData {
    pub pipeline_id: Uuid,
    pub pipeline_name: String,
    pub stages: Vec<KanbanStageWithDeals>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct KanbanStageWithDeals {
    pub stage: CrmPipelineStage,
    pub deals: Vec<CrmDealWithContact>,
    pub total_amount: f64,
}

impl CrmDeal {
    pub async fn create(pool: &SqlitePool, data: CreateCrmDeal) -> Result<Self, CrmDealError> {
        let id = Uuid::new_v4();
        let currency = data.currency.unwrap_or_else(|| "USD".to_string());
        let tags = data.tags.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let custom_fields = data.custom_fields.map(|v| v.to_string());

        // Get stage probability if stage is specified
        let (probability, stage_name) = if let Some(stage_id) = data.crm_stage_id {
            match CrmPipelineStage::find_by_id(pool, stage_id).await {
                Ok(stage) => (stage.probability, stage.name.clone()),
                Err(_) => (0, "qualification".to_string()),
            }
        } else {
            (0, "qualification".to_string())
        };

        // Calculate next position in stage
        let position = if let Some(stage_id) = data.crm_stage_id {
            let max_pos: Option<(i32,)> = sqlx::query_as(
                r#"SELECT COALESCE(MAX(position), -1) FROM crm_deals WHERE crm_stage_id = ?1"#,
            )
            .bind(stage_id)
            .fetch_optional(pool)
            .await?;
            max_pos.map(|(p,)| p + 1).unwrap_or(0)
        } else {
            0
        };

        let deal = sqlx::query_as::<_, CrmDeal>(
            r#"
            INSERT INTO crm_deals (
                id, project_id, crm_contact_id, crm_pipeline_id, crm_stage_id, position,
                name, description, amount, currency, stage, probability,
                expected_close_date, tags, custom_fields
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(data.crm_contact_id)
        .bind(data.crm_pipeline_id)
        .bind(data.crm_stage_id)
        .bind(position)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.amount)
        .bind(&currency)
        .bind(&stage_name)
        .bind(probability)
        .bind(&data.expected_close_date)
        .bind(tags)
        .bind(custom_fields)
        .fetch_one(pool)
        .await?;

        Ok(deal)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, CrmDealError> {
        sqlx::query_as::<_, CrmDeal>(r#"SELECT * FROM crm_deals WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(CrmDealError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, CrmDealError> {
        let deals = sqlx::query_as::<_, CrmDeal>(
            r#"SELECT * FROM crm_deals WHERE project_id = ?1 ORDER BY created_at DESC"#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(deals)
    }

    pub async fn find_by_pipeline(
        pool: &SqlitePool,
        pipeline_id: Uuid,
    ) -> Result<Vec<Self>, CrmDealError> {
        let deals = sqlx::query_as::<_, CrmDeal>(
            r#"SELECT * FROM crm_deals WHERE crm_pipeline_id = ?1 ORDER BY position"#,
        )
        .bind(pipeline_id)
        .fetch_all(pool)
        .await?;

        Ok(deals)
    }

    pub async fn find_by_stage(
        pool: &SqlitePool,
        stage_id: Uuid,
    ) -> Result<Vec<Self>, CrmDealError> {
        let deals = sqlx::query_as::<_, CrmDeal>(
            r#"SELECT * FROM crm_deals WHERE crm_stage_id = ?1 ORDER BY position"#,
        )
        .bind(stage_id)
        .fetch_all(pool)
        .await?;

        Ok(deals)
    }

    pub async fn find_by_contact(
        pool: &SqlitePool,
        contact_id: Uuid,
    ) -> Result<Vec<Self>, CrmDealError> {
        let deals = sqlx::query_as::<_, CrmDeal>(
            r#"SELECT * FROM crm_deals WHERE crm_contact_id = ?1 ORDER BY created_at DESC"#,
        )
        .bind(contact_id)
        .fetch_all(pool)
        .await?;

        Ok(deals)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateCrmDeal,
    ) -> Result<Self, CrmDealError> {
        let tags = data.tags.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let custom_fields = data.custom_fields.map(|v| v.to_string());

        sqlx::query_as::<_, CrmDeal>(
            r#"
            UPDATE crm_deals SET
                crm_contact_id = COALESCE(?2, crm_contact_id),
                crm_pipeline_id = COALESCE(?3, crm_pipeline_id),
                crm_stage_id = COALESCE(?4, crm_stage_id),
                position = COALESCE(?5, position),
                name = COALESCE(?6, name),
                description = COALESCE(?7, description),
                amount = COALESCE(?8, amount),
                currency = COALESCE(?9, currency),
                expected_close_date = COALESCE(?10, expected_close_date),
                owner_user_id = COALESCE(?11, owner_user_id),
                assigned_agent_id = COALESCE(?12, assigned_agent_id),
                tags = COALESCE(?13, tags),
                custom_fields = COALESCE(?14, custom_fields),
                lost_reason = COALESCE(?15, lost_reason),
                win_reason = COALESCE(?16, win_reason),
                last_activity_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.crm_contact_id)
        .bind(data.crm_pipeline_id)
        .bind(data.crm_stage_id)
        .bind(data.position)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.amount)
        .bind(&data.currency)
        .bind(&data.expected_close_date)
        .bind(&data.owner_user_id)
        .bind(data.assigned_agent_id)
        .bind(tags)
        .bind(custom_fields)
        .bind(&data.lost_reason)
        .bind(&data.win_reason)
        .fetch_optional(pool)
        .await?
        .ok_or(CrmDealError::NotFound)
    }

    /// Move deal to a new stage and position (for drag-drop)
    pub async fn move_to_stage(
        pool: &SqlitePool,
        id: Uuid,
        stage_id: Uuid,
        new_position: i32,
    ) -> Result<Self, CrmDealError> {
        let deal = Self::find_by_id(pool, id).await?;
        let old_stage_id = deal.crm_stage_id;

        // Get the new stage to update probability and stage name
        let new_stage = CrmPipelineStage::find_by_id(pool, stage_id).await.ok();

        // Shift positions in the target stage to make room
        sqlx::query(
            r#"
            UPDATE crm_deals SET
                position = position + 1
            WHERE crm_stage_id = ?1 AND position >= ?2 AND id != ?3
            "#,
        )
        .bind(stage_id)
        .bind(new_position)
        .bind(id)
        .execute(pool)
        .await?;

        // If moving between stages, compact the old stage
        if old_stage_id != Some(stage_id) {
            if let Some(old_sid) = old_stage_id {
                sqlx::query(
                    r#"
                    UPDATE crm_deals SET
                        position = position - 1
                    WHERE crm_stage_id = ?1 AND position > ?2
                    "#,
                )
                .bind(old_sid)
                .bind(deal.position.unwrap_or(0))
                .execute(pool)
                .await?;
            }
        }

        // Update the deal with new stage and position
        let (probability, stage_name, is_closed, is_won) = if let Some(ref stage) = new_stage {
            (
                stage.probability,
                stage.name.clone(),
                stage.is_closed.unwrap_or(0) == 1,
                stage.is_won.unwrap_or(0) == 1,
            )
        } else {
            (deal.probability, deal.stage.clone(), false, false)
        };

        // Set actual_close_date if moving to a closed stage
        let close_date_update = if is_closed {
            ", actual_close_date = COALESCE(actual_close_date, datetime('now', 'subsec'))"
        } else {
            ""
        };

        let query = format!(
            r#"
            UPDATE crm_deals SET
                crm_stage_id = ?2,
                position = ?3,
                stage = ?4,
                probability = ?5,
                last_activity_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
                {}
            WHERE id = ?1
            RETURNING *
            "#,
            close_date_update
        );

        let updated_deal = sqlx::query_as::<_, CrmDeal>(&query)
            .bind(id)
            .bind(stage_id)
            .bind(new_position)
            .bind(&stage_name)
            .bind(probability)
            .fetch_one(pool)
            .await?;

        // Log stage change activity if stage changed
        if old_stage_id != Some(stage_id) {
            let old_stage_name = if let Some(old_sid) = old_stage_id {
                CrmPipelineStage::find_by_id(pool, old_sid)
                    .await
                    .map(|s| s.name)
                    .unwrap_or_else(|_| "Unknown".to_string())
            } else {
                "None".to_string()
            };

            let activity_type = if is_won {
                "deal_won"
            } else if is_closed {
                "deal_lost"
            } else {
                "deal_stage_changed"
            };

            // Insert activity record
            sqlx::query(
                r#"
                INSERT INTO crm_activities (
                    id, project_id, crm_contact_id, crm_deal_id, activity_type,
                    subject, description, activity_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now', 'subsec'))
                "#,
            )
            .bind(Uuid::new_v4())
            .bind(updated_deal.project_id)
            .bind(updated_deal.crm_contact_id)
            .bind(updated_deal.id)
            .bind(activity_type)
            .bind(format!("Moved to {}", stage_name))
            .bind(format!("Deal moved from {} to {}", old_stage_name, stage_name))
            .execute(pool)
            .await?;
        }

        Ok(updated_deal)
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), CrmDealError> {
        let result = sqlx::query(r#"DELETE FROM crm_deals WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CrmDealError::NotFound);
        }

        Ok(())
    }

    /// Get Kanban board data for a pipeline
    pub async fn get_kanban_data(
        pool: &SqlitePool,
        pipeline_id: Uuid,
    ) -> Result<KanbanBoardData, CrmDealError> {
        use super::crm_contact::CrmContact;
        use super::crm_pipeline::CrmPipeline;

        let pipeline = CrmPipeline::find_by_id(pool, pipeline_id)
            .await
            .map_err(|_| CrmDealError::NotFound)?;

        let stages = CrmPipelineStage::find_by_pipeline(pool, pipeline_id)
            .await
            .map_err(|_| CrmDealError::NotFound)?;

        let mut kanban_stages = Vec::new();

        for stage in stages {
            // Get deals for this stage
            let deals = Self::find_by_stage(pool, stage.id).await?;

            // Enrich deals with contact info
            let mut deals_with_contacts = Vec::new();
            for deal in deals {
                let contact_info = if let Some(contact_id) = deal.crm_contact_id {
                    CrmContact::find_by_id(pool, contact_id).await.ok()
                } else {
                    None
                };

                deals_with_contacts.push(CrmDealWithContact {
                    contact_name: contact_info.as_ref().and_then(|c| c.full_name.clone()),
                    contact_email: contact_info.as_ref().and_then(|c| c.email.clone()),
                    contact_company: contact_info.as_ref().and_then(|c| c.company_name.clone()),
                    contact_avatar_url: contact_info.as_ref().and_then(|c| c.avatar_url.clone()),
                    deal,
                });
            }

            let total_amount: f64 = deals_with_contacts
                .iter()
                .filter_map(|d| d.deal.amount)
                .sum();

            kanban_stages.push(KanbanStageWithDeals {
                stage,
                deals: deals_with_contacts,
                total_amount,
            });
        }

        Ok(KanbanBoardData {
            pipeline_id,
            pipeline_name: pipeline.name,
            stages: kanban_stages,
        })
    }
}
