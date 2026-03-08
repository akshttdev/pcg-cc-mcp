use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post, put, delete},
};
use db::models::data_source::DataSource;
use db::models::execution_artifact::{ArtifactType, CreateExecutionArtifact, ExecutionArtifact};
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
}

#[derive(Debug, Deserialize)]
struct UpdateWorkflowRequest {
    name: Option<String>,
    description: Option<String>,
    nodes: Option<Vec<WorkflowNode>>,
    connections: Option<Vec<WorkflowConnection>>,
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
    workflow_id: String,
    workflow_name: String,
    data_source_id: Uuid,
    steps: Vec<StepResult>,
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
                        "- name: Full name\n",
                        "- role: Their role or title if mentioned\n",
                        "- company: The company they are associated with\n",
                        "- email: Email address if available, null otherwise\n",
                        "- relationship: How they relate to the organization\n\n",
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
                        "- title: Short descriptive title\n",
                        "- type: One of project, partnership, upsell, referral, other\n",
                        "- estimated_value: Rough estimate if possible, or null\n",
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
    }
}

// ── DB helpers ───────────────────────────────────────────────────────────────

/// Serialize workflow to DB JSON columns
fn serialize_workflow_data(wf: &WorkflowDefinition) -> (String, String) {
    let nodes_json = serde_json::to_string(&wf.nodes).unwrap_or_else(|_| "[]".to_string());
    let connections_json = serde_json::to_string(&wf.connections).unwrap_or_else(|_| "[]".to_string());
    (nodes_json, connections_json)
}

async fn seed_defaults(pool: &sqlx::SqlitePool) {
    let default = default_analysis_workflow();
    let (nodes_json, connections_json) = serialize_workflow_data(&default);
    let desc = default.description.unwrap_or_default();

    let _ = sqlx::query(
        r#"INSERT OR IGNORE INTO workflow_definitions (id, owner_type, name, description, steps, is_system)
           VALUES (?1, 'system', ?2, ?3, ?4, 1)"#,
    )
    .bind(&default.id)
    .bind(&default.name)
    .bind(&desc)
    .bind(format!("{{\"nodes\":{},\"connections\":{}}}", nodes_json, connections_json))
    .execute(pool)
    .await;
}

fn parse_workflow_from_row(id: String, owner_type: String, owner_id: Option<String>, name: String, description: Option<String>, steps_json: String, is_system: bool) -> WorkflowDefinition {
    // Try new format: {"nodes": [...], "connections": [...]}
    if let Ok(v) = serde_json::from_str::<Value>(&steps_json) {
        if v.get("nodes").is_some() {
            let nodes: Vec<WorkflowNode> = serde_json::from_value(v["nodes"].clone()).unwrap_or_default();
            let connections: Vec<WorkflowConnection> = serde_json::from_value(v["connections"].clone()).unwrap_or_default();
            return WorkflowDefinition { id, name, description, nodes, connections, is_system, owner_type, owner_id };
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

    WorkflowDefinition { id, name, description, nodes, connections, is_system, owner_type, owner_id }
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

fn extract_company_names_from_text(text: &str) -> Vec<String> {
    let mut companies = Vec::new();
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let word = words[i].trim_matches(|c: char| !c.is_alphanumeric());
        if !word.is_empty()
            && word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && word.len() > 1
        {
            if i + 1 < words.len() {
                let next = words[i + 1].trim_matches(|c: char| !c.is_alphanumeric());
                if !next.is_empty()
                    && next.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && next.len() > 1
                    && !["The", "This", "That", "These", "Those", "There", "When", "Where",
                         "What", "Which", "Who", "How", "But", "And", "For", "With"].contains(&word)
                {
                    let name = format!("{} {}", word, next);
                    if !companies.contains(&name) && companies.len() < 5 {
                        companies.push(name);
                    }
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }
    companies
}

fn extract_person_names_from_text(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let common_titles = ["Mr", "Mrs", "Ms", "Dr", "Prof", "CEO", "CTO", "CFO", "COO", "VP"];
    let words: Vec<&str> = text.split_whitespace().collect();
    for i in 0..words.len().saturating_sub(1) {
        let w1 = words[i].trim_matches(|c: char| !c.is_alphanumeric());
        let w2 = words[i + 1].trim_matches(|c: char| !c.is_alphanumeric());
        if w1.len() < 2 || w2.len() < 2 { continue; }
        let w1_cap = w1.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        let w2_cap = w2.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        if common_titles.contains(&w1) && w2_cap {
            let name = format!("{} {}", w1, w2);
            if !names.contains(&name) && names.len() < 5 { names.push(name); }
            continue;
        }
        if w1_cap && w2_cap
            && w1.chars().all(|c| c.is_alphabetic()) && w2.chars().all(|c| c.is_alphabetic())
            && w1.len() <= 15 && w2.len() <= 15 && w1.len() >= 2 && w2.len() >= 2
            && !["The", "This", "That", "These", "Those", "There", "When", "Where",
                 "What", "Which", "Who", "How", "But", "And", "For", "With", "New",
                 "All", "Any", "Our", "Not", "Has", "Its", "May", "Can"].contains(&w1)
        {
            let name = format!("{} {}", w1, w2);
            if !names.contains(&name) && names.len() < 5 { names.push(name); }
        }
    }
    names
}

fn generate_mock_step_result(step_id: &str, content: &str, title: &str, previous_results: &[(&str, &str)], node_type: &str, output_schema: &str) -> String {
    let context_hint = if content.is_empty() { title } else { "data source content" };
    // Match on exact step_id first (system workflows), then use output_schema to determine mock data
    let key = match step_id {
        "extract_companies" | "extract_contacts" | "identify_opportunities" => step_id.to_string(),
        _ => {
            // For custom workflows, use output_schema to pick the right mock
            let schema_lower = output_schema.to_lowercase();
            if schema_lower.contains("compan") { "extract_companies".to_string() }
            else if schema_lower.contains("contact") || schema_lower.contains("person") || schema_lower.contains("people") { "extract_contacts".to_string() }
            else if schema_lower.contains("opportunit") || node_type == "llm_analyze" { "identify_opportunities".to_string() }
            else { step_id.to_string() }
        }
    };
    match key.as_str() {
        "extract_companies" => {
            let extracted = extract_company_names_from_text(content);
            if extracted.is_empty() {
                json!({"companies": [{"name": format!("Company from '{}'", title), "context": format!("Referenced in {}", context_hint), "relationship": "potential_client"}]}).to_string()
            } else {
                let companies: Vec<Value> = extracted.iter().enumerate().map(|(i, name)| {
                    let rel = match i % 3 { 0 => "potential_client", 1 => "partner", _ => "vendor" };
                    json!({"name": name, "context": format!("Mentioned in {}", context_hint), "relationship": rel})
                }).collect();
                json!({ "companies": companies }).to_string()
            }
        }
        "extract_contacts" => {
            let extracted = extract_person_names_from_text(content);
            let company_names: Vec<String> = previous_results.iter()
                .filter(|(sid, _)| *sid == "extract_companies")
                .filter_map(|(_, result)| serde_json::from_str::<Value>(result).ok().and_then(|v| v["companies"].as_array().cloned()))
                .flatten().filter_map(|c| c["name"].as_str().map(|s| s.to_string())).collect();
            if extracted.is_empty() {
                json!({"contacts": [{"name": "Unknown Contact", "role": "Stakeholder", "company": company_names.first().cloned().unwrap_or_else(|| "Unknown".to_string()), "email": null, "relationship": format!("Referenced in {}", context_hint)}]}).to_string()
            } else {
                let contacts: Vec<Value> = extracted.iter().enumerate().map(|(i, name)| {
                    let company = company_names.get(i % company_names.len().max(1)).cloned().unwrap_or_else(|| "Unknown".to_string());
                    let role = match i % 4 { 0 => "CEO", 1 => "Director", 2 => "Manager", _ => "Contact" };
                    json!({"name": name, "role": role, "company": company, "email": null, "relationship": format!("Mentioned in {}", context_hint)})
                }).collect();
                json!({ "contacts": contacts }).to_string()
            }
        }
        "identify_opportunities" => {
            let mut opp_title = format!("Follow-up from '{}'", title);
            let mut opp_type = "project";
            for (sid, result) in previous_results {
                if *sid == "extract_companies" {
                    if let Ok(v) = serde_json::from_str::<Value>(result) {
                        if let Some(companies) = v["companies"].as_array() {
                            if let Some(first) = companies.first() {
                                if let Some(name) = first["name"].as_str() {
                                    opp_title = format!("Engagement with {}", name);
                                    if first["relationship"].as_str() == Some("partner") { opp_type = "partnership"; }
                                }
                            }
                        }
                    }
                }
            }
            json!({"opportunities": [
                {"title": opp_title, "type": opp_type, "estimated_value": "$25,000 - $75,000", "next_steps": ["Review extracted contacts and companies", "Schedule initial discovery call", "Prepare proposal outline"]},
                {"title": format!("Knowledge base entry from '{}'", title), "type": "other", "estimated_value": null, "next_steps": ["Categorize extracted data", "Update CRM records", "Set follow-up reminders"]}
            ]}).to_string()
        }
        _ => {
            json!({"result": format!("Processed step '{}' on content ({} chars)", step_id, content.len()), "source": title, "status": "completed"}).to_string()
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
}

/// POST /api/data-sources/:id/workflows/:workflow_id/run
async fn run_workflow(
    Path((data_source_id, workflow_id)): Path<(Uuid, String)>,
    State(deployment): State<DeploymentImpl>,
    body: Option<Json<RunWorkflowRequest>>,
) -> Result<Json<ApiResponse<WorkflowRunResult>>, ApiError> {
    let model = body.and_then(|b| b.0.model).unwrap_or_default(); // empty = use router's highest priority
    let pool = &deployment.db().pool;

    let data_source = DataSource::find_by_id(pool, data_source_id)
        .await.map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    let workflow = load_workflow(pool, &workflow_id).await
        .map_err(|e| ApiError::InternalError(format!("Failed to load workflow: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("Workflow '{}' not found", workflow_id)))?;

    let content = data_source.content.unwrap_or_default();
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

    let mut step_results: Vec<StepResult> = Vec::new();
    let mut step_outputs: Vec<(String, String)> = Vec::new();

    for node in &ordered_nodes {
        let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
        let previous: Vec<(&str, &str)> = step_outputs.iter()
            .filter(|(sid, _)| deps.contains(sid))
            .map(|(sid, out)| (sid.as_str(), out.as_str()))
            .collect();

        let output = execute_node_with_llm(pool, node, &content, &previous, &model).await;

        let step_index = ordered_nodes.iter().position(|n| n.id == node.id).unwrap_or(0);
        let artifact_metadata = json!({
            "data_source_id": data_source_id.to_string(),
            "workflow_id": workflow.id,
            "step_id": node.id,
            "step_index": step_index,
        });

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
        step_outputs.push((node.id.clone(), output));
    }

    Ok(Json(ApiResponse::success(WorkflowRunResult {
        workflow_id: workflow.id,
        workflow_name: workflow.name,
        data_source_id,
        steps: step_results,
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
    };
    let (nodes_json, connections_json) = serialize_workflow_data(&wf);
    let data = format!("{{\"nodes\":{},\"connections\":{}}}", nodes_json, connections_json);

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

    let wf = WorkflowDefinition {
        id: workflow_id.clone(), name: name.clone(), description: description.clone(),
        nodes: nodes.clone(), connections: connections.clone(), is_system: existing.is_system,
        owner_type: existing.owner_type.clone(), owner_id: existing.owner_id.clone(),
    };
    let (nodes_json, connections_json) = serialize_workflow_data(&wf);
    let data = format!("{{\"nodes\":{},\"connections\":{}}}", nodes_json, connections_json);

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

/// GET /api/artifacts/recent
async fn list_recent_artifacts(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ExecutionArtifact>>>, ApiError> {
    let pool = &deployment.db().pool;
    let artifacts = sqlx::query_as::<_, ExecutionArtifact>(
        r#"SELECT * FROM execution_artifacts WHERE json_extract(metadata, '$.workflow_id') IS NOT NULL ORDER BY created_at DESC LIMIT 100"#,
    ).fetch_all(pool).await
    .map_err(|e| ApiError::InternalError(format!("Failed to query artifacts: {e}")))?;

    Ok(Json(ApiResponse::success(artifacts)))
}

// ── LLM execution via PCG Router (falls back to mock) ────────────────────────

/// Execute a workflow node's LLM prompt via the PCG Router.
/// Routes through all configured providers with priority-based fallback.
/// Falls back to mock extraction if no models are available.
async fn execute_node_with_llm(
    pool: &sqlx::SqlitePool,
    node: &WorkflowNode,
    content: &str,
    previous_results: &[(&str, &str)],
    model: &str,
) -> String {
    let prompt_template = node.parameters.get("prompt_template")
        .and_then(|v| v.as_str())
        .unwrap_or("Analyze the following content:\n{{content}}");

    // Build the actual prompt by substituting template variables
    let prev_text = previous_results.iter()
        .map(|(id, result)| format!("[{}]: {}", id, result))
        .collect::<Vec<_>>()
        .join("\n\n");

    let prompt = prompt_template
        .replace("{{content}}", content)
        .replace("{{previous_results}}", &prev_text);

    // Route through PCG Router — handles multi-provider fallback
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: json!("You are a data extraction and analysis assistant. Always output valid JSON. Do not include markdown formatting or preamble — respond with raw JSON only."),
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

    match pcg_router::route_completion(pool, messages, node_model, Some(2048), None).await {
        Ok((resp, metadata)) => {
            tracing::info!(
                "[WORKFLOW] Node '{}' routed via {} ({}), tokens: {:?}/{:?}",
                node.id, metadata.model_used, metadata.provider,
                metadata.input_tokens, metadata.output_tokens
            );
            // Extract text from OpenAI-format response
            if let Some(text) = resp["choices"][0]["message"]["content"].as_str() {
                if !text.is_empty() {
                    return text.to_string();
                }
            }
            tracing::warn!("[WORKFLOW] Empty response from router for node '{}'", node.id);
        }
        Err(e) => {
            tracing::warn!("[WORKFLOW] PCG Router failed for node '{}': {e}, falling back to mock", node.id);
        }
    }

    // Fall back to mock
    let output_schema = node.parameters.get("output_schema")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    generate_mock_step_result(&node.id, content, "preview", previous_results, &node.node_type, output_schema)
}

// ── Preview (dry-run) endpoint ───────────────────────────────────────────────

/// Response for preview node result
#[derive(Debug, Serialize)]
struct PreviewNodeResult {
    node_id: String,
    node_name: String,
    node_type: String,
    output: String,
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
    let mut outputs: Vec<(String, String)> = Vec::new();

    for node in &ordered {
        let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
        let previous: Vec<(&str, &str)> = outputs.iter()
            .filter(|(sid, _)| deps.contains(sid))
            .map(|(sid, out)| (sid.as_str(), out.as_str()))
            .collect();

        let output = execute_node_with_llm(pool, node, &content, &previous, "").await;

        results.push(PreviewNodeResult {
            node_id: node.id.clone(),
            node_name: node.name.clone(),
            node_type: node.node_type.clone(),
            output: output.clone(),
        });
        outputs.push((node.id.clone(), output));
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
        .route("/artifacts/recent", get(list_recent_artifacts))
}
