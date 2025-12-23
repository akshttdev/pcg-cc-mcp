use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use db::DBService;
#[cfg(feature = "postgres")]
use db::PgDBService;
use deployment::{Deployment, DeploymentError};
use executors::profile::ExecutorConfigs;
use services::services::{
    analytics::{AnalyticsConfig, AnalyticsContext, AnalyticsService, generate_user_id},
    approvals::Approvals,
    auth::AuthService,
    config::{Config, load_config_from_file, save_config_to_file},
    container::ContainerService,
    events::EventService,
    file_search_cache::FileSearchCache,
    filesystem::FilesystemService,
    git::GitService,
    image::ImageService,
    media_pipeline::MediaPipelineService,
    sentry::SentryService,
};
#[cfg(feature = "postgres")]
use sqlx::{Pool, Postgres};
use tokio::sync::RwLock;
use utils::{
    assets::{asset_dir, config_path},
    msg_store::MsgStore,
};
use uuid::Uuid;

use crate::container::LocalContainerService;

mod command;
pub mod container;

#[derive(Clone)]
pub struct LocalDeployment {
    config: Arc<RwLock<Config>>,
    sentry: SentryService,
    user_id: String,
    db: DBService,
    #[cfg(feature = "postgres")]
    pg_db: Option<PgDBService>,
    analytics: Option<AnalyticsService>,
    msg_stores: Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>>,
    container: LocalContainerService,
    git: GitService,
    auth: AuthService,
    image: ImageService,
    filesystem: FilesystemService,
    events: EventService,
    file_search_cache: Arc<FileSearchCache>,
    approvals: Approvals,
    media_pipeline: MediaPipelineService,
}

#[async_trait]
impl Deployment for LocalDeployment {
    async fn new() -> Result<Self, DeploymentError> {
        let mut raw_config = load_config_from_file(&config_path()).await;

        let profiles = ExecutorConfigs::get_cached();
        if !raw_config.onboarding_acknowledged
            && let Ok(recommended_executor) = profiles.get_recommended_executor_profile().await
        {
            raw_config.executor_profile = recommended_executor;
        }

        // Check if app version has changed and set release notes flag
        {
            let current_version = utils::version::APP_VERSION;
            let stored_version = raw_config.last_app_version.as_deref();

            if stored_version != Some(current_version) {
                // Show release notes only if this is an upgrade (not first install)
                raw_config.show_release_notes = stored_version.is_some();
                raw_config.last_app_version = Some(current_version.to_string());
            }
        }

        // Always save config (may have been migrated or version updated)
        save_config_to_file(&raw_config, &config_path()).await?;

        let config = Arc::new(RwLock::new(raw_config));
        let sentry = SentryService::new();
        let user_id = generate_user_id();
        let analytics = AnalyticsConfig::new().map(AnalyticsService::new);
        let git = GitService::new();
        let msg_stores = Arc::new(RwLock::new(HashMap::new()));
        let auth = AuthService::new();
        let filesystem = FilesystemService::new();

        // Create shared components for EventService
        let events_msg_store = Arc::new(MsgStore::new());
        let events_entry_count = Arc::new(RwLock::new(0));

        // Create DB with event hooks
        let db = {
            let hook = EventService::create_hook(
                events_msg_store.clone(),
                events_entry_count.clone(),
                DBService::new().await?, // Temporary DB service for the hook
            );
            DBService::new_with_after_connect(hook).await?
        };

        let image = ImageService::new(db.clone().pool)?;
        {
            let image_service = image.clone();
            tokio::spawn(async move {
                tracing::info!("Starting orphaned image cleanup...");
                if let Err(e) = image_service.delete_orphaned_images().await {
                    tracing::error!("Failed to clean up orphaned images: {}", e);
                }
            });
        }

        let approvals = Approvals::new(db.pool.clone(), msg_stores.clone());

        // We need to make analytics accessible to the ContainerService
        // TODO: Handle this more gracefully
        let analytics_ctx = analytics.as_ref().map(|s| AnalyticsContext {
            user_id: user_id.clone(),
            analytics_service: s.clone(),
        });
        let container = LocalContainerService::new(
            db.clone(),
            msg_stores.clone(),
            config.clone(),
            git.clone(),
            image.clone(),
            analytics_ctx,
        );
        container.spawn_worktree_cleanup().await;

        let events = EventService::new(db.clone(), events_msg_store, events_entry_count);
        let file_search_cache = Arc::new(FileSearchCache::new());

        let media_pipeline_root = asset_dir().join("media_pipeline");
        let media_pipeline = MediaPipelineService::new(media_pipeline_root)?;

        // Try to initialize PostgreSQL connection if DATABASE_URL is set
        #[cfg(feature = "postgres")]
        let pg_db = {
            match std::env::var("DATABASE_URL") {
                Ok(_) => {
                    tracing::info!("DATABASE_URL found, initializing PostgreSQL connection");
                    match PgDBService::new().await {
                        Ok(pg) => {
                            tracing::info!("PostgreSQL connection established");
                            Some(pg)
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to connect to PostgreSQL: {}. Multi-user auth will be disabled.",
                                e
                            );
                            None
                        }
                    }
                }
                Err(_) => {
                    tracing::info!("DATABASE_URL not set, skipping PostgreSQL initialization");
                    None
                }
            }
        };

        Ok(Self {
            config,
            sentry,
            user_id,
            db,
            #[cfg(feature = "postgres")]
            pg_db,
            analytics,
            msg_stores,
            container,
            git,
            auth,
            image,
            filesystem,
            events,
            file_search_cache,
            approvals,
            media_pipeline,
        })
    }

    fn user_id(&self) -> &str {
        &self.user_id
    }

    fn shared_types() -> Vec<String> {
        vec![]
    }

    fn config(&self) -> &Arc<RwLock<Config>> {
        &self.config
    }

    fn sentry(&self) -> &SentryService {
        &self.sentry
    }

    fn db(&self) -> &DBService {
        &self.db
    }

    #[cfg(feature = "postgres")]
    fn pg_pool(&self) -> Option<&Pool<Postgres>> {
        self.pg_db.as_ref().map(|pg| &pg.pool)
    }

    fn analytics(&self) -> &Option<AnalyticsService> {
        &self.analytics
    }

    fn container(&self) -> &impl ContainerService {
        &self.container
    }
    fn auth(&self) -> &AuthService {
        &self.auth
    }

    fn git(&self) -> &GitService {
        &self.git
    }

    fn image(&self) -> &ImageService {
        &self.image
    }

    fn filesystem(&self) -> &FilesystemService {
        &self.filesystem
    }

    fn msg_stores(&self) -> &Arc<RwLock<HashMap<Uuid, Arc<MsgStore>>>> {
        &self.msg_stores
    }

    fn events(&self) -> &EventService {
        &self.events
    }

    fn file_search_cache(&self) -> &Arc<FileSearchCache> {
        &self.file_search_cache
    }

    fn approvals(&self) -> &Approvals {
        &self.approvals
    }

    fn media_pipeline(&self) -> &MediaPipelineService {
        &self.media_pipeline
    }
}
