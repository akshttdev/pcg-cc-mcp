//! Command Center — aggregated view for Aaren (account manager / EP).
//!
//! Single GET endpoint that returns a snapshot of:
//!   - Overdue tasks
//!   - Deliverables due this week
//!   - Projects waiting on client response
//!   - Leads that need follow-up (follow_up_attempts < 5 && waiting_on_client)
//!   - Proposals awaiting approval (status = 'pending_approval')
//!   - Closed projects with no invoice (complete + no AR invoice)

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{Duration, Utc};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{error::ApiError, DeploymentImpl};

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct CommandCenterSnapshot {
    pub overdue_tasks: Vec<OverdueTask>,
    pub deliverables_due_this_week: Vec<DueSoonDeliverable>,
    pub waiting_on_client_projects: Vec<WaitingProject>,
    pub follow_up_required: Vec<FollowUpLead>,
    pub proposals_awaiting_approval: Vec<PendingProposal>,
    pub closed_unpaid_projects: Vec<ClosedUnpaidProject>,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export)]
pub struct OverdueTask {
    pub id: Uuid,
    pub title: String,
    pub project_name: String,
    pub due_date: String,
    pub assignee_name: Option<String>,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export)]
pub struct DueSoonDeliverable {
    pub id: Uuid,
    pub title: String,
    pub deliverable_type: String,
    pub project_name: String,
    pub status: String,
    pub due_date: String,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export)]
pub struct WaitingProject {
    pub id: Uuid,
    pub name: String,
    pub client_name: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export)]
pub struct FollowUpLead {
    pub id: Uuid,
    pub full_name: String,
    pub email: Option<String>,
    pub follow_up_attempts: i64,
    pub lifecycle_stage: String,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export)]
pub struct PendingProposal {
    pub id: Uuid,
    pub title: String,
    pub lead_name: Option<String>,
    pub quote_amount_vibe: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize, TS, sqlx::FromRow)]
#[ts(export)]
pub struct ClosedUnpaidProject {
    pub id: Uuid,
    pub name: String,
    pub client_name: Option<String>,
    pub updated_at: String,
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CommandCenterParams {
    pub org_id: Option<Uuid>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

async fn get_command_center(
    State(d): State<DeploymentImpl>,
    Query(_p): Query<CommandCenterParams>,
) -> Result<Json<ApiResponse<CommandCenterSnapshot>>, ApiError> {
    let pool = &d.db().pool;

    let now = Utc::now();
    let week_end = (now + Duration::days(7)).format("%Y-%m-%d").to_string();
    let now_str = now.format("%Y-%m-%dT%H:%M:%S").to_string();

    // ── Overdue tasks ─────────────────────────────────────────────────────────
    let overdue_tasks: Vec<OverdueTask> = sqlx::query_as(
        "SELECT t.id, t.title, p.name AS project_name, \
                COALESCE(t.due_date,'') AS due_date, u.full_name AS assignee_name \
         FROM tasks t \
         JOIN projects p ON p.id = t.project_id \
         LEFT JOIN users u ON u.id = t.assignee_id \
         WHERE t.due_date IS NOT NULL AND t.due_date < ? \
           AND t.status NOT IN ('done','cancelled') \
           AND t.deleted_at IS NULL \
         ORDER BY t.due_date ASC LIMIT 50",
    )
    .bind(&now_str)
    .fetch_all(pool)
    .await?;

    // ── Deliverables due this week ────────────────────────────────────────────
    let deliverables_due_this_week: Vec<DueSoonDeliverable> = sqlx::query_as(
        "SELECT d.id, d.title, d.deliverable_type, p.name AS project_name, \
                d.status, COALESCE(d.due_date,'') AS due_date \
         FROM deliverables d \
         JOIN projects p ON p.id = d.project_id \
         WHERE d.due_date IS NOT NULL AND d.due_date <= ? AND d.status != 'done' \
         ORDER BY d.due_date ASC LIMIT 50",
    )
    .bind(&week_end)
    .fetch_all(pool)
    .await?;

    // ── Waiting on client projects ────────────────────────────────────────────
    let waiting_on_client_projects: Vec<WaitingProject> = sqlx::query_as(
        "SELECT p.id, p.name, cl.name AS client_name, p.updated_at \
         FROM projects p \
         LEFT JOIN clients cl ON cl.id = p.client_id \
         WHERE p.waiting_on_client = 1 AND p.deleted_at IS NULL \
         ORDER BY p.updated_at ASC LIMIT 30",
    )
    .fetch_all(pool)
    .await?;

    // ── Leads needing follow-up ───────────────────────────────────────────────
    let follow_up_required: Vec<FollowUpLead> = sqlx::query_as(
        "SELECT id, full_name, email, follow_up_attempts, lifecycle_stage \
         FROM crm_contacts \
         WHERE waiting_on_client = 1 AND follow_up_attempts < 5 \
           AND lifecycle_stage NOT IN ('churned','customer') \
         ORDER BY follow_up_attempts ASC LIMIT 30",
    )
    .fetch_all(pool)
    .await?;

    // ── Proposals awaiting approval ───────────────────────────────────────────
    let proposals_awaiting_approval: Vec<PendingProposal> = sqlx::query_as(
        "SELECT pr.id, pr.title, pe.full_name AS lead_name, \
                pr.quote_amount_vibe, pr.created_at \
         FROM proposals pr \
         LEFT JOIN crm_contacts pe ON pe.id = pr.lead_id \
         WHERE pr.status = 'pending_approval' \
         ORDER BY pr.created_at ASC LIMIT 30",
    )
    .fetch_all(pool)
    .await?;

    // ── Complete projects with no AR invoice ──────────────────────────────────
    let closed_unpaid_projects: Vec<ClosedUnpaidProject> = sqlx::query_as(
        "SELECT p.id, p.name, cl.name AS client_name, p.updated_at \
         FROM projects p \
         LEFT JOIN clients cl ON cl.id = p.client_id \
         WHERE p.project_status = 'complete' AND p.deleted_at IS NULL \
           AND NOT EXISTS (SELECT 1 FROM invoices i WHERE i.project_id = p.id AND i.invoice_type = 'ar') \
         ORDER BY p.updated_at DESC LIMIT 20",
    )
    .fetch_all(pool)
    .await?;

    Ok(Json(ApiResponse::success(CommandCenterSnapshot {
        overdue_tasks,
        deliverables_due_this_week,
        waiting_on_client_projects,
        follow_up_required,
        proposals_awaiting_approval,
        closed_unpaid_projects,
    })))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn router(deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/command-center", get(get_command_center))
        .with_state(deployment.clone())
}
