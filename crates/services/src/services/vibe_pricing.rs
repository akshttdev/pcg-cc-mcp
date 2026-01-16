use anyhow::{anyhow, Result};
use db::models::model_pricing::{estimate_cost, infer_provider, ModelPricing, CostEstimate, VIBE_USD_VALUE};
use db::models::vibe_transaction::{
    CreateVibeTransaction, VibeSourceType, VibeTransaction, VibeTransactionSummary,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use ts_rs::TS;
use uuid::Uuid;

/// VIBE Pricing Service
///
/// Handles all VIBE token economics for LLM usage:
/// - Cost estimation based on model pricing
/// - Transaction recording in the database
/// - Budget checking for projects and agents
///
/// Note: On-chain operations (deposits/withdrawals) are handled separately
/// at the project level via the treasury custody model.
#[derive(Debug, Clone)]
pub struct VibePricingService {
    pool: SqlitePool,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BudgetStatus {
    pub source_type: String,
    pub source_id: String,
    pub budget_limit: Option<i64>,
    pub spent_amount: i64,
    pub remaining: Option<i64>,
    pub has_budget: bool,
    pub is_exceeded: bool,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PreExecutionCheck {
    pub can_execute: bool,
    pub estimated_cost_vibe: i64,
    pub estimated_cost_usd: f64,
    pub project_budget: Option<BudgetStatus>,
    pub agent_budget: Option<BudgetStatus>,
    pub block_reason: Option<String>,
}

impl VibePricingService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get cost estimate for a model without executing anything
    pub async fn estimate_cost(
        &self,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<CostEstimate> {
        estimate_cost(&self.pool, model, input_tokens, output_tokens)
            .await
            .map_err(|e| anyhow!("Failed to estimate cost: {}", e))
    }

    /// Get model pricing information
    pub async fn get_model_pricing(&self, model: &str) -> Result<ModelPricing> {
        let provider = infer_provider(model);
        ModelPricing::get_with_fallback(&self.pool, model, provider)
            .await
            .map_err(|e| anyhow!("Failed to get model pricing: {}", e))
    }

    /// List all model pricing entries
    pub async fn list_model_pricing(&self) -> Result<Vec<ModelPricing>> {
        ModelPricing::list(&self.pool)
            .await
            .map_err(|e| anyhow!("Failed to list model pricing: {}", e))
    }

    /// Record a VIBE transaction for LLM usage
    ///
    /// This records the usage in the database. The actual on-chain settlement
    /// happens at the treasury level, not per-transaction.
    pub async fn record_llm_usage(
        &self,
        source_type: VibeSourceType,
        source_id: Uuid,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        task_id: Option<Uuid>,
        task_attempt_id: Option<Uuid>,
        process_id: Option<Uuid>,
    ) -> Result<VibeTransaction> {
        // Calculate cost
        let estimate = self.estimate_cost(model, input_tokens, output_tokens).await?;
        let provider = infer_provider(model);

        // Create the transaction record
        let create_data = CreateVibeTransaction {
            source_type,
            source_id,
            amount_vibe: estimate.cost_vibe,
            input_tokens: Some(input_tokens),
            output_tokens: Some(output_tokens),
            model: Some(model.to_string()),
            provider: Some(provider.to_string()),
            calculated_cost_cents: Some(estimate.cost_cents),
            task_id,
            task_attempt_id,
            process_id,
            description: Some(format!(
                "LLM usage: {} ({} in, {} out tokens)",
                model, input_tokens, output_tokens
            )),
            metadata: None,
        };

        VibeTransaction::create(&self.pool, create_data)
            .await
            .map_err(|e| anyhow!("Failed to record VIBE transaction: {}", e))
    }

    /// Get VIBE spending summary for a source
    pub async fn get_spending_summary(
        &self,
        source_type: VibeSourceType,
        source_id: Uuid,
    ) -> Result<VibeTransactionSummary> {
        VibeTransaction::sum_by_source(&self.pool, source_type, source_id, None)
            .await
            .map_err(|e| anyhow!("Failed to get spending summary: {}", e))
    }

    /// Get recent transactions for a source
    pub async fn get_recent_transactions(
        &self,
        source_type: VibeSourceType,
        source_id: Uuid,
        limit: i64,
    ) -> Result<Vec<VibeTransaction>> {
        VibeTransaction::list_by_source(&self.pool, source_type, source_id, limit)
            .await
            .map_err(|e| anyhow!("Failed to get transactions: {}", e))
    }

    /// Get transactions for a specific task
    pub async fn get_task_transactions(&self, task_id: Uuid) -> Result<Vec<VibeTransaction>> {
        VibeTransaction::list_by_task(&self.pool, task_id)
            .await
            .map_err(|e| anyhow!("Failed to get task transactions: {}", e))
    }

    /// Check if a source has sufficient budget for an estimated cost
    pub fn check_budget(
        &self,
        budget_limit: Option<i64>,
        spent_amount: i64,
        estimated_cost: i64,
        source_type: &str,
        source_id: &str,
    ) -> BudgetStatus {
        let remaining = budget_limit.map(|limit| limit - spent_amount);
        let is_exceeded = remaining.map(|r| r < estimated_cost).unwrap_or(false);

        BudgetStatus {
            source_type: source_type.to_string(),
            source_id: source_id.to_string(),
            budget_limit,
            spent_amount,
            remaining,
            has_budget: budget_limit.is_some(),
            is_exceeded,
        }
    }

    /// Pre-execution check to verify both project and agent budgets
    pub fn pre_execution_check(
        &self,
        estimated_cost_vibe: i64,
        project_budget_limit: Option<i64>,
        project_spent: i64,
        project_id: &str,
        agent_budget_limit: Option<i64>,
        agent_spent: i64,
        agent_id: &str,
    ) -> PreExecutionCheck {
        let project_budget = self.check_budget(
            project_budget_limit,
            project_spent,
            estimated_cost_vibe,
            "project",
            project_id,
        );

        let agent_budget = self.check_budget(
            agent_budget_limit,
            agent_spent,
            estimated_cost_vibe,
            "agent",
            agent_id,
        );

        let mut block_reason = None;

        // Check project budget first
        if project_budget.is_exceeded {
            block_reason = Some(format!(
                "Project budget exceeded. Remaining: {} VIBE, Required: {} VIBE",
                project_budget.remaining.unwrap_or(0),
                estimated_cost_vibe
            ));
        }

        // Check agent budget
        if agent_budget.is_exceeded {
            block_reason = Some(format!(
                "Agent budget exceeded. Remaining: {} VIBE, Required: {} VIBE",
                agent_budget.remaining.unwrap_or(0),
                estimated_cost_vibe
            ));
        }

        let can_execute = block_reason.is_none();

        PreExecutionCheck {
            can_execute,
            estimated_cost_vibe,
            estimated_cost_usd: estimated_cost_vibe as f64 * VIBE_USD_VALUE,
            project_budget: Some(project_budget),
            agent_budget: Some(agent_budget),
            block_reason,
        }
    }

    /// Convert USD to VIBE tokens
    pub fn usd_to_vibe(usd: f64) -> i64 {
        (usd / VIBE_USD_VALUE).ceil() as i64
    }

    /// Convert VIBE tokens to USD
    pub fn vibe_to_usd(vibe: i64) -> f64 {
        vibe as f64 * VIBE_USD_VALUE
    }
}
