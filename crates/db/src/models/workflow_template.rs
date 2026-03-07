use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Client engagement type
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ClientType {
    FoundationBuild,
    ManagedGrowth,
    Custom,
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::FoundationBuild => "foundation_build",
            Self::ManagedGrowth => "managed_growth",
            Self::Custom => "custom",
        };
        write!(f, "{}", s)
    }
}

/// Task execution type — who does the work
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTaskType {
    /// Agent executes autonomously
    Agent,
    /// Human must complete this task (review, approval, client delivery)
    HumanReview,
    /// Agent executes, human reviews before moving on
    Hybrid,
}

/// A task within a workflow phase
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowTaskTemplate {
    pub title: String,
    pub description: String,
    pub position: i32,
    pub task_type: WorkflowTaskType,
    /// Which agent role handles this task (scout, astra, creative, etc.)
    pub agent_role: Option<String>,
    /// Whether this task blocks the pipeline until a human approves
    pub requires_approval: bool,
    /// Priority level
    pub priority: String,
    /// Positions (within the same phase) this task depends on
    pub depends_on: Vec<i32>,
    /// Knowledge domains this task reads from
    pub knowledge_inputs: Vec<String>,
    /// Knowledge domains this task produces/enriches
    pub knowledge_outputs: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// A phase within a workflow template (becomes a board when scaffolded)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowPhaseTemplate {
    pub name: String,
    pub description: String,
    pub position: i32,
    /// Whether this phase repeats (for managed growth monthly cycles)
    pub is_recurring: bool,
    /// Tasks within this phase
    pub tasks: Vec<WorkflowTaskTemplate>,
}

/// A complete workflow template
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub client_type: ClientType,
    /// Whether the overall engagement is recurring (retainer vs project)
    pub is_recurring: bool,
    /// Phases of the workflow
    pub phases: Vec<WorkflowPhaseTemplate>,
}

/// Result of converting a deal to a project
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DealConversionResult {
    pub project_id: String,
    pub project_name: String,
    pub boards_created: i32,
    pub tasks_created: i32,
    pub dependencies_created: i32,
    pub template_used: String,
}

impl WorkflowTemplate {
    /// Returns all built-in workflow templates
    pub fn defaults() -> Vec<Self> {
        vec![Self::foundation_build(), Self::managed_growth()]
    }

    /// Foundation Build: new client needs brand, website, social presence
    pub fn foundation_build() -> Self {
        Self {
            id: "foundation_build".to_string(),
            name: "Foundation Build".to_string(),
            description: "Complete brand foundation for new clients: brand guide, website/dealflow engine, and social media presence with high-quality content.".to_string(),
            client_type: ClientType::FoundationBuild,
            is_recurring: false,
            phases: vec![
                // ── Phase 1: Discovery & Research ──
                WorkflowPhaseTemplate {
                    name: "Discovery & Research".to_string(),
                    description: "Deep research into the client's industry, competitors, and target audience to inform all downstream work.".to_string(),
                    position: 0,
                    is_recurring: false,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Industry & Market Research".to_string(),
                            description: "Research the client's industry landscape, market trends, key players, and opportunities. Build a comprehensive market context document.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec![],
                            knowledge_outputs: vec!["market_research".to_string(), "industry_context".to_string()],
                            tags: vec!["research".to_string(), "discovery".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Competitor Analysis".to_string(),
                            description: "Identify and analyze direct and indirect competitors. Assess their brand positioning, online presence, content strategy, and strengths/weaknesses.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec![],
                            knowledge_outputs: vec!["competitor_analysis".to_string(), "competitive_landscape".to_string()],
                            tags: vec!["research".to_string(), "competitors".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Target Audience Research".to_string(),
                            description: "Define and research the client's ideal customer profiles. Identify demographics, psychographics, pain points, and where they spend time online.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec!["market_research".to_string()],
                            knowledge_outputs: vec!["audience_personas".to_string(), "audience_insights".to_string()],
                            tags: vec!["research".to_string(), "audience".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Client Brand Audit".to_string(),
                            description: "Audit any existing brand materials, social accounts, and online presence. Document current state and gaps to address.".to_string(),
                            position: 3,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "medium".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec![],
                            knowledge_outputs: vec!["brand_audit".to_string(), "current_state".to_string()],
                            tags: vec!["research".to_string(), "audit".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Review Research Findings".to_string(),
                            description: "Review all research outputs. Validate findings, add context from client conversations, and approve the research foundation before proceeding to brand strategy.".to_string(),
                            position: 4,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: true,
                            priority: "critical".to_string(),
                            depends_on: vec![0, 1, 2, 3],
                            knowledge_inputs: vec!["market_research".to_string(), "competitor_analysis".to_string(), "audience_personas".to_string(), "brand_audit".to_string()],
                            knowledge_outputs: vec!["validated_research".to_string()],
                            tags: vec!["review".to_string(), "gate".to_string()],
                        },
                    ],
                },
                // ── Phase 2: Brand Guide ──
                WorkflowPhaseTemplate {
                    name: "Brand Guide".to_string(),
                    description: "Develop the complete brand identity: positioning, voice, visual system, and comprehensive brand guide document.".to_string(),
                    position: 1,
                    is_recurring: false,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Brand Strategy & Positioning".to_string(),
                            description: "Define brand positioning, unique value proposition, brand promise, and strategic narrative. This becomes the foundation for all brand expressions.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("astra".to_string()),
                            requires_approval: false,
                            priority: "critical".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec!["validated_research".to_string(), "competitor_analysis".to_string(), "audience_personas".to_string()],
                            knowledge_outputs: vec!["brand_strategy".to_string(), "brand_positioning".to_string()],
                            tags: vec!["strategy".to_string(), "brand".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Brand Voice & Messaging Framework".to_string(),
                            description: "Develop the brand voice guidelines, tone variations, key messages, taglines, and elevator pitch. Define how the brand speaks across channels.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("astra".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["brand_strategy".to_string(), "audience_personas".to_string()],
                            knowledge_outputs: vec!["brand_voice".to_string(), "messaging_framework".to_string()],
                            tags: vec!["brand".to_string(), "messaging".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Visual Identity System".to_string(),
                            description: "Design the visual brand system: color palette, typography, logo usage guidelines, imagery style, and graphic elements.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["brand_strategy".to_string(), "competitor_analysis".to_string()],
                            knowledge_outputs: vec!["visual_identity".to_string(), "design_system".to_string()],
                            tags: vec!["brand".to_string(), "design".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Compile Brand Guide Document".to_string(),
                            description: "Compile all brand elements into a comprehensive, professional brand guide document ready for client delivery.".to_string(),
                            position: 3,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![1, 2],
                            knowledge_inputs: vec!["brand_strategy".to_string(), "brand_voice".to_string(), "visual_identity".to_string()],
                            knowledge_outputs: vec!["brand_guide".to_string()],
                            tags: vec!["brand".to_string(), "deliverable".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Brand Guide Review & Approval".to_string(),
                            description: "Review the complete brand guide for quality, consistency, and alignment with client vision. Refine and approve before moving to website development.".to_string(),
                            position: 4,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: true,
                            priority: "critical".to_string(),
                            depends_on: vec![3],
                            knowledge_inputs: vec!["brand_guide".to_string()],
                            knowledge_outputs: vec!["approved_brand_guide".to_string()],
                            tags: vec!["review".to_string(), "gate".to_string(), "deliverable".to_string()],
                        },
                    ],
                },
                // ── Phase 3: Website & Dealflow Engine ──
                WorkflowPhaseTemplate {
                    name: "Website & Dealflow".to_string(),
                    description: "Design and build the client's website with integrated dealflow engine for lead capture and conversion.".to_string(),
                    position: 2,
                    is_recurring: false,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Site Architecture & Content Strategy".to_string(),
                            description: "Plan the website structure, page hierarchy, user flows, and content strategy. Define the dealflow funnel: lead capture points, CTAs, and conversion paths.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("astra".to_string()),
                            requires_approval: false,
                            priority: "critical".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec!["approved_brand_guide".to_string(), "audience_personas".to_string(), "competitor_analysis".to_string()],
                            knowledge_outputs: vec!["site_architecture".to_string(), "content_strategy".to_string(), "dealflow_plan".to_string()],
                            tags: vec!["website".to_string(), "strategy".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Website Copy & Content".to_string(),
                            description: "Write all website copy: homepage, about, services, landing pages, and CTAs. Align with brand voice and SEO best practices.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["site_architecture".to_string(), "brand_voice".to_string(), "audience_personas".to_string()],
                            knowledge_outputs: vec!["website_copy".to_string()],
                            tags: vec!["website".to_string(), "content".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Website Design & Development".to_string(),
                            description: "Design and develop the website following the brand guide and site architecture. Implement responsive design, dealflow engine, and analytics.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0, 1],
                            knowledge_inputs: vec!["site_architecture".to_string(), "visual_identity".to_string(), "website_copy".to_string(), "dealflow_plan".to_string()],
                            knowledge_outputs: vec!["website_build".to_string()],
                            tags: vec!["website".to_string(), "development".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Dealflow Engine Setup".to_string(),
                            description: "Configure lead capture forms, email sequences, booking integration, CRM connections, and conversion tracking.".to_string(),
                            position: 3,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![2],
                            knowledge_inputs: vec!["dealflow_plan".to_string(), "website_build".to_string()],
                            knowledge_outputs: vec!["dealflow_engine".to_string()],
                            tags: vec!["website".to_string(), "dealflow".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Website Review & QA".to_string(),
                            description: "Comprehensive review of the website: design fidelity, copy quality, dealflow functionality, mobile responsiveness, page speed, and SEO.".to_string(),
                            position: 4,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: true,
                            priority: "critical".to_string(),
                            depends_on: vec![2, 3],
                            knowledge_inputs: vec!["website_build".to_string(), "dealflow_engine".to_string()],
                            knowledge_outputs: vec!["approved_website".to_string()],
                            tags: vec!["review".to_string(), "gate".to_string(), "deliverable".to_string()],
                        },
                    ],
                },
                // ── Phase 4: Social Media ──
                WorkflowPhaseTemplate {
                    name: "Social Media Setup".to_string(),
                    description: "Establish and optimize social media presence with initial high-quality content library.".to_string(),
                    position: 3,
                    is_recurring: false,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Social Landscape Analysis".to_string(),
                            description: "Analyze the social media landscape for the client's industry. Identify best platforms, content formats, posting cadences, and engagement tactics.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec!["competitor_analysis".to_string(), "audience_personas".to_string(), "approved_brand_guide".to_string()],
                            knowledge_outputs: vec!["social_landscape".to_string(), "platform_strategy".to_string()],
                            tags: vec!["social".to_string(), "research".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Social Account Setup & Optimization".to_string(),
                            description: "Create and optimize social media profiles across selected platforms. Apply brand identity, write bios, configure settings.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["approved_brand_guide".to_string(), "platform_strategy".to_string()],
                            knowledge_outputs: vec!["social_accounts".to_string()],
                            tags: vec!["social".to_string(), "setup".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Content Calendar & Strategy".to_string(),
                            description: "Develop a 30-day content calendar with themes, topics, and posting schedule aligned with the brand and audience preferences.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("astra".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["platform_strategy".to_string(), "brand_voice".to_string(), "audience_insights".to_string()],
                            knowledge_outputs: vec!["content_calendar".to_string(), "content_strategy".to_string()],
                            tags: vec!["social".to_string(), "strategy".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Initial Content Library Production".to_string(),
                            description: "Produce the initial batch of high-quality content: posts, graphics, captions, and hashtag sets for the first 30 days.".to_string(),
                            position: 3,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![1, 2],
                            knowledge_inputs: vec!["content_calendar".to_string(), "approved_brand_guide".to_string(), "social_accounts".to_string()],
                            knowledge_outputs: vec!["content_library".to_string()],
                            tags: vec!["social".to_string(), "content".to_string(), "production".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Content Review & Launch Approval".to_string(),
                            description: "Review all social content for brand alignment, quality, and accuracy. Approve for scheduling and launch.".to_string(),
                            position: 4,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: true,
                            priority: "critical".to_string(),
                            depends_on: vec![3],
                            knowledge_inputs: vec!["content_library".to_string()],
                            knowledge_outputs: vec!["approved_content".to_string()],
                            tags: vec!["review".to_string(), "gate".to_string(), "deliverable".to_string()],
                        },
                    ],
                },
            ],
        }
    }

    /// Managed Growth: existing client needs professional content and platform management
    pub fn managed_growth() -> Self {
        Self {
            id: "managed_growth".to_string(),
            name: "Managed Growth".to_string(),
            description: "Ongoing professional content production and platform management for established clients seeking growth.".to_string(),
            client_type: ClientType::ManagedGrowth,
            is_recurring: true,
            phases: vec![
                // ── Phase 1: Audit & Strategy ──
                WorkflowPhaseTemplate {
                    name: "Audit & Strategy".to_string(),
                    description: "Comprehensive audit of current performance and development of growth strategy.".to_string(),
                    position: 0,
                    is_recurring: false,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Platform Performance Audit".to_string(),
                            description: "Analyze current performance across all platforms: engagement rates, growth trends, content performance, audience demographics, and conversion data.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "critical".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec![],
                            knowledge_outputs: vec!["performance_audit".to_string(), "platform_metrics".to_string()],
                            tags: vec!["audit".to_string(), "analytics".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Content & Competitor Analysis".to_string(),
                            description: "Analyze the client's existing content library and competitor strategies. Identify gaps, opportunities, and winning formats.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec![],
                            knowledge_outputs: vec!["content_audit".to_string(), "competitor_analysis".to_string()],
                            tags: vec!["audit".to_string(), "research".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Growth Strategy Development".to_string(),
                            description: "Synthesize audit findings into a comprehensive growth strategy with clear KPIs, content pillars, platform priorities, and a 90-day roadmap.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("astra".to_string()),
                            requires_approval: false,
                            priority: "critical".to_string(),
                            depends_on: vec![0, 1],
                            knowledge_inputs: vec!["performance_audit".to_string(), "content_audit".to_string(), "competitor_analysis".to_string()],
                            knowledge_outputs: vec!["growth_strategy".to_string(), "content_pillars".to_string(), "kpi_targets".to_string()],
                            tags: vec!["strategy".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Strategy Review & Approval".to_string(),
                            description: "Review the growth strategy with the team. Validate KPIs, adjust priorities, and approve before execution begins.".to_string(),
                            position: 3,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: true,
                            priority: "critical".to_string(),
                            depends_on: vec![2],
                            knowledge_inputs: vec!["growth_strategy".to_string()],
                            knowledge_outputs: vec!["approved_strategy".to_string()],
                            tags: vec!["review".to_string(), "gate".to_string()],
                        },
                    ],
                },
                // ── Phase 2: Content Production (Monthly Recurring) ──
                WorkflowPhaseTemplate {
                    name: "Content Production".to_string(),
                    description: "Monthly content creation cycle: research trends, plan calendar, produce content batch.".to_string(),
                    position: 1,
                    is_recurring: true,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Trend & Topic Research".to_string(),
                            description: "Research current trends, seasonal topics, industry news, and audience interests for the upcoming content cycle.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec!["approved_strategy".to_string(), "content_pillars".to_string(), "audience_insights".to_string()],
                            knowledge_outputs: vec!["trend_research".to_string(), "topic_ideas".to_string()],
                            tags: vec!["research".to_string(), "content".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Content Calendar Planning".to_string(),
                            description: "Build the monthly content calendar with specific posts, themes, formats, and scheduling aligned with the growth strategy.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("astra".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["trend_research".to_string(), "content_pillars".to_string(), "platform_metrics".to_string()],
                            knowledge_outputs: vec!["content_calendar".to_string()],
                            tags: vec!["planning".to_string(), "content".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Content Batch Production".to_string(),
                            description: "Produce the full month's content: copy, graphics, video scripts, captions, hashtags, and alt text. Follow brand guide and content calendar.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![1],
                            knowledge_inputs: vec!["content_calendar".to_string(), "brand_voice".to_string(), "visual_identity".to_string()],
                            knowledge_outputs: vec!["content_batch".to_string()],
                            tags: vec!["production".to_string(), "content".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Content Review & Approval".to_string(),
                            description: "Review all produced content for quality, brand alignment, accuracy, and platform optimization. Approve for scheduling.".to_string(),
                            position: 3,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: true,
                            priority: "critical".to_string(),
                            depends_on: vec![2],
                            knowledge_inputs: vec!["content_batch".to_string()],
                            knowledge_outputs: vec!["approved_content".to_string()],
                            tags: vec!["review".to_string(), "gate".to_string(), "monthly".to_string()],
                        },
                    ],
                },
                // ── Phase 3: Platform Management (Monthly Recurring) ──
                WorkflowPhaseTemplate {
                    name: "Platform Management".to_string(),
                    description: "Monthly platform management: scheduling, publishing, engagement monitoring, and community management.".to_string(),
                    position: 2,
                    is_recurring: true,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Content Scheduling & Publishing".to_string(),
                            description: "Schedule approved content across platforms using optimal posting times. Ensure proper formatting per platform.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec!["approved_content".to_string(), "platform_metrics".to_string()],
                            knowledge_outputs: vec!["publishing_log".to_string()],
                            tags: vec!["publishing".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Engagement Monitoring".to_string(),
                            description: "Monitor engagement across platforms: track comments, mentions, DMs, and audience sentiment. Flag opportunities and issues.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "medium".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["publishing_log".to_string()],
                            knowledge_outputs: vec!["engagement_data".to_string(), "sentiment_analysis".to_string()],
                            tags: vec!["monitoring".to_string(), "engagement".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Community Management".to_string(),
                            description: "Respond to comments, DMs, and mentions. Foster community engagement and handle any issues or opportunities identified during monitoring.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::Hybrid,
                            agent_role: Some("creative".to_string()),
                            requires_approval: false,
                            priority: "medium".to_string(),
                            depends_on: vec![1],
                            knowledge_inputs: vec!["engagement_data".to_string(), "brand_voice".to_string()],
                            knowledge_outputs: vec!["community_log".to_string()],
                            tags: vec!["community".to_string(), "engagement".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Performance Check-in".to_string(),
                            description: "Mid-cycle review of content performance and platform health. Adjust strategy if needed.".to_string(),
                            position: 3,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: false,
                            priority: "medium".to_string(),
                            depends_on: vec![1, 2],
                            knowledge_inputs: vec!["engagement_data".to_string(), "community_log".to_string()],
                            knowledge_outputs: vec!["performance_notes".to_string()],
                            tags: vec!["review".to_string(), "monthly".to_string()],
                        },
                    ],
                },
                // ── Phase 4: Reporting & Optimization ──
                WorkflowPhaseTemplate {
                    name: "Reporting & Optimization".to_string(),
                    description: "Monthly performance reporting, insight synthesis, and strategy optimization.".to_string(),
                    position: 3,
                    is_recurring: true,
                    tasks: vec![
                        WorkflowTaskTemplate {
                            title: "Analytics & Performance Report".to_string(),
                            description: "Compile comprehensive performance report: metrics vs KPIs, content performance rankings, audience growth, engagement trends, and conversion data.".to_string(),
                            position: 0,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("scout".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![],
                            knowledge_inputs: vec!["platform_metrics".to_string(), "engagement_data".to_string(), "kpi_targets".to_string()],
                            knowledge_outputs: vec!["performance_report".to_string(), "platform_metrics".to_string()],
                            tags: vec!["analytics".to_string(), "reporting".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Optimization Recommendations".to_string(),
                            description: "Analyze performance data and generate actionable optimization recommendations: content adjustments, platform strategy shifts, new opportunities.".to_string(),
                            position: 1,
                            task_type: WorkflowTaskType::Agent,
                            agent_role: Some("astra".to_string()),
                            requires_approval: false,
                            priority: "high".to_string(),
                            depends_on: vec![0],
                            knowledge_inputs: vec!["performance_report".to_string(), "growth_strategy".to_string(), "trend_research".to_string()],
                            knowledge_outputs: vec!["optimization_plan".to_string(), "strategy_updates".to_string()],
                            tags: vec!["strategy".to_string(), "optimization".to_string(), "monthly".to_string()],
                        },
                        WorkflowTaskTemplate {
                            title: "Report Review & Client Delivery".to_string(),
                            description: "Review the performance report and optimization recommendations. Prepare client-ready deliverable and schedule review meeting.".to_string(),
                            position: 2,
                            task_type: WorkflowTaskType::HumanReview,
                            agent_role: None,
                            requires_approval: true,
                            priority: "critical".to_string(),
                            depends_on: vec![0, 1],
                            knowledge_inputs: vec!["performance_report".to_string(), "optimization_plan".to_string()],
                            knowledge_outputs: vec!["client_report".to_string(), "approved_optimizations".to_string()],
                            tags: vec!["review".to_string(), "gate".to_string(), "deliverable".to_string(), "monthly".to_string()],
                        },
                    ],
                },
            ],
        }
    }
}
