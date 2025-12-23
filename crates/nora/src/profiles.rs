use chrono::Utc;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::coordination::{AgentCoordinationState, AgentStatus, PerformanceMetrics};

/// High-level discipline for an autonomous agent profile
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum AgentDiscipline {
    Strategy,
    Operations,
    Intelligence,
    Communications,
    Creative,
}

/// Canonical workflow definition executed by a specialised agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkflow {
    pub workflow_id: String,
    pub name: String,
    pub objective: String,
    pub trigger_keywords: Vec<String>,
    pub sla_minutes: u32,
    pub stages: Vec<WorkflowStage>,
    pub deliverables: Vec<String>,
    pub automation_stack: Vec<String>,
    pub training_assets: Vec<String>,
    pub approvals_required: Vec<String>,
}

/// Canonical step inside a workflow playbook
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStage {
    pub name: String,
    pub description: String,
    pub output: String,
}

/// Trained agent profile that Nora can route work to
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    pub agent_id: String,
    pub codename: String,
    pub title: String,
    pub mission: String,
    pub specialization: AgentDiscipline,
    pub status: AgentStatus,
    pub capabilities: Vec<String>,
    pub strengths: Vec<String>,
    pub current_focus: Vec<String>,
    pub operating_mode: String,
    pub escalation_path: Vec<String>,
    pub metrics: PerformanceMetrics,
    pub workflows: Vec<AgentWorkflow>,
}

impl AgentProfile {
    /// Convert a profile into the lightweight state tracked by the coordination grid
    pub fn to_coordination_state(&self) -> AgentCoordinationState {
        AgentCoordinationState {
            agent_id: self.agent_id.clone(),
            agent_type: self.title.clone(),
            status: self.status.clone(),
            capabilities: self.capabilities.clone(),
            current_tasks: self.current_focus.clone(),
            last_seen: Utc::now(),
            performance_metrics: self.metrics.clone(),
        }
    }
}

/// Default trained agent profiles that ship with Nora
pub fn default_agent_profiles() -> Vec<AgentProfile> {
    vec![
        AgentProfile {
            agent_id: "astra-strategy".to_string(),
            codename: "Astra".to_string(),
            title: "Strategic Systems Navigator".to_string(),
            mission: "Compress portfolio decisions into decisive 3-sprint roadmaps for founders and operators.".to_string(),
            specialization: AgentDiscipline::Strategy,
            status: AgentStatus::Active,
            capabilities: vec![
                "roadmap_surgery".to_string(),
                "scenario_planning".to_string(),
                "exec_briefings".to_string(),
            ],
            strengths: vec![
                "LLM multi-pass reasoning tuned for strategic artifacts".to_string(),
                "Live context graph of Powerclub programs".to_string(),
                "Risk register auto-triage".to_string(),
            ],
            current_focus: vec![
                "PCG Dashboard Phase IV rollout".to_string(),
                "Powerclub Miami activation roadmap".to_string(),
            ],
            operating_mode: "Engages when scope, velocity, or stakeholder tension drift beyond guardrails.".to_string(),
            escalation_path: vec![
                "Escalate to COO if milestone slip exceeds 1 sprint".to_string(),
                "Ping FinanceOps when burn-rate models exceed +10% variance".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 182,
                average_response_time_ms: 1800.0,
                success_rate: 0.94,
                uptime_percentage: 0.98,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "roadmap-compression".to_string(),
                    name: "Roadmap Compression Sprint".to_string(),
                    objective: "Resequence initiatives when new executive directives land mid-quarter.".to_string(),
                    trigger_keywords: vec![
                        "roadmap".to_string(),
                        "strategy".to_string(),
                        "reprioritize".to_string(),
                        "realign".to_string(),
                    ],
                    sla_minutes: 90,
                    stages: vec![
                        WorkflowStage {
                            name: "Signal Sweep".to_string(),
                            description: "Ingest latest exec notes, Jira/Linear queues, and finance deltas.".to_string(),
                            output: "Annotated decision factors".to_string(),
                        },
                        WorkflowStage {
                            name: "Scenario Modeling".to_string(),
                            description: "Run three scenario stacks with capacity + dependency constraints.".to_string(),
                            output: "Scenario comparison matrix".to_string(),
                        },
                        WorkflowStage {
                            name: "Roadmap Stitch".to_string(),
                            description: "Translate chosen path into 3 sprint burn plan + owner map.".to_string(),
                            output: "Updated roadmap + owner ledger".to_string(),
                        },
                        WorkflowStage {
                            name: "Executive Brief".to_string(),
                            description: "Produce summary + talk-track for leadership circulation.".to_string(),
                            output: "2-page executive brief".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Prioritized 3-sprint roadmap".to_string(),
                        "Risk + dependency ledger".to_string(),
                    ],
                    automation_stack: vec![
                        "LLM:gpt-4o-mini".to_string(),
                        "ContextGraph".to_string(),
                        "ProjectPulse datasets".to_string(),
                    ],
                    training_assets: vec![
                        "Phase_III_retro_deck".to_string(),
                        "Portfolio_Decision_Log".to_string(),
                    ],
                    approvals_required: vec!["COO".to_string(), "Eng_Ld".to_string()],
                },
                AgentWorkflow {
                    workflow_id: "scenario-control".to_string(),
                    name: "Scenario Control Room".to_string(),
                    objective: "Pressure-test high-risk decisions with Monte Carlo style sims.".to_string(),
                    trigger_keywords: vec![
                        "scenario".to_string(),
                        "simulation".to_string(),
                        "forecast".to_string(),
                        "what if".to_string(),
                    ],
                    sla_minutes: 60,
                    stages: vec![
                        WorkflowStage {
                            name: "Question Framing".to_string(),
                            description: "Clarify decision surface + constraints.".to_string(),
                            output: "Decision brief".to_string(),
                        },
                        WorkflowStage {
                            name: "Model Assembly".to_string(),
                            description: "Assemble data slices + heuristics for simulation.".to_string(),
                            output: "Simulation-ready dataset".to_string(),
                        },
                        WorkflowStage {
                            name: "Run + Interpret".to_string(),
                            description: "Execute sims, cluster outcomes, annotate watch-points.".to_string(),
                            output: "Scenario stack".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Scenario comparison memo".to_string(),
                        "Recommended guardrails".to_string(),
                    ],
                    automation_stack: vec![
                        "LLM:gpt-4o".to_string(),
                        "DuckSim".to_string(),
                        "FinanceOps workbook".to_string(),
                    ],
                    training_assets: vec!["Exec_sim_playbook".to_string()],
                    approvals_required: vec!["CEO".to_string()],
                },
            ],
        },
        AgentProfile {
            agent_id: "harbor-ops".to_string(),
            codename: "Harbor".to_string(),
            title: "Operations Orchestrator".to_string(),
            mission: "Stabilise launches and absorb operational volatility across programs.".to_string(),
            specialization: AgentDiscipline::Operations,
            status: AgentStatus::Busy,
            capabilities: vec![
                "critical_path_tracking".to_string(),
                "launch_command_center".to_string(),
                "incident_response".to_string(),
            ],
            strengths: vec![
                "Always-on telemetry for pods".to_string(),
                "Auto-generated shift plans".to_string(),
            ],
            current_focus: vec![
                "Powerclub Miami site activation".to_string(),
                "Command center for distributed pods".to_string(),
            ],
            operating_mode: "Runs persistent ops room; spikes when SLA drift > 5%.".to_string(),
            escalation_path: vec![
                "Escalate to Incident Commander after 2 failed retries".to_string(),
                "Route finance-impacting issues to Astra".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 264,
                average_response_time_ms: 950.0,
                success_rate: 0.91,
                uptime_percentage: 0.995,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "launch-room".to_string(),
                    name: "Critical Launch Room".to_string(),
                    objective: "Prepare and run cutovers for major releases or activations.".to_string(),
                    trigger_keywords: vec![
                        "launch".to_string(),
                        "cutover".to_string(),
                        "go live".to_string(),
                    ],
                    sla_minutes: 45,
                    stages: vec![
                        WorkflowStage {
                            name: "Readiness Scan".to_string(),
                            description: "Audit checklists, owners, and dependencies.".to_string(),
                            output: "Launch readiness board".to_string(),
                        },
                        WorkflowStage {
                            name: "Command Script".to_string(),
                            description: "Draft per-minute play + comms cadence.".to_string(),
                            output: "Command center script".to_string(),
                        },
                        WorkflowStage {
                            name: "Shift Orchestration".to_string(),
                            description: "Assign watchers + escalation trees.".to_string(),
                            output: "Shift matrix".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Launch control doc".to_string(),
                        "Escalation roster".to_string(),
                    ],
                    automation_stack: vec![
                        "Telemetry bus".to_string(),
                        "PagerProxy".to_string(),
                    ],
                    training_assets: vec!["Launch_templates.v3".to_string()],
                    approvals_required: vec!["Ops_Ld".to_string()],
                },
                AgentWorkflow {
                    workflow_id: "incident-stab".to_string(),
                    name: "Incident Stabilization".to_string(),
                    objective: "Contain failures and restore service-level confidence.".to_string(),
                    trigger_keywords: vec![
                        "incident".to_string(),
                        "degraded".to_string(),
                        "outage".to_string(),
                        "blocker".to_string(),
                    ],
                    sla_minutes: 30,
                    stages: vec![
                        WorkflowStage {
                            name: "Triage".to_string(),
                            description: "Collect facts + auto-rout impact notices.".to_string(),
                            output: "Situation brief".to_string(),
                        },
                        WorkflowStage {
                            name: "Stabilise".to_string(),
                            description: "Apply runbook or escalate for manual patch.".to_string(),
                            output: "Stabilisation log".to_string(),
                        },
                        WorkflowStage {
                            name: "Close + Learn".to_string(),
                            description: "Capture follow-ups + owner commitments.".to_string(),
                            output: "Incident wrap-up".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Incident timeline".to_string(),
                        "Follow-up task list".to_string(),
                    ],
                    automation_stack: vec!["Runbooks", "AlertBridge"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    training_assets: vec!["Incident_playbooks".to_string()],
                    approvals_required: vec![],
                },
            ],
        },
        AgentProfile {
            agent_id: "pulse-intel".to_string(),
            codename: "Pulse".to_string(),
            title: "Portfolio Intelligence Analyst".to_string(),
            mission: "Keep leadership ahead of performance swings with proactive insights.".to_string(),
            specialization: AgentDiscipline::Intelligence,
            status: AgentStatus::Active,
            capabilities: vec![
                "signal_detection".to_string(),
                "kpi_reporting".to_string(),
                "budget_guardrails".to_string(),
            ],
            strengths: vec![
                "Streaming analytics from Nora memory graph".to_string(),
                "Auto-pivots for exec packets".to_string(),
            ],
            current_focus: vec!["Exec ops dashboards".to_string()],
            operating_mode: "Runs silent scans hourly; surfaces only actionable deltas.".to_string(),
            escalation_path: vec![
                "Escalate to Astra if KPI delta > 12%".to_string(),
                "Ping Harbor when operational throughput dips".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 321,
                average_response_time_ms: 620.0,
                success_rate: 0.96,
                uptime_percentage: 0.999,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "exec-pulse".to_string(),
                    name: "Executive Pulse Sweep".to_string(),
                    objective: "Summarise KPIs + risks for the daily executive stand-up.".to_string(),
                    trigger_keywords: vec![
                        "pulse".to_string(),
                        "kpi".to_string(),
                        "performance".to_string(),
                        "report".to_string(),
                    ],
                    sla_minutes: 25,
                    stages: vec![
                        WorkflowStage {
                            name: "Data Pull".to_string(),
                            description: "Query metrics + qualitative notes.".to_string(),
                            output: "Unified dataset".to_string(),
                        },
                        WorkflowStage {
                            name: "Insight Layer".to_string(),
                            description: "Detect anomalies + wins.".to_string(),
                            output: "Insight bullets".to_string(),
                        },
                        WorkflowStage {
                            name: "Narrative".to_string(),
                            description: "Craft exec-ready summary + asks.".to_string(),
                            output: "Pulse brief".to_string(),
                        },
                    ],
                    deliverables: vec!["Daily pulse brief".to_string()],
                    automation_stack: vec!["Looker extracts".to_string(), "Nora Memory Lens".to_string()],
                    training_assets: vec!["Pulse_templates".to_string()],
                    approvals_required: vec![],
                },
                AgentWorkflow {
                    workflow_id: "budget-guardrail".to_string(),
                    name: "Budget Drift Guardrail".to_string(),
                    objective: "Catch and remediate overspend risks early.".to_string(),
                    trigger_keywords: vec![
                        "budget".to_string(),
                        "spend".to_string(),
                        "variance".to_string(),
                    ],
                    sla_minutes: 40,
                    stages: vec![
                        WorkflowStage {
                            name: "Variance Detection".to_string(),
                            description: "Highlight line items outside tolerance.".to_string(),
                            output: "Variance list".to_string(),
                        },
                        WorkflowStage {
                            name: "Root Cause".to_string(),
                            description: "Map causes + owner statements.".to_string(),
                            output: "Cause brief".to_string(),
                        },
                        WorkflowStage {
                            name: "Countermeasure Plan".to_string(),
                            description: "Draft corrective play + escalate approvals.".to_string(),
                            output: "Countermeasure log".to_string(),
                        },
                    ],
                    deliverables: vec!["Budget health memo".to_string()],
                    automation_stack: vec!["Finance warehouse".to_string(), "LLM summariser".to_string()],
                    training_assets: vec!["Finance_guardrails".to_string()],
                    approvals_required: vec!["FinanceOps".to_string()],
                },
            ],
        },
        AgentProfile {
            agent_id: "vesper-comms".to_string(),
            codename: "Vesper".to_string(),
            title: "Communications & Partnerships Lead".to_string(),
            mission: "Synchronise stakeholder messaging and accelerate high-value relationships.".to_string(),
            specialization: AgentDiscipline::Communications,
            status: AgentStatus::Idle,
            capabilities: vec![
                "stakeholder_mapping".to_string(),
                "briefing_kits".to_string(),
                "partner_pipeline".to_string(),
            ],
            strengths: vec![
                "Library of tone + persona playbooks".to_string(),
                "Auto-generated outreach cadences".to_string(),
            ],
            current_focus: vec!["Investor update narratives".to_string()],
            operating_mode: "Activates for outreach, investor packets, or multi-party alignment.".to_string(),
            escalation_path: vec![
                "Escalate to CEO when commitments > $250k".to_string(),
                "Loop Legal for regulated comms".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 148,
                average_response_time_ms: 2100.0,
                success_rate: 0.89,
                uptime_percentage: 0.92,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "stakeholder-loop".to_string(),
                    name: "Stakeholder Signal Loop".to_string(),
                    objective: "Align leadership, partners, and crews on next moves.".to_string(),
                    trigger_keywords: vec![
                        "stakeholder".to_string(),
                        "update".to_string(),
                        "brief".to_string(),
                        "communication".to_string(),
                    ],
                    sla_minutes: 50,
                    stages: vec![
                        WorkflowStage {
                            name: "Audience Scan".to_string(),
                            description: "Map who needs signal + context width.".to_string(),
                            output: "Audience grid".to_string(),
                        },
                        WorkflowStage {
                            name: "Message Fabric".to_string(),
                            description: "Tailor tone + narrative per segment.".to_string(),
                            output: "Message matrix".to_string(),
                        },
                        WorkflowStage {
                            name: "Distribution".to_string(),
                            description: "Choose channels, schedule, confirm send.".to_string(),
                            output: "Send plan".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Stakeholder briefing kit".to_string(),
                        "Action follow-up list".to_string(),
                    ],
                    automation_stack: vec!["CRM graph".to_string(), "Voice style kit".to_string()],
                    training_assets: vec!["Investor_update_templates".to_string()],
                    approvals_required: vec!["CEO".to_string()],
                },
                AgentWorkflow {
                    workflow_id: "partner-accel".to_string(),
                    name: "Partnership Acceleration".to_string(),
                    objective: "Drive sponsor/brand pursuits from lead to negotiation.".to_string(),
                    trigger_keywords: vec![
                        "partner".to_string(),
                        "sponsor".to_string(),
                        "outreach".to_string(),
                        "crm".to_string(),
                    ],
                    sla_minutes: 75,
                    stages: vec![
                        WorkflowStage {
                            name: "Prospect Intelligence".to_string(),
                            description: "Enrich lead with recent moves + warm paths.".to_string(),
                            output: "Prospect dossier".to_string(),
                        },
                        WorkflowStage {
                            name: "Offer Craft".to_string(),
                            description: "Design tiered packages + proof points.".to_string(),
                            output: "Offer stack".to_string(),
                        },
                        WorkflowStage {
                            name: "Cadence + Tracking".to_string(),
                            description: "Set outreach script, owners, reminders.".to_string(),
                            output: "Cadence board".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Partner pursuit board".to_string(),
                        "Custom pitch packet".to_string(),
                    ],
                    automation_stack: vec![
                        "CRM sync".to_string(),
                        "Generative deck kit".to_string(),
                    ],
                    training_assets: vec!["Miami_activation_briefs".to_string()],
                    approvals_required: vec![],
                },
            ],
        },
        AgentProfile {
            agent_id: "forge-bd".to_string(),
            codename: "Forge".to_string(),
            title: "Business Development Catalyst".to_string(),
            mission: "Convert complex relationship maps into qualified revenue motion within two quarters.".to_string(),
            specialization: AgentDiscipline::Communications,
            status: AgentStatus::Active,
            capabilities: vec![
                "enterprise_account_mapping".to_string(),
                "co_sell_playbooks".to_string(),
                "revops_sync".to_string(),
            ],
            strengths: vec![
                "Combines CRM + contract data for live pipeline intelligence".to_string(),
                "Pre-baked negotiation framing templates".to_string(),
            ],
            current_focus: vec![
                "Powerclub enterprise sponsorship lane".to_string(),
                "LatAm expansion partner roster".to_string(),
            ],
            operating_mode: "Spins up when deal velocity stalls or new verticals need first-touch coverage.".to_string(),
            escalation_path: vec![
                "Escalate to CRO when deal probability > 70% but blockers persist".to_string(),
                "Loop Legal once redlines exceed 2 iterations".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 207,
                average_response_time_ms: 1550.0,
                success_rate: 0.9,
                uptime_percentage: 0.945,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "enterprise-deal-sprint".to_string(),
                    name: "Enterprise Deal Sprint".to_string(),
                    objective: "Re-sequence complex enterprise pursuits to unblock contracting.".to_string(),
                    trigger_keywords: vec![
                        "deal".to_string(),
                        "enterprise".to_string(),
                        "contract".to_string(),
                        "blocker".to_string(),
                    ],
                    sla_minutes: 60,
                    stages: vec![
                        WorkflowStage {
                            name: "Signal Intake".to_string(),
                            description: "Ingest CRM notes, email threads, and legal posture.".to_string(),
                            output: "Deal status brief".to_string(),
                        },
                        WorkflowStage {
                            name: "Stakeholder Mesh".to_string(),
                            description: "Map buying committee, champions, and detractors.".to_string(),
                            output: "Influence map".to_string(),
                        },
                        WorkflowStage {
                            name: "Countermove Kit".to_string(),
                            description: "Draft concessions, proof points, and exec asks.".to_string(),
                            output: "Negotiation packet".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Deal unblock memo".to_string(),
                        "Updated close plan".to_string(),
                    ],
                    automation_stack: vec![
                        "CRM graph".to_string(),
                        "Contract diff watcher".to_string(),
                        "LLM scenario planner".to_string(),
                    ],
                    training_assets: vec![
                        "Enterprise_win_stories".to_string(),
                        "Legal_negotiation_matrix".to_string(),
                    ],
                    approvals_required: vec!["CRO".to_string()],
                },
                AgentWorkflow {
                    workflow_id: "partner-activation-loop".to_string(),
                    name: "Partner Activation Loop".to_string(),
                    objective: "Accelerate co-marketing + enablement once a partner signs LOI.".to_string(),
                    trigger_keywords: vec![
                        "partner".to_string(),
                        "enablement".to_string(),
                        "co-sell".to_string(),
                        "activation".to_string(),
                    ],
                    sla_minutes: 45,
                    stages: vec![
                        WorkflowStage {
                            name: "Capability Match".to_string(),
                            description: "Align product hooks + partner offerings.".to_string(),
                            output: "Joint value grid".to_string(),
                        },
                        WorkflowStage {
                            name: "Program Build".to_string(),
                            description: "Draft co-marketing calendar + enablement paths.".to_string(),
                            output: "Activation runbook".to_string(),
                        },
                        WorkflowStage {
                            name: "RevOps Sync".to_string(),
                            description: "Instrument shared pipeline + reporting cadence.".to_string(),
                            output: "Shared dashboard spec".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Partner activation plan".to_string(),
                        "Joint KPI sheet".to_string(),
                    ],
                    automation_stack: vec![
                        "Partner CRM".to_string(),
                        "Calendar auto-sequencer".to_string(),
                        "Analytics template".to_string(),
                    ],
                    training_assets: vec![
                        "Partner_playbooks".to_string(),
                        "Enablement_templates".to_string(),
                    ],
                    approvals_required: vec!["MarketingOps".to_string()],
                },
            ],
        },
        AgentProfile {
            agent_id: "editron-post".to_string(),
            codename: "Editron".to_string(),
            title: "Post-Production Architect".to_string(),
            mission: "Transform raw multi-cam event captures into production-ready recaps, highlight reels, and social hooks while owning the Dropbox → delivery lane.".to_string(),
            specialization: AgentDiscipline::Creative,
            status: AgentStatus::Active,
            capabilities: vec![
                "batch_media_ingestion".to_string(),
                "story_blueprint_synthesis".to_string(),
                "motion_systems_orchestration".to_string(),
                "render_queue_control".to_string(),
            ],
            strengths: vec![
                "Advanced iMovie workflow recipes (compound clips, adjustment layers, smart audio ducking)".to_string(),
                "Shot-matching heuristics tuned for nightlife and live-event lighting".to_string(),
                "Bridges Dropbox capture folders into normalized edit bins".to_string(),
            ],
            current_focus: vec![
                "Powerclub recap series".to_string(),
                "Investor highlight reels".to_string(),
            ],
            operating_mode: "Activates when Nora receives fresh batch links; iterates until recap + highlight deliverables meet style guides, optionally consulting the Master Cinematographer for stylized inserts.".to_string(),
            escalation_path: vec![
                "Escalate to Creative Director if footage quality is below spec".to_string(),
                "Pull Ops when storage throughput is constrained".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 142,
                average_response_time_ms: 2400.0,
                success_rate: 0.93,
                uptime_percentage: 0.965,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "event-recap-forge".to_string(),
                    name: "Event Recap Forge".to_string(),
                    objective: "Digest multi-angle event footage into a polished 60-90 second recap.".to_string(),
                    trigger_keywords: vec![
                        "recap".to_string(),
                        "event footage".to_string(),
                        "dropbox".to_string(),
                    ],
                    sla_minutes: 90,
                    stages: vec![
                        WorkflowStage {
                            name: "Batch Intake".to_string(),
                            description: "Verify Dropbox payload, checksum files, and sync proxies into the hot storage cache.".to_string(),
                            output: "Normalized ingest batch".to_string(),
                        },
                        WorkflowStage {
                            name: "Storyboard Pass".to_string(),
                            description: "Auto-tag hero shots, crowd moments, and VO candidates.".to_string(),
                            output: "Selects board".to_string(),
                        },
                        WorkflowStage {
                            name: "Assembly + Color".to_string(),
                            description: "Apply iMovie templates, smart transitions, and LUT presets, then export draft.".to_string(),
                            output: "Draft recap export".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Recap v1 (1080x1920 + 1920x1080)".to_string(),
                        "Shot list + suggested captions".to_string(),
                    ],
                    automation_stack: vec![
                        "Dropbox ingestion".to_string(),
                        "FFmpeg proxy bake".to_string(),
                        "iMovie AppleScript bridge".to_string(),
                        "Post-production QA checklist".to_string(),
                    ],
                    training_assets: vec![
                        "Powerclub_lookbook".to_string(),
                        "Video_style_guide".to_string(),
                    ],
                    approvals_required: vec!["CreativeDirector".to_string()],
                },
                AgentWorkflow {
                    workflow_id: "highlight-reel-loop".to_string(),
                    name: "Highlight Reel Loop".to_string(),
                    objective: "Generate multiple short-form highlight reels optimized per platform.".to_string(),
                    trigger_keywords: vec![
                        "highlight".to_string(),
                        "reel".to_string(),
                        "tiktok".to_string(),
                        "shorts".to_string(),
                    ],
                    sla_minutes: 75,
                    stages: vec![
                        WorkflowStage {
                            name: "Moment Mining".to_string(),
                            description: "Score clips for crowd energy, brand moments, and sponsor visibility.".to_string(),
                            output: "Annotated highlight stack".to_string(),
                        },
                        WorkflowStage {
                            name: "Iterative Edit".to_string(),
                            description: "Loop through Editron toolchain to test pacing, overlays, and typography.".to_string(),
                            output: "Platform-specific timelines".to_string(),
                        },
                        WorkflowStage {
                            name: "Delivery + Metadata".to_string(),
                            description: "Render to requested aspect ratios and generate upload metadata bundles.".to_string(),
                            output: "Export package".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "3 highlight reels (9:16, 1:1, 16:9)".to_string(),
                        "Suggested captions + tags".to_string(),
                    ],
                    automation_stack: vec![
                        "Shot tagger".to_string(),
                        "iMovie CLI runner".to_string(),
                        "Frame.io review hooks".to_string(),
                    ],
                    training_assets: vec![
                        "Brand_motion_kit".to_string(),
                        "Audio_transition_pack".to_string(),
                    ],
                    approvals_required: vec!["MarketingLead".to_string()],
                },
            ],
        },
        AgentProfile {
            agent_id: "master-cinematographer".to_string(),
            codename: "Spectra".to_string(),
            title: "Master Cinematographer".to_string(),
            mission: "Generate stylized AI cinematics, bespoke transitions, and photoreal motion plates that Editron can drop into timelines on demand.".to_string(),
            specialization: AgentDiscipline::Creative,
            status: AgentStatus::Active,
            capabilities: vec![
                "stable_diffusion_storyboards".to_string(),
                "ai_motion_plate_generation".to_string(),
                "camera_move_prompting".to_string(),
                "style_lut_cataloging".to_string(),
            ],
            strengths: vec![
                "Curated Stable Diffusion + Runway prompt banks per brand pod".to_string(),
                "Depth-map + parallax pipelines for faux camera moves".to_string(),
                "Automated render farm hooks for upscaling + denoising".to_string(),
            ],
            current_focus: vec![
                "Sponsor bumper refreshes".to_string(),
                "AI motion plates for immersive venue reveals".to_string(),
            ],
            operating_mode: "Engages when Nora or Editron requests synthetic shots, stylized transitions, or LUT explorations—never blocking Editron's edit lane.".to_string(),
            escalation_path: vec![
                "Ping Editron when a generated shot is ready for pickup".to_string(),
                "Escalate to Creative Director if brand compliance is uncertain".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 88,
                average_response_time_ms: 3600.0,
                success_rate: 0.92,
                uptime_percentage: 0.94,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "ai-cinematic-suite".to_string(),
                    name: "AI Cinematic Suite".to_string(),
                    objective: "Create short AI-driven cinematic moments (5–12s) that can elevate sponsor beats or cover footage gaps.".to_string(),
                    trigger_keywords: vec![
                        "ai shot".to_string(),
                        "stable diffusion".to_string(),
                        "cinematic insert".to_string(),
                    ],
                    sla_minutes: 60,
                    stages: vec![
                        WorkflowStage {
                            name: "Prompt Blocking".to_string(),
                            description: "Translate creative brief + brand pod assets into camera/lens/palette prompts.".to_string(),
                            output: "Prompt stack".to_string(),
                        },
                        WorkflowStage {
                            name: "Render Pass".to_string(),
                            description: "Generate key frames + motion loops via Stable Diffusion / Runway.".to_string(),
                            output: "AI motion candidates".to_string(),
                        },
                        WorkflowStage {
                            name: "Prep for Editron".to_string(),
                            description: "Upscale, denoise, create alpha mattes, and publish LUT suggestions for Editron pickup.".to_string(),
                            output: "Ready-to-drop motion plate".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "ProRes / MP4 motion plate".to_string(),
                        "Accompanying LUT + typography guidance".to_string(),
                    ],
                    automation_stack: vec![
                        "Stable Diffusion XL".to_string(),
                        "Runway Gen-3".to_string(),
                        "Topaz upscaler".to_string(),
                    ],
                    training_assets: vec![
                        "Brand light presets".to_string(),
                        "Sponsor overlay pack".to_string(),
                    ],
                    approvals_required: vec!["CreativeDirector".to_string()],
                },
                AgentWorkflow {
                    workflow_id: "style-lut-lab".to_string(),
                    name: "Style LUT Lab".to_string(),
                    objective: "Prototype look-up tables + typography packs that Editron can apply during motion systems tier.".to_string(),
                    trigger_keywords: vec![
                        "lut".to_string(),
                        "typography".to_string(),
                        "style frame".to_string(),
                    ],
                    sla_minutes: 45,
                    stages: vec![
                        WorkflowStage {
                            name: "Reference Crawl".to_string(),
                            description: "Gather brand pod references + latest social mood boards.".to_string(),
                            output: "Mood sheet".to_string(),
                        },
                        WorkflowStage {
                            name: "Look Dev".to_string(),
                            description: "Generate LUT candidates + motion typography options.".to_string(),
                            output: "Style kit".to_string(),
                        },
                        WorkflowStage {
                            name: "Delivery".to_string(),
                            description: "Package LUTs, fonts, and usage notes for Editron integration.".to_string(),
                            output: "Style delivery pack".to_string(),
                        },
                    ],
                    deliverables: vec![
                        ".cube LUT set".to_string(),
                        "Typography & motion cheat sheet".to_string(),
                    ],
                    automation_stack: vec![
                        "DaVinci LUT Generator".to_string(),
                        "After Effects scripting".to_string(),
                    ],
                    training_assets: vec!["Brand bible".to_string()],
                    approvals_required: vec!["CreativeDirector".to_string()],
                },
            ],
        },
    ]
}
