use std::time::Duration;

use anyhow::Context;
use chrono::{Duration as ChronoDuration, Utc};
use db::models::dropbox_source::{render_reference_name, DropboxSource};
use services::services::media_pipeline::{
    MediaBatchIngestRequest,
    MediaPipelineService,
    MediaStorageTier,
};
use sqlx::SqlitePool;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct DropboxMonitor {
    db_pool: SqlitePool,
    media_pipeline: MediaPipelineService,
    refresh_interval: Duration,
    stale_after: ChronoDuration,
}

impl DropboxMonitor {
    pub fn spawn(db_pool: SqlitePool, media_pipeline: MediaPipelineService) {
        let monitor = Self {
            db_pool,
            media_pipeline,
            refresh_interval: Duration::from_secs(300),
            stale_after: ChronoDuration::hours(12),
        };

        tokio::spawn(async move {
            monitor.run().await;
        });
    }

    async fn run(self) {
        loop {
            if let Err(err) = self.tick().await {
                warn!("Dropbox monitor tick failed: {err:?}");
            }
            sleep(self.refresh_interval).await;
        }
    }

    async fn tick(&self) -> anyhow::Result<()> {
        let sources = DropboxSource::list(&self.db_pool).await?;
        for source in sources {
            if !source.auto_ingest {
                continue;
            }

            if !self.needs_refresh(&source) {
                continue;
            }

            match self.ingest_source(&source).await {
                Ok(_) => {
                    DropboxSource::mark_processed(
                        &self.db_pool,
                        source.id,
                        source.cursor.clone(),
                        Utc::now(),
                    )
                    .await?;
                }
                Err(err) => {
                    warn!(
                        "Auto ingest for Dropbox source {} failed: {err:?}",
                        source.label
                    );
                }
            }
        }
        Ok(())
    }

    fn needs_refresh(&self, source: &DropboxSource) -> bool {
        match source.last_processed_at {
            Some(ts) => Utc::now() - ts > self.stale_after,
            None => true,
        }
    }

    async fn ingest_source(&self, source: &DropboxSource) -> anyhow::Result<()> {
        let url = source
            .source_url
            .clone()
            .context("Dropbox source missing source_url")?;
        let storage_tier = MediaStorageTier::from_str(&source.storage_tier)
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;

        let request = MediaBatchIngestRequest {
            source_url: url,
            reference_name: Some(render_reference_name(
                source.reference_name_template.as_deref(),
                &source.label,
            )),
            storage_tier,
            checksum_required: source.checksum_required,
            project_id: source.project_id,
        };

        match self.media_pipeline.ingest_batch(request).await {
            Ok(batch) => {
                info!(
                    "Dropbox monitor queued ingest for {} -> batch {}",
                    source.label, batch.id
                );
                Ok(())
            }
            Err(err) => Err(anyhow::anyhow!(err)),
        }
    }
}
