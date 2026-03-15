//! HTTP handlers for intake CRUD and business report endpoints.

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use db::models::{
    business_report::{BusinessReport, PatchBusinessReport},
    call_intake_item::CallIntakeItem,
    crm_deal::CrmDeal,
    proposal::{CreateProposal, Proposal},
};
use tracing::{info, warn};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError, middleware::access_control::AccessContext};
use db::db_uuid::DbUuid;
use super::{
    EmailIntakePayload, GenerateReportRequest, RevisionRequest, UploadIntakePayload,
    pipeline::run_intake_pipeline,
    report::run_report_generation,
};

// ── Intake ingestion ─────────────────────────────────────────────────────────

/// POST /api/intake/email
/// Webhook endpoint for Zoho/SendGrid inbound email parse.
pub async fn email_webhook(
    State(d): State<DeploymentImpl>,
    Json(body): Json<EmailIntakePayload>,
) -> Result<Json<ApiResponse<CallIntakeItem>>, ApiError> {
    use db::models::call_intake_item::CreateCallIntakeItem;
    use deployment::Deployment;

    let pool = &d.db().pool;

    // Parse "Name <email>" format if from_email not provided separately
    let (from_name, from_email) = super::parse_from_field(
        body.from.as_deref(),
        body.from_name.as_deref(),
        body.from_email.as_deref(),
    );

    let raw_content = body.text
        .or(body.html)
        .unwrap_or_default();

    if raw_content.trim().is_empty() {
        return Err(ApiError::BadRequest("Email body is empty".into()));
    }

    let (org_id, assigned) = super::resolve_org_and_assignee(
        pool,
        from_email.as_deref(),
        body.organization_id,
        body.assigned_to,
    ).await;

    let item = CallIntakeItem::create(
        pool,
        CreateCallIntakeItem {
            source_type: "email".into(),
            source_ref: body.message_id.clone(),
            raw_content: Some(raw_content.clone()),
            subject: body.subject.clone(),
            from_email: from_email.clone(),
            from_name: from_name.clone(),
            call_date: body.date.clone(),
            duration_seconds: None,
            metadata: Some(serde_json::json!({
                "message_id": body.message_id,
                "organization_id": org_id.map(|id| id.to_string()),
                "assigned_to": assigned.map(|id| id.to_string()),
            }).to_string()),
        },
    ).await?;

    let item_id = item.id;
    let pool_clone = pool.clone();

    // Process asynchronously
    tokio::spawn(async move {
        if let Err(e) = run_intake_pipeline(pool_clone, item_id, org_id, assigned).await {
            tracing::error!("Intake pipeline failed for {}: {}", item_id, e);
        }
    });

    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/intake/upload
/// Manual transcript upload (paste or file content).
pub async fn upload_transcript(
    State(d): State<DeploymentImpl>,
    Json(body): Json<UploadIntakePayload>,
) -> Result<Json<ApiResponse<CallIntakeItem>>, ApiError> {
    use db::models::call_intake_item::CreateCallIntakeItem;
    use deployment::Deployment;

    let pool = &d.db().pool;

    if body.content.trim().is_empty() {
        return Err(ApiError::BadRequest("Content is empty".into()));
    }

    let source_ref = body.call_log_id.map(|id| id.to_string());

    let (org_id, assigned) = super::resolve_org_and_assignee(
        pool,
        body.from_email.as_deref(),
        body.organization_id,
        body.assigned_to,
    ).await;

    let item = CallIntakeItem::create(
        pool,
        CreateCallIntakeItem {
            source_type: if body.call_log_id.is_some() { "call_log".into() } else { "upload".into() },
            source_ref,
            raw_content: Some(body.content),
            subject: body.subject,
            from_email: body.from_email,
            from_name: body.from_name,
            call_date: body.call_date,
            duration_seconds: body.duration_seconds,
            metadata: Some(serde_json::json!({
                "organization_id": org_id.map(|id| id.to_string()),
                "assigned_to": assigned.map(|id| id.to_string()),
            }).to_string()),
        },
    ).await?;

    let item_id = item.id;
    let pool_clone = pool.clone();
    let org_id = body.organization_id;
    let assigned = body.assigned_to;

    tokio::spawn(async move {
        if let Err(e) = run_intake_pipeline(pool_clone, item_id, org_id, assigned).await {
            tracing::error!("Intake pipeline failed for {}: {}", item_id, e);
        }
    });

    Ok(Json(ApiResponse::success(item)))
}

/// GET /api/intake
pub async fn list_intake(
    State(d): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<CallIntakeItem>>>, ApiError> {
    use deployment::Deployment;
    let items = CallIntakeItem::list(&d.db().pool).await?;
    Ok(Json(ApiResponse::success(items)))
}

/// GET /api/intake/:id
pub async fn get_intake_item(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<CallIntakeItem>>, ApiError> {
    use deployment::Deployment;
    let item = CallIntakeItem::find_by_id(&d.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Intake item not found".into()))?;
    Ok(Json(ApiResponse::success(item)))
}

/// POST /api/intake/:id/process — manually re-trigger pipeline
pub async fn process_intake_item_handler(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    use deployment::Deployment;
    let pool = &d.db().pool;

    // Reset to pending
    sqlx::query(
        "UPDATE call_intake_items SET status = 'pending', error = NULL, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await?;

    // Read org/assigned from item metadata if available
    #[derive(sqlx::FromRow)]
    struct MetaRow { metadata: Option<String> }
    let meta = sqlx::query_as::<_, MetaRow>("SELECT metadata FROM call_intake_items WHERE id = ?")
        .bind(id).fetch_optional(pool).await.ok().flatten();
    let (org_id, assigned) = meta.and_then(|m| {
        let v: serde_json::Value = serde_json::from_str(m.metadata.as_deref().unwrap_or("{}")).ok()?;
        let org = v["organization_id"].as_str().and_then(|s| s.parse::<Uuid>().ok());
        let asgn = v["assigned_to"].as_str().and_then(|s| s.parse::<Uuid>().ok());
        Some((org, asgn))
    }).unwrap_or((None, None));

    let pool_clone = pool.clone();
    tokio::spawn(async move {
        if let Err(e) = run_intake_pipeline(pool_clone, id, org_id, assigned).await {
            tracing::error!("Re-process pipeline failed for {}: {}", id, e);
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({ "status": "queued", "id": id }))))
}

/// GET /api/intake/:id/status
pub async fn get_intake_status(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    use deployment::Deployment;
    let item = CallIntakeItem::find_by_id(&d.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Intake item not found".into()))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": item.id,
        "status": item.status,
        "person_id": item.person_id,
        "report_id": item.report_id,
        "error": item.error,
        "processed_at": item.processed_at,
    }))))
}

// ── Business reports ─────────────────────────────────────────────────────────

/// GET /api/business-reports
pub async fn list_reports(
    State(d): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<BusinessReport>>>, ApiError> {
    use deployment::Deployment;
    let reports = BusinessReport::list(&d.db().pool).await?;
    Ok(Json(ApiResponse::success(reports)))
}

/// GET /api/business-reports/:id
pub async fn get_report(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<BusinessReport>>, ApiError> {
    use deployment::Deployment;
    let report = BusinessReport::find_by_id(&d.db().pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;
    Ok(Json(ApiResponse::success(report)))
}

/// PATCH /api/business-reports/:id — inline section editing
pub async fn patch_report(
    State(d): State<DeploymentImpl>,
    Path(id): Path<Uuid>,
    Json(body): Json<PatchBusinessReport>,
) -> Result<Json<ApiResponse<BusinessReport>>, ApiError> {
    use deployment::Deployment;
    let report = BusinessReport::patch(&d.db().pool, id, body)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;
    Ok(Json(ApiResponse::success(report)))
}

/// POST /api/business-reports/:id/approve — authenticated
/// Marks report approved, advances CRM deal to Proposal stage, auto-creates a Proposal record.
pub async fn approve_business_report(
    State(d): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    use deployment::Deployment;
    let pool = &d.db().pool;
    let user_id = access_context.user_id;

    let report = BusinessReport::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;

    // Mark approved
    sqlx::query(
        "UPDATE business_reports SET review_status = 'approved', reviewed_by = ?,
         reviewed_at = datetime('now','subsec'), updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(user_id)
    .bind(id)
    .execute(pool)
    .await?;

    // Advance CRM deal to "Proposal" stage if linked
    let mut deal_json: Option<serde_json::Value> = None;
    if let Some(deal_id) = report.crm_deal_id {
        let deal_db_id = DbUuid::from(deal_id);
        if let Ok(deal) = CrmDeal::find_by_id(pool, &deal_db_id).await {
            if let Some(ref pipeline_id) = deal.crm_pipeline_id {
                // Find the "Proposal" stage in this pipeline
                let proposal_stage: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND name = 'Proposal' LIMIT 1",
                )
                .bind(pipeline_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some((stage_id_str,)) = proposal_stage {
                    let stage_id = DbUuid::from_string(stage_id_str);
                    let _ = CrmDeal::move_to_stage(pool, &deal_db_id, &stage_id, 0).await;
                }
            }
            deal_json = Some(serde_json::json!({ "id": deal.id, "name": deal.name }));
        }
    }

    // Auto-create a proposal from the report
    let mut proposal_json: Option<serde_json::Value> = None;
    if let Some(person_id) = report.person_id {
        // Check if a proposal already exists for this person
        let exists: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM proposals WHERE lead_id = ?",
        )
        .bind(person_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

        if !exists {
            // Build description from report sections
            let summary = report.executive_summary.as_deref().unwrap_or("");
            let services = report.recommended_services.as_str();
            let next = report.next_steps.as_str();
            let description = format!(
                "## Executive Summary\n{summary}\n\n## Recommended Services\n{services}\n\n## Next Steps\n{next}"
            );

            // Look up person name for the title
            #[derive(sqlx::FromRow)]
            struct NameRow { full_name: String, company_name: Option<String> }
            let person_info = sqlx::query_as::<_, NameRow>(
                "SELECT full_name, company_name FROM persons WHERE id = ?",
            )
            .bind(person_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

            let title = if let Some(ref p) = person_info {
                if let Some(ref company) = p.company_name {
                    format!("{} — Proposal", company)
                } else {
                    format!("{} — Proposal", p.full_name)
                }
            } else {
                format!("Proposal from Report {}", id)
            };

            // Find org for this person
            #[derive(sqlx::FromRow)]
            struct OrgRow { organization_id: Uuid }
            let org_id = sqlx::query_as::<_, OrgRow>(
                "SELECT organization_id FROM person_organization_contacts WHERE person_id = ? LIMIT 1",
            )
            .bind(person_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
            .map(|r| r.organization_id);

            let new_proposal = Proposal::create(pool, CreateProposal {
                title,
                lead_id: Some(person_id),
                organization_id: org_id,
                owner_id: Some(user_id),
                project_id: None,
                company_id: report.company_id,
                description: Some(description),
                quote_amount_vibe: None,
                deal_type: None,
                contact_ids: Some(vec![person_id.to_string()]),
            }).await;

            match new_proposal {
                Ok(p) => {
                    proposal_json = Some(serde_json::json!({ "id": p.id, "title": p.title, "status": p.status }));
                    info!("Auto-created proposal {} from approved report {}", p.id, id);
                }
                Err(e) => warn!("Failed to create proposal from report {}: {}", id, e),
            }
        }
    }

    let updated = BusinessReport::find_by_id(pool, id).await?.unwrap();
    Ok(Json(ApiResponse::success(serde_json::json!({
        "report": updated,
        "deal": deal_json,
        "proposal": proposal_json,
    }))))
}

/// POST /api/business-reports/:id/request-revision — authenticated
pub async fn request_revision(
    State(d): State<DeploymentImpl>,
    Extension(access_context): Extension<AccessContext>,
    Path(id): Path<Uuid>,
    Json(body): Json<RevisionRequest>,
) -> Result<Json<ApiResponse<BusinessReport>>, ApiError> {
    use deployment::Deployment;
    let pool = &d.db().pool;
    let user_id = access_context.user_id;

    let report = BusinessReport::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found".into()))?;

    // Mark rejected with notes
    sqlx::query(
        "UPDATE business_reports SET review_status = 'rejected', reviewed_by = ?,
         reviewed_at = datetime('now','subsec'), review_notes = ?,
         updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(user_id)
    .bind(&body.notes)
    .bind(id)
    .execute(pool)
    .await?;

    // Move CRM deal back to "Research" stage if linked
    if let Some(deal_id) = report.crm_deal_id {
        let deal_db_id = DbUuid::from(deal_id);
        if let Ok(deal) = CrmDeal::find_by_id(pool, &deal_db_id).await {
            if let Some(ref pipeline_id) = deal.crm_pipeline_id {
                let research_stage: Option<(String,)> = sqlx::query_as(
                    "SELECT id FROM crm_pipeline_stages WHERE pipeline_id = ? AND name = 'Research' LIMIT 1",
                )
                .bind(pipeline_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

                if let Some((stage_id_str,)) = research_stage {
                    let stage_id = DbUuid::from_string(stage_id_str);
                    let _ = CrmDeal::move_to_stage(pool, &deal_db_id, &stage_id, 0).await;
                }
            }
        }
    }

    let updated = BusinessReport::find_by_id(pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Report not found after update".into()))?;

    Ok(Json(ApiResponse::success(updated)))
}

/// POST /api/business-reports/generate
pub async fn generate_report_handler(
    State(d): State<DeploymentImpl>,
    Json(body): Json<GenerateReportRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, ApiError> {
    use deployment::Deployment;
    let pool = &d.db().pool;

    use db::models::person::Person;
    // Verify person exists
    Person::find_by_id(pool, body.person_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Person not found".into()))?;

    let person_id = body.person_id;
    let report_type = body.report_type.unwrap_or_else(|| "business_audit".into());
    let pool_clone = pool.clone();

    tokio::spawn(async move {
        // Load businesses/individuals from most recent intake item for this person
        let (businesses, individuals, intake_id) = {
            #[derive(sqlx::FromRow)]
            struct IntakeRow { id: Uuid, extracted_businesses: String, extracted_individuals: String }
            let row = sqlx::query_as::<_, IntakeRow>(
                "SELECT id, extracted_businesses, extracted_individuals FROM call_intake_items
                 WHERE person_id = ? ORDER BY created_at DESC LIMIT 1",
            )
            .bind(person_id)
            .fetch_optional(&pool_clone)
            .await
            .ok()
            .flatten();

            if let Some(r) = row {
                let biz: Vec<super::ExtractedBusiness> = serde_json::from_str(&r.extracted_businesses).unwrap_or_default();
                let ind: Vec<super::ExtractedIndividual> = serde_json::from_str(&r.extracted_individuals).unwrap_or_default();
                (biz, ind, r.id)
            } else {
                (vec![], vec![], Uuid::nil())
            }
        };

        if let Err(e) = run_report_generation(
            pool_clone, person_id, report_type, businesses, individuals, intake_id, None,
        ).await {
            tracing::error!("Report generation failed for person {}: {}", person_id, e);
        }
    });

    Ok(Json(ApiResponse::success(serde_json::json!({
        "status": "queued",
        "person_id": person_id,
        "message": "Report generation started — poll GET /api/persons/:id/reports"
    }))))
}
