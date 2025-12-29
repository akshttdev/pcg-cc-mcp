use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use ts_rs::TS;
use uuid::Uuid;

use crate::{agent::{ExecutiveAction, NoraRequest}, NoraError};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphPlan {
    pub id: String,
    pub title: String,
    pub request_id: String,
    pub session_id: String,
    pub created_at: DateTime<Utc>,
    pub status: GraphPlanStatus,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphPlanSummary {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub status: GraphPlanStatus,
    pub node_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub agent: Option<String>,
    pub description: Option<String>,
    pub status: GraphNodeStatus,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub from_node: String,
    pub to_node: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum GraphPlanStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum GraphNodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Default)]
pub struct GraphOrchestrator {
    plans: Arc<RwLock<HashMap<Uuid, GraphPlan>>>,
}

impl GraphOrchestrator {
    pub fn new() -> Self {
        Self {
            plans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_plan(
        &self,
        request: &NoraRequest,
        actions: &[ExecutiveAction],
    ) -> Option<String> {
        if actions.is_empty() {
            return None;
        }

        let plan_id = Uuid::new_v4();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (idx, action) in actions.iter().enumerate() {
            let node_id = Uuid::new_v4().to_string();
            nodes.push(GraphNode {
                id: node_id.clone(),
                label: action.action_type.clone(),
                agent: action.assigned_to.clone(),
                description: Some(action.description.clone()),
                status: GraphNodeStatus::Pending,
                metadata: action.parameters.clone(),
            });

            if idx > 0 {
                let prev_id = nodes[idx - 1].id.clone();
                edges.push(GraphEdge {
                    from_node: prev_id,
                    to_node: node_id,
                });
            }
        }

        let plan = GraphPlan {
            id: plan_id.to_string(),
            title: format!("{} ({:?})", request.content.trim(), request.request_type),
            request_id: request.request_id.clone(),
            session_id: request.session_id.clone(),
            created_at: Utc::now(),
            status: GraphPlanStatus::Pending,
            nodes,
            edges,
        };

        let mut plans = self.plans.write().await;
        plans.insert(plan_id, plan);

        Some(plan_id.to_string())
    }

    pub async fn list_plans(&self) -> Vec<GraphPlanSummary> {
        let plans = self.plans.read().await;
        let mut summaries: Vec<_> = plans
            .values()
            .map(|plan| GraphPlanSummary {
                id: plan.id.clone(),
                title: plan.title.clone(),
                created_at: plan.created_at,
                status: plan.status.clone(),
                node_count: plan.nodes.len(),
            })
            .collect();
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        summaries
    }

    pub async fn get_plan(&self, id: &str) -> Option<GraphPlan> {
        let plans = self.plans.read().await;
        plans
            .values()
            .find(|plan| plan.id == id)
            .cloned()
    }

    pub async fn update_node_status(
        &self,
        plan_id: &str,
        node_id: &str,
        status: GraphNodeStatus,
    ) -> Result<GraphPlan, NoraError> {
        let mut plans = self.plans.write().await;
        let plan_entry = plans
            .values_mut()
            .find(|plan| plan.id == plan_id)
            .ok_or_else(|| NoraError::ConfigError("Plan not found".to_string()))?;

        let mut found = false;
        for node in &mut plan_entry.nodes {
            if node.id == node_id {
                node.status = status.clone();
                found = true;
                break;
            }
        }

        if !found {
            return Err(NoraError::ConfigError("Node not found".to_string()));
        }

        plan_entry.status = if plan_entry
            .nodes
            .iter()
            .all(|node| matches!(node.status, GraphNodeStatus::Completed))
        {
            GraphPlanStatus::Completed
        } else if plan_entry
            .nodes
            .iter()
            .any(|node| matches!(node.status, GraphNodeStatus::Failed))
        {
            GraphPlanStatus::Failed
        } else if plan_entry
            .nodes
            .iter()
            .any(|node| matches!(node.status, GraphNodeStatus::Running))
        {
            GraphPlanStatus::InProgress
        } else {
            GraphPlanStatus::Pending
        };

        Ok(plan_entry.clone())
    }
}
