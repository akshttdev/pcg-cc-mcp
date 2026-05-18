use db::models::social_performance_benchmark::SocialPerformanceBenchmark;
use sqlx::SqlitePool;

pub struct SnapshotAnalyzer {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct PostMetricRow {
    account_id: String,
    day_of_week: i64,
    hour_of_day: i64,
    engagement_rate: f64,
    reach: f64,
    saves: f64,
    impressions: f64,
}

#[derive(sqlx::FromRow)]
struct AccountIdRow {
    id: String,
}

impl SnapshotAnalyzer {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Process all accounts — compute benchmarks from published post metrics.
    pub async fn run_all_accounts(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let accounts: Vec<AccountIdRow> =
            sqlx::query_as("SELECT id FROM social_accounts WHERE status = 'active'")
                .fetch_all(&self.pool)
                .await?;

        for account in accounts {
            if let Err(e) = self.run_for_account(&account.id).await {
                tracing::warn!("SnapshotAnalyzer: account {} error: {}", account.id, e);
            }
        }
        Ok(())
    }

    /// Compute benchmarks for one account from its published post metrics (last 90 days).
    pub async fn run_for_account(
        &self,
        account_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Pull published post metrics joined with post publish time
        // Bucket by (day_of_week, hour_of_day) using strftime
        let rows: Vec<PostMetricRow> = sqlx::query_as(
            r#"
            SELECT
                sp.social_account_id AS account_id,
                CAST(strftime('%w', sp.published_at) AS INTEGER) AS day_of_week,
                CAST(strftime('%H', sp.published_at) AS INTEGER) AS hour_of_day,
                COALESCE(sp.engagement_rate, 0.0)                AS engagement_rate,
                COALESCE(CAST(sp.reach AS REAL), 0.0)            AS reach,
                COALESCE(CAST(sp.saves AS REAL), 0.0)            AS saves,
                COALESCE(CAST(sp.impressions AS REAL), 0.0)      AS impressions
            FROM social_posts sp
            WHERE sp.social_account_id = ?1
              AND sp.status = 'published'
              AND sp.published_at >= datetime('now', '-90 days')
              AND sp.published_at IS NOT NULL
            "#,
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            SocialPerformanceBenchmark::upsert(
                &self.pool,
                &row.account_id,
                row.day_of_week,
                row.hour_of_day,
                row.engagement_rate,
                row.reach,
                row.saves,
                row.impressions,
            )
            .await?;
        }

        tracing::info!("SnapshotAnalyzer: updated benchmarks for account {account_id}");
        Ok(())
    }
}
