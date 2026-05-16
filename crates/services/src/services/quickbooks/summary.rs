//! Read-only AR/AP aggregates for the command-center widget.
//!
//! Computed entirely from the local `invoices` table — we trust whatever the
//! last QBO sync wrote, so this is a one-table query and stays cheap.

use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

use super::QboError;

type SummaryRow = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    i64,
    i64,
);

#[derive(Debug, Default, Serialize, TS)]
#[ts(export)]
pub struct FinancialSummary {
    pub ar_outstanding_usd: f64,
    pub ar_overdue_usd: f64,
    pub ap_outstanding_usd: f64,
    pub revenue_30d_usd: f64,
    pub revenue_60d_usd: f64,
    pub revenue_90d_usd: f64,
    pub open_invoice_count: i64,
    pub overdue_invoice_count: i64,
}

/// Aggregate AR/AP totals for an organization. AR = invoices we issued (`invoice_type = 'ar'`),
/// AP = bills we owe (`invoice_type = 'ap'`).
pub async fn financial_summary(
    pool: &SqlitePool,
    organization_id: Uuid,
) -> Result<FinancialSummary, QboError> {
    let today = Utc::now().date_naive().to_string();
    let org_text = organization_id.to_string();
    let org_bytes = organization_id.as_bytes().to_vec();

    // `invoices.organization_id` is a legacy BLOB column on some envs and TEXT on others;
    // bind both forms so the WHERE clause matches regardless of storage type.
    let row: SummaryRow = sqlx::query_as(
            r#"
            SELECT
                COALESCE(SUM(CASE WHEN invoice_type = 'ar' AND status IN ('sent','viewed','partial','overdue') THEN amount_usd ELSE 0.0 END), 0.0) AS ar_outstanding,
                COALESCE(SUM(CASE WHEN invoice_type = 'ar' AND status = 'overdue' THEN amount_usd ELSE 0.0 END), 0.0) AS ar_overdue,
                COALESCE(SUM(CASE WHEN invoice_type = 'ap' AND status IN ('sent','viewed','partial','overdue') THEN amount_usd ELSE 0.0 END), 0.0) AS ap_outstanding,
                COALESCE(SUM(CASE WHEN invoice_type = 'ar' AND status = 'paid' AND date(paid_at) >= date(?2, '-30 days') THEN amount_usd ELSE 0.0 END), 0.0) AS rev_30,
                COALESCE(SUM(CASE WHEN invoice_type = 'ar' AND status = 'paid' AND date(paid_at) >= date(?2, '-60 days') THEN amount_usd ELSE 0.0 END), 0.0) AS rev_60,
                COALESCE(SUM(CASE WHEN invoice_type = 'ar' AND status = 'paid' AND date(paid_at) >= date(?2, '-90 days') THEN amount_usd ELSE 0.0 END), 0.0) AS rev_90,
                COALESCE(SUM(CASE WHEN invoice_type = 'ar' AND status IN ('sent','viewed','partial','overdue') THEN 1 ELSE 0 END), 0) AS open_count,
                COALESCE(SUM(CASE WHEN invoice_type = 'ar' AND status = 'overdue' THEN 1 ELSE 0 END), 0) AS overdue_count
            FROM invoices
            WHERE organization_id = ?1 OR organization_id = ?3
            "#,
        )
        .bind(&org_text)
        .bind(&today)
        .bind(&org_bytes)
        .fetch_one(pool)
        .await?;

    Ok(FinancialSummary {
        ar_outstanding_usd: row.0.unwrap_or(0.0),
        ar_overdue_usd: row.1.unwrap_or(0.0),
        ap_outstanding_usd: row.2.unwrap_or(0.0),
        revenue_30d_usd: row.3.unwrap_or(0.0),
        revenue_60d_usd: row.4.unwrap_or(0.0),
        revenue_90d_usd: row.5.unwrap_or(0.0),
        open_invoice_count: row.6,
        overdue_invoice_count: row.7,
    })
}
