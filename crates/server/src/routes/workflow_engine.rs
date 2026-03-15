//! Workflow Execution Engine
//!
//! Unified execution logic for workflow node graphs. Eliminates duplication across
//! run_workflow, preview_workflow, fire_triggers, and schedule triggers.
//!
//! This module owns:
//! - Topological sort of workflow nodes
//! - Node execution dispatch (LLM, action, output pass-through)
//! - Staging record creation with dedup/validation
//! - Usage aggregation
//!
//! Route handlers and trigger functions call into this shared engine.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value};
use sqlx::SqlitePool;
use uuid::Uuid;

use db::models::execution_artifact::{ArtifactType, CreateExecutionArtifact, ExecutionArtifact};
use db::models::workflow_run::{WorkflowRun, UpdateWorkflowRunOnComplete};
use db::models::workflow_staging::{WorkflowStagingRecord, CreateStagingRecord};

/// Maximum number of records that can be auto-approved in a single workflow run.
/// Acts as a safety guard against runaway auto-approve on large batches.
pub const MAX_AUTO_APPROVE_RECORDS: i64 = 50;

use services::services::workflow_execution::{
    WorkflowDefinition, WorkflowNode,
    execute_node_with_llm, execute_action_node,
    extract_records_from_output, check_contact_duplicate, check_company_duplicate,
    check_deal_duplicate, check_task_duplicate, check_intra_batch_duplicate,
    validate_record_against_schema, is_fallback_placeholder, compute_confidence,
};

/// Result of executing a single node
pub struct NodeResult {
    pub node_id: String,
    pub node_name: String,
    pub node_type: String,
    pub output: String,
    pub schema_name: String,
    pub usage: Option<Value>,
}

/// Result of a full workflow execution
pub struct WorkflowExecutionResult {
    pub node_results: Vec<NodeResult>,
    pub total_usage: Option<Value>,
    pub staged_records: i64,
    pub duplicates_found: i64,
}

/// Options for workflow execution
pub struct ExecutionOptions {
    pub model: String,
    pub data_source_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub project_id: Option<Uuid>,
    pub workflow_run_id: Option<Uuid>,
    /// If true, create execution artifacts for each node
    pub create_artifacts: bool,
    /// If true, create staging records for output nodes
    pub create_staging: bool,
    /// Extra metadata to add to artifacts (e.g. trigger_id)
    pub artifact_metadata_extra: Option<Value>,
    /// If true, auto-approve valid staged records after execution completes.
    /// Only effective when `create_staging` is also true.
    pub auto_approve: bool,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            model: String::new(),
            data_source_id: None,
            organization_id: None,
            project_id: None,
            workflow_run_id: None,
            create_artifacts: true,
            create_staging: true,
            artifact_metadata_extra: None,
            auto_approve: false,
        }
    }
}

/// Execute a workflow's node graph in topological order.
///
/// This is the core execution engine shared by all workflow entry points:
/// - `run_workflow` (HTTP handler)
/// - `preview_workflow` (HTTP handler, create_artifacts=false, create_staging=false)
/// - `fire_triggers_for_data_source` (background task)
/// - `spawn_workflow_schedule_loop` (background task)
pub async fn execute_workflow_nodes(
    pool: &SqlitePool,
    workflow: &WorkflowDefinition,
    content: &str,
    opts: &ExecutionOptions,
) -> WorkflowExecutionResult {
    // Build dependency map from connections
    let mut deps_map: HashMap<String, Vec<String>> = HashMap::new();
    for conn in &workflow.connections {
        deps_map.entry(conn.target.clone()).or_default().push(conn.source.clone());
    }

    // Topological sort: process nodes in dependency order
    let mut processed: HashSet<String> = HashSet::new();
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
            // Circular dependency — process remaining in order
            for node in &remaining {
                ordered_nodes.push(node);
            }
            break;
        }
    }

    // Build downstream output target map: node_id → list of target types
    let mut downstream_targets: HashMap<String, Vec<String>> = HashMap::new();
    for conn in &workflow.connections {
        if let Some(target_node) = workflow.nodes.iter().find(|n| n.id == conn.target) {
            if target_node.node_type.starts_with("output_") {
                let target_type = target_node.node_type.strip_prefix("output_").unwrap_or("").to_string();
                downstream_targets.entry(conn.source.clone()).or_default().push(target_type);
            }
        }
    }

    // Execute nodes
    let mut node_results: Vec<NodeResult> = Vec::new();
    let mut step_outputs: Vec<(String, String, String)> = Vec::new(); // (node_id, output, schema_name)
    let mut all_usage: Vec<Value> = Vec::new();

    for node in &ordered_nodes {
        let deps = deps_map.get(&node.id).cloned().unwrap_or_default();
        let previous: Vec<(&str, &str, &str)> = step_outputs
            .iter()
            .filter(|(sid, _, _)| deps.contains(sid))
            .map(|(sid, out, schema)| (sid.as_str(), out.as_str(), schema.as_str()))
            .collect();

        let (output, usage_meta) = if node.node_type.starts_with("output_") {
            // Output nodes pass through their input data unchanged
            let input_data = previous
                .iter()
                .map(|(_, result, _)| result.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            (input_data, None)
        } else if let Some(action_result) =
            execute_action_node(pool, node, &previous, opts.project_id, opts.organization_id, opts.workflow_run_id).await
        {
            action_result
        } else {
            let targets = downstream_targets
                .get(&node.id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            execute_node_with_llm(pool, node, content, &previous, &opts.model, targets).await
        };

        // Derive schema_name from explicit output_schema param, or auto-infer from
        // downstream output nodes so user-created workflows work without manual config.
        let explicit_schema = node
            .parameters
            .get("output_schema")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim_end_matches("[]");
        let schema_name = if !explicit_schema.is_empty() {
            explicit_schema.to_string()
        } else if let Some(targets) = downstream_targets.get(&node.id) {
            // Map output node types to schema names (e.g. "crm_contacts" → "contacts")
            targets.iter().map(|t| match t.as_str() {
                "crm_contacts" => "contacts",
                "crm_companies" | "companies" => "companies",
                "crm_deals" | "deals" => "deals",
                "tasks" => "tasks",
                other => other,
            }).collect::<Vec<_>>().join("_")
        } else {
            String::new()
        };

        if let Some(ref usage) = usage_meta {
            all_usage.push(usage.clone());
        }

        // Create execution artifact if requested
        if opts.create_artifacts {
            let step_index = ordered_nodes.iter().position(|n| n.id == node.id).unwrap_or(0);
            let mut artifact_metadata = json!({
                "workflow_id": workflow.id,
                "step_id": node.id,
                "step_index": step_index,
            });
            if let Some(ds_id) = opts.data_source_id {
                artifact_metadata["data_source_id"] = json!(ds_id.to_string());
            }
            if let Some(ref extra) = opts.artifact_metadata_extra {
                if let Some(extra_obj) = extra.as_object() {
                    for (k, v) in extra_obj {
                        artifact_metadata[k] = v.clone();
                    }
                }
            }
            if let Some(ref usage) = usage_meta {
                artifact_metadata["usage"] = usage.clone();
            }

            if let Err(e) = ExecutionArtifact::create(
                pool,
                CreateExecutionArtifact {
                    execution_process_id: None,
                    artifact_type: ArtifactType::ResearchReport,
                    title: format!("{} - {}", workflow.name, node.name),
                    content: Some(output.clone()),
                    file_path: None,
                    metadata: Some(artifact_metadata),
                },
            )
            .await
            {
                tracing::error!(
                    "[WORKFLOW] Failed to create artifact for node '{}': {e}",
                    node.id
                );
            }
        }

        node_results.push(NodeResult {
            node_id: node.id.clone(),
            node_name: node.name.clone(),
            node_type: node.node_type.clone(),
            output: output.clone(),
            schema_name: schema_name.clone(),
            usage: usage_meta,
        });
        step_outputs.push((node.id.clone(), output, schema_name));
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

    // Create staging records for output nodes if requested
    let mut staged_records: i64 = 0;
    if opts.create_staging {
        if let Some(run_id) = opts.workflow_run_id {
            for node in ordered_nodes.iter().filter(|n| n.node_type.starts_with("output_")) {
                let target_type = node.node_type.strip_prefix("output_").unwrap_or("");
                tracing::debug!(
                    "[WORKFLOW] Staging: output node '{}' type='output_{}' → target='{}'",
                    node.id, target_type, target_type
                );
                let staging_target = match target_type {
                    "crm_contacts" => "crm_contact",
                    "crm_companies" => "company",
                    "crm_deals" => "crm_deal",
                    "tasks" => "task",
                    _ => {
                        tracing::warn!(
                            "[WORKFLOW] Staging: skipping output node '{}' — unrecognized type 'output_{}'",
                            node.id, target_type
                        );
                        continue;
                    }
                };

                if let Some((_, output, _)) = step_outputs.iter().find(|(id, _, _)| id == &node.id)
                {
                    tracing::debug!(
                        "[WORKFLOW] Staging: node '{}' raw output (first 500 chars): {}",
                        node.id, &output[..output.len().min(500)]
                    );
                    if let Ok(parsed) = serde_json::from_str::<Value>(output) {
                        let records = extract_records_from_output(&parsed, staging_target);
                        tracing::debug!(
                            "[WORKFLOW] Staging: node '{}' extracted {} records for target '{}'",
                            node.id, records.len(), staging_target
                        );
                        for record in records {
                            let dup = match staging_target {
                                "crm_contact" => {
                                    check_contact_duplicate(pool, &record, opts.project_id).await
                                }
                                "company" => {
                                    check_company_duplicate(pool, &record, opts.organization_id)
                                        .await
                                }
                                "crm_deal" => {
                                    check_deal_duplicate(pool, &record, opts.project_id).await
                                }
                                "task" => {
                                    check_task_duplicate(pool, &record, opts.project_id).await
                                }
                                _ => None,
                            };
                            let dup = dup.or(
                                check_intra_batch_duplicate(pool, run_id, staging_target, &record)
                                    .await,
                            );
                            let (dup_id, dup_type) = match dup {
                                Some((id, t)) => (Some(id), Some(t)),
                                None => (None, None),
                            };

                            let mut validation_errors =
                                match validate_record_against_schema(&record, staging_target) {
                                    Ok(()) => None,
                                    Err(errs) => {
                                        tracing::warn!(
                                        "[WORKFLOW] Validation errors for {} record in node '{}': {:?}",
                                        staging_target, node.id, errs
                                    );
                                        Some(errs)
                                    }
                                };

                            if is_fallback_placeholder(&record) {
                                validation_errors.get_or_insert_with(Vec::new).push(
                                    "Placeholder record generated without LLM — no real data extracted".to_string(),
                                );
                            }

                            let precommit_errs = super::workflow_staging::validate_staging_record(
                                staging_target,
                                &record,
                            );
                            if !precommit_errs.is_empty() {
                                validation_errors
                                    .get_or_insert_with(Vec::new)
                                    .extend(precommit_errs);
                            }

                            let is_duplicate = dup_id.is_some();
                            let confidence = compute_confidence(
                                &record,
                                staging_target,
                                validation_errors.as_deref().unwrap_or(&[]),
                                is_duplicate,
                            );

                            match WorkflowStagingRecord::create(
                                pool,
                                CreateStagingRecord {
                                    workflow_run_id: run_id,
                                    workflow_id: workflow.id.clone(),
                                    node_id: node.id.clone(),
                                    data_source_id: opts.data_source_id,
                                    organization_id: opts.organization_id,
                                    project_id: opts.project_id,
                                    target_type: staging_target.to_string(),
                                    record_data: record,
                                    duplicate_of_id: dup_id,
                                    duplicate_of_type: dup_type,
                                    confidence: Some(confidence),
                                    validation_errors,
                                },
                            )
                            .await
                            {
                                Ok(_) => staged_records += 1,
                                Err(e) => {
                                    tracing::error!(
                                        "[WORKFLOW] Failed to create staging record: {e}"
                                    )
                                }
                            }
                        }
                    } else {
                        tracing::warn!(
                            "[WORKFLOW] Staging: node '{}' output is not valid JSON — cannot extract records",
                            node.id
                        );
                    }
                } else {
                    tracing::warn!(
                        "[WORKFLOW] Staging: node '{}' not found in step_outputs (available: {:?})",
                        node.id,
                        step_outputs.iter().map(|(id, _, _)| id.as_str()).collect::<Vec<_>>()
                    );
                }
            }
        }
    }

    // Count duplicates
    let duplicates_found = if let Some(run_id) = opts.workflow_run_id {
        let staging = WorkflowStagingRecord::find_by_run(pool, run_id)
            .await
            .unwrap_or_default();
        staging
            .iter()
            .filter(|r| r.duplicate_of_id.is_some())
            .count() as i64
    } else {
        0
    };

    WorkflowExecutionResult {
        node_results,
        total_usage,
        staged_records,
        duplicates_found,
    }
}

/// Update a WorkflowRun record with completion stats from an execution result.
pub async fn finalize_workflow_run(
    pool: &SqlitePool,
    run_id: Uuid,
    workflow: &WorkflowDefinition,
    result: &WorkflowExecutionResult,
    duration_ms: i64,
    status: &str,
) {
    let (total_input, total_output, total_cost) = if let Some(ref usage) = result.total_usage {
        (
            usage["total_input_tokens"].as_i64().unwrap_or(0),
            usage["total_output_tokens"].as_i64().unwrap_or(0),
            usage["total_estimated_cost_micros"].as_i64().unwrap_or(0),
        )
    } else {
        (0, 0, 0)
    };

    let node_count = workflow.nodes.len() as i64;
    let llm_node_count = workflow
        .nodes
        .iter()
        .filter(|n| n.node_type.starts_with("llm_"))
        .count() as i64;

    if let Err(e) = WorkflowRun::update_on_complete(
        pool,
        &run_id.to_string(),
        UpdateWorkflowRunOnComplete {
            status: status.to_string(),
            total_input_tokens: total_input,
            total_output_tokens: total_output,
            total_estimated_cost_micros: total_cost,
            total_records_staged: result.staged_records,
            total_duplicates_found: result.duplicates_found,
            node_count,
            llm_node_count,
            duration_ms,
        },
    )
    .await
    {
        tracing::error!(
            "[WORKFLOW] Failed to update workflow run on complete: {e}"
        );
    }
}

/// Check if any node returned an error (failed LLM call).
pub fn check_for_llm_errors(results: &[NodeResult]) -> Vec<String> {
    results
        .iter()
        .filter_map(|r| {
            serde_json::from_str::<Value>(&r.output)
                .ok()
                .and_then(|v| {
                    v.get("error")
                        .and_then(|e| e.as_str().map(|s| format!("Node '{}': {}", r.node_id, s)))
                })
        })
        .collect()
}

/// Auto-approve, commit, and post-process staged records for a workflow run.
///
/// This encapsulates the full auto-approve pipeline:
/// 1. Approve non-duplicate records with confidence >= 0.7 (via `auto_approve_valid`)
/// 2. Reject duplicate records
/// 3. Commit approved records in dependency order (companies → contacts → deals → tasks)
/// 4. Post-commit: link contacts to companies by matching `company_name`
/// 5. Post-commit: auto-start agent execution for tasks with `agent_id`
///
/// Respects `MAX_AUTO_APPROVE_RECORDS` as a safety guard.
pub async fn auto_approve_staged_records(
    pool: &SqlitePool,
    workflow_run_id: Uuid,
    staged_records: i64,
    deployment: &crate::DeploymentImpl,
    trigger_label: &str,
) {
    if staged_records == 0 {
        return;
    }

    if staged_records > MAX_AUTO_APPROVE_RECORDS {
        tracing::warn!(
            "[WORKFLOW] Skipping auto-approve for {trigger_label}: {staged_records} records exceeds limit of {MAX_AUTO_APPROVE_RECORDS}"
        );
        return;
    }

    tracing::info!(
        "[WORKFLOW] auto_approve enabled for {trigger_label} — approving valid records"
    );

    // Step 1: auto-approve non-duplicate records with confidence >= 0.7
    match WorkflowStagingRecord::auto_approve_valid(pool, workflow_run_id).await {
        Ok(approved) => {
            tracing::info!(
                "[WORKFLOW] Auto-approved {} record(s) for workflow run {}",
                approved, workflow_run_id
            );

            // Step 2: reject duplicates
            if let Err(e) = WorkflowStagingRecord::reject_duplicates(pool, workflow_run_id).await {
                tracing::warn!("[WORKFLOW] Failed to reject duplicates: {e}");
            }

            // Step 3: commit approved records
            if approved > 0 {
                let records = WorkflowStagingRecord::find_by_run(pool, workflow_run_id)
                    .await
                    .unwrap_or_default();
                let mut approved_records: Vec<_> = records.into_iter()
                    .filter(|r| r.status == "approved")
                    .collect();

                // Sort: companies first, then contacts, then deals, then tasks
                approved_records.sort_by_key(|r| match r.target_type.as_str() {
                    "company" => 0,
                    "crm_contact" => 1,
                    "crm_deal" => 2,
                    "task" => 3,
                    _ => 4,
                });

                let mut committed = 0u64;
                let mut commit_results: Vec<(Uuid, String, Option<Uuid>)> = Vec::new();
                for record in &approved_records {
                    match super::workflow_staging::commit_record_internal(pool, record).await {
                        Ok(created_id) => {
                            committed += 1;
                            commit_results.push((record.id, record.target_type.clone(), Some(created_id)));
                        }
                        Err(e) => {
                            tracing::warn!(
                                "[WORKFLOW] Auto-commit failed for record {}: {e}",
                                record.id
                            );
                            commit_results.push((record.id, record.target_type.clone(), None));
                        }
                    }
                }

                tracing::info!(
                    "[WORKFLOW] Auto-committed {}/{} records for {trigger_label}",
                    committed, approved
                );

                // Post-commit: link contacts to companies by matching company_name
                let org_id = approved_records.first().and_then(|r| r.organization_id);
                if let Some(org_id) = org_id {
                    for (_, target_type, created_id) in &commit_results {
                        if target_type == "crm_contact" {
                            if let Some(contact_id) = created_id {
                                if let Ok(contact) = db::models::crm_contact::CrmContact::find_by_id(pool, *contact_id).await {
                                    if let Some(company_name) = &contact.company_name {
                                        if let Ok(Some(company)) = db::models::company::Company::find_by_name_and_org(pool, company_name, org_id).await {
                                            let _ = super::workflow_staging::store_company_id_in_custom_fields(pool, contact.id, company.id).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Post-commit: auto-start agent execution for tasks with agent_id
                for (idx, (_, target_type, created_id)) in commit_results.iter().enumerate() {
                    if target_type == "task" {
                        if let Some(task_id) = created_id {
                            if let Ok(data) = serde_json::from_str::<Value>(&approved_records[idx].record_data) {
                                if data["agent_id"].as_str().is_some() {
                                    let pool = pool.clone();
                                    let dep = deployment.clone();
                                    let tid = *task_id;
                                    tokio::spawn(async move {
                                        if let Err(e) = super::workflow_staging::auto_start_agent_execution(&pool, &dep, tid).await {
                                            tracing::warn!("Auto-execute for task {tid} failed: {e}");
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("[WORKFLOW] Auto-approve failed for workflow run {}: {e}", workflow_run_id);
        }
    }
}
