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
                        300, // 5 minutes
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
        struct AccountRow { owner_id: String }
        let nora_account = sqlx::query_as::<_, AccountRow>(
            "SELECT owner_id FROM email_accounts WHERE email_address = 'nora@powerclubglobal.com' AND owner_type = 'agent' AND status != 'revoked' ORDER BY last_sync_at DESC NULLS LAST LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await;

        let nora_id = match nora_account {
            Ok(Some(r)) => match uuid::Uuid::parse_str(&r.owner_id) {
                Ok(id) => id,
                Err(_) => {
                    tracing::warn!("[NORA_INBOX] Could not parse Nora agent UUID '{}' — poller disabled", r.owner_id);
                    return;
                }
            },
            _ => {
                tracing::warn!("[NORA_INBOX] nora@powerclubglobal.com email account not found — poller disabled");
                return;
            }
        };
        let owner = ChannelOwner::Agent(nora_id);

        let mut interval = tokio::time::interval(std::time::Duration::from_secs(180)); // 3 min

        tracing::info!("[NORA_INBOX] Poller started — checking nora@powerclubglobal.com every 3 min");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match svc.read_inbox(&owner, 20).await {
                        Err(e) => tracing::warn!("[NORA_INBOX] read_inbox failed: {}", e),
                        Ok(messages) => {
                            for msg in messages {
                                let msg_id = msg.message_id.clone();

                                // Only process emails from trusted intake senders.
                                // Currently: sirakstudios.com — Sirak shares discovery calls with Nora
                                // as a PCG client service. Other senders (marketing, notifications)
                                // are not discovery leads.
                                let sender_domain = msg.from_address
                                    .split('@')
                                    .nth(1)
                                    .unwrap_or("")
                                    .to_lowercase();
                                let is_trusted = sender_domain == "sirakstudios.com";
                                if !is_trusted {
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
                                        raw_content: Some(full_body),
                                        subject: Some(msg.subject.clone()),
                                        from_email: Some(msg.from_address.clone()),
                                        from_name: None,
                                        call_date: None,
                                        duration_seconds: None,
                                        metadata: Some(serde_json::json!({
                                            "message_id": msg_id,
                                            "ingested_by": "nora_inbox_poller",
                                            "organization_id": org_id.map(|id| id.to_string()),
                                        }).to_string()),
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
