use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TokenUsageError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Token usage record not found")]
    NotFound,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TokenUsage {
    pub id: Uuid,
    pub task_attempt_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub project_id: Uuid,
    pub model: String,
    pub provider: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
    pub cost_cents: Option<i64>,
    pub operation_type: Option<String>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateTokenUsage {
    pub task_attempt_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub project_id: Uuid,
    pub model: String,
    pub provider: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub operation_type: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, FromRow, Serialize, TS)]
#[ts(export)]
pub struct TokenUsageSummary {
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub total_cost_cents: Option<i64>,
    pub request_count: i64,
}

#[derive(Debug, FromRow, Serialize, TS)]
#[ts(export)]
pub struct DailyTokenUsage {
    pub usage_date: String,
    pub project_id: Uuid,
    pub model: String,
    pub provider: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_tokens: i64,
    pub total_cost_cents: Option<i64>,
    pub request_count: i64,
}

#[derive(Debug, FromRow, Serialize, TS)]
#[ts(export)]
pub struct TokenUsageByAgent {
    pub agent_id: Uuid,
    pub agent_name: Option<String>,
    pub total_tokens: i64,
    pub request_count: i64,
}

#[derive(Debug, FromRow, Serialize, TS)]
#[ts(export)]
pub struct TokenUsageByProject {
    pub project_id: Uuid,
    pub project_name: Option<String>,
    pub total_tokens: i64,
    pub request_count: i64,
}

impl TokenUsage {
    /// Record new token usage
    pub async fn create(pool: &SqlitePool, data: CreateTokenUsage) -> Result<Self, TokenUsageError> {
        let id = Uuid::new_v4();
        let total_tokens = data.input_tokens + data.output_tokens;
        let provider = data.provider.unwrap_or_else(|| "anthropic".to_string());
        let metadata_str = data.metadata.map(|v| v.to_string());

        let usage = sqlx::query_as::<_, TokenUsage>(
            r#"
            INSERT INTO token_usage (
                id, task_attempt_id, agent_id, project_id, model, provider,
                input_tokens, output_tokens, total_tokens, operation_type, metadata
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.task_attempt_id)
        .bind(data.agent_id)
        .bind(data.project_id)
        .bind(&data.model)
        .bind(&provider)
        .bind(data.input_tokens)
        .bind(data.output_tokens)
        .bind(total_tokens)
        .bind(&data.operation_type)
        .bind(metadata_str)
        .fetch_one(pool)
        .await?;

        Ok(usage)
    }

    /// Get total usage for a project since a given time
    pub async fn sum_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<TokenUsageSummary, TokenUsageError> {
        let result = sqlx::query_as::<_, TokenUsageSummary>(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                SUM(cost_cents) as total_cost_cents,
                COUNT(*) as request_count
            FROM token_usage
            WHERE project_id = ?1 AND created_at >= ?2
            "#,
        )
        .bind(project_id)
        .bind(since)
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get total usage for an agent since a given time
    pub async fn sum_by_agent(
        pool: &SqlitePool,
        agent_id: Uuid,
        since: DateTime<Utc>,
    ) -> Result<TokenUsageSummary, TokenUsageError> {
        let result = sqlx::query_as::<_, TokenUsageSummary>(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                SUM(cost_cents) as total_cost_cents,
                COUNT(*) as request_count
            FROM token_usage
            WHERE agent_id = ?1 AND created_at >= ?2
            "#,
        )
        .bind(agent_id)
        .bind(since)
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get total usage for a task attempt
    pub async fn sum_by_task(
        pool: &SqlitePool,
        task_attempt_id: Uuid,
    ) -> Result<TokenUsageSummary, TokenUsageError> {
        let result = sqlx::query_as::<_, TokenUsageSummary>(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                SUM(cost_cents) as total_cost_cents,
                COUNT(*) as request_count
            FROM token_usage
            WHERE task_attempt_id = ?1
            "#,
        )
        .bind(task_attempt_id)
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get daily totals for the last N days
    pub async fn daily_totals(
        pool: &SqlitePool,
        days: i32,
    ) -> Result<Vec<DailyTokenUsage>, TokenUsageError> {
        let results = sqlx::query_as::<_, DailyTokenUsage>(
            r#"
            SELECT
                date(created_at) as usage_date,
                project_id,
                model,
                provider,
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                SUM(cost_cents) as total_cost_cents,
                COUNT(*) as request_count
            FROM token_usage
            WHERE created_at >= datetime('now', '-' || ?1 || ' days')
            GROUP BY date(created_at), project_id, model, provider
            ORDER BY date(created_at) DESC
            "#,
        )
        .bind(days)
        .fetch_all(pool)
        .await?;

        Ok(results)
    }

    /// Get today's total usage across all projects
    pub async fn today_total(pool: &SqlitePool) -> Result<TokenUsageSummary, TokenUsageError> {
        let result = sqlx::query_as::<_, TokenUsageSummary>(
            r#"
            SELECT
                COALESCE(SUM(input_tokens), 0) as total_input_tokens,
                COALESCE(SUM(output_tokens), 0) as total_output_tokens,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                SUM(cost_cents) as total_cost_cents,
                COUNT(*) as request_count
            FROM token_usage
            WHERE date(created_at) = date('now')
            "#,
        )
        .fetch_one(pool)
        .await?;

        Ok(result)
    }

    /// Get usage breakdown by agent for a time period
    pub async fn by_agent(
        pool: &SqlitePool,
        since: DateTime<Utc>,
    ) -> Result<Vec<TokenUsageByAgent>, TokenUsageError> {
        let results = sqlx::query_as::<_, TokenUsageByAgent>(
            r#"
            SELECT
                tu.agent_id as agent_id,
                a.short_name as agent_name,
                COALESCE(SUM(tu.total_tokens), 0) as total_tokens,
                COUNT(*) as request_count
            FROM token_usage tu
            LEFT JOIN agents a ON tu.agent_id = a.id
            WHERE tu.agent_id IS NOT NULL AND tu.created_at >= ?1
            GROUP BY tu.agent_id, a.short_name
            ORDER BY total_tokens DESC
            "#,
        )
        .bind(since)
        .fetch_all(pool)
        .await?;

        Ok(results)
    }

    /// Get usage breakdown by project for a time period
    pub async fn by_project(
        pool: &SqlitePool,
        since: DateTime<Utc>,
    ) -> Result<Vec<TokenUsageByProject>, TokenUsageError> {
        let results = sqlx::query_as::<_, TokenUsageByProject>(
            r#"
            SELECT
                tu.project_id as project_id,
                p.name as project_name,
                COALESCE(SUM(tu.total_tokens), 0) as total_tokens,
                COUNT(*) as request_count
            FROM token_usage tu
            LEFT JOIN projects p ON tu.project_id = p.id
            WHERE tu.created_at >= ?1
            GROUP BY tu.project_id, p.name
            ORDER BY total_tokens DESC
            "#,
        )
        .bind(since)
        .fetch_all(pool)
        .await?;

        Ok(results)
    }
}
