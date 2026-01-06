//! Social Media Scheduler Service
//!
//! Manages content scheduling with support for:
//! - Category-based queues (GoHighLevel style)
//! - Optimal time scheduling
//! - Evergreen content rotation

use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::info;
use uuid::Uuid;

use db::models::social_post::{SocialPost, UpdateSocialPost, PostStatus};

/// Schedule rule for category queues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRule {
    /// Days of the week (0 = Sunday, 6 = Saturday)
    pub days: Vec<u8>,
    /// Times of day (HH:MM format)
    pub times: Vec<String>,
    /// Timezone (e.g., "America/Denver")
    pub timezone: String,
}

/// Scheduler configuration
pub struct SchedulerConfig {
    /// Minimum hours between posts to the same account
    pub min_hours_between_posts: u32,
    /// Maximum posts per day per account
    pub max_posts_per_day: u32,
    /// Days to look ahead for scheduling
    pub scheduling_horizon_days: u32,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            min_hours_between_posts: 4,
            max_posts_per_day: 3,
            scheduling_horizon_days: 30,
        }
    }
}

/// Social Media Scheduler
pub struct Scheduler {
    pool: SqlitePool,
    config: SchedulerConfig,
}

impl Scheduler {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            config: SchedulerConfig::default(),
        }
    }

    pub fn with_config(pool: SqlitePool, config: SchedulerConfig) -> Self {
        Self { pool, config }
    }

    /// Schedule a post for optimal time based on category
    pub async fn schedule_post(
        &self,
        post_id: Uuid,
        category: Option<&str>,
        preferred_date: Option<DateTime<Utc>>,
    ) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
        let post = SocialPost::find_by_id(&self.pool, post_id).await?;

        // Get available slots
        let scheduled_time = self
            .find_next_available_slot(&post, category, preferred_date)
            .await?;

        // Update the post with scheduled time
        SocialPost::update(
            &self.pool,
            post_id,
            UpdateSocialPost {
                scheduled_for: Some(scheduled_time),
                status: Some(PostStatus::Scheduled),
                category: category.map(|s| s.to_string()),
                ..Default::default()
            },
        )
        .await?;

        info!("Scheduled post {} for {}", post_id, scheduled_time);

        Ok(scheduled_time)
    }

    /// Find the next available slot considering existing scheduled posts
    async fn find_next_available_slot(
        &self,
        post: &SocialPost,
        category: Option<&str>,
        preferred_date: Option<DateTime<Utc>>,
    ) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
        // Get existing scheduled posts for the same project
        let existing = SocialPost::find_scheduled(&self.pool, Some(post.project_id)).await?;

        let start_date = preferred_date.unwrap_or_else(Utc::now);

        // Get optimal times based on category or use defaults
        let optimal_times = self.get_optimal_times(category);

        // Find next available slot
        for day_offset in 0..self.config.scheduling_horizon_days {
            let date = start_date + chrono::Duration::days(day_offset as i64);

            for time_str in &optimal_times {
                let scheduled_time = self.combine_date_time(&date, time_str)?;

                // Skip if in the past
                if scheduled_time < Utc::now() {
                    continue;
                }

                // Check if slot is available
                if self.is_slot_available(&existing, scheduled_time) {
                    return Ok(scheduled_time);
                }
            }
        }

        // If no slot found, just schedule for tomorrow at first optimal time
        let tomorrow = Utc::now() + chrono::Duration::days(1);
        let default_time = "10:00".to_string();
        let first_time = optimal_times.first().unwrap_or(&default_time);
        self.combine_date_time(&tomorrow, first_time)
    }

    /// Check if a time slot is available
    fn is_slot_available(&self, existing: &[SocialPost], proposed_time: DateTime<Utc>) -> bool {
        let min_gap = chrono::Duration::hours(self.config.min_hours_between_posts as i64);

        for post in existing {
            if let Some(scheduled) = post.scheduled_for {
                let diff = if proposed_time > scheduled {
                    proposed_time - scheduled
                } else {
                    scheduled - proposed_time
                };

                if diff < min_gap {
                    return false;
                }
            }
        }

        // Check daily limit
        let posts_on_day = existing
            .iter()
            .filter(|p| {
                p.scheduled_for
                    .map(|s| s.date_naive() == proposed_time.date_naive())
                    .unwrap_or(false)
            })
            .count();

        posts_on_day < self.config.max_posts_per_day as usize
    }

    /// Get optimal posting times for a category
    fn get_optimal_times(&self, category: Option<&str>) -> Vec<String> {
        // Default optimal times based on general social media research
        // These could be customized per category
        match category {
            Some("events") => vec!["18:00".to_string(), "19:00".to_string(), "12:00".to_string()],
            Some("community") => vec!["10:00".to_string(), "15:00".to_string(), "20:00".to_string()],
            Some("drinks") => vec!["17:00".to_string(), "18:00".to_string(), "21:00".to_string()],
            Some("vibe") => vec!["11:00".to_string(), "14:00".to_string(), "19:00".to_string()],
            _ => vec![
                "09:00".to_string(),
                "12:00".to_string(),
                "15:00".to_string(),
                "18:00".to_string(),
            ],
        }
    }

    /// Combine a date and time string into a DateTime
    fn combine_date_time(
        &self,
        date: &DateTime<Utc>,
        time_str: &str,
    ) -> Result<DateTime<Utc>, Box<dyn std::error::Error + Send + Sync>> {
        let parts: Vec<&str> = time_str.split(':').collect();
        let hour: u32 = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(12);
        let minute: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

        let naive_date = date.date_naive();
        let naive_time = NaiveTime::from_hms_opt(hour, minute, 0)
            .ok_or("Invalid time")?;
        let naive_datetime = naive_date.and_time(naive_time);

        Ok(DateTime::from_naive_utc_and_offset(naive_datetime, Utc))
    }

    /// Process evergreen content rotation
    pub async fn rotate_evergreen_content(
        &self,
        project_id: Uuid,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let evergreen_posts = SocialPost::find_evergreen(&self.pool, project_id).await?;

        let mut scheduled_count = 0;

        for post in evergreen_posts {
            // Check if enough time has passed since last recycle
            let can_recycle = match (post.last_recycled_at, post.recycle_after_days) {
                (Some(last), Some(days)) => {
                    let days_since = (Utc::now() - last).num_days();
                    days_since >= days
                }
                (None, _) => true, // Never recycled, can schedule
                _ => true,
            };

            if can_recycle {
                // Schedule the evergreen post
                if let Ok(time) = self
                    .schedule_post(post.id, post.category.as_deref(), None)
                    .await
                {
                    info!(
                        "Recycled evergreen post {} scheduled for {}",
                        post.id, time
                    );
                    scheduled_count += 1;
                }
            }
        }

        Ok(scheduled_count)
    }

    /// Get upcoming scheduled posts for a project
    pub async fn get_upcoming(
        &self,
        project_id: Uuid,
        days: u32,
    ) -> Result<Vec<SocialPost>, Box<dyn std::error::Error + Send + Sync>> {
        let all_scheduled = SocialPost::find_scheduled(&self.pool, Some(project_id)).await?;

        let cutoff = Utc::now() + chrono::Duration::days(days as i64);

        let upcoming: Vec<SocialPost> = all_scheduled
            .into_iter()
            .filter(|p| {
                p.scheduled_for
                    .map(|s| s <= cutoff)
                    .unwrap_or(false)
            })
            .collect();

        Ok(upcoming)
    }
}
