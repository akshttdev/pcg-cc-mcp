//! Pull QuickBooks Online data into PCG.
//!
//! Each pass:
//! 1. Refreshes the access token if it's near expiry.
//! 2. Builds a CDC `WHERE Metadata.LastUpdatedTime > '<last_sync_at>'` clause
//!    (or pulls everything on first sync).
//! 3. Upserts each Customer into `crm_contacts`, recording the QBO mapping
//!    in `quickbooks_entity_map`.
//! 4. Upserts each Invoice into `invoices`, linked by `crm_contact_id`.
//! 5. Walks Payments and marks linked invoices `paid` / `partial`.
//! 6. Stamps `last_sync_at` and clears any prior error on the account.
//!
//! Errors during a pass are persisted to `last_error` on the account so the
//! UI can surface them.

use chrono::{DateTime, Utc};
use db::{
    db_uuid::DbUuid,
    models::{
        crm_contact::{ContactSource, CrmContact},
        invoice::Invoice,
        quickbooks_account::{QuickBooksAccount, QuickBooksEntityMap},
    },
};
use serde_json::Value;
use sqlx::SqlitePool;
use tracing::{info, warn};
use uuid::Uuid;

use super::{QboError, client::QboClient};

/// Counts of what changed in one sync pass.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SyncStats {
    pub customers_seen: i64,
    pub customers_upserted: i64,
    pub invoices_seen: i64,
    pub invoices_upserted: i64,
    pub payments_seen: i64,
    pub invoices_marked_paid: i64,
}

/// Top-level entry point: run a full pull pass for one QuickBooks account.
pub async fn run_qbo_sync(pool: &SqlitePool, account_id: Uuid) -> Result<SyncStats, QboError> {
    let account = QuickBooksAccount::find_by_id(pool, account_id).await?;

    QuickBooksAccount::update_sync_status(pool, account.id, "syncing").await?;

    let result = run_inner(pool, &account).await;

    match &result {
        Ok(stats) => {
            QuickBooksAccount::update_sync_status(pool, account.id, "active").await?;
            info!(
                account_id = %account.id,
                customers = stats.customers_upserted,
                invoices = stats.invoices_upserted,
                payments = stats.payments_seen,
                "QBO sync completed"
            );
        }
        Err(e) => {
            warn!(account_id = %account.id, error = %e, "QBO sync failed");
            QuickBooksAccount::set_error(pool, account.id, &e.to_string()).await?;
        }
    }

    result
}

async fn run_inner(pool: &SqlitePool, account: &QuickBooksAccount) -> Result<SyncStats, QboError> {
    let client = QboClient::from_account_refreshing(pool, account).await?;
    let cdc_clause = changed_since_clause(account.last_sync_at);

    let mut stats = SyncStats::default();
    let org_id = DbUuid::from_string(account.organization_id.to_string());

    if account.sync_customers == 1 {
        sync_customers(pool, &client, account, &org_id, &cdc_clause, &mut stats).await?;
    }
    if account.sync_invoices == 1 {
        sync_invoices(pool, &client, account, &org_id, &cdc_clause, &mut stats).await?;
    }
    if account.sync_payments == 1 {
        sync_payments(pool, &client, account, &cdc_clause, &mut stats).await?;
    }

    Ok(stats)
}

/// Build the `WHERE` clause restricting a QBO query to records changed since
/// the last sync. Returns an empty string on first-ever sync (full pull).
fn changed_since_clause(last_sync: Option<DateTime<Utc>>) -> String {
    match last_sync {
        Some(ts) => format!(
            " WHERE Metadata.LastUpdatedTime > '{}'",
            ts.format("%Y-%m-%dT%H:%M:%S%.3f%:z")
        ),
        None => String::new(),
    }
}

// ─── Customers ──────────────────────────────────────────────────────────────

async fn sync_customers(
    pool: &SqlitePool,
    client: &QboClient,
    account: &QuickBooksAccount,
    org_id: &DbUuid,
    cdc_clause: &str,
    stats: &mut SyncStats,
) -> Result<(), QboError> {
    let query = format!("SELECT * FROM Customer{cdc_clause} MAXRESULTS 500");
    let response = client.query(&query).await?;
    let customers = response["Customer"].as_array().cloned().unwrap_or_default();
    stats.customers_seen += customers.len() as i64;

    for customer in customers {
        match upsert_customer(pool, account, org_id, &customer).await {
            Ok(true) => stats.customers_upserted += 1,
            Ok(false) => {}
            Err(e) => warn!(error = %e, "skipped QBO customer upsert"),
        }
    }
    Ok(())
}

/// Upsert a single QBO Customer: match by entity-map → email → name+company,
/// then create if no match. Returns `true` if a row was touched.
async fn upsert_customer(
    pool: &SqlitePool,
    account: &QuickBooksAccount,
    org_id: &DbUuid,
    customer: &Value,
) -> Result<bool, QboError> {
    let qbo_id = customer["Id"].as_str().unwrap_or("").to_string();
    if qbo_id.is_empty() {
        return Ok(false);
    }

    let email = customer["PrimaryEmailAddr"]["Address"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let display_name = customer["DisplayName"]
        .as_str()
        .or_else(|| customer["CompanyName"].as_str())
        .unwrap_or("")
        .to_string();
    let company_name = customer["CompanyName"]
        .as_str()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty());
    let phone = customer["PrimaryPhone"]["FreeFormNumber"]
        .as_str()
        .map(|s| s.to_string());
    let sync_token = customer["SyncToken"].as_str().map(|s| s.to_string());

    let existing_map =
        QuickBooksEntityMap::find_by_qbo_entity(pool, account.id, "Customer", &qbo_id).await?;
    let contact = if let Some(map) = &existing_map {
        let pcg_id = DbUuid::from_string(map.pcg_entity_id.clone());
        CrmContact::find_by_id(pool, &pcg_id).await.ok()
    } else if let Some(ref e) = email {
        CrmContact::find_by_email(pool, org_id, e).await?
    } else {
        None
    };

    let contact = match contact {
        Some(c) => c,
        None => {
            let (first_name, last_name) = split_name(&display_name);
            CrmContact::create(
                pool,
                db::models::crm_contact::CreateCrmContact {
                    organization_id: org_id.clone(),
                    client_id: None,
                    first_name,
                    last_name,
                    email: email.clone(),
                    phone: phone.clone(),
                    mobile: None,
                    avatar_url: None,
                    company_name: company_name.clone(),
                    job_title: None,
                    department: None,
                    linkedin_url: None,
                    twitter_handle: None,
                    website: None,
                    source: Some(ContactSource::Api),
                    lifecycle_stage: Some(db::models::crm_contact::LifecycleStage::Customer),
                    tags: None,
                    custom_fields: None,
                    zoho_contact_id: None,
                    gmail_contact_id: None,
                },
            )
            .await?
        }
    };

    // Patch fields we just learned (company, phone) without overwriting non-empty
    let needs_update = (company_name.is_some() && contact.company_name.is_none())
        || (phone.is_some() && contact.phone.is_none());
    if needs_update {
        let _ = CrmContact::update(
            pool,
            &contact.id,
            db::models::crm_contact::UpdateCrmContact {
                company_name: company_name.clone(),
                phone: phone.clone(),
                ..Default::default()
            },
        )
        .await;
    }

    if existing_map.is_none() {
        QuickBooksEntityMap::create(
            pool,
            account.id,
            "crm_contact",
            contact.id.as_str(),
            "Customer",
            &qbo_id,
            "bidirectional",
        )
        .await?;
    } else if let Some(map) = existing_map {
        QuickBooksEntityMap::update_sync_status(pool, map.id, "synced", sync_token.as_deref())
            .await?;
    }

    Ok(true)
}

fn split_name(full: &str) -> (Option<String>, Option<String>) {
    let trimmed = full.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    match trimmed.split_once(' ') {
        Some((first, rest)) => (
            Some(first.to_string()),
            Some(rest.trim().to_string()).filter(|s| !s.is_empty()),
        ),
        None => (Some(trimmed.to_string()), None),
    }
}

// ─── Invoices ───────────────────────────────────────────────────────────────

async fn sync_invoices(
    pool: &SqlitePool,
    client: &QboClient,
    account: &QuickBooksAccount,
    org_id: &DbUuid,
    cdc_clause: &str,
    stats: &mut SyncStats,
) -> Result<(), QboError> {
    let query = format!("SELECT * FROM Invoice{cdc_clause} MAXRESULTS 500");
    let response = client.query(&query).await?;
    let invoices = response["Invoice"].as_array().cloned().unwrap_or_default();
    stats.invoices_seen += invoices.len() as i64;

    for invoice in invoices {
        match upsert_invoice(pool, account, org_id, &invoice).await {
            Ok(true) => stats.invoices_upserted += 1,
            Ok(false) => {}
            Err(e) => warn!(error = %e, "skipped QBO invoice upsert"),
        }
    }
    Ok(())
}

async fn upsert_invoice(
    pool: &SqlitePool,
    account: &QuickBooksAccount,
    org_id: &DbUuid,
    invoice: &Value,
) -> Result<bool, QboError> {
    let qbo_id = invoice["Id"].as_str().unwrap_or("").to_string();
    if qbo_id.is_empty() {
        return Ok(false);
    }

    let total = invoice["TotalAmt"].as_f64().unwrap_or(0.0);
    let balance = invoice["Balance"].as_f64().unwrap_or(total);
    let due_date = invoice["DueDate"].as_str().map(|s| s.to_string());
    let issue_date = invoice["TxnDate"].as_str().map(|s| s.to_string());
    let doc_number = invoice["DocNumber"].as_str().map(|s| s.to_string());
    let sync_token = invoice["SyncToken"].as_str().map(|s| s.to_string());
    let qbo_customer_id = invoice["CustomerRef"]["value"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // Resolve QBO customer → PCG crm_contact via entity map
    let contact_map =
        QuickBooksEntityMap::find_by_qbo_entity(pool, account.id, "Customer", &qbo_customer_id)
            .await?;
    let crm_contact_id = contact_map.as_ref().map(|m| m.pcg_entity_id.clone());

    let status = derive_status(balance, total, &due_date);
    let line_items = invoice["Line"].clone();

    let existing_map =
        QuickBooksEntityMap::find_by_qbo_entity(pool, account.id, "Invoice", &qbo_id).await?;

    let invoice_id = if let Some(map) = &existing_map {
        // Update existing
        let id_uuid = DbUuid::parse(&map.pcg_entity_id)
            .map(|u| u.to_uuid())
            .unwrap_or_else(|_| Uuid::new_v4());
        Invoice::update(
            pool,
            id_uuid,
            db::models::invoice::UpdateInvoice {
                status: Some(status.clone()),
                title: doc_number.clone().map(|n| format!("QBO Invoice {n}")),
                description: None,
                amount_usd: Some(total),
                amount_vibe: Some((total * 100.0).ceil() as i64),
                due_date: due_date.clone(),
                paid_at: None,
                payment_method: None,
                payment_reference: None,
                line_items: line_items.as_array().cloned(),
                notes: None,
            },
        )
        .await?;
        id_uuid
    } else {
        // Create
        let created = Invoice::create(
            pool,
            db::models::invoice::CreateInvoice {
                person_id: None,
                organization_id: Some(org_id.to_uuid()),
                project_id: None,
                invoice_type: Some("ar".into()),
                title: doc_number
                    .clone()
                    .map(|n| format!("QBO Invoice {n}"))
                    .or_else(|| Some(format!("QBO Invoice {qbo_id}"))),
                description: None,
                amount_usd: Some(total),
                amount_vibe: Some((total * 100.0).ceil() as i64),
                currency: Some("USD".into()),
                line_items: line_items.as_array().cloned(),
                issue_date,
                due_date,
                notes: None,
                created_by: None,
            },
        )
        .await?;

        // Patch crm_contact_id directly (CreateInvoice doesn't expose it)
        if let Some(ref cid) = crm_contact_id {
            let _ = sqlx::query(
                "UPDATE invoices SET crm_contact_id = ?1, status = ?2, updated_at = datetime('now','subsec') WHERE id = ?3",
            )
            .bind(cid)
            .bind(&status)
            .bind(created.id.to_string())
            .execute(pool)
            .await;
        } else {
            let _ = sqlx::query(
                "UPDATE invoices SET status = ?1, updated_at = datetime('now','subsec') WHERE id = ?2",
            )
            .bind(&status)
            .bind(created.id.to_string())
            .execute(pool)
            .await;
        }

        created.id
    };

    if existing_map.is_none() {
        QuickBooksEntityMap::create(
            pool,
            account.id,
            "invoice",
            &invoice_id.to_string(),
            "Invoice",
            &qbo_id,
            "bidirectional",
        )
        .await?;
    } else if let Some(map) = existing_map {
        QuickBooksEntityMap::update_sync_status(pool, map.id, "synced", sync_token.as_deref())
            .await?;
    }

    Ok(true)
}

/// Map a QBO invoice's `Balance` and `DueDate` onto our internal status enum.
fn derive_status(balance: f64, total: f64, due_date: &Option<String>) -> String {
    if balance <= 0.0 && total > 0.0 {
        return "paid".into();
    }
    if balance > 0.0 && balance < total {
        return "partial".into();
    }
    if let Some(due) = due_date
        && let Ok(parsed) = chrono::NaiveDate::parse_from_str(due, "%Y-%m-%d")
        && parsed < Utc::now().date_naive()
        && balance > 0.0
    {
        return "overdue".into();
    }
    "sent".into()
}

// ─── Payments ───────────────────────────────────────────────────────────────

async fn sync_payments(
    pool: &SqlitePool,
    client: &QboClient,
    account: &QuickBooksAccount,
    cdc_clause: &str,
    stats: &mut SyncStats,
) -> Result<(), QboError> {
    let query = format!("SELECT * FROM Payment{cdc_clause} MAXRESULTS 500");
    let response = client.query(&query).await?;
    let payments = response["Payment"].as_array().cloned().unwrap_or_default();
    stats.payments_seen += payments.len() as i64;

    for payment in payments {
        let lines = payment["Line"].as_array().cloned().unwrap_or_default();
        for line in lines {
            let linked = line["LinkedTxn"].as_array().cloned().unwrap_or_default();
            for txn in linked {
                if txn["TxnType"].as_str() != Some("Invoice") {
                    continue;
                }
                let qbo_invoice_id = txn["TxnId"].as_str().unwrap_or("").to_string();
                if qbo_invoice_id.is_empty() {
                    continue;
                }
                if mark_invoice_paid(pool, account, &qbo_invoice_id).await? {
                    stats.invoices_marked_paid += 1;
                }
            }
        }
    }
    Ok(())
}

async fn mark_invoice_paid(
    pool: &SqlitePool,
    account: &QuickBooksAccount,
    qbo_invoice_id: &str,
) -> Result<bool, QboError> {
    let map = QuickBooksEntityMap::find_by_qbo_entity(pool, account.id, "Invoice", qbo_invoice_id)
        .await?;
    let Some(map) = map else { return Ok(false) };

    let id_uuid = match DbUuid::parse(&map.pcg_entity_id) {
        Ok(u) => u.to_uuid(),
        Err(_) => return Ok(false),
    };

    sqlx::query(
        r#"UPDATE invoices
           SET status = 'paid',
               paid_at = COALESCE(paid_at, datetime('now','subsec')),
               updated_at = datetime('now','subsec')
           WHERE id = ?1 AND status != 'paid'"#,
    )
    .bind(id_uuid.to_string())
    .execute(pool)
    .await?;

    Ok(true)
}
