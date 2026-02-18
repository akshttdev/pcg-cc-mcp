//! ORCHA First-Time Setup Wizard (CLI)
//!
//! Beautiful interactive terminal-based setup for new ORCHA nodes.
//! Guides users through initial configuration and database initialization.

use anyhow::{Context, Result};
use colored::*;
use inquire::{Confirm, Password, Select, Text};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use sysinfo::System;

#[derive(Debug, Serialize, Deserialize)]
struct SetupConfig {
    user: UserSetup,
    device: DeviceSetup,
    storage: StorageSetup,
    network: NetworkSetup,
}

#[derive(Debug, Serialize, Deserialize)]
struct UserSetup {
    username: String,
    password_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeviceSetup {
    device_name: String,
    device_type: String,
    device_id: String,
    gpu_info: Option<String>,
    cpu_info: String,
    total_memory_gb: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct StorageSetup {
    projects_path: PathBuf,
    topsi_db_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct NetworkSetup {
    apn_enabled: bool,
    relay_url: String,
}

fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".config/orcha/config.toml")
}

fn is_already_configured() -> bool {
    let config_path = get_config_path();
    let shared_db = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/duck-kanban/db.sqlite");
    config_path.exists() || shared_db.exists()
}

async fn run_signin() -> Result<()> {
    println!("{}", "\n┌─────────────────────────────────────────────────────┐".bright_blue());
    println!("{}", "│  Sign In to ORCHA                                  │".bright_blue());
    println!("{}", "│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │".bright_blue());
    println!("{}", "└─────────────────────────────────────────────────────┘".bright_blue());
    println!();

    let username = Text::new("Username:")
        .with_help_message("Your ORCHA username")
        .prompt()?;

    let password = Password::new("Password:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()?;

    let shared_db = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/duck-kanban/db.sqlite");

    let db_url = format!("sqlite://{}?mode=rw", shared_db.display());
    let pool = sqlx::SqlitePool::connect(&db_url).await
        .context("Could not connect to ORCHA database")?;

    let row: Option<(Vec<u8>, String, i64)> = sqlx::query_as(
        "SELECT id, password_hash, is_active FROM users WHERE LOWER(username) = LOWER(?)"
    )
    .bind(&username)
    .fetch_optional(&pool)
    .await?;

    pool.close().await;

    match row {
        None => {
            println!("\n{}", "  ✗ User not found.".bright_red());
            anyhow::bail!("Authentication failed");
        }
        Some((_id, hash, is_active)) => {
            if is_active == 0 {
                println!("\n{}", "  ✗ Account is inactive.".bright_red());
                anyhow::bail!("Account inactive");
            }
            let valid = bcrypt::verify(&password, &hash)
                .unwrap_or(false);
            if !valid {
                println!("\n{}", "  ✗ Incorrect password.".bright_red());
                anyhow::bail!("Authentication failed");
            }
            println!("\n  {} Welcome back, {}!", "✓".bright_green(), username.bright_cyan().bold());
            println!("  {} This device is recognized.", "✓".bright_green());
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Welcome banner
    print_welcome();

    // Check if already configured — offer sign-in instead
    if is_already_configured() {
        println!("{}", "  This device is already configured.".bright_green());
        println!();

        let choices = vec!["Sign In", "Re-run Setup (reset this device)"];
        let choice = Select::new("What would you like to do?", choices).prompt()?;

        if choice == "Sign In" {
            return run_signin().await;
        }
        println!();
        println!("{}", "  ⚠  Re-running setup will overwrite device configuration.".bright_yellow());
        println!();
    }

    // Run setup wizard
    let config = run_wizard().await?;

    // Initialize system
    initialize_system(&config).await?;

    // Success!
    print_success(&config);

    Ok(())
}

fn print_welcome() {
    println!("\n{}", "═══════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "                   🌟 Welcome to ORCHA                   ".bright_cyan().bold());
    println!("{}", "              Orchestration Application                ".bright_cyan());
    println!("{}", "═══════════════════════════════════════════════════════".bright_cyan());
    println!();
    println!("{}", "Let's set up your sovereign node.".white());
    println!("{}", "This will only take a few minutes.".white().dimmed());
    println!();
}

async fn run_wizard() -> Result<SetupConfig> {
    // Step 1: User Account
    println!("{}", "\n┌─────────────────────────────────────────────────────┐".bright_blue());
    println!("{}", "│  Step 1: User Account                              │".bright_blue());
    println!("{}", "│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │".bright_blue());
    println!("{}", "└─────────────────────────────────────────────────────┘".bright_blue());
    println!();

    let username = Text::new("Username:")
        .with_help_message("Your display name in ORCHA")
        .prompt()?;

    let password = Password::new("Password:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .with_help_message("Choose a strong password")
        .prompt()?;

    let password_confirm = Password::new("Confirm password:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()?;

    if password != password_confirm {
        anyhow::bail!("Passwords do not match!");
    }

    let password_hash = bcrypt::hash(&password, bcrypt::DEFAULT_COST)?;

    // Step 2: Device Configuration
    println!("{}", "\n┌─────────────────────────────────────────────────────┐".bright_blue());
    println!("{}", "│  Step 2: Device Configuration                      │".bright_blue());
    println!("{}", "│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │".bright_blue());
    println!("{}", "└─────────────────────────────────────────────────────┘".bright_blue());
    println!();

    let hostname = whoami::hostname();
    let device_name = Text::new("Device name:")
        .with_default(&hostname)
        .with_help_message("A friendly name for this device")
        .prompt()?;

    let device_type = Select::new(
        "Device type:",
        vec!["Always-On (Desktop/Server)", "Mobile (Laptop/Portable)"],
    )
    .with_help_message("How reliably is this device online?")
    .prompt()?;

    let device_type = if device_type.starts_with("Always") {
        "always_on"
    } else {
        "mobile"
    };

    // Auto-detect hardware
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_info = sys
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_string())
        .unwrap_or_else(|| "Unknown CPU".to_string());

    let total_memory_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

    // Try to detect GPU (simplified)
    let gpu_info = detect_gpu();

    println!();
    println!("{}", "  Hardware detected:".bright_green());
    println!("    CPU: {}", cpu_info.white());
    println!("    RAM: {:.1} GB", total_memory_gb.to_string().white());
    if let Some(ref gpu) = gpu_info {
        println!("    GPU: {}", gpu.white());
    }
    println!();

    let device_id = format!(
        "{}-{:03}",
        device_name.to_lowercase().replace(' ', "-"),
        rand::random::<u16>() % 1000
    );

    // Step 3: Projects & Storage
    println!("{}", "\n┌─────────────────────────────────────────────────────┐".bright_blue());
    println!("{}", "│  Step 3: Projects & Storage                        │".bright_blue());
    println!("{}", "│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │".bright_blue());
    println!("{}", "└─────────────────────────────────────────────────────┘".bright_blue());
    println!();

    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let default_projects = home_dir.join("topos");

    let projects_path_str = Text::new("Projects directory:")
        .with_default(&default_projects.to_string_lossy())
        .with_help_message("Where your projects will be stored")
        .prompt()?;

    let projects_path = PathBuf::from(projects_path_str);

    let default_topsi = home_dir.join(format!(".local/share/pcg/data/{}/topsi.db", username));

    let topsi_db_path_str = Text::new("Topsi database:")
        .with_default(&default_topsi.to_string_lossy())
        .with_help_message("Your personal Topsi database location")
        .prompt()?;

    let topsi_db_path = PathBuf::from(topsi_db_path_str);

    // Step 4: APN Network
    println!("{}", "\n┌─────────────────────────────────────────────────────┐".bright_blue());
    println!("{}", "│  Step 4: Alpha Protocol Network                    │".bright_blue());
    println!("{}", "│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │".bright_blue());
    println!("{}", "└─────────────────────────────────────────────────────┘".bright_blue());
    println!();

    let apn_enabled = Confirm::new("Connect to Alpha Protocol Network?")
        .with_default(true)
        .with_help_message("Enables mesh networking with other ORCHA nodes")
        .prompt()?;

    let relay_url = if apn_enabled {
        Text::new("APN Relay URL:")
            .with_default("nats://nonlocal.info:4222")
            .prompt()?
    } else {
        String::new()
    };

    Ok(SetupConfig {
        user: UserSetup {
            username,
            password_hash,
        },
        device: DeviceSetup {
            device_name,
            device_type: device_type.to_string(),
            device_id,
            gpu_info,
            cpu_info,
            total_memory_gb,
        },
        storage: StorageSetup {
            projects_path,
            topsi_db_path,
        },
        network: NetworkSetup {
            apn_enabled,
            relay_url,
        },
    })
}

async fn initialize_system(config: &SetupConfig) -> Result<()> {
    println!("{}", "\n┌─────────────────────────────────────────────────────┐".bright_magenta());
    println!("{}", "│  Initializing ORCHA...                             │".bright_magenta());
    println!("{}", "└─────────────────────────────────────────────────────┘".bright_magenta());
    println!();

    // 1. Create directories
    print!("  {} Creating directories...", "▸".bright_yellow());
    std::io::Write::flush(&mut std::io::stdout())?;

    std::fs::create_dir_all(&config.storage.projects_path)?;
    if let Some(parent) = config.storage.topsi_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!(" {}", "✓".bright_green());

    // 2. Generate ORCHA config
    print!("  {} Generating configuration...", "▸".bright_yellow());
    std::io::Write::flush(&mut std::io::stdout())?;

    generate_orcha_config(config)?;

    println!(" {}", "✓".bright_green());

    // 3. Initialize Topsi database
    print!("  {} Initializing Topsi database...", "▸".bright_yellow());
    std::io::Write::flush(&mut std::io::stdout())?;

    initialize_topsi_database(config).await?;

    println!(" {}", "✓".bright_green());

    // 4. Create user account
    print!("  {} Creating user account...", "▸".bright_yellow());
    std::io::Write::flush(&mut std::io::stdout())?;

    create_user_account(config).await?;

    println!(" {}", "✓".bright_green());

    // 5. Register device
    print!("  {} Registering device...", "▸".bright_yellow());
    std::io::Write::flush(&mut std::io::stdout())?;

    register_device(config).await?;

    println!(" {}", "✓".bright_green());

    Ok(())
}

fn generate_orcha_config(config: &SetupConfig) -> Result<()> {
    let orcha_config = format!(
        r#"# ORCHA Configuration for {}

[orcha]
name = "ORCHA"
version = "1.0.0"
description = "Federated Topological Orchestration System"

[apn]
relay_url = "{}"
enabled = {}

[[users]]
username = "{}"
primary_device = "{}"
topsi_db_path = "{}"
projects_path = "{}"
uptime_guarantee = "{}"
description = "{} sovereign node"

[[devices]]
id = "{}"
name = "{}"
type = "{}"
owner = "{}"
apn_node_id = "apn_{}"
serves_data = true

[devices.hardware]
cpu = "{}"
memory_gb = "{:.1}"
{}

[routing]
default_strategy = "primary_device"
fallback_strategy = "apn_cloud"
multi_device_orchestration = false

[routing.{}]
primary = "{}"
"#,
        config.device.device_name,
        config.network.relay_url,
        config.network.apn_enabled,
        config.user.username,
        config.device.device_id,
        config.storage.topsi_db_path.display(),
        config.storage.projects_path.display(),
        if config.device.device_type == "always_on" {
            "100%"
        } else {
            "<100%"
        },
        config.user.username,
        config.device.device_id,
        config.device.device_name,
        config.device.device_type,
        config.user.username,
        config.user.username.to_lowercase(),
        config.device.cpu_info,
        config.device.total_memory_gb,
        config
            .device
            .gpu_info
            .as_ref()
            .map(|gpu| format!("gpu = \"{}\"", gpu))
            .unwrap_or_default(),
        config.user.username,
        config.device.device_id,
    );

    std::fs::write("orcha_config.toml", orcha_config)?;

    Ok(())
}

async fn initialize_topsi_database(config: &SetupConfig) -> Result<()> {
    let db_url = format!(
        "sqlite://{}?mode=rwc",
        config.storage.topsi_db_path.display()
    );

    let pool = sqlx::SqlitePool::connect(&db_url).await?;

    // Check if database is already initialized (tables exist)
    let table_exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'"
    )
    .fetch_one(&pool)
    .await
    .unwrap_or((0,));

    if table_exists.0 == 0 {
        // Fresh database — run migrations
        sqlx::migrate!("../db/migrations")
            .run(&pool)
            .await
            .context("Failed to run Topsi database migrations")?;
    }
    // else: DB already initialized, skip migrations

    // Enable WAL mode for performance
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;

    pool.close().await;

    Ok(())
}

async fn create_user_account(config: &SetupConfig) -> Result<()> {
    let shared_db_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/duck-kanban/db.sqlite");

    if let Some(parent) = shared_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db_url = format!("sqlite://{}?mode=rwc", shared_db_path.display());
    let pool = sqlx::SqlitePool::connect(&db_url).await?;

    let user_id = uuid::Uuid::new_v4();

    sqlx::query(
        "INSERT OR IGNORE INTO users (id, username, password_hash, email, is_active, created_at)
         VALUES (?, ?, ?, ?, 1, datetime('now'))",
    )
    .bind(user_id.as_bytes().to_vec())
    .bind(&config.user.username)
    .bind(&config.user.password_hash)
    .bind(format!("{}@local", config.user.username))
    .execute(&pool)
    .await?;

    pool.close().await;

    Ok(())
}

async fn register_device(config: &SetupConfig) -> Result<()> {
    let shared_db_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/duck-kanban/db.sqlite");

    let db_url = format!("sqlite://{}?mode=rwc", shared_db_path.display());
    let pool = sqlx::SqlitePool::connect(&db_url).await?;

    sqlx::query(
        "INSERT OR REPLACE INTO device_registry
         (id, device_name, device_type, owner, is_online, last_seen, capabilities)
         VALUES (?, ?, ?, ?, 1, datetime('now'), ?)",
    )
    .bind(&config.device.device_id)
    .bind(&config.device.device_name)
    .bind(&config.device.device_type)
    .bind(&config.user.username)
    .bind(
        serde_json::json!({
            "cpu": config.device.cpu_info,
            "memory_gb": config.device.total_memory_gb,
            "gpu": config.device.gpu_info,
        })
        .to_string(),
    )
    .execute(&pool)
    .await?;

    pool.close().await;

    Ok(())
}

fn print_success(config: &SetupConfig) {
    println!("{}", "\n┌─────────────────────────────────────────────────────┐".bright_green());
    println!("{}", "│  🎉 Setup Complete!                                 │".bright_green());
    println!("{}", "└─────────────────────────────────────────────────────┘".bright_green());
    println!();
    println!("{}", "  Your ORCHA node is configured:".white().bold());
    println!("    • User: {}", config.user.username.bright_cyan());
    println!(
        "    • Device: {}",
        config.device.device_name.bright_cyan()
    );
    println!(
        "    • Projects: {}",
        config.storage.projects_path.display().to_string().bright_cyan()
    );
    println!(
        "    • APN: {}",
        if config.network.apn_enabled {
            "Connected".bright_green()
        } else {
            "Disabled".bright_yellow()
        }
    );
    println!();
    println!("{}", "  To start ORCHA:".white().bold());
    println!("    {}", "orcha start".bright_cyan());
    println!();
    println!(
        "{}",
        "  Or install as a system service:".white().bold()
    );
    println!("    {}", "sudo orcha install-service".bright_cyan());
    println!();
}

fn detect_gpu() -> Option<String> {
    // Try to detect NVIDIA GPU
    if let Ok(output) = std::process::Command::new("nvidia-smi")
        .arg("--query-gpu=name")
        .arg("--format=csv,noheader")
        .output()
    {
        if output.status.success() {
            if let Ok(gpu_name) = String::from_utf8(output.stdout) {
                return Some(gpu_name.trim().to_string());
            }
        }
    }

    // Try to detect AMD GPU (Linux)
    if let Ok(output) = std::process::Command::new("lspci")
        .output()
    {
        if output.status.success() {
            if let Ok(pci_info) = String::from_utf8(output.stdout) {
                for line in pci_info.lines() {
                    if line.contains("VGA") || line.contains("3D controller") {
                        if line.contains("AMD") || line.contains("Radeon") {
                            return Some(line.split(':').nth(2)?.trim().to_string());
                        }
                    }
                }
            }
        }
    }

    None
}
