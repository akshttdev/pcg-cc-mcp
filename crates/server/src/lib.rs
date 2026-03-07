pub mod apn_data_service;
pub mod error;
pub mod mcp;
pub mod middleware;
pub mod nora_metrics;
pub mod orcha_routing;
pub mod pulse_consumer;
pub mod pulse_publisher;
pub mod routes;
pub mod sovereign_storage;
pub mod task_scheduler;
pub mod twilio_sms;

// #[cfg(feature = "cloud")]
// type DeploymentImpl = duck_kanban_cloud::deployment::CloudDeployment;
// #[cfg(not(feature = "cloud"))]
pub type DeploymentImpl = local_deployment::LocalDeployment;
