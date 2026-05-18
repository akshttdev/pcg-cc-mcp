pub mod competitor_analyzer;
pub mod kol_signal_extractor;
pub mod mention_clusterer;
pub mod news_event_detector;
pub mod opportunity_drafter;
pub mod share_of_voice_calculator;
pub mod snapshot_analyzer;
pub mod trend_detector;

pub use competitor_analyzer::CompetitorAnalyzer;
pub use kol_signal_extractor::KolSignalExtractor;
pub use mention_clusterer::MentionClusterer;
pub use news_event_detector::NewsEventDetector;
pub use opportunity_drafter::OpportunityDrafter;
pub use share_of_voice_calculator::ShareOfVoiceCalculator;
pub use snapshot_analyzer::SnapshotAnalyzer;
use sqlx::SqlitePool;
pub use trend_detector::TrendDetector;

/// Entry point that wires all processors. Wired into the automation loop in main.rs.
pub struct IntelligenceEngine {
    pool: SqlitePool,
}

impl IntelligenceEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Run processors that fire on new PulseContentItem ingest.
    pub async fn process_new_content_item(&self, item_id: &str) {
        let trend = TrendDetector::new(self.pool.clone());
        if let Err(e) = trend.process_item(item_id).await {
            tracing::warn!("TrendDetector error for item {item_id}: {e}");
        }
        let news = NewsEventDetector::new(self.pool.clone());
        if let Err(e) = news.process_item(item_id).await {
            tracing::warn!("NewsEventDetector error for item {item_id}: {e}");
        }
        let kol = KolSignalExtractor::new(self.pool.clone());
        if let Err(e) = kol.process_item(item_id).await {
            tracing::warn!("KolSignalExtractor error for item {item_id}: {e}");
        }
    }

    /// Run processors on a cadence (hourly/daily). Called from automation loop.
    pub async fn run_periodic_processors(&self) {
        let clusterer = MentionClusterer::new(self.pool.clone());
        if let Err(e) = clusterer.run_all_orgs().await {
            tracing::warn!("MentionClusterer error: {e}");
        }

        let snapshot = SnapshotAnalyzer::new(self.pool.clone());
        if let Err(e) = snapshot.run_all_accounts().await {
            tracing::warn!("SnapshotAnalyzer error: {e}");
        }
    }

    /// Run weekly competitive analysis. Called from weekly automation.
    pub async fn run_weekly_processors(&self) {
        let competitor = CompetitorAnalyzer::new(self.pool.clone());
        if let Err(e) = competitor.run_all_orgs().await {
            tracing::warn!("CompetitorAnalyzer error: {e}");
        }

        let sov = ShareOfVoiceCalculator::new(self.pool.clone());
        if let Err(e) = sov.run_all_orgs().await {
            tracing::warn!("ShareOfVoiceCalculator error: {e}");
        }
    }
}
