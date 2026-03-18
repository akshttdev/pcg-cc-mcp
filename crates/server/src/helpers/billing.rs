//! Centralized VIBE billing helpers.
//!
//! Pre-chat balance checks and post-chat usage recording were previously
//! copy-pasted across topsi, nora, agent_chat, and task routes. This module
//! provides two functions that replace all of those copies.

use db::models::project::Project;
use db::models::vibe_deposit::{VibeDeposit, VibeWithdrawal};
use db::models::vibe_transaction::{VibeSourceType, VibeTransaction};
use services::services::vibe_pricing::VibePricingService;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::ApiError;

/// Pre-chat: check VIBE balance via the deposit ledger, returning
/// `Err(ApiError::PaymentRequired)` when the project has no remaining balance.
///
/// Respects the debug-mode bypass toggle from `SystemSettings`.
pub async fn ensure_vibe_balance(pool: &SqlitePool, project_id: Uuid) -> Result<(), ApiError> {
    if crate::helpers::vibe_check::is_vibe_bypass_active(pool).await {
        return Ok(());
    }

    let total_deposited = VibeDeposit::total_deposited(pool, project_id)
        .await
        .unwrap_or(0);
    let total_withdrawn = VibeWithdrawal::total_withdrawn(pool, project_id)
        .await
        .unwrap_or(0);
    let total_spent = VibeTransaction::sum_by_source(pool, VibeSourceType::Project, project_id, None)
        .await
        .map(|s| s.total_vibe)
        .unwrap_or(0);

    let balance = total_deposited - total_withdrawn - total_spent;
    if balance <= 0 {
        return Err(ApiError::PaymentRequired(
            "Insufficient VIBE balance. Deposit VIBE tokens to your project to continue.".into(),
        ));
    }

    Ok(())
}

/// Post-chat: record LLM token usage as a VIBE transaction and update the
/// project's cumulative `vibe_spent` counter.
///
/// Returns the VIBE cost recorded on success, or logs the error and returns 0 —
/// billing failures should never block a response that has already been generated.
pub async fn record_llm_vibe_usage(
    pool: &SqlitePool,
    project_id: Uuid,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    task_id: Option<Uuid>,
    task_attempt_id: Option<Uuid>,
    process_id: Option<Uuid>,
    label: &str,
) -> i64 {
    if input_tokens <= 0 && output_tokens <= 0 {
        return 0;
    }

    let vibe_pricing = VibePricingService::new(pool.clone());
    match vibe_pricing
        .record_llm_usage(
            VibeSourceType::Project,
            project_id,
            model,
            input_tokens,
            output_tokens,
            task_id,
            task_attempt_id,
            process_id,
        )
        .await
    {
        Ok(tx) => {
            if let Err(e) =
                Project::adjust_vibe_spent(pool, &project_id.to_string(), tx.amount_vibe).await
            {
                tracing::error!("[VIBE] Failed to update project spent amount: {}", e);
            }
            tracing::info!(
                "[VIBE] {} recorded {} VIBE for project {}",
                label,
                tx.amount_vibe,
                project_id
            );
            tx.amount_vibe
        }
        Err(e) => {
            tracing::error!("[VIBE] Failed to record {} usage: {}", label, e);
            0
        }
    }
}
