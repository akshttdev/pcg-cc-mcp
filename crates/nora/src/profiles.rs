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
        // ============================================================================
        // SOCIAL MEDIA AGENTS (NORA Social Command Suite)
        // ============================================================================
        AgentProfile {
            agent_id: "scout-research".to_string(),
            codename: "Scout".to_string(),
            title: "Social Intelligence Analyst".to_string(),
            mission: "Conduct deep reconnaissance across social platforms to surface competitor moves, trending topics, and engagement opportunities.".to_string(),
            specialization: AgentDiscipline::Intelligence,
            status: AgentStatus::Active,
            capabilities: vec![
                "competitor_analysis".to_string(),
                "trend_detection".to_string(),
                "hashtag_research".to_string(),
                "audience_profiling".to_string(),
                "sentiment_analysis".to_string(),
            ],
            strengths: vec![
                "Multi-platform data aggregation and normalization".to_string(),
                "Pattern recognition across competitor content strategies".to_string(),
                "Real-time trend surfacing with relevance scoring".to_string(),
            ],
            current_focus: vec![
                "Prime Hospitality competitor landscape".to_string(),
                "Instagram engagement pattern analysis".to_string(),
            ],
            operating_mode: "Engages on research requests, runs scheduled competitor scans, and proactively alerts on significant trend shifts.".to_string(),
            escalation_path: vec![
                "Escalate to Oracle when research reveals strategic opportunities".to_string(),
                "Alert Nora on high-priority competitor moves".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 0,
                average_response_time_ms: 2500.0,
                success_rate: 0.95,
                uptime_percentage: 0.99,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "competitor-deep-dive".to_string(),
                    name: "Competitor Deep Dive".to_string(),
                    objective: "Analyze competitor social presence and extract actionable insights.".to_string(),
                    trigger_keywords: vec![
                        "competitor".to_string(),
                        "research".to_string(),
                        "analyze".to_string(),
                        "scout".to_string(),
                    ],
                    sla_minutes: 45,
                    stages: vec![
                        WorkflowStage {
                            name: "Account Discovery".to_string(),
                            description: "Identify and catalog competitor accounts across platforms.".to_string(),
                            output: "Competitor account inventory".to_string(),
                        },
                        WorkflowStage {
                            name: "Content Analysis".to_string(),
                            description: "Analyze posting patterns, content themes, and engagement metrics.".to_string(),
                            output: "Content strategy breakdown".to_string(),
                        },
                        WorkflowStage {
                            name: "Insight Synthesis".to_string(),
                            description: "Distill findings into actionable recommendations.".to_string(),
                            output: "Research report artifact".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Research report artifact".to_string(),
                        "Competitor comparison matrix".to_string(),
                    ],
                    automation_stack: vec![
                        "Instagram Graph API".to_string(),
                        "LinkedIn API".to_string(),
                        "Web scraping pipeline".to_string(),
                    ],
                    training_assets: vec!["Industry benchmarks".to_string()],
                    approvals_required: vec![],
                },
            ],
        },
        AgentProfile {
            agent_id: "oracle-strategy".to_string(),
            codename: "Oracle".to_string(),
            title: "Content Strategy Architect".to_string(),
            mission: "Transform business goals and research insights into comprehensive content calendars and campaign strategies.".to_string(),
            specialization: AgentDiscipline::Strategy,
            status: AgentStatus::Active,
            capabilities: vec![
                "content_calendar_planning".to_string(),
                "campaign_architecture".to_string(),
                "posting_optimization".to_string(),
                "content_pillar_design".to_string(),
                "seasonal_planning".to_string(),
            ],
            strengths: vec![
                "Data-driven optimal posting time calculation".to_string(),
                "Content pillar mapping to business objectives".to_string(),
                "Platform-specific strategy adaptation".to_string(),
            ],
            current_focus: vec![
                "Prime Hospitality 30-day content calendar".to_string(),
                "Instagram launch strategy".to_string(),
            ],
            operating_mode: "Engages when content strategy is needed, receives Scout research handoffs, and generates strategies for Muse execution.".to_string(),
            escalation_path: vec![
                "Hand off content plans to Muse for creation".to_string(),
                "Escalate to Nora for budget-impacting strategy decisions".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 0,
                average_response_time_ms: 3000.0,
                success_rate: 0.93,
                uptime_percentage: 0.98,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "content-calendar-30day".to_string(),
                    name: "30-Day Content Calendar".to_string(),
                    objective: "Create a comprehensive 30-day content plan aligned with business goals.".to_string(),
                    trigger_keywords: vec![
                        "content plan".to_string(),
                        "calendar".to_string(),
                        "strategy".to_string(),
                        "30 day".to_string(),
                    ],
                    sla_minutes: 60,
                    stages: vec![
                        WorkflowStage {
                            name: "Research Integration".to_string(),
                            description: "Ingest Scout's research report and business objectives.".to_string(),
                            output: "Strategy brief".to_string(),
                        },
                        WorkflowStage {
                            name: "Pillar Definition".to_string(),
                            description: "Define content pillars and category distribution.".to_string(),
                            output: "Content pillar framework".to_string(),
                        },
                        WorkflowStage {
                            name: "Calendar Build".to_string(),
                            description: "Generate dated posts with optimal timing and category rotation.".to_string(),
                            output: "30-day calendar".to_string(),
                        },
                        WorkflowStage {
                            name: "Task Creation".to_string(),
                            description: "Create individual tasks on the social_assets board for each post.".to_string(),
                            output: "Board populated with scheduled tasks".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Strategy document artifact".to_string(),
                        "Content calendar artifact".to_string(),
                        "Tasks with due dates on board".to_string(),
                    ],
                    automation_stack: vec![
                        "PCG Task API".to_string(),
                        "Calendar board integration".to_string(),
                    ],
                    training_assets: vec![
                        "Platform best practices".to_string(),
                        "Industry content benchmarks".to_string(),
                    ],
                    approvals_required: vec!["User".to_string()],
                },
            ],
        },
        AgentProfile {
            agent_id: "muse-creative".to_string(),
            codename: "Muse".to_string(),
            title: "Content Creation Specialist".to_string(),
            mission: "Transform content briefs into platform-ready posts with compelling copy, optimized hashtags, and media specifications.".to_string(),
            specialization: AgentDiscipline::Creative,
            status: AgentStatus::Active,
            capabilities: vec![
                "copywriting".to_string(),
                "hashtag_optimization".to_string(),
                "platform_adaptation".to_string(),
                "visual_brief_creation".to_string(),
                "hook_generation".to_string(),
            ],
            strengths: vec![
                "Brand voice consistency across platforms".to_string(),
                "Engagement-optimized copy formulas".to_string(),
                "Multi-platform content adaptation".to_string(),
            ],
            current_focus: vec![
                "Prime Hospitality brand voice development".to_string(),
                "Instagram post creation pipeline".to_string(),
            ],
            operating_mode: "Receives tasks from Oracle's calendar, creates content drafts, and hands off to Herald for publishing.".to_string(),
            escalation_path: vec![
                "Request user approval for content before Herald publishing".to_string(),
                "Escalate to Spectra for AI media generation needs".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 0,
                average_response_time_ms: 2000.0,
                success_rate: 0.91,
                uptime_percentage: 0.97,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "content-creation".to_string(),
                    name: "Content Creation".to_string(),
                    objective: "Create engaging, platform-optimized content from strategy briefs.".to_string(),
                    trigger_keywords: vec![
                        "create post".to_string(),
                        "write content".to_string(),
                        "draft".to_string(),
                        "caption".to_string(),
                    ],
                    sla_minutes: 30,
                    stages: vec![
                        WorkflowStage {
                            name: "Brief Analysis".to_string(),
                            description: "Parse content brief and identify key messaging requirements.".to_string(),
                            output: "Content requirements".to_string(),
                        },
                        WorkflowStage {
                            name: "Copy Creation".to_string(),
                            description: "Write hook, body, CTA, and hashtags following brand voice.".to_string(),
                            output: "Content draft artifact".to_string(),
                        },
                        WorkflowStage {
                            name: "Platform Adaptation".to_string(),
                            description: "Adapt content for each target platform's requirements.".to_string(),
                            output: "Platform-specific versions".to_string(),
                        },
                        WorkflowStage {
                            name: "Visual Brief".to_string(),
                            description: "Create specifications for visual content if needed.".to_string(),
                            output: "Visual brief artifact".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Content draft artifact".to_string(),
                        "Visual brief artifact".to_string(),
                        "Platform adaptation notes".to_string(),
                    ],
                    automation_stack: vec![
                        "LLM content generation".to_string(),
                        "Brand voice embeddings".to_string(),
                    ],
                    training_assets: vec![
                        "Brand voice guide".to_string(),
                        "High-performing post examples".to_string(),
                    ],
                    approvals_required: vec!["User".to_string()],
                },
            ],
        },
        AgentProfile {
            agent_id: "herald-distribution".to_string(),
            codename: "Herald".to_string(),
            title: "Content Distribution Manager".to_string(),
            mission: "Execute flawless multi-platform content publishing with optimal timing and category queue rotation.".to_string(),
            specialization: AgentDiscipline::Operations,
            status: AgentStatus::Active,
            capabilities: vec![
                "multi_platform_publishing".to_string(),
                "schedule_management".to_string(),
                "queue_rotation".to_string(),
                "publish_verification".to_string(),
                "evergreen_recycling".to_string(),
            ],
            strengths: vec![
                "Platform API integration expertise".to_string(),
                "GoHighLevel-style category queue management".to_string(),
                "Publish verification and retry logic".to_string(),
            ],
            current_focus: vec![
                "Prime Hospitality Instagram publishing pipeline".to_string(),
                "Category queue optimization".to_string(),
            ],
            operating_mode: "Executes scheduled publishing, manages category queue rotation, and verifies successful posts.".to_string(),
            escalation_path: vec![
                "Hand off to Echo for post-publish engagement monitoring".to_string(),
                "Escalate to Nora on publishing failures".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 0,
                average_response_time_ms: 1500.0,
                success_rate: 0.98,
                uptime_percentage: 0.99,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "content-publishing".to_string(),
                    name: "Content Publishing".to_string(),
                    objective: "Publish approved content to target platforms at optimal times.".to_string(),
                    trigger_keywords: vec![
                        "publish".to_string(),
                        "post".to_string(),
                        "schedule".to_string(),
                        "distribute".to_string(),
                    ],
                    sla_minutes: 15,
                    stages: vec![
                        WorkflowStage {
                            name: "Pre-flight Check".to_string(),
                            description: "Verify content approval, media availability, and account tokens.".to_string(),
                            output: "Ready for publish confirmation".to_string(),
                        },
                        WorkflowStage {
                            name: "Platform Publish".to_string(),
                            description: "Execute publish via platform APIs with retry logic.".to_string(),
                            output: "Publish result".to_string(),
                        },
                        WorkflowStage {
                            name: "Verification".to_string(),
                            description: "Confirm post is live and capture platform post ID/URL.".to_string(),
                            output: "Schedule manifest artifact".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Schedule manifest artifact".to_string(),
                        "Platform post URLs".to_string(),
                    ],
                    automation_stack: vec![
                        "Instagram Graph API".to_string(),
                        "LinkedIn API".to_string(),
                        "Social publisher service".to_string(),
                    ],
                    training_assets: vec!["Platform publishing guidelines".to_string()],
                    approvals_required: vec![],
                },
            ],
        },
        AgentProfile {
            agent_id: "echo-engagement".to_string(),
            codename: "Echo".to_string(),
            title: "Community Engagement Specialist".to_string(),
            mission: "Monitor, analyze, and respond to social engagement across all platforms while maintaining brand voice consistency.".to_string(),
            specialization: AgentDiscipline::Communications,
            status: AgentStatus::Active,
            capabilities: vec![
                "mention_monitoring".to_string(),
                "sentiment_analysis".to_string(),
                "response_drafting".to_string(),
                "engagement_analytics".to_string(),
                "crisis_detection".to_string(),
            ],
            strengths: vec![
                "Real-time inbox aggregation across platforms".to_string(),
                "Priority scoring for high-value engagement".to_string(),
                "Brand-voice-consistent response generation".to_string(),
            ],
            current_focus: vec![
                "Prime Hospitality engagement monitoring setup".to_string(),
                "Response template library building".to_string(),
            ],
            operating_mode: "Continuously monitors for mentions/comments, prioritizes high-value engagement, and drafts responses for approval.".to_string(),
            escalation_path: vec![
                "Escalate negative sentiment to Nora immediately".to_string(),
                "Flag VIP/influencer engagement for user attention".to_string(),
            ],
            metrics: PerformanceMetrics {
                tasks_completed: 0,
                average_response_time_ms: 1000.0,
                success_rate: 0.94,
                uptime_percentage: 0.99,
            },
            workflows: vec![
                AgentWorkflow {
                    workflow_id: "engagement-response".to_string(),
                    name: "Engagement Response".to_string(),
                    objective: "Respond to social mentions and comments while maintaining brand voice.".to_string(),
                    trigger_keywords: vec![
                        "respond".to_string(),
                        "reply".to_string(),
                        "engage".to_string(),
                        "comment".to_string(),
                    ],
                    sla_minutes: 30,
                    stages: vec![
                        WorkflowStage {
                            name: "Inbox Scan".to_string(),
                            description: "Fetch and prioritize new mentions across platforms.".to_string(),
                            output: "Prioritized mention queue".to_string(),
                        },
                        WorkflowStage {
                            name: "Sentiment Analysis".to_string(),
                            description: "Classify sentiment and flag urgent items.".to_string(),
                            output: "Sentiment tags".to_string(),
                        },
                        WorkflowStage {
                            name: "Response Draft".to_string(),
                            description: "Generate brand-appropriate response drafts.".to_string(),
                            output: "Response drafts".to_string(),
                        },
                        WorkflowStage {
                            name: "Execute & Log".to_string(),
                            description: "Post approved responses and update engagement log.".to_string(),
                            output: "Engagement log artifact".to_string(),
                        },
                    ],
                    deliverables: vec![
                        "Engagement log artifact".to_string(),
                        "Response analytics".to_string(),
                    ],
                    automation_stack: vec![
                        "Unified inbox aggregator".to_string(),
                        "Sentiment analysis model".to_string(),
                        "Platform reply APIs".to_string(),
                    ],
                    training_assets: vec![
                        "Response template library".to_string(),
                        "Brand voice guide".to_string(),
                    ],
                    approvals_required: vec!["User".to_string()],
                },
            ],
        },
    ]
}
