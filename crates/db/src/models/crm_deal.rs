use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;

use super::crm_pipeline::CrmPipelineStage;
use crate::db_uuid::DbUuid;

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
    pub id: DbUuid,
    pub project_id: Option<DbUuid>,
    pub organization_id: Option<DbUuid>,
    pub client_id: Option<DbUuid>,
    pub crm_contact_id: Option<DbUuid>,
    pub crm_pipeline_id: Option<DbUuid>,
    pub crm_stage_id: Option<DbUuid>,
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
    pub assigned_agent_id: Option<DbUuid>,
    pub zoho_deal_id: Option<String>,
    pub external_ids: Option<String>,
    pub tags: Option<String>,
    pub custom_fields: Option<String>,
    pub lost_reason: Option<String>,
    pub win_reason: Option<String>,
    pub proposal_text: Option<String>,
    pub proposal_status: String,
    pub deck_url: Option<String>,
    pub invoice_id: Option<String>,
    pub won_at: Option<DateTime<Utc>>,
    pub lost_at: Option<DateTime<Utc>>,
    pub expedited: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCrmDeal {
    pub organization_id: DbUuid,
    pub client_id: Option<DbUuid>,
    pub crm_contact_id: Option<DbUuid>,
    pub crm_pipeline_id: Option<DbUuid>,
    pub crm_stage_id: Option<DbUuid>,
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
    pub crm_contact_id: Option<DbUuid>,
    pub crm_pipeline_id: Option<DbUuid>,
    pub crm_stage_id: Option<DbUuid>,
    pub project_id: Option<String>,
    pub position: Option<i32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub expected_close_date: Option<String>,
    pub owner_user_id: Option<String>,
    pub assigned_agent_id: Option<DbUuid>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub lost_reason: Option<String>,
    pub win_reason: Option<String>,
    pub proposal_text: Option<String>,
    pub proposal_status: Option<String>,
    pub deck_url: Option<String>,
    pub invoice_id: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DealTranscript {
    pub id: DbUuid,
    pub deal_id: DbUuid,
    pub intake_item_id: Option<DbUuid>,
    pub call_log_id: Option<String>,
    pub transcript_text: Option<String>,
    pub summary: Option<String>,
    pub matched_at: String,
    pub matched_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct LinkTranscriptRequest {
    pub intake_item_id: Option<String>,
    pub call_log_id: Option<String>,
    pub transcript_text: Option<String>,
    pub summary: Option<String>,
    pub matched_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MoveDealRequest {
    pub stage_id: DbUuid,
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
    pub project_name: Option<String>,
    pub task_total: i64,
    pub task_done: i64,
    pub deliverable_count: i64,
    /// person_id from crm_contacts — bridges to persons table for wiki/intel
    pub person_id: Option<DbUuid>,
    /// report_id — id of the business report (wiki) for this deal's person
    pub report_id: Option<DbUuid>,
    // Person intelligence fields
    pub intelligence_status: Option<String>,
    pub intelligence_summary: Option<String>,
    pub intelligence_confidence: Option<f64>,
    pub research_pass_count: Option<i64>,
    // Business report status
    pub report_status: Option<String>,
    pub report_review_status: Option<String>,
    // Review task
    pub review_task_id: Option<DbUuid>,
    pub review_task_status: Option<String>,
    pub review_task_assignee: Option<String>,
    // Company intelligence
    pub company_intelligence_status: Option<String>,
    pub company_intelligence_summary: Option<String>,
    pub company_id: Option<DbUuid>,
}

/// Kanban board data structure - deals grouped by stage
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct KanbanBoardData {
    pub pipeline_id: DbUuid,
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
        let id = DbUuid::new();
        let currency = data.currency.unwrap_or_else(|| "USD".to_string());
        let tags = data
            .tags
            .map(|v| serde_json::to_string(&v).unwrap_or_default());
        let custom_fields = data.custom_fields.map(|v| v.to_string());

        // Get stage probability if stage is specified
        // Note: legacy `stage` column has CHECK constraint with fixed values,
        // so we always default to 'qualification' and use crm_stage_id for real stage tracking
        let probability = if let Some(ref stage_id) = data.crm_stage_id {
            match CrmPipelineStage::find_by_id(pool, stage_id).await {
                Ok(stage) => stage.probability,
                Err(_) => 0,
            }
        } else {
            0
        };
        let stage_name = if let Some(ref stage_id) = data.crm_stage_id {
            match CrmPipelineStage::find_by_id(pool, stage_id).await {
                Ok(stage) => stage.name,
                Err(_) => "qualification".to_string(),
            }
        } else {
            "qualification".to_string()
        };

        // Calculate next position in stage
        let position = if let Some(ref stage_id) = data.crm_stage_id {
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
                id, organization_id, project_id, client_id,
                crm_contact_id, crm_pipeline_id, crm_stage_id, position,
                name, description, amount, currency, stage, probability,
                expected_close_date, tags, custom_fields
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            RETURNING *
            "#,
        )
        .bind(&id)
        .bind(&data.organization_id)
        .bind(None::<DbUuid>)
        .bind(&data.client_id)
        .bind(&data.crm_contact_id)
        .bind(&data.crm_pipeline_id)
        .bind(&data.crm_stage_id)
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

    pub async fn find_by_id(pool: &SqlitePool, id: &DbUuid) -> Result<Self, CrmDealError> {
        sqlx::query_as::<_, CrmDeal>(r#"SELECT * FROM crm_deals WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(CrmDealError::NotFound)
    }

    pub async fn find_by_organization(
        pool: &SqlitePool,
        organization_id: &DbUuid,
    ) -> Result<Vec<Self>, CrmDealError> {
        let deals = sqlx::query_as::<_, CrmDeal>(
            r#"SELECT * FROM crm_deals WHERE organization_id = ?1 ORDER BY created_at DESC"#,
        )
        .bind(organization_id)
        .fetch_all(pool)
        .await?;

        Ok(deals)
    }

    /// Find deals by project (legacy/compatibility)
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: &DbUuid,
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
        pipeline_id: &DbUuid,
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
        stage_id: &DbUuid,
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
        contact_id: &DbUuid,
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
        id: &DbUuid,
        data: UpdateCrmDeal,
    ) -> Result<Self, CrmDealError> {
        let tags = data
            .tags
            .map(|v| serde_json::to_string(&v).unwrap_or_default());
        let custom_fields = data.custom_fields.map(|v| v.to_string());

        sqlx::query_as::<_, CrmDeal>(
            r#"
            UPDATE crm_deals SET
                crm_contact_id = COALESCE(?2, crm_contact_id),
                crm_pipeline_id = COALESCE(?3, crm_pipeline_id),
                crm_stage_id = COALESCE(?4, crm_stage_id),
                project_id = COALESCE(?5, project_id),
                position = COALESCE(?6, position),
                name = COALESCE(?7, name),
                description = COALESCE(?8, description),
                amount = COALESCE(?9, amount),
                currency = COALESCE(?10, currency),
                expected_close_date = COALESCE(?11, expected_close_date),
                owner_user_id = COALESCE(?12, owner_user_id),
                assigned_agent_id = COALESCE(?13, assigned_agent_id),
                tags = COALESCE(?14, tags),
                custom_fields = COALESCE(?15, custom_fields),
                lost_reason = COALESCE(?16, lost_reason),
                win_reason = COALESCE(?17, win_reason),
                proposal_text = COALESCE(?18, proposal_text),
                proposal_status = COALESCE(?19, proposal_status),
                deck_url = COALESCE(?20, deck_url),
                invoice_id = COALESCE(?21, invoice_id),
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
        .bind(&data.project_id)
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
        .bind(&data.proposal_text)
        .bind(&data.proposal_status)
        .bind(&data.deck_url)
        .bind(&data.invoice_id)
        .fetch_optional(pool)
        .await?
        .ok_or(CrmDealError::NotFound)
    }

    /// Move deal to a new stage and position (for drag-drop)
    pub async fn move_to_stage(
        pool: &SqlitePool,
        id: &DbUuid,
        stage_id: &DbUuid,
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
        if old_stage_id.as_ref() != Some(stage_id) {
            if let Some(ref old_sid) = old_stage_id {
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
        if old_stage_id.as_ref() != Some(stage_id) {
            let old_stage_name = if let Some(ref old_sid) = old_stage_id {
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
                    id, organization_id, project_id, crm_contact_id, crm_deal_id, activity_type,
                    subject, description, activity_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now', 'subsec'))
                "#,
            )
            .bind(DbUuid::new())
            .bind(&updated_deal.organization_id)
            .bind(None::<DbUuid>)
            .bind(&updated_deal.crm_contact_id)
            .bind(&updated_deal.id)
            .bind(activity_type)
            .bind(format!("Moved to {}", stage_name))
            .bind(format!(
                "Deal moved from {} to {}",
                old_stage_name, stage_name
            ))
            .execute(pool)
            .await?;
        }

        Ok(updated_deal)
    }

    /// Find a deal by name + organization (fallback deduplication without requiring contact/pipeline match).
    pub async fn find_by_name_and_org(
        pool: &SqlitePool,
        name: &str,
        organization_id: &DbUuid,
    ) -> Result<Option<Self>, CrmDealError> {
        let deal = sqlx::query_as::<_, CrmDeal>(
            r#"SELECT * FROM crm_deals
               WHERE name = ?1 AND organization_id = ?2
               ORDER BY created_at DESC
               LIMIT 1"#,
        )
        .bind(name)
        .bind(organization_id)
        .fetch_optional(pool)
        .await?;
        Ok(deal)
    }

    /// Find a deal by name + contact + pipeline combination (for deduplication).
    pub async fn find_by_name_contact_pipeline(
        pool: &SqlitePool,
        name: &str,
        contact_id: &DbUuid,
        pipeline_id: &DbUuid,
    ) -> Result<Option<Self>, CrmDealError> {
        let deal = sqlx::query_as::<_, CrmDeal>(
            r#"SELECT * FROM crm_deals
               WHERE name = ?1 AND crm_contact_id = ?2 AND crm_pipeline_id = ?3
               LIMIT 1"#,
        )
        .bind(name)
        .bind(contact_id)
        .bind(pipeline_id)
        .fetch_optional(pool)
        .await?;
        Ok(deal)
    }

    pub async fn delete(pool: &SqlitePool, id: &DbUuid) -> Result<(), CrmDealError> {
        let result = sqlx::query(r#"DELETE FROM crm_deals WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CrmDealError::NotFound);
        }

        Ok(())
    }

    /// Fetch project name + task/deliverable counts for a given project.
    async fn fetch_project_stats(
        pool: &SqlitePool,
        project_id: &DbUuid,
    ) -> (Option<String>, i64, i64, i64) {
        let project_name: Option<String> =
            sqlx::query_scalar("SELECT name FROM projects WHERE id = ?")
                .bind(project_id)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten();

        let task_total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND deleted_at IS NULL",
        )
        .bind(project_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let task_done: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks WHERE project_id = ? AND status = 'done' AND deleted_at IS NULL",
        )
        .bind(project_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let deliverable_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM deliverables WHERE project_id = ?")
                .bind(project_id)
                .fetch_one(pool)
                .await
                .unwrap_or(0);

        (project_name, task_total, task_done, deliverable_count)
    }

    /// Get Kanban board data for a pipeline
    /// Look up the business_report id for a given person (latest report)
    pub async fn report_id_for_person_pub(pool: &SqlitePool, person_id: &DbUuid) -> Option<DbUuid> {
        Self::report_id_for_person(pool, person_id).await
    }

    pub async fn fetch_project_stats_pub(
        pool: &SqlitePool,
        project_id: &DbUuid,
    ) -> (Option<String>, i64, i64, i64) {
        Self::fetch_project_stats(pool, project_id).await
    }

    #[allow(clippy::type_complexity)]
    pub async fn fetch_intel_data_pub(
        pool: &SqlitePool,
        deal_id: &DbUuid,
        person_id: Option<&DbUuid>,
    ) -> (
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<DbUuid>,
        Option<String>,
        Option<String>,
    ) {
        Self::fetch_intel_data(pool, deal_id, person_id).await
    }

    async fn report_id_for_person(pool: &SqlitePool, person_id: &DbUuid) -> Option<DbUuid> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: DbUuid,
        }
        sqlx::query_as::<_, Row>(
            "SELECT id FROM business_reports WHERE person_id = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(person_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(|r| r.id)
    }

    /// Fetch intelligence + report + task data for a deal.
    /// Returns (intelligence_status, intelligence_summary, intelligence_confidence, research_pass_count,
    ///          report_status, report_review_status, review_task_id, review_task_status, review_task_assignee)
    #[allow(clippy::type_complexity)]
    async fn fetch_intel_data(
        pool: &SqlitePool,
        deal_id: &DbUuid,
        person_id: Option<&DbUuid>,
    ) -> (
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<DbUuid>,
        Option<String>,
        Option<String>,
    ) {
        // Person intelligence
        let (
            intelligence_status,
            intelligence_summary,
            intelligence_confidence,
            research_pass_count,
        ) = if let Some(pid) = person_id {
            #[derive(sqlx::FromRow)]
            struct PersonIntel {
                intelligence_status: Option<String>,
                intelligence_summary: Option<String>,
                intelligence_confidence: Option<f64>,
                research_pass_count: Option<i64>,
            }
            if let Ok(Some(intel)) = sqlx::query_as::<_, PersonIntel>(
                    "SELECT intelligence_status, intelligence_summary, intelligence_confidence, research_pass_count FROM persons WHERE id = ?"
                )
                .bind(pid)
                .fetch_optional(pool)
                .await
                {
                    (intel.intelligence_status, intel.intelligence_summary, intel.intelligence_confidence, intel.research_pass_count)
                } else {
                    (None, None, None, None)
                }
        } else {
            (None, None, None, None)
        };

        // Business report status (via deal id bridge)
        let (report_status, report_review_status) = {
            #[derive(sqlx::FromRow)]
            struct ReportStatus {
                status: Option<String>,
                review_status: Option<String>,
            }
            if let Ok(Some(rs)) = sqlx::query_as::<_, ReportStatus>(
                "SELECT status, review_status FROM business_reports WHERE crm_deal_id = ? ORDER BY created_at DESC LIMIT 1"
            )
            .bind(deal_id)
            .fetch_optional(pool)
            .await
            {
                (rs.status, rs.review_status)
            } else {
                (None, None)
            }
        };

        // Review task (active task linked to this deal)
        let (review_task_id, review_task_status, review_task_assignee) = {
            #[derive(sqlx::FromRow)]
            struct TaskRow {
                id: DbUuid,
                status: Option<String>,
                assignee_name: Option<String>,
            }
            if let Ok(Some(t)) = sqlx::query_as::<_, TaskRow>(
                r#"
                SELECT t.id, t.status, u.full_name as assignee_name
                FROM tasks t
                LEFT JOIN users u ON CAST(u.id AS TEXT) = t.assignee_id
                WHERE t.crm_deal_id = ?
                  AND t.status NOT IN ('cancelled', 'done')
                  AND t.deleted_at IS NULL
                ORDER BY t.created_at ASC
                LIMIT 1
                "#,
            )
            .bind(deal_id)
            .fetch_optional(pool)
            .await
            {
                (Some(t.id), t.status, t.assignee_name)
            } else {
                (None, None, None)
            }
        };

        (
            intelligence_status,
            intelligence_summary,
            intelligence_confidence,
            research_pass_count,
            report_status,
            report_review_status,
            review_task_id,
            review_task_status,
            review_task_assignee,
        )
    }

    /// Look up company intelligence status, summary and id by company name
    pub async fn fetch_company_intel_status(
        pool: &SqlitePool,
        company_name: Option<&str>,
    ) -> (Option<String>, Option<DbUuid>, Option<String>) {
        let Some(company_name) = company_name else {
            return (None, None, None);
        };
        if company_name.is_empty() {
            return (None, None, None);
        }
        #[derive(sqlx::FromRow)]
        struct Row {
            id: DbUuid,
            intelligence_status: Option<String>,
            intelligence_summary: Option<String>,
        }
        if let Ok(Some(row)) = sqlx::query_as::<_, Row>(
            "SELECT id, intelligence_status, intelligence_summary FROM companies WHERE name = ? COLLATE NOCASE LIMIT 1",
        )
        .bind(company_name)
        .fetch_optional(pool)
        .await
        {
            (row.intelligence_status, Some(row.id), row.intelligence_summary)
        } else {
            (None, None, None)
        }
    }

    pub async fn get_kanban_data(
        pool: &SqlitePool,
        pipeline_id: &DbUuid,
    ) -> Result<KanbanBoardData, CrmDealError> {
        use super::{crm_contact::CrmContact, crm_pipeline::CrmPipeline};

        let pipeline = CrmPipeline::find_by_id(pool, pipeline_id)
            .await
            .map_err(|_| CrmDealError::NotFound)?;

        let stages = CrmPipelineStage::find_by_pipeline(pool, pipeline_id)
            .await
            .map_err(|_| CrmDealError::NotFound)?;

        let mut kanban_stages = Vec::new();

        for stage in stages {
            // Get deals for this stage
            let deals = Self::find_by_stage(pool, &stage.id).await?;

            // Enrich deals with contact info
            let mut deals_with_contacts = Vec::new();
            for deal in deals {
                let contact_info = if let Some(ref contact_id) = deal.crm_contact_id {
                    CrmContact::find_by_id(pool, contact_id).await.ok()
                } else {
                    None
                };

                // Fetch person_id from the contact (bridge to persons table)
                let person_id: Option<DbUuid> = if let Some(ref contact_id) = deal.crm_contact_id {
                    #[derive(sqlx::FromRow)]
                    struct Row {
                        id: DbUuid,
                    }
                    sqlx::query_as::<_, Row>("SELECT id FROM persons WHERE crm_contact_id = ?")
                        .bind(contact_id)
                        .fetch_optional(pool)
                        .await
                        .ok()
                        .flatten()
                        .map(|r| r.id)
                } else {
                    None
                };

                let report_id = if let Some(ref pid) = person_id {
                    Self::report_id_for_person(pool, pid).await
                } else {
                    None
                };

                let (project_name, task_total, task_done, deliverable_count) =
                    if let Some(ref pid) = deal.project_id {
                        Self::fetch_project_stats(pool, pid).await
                    } else {
                        (None, 0, 0, 0)
                    };

                let (
                    intelligence_status,
                    intelligence_summary,
                    intelligence_confidence,
                    research_pass_count,
                    report_status,
                    report_review_status,
                    review_task_id,
                    review_task_status,
                    review_task_assignee,
                ) = Self::fetch_intel_data(pool, &deal.id, person_id.as_ref()).await;

                let (company_intel_status, company_id_val, company_intel_summary) =
                    Self::fetch_company_intel_status(
                        pool,
                        contact_info
                            .as_ref()
                            .and_then(|c| c.company_name.as_deref()),
                    )
                    .await;
                deals_with_contacts.push(CrmDealWithContact {
                    contact_name: contact_info.as_ref().and_then(|c| c.full_name.clone()),
                    contact_email: contact_info.as_ref().and_then(|c| c.email.clone()),
                    contact_company: contact_info.as_ref().and_then(|c| c.company_name.clone()),
                    contact_avatar_url: contact_info.as_ref().and_then(|c| c.avatar_url.clone()),
                    project_name,
                    task_total,
                    task_done,
                    deliverable_count,
                    person_id,
                    report_id,
                    intelligence_status,
                    intelligence_summary,
                    intelligence_confidence,
                    research_pass_count,
                    report_status,
                    report_review_status,
                    review_task_id,
                    review_task_status,
                    review_task_assignee,
                    company_intelligence_status: company_intel_status,
                    company_intelligence_summary: company_intel_summary,
                    company_id: company_id_val,
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
            pipeline_id: pipeline_id.clone(),
            pipeline_name: pipeline.name,
            stages: kanban_stages,
        })
    }

    /// Get aggregated Kanban board data across all org projects for a given pipeline type.
    /// Merges stages from all matching pipelines and aggregates deals into a unified board.
    pub async fn get_kanban_by_organization(
        pool: &SqlitePool,
        organization_id: &DbUuid,
        pipeline_id: &DbUuid,
    ) -> Result<KanbanBoardData, CrmDealError> {
        use super::{
            crm_contact::CrmContact,
            crm_pipeline::{CrmPipeline, PipelineType},
        };

        // Get the reference pipeline (determines stages/layout)
        let ref_pipeline = CrmPipeline::find_by_id(pool, pipeline_id)
            .await
            .map_err(|_| CrmDealError::NotFound)?;

        let ref_type: PipelineType = ref_pipeline
            .pipeline_type
            .parse()
            .unwrap_or(super::crm_pipeline::PipelineType::Custom);

        // Use the reference pipeline's stages as the canonical stage list
        let stages = CrmPipelineStage::find_by_pipeline(pool, pipeline_id)
            .await
            .map_err(|_| CrmDealError::NotFound)?;

        // Collect deals: if this is an org-level pipeline (no project_id), use it directly.
        // If it's a project pipeline, aggregate across all same-type pipelines in the org.
        let mut all_deals: Vec<CrmDeal> = Vec::new();
        if ref_pipeline.project_id.is_none() {
            // Org-level pipeline — only its own deals
            let deals = Self::find_by_pipeline(pool, pipeline_id).await?;
            all_deals.extend(deals);
        } else {
            // Project pipeline — aggregate across org
            let org_pipelines =
                CrmPipeline::find_by_organization(pool, organization_id, Some(ref_type))
                    .await
                    .map_err(|_| CrmDealError::NotFound)?;
            for p in &org_pipelines {
                let deals = Self::find_by_pipeline(pool, &p.id).await?;
                all_deals.extend(deals);
            }
        }

        let mut kanban_stages = Vec::new();
        for stage in &stages {
            // Find deals that belong to this stage by stage_id directly,
            // OR by matching stage name (for deals from other project pipelines)
            let mut stage_deals: Vec<CrmDeal> = Vec::new();
            for deal in &all_deals {
                if deal.crm_stage_id.as_ref() == Some(&stage.id) {
                    stage_deals.push(deal.clone());
                } else if let Some(ref deal_stage_id) = deal.crm_stage_id {
                    // Check if this deal's stage has the same name as our reference stage
                    if let Ok(deal_stage) = CrmPipelineStage::find_by_id(pool, deal_stage_id).await
                    {
                        if deal_stage.name == stage.name {
                            stage_deals.push(deal.clone());
                        }
                    }
                }
            }

            stage_deals.sort_by_key(|d| d.position.unwrap_or(0));

            // Enrich with contact info
            let mut deals_with_contacts = Vec::new();
            for deal in stage_deals {
                let contact_info = if let Some(ref contact_id) = deal.crm_contact_id {
                    CrmContact::find_by_id(pool, contact_id).await.ok()
                } else {
                    None
                };

                let person_id: Option<DbUuid> = if let Some(ref contact_id) = deal.crm_contact_id {
                    #[derive(sqlx::FromRow)]
                    struct Row {
                        id: DbUuid,
                    }
                    sqlx::query_as::<_, Row>("SELECT id FROM persons WHERE crm_contact_id = ?")
                        .bind(contact_id)
                        .fetch_optional(pool)
                        .await
                        .ok()
                        .flatten()
                        .map(|r| r.id)
                } else {
                    None
                };

                let report_id = if let Some(ref pid) = person_id {
                    Self::report_id_for_person(pool, pid).await
                } else {
                    None
                };

                let (project_name, task_total, task_done, deliverable_count) =
                    if let Some(ref pid) = deal.project_id {
                        Self::fetch_project_stats(pool, pid).await
                    } else {
                        (None, 0, 0, 0)
                    };

                let (
                    intelligence_status,
                    intelligence_summary,
                    intelligence_confidence,
                    research_pass_count,
                    report_status,
                    report_review_status,
                    review_task_id,
                    review_task_status,
                    review_task_assignee,
                ) = Self::fetch_intel_data(pool, &deal.id, person_id.as_ref()).await;

                let (company_intel_status, company_id_val, company_intel_summary) =
                    Self::fetch_company_intel_status(
                        pool,
                        contact_info
                            .as_ref()
                            .and_then(|c| c.company_name.as_deref()),
                    )
                    .await;
                deals_with_contacts.push(CrmDealWithContact {
                    contact_name: contact_info.as_ref().and_then(|c| c.full_name.clone()),
                    contact_email: contact_info.as_ref().and_then(|c| c.email.clone()),
                    contact_company: contact_info.as_ref().and_then(|c| c.company_name.clone()),
                    contact_avatar_url: contact_info.as_ref().and_then(|c| c.avatar_url.clone()),
                    project_name,
                    task_total,
                    task_done,
                    deliverable_count,
                    person_id,
                    report_id,
                    intelligence_status,
                    intelligence_summary,
                    intelligence_confidence,
                    research_pass_count,
                    report_status,
                    report_review_status,
                    review_task_id,
                    review_task_status,
                    review_task_assignee,
                    company_intelligence_status: company_intel_status,
                    company_intelligence_summary: company_intel_summary,
                    company_id: company_id_val,
                    deal,
                });
            }

            let total_amount: f64 = deals_with_contacts
                .iter()
                .filter_map(|d| d.deal.amount)
                .sum();

            kanban_stages.push(KanbanStageWithDeals {
                stage: stage.clone(),
                deals: deals_with_contacts,
                total_amount,
            });
        }

        Ok(KanbanBoardData {
            pipeline_id: pipeline_id.clone(),
            pipeline_name: ref_pipeline.name,
            stages: kanban_stages,
        })
    }
}
