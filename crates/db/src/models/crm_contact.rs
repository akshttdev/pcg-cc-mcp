use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CrmContactError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Contact not found")]
    NotFound,
    #[error("Contact with this email already exists")]
    AlreadyExists,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ContactSource {
    Manual,
    Email,
    Social,
    Website,
    Referral,
    Import,
    Api,
    ZohoSync,
    GmailSync,
}

impl std::fmt::Display for ContactSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ContactSource::Manual => "manual",
            ContactSource::Email => "email",
            ContactSource::Social => "social",
            ContactSource::Website => "website",
            ContactSource::Referral => "referral",
            ContactSource::Import => "import",
            ContactSource::Api => "api",
            ContactSource::ZohoSync => "zoho_sync",
            ContactSource::GmailSync => "gmail_sync",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStage {
    Subscriber,
    Lead,
    Mql,
    Sql,
    Opportunity,
    Customer,
    Evangelist,
    Churned,
}

impl std::fmt::Display for LifecycleStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LifecycleStage::Subscriber => "subscriber",
            LifecycleStage::Lead => "lead",
            LifecycleStage::Mql => "mql",
            LifecycleStage::Sql => "sql",
            LifecycleStage::Opportunity => "opportunity",
            LifecycleStage::Customer => "customer",
            LifecycleStage::Evangelist => "evangelist",
            LifecycleStage::Churned => "churned",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for LifecycleStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "subscriber" => Ok(LifecycleStage::Subscriber),
            "lead" => Ok(LifecycleStage::Lead),
            "mql" => Ok(LifecycleStage::Mql),
            "sql" => Ok(LifecycleStage::Sql),
            "opportunity" => Ok(LifecycleStage::Opportunity),
            "customer" => Ok(LifecycleStage::Customer),
            "evangelist" => Ok(LifecycleStage::Evangelist),
            "churned" => Ok(LifecycleStage::Churned),
            _ => Err(format!("Unknown lifecycle stage: {}", s)),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CrmContact {
    pub id: Uuid,
    pub project_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub avatar_url: Option<String>,
    pub company_name: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub website: Option<String>,
    pub source: Option<String>,
    pub lifecycle_stage: String,
    pub lead_score: i32,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub last_contacted_at: Option<DateTime<Utc>>,
    pub last_replied_at: Option<DateTime<Utc>>,
    pub owner_user_id: Option<String>,
    pub assigned_agent_id: Option<Uuid>,
    pub zoho_contact_id: Option<String>,
    pub gmail_contact_id: Option<String>,
    pub external_ids: Option<String>,
    pub tags: Option<String>,
    pub lists: Option<String>,
    pub custom_fields: Option<String>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub email_opt_in: Option<i32>,
    pub sms_opt_in: Option<i32>,
    pub do_not_contact: Option<i32>,
    pub email_count: i32,
    pub meeting_count: i32,
    pub deal_count: i32,
    pub total_revenue: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCrmContact {
    pub project_id: Uuid,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub avatar_url: Option<String>,
    pub company_name: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub website: Option<String>,
    pub source: Option<ContactSource>,
    pub lifecycle_stage: Option<LifecycleStage>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub zoho_contact_id: Option<String>,
    pub gmail_contact_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCrmContact {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub avatar_url: Option<String>,
    pub company_name: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub linkedin_url: Option<String>,
    pub twitter_handle: Option<String>,
    pub website: Option<String>,
    pub source: Option<ContactSource>,
    pub lifecycle_stage: Option<LifecycleStage>,
    pub lead_score: Option<i32>,
    pub owner_user_id: Option<String>,
    pub assigned_agent_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
    pub custom_fields: Option<serde_json::Value>,
    pub address_line1: Option<String>,
    pub address_line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub email_opt_in: Option<bool>,
    pub sms_opt_in: Option<bool>,
    pub do_not_contact: Option<bool>,
    pub zoho_contact_id: Option<String>,
    pub gmail_contact_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ContactSearchParams {
    pub project_id: Uuid,
    pub query: Option<String>,
    pub lifecycle_stage: Option<LifecycleStage>,
    pub company_name: Option<String>,
    pub tags: Option<Vec<String>>,
    pub min_lead_score: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl CrmContact {
    fn compute_full_name(first: &Option<String>, last: &Option<String>) -> Option<String> {
        match (first, last) {
            (Some(f), Some(l)) => Some(format!("{} {}", f.trim(), l.trim())),
            (Some(f), None) => Some(f.trim().to_string()),
            (None, Some(l)) => Some(l.trim().to_string()),
            (None, None) => None,
        }
    }

    pub async fn create(
        pool: &SqlitePool,
        data: CreateCrmContact,
    ) -> Result<Self, CrmContactError> {
        let id = Uuid::new_v4();
        let source = data.source.map(|s| s.to_string());
        let lifecycle_stage = data
            .lifecycle_stage
            .map(|s| s.to_string())
            .unwrap_or_else(|| "lead".to_string());
        let full_name = Self::compute_full_name(&data.first_name, &data.last_name);
        let tags = data.tags.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let custom_fields = data.custom_fields.map(|v| v.to_string());

        let contact = sqlx::query_as::<_, CrmContact>(
            r#"
            INSERT INTO crm_contacts (
                id, project_id, first_name, last_name, full_name, email,
                phone, mobile, avatar_url, company_name, job_title, department,
                linkedin_url, twitter_handle, website, source, lifecycle_stage,
                tags, custom_fields, zoho_contact_id, gmail_contact_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(&full_name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.mobile)
        .bind(&data.avatar_url)
        .bind(&data.company_name)
        .bind(&data.job_title)
        .bind(&data.department)
        .bind(&data.linkedin_url)
        .bind(&data.twitter_handle)
        .bind(&data.website)
        .bind(&source)
        .bind(&lifecycle_stage)
        .bind(tags)
        .bind(custom_fields)
        .bind(&data.zoho_contact_id)
        .bind(&data.gmail_contact_id)
        .fetch_one(pool)
        .await?;

        Ok(contact)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, CrmContactError> {
        sqlx::query_as::<_, CrmContact>(
            r#"SELECT * FROM crm_contacts WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(CrmContactError::NotFound)
    }

    pub async fn find_by_email(
        pool: &SqlitePool,
        project_id: Uuid,
        email: &str,
    ) -> Result<Option<Self>, CrmContactError> {
        let contact = sqlx::query_as::<_, CrmContact>(
            r#"SELECT * FROM crm_contacts WHERE project_id = ?1 AND email = ?2"#,
        )
        .bind(project_id)
        .bind(email)
        .fetch_optional(pool)
        .await?;

        Ok(contact)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<Self>, CrmContactError> {
        let limit = limit.unwrap_or(100);
        let contacts = sqlx::query_as::<_, CrmContact>(
            r#"
            SELECT * FROM crm_contacts
            WHERE project_id = ?1
            ORDER BY last_activity_at DESC NULLS LAST, created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(contacts)
    }

    pub async fn find_by_lifecycle_stage(
        pool: &SqlitePool,
        project_id: Uuid,
        stage: LifecycleStage,
    ) -> Result<Vec<Self>, CrmContactError> {
        let stage_str = stage.to_string();
        let contacts = sqlx::query_as::<_, CrmContact>(
            r#"SELECT * FROM crm_contacts WHERE project_id = ?1 AND lifecycle_stage = ?2 ORDER BY lead_score DESC"#,
        )
        .bind(project_id)
        .bind(&stage_str)
        .fetch_all(pool)
        .await?;

        Ok(contacts)
    }

    pub async fn find_by_zoho_id(
        pool: &SqlitePool,
        zoho_contact_id: &str,
    ) -> Result<Option<Self>, CrmContactError> {
        let contact = sqlx::query_as::<_, CrmContact>(
            r#"SELECT * FROM crm_contacts WHERE zoho_contact_id = ?1"#,
        )
        .bind(zoho_contact_id)
        .fetch_optional(pool)
        .await?;

        Ok(contact)
    }

    pub async fn search(
        pool: &SqlitePool,
        params: ContactSearchParams,
    ) -> Result<Vec<Self>, CrmContactError> {
        let mut query = String::from(
            "SELECT * FROM crm_contacts WHERE project_id = ?1"
        );
        let mut bindings: Vec<String> = vec![];

        if let Some(ref q) = params.query {
            query.push_str(" AND (full_name LIKE ?2 OR email LIKE ?2 OR company_name LIKE ?2)");
            bindings.push(format!("%{}%", q));
        }

        if let Some(ref stage) = params.lifecycle_stage {
            let idx = bindings.len() + 2;
            query.push_str(&format!(" AND lifecycle_stage = ?{}", idx));
            bindings.push(stage.to_string());
        }

        if let Some(ref company) = params.company_name {
            let idx = bindings.len() + 2;
            query.push_str(&format!(" AND company_name LIKE ?{}", idx));
            bindings.push(format!("%{}%", company));
        }

        if let Some(min_score) = params.min_lead_score {
            let idx = bindings.len() + 2;
            query.push_str(&format!(" AND lead_score >= ?{}", idx));
            bindings.push(min_score.to_string());
        }

        query.push_str(" ORDER BY lead_score DESC, last_activity_at DESC NULLS LAST");

        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));

        // Build the query dynamically
        let mut db_query = sqlx::query_as::<_, CrmContact>(&query).bind(params.project_id);

        for binding in bindings {
            db_query = db_query.bind(binding);
        }

        let contacts = db_query.fetch_all(pool).await?;
        Ok(contacts)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateCrmContact,
    ) -> Result<Self, CrmContactError> {
        // First get the current contact to compute full_name properly
        let current = Self::find_by_id(pool, id).await?;

        let first_name = data.first_name.clone().or(current.first_name.clone());
        let last_name = data.last_name.clone().or(current.last_name.clone());
        let full_name = Self::compute_full_name(&first_name, &last_name);

        let source = data.source.map(|s| s.to_string());
        let lifecycle_stage = data.lifecycle_stage.map(|s| s.to_string());
        let tags = data.tags.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let custom_fields = data.custom_fields.map(|v| v.to_string());
        let email_opt_in = data.email_opt_in.map(|b| if b { 1 } else { 0 });
        let sms_opt_in = data.sms_opt_in.map(|b| if b { 1 } else { 0 });
        let do_not_contact = data.do_not_contact.map(|b| if b { 1 } else { 0 });

        sqlx::query_as::<_, CrmContact>(
            r#"
            UPDATE crm_contacts SET
                first_name = COALESCE(?2, first_name),
                last_name = COALESCE(?3, last_name),
                full_name = COALESCE(?4, full_name),
                email = COALESCE(?5, email),
                phone = COALESCE(?6, phone),
                mobile = COALESCE(?7, mobile),
                avatar_url = COALESCE(?8, avatar_url),
                company_name = COALESCE(?9, company_name),
                job_title = COALESCE(?10, job_title),
                department = COALESCE(?11, department),
                linkedin_url = COALESCE(?12, linkedin_url),
                twitter_handle = COALESCE(?13, twitter_handle),
                website = COALESCE(?14, website),
                source = COALESCE(?15, source),
                lifecycle_stage = COALESCE(?16, lifecycle_stage),
                lead_score = COALESCE(?17, lead_score),
                owner_user_id = COALESCE(?18, owner_user_id),
                assigned_agent_id = COALESCE(?19, assigned_agent_id),
                tags = COALESCE(?20, tags),
                custom_fields = COALESCE(?21, custom_fields),
                address_line1 = COALESCE(?22, address_line1),
                address_line2 = COALESCE(?23, address_line2),
                city = COALESCE(?24, city),
                state = COALESCE(?25, state),
                postal_code = COALESCE(?26, postal_code),
                country = COALESCE(?27, country),
                email_opt_in = COALESCE(?28, email_opt_in),
                sms_opt_in = COALESCE(?29, sms_opt_in),
                do_not_contact = COALESCE(?30, do_not_contact),
                zoho_contact_id = COALESCE(?31, zoho_contact_id),
                gmail_contact_id = COALESCE(?32, gmail_contact_id),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.first_name)
        .bind(&data.last_name)
        .bind(&full_name)
        .bind(&data.email)
        .bind(&data.phone)
        .bind(&data.mobile)
        .bind(&data.avatar_url)
        .bind(&data.company_name)
        .bind(&data.job_title)
        .bind(&data.department)
        .bind(&data.linkedin_url)
        .bind(&data.twitter_handle)
        .bind(&data.website)
        .bind(&source)
        .bind(&lifecycle_stage)
        .bind(data.lead_score)
        .bind(&data.owner_user_id)
        .bind(data.assigned_agent_id)
        .bind(tags)
        .bind(custom_fields)
        .bind(&data.address_line1)
        .bind(&data.address_line2)
        .bind(&data.city)
        .bind(&data.state)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(email_opt_in)
        .bind(sms_opt_in)
        .bind(do_not_contact)
        .bind(&data.zoho_contact_id)
        .bind(&data.gmail_contact_id)
        .fetch_optional(pool)
        .await?
        .ok_or(CrmContactError::NotFound)
    }

    pub async fn record_activity(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<(), CrmContactError> {
        sqlx::query(
            r#"
            UPDATE crm_contacts SET
                last_activity_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn record_contact_made(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<(), CrmContactError> {
        sqlx::query(
            r#"
            UPDATE crm_contacts SET
                last_contacted_at = datetime('now', 'subsec'),
                last_activity_at = datetime('now', 'subsec'),
                email_count = email_count + 1,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn record_reply_received(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<(), CrmContactError> {
        sqlx::query(
            r#"
            UPDATE crm_contacts SET
                last_replied_at = datetime('now', 'subsec'),
                last_activity_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_lead_score(
        pool: &SqlitePool,
        id: Uuid,
        score_delta: i32,
    ) -> Result<(), CrmContactError> {
        sqlx::query(
            r#"
            UPDATE crm_contacts SET
                lead_score = MAX(0, lead_score + ?2),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(score_delta)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), CrmContactError> {
        let result = sqlx::query(r#"DELETE FROM crm_contacts WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CrmContactError::NotFound);
        }

        Ok(())
    }

    /// Find or create a contact from an email address
    pub async fn find_or_create_from_email(
        pool: &SqlitePool,
        project_id: Uuid,
        email: &str,
        name: Option<&str>,
        source: ContactSource,
    ) -> Result<Self, CrmContactError> {
        if let Some(existing) = Self::find_by_email(pool, project_id, email).await? {
            return Ok(existing);
        }

        // Parse name into first/last if provided
        let (first_name, last_name) = if let Some(n) = name {
            let parts: Vec<&str> = n.trim().splitn(2, ' ').collect();
            match parts.as_slice() {
                [first] => (Some(first.to_string()), None),
                [first, last] => (Some(first.to_string()), Some(last.to_string())),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

        Self::create(pool, CreateCrmContact {
            project_id,
            first_name,
            last_name,
            email: Some(email.to_string()),
            phone: None,
            mobile: None,
            avatar_url: None,
            company_name: None,
            job_title: None,
            department: None,
            linkedin_url: None,
            twitter_handle: None,
            website: None,
            source: Some(source),
            lifecycle_stage: Some(LifecycleStage::Lead),
            tags: None,
            custom_fields: None,
            zoho_contact_id: None,
            gmail_contact_id: None,
        }).await
    }
}
