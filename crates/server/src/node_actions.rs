//! Node → Agent action registry.
//!
//! Maps graph node types to the agent/mutation actions available for them,
//! each pointing at a real HTTP handler that already exists in this server.
//! The frontend fetches this registry via `GET /api/graph/actions?node_type=`
//! so UI affordances never drift from the real handler surface.
//!
//! Phase 0 includes only the handlers that already exist. Phase 5 adds the
//! mutation-only ones (advance-stage, assign-task, reassign, retry,
//! inspect-logs wiring). Phase 5b adds the generative ones (create-proposal,
//! create-deck, draft-outreach) alongside Nora/Topsi.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Organization,
    Company,
    Client,
    Person,
    Deal,
    Proposal,
    Project,
    Deliverable,
    Task,
    TaskAttempt,
    KnowledgeSource,
    ResearchPass,
    Pipeline,
    BrandProfile,
    Deck,
    Activity,
    Agent,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Organization => "organization",
            NodeType::Company => "company",
            NodeType::Client => "client",
            NodeType::Person => "person",
            NodeType::Deal => "deal",
            NodeType::Proposal => "proposal",
            NodeType::Project => "project",
            NodeType::Deliverable => "deliverable",
            NodeType::Task => "task",
            NodeType::TaskAttempt => "task_attempt",
            NodeType::KnowledgeSource => "knowledge_source",
            NodeType::ResearchPass => "research_pass",
            NodeType::Pipeline => "pipeline",
            NodeType::BrandProfile => "brand_profile",
            NodeType::Deck => "deck",
            NodeType::Activity => "activity",
            NodeType::Agent => "agent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "organization" => Some(NodeType::Organization),
            "company" => Some(NodeType::Company),
            "client" => Some(NodeType::Client),
            "person" => Some(NodeType::Person),
            "deal" => Some(NodeType::Deal),
            "proposal" => Some(NodeType::Proposal),
            "project" => Some(NodeType::Project),
            "deliverable" => Some(NodeType::Deliverable),
            "task" => Some(NodeType::Task),
            "task_attempt" => Some(NodeType::TaskAttempt),
            "knowledge_source" => Some(NodeType::KnowledgeSource),
            "research_pass" => Some(NodeType::ResearchPass),
            "pipeline" => Some(NodeType::Pipeline),
            "brand_profile" => Some(NodeType::BrandProfile),
            "deck" => Some(NodeType::Deck),
            "activity" => Some(NodeType::Activity),
            "agent" => Some(NodeType::Agent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAction {
    Research,
    Summarize,
    OpenProfile,
    InspectLogs,
    StartConversation,
    // Phase 5 adds: AdvanceStage, AssignTask, Reassign, Retry
    // Phase 5b adds: CreateProposal, CreateDeck, DraftOutreach, ReIngest, ExtractFacts
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionHandler {
    pub action: AgentAction,
    pub route: &'static str,
    pub method: &'static str,
    pub description: &'static str,
    pub implemented: bool,
}

pub struct NodeActionRegistry {
    actions: HashMap<(NodeType, AgentAction), ActionHandler>,
}

impl Default for NodeActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeActionRegistry {
    pub fn new() -> Self {
        let mut actions: HashMap<(NodeType, AgentAction), ActionHandler> = HashMap::new();

        // Company actions
        actions.insert(
            (NodeType::Company, AgentAction::Research),
            ActionHandler {
                action: AgentAction::Research,
                route: "/api/companies/{id}/research",
                method: "POST",
                description: "Trigger company intelligence research (Scout via Nora)",
                implemented: true,
            },
        );
        actions.insert(
            (NodeType::Company, AgentAction::Summarize),
            ActionHandler {
                action: AgentAction::Summarize,
                route: "/api/companies/{id}",
                method: "GET",
                description: "Fetch company detail with intelligence summary",
                implemented: true,
            },
        );
        actions.insert(
            (NodeType::Company, AgentAction::OpenProfile),
            ActionHandler {
                action: AgentAction::OpenProfile,
                route: "/organizations/{orgId}/companies/{id}",
                method: "GET",
                description: "Open company profile page",
                implemented: true,
            },
        );

        // Person / contact actions
        actions.insert(
            (NodeType::Person, AgentAction::Research),
            ActionHandler {
                action: AgentAction::Research,
                route: "/api/contacts/{id}/research",
                method: "POST",
                description: "Trigger contact intelligence research (Scout via Nora)",
                implemented: true,
            },
        );
        actions.insert(
            (NodeType::Person, AgentAction::OpenProfile),
            ActionHandler {
                action: AgentAction::OpenProfile,
                route: "/people/{id}",
                method: "GET",
                description: "Open person profile page",
                implemented: true,
            },
        );

        // Task actions
        actions.insert(
            (NodeType::Task, AgentAction::InspectLogs),
            ActionHandler {
                action: AgentAction::InspectLogs,
                route: "/api/events/processes/{process_id}/logs",
                method: "GET",
                description: "Stream execution logs for this task",
                implemented: true,
            },
        );

        Self { actions }
    }

    pub fn handler(&self, node_type: NodeType, action: AgentAction) -> Option<&ActionHandler> {
        self.actions.get(&(node_type, action))
    }

    pub fn actions_for(&self, node_type: NodeType) -> Vec<&ActionHandler> {
        self.actions
            .iter()
            .filter_map(|((nt, _), h)| if *nt == node_type { Some(h) } else { None })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_existing_handlers() {
        let reg = NodeActionRegistry::new();

        assert!(reg
            .handler(NodeType::Company, AgentAction::Research)
            .is_some());
        assert!(reg
            .handler(NodeType::Company, AgentAction::Summarize)
            .is_some());
        assert!(reg
            .handler(NodeType::Person, AgentAction::Research)
            .is_some());
        assert!(reg
            .handler(NodeType::Task, AgentAction::InspectLogs)
            .is_some());

        // Company has 3 actions registered today
        assert_eq!(reg.actions_for(NodeType::Company).len(), 3);

        // All registered handlers are implemented; Phase 5 will add ones
        // with `implemented: false` and the UI will hide them.
        for h in reg.actions.values() {
            assert!(h.implemented, "action {:?} should be implemented", h.action);
        }
    }

    #[test]
    fn node_type_roundtrip() {
        for nt in [
            NodeType::Organization,
            NodeType::Company,
            NodeType::Person,
            NodeType::Task,
        ] {
            assert_eq!(NodeType::parse(nt.as_str()), Some(nt));
        }
        assert_eq!(NodeType::parse("nonsense"), None);
    }
}
