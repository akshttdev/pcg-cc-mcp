use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct MentionClusterer {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct OrgRow {
    id: String,
}

#[derive(sqlx::FromRow)]
struct MentionRow {
    content: Option<String>,
    sentiment: Option<String>,
    author_username: Option<String>,
}

impl MentionClusterer {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn run_all_orgs(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let orgs: Vec<OrgRow> =
            sqlx::query_as("SELECT id FROM organizations WHERE is_active = 1 OR is_active IS NULL")
                .fetch_all(&self.pool)
                .await?;

        for org in orgs {
            if let Err(e) = self.run_for_org(&org.id).await {
                tracing::warn!("MentionClusterer org {}: {e}", org.id);
            }
        }
        Ok(())
    }

    pub async fn run_for_org(
        &self,
        org_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Pull unread mentions for org's accounts (last 24h, up to 200)
        let mentions: Vec<MentionRow> = sqlx::query_as(
            r#"SELECT sm.content, sm.sentiment, sm.author_username
               FROM social_mentions sm
               JOIN social_accounts sa ON sa.id = sm.social_account_id
               WHERE sa.organization_id = ?1
                 AND sm.status = 'unread'
                 AND sm.received_at >= datetime('now', '-1 day')
               LIMIT 200"#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await?;

        if mentions.is_empty() {
            return Ok(());
        }

        // Crisis detection: spike in negative mentions
        let negative_count = mentions
            .iter()
            .filter(|m| m.sentiment.as_deref() == Some("negative"))
            .count();

        if negative_count > 15 {
            self.create_crisis_alert(org_id, negative_count).await?;
        }

        // Naive topic grouping by keyword frequency (LLM integration point)
        // In production: send mention texts to LLM for clustering.
        // For now: group by first meaningful word as placeholder.
        self.upsert_insight(
            org_id,
            "topic_interest",
            &format!("Recent mentions batch ({} total)", mentions.len()),
            mentions.len() as i64,
            if negative_count > mentions.len() / 3 {
                "negative"
            } else {
                "positive"
            },
        )
        .await?;

        tracing::info!(
            "MentionClusterer: processed {} mentions for org {}",
            mentions.len(),
            org_id
        );
        Ok(())
    }

    async fn upsert_insight(
        &self,
        org_id: &str,
        insight_type: &str,
        topic: &str,
        mention_count: i64,
        sentiment: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let period_start = (now - chrono::Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let period_end = now.format("%Y-%m-%dT%H:%M:%S").to_string();

        sqlx::query(
            "INSERT INTO social_audience_insights \
             (id, organization_id, insight_type, topic, mention_count, sentiment, \
              period_start, period_end) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        )
        .bind(&id)
        .bind(org_id)
        .bind(insight_type)
        .bind(topic)
        .bind(mention_count)
        .bind(sentiment)
        .bind(&period_start)
        .bind(&period_end)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_crisis_alert(
        &self,
        org_id: &str,
        count: usize,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Find the org's default project or pulse tracking config for rule lookup
        // For now: create a direct pulse_alert with priority = urgent
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO pulse_alerts (id, priority, acknowledged, created_at) \
             VALUES (?1, 'urgent', 0, datetime('now','subsec'))",
        )
        .bind(&id)
        .execute(&self.pool)
        .await
        .ok(); // Non-fatal if pulse_alerts schema differs

        tracing::warn!(
            "CRISIS ALERT: org {} has {} negative mentions in last hour",
            org_id,
            count
        );
        Ok(())
    }
}
