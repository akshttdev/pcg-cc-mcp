pub mod agent_flow_executor;
pub mod apn_data_service;
pub mod apn_peer_manager;
pub mod error;
pub mod helpers;
pub mod mcp;
pub mod middleware;
pub mod nora_metrics;
pub mod orcha_routing;
pub mod orchestration_engine;
pub mod orchestration_events;
pub mod orchestration_state;
pub mod org_cloud_indexer;
pub mod pulse_consumer;
pub mod pulse_publisher;
pub mod routes;
pub mod sovereign_stack;
pub mod sovereign_storage;
pub mod stage_transition;
pub mod tool_partitioner;
pub mod twilio_sms;
pub mod workers;

// #[cfg(feature = "cloud")]
// type DeploymentImpl = duck_kanban_cloud::deployment::CloudDeployment;
// #[cfg(not(feature = "cloud"))]
pub type DeploymentImpl = local_deployment::LocalDeployment;
