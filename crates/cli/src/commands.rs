//! CLI subcommand handlers
//!
//! Handles non-interactive commands like status, tasks, projects, etc.

use anyhow::Result;
use colored::Colorize;

use crate::{
    api::{ApiClient, CreateTaskRequest},
    config::Config,
    output::OutputHandler,
};

/// Format a number with thousand separators
fn format_num(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

/// Show current session status
pub async fn status(api: &ApiClient) -> Result<()> {
    let output = OutputHandler::new(false, false);

    // Check server health
    let healthy = api.health_check().await?;

    output.print_header("PCG CLI Status");

    if healthy {
        output.print_success("Server: Connected");
    } else {
        output.print_error("Server: Not reachable");
    }

    // Show version info
    println!();
    println!("  {} {}", "Version:".dimmed(), env!("CARGO_PKG_VERSION"));
    println!(
        "  {} {}",
        "Config:".dimmed(),
        Config::config_path().display()
    );

    Ok(())
}

/// List all projects
pub async fn list_projects(api: &ApiClient) -> Result<()> {
    let output = OutputHandler::new(false, false);

    output.print_header("Projects");

    let projects = api.list_projects().await?;

    if projects.is_empty() {
        output.print_info("No projects found.");
        return Ok(());
    }

    let project_data: Vec<(String, String, i64, i64)> = projects
        .iter()
        .map(|p| {
            (
                p.id.to_string(),
                p.name.clone(),
                0i64, // Would need a separate API call for task count
                p.vibe_spent_amount,
            )
        })
        .collect();

    output.print_projects_table(&project_data);

    Ok(())
}

/// List tasks in a project
pub async fn list_tasks(
    api: &ApiClient,
    project: Option<&str>,
    status_filter: Option<&str>,
) -> Result<()> {
    let output = OutputHandler::new(false, false);

    // Find project
    let project_id = if let Some(name_or_id) = project {
        // Try parsing as UUID first
        if let Ok(id) = uuid::Uuid::parse_str(name_or_id) {
            id
        } else {
            // Look up by name
            let proj = api.find_project_by_name(name_or_id).await?;
            match proj {
                Some(p) => p.id,
                None => {
                    output.print_error(&format!("Project not found: {}", name_or_id));
                    return Ok(());
                }
            }
        }
    } else {
        output.print_error("Please specify a project with --project");
        return Ok(());
    };

    output.print_header(&format!(
        "Tasks{}",
        if let Some(s) = status_filter {
            format!(" (status: {})", s)
        } else {
            String::new()
        }
    ));

    let tasks = api.list_tasks(project_id, status_filter).await?;

    if tasks.is_empty() {
        output.print_info("No tasks found.");
        return Ok(());
    }

    let task_data: Vec<(String, String, String, String)> = tasks
        .iter()
        .map(|t| {
            (
                t.id.to_string(),
                t.title.clone(),
                t.status.clone(),
                t.updated_at
                    .clone()
                    .unwrap_or_else(|| "N/A".to_string())
                    .chars()
                    .take(10)
                    .collect(),
            )
        })
        .collect();

    output.print_tasks_table(&task_data);

    Ok(())
}

/// Create a new task
pub async fn create_task(
    api: &ApiClient,
    project: Option<&str>,
    title: &str,
    description: Option<&str>,
) -> Result<()> {
    let output = OutputHandler::new(false, false);

    // Find project
    let project_id = if let Some(name_or_id) = project {
        if let Ok(id) = uuid::Uuid::parse_str(name_or_id) {
            id
        } else {
            let proj = api.find_project_by_name(name_or_id).await?;
            match proj {
                Some(p) => p.id,
                None => {
                    output.print_error(&format!("Project not found: {}", name_or_id));
                    return Ok(());
                }
            }
        }
    } else {
        output.print_error("Please specify a project with --project");
        return Ok(());
    };

    let request = CreateTaskRequest {
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
        created_by: "pcg-cli".to_string(),
    };

    match api.create_task(project_id, None, &request).await {
        Ok(task) => {
            output.print_task("created", &task.title, Some(&task.id.to_string()));
        }
        Err(e) => {
            output.print_error(&format!("Failed to create task: {}", e));
        }
    }

    Ok(())
}

/// Show session history
pub async fn session_history(_api: &ApiClient, limit: usize) -> Result<()> {
    let output = OutputHandler::new(false, false);

    output.print_header(&format!("Recent Sessions (last {})", limit));

    // Note: This endpoint may not exist yet
    output.print_info("Session history will be available once DevelopmentSession is implemented.");
    output.print_info("See docs/DEVELOPMENT_WORKFLOW_INTEGRATION_PLAN.md for the roadmap.");

    Ok(())
}

/// Show cost breakdown
pub async fn show_cost(api: &ApiClient, _session: Option<&str>) -> Result<()> {
    let output = OutputHandler::new(false, false);

    output.print_header("Cost Summary");

    // Note: This would query the VIBE transaction history
    output.print_info("Cost tracking will be available once integrated with the session system.");

    // Show projects with VIBE spent as a fallback
    let projects = api.list_projects().await?;
    let total_vibe: i64 = projects.iter().map(|p| p.vibe_spent_amount).sum();

    println!();
    println!(
        "  {} {} VIBE (${:.2} USD)",
        "Total VIBE Spent:".bright_white(),
        format_num(total_vibe),
        total_vibe as f64 * 0.001
    );
    println!();

    for project in projects.iter().filter(|p| p.vibe_spent_amount > 0) {
        println!(
            "    {} {:>12} VIBE",
            format!("{}:", project.name).dimmed(),
            format_num(project.vibe_spent_amount)
        );
    }

    Ok(())
}

/// Show current configuration
pub fn show_config(config: &Config) -> Result<()> {
    let output = OutputHandler::new(false, false);

    output.print_header("Configuration");

    println!();
    println!("  {}", "[server]".bright_cyan());
    println!("    {} = \"{}\"", "url".dimmed(), config.server.url);
    println!(
        "    {} = {}",
        "api_key".dimmed(),
        if config.server.api_key.is_some() {
            "\"***\"".to_string()
        } else {
            "not set".dimmed().to_string()
        }
    );

    println!();
    println!("  {}", "[session]".bright_cyan());
    println!(
        "    {} = {}",
        "auto_create_tasks".dimmed(),
        config.session.auto_create_tasks
    );
    println!(
        "    {} = {}",
        "default_project".dimmed(),
        config
            .session
            .default_project
            .as_deref()
            .unwrap_or("not set")
    );

    println!();
    println!("  {}", "[display]".bright_cyan());
    println!("    {} = \"{}\"", "theme".dimmed(), config.display.theme);
    println!(
        "    {} = {}",
        "show_cost_bar".dimmed(),
        config.display.show_cost_bar
    );

    println!();
    println!("  {}", "[agents]".bright_cyan());
    println!("    {} = \"{}\"", "default".dimmed(), config.agents.default);

    println!();
    println!(
        "  {} {}",
        "Config file:".dimmed(),
        Config::config_path().display()
    );

    Ok(())
}

/// Set a configuration value
pub fn set_config(kv: &str) -> Result<()> {
    let output = OutputHandler::new(false, false);

    let parts: Vec<&str> = kv.splitn(2, '=').collect();
    if parts.len() != 2 {
        output.print_error("Invalid format. Use: key=value");
        return Ok(());
    }

    let key = parts[0].trim();
    let value = parts[1].trim().trim_matches('"');

    let mut config = Config::load()?;
    match config.set(key, value) {
        Ok(()) => {
            output.print_success(&format!("Set {} = \"{}\"", key, value));
        }
        Err(e) => {
            output.print_error(&format!("Failed to set config: {}", e));
        }
    }

    Ok(())
}
