//! QuickBooks Online sync + push services.
//!
//! - `client`  — thin HTTP wrapper around the QBO v3 REST API with automatic
//!   token refresh.
//! - `sync`    — pull QBO entities (Customers, Invoices, Payments) into PCG
//!   (`crm_contacts`, `invoices`, `quickbooks_entity_map`).
//! - `push`    — push a PCG invoice up to QBO as a Customer + Invoice.
//! - `summary` — read-only AR/AP aggregates for the command-center widget.
//!
//! Credentials come from env vars (`QUICKBOOKS_CLIENT_ID` / `..._SECRET`) and
//! are managed by the existing `quickbooks_accounts` table; this module never
//! writes tokens directly — it goes through `QuickBooksAccount::update_tokens`.

pub mod client;
pub mod push;
pub mod summary;
pub mod sync;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum QboError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    QuickBooksAccount(#[from] db::models::quickbooks_account::QuickBooksAccountError),
    #[error(transparent)]
    CrmContact(#[from] db::models::crm_contact::CrmContactError),
    #[error("missing access token for QuickBooks account")]
    MissingAccessToken,
    #[error("missing refresh token for QuickBooks account")]
    MissingRefreshToken,
    #[error("QuickBooks credentials not configured: {0}")]
    NotConfigured(String),
    #[error("QuickBooks API error ({status}): {body}")]
    Api { status: u16, body: String },
    #[error("network error talking to QuickBooks: {0}")]
    Network(#[from] reqwest::Error),
    #[error("could not parse QuickBooks response: {0}")]
    Parse(String),
    #[error("invoice {0} has no customer linked (person_id and crm_contact_id are both null)")]
    InvoiceMissingCustomer(uuid::Uuid),
}

pub use client::QboClient;
pub use push::push_invoice;
pub use summary::{financial_summary, FinancialSummary};
pub use sync::{run_qbo_sync, SyncStats};
