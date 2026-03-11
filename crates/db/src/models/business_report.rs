use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BusinessReport {
    pub id: Uuid,
    pub person_id: Option<Uuid>,
    pub company_id: Option<Uuid>,
    pub report_type: String,
    pub title: String,
    pub status: String,
    pub executive_summary: Option<String>,
    pub company_overview: Option<String>,
    pub pain_points: String,
    pub opportunities: String,
    pub recommended_services: String,
    pub next_steps: String,
    pub full_report_md: Option<String>,
    // Enhanced analytics sections
    pub individual_profiles: String,
    pub market_analysis: Option<String>,
    pub competitor_analysis: String,
    pub target_clients: Option<String>,
    pub brand_positioning: Option<String>,
    pub digital_presence: Option<String>,
    pub sources: String,
    // Source tracking
    pub intake_item_ids: String,
    pub call_log_ids: String,
    pub created_by: Option<Uuid>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBusinessReport {
    pub person_id: Option<Uuid>,
    pub company_id: Option<Uuid>,
    pub report_type: Option<String>,
    pub title: String,
    pub executive_summary: Option<String>,
    pub company_overview: Option<String>,
    pub pain_points: Option<String>,
    pub opportunities: Option<String>,
    pub recommended_services: Option<String>,
    pub next_steps: Option<String>,
    pub full_report_md: Option<String>,
    pub individual_profiles: Option<String>,
    pub market_analysis: Option<String>,
    pub competitor_analysis: Option<String>,
    pub target_clients: Option<String>,
    pub brand_positioning: Option<String>,
    pub digital_presence: Option<String>,
    pub sources: Option<String>,
    pub intake_item_ids: Option<String>,
    pub call_log_ids: Option<String>,
    pub created_by: Option<Uuid>,
}

/// Fields that can be patched on a business report (all optional)
#[derive(Debug, Deserialize, Default)]
pub struct PatchBusinessReport {
    pub title: Option<String>,
    pub executive_summary: Option<String>,
    pub company_overview: Option<String>,
    pub pain_points: Option<String>,
    pub opportunities: Option<String>,
    pub recommended_services: Option<String>,
    pub next_steps: Option<String>,
    pub full_report_md: Option<String>,
    pub individual_profiles: Option<String>,
    pub market_analysis: Option<String>,
    pub competitor_analysis: Option<String>,
    pub target_clients: Option<String>,
    pub brand_positioning: Option<String>,
    pub digital_presence: Option<String>,
    pub sources: Option<String>,
}

impl BusinessReport {
    pub async fn create(
        pool: &sqlx::SqlitePool,
        data: CreateBusinessReport,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        sqlx::query_as::<_, Self>(
            "INSERT INTO business_reports
             (id, person_id, company_id, report_type, title, status,
              executive_summary, company_overview, pain_points, opportunities,
              recommended_services, next_steps, full_report_md,
              individual_profiles, market_analysis, competitor_analysis,
              target_clients, brand_positioning, digital_presence, sources,
              intake_item_ids, call_log_ids, created_by)
             VALUES (?, ?, ?, ?, ?, 'draft', ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING *",
        )
        .bind(id)
        .bind(data.person_id)
        .bind(data.company_id)
        .bind(data.report_type.as_deref().unwrap_or("business_audit"))
        .bind(&data.title)
        .bind(&data.executive_summary)
        .bind(&data.company_overview)
        .bind(data.pain_points.as_deref().unwrap_or("[]"))
        .bind(data.opportunities.as_deref().unwrap_or("[]"))
        .bind(data.recommended_services.as_deref().unwrap_or("[]"))
        .bind(data.next_steps.as_deref().unwrap_or("[]"))
        .bind(&data.full_report_md)
        .bind(data.individual_profiles.as_deref().unwrap_or("[]"))
        .bind(&data.market_analysis)
        .bind(data.competitor_analysis.as_deref().unwrap_or("[]"))
        .bind(&data.target_clients)
        .bind(&data.brand_positioning)
        .bind(&data.digital_presence)
        .bind(data.sources.as_deref().unwrap_or("[]"))
        .bind(data.intake_item_ids.as_deref().unwrap_or("[]"))
        .bind(data.call_log_ids.as_deref().unwrap_or("[]"))
        .bind(data.created_by)
        .fetch_one(pool)
        .await
    }

    pub async fn find_by_id(
        pool: &sqlx::SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM business_reports WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn list(pool: &sqlx::SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM business_reports ORDER BY created_at DESC LIMIT 200",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn list_by_person(
        pool: &sqlx::SqlitePool,
        person_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "SELECT * FROM business_reports WHERE person_id = ? ORDER BY created_at DESC",
        )
        .bind(person_id)
        .fetch_all(pool)
        .await
    }

    pub async fn mark_ready(
        pool: &sqlx::SqlitePool,
        id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE business_reports SET status = 'ready', updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn patch(
        pool: &sqlx::SqlitePool,
        id: Uuid,
        data: PatchBusinessReport,
    ) -> Result<Option<Self>, sqlx::Error> {
        // Build SET clause dynamically — only update fields that were provided
        let mut sets: Vec<&str> = vec!["updated_at = datetime('now','subsec')"];
        macro_rules! push_if_some {
            ($field:expr, $col:literal) => {
                if $field.is_some() { sets.push($col); }
            }
        }
        push_if_some!(data.title, "title = ?");
        push_if_some!(data.executive_summary, "executive_summary = ?");
        push_if_some!(data.company_overview, "company_overview = ?");
        push_if_some!(data.pain_points, "pain_points = ?");
        push_if_some!(data.opportunities, "opportunities = ?");
        push_if_some!(data.recommended_services, "recommended_services = ?");
        push_if_some!(data.next_steps, "next_steps = ?");
        push_if_some!(data.full_report_md, "full_report_md = ?");
        push_if_some!(data.individual_profiles, "individual_profiles = ?");
        push_if_some!(data.market_analysis, "market_analysis = ?");
        push_if_some!(data.competitor_analysis, "competitor_analysis = ?");
        push_if_some!(data.target_clients, "target_clients = ?");
        push_if_some!(data.brand_positioning, "brand_positioning = ?");
        push_if_some!(data.digital_presence, "digital_presence = ?");
        push_if_some!(data.sources, "sources = ?");

        let sql = format!(
            "UPDATE business_reports SET {} WHERE id = ? RETURNING *",
            sets.join(", ")
        );

        let mut q = sqlx::query_as::<_, Self>(&sql);
        macro_rules! bind_if_some {
            ($field:expr) => {
                if let Some(v) = $field { q = q.bind(v); }
            }
        }
        bind_if_some!(data.title);
        bind_if_some!(data.executive_summary);
        bind_if_some!(data.company_overview);
        bind_if_some!(data.pain_points);
        bind_if_some!(data.opportunities);
        bind_if_some!(data.recommended_services);
        bind_if_some!(data.next_steps);
        bind_if_some!(data.full_report_md);
        bind_if_some!(data.individual_profiles);
        bind_if_some!(data.market_analysis);
        bind_if_some!(data.competitor_analysis);
        bind_if_some!(data.target_clients);
        bind_if_some!(data.brand_positioning);
        bind_if_some!(data.digital_presence);
        bind_if_some!(data.sources);
        q = q.bind(id);

        q.fetch_optional(pool).await
    }
}
