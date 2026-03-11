use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post, put},
};
use db::models::data_source::DataSource;
use db::models::execution_artifact::{ArtifactType, CreateExecutionArtifact, ExecutionArtifact};
use db::models::workflow_run::{WorkflowRun, CreateWorkflowRun, UpdateWorkflowRunOnComplete};
use db::models::workflow_staging::{WorkflowStagingRecord, CreateStagingRecord};
use regex::Regex;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utils::response::ApiResponse;
use uuid::Uuid;

use deployment::Deployment;
use db::models::pcg_router_model::PcgRouterModel;
use crate::{DeploymentImpl, error::ApiError};
use super::pcg_router::{self, ChatMessage};

// ── Workflow types (n8n-inspired schema) ─────────────────────────────────────

/// Position on the canvas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// A single node in the workflow (n8n-style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,          // "llm_extract", "llm_analyze", "llm_summarize", "transform", "filter", "merge"
    pub parameters: Value,          // type-specific config (prompt_template, output_schema, etc.)
    pub position: NodePosition,     // canvas coordinates
}

/// A connection between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConnection {
    pub source: String,             // source node id
    pub target: String,             // target node id
    pub source_output: Option<i32>, // output index (default 0)
    pub target_input: Option<i32>,  // input index (default 0)
}

/// Full workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<WorkflowNode>,
    pub connections: Vec<WorkflowConnection>,
    pub is_system: bool,
    #[serde(default = "default_owner_type")]
    pub owner_type: String,     // "system", "organization", "user"
    pub owner_id: Option<String>,
    pub default_model: Option<String>,
}

fn default_owner_type() -> String { "system".to_string() }

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
    owner_type: Option<String>,  // "organization" or "user"
    owner_id: Option<String>,    // UUID of the owner
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
        description: Some("Extract companies, contacts, and opportunities from data source content.".to_string()),
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

async fn seed_defaults(pool: &sqlx::SqlitePool) {
    let default = default_analysis_workflow();
    let data = serialize_workflow_data(&default);
    let desc = default.description.unwrap_or_default();

    let _ = sqlx::query(
        r#"INSERT OR IGNORE INTO workflow_definitions (id, owner_type, name, description, steps, is_system)
           VALUES (?1, 'system', ?2, ?3, ?4, 1)"#,
    )
    .bind(&default.id)
    .bind(&default.name)
    .bind(&desc)
    .bind(&data)
    .execute(pool)
    .await;
}

fn parse_workflow_from_row(id: String, owner_type: String, owner_id: Option<String>, name: String, description: Option<String>, steps_json: String, is_system: bool) -> WorkflowDefinition {
    // Try new format: {"nodes": [...], "connections": [...], "default_model": "..."}
    if let Ok(v) = serde_json::from_str::<Value>(&steps_json) {
        if v.get("nodes").is_some() {
            let nodes: Vec<WorkflowNode> = serde_json::from_value(v["nodes"].clone()).unwrap_or_default();
            let connections: Vec<WorkflowConnection> = serde_json::from_value(v["connections"].clone()).unwrap_or_default();
            let default_model = v.get("default_model").and_then(|dm| dm.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string());
            return WorkflowDefinition { id, name, description, nodes, connections, is_system, owner_type, owner_id, default_model };
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
                x: if step.depends_on.is_empty() { 100.0 } else { 100.0 + 400.0 },
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

    WorkflowDefinition { id, name, description, nodes, connections, is_system, owner_type, owner_id, default_model: None }
}

async fn load_all_workflows(pool: &sqlx::SqlitePool) -> Result<Vec<WorkflowDefinition>, sqlx::Error> {
    seed_defaults(pool).await;

    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, String, bool)>(
        "SELECT id, owner_type, owner_id, name, description, steps, is_system FROM workflow_definitions ORDER BY is_system DESC, name ASC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|(id, ot, oid, name, desc, steps, sys)| parse_workflow_from_row(id, ot, oid, name, desc, steps, sys)).collect())
}

async fn load_workflow(pool: &sqlx::SqlitePool, workflow_id: &str) -> Result<Option<WorkflowDefinition>, sqlx::Error> {
    seed_defaults(pool).await;

    let row = sqlx::query_as::<_, (String, String, Option<String>, String, Option<String>, String, bool)>(
        "SELECT id, owner_type, owner_id, name, description, steps, is_system FROM workflow_definitions WHERE id = ?1",
    )
    .bind(workflow_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(id, ot, oid, name, desc, steps, sys)| parse_workflow_from_row(id, ot, oid, name, desc, steps, sys)))
}

// ── Mock LLM: content-aware extraction ──────────────────────────────────────

/// Extract all email addresses from text using regex
fn extract_emails_from_text(text: &str) -> Vec<String> {
    let re = Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap();
    let mut emails: Vec<String> = Vec::new();
    for cap in re.find_iter(text) {
        let email = cap.as_str().to_string();
        if !emails.contains(&email) {
            emails.push(email);
        }
    }
    emails
}

/// Extract dollar amounts and nearby context from text.
/// Returns (amount_str, nearby_context_line) pairs.
fn extract_dollar_amounts_from_text(text: &str) -> Vec<(String, String)> {
    let re = Regex::new(r"\$[\d,]+(?:\.\d+)?(?:\s*[KkMmBb])?").unwrap();
    let mut results: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        for cap in re.find_iter(line) {
            let amount = cap.as_str().to_string();
            if !results.iter().any(|(a, _)| a == &amount) {
                results.push((amount, line.trim().to_string()));
            }
        }
    }
    results
}

/// Try to find a date near some context text. Looks for YYYY-MM-DD patterns.
fn extract_date_near_text(text: &str, context_line: &str) -> Option<String> {
    // First try the specific context line
    let date_re = Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();
    if let Some(m) = date_re.find(context_line) {
        return Some(m.as_str().to_string());
    }
    // Also try nearby lines (within 3 lines of the context)
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(context_line.split_whitespace().next().unwrap_or("")) {
            // Check lines i-3..i+3
            let start = i.saturating_sub(3);
            let end = (i + 4).min(lines.len());
            for nearby in &lines[start..end] {
                if let Some(m) = date_re.find(nearby) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    None
}

fn extract_company_names_from_text(text: &str) -> Vec<String> {
    let mut companies = Vec::new();
    // Look for organization suffixes: Group, Inc, Corp, LLC, Ltd, Co, Foundation, etc.
    let org_suffixes = ["Group", "Inc", "Corp", "Corporation", "LLC", "Ltd", "Co", "Company",
                        "Foundation", "Institute", "Associates", "Partners", "Solutions",
                        "Technologies", "Systems", "Services", "Global", "International",
                        "Health", "Medical", "Consulting", "Labs", "Studio", "Agency"];
    // Words that cannot be part of a company name (stop walk-back)
    let stop_words = ["Attendees", "CEO", "CFO", "CTO", "COO", "VP", "Director", "Manager",
                      "Head", "Lead", "Senior", "Junior", "Chief", "President", "Chair",
                      "Date", "Time", "Location", "Agenda", "Notes", "Summary", "Action",
                      "Items", "Discussion", "Meeting", "Call", "Review", "Update", "Status",
                      "Follow", "Next", "Steps", "Budget", "Revenue", "Cost", "Total"];
    let words: Vec<&str> = text.split_whitespace().collect();
    for i in 0..words.len() {
        let w = words[i].trim_matches(|c: char| !c.is_alphanumeric());
        if org_suffixes.contains(&w) {
            // Walk backwards to collect the full company name (max 4 words back)
            let mut parts: Vec<&str> = vec![w];
            let mut j = i;
            let max_walk = 4;
            while j > 0 && parts.len() <= max_walk {
                j -= 1;
                let prev = words[j].trim_matches(|c: char| !c.is_alphanumeric());
                if prev.is_empty() { break; }
                if stop_words.contains(&prev) { break; }
                let starts_upper = prev.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
                if starts_upper && prev.len() >= 2 {
                    parts.insert(0, prev);
                } else {
                    break;
                }
            }
            if parts.len() >= 2 {
                let name = parts.join(" ");
                if !companies.contains(&name) && companies.len() < 5 {
                    companies.push(name);
                }
            }
        }
    }
    // Also look for "COMPANY:" lines
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("COMPANY:") {
            let company_part = rest.trim().split('|').next().unwrap_or("").trim();
            if !company_part.is_empty() && !companies.contains(&company_part.to_string()) && companies.len() < 10 {
                companies.push(company_part.to_string());
            }
        }
    }

    // Look for "CEO of CompanyName", "VP at CompanyName", "works at CompanyName", etc.
    // Only match "at CompanyName" (not "of") to avoid picking up job title fragments like "of Business Development"
    let at_re = Regex::new(r"\bat\s+([A-Z][A-Za-z0-9]+(?:\s+[A-Z][A-Za-z0-9]+){0,4})").unwrap();
    for cap in at_re.captures_iter(text) {
        if let Some(m) = cap.get(1) {
            let candidate = m.as_str().trim().to_string();
            // Filter out common false positives (people names are handled by contact extraction)
            let false_positives = ["The", "This", "That", "These", "Those", "Our", "Your",
                                   "His", "Her", "Its", "My", "January", "February", "March",
                                   "April", "May", "June", "July", "August", "September",
                                   "October", "November", "December", "Monday", "Tuesday",
                                   "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
                                   // Job title/department words that aren't company names
                                   "Business", "Product", "Engineering", "Marketing", "Sales",
                                   "Operations", "Human", "Finance", "Legal", "Research"];
            let first_word = candidate.split_whitespace().next().unwrap_or("");
            let word_count = candidate.split_whitespace().count();
            // Require at least 2 words for "at X" companies (single words too ambiguous)
            if word_count >= 2 && !false_positives.contains(&first_word) && candidate.len() >= 3
                && !companies.contains(&candidate) && companies.len() < 10
            {
                companies.push(candidate);
            }
        }
    }

    // Look for "CompanyName: Series B" or "CompanyName - description" patterns
    // (lines starting with a capitalized name followed by colon or dash)
    let label_re = Regex::new(r"^([A-Z][A-Za-z0-9]+(?:\s+[A-Z][A-Za-z0-9]+){0,4})\s*(?::\s+\S|–\s+\S|-\s+\S)").unwrap();
    for line in text.lines() {
        let trimmed = line.trim().trim_start_matches('-').trim().trim_start_matches('*').trim();
        if let Some(cap) = label_re.captures(trimmed) {
            if let Some(m) = cap.get(1) {
                let candidate = m.as_str().trim().to_string();
                // Must not be a generic label
                let generic_labels = ["Date", "Time", "Location", "Agenda", "Notes", "Summary",
                                      "Action", "Items", "Discussion", "Meeting", "Budget",
                                      "Revenue", "Status", "Update", "Follow", "Next",
                                      "Estimated", "Expected", "Total", "Contact", "Phone",
                                      "Email", "Description", "Details", "Subject", "Title",
                                      "Priority", "Attendees", "Participants"];
                let first_word = candidate.split_whitespace().next().unwrap_or("");
                if !generic_labels.contains(&first_word) && candidate.len() >= 3
                    && !companies.contains(&candidate) && companies.len() < 10
                {
                    companies.push(candidate);
                }
            }
        }
    }

    companies
}

/// Structured contact info extracted from text
struct ExtractedContact {
    name: String,
    role: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    company: Option<String>,
}

fn extract_contacts_from_text(text: &str) -> Vec<ExtractedContact> {
    let mut contacts = Vec::new();
    let email_re_simple = |s: &str| -> Option<String> {
        // Find email-like pattern in a string
        for word in s.split_whitespace() {
            let w = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-');
            if w.contains('@') && w.contains('.') && w.len() > 5 {
                return Some(w.to_string());
            }
        }
        None
    };
    let phone_re_simple = |s: &str| -> Option<String> {
        // Find phone-like pattern: (xxx) xxx-xxxx or xxx-xxx-xxxx
        for segment in s.split_whitespace().collect::<Vec<_>>().windows(3) {
            let combined = segment.join(" ");
            let digits: String = combined.chars().filter(|c| c.is_ascii_digit()).collect();
            if digits.len() >= 10 && digits.len() <= 11 && combined.contains(|c: char| c == '(' || c == '-') {
                return Some(combined);
            }
        }
        None
    };

    // Parse "- Name, Title | email | phone | linkedin" lines (contact info format)
    for line in text.lines() {
        let trimmed = line.trim().trim_start_matches('-').trim();
        // Look for lines with pipe separators that contain email addresses
        if trimmed.contains('|') && trimmed.contains('@') {
            let parts: Vec<&str> = trimmed.split('|').map(|s| s.trim()).collect();
            if let Some(name_role) = parts.first() {
                let (name, role) = if let Some(comma_pos) = name_role.find(',') {
                    (name_role[..comma_pos].trim().to_string(), Some(name_role[comma_pos+1..].trim().to_string()))
                } else {
                    (name_role.trim().to_string(), None)
                };
                // Clean "Dr. " prefix but keep it recognizable
                let clean_name = name.replace("Dr. ", "").trim().to_string();
                let display_name = if name.starts_with("Dr.") { name.clone() } else { clean_name.clone() };
                if display_name.len() >= 3 && display_name.contains(' ') {
                    contacts.push(ExtractedContact {
                        name: display_name,
                        role,
                        email: parts.get(1).and_then(|s| email_re_simple(s)),
                        phone: parts.get(2).and_then(|s| phone_re_simple(s)),
                        company: None,
                    });
                }
            }
        }
    }

    // Fallback: look for "Title (Role, Company)" patterns in attendee lines
    if contacts.is_empty() {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.to_lowercase().contains("attendee") || trimmed.contains("(CEO") || trimmed.contains("(CFO") || trimmed.contains("(CTO") {
                // Parse "Name (Role, Company)" patterns
                let mut rest = trimmed;
                if let Some(pos) = trimmed.find(':') {
                    rest = &trimmed[pos+1..];
                }
                for segment in rest.split(',') {
                    let seg = segment.trim();
                    if let Some(paren_pos) = seg.find('(') {
                        let name = seg[..paren_pos].trim();
                        let role_info = seg[paren_pos..].trim_matches(|c| c == '(' || c == ')');
                        if name.len() >= 3 && name.contains(' ') && contacts.len() < 10 {
                            let (role, company) = if let Some(comma) = role_info.find(',') {
                                (Some(role_info[..comma].trim().to_string()), Some(role_info[comma+1..].trim().to_string()))
                            } else {
                                (Some(role_info.trim().to_string()), None)
                            };
                            contacts.push(ExtractedContact { name: name.to_string(), role, email: None, phone: None, company });
                        }
                    }
                }
            }
        }
    }

    // Fallback 2: look for "Name, Role of/at Company (email)" patterns
    // e.g. "Sarah Kim, CEO of NovaBridge Analytics (sarah.kim@novabridge.ai)"
    if contacts.is_empty() {
        let name_role_email_re = Regex::new(
            r"(?m)^[\s\-\*]*([A-Z][a-z]+(?:\s+[A-Z][a-z]+)+),\s*((?:CEO|CTO|CFO|COO|VP|Director|Manager|Head|Lead|President|Founder|Partner|Principal|Senior|Chief|SVP|EVP|CMO|CIO|CISO|CRO)\b[^()\n]*?)\s*\(([a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,})\)"
        ).unwrap();
        for cap in name_role_email_re.captures_iter(text) {
            let name = cap[1].trim().to_string();
            let role_str = cap[2].trim().to_string();
            let email = cap[3].trim().to_string();
            // Parse "VP of Business Development at CompanyName" or "CEO of CompanyName"
            // Prefer splitting at " at " (last occurrence) since it typically separates role from company
            let (role, company) = {
                let role_lower = role_str.to_lowercase();
                if let Some(pos) = role_lower.rfind(" at ") {
                    let r = role_str[..pos].trim().to_string();
                    let c = role_str[pos+4..].trim().to_string();
                    (Some(r), if c.is_empty() { None } else { Some(c) })
                } else if let Some(pos) = role_lower.find(" of ") {
                    let r = role_str[..pos].trim().to_string();
                    let c = role_str[pos+4..].trim().to_string();
                    (Some(r), if c.is_empty() { None } else { Some(c) })
                } else {
                    (Some(role_str), None)
                }
            };
            if contacts.len() < 10 {
                contacts.push(ExtractedContact { name, role, email: Some(email), phone: None, company });
            }
        }
    }

    // Fallback 3: find all emails and try to associate names with them
    if contacts.is_empty() {
        let emails = extract_emails_from_text(text);
        for email in &emails {
            if contacts.len() >= 10 { break; }
            let mut found_name: Option<String> = None;
            let mut found_role: Option<String> = None;
            let mut found_company: Option<String> = None;

            for line in text.lines() {
                if !line.contains(email.as_str()) { continue; }
                let trimmed = line.trim();

                // Pattern: "Name (email)" or "Name <email>"
                let before_email = if let Some(pos) = trimmed.find(email.as_str()) {
                    trimmed[..pos].trim().trim_end_matches(|c: char| c == '(' || c == '<' || c == ',' || c == ' ')
                } else { "" };

                if !before_email.is_empty() {
                    // Walk backwards through the before_email to extract the name
                    let clean = before_email.trim_start_matches(|c: char| c == '-' || c == '*' || c == ' ');
                    // Check if there's a comma-separated role+company before name
                    if let Some(comma_pos) = clean.rfind(',') {
                        let name_part = clean[..comma_pos].trim();
                        // The name_part might itself contain "Name, Role of Company"
                        if name_part.contains(' ') && name_part.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                            found_name = Some(name_part.to_string());
                        }
                        let role_part = clean[comma_pos+1..].trim();
                        if !role_part.is_empty() {
                            // Parse "VP of Business Development at CompanyName" or "CEO of CompanyName"
                            // Prefer " at " (last occurrence) over " of " since it typically separates role from company
                            let role_lower = role_part.to_lowercase();
                            if let Some(pos) = role_lower.rfind(" at ") {
                                found_role = Some(role_part[..pos].trim().to_string());
                                let c = role_part[pos+4..].trim().to_string();
                                if !c.is_empty() { found_company = Some(c); }
                            } else if let Some(pos) = role_lower.find(" of ") {
                                found_role = Some(role_part[..pos].trim().to_string());
                                let c = role_part[pos+4..].trim().to_string();
                                if !c.is_empty() { found_company = Some(c); }
                            } else {
                                found_role = Some(role_part.to_string());
                            }
                        }
                    } else if clean.contains(' ') && clean.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                        found_name = Some(clean.to_string());
                    }
                }
            }

            // If we couldn't find a name, try to derive one from the email
            let name = found_name.unwrap_or_else(|| {
                // e.g. sarah.kim@novabridge.ai -> Sarah Kim
                let local = email.split('@').next().unwrap_or("");
                let parts: Vec<String> = local.split(|c: char| c == '.' || c == '_' || c == '-')
                    .filter(|p| !p.is_empty() && p.len() > 1)
                    .map(|p| {
                        let mut chars = p.chars();
                        match chars.next() {
                            Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                            None => String::new(),
                        }
                    })
                    .collect();
                if parts.len() >= 2 { parts.join(" ") } else { local.to_string() }
            });

            if name.len() >= 3 && !contacts.iter().any(|c| c.email.as_deref() == Some(email.as_str())) {
                contacts.push(ExtractedContact {
                    name,
                    role: found_role,
                    email: Some(email.clone()),
                    phone: None,
                    company: found_company,
                });
            }
        }
    }

    contacts
}

fn generate_mock_step_result(step_id: &str, content: &str, title: &str, previous_results: &[(&str, &str)], node_type: &str, output_schema: &str) -> String {
    // Match on exact step_id first (system workflows), then use output_schema to determine mock data
    let key = match step_id {
        "extract_companies" | "extract_contacts" | "identify_opportunities" | "identify_deals" => step_id.to_string(),

        _ => {
            // For custom workflows, use output_schema to pick the right mock
            let schema_lower = output_schema.to_lowercase();
            let has_companies = schema_lower.contains("compan");
            let has_contacts = schema_lower.contains("contact") || schema_lower.contains("person") || schema_lower.contains("people");
            let has_deals = schema_lower.contains("deal") || schema_lower.contains("opportunit");
            let multi_type_count = [has_companies, has_contacts, has_deals].iter().filter(|&&b| b).count();

            if multi_type_count >= 2 {
                // Multi-schema: generate combined output for all requested types
                "multi_extract".to_string()
            } else if has_companies { "extract_companies".to_string() }
            else if has_contacts { "extract_contacts".to_string() }
            else if has_deals { "identify_deals".to_string() }
            else if node_type == "llm_analyze" { "identify_opportunities".to_string() }
            else { step_id.to_string() }
        }
    };
    match key.as_str() {
        "multi_extract" => {
            // Combined extraction: produce contacts, companies, and deals in one JSON object
            let schema_lower = output_schema.to_lowercase();
            let mut result = serde_json::Map::new();
            // Extract contacts first so we can filter person names from companies
            let contacts_extracted = extract_contacts_from_text(content);
            let contact_names: Vec<String> = contacts_extracted.iter().map(|c| c.name.to_lowercase()).collect();

            if schema_lower.contains("compan") {
                let extracted = extract_company_names_from_text(content);
                // Filter out entries that match known contact names (person != company)
                let companies: Vec<Value> = extracted.iter()
                    .filter(|name| !contact_names.contains(&name.to_lowercase()))
                    .enumerate().map(|(i, name)| {
                    let rel = match i % 3 { 0 => "potential_client", 1 => "partner", _ => "vendor" };
                    json!({
                        "name": name,
                        "description": "Extracted from source content",
                        "relationship": rel,
                        "context": format!("Company '{}' mentioned in document", name),
                    })
                }).collect();
                result.insert("companies".to_string(), json!(companies));
            }

            if schema_lower.contains("contact") || schema_lower.contains("person") || schema_lower.contains("people") {
                let extracted = extract_contacts_from_text(content);
                let contacts: Vec<Value> = extracted.iter().map(|c| {
                    let (first_name, last_name) = {
                        let parts: Vec<&str> = c.name.split_whitespace().collect();
                        if parts.len() >= 2 {
                            (parts[0].to_string(), parts[1..].join(" "))
                        } else {
                            (c.name.clone(), String::new())
                        }
                    };
                    json!({
                        "first_name": first_name,
                        "last_name": last_name,
                        "name": c.name,
                        "job_title": c.role,
                        "email": c.email,
                        "phone": c.phone,
                        "company_name": c.company,
                    })
                }).collect();
                result.insert("contacts".to_string(), json!(contacts));
            }

            if schema_lower.contains("deal") || schema_lower.contains("opportunit") {
                let amounts = extract_dollar_amounts_from_text(content);
                let contacts_extracted = extract_contacts_from_text(content);

                let deals: Vec<Value> = if amounts.is_empty() && !contacts_extracted.is_empty() {
                    let c = &contacts_extracted[0];
                    let company = c.company.clone().unwrap_or_default();
                    vec![json!({
                        "name": format!("Opportunity with {}", if company.is_empty() { c.name.clone() } else { company }),
                        "description": format!("Potential opportunity identified with {}", c.name),
                        "amount": null,
                        "currency": "USD",
                        "deal_type": "project",
                    })]
                } else {
                    amounts.iter().enumerate().map(|(i, (amount, context_line))| {
                        let amount_num: Option<f64> = {
                            let cleaned: String = amount.chars()
                                .filter(|c| c.is_ascii_digit() || *c == '.')
                                .collect();
                            let multiplier = if amount.contains('K') || amount.contains('k') { 1_000.0 }
                                else if amount.contains('M') || amount.contains('m') { 1_000_000.0 }
                                else { 1.0 };
                            cleaned.parse::<f64>().ok().map(|v| v * multiplier)
                        };
                        // Associate each deal with the nearest contact by matching context line
                        let associated_contact = contacts_extracted.iter().find(|c| {
                            let ctx_lower = context_line.to_lowercase();
                            if let Some(ref name) = Some(&c.name) {
                                ctx_lower.contains(&name.to_lowercase())
                            } else {
                                false
                            }
                        }).or(contacts_extracted.get(i));
                        let company = associated_contact.and_then(|c| c.company.clone()).unwrap_or_default();
                        let deal_name = if !company.is_empty() {
                            format!("Deal with {}", company)
                        } else if let Some(c) = associated_contact {
                            format!("Deal with {}", c.name)
                        } else {
                            format!("Deal #{}", i + 1)
                        };
                        let contact_name = associated_contact.map(|c| c.name.clone());
                        let contact_email = associated_contact.and_then(|c| c.email.clone());
                        // Truncate description to a clean summary
                        let desc = if context_line.len() > 200 {
                            format!("{}...", &context_line[..200])
                        } else {
                            context_line.clone()
                        };
                        json!({
                            "name": deal_name,
                            "description": desc,
                            "amount": amount_num,
                            "currency": "USD",
                            "contact_name": contact_name,
                            "contact_email": contact_email,
                            "deal_type": "project",
                        })
                    }).collect()
                };
                result.insert("deals".to_string(), json!(deals));
            }

            Value::Object(result).to_string()
        }
        "extract_companies" => {
            let extracted = extract_company_names_from_text(content);
            if extracted.is_empty() {
                // Return empty array rather than a placeholder when nothing found
                json!({"companies": []}).to_string()
            } else {
                let companies: Vec<Value> = extracted.iter().enumerate().map(|(i, name)| {
                    let rel = match i % 3 { 0 => "potential_client", 1 => "partner", _ => "vendor" };
                    json!({
                        "name": name,
                        "description": format!("Extracted from source content"),
                        "relationship": rel,
                        "context": format!("Company '{}' mentioned in document", name),
                    })

                }).collect();
                json!({ "companies": companies }).to_string()
            }
        }
        "extract_contacts" => {
            let extracted = extract_contacts_from_text(content);

            let company_names: Vec<String> = previous_results.iter()
                .filter(|(sid, _)| *sid == "extract_companies")
                .filter_map(|(_, result)| serde_json::from_str::<Value>(result).ok().and_then(|v| v["companies"].as_array().cloned()))
                .flatten().filter_map(|c| c["name"].as_str().map(|s| s.to_string())).collect();
            if extracted.is_empty() {
                // Return empty array rather than placeholder contacts
                json!({"contacts": []}).to_string()
            } else {
                let contacts: Vec<Value> = extracted.iter().enumerate().map(|(i, c)| {
                    let company = c.company.clone()
                        .or_else(|| company_names.get(i % company_names.len().max(1)).cloned());
                    // Split name into first/last for CRM contact schema
                    let (first_name, last_name) = {
                        let parts: Vec<&str> = c.name.split_whitespace().collect();
                        if parts.len() >= 2 {
                            (parts[0].to_string(), parts[1..].join(" "))
                        } else {
                            (c.name.clone(), String::new())
                        }
                    };
                    json!({
                        "first_name": first_name,
                        "last_name": last_name,
                        "name": c.name,
                        "job_title": c.role,
                        "email": c.email,
                        "phone": c.phone,
                        "company_name": company,
                    })

                }).collect();
                json!({ "contacts": contacts }).to_string()
            }
        }
        "identify_opportunities" | "identify_deals" => {
            // Gather context from previous extraction steps
            let mut primary_company = String::new();
            let mut primary_contact_name = String::new();
            let mut primary_contact_email = String::new();

            for (sid, result) in previous_results {
                if *sid == "extract_companies" {
                    if let Ok(v) = serde_json::from_str::<Value>(result) {
                        if let Some(companies) = v["companies"].as_array() {
                            if let Some(first) = companies.first() {
                                if let Some(name) = first["name"].as_str() {
                                    primary_company = name.to_string();
                                }
                            }
                        }
                    }
                }
                if *sid == "extract_contacts" {
                    if let Ok(v) = serde_json::from_str::<Value>(result) {
                        if let Some(contacts) = v["contacts"].as_array() {
                            if let Some(first) = contacts.first() {
                                if let Some(name) = first["name"].as_str().or(first["first_name"].as_str()) {
                                    primary_contact_name = if let Some(last) = first["last_name"].as_str() {
                                        format!("{} {}", name, last)
                                    } else {
                                        name.to_string()
                                    };
                                }
                                if let Some(email) = first["email"].as_str() {
                                    primary_contact_email = email.to_string();

                                }
                            }
                        }
                    }
                }
            }

            // Extract dollar amounts from content
            let amounts = extract_dollar_amounts_from_text(content);

            if amounts.is_empty() && primary_company.is_empty() {
                // Nothing to extract — return empty
                json!({"opportunities": []}).to_string()
            } else if !amounts.is_empty() {
                // Build real opportunities from extracted dollar amounts
                let opportunities: Vec<Value> = amounts.iter().enumerate().map(|(i, (amount, context_line))| {
                    // Try to find a deal name from nearby context
                    let deal_name = {
                        // Look for "Value:" or "Estimated Value:" label — the deal is likely described nearby
                        let lower_ctx = context_line.to_lowercase();
                        if lower_ctx.contains("value") || lower_ctx.contains("budget") || lower_ctx.contains("amount") {
                            // The deal name is probably from the surrounding context
                            // Look a few lines above for a company or project name
                            let lines: Vec<&str> = content.lines().collect();
                            let mut found_name = String::new();
                            for (li, line) in lines.iter().enumerate() {
                                if line.contains(context_line.split_whitespace().next().unwrap_or("")) {
                                    // Walk backwards up to 5 lines to find a descriptive name
                                    let start = li.saturating_sub(5);
                                    for check_line in &lines[start..li] {
                                        let trimmed = check_line.trim().trim_start_matches(|c: char| c == '-' || c == '*' || c == '#');
                                        let trimmed = trimmed.trim();
                                        // Look for a line that seems like a deal/project header
                                        if !trimmed.is_empty() && trimmed.len() > 5 && trimmed.len() < 100
                                            && trimmed.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                                            && !trimmed.to_lowercase().starts_with("estimated")
                                            && !trimmed.to_lowercase().starts_with("expected")
                                            && !trimmed.to_lowercase().starts_with("budget")
                                        {
                                            found_name = trimmed.to_string();
                                        }
                                    }
                                    break;
                                }
                            }
                            if found_name.is_empty() && !primary_company.is_empty() {
                                format!("Opportunity with {}", primary_company)
                            } else if !found_name.is_empty() {
                                // Truncate if too long
                                if found_name.len() > 80 { found_name.truncate(80); }
                                found_name
                            } else {
                                format!("Deal #{}", i + 1)
                            }
                        } else {
                            // Use the context line itself (truncated)
                            let mut name = context_line.clone();
                            if name.len() > 80 { name.truncate(80); }
                            name
                        }
                    };

                    // Parse dollar amount to a number
                    let amount_num: Option<f64> = {
                        let cleaned: String = amount.chars()
                            .filter(|c| c.is_ascii_digit() || *c == '.')
                            .collect();
                        let multiplier = if amount.contains('K') || amount.contains('k') { 1_000.0 }
                            else if amount.contains('M') || amount.contains('m') { 1_000_000.0 }
                            else if amount.contains('B') || amount.contains('b') { 1_000_000_000.0 }
                            else { 1.0 };
                        cleaned.parse::<f64>().ok().map(|v| v * multiplier)
                    };

                    // Try to find an expected close date near this amount
                    let close_date = extract_date_near_text(content, context_line);

                    let contact_name = if !primary_contact_name.is_empty() { Some(primary_contact_name.clone()) } else { None };
                    let contact_email = if !primary_contact_email.is_empty() { Some(primary_contact_email.clone()) } else { None };

                    json!({
                        "name": deal_name,
                        "description": context_line,
                        "amount": amount_num,
                        "currency": "USD",
                        "contact_name": contact_name,
                        "contact_email": contact_email,
                        "deal_type": "project",
                        "estimated_value": amount_num.map(|a| format!("${}", if a >= 1_000_000.0 { format!("{}M", (a / 1_000_000.0 * 10.0).round() / 10.0) } else if a >= 1_000.0 { format!("{}K", (a / 1_000.0).round()) } else { format!("{}", a.round()) })),
                        "expected_close_date": close_date,
                        "next_steps": ["Review extracted deal details", "Schedule follow-up meeting"]
                    })
                }).collect();
                json!({"opportunities": opportunities}).to_string()
            } else {
                // We have company info but no dollar amounts — create a general opportunity
                let opp_title = format!("Opportunity with {}", primary_company);
                let contact_name = if !primary_contact_name.is_empty() { Value::String(primary_contact_name) } else { Value::Null };
                let contact_email = if !primary_contact_email.is_empty() { Value::String(primary_contact_email) } else { Value::Null };
                json!({"opportunities": [
                    {
                        "name": opp_title,
                        "description": format!("Potential opportunity identified with {}", primary_company),
                        "amount": null,
                        "currency": "USD",
                        "contact_name": contact_name,
                        "contact_email": contact_email,
                        "deal_type": "project",
                        "next_steps": ["Review opportunity details", "Schedule discovery call"]
                    }
                ]}).to_string()
            }
        }
        _ => {
            json!({"result": format!("Analysis of {} chars of content", content.len()), "status": "completed"}).to_string()

        }
    }
}

// ── Route handlers ──────────────────────────────────────────────────────────

/// GET /api/data-sources/:id/workflows
async fn list_workflows_for_data_source(
    Path(data_source_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<WorkflowDefinition>>>, ApiError> {
    let pool = &deployment.db().pool;
    DataSource::find_by_id(pool, data_source_id)
        .await.map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;
    let workflows = load_all_workflows(pool).await
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
fn extract_records_from_output(data: &Value, target_type: &str) -> Vec<Value> {
    // Skip error responses from failed LLM calls
    if data.get("error").is_some() {
        tracing::warn!("[WORKFLOW] Skipping record extraction — node returned an error: {}", data);
        return vec![];
    }

    // Try direct array
    if let Some(arr) = data.as_array() {
        return arr.clone();
    }

    // Try common keys based on target type
    let keys = match target_type {
        "crm_contact" => vec!["contacts", "people", "persons"],
        "company" => vec!["companies", "organizations"],
        "crm_deal" => vec!["deals", "opportunities", "proposals"],
        "task" => vec!["tasks", "action_items", "actions"],
        _ => vec![],
    };

    for key in keys {
        if let Some(arr) = data.get(key).and_then(|v| v.as_array()) {
            return arr.clone();
        }
    }

    // If it's a single object with recognized entity fields, wrap it
    if data.is_object() && !data.as_object().unwrap().is_empty() {
        // Only wrap if it looks like an actual entity record (has name/title/email)
        let obj = data.as_object().unwrap();
        let looks_like_record = obj.contains_key("name") || obj.contains_key("title")
            || obj.contains_key("email") || obj.contains_key("first_name");
        if looks_like_record {
            return vec![data.clone()];
        }
    }

    vec![]
}

/// Check if a record looks like a fallback placeholder produced by the mock extraction
/// engine (i.e. when no LLM is connected). These records contain generic names and
/// no real data, so they should be flagged as very low confidence.
///
/// Records with real extracted data (valid emails, real names, real dollar amounts)
/// should NOT be flagged as placeholders even if they come from the fallback engine.
fn is_fallback_placeholder(record: &Value) -> bool {
    let obj = match record.as_object() {
        Some(o) => o,
        None => return false,
    };

    // If the record has a real email address, it's not a placeholder
    if let Some(email) = obj.get("email").and_then(|v| v.as_str()) {
        if email.contains('@') && email.contains('.') && email.len() > 5 {
            return false;
        }
    }

    // If the record has a real dollar amount (number), it's not a placeholder
    if let Some(amount) = obj.get("amount") {
        if amount.is_number() && amount.as_f64().unwrap_or(0.0) > 0.0 {
            return false;
        }
    }

    // If the record has both first_name and last_name that look real, not a placeholder
    if let (Some(first), Some(last)) = (
        obj.get("first_name").and_then(|v| v.as_str()),
        obj.get("last_name").and_then(|v| v.as_str()),
    ) {
        if !first.is_empty() && !last.is_empty()
            && first != "Unknown" && last != "Contact"
            && first.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
        {
            return false;
        }
    }

    // Collect all string values from the record for pattern matching
    let string_values: Vec<&str> = obj.values().filter_map(|v| v.as_str()).collect();

    let placeholder_patterns = [
        "Follow-up from '",
        "CRM Records from '",
        "Company from '",
        "Unknown Contact",
        "Processed step '",
        "Referenced in data source content",
        "Referenced in ",
        "Mentioned in data source content",
    ];

    for val in &string_values {
        for pattern in &placeholder_patterns {
            if val.contains(pattern) {
                return true;
            }
        }
    }

    // Also check nested arrays (e.g. next_steps) for generic content
    for v in obj.values() {
        if let Some(arr) = v.as_array() {
            for item in arr {
                if let Some(s) = item.as_str() {
                    for pattern in &placeholder_patterns {
                        if s.contains(pattern) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}


/// Compute a confidence score (0.0–1.0) for a staging record based on simple heuristics.
/// - Start at 1.0
/// - Subtract 0.15 for each missing required field
/// - Subtract 0.05 for each validation error
/// - Subtract 0.3 if marked as duplicate
/// - Subtract 0.1 if more than half of all fields are null/empty
/// - Cap at 0.1 if the record is a fallback placeholder (no LLM connected)
/// - Floor at 0.0
fn compute_confidence(record: &Value, target_type: &str, validation_errors: &[String], is_duplicate: bool) -> f64 {
    // If this record was produced by the mock/fallback engine, cap confidence very low
    if is_fallback_placeholder(record) {
        tracing::warn!(
            "[WORKFLOW] Detected fallback placeholder record (no LLM connected) — setting low confidence"
        );
        return 0.1;
    }


    let mut score: f64 = 1.0;

    // Check required fields against schema
    if let Some(schema) = super::output_schemas::get_schema_for_target(target_type) {
        let total_fields = schema.fields.len();
        let mut null_or_empty_count = 0;

        for (name, field) in &schema.fields {
            let value = record.get(name.as_str());
            let is_missing = match value {
                None => true,
                Some(Value::Null) => true,
                Some(Value::String(s)) => s.is_empty(),
                _ => false,
            };

            if is_missing {
                null_or_empty_count += 1;
                if field.required {
                    score -= 0.15;
                }
            }
        }

        // Penalize if more than half of all fields are null/empty
        if total_fields > 0 && null_or_empty_count > total_fields / 2 {
            score -= 0.1;
        }
    }

    // Penalize for each validation error
    score -= 0.05 * validation_errors.len() as f64;

    // Penalize if duplicate
    if is_duplicate {
        score -= 0.3;
    }

    // Floor at 0.0
    score.max(0.0)
}

/// Build a schema prompt text dynamically from output_schemas definitions.
/// Maps output node target types to their schema definitions and generates
/// a human-readable prompt describing the expected JSON format.
fn build_schema_prompt_text(target_type: &str) -> Option<String> {
    // Map output node types to schema target types
    let schema_target = match target_type {
        "crm_contacts" => "crm_contact",
        "crm_companies" | "companies" => "company",
        "crm_deals" | "deals" => "crm_deal",
        "tasks" => "task",
        _ => return None,
    };

    // Map to the expected JSON array key
    let array_key = match target_type {
        "crm_contacts" => "contacts",
        "crm_companies" | "companies" => "companies",
        "crm_deals" | "deals" => "deals",
        "tasks" => "tasks",
        _ => return None,
    };

    let schema = super::output_schemas::get_schema_for_target(schema_target)?;

    let mut parts = vec![format!("Output JSON must contain a \"{}\" array.", array_key)];

    let mut required_fields = Vec::new();
    let mut optional_fields = Vec::new();

    for (name, field) in &schema.fields {
        let mut desc = format!("{} ({}", name, field.field_type);
        if let Some(ref enums) = field.enum_values {
            desc.push_str(&format!(", one of: {}", enums.join(", ")));
        }
        desc.push(')');
        if field.required {
            required_fields.push(desc);
        } else {
            optional_fields.push(format!("{} or null", desc));
        }
    }

    if !required_fields.is_empty() {
        parts.push(format!("Required fields: {}.", required_fields.join(", ")));
    }
    if !optional_fields.is_empty() {
        parts.push(format!("Optional fields: {}.", optional_fields.join(", ")));
    }

    // Add anti-hallucination instruction based on entity type
    let entity_hint = match target_type {
        "crm_contacts" => "Each entry must be a REAL person explicitly named in the source content. Do NOT use document titles, section headings, or metadata as contact names.",
        "crm_companies" | "companies" => "Each entry must be a REAL company/organization explicitly named in the source. Do NOT use document titles, dates, or headings as company names.",
        "crm_deals" | "deals" => "Each entry must represent a REAL business opportunity described in the source.",
        "tasks" => "Each entry must be a REAL action item or follow-up explicitly described in the source.",
        _ => "Only extract entities explicitly mentioned in the source content.",
    };
    parts.push(format!("IMPORTANT: {} If none exist, return an empty array.", entity_hint));

    Some(parts.join(" "))
}

/// Validate a single record (serde_json::Value) against the TargetSchema for the given target_type.
/// Returns Ok(()) if valid, or Err(Vec<String>) with a list of human-readable validation errors.
fn validate_record_against_schema(record: &Value, target_type: &str) -> Result<(), Vec<String>> {
    let schema = match super::output_schemas::get_schema_for_target(target_type) {
        Some(s) => s,
        None => return Ok(()), // unknown target type — skip validation
    };

    let obj = match record.as_object() {
        Some(o) => o,
        None => return Err(vec!["Record is not a JSON object".to_string()]),
    };

    let mut errors = Vec::new();

    for (field_name, field_def) in &schema.fields {
        let value = obj.get(field_name);

        // Check required fields
        if field_def.required {
            match value {
                None | Some(Value::Null) => {
                    errors.push(format!("Missing required field: {}", field_name));
                    continue;
                }
                Some(Value::String(s)) if s.is_empty() => {
                    errors.push(format!("Required field '{}' is empty", field_name));
                    continue;
                }
                _ => {}
            }
        }

        // If the field is present and not null, check types
        if let Some(val) = value {
            if val.is_null() {
                continue; // null is OK for optional fields
            }

            let type_ok = match field_def.field_type.as_str() {
                "string" => val.is_string(),
                "number" => val.is_number() || val.is_f64() || val.is_i64() || val.is_u64(),
                "array" => val.is_array(),
                "object" => val.is_object(),
                "boolean" => val.is_boolean(),
                _ => true, // unknown type — don't validate
            };

            if !type_ok {
                errors.push(format!(
                    "Field '{}' expected type '{}', got {}",
                    field_name,
                    field_def.field_type,
                    value_type_name(val)
                ));
            }

            // Check enum constraints
            if let Some(ref enum_values) = field_def.enum_values {
                if let Some(s) = val.as_str() {
                    if !enum_values.iter().any(|e| e == s) {
                        errors.push(format!(
                            "Field '{}' value '{}' not in allowed values: [{}]",
                            field_name, s,
                            enum_values.join(", ")
                        ));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Helper to get a human-readable type name for a serde_json::Value
fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Compute Levenshtein distance between two strings
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();
    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost)
                .min(prev[j + 1] + 1)
                .min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_len]
}

/// Fuzzy name match: lowercases, trims whitespace, then checks Levenshtein distance.
/// Returns true if distance <= 2 for short names (<=6 chars) or >80% similarity.
fn fuzzy_name_match(a: &str, b: &str) -> bool {
    let a = a.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    let b = b.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
    if a == b { return true; }
    let dist = levenshtein_distance(&a, &b);
    let max_len = a.len().max(b.len());
    if max_len == 0 { return true; }
    if max_len <= 6 {
        dist <= 2
    } else {
        let similarity = 1.0 - (dist as f64 / max_len as f64);
        similarity > 0.8
    }
}

/// Normalize a company name by stripping common suffixes and trimming
fn normalize_company_name(name: &str) -> String {
    let suffixes = [
        " incorporated", " corporation", " company", " limited",
        " inc.", " inc", " llc.", " llc", " ltd.", " ltd",
        " corp.", " corp", " co.", " co", " l.l.c.", " l.l.c",
        " plc", " gmbh", " ag", " s.a.", " sa",
    ];
    let mut normalized = name.to_lowercase().trim().to_string();
    // Strip trailing punctuation like commas
    normalized = normalized.trim_end_matches(',').trim().to_string();
    for suffix in &suffixes {
        if normalized.ends_with(suffix) {
            let new_len = normalized.len() - suffix.len();
            normalized.truncate(new_len);
            normalized = normalized.trim().to_string();
            break; // Only strip one suffix
        }
    }
    // Collapse extra whitespace
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Check if a CRM contact already exists by email, phone, linkedin, or fuzzy name match.
/// Returns (existing_id, match_type) where match_type indicates what matched.
async fn check_contact_duplicate(
    pool: &sqlx::SqlitePool,
    record: &Value,
    project_id: Option<Uuid>,
) -> Option<(Uuid, String)> {
    let project_id = project_id?;

    // Try email exact match first (strongest signal)
    if let Some(email) = record["email"].as_str().filter(|s| !s.is_empty()) {
        let result = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM crm_contacts WHERE project_id = ?1 AND LOWER(email) = LOWER(?2) LIMIT 1"
        )
        .bind(project_id)
        .bind(email)
        .fetch_optional(pool)
        .await
        .ok()?;

        if let Some((id,)) = result {
            return Some((id, "email".to_string()));
        }
    }

    // Try phone/mobile match
    for field in &["phone", "mobile"] {
        if let Some(phone) = record[*field].as_str().filter(|s| !s.is_empty()) {
            let result = sqlx::query_as::<_, (Uuid,)>(
                "SELECT id FROM crm_contacts WHERE project_id = ?1 AND (phone = ?2 OR mobile = ?2) LIMIT 1"
            )
            .bind(project_id)
            .bind(phone.trim())
            .fetch_optional(pool)
            .await
            .ok()?;

            if let Some((id,)) = result {
                return Some((id, "phone".to_string()));
            }
        }
    }

    // Try LinkedIn URL exact match
    if let Some(linkedin) = record["linkedin_url"].as_str().filter(|s| !s.is_empty()) {
        let result = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM crm_contacts WHERE project_id = ?1 AND LOWER(linkedin_url) = LOWER(?2) LIMIT 1"
        )
        .bind(project_id)
        .bind(linkedin)
        .fetch_optional(pool)
        .await
        .ok()?;

        if let Some((id,)) = result {
            return Some((id, "linkedin".to_string()));
        }
    }

    // Try fuzzy name match (first_name + last_name)
    let first = record["first_name"].as_str().unwrap_or("").trim();
    let last = record["last_name"].as_str().unwrap_or("").trim();
    if !first.is_empty() && !last.is_empty() {
        // Fetch candidate contacts with the same project_id that have names
        let candidates = sqlx::query_as::<_, (Uuid, String, String)>(
            "SELECT id, first_name, last_name FROM crm_contacts WHERE project_id = ?1 AND first_name IS NOT NULL AND last_name IS NOT NULL"
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
        .ok()?;

        let full_name = format!("{} {}", first, last);
        for (id, existing_first, existing_last) in &candidates {
            let existing_full = format!("{} {}", existing_first, existing_last);
            if fuzzy_name_match(&full_name, &existing_full) {
                return Some((*id, "name".to_string()));
            }
        }
    }

    None
}

/// Check if a company already exists by normalized name or website match
async fn check_company_duplicate(
    pool: &sqlx::SqlitePool,
    record: &Value,
    _organization_id: Option<Uuid>,
) -> Option<(Uuid, String)> {
    let name = record["name"].as_str().filter(|s| !s.is_empty())?;
    let normalized_input = normalize_company_name(name);

    // Fetch all companies and compare with normalized names
    let companies = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id, name FROM companies"
    )
    .fetch_all(pool)
    .await
    .ok()?;

    for (id, existing_name) in &companies {
        let normalized_existing = normalize_company_name(existing_name);
        if normalized_input == normalized_existing {
            return Some((*id, "companies".to_string()));
        }
    }

    // Also try matching by website domain if available
    if let Some(website) = record["website"].as_str().filter(|s| !s.is_empty()) {
        let result = sqlx::query_as::<_, (Uuid,)>(
            "SELECT id FROM companies WHERE LOWER(website) = LOWER(?1) LIMIT 1"
        )
        .bind(website)
        .fetch_optional(pool)
        .await
        .ok()?;

        if let Some((id,)) = result {
            return Some((id, "companies".to_string()));
        }
    }

    None
}

/// Check if a deal already exists by name + pipeline
async fn check_deal_duplicate(
    pool: &sqlx::SqlitePool,
    record: &Value,
    project_id: Option<Uuid>,
) -> Option<(Uuid, String)> {
    let project_id = project_id?;
    let name = record["name"].as_str().filter(|s| !s.is_empty())?;

    let result = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM crm_deals WHERE project_id = ?1 AND LOWER(name) = LOWER(?2) LIMIT 1"
    )
    .bind(project_id)
    .bind(name)
    .fetch_optional(pool)
    .await
    .ok()?;

    if let Some((id,)) = result {
        return Some((id, "crm_deals".to_string()));
    }

    None
}

/// Check if a task already exists by title
async fn check_task_duplicate(
    pool: &sqlx::SqlitePool,
    record: &Value,
    project_id: Option<Uuid>,
) -> Option<(Uuid, String)> {
    let project_id = project_id?;
    let title = record["title"].as_str().filter(|s| !s.is_empty())?;

    let result = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id FROM tasks WHERE project_id = ?1 AND LOWER(title) = LOWER(?2) LIMIT 1"
    )
    .bind(project_id)
    .bind(title)
    .fetch_optional(pool)
    .await
    .ok()?;

    if let Some((id,)) = result {
        return Some((id, "tasks".to_string()));
    }

    None
}

/// Also check within the current staging batch for duplicates (same run producing duplicate records)
async fn check_intra_batch_duplicate(
    pool: &sqlx::SqlitePool,
    workflow_run_id: Uuid,
    target_type: &str,
    record: &Value,
) -> Option<(Uuid, String)> {
    // For contacts: check if same email or phone already staged in this run
    if target_type == "crm_contact" {
        if let Some(email) = record["email"].as_str().filter(|s| !s.is_empty()) {
            let result = sqlx::query_as::<_, (Uuid,)>(
                r#"SELECT id FROM workflow_output_staging
                   WHERE workflow_run_id = ?1 AND target_type = 'crm_contact'
                   AND json_extract(record_data, '$.email') = ?2
                   AND status != 'rejected' LIMIT 1"#
            )
            .bind(workflow_run_id)
            .bind(email)
            .fetch_optional(pool)
            .await
            .ok()?;

            if let Some((id,)) = result {
                return Some((id, "workflow_output_staging".to_string()));
            }
        }

        // Also check phone/mobile within the batch
        for field in &["phone", "mobile"] {
            if let Some(phone) = record[*field].as_str().filter(|s| !s.is_empty()) {
                let phone_trimmed = phone.trim();
                let result = sqlx::query_as::<_, (Uuid,)>(
                    r#"SELECT id FROM workflow_output_staging
                       WHERE workflow_run_id = ?1 AND target_type = 'crm_contact'
                       AND (json_extract(record_data, '$.phone') = ?2 OR json_extract(record_data, '$.mobile') = ?2)
                       AND status != 'rejected' LIMIT 1"#
                )
                .bind(workflow_run_id)
                .bind(phone_trimmed)
                .fetch_optional(pool)
                .await
                .ok()?;

                if let Some((id,)) = result {
                    return Some((id, "workflow_output_staging".to_string()));
                }
            }
        }
    }

    // For companies: check if normalized name already staged
    if target_type == "company" {
        if let Some(name) = record["name"].as_str().filter(|s| !s.is_empty()) {
            let normalized_input = normalize_company_name(name);
            // Fetch all staged company names in this batch and compare normalized
            let staged = sqlx::query_as::<_, (Uuid, String)>(
                r#"SELECT id, json_extract(record_data, '$.name') as staged_name
                   FROM workflow_output_staging
                   WHERE workflow_run_id = ?1 AND target_type = 'company'
                   AND status != 'rejected'"#
            )
            .bind(workflow_run_id)
            .fetch_all(pool)
            .await
            .ok()?;

            for (id, staged_name) in &staged {
                if normalize_company_name(staged_name) == normalized_input {
                    return Some((*id, "workflow_output_staging".to_string()));
                }
            }
        }
    }

    None
}

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

    let data_source = DataSource::find_by_id(pool, data_source_id)
        .await.map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    let workflow = load_workflow(pool, &workflow_id).await
        .map_err(|e| ApiError::InternalError(format!("Failed to load workflow: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow '{}' not found", workflow_id)))?;

    let model = request_model.or(workflow.default_model.clone()).unwrap_or_default();

    let content = data_source.content.clone().unwrap_or_default();

    // Compute content hash for idempotency check (includes workflow structure so
    // definition changes invalidate the cache even if the source content is unchanged)
    let content_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        workflow_id.hash(&mut hasher);
        serde_json::to_string(&workflow.nodes).unwrap_or_default().hash(&mut hasher);
        serde_json::to_string(&workflow.connections).unwrap_or_default().hash(&mut hasher);
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    };

    // Check for existing completed run with same content hash (unless force=true)
    if !force {
        if let Ok(Some(existing_run)) = WorkflowRun::find_by_content_hash(pool, &content_hash).await {
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
    if let Err(e) = WorkflowRun::create(pool, CreateWorkflowRun {
        id: workflow_run_id,
        workflow_id: workflow.id.clone(),
        workflow_name: workflow.name.clone(),
        data_source_id: Some(data_source_id),
        organization_id: data_source.organization_id,
        project_id: data_source.project_id,
        model_used: if model.is_empty() { None } else { Some(model.clone()) },
        content_hash: Some(content_hash),
    }).await {
        tracing::error!("[WORKFLOW] Failed to create workflow run record: {e}");
    }

    let title = &data_source.title;

    // Build dependency map from connections
    let mut deps_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for conn in &workflow.connections {
        deps_map.entry(conn.target.clone()).or_default().push(conn.source.clone());
    }

    // Topological sort: process nodes in dependency order
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut ordered_nodes: Vec<&WorkflowNode> = Vec::new();
    let mut remaining: Vec<&WorkflowNode> = workflow.nodes.iter().collect();

    while !remaining.is_empty() {
        let mut progress = false;
        remaining.retain(|node| {
            let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
            if deps.iter().all(|d| processed.contains(d)) {
                processed.insert(node.id.clone());
                ordered_nodes.push(node);
                progress = true;
                false // remove from remaining
            } else {
                true // keep in remaining
            }
        });
        if !progress {
            // Circular dependency — just process remaining in order
            for node in &remaining {
                ordered_nodes.push(node);
            }
            break;
        }
    }

    // Build downstream output target map: node_id → list of target types
    let mut downstream_targets: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for conn in &workflow.connections {
        if let Some(target_node) = workflow.nodes.iter().find(|n| n.id == conn.target) {
            if target_node.node_type.starts_with("output_") {
                let target_type = target_node.node_type.strip_prefix("output_").unwrap_or("").to_string();
                downstream_targets.entry(conn.source.clone()).or_default().push(target_type);
            }
        }
    }

    let mut step_results: Vec<StepResult> = Vec::new();
    let mut step_outputs: Vec<(String, String, String)> = Vec::new(); // (node_id, output, schema_name)

    let mut all_usage: Vec<Value> = Vec::new();

    for node in &ordered_nodes {
        let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
        let previous: Vec<(&str, &str, &str)> = step_outputs.iter()
            .filter(|(sid, _, _)| deps.contains(sid))
            .map(|(sid, out, schema)| (sid.as_str(), out.as_str(), schema.as_str()))

            .collect();

        let (output, usage_meta) = if node.node_type.starts_with("output_") {
            // Output nodes pass through their input data unchanged
            let input_data = previous.iter()
                .map(|(_, result, _)| result.to_string())

                .collect::<Vec<_>>()
                .join("\n");
            (input_data, None)
        } else {
            let targets = downstream_targets.get(&node.id).map(|v| v.as_slice()).unwrap_or(&[]);
            execute_node_with_llm(pool, node, &content, &previous, &model, targets).await
        };

        // Extract schema name for this node (strip trailing "[]")
        let schema_name = node.parameters.get("output_schema")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_end_matches("[]")
            .to_string();


        let step_index = ordered_nodes.iter().position(|n| n.id == node.id).unwrap_or(0);
        let mut artifact_metadata = json!({
            "data_source_id": data_source_id.to_string(),
            "workflow_id": workflow.id,
            "step_id": node.id,
            "step_index": step_index,
        });
        if let Some(usage) = &usage_meta {
            artifact_metadata["usage"] = usage.clone();
            all_usage.push(usage.clone());
        }

        let artifact = ExecutionArtifact::create(
            pool,
            CreateExecutionArtifact {
                execution_process_id: None,
                artifact_type: ArtifactType::ResearchReport,
                title: format!("{} - {}", workflow.name, node.name),
                content: Some(output.clone()),
                file_path: None,
                metadata: Some(artifact_metadata),
            },
        ).await.map_err(|e| ApiError::InternalError(format!("Failed to create artifact: {e}")))?;

        step_results.push(StepResult {
            step_id: node.id.clone(),
            step_name: node.name.clone(),
            artifact_id: artifact.id,
        });
        step_outputs.push((node.id.clone(), output, schema_name));

    }

    // Check if any LLM node failed (returned error JSON) — fail the run early
    let llm_errors: Vec<String> = step_outputs.iter()
        .filter_map(|(node_id, output, _)| {

            serde_json::from_str::<Value>(output).ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str().map(|s| format!("Node '{}': {}", node_id, s))))
        })
        .collect();

    if !llm_errors.is_empty() {
        // Update run as failed
        let _ = WorkflowRun::update_on_complete(pool, &workflow_run_id.to_string(), UpdateWorkflowRunOnComplete {
            status: "failed".to_string(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_estimated_cost_micros: 0,
            total_records_staged: 0,
            total_duplicates_found: 0,
            node_count: workflow.nodes.len() as i64,
            llm_node_count: workflow.nodes.iter().filter(|n| n.node_type.starts_with("llm_")).count() as i64,
            duration_ms: run_start.elapsed().as_millis() as i64,
        }).await;

        return Err(ApiError::InternalError(format!(
            "Workflow failed — LLM calls returned errors. {}. Check that API keys are configured as environment variables (e.g. ANTHROPIC_API_KEY).",
            llm_errors.first().unwrap_or(&String::new())
        )));
    }

    // Aggregate usage stats
    let total_usage = if all_usage.is_empty() {
        None
    } else {
        let mut total_input: i64 = 0;
        let mut total_output: i64 = 0;
        let mut total_cost: i64 = 0;
        for u in &all_usage {
            total_input += u["input_tokens"].as_i64().unwrap_or(0);
            total_output += u["output_tokens"].as_i64().unwrap_or(0);
            total_cost += u["estimated_cost_micros"].as_i64().unwrap_or(0);
        }
        Some(json!({
            "total_input_tokens": total_input,
            "total_output_tokens": total_output,
            "total_estimated_cost_micros": total_cost,
            "steps": all_usage.len(),
        }))
    };

    // Create staging records for output nodes
    let mut staged_records: i64 = 0;
    for node in ordered_nodes.iter().filter(|n| n.node_type.starts_with("output_")) {
        let target_type = node.node_type.strip_prefix("output_").unwrap_or("");
        let staging_target = match target_type {
            "crm_contacts" => "crm_contact",
            "crm_companies" => "company",
            "crm_deals" => "crm_deal",
            "tasks" => "task",
            _ => continue,
        };

        // Find this node's output
        if let Some((_, output, _)) = step_outputs.iter().find(|(id, _, _)| id == &node.id) {

            if let Ok(parsed) = serde_json::from_str::<Value>(output) {
                let records = extract_records_from_output(&parsed, staging_target);
                for record in records {
                    // Check for duplicates against existing records
                    let dup = match staging_target {
                        "crm_contact" => check_contact_duplicate(pool, &record, data_source.project_id).await,
                        "company" => check_company_duplicate(pool, &record, data_source.organization_id).await,
                        "crm_deal" => check_deal_duplicate(pool, &record, data_source.project_id).await,
                        "task" => check_task_duplicate(pool, &record, data_source.project_id).await,
                        _ => None,
                    };

                    // Also check within the current batch
                    let dup = dup.or(
                        check_intra_batch_duplicate(pool, workflow_run_id, staging_target, &record).await
                    );

                    let (dup_id, dup_type) = match dup {
                        Some((id, t)) => (Some(id), Some(t)),
                        None => (None, None),
                    };

                    // Validate the record against the target schema
                    let mut validation_errors = match validate_record_against_schema(&record, staging_target) {

                        Ok(()) => None,
                        Err(errs) => {
                            tracing::warn!(
                                "[WORKFLOW] Validation errors for {} record in node '{}': {:?}",
                                staging_target, node.id, errs
                            );
                            Some(errs)
                        }
                    };

                    // Flag fallback placeholder records with an explanatory validation error
                    if is_fallback_placeholder(&record) {
                        validation_errors.get_or_insert_with(Vec::new).push(
                            "Placeholder record generated without LLM — no real data extracted".to_string()
                        );
                    }

                    // Run pre-commit business-rule validation
                    let precommit_errs = super::workflow_staging::validate_staging_record(staging_target, &record);
                    if !precommit_errs.is_empty() {
                        validation_errors.get_or_insert_with(Vec::new).extend(precommit_errs);
                    }


                    let is_duplicate = dup_id.is_some();
                    let confidence = compute_confidence(
                        &record,
                        staging_target,
                        validation_errors.as_deref().unwrap_or(&[]),
                        is_duplicate,
                    );

                    // NOTE: Records within a single run are processed sequentially in this
                    // for loop, so intra-batch race conditions don't apply. The dedup check
                    // + create sequence is only vulnerable to races across concurrent trigger
                    // firings for the same data source — a known limitation with SQLite.
                    match WorkflowStagingRecord::create(pool, CreateStagingRecord {
                        workflow_run_id,
                        workflow_id: workflow.id.clone(),
                        node_id: node.id.clone(),
                        data_source_id: Some(data_source_id),
                        organization_id: data_source.organization_id,
                        project_id: data_source.project_id,
                        target_type: staging_target.to_string(),
                        record_data: record,
                        duplicate_of_id: dup_id,
                        duplicate_of_type: dup_type,
                        confidence: Some(confidence),
                        validation_errors,
                    }).await {
                        Ok(_) => staged_records += 1,
                        Err(e) => tracing::error!("[WORKFLOW] Failed to create staging record: {e}"),
                    }
                }
            }
        }
    }

    // Count duplicates and LLM nodes
    let duplicates_found = {
        let staging_records = WorkflowStagingRecord::find_by_run(pool, workflow_run_id).await.unwrap_or_default();
        staging_records.iter().filter(|r| r.duplicate_of_id.is_some()).count() as i64
    };
    let node_count = workflow.nodes.len() as i64;
    let llm_node_count = workflow.nodes.iter().filter(|n| n.node_type.starts_with("llm_")).count() as i64;
    let duration_ms = run_start.elapsed().as_millis() as i64;

    let (total_input, total_output, total_cost) = if let Some(ref usage) = total_usage {
        (
            usage["total_input_tokens"].as_i64().unwrap_or(0),
            usage["total_output_tokens"].as_i64().unwrap_or(0),
            usage["total_estimated_cost_micros"].as_i64().unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };

    // Update workflow run with final stats
    if let Err(e) = WorkflowRun::update_on_complete(pool, &workflow_run_id.to_string(), UpdateWorkflowRunOnComplete {
        status: "completed".to_string(),
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_estimated_cost_micros: total_cost,
        total_records_staged: staged_records,
        total_duplicates_found: duplicates_found,
        node_count,
        llm_node_count,
        duration_ms,
    }).await {
        tracing::error!("[WORKFLOW] Failed to update workflow run on complete: {e}");
    }

    Ok(Json(ApiResponse::success(WorkflowRunResult {
        workflow_run_id,
        workflow_id: workflow.id,
        workflow_name: workflow.name,
        data_source_id,
        steps: step_results,
        total_usage,
        staged_records,
        reused: None,
    })))
}

/// GET /api/data-sources/:id/artifacts
async fn list_data_source_artifacts(
    Path(data_source_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ExecutionArtifact>>>, ApiError> {
    let pool = &deployment.db().pool;
    DataSource::find_by_id(pool, data_source_id)
        .await.map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
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
    let workflows = load_all_workflows(pool).await
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

    let existing = load_workflow(pool, &req.id).await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?;
    if existing.is_some() {
        return Err(ApiError::BadRequest(format!("Workflow '{}' already exists", req.id)));
    }

    let owner_type = req.owner_type.unwrap_or_else(|| "organization".to_string());
    if !["organization", "user"].contains(&owner_type.as_str()) {
        return Err(ApiError::BadRequest("owner_type must be 'organization' or 'user'".to_string()));
    }

    let wf = WorkflowDefinition {
        id: req.id, name: req.name, description: req.description,
        nodes: req.nodes, connections: req.connections, is_system: false,
        owner_type: owner_type.clone(), owner_id: req.owner_id.clone(),
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

    let existing = load_workflow(pool, &workflow_id).await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow '{}' not found", workflow_id)))?;

    let name = req.name.unwrap_or(existing.name);
    let description = req.description.or(existing.description);
    let nodes = req.nodes.unwrap_or(existing.nodes);
    let connections = req.connections.unwrap_or(existing.connections);
    let default_model = req.default_model.or(existing.default_model);

    let wf = WorkflowDefinition {
        id: workflow_id.clone(), name: name.clone(), description: description.clone(),
        nodes: nodes.clone(), connections: connections.clone(), is_system: existing.is_system,
        owner_type: existing.owner_type.clone(), owner_id: existing.owner_id.clone(),
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

    let existing = load_workflow(pool, &workflow_id).await
        .map_err(|e| ApiError::InternalError(format!("DB error: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow '{}' not found", workflow_id)))?;

    if existing.is_system {
        return Err(ApiError::BadRequest("Cannot delete system workflows".to_string()));
    }

    sqlx::query("DELETE FROM workflow_definitions WHERE id = ?1")
        .bind(&workflow_id).execute(pool).await
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

// ── LLM execution via PCG Router ─────────────────────────────────────────────

/// Execute a workflow node's LLM prompt via the PCG Router.
/// Routes through all configured providers with priority-based fallback.
/// Returns an error string (instead of mock data) if no models are available.
async fn execute_node_with_llm(
    pool: &sqlx::SqlitePool,
    node: &WorkflowNode,
    content: &str,
    previous_results: &[(&str, &str, &str)],  // (node_id, output, schema_name)

    model: &str,
    target_schemas: &[String],
) -> (String, Option<Value>) {
    let prompt_template = node.parameters.get("prompt_template")
        .and_then(|v| v.as_str())
        .unwrap_or("Analyze the following content:\n{{content}}");

    // Build the actual prompt by substituting template variables
    // {{previous_results}} — backwards-compatible merged text of all upstream outputs
    let prev_text = previous_results.iter()
        .map(|(id, result, _)| format!("[{}]: {}", id, result))

        .collect::<Vec<_>>()
        .join("\n\n");

    // Build schema text dynamically from output_schemas definitions
    let schema_text = if !target_schemas.is_empty() {
        let schemas: Vec<String> = target_schemas.iter().filter_map(|t| {
            build_schema_prompt_text(t)
        }).collect();
        schemas.join("\n\n")
    } else {
        String::new()
    };

    // Wrap content with clear delimiters so the LLM distinguishes data from instructions
    let wrapped_content = format!("--- BEGIN SOURCE CONTENT ---\n{}\n--- END SOURCE CONTENT ---", content);

    let mut prompt = prompt_template

        .replace("{{content}}", &wrapped_content)
        .replace("{{previous_results}}", &prev_text)
        .replace("{{target_schema}}", &schema_text);

    // Named variable substitution: replace {{schema_name}} with that upstream node's output
    // e.g. if an upstream node has output_schema "contacts[]", replace {{contacts}} with its output
    for (_, result, schema_name) in previous_results {
        if !schema_name.is_empty() {
            let placeholder = format!("{{{{{}}}}}", schema_name); // produces {{schema_name}}
            prompt = prompt.replace(&placeholder, result);
        }
    }


    // If target_schema is non-empty but the prompt didn't contain the placeholder, append it
    let prompt = if !schema_text.is_empty() && !prompt_template.contains("{{target_schema}}") {
        format!("{}\n\n--- OUTPUT FORMAT ---\n{}", prompt, schema_text)
    } else {
        prompt
    };

    // Route through PCG Router — handles multi-provider fallback
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: json!("You are a precise data extraction assistant. Your task is to extract REAL entities (people, companies, deals, tasks) that are explicitly mentioned in the source content provided. Rules:\n1. Only extract entities that are clearly and explicitly named in the source text.\n2. NEVER use document metadata (titles, dates, section headings) as entity names.\n3. NEVER fabricate or hallucinate entities that are not in the source.\n4. If no entities of the requested type exist in the source, return an empty array.\n5. Always output valid JSON without markdown formatting or preamble."),
        },
        ChatMessage {
            role: "user".to_string(),
            content: json!(prompt),
        },
    ];

    // Use per-node model override if set, otherwise use the workflow-level model
    let node_model = node.parameters.get("model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| if model.is_empty() { None } else { Some(model) });

    match pcg_router::route_completion(pool, messages.clone(), node_model, Some(2048), None).await {
        Ok((resp, metadata)) => {
            tracing::info!(
                "[WORKFLOW] Node '{}' routed via {} ({}), tokens: {:?}/{:?}",
                node.id, metadata.model_used, metadata.provider,
                metadata.input_tokens, metadata.output_tokens
            );
            let usage_meta = json!({
                "model_used": metadata.model_used,
                "provider": metadata.provider,
                "input_tokens": metadata.input_tokens,
                "output_tokens": metadata.output_tokens,
                "estimated_cost_micros": metadata.estimated_cost_micros,
            });
            // Extract text from OpenAI-format response
            if let Some(text) = resp["choices"][0]["message"]["content"].as_str() {
                if !text.is_empty() {
                    // Try to parse the response as JSON — if it fails, attempt one repair retry
                    let trimmed = text.trim();
                    // Strip markdown code fences if present
                    let json_text = if trimmed.starts_with("```") {
                        trimmed
                            .trim_start_matches("```json")
                            .trim_start_matches("```")
                            .trim_end_matches("```")
                            .trim()
                    } else {
                        trimmed
                    };

                    if serde_json::from_str::<Value>(json_text).is_ok() {
                        // Valid JSON — return the cleaned text
                        return (json_text.to_string(), Some(usage_meta));
                    }

                    // JSON parse failed — attempt one repair retry
                    tracing::warn!(
                        "[WORKFLOW] Node '{}' returned invalid JSON, attempting repair retry",
                        node.id
                    );
                    let repair_messages = vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: json!("You are a data extraction and analysis assistant. Always output valid JSON. Do not include markdown formatting or preamble — respond with raw JSON only."),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: json!(format!(
                                "The previous response was not valid JSON. Please fix it and return only valid JSON. Do not include any explanation or markdown formatting.\n\nOriginal response:\n{}",
                                text
                            )),
                        },
                    ];

                    match pcg_router::route_completion(pool, repair_messages, node_model, Some(2048), None).await {
                        Ok((retry_resp, retry_meta)) => {
                            tracing::info!(
                                "[WORKFLOW] Node '{}' repair retry via {} ({})",
                                node.id, retry_meta.model_used, retry_meta.provider
                            );
                            // Merge usage metadata
                            let combined_usage = json!({
                                "model_used": retry_meta.model_used,
                                "provider": retry_meta.provider,
                                "input_tokens": metadata.input_tokens.unwrap_or(0) + retry_meta.input_tokens.unwrap_or(0),
                                "output_tokens": metadata.output_tokens.unwrap_or(0) + retry_meta.output_tokens.unwrap_or(0),
                                "estimated_cost_micros": metadata.estimated_cost_micros.unwrap_or(0) + retry_meta.estimated_cost_micros.unwrap_or(0),
                                "retry_used": true,
                            });
                            if let Some(retry_text) = retry_resp["choices"][0]["message"]["content"].as_str() {
                                let retry_trimmed = retry_text.trim();
                                let retry_json = if retry_trimmed.starts_with("```") {
                                    retry_trimmed
                                        .trim_start_matches("```json")
                                        .trim_start_matches("```")
                                        .trim_end_matches("```")
                                        .trim()
                                } else {
                                    retry_trimmed
                                };
                                if !retry_json.is_empty() {
                                    return (retry_json.to_string(), Some(combined_usage));
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("[WORKFLOW] Repair retry failed for node '{}': {e}", node.id);
                        }
                    }

                    // Return original text even if repair failed — validation will catch errors downstream
                    return (text.to_string(), Some(usage_meta));
                }
            }
            tracing::warn!("[WORKFLOW] Empty response from router for node '{}'", node.id);
        }
        Err(e) => {
            tracing::error!("[WORKFLOW] PCG Router failed for node '{}': {e}. Check that API keys are configured (e.g. ANTHROPIC_API_KEY env var).", node.id);
        }
    }

    // Fallback to content-aware mock extraction when no LLM is available
    tracing::warn!("[WORKFLOW] Falling back to mock extraction for node '{}' ({})", node.id, node.name);
    let output_schema = node.parameters.get("output_schema")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let node_type = &node.node_type;
    let prev_refs: Vec<(&str, &str)> = previous_results.iter().map(|(id, out, _)| (*id, *out)).collect();
    let mock_result = generate_mock_step_result(&node.id, content, &node.name, &prev_refs, node_type, output_schema);
    (mock_result, None)

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

    // Build dependency map
    let mut deps_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for conn in &req.connections {
        deps_map.entry(conn.target.clone()).or_default().push(conn.source.clone());
    }

    // Build downstream output target map
    let mut downstream_targets: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for conn in &req.connections {
        if let Some(target_node) = req.nodes.iter().find(|n| n.id == conn.target) {
            if target_node.node_type.starts_with("output_") {
                let target_type = target_node.node_type.strip_prefix("output_").unwrap_or("").to_string();
                downstream_targets.entry(conn.source.clone()).or_default().push(target_type);
            }
        }
    }

    // Topological sort
    let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut ordered: Vec<&WorkflowNode> = Vec::new();
    let mut remaining: Vec<&WorkflowNode> = req.nodes.iter().collect();

    while !remaining.is_empty() {
        let mut progress = false;
        remaining.retain(|node| {
            let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
            if deps.iter().all(|d| processed.contains(d)) {
                processed.insert(node.id.clone());
                ordered.push(node);
                progress = true;
                false
            } else {
                true
            }
        });
        if !progress {
            for node in &remaining { ordered.push(node); }
            break;
        }
    }

    let mut results: Vec<PreviewNodeResult> = Vec::new();
    let mut outputs: Vec<(String, String, String)> = Vec::new(); // (node_id, output, schema_name)

    for node in &ordered {
        let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
        let previous: Vec<(&str, &str, &str)> = outputs.iter()
            .filter(|(sid, _, _)| deps.contains(sid))
            .map(|(sid, out, schema)| (sid.as_str(), out.as_str(), schema.as_str()))

            .collect();

        let (output, usage_meta) = if node.node_type.starts_with("output_") {
            // Output nodes pass through their input data unchanged
            let input_data = previous.iter()
                .map(|(_, result, _)| result.to_string())

                .collect::<Vec<_>>()
                .join("\n");
            (input_data, None)
        } else {
            let targets = downstream_targets.get(&node.id).map(|v| v.as_slice()).unwrap_or(&[]);
            execute_node_with_llm(pool, node, &content, &previous, "", targets).await
        };

        let schema_name = node.parameters.get("output_schema")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_end_matches("[]")
            .to_string();

        // For output nodes, extract and validate records against schema
        let records = if node.node_type.starts_with("output_") {
            let target_type = node.node_type.strip_prefix("output_").unwrap_or("");
            let staging_target = match target_type {
                "crm_contacts" => Some("crm_contact"),
                "crm_companies" => Some("company"),
                "crm_deals" => Some("crm_deal"),
                "tasks" => Some("task"),
                _ => None,
            };
            staging_target.and_then(|st| {
                serde_json::from_str::<Value>(&output).ok().map(|parsed| {
                    extract_records_from_output(&parsed, st).iter().map(|record| {
                        let validation_errors = match validate_record_against_schema(record, st) {
                            Ok(()) => vec![],
                            Err(errs) => errs,
                        };
                        let confidence = compute_confidence(record, st, &validation_errors, false);
                        PreviewRecordInfo {
                            data: record.clone(),
                            validation_errors,
                            confidence,
                        }
                    }).collect::<Vec<_>>()
                })
            })
        } else {
            None
        };


        results.push(PreviewNodeResult {
            node_id: node.id.clone(),
            node_name: node.name.clone(),
            node_type: node.node_type.clone(),
            output: output.clone(),
            usage: usage_meta,
            records,
        });
        outputs.push((node.id.clone(), output, schema_name));

    }

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

    let available: Vec<AvailableModel> = models.iter().enumerate().map(|(i, m)| {
        AvailableModel {
            id: m.model_id.clone(),
            label: format!("{} ({})", m.name, m.provider),
            is_default: i == 0, // highest priority (first) is default
            provider: m.provider.clone(),
            cost_per_million_input: m.cost_per_million_input,
            cost_per_million_output: m.cost_per_million_output,
        }
    }).collect();

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
    let run_uuid = Uuid::parse_str(&id)
        .map_err(|e| ApiError::InternalError(format!("Invalid UUID: {e}")))?;
    let staging_records = WorkflowStagingRecord::find_by_run(pool, run_uuid)
        .await
        .unwrap_or_default();

    let total = staging_records.len() as f64;
    let approved = staging_records.iter().filter(|r| r.status == "approved" || r.status == "committed").count() as f64;
    let rejected = staging_records.iter().filter(|r| r.status == "rejected").count() as f64;
    let committed = staging_records.iter().filter(|r| r.status == "committed").count() as f64;
    let duplicates = staging_records.iter().filter(|r| r.duplicate_of_id.is_some()).count() as f64;

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
pub async fn fire_triggers_for_data_source(pool: sqlx::SqlitePool, data_source_id: Uuid) {
    use db::models::workflow_trigger::WorkflowTrigger;

    let ds = match DataSource::find_by_id(&pool, data_source_id).await {
        Ok(Some(ds)) => ds,
        _ => return,
    };

    let org_id = ds.organization_id.map(|u| u.to_string());
    let proj_id = ds.project_id.map(|u| u.to_string());

    let triggers = match WorkflowTrigger::find_matching_triggers(
        &pool,
        &ds.data_type,
        org_id.as_deref(),
        proj_id.as_deref(),
    ).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("[TRIGGER] Failed to find matching triggers for ds {}: {}", data_source_id, e);
            return;
        }
    };

    if triggers.is_empty() {
        return;
    }

    tracing::info!(
        "[TRIGGER] Found {} matching trigger(s) for data source {} (type={})",
        triggers.len(), data_source_id, ds.data_type
    );

    for trigger in triggers {
        let pool = pool.clone();
        let ds_id = data_source_id;
        let trigger_id = trigger.id.clone();
        let workflow_id = trigger.workflow_id.clone();
        let model_override = trigger.model_override.clone();

        tokio::spawn(async move {
            tracing::info!(
                "[TRIGGER] Firing trigger '{}' (workflow={}) for data source {}",
                trigger_id, workflow_id, ds_id
            );

            // Increment trigger count
            if let Err(e) = WorkflowTrigger::increment_trigger_count(&pool, &trigger_id).await {
                tracing::warn!("[TRIGGER] Failed to increment trigger count for '{}': {e}", trigger_id);
            }

            // Load the workflow definition
            let workflow = match load_workflow(&pool, &workflow_id).await {
                Ok(Some(wf)) => wf,
                Ok(None) => {
                    tracing::error!("[TRIGGER] Workflow '{}' not found for trigger '{}'", workflow_id, trigger_id);
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
            let data_source = match DataSource::find_by_id(&pool, ds_id).await {
                Ok(Some(ds)) => ds,
                _ => {
                    tracing::error!("[TRIGGER] Data source {} not found", ds_id);
                    return;
                }
            };

            let workflow_run_id = uuid::Uuid::new_v4();
            let run_start = std::time::Instant::now();

            // Create workflow run record
            if let Err(e) = WorkflowRun::create(&pool, CreateWorkflowRun {
                id: workflow_run_id,
                workflow_id: workflow.id.clone(),
                workflow_name: workflow.name.clone(),
                data_source_id: Some(ds_id),
                organization_id: data_source.organization_id,
                project_id: data_source.project_id,
                model_used: if model.is_empty() { None } else { Some(model.clone()) },
                content_hash: None,
            }).await {
                tracing::error!("[TRIGGER] Failed to create workflow run record: {e}");
                return;
            }

            let content = data_source.content.unwrap_or_default();

            // Build dependency map
            let mut deps_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
            for conn in &workflow.connections {
                deps_map.entry(conn.target.clone()).or_default().push(conn.source.clone());
            }

            // Topological sort
            let mut processed: std::collections::HashSet<String> = std::collections::HashSet::new();
            let mut ordered_nodes: Vec<&WorkflowNode> = Vec::new();
            let mut remaining: Vec<&WorkflowNode> = workflow.nodes.iter().collect();

            while !remaining.is_empty() {
                let mut progress = false;
                remaining.retain(|node| {
                    let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
                    if deps.iter().all(|d| processed.contains(d)) {
                        processed.insert(node.id.clone());
                        ordered_nodes.push(node);
                        progress = true;
                        false
                    } else {
                        true
                    }
                });
                if !progress {
                    for node in &remaining {
                        ordered_nodes.push(node);
                    }
                    break;
                }
            }

            // Build downstream output target map
            let mut downstream_targets: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
            for conn in &workflow.connections {
                if let Some(target_node) = workflow.nodes.iter().find(|n| n.id == conn.target) {
                    if target_node.node_type.starts_with("output_") {
                        let target_type = target_node.node_type.strip_prefix("output_").unwrap_or("").to_string();
                        downstream_targets.entry(conn.source.clone()).or_default().push(target_type);
                    }
                }
            }

            let mut step_outputs: Vec<(String, String, String)> = Vec::new(); // (node_id, output, schema_name)

            let mut all_usage: Vec<serde_json::Value> = Vec::new();
            let mut staged_records: i64 = 0;

            for node in &ordered_nodes {
                let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
                let previous: Vec<(&str, &str, &str)> = step_outputs.iter()
                    .filter(|(sid, _, _)| deps.contains(sid))
                    .map(|(sid, out, schema)| (sid.as_str(), out.as_str(), schema.as_str()))

                    .collect();

                let (output, usage_meta) = if node.node_type.starts_with("output_") {
                    let input_data = previous.iter()
                        .map(|(_, result, _)| result.to_string())

                        .collect::<Vec<_>>()
                        .join("\n");
                    (input_data, None)
                } else {
                    let targets = downstream_targets.get(&node.id).map(|v| v.as_slice()).unwrap_or(&[]);
                    execute_node_with_llm(&pool, node, &content, &previous, &model, targets).await
                };

                let schema_name = node.parameters.get("output_schema")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim_end_matches("[]")
                    .to_string();


                let step_index = ordered_nodes.iter().position(|n| n.id == node.id).unwrap_or(0);
                let mut artifact_metadata = serde_json::json!({
                    "data_source_id": ds_id.to_string(),
                    "workflow_id": workflow.id,
                    "step_id": node.id,
                    "step_index": step_index,
                    "trigger_id": trigger_id,
                });
                if let Some(usage) = &usage_meta {
                    artifact_metadata["usage"] = usage.clone();
                    all_usage.push(usage.clone());
                }

                if let Err(e) = ExecutionArtifact::create(
                    &pool,
                    CreateExecutionArtifact {
                        execution_process_id: None,
                        artifact_type: ArtifactType::ResearchReport,
                        title: format!("{} - {} [auto]", workflow.name, node.name),
                        content: Some(output.clone()),
                        file_path: None,
                        metadata: Some(artifact_metadata),
                    },
                ).await {
                    tracing::error!("[TRIGGER] Failed to create execution artifact for node '{}': {e}", node.id);
                }

                step_outputs.push((node.id.clone(), output, schema_name));

            }

            // Create staging records for output nodes
            for node in ordered_nodes.iter().filter(|n| n.node_type.starts_with("output_")) {
                let target_type = node.node_type.strip_prefix("output_").unwrap_or("");
                let staging_target = match target_type {
                    "crm_contacts" => "crm_contact",
                    "crm_companies" => "company",
                    "crm_deals" => "crm_deal",
                    "tasks" => "task",
                    _ => continue,
                };

                if let Some((_, output, _)) = step_outputs.iter().find(|(id, _, _)| id == &node.id) {

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(output) {
                        let records = extract_records_from_output(&parsed, staging_target);
                        for record in records {
                            let dup = match staging_target {
                                "crm_contact" => check_contact_duplicate(&pool, &record, data_source.project_id).await,
                                "company" => check_company_duplicate(&pool, &record, data_source.organization_id).await,
                                "crm_deal" => check_deal_duplicate(&pool, &record, data_source.project_id).await,
                                "task" => check_task_duplicate(&pool, &record, data_source.project_id).await,
                                _ => None,
                            };
                            let dup = dup.or(
                                check_intra_batch_duplicate(&pool, workflow_run_id, staging_target, &record).await
                            );
                            let (dup_id, dup_type) = match dup {
                                Some((id, t)) => (Some(id), Some(t)),
                                None => (None, None),
                            };
                            let mut validation_errors = match validate_record_against_schema(&record, staging_target) {

                                Ok(()) => None,
                                Err(errs) => Some(errs),
                            };

                            // Flag fallback placeholder records with an explanatory validation error
                            if is_fallback_placeholder(&record) {
                                validation_errors.get_or_insert_with(Vec::new).push(
                                    "Placeholder record generated without LLM — no real data extracted".to_string()
                                );
                            }

                            // Run pre-commit business-rule validation
                            let precommit_errs = super::workflow_staging::validate_staging_record(staging_target, &record);
                            if !precommit_errs.is_empty() {
                                validation_errors.get_or_insert_with(Vec::new).extend(precommit_errs);
                            }


                            let is_duplicate = dup_id.is_some();
                            let confidence = compute_confidence(
                                &record,
                                staging_target,
                                validation_errors.as_deref().unwrap_or(&[]),
                                is_duplicate,
                            );

                            // NOTE: Records within a single trigger run are processed sequentially,
                            // so intra-batch dedup races don't apply here. Cross-run races are a
                            // known limitation with SQLite's limited concurrency.
                            match WorkflowStagingRecord::create(&pool, CreateStagingRecord {
                                workflow_run_id,
                                workflow_id: workflow.id.clone(),
                                node_id: node.id.clone(),
                                data_source_id: Some(ds_id),
                                organization_id: data_source.organization_id,
                                project_id: data_source.project_id,
                                target_type: staging_target.to_string(),
                                record_data: record,
                                duplicate_of_id: dup_id,
                                duplicate_of_type: dup_type,
                                confidence: Some(confidence),
                                validation_errors,
                            }).await {
                                Ok(_) => staged_records += 1,
                                Err(e) => tracing::error!("[TRIGGER] Failed to create staging record: {e}"),
                            }
                        }
                    }
                }
            }

            // Aggregate stats
            let mut total_input: i64 = 0;
            let mut total_output: i64 = 0;
            let mut total_cost: i64 = 0;
            for u in &all_usage {
                total_input += u["input_tokens"].as_i64().unwrap_or(0);
                total_output += u["output_tokens"].as_i64().unwrap_or(0);
                total_cost += u["estimated_cost_micros"].as_i64().unwrap_or(0);
            }

            let duplicates_found = {
                let staging_records = WorkflowStagingRecord::find_by_run(&pool, workflow_run_id).await.unwrap_or_default();
                staging_records.iter().filter(|r| r.duplicate_of_id.is_some()).count() as i64
            };
            let node_count = workflow.nodes.len() as i64;
            let llm_node_count = workflow.nodes.iter().filter(|n| n.node_type.starts_with("llm_")).count() as i64;
            let duration_ms = run_start.elapsed().as_millis() as i64;

            if let Err(e) = WorkflowRun::update_on_complete(&pool, &workflow_run_id.to_string(), UpdateWorkflowRunOnComplete {
                status: "completed".to_string(),
                total_input_tokens: total_input,
                total_output_tokens: total_output,
                total_estimated_cost_micros: total_cost,
                total_records_staged: staged_records,
                total_duplicates_found: duplicates_found,
                node_count,
                llm_node_count,
                duration_ms,
            }).await {
                tracing::error!("[TRIGGER] Failed to update workflow run on complete: {e}");
            }

            tracing::info!(
                "[TRIGGER] Completed trigger '{}' workflow run {} ({} records staged, {}ms)",
                trigger_id, workflow_run_id, staged_records, duration_ms
            );
        });
    }
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/data-sources/{id}/workflows", get(list_workflows_for_data_source))
        .route("/data-sources/{id}/workflows/{workflow_id}/run", post(run_workflow))
        .route("/data-sources/{id}/artifacts", get(list_data_source_artifacts))
        .route("/workflows/definitions", get(list_all_workflow_definitions).post(create_workflow_definition))
        .route("/workflows/definitions/{id}", put(update_workflow_definition).delete(delete_workflow_definition))
        .route("/workflows/preview", post(preview_workflow))
        .route("/workflows/models", get(list_available_models))
        .route("/workflows/runs/recent", get(list_recent_runs))
        .route("/workflows/runs/{id}", get(get_run_by_id))
        .route("/workflows/runs/{id}/stats", get(get_run_stats))
        .route("/artifacts/recent", get(list_recent_artifacts))
}
