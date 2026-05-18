//! Background task workers migrated from bare tokio::spawn loops in main.rs.
//!
//! Each implements BackgroundWorker for graceful shutdown support.

use tokio_util::sync::CancellationToken;

use super::BackgroundWorker;

/// VIBE deposit watcher — polls platform revenue wallet for incoming transfers.
pub struct VibeDepositWatcher {
    pool: sqlx::SqlitePool,
}

impl VibeDepositWatcher {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for VibeDepositWatcher {
    fn name(&self) -> &str {
        "vibe_deposit_watcher"
    }

    async fn run(&self, shutdown: CancellationToken) {
        let revenue_addr = match std::env::var("PLATFORM_REVENUE_ADDRESS") {
            Ok(a) if !a.is_empty() => a,
            _ => {
                tracing::warn!("[VIBE] PLATFORM_REVENUE_ADDRESS not set; deposit watcher disabled");
                return;
            }
        };

        let aptos = services::services::aptos::AptosService::testnet();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        tracing::info!(
            "[VIBE] Deposit watcher started for revenue address {}",
            &revenue_addr[..10.min(revenue_addr.len())]
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match aptos.get_transactions(&revenue_addr, Some(25)).await {
                        Ok(txns) => {
                            for tx in txns.iter().filter(|t| t.success) {
                                match db::models::vibe_deposit::VibeDeposit::find_by_tx_hash(
                                    &self.pool,
                                    &tx.hash,
                                )
                                .await
                                {
                                    Ok(Some(_)) => {} // already recorded
                                    _ => {
                                        tracing::info!(
                                            "[VIBE] Detected transfer to revenue wallet: hash={}, sender={}",
                                            tx.hash,
                                            tx.sender
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => tracing::error!("[VIBE] Deposit watcher error: {e}"),
                    }
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[VIBE] Deposit watcher shutting down");
                    break;
                }
            }
        }
    }
}

/// VIBE withdrawal executor — processes pending withdrawal requests.
pub struct VibeWithdrawalExecutor {
    pool: sqlx::SqlitePool,
}

impl VibeWithdrawalExecutor {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for VibeWithdrawalExecutor {
    fn name(&self) -> &str {
        "vibe_withdrawal_executor"
    }

    async fn run(&self, shutdown: CancellationToken) {
        let private_key = match std::env::var("PLATFORM_REVENUE_PRIVATE_KEY") {
            Ok(k) if !k.is_empty() => k,
            _ => {
                tracing::warn!(
                    "[VIBE] PLATFORM_REVENUE_PRIVATE_KEY not set; withdrawal executor disabled"
                );
                return;
            }
        };
        let revenue_addr = match std::env::var("PLATFORM_REVENUE_ADDRESS") {
            Ok(a) if !a.is_empty() => a,
            _ => {
                tracing::warn!(
                    "[VIBE] PLATFORM_REVENUE_ADDRESS not set; withdrawal executor disabled"
                );
                return;
            }
        };

        let aptos = services::services::aptos::AptosService::testnet();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        tracing::info!("[VIBE] Withdrawal executor started");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let pending = match db::models::vibe_deposit::VibeWithdrawal::list_pending(
                        &self.pool,
                        5,
                    )
                    .await
                    {
                        Ok(p) => p,
                        Err(e) => {
                            tracing::error!("[VIBE] Failed to list pending withdrawals: {e}");
                            continue;
                        }
                    };
                    for withdrawal in pending {
                        let _ = db::models::vibe_deposit::VibeWithdrawal::mark_processing(
                            &self.pool,
                            withdrawal.id,
                        )
                        .await;
                        match aptos
                            .transfer_vibe(
                                &private_key,
                                &revenue_addr,
                                &withdrawal.destination_address,
                                withdrawal.amount_vibe as u64,
                            )
                            .await
                        {
                            Ok(resp) if resp.success => {
                                let _ = db::models::vibe_deposit::VibeWithdrawal::mark_completed(
                                    &self.pool,
                                    withdrawal.id,
                                    &resp.tx_hash,
                                )
                                .await;
                                tracing::info!(
                                    "[VIBE] Withdrawal {} completed: tx={}",
                                    withdrawal.id,
                                    resp.tx_hash
                                );
                            }
                            Ok(resp) => {
                                let _ = db::models::vibe_deposit::VibeWithdrawal::mark_failed(
                                    &self.pool,
                                    withdrawal.id,
                                    &resp.message,
                                )
                                .await;
                            }
                            Err(e) => {
                                let _ = db::models::vibe_deposit::VibeWithdrawal::mark_failed(
                                    &self.pool,
                                    withdrawal.id,
                                    &e.to_string(),
                                )
                                .await;
                            }
                        }
                    }
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[VIBE] Withdrawal executor shutting down");
                    break;
                }
            }
        }
    }
}

/// APN peer cleanup — deduplicates and marks stale peers inactive.
pub struct ApnPeerCleanup {
    pool: sqlx::SqlitePool,
}

impl ApnPeerCleanup {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for ApnPeerCleanup {
    fn name(&self) -> &str {
        "apn_peer_cleanup"
    }

    async fn run(&self, shutdown: CancellationToken) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match db::models::peer_node::PeerNode::cleanup_duplicates(&self.pool).await {
                        Ok(count) if count > 0 => {
                            tracing::info!("APN: Marked {} duplicate peers as inactive", count);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("APN: Failed to cleanup duplicates: {}", e);
                        }
                    }
                    match db::models::peer_node::PeerNode::mark_stale_inactive(&self.pool).await {
                        Ok(count) if count > 0 => {
                            tracing::info!("APN: Marked {} stale peers as inactive", count);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("APN: Failed to mark stale peers: {}", e);
                        }
                    }
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[APN] Peer cleanup shutting down");
                    break;
                }
            }
        }
    }
}

/// Meeting stale-session cleanup — auto-ends sessions with no heartbeat.
pub struct MeetingSessionCleanup {
    pool: sqlx::SqlitePool,
}

impl MeetingSessionCleanup {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for MeetingSessionCleanup {
    fn name(&self) -> &str {
        "meeting_session_cleanup"
    }

    async fn run(&self, shutdown: CancellationToken) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(120));
        tracing::info!("[MEETING] Stale-session cleanup started (5-min timeout)");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let stale = match db::models::meeting_session::MeetingSession::find_stale_active(
                        &self.pool,
                        1800, // 30 minutes
                    )
                    .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("[MEETING] Failed to query stale sessions: {e}");
                            continue;
                        }
                    };
                    for session in stale {
                        match sqlx::query(
                            r#"UPDATE meeting_sessions
                               SET status = 'ended',
                                   ended_at = datetime('now','subsec'),
                                   duration_seconds = CAST(unixepoch('now') - unixepoch(started_at) AS INTEGER),
                                   updated_at = datetime('now','subsec')
                               WHERE id = ?"#,
                        )
                        .bind(&session.id)
                        .execute(&self.pool)
                        .await
                        {
                            Ok(_) => tracing::info!(
                                "[MEETING] Auto-ended stale session {} (project={})",
                                session.id,
                                session.project_id
                            ),
                            Err(e) => tracing::error!(
                                "[MEETING] Failed to auto-end session {}: {e}",
                                session.id
                            ),
                        }
                    }
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[MEETING] Session cleanup shutting down");
                    break;
                }
            }
        }
    }
}

/// Nora inbox poller — checks nora@powerclubglobal.com every 3 minutes.
/// Any unprocessed email is pushed through the call-intake pipeline.
/// This is a PCG platform service: Nora monitors on behalf of all org clients
/// (e.g. Sirak Studios), routing via resolve_org_and_assignee.
pub struct NoraInboxPoller {
    pool: sqlx::SqlitePool,
}

impl NoraInboxPoller {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for NoraInboxPoller {
    fn name(&self) -> &str {
        "nora_inbox_poller"
    }

    async fn run(&self, shutdown: CancellationToken) {
        use std::sync::Arc;

        use services::services::agent_channels::{AgentChannelService, ChannelOwner};

        let svc = Arc::new(AgentChannelService::new(self.pool.clone()));

        // Look up Nora's agent owner_id from her email account record.
        // owner_id is stored as a 32-char hex UUID string (no dashes).
        #[derive(sqlx::FromRow)]
        struct AccountRow {
            owner_id: String,
        }
        let nora_account = sqlx::query_as::<_, AccountRow>(
            "SELECT owner_id FROM email_accounts WHERE email_address = 'nora@powerclubglobal.com' AND owner_type = 'agent' AND status != 'revoked' ORDER BY last_sync_at DESC NULLS LAST LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await;

        let nora_id = match nora_account {
            Ok(Some(r)) => match uuid::Uuid::parse_str(&r.owner_id) {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!(
                        "[NORA_INBOX] Could not parse Nora agent UUID '{}' — poller disabled",
                        r.owner_id
                    );
                    return;
                }
            },
            _ => {
                tracing::warn!(
                    "[NORA_INBOX] nora@powerclubglobal.com email account not found — poller disabled"
                );
                return;
            }
        };
        let owner = ChannelOwner::Agent(nora_id);

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(180)); // 3 min

        tracing::info!(
            "[NORA_INBOX] Poller started — checking nora@powerclubglobal.com every 3 min"
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match svc.read_inbox(&owner, 20).await {
                        Err(e) => tracing::warn!("[NORA_INBOX] read_inbox failed: {}", e),
                        Ok(messages) => {
                            for msg in messages {
                                let msg_id = msg.message_id.clone();

                                // Only process emails from trusted intake senders.
                                // Configured via EMAIL_TRUSTED_SENDERS env var:
                                //   comma-separated list of full addresses (alice@x.com)
                                //   or domains (sirakstudios.com).
                                // Default if unset: "sirakstudios.com" (preserves Nora's
                                // PCG-client behavior pre-env-var).
                                if !is_trusted_sender(&msg.from_address) {
                                    continue;
                                }

                                // Dedup: skip if already ingested (source_ref match)
                                let already: bool = sqlx::query_scalar(
                                    "SELECT COUNT(*) > 0 FROM call_intake_items WHERE source_ref = ?"
                                )
                                .bind(&msg_id)
                                .fetch_one(&self.pool)
                                .await
                                .unwrap_or(false);

                                if already {
                                    continue;
                                }

                                // Fetch full message body (inbox API returns summary only)
                                let full_body = match svc.fetch_message_body(&owner, &msg_id).await {
                                    Ok(b) if !b.trim().is_empty() => b,
                                    Ok(_) => {
                                        tracing::warn!("[NORA_INBOX] Empty body for {}, using summary", msg_id);
                                        msg.summary.clone()
                                    }
                                    Err(e) => {
                                        tracing::warn!("[NORA_INBOX] Could not fetch body for {}: {}", msg_id, e);
                                        msg.summary.clone()
                                    }
                                };

                                // Classify the email before deciding how to handle it
                                use crate::routes::intake::report::{EmailClass, classify_email};
                                let class = classify_email(&msg.subject, &full_body).await;

                                tracing::info!(
                                    "[NORA_INBOX] Email from {} — subject: {:?} — class: {:?}",
                                    msg.from_address, msg.subject, class
                                );

                                match class {
                                    EmailClass::DiscoveryCall => {
                                        // Fall through to create intake item + run pipeline
                                    }
                                    EmailClass::OngoingClient => {
                                        tracing::info!("[NORA_INBOX] Ongoing client email — logged, no new pipeline");
                                        continue;
                                    }
                                    EmailClass::Other => {
                                        tracing::info!("[NORA_INBOX] Non-actionable email — skipped");
                                        continue;
                                    }
                                }

                                tracing::info!(
                                    "[NORA_INBOX] Discovery call detected — routing to intake pipeline"
                                );

                                // Resolve org/assignee from sender
                                let (org_id, assigned) = crate::routes::intake::resolve_org_and_assignee(
                                    &self.pool,
                                    Some(msg.from_address.as_str()),
                                    None,
                                    None,
                                )
                                .await;

                                // Create intake item
                                use db::models::call_intake_item::{CallIntakeItem, CreateCallIntakeItem};
                                let item = CallIntakeItem::create(
                                    &self.pool,
                                    CreateCallIntakeItem {
                                        source_type: "email".into(),
                                        source_ref: Some(msg_id.clone()),
                                        subject: Some(msg.subject.clone()),
                                        from_email: Some(msg.from_address.clone()),
                                        from_name: None,
                                        call_date: None,
                                        duration_seconds: None,
                                        metadata: Some(serde_json::json!({
                                            "message_id": msg_id,
                                            "ingested_by": "nora_inbox_poller",
                                            "organization_id": org_id.map(|id| id.to_string()),
                                            "source_links": extract_source_links(&full_body),
                                        }).to_string()),
                                        raw_content: Some(full_body),
                                    },
                                )
                                .await;

                                match item {
                                    Err(e) => tracing::error!("[NORA_INBOX] Failed to create intake item: {}", e),
                                    Ok(item) => {
                                        let pool2 = self.pool.clone();
                                        tokio::spawn(async move {
                                            if let Err(e) = crate::routes::intake::pipeline::run_intake_pipeline(
                                                pool2, item.id, org_id, assigned,
                                            )
                                            .await
                                            {
                                                tracing::error!("[NORA_INBOX] Pipeline failed for {}: {}", item.id, e);
                                            }
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[NORA_INBOX] Poller shutting down");
                    break;
                }
            }
        }
    }
}

/// EmailSyncWorker — pulls new messages from every active email account on a
/// short cadence and writes them into `email_messages`. Trusted-sender mail
/// is also handed off to the call-intake pipeline.
///
/// Today this implements Gmail (history-API delta, falling back to a 50-message
/// bootstrap on first run). Zoho accounts are skipped — `NoraInboxPoller`
/// covers the one Zoho address we care about; a full Zoho sync can layer on
/// later via the same dispatch shape.
pub struct EmailSyncWorker {
    pool: sqlx::SqlitePool,
}

impl EmailSyncWorker {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl BackgroundWorker for EmailSyncWorker {
    fn name(&self) -> &str {
        "email_sync_worker"
    }

    async fn run(&self, shutdown: CancellationToken) {
        // Tick every 60s; per-account `sync_frequency_minutes` decides whether
        // a given account is actually due.
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        tracing::info!(
            "[EMAIL_SYNC] Worker started — 60s tick, gates on per-account sync_frequency_minutes"
        );

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.sync_due_accounts().await {
                        tracing::error!("[EMAIL_SYNC] tick failed: {e}");
                    }
                }
                _ = shutdown.cancelled() => {
                    tracing::info!("[EMAIL_SYNC] Worker shutting down");
                    break;
                }
            }
        }
    }
}

impl EmailSyncWorker {
    async fn sync_due_accounts(&self) -> Result<(), sqlx::Error> {
        use db::models::email_account::EmailAccount;

        let accounts = match EmailAccount::find_needs_sync(&self.pool).await {
            Ok(a) => a,
            Err(e) => {
                tracing::error!("[EMAIL_SYNC] find_needs_sync: {e}");
                return Ok(());
            }
        };

        for account in accounts {
            sync_account_with_claim(self.pool.clone(), account).await;
        }

        Ok(())
    }
}

/// Run one full sync pass for a specific account, claiming the row first so
/// concurrent ticks/manual triggers don't double-sync. Used by both the
/// background ticker and the manual `POST /email/accounts/:id/sync` route.
pub async fn sync_account_now(
    pool: sqlx::SqlitePool,
    account_id: uuid::Uuid,
) -> Result<(), String> {
    use db::models::email_account::EmailAccount;
    let account = EmailAccount::find_by_id(&pool, account_id)
        .await
        .map_err(|e| format!("account lookup: {e}"))?;
    sync_account_with_claim(pool, account).await;
    Ok(())
}

async fn sync_account_with_claim(
    pool: sqlx::SqlitePool,
    account: db::models::email_account::EmailAccount,
) {
    use db::models::email_account::EmailAccount;
    let claimed = EmailAccount::try_claim_for_sync(&pool, account.id)
        .await
        .unwrap_or(false);
    if !claimed {
        return;
    }

    let result = match account.provider.as_str() {
        "gmail" => sync_gmail_account(pool.clone(), &account).await,
        "zoho" => sync_zoho_account(pool.clone(), &account).await,
        other => Err(format!("provider '{other}' sync not yet implemented")),
    };

    match result {
        Ok(outcome) => {
            tracing::info!(
                "[EMAIL_SYNC] {} ({}): synced {} new, {} routed to intake",
                account.email_address,
                account.provider,
                outcome.imported,
                outcome.intake_routed,
            );
            let _ = EmailAccount::finish_sync(
                &pool,
                account.id,
                outcome.gmail_history_id.as_deref(),
                outcome.sync_cursor.as_deref(),
                None,
            )
            .await;
        }
        Err(err) => {
            tracing::warn!(
                "[EMAIL_SYNC] {} ({}) sync failed: {err}",
                account.email_address,
                account.provider
            );
            let _ = EmailAccount::finish_sync(&pool, account.id, None, None, Some(&err)).await;
        }
    }
}

#[derive(Default)]
struct SyncOutcome {
    imported: usize,
    intake_routed: usize,
    gmail_history_id: Option<String>,
    sync_cursor: Option<String>,
}

async fn sync_gmail_account(
    pool: sqlx::SqlitePool,
    account: &db::models::email_account::EmailAccount,
) -> Result<SyncOutcome, String> {
    use services::services::{agent_channels::AgentChannelService, email_providers::GmailClient};

    let svc = AgentChannelService::new(pool.clone());

    // Refresh-or-reuse token; persists any new value.
    let token = svc
        .valid_gmail_token(account)
        .await
        .map_err(|e| format!("gmail token: {e}"))?;
    let client = GmailClient::new(token);

    let (message_ids, latest_history): (Vec<String>, Option<String>) =
        match account.gmail_history_id.as_deref() {
            Some(cursor) => match client.list_history_since(cursor).await {
                Ok((ids, latest)) => (ids, latest),
                Err(e) => {
                    // History cursor older than ~7 days returns 404; bootstrap fresh.
                    tracing::warn!(
                        "[EMAIL_SYNC] history-since failed for {} ({}); bootstrapping. err={e}",
                        account.email_address,
                        cursor,
                    );
                    bootstrap_gmail(&client).await?
                }
            },
            None => bootstrap_gmail(&client).await?,
        };

    let mut imported = 0usize;
    let mut intake_routed = 0usize;

    for mid in message_ids {
        match client.get_message(&mid).await {
            Err(e) => tracing::warn!("[EMAIL_SYNC] get_message {mid} failed: {e}"),
            Ok(msg) => {
                let from_address = msg.from_address.clone();
                let subject = msg.subject.clone().unwrap_or_default();
                let body_for_intake = msg.body_text.clone().or_else(|| msg.body_html.clone());

                if let Some(stored) = persist_normalized(&pool, account, &msg).await? {
                    imported += 1;
                    if is_trusted_sender(&from_address) {
                        let body = body_for_intake.unwrap_or_default();
                        if !body.trim().is_empty() {
                            route_to_intake(
                                pool.clone(),
                                stored.provider_message_id.clone(),
                                from_address,
                                subject,
                                body,
                            )
                            .await;
                            intake_routed += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(SyncOutcome {
        imported,
        intake_routed,
        gmail_history_id: latest_history,
        sync_cursor: None,
    })
}

/// Zoho sync: list inbox newest-first, take everything past `sync_cursor`
/// (stored as the epoch-millis `receivedTime` of the highest imported
/// message), fetch each one, persist, and route trusted-sender mail to intake.
///
/// On first run (no cursor), we cap the bootstrap at 50 messages — same as
/// Gmail — to keep the first tick bounded. Subsequent ticks paginate as
/// needed in 100-message pages.
async fn sync_zoho_account(
    pool: sqlx::SqlitePool,
    account: &db::models::email_account::EmailAccount,
) -> Result<SyncOutcome, String> {
    use services::services::email_providers::{ZohoClient, ZohoError};

    let token = valid_zoho_access_token(&pool, account)
        .await
        .map_err(|e| format!("zoho token: {e}"))?;
    let (domain, account_id) = zoho_domain_and_account_id(account)?;
    let client = ZohoClient::new(token, &domain, &account_id);

    let folder_id = client
        .inbox_folder_id()
        .await
        .map_err(|e: ZohoError| format!("zoho inbox folder: {e}"))?;

    // Parse cursor as epoch millis. None = bootstrap.
    let cursor_ms = account
        .sync_cursor
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok());

    let page_limit = if cursor_ms.is_some() { 100 } else { 50 };
    let summaries = client
        .list_inbox(page_limit)
        .await
        .map_err(|e| format!("zoho list inbox: {e}"))?;

    // Zoho returns newest first. Filter to messages strictly newer than cursor
    // (skip the cursor itself to avoid re-importing the boundary message).
    let new_summaries: Vec<_> = match cursor_ms {
        Some(cur) => summaries
            .into_iter()
            .filter(|s| {
                s.received_time
                    .as_deref()
                    .and_then(|t| t.parse::<i64>().ok())
                    .map(|ms| ms > cur)
                    .unwrap_or(false)
            })
            .collect(),
        None => summaries,
    };

    let next_cursor_ms = new_summaries
        .iter()
        .filter_map(|s| {
            s.received_time
                .as_deref()
                .and_then(|t| t.parse::<i64>().ok())
        })
        .max();

    let mut imported = 0usize;
    let mut intake_routed = 0usize;

    // Oldest-first so the next cursor is monotonic if we crash mid-batch.
    for summary in new_summaries.into_iter().rev() {
        let mid = summary.message_id.clone();
        match client.get_message(&folder_id, &mid).await {
            Err(e) => tracing::warn!("[EMAIL_SYNC] zoho get_message {mid} failed: {e}"),
            Ok(msg) => {
                let from_address = msg.from_address.clone();
                let subject = msg.subject.clone().unwrap_or_default();
                let body_for_intake = msg.body_text.clone().or_else(|| msg.body_html.clone());

                if let Some(stored) = persist_normalized(&pool, account, &msg).await? {
                    imported += 1;
                    if is_trusted_sender(&from_address) {
                        let body = body_for_intake.unwrap_or_default();
                        if !body.trim().is_empty() {
                            route_to_intake(
                                pool.clone(),
                                stored.provider_message_id.clone(),
                                from_address,
                                subject,
                                body,
                            )
                            .await;
                            intake_routed += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(SyncOutcome {
        imported,
        intake_routed,
        gmail_history_id: None,
        sync_cursor: next_cursor_ms
            .map(|ms| ms.to_string())
            .or_else(|| account.sync_cursor.clone()),
    })
}

/// Pull Zoho TLD and account id out of `email_accounts.metadata`. Both are
/// stamped at OAuth-completion time in `routes::email_accounts::zoho_oauth_callback`.
fn zoho_domain_and_account_id(
    account: &db::models::email_account::EmailAccount,
) -> Result<(String, String), String> {
    let meta = account
        .metadata
        .as_deref()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
        .ok_or_else(|| "zoho account has no metadata".to_string())?;
    let domain = meta["zoho_domain"]
        .as_str()
        .map(String::from)
        .unwrap_or_else(|| "com".to_string());
    let account_id = meta["zoho_account_id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| "zoho_account_id missing from account metadata".to_string())?;
    Ok((domain, account_id))
}

/// Refresh-or-reuse the Zoho access token, persisting any new value to the row.
/// Mirrors `AgentChannelService::valid_access_token` but lives at the worker
/// layer to keep the provider client free of DB writes.
async fn valid_zoho_access_token(
    pool: &sqlx::SqlitePool,
    account: &db::models::email_account::EmailAccount,
) -> Result<String, String> {
    use services::services::email_providers::ZohoClient;

    if let (Some(token), Some(expires_at)) =
        (account.access_token.as_ref(), account.token_expires_at)
    {
        if chrono::Utc::now() + chrono::Duration::seconds(60) < expires_at {
            return Ok(token.clone());
        }
    }

    let refresh_token = account
        .refresh_token
        .as_deref()
        .ok_or_else(|| "zoho account has no refresh token".to_string())?;
    let (domain, _) = zoho_domain_and_account_id(account)?;

    let refreshed = ZohoClient::refresh_access_token(refresh_token, &domain)
        .await
        .map_err(|e| format!("zoho refresh: {e}"))?;

    sqlx::query(
        "UPDATE email_accounts SET access_token = ?, token_expires_at = ?, status = 'active', last_error = NULL, updated_at = datetime('now','subsec') WHERE id = ?",
    )
    .bind(&refreshed.access_token)
    .bind(refreshed.expires_at.map(|t| t.to_rfc3339()))
    .bind(account.id)
    .execute(pool)
    .await
    .map_err(|e| format!("persist refreshed zoho token: {e}"))?;

    Ok(refreshed.access_token)
}

async fn bootstrap_gmail(
    client: &services::services::email_providers::GmailClient,
) -> Result<(Vec<String>, Option<String>), String> {
    let ids = client
        .list_inbox_ids(50)
        .await
        .map_err(|e| format!("gmail bootstrap list: {e}"))?;
    let history_id = client
        .get_history_id()
        .await
        .map_err(|e| format!("gmail profile: {e}"))?;
    Ok((ids, Some(history_id)))
}

async fn persist_normalized(
    pool: &sqlx::SqlitePool,
    account: &db::models::email_account::EmailAccount,
    msg: &services::services::email_providers::NormalizedMessage,
) -> Result<Option<db::models::email_message::EmailMessage>, String> {
    use db::models::email_message::{CreateEmailMessage, EmailMessage};

    let Some(project_id) = account.project_id else {
        // Org/agent/user-scoped accounts have no project context — the
        // email_messages table requires a project_id FK, so skip body persistence
        // for now. The OAuth account row + token still exist; per-account
        // message storage for these scopes is a separate feature.
        return Ok(None);
    };

    let attachments_json = if msg.attachments.is_empty() {
        None
    } else {
        Some(serde_json::json!(msg
            .attachments
            .iter()
            .map(|a| serde_json::json!({
                "provider_attachment_id": a.provider_attachment_id,
                "filename": a.filename,
                "content_type": a.content_type,
                "size_bytes": a.size_bytes,
                "content_id": a.content_id,
            }))
            .collect::<Vec<_>>()))
    };

    let row = EmailMessage::insert_if_absent(
        pool,
        CreateEmailMessage {
            email_account_id: account.id,
            project_id,
            provider_message_id: msg.provider_message_id.clone(),
            thread_id: msg.thread_id.clone(),
            from_address: msg.from_address.clone(),
            from_name: msg.from_name.clone(),
            to_addresses: msg.to_addresses.clone(),
            cc_addresses: Some(msg.cc_addresses.clone()).filter(|v| !v.is_empty()),
            bcc_addresses: Some(msg.bcc_addresses.clone()).filter(|v| !v.is_empty()),
            reply_to: msg.reply_to.clone(),
            subject: msg.subject.clone(),
            body_text: msg.body_text.clone(),
            body_html: msg.body_html.clone(),
            snippet: msg.snippet.clone(),
            has_attachments: !msg.attachments.is_empty(),
            attachments: attachments_json,
            labels: Some(msg.labels.clone()).filter(|v| !v.is_empty()),
            is_read: msg.is_read,
            is_starred: msg.is_starred,
            is_draft: msg.is_draft,
            is_sent: msg.is_sent,
            received_at: msg.received_at,
            sent_at: msg.sent_at,
        },
    )
    .await
    .map_err(|e| format!("persist email_message: {e}"))?;

    Ok(row)
}

async fn route_to_intake(
    pool: sqlx::SqlitePool,
    message_id: String,
    from_address: String,
    subject: String,
    body: String,
) {
    use db::models::call_intake_item::{CallIntakeItem, CreateCallIntakeItem};

    // Skip if we've already ingested this provider message_id.
    let already: bool =
        sqlx::query_scalar("SELECT COUNT(*) > 0 FROM call_intake_items WHERE source_ref = ?")
            .bind(&message_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
    if already {
        return;
    }

    let (org_id, assigned) =
        crate::routes::intake::resolve_org_and_assignee(&pool, Some(&from_address), None, None)
            .await;

    let item = CallIntakeItem::create(
        &pool,
        CreateCallIntakeItem {
            source_type: "email".into(),
            source_ref: Some(message_id.clone()),
            subject: Some(subject),
            from_email: Some(from_address),
            from_name: None,
            call_date: None,
            duration_seconds: None,
            metadata: Some(
                serde_json::json!({
                    "message_id": message_id,
                    "ingested_by": "email_sync_worker",
                    "organization_id": org_id.map(|id| id.to_string()),
                    "source_links": extract_source_links(&body),
                })
                .to_string(),
            ),
            raw_content: Some(body),
        },
    )
    .await;

    match item {
        Err(e) => tracing::error!("[EMAIL_SYNC] failed to create intake item: {e}"),
        Ok(item) => {
            let pool2 = pool;
            tokio::spawn(async move {
                if let Err(e) = crate::routes::intake::pipeline::run_intake_pipeline(
                    pool2, item.id, org_id, assigned,
                )
                .await
                {
                    tracing::error!("[EMAIL_SYNC] pipeline failed for {}: {e}", item.id);
                }
            });
        }
    }
}

/// Returns true if the sender is in the EMAIL_TRUSTED_SENDERS allowlist.
/// Allowlist accepts either full email addresses (`alice@x.com`) or bare
/// domains (`sirakstudios.com`); matching is case-insensitive.
/// Defaults to `sirakstudios.com` when the env var is unset (preserves
/// the original hardcoded Nora behavior).
pub fn is_trusted_sender(from_address: &str) -> bool {
    let allowlist =
        std::env::var("EMAIL_TRUSTED_SENDERS").unwrap_or_else(|_| "sirakstudios.com".to_string());
    let from_lower = from_address.to_lowercase();
    let domain = from_lower.split('@').nth(1).unwrap_or("");

    allowlist
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .any(|entry| {
            if entry.contains('@') {
                from_lower == entry
            } else {
                domain == entry
            }
        })
}

/// Extract known transcript/document source links from email body.
/// Returns a JSON array of {type, url, label} objects.
fn extract_source_links(body: &str) -> Vec<serde_json::Value> {
    let mut links = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.contains("fireflies.ai/view/") {
            // Extract the URL
            if let Some(start) = trimmed.find("https://app.fireflies.ai") {
                let url = &trimmed[start..];
                let url = url.split_whitespace().next().unwrap_or(url);
                links.push(serde_json::json!({
                    "type": "fireflies_transcript",
                    "label": "Fireflies Transcript",
                    "url": url
                }));
            }
        } else if trimmed.contains("docs.google.com/document/") {
            if let Some(start) = trimmed.find("https://docs.google.com") {
                let url = &trimmed[start..];
                let url = url.split_whitespace().next().unwrap_or(url);
                links.push(serde_json::json!({
                    "type": "google_doc",
                    "label": "Meeting Notes (Google Doc)",
                    "url": url
                }));
            }
        } else if trimmed.contains("otter.ai/") {
            if let Some(start) = trimmed.find("https://otter.ai") {
                let url = &trimmed[start..];
                let url = url.split_whitespace().next().unwrap_or(url);
                links.push(serde_json::json!({
                    "type": "otter_transcript",
                    "label": "Otter.ai Transcript",
                    "url": url
                }));
            }
        }
    }
    links
}
