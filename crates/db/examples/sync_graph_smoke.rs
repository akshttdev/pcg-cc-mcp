//! Smoke-runs `sync_global_graph` against a local SQLite DB and reports the
//! resulting node/edge counts. Usage:
//!   DATABASE_URL="sqlite:dev_assets/db.sqlite" \
//!     cargo run -p db --example sync_graph_smoke
//!
//! Not part of the test suite — purely for manual verification during
//! Phase 1 development.

use db::models::entity_graph::sync_global_graph;
use sqlx::{Row, SqlitePool};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:dev_assets/db.sqlite".into());
    println!("Connecting to {}", url);
    let pool = SqlitePool::connect(&url).await?;

    let before_nodes: i64 = sqlx::query("SELECT COUNT(*) AS c FROM entity_graph_nodes")
        .fetch_one(&pool)
        .await?
        .get("c");
    let before_edges: i64 = sqlx::query("SELECT COUNT(*) AS c FROM entity_graph_edges")
        .fetch_one(&pool)
        .await?
        .get("c");

    println!("Before:  nodes={}, edges={}", before_nodes, before_edges);

    let stats = sync_global_graph(&pool).await?;
    println!("Sync stats: {:?}", stats);

    let after_nodes: i64 = sqlx::query("SELECT COUNT(*) AS c FROM entity_graph_nodes")
        .fetch_one(&pool)
        .await?
        .get("c");
    let after_edges: i64 = sqlx::query("SELECT COUNT(*) AS c FROM entity_graph_edges")
        .fetch_one(&pool)
        .await?
        .get("c");

    println!("After:   nodes={}, edges={}", after_nodes, after_edges);
    println!(
        "Delta:   +{} nodes, +{} edges",
        after_nodes - before_nodes,
        after_edges - before_edges
    );

    // Idempotence check: run it again and confirm counts stay the same.
    println!("\nIdempotence check — running again...");
    sync_global_graph(&pool).await?;

    let final_nodes: i64 = sqlx::query("SELECT COUNT(*) AS c FROM entity_graph_nodes")
        .fetch_one(&pool)
        .await?
        .get("c");
    let final_edges: i64 = sqlx::query("SELECT COUNT(*) AS c FROM entity_graph_edges")
        .fetch_one(&pool)
        .await?
        .get("c");

    assert_eq!(
        after_nodes, final_nodes,
        "nodes changed on 2nd sync (not idempotent)"
    );
    assert_eq!(
        after_edges, final_edges,
        "edges changed on 2nd sync (not idempotent)"
    );
    println!(
        "Idempotent ✓  (counts stable at nodes={}, edges={})",
        final_nodes, final_edges
    );

    // Per-type breakdown
    println!("\nNode type breakdown:");
    let rows = sqlx::query("SELECT node_type, COUNT(*) AS c FROM entity_graph_nodes GROUP BY node_type ORDER BY c DESC")
        .fetch_all(&pool)
        .await?;
    for r in rows {
        let nt: String = r.get("node_type");
        let c: i64 = r.get("c");
        println!("  {:20} {}", nt, c);
    }

    println!("\nEdge type breakdown:");
    let rows = sqlx::query("SELECT edge_type, COUNT(*) AS c FROM entity_graph_edges GROUP BY edge_type ORDER BY c DESC")
        .fetch_all(&pool)
        .await?;
    for r in rows {
        let et: String = r.get("edge_type");
        let c: i64 = r.get("c");
        println!("  {:20} {}", et, c);
    }

    Ok(())
}
