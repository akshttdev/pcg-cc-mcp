//! Push a PCG invoice up to QuickBooks Online.
//!
//! Steps:
//! 1. Resolve the linked PCG contact → QBO Customer (creating one in QBO if
//!    we don't have an entity-map row yet).
//! 2. POST `/invoice` with line items derived from `invoices.line_items`.
//! 3. Persist the QBO `Id` and `DocNumber` back into `quickbooks_entity_map`
//!    plus the `invoices.metadata` JSON (so the frontend can display
//!    "Pushed to QBO #1042" without joining the map).

use db::{
    db_uuid::DbUuid,
    models::{
        crm_contact::CrmContact,
        invoice::Invoice,
        quickbooks_account::{QuickBooksAccount, QuickBooksEntityMap},
    },
};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

use super::{QboError, client::QboClient};

/// Push one PCG invoice into the org's connected QBO realm.
///
/// Returns the QBO `DocNumber` (or `Id`, if QBO didn't assign a doc number).
pub async fn push_invoice(
    pool: &SqlitePool,
    account_id: Uuid,
    invoice_id: Uuid,
) -> Result<String, QboError> {
    let account = QuickBooksAccount::find_by_id(pool, account_id).await?;
    let invoice = Invoice::find_by_id(pool, invoice_id)
        .await?
        .ok_or(QboError::InvoiceMissingCustomer(invoice_id))?;

    let client = QboClient::from_account_refreshing(pool, &account).await?;
    let qbo_customer_id = ensure_qbo_customer(pool, &client, &account, &invoice).await?;
    let body = build_invoice_body(&invoice, &qbo_customer_id);

    let resp = client.create_entity("invoice", &body).await?;
    let new_invoice = &resp["Invoice"];
    let qbo_id = new_invoice["Id"].as_str().unwrap_or("").to_string();
    let doc_number = new_invoice["DocNumber"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| qbo_id.clone());

    if !qbo_id.is_empty() {
        QuickBooksEntityMap::create(
            pool,
            account.id,
            "invoice",
            &invoice_id.to_string(),
            "Invoice",
            &qbo_id,
            "pcg_to_qbo",
        )
        .await?;
    }

    persist_qbo_doc_number(pool, invoice_id, &qbo_id, &doc_number).await?;
    info!(
        invoice_id = %invoice_id,
        qbo_id = %qbo_id,
        doc_number = %doc_number,
        "pushed invoice to QBO"
    );
    Ok(doc_number)
}

/// Find an existing QBO Customer (via entity-map) or create one based on
/// `invoice.crm_contact_id`. Returns the QBO Customer Id.
async fn ensure_qbo_customer(
    pool: &SqlitePool,
    client: &QboClient,
    account: &QuickBooksAccount,
    invoice: &Invoice,
) -> Result<String, QboError> {
    let crm_contact_id = invoice
        .crm_contact_id
        .as_deref()
        .ok_or(QboError::InvoiceMissingCustomer(invoice.id))?;

    let map =
        QuickBooksEntityMap::find_by_pcg_entity(pool, account.id, "crm_contact", crm_contact_id)
            .await?;

    if let Some(m) = map {
        return Ok(m.qbo_entity_id);
    }

    let contact_uuid = DbUuid::from_string(crm_contact_id);
    let contact = CrmContact::find_by_id(pool, &contact_uuid).await?;

    let display_name = contact
        .full_name
        .clone()
        .or_else(|| contact.company_name.clone())
        .or_else(|| contact.email.clone())
        .unwrap_or_else(|| format!("PCG Contact {}", contact_uuid.as_str()));

    let mut customer_body = json!({
        "DisplayName": display_name,
    });
    if let Some(email) = &contact.email {
        customer_body["PrimaryEmailAddr"] = json!({ "Address": email });
    }
    if let Some(company) = &contact.company_name {
        customer_body["CompanyName"] = json!(company);
    }
    if let Some(phone) = contact.phone.as_ref().or(contact.mobile.as_ref()) {
        customer_body["PrimaryPhone"] = json!({ "FreeFormNumber": phone });
    }

    let resp = client.create_entity("customer", &customer_body).await?;
    let qbo_id = resp["Customer"]["Id"]
        .as_str()
        .ok_or_else(|| QboError::Parse("QBO Customer create returned no Id".into()))?
        .to_string();

    QuickBooksEntityMap::create(
        pool,
        account.id,
        "crm_contact",
        crm_contact_id,
        "Customer",
        &qbo_id,
        "pcg_to_qbo",
    )
    .await?;

    Ok(qbo_id)
}

fn build_invoice_body(invoice: &Invoice, qbo_customer_id: &str) -> Value {
    let line_items = parsed_line_items(invoice);

    let mut body = json!({
        "CustomerRef": { "value": qbo_customer_id },
        "Line": line_items,
    });
    if let Some(due_date) = &invoice.due_date {
        body["DueDate"] = json!(due_date);
    }
    if let Some(issue_date) = &invoice.issue_date {
        body["TxnDate"] = json!(issue_date);
    }
    if let Some(notes) = &invoice.notes {
        body["CustomerMemo"] = json!({ "value": notes });
    }
    body
}

/// Build QBO `Line` items from our stored JSON. Falls back to one
/// `SalesItemLineDetail` for the full invoice total if the stored items
/// can't be reshaped (or are empty).
fn parsed_line_items(invoice: &Invoice) -> Value {
    let parsed: Result<Vec<Value>, _> = serde_json::from_str(&invoice.line_items);
    let items = parsed.unwrap_or_default();

    if items.is_empty() {
        return json!([{
            "DetailType": "SalesItemLineDetail",
            "Amount": invoice.amount_usd,
            "Description": invoice.description
                .clone()
                .or_else(|| invoice.title.clone())
                .unwrap_or_else(|| invoice.invoice_number.clone()),
            "SalesItemLineDetail": {
                "ItemRef": { "value": "1", "name": "Services" }
            }
        }]);
    }

    items
        .into_iter()
        .map(|item| {
            let amount = item["amount_usd"]
                .as_f64()
                .or_else(|| item["amount"].as_f64())
                .unwrap_or(0.0);
            let description = item["description"]
                .as_str()
                .or_else(|| item["title"].as_str())
                .unwrap_or("Line item")
                .to_string();
            json!({
                "DetailType": "SalesItemLineDetail",
                "Amount": amount,
                "Description": description,
                "SalesItemLineDetail": {
                    "ItemRef": { "value": "1", "name": "Services" }
                }
            })
        })
        .collect::<Vec<_>>()
        .into()
}

async fn persist_qbo_doc_number(
    pool: &SqlitePool,
    invoice_id: Uuid,
    qbo_id: &str,
    doc_number: &str,
) -> Result<(), QboError> {
    sqlx::query(
        r#"UPDATE invoices
           SET metadata = json_set(
                   COALESCE(metadata, '{}'),
                   '$.qbo_id', ?2,
                   '$.qbo_doc_number', ?3,
                   '$.qbo_pushed_at', datetime('now','subsec')
               ),
               updated_at = datetime('now','subsec')
           WHERE id = ?1"#,
    )
    .bind(invoice_id.to_string())
    .bind(qbo_id)
    .bind(doc_number)
    .execute(pool)
    .await?;
    Ok(())
}
