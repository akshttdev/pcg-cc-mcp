use sqlx::SqlitePool;
use uuid::Uuid;

pub struct KolSignalExtractor {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct KolItem {
    id: String,
    title: Option<String>,
    summary: Option<String>,
    organization_id: Option<String>,
    relevance_score: Option<f64>,
}

#[derive(sqlx::FromRow)]
struct EntityRow {
    id: String,
    name: String,
    crm_contact_id: Option<String>,
}

impl KolSignalExtractor {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn process_item(
        &self,
        item_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let item: Option<KolItem> = sqlx::query_as(
            r#"SELECT pci.id, pci.title, pci.summary, pci.organization_id, pci.relevance_score
               FROM pulse_content_items pci
               JOIN pulse_sources ps ON ps.id = pci.source_id
               WHERE pci.id = ?1
                 AND ps.source_category IN ('kol_influencer', 'thought_leader')"#,
        )
        .bind(item_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(item) = item else { return Ok(()) };
        let Some(org_id) = &item.organization_id else {
            return Ok(());
        };

        let relevance = item.relevance_score.unwrap_or(0.0);
        if relevance < 0.6 {
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

        let title = item
            .title
            .as_deref()
            .unwrap_or("KOL/Thought leader signal")
            .to_string();

        let entity: Option<EntityRow> = sqlx::query_as(
            "SELECT ste.id, ste.name, ste.crm_contact_id \
             FROM social_tracked_entities ste \
             WHERE ste.organization_id = ?1 \
               AND ste.entity_type IN ('kol_influencer', 'thought_leader') \
               AND ste.is_active = 1 \
             LIMIT 1",
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await?;

        let brief = format!(
            "KOL signal: {title}. {}Relevance: {relevance:.2}. \
             Consider creating content that references or responds to this angle.",
            item.summary
                .as_deref()
                .map(|s| format!("{s} "))
                .unwrap_or_default()
        );

        let id = Uuid::new_v4().to_string();
        let expires_at = (chrono::Utc::now() + chrono::Duration::hours(48))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        sqlx::query(
            "INSERT INTO social_content_opportunities \
             (id, organization_id, opportunity_type, signal_source_type, signal_source_id, \
              tracked_entity_id, title, brief, status, relevance_score, expires_at) \
             VALUES (?1,?2,'kol_signal','pulse_content_item',?3,?4,?5,?6,'detected',?7,?8)",
        )
        .bind(&id)
        .bind(org_id)
        .bind(&item.id)
        .bind(entity.as_ref().map(|e| &e.id))
        .bind(&title)
        .bind(&brief)
        .bind(relevance)
        .bind(&expires_at)
        .execute(&self.pool)
        .await?;

        // Bridge to CRM if entity has a linked contact
        if let Some(entity) = &entity {
            if let Some(contact_id) = &entity.crm_contact_id {
                sqlx::query(
                    "INSERT INTO crm_activities \
                     (id, crm_contact_id, activity_type, subject, notes, created_at) \
                     VALUES (?1,?2,'note',?3,?4,datetime('now','subsec'))",
                )
                .bind(Uuid::new_v4().to_string())
                .bind(contact_id)
                .bind(format!("KOL Signal: {}", entity.name))
                .bind(format!(
                    "High-engagement content from {}. Topic: {}. Consider outreach.",
                    entity.name, title
                ))
                .execute(&self.pool)
                .await
                .ok();
            }
        }

        tracing::info!("KolSignalExtractor: opportunity '{title}' for org {org_id}");
        Ok(())
    }
}
