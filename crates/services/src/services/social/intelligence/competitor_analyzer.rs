use sqlx::SqlitePool;
use uuid::Uuid;

pub struct CompetitorAnalyzer {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct OrgRow {
    id: String,
}

impl CompetitorAnalyzer {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn run_all_orgs(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let orgs: Vec<OrgRow> = sqlx::query_as("SELECT id FROM organizations")
            .fetch_all(&self.pool)
            .await?;

        for org in orgs {
            if let Err(e) = self.run_for_org(&org.id).await {
                tracing::warn!("CompetitorAnalyzer org {}: {e}", org.id);
            }
        }
        Ok(())
    }

    pub async fn run_for_org(
        &self,
        org_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let competitor_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pulse_content_items pci \
             JOIN pulse_sources ps ON ps.id = pci.source_id \
             WHERE pci.organization_id = ?1 \
               AND ps.source_category = 'competitor' \
               AND pci.collected_at >= datetime('now', '-7 days')",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if competitor_count == 0 {
            return Ok(());
        }

        let existing_this_week: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM social_content_opportunities \
             WHERE organization_id = ?1 AND opportunity_type = 'competitor_gap' \
               AND created_at >= datetime('now', '-7 days')",
        )
        .bind(org_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if existing_this_week > 0 {
            return Ok(());
        }

        let id = Uuid::new_v4().to_string();
        let brief = format!(
            "Weekly competitor analysis: {competitor_count} competitor posts detected in the \
             last 7 days. Review for content gap opportunities and format inspiration."
        );

        sqlx::query(
            "INSERT INTO social_content_opportunities \
             (id, organization_id, opportunity_type, signal_source_type, \
              title, brief, status, relevance_score) \
             VALUES (?1,?2,'competitor_gap','snapshot_analysis',\
                     'Weekly competitor content analysis',?3,'detected',0.6)",
        )
        .bind(&id)
        .bind(org_id)
        .bind(&brief)
        .execute(&self.pool)
        .await?;

        tracing::info!("CompetitorAnalyzer: weekly gap opportunity for org {org_id}");
        Ok(())
    }
}
