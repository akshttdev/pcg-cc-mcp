//! Interactive REPL for Topsi CLI
//!
//! Provides the Claude Code-like interactive terminal experience.

use std::path::PathBuf;

use anyhow::Result;
use colored::Colorize;
use rustyline::{error::ReadlineError, history::DefaultHistory, Editor};
use uuid::Uuid;

use crate::{
    api::{ApiClient, CreateTaskRequest, RouterModel, UpdateTaskRequest},
    config::Config,
    output::OutputHandler,
    session::{ConversationLog, DevSession},
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

/// Interactive REPL for Topsi CLI
pub struct TopsiRepl {
    api: ApiClient,
    config: Config,
    work_dir: PathBuf,
    session: Option<DevSession>,
    output: OutputHandler,
    editor: Editor<(), DefaultHistory>,
    project_id: Option<Uuid>,
    project_name: Option<String>,
    /// Current model override (None = use agent default)
    current_model: Option<String>,
    /// Current provider override (None = use agent default)
    current_provider: Option<String>,
    /// Persistent conversation log saved on exit
    conversation_log: ConversationLog,
}

impl TopsiRepl {
    pub fn new(
        api: ApiClient,
        config: Config,
        work_dir: PathBuf,
        project: Option<String>,
        resume_session: Option<String>,
    ) -> Result<Self> {
        let output = OutputHandler::new(
            config.display.show_cost_bar,
            config.display.markdown_rendering,
        );

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
            current_model: None,
            current_provider: None,
            conversation_log: ConversationLog::new(None),
        };

        // If resuming, parse session ID
        if let Some(session_id) = resume_session {
            if let Ok(id) = Uuid::parse_str(&session_id) {
                let resumed = tokio::runtime::Handle::current()
                    .block_on(async { DevSession::resume(&repl.api, id).await });
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

        // Now that project name is known, update the conversation log
        self.conversation_log = ConversationLog::new(self.project_name.clone());

        // Display welcome banner
        self.output.print_banner(
            self.project_name.as_deref(),
            &self
                .session
                .as_ref()
                .map(|s| s.id.to_string())
                .unwrap_or_default(),
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
                    self.output
                        .print_info("Use /exit to quit or /session complete to save session.");
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

        // Save conversation log
        if !self.conversation_log.is_empty() {
            match self.conversation_log.save() {
                Ok(()) => self.output.print_info(&format!(
                    "Session saved ({} messages). View with: topsi history",
                    self.conversation_log.len()
                )),
                Err(e) => self
                    .output
                    .print_warning(&format!("Could not save session log: {}", e)),
            }
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
                    self.output.print_warning(&format!(
                        "Project '{}' not found. Running without project context.",
                        name
                    ));
                }
                Err(e) => {
                    self.output.print_warning(&format!(
                        "Could not load project: {}. Running without project context.",
                        e
                    ));
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
            let title = format!(
                "Development Session - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M")
            );

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

        let model_part = self
            .current_provider
            .as_ref()
            .map(|p| format!(" {}", p.bright_magenta()))
            .unwrap_or_default();

        format!(
            "\n{} [{}{}{}] {} ",
            "topsi".bright_green().bold(),
            project_part,
            session_part,
            model_part,
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

            "/model" => {
                self.handle_model_command(&parts[1..]).await?;
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

            "/boards" | "/board" => {
                self.handle_boards_command().await?;
            }

            "/history" => {
                self.handle_history_command(&parts[1..]).await?;
            }

            "/clear" => {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen
            }

            _ => {
                self.output.print_error(&format!(
                    "Unknown command: {}. Use /help for available commands.",
                    command
                ));
            }
        }

        Ok(false)
    }

    /// Print help information
    fn print_help(&self) {
        println!();
        println!("{}", "Topsi CLI Commands".bright_white().bold());
        println!("{}", "─".repeat(50).dimmed());
        println!();

        println!("{}", "Session Commands:".bright_cyan());
        println!(
            "  {}         Show current session info",
            "/session".bright_yellow()
        );
        println!(
            "  {}    Pause current session",
            "/session pause".bright_yellow()
        );
        println!(
            "  {} Complete session and generate report",
            "/session complete".bright_yellow()
        );
        println!();

        println!("{}", "Task Commands:".bright_cyan());
        println!(
            "  {}            List tasks in current project",
            "/tasks".bright_yellow()
        );
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
        println!(
            "  {}          List available agents",
            "/agent list".bright_yellow()
        );
        println!(
            "  {}  Switch primary agent",
            "/agent switch <name>".bright_yellow()
        );
        println!();

        println!("{}", "Model Commands:".bright_cyan());
        println!(
            "  {}                  List PCG Router models",
            "/model".bright_yellow()
        );
        println!(
            "  {}   Pick a model (substring matches model_id or name)",
            "/model <query>".bright_yellow()
        );
        println!(
            "  {}          Clear override; use Topsi's configured default",
            "/model default".bright_yellow()
        );
        println!();

        println!("{}", "Project Commands:".bright_cyan());
        println!(
            "  {}           Show current project",
            "/project".bright_yellow()
        );
        println!(
            "  {}      List all projects",
            "/project list".bright_yellow()
        );
        println!(
            "  {} Create a new project",
            "/project create <name>".bright_yellow()
        );
        println!(
            "  {}    Switch to a project",
            "/project <name>".bright_yellow()
        );
        println!();

        println!("{}", "Board Commands:".bright_cyan());
        println!(
            "  {}           Task board for current project",
            "/boards".bright_yellow()
        );
        println!(
            "  {}          Show recent session history",
            "/history".bright_yellow()
        );
        println!();

        println!("{}", "Other Commands:".bright_cyan());
        println!(
            "  {}             Show cost breakdown",
            "/cost".bright_yellow()
        );
        println!("  {}            Clear screen", "/clear".bright_yellow());
        println!("  {}             Show this help", "/help".bright_yellow());
        println!("  {}             Exit the CLI", "/exit".bright_yellow());
        println!();
    }

    /// Handle task subcommands
    async fn handle_task_command(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() {
            self.output
                .print_error("Usage: /task <create|complete|link> [args]");
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
                        created_by: "topsi-cli".to_string(),
                    };

                    match self.api.create_task(project_id, None, &request).await {
                        Ok(task) => {
                            self.output.print_task(
                                "created",
                                &task.title,
                                Some(&task.id.to_string()),
                            );

                            // Record in session
                            if let Some(session) = &self.session {
                                session.record_task_created(task.id);
                            }
                        }
                        Err(e) => {
                            self.output
                                .print_error(&format!("Failed to create task: {}", e));
                        }
                    }
                } else {
                    self.output
                        .print_error("No project selected. Use /project <name> first.");
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
                            self.output.print_task(
                                "completed",
                                &task.title,
                                Some(&task.id.to_string()),
                            );

                            // Record in session
                            if let Some(session) = &self.session {
                                session.record_task_completed();
                            }
                        }
                        Err(e) => {
                            self.output
                                .print_error(&format!("Failed to complete task: {}", e));
                        }
                    }
                } else {
                    self.output.print_error("Invalid task ID format.");
                }
            }

            "link" => {
                self.output.print_info(
                    "Task linking will be available once DevelopmentSession is fully implemented.",
                );
            }

            _ => {
                self.output
                    .print_error("Unknown task command. Use: create, complete, link");
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
            self.output
                .print_error("No project selected. Use /project <name> first.");
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
                self.output
                    .print_success(&format!("Switched to agent: {}", name));
            }

            _ => {
                self.output
                    .print_error("Unknown agent command. Use: list, switch");
            }
        }

        Ok(())
    }

    /// Handle model subcommands
    /// `/model` slash-command handler — surfaces PCG Router-registered models.
    ///
    /// Usage:
    ///   `/model`            — list available models with the current override
    ///                         highlighted, then return
    ///   `/model <query>`    — set override; query is matched as a substring of
    ///                         `model_id` (case-insensitive). Exact match wins;
    ///                         otherwise a single substring hit is accepted.
    ///                         Multiple matches print the disambiguation list
    ///   `/model default`    — clear the override (Topsi uses its configured
    ///   `/model reset`        default LLM)
    async fn handle_model_command(&mut self, args: &[&str]) -> Result<()> {
        if args.is_empty() || args[0] == "list" {
            return self.render_model_list().await;
        }

        if matches!(args[0], "default" | "reset" | "clear") {
            self.current_model = None;
            self.current_provider = None;
            self.output
                .print_success("Model override cleared. Topsi will use its configured default.");
            return Ok(());
        }

        let query = args[0];
        let models = match self.api.list_router_models().await {
            Ok(m) => m,
            Err(e) => {
                self.output
                    .print_error(&format!("Could not fetch router models: {}", e));
                return Ok(());
            }
        };

        let enabled: Vec<&RouterModel> = models.iter().filter(|m| m.is_enabled).collect();
        let lower = query.to_lowercase();

        // Exact match on model_id wins
        let exact: Vec<&&RouterModel> = enabled
            .iter()
            .filter(|m| m.model_id.eq_ignore_ascii_case(query))
            .collect();

        let chosen: Option<&RouterModel> = if let Some(&m) = exact.first() {
            Some(*m)
        } else {
            let matches: Vec<&&RouterModel> = enabled
                .iter()
                .filter(|m| {
                    m.model_id.to_lowercase().contains(&lower)
                        || m.name.to_lowercase().contains(&lower)
                })
                .collect();

            match matches.len() {
                0 => {
                    self.output
                        .print_error(&format!("No router model matches '{}'", query));
                    self.output
                        .print_info("Run `/model` with no args to list available models.");
                    return Ok(());
                }
                1 => Some(*matches[0]),
                _ => {
                    self.output.print_warning(&format!(
                        "'{}' matched {} models — be more specific:",
                        query,
                        matches.len()
                    ));
                    for m in matches {
                        println!(
                            "  {} {}  ({})",
                            "•".dimmed(),
                            m.model_id.bright_cyan(),
                            m.provider.dimmed()
                        );
                    }
                    return Ok(());
                }
            }
        };

        if let Some(m) = chosen {
            self.current_model = Some(m.model_id.clone());
            self.current_provider = Some(m.provider.clone());
            self.output.print_success(&format!(
                "Model override set: {} ({})",
                m.model_id.bright_cyan(),
                m.provider.dimmed()
            ));
        }
        Ok(())
    }

    /// Fetch and render the PCG Router model list. Marks the current override.
    async fn render_model_list(&self) -> Result<()> {
        self.output.print_header("PCG Router Models");

        let models = match self.api.list_router_models().await {
            Ok(m) => m,
            Err(e) => {
                self.output
                    .print_error(&format!("Could not fetch router models: {}", e));
                self.output.print_info(
                    "Check that the PCG backend is reachable and that you are authenticated.",
                );
                return Ok(());
            }
        };

        if models.is_empty() {
            self.output
                .print_warning("No models registered in PCG Router.");
            return Ok(());
        }

        let current = self.current_model.as_deref();
        println!();
        println!(
            "  {:>3}  {:<32} {:<12} {:>10}  {:>12}  flags",
            "#".dimmed(),
            "model_id".bright_white(),
            "provider".bright_white(),
            "ctx".bright_white(),
            "$/M in→out".bright_white(),
        );
        for (idx, m) in models.iter().enumerate() {
            let marker = if Some(m.model_id.as_str()) == current {
                "▶".bright_green().to_string()
            } else {
                " ".to_string()
            };
            let ctx = m
                .context_window
                .map(|n| format_num(n))
                .unwrap_or_else(|| "—".to_string());
            let cost = format!(
                "{}→{}",
                format_num(m.cost_per_million_input),
                format_num(m.cost_per_million_output)
            );
            let mut flags = Vec::new();
            if !m.is_enabled {
                flags.push("disabled".dimmed().to_string());
            }
            if m.supports_tools {
                flags.push("tools".bright_blue().to_string());
            }
            if m.supports_vision {
                flags.push("vision".bright_magenta().to_string());
            }

            let name = if m.model_id == m.name {
                String::new()
            } else {
                format!("  {}", m.name.dimmed())
            };

            println!(
                "  {} {:>2}  {:<32} {:<12} {:>10}  {:>12}  {}{}",
                marker,
                idx + 1,
                m.model_id.bright_cyan(),
                m.provider,
                ctx,
                cost,
                flags.join(" "),
                name,
            );
        }
        println!();
        println!(
            "{}  {}  {}",
            "Pick:".dimmed(),
            "/model <id-or-substring>".bright_yellow(),
            "or /model default to clear".dimmed()
        );
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
                metrics.total_vibe_cost as f64 * 0.01
            );
            println!("  {} {}", "Duration:".dimmed(), session.duration_string());
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
                println!(
                    "  {} {}",
                    "Started:".dimmed(),
                    session.started_at.format("%Y-%m-%d %H:%M")
                );
                println!("  {} {}", "Duration:".dimmed(), session.duration_string());

                if let Some(branch) = &session.git_branch {
                    println!("  {} {}", "Git Branch:".dimmed(), branch);
                }
                if let Some(sha) = &session.git_start_sha {
                    println!("  {} {}", "Start SHA:".dimmed(), sha);
                }

                let metrics = session.get_metrics();
                println!();
                println!(
                    "  {} {}",
                    "Tokens:".dimmed(),
                    format_num(metrics.total_tokens)
                );
                println!(
                    "  {} {}",
                    "VIBE:".dimmed(),
                    format_num(metrics.total_vibe_cost)
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
                        report.total_vibe_cost as f64 * 0.01,
                        &tasks,
                    );

                    self.session = None;
                } else {
                    self.output.print_info("No active session to complete.");
                }
            }

            "pause" => {
                self.output
                    .print_info("Session paused. Resume later with: topsi --resume <session-id>");
            }

            _ => {
                self.output
                    .print_error("Unknown session command. Use: complete, pause");
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
                self.output
                    .print_info("No project selected. Use /project <name> to select one.");
            }
            return Ok(());
        }

        match args[0] {
            "create" => {
                if args.len() < 2 {
                    self.output.print_error("Usage: /project create <name>");
                    return Ok(());
                }
                let name = args[1..].join(" ");

                // Use work_dir as default git repo path for the new project
                let git_path = self.work_dir.join(name.replace(' ', "-").to_lowercase());
                let git_path_str = git_path.to_string_lossy().to_string();

                self.output.print_info(&format!(
                    "Creating project '{}' at {}...",
                    name, git_path_str
                ));

                match self.api.create_project(&name, &git_path_str, None).await {
                    Ok(project) => {
                        self.output
                            .print_success(&format!("Project '{}' created!", project.name));
                        self.project_id = Some(project.id);
                        self.project_name = Some(project.name);

                        // Start new session for this project
                        self.session = None;
                        self.init_session().await?;
                    }
                    Err(e) => {
                        self.output
                            .print_error(&format!("Failed to create project: {}", e));
                    }
                }
            }

            "list" => {
                let projects = self.api.list_projects().await?;
                if projects.is_empty() {
                    self.output.print_info("No projects found.");
                    return Ok(());
                }

                self.output.print_header("Projects");
                for project in &projects {
                    let marker = if self.project_id == Some(project.id) {
                        " (active)"
                    } else {
                        ""
                    };
                    println!(
                        "  {} {}{}",
                        project.name.bright_cyan(),
                        format!("[{}]", &project.id.to_string()[..8]).dimmed(),
                        marker.bright_green()
                    );
                }
            }

            _ => {
                // Treat as project name to switch to
                let name = args.join(" ");
                match self.api.find_project_by_name(&name).await {
                    Ok(Some(project)) => {
                        self.project_id = Some(project.id);
                        self.project_name = Some(project.name.clone());
                        self.output
                            .print_success(&format!("Switched to project: {}", project.name));

                        // Start new session for this project
                        self.session = None;
                        self.init_session().await?;
                    }
                    Ok(None) => {
                        self.output.print_error(&format!(
                            "Project '{}' not found. Use /project create {} to create it.",
                            name, name
                        ));
                    }
                    Err(e) => {
                        self.output
                            .print_error(&format!("Error finding project: {}", e));
                    }
                }
            }
        }

        Ok(())
    }

    /// Show kanban board — scoped to current project if set, otherwise full platform overview
    async fn handle_boards_command(&mut self) -> Result<()> {
        if let Some(project_id) = self.project_id {
            // ── Single-project kanban ──
            let project_name = self.project_name.clone().unwrap_or_default();
            self.output.print_info("Fetching tasks…");

            let tasks = self.api.list_tasks(project_id, None).await?;

            let mut todo: Vec<(String, String)> = vec![];
            let mut inprogress: Vec<(String, String)> = vec![];
            let mut done: Vec<(String, String)> = vec![];

            for t in &tasks {
                let entry = (t.id.to_string()[..8].to_string(), t.title.clone());
                match t.status.as_str() {
                    "inprogress" | "in-progress" | "in_progress" => inprogress.push(entry),
                    "done" | "completed" => done.push(entry),
                    _ => todo.push(entry),
                }
            }

            self.output.print_task_board(
                &project_name,
                &[("TODO", todo), ("IN PROGRESS", inprogress), ("DONE", done)],
            );
        } else {
            // ── Platform-wide overview ──
            self.output.print_info("Fetching all projects…");
            let projects = self.api.list_projects().await?;

            if projects.is_empty() {
                self.output.print_info("No projects found.");
                return Ok(());
            }

            // Fetch tasks for all projects in parallel (cap at 20 to keep it fast)
            let active_projects: Vec<_> = projects.iter().take(20).collect();
            self.output.print_info(&format!(
                "Loading tasks for {} projects…",
                active_projects.len()
            ));

            let task_futures: Vec<_> = active_projects
                .iter()
                .map(|p| self.api.list_tasks(p.id, None))
                .collect();

            let task_results = futures::future::join_all(task_futures).await;

            println!();
            println!(
                "{}",
                "▶ Platform Board — All Projects".bright_yellow().bold()
            );
            println!("{}", "─".repeat(80).dimmed());
            println!();
            println!(
                "{}",
                format!(
                    "{:<32} {:>6} {:>12} {:>6}",
                    "Project", "TODO", "IN PROGRESS", "DONE"
                )
                .bright_white()
                .bold()
            );
            println!("{}", "─".repeat(60).dimmed());

            let mut total_todo = 0usize;
            let mut total_ip = 0usize;
            let mut total_done = 0usize;

            for (project, tasks_result) in active_projects.iter().zip(task_results.iter()) {
                let tasks = match tasks_result {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let todo = tasks
                    .iter()
                    .filter(|t| {
                        !matches!(
                            t.status.as_str(),
                            "inprogress" | "in-progress" | "in_progress" | "done" | "completed"
                        )
                    })
                    .count();
                let ip = tasks
                    .iter()
                    .filter(|t| {
                        matches!(
                            t.status.as_str(),
                            "inprogress" | "in-progress" | "in_progress"
                        )
                    })
                    .count();
                let done = tasks
                    .iter()
                    .filter(|t| matches!(t.status.as_str(), "done" | "completed"))
                    .count();

                if tasks.is_empty() {
                    continue;
                }

                total_todo += todo;
                total_ip += ip;
                total_done += done;

                let name = if project.name.len() > 30 {
                    format!("{}…", &project.name[..29])
                } else {
                    project.name.clone()
                };

                let ip_display = if ip > 0 {
                    ip.to_string().bright_blue().to_string()
                } else {
                    ip.to_string()
                };

                println!(
                    "{:<32} {:>6} {:>12} {:>6}",
                    name.bright_white(),
                    if todo > 0 {
                        todo.to_string().bright_yellow().to_string()
                    } else {
                        todo.to_string()
                    },
                    ip_display,
                    if done > 0 {
                        done.to_string().bright_green().to_string()
                    } else {
                        done.to_string()
                    },
                );
            }

            println!("{}", "─".repeat(60).dimmed());
            println!(
                "{:<32} {:>6} {:>12} {:>6}",
                "TOTAL".bright_white().bold(),
                total_todo.to_string().bright_yellow().bold(),
                total_ip.to_string().bright_blue().bold(),
                total_done.to_string().bright_green().bold(),
            );
            println!();
            println!(
                "{}",
                "Tip: /project <name> then /boards for a full kanban view".dimmed()
            );
            println!();
        }

        Ok(())
    }

    /// Show recent session history
    async fn handle_history_command(&mut self, _args: &[&str]) -> Result<()> {
        use crate::session::ConversationLog;

        let sessions = ConversationLog::list_saved(20);

        if sessions.is_empty() {
            self.output
                .print_info("No sessions saved yet. Sessions are saved when you exit topsi.");
            return Ok(());
        }

        self.output.print_header("Recent Sessions");
        println!();
        println!(
            "{}",
            format!(
                "{:<20} {:<24} {:>8} {:>7}",
                "Date", "Project", "Messages", "Tokens"
            )
            .bright_white()
            .bold()
        );
        println!("{}", "─".repeat(65).dimmed());

        for s in &sessions {
            let date = &s.started_at[..16].replace('T', " ");
            let project = s.project.as_deref().unwrap_or("(no project)");
            let project_display = if project.len() > 22 {
                format!("{}…", &project[..21])
            } else {
                project.to_string()
            };
            println!(
                "{:<20} {:<24} {:>8} {:>7}",
                date.dimmed(),
                project_display.bright_cyan(),
                s.message_count.to_string().bright_white(),
                s.total_tokens.to_string().bright_yellow(),
            );
        }
        println!();

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
                "add",
                "implement",
                "create",
                "build",
                "fix",
                "update",
                "refactor",
                "remove",
                "delete",
                "change",
                "modify",
            ];

            let input_lower = input.to_lowercase();
            let is_task_request = task_indicators
                .iter()
                .any(|ind| input_lower.starts_with(ind));

            if is_task_request {
                if let Some(project_id) = self.project_id {
                    let request = CreateTaskRequest {
                        title: input.to_string(),
                        description: None,
                        created_by: "topsi-cli".to_string(),
                    };

                    if let Ok(task) = self.api.create_task(project_id, None, &request).await {
                        self.output
                            .print_task("created", &task.title, Some(&task.id.to_string()));

                        if let Some(session) = &self.session {
                            session.record_task_created(task.id);
                        }
                    }
                }
            }
        }

        // Route to the appropriate agent
        let agent_name = self.config.agents.default.clone();
        let session_id = self
            .session
            .as_ref()
            .map(|s| s.id.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Build working-directory context so Topsi knows where we are
        let dir_context = {
            let dir_str = self.work_dir.to_string_lossy().to_string();
            let git_branch = get_git_branch(&self.work_dir);
            serde_json::json!({
                "working_dir": dir_str,
                "git_branch": git_branch,
                "project": self.project_name,
            })
        };

        // Log user message
        self.conversation_log.add("user", input, 0, vec![]);

        if agent_name == "topsi" {
            // Topsi has a dedicated high-level endpoint
            match self
                .api
                .chat_with_topsi(
                    input,
                    &session_id,
                    self.project_id,
                    Some(dir_context),
                    self.current_model.as_deref(),
                )
                .await
            {
                Ok(response) => {
                    let tokens =
                        response.input_tokens.unwrap_or(0) + response.output_tokens.unwrap_or(0);

                    // Show tool calls inline (Claude Code-style)
                    if !response.tool_calls.is_empty() {
                        println!();
                        for tool in &response.tool_calls {
                            self.output.print_tool_call(tool);
                        }
                    }

                    if let Some(session) = &self.session {
                        let vibe = (tokens as f64 * 0.05) as i64;
                        session.update_cost(tokens, vibe);
                    }

                    // Log assistant response
                    self.conversation_log.add(
                        "assistant",
                        &response.content,
                        tokens,
                        response.tool_calls.clone(),
                    );

                    self.output.print_response(&response.content);

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
                    self.output.print_error(&format!("Topsi error: {}", e));
                    self.output.print_info(
                        "Note: Make sure the PCG backend is running and Topsi is initialized.",
                    );
                }
            }
        } else if let Ok(Some(agent)) = self.api.get_agent_by_name(&agent_name).await {
            match self
                .api
                .chat_with_agent(
                    agent.id,
                    input,
                    &session_id,
                    self.project_id,
                    self.current_model.as_deref(),
                    self.current_provider.as_deref(),
                )
                .await
            {
                Ok(response) => {
                    let tokens =
                        response.input_tokens.unwrap_or(0) + response.output_tokens.unwrap_or(0);

                    if let Some(session) = &self.session {
                        let vibe = (tokens as f64 * 0.05) as i64;
                        session.update_cost(tokens, vibe);
                    }

                    self.conversation_log
                        .add("assistant", &response.content, tokens, vec![]);
                    self.output.print_response(&response.content);

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

/// Get the current git branch name for a directory (best-effort)
fn get_git_branch(dir: &PathBuf) -> Option<String> {
    let repo = git2::Repository::discover(dir).ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(|s| s.to_string())
}
