use axum::{
    Router,
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{IntoMakeService, get},
};

use crate::{DeploymentImpl, middleware as app_middleware};

pub mod activity;
pub mod approvals;
pub mod auth;
pub mod comments;
pub mod config;
pub mod containers;
pub mod filesystem;
// pub mod github;
pub mod agent_wallets;
pub mod events;
pub mod execution_processes;
pub mod frontend;
pub mod health;
pub mod images;
pub mod nora;
pub mod permissions;
pub mod project_boards;
pub mod projects;
pub mod task_attempts;
pub mod task_templates;
pub mod tasks;
pub mod twilio;
pub mod users;

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
        .merge(task_templates::router(&deployment))
        .merge(auth::router(&deployment))
        .merge(filesystem::router())
        .merge(events::router(&deployment))
        .merge(approvals::router())
        .merge(agent_wallets::router(&deployment))
        .nest("/permissions", permissions::router(&deployment))
        .nest("/images", images::routes())
        .merge(nora::nora_routes())
        .merge(twilio::twilio_routes())
        .merge(comments::router())
        .merge(activity::router())
        .merge(admin_routes)
        .with_state(deployment);

    Router::new()
        .route("/", get(frontend::serve_frontend_root))
        .route("/{*path}", get(frontend::serve_frontend))
        .nest("/api", base_routes)
        .into_make_service()
}
