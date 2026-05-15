//! Stripe billing layer.
//!
//! - `stripe`  — minimal REST client (no SDK dep).
//! - `webhook` — HMAC verification + event dispatch.
//! - `usage`   — per-org per-period action metering.
//!
//! The internal VIBE economy (`vibe_transactions`, `model_pricing`) is the
//! source of truth for what each AI action costs. This layer adds:
//!   1. The external payment bridge (Stripe checkout + webhooks).
//!   2. Plan-aware limits — block / warn before the next action.

pub mod stripe;
pub mod usage;
pub mod webhook;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BillingError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Subscription(#[from] db::models::org_subscription::OrgSubscriptionError),
    #[error(transparent)]
    Usage(#[from] db::models::usage_record::UsageRecordError),
    #[error("Stripe credentials not configured: {0}")]
    NotConfigured(&'static str),
    #[error("Stripe API error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("network error talking to Stripe: {0}")]
    Network(#[from] reqwest::Error),
    #[error("could not parse Stripe response: {0}")]
    Parse(String),
    #[error("invalid webhook signature")]
    InvalidSignature,
    #[error("webhook missing organization_id metadata")]
    MissingOrgMetadata,
    #[error("unknown plan tier: {0}")]
    UnknownPlan(String),
}

pub use stripe::{
    StripeCheckoutSession, StripePortalSession, create_billing_portal_session,
    create_checkout_session, ensure_customer, price_id_for_plan,
};
pub use usage::{UsageStatus, record_action, usage_status};
pub use webhook::handle_event;
