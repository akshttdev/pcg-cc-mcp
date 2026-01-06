use axum::{
    Router,
    http::{StatusCode, Method, header},
    middleware,
    response::IntoResponse,
    routing::{IntoMakeService, get},
};
use tower_http::cors::CorsLayer;

use crate::{DeploymentImpl, middleware as app_middleware};

pub mod activity;
pub mod agent_flow_events;
pub mod agent_flows;
pub mod airtable;
pub mod approvals;
pub mod artifact_reviews;
pub mod auth;
pub mod bowser;
pub mod collaboration;
pub mod comments;
pub mod config;
pub mod containers;
pub mod filesystem;
// pub mod github;
pub mod agent_chat;
pub mod agent_wallets;
pub mod agents;
pub mod events;
pub mod execution_processes;
pub mod execution_summaries;
pub mod frontend;
pub mod health;
pub mod images;
pub mod mission_control;
pub mod nora;
pub mod permissions;
pub mod project_boards;
pub mod projects;
pub mod task_artifacts;
pub mod task_attempts;
pub mod task_templates;
pub mod tasks;
pub mod twilio;
pub mod users;
pub mod autonomy;
pub mod cinematics;
pub mod webhooks;
pub mod dropbox;
pub mod wide_research;
pub mod token_usage;
pub mod system_metrics;
pub mod event_stream;
pub mod social_accounts;
pub mod social_posts;
pub mod social_inbox;
pub mod email_accounts;
pub mod crm_contacts;
pub mod onboarding;

/// Handler for the /metrics endpoint that exposes Prometheus metrics
async fn metrics_handler() -> impl IntoResponse {
    match crate::nora_metrics::export_metrics() {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to export metrics: {}", e),
        ),
    }
}

pub fn router(deployment: DeploymentImpl) -> IntoMakeService<Router> {
    // Admin routes with require_admin middleware applied BEFORE state
    let admin_routes =
        Router::new()
            .merge(users::router(&deployment))
            .layer(middleware::from_fn_with_state(
                deployment.clone(),
                app_middleware::require_admin,
            ));

    // All routes (public and protected)
    let base_routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics_handler))
        .merge(config::router())
        .merge(containers::router(&deployment))
        .merge(projects::router(&deployment))
        .merge(tasks::router(&deployment))
        .merge(task_attempts::router(&deployment))
        .merge(execution_processes::router(&deployment))
        .merge(execution_summaries::routes())
        .merge(task_templates::router(&deployment))
        .merge(auth::router(&deployment))
        .merge(filesystem::router())
        .merge(events::router(&deployment))
        .merge(approvals::router())
        .merge(agent_wallets::router(&deployment))
        .merge(agents::routes())
        .merge(agent_chat::routes())
        .nest("/permissions", permissions::router(&deployment))
        .nest("/images", images::routes())
        .merge(nora::nora_routes())
        .merge(cinematics::router(&deployment))
        .merge(twilio::twilio_routes())
        .merge(comments::router())
        .merge(activity::router())
        .merge(dropbox::router())
        .merge(airtable::router())
        .merge(webhooks::router())
        .merge(mission_control::router(&deployment))
        .merge(bowser::router(&deployment))
        .merge(collaboration::router(&deployment))
        .merge(autonomy::router(&deployment))
        .merge(agent_flows::router(&deployment))
        .merge(agent_flow_events::router(&deployment))
        .merge(wide_research::router(&deployment))
        .merge(artifact_reviews::router(&deployment))
        .merge(task_artifacts::router(&deployment))
        .merge(token_usage::router(&deployment))
        .merge(system_metrics::router(&deployment))
        .merge(event_stream::router(&deployment))
        .merge(social_accounts::router(&deployment))
        .merge(social_posts::router(&deployment))
        .merge(social_inbox::router(&deployment))
        .merge(email_accounts::router(&deployment))
        .merge(crm_contacts::router(&deployment))
        .merge(onboarding::router(&deployment))
        .merge(admin_routes)
        .with_state(deployment);

    // CORS configuration for external embeds (e.g., Jungleverse iframe)
    let allowed_origins = std::env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3001".to_string());

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect::<Vec<_>>()
        )
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
        .allow_credentials(true);

    Router::new()
        .route("/", get(frontend::serve_frontend_root))
        .route("/{*path}", get(frontend::serve_frontend))
        .nest("/api", base_routes)
        .layer(cors)
        .into_make_service()
}
