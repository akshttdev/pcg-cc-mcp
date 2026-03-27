use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use db::models::{
    data_source::DataSource,
    execution_artifact::ExecutionArtifact,
    pcg_router_model::PcgRouterModel,
    workflow_run::{CreateWorkflowRun, WorkflowRun},
    workflow_staging::WorkflowStagingRecord,
};
use deployment::Deployment;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
pub use services::services::workflow_execution::{
    NodePosition, WorkflowConnection, WorkflowDefinition, WorkflowNode, build_schema_prompt_text,
    check_company_duplicate, check_contact_duplicate, check_deal_duplicate,
    check_intra_batch_duplicate, check_task_duplicate, compute_confidence, execute_action_node,
    execute_node_with_llm, extract_records_from_output, is_fallback_placeholder,
    validate_record_against_schema,
};
use utils::response::ApiResponse;
use uuid::Uuid;

use crate::{DeploymentImpl, error::ApiError};

// Legacy step type for backwards compat with run_workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyStep {
    id: String,
    name: String,
    prompt_template: String,
    depends_on: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateWorkflowRequest {
    id: String,
    name: String,
    description: Option<String>,
    nodes: Vec<WorkflowNode>,
    connections: Vec<WorkflowConnection>,
    owner_type: Option<String>, // "organization" or "user"
    owner_id: Option<String>,   // UUID of the owner
    default_model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateWorkflowRequest {
    name: Option<String>,
    description: Option<String>,
    nodes: Option<Vec<WorkflowNode>>,
    connections: Option<Vec<WorkflowConnection>>,
    default_model: Option<String>,
}

/// Request for dry-run preview (no artifacts saved)
#[derive(Debug, Deserialize)]
struct PreviewWorkflowRequest {
    nodes: Vec<WorkflowNode>,
    connections: Vec<WorkflowConnection>,
    /// Optional content to preview against (if not provided, uses sample text)
    content: Option<String>,
}

/// Response for a single step execution result
#[derive(Debug, Serialize, Deserialize)]
struct StepResult {
    step_id: String,
    step_name: String,
    artifact_id: Uuid,
}

/// Response for a full workflow run
#[derive(Debug, Serialize, Deserialize)]
struct WorkflowRunResult {
    workflow_run_id: Uuid,
    workflow_id: String,
    workflow_name: String,
    data_source_id: Uuid,
    steps: Vec<StepResult>,
    total_usage: Option<Value>,
    staged_records: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    reused: Option<bool>,
}

// ── Default workflow seed ────────────────────────────────────────────────────

fn default_analysis_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "default_analysis".to_string(),
        name: "Data Source Analysis".to_string(),
        description: Some(
            "Extract companies, contacts, and opportunities from data source content.".to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "extract_companies".to_string(),
                name: "Extract Companies".to_string(),
                node_type: "llm_extract".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Analyze the following data source content and extract all mentioned ",
                        "companies and organizations. For each company, provide:\n",
                        "- name: The company/organization name\n",
                        "- context: How/where it was mentioned\n",
                        "- relationship: One of potential_client, existing_client, partner, competitor, vendor, other\n\n",
                        "Output as structured JSON with a top-level \"companies\" array.\n\n",
                        "Content:\n{{content}}"
                    ),
                    "output_schema": "companies[]"
                }),
                position: NodePosition { x: 100.0, y: 100.0 },
            },
            WorkflowNode {
                id: "extract_contacts".to_string(),
                name: "Extract Contacts".to_string(),
                node_type: "llm_extract".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Analyze the following data source content and extract all mentioned ",
                        "people and contacts. For each person, provide:\n",
                        "- first_name: First/given name\n",
                        "- last_name: Last/family name\n",
                        "- email: Email address if available, null otherwise\n",
                        "- phone: Phone number if available, null otherwise\n",
                        "- company_name: The company they are associated with, null if unknown\n",
                        "- job_title: Their role or title if mentioned, null otherwise\n",
                        "- department: Their department if mentioned, null otherwise\n",
                        "- linkedin_url: LinkedIn URL if available, null otherwise\n\n",

                        "Output as structured JSON with a top-level \"contacts\" array.\n\n",
                        "Content:\n{{content}}"
                    ),
                    "output_schema": "contacts[]"
                }),
                position: NodePosition { x: 100.0, y: 300.0 },
            },
            WorkflowNode {
                id: "identify_opportunities".to_string(),
                name: "Identify Opportunities".to_string(),
                node_type: "llm_analyze".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Analyze the following data source content in a business context. ",
                        "Identify potential business opportunities, partnerships, and follow-up actions. ",
                        "For each opportunity, provide:\n",
                        "- name: Short descriptive name for the deal\n",
                        "- description: Brief description of the opportunity\n",
                        "- amount: Estimated monetary value if possible, or null\n",
                        "- currency: Currency code (e.g. USD), default USD\n",
                        "- contact_email: Email of the primary contact for this deal, if known\n",
                        "- contact_name: Name of the primary contact, if known\n",
                        "- type: One of project, partnership, upsell, referral, other\n",

                        "- next_steps: Array of concrete next actions\n\n",
                        "Also consider any companies and contacts extracted in previous steps.\n\n",
                        "Output as structured JSON with a top-level \"opportunities\" array.\n\n",
                        "Content:\n{{content}}\n\n",
                        "Previous extraction results:\n{{previous_results}}"
                    ),
                    "output_schema": "opportunities[]"
                }),
                position: NodePosition { x: 500.0, y: 200.0 },
            },
        ],
        connections: vec![
            WorkflowConnection {
                source: "extract_companies".to_string(),
                target: "identify_opportunities".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
            WorkflowConnection {
                source: "extract_contacts".to_string(),
                target: "identify_opportunities".to_string(),
                source_output: Some(0),
                target_input: Some(1),
            },
        ],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

// ── Sprint 2D: Additional system workflows ──────────────────────────────────

/// Feedback Triage Pipeline: Analyze feedback/bug reports → filter by severity → create prioritized tasks
fn bug_triage_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "bug_triage_pipeline".to_string(),
        name: "Feedback Triage Pipeline".to_string(),
        description: Some(
            "Investigate and triage bug reports into 4 outcomes: critical fix-now, \
             low-cost fix-now, high-cost planning, or report-findings-to-user."
                .to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "investigate".to_string(),
                name: "Investigate & Triage".to_string(),
                node_type: "llm_analyze".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "You are a senior platform engineer triaging a bug report for the ORCHA ",
                        "dashboard (Rust/Axum backend, React/TypeScript frontend, SQLite DB).\n\n",
                        "INVESTIGATE the bug report below. Determine:\n",
                        "1. What component is affected (frontend, backend, db, mcp, workflow, infra)\n",
                        "2. Whether it's reproducible from the description\n",
                        "3. Estimated fix complexity (lines of code, number of files)\n",
                        "4. Risk of the bug (data loss, security, UX degradation, cosmetic)\n\n",
                        "Then CLASSIFY into exactly ONE triage outcome:\n\n",
                        "- **critical_fix_now**: Bug causes data loss, security issue, or blocks core ",
                        "functionality. Must be fixed immediately. Create a task with priority=critical, ",
                        "tags=[\"bug\",\"critical\",\"fix-now\"], and detailed completion_criteria.\n\n",
                        "- **low_cost_fix_now**: Bug is straightforward to fix (< 50 lines, 1-2 files). ",
                        "Just fix it. Create a task with priority=high, tags=[\"bug\",\"quick-fix\"], ",
                        "and completion_criteria.\n\n",
                        "- **high_cost_planning**: Bug requires significant refactoring or touches many ",
                        "files/systems. Create a task with priority=medium, tags=[\"bug\",\"needs-planning\"], ",
                        "description includes an investigation summary and proposed approach, and ",
                        "output_format=\"Planning document with implementation steps\".\n\n",
                        "- **report_findings**: Unclear if it's a real bug, needs more info, or is ",
                        "actually expected behavior. Create a task with priority=low, ",
                        "tags=[\"bug\",\"needs-triage\",\"awaiting-feedback\"], description includes ",
                        "findings and recommended options for the reporter.\n\n",
                        "OUTPUT as JSON:\n",
                        "```json\n",
                        "{\n",
                        "  \"triage_outcome\": \"critical_fix_now|low_cost_fix_now|high_cost_planning|report_findings\",\n",
                        "  \"investigation_summary\": \"What you found\",\n",
                        "  \"affected_component\": \"frontend|backend|db|mcp|workflow|infra\",\n",
                        "  \"estimated_fix_lines\": 25,\n",
                        "  \"estimated_files\": 2,\n",
                        "  \"risk_level\": \"critical|high|medium|low\",\n",
                        "  \"tasks\": [\n",
                        "    {\n",
                        "      \"title\": \"Fix: short description\",\n",
                        "      \"description\": \"Detailed description with context\",\n",
                        "      \"priority\": \"critical|high|medium|low\",\n",
                        "      \"tags\": [\"bug\", ...],\n",
                        "      \"completion_criteria\": \"Bullet list of done conditions\",\n",
                        "      \"output_format\": \"PR with tests|Planning document|Investigation report\",\n",
                        "      \"board_id\": \"d0600000-0000-0000-0000-000000000001\"\n",
                        "    }\n",
                        "  ]\n",
                        "}\n",
                        "```\n\n",
                        "Bug Report:\n{{content}}"
                    ),
                    "output_schema": "tasks[]"
                }),
                position: NodePosition { x: 100.0, y: 200.0 },
            },
            WorkflowNode {
                id: "create_tasks".to_string(),
                name: "Create Triage Tasks".to_string(),
                node_type: "output_tasks".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 500.0, y: 200.0 },
            },
        ],
        connections: vec![WorkflowConnection {
            source: "investigate".to_string(),
            target: "create_tasks".to_string(),
            source_output: Some(0),
            target_input: Some(0),
        }],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

/// Sprint Planning: Extract requirements → break into agent-ready tasks with completion criteria
fn sprint_planning_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "sprint_planning".to_string(),
        name: "Sprint Planning from Requirements".to_string(),
        description: Some(
            "Break down requirements documents into agent-ready tasks with completion criteria."
                .to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "extract_requirements".to_string(),
                name: "Extract Requirements".to_string(),
                node_type: "llm_extract".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Analyze the following requirements document and extract individual work items.\n",
                        "For each requirement, provide:\n",
                        "- title: Task title (action-oriented, e.g. 'Implement user login')\n",
                        "- description: Detailed task description with acceptance criteria\n",
                        "- priority: One of critical, high, medium, low\n",
                        "- estimated_effort: small, medium, large\n",
                        "- completion_criteria: Bullet-pointed list of what must be true for this to be done\n",
                        "- output_format: Expected deliverable (e.g. 'Pull request with tests', 'Design document')\n",
                        "- dependencies: Array of other task titles this depends on\n\n",
                        "Output as JSON with a top-level \"tasks\" array.\n\n",
                        "Content:\n{{content}}"
                    ),
                    "output_schema": "tasks[]"
                }),
                position: NodePosition { x: 100.0, y: 200.0 },
            },
            WorkflowNode {
                id: "output_sprint_tasks".to_string(),
                name: "Create Sprint Tasks".to_string(),
                node_type: "output_tasks".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 500.0, y: 200.0 },
            },
        ],
        connections: vec![WorkflowConnection {
            source: "extract_requirements".to_string(),
            target: "output_sprint_tasks".to_string(),
            source_output: Some(0),
            target_input: Some(0),
        }],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

/// Client Onboarding: Process client data → create CRM contacts → create onboarding tasks
fn client_onboarding_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "client_onboarding".to_string(),
        name: "Client Onboarding Pipeline".to_string(),
        description: Some(
            "Process new client data, create CRM contacts, and generate onboarding task checklist."
                .to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "extract_client_info".to_string(),
                name: "Extract Client Info".to_string(),
                node_type: "llm_extract".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Extract client and contact information from the following data.\n",
                        "Provide:\n",
                        "- contacts: Array of people with first_name, last_name, email, phone, job_title, company_name\n",
                        "- company: Object with name, website, industry, address\n",
                        "- onboarding_notes: Any special requirements or notes mentioned\n\n",
                        "Output as JSON.\n\n",
                        "Content:\n{{content}}"
                    ),
                    "output_schema": "contacts[]"
                }),
                position: NodePosition { x: 100.0, y: 200.0 },
            },
            WorkflowNode {
                id: "create_contacts".to_string(),
                name: "Create CRM Contacts".to_string(),
                node_type: "output_crm_contacts".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 500.0, y: 100.0 },
            },
            WorkflowNode {
                id: "create_onboarding_tasks".to_string(),
                name: "Create Onboarding Tasks".to_string(),
                node_type: "output_tasks".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 500.0, y: 300.0 },
            },
        ],
        connections: vec![
            WorkflowConnection {
                source: "extract_client_info".to_string(),
                target: "create_contacts".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
            WorkflowConnection {
                source: "extract_client_info".to_string(),
                target: "create_onboarding_tasks".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
        ],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

/// Content Pipeline: Summarize content → analyze SEO → create content tasks
fn content_pipeline_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "content_pipeline".to_string(),
        name: "Content Analysis Pipeline".to_string(),
        description: Some(
            "Analyze content for SEO opportunities and create actionable content tasks."
                .to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "summarize_content".to_string(),
                name: "Summarize Content".to_string(),
                node_type: "llm_summarize".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Summarize the following content, identifying:\n",
                        "- key_topics: Main topics covered\n",
                        "- target_audience: Who this content is for\n",
                        "- content_type: blog, documentation, marketing, technical, etc.\n",
                        "- word_count: Approximate word count\n",
                        "- summary: 2-3 sentence summary\n\n",
                        "Output as JSON.\n\n",
                        "Content:\n{{content}}"
                    ),
                    "output_schema": "summary"
                }),
                position: NodePosition { x: 100.0, y: 200.0 },
            },
            WorkflowNode {
                id: "analyze_seo".to_string(),
                name: "SEO Analysis".to_string(),
                node_type: "llm_analyze".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Based on the content summary below, perform an SEO analysis.\n",
                        "Provide:\n",
                        "- keywords: Array of target keywords with search intent\n",
                        "- content_gaps: Topics that should be covered but aren't\n",
                        "- optimization_tasks: Specific actions to improve SEO\n",
                        "- competitor_angles: Angles competitors might use\n\n",
                        "Output as JSON with arrays for each field.\n\n",
                        "Summary:\n{{previous_results}}"
                    ),
                    "output_schema": "seo_analysis"
                }),
                position: NodePosition { x: 500.0, y: 200.0 },
            },
            WorkflowNode {
                id: "create_content_tasks".to_string(),
                name: "Create Content Tasks".to_string(),
                node_type: "output_tasks".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 900.0, y: 200.0 },
            },
        ],
        connections: vec![
            WorkflowConnection {
                source: "summarize_content".to_string(),
                target: "analyze_seo".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
            WorkflowConnection {
                source: "analyze_seo".to_string(),
                target: "create_content_tasks".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
        ],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

// ── Demo Sprint: CRM-focused workflow templates ─────────────────────────────

/// Sales Conversation: Extract prospects, opportunities, and next steps from call transcripts
fn sales_conversation_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "sales_conversation".to_string(),
        name: "Sales Conversation Analysis".to_string(),
        description: Some(
            "Extract prospects, deals, and action items from sales call transcripts or meeting notes."
                .to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "extract_participants".to_string(),
                name: "Extract Participants".to_string(),
                node_type: "llm_extract".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Analyze this sales conversation transcript. Extract every person mentioned or participating.\n",
                        "For each person, provide:\n",
                        "- first_name: First/given name\n",
                        "- last_name: Last/family name\n",
                        "- email: Email if mentioned, null otherwise\n",
                        "- phone: Phone if mentioned, null otherwise\n",
                        "- company_name: Their company/organization\n",
                        "- job_title: Role or title if mentioned\n",
                        "- role_in_conversation: buyer, seller, decision_maker, influencer, technical_contact\n\n",
                        "Output as JSON with a top-level \"contacts\" array.\n\n",
                        "Transcript:\n{{content}}"
                    ),
                    "output_schema": "contacts[]"
                }),
                position: NodePosition { x: 100.0, y: 100.0 },
            },
            WorkflowNode {
                id: "extract_opportunities".to_string(),
                name: "Extract Opportunities".to_string(),
                node_type: "llm_analyze".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Analyze this sales conversation for business opportunities.\n",
                        "For each opportunity discussed, provide:\n",
                        "- name: Short deal name (e.g. 'Acme Corp Brand Refresh')\n",
                        "- description: What was discussed, key requirements, pain points\n",
                        "- amount: Estimated deal value if mentioned, null otherwise\n",
                        "- currency: USD unless stated otherwise\n",
                        "- contact_name: Primary contact for this opportunity\n",
                        "- contact_email: Their email if mentioned\n",
                        "- type: project, retainer, partnership, referral, or other\n",
                        "- urgency: hot, warm, cold based on conversation tone\n",
                        "- next_steps: Array of concrete follow-up actions with owners\n\n",
                        "Also consider participants extracted previously.\n\n",
                        "Output as JSON with a top-level \"opportunities\" array.\n\n",
                        "Transcript:\n{{content}}\n\n",
                        "Participants:\n{{previous_results}}"
                    ),
                    "output_schema": "opportunities[]"
                }),
                position: NodePosition { x: 500.0, y: 100.0 },
            },
            WorkflowNode {
                id: "create_follow_up_tasks".to_string(),
                name: "Create Follow-up Tasks".to_string(),
                node_type: "output_tasks".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 900.0, y: 100.0 },
            },
        ],
        connections: vec![
            WorkflowConnection {
                source: "extract_participants".to_string(),
                target: "extract_opportunities".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
            WorkflowConnection {
                source: "extract_opportunities".to_string(),
                target: "create_follow_up_tasks".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
        ],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

/// Prospect List: Parse CSV/text lists of potential clients into CRM contacts
fn prospect_list_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "prospect_list".to_string(),
        name: "Prospect List Import".to_string(),
        description: Some(
            "Parse a list of prospects (CSV, text, or pasted data) into structured CRM contacts with companies."
                .to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "parse_prospects".to_string(),
                name: "Parse Prospect Data".to_string(),
                node_type: "llm_extract".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Parse this prospect list into structured contacts. The input may be CSV, ",
                        "tab-separated, a pasted table, or free-form text with contact information.\n\n",
                        "For each prospect, extract:\n",
                        "- first_name: First/given name\n",
                        "- last_name: Last/family name\n",
                        "- email: Email address if available\n",
                        "- phone: Phone number if available\n",
                        "- company_name: Company or organization\n",
                        "- job_title: Title or role\n",
                        "- department: Department if mentioned\n",
                        "- linkedin_url: LinkedIn URL if available\n",
                        "- source: Where this prospect came from (infer from context)\n",
                        "- notes: Any additional context about this prospect\n\n",
                        "Be thorough — extract every person mentioned, even if some fields are missing.\n\n",
                        "Output as JSON with a top-level \"contacts\" array.\n\n",
                        "Prospect Data:\n{{content}}"
                    ),
                    "output_schema": "contacts[]"
                }),
                position: NodePosition { x: 100.0, y: 200.0 },
            },
            WorkflowNode {
                id: "deduplicate_companies".to_string(),
                name: "Identify Companies".to_string(),
                node_type: "llm_analyze".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "From the extracted contacts below, identify unique companies.\n",
                        "For each company, provide:\n",
                        "- name: Canonical company name (normalize variations)\n",
                        "- context: How many contacts from this company, their roles\n",
                        "- relationship: potential_client (default for prospect lists)\n\n",
                        "Also flag any duplicate contacts (same person listed twice with variations).\n\n",
                        "Output as JSON with \"companies\" and \"duplicates\" arrays.\n\n",
                        "Contacts:\n{{previous_results}}"
                    ),
                    "output_schema": "companies[]"
                }),
                position: NodePosition { x: 500.0, y: 200.0 },
            },
            WorkflowNode {
                id: "create_crm_contacts".to_string(),
                name: "Create CRM Contacts".to_string(),
                node_type: "output_crm_contacts".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 900.0, y: 200.0 },
            },
        ],
        connections: vec![
            WorkflowConnection {
                source: "parse_prospects".to_string(),
                target: "deduplicate_companies".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
            WorkflowConnection {
                source: "deduplicate_companies".to_string(),
                target: "create_crm_contacts".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
        ],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

/// Company Research: Extract company profile and competitive intel from articles or web content
fn company_research_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "company_research".to_string(),
        name: "Company Research Brief".to_string(),
        description: Some(
            "Extract a structured company profile from an article, press release, or web content."
                .to_string(),
        ),
        nodes: vec![
            WorkflowNode {
                id: "extract_company_profile".to_string(),
                name: "Extract Company Profile".to_string(),
                node_type: "llm_extract".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Analyze this content and extract a comprehensive company profile.\n",
                        "Provide:\n",
                        "- name: Company name\n",
                        "- website: Company website if mentioned\n",
                        "- industry: Primary industry/sector\n",
                        "- description: 2-3 sentence company description\n",
                        "- founded: Year founded if mentioned\n",
                        "- headquarters: Location if mentioned\n",
                        "- size: Employee count or size category if mentioned\n",
                        "- revenue: Revenue or funding if mentioned\n",
                        "- key_people: Array of {name, title} for leadership mentioned\n",
                        "- products_services: Array of main offerings\n",
                        "- target_market: Who they serve\n\n",
                        "Output as JSON with a top-level \"company\" object.\n\n",
                        "Content:\n{{content}}"
                    ),
                    "output_schema": "company"
                }),
                position: NodePosition { x: 100.0, y: 200.0 },
            },
            WorkflowNode {
                id: "competitive_analysis".to_string(),
                name: "Competitive & Opportunity Analysis".to_string(),
                node_type: "llm_analyze".to_string(),
                parameters: json!({
                    "prompt_template": concat!(
                        "Based on this company profile, provide a competitive and opportunity analysis ",
                        "for a creative agency (Power Club Global) considering this company as a client.\n\n",
                        "Provide:\n",
                        "- opportunities: Array of potential service offerings (branding, content, digital, events)\n",
                        "- pain_points: Likely challenges this company faces that PCG can solve\n",
                        "- competitors: Known competitors in their space\n",
                        "- partnership_angle: How PCG could position a pitch\n",
                        "- recommended_contacts: Which key people to reach out to and why\n",
                        "- deal_estimate: Estimated deal size range and type (project vs retainer)\n\n",
                        "Output as JSON.\n\n",
                        "Company Profile:\n{{previous_results}}"
                    ),
                    "output_schema": "analysis"
                }),
                position: NodePosition { x: 500.0, y: 200.0 },
            },
            WorkflowNode {
                id: "create_research_tasks".to_string(),
                name: "Create Research Tasks".to_string(),
                node_type: "output_tasks".to_string(),
                parameters: json!({}),
                position: NodePosition { x: 900.0, y: 200.0 },
            },
        ],
        connections: vec![
            WorkflowConnection {
                source: "extract_company_profile".to_string(),
                target: "competitive_analysis".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
            WorkflowConnection {
                source: "competitive_analysis".to_string(),
                target: "create_research_tasks".to_string(),
                source_output: Some(0),
                target_input: Some(0),
            },
        ],
        is_system: true,
        owner_type: "system".to_string(),
        owner_id: None,
        default_model: None,
    }
}

// ── DB helpers ───────────────────────────────────────────────────────────────

/// Serialize workflow to DB JSON column
fn serialize_workflow_data(wf: &WorkflowDefinition) -> String {
    let data = serde_json::json!({
        "nodes": wf.nodes,
        "connections": wf.connections,
        "default_model": wf.default_model,
    });
    data.to_string()
}

pub(crate) async fn seed_defaults(pool: &sqlx::SqlitePool) {
    let workflows = vec![
        default_analysis_workflow(),
        bug_triage_workflow(),
        sprint_planning_workflow(),
        client_onboarding_workflow(),
        content_pipeline_workflow(),
        sales_conversation_workflow(),
        prospect_list_workflow(),
        company_research_workflow(),
    ];

    for wf in &workflows {
        let data = serialize_workflow_data(wf);
        let desc = wf.description.clone().unwrap_or_default();
        let _ = sqlx::query(
            r#"INSERT OR REPLACE INTO workflow_definitions (id, owner_type, name, description, steps, is_system)
               VALUES (?1, 'system', ?2, ?3, ?4, 1)"#,
        )
        .bind(&wf.id)
        .bind(&wf.name)
        .bind(&desc)
        .bind(&data)
        .execute(pool)
        .await;
    }
}

fn parse_workflow_from_row(
    id: String,
    owner_type: String,
    owner_id: Option<String>,
    name: String,
    description: Option<String>,
    steps_json: String,
    is_system: bool,
) -> WorkflowDefinition {
    // Try new format: {"nodes": [...], "connections": [...], "default_model": "..."}
    if let Ok(v) = serde_json::from_str::<Value>(&steps_json) {
        if v.get("nodes").is_some() {
            let nodes: Vec<WorkflowNode> =
                serde_json::from_value(v["nodes"].clone()).unwrap_or_default();
            let connections: Vec<WorkflowConnection> =
                serde_json::from_value(v["connections"].clone()).unwrap_or_default();
            let default_model = v
                .get("default_model")
                .and_then(|dm| dm.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            return WorkflowDefinition {
                id,
                name,
                description,
                nodes,
                connections,
                is_system,
                owner_type,
                owner_id,
                default_model,
            };
        }
    }

    // Legacy format: array of steps — convert to nodes+connections
    let steps: Vec<LegacyStep> = serde_json::from_str(&steps_json).unwrap_or_default();
    let mut nodes = Vec::new();
    let mut connections = Vec::new();

    for (i, step) in steps.iter().enumerate() {
        nodes.push(WorkflowNode {
            id: step.id.clone(),
            name: step.name.clone(),
            node_type: "llm_extract".to_string(),
            parameters: json!({ "prompt_template": step.prompt_template }),
            position: NodePosition {
                x: if step.depends_on.is_empty() {
                    100.0
                } else {
                    100.0 + 400.0
                },
                y: 100.0 + (i as f64) * 200.0,
            },
        });
        for dep in &step.depends_on {
            connections.push(WorkflowConnection {
                source: dep.clone(),
                target: step.id.clone(),
                source_output: Some(0),
                target_input: Some(0),
            });
        }
    }

    WorkflowDefinition {
        id,
        name,
        description,
        nodes,
        connections,
        is_system,
        owner_type,
        owner_id,
        default_model: None,
    }
}

pub(crate) async fn load_all_workflows(
    pool: &sqlx::SqlitePool,
) -> Result<Vec<WorkflowDefinition>, sqlx::Error> {
    seed_defaults(pool).await;

    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, String, bool)>(
        "SELECT id, owner_type, owner_id, name, description, steps, is_system FROM workflow_definitions ORDER BY is_system DESC, name ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, ot, oid, name, desc, steps, sys)| {
            parse_workflow_from_row(id, ot, oid, name, desc, steps, sys)
        })
        .collect())
}

pub(crate) async fn load_workflow(
    pool: &sqlx::SqlitePool,
    workflow_id: &str,
) -> Result<Option<WorkflowDefinition>, sqlx::Error> {
    seed_defaults(pool).await;

    let row = sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, String, bool)>(
        "SELECT id, owner_type, owner_id, name, description, steps, is_system FROM workflow_definitions WHERE id = ?1",
    )
    .bind(workflow_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, ot, oid, name, desc, steps, sys)| {
        parse_workflow_from_row(id, ot, oid, name, desc, steps, sys)
    }))
}

// Text extraction, mock LLM, record helpers, dedup, action node execution,
// and LLM node execution have been extracted to
// services::services::workflow_execution and re-exported above.

// ── Route handlers ──────────────────────────────────────────────────────────

/// GET /api/data-sources/:id/workflows
async fn list_workflows_for_data_source(
    Path(data_source_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<WorkflowDefinition>>>, ApiError> {
    let pool = &deployment.db().pool;
    DataSource::find_by_id(pool, &data_source_id.to_string())
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;
    let workflows = load_all_workflows(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load workflows: {e}")))?;
    Ok(Json(ApiResponse::success(workflows)))
}

/// Optional body for run_workflow to specify model
#[derive(Debug, Deserialize)]
struct RunWorkflowRequest {
    model: Option<String>,
    force: Option<bool>,
}

/// Extract individual records from LLM output JSON
/// POST /api/data-sources/:id/workflows/:workflow_id/run
async fn run_workflow(
    Path((data_source_id, workflow_id)): Path<(Uuid, String)>,
    State(deployment): State<DeploymentImpl>,
    body: Option<Json<RunWorkflowRequest>>,
) -> Result<Json<ApiResponse<WorkflowRunResult>>, ApiError> {
    let (request_model, force) = match body {
        Some(Json(req)) => (req.model, req.force.unwrap_or(false)),
        None => (None, false),
    };
    let pool = &deployment.db().pool;
    let workflow_run_id = Uuid::new_v4();
    let run_start = std::time::Instant::now();

    let data_source = DataSource::find_by_id(pool, &data_source_id.to_string())
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    let workflow = load_workflow(pool, &workflow_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load workflow: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow '{}' not found", workflow_id)))?;

    let model = request_model
        .or(workflow.default_model.clone())
        .unwrap_or_default();

    let content = data_source.content.clone().unwrap_or_default();

    // Compute content hash for idempotency check (includes workflow structure so
    // definition changes invalidate the cache even if the source content is unchanged)
    let content_hash = {
        use std::{
            collections::hash_map::DefaultHasher,
            hash::{Hash, Hasher},
        };
        let mut hasher = DefaultHasher::new();
        workflow_id.hash(&mut hasher);
        serde_json::to_string(&workflow.nodes)
            .unwrap_or_default()
            .hash(&mut hasher);
        serde_json::to_string(&workflow.connections)
            .unwrap_or_default()
            .hash(&mut hasher);
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };

    // Check for existing completed run with same content hash (unless force=true)
    if !force {
        if let Ok(Some(existing_run)) = WorkflowRun::find_by_content_hash(pool, &content_hash).await
        {
            // Return the existing run's results with reconstructed usage stats
            let existing_run_id = existing_run.id.clone();
            let staged = existing_run.total_records_staged.unwrap_or(0);
            let run_uuid = Uuid::parse_str(&existing_run_id).unwrap_or(workflow_run_id);

            // Reconstruct total_usage from the stored WorkflowRun fields
            let total_usage = {
                let input_tokens = existing_run.total_input_tokens.unwrap_or(0);
                let output_tokens = existing_run.total_output_tokens.unwrap_or(0);
                let cost_micros = existing_run.total_estimated_cost_micros.unwrap_or(0);
                if input_tokens > 0 || output_tokens > 0 || cost_micros > 0 {
                    Some(json!({
                        "total_input_tokens": input_tokens,
                        "total_output_tokens": output_tokens,
                        "total_estimated_cost_micros": cost_micros,
                    }))
                } else {
                    None
                }
            };

            return Ok(Json(ApiResponse::success(WorkflowRunResult {
                workflow_run_id: run_uuid,
                workflow_id: existing_run.workflow_id,
                workflow_name: existing_run.workflow_name,
                data_source_id,
                steps: vec![],
                total_usage,
                staged_records: staged,
                reused: Some(true),
            })));
        }
    }

    // Create workflow run record
    let ds_org_uuid = data_source
        .organization_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());
    let ds_proj_uuid = data_source
        .project_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok());
    if let Err(e) = WorkflowRun::create(
        pool,
        CreateWorkflowRun {
            id: workflow_run_id,
            workflow_id: workflow.id.clone(),
            workflow_name: workflow.name.clone(),
            data_source_id: Some(data_source_id),
            organization_id: ds_org_uuid,
            project_id: ds_proj_uuid,
            model_used: if model.is_empty() {
                None
            } else {
                Some(model.clone())
            },
            content_hash: Some(content_hash),
        },
    )
    .await
    {
        tracing::error!("[WORKFLOW] Failed to create workflow run record: {e}");
    }

    // Execute workflow via shared engine
    let opts = super::workflow_engine::ExecutionOptions {
        model: model.clone(),
        data_source_id: Some(data_source_id),
        organization_id: ds_org_uuid,
        project_id: ds_proj_uuid,
        workflow_run_id: Some(workflow_run_id),
        create_artifacts: true,
        create_staging: true,
        artifact_metadata_extra: None,
        auto_approve: false,
    };

    let result =
        super::workflow_engine::execute_workflow_nodes(pool, &workflow, &content, &opts).await;

    // Check for LLM errors
    let llm_errors = super::workflow_engine::check_for_llm_errors(&result.node_results);
    if !llm_errors.is_empty() {
        let duration_ms = run_start.elapsed().as_millis() as i64;
        super::workflow_engine::finalize_workflow_run(
            pool,
            workflow_run_id,
            &workflow,
            &result,
            duration_ms,
            "failed",
        )
        .await;
        return Err(ApiError::InternalError(format!(
            "Workflow failed — LLM calls returned errors. {}. Check that API keys are configured as environment variables (e.g. ANTHROPIC_API_KEY).",
            llm_errors.first().unwrap_or(&String::new())
        )));
    }

    // Finalize the run
    let duration_ms = run_start.elapsed().as_millis() as i64;
    super::workflow_engine::finalize_workflow_run(
        pool,
        workflow_run_id,
        &workflow,
        &result,
        duration_ms,
        "completed",
    )
    .await;

    // Build step results from node results (artifact_id is not tracked by engine — use Uuid::nil as placeholder)
    let step_results: Vec<StepResult> = result
        .node_results
        .iter()
        .map(|nr| StepResult {
            step_id: nr.node_id.clone(),
            step_name: nr.node_name.clone(),
            artifact_id: Uuid::nil(),
        })
        .collect();

    Ok(Json(ApiResponse::success(WorkflowRunResult {
        workflow_run_id,
        workflow_id: workflow.id,
        workflow_name: workflow.name,
        data_source_id,
        steps: step_results,
        total_usage: result.total_usage,
        staged_records: result.staged_records,
        reused: None,
    })))
}

/// GET /api/data-sources/:id/artifacts
async fn list_data_source_artifacts(
    Path(data_source_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ExecutionArtifact>>>, ApiError> {
    let pool = &deployment.db().pool;
    DataSource::find_by_id(pool, &data_source_id.to_string())
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    let ds_id_str = data_source_id.to_string();
    let artifacts = sqlx::query_as::<_, ExecutionArtifact>(
        r#"SELECT * FROM execution_artifacts WHERE json_extract(metadata, '$.data_source_id') = ?1 ORDER BY created_at ASC"#,
    ).bind(&ds_id_str).fetch_all(pool).await
    .map_err(|e| ApiError::InternalError(format!("Failed to query artifacts: {e}")))?;

    Ok(Json(ApiResponse::success(artifacts)))
}

/// GET /api/workflows/definitions
async fn list_all_workflow_definitions(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<WorkflowDefinition>>>, ApiError> {
    let pool = &deployment.db().pool;
    let workflows = load_all_workflows(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to load workflows: {e}")))?;
    Ok(Json(ApiResponse::success(workflows)))
}

/// POST /api/workflows/definitions
async fn create_workflow_definition(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<CreateWorkflowRequest>,
) -> Result<Json<ApiResponse<WorkflowDefinition>>, ApiError> {
    let pool = &deployment.db().pool;

    if req.id.is_empty() || req.name.is_empty() {
        return Err(ApiError::BadRequest("id and name are required".to_string()));
    }

    let existing = load_workflow(pool, &req.id)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?;
    if existing.is_some() {
        return Err(ApiError::BadRequest(format!(
            "Workflow '{}' already exists",
            req.id
        )));
    }

    let owner_type = req.owner_type.unwrap_or_else(|| "organization".to_string());
    if !["organization", "user"].contains(&owner_type.as_str()) {
        return Err(ApiError::BadRequest(
            "owner_type must be 'organization' or 'user'".to_string(),
        ));
    }

    let wf = WorkflowDefinition {
        id: req.id,
        name: req.name,
        description: req.description,
        nodes: req.nodes,
        connections: req.connections,
        is_system: false,
        owner_type: owner_type.clone(),
        owner_id: req.owner_id.clone(),
        default_model: req.default_model,
    };
    let data = serialize_workflow_data(&wf);

    sqlx::query(
        r#"INSERT INTO workflow_definitions (id, owner_type, owner_id, name, description, steps, is_system) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)"#,
    )
    .bind(&wf.id).bind(&owner_type).bind(&req.owner_id).bind(&wf.name).bind(&wf.description).bind(&data)
    .execute(pool).await
    .map_err(|e| ApiError::InternalError(format!("Failed to create workflow: {e}")))?;

    Ok(Json(ApiResponse::success(wf)))
}

/// PUT /api/workflows/definitions/:id
async fn update_workflow_definition(
    Path(workflow_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<UpdateWorkflowRequest>,
) -> Result<Json<ApiResponse<WorkflowDefinition>>, ApiError> {
    let pool = &deployment.db().pool;

    let existing = load_workflow(pool, &workflow_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow '{}' not found", workflow_id)))?;

    let name = req.name.unwrap_or(existing.name);
    let description = req.description.or(existing.description);
    let nodes = req.nodes.unwrap_or(existing.nodes);
    let connections = req.connections.unwrap_or(existing.connections);
    let default_model = req.default_model.or(existing.default_model);

    let wf = WorkflowDefinition {
        id: workflow_id.clone(),
        name: name.clone(),
        description: description.clone(),
        nodes: nodes.clone(),
        connections: connections.clone(),
        is_system: existing.is_system,
        owner_type: existing.owner_type.clone(),
        owner_id: existing.owner_id.clone(),
        default_model,
    };
    let data = serialize_workflow_data(&wf);

    sqlx::query(
        r#"UPDATE workflow_definitions SET name = ?1, description = ?2, steps = ?3, updated_at = datetime('now', 'subsec') WHERE id = ?4"#,
    )
    .bind(&name).bind(&description).bind(&data).bind(&workflow_id)
    .execute(pool).await
    .map_err(|e| ApiError::InternalError(format!("Failed to update workflow: {e}")))?;

    Ok(Json(ApiResponse::success(wf)))
}

/// DELETE /api/workflows/definitions/:id
async fn delete_workflow_definition(
    Path(workflow_id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<()>>, ApiError> {
    let pool = &deployment.db().pool;

    let existing = load_workflow(pool, &workflow_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow '{}' not found", workflow_id)))?;

    if existing.is_system {
        return Err(ApiError::BadRequest(
            "Cannot delete system workflows".to_string(),
        ));
    }

    sqlx::query("DELETE FROM workflow_definitions WHERE id = ?1")
        .bind(&workflow_id)
        .execute(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete workflow: {e}")))?;

    Ok(Json(ApiResponse::success(())))
}

/// GET /api/artifacts/recent?organization_id=...
///
/// When `organization_id` is provided the query is scoped to artifacts whose
/// originating data source belongs to that organization.  Without it the
/// endpoint falls back to global (for backwards-compat / admin use).
#[derive(Deserialize)]
struct ArtifactRecentParams {
    organization_id: Option<String>,
}

async fn list_recent_artifacts(
    State(deployment): State<DeploymentImpl>,
    Query(params): Query<ArtifactRecentParams>,
) -> Result<Json<ApiResponse<Vec<ExecutionArtifact>>>, ApiError> {
    let pool = &deployment.db().pool;

    let artifacts = if let Some(ref org_id) = params.organization_id {
        // Scope artifacts to those created from data sources owned by this org.
        // data_sources.id is a BLOB UUID so we convert to hyphenated text for
        // comparison with json_extract(metadata, '$.data_source_id') which is TEXT.
        sqlx::query_as::<_, ExecutionArtifact>(
            r#"SELECT ea.*
               FROM execution_artifacts ea
               JOIN data_sources ds
                 ON LOWER(
                      SUBSTR(hex(ds.id), 1, 8) || '-' ||
                      SUBSTR(hex(ds.id), 9, 4) || '-' ||
                      SUBSTR(hex(ds.id), 13, 4) || '-' ||
                      SUBSTR(hex(ds.id), 17, 4) || '-' ||
                      SUBSTR(hex(ds.id), 21, 12)
                    ) = json_extract(ea.metadata, '$.data_source_id')
               WHERE json_extract(ea.metadata, '$.workflow_id') IS NOT NULL
                 AND LOWER(
                      SUBSTR(hex(ds.organization_id), 1, 8) || '-' ||
                      SUBSTR(hex(ds.organization_id), 9, 4) || '-' ||
                      SUBSTR(hex(ds.organization_id), 13, 4) || '-' ||
                      SUBSTR(hex(ds.organization_id), 17, 4) || '-' ||
                      SUBSTR(hex(ds.organization_id), 21, 12)
                    ) = ?1
               ORDER BY ea.created_at DESC
               LIMIT 100"#,
        ).bind(org_id).fetch_all(pool).await
    } else {
        sqlx::query_as::<_, ExecutionArtifact>(
            r#"SELECT * FROM execution_artifacts WHERE json_extract(metadata, '$.workflow_id') IS NOT NULL ORDER BY created_at DESC LIMIT 100"#,
        ).fetch_all(pool).await
    }.map_err(|e| ApiError::InternalError(format!("Failed to query artifacts: {e}")))?;

    Ok(Json(ApiResponse::success(artifacts)))
}

// ── Preview (dry-run) endpoint ───────────────────────────────────────────────

/// Response for preview node result
#[derive(Debug, Serialize)]
struct PreviewNodeResult {
    node_id: String,
    node_name: String,
    node_type: String,
    output: String,
    usage: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    records: Option<Vec<PreviewRecordInfo>>,
}

#[derive(Debug, Serialize)]
struct PreviewRecordInfo {
    data: Value,
    validation_errors: Vec<String>,
    confidence: f64,
}

/// POST /api/workflows/preview — dry-run a workflow without saving artifacts
async fn preview_workflow(
    State(deployment): State<DeploymentImpl>,
    Json(req): Json<PreviewWorkflowRequest>,
) -> Result<Json<ApiResponse<Vec<PreviewNodeResult>>>, ApiError> {
    let pool = &deployment.db().pool;
    let content = req.content.unwrap_or_else(|| {
        "Acme Corp CEO John Smith met with TechStart Inc CTO Jane Doe to discuss a potential partnership. \
         Also present were VP Michael Chen from GlobalTech and Dr. Sarah Park from InnovateLabs. \
         The meeting covered AI integration services valued at approximately $50,000.".to_string()
    });

    // Build an ad-hoc workflow definition from the request
    let preview_workflow = WorkflowDefinition {
        id: String::new(),
        name: "Preview".to_string(),
        description: None,
        nodes: req.nodes,
        connections: req.connections,
        is_system: false,
        owner_type: String::new(),
        owner_id: None,
        default_model: None,
    };

    // Execute via shared engine — no artifacts, no staging
    let opts = super::workflow_engine::ExecutionOptions {
        model: String::new(),
        create_artifacts: false,
        create_staging: false,
        ..Default::default()
    };

    let result =
        super::workflow_engine::execute_workflow_nodes(pool, &preview_workflow, &content, &opts)
            .await;

    // Convert node results to preview format with validation info
    let results: Vec<PreviewNodeResult> = result
        .node_results
        .iter()
        .map(|nr| {
            let records = if nr.node_type.starts_with("output_") {
                let target_type = nr.node_type.strip_prefix("output_").unwrap_or("");
                let staging_target = match target_type {
                    "crm_contacts" => Some("crm_contact"),
                    "crm_companies" => Some("company"),
                    "crm_deals" => Some("crm_deal"),
                    "tasks" => Some("task"),
                    _ => None,
                };
                staging_target.and_then(|st| {
                    serde_json::from_str::<Value>(&nr.output)
                        .ok()
                        .map(|parsed| {
                            extract_records_from_output(&parsed, st)
                                .iter()
                                .map(|record| {
                                    let validation_errors =
                                        match validate_record_against_schema(record, st) {
                                            Ok(()) => vec![],
                                            Err(errs) => errs,
                                        };
                                    let confidence =
                                        compute_confidence(record, st, &validation_errors, false);
                                    PreviewRecordInfo {
                                        data: record.clone(),
                                        validation_errors,
                                        confidence,
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                })
            } else {
                None
            };

            PreviewNodeResult {
                node_id: nr.node_id.clone(),
                node_name: nr.node_name.clone(),
                node_type: nr.node_type.clone(),
                output: nr.output.clone(),
                usage: nr.usage.clone(),
                records,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(results)))
}

// ── Available models endpoint (backed by PCG Router registry) ────────────────

#[derive(Debug, Serialize)]
struct AvailableModel {
    id: String,
    label: String,
    is_default: bool,
    provider: String,
    cost_per_million_input: i64,
    cost_per_million_output: i64,
}

async fn list_available_models(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<AvailableModel>>>, ApiError> {
    let pool = &deployment.db().pool;
    let models = PcgRouterModel::list_enabled(pool)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to list models: {e}")))?;

    let available: Vec<AvailableModel> = models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            AvailableModel {
                id: m.model_id.clone(),
                label: format!("{} ({})", m.name, m.provider),
                is_default: i == 0, // highest priority (first) is default
                provider: m.provider.clone(),
                cost_per_million_input: m.cost_per_million_input,
                cost_per_million_output: m.cost_per_million_output,
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(available)))
}

// ── Workflow Run Metrics Endpoints ────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RecentRunsQuery {
    workflow_id: Option<String>,
    organization_id: Option<String>,
    limit: Option<i64>,
}

/// GET /api/workflows/runs/recent
async fn list_recent_runs(
    State(deployment): State<DeploymentImpl>,
    axum::extract::Query(params): axum::extract::Query<RecentRunsQuery>,
) -> Result<Json<ApiResponse<Vec<WorkflowRun>>>, ApiError> {
    let pool = &deployment.db().pool;
    let limit = params.limit.unwrap_or(50);
    let runs = WorkflowRun::find_recent(
        pool,
        limit,
        params.workflow_id.as_deref(),
        params.organization_id.as_deref(),
    )
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to list runs: {e}")))?;
    Ok(Json(ApiResponse::success(runs)))
}

/// GET /api/workflows/runs/:id
async fn get_run_by_id(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<WorkflowRun>>, ApiError> {
    let pool = &deployment.db().pool;
    let run = WorkflowRun::find_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to find run: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Workflow run not found".to_string()))?;
    Ok(Json(ApiResponse::success(run)))
}

/// GET /api/workflows/runs/:id/stats
async fn get_run_stats(
    Path(id): Path<String>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Value>>, ApiError> {
    let pool = &deployment.db().pool;
    let run = WorkflowRun::find_by_id(pool, &id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to find run: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Workflow run not found".to_string()))?;

    // Get current staging record counts for live stats
    let run_uuid =
        Uuid::parse_str(&id).map_err(|e| ApiError::InternalError(format!("Invalid UUID: {e}")))?;
    let staging_records = WorkflowStagingRecord::find_by_run(pool, run_uuid)
        .await
        .unwrap_or_default();

    let total = staging_records.len() as f64;
    let approved = staging_records
        .iter()
        .filter(|r| r.status == "approved" || r.status == "committed")
        .count() as f64;
    let rejected = staging_records
        .iter()
        .filter(|r| r.status == "rejected")
        .count() as f64;
    let committed = staging_records
        .iter()
        .filter(|r| r.status == "committed")
        .count() as f64;
    let duplicates = staging_records
        .iter()
        .filter(|r| r.duplicate_of_id.is_some())
        .count() as f64;

    let approval_rate = if total > 0.0 { approved / total } else { 0.0 };
    let duplicate_rate = if total > 0.0 { duplicates / total } else { 0.0 };

    let cost_dollars = run.total_estimated_cost_micros.unwrap_or(0) as f64 / 1_000_000.0;

    Ok(Json(ApiResponse::success(json!({
        "run": run,
        "live_counts": {
            "total": total as i64,
            "approved": approved as i64,
            "rejected": rejected as i64,
            "committed": committed as i64,
            "duplicates": duplicates as i64,
            "pending": staging_records.iter().filter(|r| r.status == "pending_review").count(),
        },
        "rates": {
            "approval_rate": approval_rate,
            "duplicate_rate": duplicate_rate,
        },
        "cost_dollars": cost_dollars,
    }))))
}

// ── Auto-trigger helper ─────────────────────────────────────────────────────

/// Check for matching workflow triggers and run them in the background.
/// Called after a new data source is created. Non-blocking — spawns tokio tasks.
pub async fn fire_triggers_for_data_source(
    pool: sqlx::SqlitePool,
    data_source_id: String,
    deployment: crate::DeploymentImpl,
) {
    use db::models::workflow_trigger::WorkflowTrigger;

    let ds = match DataSource::find_by_id(&pool, &data_source_id).await {
        Ok(Some(ds)) => ds,
        _ => return,
    };

    let org_id = ds.organization_id.clone();
    let proj_id = ds.project_id.clone();

    let triggers = match WorkflowTrigger::find_matching_triggers(
        &pool,
        &ds.data_type,
        org_id.as_deref(),
        proj_id.as_deref(),
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(
                "[TRIGGER] Failed to find matching triggers for ds {}: {}",
                data_source_id,
                e
            );
            return;
        }
    };

    if triggers.is_empty() {
        return;
    }

    tracing::info!(
        "[TRIGGER] Found {} matching trigger(s) for data source {} (type={})",
        triggers.len(),
        data_source_id,
        ds.data_type
    );

    for trigger in triggers {
        // Atomically claim the trigger (prevents race conditions with concurrent webhooks)
        let claimed = WorkflowTrigger::try_claim_trigger(&pool, trigger.id.as_str()).await;
        match claimed {
            Ok(None) => {
                tracing::info!(
                    "[TRIGGER] Skipping trigger '{}' — still in cooldown ({}s)",
                    trigger.id,
                    trigger.cooldown_seconds
                );
                continue;
            }
            Err(e) => {
                tracing::error!("[TRIGGER] Failed to claim trigger '{}': {}", trigger.id, e);
                continue;
            }
            Ok(Some(_)) => {} // Claimed successfully, proceed
        }

        let pool = pool.clone();
        let deployment = deployment.clone();
        let ds_id = data_source_id.clone();
        let trigger_id = trigger.id.clone();
        let workflow_id = trigger.workflow_id.clone();
        let model_override = trigger.model_override.clone();
        let trigger_auto_approve = trigger.auto_approve;

        tokio::spawn(async move {
            let _trigger_start = std::time::Instant::now();

            // Create audit trail entry
            use db::models::trigger_execution::{CreateTriggerExecution, TriggerExecution};
            let execution = TriggerExecution::create(
                &pool,
                CreateTriggerExecution {
                    trigger_id: trigger_id.clone(),
                    source_type: Some("data_source".to_string()),
                    source_id: Some(ds_id.clone()),
                    metadata: None,
                },
            )
            .await
            .ok();
            let span = tracing::info_span!("trigger_execution",
                trigger_id = %trigger_id,
                workflow_id = %workflow_id,
                data_source_id = %ds_id,
            );
            let _enter = span.enter();

            tracing::info!(
                "[TRIGGER] Firing trigger '{}' (workflow={}) for data source {}",
                trigger_id,
                workflow_id,
                ds_id
            );

            // Note: trigger count already incremented by try_claim_trigger()

            // Load the workflow definition
            let workflow = match load_workflow(&pool, &workflow_id).await {
                Ok(Some(wf)) => wf,
                Ok(None) => {
                    tracing::error!(
                        "[TRIGGER] Workflow '{}' not found for trigger '{}'",
                        workflow_id,
                        trigger_id
                    );
                    return;
                }
                Err(e) => {
                    tracing::error!("[TRIGGER] Failed to load workflow '{}': {}", workflow_id, e);
                    return;
                }
            };

            // Determine model
            let model = model_override
                .or(workflow.default_model.clone())
                .unwrap_or_default();

            // Load data source
            let data_source = match DataSource::find_by_id(&pool, &ds_id).await {
                Ok(Some(ds)) => ds,
                _ => {
                    tracing::error!("[TRIGGER] Data source {} not found", ds_id);
                    return;
                }
            };

            let workflow_run_id = uuid::Uuid::new_v4();
            let run_start = std::time::Instant::now();

            // Create workflow run record
            if let Err(e) = WorkflowRun::create(
                &pool,
                CreateWorkflowRun {
                    id: workflow_run_id,
                    workflow_id: workflow.id.clone(),
                    workflow_name: workflow.name.clone(),
                    data_source_id: Uuid::parse_str(&ds_id).ok(),
                    organization_id: data_source
                        .organization_id
                        .as_deref()
                        .and_then(|s| Uuid::parse_str(s).ok()),
                    project_id: data_source
                        .project_id
                        .as_deref()
                        .and_then(|s| Uuid::parse_str(s).ok()),
                    model_used: if model.is_empty() {
                        None
                    } else {
                        Some(model.clone())
                    },
                    content_hash: None,
                },
            )
            .await
            {
                tracing::error!("[TRIGGER] Failed to create workflow run record: {e}");
                return;
            }

            let ctx_project_id = data_source
                .project_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok());
            let ctx_org_id = data_source
                .organization_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok());
            let content = data_source.content.unwrap_or_default();

            // Execute via shared engine
            let opts = super::workflow_engine::ExecutionOptions {
                model: model.clone(),
                data_source_id: Uuid::parse_str(&ds_id).ok(),
                organization_id: ctx_org_id,
                project_id: ctx_project_id,
                workflow_run_id: Some(workflow_run_id),
                create_artifacts: true,
                create_staging: true,
                artifact_metadata_extra: Some(serde_json::json!({"trigger_id": trigger_id})),
                auto_approve: trigger_auto_approve,
            };

            // Mark execution as running
            if let Some(ref exec) = execution {
                let _ = TriggerExecution::mark_running(
                    &pool,
                    &exec.id,
                    Some(&workflow_run_id.to_string()),
                )
                .await;
            }

            let result =
                super::workflow_engine::execute_workflow_nodes(&pool, &workflow, &content, &opts)
                    .await;

            let duration_ms = run_start.elapsed().as_millis() as i64;
            super::workflow_engine::finalize_workflow_run(
                &pool,
                workflow_run_id,
                &workflow,
                &result,
                duration_ms,
                "completed",
            )
            .await;

            // Record execution result
            if let Some(ref exec) = execution {
                let _ = TriggerExecution::mark_completed(
                    &pool,
                    &exec.id,
                    result.staged_records,
                    duration_ms,
                )
                .await;
            }
            let _ = WorkflowTrigger::clear_error(&pool, &trigger_id).await;

            tracing::info!(
                "[TRIGGER] Completed trigger '{}' workflow run {} ({} records staged, {}ms)",
                trigger_id,
                workflow_run_id,
                result.staged_records,
                duration_ms
            );

            // Auto-approve + batch-commit when trigger has auto_approve enabled
            if trigger_auto_approve && result.staged_records > 0 {
                let label = format!("trigger '{trigger_id}'");
                super::workflow_engine::auto_approve_staged_records(
                    &pool,
                    workflow_run_id,
                    result.staged_records,
                    &deployment,
                    &label,
                )
                .await;
            }
        });
    }
}

// ── Router ──────────────────────────────────────────────────────────────────

// ── Sprint 2C: Workflow schedule background loop ─────────────────────────────

/// Spawn a background task that checks for due schedule triggers every 5 minutes.
/// Schedule triggers run workflows without a data source — they use an empty content string.
pub fn spawn_workflow_schedule_loop(
    pool: sqlx::SqlitePool,
    shutdown: tokio_util::sync::CancellationToken,
) {
    tokio::spawn(async move {
        use std::time::Duration;

        use db::models::workflow_trigger::WorkflowTrigger;
        use tokio::time::interval;

        // Check every 5 minutes
        let mut ticker = interval(Duration::from_secs(300));
        ticker.tick().await; // discard immediate first tick

        loop {
            tokio::select! {
                _ = ticker.tick() => {}
                _ = shutdown.cancelled() => {
                    tracing::info!("[SCHEDULE] Workflow schedule loop shutting down");
                    break;
                }
            }

            let due_triggers = match WorkflowTrigger::find_due_schedules(&pool).await {
                Ok(triggers) => triggers,
                Err(e) => {
                    tracing::error!("[SCHEDULE] Failed to check schedule triggers: {e}");
                    continue;
                }
            };

            if due_triggers.is_empty() {
                continue;
            }

            tracing::info!(
                "[SCHEDULE] Found {} due schedule trigger(s)",
                due_triggers.len()
            );

            for trigger in due_triggers {
                let pool = pool.clone();
                let trigger_id = trigger.id.clone();
                let workflow_id = trigger.workflow_id.clone();
                let model_override = trigger.model_override.clone();

                tokio::spawn(async move {
                    let span = tracing::info_span!("schedule_trigger",
                        trigger_id = %trigger_id,
                        workflow_id = %workflow_id,
                    );
                    let _enter = span.enter();
                    let start = std::time::Instant::now();

                    // Atomically claim trigger (prevents double-fire if schedule overlaps)
                    match WorkflowTrigger::try_claim_trigger(&pool, &trigger_id).await {
                        Ok(None) => {
                            tracing::debug!(
                                "[SCHEDULE] Trigger {} still in cooldown, skipping",
                                trigger_id
                            );
                            return;
                        }
                        Err(e) => {
                            tracing::warn!("[SCHEDULE] Failed to claim trigger: {e}");
                            return;
                        }
                        Ok(Some(_)) => {} // Claimed successfully, continue
                    }

                    // Load workflow
                    let workflow = match load_workflow(&pool, &workflow_id).await {
                        Ok(Some(wf)) => wf,
                        Ok(None) => {
                            tracing::error!("[SCHEDULE] Workflow '{}' not found", workflow_id);
                            return;
                        }
                        Err(e) => {
                            tracing::error!(
                                "[SCHEDULE] Failed to load workflow '{}': {e}",
                                workflow_id
                            );
                            return;
                        }
                    };

                    let model = model_override
                        .or(workflow.default_model.clone())
                        .unwrap_or_default();

                    // Schedule triggers run without source data — they rely on action nodes
                    // (like assign_to_agent, http_request, send_notification) rather than data extraction
                    let opts = super::workflow_engine::ExecutionOptions {
                        model: model.clone(),
                        create_artifacts: false,
                        create_staging: false,
                        ..Default::default()
                    };

                    let result =
                        super::workflow_engine::execute_workflow_nodes(&pool, &workflow, "", &opts)
                            .await;
                    let duration_ms = start.elapsed().as_millis();
                    tracing::info!(
                        "[SCHEDULE] Completed workflow '{}' (trigger '{}') in {}ms, {} records staged",
                        workflow_id,
                        trigger_id,
                        duration_ms,
                        result.staged_records
                    );
                });
            }
        }
    });
}

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/data-sources/{id}/workflows",
            get(list_workflows_for_data_source),
        )
        .route(
            "/data-sources/{id}/workflows/{workflow_id}/run",
            post(run_workflow),
        )
        .route(
            "/data-sources/{id}/artifacts",
            get(list_data_source_artifacts),
        )
        .route(
            "/workflows/definitions",
            get(list_all_workflow_definitions).post(create_workflow_definition),
        )
        .route(
            "/workflows/definitions/{id}",
            put(update_workflow_definition).delete(delete_workflow_definition),
        )
        .route("/workflows/preview", post(preview_workflow))
        .route("/workflows/models", get(list_available_models))
        .route("/workflows/runs/recent", get(list_recent_runs))
        .route("/workflows/runs/{id}", get(get_run_by_id))
        .route("/workflows/runs/{id}/stats", get(get_run_stats))
        .route("/artifacts/recent", get(list_recent_artifacts))
}
