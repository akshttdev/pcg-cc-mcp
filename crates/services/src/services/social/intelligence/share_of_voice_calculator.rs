use serde_json::json;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct ShareOfVoiceCalculator {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct OrgRow {
    id: String,
}

impl ShareOfVoiceCalculator {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn run_all_orgs(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let orgs: Vec<OrgRow> = sqlx::query_as("SELECT id FROM organizations")
            .fetch_all(&self.pool)
            .await?;

        for org in orgs {
            if let Err(e) = self.calculate_for_org(&org.id).await {
                tracing::warn!("ShareOfVoiceCalculator org {}: {e}", org.id);
            }
        }
        Ok(())
    }

    pub async fn calculate_for_org(
        &self,
        org_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let existing_this_week: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM social_share_of_voice \
             WHERE organization_id = ?1 AND period_end >= datetime('now', '-7 days')",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if existing_this_week > 0 {
            return Ok(());
        }

        let own_mentions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM social_mentions sm \
             JOIN social_accounts sa ON sa.id = sm.social_account_id \
             WHERE sa.organization_id = ?1 \
               AND sm.received_at >= datetime('now', '-7 days')",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let total_niche: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pulse_content_items pci \
             JOIN pulse_sources ps ON ps.id = pci.source_id \
             WHERE pci.organization_id = ?1 \
               AND ps.source_category = 'keyword_feed' \
               AND pci.collected_at >= datetime('now', '-7 days')",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let sov_pct = if total_niche > 0 {
            (own_mentions as f64 / total_niche as f64) * 100.0
        } else {
            0.0
        };

        let now = chrono::Utc::now();
        let period_start = (now - chrono::Duration::days(7))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();
        let period_end = now.format("%Y-%m-%dT%H:%M:%S").to_string();

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO social_share_of_voice \
             (id, organization_id, period_start, period_end, keyword_set, \
              own_mention_count, total_niche_mention_count, share_of_voice_pct, \
              competitor_summary) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        )
        .bind(&id)
        .bind(org_id)
        .bind(&period_start)
        .bind(&period_end)
        .bind(json!(["brand", "niche"]).to_string())
        .bind(own_mentions)
        .bind(total_niche)
        .bind(sov_pct)
        .bind(json!([]).to_string())
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "ShareOfVoice: org {} → {:.1}% ({}/{})",
            org_id,
            sov_pct,
            own_mentions,
            total_niche
        );
        Ok(())
    }
}
