pub mod error;
pub mod mcp;
pub mod middleware;
pub mod nora_metrics;
pub mod routes;
pub mod sovereign_storage;
pub mod task_scheduler;

// #[cfg(feature = "cloud")]
// type DeploymentImpl = duck_kanban_cloud::deployment::CloudDeployment;
// #[cfg(not(feature = "cloud"))]
pub type DeploymentImpl = local_deployment::LocalDeployment;
