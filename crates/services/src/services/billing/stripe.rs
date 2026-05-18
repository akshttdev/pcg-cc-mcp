//! Stripe REST API client. No SDK — just `reqwest` + form-encoded bodies.
//!
//! Endpoints used:
//!   POST /v1/customers
//!   POST /v1/checkout/sessions
//!   POST /v1/billing_portal/sessions
//!
//! Auth: Bearer token from `STRIPE_SECRET_KEY`.

use db::{
    db_uuid::DbUuid,
    models::org_subscription::{OrgSubscription, PlanTier},
};
use reqwest::Client;
use serde::Deserialize;
use sqlx::SqlitePool;

use super::BillingError;

const STRIPE_API: &str = "https://api.stripe.com/v1";

fn secret_key() -> Result<String, BillingError> {
    std::env::var("STRIPE_SECRET_KEY").map_err(|_| BillingError::NotConfigured("STRIPE_SECRET_KEY"))
}

/// Look up the configured Stripe price id for a plan tier. We expect one env
/// var per non-free tier — Stripe price ids are env-specific so this can't be
/// hard-coded.
pub fn price_id_for_plan(plan: PlanTier) -> Result<String, BillingError> {
    let env_var = match plan {
        PlanTier::Free => return Err(BillingError::UnknownPlan("free".into())),
        PlanTier::Starter => "STRIPE_PRICE_STARTER",
        PlanTier::Pro => "STRIPE_PRICE_PRO",
        PlanTier::Scale => "STRIPE_PRICE_SCALE",
        PlanTier::Enterprise => "STRIPE_PRICE_ENTERPRISE",
    };
    std::env::var(env_var).map_err(|_| BillingError::NotConfigured(box_leak(env_var)))
}

// `NotConfigured` takes a `&'static str`. `env_var` already is.
fn box_leak(s: &'static str) -> &'static str {
    s
}

#[derive(Debug, Deserialize)]
struct StripeCustomerResponse {
    id: String,
}

/// Find or create the Stripe Customer for this org. Persists the id back
/// onto `org_subscriptions` so subsequent calls skip the lookup.
pub async fn ensure_customer(
    pool: &SqlitePool,
    sub: &OrgSubscription,
    org_name: &str,
    org_email: Option<&str>,
) -> Result<String, BillingError> {
    if let Some(id) = &sub.stripe_customer_id {
        return Ok(id.clone());
    }

    let key = secret_key()?;
    let mut form: Vec<(String, String)> = vec![
        ("name".into(), org_name.to_string()),
        (
            "metadata[organization_id]".into(),
            sub.organization_id.as_str().to_string(),
        ),
    ];
    if let Some(email) = org_email {
        form.push(("email".into(), email.to_string()));
    }

    let resp = Client::new()
        .post(format!("{STRIPE_API}/customers"))
        .bearer_auth(&key)
        .form(&form)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(BillingError::Api {
            status: status.as_u16(),
            body,
        });
    }
    let parsed: StripeCustomerResponse = resp.json().await?;

    let org_id = DbUuid::from_string(sub.organization_id.as_str().to_string());
    OrgSubscription::set_stripe_customer(pool, &org_id, &parsed.id).await?;

    Ok(parsed.id)
}

#[derive(Debug, Deserialize)]
pub struct StripeCheckoutSession {
    pub id: String,
    pub url: String,
}

/// Create a Stripe Checkout session for the requested plan. Caller must have
/// already resolved a Stripe customer id (`ensure_customer`).
pub async fn create_checkout_session(
    customer_id: &str,
    organization_id: &str,
    plan: PlanTier,
    success_url: &str,
    cancel_url: &str,
) -> Result<StripeCheckoutSession, BillingError> {
    let key = secret_key()?;
    let price_id = price_id_for_plan(plan)?;

    let form: Vec<(String, String)> = vec![
        ("customer".into(), customer_id.to_string()),
        ("mode".into(), "subscription".into()),
        ("line_items[0][price]".into(), price_id),
        ("line_items[0][quantity]".into(), "1".into()),
        ("success_url".into(), success_url.to_string()),
        ("cancel_url".into(), cancel_url.to_string()),
        (
            "metadata[organization_id]".into(),
            organization_id.to_string(),
        ),
        ("metadata[plan]".into(), plan.as_str().to_string()),
        // Mirror the same metadata onto the subscription so the webhook for
        // `customer.subscription.updated` can read it without round-tripping.
        (
            "subscription_data[metadata][organization_id]".into(),
            organization_id.to_string(),
        ),
        (
            "subscription_data[metadata][plan]".into(),
            plan.as_str().to_string(),
        ),
    ];

    let resp = Client::new()
        .post(format!("{STRIPE_API}/checkout/sessions"))
        .bearer_auth(&key)
        .form(&form)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(BillingError::Api {
            status: status.as_u16(),
            body,
        });
    }
    Ok(resp.json().await?)
}

#[derive(Debug, Deserialize)]
pub struct StripePortalSession {
    pub id: String,
    pub url: String,
}

/// Create a Stripe Billing Portal session so the customer can self-serve
/// (update card, cancel, switch plan).
pub async fn create_billing_portal_session(
    customer_id: &str,
    return_url: &str,
) -> Result<StripePortalSession, BillingError> {
    let key = secret_key()?;
    let form = [("customer", customer_id), ("return_url", return_url)];
    let resp = Client::new()
        .post(format!("{STRIPE_API}/billing_portal/sessions"))
        .bearer_auth(&key)
        .form(&form)
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(BillingError::Api {
            status: status.as_u16(),
            body,
        });
    }
    Ok(resp.json().await?)
}
