//! Provider-agnostic email transport.
//!
//! Each provider (Gmail, Zoho, future: Outlook, IMAP) implements the same
//! shape: refresh-token → list new message ids since cursor → fetch full
//! payloads → send a new message. Higher layers (the EmailSyncWorker, the
//! `/email/send` route, AgentChannelService) compose against these clients
//! without caring which backend serves the account.
//!
//! Pure HTTP — no DB writes here. The caller decides what to persist.

pub mod gmail;
pub mod normalized;
pub mod zoho;

pub use gmail::{GmailClient, GmailError};
pub use normalized::{NormalizedAttachment, NormalizedMessage};
pub use zoho::{MessageSummary as ZohoMessageSummary, ZohoAccountInfo, ZohoClient, ZohoError};
