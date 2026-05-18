use sqlx::SqlitePool;
use uuid::Uuid;

pub struct OpportunityDrafter {
    pool: SqlitePool,
}

#[derive(sqlx::FromRow)]
struct OpportunityRow {
    id: String,
    organization_id: Option<String>,
    project_id: Option<String>,
    opportunity_type: String,
    title: String,
    brief: Option<String>,
}

impl OpportunityDrafter {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Draft all detected opportunities that don't yet have a draft post.
    pub async fn draft_pending(&self) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let pending: Vec<OpportunityRow> = sqlx::query_as(
            "SELECT id, organization_id, project_id, opportunity_type, title, brief \
             FROM social_content_opportunities \
             WHERE status = 'detected' AND draft_post_id IS NULL \
             ORDER BY relevance_score DESC \
             LIMIT 20",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut drafted = 0;
        for opp in pending {
            if let Err(e) = self.draft_opportunity(&opp).await {
                tracing::warn!("OpportunityDrafter: opp {} error: {e}", opp.id);
            } else {
                drafted += 1;
            }
        }
        Ok(drafted)
    }

    async fn draft_opportunity(
        &self,
        opp: &OpportunityRow,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let caption = self.generate_caption(opp);
        let hashtags = self.generate_hashtags(&opp.opportunity_type);
        let org_id = opp.organization_id.as_deref().unwrap_or("");
        let post_id = Uuid::new_v4().to_string();

        sqlx::query(
            "INSERT INTO social_posts \
             (id, organization_id, project_id, content_type, caption, hashtags, \
              platforms, status, category, created_at, updated_at) \
             VALUES (?1,?2,?3,'post',?4,?5,'[]','draft',?6,\
                     datetime('now','subsec'),datetime('now','subsec'))",
        )
        .bind(&post_id)
        .bind(org_id)
        .bind(&opp.project_id)
        .bind(&caption)
        .bind(&hashtags)
        .bind(&opp.opportunity_type)
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "UPDATE social_content_opportunities SET \
             draft_post_id = ?2, status = 'draft_created', \
             updated_at = datetime('now','subsec') WHERE id = ?1",
        )
        .bind(&opp.id)
        .bind(&post_id)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            "OpportunityDrafter: drafted post {} for opportunity '{}'",
            post_id,
            opp.title
        );
        Ok(())
    }

    fn generate_caption(&self, opp: &OpportunityRow) -> String {
        match opp.opportunity_type.as_str() {
            "trending_topic" => format!(
                "✨ {}\n\n[Add your unique perspective here]\n\nWhat do you think?",
                opp.title
            ),
            "news_event" => format!(
                "📰 {}\n\n[Add your take here]\n\nStay tuned for more!",
                opp.title
            ),
            "audience_question" => format!(
                "You asked, we're answering! 💬\n\n{}\n\nDrop your questions in the comments!",
                opp.title
            ),
            "kol_signal" => format!("🔥 {}\n\n[Add your perspective on this trend]", opp.title),
            _ => format!("{}\n\n[Draft — edit before publishing]", opp.title),
        }
    }

    fn generate_hashtags(&self, opportunity_type: &str) -> String {
        let tags: &[&str] = match opportunity_type {
            "trending_topic" => &["trending", "viral", "mustsee"],
            "news_event" => &["industrynews", "stayinformed", "community"],
            "audience_question" => &["faq", "community", "youasked"],
            "kol_signal" => &["inspired", "thoughtleader", "insights"],
            _ => &["content", "socialmedia"],
        };
        serde_json::to_string(tags).unwrap_or_default()
    }
}
