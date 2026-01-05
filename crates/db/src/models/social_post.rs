use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SocialPostError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Social post not found")]
    NotFound,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Post,
    Story,
    Reel,
    Carousel,
    Thread,
    Video,
    Article,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    Draft,
    PendingReview,
    Approved,
    Scheduled,
    Publishing,
    Published,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SocialPost {
    pub id: Uuid,
    pub project_id: Uuid,
    pub social_account_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub content_type: String,
    pub caption: Option<String>,
    pub content_blocks: Option<String>,
    pub media_urls: Option<String>,
    pub hashtags: Option<String>,
    pub mentions: Option<String>,
    pub platforms: String,
    pub platform_specific: Option<String>,
    pub status: String,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub category: Option<String>,
    pub queue_position: Option<i64>,
    pub is_evergreen: bool,
    pub recycle_after_days: Option<i64>,
    pub last_recycled_at: Option<DateTime<Utc>>,
    pub created_by_agent_id: Option<Uuid>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub platform_post_id: Option<String>,
    pub platform_url: Option<String>,
    pub publish_error: Option<String>,
    pub impressions: i64,
    pub reach: i64,
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub saves: i64,
    pub clicks: i64,
    pub engagement_rate: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSocialPost {
    pub project_id: Uuid,
    pub social_account_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub content_type: Option<ContentType>,
    pub caption: Option<String>,
    pub content_blocks: Option<serde_json::Value>,
    pub media_urls: Option<Vec<String>>,
    pub hashtags: Option<Vec<String>>,
    pub mentions: Option<Vec<String>>,
    pub platforms: Vec<Uuid>,
    pub platform_specific: Option<serde_json::Value>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub category: Option<String>,
    pub is_evergreen: Option<bool>,
    pub recycle_after_days: Option<i64>,
    pub created_by_agent_id: Option<Uuid>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateSocialPost {
    pub caption: Option<String>,
    pub content_blocks: Option<serde_json::Value>,
    pub media_urls: Option<Vec<String>>,
    pub hashtags: Option<Vec<String>>,
    pub mentions: Option<Vec<String>>,
    pub platform_specific: Option<serde_json::Value>,
    pub status: Option<PostStatus>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub category: Option<String>,
    pub queue_position: Option<i64>,
    pub is_evergreen: Option<bool>,
    pub approved_by: Option<String>,
}

impl SocialPost {
    pub async fn create(pool: &SqlitePool, data: CreateSocialPost) -> Result<Self, SocialPostError> {
        let id = Uuid::new_v4();
        let content_type = data
            .content_type
            .map(|t| format!("{:?}", t).to_lowercase())
            .unwrap_or_else(|| "post".to_string());
        let content_blocks = data.content_blocks.map(|v| v.to_string());
        let media_urls = data.media_urls.map(|v| serde_json::to_string(&v).unwrap());
        let hashtags = data.hashtags.map(|v| serde_json::to_string(&v).unwrap());
        let mentions = data.mentions.map(|v| serde_json::to_string(&v).unwrap());
        let platforms = serde_json::to_string(&data.platforms).unwrap();
        let platform_specific = data.platform_specific.map(|v| v.to_string());
        let is_evergreen = data.is_evergreen.unwrap_or(false);

        let post = sqlx::query_as::<_, SocialPost>(
            r#"
            INSERT INTO social_posts (
                id, project_id, social_account_id, task_id, content_type,
                caption, content_blocks, media_urls, hashtags, mentions,
                platforms, platform_specific, scheduled_for, category,
                is_evergreen, recycle_after_days, created_by_agent_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(data.social_account_id)
        .bind(data.task_id)
        .bind(&content_type)
        .bind(&data.caption)
        .bind(content_blocks)
        .bind(media_urls)
        .bind(hashtags)
        .bind(mentions)
        .bind(&platforms)
        .bind(platform_specific)
        .bind(data.scheduled_for)
        .bind(&data.category)
        .bind(is_evergreen)
        .bind(data.recycle_after_days)
        .bind(data.created_by_agent_id)
        .fetch_one(pool)
        .await?;

        Ok(post)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, SocialPostError> {
        sqlx::query_as::<_, SocialPost>(r#"SELECT * FROM social_posts WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(SocialPostError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: Option<i64>,
    ) -> Result<Vec<Self>, SocialPostError> {
        let limit = limit.unwrap_or(100);
        let posts = sqlx::query_as::<_, SocialPost>(
            r#"
            SELECT * FROM social_posts
            WHERE project_id = ?1
            ORDER BY created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(posts)
    }

    pub async fn find_scheduled(
        pool: &SqlitePool,
        project_id: Option<Uuid>,
    ) -> Result<Vec<Self>, SocialPostError> {
        let posts = if let Some(pid) = project_id {
            sqlx::query_as::<_, SocialPost>(
                r#"
                SELECT * FROM social_posts
                WHERE status = 'scheduled' AND project_id = ?1
                ORDER BY scheduled_for ASC
                "#,
            )
            .bind(pid)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, SocialPost>(
                r#"
                SELECT * FROM social_posts
                WHERE status = 'scheduled'
                ORDER BY scheduled_for ASC
                "#,
            )
            .fetch_all(pool)
            .await?
        };

        Ok(posts)
    }

    pub async fn find_due_for_publish(pool: &SqlitePool) -> Result<Vec<Self>, SocialPostError> {
        let posts = sqlx::query_as::<_, SocialPost>(
            r#"
            SELECT * FROM social_posts
            WHERE status = 'scheduled'
            AND datetime(scheduled_for) <= datetime('now')
            ORDER BY scheduled_for ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(posts)
    }

    pub async fn find_by_category(
        pool: &SqlitePool,
        project_id: Uuid,
        category: &str,
    ) -> Result<Vec<Self>, SocialPostError> {
        let posts = sqlx::query_as::<_, SocialPost>(
            r#"
            SELECT * FROM social_posts
            WHERE project_id = ?1 AND category = ?2
            ORDER BY queue_position ASC, created_at DESC
            "#,
        )
        .bind(project_id)
        .bind(category)
        .fetch_all(pool)
        .await?;

        Ok(posts)
    }

    pub async fn find_evergreen(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, SocialPostError> {
        let posts = sqlx::query_as::<_, SocialPost>(
            r#"
            SELECT * FROM social_posts
            WHERE project_id = ?1 AND is_evergreen = 1
            ORDER BY last_recycled_at ASC NULLS FIRST
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(posts)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateSocialPost,
    ) -> Result<Self, SocialPostError> {
        let status = data.status.map(|s| format!("{:?}", s).to_lowercase());
        let content_blocks = data.content_blocks.map(|v| v.to_string());
        let media_urls = data.media_urls.map(|v| serde_json::to_string(&v).unwrap());
        let hashtags = data.hashtags.map(|v| serde_json::to_string(&v).unwrap());
        let mentions = data.mentions.map(|v| serde_json::to_string(&v).unwrap());
        let platform_specific = data.platform_specific.map(|v| v.to_string());

        let approved_at = data.approved_by.as_ref().map(|_| Utc::now());

        sqlx::query_as::<_, SocialPost>(
            r#"
            UPDATE social_posts SET
                caption = COALESCE(?2, caption),
                content_blocks = COALESCE(?3, content_blocks),
                media_urls = COALESCE(?4, media_urls),
                hashtags = COALESCE(?5, hashtags),
                mentions = COALESCE(?6, mentions),
                platform_specific = COALESCE(?7, platform_specific),
                status = COALESCE(?8, status),
                scheduled_for = COALESCE(?9, scheduled_for),
                category = COALESCE(?10, category),
                queue_position = COALESCE(?11, queue_position),
                is_evergreen = COALESCE(?12, is_evergreen),
                approved_by = COALESCE(?13, approved_by),
                approved_at = COALESCE(?14, approved_at),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.caption)
        .bind(content_blocks)
        .bind(media_urls)
        .bind(hashtags)
        .bind(mentions)
        .bind(platform_specific)
        .bind(&status)
        .bind(data.scheduled_for)
        .bind(&data.category)
        .bind(data.queue_position)
        .bind(data.is_evergreen)
        .bind(&data.approved_by)
        .bind(approved_at)
        .fetch_optional(pool)
        .await?
        .ok_or(SocialPostError::NotFound)
    }

    pub async fn mark_published(
        pool: &SqlitePool,
        id: Uuid,
        platform_post_id: &str,
        platform_url: Option<&str>,
    ) -> Result<Self, SocialPostError> {
        sqlx::query_as::<_, SocialPost>(
            r#"
            UPDATE social_posts SET
                status = 'published',
                platform_post_id = ?2,
                platform_url = ?3,
                published_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(platform_post_id)
        .bind(platform_url)
        .fetch_optional(pool)
        .await?
        .ok_or(SocialPostError::NotFound)
    }

    pub async fn mark_failed(
        pool: &SqlitePool,
        id: Uuid,
        error: &str,
    ) -> Result<(), SocialPostError> {
        sqlx::query(
            r#"
            UPDATE social_posts SET
                status = 'failed',
                publish_error = ?2,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn update_metrics(
        pool: &SqlitePool,
        id: Uuid,
        impressions: i64,
        reach: i64,
        likes: i64,
        comments: i64,
        shares: i64,
        saves: i64,
        clicks: i64,
    ) -> Result<(), SocialPostError> {
        let total_engagement = (likes + comments + shares + saves) as f64;
        let engagement_rate = if impressions > 0 {
            total_engagement / impressions as f64
        } else {
            0.0
        };

        sqlx::query(
            r#"
            UPDATE social_posts SET
                impressions = ?2,
                reach = ?3,
                likes = ?4,
                comments = ?5,
                shares = ?6,
                saves = ?7,
                clicks = ?8,
                engagement_rate = ?9,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            "#,
        )
        .bind(id)
        .bind(impressions)
        .bind(reach)
        .bind(likes)
        .bind(comments)
        .bind(shares)
        .bind(saves)
        .bind(clicks)
        .bind(engagement_rate)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), SocialPostError> {
        let result = sqlx::query(r#"DELETE FROM social_posts WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(SocialPostError::NotFound);
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
    use chrono::{Duration, Utc};
    use serde_json::json;

    #[tokio::test]
    async fn social_post_lifecycle() {
        let pool = setup_test_pool().await;
        let project_id = create_test_project(&pool).await;
        let account = create_test_social_account(&pool, project_id).await;
        let initial_time = Utc::now() + Duration::hours(2);

        let created = SocialPost::create(
            &pool,
            CreateSocialPost {
                project_id,
                social_account_id: Some(account.id),
                task_id: None,
                content_type: Some(ContentType::Carousel),
                caption: Some("Launch teaser".into()),
                content_blocks: Some(json!({
                    "hook": "Sneak peek",
                    "body": "Highlighting the new launch",
                })),
                media_urls: Some(vec!["https://cdn.example.com/post.png".into()]),
                hashtags: Some(vec!["#pcg".into(), "#social".into()]),
                mentions: Some(vec!["@prime".into()]),
                platforms: vec![account.id],
                platform_specific: Some(json!({"instagram": {"cta": "RSVP"}})),
                scheduled_for: Some(initial_time),
                category: Some("launch".into()),
                is_evergreen: Some(true),
                recycle_after_days: Some(14),
                created_by_agent_id: None,
            },
        )
        .await
        .expect("failed to create post");

        assert_eq!(created.status, "draft");
        assert!(created.is_evergreen);

        let by_project = SocialPost::find_by_project(&pool, project_id, None)
            .await
            .expect("project lookup failed");
        assert_eq!(by_project.len(), 1);

        let by_category = SocialPost::find_by_category(&pool, project_id, "launch")
            .await
            .expect("category lookup failed");
        assert_eq!(by_category.len(), 1);

        let evergreen = SocialPost::find_evergreen(&pool, project_id)
            .await
            .expect("evergreen lookup failed");
        assert_eq!(evergreen.len(), 1);

        let scheduled_time = Utc::now() - Duration::minutes(10);
        let updated = SocialPost::update(
            &pool,
            created.id,
            UpdateSocialPost {
                status: Some(PostStatus::Scheduled),
                scheduled_for: Some(scheduled_time),
                queue_position: Some(1),
                is_evergreen: Some(false),
                approved_by: Some("nora".into()),
                ..Default::default()
            },
        )
        .await
        .expect("update failed");

        assert_eq!(updated.status, "scheduled");
        assert_eq!(updated.queue_position, Some(1));
        assert!(!updated.is_evergreen);
        assert!(updated.approved_at.is_some());

        let scheduled = SocialPost::find_scheduled(&pool, Some(project_id))
            .await
            .expect("scheduled lookup failed");
        assert_eq!(scheduled.len(), 1);

        let due = SocialPost::find_due_for_publish(&pool)
            .await
            .expect("due lookup failed");
        assert_eq!(due.len(), 1);

        // Evergreen list should now be empty after toggle
        let evergreen_after = SocialPost::find_evergreen(&pool, project_id)
            .await
            .expect("evergreen lookup failed");
        assert!(evergreen_after.is_empty());

        SocialPost::update_metrics(&pool, created.id, 1_000, 800, 120, 40, 25, 15, 60)
            .await
            .expect("metrics update failed");

        let metrics = SocialPost::find_by_id(&pool, created.id)
            .await
            .expect("metrics lookup failed");
        let expected_rate = (120.0 + 40.0 + 25.0 + 15.0) / 1_000.0;
        assert!((metrics.engagement_rate - expected_rate).abs() < f64::EPSILON);
        assert_eq!(metrics.likes, 120);
        assert_eq!(metrics.reach, 800);

        let published = SocialPost::mark_published(
            &pool,
            created.id,
            "ig_post_42",
            Some("https://instagram.com/p/ig_post_42"),
        )
        .await
        .expect("publish failed");

        assert_eq!(published.status, "published");
        assert_eq!(published.platform_post_id.as_deref(), Some("ig_post_42"));
        assert!(published.published_at.is_some());

        SocialPost::delete(&pool, created.id)
            .await
            .expect("delete failed");

        let lookup = SocialPost::find_by_id(&pool, created.id).await;
        assert!(matches!(lookup, Err(SocialPostError::NotFound)));
    }
}
