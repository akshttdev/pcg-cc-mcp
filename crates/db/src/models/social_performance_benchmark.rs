use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SocialPerformanceBenchmark {
    pub id: String,
    pub social_account_id: String,
    pub day_of_week: i64,
    pub hour_of_day: i64,
    pub avg_engagement_rate: f64,
    pub avg_reach: f64,
    pub avg_saves: f64,
    pub avg_impressions: f64,
    pub sample_count: i64,
    pub confidence: f64,
    pub updated_at: String,
}

/// Optimal time slot with computed score
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OptimalSlot {
    pub day_of_week: i64,
    pub hour_of_day: i64,
    pub score: f64,
    pub sample_count: i64,
    pub confidence: f64,
}

impl SocialPerformanceBenchmark {
    /// Upsert a benchmark data point using rolling average formula.
    pub async fn upsert(
        pool: &SqlitePool,
        account_id: &str,
        day_of_week: i64,
        hour_of_day: i64,
        new_engagement_rate: f64,
        new_reach: f64,
        new_saves: f64,
        new_impressions: f64,
    ) -> Result<(), BenchmarkError> {
        let id = Uuid::new_v4().to_string();
        // Rolling average: new_avg = (old_avg * n + new_val) / (n + 1)
        // Implemented via SQLite CASE expression.
        // confidence = min(sample_count / 20.0, 1.0) — reaches full confidence at 20 samples
        sqlx::query(
            r#"
            INSERT INTO social_performance_benchmarks
                (id, social_account_id, day_of_week, hour_of_day,
                 avg_engagement_rate, avg_reach, avg_saves, avg_impressions,
                 sample_count, confidence, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1,
                    MIN(1.0 / 20.0, 1.0),
                    datetime('now','subsec'))
            ON CONFLICT(social_account_id, day_of_week, hour_of_day) DO UPDATE SET
                avg_engagement_rate = (avg_engagement_rate * sample_count + ?5) / (sample_count + 1),
                avg_reach           = (avg_reach           * sample_count + ?6) / (sample_count + 1),
                avg_saves           = (avg_saves           * sample_count + ?7) / (sample_count + 1),
                avg_impressions     = (avg_impressions     * sample_count + ?8) / (sample_count + 1),
                sample_count        = sample_count + 1,
                confidence          = MIN(CAST(sample_count + 1 AS REAL) / 20.0, 1.0),
                updated_at          = datetime('now','subsec')
            "#,
        )
        .bind(&id)
        .bind(account_id)
        .bind(day_of_week)
        .bind(hour_of_day)
        .bind(new_engagement_rate)
        .bind(new_reach)
        .bind(new_saves)
        .bind(new_impressions)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Return top N slots by saves score for an account, only where confidence > threshold.
    pub async fn get_optimal_slots(
        pool: &SqlitePool,
        account_id: &str,
        min_confidence: f64,
        limit: i64,
    ) -> Result<Vec<OptimalSlot>, BenchmarkError> {
        let rows: Vec<SocialPerformanceBenchmark> = sqlx::query_as(
            r#"
            SELECT * FROM social_performance_benchmarks
            WHERE social_account_id = ?1 AND confidence >= ?2
            ORDER BY avg_saves DESC, avg_engagement_rate DESC
            LIMIT ?3
            "#,
        )
        .bind(account_id)
        .bind(min_confidence)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let slots = rows
            .into_iter()
            .map(|r| OptimalSlot {
                day_of_week: r.day_of_week,
                hour_of_day: r.hour_of_day,
                // Composite score: saves weighted 60%, engagement 40%
                score: r.avg_saves * 0.6 + r.avg_engagement_rate * 100.0 * 0.4,
                sample_count: r.sample_count,
                confidence: r.confidence,
            })
            .collect();

        Ok(slots)
    }

    /// Get all benchmarks for an account (for heatmap display).
    pub async fn get_heatmap(
        pool: &SqlitePool,
        account_id: &str,
    ) -> Result<Vec<SocialPerformanceBenchmark>, BenchmarkError> {
        let rows = sqlx::query_as(
            "SELECT * FROM social_performance_benchmarks WHERE social_account_id = ?1",
        )
        .bind(account_id)
        .fetch_all(pool)
        .await?;
        Ok(rows)
    }
}
