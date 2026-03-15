//! Workflow Builder Specialist Agent
//!
//! A dedicated specialist that generates workflow node graphs from natural language.
//! Called by Topsi (orchestrator) via the `build_workflow` tool — keeps Topsi's
//! system prompt clean while giving the builder full context about node types,
//! output schemas, and connection patterns.
//!
//! The builder queries the database directly for any information it needs
//! (existing workflows, output schemas, etc.) rather than relying on Topsi
//! to pre-fetch everything.

use nora::brain::LLMClient;
use serde_json::{json, Value};
use sqlx::SqlitePool;

/// System prompt for the Workflow Builder specialist.
/// Contains the full node type registry and output schema reference.
const BUILDER_SYSTEM_PROMPT: &str = r#"You are the ORCHA Workflow Builder — a specialist agent that generates workflow node graphs from natural language descriptions.

## Your Output Format

You MUST respond with a single valid JSON object containing:
```json
{
  "name": "Workflow Name",
  "description": "What the workflow does",
  "nodes": [ ... ],
  "connections": [ ... ],
  "default_model": null
}
```

Do NOT include any text outside the JSON. No markdown fences, no explanations — just the JSON object.

## Node Types Reference

### LLM Nodes (process data with AI)
- **llm_extract**: Extract structured data from text. Best for pulling entities (contacts, companies, deals) from unstructured content.
  - Parameters: `prompt_template` (string), `output_schema` (string, e.g. "contacts[]"), `output_mode` ("structured"|"text"|"auto")
- **llm_analyze**: Analyze, classify, or evaluate data. Best for sentiment analysis, categorization, scoring.
  - Parameters: `prompt_template` (string), `output_schema` (string), `output_mode` ("structured"|"text"|"auto")
- **llm_summarize**: Summarize or condense text. Best for creating reports, briefs, summaries.
  - Parameters: `prompt_template` (string), `output_schema` (string), `output_mode` ("text"|"structured"|"auto")

### Output Nodes (send results to CRM/tasks staging)
- **output_crm_contacts**: Stage extracted contacts for CRM import. Connect from an LLM node that outputs contact data.
- **output_crm_companies**: Stage extracted companies for CRM import.
- **output_crm_deals**: Stage extracted deals for CRM import.
- **output_tasks**: Stage extracted tasks for project task board import.

Output nodes have no parameters — they pass through upstream data to the staging system for human review before committing.

### Action Nodes (side effects)
- **conditional**: Branch execution based on upstream data.
  - Parameters: `condition` (string — e.g. "count>0", "contains:keyword"), `true_label`, `false_label`
- **send_notification**: Send a notification.
  - Parameters: `channel` ("email"|"slack"|"in_app"), `message_template` (string with {{placeholders}})
- **assign_to_agent**: Assign work to an AI agent.
  - Parameters: `agent_type` (string), `task_name` (string), `instructions` (string)
- **http_request**: Make an HTTP API call.
  - Parameters: `url` (string), `method` ("GET"|"POST"|"PUT"), `headers` (object), `body_template` (string)

## Output Schemas

When an LLM node feeds into an output node, set `output_schema` to tell the LLM what structure to produce:

- `contacts[]` → array of `{first_name, last_name, email, phone, company_name, job_title, linkedin_url}`
- `companies[]` → array of `{name, industry, website, description, employee_count, headquarters}`
- `crm_deals[]` → array of `{name, amount, currency, pipeline_name, stage_name, expected_close_date, contact_email}`
- `tasks[]` → array of `{title, description, status, priority, tags, due_date}`

For nodes NOT feeding into output nodes, use descriptive schema names like `"summary"`, `"analysis"`, `"report"`.

## Connection Rules

- Connections flow from `source` (upstream) to `target` (downstream)
- A node can have multiple inputs (data is merged) and multiple outputs
- LLM nodes typically connect to output nodes or other LLM nodes
- Output nodes are always terminal (no outgoing connections)
- First node(s) in the graph receive the data source content directly

## Node ID Convention

Use descriptive snake_case IDs: `extract_contacts`, `analyze_sentiment`, `output_deals`, etc.

## Position Convention

Arrange nodes left-to-right: first nodes at x=100, second column at x=500, third at x=900. Vertical spacing: y starts at 100, increment by 200 for parallel nodes.

## Examples

**"Extract contacts and companies from meeting notes"**
→ Two parallel LLM extract nodes → two output nodes

**"Analyze customer feedback, extract action items, and notify the team"**
→ LLM analyze → LLM extract (tasks) → output_tasks + send_notification
"#;

/// Build a workflow from a natural language description.
///
/// The builder gathers its own context from the database — Topsi only needs
/// to pass the user's request and scoping info (org/workflow IDs).
pub async fn build_workflow(
    llm: &LLMClient,
    pool: &SqlitePool,
    action: &str,
    user_request: &str,
    context: &str,
    workflow_id: Option<&str>,
    owner_id: Option<&str>,
) -> Result<Value, String> {
    // Build context by querying the database directly
    let mut builder_context = String::new();

    // Always include any orchestrator-provided context (org/project scope, user intent)
    if !context.is_empty() {
        builder_context.push_str("## Orchestrator Context\n");
        builder_context.push_str(context);
        builder_context.push_str("\n\n");
    }

    // For modify actions, load the existing workflow
    if action == "modify" {
        let wf_id = workflow_id
            .ok_or("workflow_id is required for modify action")?;
        match load_existing_workflow(pool, wf_id).await {
            Some(wf) => {
                builder_context.push_str("## Existing Workflow to Modify\n```json\n");
                builder_context.push_str(
                    &serde_json::to_string_pretty(&wf).unwrap_or_default()
                );
                builder_context.push_str("\n```\n\nModify this workflow according to the user's request. Return the complete updated workflow (not just the diff).\n\n");
            }
            None => return Err(format!("Workflow '{}' not found", wf_id)),
        }
    }

    // List existing workflows so the builder knows what already exists
    let existing_workflows = list_workflow_summaries(pool).await;
    if !existing_workflows.is_empty() {
        builder_context.push_str("## Existing Workflows in System\n");
        for (id, name, desc) in &existing_workflows {
            builder_context.push_str(&format!("- `{}`: {} {}\n", id, name,
                desc.as_deref().map(|d| format!("— {}", d)).unwrap_or_default()));
        }
        builder_context.push('\n');
    }

    // Call the LLM with the builder system prompt
    let response = llm
        .generate(BUILDER_SYSTEM_PROMPT, user_request, &builder_context)
        .await
        .map_err(|e| format!("LLM generation failed: {}", e))?;

    // Parse the response as JSON
    let workflow_json = parse_workflow_json(&response)?;

    // Validate the workflow structure
    validate_workflow(&workflow_json)?;

    // Determine the workflow ID
    let wf_id = if action == "modify" {
        workflow_id.unwrap_or("").to_string()
    } else {
        let name = workflow_json["name"].as_str().unwrap_or("unnamed");
        slug_from_name(name)
    };

    // Save to database
    let name = workflow_json["name"].as_str().unwrap_or("Unnamed Workflow").to_string();
    let description = workflow_json["description"].as_str().map(|s| s.to_string());
    let default_model = workflow_json["default_model"].as_str().map(|s| s.to_string());

    let steps_data = json!({
        "nodes": workflow_json["nodes"],
        "connections": workflow_json["connections"],
        "default_model": default_model,
    });
    let steps_str = steps_data.to_string();

    if action == "modify" {
        sqlx::query(
            "UPDATE workflow_definitions SET name = ?1, description = ?2, steps = ?3, updated_at = datetime('now', 'subsec') WHERE id = ?4"
        )
        .bind(&name)
        .bind(&description)
        .bind(&steps_str)
        .bind(&wf_id)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to update workflow: {}", e))?;
    } else {
        // Check for ID collision
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM workflow_definitions WHERE id = ?1"
        )
        .bind(&wf_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if existing > 0 {
            return Err(format!("Workflow with ID '{}' already exists. Use modify action to update it.", wf_id));
        }

        sqlx::query(
            "INSERT INTO workflow_definitions (id, owner_type, owner_id, name, description, steps, is_system) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)"
        )
        .bind(&wf_id)
        .bind("organization")
        .bind(owner_id)
        .bind(&name)
        .bind(&description)
        .bind(&steps_str)
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to create workflow: {}", e))?;
    }

    Ok(json!({
        "workflow_id": wf_id,
        "name": name,
        "description": description,
        "node_count": workflow_json["nodes"].as_array().map(|a| a.len()).unwrap_or(0),
        "connection_count": workflow_json["connections"].as_array().map(|a| a.len()).unwrap_or(0),
        "action": action,
    }))
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Parse the LLM's response, extracting JSON from potential markdown fences.
fn parse_workflow_json(response: &str) -> Result<Value, String> {
    let trimmed = response.trim();

    // Try direct parse first
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return Ok(v);
    }

    // Try extracting from markdown code fences
    let json_str = if let Some(start) = trimmed.find("```json") {
        let after_fence = &trimmed[start + 7..];
        if let Some(end) = after_fence.find("```") {
            after_fence[..end].trim()
        } else {
            after_fence.trim()
        }
    } else if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        if let Some(end) = after_fence.find("```") {
            after_fence[..end].trim()
        } else {
            after_fence.trim()
        }
    } else {
        trimmed
    };

    serde_json::from_str::<Value>(json_str)
        .map_err(|e| format!(
            "Failed to parse workflow JSON from LLM response: {}. Response start: {}",
            e, &trimmed[..trimmed.len().min(200)]
        ))
}

/// Validate the workflow structure has required fields and valid node types.
fn validate_workflow(wf: &Value) -> Result<(), String> {
    let nodes = wf["nodes"].as_array()
        .ok_or("Workflow must have a 'nodes' array")?;

    if nodes.is_empty() {
        return Err("Workflow must have at least one node".to_string());
    }

    let valid_types = [
        "llm_extract", "llm_analyze", "llm_summarize",
        "output_crm_contacts", "output_crm_companies", "output_crm_deals", "output_tasks",
        "conditional", "send_notification", "assign_to_agent", "http_request",
    ];

    for node in nodes {
        let node_type = node["type"].as_str()
            .or_else(|| node["node_type"].as_str())
            .ok_or_else(|| format!("Node {:?} missing 'type' field", node["id"]))?;

        if !valid_types.contains(&node_type) {
            return Err(format!(
                "Invalid node type '{}'. Valid types: {:?}", node_type, valid_types
            ));
        }

        if node["id"].as_str().is_none() {
            return Err("Each node must have an 'id' field".to_string());
        }
        if node["name"].as_str().is_none() {
            return Err(format!("Node '{}' missing 'name' field", node["id"]));
        }
    }

    // Validate connections reference existing nodes
    if let Some(conns) = wf["connections"].as_array() {
        let node_ids: Vec<&str> = nodes.iter()
            .filter_map(|n| n["id"].as_str())
            .collect();

        for conn in conns {
            let source = conn["source"].as_str()
                .ok_or("Connection missing 'source' field")?;
            let target = conn["target"].as_str()
                .ok_or("Connection missing 'target' field")?;

            if !node_ids.contains(&source) {
                return Err(format!("Connection source '{}' references non-existent node", source));
            }
            if !node_ids.contains(&target) {
                return Err(format!("Connection target '{}' references non-existent node", target));
            }
        }
    }

    Ok(())
}

/// Generate a URL-safe slug from a workflow name.
fn slug_from_name(name: &str) -> String {
    let slug: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let mut result = String::new();
    let mut prev_underscore = false;
    for c in slug.chars() {
        if c == '_' {
            if !prev_underscore && !result.is_empty() {
                result.push('_');
            }
            prev_underscore = true;
        } else {
            result.push(c);
            prev_underscore = false;
        }
    }
    result.trim_end_matches('_').to_string()
}

/// Load an existing workflow definition from the database as JSON.
async fn load_existing_workflow(pool: &SqlitePool, workflow_id: &str) -> Option<Value> {
    let row = sqlx::query_as::<_, (String, Option<String>, String)>(
        "SELECT name, description, steps FROM workflow_definitions WHERE id = ?1"
    )
    .bind(workflow_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    let (name, description, steps_str) = row;
    let steps: Value = serde_json::from_str(&steps_str).unwrap_or(json!({}));

    Some(json!({
        "name": name,
        "description": description,
        "nodes": steps["nodes"],
        "connections": steps["connections"],
        "default_model": steps["default_model"],
    }))
}

/// List all workflow definitions as (id, name, description) summaries.
async fn list_workflow_summaries(pool: &SqlitePool) -> Vec<(String, String, Option<String>)> {
    sqlx::query_as::<_, (String, String, Option<String>)>(
        "SELECT id, name, description FROM workflow_definitions ORDER BY is_system DESC, name ASC"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}
