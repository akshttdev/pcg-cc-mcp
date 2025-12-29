//! Workflow routing - matches user requests to agent workflows

use crate::{
    coordination::{AgentCoordinationState, AgentStatus},
    profiles::{AgentProfile, AgentWorkflow},
};

/// Routes user requests to appropriate agent workflows
pub struct WorkflowRouter {
    agents: Vec<AgentProfile>,
}

impl WorkflowRouter {
    pub fn new(agents: Vec<AgentProfile>) -> Self {
        Self { agents }
    }

    /// Get all agent profiles
    pub fn get_agents(&self) -> &[AgentProfile] {
        &self.agents
    }

    /// Get workflows for a specific agent
    pub fn get_agent_workflows(&self, agent_id: &str) -> Option<&Vec<AgentWorkflow>> {
        self.agents
            .iter()
            .find(|a| a.agent_id == agent_id)
            .map(|a| &a.workflows)
    }

    /// Find the best matching agent and workflow for a user request
    pub fn route_request(
        &self,
        user_request: &str,
        agent_states: &[AgentCoordinationState],
    ) -> Option<(AgentProfile, AgentWorkflow)> {
        let request_lower = user_request.to_lowercase();

        // Score each agent's workflows
        let mut matches: Vec<(AgentProfile, AgentWorkflow, f32)> = Vec::new();

        for agent in &self.agents {
            // Skip agents that are not active
            if let Some(state) = agent_states.iter().find(|s| s.agent_id == agent.agent_id) {
                if matches!(state.status, AgentStatus::Offline | AgentStatus::Error) {
                    continue;
                }
            }

            for workflow in &agent.workflows {
                let score = self.calculate_match_score(&request_lower, agent, workflow);
                if score > 0.3 {
                    // Threshold for relevance
                    matches.push((agent.clone(), workflow.clone(), score));
                }
            }
        }

        // Sort by score (highest first)
        matches.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        // Return the best match
        matches.into_iter().next().map(|(agent, workflow, _)| (agent, workflow))
    }

    /// Calculate how well a workflow matches a user request
    fn calculate_match_score(
        &self,
        request: &str,
        agent: &AgentProfile,
        workflow: &AgentWorkflow,
    ) -> f32 {
        let mut score = 0.0;

        // Check if agent codename is mentioned
        if request.contains(&agent.codename.to_lowercase()) {
            score += 0.5;
        }

        // Check if agent title keywords are mentioned
        let title_lower = agent.title.to_lowercase();
        let title_words: Vec<&str> = title_lower.split_whitespace().collect();
        for word in title_words {
            if word.len() > 3 && request.contains(word) {
                score += 0.2;
            }
        }

        // Check workflow trigger keywords (most important)
        for keyword in &workflow.trigger_keywords {
            if request.contains(&keyword.to_lowercase()) {
                score += 0.4;
            }
        }

        // Check workflow name
        if request.contains(&workflow.name.to_lowercase()) {
            score += 0.3;
        }

        // Check workflow objective keywords
        let objective_lower = workflow.objective.to_lowercase();
        let objective_words: Vec<&str> = objective_lower.split_whitespace().collect();
        let mut objective_matches = 0;
        for word in objective_words {
            if word.len() > 4 && request.contains(word) {
                objective_matches += 1;
            }
        }
        if objective_matches > 2 {
            score += 0.3;
        }

        // Check agent capabilities
        for capability in &agent.capabilities {
            if request.contains(&capability.to_lowercase()) {
                score += 0.2;
            }
        }

        score
    }

    /// Find a specific workflow by agent and workflow IDs
    pub fn find_workflow(
        &self,
        agent_id: &str,
        workflow_id: &str,
    ) -> Option<(AgentProfile, AgentWorkflow)> {
        for agent in &self.agents {
            if agent.agent_id == agent_id {
                for workflow in &agent.workflows {
                    if workflow.workflow_id == workflow_id {
                        return Some((agent.clone(), workflow.clone()));
                    }
                }
            }
        }
        None
    }

    /// Get all workflows for a specific agent
    pub fn get_agent_workflows(&self, agent_id: &str) -> Vec<AgentWorkflow> {
        self.agents
            .iter()
            .find(|a| a.agent_id == agent_id)
            .map(|a| a.workflows.clone())
            .unwrap_or_default()
    }

    /// List all available workflows across all agents
    pub fn list_all_workflows(&self) -> Vec<(String, AgentWorkflow)> {
        self.agents
            .iter()
            .flat_map(|agent| {
                agent.workflows.iter().map(move |workflow| {
                    (agent.agent_id.clone(), workflow.clone())
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::default_agent_profiles;

    #[test]
    fn test_editron_routing() {
        let router = WorkflowRouter::new(default_agent_profiles());
        let request = "Editron, create a recap video from this event footage";

        let result = router.route_request(request, &[]);
        assert!(result.is_some());

        let (agent, workflow) = result.unwrap();
        assert_eq!(agent.agent_id, "editron-post");
        assert_eq!(workflow.workflow_id, "event-recap-forge");
    }

    #[test]
    fn test_highlight_reel_routing() {
        let router = WorkflowRouter::new(default_agent_profiles());
        let request = "Create highlight reels for social media from the event";

        let result = router.route_request(request, &[]);
        assert!(result.is_some());

        let (agent, _) = result.unwrap();
        assert_eq!(agent.agent_id, "editron-post");
    }

    #[test]
    fn test_roadmap_routing() {
        let router = WorkflowRouter::new(default_agent_profiles());
        let request = "Astra, we need to reprioritize the roadmap for next quarter";

        let result = router.route_request(request, &[]);
        assert!(result.is_some());

        let (agent, workflow) = result.unwrap();
        assert_eq!(agent.agent_id, "astra-strategy");
        assert_eq!(workflow.workflow_id, "roadmap-compression");
    }
}
