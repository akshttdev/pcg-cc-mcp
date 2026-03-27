//! CRM workflow automations — runs hourly in the background.
//!
//! Five triggers:
//!   1. waiting_on_client project → create follow-up task in 3 days
//!   2. 5+ follow-up attempts on a lead → mark lifecycle_stage = 'churned'
//!   3. Lead enters SQL stage → create "Draft proposal" task for owner
//!   4. Proposal contract_signed → create project (if no project linked)
//!   5. Project marked complete → draft AR invoice

use sqlx::SqlitePool;
use tokio::time::{Duration, interval};
use tracing::{error, info};
use uuid::Uuid;

pub fn spawn_automation_loop(pool: SqlitePool) {
    tokio::spawn(async move {
        // Wait 1 hour before first run so server is fully up
        let mut ticker = interval(Duration::from_secs(3600));
        ticker.tick().await; // discard the immediate first tick
        loop {
            ticker.tick().await;
            if let Err(e) = run_all(&pool).await {
                error!("Automation loop error: {e}");
            }
        }
    });
}

async fn run_all(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    info!("Running CRM automations");
    automation_waiting_on_client_followup(pool).await?;
    automation_churn_overdue_leads(pool).await?;
    automation_sql_draft_proposal_task(pool).await?;
    automation_signed_proposal_create_project(pool).await?;
    automation_complete_project_draft_invoice(pool).await?;
    automation_research_orphaned_companies(pool).await;
    Ok(())
}

// ── 1. Waiting-on-client → schedule follow-up task ───────────────────────────

async fn automation_waiting_on_client_followup(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Proj {
        id: Uuid,
        name: String,
        owner_id: Option<Uuid>,
    }

    let projects: Vec<Proj> = sqlx::query_as(
        "SELECT id, name, owner_id FROM projects WHERE waiting_on_client = 1 AND deleted_at IS NULL",
    )
    .fetch_all(pool)
    .await?;

    for p in projects {
        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks \
             WHERE project_id = ? AND title LIKE '%follow up%' \
             AND status NOT IN ('done','cancelled') \
             AND created_at >= datetime('now', '-3 days')",
        )
        .bind(p.id)
        .fetch_one(pool)
        .await?;

        if existing == 0 {
            let task_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO tasks (id, project_id, title, description, status, due_date, assignee_id) \
                 VALUES (?, ?, 'Follow up with client', 'Waiting-on-client auto-reminder', 'todo', \
                 datetime('now','+3 days'), ?)",
            )
            .bind(task_id)
            .bind(p.id)
            .bind(p.owner_id)
            .execute(pool)
            .await?;
            info!("Auto: created follow-up task for project '{}'", p.name);
        }
    }
    Ok(())
}

// ── 2. Lead with 5+ follow-up attempts → churn ───────────────────────────────

async fn automation_churn_overdue_leads(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        "UPDATE crm_contacts SET lifecycle_stage = 'churned', updated_at = datetime('now','subsec') \
         WHERE follow_up_attempts >= 5 AND lifecycle_stage NOT IN ('churned','customer')",
    )
    .execute(pool)
    .await?;

    if result.rows_affected() > 0 {
        info!(
            "Auto: churned {} over-followed leads",
            result.rows_affected()
        );
    }
    Ok(())
}

// ── 3. Lead in SQL stage → create "Draft proposal" task ──────────────────────

async fn automation_sql_draft_proposal_task(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Lead {
        id: Uuid,
        full_name: Option<String>,
    }

    let leads: Vec<Lead> =
        sqlx::query_as("SELECT id, full_name FROM crm_contacts WHERE lifecycle_stage = 'sql'")
            .fetch_all(pool)
            .await?;

    let admin_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM users WHERE is_admin = 1 LIMIT 1")
            .fetch_optional(pool)
            .await?;

    for lead in leads {
        // Check for existing draft-proposal task mentioning this lead
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tasks \
             WHERE title = 'Draft proposal' AND description LIKE ? \
             AND created_at >= datetime('now', '-7 days')",
        )
        .bind(format!("%{}%", lead.id))
        .fetch_one(pool)
        .await?;

        if exists == 0 {
            let task_id = Uuid::new_v4();
            let name = lead.full_name.as_deref().unwrap_or("unknown lead");
            let desc = format!(
                "Lead {} ({}) has reached SQL stage. Draft a proposal.",
                name, lead.id
            );
            sqlx::query(
                "INSERT INTO tasks (id, title, description, status, assignee_id) \
                 VALUES (?, 'Draft proposal', ?, 'todo', ?)",
            )
            .bind(task_id)
            .bind(&desc)
            .bind(admin_id)
            .execute(pool)
            .await?;
            info!("Auto: created draft-proposal task for SQL lead '{}'", name);
        }
    }
    Ok(())
}

// ── 4. Proposal contract_signed → create project ─────────────────────────────

async fn automation_signed_proposal_create_project(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct SignedProposal {
        id: Uuid,
        title: String,
        owner_id: Option<Uuid>,
    }

    let proposals: Vec<SignedProposal> = sqlx::query_as(
        "SELECT id, title, owner_id FROM proposals \
         WHERE status = 'contract_signed' AND project_id IS NULL AND signed_at IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    for prop in proposals {
        let project_id = Uuid::new_v4();
        let project_name = format!("Project: {}", prop.title);

        sqlx::query(
            "INSERT INTO projects (id, name, owner_id, created_at, updated_at) \
             VALUES (?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))",
        )
        .bind(project_id)
        .bind(&project_name)
        .bind(prop.owner_id)
        .execute(pool)
        .await?;

        sqlx::query(
            "UPDATE proposals SET project_id = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(project_id)
        .bind(prop.id)
        .execute(pool)
        .await?;

        info!(
            "Auto: created project '{}' from signed proposal",
            project_name
        );
    }
    Ok(())
}

// ── 5. Project complete → draft AR invoice ────────────────────────────────────

async fn automation_complete_project_draft_invoice(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct CompleteProj {
        id: Uuid,
        name: String,
        client_budget_vibe: Option<i64>,
    }

    let projects: Vec<CompleteProj> = sqlx::query_as(
        "SELECT p.id, p.name, p.client_budget_vibe FROM projects p \
         WHERE p.project_status = 'complete' AND p.deleted_at IS NULL \
         AND NOT EXISTS (SELECT 1 FROM invoices i WHERE i.project_id = p.id AND i.invoice_type = 'ar')",
    )
    .fetch_all(pool)
    .await?;

    for proj in projects {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM invoices WHERE invoice_type = 'ar'")
                .fetch_one(pool)
                .await?;

        let invoice_number = format!("AR-{:05}", count + 1);
        let invoice_id = Uuid::new_v4();
        let amount_vibe = proj.client_budget_vibe.unwrap_or(0);

        sqlx::query(
            "INSERT INTO invoices (id, invoice_number, invoice_type, status, amount_vibe, project_id) \
             VALUES (?, ?, 'ar', 'draft', ?, ?)",
        )
        .bind(invoice_id)
        .bind(&invoice_number)
        .bind(amount_vibe)
        .bind(proj.id)
        .execute(pool)
        .await?;

        info!(
            "Auto: drafted invoice {} for completed project '{}'",
            invoice_number, proj.name
        );
    }
    Ok(())
}

// ── REST API ──────────────────────────────────────────────────────────────────

use axum::{Json, Router, routing::get};
use serde::Serialize;
use utils::response::ApiResponse;

use crate::DeploymentImpl;

#[derive(Serialize)]
pub struct AutomationDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub trigger: &'static str,
    pub action: &'static str,
    pub schedule: &'static str,
}

fn automation_definitions() -> Vec<AutomationDefinition> {
    vec![
        AutomationDefinition {
            id: "waiting_on_client_followup",
            name: "Waiting-on-Client Follow-up",
            description: "Creates a follow-up task when a project is waiting on the client for 3+ days.",
            trigger: "Project status = waiting_on_client",
            action: "Create follow-up task assigned to account manager",
            schedule: "Hourly",
        },
        AutomationDefinition {
            id: "churn_overdue_leads",
            name: "Churn Overdue Leads",
            description: "Marks leads as churned after 5+ unanswered follow-up attempts.",
            trigger: "Lead with 5+ follow-up tasks all completed",
            action: "Set lifecycle_stage = churned",
            schedule: "Hourly",
        },
        AutomationDefinition {
            id: "sql_draft_proposal_task",
            name: "SQL → Draft Proposal Task",
            description: "Creates a 'Draft proposal' task when a lead enters the SQL pipeline stage.",
            trigger: "Lead enters SQL stage",
            action: "Create Draft Proposal task for lead owner",
            schedule: "Hourly",
        },
        AutomationDefinition {
            id: "signed_proposal_create_project",
            name: "Signed Proposal → Create Project",
            description: "Automatically provisions a new project when a proposal is marked contract_signed.",
            trigger: "Proposal status = contract_signed",
            action: "Create project and add contact as client member",
            schedule: "Hourly",
        },
        AutomationDefinition {
            id: "complete_project_draft_invoice",
            name: "Complete Project → Draft Invoice",
            description: "Drafts an AR invoice when a project is marked complete.",
            trigger: "Project status = complete",
            action: "Create AR invoice for project budget",
            schedule: "Hourly",
        },
    ]
}

async fn list_automations() -> Json<ApiResponse<Vec<AutomationDefinition>>> {
    Json(ApiResponse::success(automation_definitions()))
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new().route("/automations", get(list_automations))
}

// ── 6. Research orphaned companies (idle, older than 1 hour) ─────────────────

async fn automation_research_orphaned_companies(pool: &SqlitePool) {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: Vec<u8>,
        name: String,
    }

    let companies: Vec<Row> = match sqlx::query_as(
        "SELECT id, name FROM companies
         WHERE intelligence_status = 'idle'
           AND created_at < datetime('now', '-1 hour')
         ORDER BY created_at ASC
         LIMIT 5",
    )
    .fetch_all(pool)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Auto-research: failed to query orphaned companies: {e}");
            return;
        }
    };

    if companies.is_empty() {
        return;
    }

    info!("Auto-research: queuing {} orphaned companies for Scout Phase I", companies.len());

    for c in companies {
        let Ok(company_id) = Uuid::from_slice(&c.id) else { continue };

        // Mark as queued
        let _ = sqlx::query(
            "UPDATE companies SET intelligence_status = 'queued', updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(&c.id)
        .execute(pool)
        .await;

        let pool2 = pool.clone();
        let name = c.name.clone();
        tokio::spawn(async move {
            crate::routes::intelligence::run_company_research_direct(&pool2, company_id, &name, None).await;
        });
    }
}
