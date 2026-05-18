use sqlx::SqlitePool;
use uuid::Uuid;

pub struct NewsEventDetector {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct TradeItem {
    id: String,
    title: Option<String>,
    summary: Option<String>,
    organization_id: Option<String>,
}

impl NewsEventDetector {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn process_item(
        &self,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let item: Option<TradeItem> = sqlx::query_as(
            r#"SELECT pci.id, pci.title, pci.summary, pci.organization_id
               FROM pulse_content_items pci
               JOIN pulse_sources ps ON ps.id = pci.source_id
               WHERE pci.id = ?1 AND ps.source_category = 'trade_press'"#,
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(item) = item else { return Ok(()) };
        let Some(org_id) = &item.organization_id else {
            return Ok(());
        };

        let title = item.title.as_deref().unwrap_or("").to_string();
        if title.is_empty() {
            return Ok(());
        }

        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM social_content_opportunities \
             WHERE organization_id = ?1 AND signal_source_id = ?2",
        )
        .bind(org_id)
        .bind(&item.id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if existing > 0 {
            return Ok(());
        }

        let id = Uuid::new_v4().to_string();
        let brief = format!(
            "Industry news: {title}. {}Create timely content to capitalize on this news cycle.",
            item.summary
                .as_deref()
                .map(|s| format!("{s} "))
                .unwrap_or_default()
        );
        let expires_at = (chrono::Utc::now() + chrono::Duration::hours(72))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        sqlx::query(
            "INSERT INTO social_content_opportunities \
             (id, organization_id, opportunity_type, signal_source_type, signal_source_id, \
              title, brief, status, relevance_score, expires_at) \
             VALUES (?1,?2,'news_event','pulse_content_item',?3,?4,?5,'detected',0.65,?6)",
        )
        .bind(&id)
        .bind(org_id)
        .bind(&item.id)
        .bind(&title)
        .bind(&brief)
        .bind(&expires_at)
        .execute(&self.pool)
        .await?;

        tracing::info!("NewsEventDetector: opportunity '{title}' for org {org_id}");
        Ok(())
    }
}
