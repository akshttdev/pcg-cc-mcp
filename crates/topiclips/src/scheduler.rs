//! Scheduler - Daily generation scheduling (Beeple-style)
//!
//! Manages daily clip generation schedules, streak tracking,
//! and background generation tasks.

use anyhow::Result;
use chrono::{Duration, NaiveTime, Timelike, Utc};
use db::models::topiclip::{TopiClipDailySchedule, TopiClipTriggerType};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::{TopiClipGenerator, TopiClipsService};

/// Scheduler for daily TopiClip generation
pub struct TopiClipsScheduler {
    pool: SqlitePool,
    service: Arc<Mutex<TopiClipsService>>,
    is_running: Arc<Mutex<bool>>,
}

impl TopiClipsScheduler {
    pub fn new(pool: SqlitePool, service: TopiClipsService) -> Self {
        Self {
            pool,
            service: Arc::new(Mutex::new(service)),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the scheduler background task
    pub async fn start(&self) -> Result<()> {
        let mut running = self.is_running.lock().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        let pool = self.pool.clone();
        let service = self.service.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            info!("TopiClips scheduler started");

            loop {
                // Check if still running
                let running = is_running.lock().await;
                if !*running {
                    break;
                }
                drop(running);

                // Process due schedules
                if let Err(e) = Self::process_due_schedules(&pool, &service).await {
                    error!("Error processing TopiClips schedules: {}", e);
                }

                // Sleep for 1 minute before next check
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }

            info!("TopiClips scheduler stopped");
        });

        Ok(())
    }

    /// Stop the scheduler
    pub async fn stop(&self) {
        let mut running = self.is_running.lock().await;
        *running = false;
    }

    /// Process all due schedules
    async fn process_due_schedules(
        pool: &SqlitePool,
        service: &Arc<Mutex<TopiClipsService>>,
    ) -> Result<()> {
        let schedules = TopiClipDailySchedule::list_enabled(pool).await?;
        let now = Utc::now();
        let today = now.format("%Y-%m-%d").to_string();

        for schedule in schedules {
            // Check if already generated today
            if schedule.last_generation_date.as_ref() == Some(&today) {
                continue;
            }

            // Parse scheduled time
            let scheduled_time = match NaiveTime::parse_from_str(&schedule.scheduled_time, "%H:%M")
            {
                Ok(t) => t,
                Err(e) => {
                    error!(
                        "Invalid scheduled time '{}' for project {}: {}",
                        schedule.scheduled_time, schedule.project_id, e
                    );
                    continue;
                }
            };

            // Get current time in schedule's timezone (simplified: assume UTC for now)
            let current_time = now.time();

            // Check if it's past the scheduled time
            if current_time >= scheduled_time {
                info!(
                    "Triggering daily TopiClip for project {}",
                    schedule.project_id
                );

                // Generate the clip
                let service_guard = service.lock().await;
                match Self::generate_daily_clip(&service_guard, schedule.project_id).await {
                    Ok(_) => {
                        info!(
                            "Daily TopiClip generated for project {}",
                            schedule.project_id
                        );
                    }
                    Err(e) => {
                        error!(
                            "Failed to generate daily TopiClip for project {}: {}",
                            schedule.project_id, e
                        );

                        // Check if we should reset streak (missed day)
                        if let Some(last_date) = &schedule.last_generation_date {
                            let yesterday = (now - Duration::days(1)).format("%Y-%m-%d").to_string();
                            if last_date != &yesterday {
                                // Streak broken
                                if let Err(e) =
                                    TopiClipDailySchedule::reset_streak(pool, schedule.id).await
                                {
                                    error!("Failed to reset streak: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Generate a daily clip for a project
    async fn generate_daily_clip(
        service: &TopiClipsService,
        project_id: uuid::Uuid,
    ) -> Result<()> {
        // Calculate period (last 24 hours)
        let end = Utc::now();
        let start = end - Duration::hours(24);

        // Create session
        let session = service
            .create_session(
                project_id,
                TopiClipTriggerType::Daily,
                Some(start.to_rfc3339()),
                Some(end.to_rfc3339()),
            )
            .await?;

        // Generate
        service.generate(session.id).await?;

        Ok(())
    }

    /// Check all schedules and return those that are due
    pub async fn get_due_schedules(&self) -> Result<Vec<TopiClipDailySchedule>> {
        let schedules = TopiClipDailySchedule::list_enabled(&self.pool).await?;
        let now = Utc::now();
        let today = now.format("%Y-%m-%d").to_string();
        let current_time = now.time();

        let due: Vec<TopiClipDailySchedule> = schedules
            .into_iter()
            .filter(|s| {
                // Not generated today
                if s.last_generation_date.as_ref() == Some(&today) {
                    return false;
                }
                // Past scheduled time
                if let Ok(scheduled_time) = NaiveTime::parse_from_str(&s.scheduled_time, "%H:%M") {
                    return current_time >= scheduled_time;
                }
                false
            })
            .collect();

        Ok(due)
    }

    /// Manually trigger daily generation for a project (force)
    pub async fn force_daily_generation(&self, project_id: uuid::Uuid) -> Result<()> {
        let service = self.service.lock().await;
        Self::generate_daily_clip(&service, project_id).await
    }

    /// Get streak info for a project
    pub async fn get_streak_info(
        &self,
        project_id: uuid::Uuid,
    ) -> Result<Option<StreakInfo>> {
        let schedule = TopiClipDailySchedule::find_by_project(&self.pool, project_id).await?;

        Ok(schedule.map(|s| StreakInfo {
            current_streak: s.current_streak,
            longest_streak: s.longest_streak,
            total_clips: s.total_clips_generated,
            last_generation: s.last_generation_date,
            is_active: s.is_enabled,
        }))
    }
}

/// Streak information for a project
#[derive(Debug, Clone)]
pub struct StreakInfo {
    pub current_streak: i64,
    pub longest_streak: i64,
    pub total_clips: i64,
    pub last_generation: Option<String>,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_parsing() {
        let time = NaiveTime::parse_from_str("09:00", "%H:%M");
        assert!(time.is_ok());
        assert_eq!(time.unwrap().hour(), 9);
        assert_eq!(time.unwrap().minute(), 0);
    }
}
