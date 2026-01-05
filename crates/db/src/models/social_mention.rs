use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SocialMentionError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Social mention not found")]
    NotFound,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MentionType {
    Comment,
    Mention,
    Dm,
    Reply,
    Quote,
    Tag,
    Review,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MentionStatus {
    Unread,
    Read,
    Replied,
    Archived,
    Flagged,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum MentionPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SocialMention {
    pub id: Uuid,
    pub social_account_id: Uuid,
    pub project_id: Uuid,
    pub mention_type: String,
    pub platform: String,
    pub platform_mention_id: String,
    pub author_username: Option<String>,
    pub author_display_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub author_follower_count: Option<i64>,
    pub author_is_verified: bool,
    pub content: Option<String>,
    pub media_urls: Option<String>,
    pub parent_post_id: Option<Uuid>,
    pub parent_platform_id: Option<String>,
    pub status: String,
    pub sentiment: Option<String>,
    pub priority: String,
    pub replied_at: Option<DateTime<Utc>>,
    pub replied_by: Option<String>,
    pub reply_content: Option<String>,
    pub assigned_agent_id: Option<Uuid>,
    pub auto_response_sent: bool,
    pub received_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSocialMention {
    pub social_account_id: Uuid,
    pub project_id: Uuid,
    pub mention_type: MentionType,
    pub platform: String,
    pub platform_mention_id: String,
    pub author_username: Option<String>,
    pub author_display_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub author_follower_count: Option<i64>,
    pub author_is_verified: Option<bool>,
    pub content: Option<String>,
    pub media_urls: Option<Vec<String>>,
    pub parent_post_id: Option<Uuid>,
    pub parent_platform_id: Option<String>,
    pub sentiment: Option<Sentiment>,
    pub priority: Option<MentionPriority>,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateSocialMention {
    pub status: Option<MentionStatus>,
    pub sentiment: Option<Sentiment>,
    pub priority: Option<MentionPriority>,
    pub replied_by: Option<String>,
    pub reply_content: Option<String>,
    pub assigned_agent_id: Option<Uuid>,
}

impl SocialMention {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateSocialMention,
    ) -> Result<Self, SocialMentionError> {
        let id = Uuid::new_v4();
        let mention_type = format!("{:?}", data.mention_type).to_lowercase();
        let sentiment = data.sentiment.map(|s| format!("{:?}", s).to_lowercase());
        let priority = data
            .priority
            .map(|p| format!("{:?}", p).to_lowercase())
            .unwrap_or_else(|| "normal".to_string());
        let media_urls = data.media_urls.map(|v| serde_json::to_string(&v).unwrap());
        let author_is_verified = data.author_is_verified.unwrap_or(false);

        let mention = sqlx::query_as::<_, SocialMention>(
            r#"
            INSERT INTO social_mentions (
                id, social_account_id, project_id, mention_type, platform,
                platform_mention_id, author_username, author_display_name,
                author_avatar_url, author_follower_count, author_is_verified,
                content, media_urls, parent_post_id, parent_platform_id,
                sentiment, priority, received_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.social_account_id)
        .bind(data.project_id)
        .bind(&mention_type)
        .bind(&data.platform)
        .bind(&data.platform_mention_id)
        .bind(&data.author_username)
        .bind(&data.author_display_name)
        .bind(&data.author_avatar_url)
        .bind(data.author_follower_count)
        .bind(author_is_verified)
        .bind(&data.content)
        .bind(media_urls)
        .bind(data.parent_post_id)
        .bind(&data.parent_platform_id)
        .bind(&sentiment)
        .bind(&priority)
        .bind(data.received_at)
        .fetch_one(pool)
        .await?;

        Ok(mention)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, SocialMentionError> {
        sqlx::query_as::<_, SocialMention>(r#"SELECT * FROM social_mentions WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(SocialMentionError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, SocialMentionError> {
        let limit = limit.unwrap_or(100);
        let mentions = sqlx::query_as::<_, SocialMention>(
            r#"
            SELECT * FROM social_mentions
            WHERE project_id = ?1
            ORDER BY received_at DESC
            LIMIT ?2
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(mentions)
    }

    pub async fn find_unread(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, SocialMentionError> {
        let mentions = sqlx::query_as::<_, SocialMention>(
            r#"
            SELECT * FROM social_mentions
            WHERE project_id = ?1 AND status = 'unread'
            ORDER BY priority DESC, received_at DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(mentions)
    }

    pub async fn find_high_priority(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, SocialMentionError> {
        let mentions = sqlx::query_as::<_, SocialMention>(
            r#"
            SELECT * FROM social_mentions
            WHERE project_id = ?1 AND priority IN ('high', 'urgent') AND status != 'archived'
            ORDER BY priority DESC, received_at DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(mentions)
    }

    pub async fn find_by_account(
        pool: &SqlitePool,
        account_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, SocialMentionError> {
        let limit = limit.unwrap_or(100);
        let mentions = sqlx::query_as::<_, SocialMention>(
            r#"
            SELECT * FROM social_mentions
            WHERE social_account_id = ?1
            ORDER BY received_at DESC
            LIMIT ?2
            "#,
        )
        .bind(account_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(mentions)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateSocialMention,
    ) -> Result<Self, SocialMentionError> {
        let status = data.status.map(|s| format!("{:?}", s).to_lowercase());
        let sentiment = data.sentiment.map(|s| format!("{:?}", s).to_lowercase());
        let priority = data.priority.map(|p| format!("{:?}", p).to_lowercase());

        let replied_at = data.reply_content.as_ref().map(|_| Utc::now());

        sqlx::query_as::<_, SocialMention>(
            r#"
            UPDATE social_mentions SET
                status = COALESCE(?2, status),
                sentiment = COALESCE(?3, sentiment),
                priority = COALESCE(?4, priority),
                replied_by = COALESCE(?5, replied_by),
                reply_content = COALESCE(?6, reply_content),
                replied_at = COALESCE(?7, replied_at),
                assigned_agent_id = COALESCE(?8, assigned_agent_id),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&status)
        .bind(&sentiment)
        .bind(&priority)
        .bind(&data.replied_by)
        .bind(&data.reply_content)
        .bind(replied_at)
        .bind(data.assigned_agent_id)
        .fetch_optional(pool)
        .await?
        .ok_or(SocialMentionError::NotFound)
    }

    pub async fn mark_read(pool: &SqlitePool, id: Uuid) -> Result<(), SocialMentionError> {
        sqlx::query(
            r#"
            UPDATE social_mentions SET
                status = 'read',
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1 AND status = 'unread'
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn count_unread(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<i64, SocialMentionError> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM social_mentions
            WHERE project_id = ?1 AND status = 'unread'
            "#,
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), SocialMentionError> {
        let result = sqlx::query(r#"DELETE FROM social_mentions WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(SocialMentionError::NotFound);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_utils::{
        create_test_project,
        create_test_social_account,
        setup_test_pool,
    };
    use chrono::Utc;

    #[tokio::test]
    async fn social_mention_inbox_flow() {
        let pool = setup_test_pool().await;
        let project_id = create_test_project(&pool).await;
        let account = create_test_social_account(&pool, project_id).await;

        let mention = SocialMention::create(
            &pool,
            CreateSocialMention {
                social_account_id: account.id,
                project_id,
                mention_type: MentionType::Comment,
                platform: "instagram".into(),
                platform_mention_id: "mention_1".into(),
                author_username: Some("pcgfan".into()),
                author_display_name: Some("PCG Fan".into()),
                author_avatar_url: None,
                author_follower_count: Some(5_000),
                author_is_verified: Some(true),
                content: Some("Loved the drop".into()),
                media_urls: Some(vec!["https://cdn.example.com/comment.png".into()]),
                parent_post_id: None,
                parent_platform_id: Some("post_1".into()),
                sentiment: Some(Sentiment::Positive),
                priority: Some(MentionPriority::High),
                received_at: Utc::now(),
            },
        )
        .await
        .expect("failed to create mention");

        assert_eq!(mention.status, "unread");
        assert_eq!(mention.priority, "high");

        let by_project = SocialMention::find_by_project(&pool, project_id, None)
            .await
            .expect("project lookup failed");
        assert_eq!(by_project.len(), 1);

        let unread = SocialMention::find_unread(&pool, project_id)
            .await
            .expect("unread lookup failed");
        assert_eq!(unread.len(), 1);

        let high_priority = SocialMention::find_high_priority(&pool, project_id)
            .await
            .expect("priority lookup failed");
        assert_eq!(high_priority.len(), 1);

        let by_account = SocialMention::find_by_account(&pool, account.id, None)
            .await
            .expect("account lookup failed");
        assert_eq!(by_account.len(), 1);

        let unread_count = SocialMention::count_unread(&pool, project_id)
            .await
            .expect("count failed");
        assert_eq!(unread_count, 1);

        SocialMention::mark_read(&pool, mention.id)
            .await
            .expect("mark read failed");

        let unread_after = SocialMention::find_unread(&pool, project_id)
            .await
            .expect("unread lookup failed");
        assert!(unread_after.is_empty());

        let updated = SocialMention::update(
            &pool,
            mention.id,
            UpdateSocialMention {
                status: Some(MentionStatus::Replied),
                sentiment: Some(Sentiment::Positive),
                priority: Some(MentionPriority::Normal),
                replied_by: Some("echo-agent".into()),
                reply_content: Some("Appreciate the love!".into()),
                assigned_agent_id: None,
            },
        )
        .await
        .expect("update failed");

        assert_eq!(updated.status, "replied");
        assert!(updated.replied_at.is_some());
        assert_eq!(updated.priority, "normal");
        assert_eq!(updated.reply_content.as_deref(), Some("Appreciate the love!"));

        let high_priority_after = SocialMention::find_high_priority(&pool, project_id)
            .await
            .expect("priority lookup failed");
        assert!(high_priority_after.is_empty());

        SocialMention::delete(&pool, mention.id)
            .await
            .expect("delete failed");

        let lookup = SocialMention::find_by_id(&pool, mention.id).await;
        assert!(matches!(lookup, Err(SocialMentionError::NotFound)));
    }
}
