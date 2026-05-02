use axum::{
    http::{header, Method, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, IntoMakeService},
    Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::{middleware as app_middleware, DeploymentImpl};

pub mod activity;
pub mod agent_flow_events;
pub mod agent_flows;
pub mod airtable;
pub mod apn_data;
pub mod approvals;
pub mod aptos;
pub mod artifact_reviews;
pub mod artifacts;
pub mod auth;
pub mod bowser;
pub mod cms;
pub mod collaboration;
pub mod comments;
pub mod config;
pub mod containers;
pub mod dav;
pub mod editron_export;
pub mod filesystem;
// pub mod github;
pub mod agent_chat;
pub mod agent_wallets;
pub mod agents;
pub mod automations;
pub mod autonomy;
pub mod board_shares;
pub mod bot_bridge;
pub mod cinematics;
pub mod clients;
pub mod command_center;
pub mod communications;
pub mod companies;
pub mod crm_activities;
pub mod crm_contacts;
pub mod crm_deal_automations;
pub mod crm_deal_transitions;
pub mod crm_deals;
pub mod crm_pipelines;
pub mod data_source_workflows;
pub mod data_sources;
pub mod deliverables;
pub mod discord;
pub mod dropbox;
pub mod email_accounts;
pub mod entity_conversion;
pub mod event_stream;
pub mod events;
pub mod execution_processes;
pub mod execution_summaries;
pub mod feedback;
pub mod frontend;
pub mod graph;
pub mod health;
pub mod images;
pub mod intake;
pub mod integrations;
pub mod intelligence;
pub mod invitations;
pub mod invite_dispatch;
pub mod invoices;
pub mod knowledge;
pub mod marketplace;
pub mod media_library;
pub mod meet;
pub mod mesh;
pub mod mission_control;
pub mod model_pricing;
pub mod multiplayer;
pub mod nora;
pub mod nora_classifier;
pub mod notifications;
pub mod onboarding;
pub mod operator_rates;
pub mod orcha;
pub mod orchestration;
pub mod org_cloud;
pub mod org_invitations;
pub mod org_onboarding;
pub mod organizations;
pub mod oss_listener;
pub mod oss_listener_bg;
pub mod output_schemas;
pub mod pcg_router;
pub mod peer_rewards;
pub mod permissions;
pub mod pipeline_events;
pub mod project_boards;
pub mod project_controllers;
pub mod project_folders;
pub mod projects;
pub mod proposals;
pub mod pulse;
pub mod pythia;
pub mod quickbooks;
pub mod repos;
pub mod review;
pub mod scratch;
pub mod sessions;
pub mod sidebar;
pub mod social_accounts;
pub mod social_inbox;
pub mod social_posts;
pub mod storage;
pub mod system_metrics;
pub mod tags;
pub mod task_artifacts;
pub mod task_attempts;
pub mod task_templates;
pub mod tasks;
pub mod token_usage;
pub mod topsi;
pub mod twilio;
pub mod users;
pub mod vibe_treasury;
pub mod video_gen;
pub mod wallet;
pub mod webhooks;
pub mod wide_research;
pub mod workflow_engine;
pub mod workflow_staging;
pub mod workflow_templates;
pub mod workflow_triggers;

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
    let admin_routes = Router::new()
        .merge(users::router(&deployment))
        .merge(operator_rates::router(&deployment))
        .layer(middleware::from_fn_with_state(
            deployment.clone(),
            app_middleware::require_admin,
        ));

    // Protected routes that require authentication
    // These routes handle sensitive data and must not be publicly accessible
    let protected_routes = Router::new()
        .merge(onboarding::router(&deployment))
        .merge(org_onboarding::router(&deployment))
        .merge(invitations::router(&deployment))
        .merge(apn_data::router())
        .merge(airtable::router())
        .merge(social_accounts::router(&deployment))
        .merge(social_posts::router(&deployment))
        .merge(social_inbox::router(&deployment))
        .merge(integrations::router(&deployment))
        .merge(storage::router(&deployment))
        .merge(email_accounts::router(&deployment))
        .merge(crm_contacts::router(&deployment))
        .merge(crm_pipelines::router(&deployment))
        .merge(crm_deals::router(&deployment))
        .merge(crm_activities::router(&deployment))
        .merge(dropbox::router())
        .merge(quickbooks::router(&deployment))
        .merge(agents::routes())
        .merge(agent_chat::routes())
        .merge(comments::router())
        // Project and task routes require auth for user-scoped access control
        .merge(projects::router(&deployment))
        .merge(tasks::router(&deployment))
        .merge(task_attempts::router(&deployment))
        .merge(task_templates::router(&deployment))
        .merge(approvals::router())
        .merge(agent_wallets::router(&deployment))
        .nest("/permissions", permissions::router(&deployment))
        .merge(vibe_treasury::router(&deployment))
        .merge(wallet::router())
        .merge(pulse::router(&deployment))
        .merge(organizations::router(&deployment))
        .merge(org_invitations::router(&deployment))
        .merge(clients::router(&deployment))
        // project_folders routes deprecated — projects now use parent_project_id nesting
        .merge(board_shares::router(&deployment))
        .merge(sidebar::router(&deployment))
        .merge(entity_conversion::router(&deployment))
        .merge(knowledge::router(&deployment))
        .merge(sessions::router(&deployment))
        .merge(tags::router(&deployment))
        .merge(scratch::router(&deployment))
        .merge(repos::router(&deployment))
        .merge(notifications::router())
        .merge(workflow_templates::router(&deployment))
        .merge(companies::router(&deployment))
        .merge(invoices::router(&deployment))
        .merge(proposals::router(&deployment))
        .merge(deliverables::router(&deployment))
        .merge(command_center::router(&deployment))
        .merge(intelligence::router(&deployment))
        .merge(communications::router(&deployment))
        .merge(data_sources::router(&deployment))
        .merge(data_source_workflows::router(&deployment))
        .merge(workflow_staging::router(&deployment))
        .merge(workflow_triggers::router(&deployment))
        .merge(output_schemas::router())
        .merge(graph::router(&deployment))
        .merge(invite_dispatch::router(&deployment))
        .merge(discord::router(&deployment))
        .merge(media_library::router(&deployment))
        .merge(oss_listener::router(&deployment))
        .merge(intake::router(&deployment))
        .merge(nora::nora_routes())
        .merge(topsi::topsi_routes())
        .merge(nora_classifier::router(&deployment))
        .merge(org_cloud::router(&deployment))
        .merge(feedback::router(&deployment))
        .merge(agent_flows::router(&deployment))
        .merge(agent_flow_events::router(&deployment))
        .merge(orchestration::router(&deployment))
        .merge(video_gen::router(&deployment))
        .layer(middleware::from_fn_with_state(
            deployment.clone(),
            app_middleware::require_auth,
        ));

    // All routes (public and protected)
    let base_routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/metrics", get(metrics_handler))
        .merge(config::router())
        .merge(agent_chat::public_routes()) // Public read-only conversation endpoints for team collaboration
        .merge(containers::router(&deployment))
        .merge(execution_processes::router(&deployment))
        .merge(execution_summaries::routes())
        .merge(auth::router(&deployment))
        .merge(invitations::public_router(&deployment))
        .merge(org_invitations::public_router(&deployment))
        .merge(organizations::public_router(&deployment))
        .merge(filesystem::router())
        .merge(events::router(&deployment))
        .nest("/images", images::routes())
        .merge(cinematics::router(&deployment))
        .merge(twilio::twilio_routes())
        .merge(bot_bridge::router())
        .merge(activity::router())
        .merge(aptos::router(&deployment))
        .merge(webhooks::router())
        .merge(mission_control::router(&deployment))
        .merge(bowser::router(&deployment))
        .merge(collaboration::router(&deployment))
        .merge(autonomy::router(&deployment))
        .merge(automations::router(&deployment))
        .merge(wide_research::router(&deployment))
        .merge(artifact_reviews::router(&deployment))
        .merge(task_artifacts::router(&deployment))
        .merge(artifacts::router(&deployment))
        .merge(review::protected_router(&deployment))
        .merge(editron_export::router(&deployment))
        .merge(dav::router(&deployment))
        .merge(token_usage::router(&deployment))
        .merge(system_metrics::router(&deployment))
        .merge(event_stream::router(&deployment))
        .merge(pipeline_events::router())
        .merge(multiplayer::router(&deployment))
        .merge(cms::router(&deployment))
        .merge(tasks::global_router(&deployment))
        .merge(model_pricing::router(&deployment))
        .merge(vibe_treasury::public_router(&deployment))
        .merge(meet::meet_routes(&deployment))
        .merge(review::router(&deployment))
        .merge(social_accounts::bio_router(&deployment))
        .merge(orcha::orcha_routes())
        .merge(mesh::router(&deployment))
        .merge(peer_rewards::router(&deployment))
        .merge(marketplace::public_router(&deployment))
        .merge(nora_classifier::public_router(&deployment))
        .merge(workflow_triggers::public_router(&deployment))
        .merge(video_gen::public_router(&deployment))
        .merge(pythia::router(&deployment))
        .merge(pcg_router::router(&deployment))
        .route("/data-sync-test", get(apn_data::apn_ping))
        .merge(protected_routes)
        .merge(admin_routes)
        .with_state(deployment);

    // CORS configuration
    let allowed_origins =
        std::env::var("ALLOWED_ORIGINS").unwrap_or_else(|_| "http://localhost:3001".to_string());

    // Collect valid HTTP origins for standard matching
    let parsed_origins: Vec<header::HeaderValue> = allowed_origins
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    // Check if any Tauri origins were requested (tauri:// can't be parsed as HeaderValue)
    let has_tauri_origin = allowed_origins.contains("tauri://");

    let cors = if has_tauri_origin {
        // Use a predicate to allow both standard HTTP origins and tauri:// origins
        CorsLayer::new()
            .allow_origin(AllowOrigin::predicate(move |origin, _| {
                let origin_str = origin.to_str().unwrap_or("");
                origin_str.starts_with("tauri://")
                    || origin_str.starts_with("https://tauri.")
                    || parsed_origins.iter().any(|allowed| allowed == origin)
            }))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
            .allow_credentials(true)
    } else {
        CorsLayer::new()
            .allow_origin(parsed_origins)
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION, header::COOKIE])
            .allow_credentials(true)
    };

    Router::new()
        .route("/", get(frontend::serve_frontend_root))
        .nest("/api", base_routes)
        .fallback(frontend::serve_frontend_fallback)
        .layer(cors)
        .into_make_service()
}
