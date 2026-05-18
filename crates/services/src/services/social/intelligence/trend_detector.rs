use sqlx::SqlitePool;
use uuid::Uuid;

pub struct TrendDetector {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct ContentItemRow {
    id: String,
    title: Option<String>,
    body: Option<String>,
    summary: Option<String>,
    source_type: String,
    organization_id: Option<String>,
    project_id: Option<String>,
    extracted_entities: Option<String>,
}

impl TrendDetector {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Process a single new PulseContentItem for trending signals.
    pub async fn process_item(
        &self,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let item: Option<ContentItemRow> = sqlx::query_as(
            r#"SELECT pci.id, pci.title, pci.body, pci.summary, pci.source_type,
                      pci.organization_id, pci.project_id, pci.extracted_entities
               FROM pulse_content_items pci
               JOIN pulse_sources ps ON ps.id = pci.source_id
               WHERE pci.id = ?1
                 AND ps.source_category IN ('keyword_feed', 'kol_influencer', 'thought_leader')"#,
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(item) = item else { return Ok(()) };

        let org_id = match &item.organization_id {
            Some(o) => o.clone(),
            None => return Ok(()),
        };

        // Extract keyword signals from content
        let text = [
            item.title.as_deref(),
            item.summary.as_deref(),
            item.body.as_deref().map(|b| &b[..b.len().min(500)]),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");

        if text.len() < 20 {
            return Ok(());
        }

        // Count recent items with similar content in 24h window as velocity proxy
        let recent_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pulse_content_items pci \
             JOIN pulse_sources ps ON ps.id = pci.source_id \
             WHERE pci.organization_id = ?1 \
               AND ps.source_category IN ('keyword_feed', 'kol_influencer') \
               AND pci.collected_at >= datetime('now', '-1 day')",
        )
        .bind(&org_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // 7-day baseline: avg items per day
        let baseline: f64 = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM pulse_content_items pci \
             JOIN pulse_sources ps ON ps.id = pci.source_id \
             WHERE pci.organization_id = ?1 \
               AND ps.source_category IN ('keyword_feed', 'kol_influencer') \
               AND pci.collected_at >= datetime('now', '-7 days')",
        )
        .bind(&org_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(1) as f64
            / 7.0;

        // Velocity > 3× baseline → trending
        if baseline > 0.0 && (recent_count as f64) > baseline * 3.0 {
            let title = item
                .title
                .as_deref()
                .unwrap_or("Trending topic detected")
                .to_string();

            self.create_opportunity(&org_id, item_id, &title, &text, recent_count, baseline)
                .await?;
        }

        Ok(())
    }

    async fn create_opportunity(
        &self,
        org_id: &str,
        signal_id: &str,
        title: &str,
        _text: &str,
        count: i64,
        baseline: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Check: does this org already have an opportunity for this signal today?
        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM social_content_opportunities \
             WHERE organization_id = ?1 AND signal_source_id = ?2",
        )
        .bind(org_id)
        .bind(signal_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if existing > 0 {
            return Ok(());
        }

        let id = Uuid::new_v4().to_string();
        let velocity_x = count as f64 / baseline.max(0.1);
        let brief = format!(
            "Trending topic detected: {title}. \
             Signal velocity {velocity_x:.1}× above 7-day baseline ({count} items in last 24h). \
             Consider posting while the topic is active."
        );

        // expires in 48h
        let expires_at = (chrono::Utc::now() + chrono::Duration::hours(48))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        sqlx::query(
            "INSERT INTO social_content_opportunities \
             (id, organization_id, opportunity_type, signal_source_type, signal_source_id, \
              title, brief, status, relevance_score, expires_at) \
             VALUES (?1,?2,'trending_topic','pulse_content_item',?3,?4,?5,'detected',0.7,?6)",
        )
        .bind(&id)
        .bind(org_id)
        .bind(signal_id)
        .bind(title)
        .bind(&brief)
        .bind(&expires_at)
        .execute(&self.pool)
        .await?;

        tracing::info!("TrendDetector: created opportunity '{title}' for org {org_id}");
        Ok(())
    }
}
