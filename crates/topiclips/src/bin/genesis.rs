//! TopiClips Genesis - Generate the first clip in the series
//!
//! This binary creates a "genesis" TopiClip representing the initial state
//! or birth of the topology, even if no events have occurred yet.
//!
//! Usage:
//!   cargo run --bin topiclips-genesis -- --project-id <UUID> [--db-path <PATH>]
//!
//! If no project-id is provided, it will list available projects.

use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Row, SqlitePool};
use std::env;
use std::path::PathBuf;
use uuid::Uuid;

const GENESIS_THEME: &str = "transformation";
const GENESIS_ARC: &str = "hopeful";

/// Genesis artistic prompt - the birth of a topology
const GENESIS_PROMPT: &str = r#"In an infinite void of deep cosmic blue, a single point of golden light emerges from nothingness.
The light expands into geometric fractals, each angle and edge representing potential connections yet to form.
A crystalline seed structure takes shape at the center - translucent, faceted, pulsing with inner radiance.
Around it, faint outlines of pathways and nodes begin to shimmer into existence like stars being born.
The scene captures the moment of creation, where infinite possibility crystallizes into the first form.
Particles of light swirl in orbital patterns, each one a future agent, task, or idea waiting to manifest.
The color palette transitions from deep void-black through sapphire blue to warm amber and brilliant white at the center.
Style: Beeple-inspired digital surrealism, cosmic scale, geometric precision, ethereal lighting."#;

const GENESIS_NEGATIVE: &str = "text, watermark, signature, blurry, low quality, realistic human faces, mundane objects";

async fn ensure_tables(pool: &SqlitePool) -> Result<()> {
    // Check if tables exist - if so, we don't need to create them
    // This preserves existing sessions instead of dropping them
    let sessions_exist = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='topiclip_sessions'"
    )
    .fetch_optional(pool)
    .await?
    .is_some();

    if sessions_exist {
        println!("  TopiClips tables already exist (preserving existing data)");
        return Ok(());
    }

    println!("  Creating TopiClips tables...");

    // Create tables only if they don't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS topiclip_sessions (
            id TEXT PRIMARY KEY NOT NULL,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            day_number INTEGER NOT NULL,
            trigger_type TEXT NOT NULL,
            primary_theme TEXT,
            emotional_arc TEXT,
            narrative_summary TEXT,
            artistic_prompt TEXT,
            negative_prompt TEXT,
            symbol_mapping TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            cinematic_brief_id TEXT,
            output_asset_ids TEXT,
            duration_seconds INTEGER DEFAULT 4,
            llm_notes TEXT,
            error_message TEXT,
            events_analyzed INTEGER DEFAULT 0,
            significance_score REAL,
            period_start TEXT,
            period_end TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            delivered_at TEXT
        )
        "#
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS topiclip_captured_events (
            id TEXT PRIMARY KEY NOT NULL,
            session_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            event_data TEXT NOT NULL,
            narrative_role TEXT,
            significance_score REAL,
            assigned_symbol TEXT,
            symbol_prompt TEXT,
            affected_node_ids TEXT,
            affected_edge_ids TEXT,
            occurred_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn list_projects(pool: &SqlitePool) -> Result<()> {
    let rows = sqlx::query(
        "SELECT hex(id) as id_hex, name, git_repo_path FROM projects ORDER BY created_at DESC LIMIT 20"
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        println!("  No projects found. Create a project first.");
        println!("\n  Or run with: --project-id <UUID> to specify a project");
        return Ok(());
    }

    for row in &rows {
        let id_hex: String = row.get("id_hex");
        // Convert hex to UUID format (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
        let id = if id_hex.len() == 32 {
            format!(
                "{}-{}-{}-{}-{}",
                &id_hex[0..8],
                &id_hex[8..12],
                &id_hex[12..16],
                &id_hex[16..20],
                &id_hex[20..32]
            ).to_lowercase()
        } else {
            id_hex.to_lowercase()
        };
        let name: String = row.get("name");
        let repo_path: String = row.get("git_repo_path");

        println!("  {} - {}", id, name);
        println!("    repo: {}", repo_path);
    }

    println!("\nUsage: cargo run --bin topiclips-genesis -- --project-id <UUID>");
    Ok(())
}

async fn verify_project(pool: &SqlitePool, project_id: &Uuid) -> Result<String> {
    let row = sqlx::query("SELECT id, name FROM projects WHERE id = ?")
        .bind(project_id.as_bytes().as_slice())
        .fetch_optional(pool)
        .await?
        .context("Project not found")?;

    let name: String = row.get("name");
    Ok(name)
}

async fn check_genesis_exists(pool: &SqlitePool, project_id: &Uuid) -> Result<bool> {
    let row = sqlx::query(
        "SELECT id FROM topiclip_sessions WHERE project_id = ? AND day_number = 0"
    )
    .bind(project_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

async fn create_genesis_clip(pool: &SqlitePool, project_id: &Uuid) -> Result<Uuid> {
    let session_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let symbol_mapping = json!({"genesis": {"symbol": "Cosmic Seed", "description": "The first spark of creation"}}).to_string();
    let event_data = json!({"type": "genesis", "description": "The first moment of the topology"}).to_string();
    let symbol_prompt = "A crystalline seed of light emerges from the void, containing all future possibilities";

    // Create the session (topiclip tables use text UUIDs)
    sqlx::query(
        r#"
        INSERT INTO topiclip_sessions (
            id, project_id, title, day_number, trigger_type,
            primary_theme, emotional_arc, narrative_summary,
            artistic_prompt, negative_prompt, symbol_mapping,
            status, events_analyzed, significance_score,
            created_at, updated_at
        ) VALUES (?, ?, 'Day 0 - Genesis', 0, 'manual', ?, ?,
            'The birth of a new topology - infinite possibility crystallizing into form.',
            ?, ?, ?, 'interpreting', 0, 1.0, ?, ?)
        "#
    )
    .bind(session_id.to_string())
    .bind(project_id.to_string())
    .bind(GENESIS_THEME)
    .bind(GENESIS_ARC)
    .bind(GENESIS_PROMPT)
    .bind(GENESIS_NEGATIVE)
    .bind(&symbol_mapping)
    .bind(&now_str)
    .bind(&now_str)
    .execute(pool)
    .await
    .context("Failed to create genesis session")?;

    // Create a genesis event
    sqlx::query(
        r#"
        INSERT INTO topiclip_captured_events (
            id, session_id, event_type, event_data,
            narrative_role, significance_score,
            assigned_symbol, symbol_prompt,
            occurred_at, created_at
        ) VALUES (?, ?, 'Genesis', ?, 'protagonist', 1.0, 'Cosmic Seed', ?, ?, ?)
        "#
    )
    .bind(event_id.to_string())
    .bind(session_id.to_string())
    .bind(&event_data)
    .bind(symbol_prompt)
    .bind(&now_str)
    .bind(&now_str)
    .execute(pool)
    .await
    .context("Failed to create genesis event")?;

    Ok(session_id)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let mut project_id: Option<Uuid> = None;
    let mut db_path: Option<PathBuf> = None;
    let mut render = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--project-id" | "-p" => {
                if i + 1 < args.len() {
                    project_id = Some(Uuid::parse_str(&args[i + 1])
                        .context("Invalid project ID format")?);
                    i += 1;
                }
            }
            "--db-path" | "-d" => {
                if i + 1 < args.len() {
                    db_path = Some(PathBuf::from(&args[i + 1]));
                    i += 1;
                }
            }
            "--render" | "-r" => {
                render = true;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    // Determine database path
    let db_path = db_path.unwrap_or_else(|| {
        PathBuf::from(env::var("DATABASE_URL")
            .unwrap_or_else(|_| "dev_assets/db.sqlite".to_string())
            .trim_start_matches("sqlite://"))
    });

    println!("TopiClips Genesis - Creating the first clip in the series");
    println!("=========================================================\n");
    println!("Database: {}", db_path.display());

    // Connect to database
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .context("Failed to connect to database")?;

    // Ensure tables exist
    ensure_tables(&pool).await?;

    // If no project ID provided, list available projects
    if project_id.is_none() {
        println!("\nNo project ID provided. Available projects:\n");
        list_projects(&pool).await?;
        return Ok(());
    }

    let project_id = project_id.unwrap();

    // Verify project exists
    let project_name = verify_project(&pool, &project_id).await?;
    println!("Project: {} ({})", project_name, project_id);

    // Check if genesis clip already exists
    if check_genesis_exists(&pool, &project_id).await? {
        println!("\nGenesis clip (Day 0) already exists for this project.");
        println!("To create a new clip, the server's /api/topiclips/sessions endpoint can be used.");
        return Ok(());
    }

    // Create genesis session (Day 0)
    println!("\nCreating genesis clip (Day 0)...\n");
    let session_id = create_genesis_clip(&pool, &project_id).await?;

    println!("Genesis clip created!");
    println!("\n--- Session Details ---");
    println!("Session ID: {}", session_id);
    println!("Day Number: 0 (Genesis)");
    println!("Theme: {}", GENESIS_THEME);
    println!("Emotional Arc: {}", GENESIS_ARC);
    println!("Status: interpreting");

    println!("\n--- Artistic Prompt ---");
    println!("{}", GENESIS_PROMPT);

    println!("\n--- Negative Prompt ---");
    println!("{}", GENESIS_NEGATIVE);

    if render {
        println!("\n--- Rendering ---");
        println!("Render flag set, but ComfyUI integration requires the server to be running.");
        println!("To render, start the server and call:");
        println!("  POST /api/topiclips/sessions/{}/generate", session_id);
    } else {
        println!("\n--- Next Steps ---");
        println!("1. Copy the artistic prompt above to your preferred image generator");
        println!("2. Or start the server and trigger rendering via API:");
        println!("   POST /api/topiclips/sessions/{}/generate", session_id);
        println!("3. The prompt is designed for Beeple-style surreal digital art");
    }

    println!("\n--- Simplified Prompt for Image Generators ---");
    println!("For Midjourney, DALL-E, or Stable Diffusion:\n");
    let simplified_prompt = "Cosmic genesis scene: A single point of golden light emerges from infinite deep blue void, expanding into geometric fractals. Crystalline seed structure at center, translucent and faceted, pulsing with inner radiance. Faint pathways and nodes shimmer like stars being born. Light particles swirl in orbital patterns. Color: void-black through sapphire to amber and white center. Style: Beeple digital surrealism, cosmic scale, geometric precision, ethereal lighting, 8k, cinematic --ar 16:9";
    println!("{}", simplified_prompt);

    Ok(())
}

fn print_help() {
    println!("TopiClips Genesis - Generate the first clip in the series");
    println!();
    println!("USAGE:");
    println!("    topiclips-genesis [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -p, --project-id <UUID>    Project ID to create genesis clip for");
    println!("    -d, --db-path <PATH>       Path to SQLite database (default: dev_assets/db.sqlite)");
    println!("    -r, --render               Attempt to trigger rendering (requires server)");
    println!("    -h, --help                 Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # List available projects:");
    println!("    cargo run --bin topiclips-genesis");
    println!();
    println!("    # Create genesis clip for a specific project:");
    println!("    cargo run --bin topiclips-genesis -- --project-id <UUID>");
}
