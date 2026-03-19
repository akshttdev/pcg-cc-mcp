//! Knowledge completeness enrichment for EFE actions
//!
//! Batch-fetches project-level knowledge completeness from the
//! `v_project_knowledge_completeness` view and blends it into
//! each PotentialAction's data_completeness field.

use std::collections::HashMap;

use sqlx::SqlitePool;
use uuid::Uuid;

use super::free_energy::PotentialAction;

/// Enrich a set of PotentialActions with project-level knowledge completeness.
/// Call this before `EFECalculator::rank_actions()`.
pub async fn enrich_actions_with_knowledge(actions: &mut [PotentialAction], pool: &SqlitePool) {
    // Collect unique project IDs from actions
    // We need a way to map action -> project. Actions carry a task ID;
    // we batch-query tasks to get project_ids, then fetch completeness.
    let task_ids: Vec<Uuid> = actions.iter().map(|a| a.id).collect();
    if task_ids.is_empty() {
        return;
    }

    // Batch-fetch task -> project_id mapping
    let placeholders: Vec<String> = task_ids.iter().map(|_| "?".to_string()).collect();
    let in_clause = placeholders.join(",");

    #[derive(Debug, sqlx::FromRow)]
    struct TaskProjectRow {
        id: Uuid,
        project_id: Uuid,
    }

    let query = format!(
        "SELECT id, project_id FROM tasks WHERE id IN ({})",
        in_clause
    );

    let mut q = sqlx::query_as::<_, TaskProjectRow>(&query);
    for tid in &task_ids {
        q = q.bind(tid);
    }

    let task_projects = q.fetch_all(pool).await.unwrap_or_default();

    let task_to_project: HashMap<Uuid, Uuid> = task_projects
        .into_iter()
        .map(|tp| (tp.id, tp.project_id))
        .collect();

    // Collect unique project IDs
    let unique_project_ids: Vec<String> = task_to_project
        .values()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .map(|pid| pid.to_string())
        .collect();

    if unique_project_ids.is_empty() {
        return;
    }

    // Batch-fetch knowledge completeness
    let kc_placeholders: Vec<String> = unique_project_ids.iter().map(|_| "?".to_string()).collect();
    let kc_in_clause = kc_placeholders.join(",");

    #[derive(Debug, sqlx::FromRow)]
    struct KcRow {
        project_id: Uuid,
        knowledge_completeness: f64,
    }

    let kc_query = format!(
        "SELECT project_id, knowledge_completeness FROM v_project_knowledge_completeness WHERE project_id IN ({})",
        kc_in_clause
    );

    let mut kc_q = sqlx::query_as::<_, KcRow>(&kc_query);
    for pid in &unique_project_ids {
        kc_q = kc_q.bind(pid);
    }

    let kc_rows = kc_q.fetch_all(pool).await.unwrap_or_default();

    let project_completeness: HashMap<Uuid, f64> = kc_rows
        .into_iter()
        .map(|r| (r.project_id, r.knowledge_completeness))
        .collect();

    // Apply knowledge completeness to each action
    for action in actions.iter_mut() {
        if let Some(project_id) = task_to_project.get(&action.id) {
            if let Some(&kc) = project_completeness.get(project_id) {
                let task_completeness = action.data_completeness;
                action.data_completeness = 0.5 * task_completeness + 0.5 * kc;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_blending() {
        let mut action = PotentialAction::new(Uuid::new_v4(), "Test task");
        action.data_completeness = 0.6;

        // Simulate blending with project completeness of 0.8
        let blended = action.with_knowledge_completeness(0.8);
        // 0.5 * 0.6 + 0.5 * 0.8 = 0.3 + 0.4 = 0.7
        assert!((blended.data_completeness - 0.7).abs() < 0.001);
    }
}
