use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use db::models::data_source::DataSource;
use db::models::execution_artifact::{ArtifactType, CreateExecutionArtifact, ExecutionArtifact};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utils::response::ApiResponse;
use uuid::Uuid;

use deployment::Deployment;
use crate::{DeploymentImpl, error::ApiError};

// ── Workflow definitions (in-memory, not DB) ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub prompt_template: String,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
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

// ── Default workflow ────────────────────────────────────────────────────────

fn default_analysis_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "default_analysis".to_string(),
        name: "Data Source Analysis".to_string(),
        steps: vec![
            WorkflowStep {
                id: "extract_companies".to_string(),
                name: "Extract Companies".to_string(),
                prompt_template: concat!(
                    "Analyze the following data source content and extract all mentioned ",
                    "companies and organizations. For each company, provide:\n",
                    "- name: The company/organization name\n",
                    "- context: How/where it was mentioned\n",
                    "- relationship: One of potential_client, existing_client, partner, competitor, vendor, other\n\n",
                    "Output as structured JSON with a top-level \"companies\" array.\n\n",
                    "Content:\n{{content}}"
                ).to_string(),
                depends_on: vec![],
            },
            WorkflowStep {
                id: "extract_contacts".to_string(),
                name: "Extract Contacts".to_string(),
                prompt_template: concat!(
                    "Analyze the following data source content and extract all mentioned ",
                    "people and contacts. For each person, provide:\n",
                    "- name: Full name\n",
                    "- role: Their role or title if mentioned\n",
                    "- company: The company they are associated with\n",
                    "- email: Email address if available, null otherwise\n",
                    "- relationship: How they relate to the organization\n\n",
                    "Output as structured JSON with a top-level \"contacts\" array.\n\n",
                    "Content:\n{{content}}"
                ).to_string(),
                depends_on: vec![],
            },
            WorkflowStep {
                id: "identify_opportunities".to_string(),
                name: "Identify Opportunities".to_string(),
                prompt_template: concat!(
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
                ).to_string(),
                depends_on: vec!["extract_companies".to_string(), "extract_contacts".to_string()],
            },
        ],
    }
}

fn available_workflows() -> Vec<WorkflowDefinition> {
    vec![default_analysis_workflow()]
}

// ── Mock LLM: content-aware extraction ──────────────────────────────────────

/// Attempt to extract company-like names from text content.
/// Looks for capitalized multi-word phrases that could be company names.
fn extract_company_names_from_text(text: &str) -> Vec<String> {
    let mut companies = Vec::new();
    // Simple heuristic: find sequences of capitalized words (2+ words)
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        let word = words[i].trim_matches(|c: char| !c.is_alphanumeric());
        if !word.is_empty()
            && word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && word.len() > 1
        {
            // Check if next word is also capitalized to form a company name
            if i + 1 < words.len() {
                let next = words[i + 1].trim_matches(|c: char| !c.is_alphanumeric());
                if !next.is_empty()
                    && next.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    && next.len() > 1
                    // Skip common sentence starters
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
            // Single capitalized word that looks like a proper noun (not common English)
            if word.len() > 3
                && !["About", "After", "Also", "Back", "Because", "Before", "Between",
                     "Both", "Could", "Does", "Down", "Each", "Even", "Find", "First",
                     "From", "Give", "Good", "Great", "Have", "Here", "High", "Into",
                     "Just", "Keep", "Know", "Last", "Life", "Like", "Line", "Long",
                     "Look", "Made", "Make", "Many", "Most", "Much", "Must", "Name",
                     "Never", "Next", "Note", "Only", "Other", "Over", "Part", "Place",
                     "Play", "Point", "Right", "Same", "Should", "Show", "Side", "Since",
                     "Some", "State", "Still", "Such", "Take", "Tell", "Than", "Their",
                     "Them", "Then", "They", "Think", "Through", "Time", "Under", "Upon",
                     "Very", "Want", "Well", "Were", "Will", "With", "Word", "Work",
                     "Would", "Year", "Your", "Content", "Data", "Source", "Step",
                     "Extract", "Output", "None", "True", "False", "Null"].contains(&word)
            {
                // Could be a single-word company name; only add if it appears significant
                // Skip for now to reduce false positives
            }
        }
        i += 1;
    }
    companies
}

/// Attempt to extract person-like names from text content.
fn extract_person_names_from_text(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let common_titles = ["Mr", "Mrs", "Ms", "Dr", "Prof", "CEO", "CTO", "CFO", "COO", "VP"];
    let words: Vec<&str> = text.split_whitespace().collect();

    for i in 0..words.len().saturating_sub(1) {
        let w1 = words[i].trim_matches(|c: char| !c.is_alphanumeric());
        let w2 = words[i + 1].trim_matches(|c: char| !c.is_alphanumeric());

        // Skip if either word is empty or too short
        if w1.len() < 2 || w2.len() < 2 {
            continue;
        }

        let w1_cap = w1.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
        let w2_cap = w2.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

        // Title + Name pattern
        if common_titles.contains(&w1) && w2_cap {
            let name = format!("{} {}", w1, w2);
            if !names.contains(&name) && names.len() < 5 {
                names.push(name);
            }
            continue;
        }

        // Two capitalized words that look like first + last name
        // Heuristic: both words are 2-15 chars, purely alphabetic
        if w1_cap && w2_cap
            && w1.chars().all(|c| c.is_alphabetic())
            && w2.chars().all(|c| c.is_alphabetic())
            && w1.len() <= 15 && w2.len() <= 15
            && w1.len() >= 2 && w2.len() >= 2
            // Filter out common non-name pairs
            && !["The", "This", "That", "These", "Those", "There", "When", "Where",
                 "What", "Which", "Who", "How", "But", "And", "For", "With", "New",
                 "All", "Any", "Our", "Not", "Has", "Its", "May", "Can"].contains(&w1)
        {
            let name = format!("{} {}", w1, w2);
            if !names.contains(&name) && names.len() < 5 {
                names.push(name);
            }
        }
    }
    names
}

/// Generate mock but content-aware extraction results for a workflow step.
fn generate_mock_step_result(
    step_id: &str,
    content: &str,
    title: &str,
    previous_results: &[(&str, &str)],
) -> String {
    let context_hint = if content.is_empty() { title } else { "data source content" };

    match step_id {
        "extract_companies" => {
            let extracted = extract_company_names_from_text(content);
            if extracted.is_empty() {
                // Fallback mock based on title
                json!({
                    "companies": [
                        {
                            "name": format!("Company from '{}'", title),
                            "context": format!("Referenced in {}", context_hint),
                            "relationship": "potential_client"
                        }
                    ]
                })
                .to_string()
            } else {
                let companies: Vec<Value> = extracted
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let relationship = match i % 3 {
                            0 => "potential_client",
                            1 => "partner",
                            _ => "vendor",
                        };
                        json!({
                            "name": name,
                            "context": format!("Mentioned in {}", context_hint),
                            "relationship": relationship
                        })
                    })
                    .collect();
                json!({ "companies": companies }).to_string()
            }
        }
        "extract_contacts" => {
            let extracted = extract_person_names_from_text(content);
            // Try to associate contacts with previously extracted companies
            let company_names: Vec<String> = previous_results
                .iter()
                .filter(|(sid, _)| *sid == "extract_companies")
                .filter_map(|(_, result)| {
                    serde_json::from_str::<Value>(result)
                        .ok()
                        .and_then(|v| v["companies"].as_array().cloned())
                })
                .flatten()
                .filter_map(|c| c["name"].as_str().map(|s| s.to_string()))
                .collect();

            if extracted.is_empty() {
                json!({
                    "contacts": [
                        {
                            "name": "Unknown Contact",
                            "role": "Stakeholder",
                            "company": company_names.first().cloned().unwrap_or_else(|| "Unknown".to_string()),
                            "email": null,
                            "relationship": format!("Referenced in {}", context_hint)
                        }
                    ]
                })
                .to_string()
            } else {
                let contacts: Vec<Value> = extracted
                    .iter()
                    .enumerate()
                    .map(|(i, name)| {
                        let company = company_names
                            .get(i % company_names.len().max(1))
                            .cloned()
                            .unwrap_or_else(|| "Unknown".to_string());
                        let role = match i % 4 {
                            0 => "CEO",
                            1 => "Director",
                            2 => "Manager",
                            _ => "Contact",
                        };
                        json!({
                            "name": name,
                            "role": role,
                            "company": company,
                            "email": null,
                            "relationship": format!("Mentioned in {}", context_hint)
                        })
                    })
                    .collect();
                json!({ "contacts": contacts }).to_string()
            }
        }
        "identify_opportunities" => {
            // Build opportunities based on previous extraction results
            let mut opp_title = format!("Follow-up from '{}'", title);
            let mut opp_type = "project";

            for (sid, result) in previous_results {
                if *sid == "extract_companies" {
                    if let Ok(v) = serde_json::from_str::<Value>(result) {
                        if let Some(companies) = v["companies"].as_array() {
                            if let Some(first) = companies.first() {
                                if let Some(name) = first["name"].as_str() {
                                    opp_title = format!("Engagement with {}", name);
                                    if first["relationship"].as_str() == Some("partner") {
                                        opp_type = "partnership";
                                    }
                                }
                            }
                        }
                    }
                }
            }

            json!({
                "opportunities": [
                    {
                        "title": opp_title,
                        "type": opp_type,
                        "estimated_value": "$25,000 - $75,000",
                        "next_steps": [
                            "Review extracted contacts and companies",
                            "Schedule initial discovery call",
                            "Prepare proposal outline"
                        ]
                    },
                    {
                        "title": format!("Knowledge base entry from '{}'", title),
                        "type": "other",
                        "estimated_value": null,
                        "next_steps": [
                            "Categorize extracted data",
                            "Update CRM records",
                            "Set follow-up reminders"
                        ]
                    }
                ]
            })
            .to_string()
        }
        _ => json!({ "result": "Unknown step" }).to_string(),
    }
}

// ── Route handlers ──────────────────────────────────────────────────────────

/// GET /api/data-sources/:id/workflows
async fn list_workflows(
    Path(data_source_id): Path<Uuid>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<WorkflowDefinition>>>, ApiError> {
    let pool = &deployment.db().pool;

    // Verify data source exists
    DataSource::find_by_id(pool, data_source_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    Ok(Json(ApiResponse::success(available_workflows())))
}

/// POST /api/data-sources/:id/workflows/:workflow_id/run
async fn run_workflow(
    Path((data_source_id, workflow_id)): Path<(Uuid, String)>,
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<WorkflowRunResult>>, ApiError> {
    let pool = &deployment.db().pool;

    // 1. Load data source
    let data_source = DataSource::find_by_id(pool, data_source_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    // 2. Find workflow definition
    let workflow = available_workflows()
        .into_iter()
        .find(|w| w.id == workflow_id)
        .ok_or_else(|| {
            ApiError::NotFound(format!("Workflow '{}' not found", workflow_id))
        })?;

    // 3. Get content from data source
    let content = data_source.content.unwrap_or_default();
    let title = &data_source.title;

    // 4. Execute steps in dependency order
    let mut step_results: Vec<StepResult> = Vec::new();
    let mut step_outputs: Vec<(String, String)> = Vec::new(); // (step_id, output_content)

    for step in &workflow.steps {
        // Gather previous results that this step depends on
        let previous: Vec<(&str, &str)> = step_outputs
            .iter()
            .filter(|(sid, _)| step.depends_on.contains(sid))
            .map(|(sid, out)| (sid.as_str(), out.as_str()))
            .collect();

        // Generate mock result (content-aware)
        let output = generate_mock_step_result(&step.id, &content, title, &previous);

        // Build metadata for this artifact
        let step_index = workflow.steps.iter().position(|s| s.id == step.id).unwrap_or(0);
        let artifact_metadata = json!({
            "data_source_id": data_source_id.to_string(),
            "workflow_id": workflow.id,
            "step_id": step.id,
            "step_index": step_index,
        });

        // Create artifact
        let artifact = ExecutionArtifact::create(
            pool,
            CreateExecutionArtifact {
                execution_process_id: None,
                artifact_type: ArtifactType::ResearchReport,
                title: format!("{} - {}", workflow.name, step.name),
                content: Some(output.clone()),
                file_path: None,
                metadata: Some(artifact_metadata),
            },
        )
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to create artifact: {e}")))?;

        step_results.push(StepResult {
            step_id: step.id.clone(),
            step_name: step.name.clone(),
            artifact_id: artifact.id,
        });

        step_outputs.push((step.id.clone(), output));
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

    // Verify data source exists
    DataSource::find_by_id(pool, data_source_id)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to look up data source: {e}")))?
        .ok_or_else(|| ApiError::NotFound("Data source not found".to_string()))?;

    // Query artifacts whose metadata JSON contains this data_source_id
    let ds_id_str = data_source_id.to_string();
    let artifacts = sqlx::query_as::<_, ExecutionArtifact>(
        r#"
        SELECT * FROM execution_artifacts
        WHERE json_extract(metadata, '$.data_source_id') = ?1
        ORDER BY created_at ASC
        "#,
    )
    .bind(&ds_id_str)
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to query artifacts: {e}")))?;

    Ok(Json(ApiResponse::success(artifacts)))
}

/// GET /api/workflows/definitions — list all available workflow definitions
async fn list_all_workflow_definitions() -> Json<ApiResponse<Vec<WorkflowDefinition>>> {
    Json(ApiResponse::success(available_workflows()))
}

/// GET /api/artifacts/recent — list recent workflow-generated artifacts
async fn list_recent_artifacts(
    State(deployment): State<DeploymentImpl>,
) -> Result<Json<ApiResponse<Vec<ExecutionArtifact>>>, ApiError> {
    let pool = &deployment.db().pool;

    let artifacts = sqlx::query_as::<_, ExecutionArtifact>(
        r#"
        SELECT * FROM execution_artifacts
        WHERE json_extract(metadata, '$.workflow_id') IS NOT NULL
        ORDER BY created_at DESC
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| ApiError::InternalError(format!("Failed to query artifacts: {e}")))?;

    Ok(Json(ApiResponse::success(artifacts)))
}

// ── Router ──────────────────────────────────────────────────────────────────

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route(
            "/data-sources/{id}/workflows",
            get(list_workflows),
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
            get(list_all_workflow_definitions),
        )
        .route(
            "/artifacts/recent",
            get(list_recent_artifacts),
        )
}
