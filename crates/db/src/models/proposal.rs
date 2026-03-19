use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// 8-stage proposal pipeline.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Proposal {
    pub id: Uuid,
    pub lead_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub project_id: Option<Uuid>,

    /// drafted | pending_approval | approved | meeting_scheduled | sent | seen | verbal | contract_signed | declined | deferred
    pub status: String,
    pub title: String,
    pub description: String,
    pub quote_amount_vibe: i64,
    /// one-off | retainer | hybrid
    pub deal_type: String,

    pub company_id: Option<Uuid>,
    /// JSON array of contact person UUIDs: ["uuid", ...]
    pub contact_ids: String,

    pub sent_at: Option<DateTime<Utc>>,
    pub seen_at: Option<DateTime<Utc>>,
    pub verbal_at: Option<DateTime<Utc>>,
    pub signed_at: Option<DateTime<Utc>>,
    pub declined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateProposal {
    pub title: String,
    pub lead_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub company_id: Option<Uuid>,
    pub description: Option<String>,
    pub quote_amount_vibe: Option<i64>,
    pub deal_type: Option<String>,
    pub contact_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProposal {
    pub title: Option<String>,
    pub description: Option<String>,
    pub quote_amount_vibe: Option<i64>,
    pub deal_type: Option<String>,
    pub lead_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub company_id: Option<Uuid>,
    pub contact_ids: Option<Vec<String>>,
}

impl Proposal {
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM proposals WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &SqlitePool, input: CreateProposal) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let description = input.description.unwrap_or_default();
        let quote = input.quote_amount_vibe.unwrap_or(0);
        let deal_type = input.deal_type.unwrap_or_else(|| "one-off".into());

        let contact_ids_json = input
            .contact_ids
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or_else(|| "[]".to_string());

        sqlx::query(
            r#"INSERT INTO proposals
               (id, lead_id, organization_id, owner_id, project_id, company_id,
                title, description, quote_amount_vibe, deal_type, contact_ids)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(id)
        .bind(input.lead_id)
        .bind(input.organization_id)
        .bind(input.owner_id)
        .bind(input.project_id)
        .bind(input.company_id)
        .bind(&input.title)
        .bind(&description)
        .bind(quote)
        .bind(&deal_type)
        .bind(&contact_ids_json)
        .execute(pool)
        .await?;

        Self::find_by_id(pool, id)
            .await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    pub async fn list(
        pool: &SqlitePool,
        status: Option<&str>,
        lead_id: Option<Uuid>,
        project_id: Option<Uuid>,
        owner_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let mut qb = sqlx::QueryBuilder::new("SELECT * FROM proposals WHERE 1=1");
        if let Some(s) = status {
            qb.push(" AND status = ").push_bind(s.to_string());
        }
        if let Some(lid) = lead_id {
            qb.push(" AND lead_id = ").push_bind(lid);
        }
        if let Some(pid) = project_id {
            qb.push(" AND project_id = ").push_bind(pid);
        }
        if let Some(oid) = owner_id {
            qb.push(" AND owner_id = ").push_bind(oid);
        }
        if let Some(org) = organization_id {
            qb.push(" AND organization_id = ").push_bind(org);
        }
        qb.push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(limit.unwrap_or(200));
        qb.build_query_as::<Self>().fetch_all(pool).await
    }

    pub async fn list_by_company(
        pool: &SqlitePool,
        company_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as(
            "SELECT * FROM proposals WHERE company_id = ? \
             ORDER BY created_at DESC LIMIT ?",
        )
        .bind(company_id)
        .bind(limit.unwrap_or(200))
        .fetch_all(pool)
        .await
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        input: UpdateProposal,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut qb =
            sqlx::QueryBuilder::new("UPDATE proposals SET updated_at = datetime('now','subsec')");
        if let Some(v) = input.title {
            qb.push(", title = ").push_bind(v);
        }
        if let Some(v) = input.description {
            qb.push(", description = ").push_bind(v);
        }
        if let Some(v) = input.quote_amount_vibe {
            qb.push(", quote_amount_vibe = ").push_bind(v);
        }
        if let Some(v) = input.deal_type {
            qb.push(", deal_type = ").push_bind(v);
        }
        if let Some(v) = input.lead_id {
            qb.push(", lead_id = ").push_bind(v);
        }
        if let Some(v) = input.project_id {
            qb.push(", project_id = ").push_bind(v);
        }
        if let Some(v) = input.owner_id {
            qb.push(", owner_id = ").push_bind(v);
        }
        if let Some(v) = input.company_id {
            qb.push(", company_id = ").push_bind(v);
        }
        if let Some(v) = input.contact_ids {
            let json = serde_json::to_string(&v).unwrap_or_else(|_| "[]".to_string());
            qb.push(", contact_ids = ").push_bind(json);
        }
        qb.push(" WHERE id = ").push_bind(id);
        qb.build().execute(pool).await?;
        Self::find_by_id(pool, id).await
    }

    /// Advance to a new pipeline stage; updates the relevant timestamp.
    pub async fn move_status(
        pool: &SqlitePool,
        id: Uuid,
        new_status: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let ts_col = match new_status {
            "sent" => Some("sent_at"),
            "seen" => Some("seen_at"),
            "verbal" => Some("verbal_at"),
            "contract_signed" => Some("signed_at"),
            "declined" => Some("declined_at"),
            _ => None,
        };

        if let Some(col) = ts_col {
            let sql = format!(
                "UPDATE proposals SET status = ?, {} = datetime('now','subsec'), \
                 updated_at = datetime('now','subsec') WHERE id = ?",
                col
            );
            sqlx::query(&sql)
                .bind(new_status)
                .bind(id)
                .execute(pool)
                .await?;
        } else {
            sqlx::query(
                "UPDATE proposals SET status = ?, updated_at = datetime('now','subsec') WHERE id = ?",
            )
            .bind(new_status)
            .bind(id)
            .execute(pool)
            .await?;
        }

        Self::find_by_id(pool, id).await
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM proposals WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
