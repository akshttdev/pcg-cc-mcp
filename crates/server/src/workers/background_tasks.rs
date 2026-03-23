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
