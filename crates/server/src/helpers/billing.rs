//! Centralized VIBE billing helpers.
//!
//! Pre-chat balance checks and post-chat usage recording were previously
//! copy-pasted across topsi, nora, agent_chat, and task routes. This module
//! provides functions that replace all of those copies, plus an
//! organization-scoped set used by the avatar engine pipelines (pre-debit
//! estimate → settle on completion → refund on failure).

use db::models::{
    org_member_vibe_limit::OrgMemberVibeLimit,
    project::Project,
    vibe_deposit::{VibeDeposit, VibeWithdrawal},
    vibe_transaction::{VibeSourceType, VibeTransaction},
};
use serde_json::json;
use services::services::vibe_pricing::VibePricingService;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::ApiError;

/// Check whether the client linked to a project has exceeded its VIBE budget.
/// Returns `Err(ApiError::BadRequest(...))` if the budget is exceeded.
/// Returns `Ok(())` if there is no client, no budget set, or budget not yet exceeded.
pub async fn check_client_vibe_budget(pool: &SqlitePool, project_id: Uuid) -> Result<(), ApiError> {
    // Look up the project's client_id
    let client_id: Option<String> =
        sqlx::query_scalar("SELECT client_id FROM projects WHERE id = ?")
            .bind(project_id.to_string())
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .flatten();

    let Some(cid) = client_id else {
        return Ok(());
    };

    // Look up vibe_budget_limit and vibe_spent_amount on the client
    let row: Option<(Option<f64>, f64)> =
        sqlx::query_as("SELECT vibe_budget_limit, vibe_spent_amount FROM clients WHERE id = ?")
            .bind(&cid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

    if let Some((Some(limit), spent)) = row {
        if spent >= limit {
            return Err(ApiError::BadRequest("Client VIBE budget exceeded".into()));
        }
    }

    Ok(())
}

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
    let total_spent =
        VibeTransaction::sum_by_source(pool, VibeSourceType::Project, project_id, None)
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
            // Roll up spend to the client if one is linked to the project
            let client_id: Option<String> =
                sqlx::query_scalar("SELECT client_id FROM projects WHERE id = ?")
                    .bind(project_id.to_string())
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten()
                    .flatten();
            if let Some(cid) = client_id {
                let vibe_as_f64 = tx.amount_vibe as f64;
                if let Err(e) = sqlx::query(
                    "UPDATE clients SET vibe_spent_amount = vibe_spent_amount + ? WHERE id = ?",
                )
                .bind(vibe_as_f64)
                .bind(&cid)
                .execute(pool)
                .await
                {
                    tracing::error!("[VIBE] Failed to update client vibe_spent_amount: {}", e);
                }
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

// ─── Organization-scoped billing (avatar engine pipelines) ────────────────────

/// Atomically check + debit an organization's VIBE balance for an estimated
/// pipeline cost.
///
/// Three guards apply in order:
///   1. **vibe-bypass** is honored — short-circuits with a sentinel transaction
///      so callers' `settle`/`refund` paths still work
///   2. **Per-user cap** — if `org_member_vibe_limits` has a row for this
///      (org, user) with `monthly_limit_vibe IS NOT NULL`, bump the user's
///      `current_period_spent` only if it stays within the cap. Returns
///      `ApiError::Forbidden` when exceeded
///   3. **Org balance** — atomic `UPDATE` on `organizations.vibe_balance`
///      with `WHERE vibe_balance >= ?`. Returns `ApiError::PaymentRequired`
///      when insufficient
///
/// On success, inserts a `vibe_transactions` row with `source_type='organization'`,
/// `source_id=org_id`, `amount_vibe=estimated_vibe`, and metadata recording
/// `{user_id, linked_avatar_id, status: "pending"}`. The returned `charge_id`
/// is the new transaction's `id`; callers pass it into [`settle_org_vibe_charge`]
/// or [`refund_org_vibe_charge`] when the pipeline finishes.
pub async fn ensure_org_vibe_balance_and_debit(
    pool: &SqlitePool,
    org_id: &str,
    user_id_str: &str,
    estimated_vibe: i64,
    linked_avatar_id: Option<Uuid>,
    description: &str,
) -> Result<Uuid, ApiError> {
    if crate::helpers::vibe_check::is_vibe_bypass_active(pool).await {
        // Record a bypass row so settle/refund are no-ops with a real id.
        return record_org_charge_row(
            pool,
            org_id,
            user_id_str,
            0,
            linked_avatar_id,
            &format!("BYPASS: {description}"),
            "bypass",
        )
        .await;
    }

    if estimated_vibe <= 0 {
        return Err(ApiError::BadRequest(
            "estimated VIBE cost must be positive".into(),
        ));
    }

    // 1. Per-user cap — only enforced if a row exists for this user. We do NOT
    //    auto-create rows; "no row" means "no cap" and the user is bound only
    //    by the org's balance.
    let existing_cap = OrgMemberVibeLimit::find(pool, org_id, user_id_str)
        .await
        .map_err(ApiError::Database)?;
    if existing_cap.is_some() {
        let bumped = OrgMemberVibeLimit::try_bump_spent(pool, org_id, user_id_str, estimated_vibe)
            .await
            .map_err(ApiError::Database)?;
        if !bumped {
            return Err(ApiError::Forbidden(format!(
                "monthly VIBE spend limit exceeded for this user in org {org_id}; ask your org admin"
            )));
        }
    }

    // 2. Org balance — atomic conditional debit.
    let result = sqlx::query(
        "UPDATE organizations
         SET vibe_balance = vibe_balance - ?,
             updated_at = datetime('now','subsec')
         WHERE id = ? AND vibe_balance >= ?",
    )
    .bind(estimated_vibe as f64)
    .bind(org_id)
    .bind(estimated_vibe as f64)
    .execute(pool)
    .await
    .map_err(ApiError::Database)?;

    if result.rows_affected() == 0 {
        // Roll back the per-user cap bump we just made.
        if existing_cap.is_some() {
            let _ = OrgMemberVibeLimit::decrement_spent(pool, org_id, user_id_str, estimated_vibe)
                .await;
        }
        return Err(ApiError::PaymentRequired(format!(
            "insufficient VIBE balance in org {org_id} for estimated cost {estimated_vibe}"
        )));
    }

    // 3. Audit row.
    record_org_charge_row(
        pool,
        org_id,
        user_id_str,
        estimated_vibe,
        linked_avatar_id,
        description,
        "pending",
    )
    .await
}

/// Settle a previously debited charge with the actual upstream cost.
///
/// Refunds `(estimated_vibe - actual_vibe)` to the org balance and decrements
/// the user's `current_period_spent` by the same delta. Updates the audit
/// row to record the actual amount + `status: "settled"`.
///
/// Silently no-ops on a bypass-tagged charge (the row was inserted at
/// amount 0 during the original debit).
pub async fn settle_org_vibe_charge(
    pool: &SqlitePool,
    charge_id: Uuid,
    actual_vibe: i64,
) -> Result<(), ApiError> {
    let row = sqlx::query_as::<_, (i64, String, Option<String>)>(
        "SELECT amount_vibe, source_id, metadata
         FROM vibe_transactions
         WHERE id = ?",
    )
    .bind(charge_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?;

    let Some((estimated_vibe, org_id, metadata_json)) = row else {
        return Err(ApiError::NotFound(format!(
            "vibe charge {} not found",
            charge_id
        )));
    };

    let (user_id_str, status) = parse_charge_metadata(metadata_json.as_deref());
    if status.as_deref() == Some("bypass") {
        return Ok(());
    }

    let refund = (estimated_vibe - actual_vibe).max(0);
    if refund > 0 {
        sqlx::query(
            "UPDATE organizations
             SET vibe_balance = vibe_balance + ?,
                 updated_at = datetime('now','subsec')
             WHERE id = ?",
        )
        .bind(refund as f64)
        .bind(&org_id)
        .execute(pool)
        .await
        .map_err(ApiError::Database)?;

        if let Some(uid) = user_id_str.as_deref() {
            let _ = OrgMemberVibeLimit::decrement_spent(pool, &org_id, uid, refund).await;
        }
    }

    sqlx::query(
        "UPDATE vibe_transactions
         SET amount_vibe = ?,
             metadata = json_set(COALESCE(metadata, '{}'), '$.status', 'settled'),
             updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(actual_vibe)
    .bind(charge_id)
    .execute(pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}

/// Reverse a debit in full. Used when the pipeline fails before producing
/// anything billable.
pub async fn refund_org_vibe_charge(pool: &SqlitePool, charge_id: Uuid) -> Result<(), ApiError> {
    let row = sqlx::query_as::<_, (i64, String, Option<String>)>(
        "SELECT amount_vibe, source_id, metadata
         FROM vibe_transactions
         WHERE id = ?",
    )
    .bind(charge_id)
    .fetch_optional(pool)
    .await
    .map_err(ApiError::Database)?;

    let Some((estimated_vibe, org_id, metadata_json)) = row else {
        return Err(ApiError::NotFound(format!(
            "vibe charge {} not found",
            charge_id
        )));
    };

    let (user_id_str, status) = parse_charge_metadata(metadata_json.as_deref());
    if status.as_deref() == Some("bypass") {
        return Ok(());
    }

    if estimated_vibe > 0 {
        sqlx::query(
            "UPDATE organizations
             SET vibe_balance = vibe_balance + ?,
                 updated_at = datetime('now','subsec')
             WHERE id = ?",
        )
        .bind(estimated_vibe as f64)
        .bind(&org_id)
        .execute(pool)
        .await
        .map_err(ApiError::Database)?;

        if let Some(uid) = user_id_str.as_deref() {
            let _ = OrgMemberVibeLimit::decrement_spent(pool, &org_id, uid, estimated_vibe).await;
        }
    }

    sqlx::query(
        "UPDATE vibe_transactions
         SET amount_vibe = 0,
             metadata = json_set(COALESCE(metadata, '{}'), '$.status', 'refunded'),
             updated_at = datetime('now','subsec')
         WHERE id = ?",
    )
    .bind(charge_id)
    .execute(pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(())
}

// ─── private helpers ─────────────────────────────────────────────────────────

async fn record_org_charge_row(
    pool: &SqlitePool,
    org_id: &str,
    user_id_str: &str,
    amount_vibe: i64,
    linked_avatar_id: Option<Uuid>,
    description: &str,
    status: &str,
) -> Result<Uuid, ApiError> {
    let charge_id = Uuid::new_v4();
    let metadata = json!({
        "user_id": user_id_str,
        "linked_avatar_id": linked_avatar_id.map(|u| u.to_string()),
        "status": status,
    })
    .to_string();

    sqlx::query(
        "INSERT INTO vibe_transactions
            (id, source_type, source_id, amount_vibe, description, metadata)
         VALUES (?, 'organization', ?, ?, ?, ?)",
    )
    .bind(charge_id)
    .bind(org_id)
    .bind(amount_vibe)
    .bind(description)
    .bind(&metadata)
    .execute(pool)
    .await
    .map_err(ApiError::Database)?;

    Ok(charge_id)
}

fn parse_charge_metadata(raw: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(s) = raw else {
        return (None, None);
    };
    let parsed: serde_json::Value = serde_json::from_str(s).unwrap_or(json!({}));
    let user_id = parsed
        .get("user_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let status = parsed
        .get("status")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    (user_id, status)
}
