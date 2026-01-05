//! Execution Router - Analyzes requests and selects agent/workflow
//!
//! Implements the Router phase of the Router-Executor-Observer loop.
//! Loads agent profiles and matches requests to appropriate workflows.

use crate::profiles::{AgentProfile, AgentWorkflow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use ts_rs::TS;

/// Result of routing a request to an agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentMatch {
    pub agent: AgentProfile,
    pub workflow: AgentWorkflow,
    pub confidence: f32,
    pub match_reasons: Vec<String>,
}

/// Routes execution requests to appropriate agents
pub struct ExecutionRouter {
    agents: Vec<AgentProfile>,
}

impl ExecutionRouter {
    /// Create router with agent profiles
    pub fn new(agents: Vec<AgentProfile>) -> Arc<Self> {
        Arc::new(Self { agents })
    }

    /// Get all available agents
    pub fn agents(&self) -> &[AgentProfile] {
        &self.agents
    }

    /// Get agent by ID
    pub fn get_agent(&self, agent_id: &str) -> Option<&AgentProfile> {
        self.agents.iter().find(|a| a.agent_id == agent_id)
    }

    /// Get agent by codename (e.g., "Scout", "Maci")
    pub fn get_agent_by_codename(&self, codename: &str) -> Option<&AgentProfile> {
        let codename_lower = codename.to_lowercase();
        self.agents.iter().find(|a| a.codename.to_lowercase() == codename_lower)
    }

    /// Find workflow by agent and workflow ID
    pub fn get_workflow(&self, agent_id: &str, workflow_id: &str) -> Option<(&AgentProfile, &AgentWorkflow)> {
        self.agents.iter()
            .find(|a| a.agent_id == agent_id)
            .and_then(|agent| {
                agent.workflows.iter()
                    .find(|w| w.workflow_id == workflow_id)
                    .map(|workflow| (agent, workflow))
            })
    }

    /// Route a user request to the best matching agent/workflow
    pub fn route(&self, request: &str) -> Option<AgentMatch> {
        let request_lower = request.to_lowercase();
        let mut best_match: Option<AgentMatch> = None;
        let mut best_score = 0.0f32;

        for agent in &self.agents {
            // Skip offline agents
            if matches!(agent.status, crate::coordination::AgentStatus::Offline) {
                continue;
            }

            for workflow in &agent.workflows {
                let (score, reasons) = self.calculate_match(&request_lower, agent, workflow);

                if score > best_score && score >= 0.3 {
                    best_score = score;
                    best_match = Some(AgentMatch {
                        agent: agent.clone(),
                        workflow: workflow.clone(),
                        confidence: score,
                        match_reasons: reasons,
                    });
                }
            }
        }

        best_match
    }

    /// Route to a specific agent by codename
    pub fn route_to_agent(&self, codename: &str) -> Option<AgentMatch> {
        self.get_agent_by_codename(codename).and_then(|agent| {
            // Return first workflow for the agent
            agent.workflows.first().map(|workflow| AgentMatch {
                agent: agent.clone(),
                workflow: workflow.clone(),
                confidence: 1.0,
                match_reasons: vec![format!("Direct agent reference: {}", codename)],
            })
        })
    }

    /// Calculate match score between request and workflow
    fn calculate_match(
        &self,
        request: &str,
        agent: &AgentProfile,
        workflow: &AgentWorkflow,
    ) -> (f32, Vec<String>) {
        let mut score = 0.0f32;
        let mut reasons = Vec::new();

        // Check agent codename (highest weight)
        if request.contains(&agent.codename.to_lowercase()) {
            score += 0.5;
            reasons.push(format!("Agent name '{}' mentioned", agent.codename));
        }

        // Check workflow trigger keywords
        for keyword in &workflow.trigger_keywords {
            if request.contains(&keyword.to_lowercase()) {
                score += 0.4;
                reasons.push(format!("Trigger keyword '{}' matched", keyword));
            }
        }

        // Check workflow name
        let workflow_name_lower = workflow.name.to_lowercase();
        if request.contains(&workflow_name_lower) {
            score += 0.3;
            reasons.push(format!("Workflow name '{}' matched", workflow.name));
        }

        // Check agent title keywords
        for word in agent.title.to_lowercase().split_whitespace() {
            if word.len() > 3 && request.contains(word) {
                score += 0.2;
                reasons.push(format!("Title keyword '{}' matched", word));
            }
        }

        // Check agent capabilities
        for capability in &agent.capabilities {
            let cap_lower = capability.to_lowercase().replace('_', " ");
            if request.contains(&cap_lower) || request.contains(capability) {
                score += 0.2;
                reasons.push(format!("Capability '{}' matched", capability));
            }
        }

        // Check workflow objective
        let objective_lower = workflow.objective.to_lowercase();
        let mut objective_matches = 0;
        for word in objective_lower.split_whitespace() {
            if word.len() > 4 && request.contains(word) {
                objective_matches += 1;
            }
        }
        if objective_matches >= 2 {
            score += 0.2;
            reasons.push(format!("{} objective keywords matched", objective_matches));
        }

        (score.min(1.0), reasons)
    }

    /// List all available workflows with their agents
    pub fn list_workflows(&self) -> Vec<(String, String, String, String)> {
        self.agents
            .iter()
            .flat_map(|agent| {
                agent.workflows.iter().map(move |workflow| {
                    (
                        agent.agent_id.clone(),
                        agent.codename.clone(),
                        workflow.workflow_id.clone(),
                        workflow.name.clone(),
                    )
                })
            })
            .collect()
    }

    /// Get agents for a specific category
    pub fn get_social_agents(&self) -> Vec<&AgentProfile> {
        self.agents
            .iter()
            .filter(|a| {
                matches!(a.agent_id.as_str(),
                    "scout-research" | "oracle-strategy" | "muse-creative" |
                    "herald-distribution" | "echo-engagement" |
                    "master-cinematographer" | "editron-post"
                )
            })
            .collect()
    }

    /// Get creative/production agents
    pub fn get_creative_agents(&self) -> Vec<&AgentProfile> {
        self.agents
            .iter()
            .filter(|a| {
                matches!(a.agent_id.as_str(),
                    "master-cinematographer" | "editron-post" | "muse-creative"
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::default_agent_profiles;

    #[test]
    fn test_route_by_codename() {
        let router = ExecutionRouter::new(default_agent_profiles());

        let result = router.route("Tell Scout to research competitors");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.agent.codename, "Scout");
    }

    #[test]
    fn test_route_by_keyword() {
        let router = ExecutionRouter::new(default_agent_profiles());

        let result = router.route("I need competitor analysis for our industry");
        assert!(result.is_some());

        let m = result.unwrap();
        assert!(m.confidence > 0.3);
    }
}
