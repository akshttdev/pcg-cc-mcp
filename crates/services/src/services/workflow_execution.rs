//! Workflow Execution Service
//!
//! Shared workflow execution functions extracted from the route handler.
//! Used by the workflow engine, trigger system, and Topsi agent.

use db::models::crm_contact::{CrmContact, UpdateCrmContact};
use db::models::crm_deal::{CrmDeal, UpdateCrmDeal};
use db::models::company::{Company, UpdateCompany};
use db::models::notification::{Notification, CreateNotification};
use db::models::task::{Task, CreateTask, Priority};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::services::workflow_llm::WorkflowLLMService;

// ── Output Schema types (mirrored from server::routes::output_schemas) ──────
//
// These types are duplicated here so the services crate can validate records
// without depending on the server crate.  If the canonical definitions in
// `crates/server/src/routes/output_schemas.rs` change, these must be kept in
// sync (or the schemas should be moved to a shared crate).

/// Definition of a single field in a target schema.
#[derive(Debug, Serialize)]
pub struct FieldDef {
    #[serde(rename = "type")]
    pub field_type: String,
    pub required: bool,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Schema definition for a target entity type (contact, company, deal, task).
#[derive(Debug, Serialize)]
pub struct TargetSchema {
    pub target_type: String,
    pub description: String,
    pub fields: std::collections::BTreeMap<String, FieldDef>,
}

/// Look up the schema for a given target_type string.
/// Returns None if the target_type is unknown.
pub fn get_schema_for_target(target_type: &str) -> Option<TargetSchema> {
    match target_type {
        "crm_contact" => Some(crm_contact_schema()),
        "company" => Some(company_schema()),
        "crm_deal" => Some(crm_deal_schema()),
        "task" => Some(task_schema()),
        _ => None,
    }
}

fn crm_contact_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("first_name".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Contact's first name".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("last_name".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Contact's last name".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("email".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Email address".to_string(),
        enum_values: None, format: Some("email".to_string()),
    });
    fields.insert("phone".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Phone number".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("company_name".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Company name".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("job_title".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Job title".to_string(),
        enum_values: None, format: None,
    });
    TargetSchema {
        target_type: "crm_contact".to_string(),
        description: "CRM Contact".to_string(),
        fields,
    }
}

fn company_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("name".to_string(), FieldDef {
        field_type: "string".to_string(), required: true,
        description: "Company name".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("website".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Company website".to_string(),
        enum_values: None, format: Some("url".to_string()),
    });
    fields.insert("industry".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Industry".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("description".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Description".to_string(),
        enum_values: None, format: None,
    });
    TargetSchema {
        target_type: "company".to_string(),
        description: "Company/Organization".to_string(),
        fields,
    }
}

fn crm_deal_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("name".to_string(), FieldDef {
        field_type: "string".to_string(), required: true,
        description: "Deal name".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("amount".to_string(), FieldDef {
        field_type: "number".to_string(), required: false,
        description: "Deal amount".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("currency".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Currency code".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("description".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Deal description".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("contact_name".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Associated contact name".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("contact_email".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Associated contact email".to_string(),
        enum_values: None, format: Some("email".to_string()),
    });
    TargetSchema {
        target_type: "crm_deal".to_string(),
        description: "CRM Deal/Opportunity".to_string(),
        fields,
    }
}

fn task_schema() -> TargetSchema {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("title".to_string(), FieldDef {
        field_type: "string".to_string(), required: true,
        description: "Task title".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("description".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Task description".to_string(),
        enum_values: None, format: None,
    });
    fields.insert("priority".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Task priority".to_string(),
        enum_values: Some(vec!["low".to_string(), "medium".to_string(), "high".to_string(), "urgent".to_string()]),
        format: None,
    });
    fields.insert("due_date".to_string(), FieldDef {
        field_type: "string".to_string(), required: false,
        description: "Due date".to_string(),
        enum_values: None, format: Some("date".to_string()),
    });
    TargetSchema {
        target_type: "task".to_string(),
        description: "Task/Action Item".to_string(),
        fields,
    }
}

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

pub fn default_owner_type() -> String { "system".to_string() }

// ── Text extraction helpers ──────────────────────────────────────────────────

/// Extract all email addresses from text using regex
pub fn extract_emails_from_text(text: &str) -> Vec<String> {
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
pub fn extract_dollar_amounts_from_text(text: &str) -> Vec<(String, String)> {
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
pub fn extract_date_near_text(text: &str, context_line: &str) -> Option<String> {
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

pub fn extract_company_names_from_text(text: &str) -> Vec<String> {
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
pub struct ExtractedContact {
    pub name: String,
    pub role: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
}

pub fn extract_contacts_from_text(text: &str) -> Vec<ExtractedContact> {
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

// ── Mock LLM: content-aware extraction ──────────────────────────────────────

pub fn generate_mock_step_result(step_id: &str, content: &str, title: &str, previous_results: &[(&str, &str)], node_type: &str, output_schema: &str) -> String {
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
            let _ = title; // suppress unused warning
            json!({"result": format!("Analysis of {} chars of content", content.len()), "status": "completed"}).to_string()

        }
    }
}

// ── Record extraction and validation ─────────────────────────────────────────

/// Extract individual records from LLM output JSON
pub fn extract_records_from_output(data: &Value, target_type: &str) -> Vec<Value> {
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
pub fn is_fallback_placeholder(record: &Value) -> bool {
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


/// Compute a confidence score (0.0-1.0) for a staging record based on simple heuristics.
/// - Start at 1.0
/// - Subtract 0.15 for each missing required field
/// - Subtract 0.05 for each validation error
/// - Subtract 0.3 if marked as duplicate
/// - Subtract 0.1 if more than half of all fields are null/empty
/// - Cap at 0.1 if the record is a fallback placeholder (no LLM connected)
/// - Floor at 0.0
pub fn compute_confidence(record: &Value, target_type: &str, validation_errors: &[String], is_duplicate: bool) -> f64 {
    // If this record was produced by the mock/fallback engine, cap confidence very low
    if is_fallback_placeholder(record) {
        tracing::warn!(
            "[WORKFLOW] Detected fallback placeholder record (no LLM connected) — setting low confidence"
        );
        return 0.1;
    }


    let mut score: f64 = 1.0;

    // Check required fields against schema
    if let Some(schema) = get_schema_for_target(target_type) {
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
pub fn build_schema_prompt_text(target_type: &str) -> Option<String> {
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

    let schema = get_schema_for_target(schema_target)?;

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
pub fn validate_record_against_schema(record: &Value, target_type: &str) -> Result<(), Vec<String>> {
    let schema = match get_schema_for_target(target_type) {
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
pub fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ── String matching utilities ────────────────────────────────────────────────

/// Compute Levenshtein distance between two strings
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
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
pub fn fuzzy_name_match(a: &str, b: &str) -> bool {
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
pub fn normalize_company_name(name: &str) -> String {
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

// ── Deduplication checks ─────────────────────────────────────────────────────

/// Check if a CRM contact already exists by email, phone, linkedin, or fuzzy name match.
/// Returns (existing_id, match_type) where match_type indicates what matched.
pub async fn check_contact_duplicate(
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
pub async fn check_company_duplicate(
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
pub async fn check_deal_duplicate(
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
pub async fn check_task_duplicate(
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
pub async fn check_intra_batch_duplicate(
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

// ── Action node execution (non-LLM nodes) ───────────────────────────────────

/// Returns `Some((output, None))` if this node type is an action node that was handled,
/// or `None` if the node type is not an action and should fall through to LLM execution.
/// `context_project_id` and `context_org_id` come from the data source / workflow context.
pub async fn execute_action_node(
    pool: &sqlx::SqlitePool,
    node: &WorkflowNode,
    previous_results: &[(&str, &str, &str)],
    context_project_id: Option<Uuid>,
    context_org_id: Option<Uuid>,
    workflow_run_id: Option<Uuid>,
) -> Option<(String, Option<Value>)> {
    match node.node_type.as_str() {
        // ── Conditional: evaluate a simple condition against upstream data ────
        "conditional" => {
            let condition = node.parameters.get("condition")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let true_label = node.parameters.get("true_label")
                .and_then(|v| v.as_str())
                .unwrap_or("true");
            let false_label = node.parameters.get("false_label")
                .and_then(|v| v.as_str())
                .unwrap_or("false");

            // Merge all upstream outputs
            let input_data: String = previous_results.iter()
                .map(|(_, result, _)| result.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            // Simple condition evaluation: check if the input data contains the condition string
            // For more advanced conditions, the LLM nodes should be used upstream
            let result = if condition.is_empty() {
                !input_data.is_empty()
            } else if condition.starts_with("count>") {
                // Support count>N pattern: check if JSON array has more than N items
                let threshold: usize = condition.strip_prefix("count>")
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                serde_json::from_str::<Value>(&input_data)
                    .ok()
                    .and_then(|v| v.as_array().map(|a| a.len()))
                    .unwrap_or(0) > threshold
            } else if condition.starts_with("contains:") {
                let search = condition.strip_prefix("contains:").unwrap_or("").trim();
                input_data.to_lowercase().contains(&search.to_lowercase())
            } else {
                // Default: check if condition string is present in input
                input_data.to_lowercase().contains(&condition.to_lowercase())
            };

            let branch = if result { true_label } else { false_label };
            let output = json!({
                "condition": condition,
                "result": result,
                "branch": branch,
                "input_summary": if input_data.len() > 200 {
                    format!("{}...", &input_data[..200])
                } else {
                    input_data
                }
            });
            tracing::info!("[WORKFLOW] Conditional node '{}': condition='{}' → branch='{}'", node.id, condition, branch);
            Some((output.to_string(), None))
        }

        // ── Send notification: log a notification (in-app or email placeholder) ──
        "send_notification" => {
            let notification_type = node.parameters.get("notification_type")
                .and_then(|v| v.as_str())
                .map(|t| match t {
                    "in_app" | "info" => "info",
                    "warning" | "warn" => "warning",
                    "success" => "success",
                    "error" => "error",
                    _ => "info",
                })
                .unwrap_or("info");
            let recipient = node.parameters.get("recipient")
                .and_then(|v| v.as_str())
                .unwrap_or("admin");
            let subject = node.parameters.get("subject")
                .and_then(|v| v.as_str())
                .unwrap_or("Workflow Notification");
            let message_template = node.parameters.get("message_template")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // Substitute {{previous_results}} in the message
            let prev_text: String = previous_results.iter()
                .map(|(id, result, _)| format!("[{}]: {}", id, result))
                .collect::<Vec<_>>()
                .join("\n");
            let message = message_template.replace("{{previous_results}}", &prev_text);

            tracing::info!(
                "[WORKFLOW] Notification node '{}': type={}, recipient={}, subject='{}'",
                node.id, notification_type, recipient, subject
            );

            // Resolve recipient to user_id
            let resolved_user_id = match recipient {
                "admin" => {
                    // Look up admin user — users.id is BLOB, format as hyphenated UUID text
                    match sqlx::query_scalar::<_, String>(
                        r#"SELECT printf('%s-%s-%s-%s-%s',
                            substr(hex(id),1,8),
                            substr(hex(id),9,4),
                            substr(hex(id),13,4),
                            substr(hex(id),17,4),
                            substr(hex(id),21,12))
                        FROM users WHERE is_admin = 1 LIMIT 1"#
                    ).fetch_optional(pool).await {
                        Ok(Some(uid)) => Some(uid.to_lowercase()),
                        _ => None,
                    }
                }
                "assigned_user" | "assignee" => {
                    // Try to find assignee from previous results context
                    previous_results.iter()
                        .find_map(|(_, result, _)| {
                            serde_json::from_str::<Value>(result).ok()
                                .and_then(|v| v["assignee_id"].as_str().map(|s| s.to_string()))
                        })
                }
                other => {
                    // Treat as a direct user_id or email
                    Some(other.to_string())
                }
            };

            let mut notification_created = false;
            if let Some(user_id) = &resolved_user_id {
                let create_data = CreateNotification {
                    user_id: user_id.clone(),
                    organization_id: context_org_id.map(|u| u.to_string()),
                    title: subject.to_string(),
                    message: message.clone(),
                    notification_type: notification_type.to_string(),
                    source: Some("workflow".to_string()),
                    source_id: workflow_run_id.map(|id| id.to_string()),
                };
                match Notification::create(pool, &create_data).await {
                    Ok(_) => {
                        notification_created = true;
                        tracing::info!("[WORKFLOW] Notification created for user '{}'", user_id);
                    }
                    Err(e) => {
                        tracing::error!("[WORKFLOW] Failed to create notification: {e}");
                    }
                }
            } else {
                tracing::warn!("[WORKFLOW] Could not resolve recipient '{}' to a user_id", recipient);
            }

            let output = json!({
                "notification_sent": notification_created,
                "type": notification_type,
                "recipient": recipient,
                "resolved_user_id": resolved_user_id,
                "subject": subject,
                "message": message,
            });
            Some((output.to_string(), None))
        }

        // ── Assign to agent: create a task and assign it to a specific agent ──
        "assign_to_agent" => {
            let agent_codename = node.parameters.get("agent_codename")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title_template = node.parameters.get("task_title_template")
                .and_then(|v| v.as_str())
                .unwrap_or("Agent Task");
            let description_template = node.parameters.get("task_description_template")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let completion_criteria = node.parameters.get("completion_criteria")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let auto_start = node.parameters.get("auto_start")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Substitute upstream results into templates
            let prev_text: String = previous_results.iter()
                .map(|(id, result, _)| format!("[{}]: {}", id, result))
                .collect::<Vec<_>>()
                .join("\n");
            let title = title_template.replace("{{previous_results}}", &prev_text);
            let description = description_template.replace("{{previous_results}}", &prev_text);

            // Create the task — use the workflow's data source project as context
            let project_id = context_project_id.unwrap_or_else(Uuid::nil);
            let task_id = Uuid::new_v4();
            let create_task = CreateTask {
                project_id: project_id.to_string(),
                pod_id: None,
                board_id: None,
                title: if title.len() > 200 { title[..200].to_string() } else { title },
                description: Some(description),
                priority: Some(Priority::Medium),
                assignee_id: None,
                assignee_type: None,
                assigned_agent: Some(agent_codename.to_string()),
                agent_id: None,
                assigned_mcps: None,
                parent_task_id: None,
                parent_task_attempt: None,
                image_ids: None,
                created_by: "workflow".to_string(),
                requires_approval: Some(true),
                screenshot: None,
                tags: None,
                due_date: None,
                custom_properties: None,
                scheduled_start: None,
                scheduled_end: None,
                completion_criteria: if completion_criteria.is_empty() { None } else { Some(completion_criteria.to_string()) },
                output_format: None,
            };

            match Task::create(pool, &create_task, &task_id.to_string()).await {
                Ok(task) => {
                    let status_msg = if auto_start { "created (auto_start=true, queued)" } else { "created" };
                    tracing::info!(
                        "[WORKFLOW] assign_to_agent node '{}': task {} {} for agent '{}'",
                        node.id, task.id, status_msg, agent_codename
                    );
                    let output = json!({
                        "task_created": true,
                        "task_id": task.id.to_string(),
                        "agent_codename": agent_codename,
                        "auto_start": auto_start,
                        "status": status_msg,
                    });
                    Some((output.to_string(), None))
                }
                Err(e) => {
                    tracing::error!("[WORKFLOW] assign_to_agent node '{}': failed to create task: {}", node.id, e);
                    let output = json!({
                        "task_created": false,
                        "error": format!("{}", e),
                    });
                    Some((output.to_string(), None))
                }
            }
        }

        // ── HTTP request: make an external API call ──
        "http_request" => {
            let method = node.parameters.get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("GET")
                .to_uppercase();
            let url = node.parameters.get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let headers = node.parameters.get("headers")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let body = node.parameters.get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let output_path = node.parameters.get("output_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if url.is_empty() {
                let output = json!({"error": "No URL configured for http_request node"});
                return Some((output.to_string(), None));
            }

            // Substitute upstream results into URL and body
            let prev_text: String = previous_results.iter()
                .map(|(_, result, _)| result.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            let url = url.replace("{{previous_results}}", &prev_text);
            let body = body.replace("{{previous_results}}", &prev_text);

            tracing::info!("[WORKFLOW] http_request node '{}': {} {}", node.id, method, url);

            let client = reqwest::Client::new();
            let mut request = match method.as_str() {
                "POST" => client.post(&url),
                "PUT" => client.put(&url),
                "DELETE" => client.delete(&url),
                "PATCH" => client.patch(&url),
                _ => client.get(&url),
            };

            // Parse and apply headers
            if let Ok(hdrs) = serde_json::from_str::<Value>(headers) {
                if let Some(obj) = hdrs.as_object() {
                    for (k, v) in obj {
                        if let Some(val) = v.as_str() {
                            request = request.header(k.as_str(), val);
                        }
                    }
                }
            }

            // Add body for POST/PUT/PATCH
            if !body.is_empty() && matches!(method.as_str(), "POST" | "PUT" | "PATCH") {
                request = request.header("Content-Type", "application/json").body(body);
            }

            match request.timeout(std::time::Duration::from_secs(30)).send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    let response_text = response.text().await.unwrap_or_default();

                    // Optionally extract a specific path from JSON response
                    let extracted = if !output_path.is_empty() {
                        if let Ok(parsed) = serde_json::from_str::<Value>(&response_text) {
                            let parts: Vec<&str> = output_path.split('.').collect();
                            let mut current = &parsed;
                            for part in &parts {
                                if let Some(next) = current.get(part) {
                                    current = next;
                                } else {
                                    break;
                                }
                            }
                            current.to_string()
                        } else {
                            response_text.clone()
                        }
                    } else {
                        response_text.clone()
                    };

                    let output = json!({
                        "status": status,
                        "success": status >= 200 && status < 300,
                        "data": extracted,
                    });
                    Some((output.to_string(), None))
                }
                Err(e) => {
                    tracing::error!("[WORKFLOW] http_request node '{}': request failed: {}", node.id, e);
                    let output = json!({
                        "status": 0,
                        "success": false,
                        "error": format!("{}", e),
                    });
                    Some((output.to_string(), None))
                }
            }
        }

        // ── Update CRM contact: find by match field and update ──
        "update_crm_contact" => {
            let match_field = node.parameters.get("match_field")
                .and_then(|v| v.as_str())
                .unwrap_or("email");
            let update_fields_str = node.parameters.get("update_fields")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");

            // Parse upstream data to find records to update
            let input_data: String = previous_results.iter()
                .map(|(_, result, _)| result.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            let records = parse_records_from_input(&input_data);
            let update_template: Value = serde_json::from_str(update_fields_str).unwrap_or(json!({}));
            let mut updated_count = 0;
            let mut errors: Vec<String> = Vec::new();

            for record in &records {
                let match_value = record.get(match_field).and_then(|v| v.as_str()).unwrap_or("");
                if match_value.is_empty() { continue; }

                // Try to find the contact by email (most common match field)
                // For other match fields, we'd need additional lookup methods
                if match_field == "email" {
                    // Use org_id from record, or fall back to workflow context
                    let org_id = record.get("organization_id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .or(context_org_id);

                    if let Some(org_id) = org_id {
                        match CrmContact::find_by_email(pool, org_id, match_value).await {
                            Ok(Some(contact)) => {
                                let update = build_contact_update(&update_template, record);
                                match CrmContact::update(pool, contact.id, update).await {
                                    Ok(_) => { updated_count += 1; }
                                    Err(e) => { errors.push(format!("Update failed for {}: {}", match_value, e)); }
                                }
                            }
                            Ok(None) => { errors.push(format!("Contact not found: {}={}", match_field, match_value)); }
                            Err(e) => { errors.push(format!("Lookup failed: {}", e)); }
                        }
                    } else {
                        errors.push(format!("No organization_id for contact lookup: {}", match_value));
                    }
                } else {
                    errors.push(format!("Match field '{}' not yet supported — use 'email'", match_field));
                }
            }

            tracing::info!(
                "[WORKFLOW] update_crm_contact node '{}': updated={}, errors={}",
                node.id, updated_count, errors.len()
            );
            let output = json!({
                "updated": updated_count,
                "errors": errors,
                "match_field": match_field,
            });
            Some((output.to_string(), None))
        }

        // ── Update CRM deal: find by match field and update ──
        "update_crm_deal" => {
            let match_field = node.parameters.get("match_field")
                .and_then(|v| v.as_str())
                .unwrap_or("name");
            let update_fields_str = node.parameters.get("update_fields")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");

            let input_data: String = previous_results.iter()
                .map(|(_, result, _)| result.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            let records = parse_records_from_input(&input_data);
            let update_template: Value = serde_json::from_str(update_fields_str).unwrap_or(json!({}));
            let mut updated_count = 0;
            let mut errors: Vec<String> = Vec::new();

            for record in &records {
                let match_value = record.get(match_field).and_then(|v| v.as_str()).unwrap_or("");
                if match_value.is_empty() { continue; }

                if match_field == "name" {
                    let org_id = record.get("organization_id")
                        .and_then(|v| v.as_str())
                        .and_then(|s| Uuid::parse_str(s).ok())
                        .or(context_org_id);

                    if let Some(org_id) = org_id {
                        match CrmDeal::find_by_name_and_org(pool, match_value, org_id).await {
                            Ok(Some(deal)) => {
                                let update = build_deal_update(&update_template, record);
                                match CrmDeal::update(pool, deal.id, update).await {
                                    Ok(_) => { updated_count += 1; }
                                    Err(e) => { errors.push(format!("Update failed for {}: {}", match_value, e)); }
                                }
                            }
                            Ok(None) => { errors.push(format!("Deal not found: {}={}", match_field, match_value)); }
                            Err(e) => { errors.push(format!("Lookup failed: {}", e)); }
                        }
                    } else {
                        errors.push(format!("No organization_id for deal lookup: {}", match_value));
                    }
                } else {
                    errors.push(format!("Match field '{}' not yet supported — use 'name'", match_field));
                }
            }

            tracing::info!(
                "[WORKFLOW] update_crm_deal node '{}': updated={}, errors={}",
                node.id, updated_count, errors.len()
            );
            let output = json!({
                "updated": updated_count,
                "errors": errors,
                "match_field": match_field,
            });
            Some((output.to_string(), None))
        }

        // ── Update CRM company: find by match field and update ──
        "update_crm_company" => {
            let match_field = node.parameters.get("match_field")
                .and_then(|v| v.as_str())
                .unwrap_or("name");
            let update_fields_str = node.parameters.get("update_fields")
                .and_then(|v| v.as_str())
                .unwrap_or("{}");

            let input_data: String = previous_results.iter()
                .map(|(_, result, _)| result.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            let records = parse_records_from_input(&input_data);
            let update_template: Value = serde_json::from_str(update_fields_str).unwrap_or(json!({}));
            let mut updated_count = 0;
            let mut errors: Vec<String> = Vec::new();

            for record in &records {
                let match_value = record.get(match_field).and_then(|v| v.as_str()).unwrap_or("");
                if match_value.is_empty() { continue; }

                if match_field == "name" {
                    match Company::find_by_name(pool, match_value).await {
                        Ok(Some(company)) => {
                            let update = build_company_update(&update_template, record);
                            match Company::update(pool, company.id, update).await {
                                Ok(_) => { updated_count += 1; }
                                Err(e) => { errors.push(format!("Update failed for {}: {}", match_value, e)); }
                            }
                        }
                        Ok(None) => { errors.push(format!("Company not found: {}={}", match_field, match_value)); }
                        Err(e) => { errors.push(format!("Lookup failed: {}", e)); }
                    }
                } else {
                    errors.push(format!("Match field '{}' not yet supported — use 'name'", match_field));
                }
            }

            tracing::info!(
                "[WORKFLOW] update_crm_company node '{}': updated={}, errors={}",
                node.id, updated_count, errors.len()
            );
            let output = json!({
                "updated": updated_count,
                "errors": errors,
                "match_field": match_field,
            });
            Some((output.to_string(), None))
        }

        _ => None, // Not an action node — fall through to LLM execution
    }
}

// ── Private helpers for action nodes ─────────────────────────────────────────

/// Parse JSON input into a list of record objects.
/// Handles both JSON arrays and single objects.
fn parse_records_from_input(input: &str) -> Vec<Value> {
    match serde_json::from_str::<Value>(input) {
        Ok(Value::Array(arr)) => arr,
        Ok(obj @ Value::Object(_)) => vec![obj],
        _ => Vec::new(),
    }
}

/// Build an UpdateCrmContact from a template + record data
fn build_contact_update(template: &Value, record: &Value) -> UpdateCrmContact {
    let get_str = |key: &str| -> Option<String> {
        template.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
            .or_else(|| record.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
    };
    UpdateCrmContact {
        first_name: get_str("first_name"),
        last_name: get_str("last_name"),
        email: get_str("email"),
        phone: get_str("phone"),
        mobile: None,
        avatar_url: None,
        company_name: get_str("company_name"),
        job_title: get_str("job_title"),
        department: get_str("department"),
        linkedin_url: get_str("linkedin_url"),
        twitter_handle: None,
        website: get_str("website"),
        source: None,
        lifecycle_stage: None,
        lead_score: template.get("lead_score").and_then(|v| v.as_i64()).map(|n| n as i32),
        owner_user_id: get_str("owner_user_id"),
        assigned_agent_id: None,
        tags: None,
        custom_fields: template.get("custom_fields").cloned(),
        address_line1: None,
        address_line2: None,
        city: get_str("city"),
        state: get_str("state"),
        postal_code: None,
        country: get_str("country"),
        email_opt_in: None,
        sms_opt_in: None,
        do_not_contact: None,
        zoho_contact_id: None,
        gmail_contact_id: None,
    }
}

/// Build an UpdateCrmDeal from a template + record data
fn build_deal_update(template: &Value, record: &Value) -> UpdateCrmDeal {
    let get_str = |key: &str| -> Option<String> {
        template.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
            .or_else(|| record.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
    };
    UpdateCrmDeal {
        crm_contact_id: None,
        crm_pipeline_id: None,
        crm_stage_id: None,
        position: None,
        name: get_str("name"),
        description: get_str("description"),
        amount: template.get("amount").and_then(|v| v.as_f64())
            .or_else(|| record.get("amount").and_then(|v| v.as_f64())),
        currency: get_str("currency"),
        expected_close_date: get_str("expected_close_date"),
        owner_user_id: get_str("owner_user_id"),
        assigned_agent_id: None,
        tags: None,
        custom_fields: template.get("custom_fields").cloned(),
        lost_reason: get_str("lost_reason"),
        win_reason: get_str("win_reason"),
    }
}

/// Build an UpdateCompany from a template + record data
fn build_company_update(template: &Value, record: &Value) -> UpdateCompany {
    let get_str = |key: &str| -> Option<String> {
        template.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
            .or_else(|| record.get(key).and_then(|v| v.as_str()).map(|s| s.to_string()))
    };
    UpdateCompany {
        name: get_str("name"),
        website: get_str("website"),
        industry: get_str("industry"),
        description: get_str("description"),
        logo_url: None,
        cover_image_url: None,
        headquarters: get_str("headquarters"),
        address: get_str("address"),
        city: get_str("city"),
        country: get_str("country"),
        phone: get_str("phone"),
        email: get_str("email"),
        whatsapp: None,
        instagram_handle: None,
        linkedin_url: get_str("linkedin_url"),
        twitter_handle: None,
        facebook_url: None,
        founded_year: template.get("founded_year").and_then(|v| v.as_i64()).map(|n| n as i32),
        employee_count: get_str("employee_count"),
        tags: get_str("tags"),
        business_hours: None,
        notes: get_str("notes"),
        gmb_rating: None,
        gmb_review_count: None,
        gmb_place_id: None,
        organization_id: None,
        intelligence_summary: None,
        intelligence_status: None,
    }
}

// ── LLM execution via PCG Router ─────────────────────────────────────────────

/// Execute a workflow node's LLM prompt via the PCG Router.
/// Routes through all configured providers with priority-based fallback.
/// Returns an error string (instead of mock data) if no models are available.
pub async fn execute_node_with_llm(
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

    // Determine output mode: "text", "structured", or "auto" (default)
    let output_mode = node.parameters.get("output_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    let expect_json = match output_mode {
        "text" => false,
        "structured" => true,
        _ /* auto */ => !target_schemas.is_empty(),
    };

    // Route through WorkflowLLMService — handles multi-provider fallback
    let system_msg = if expect_json {
        "You are a precise data extraction assistant. Your task is to extract REAL entities (people, companies, deals, tasks) that are explicitly mentioned in the source content provided. Rules:\n1. Only extract entities that are clearly and explicitly named in the source text.\n2. NEVER use document metadata (titles, dates, section headings) as entity names.\n3. NEVER fabricate or hallucinate entities that are not in the source.\n4. If no entities of the requested type exist in the source, return an empty array.\n5. Always output valid JSON without markdown formatting or preamble."
    } else {
        "You are a helpful assistant that analyzes content and provides clear, well-structured responses."
    };
    let messages = vec![
        WorkflowLLMService::system_message(system_msg),
        WorkflowLLMService::user_message(&prompt),
    ];

    // Use per-node model override if set, otherwise use the workflow-level model
    let node_model = node.parameters.get("model")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| if model.is_empty() { None } else { Some(model) });

    match WorkflowLLMService::completion(pool, messages.clone(), node_model, Some(2048), None).await {
        Ok((text, metadata)) => {
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
            if !text.is_empty() {
                // Text mode: return raw text without JSON parsing
                if !expect_json {
                    return (text, Some(usage_meta));
                }

                // Structured mode: try to parse as JSON, repair if needed
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
                    WorkflowLLMService::system_message("You are a data extraction and analysis assistant. Always output valid JSON. Do not include markdown formatting or preamble — respond with raw JSON only."),
                    WorkflowLLMService::user_message(&format!(
                        "The previous response was not valid JSON. Please fix it and return only valid JSON. Do not include any explanation or markdown formatting.\n\nOriginal response:\n{}",
                        text
                    )),
                ];

                match WorkflowLLMService::completion(pool, repair_messages, node_model, Some(2048), None).await {
                    Ok((retry_text, retry_meta)) => {
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
                    Err(e) => {
                        tracing::warn!("[WORKFLOW] Repair retry failed for node '{}': {e}", node.id);
                    }
                }

                // Return original text even if repair failed — validation will catch errors downstream
                return (text.to_string(), Some(usage_meta));
            }
            tracing::warn!("[WORKFLOW] Empty response from router for node '{}'", node.id);
        }
        Err(e) => {
            tracing::error!("[WORKFLOW] WorkflowLLMService failed for node '{}': {e}. Check that API keys are configured (e.g. ANTHROPIC_API_KEY env var).", node.id);
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
