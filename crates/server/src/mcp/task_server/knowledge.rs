use db::models::project_knowledge_source::{
    KnowledgeSourceType, ProjectKnowledgeSource,
};
use rmcp::{
    ErrorData,
    handler::server::tool::Parameters,
    model::CallToolResult,
    tool,
};
use serde_json::Value;

use super::TaskServer;
use super::helpers::*;
use super::types::*;

impl TaskServer {
    #[tool(
        description = "Add or update a knowledge source for a project. Used to track what information the project has ingested."
    )]
    pub(super) async fn add_knowledge(
        &self,
        Parameters(req): Parameters<AddKnowledgeRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let source_type: KnowledgeSourceType = match req.source_type.parse() {
            Ok(st) => st,
            Err(_) => return Ok(error_result(
                "Invalid source_type. Valid: conversation, artifact, pulse_content, context_injection, entity, topology_snapshot",
                None,
            )),
        };

        let coverage = req.coverage_score.unwrap_or(0.5).clamp(0.0, 1.0);

        match ProjectKnowledgeSource::upsert_source(
            &self.pool,
            project_uuid,
            &source_type,
            &req.source_id,
            &req.source_title,
            req.source_summary.as_deref(),
            coverage,
        )
        .await
        {
            Ok(()) => Ok(success_json(&serde_json::json!({
                "success": true,
                "message": "Knowledge source added/updated",
                "project_id": req.project_id,
                "source_type": req.source_type,
                "source_id": req.source_id,
            }))),
            Err(e) => Ok(error_result("Failed to add knowledge source", Some(&e.to_string()))),
        }
    }

    #[tool(description = "List knowledge sources for a project, optionally filtered by type.")]
    pub(super) async fn list_knowledge(
        &self,
        Parameters(req): Parameters<ListKnowledgeRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        let include_stale = req.include_stale.unwrap_or(false);

        match ProjectKnowledgeSource::find_by_project(&self.pool, project_uuid).await {
            Ok(sources) => {
                let filtered: Vec<Value> = sources
                    .iter()
                    .filter(|s| {
                        if !include_stale && s.is_stale {
                            return false;
                        }
                        if let Some(ref st) = req.source_type {
                            if s.source_type != *st {
                                return false;
                            }
                        }
                        true
                    })
                    .map(|s| serde_json::json!({
                        "id": s.id.to_string(),
                        "source_type": s.source_type,
                        "source_id": s.source_id,
                        "source_title": s.source_title,
                        "source_summary": s.source_summary,
                        "coverage_score": s.coverage_score,
                        "is_stale": s.is_stale,
                        "last_refreshed_at": s.last_refreshed_at.to_rfc3339(),
                        "created_at": s.created_at.to_rfc3339(),
                    }))
                    .collect();

                Ok(success_json(&serde_json::json!({
                    "success": true,
                    "project_id": req.project_id,
                    "count": filtered.len(),
                    "sources": filtered,
                })))
            }
            Err(e) => Ok(error_result("Failed to list knowledge sources", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Get knowledge completeness metrics for a project — total sources, freshness, coverage, and completeness score."
    )]
    pub(super) async fn get_knowledge_completeness(
        &self,
        Parameters(req): Parameters<GetKnowledgeCompletenessRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match ProjectKnowledgeSource::get_completeness(&self.pool, project_uuid).await {
            Ok(Some(kc)) => Ok(success_json(&serde_json::json!({
                "success": true,
                "project_id": req.project_id,
                "total_sources": kc.total_sources,
                "fresh_sources": kc.fresh_sources,
                "avg_coverage": kc.avg_coverage,
                "type_count": kc.type_count,
                "knowledge_completeness": kc.knowledge_completeness,
            }))),
            Ok(None) => Ok(success_json(&serde_json::json!({
                "success": true,
                "project_id": req.project_id,
                "total_sources": 0,
                "fresh_sources": 0,
                "avg_coverage": 0.0,
                "type_count": 0,
                "knowledge_completeness": 0.0,
            }))),
            Err(e) => Ok(error_result("Failed to get knowledge completeness", Some(&e.to_string()))),
        }
    }

    #[tool(
        description = "Manage a knowledge source: mark_stale, mark_refreshed, or delete. `source_id` is the knowledge source UUID."
    )]
    pub(super) async fn manage_knowledge(
        &self,
        Parameters(req): Parameters<ManageKnowledgeRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let _project_uuid = match parse_uuid(&req.project_id, "project_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };
        let source_uuid = match parse_uuid(&req.source_id, "source_id") {
            Ok(u) => u,
            Err(r) => return Ok(r),
        };

        match req.action.as_str() {
            "mark_stale" => {
                match ProjectKnowledgeSource::mark_stale(&self.pool, source_uuid).await {
                    Ok(()) => Ok(success_json(&serde_json::json!({
                        "success": true,
                        "message": "Knowledge source marked as stale",
                    }))),
                    Err(e) => Ok(error_result("Failed to mark stale", Some(&e.to_string()))),
                }
            }
            "mark_refreshed" => {
                match ProjectKnowledgeSource::mark_refreshed(&self.pool, source_uuid).await {
                    Ok(()) => Ok(success_json(&serde_json::json!({
                        "success": true,
                        "message": "Knowledge source marked as refreshed",
                    }))),
                    Err(e) => Ok(error_result("Failed to mark refreshed", Some(&e.to_string()))),
                }
            }
            "delete" => {
                let id_bytes = source_uuid.to_string();
                match sqlx::query("DELETE FROM project_knowledge_sources WHERE id = ?")
                    .bind(&id_bytes)
                    .execute(&self.pool)
                    .await
                {
                    Ok(r) => {
                        if r.rows_affected() > 0 {
                            Ok(success_json(&serde_json::json!({
                                "success": true,
                                "message": "Knowledge source deleted",
                            })))
                        } else {
                            Ok(error_result("Knowledge source not found", None))
                        }
                    }
                    Err(e) => Ok(error_result("Failed to delete knowledge source", Some(&e.to_string()))),
                }
            }
            _ => Ok(error_result("Invalid action. Use 'mark_stale', 'mark_refreshed', or 'delete'", None)),
        }
    }
}
