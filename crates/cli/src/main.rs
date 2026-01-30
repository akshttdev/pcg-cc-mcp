//! PCG CLI - Interactive Development Session Management
//!
//! Provides a Claude Code-like terminal experience integrated with PCG Dashboard
//! for task tracking, agent coordination, and VIBE cost management.

mod api;
mod commands;
mod config;
mod output;
mod repl;
mod session;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// PCG CLI - AI-Native Development Assistant
#[derive(Parser)]
#[command(name = "pcg")]
#[command(author = "PCG Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Interactive development session with task tracking and agent coordination")]
#[command(long_about = r#"
PCG CLI provides a Claude Code-like terminal experience integrated with PCG Dashboard.

Features:
  - Interactive REPL for AI-assisted development
  - Automatic task creation and tracking
  - Real-time VIBE cost monitoring
  - Multi-agent coordination (Duck, Nora, Scout, etc.)
  - Session history and reports

Examples:
  pcg                          # Start session in current directory
  pcg --project "My Project"   # Start with specific project
  pcg status                   # Show current session status
  pcg tasks                    # List tasks in current project
"#)]
struct Cli {
    /// Project name or ID to work with
    #[arg(short, long, env = "PCG_PROJECT")]
    project: Option<String>,

    /// Server URL
    #[arg(long, env = "PCG_SERVER_URL", default_value = "http://localhost:3002")]
    server: String,

    /// Working directory (defaults to current directory)
    #[arg(short = 'd', long)]
    directory: Option<PathBuf>,

    /// Resume a previous session by ID
    #[arg(long)]
    resume: Option<String>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current session status
    Status,

    /// List projects
    Projects,

    /// List tasks in current project
    Tasks {
        /// Filter by status (todo, inprogress, done)
        #[arg(short, long)]
        status: Option<String>,
    },

    /// Create a new task
    Task {
        /// Task title
        title: String,

        /// Task description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Show session history
    History {
        /// Number of sessions to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show cost breakdown
    Cost {
        /// Show costs for specific session
        #[arg(long)]
        session: Option<String>,
    },

    /// Configuration management
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Set a configuration value (key=value)
        #[arg(long)]
        set: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("pcg_cli={},warn", log_level).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    // Load configuration
    let config = config::Config::load()?;
    let server_url = cli.server.clone();

    // Determine working directory
    let work_dir = cli
        .directory
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Create API client
    let api = api::ApiClient::new(&server_url);

    // Handle subcommands or start REPL
    match cli.command {
        Some(Commands::Status) => {
            commands::status(&api).await?;
        }
        Some(Commands::Projects) => {
            commands::list_projects(&api).await?;
        }
        Some(Commands::Tasks { status }) => {
            commands::list_tasks(&api, cli.project.as_deref(), status.as_deref()).await?;
        }
        Some(Commands::Task { title, description }) => {
            commands::create_task(&api, cli.project.as_deref(), &title, description.as_deref())
                .await?;
        }
        Some(Commands::History { limit }) => {
            commands::session_history(&api, limit).await?;
        }
        Some(Commands::Cost { session }) => {
            commands::show_cost(&api, session.as_deref()).await?;
        }
        Some(Commands::Config { show, set }) => {
            if show {
                commands::show_config(&config)?;
            } else if let Some(kv) = set {
                commands::set_config(&kv)?;
            } else {
                commands::show_config(&config)?;
            }
        }
        None => {
            // Start interactive REPL
            let mut repl = repl::PcgRepl::new(api, config, work_dir, cli.project, cli.resume)?;
            repl.run().await?;
        }
    }

    Ok(())
}
