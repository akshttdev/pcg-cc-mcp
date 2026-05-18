//! Stripe webhook signature verification + event dispatch.
//!
//! Verification follows Stripe's spec: the `Stripe-Signature` header carries
//! `t=<timestamp>,v1=<hmac_sha256(secret, timestamp + "." + body)>`. We reject
//! anything older than 5 minutes to limit replay.
//!
//! Idempotency: every event ID is recorded in `stripe_webhook_events` with
//! `INSERT OR IGNORE`. If we see the same id twice we skip processing.
//!
//! Supported events (per spec):
//!   checkout.session.completed     → activate subscription
//!   invoice.payment_succeeded      → period roll, reset usage
//!   invoice.payment_failed         → mark past_due
//!   customer.subscription.deleted  → drop to free tier
//!   customer.subscription.updated  → plan change

use std::str::FromStr;

use db::{
    db_uuid::DbUuid,
    models::org_subscription::{OrgSubscription, PlanTier},
};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::Sha256;
use sqlx::SqlitePool;
use tracing::{info, warn};

use super::BillingError;

type HmacSha256 = Hmac<Sha256>;

/// Maximum age of a Stripe webhook timestamp before we reject it. Stripe
/// recommends 5 minutes.
const TOLERANCE_SECS: i64 = 300;

/// Verify the `Stripe-Signature` header against the raw request body using
/// the configured webhook secret. Returns `Ok(())` only if every check passes.
pub fn verify_signature(
    signature_header: &str,
    body: &[u8],
    secret: &str,
) -> Result<(), BillingError> {
    let mut timestamp: Option<i64> = None;
    let mut sigs: Vec<&str> = Vec::new();
    for part in signature_header.split(',') {
        let (k, v) = match part.split_once('=') {
            Some(kv) => kv,
            None => continue,
        };
        match k {
            "t" => timestamp = v.parse().ok(),
            "v1" => sigs.push(v),
            _ => {}
        }
    }
    let Some(ts) = timestamp else {
        return Err(BillingError::InvalidSignature);
    };
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > TOLERANCE_SECS {
        return Err(BillingError::InvalidSignature);
    }

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| BillingError::InvalidSignature)?;
    mac.update(format!("{ts}.").as_bytes());
    mac.update(body);
    let expected = mac.finalize().into_bytes();
    let expected_hex = hex::encode(expected);

    if sigs
        .iter()
        .any(|s| constant_time_eq(s.as_bytes(), expected_hex.as_bytes()))
    {
        Ok(())
    } else {
        Err(BillingError::InvalidSignature)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

/// Top-level dispatch: inserts the event id (idempotent) and routes based on
/// `event.type`. Unknown events are recorded but ignored — common for Stripe.
pub async fn handle_event(pool: &SqlitePool, body: &[u8]) -> Result<(), BillingError> {
    let event: Value = serde_json::from_slice(body)
        .map_err(|e| BillingError::Parse(format!("event body: {e}")))?;
    let event_id = event["id"]
        .as_str()
        .ok_or_else(|| BillingError::Parse("event missing id".into()))?;
    let event_type = event["type"]
        .as_str()
        .ok_or_else(|| BillingError::Parse("event missing type".into()))?;

    let inserted = sqlx::query(
        r#"
        INSERT OR IGNORE INTO stripe_webhook_events (id, event_type, payload)
        VALUES (?1, ?2, ?3)
        "#,
    )
    .bind(event_id)
    .bind(event_type)
    .bind(body)
    .execute(pool)
    .await?;
    if inserted.rows_affected() == 0 {
        info!(event_id = %event_id, "stripe webhook duplicate, skipping");
        return Ok(());
    }

    let object = &event["data"]["object"];
    let result = match event_type {
        "checkout.session.completed" => handle_checkout_completed(pool, object).await,
        "invoice.payment_succeeded" => handle_invoice_paid(pool, object).await,
        "invoice.payment_failed" => handle_invoice_failed(pool, object).await,
        "customer.subscription.updated" => handle_subscription_updated(pool, object).await,
        "customer.subscription.deleted" => handle_subscription_deleted(pool, object).await,
        other => {
            info!(event_type = %other, "stripe webhook ignored (no handler)");
            Ok(())
        }
    };

    let processed = result.is_ok() as i64;
    let last_error = result.as_ref().err().map(|e| e.to_string());
    sqlx::query(
        r#"
        UPDATE stripe_webhook_events
        SET processed = ?2, last_error = ?3
        WHERE id = ?1
        "#,
    )
    .bind(event_id)
    .bind(processed)
    .bind(last_error)
    .execute(pool)
    .await?;
    result
}

// ─── Per-event handlers ─────────────────────────────────────────────────────

async fn handle_checkout_completed(pool: &SqlitePool, object: &Value) -> Result<(), BillingError> {
    let org_id = object["metadata"]["organization_id"]
        .as_str()
        .ok_or(BillingError::MissingOrgMetadata)?;
    let plan_str = object["metadata"]["plan"].as_str().unwrap_or("starter");
    let plan =
        PlanTier::from_str(plan_str).map_err(|_| BillingError::UnknownPlan(plan_str.into()))?;
    let subscription_id = object["subscription"].as_str();
    let customer_id = object["customer"].as_str();

    let org_uuid = DbUuid::from_string(org_id.to_string());
    OrgSubscription::ensure_free(pool, &org_uuid).await?;
    if let Some(cust) = customer_id {
        OrgSubscription::set_stripe_customer(pool, &org_uuid, cust).await?;
    }
    OrgSubscription::upgrade(
        pool,
        &org_uuid,
        plan,
        "active",
        subscription_id,
        None,
        None,
        None,
        false,
    )
    .await?;
    info!(org_id = %org_id, plan = %plan_str, "subscription activated via checkout");
    Ok(())
}

async fn handle_invoice_paid(pool: &SqlitePool, object: &Value) -> Result<(), BillingError> {
    let customer_id = match object["customer"].as_str() {
        Some(c) => c,
        None => return Ok(()),
    };
    let Some(sub) = OrgSubscription::find_by_stripe_customer(pool, customer_id).await? else {
        warn!(customer = %customer_id, "invoice.paid for unknown stripe customer");
        return Ok(());
    };
    // The new period bounds come on the embedded `lines.data[0].period`. If
    // we can't find them, skip the period roll — usage stays in the prior row,
    // not ideal but safe.
    let period = &object["lines"]["data"][0]["period"];
    let new_start = period["start"]
        .as_i64()
        .map(unix_to_iso_date)
        .or_else(|| object["period_start"].as_i64().map(unix_to_iso_date));
    let new_end = period["end"]
        .as_i64()
        .map(unix_to_iso_date)
        .or_else(|| object["period_end"].as_i64().map(unix_to_iso_date));

    let plan =
        PlanTier::from_str(&sub.plan).map_err(|_| BillingError::UnknownPlan(sub.plan.clone()))?;
    let org_uuid = DbUuid::from_string(sub.organization_id.as_str().to_string());
    OrgSubscription::upgrade(
        pool,
        &org_uuid,
        plan,
        "active",
        sub.stripe_subscription_id.as_deref(),
        sub.stripe_price_id.as_deref(),
        new_start.as_deref(),
        new_end.as_deref(),
        sub.cancel_at_period_end == 1,
    )
    .await?;
    info!(customer = %customer_id, "invoice paid; period rolled");
    Ok(())
}

async fn handle_invoice_failed(pool: &SqlitePool, object: &Value) -> Result<(), BillingError> {
    let customer_id = match object["customer"].as_str() {
        Some(c) => c,
        None => return Ok(()),
    };
    let Some(sub) = OrgSubscription::find_by_stripe_customer(pool, customer_id).await? else {
        return Ok(());
    };
    let plan =
        PlanTier::from_str(&sub.plan).map_err(|_| BillingError::UnknownPlan(sub.plan.clone()))?;
    let org_uuid = DbUuid::from_string(sub.organization_id.as_str().to_string());
    OrgSubscription::upgrade(
        pool,
        &org_uuid,
        plan,
        "past_due",
        sub.stripe_subscription_id.as_deref(),
        sub.stripe_price_id.as_deref(),
        sub.current_period_start.as_deref(),
        sub.current_period_end.as_deref(),
        sub.cancel_at_period_end == 1,
    )
    .await?;
    warn!(customer = %customer_id, "invoice payment failed; marked past_due");
    Ok(())
}

async fn handle_subscription_updated(
    pool: &SqlitePool,
    object: &Value,
) -> Result<(), BillingError> {
    let customer_id = match object["customer"].as_str() {
        Some(c) => c,
        None => return Ok(()),
    };
    let Some(sub) = OrgSubscription::find_by_stripe_customer(pool, customer_id).await? else {
        return Ok(());
    };
    let status = object["status"].as_str().unwrap_or("active").to_string();
    let plan_str = object["metadata"]["plan"]
        .as_str()
        .unwrap_or(sub.plan.as_str());
    let plan =
        PlanTier::from_str(plan_str).map_err(|_| BillingError::UnknownPlan(plan_str.into()))?;
    let price_id = object["items"]["data"][0]["price"]["id"].as_str();
    let period_start = object["current_period_start"]
        .as_i64()
        .map(unix_to_iso_date);
    let period_end = object["current_period_end"].as_i64().map(unix_to_iso_date);
    let cancel_at_period_end = object["cancel_at_period_end"].as_bool().unwrap_or(false);
    let subscription_id = object["id"].as_str();

    let org_uuid = DbUuid::from_string(sub.organization_id.as_str().to_string());
    OrgSubscription::upgrade(
        pool,
        &org_uuid,
        plan,
        &status,
        subscription_id,
        price_id,
        period_start.as_deref(),
        period_end.as_deref(),
        cancel_at_period_end,
    )
    .await?;
    info!(customer = %customer_id, plan = %plan_str, status = %status, "subscription updated");
    Ok(())
}

async fn handle_subscription_deleted(
    pool: &SqlitePool,
    object: &Value,
) -> Result<(), BillingError> {
    if let Some(subscription_id) = object["id"].as_str() {
        OrgSubscription::mark_canceled(pool, subscription_id).await?;
        info!(subscription = %subscription_id, "subscription canceled");
    }
    Ok(())
}

fn unix_to_iso_date(ts: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
        .map(|d| d.to_rfc3339())
        .unwrap_or_default()
}
