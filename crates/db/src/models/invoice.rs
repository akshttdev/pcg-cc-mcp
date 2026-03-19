use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// An Invoice represents either:
/// - AR (Accounts Receivable): a client owes PCG money for services delivered
/// - AP (Accounts Payable):    PCG owes a contractor money for work performed
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Invoice {
    pub id: Uuid,
    pub invoice_number: String,

    pub person_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,

    /// 'ar' | 'ap'
    pub invoice_type: String,

    /// 'draft' | 'sent' | 'viewed' | 'paid' | 'partial' | 'overdue' | 'void' | 'cancelled'
    pub status: String,

    pub amount_usd: f64,
    pub amount_vibe: i64,
    pub currency: String,

    pub title: Option<String>,
    pub description: Option<String>,
    /// JSON array of line items
    pub line_items: String,

    pub issue_date: Option<String>,
    pub due_date: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,

    pub payment_method: Option<String>,
    pub payment_reference: Option<String>,

    pub notes: Option<String>,
    pub metadata: String,

    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateInvoice {
    pub person_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    /// 'ar' | 'ap'
    pub invoice_type: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub amount_usd: Option<f64>,
    pub amount_vibe: Option<i64>,
    pub currency: Option<String>,
    pub line_items: Option<Vec<serde_json::Value>>,
    pub issue_date: Option<String>,
    pub due_date: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateInvoice {
    pub status: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub amount_usd: Option<f64>,
    pub amount_vibe: Option<i64>,
    pub due_date: Option<String>,
    pub paid_at: Option<String>,
    pub payment_method: Option<String>,
    pub payment_reference: Option<String>,
    pub line_items: Option<Vec<serde_json::Value>>,
    pub notes: Option<String>,
}

impl Invoice {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> sqlx::Result<Option<Self>> {
        sqlx::query_as("SELECT * FROM invoices WHERE id = ?1")
            .bind(id.to_string())
            .fetch_optional(pool)
            .await
    }

    pub async fn list_for_person(pool: &SqlitePool, person_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT * FROM invoices WHERE person_id = ?1 ORDER BY created_at DESC")
            .bind(person_id.to_string())
            .fetch_all(pool)
            .await
    }

    pub async fn list_for_project(pool: &SqlitePool, project_id: Uuid) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as("SELECT * FROM invoices WHERE project_id = ?1 ORDER BY created_at DESC")
            .bind(project_id.to_string())
            .fetch_all(pool)
            .await
    }

    pub async fn create(pool: &SqlitePool, data: CreateInvoice) -> sqlx::Result<Self> {
        let id = Uuid::new_v4();

        // Generate sequential invoice number
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM invoices")
            .fetch_one(pool)
            .await?;
        let invoice_type = data.invoice_type.as_deref().unwrap_or("ar");
        let prefix = if invoice_type == "ap" { "AP" } else { "AR" };
        let invoice_number = format!("{}-{:05}", prefix, count + 1);

        let line_items = data
            .line_items
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".into()))
            .unwrap_or_else(|| "[]".into());

        sqlx::query(
            r#"INSERT INTO invoices
               (id, invoice_number, person_id, organization_id, project_id,
                invoice_type, amount_usd, amount_vibe, currency,
                title, description, line_items, issue_date, due_date,
                notes, created_by)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)"#,
        )
        .bind(id.to_string())
        .bind(&invoice_number)
        .bind(data.person_id.as_ref().map(|u| u.to_string()))
        .bind(data.organization_id.as_ref().map(|u| u.to_string()))
        .bind(data.project_id.as_ref().map(|u| u.to_string()))
        .bind(invoice_type)
        .bind(data.amount_usd.unwrap_or(0.0))
        .bind(data.amount_vibe.unwrap_or(0))
        .bind(data.currency.as_deref().unwrap_or("USD"))
        .bind(&data.title)
        .bind(&data.description)
        .bind(&line_items)
        .bind(&data.issue_date)
        .bind(&data.due_date)
        .bind(&data.notes)
        .bind(data.created_by.as_ref().map(|u| u.to_string()))
        .execute(pool)
        .await?;

        Ok(Self::find_by_id(pool, id).await?.expect("just inserted"))
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateInvoice,
    ) -> sqlx::Result<Option<Self>> {
        let existing = match Self::find_by_id(pool, id).await? {
            Some(i) => i,
            None => return Ok(None),
        };

        let status = data.status.unwrap_or(existing.status);
        let amount_usd = data.amount_usd.unwrap_or(existing.amount_usd);
        let amount_vibe = data.amount_vibe.unwrap_or(existing.amount_vibe);
        let line_items = data
            .line_items
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".into()))
            .unwrap_or(existing.line_items);

        sqlx::query(
            r#"UPDATE invoices SET
               status=?2, title=?3, description=?4, amount_usd=?5, amount_vibe=?6,
               due_date=?7, paid_at=?8, payment_method=?9, payment_reference=?10,
               line_items=?11, notes=?12, updated_at=datetime('now','subsec')
               WHERE id=?1"#,
        )
        .bind(id.to_string())
        .bind(&status)
        .bind(data.title.or(existing.title))
        .bind(data.description.or(existing.description))
        .bind(amount_usd)
        .bind(amount_vibe)
        .bind(data.due_date.or(existing.due_date))
        .bind(data.paid_at)
        .bind(data.payment_method.or(existing.payment_method))
        .bind(data.payment_reference.or(existing.payment_reference))
        .bind(&line_items)
        .bind(data.notes.or(existing.notes))
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM invoices WHERE id = ?1")
            .bind(id.to_string())
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
