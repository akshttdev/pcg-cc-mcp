//! Interactive REPL for PCG CLI
//!
//! Provides the Claude Code-like interactive terminal experience.

use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use rustyline::{error::ReadlineError, history::DefaultHistory, Editor};
use uuid::Uuid;

use crate::{
    api::{ApiClient, CreateTaskRequest, UpdateTaskRequest},
    config::Config,
    output::OutputHandler,
    session::DevSession,
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

/// Interactive REPL for PCG CLI
pub struct PcgRepl {
    api: ApiClient,
    config: Config,
    work_dir: PathBuf,
    session: Option<DevSession>,
    output: OutputHandler,
    editor: Editor<(), DefaultHistory>,
    project_id: Option<Uuid>,
    project_name: Option<String>,
}

impl PcgRepl {
    pub fn new(
        api: ApiClient,
        config: Config,
        work_dir: PathBuf,
        project: Option<String>,
        resume_session: Option<String>,
    ) -> Result<Self> {
        let output = OutputHandler::new(config.display.show_cost_bar, config.display.markdown_rendering);

        let editor = Editor::new()?;

        let mut repl = Self {
            api,
            config,
            work_dir,
            session: None,
            output,
            editor,
            project_id: None,
            project_name: project,
        };

        // If resuming, parse session ID
        if let Some(session_id) = resume_session {
            if let Ok(id) = Uuid::parse_str(&session_id) {
                let resumed = tokio::runtime::Handle::current().block_on(async {
                    DevSession::resume(&repl.api, id).await
                });
                match resumed {
                    Ok(session) => repl.session = Some(session),
                    Err(e) => panic!("Failed to resume session {}: {}", session_id, e),
                }
            }
        }

        Ok(repl)
    }

    /// Run the interactive REPL
    pub async fn run(&mut self) -> Result<()> {
        // Initialize project
        self.init_project().await?;

        // Start or resume session
        self.init_session().await?;

        // Display welcome banner
        self.output.print_banner(
            self.project_name.as_deref(),
            &self.session.as_ref().map(|s| s.id.to_string()).unwrap_or_default(),
        );

        // Main REPL loop
        loop {
            let prompt = self.build_prompt();

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let input = line.trim();

                    if input.is_empty() {
                        continue;
                    }

                    // Add to history
                    let _ = self.editor.add_history_entry(input);

                    // Handle commands
                    if input.starts_with('/') {
                        match self.handle_command(input).await {
                            Ok(should_exit) => {
                                if should_exit {
                                    break;
                                }
                            }
                            Err(e) => {
                                self.output.print_error(&format!("Command error: {}", e));
                            }
                        }
                    } else {
                        // Process as natural language input
                        if let Err(e) = self.process_input(input).await {
                            self.output.print_error(&format!("Error: {}", e));
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!();
                    self.output.print_info("Use /exit to quit or /session complete to save session.");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!();
                    break;
                }
                Err(e) => {
                    self.output.print_error(&format!("Input error: {}", e));
                    break;
                }
            }
        }

        // Prompt to save session
        if self.session.is_some() {
            self.output.print_info("Session ended. Use 'pcg status' to view history.");
        }

        Ok(())
    }

    /// Initialize project from config or command line
    async fn init_project(&mut self) -> Result<()> {
        // Try to find project
        let project_name = self
            .project_name
            .clone()
            .or_else(|| self.config.session.default_project.clone());

        if let Some(name) = project_name {
            match self.api.find_project_by_name(&name).await {
                Ok(Some(project)) => {
                    self.project_id = Some(project.id);
                    self.project_name = Some(project.name);
                }
                Ok(None) => {
                    self.output
                        .print_warning(&format!("Project '{}' not found. Running without project context.", name));
                }
                Err(e) => {
                    self.output
                        .print_warning(&format!("Could not load project: {}. Running without project context.", e));
                }
            }
        }

        Ok(())
    }

    /// Initialize or resume session
    async fn init_session(&mut self) -> Result<()> {
        if self.session.is_some() {
            return Ok(()); // Already have a session (resumed)
        }

        if let Some(project_id) = self.project_id {
            let title = format!("Development Session - {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));

            self.session = Some(
                DevSession::start(
                    &self.api,
                    project_id,
                    self.project_name.as_deref().unwrap_or("Unknown"),
                    &title,
                    &self.work_dir,
                )
                .await?,
            );
        }

        Ok(())
    }

    /// Build the prompt string
    fn build_prompt(&self) -> String {
        let project_part = self
            .project_name
            .as_ref()
            .map(|n| format!("{}", n.bright_cyan()))
            .unwrap_or_else(|| "no project".dimmed().to_string());

        let session_part = self
            .session
            .as_ref()
            .map(|s| {
                let metrics = s.get_metrics();
                format!(
                    " {} VIBE",
                    format_num(metrics.total_vibe_cost).bright_yellow()
                )
            })
            .unwrap_or_default();

        format!(
            "\n{} [{}{}] {} ",
            "pcg".bright_green().bold(),
            project_part,
            session_part,
            ">".bright_green()
        )
    }

    /// Handle slash commands
    async fn handle_command(&mut self, input: &str) -> Result<bool> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts.first().unwrap_or(&"");

        match *command {
            "/exit" | "/quit" | "/q" => {
                return Ok(true);
            }

            "/help" | "/h" | "/?" => {
                self.print_help();
            }

            "/task" => {
                self.handle_task_command(&parts[1..]).await?;
            }

            "/tasks" => {
                self.handle_list_tasks().await?;
            }

            "/agent" => {
                self.handle_agent_command(&parts[1..]).await?;
            }

            "/cost" => {
                self.print_cost_summary();
            }

            "/session" => {
                self.handle_session_command(&parts[1..]).await?;
            }

            "/project" => {
                self.handle_project_command(&parts[1..]).await?;
            }

            "/clear" => {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
            }

            _ => {
                self.output.print_error(&format!("Unknown command: {}. Use /help for available commands.", command));
            }
        }

        Ok(false)
    }

    /// Print help information
    fn print_help(&self) {
        println!();
        println!("{}", "PCG CLI Commands".bright_white().bold());
        println!("{}", "─".repeat(50).dimmed());
        println!();

        println!("{}", "Session Commands:".bright_cyan());
        println!("  {}         Show current session info", "/session".bright_yellow());
        println!("  {}    Pause current session", "/session pause".bright_yellow());
        println!("  {} Complete session and generate report", "/session complete".bright_yellow());
        println!();

        println!("{}", "Task Commands:".bright_cyan());
        println!("  {}            List tasks in current project", "/tasks".bright_yellow());
        println!(
            "  {} Create a new task",
            "/task create <title>".bright_yellow()
        );
        println!(
            "  {}   Mark task as done",
            "/task complete <id>".bright_yellow()
        );
        println!(
            "  {} Link task to current session",
            "/task link <id>".bright_yellow()
        );
        println!();

        println!("{}", "Agent Commands:".bright_cyan());
        println!("  {}          List available agents", "/agent list".bright_yellow());
        println!(
            "  {}  Switch primary agent",
            "/agent switch <name>".bright_yellow()
        );
        println!();

        println!("{}", "Other Commands:".bright_cyan());
        println!("  {}             Show cost breakdown", "/cost".bright_yellow());
        println!("  {}           Show current project", "/project".bright_yellow());
        println!("  {}            Clear screen", "/clear".bright_yellow());
        println!("  {}             Show this help", "/help".bright_yellow());
        println!("  {}             Exit the CLI", "/exit".bright_yellow());
        println!();
    }

    /// Handle task subcommands
    async fn handle_task_command(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            self.output.print_error("Usage: /task <create|complete|link> [args]");
            return Ok(());
        }

        match args[0] {
            "create" => {
                if args.len() < 2 {
                    self.output.print_error("Usage: /task create <title>");
                    return Ok(());
                }

                let title = args[1..].join(" ");

                if let Some(project_id) = self.project_id {
                    let request = CreateTaskRequest {
                        title: title.clone(),
                        description: None,
                        created_by: "pcg-cli".to_string(),
                    };

                    match self.api.create_task(project_id, None, &request).await {
                        Ok(task) => {
                            self.output.print_task("created", &task.title, Some(&task.id.to_string()));

                            // Record in session
                            if let Some(session) = &self.session {
                                session.record_task_created(task.id);
                            }
                        }
                        Err(e) => {
                            self.output.print_error(&format!("Failed to create task: {}", e));
                        }
                    }
                } else {
                    self.output.print_error("No project selected. Use /project <name> first.");
                }
            }

            "complete" => {
                if args.len() < 2 {
                    self.output.print_error("Usage: /task complete <id>");
                    return Ok(());
                }

                let task_id = args[1];
                if let Ok(id) = Uuid::parse_str(task_id) {
                    let update = UpdateTaskRequest {
                        title: None,
                        description: None,
                        status: Some("done".to_string()),
                    };

                    match self.api.update_task(id, &update).await {
                        Ok(task) => {
                            self.output.print_task("completed", &task.title, Some(&task.id.to_string()));

                            // Record in session
                            if let Some(session) = &self.session {
                                session.record_task_completed();
                            }
                        }
                        Err(e) => {
                            self.output.print_error(&format!("Failed to complete task: {}", e));
                        }
                    }
                } else {
                    self.output.print_error("Invalid task ID format.");
                }
            }

            "link" => {
                self.output.print_info("Task linking will be available once DevelopmentSession is fully implemented.");
            }

            _ => {
                self.output.print_error("Unknown task command. Use: create, complete, link");
            }
        }

        Ok(())
    }

    /// List tasks
    async fn handle_list_tasks(&mut self) -> Result<()> {
        if let Some(project_id) = self.project_id {
            let tasks = self.api.list_tasks(project_id, None).await?;

            if tasks.is_empty() {
                self.output.print_info("No tasks found.");
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

            self.output.print_tasks_table(&task_data);
        } else {
            self.output.print_error("No project selected. Use /project <name> first.");
        }

        Ok(())
    }

    /// Handle agent subcommands
    async fn handle_agent_command(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() || args[0] == "list" {
            let agents = self.api.list_agents().await?;

            self.output.print_header("Available Agents");

            for agent in agents {
                println!(
                    "  {} - {} {}",
                    agent.short_name.bright_cyan(),
                    agent.designation.bright_white(),
                    if self.config.agents.default == agent.short_name {
                        "(default)".bright_green().to_string()
                    } else {
                        String::new()
                    }
                );
            }

            return Ok(());
        }

        match args[0] {
            "switch" => {
                if args.len() < 2 {
                    self.output.print_error("Usage: /agent switch <name>");
                    return Ok(());
                }

                let name = args[1];
                self.config.agents.default = name.to_string();
                self.output.print_success(&format!("Switched to agent: {}", name));
            }

            _ => {
                self.output.print_error("Unknown agent command. Use: list, switch");
            }
        }

        Ok(())
    }

    /// Print cost summary
    fn print_cost_summary(&self) {
        if let Some(session) = &self.session {
            let metrics = session.get_metrics();

            self.output.print_header("Session Cost Summary");

            println!();
            println!(
                "  {} {}",
                "Tokens Used:".dimmed(),
                format_num(metrics.total_tokens)
            );
            println!(
                "  {} {} VIBE (${:.2} USD)",
                "VIBE Cost:".dimmed(),
                format_num(metrics.total_vibe_cost),
                metrics.total_vibe_cost as f64 * 0.001
            );
            println!(
                "  {} {}",
                "Duration:".dimmed(),
                session.duration_string()
            );
            println!(
                "  {} {} created, {} completed",
                "Tasks:".dimmed(),
                metrics.tasks_created,
                metrics.tasks_completed
            );
            println!();
        } else {
            self.output.print_info("No active session.");
        }
    }

    /// Handle session subcommands
    async fn handle_session_command(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            // Show session info
            if let Some(session) = &self.session {
                self.output.print_header("Current Session");

                println!();
                println!("  {} {}", "ID:".dimmed(), &session.id.to_string()[..8]);
                println!("  {} {}", "Project:".dimmed(), session.project_name);
                println!("  {} {}", "Started:".dimmed(), session.started_at.format("%Y-%m-%d %H:%M"));
                println!("  {} {}", "Duration:".dimmed(), session.duration_string());

                if let Some(branch) = &session.git_branch {
                    println!("  {} {}", "Git Branch:".dimmed(), branch);
                }
                if let Some(sha) = &session.git_start_sha {
                    println!("  {} {}", "Start SHA:".dimmed(), sha);
                }

                let metrics = session.get_metrics();
                println!();
                println!("  {} {}", "Tokens:".dimmed(), format_num(metrics.total_tokens));
                println!("  {} {}", "VIBE:".dimmed(), format_num(metrics.total_vibe_cost));
                println!(
                    "  {} {} created, {} completed",
                    "Tasks:".dimmed(),
                    metrics.tasks_created,
                    metrics.tasks_completed
                );
                println!();
            } else {
                self.output.print_info("No active session.");
            }
            return Ok(());
        }

        match args[0] {
            "complete" => {
                if let Some(session) = &self.session {
                    let report = session.complete(&self.api, &self.work_dir).await?;

                    // Display report
                    let duration = format!("{}m", report.duration_minutes);
                    let tasks: Vec<(String, String)> = vec![]; // Would need to fetch linked tasks

                    self.output.print_session_report(
                        &duration,
                        report.files_changed,
                        report.lines_added,
                        report.lines_removed,
                        0, // commits - would need git integration
                        report.total_tokens,
                        report.total_vibe_cost,
                        report.total_vibe_cost as f64 * 0.001,
                        &tasks,
                    );

                    self.session = None;
                } else {
                    self.output.print_info("No active session to complete.");
                }
            }

            "pause" => {
                self.output.print_info("Session paused. Resume later with: pcg --resume <session-id>");
            }

            _ => {
                self.output.print_error("Unknown session command. Use: complete, pause");
            }
        }

        Ok(())
    }

    /// Handle project subcommands
    async fn handle_project_command(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            if let Some(name) = &self.project_name {
                println!();
                println!("  {} {}", "Current Project:".dimmed(), name.bright_cyan());
                if let Some(id) = &self.project_id {
                    println!("  {} {}", "ID:".dimmed(), id);
                }
                println!();
            } else {
                self.output.print_info("No project selected. Use /project <name> to select one.");
            }
            return Ok(());
        }

        // Switch project
        let name = args.join(" ");
        match self.api.find_project_by_name(&name).await {
            Ok(Some(project)) => {
                self.project_id = Some(project.id);
                self.project_name = Some(project.name.clone());
                self.output.print_success(&format!("Switched to project: {}", project.name));

                // Start new session for this project
                self.session = None;
                self.init_session().await?;
            }
            Ok(None) => {
                self.output.print_error(&format!("Project not found: {}", name));
            }
            Err(e) => {
                self.output.print_error(&format!("Error finding project: {}", e));
            }
        }

        Ok(())
    }

    /// Process natural language input
    async fn process_input(&mut self, input: &str) -> Result<()> {
        // For now, route to the default agent
        // In the future, this would classify intent and route accordingly

        self.output.print_info("Processing...");
        println!();

        // Check if we should auto-create a task
        if self.config.session.auto_create_tasks {
            // Simple heuristic: if input looks like a task request, create a task
            let task_indicators = [
                "add", "implement", "create", "build", "fix", "update", "refactor",
                "remove", "delete", "change", "modify",
            ];

            let input_lower = input.to_lowercase();
            let is_task_request = task_indicators.iter().any(|ind| input_lower.starts_with(ind));

            if is_task_request {
                if let Some(project_id) = self.project_id {
                    let request = CreateTaskRequest {
                        title: input.to_string(),
                        description: None,
                        created_by: "pcg-cli".to_string(),
                    };

                    if let Ok(task) = self.api.create_task(project_id, None, &request).await {
                        self.output.print_task("created", &task.title, Some(&task.id.to_string()));

                        if let Some(session) = &self.session {
                            session.record_task_created(task.id);
                        }
                    }
                }
            }
        }

        // Try to chat with an agent
        let agent_name = &self.config.agents.default;

        if let Ok(Some(agent)) = self.api.get_agent_by_name(agent_name).await {
            let session_id = self
                .session
                .as_ref()
                .map(|s| s.id.to_string())
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            match self
                .api
                .chat_with_agent(agent.id, input, &session_id, self.project_id)
                .await
            {
                Ok(response) => {
                    // Update session metrics
                    if let Some(session) = &self.session {
                        let tokens = response.input_tokens.unwrap_or(0) + response.output_tokens.unwrap_or(0);
                        let vibe = (tokens as f64 * 0.05) as i64; // Rough VIBE estimate
                        session.update_cost(tokens, vibe);
                    }

                    // Display response
                    self.output.print_response(&response.content);

                    // Show status bar
                    if let Some(session) = &self.session {
                        let metrics = session.get_metrics();
                        self.output.print_status_bar(
                            metrics.total_tokens,
                            metrics.total_vibe_cost,
                            metrics.tasks_created,
                            metrics.tasks_completed,
                        );
                    }
                }
                Err(e) => {
                    self.output.print_error(&format!("Agent error: {}", e));
                    self.output.print_info(
                        "Note: Make sure the PCG backend server is running on the configured URL.",
                    );
                }
            }
        } else {
            // Fallback: just echo the input for now
            self.output.print_warning(&format!(
                "Agent '{}' not available. Running in offline mode.",
                agent_name
            ));
            self.output.print_info(&format!("Your request: {}", input));
            self.output.print_info(
                "To enable agent chat, ensure the PCG backend is running: pnpm run dev",
            );
        }

        Ok(())
    }
}
