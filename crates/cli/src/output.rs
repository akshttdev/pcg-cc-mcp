//! Output formatting and terminal rendering
//!
//! Handles rich terminal output with colors, markdown rendering, and status bars.

use colored::Colorize;

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

/// Output handler for terminal display
pub struct OutputHandler {
    pub show_cost_bar: bool,
    pub markdown_enabled: bool,
}

impl OutputHandler {
    pub fn new(show_cost_bar: bool, markdown_enabled: bool) -> Self {
        Self {
            show_cost_bar,
            markdown_enabled,
        }
    }

    /// Print the welcome banner
    pub fn print_banner(&self, project_name: Option<&str>, session_id: &str) {
        println!();
        println!(
            "{}",
            "╔═══════════════════════════════════════════════════════════════╗"
                .bright_cyan()
        );
        println!(
            "{}",
            "║             PCG Development Session                           ║"
                .bright_cyan()
        );
        println!(
            "{}",
            "╠═══════════════════════════════════════════════════════════════╣"
                .bright_cyan()
        );

        if let Some(name) = project_name {
            println!(
                "{}  Project: {:<50}{}",
                "║".bright_cyan(),
                name.bright_white(),
                "║".bright_cyan()
            );
        }
        println!(
            "{}  Session: {:<50}{}",
            "║".bright_cyan(),
            &session_id[..8.min(session_id.len())].dimmed(),
            "║".bright_cyan()
        );
        println!(
            "{}",
            "╠═══════════════════════════════════════════════════════════════╣"
                .bright_cyan()
        );
        println!(
            "{}                                                               {}",
            "║".bright_cyan(),
            "║".bright_cyan()
        );
        println!(
            "{}  {}                               {}",
            "║".bright_cyan(),
            "Type your request or use /help for commands".dimmed(),
            "║".bright_cyan()
        );
        println!(
            "{}                                                               {}",
            "║".bright_cyan(),
            "║".bright_cyan()
        );
        println!(
            "{}",
            "╚═══════════════════════════════════════════════════════════════╝"
                .bright_cyan()
        );
        println!();
    }

    /// Print a section header
    pub fn print_header(&self, text: &str) {
        println!();
        println!("{}", format!("▶ {}", text).bright_yellow().bold());
        println!("{}", "─".repeat(60).dimmed());
    }

    /// Print a success message
    pub fn print_success(&self, text: &str) {
        println!("{} {}", "✓".bright_green(), text.bright_white());
    }

    /// Print an error message
    pub fn print_error(&self, text: &str) {
        println!("{} {}", "✗".bright_red(), text.bright_red());
    }

    /// Print a warning message
    pub fn print_warning(&self, text: &str) {
        println!("{} {}", "⚠".bright_yellow(), text.yellow());
    }

    /// Print an info message
    pub fn print_info(&self, text: &str) {
        println!("{} {}", "ℹ".bright_blue(), text);
    }

    /// Print a task status
    pub fn print_task(&self, action: &str, title: &str, id: Option<&str>) {
        let action_colored = match action {
            "created" => format!("Task Created: ").bright_green(),
            "completed" => format!("Task Completed: ").bright_cyan(),
            "updated" => format!("Task Updated: ").bright_yellow(),
            _ => format!("Task {}: ", action).normal(),
        };

        if let Some(task_id) = id {
            println!(
                "{}{} {}",
                action_colored,
                title.bright_white(),
                format!("(#{})", &task_id[..8]).dimmed()
            );
        } else {
            println!("{}{}", action_colored, title.bright_white());
        }
    }

    /// Print code block with optional syntax highlighting
    pub fn print_code(&self, language: &str, code: &str) {
        println!();
        println!(
            "{}",
            format!("```{}", language).bright_black().on_bright_black()
        );
        for line in code.lines() {
            // Basic syntax highlighting
            let highlighted = self.highlight_line(language, line);
            println!("  {}", highlighted);
        }
        println!("{}", "```".bright_black().on_bright_black());
        println!();
    }

    /// Print a file operation (read, write, edit)
    pub fn print_file_operation(&self, operation: &str, path: &str) {
        let op_colored = match operation {
            "Reading" => "Reading:".bright_blue(),
            "Writing" => "Writing:".bright_green(),
            "Editing" => "Editing:".bright_yellow(),
            "Deleted" => "Deleted:".bright_red(),
            _ => format!("{}:", operation).normal(),
        };

        println!("{} {}", op_colored, path.bright_white());
    }

    /// Print diff output
    pub fn print_diff(&self, additions: &[String], deletions: &[String]) {
        for line in deletions {
            println!("{}", format!("- {}", line).red());
        }
        for line in additions {
            println!("{}", format!("+ {}", line).green());
        }
    }

    /// Print the cost/status bar
    pub fn print_status_bar(&self, tokens: i64, vibe: i64, tasks_created: i32, tasks_completed: i32) {
        if !self.show_cost_bar {
            return;
        }

        println!();
        println!(
            "{}",
            "───────────────────────────────────────────────────────────────".dimmed()
        );
        println!(
            "  {} {} | {} {} | Tasks: {} created, {} completed",
            "Tokens:".dimmed(),
            format_num(tokens).bright_white(),
            "VIBE:".dimmed(),
            format_num(vibe).bright_cyan(),
            tasks_created.to_string().bright_green(),
            tasks_completed.to_string().bright_blue()
        );
    }

    /// Print assistant response (with optional markdown rendering)
    pub fn print_response(&self, content: &str) {
        println!();

        if self.markdown_enabled {
            // Basic markdown rendering
            for line in content.lines() {
                let rendered = self.render_markdown_line(line);
                println!("{}", rendered);
            }
        } else {
            println!("{}", content);
        }
    }

    /// Print projects table
    pub fn print_projects_table(&self, projects: &[(String, String, i64, i64)]) {
        println!();
        println!(
            "{}",
            format!(
                "{:<36} {:<30} {:>10} {:>10}",
                "ID", "Name", "Tasks", "VIBE Spent"
            )
            .bright_white()
            .bold()
        );
        println!("{}", "─".repeat(90).dimmed());

        for (id, name, task_count, vibe_spent) in projects {
            println!(
                "{:<36} {:<30} {:>10} {:>10}",
                id.dimmed(),
                name.bright_white(),
                task_count,
                format_num(*vibe_spent).bright_cyan()
            );
        }
        println!();
    }

    /// Print tasks table
    pub fn print_tasks_table(&self, tasks: &[(String, String, String, String)]) {
        println!();
        println!(
            "{}",
            format!("{:<36} {:<40} {:>10} {:>12}", "ID", "Title", "Status", "Updated")
                .bright_white()
                .bold()
        );
        println!("{}", "─".repeat(100).dimmed());

        for (id, title, status, updated) in tasks {
            let status_colored = match status.as_str() {
                "todo" => status.bright_yellow(),
                "inprogress" | "in-progress" => status.bright_blue(),
                "done" => status.bright_green(),
                "cancelled" => status.dimmed(),
                _ => status.normal(),
            };

            // Truncate title if too long
            let title_display = if title.len() > 38 {
                format!("{}...", &title[..35])
            } else {
                title.clone()
            };

            println!(
                "{:<36} {:<40} {:>10} {:>12}",
                &id[..8].dimmed(),
                title_display.bright_white(),
                status_colored,
                updated.dimmed()
            );
        }
        println!();
    }

    /// Print session report
    pub fn print_session_report(
        &self,
        duration: &str,
        files_changed: i32,
        lines_added: i32,
        lines_removed: i32,
        commits: i32,
        tokens: i64,
        vibe: i64,
        usd: f64,
        tasks: &[(String, String)],
    ) {
        println!();
        println!(
            "{}",
            "╔═══════════════════════════════════════════════════════════════╗"
                .bright_green()
        );
        println!(
            "{}",
            "║                    SESSION COMPLETED                          ║"
                .bright_green()
        );
        println!(
            "{}",
            "╠═══════════════════════════════════════════════════════════════╣"
                .bright_green()
        );

        println!(
            "{}                                                               {}",
            "║".bright_green(),
            "║".bright_green()
        );

        // Duration and git stats
        println!(
            "{}  Duration: {:<52}{}",
            "║".bright_green(),
            duration.bright_white(),
            "║".bright_green()
        );
        println!(
            "{}  Files Changed: {:<46}{}",
            "║".bright_green(),
            files_changed.to_string().bright_white(),
            "║".bright_green()
        );
        println!(
            "{}  Lines: {} / {}{}",
            "║".bright_green(),
            format!("+{}", lines_added).green(),
            format!("-{}", lines_removed).red(),
            " ".repeat(40 - lines_added.to_string().len() - lines_removed.to_string().len())
                + "║"
        );
        println!(
            "{}  Commits: {:<51}{}",
            "║".bright_green(),
            commits.to_string().bright_white(),
            "║".bright_green()
        );

        println!(
            "{}                                                               {}",
            "║".bright_green(),
            "║".bright_green()
        );

        // Cost breakdown
        println!(
            "{}  Token Usage: {:<48}{}",
            "║".bright_green(),
            format!("{} tokens", format_num(tokens)).bright_white(),
            "║".bright_green()
        );
        println!(
            "{}  VIBE Cost: {:<50}{}",
            "║".bright_green(),
            format!("{} VIBE (${:.2} USD)", format_num(vibe), usd).bright_cyan(),
            "║".bright_green()
        );

        println!(
            "{}                                                               {}",
            "║".bright_green(),
            "║".bright_green()
        );

        // Tasks
        if !tasks.is_empty() {
            println!(
                "{}  {}{}",
                "║".bright_green(),
                "Tasks:".bright_white().bold(),
                " ".repeat(55) + "║"
            );
            for (title, status) in tasks {
                let status_icon = if status == "done" { "✓" } else { "○" };
                let title_truncated = if title.len() > 50 {
                    format!("{}...", &title[..47])
                } else {
                    title.clone()
                };
                println!(
                    "{}  {} {:<55}{}",
                    "║".bright_green(),
                    status_icon.bright_green(),
                    title_truncated,
                    "║".bright_green()
                );
            }
        }

        println!(
            "{}                                                               {}",
            "║".bright_green(),
            "║".bright_green()
        );
        println!(
            "{}",
            "╚═══════════════════════════════════════════════════════════════╝"
                .bright_green()
        );
        println!();
    }

    // ============ Helper Methods ============

    fn highlight_line(&self, _language: &str, line: &str) -> String {
        // Basic syntax highlighting (can be expanded)
        let line = line.to_string();

        // Keywords
        let keywords = [
            "fn", "let", "const", "mut", "pub", "async", "await", "impl", "struct", "enum",
            "trait", "use", "mod", "if", "else", "match", "return", "self", "Self",
        ];

        let mut result = line.clone();
        for kw in keywords {
            result = result.replace(
                &format!(" {} ", kw),
                &format!(" {} ", kw.bright_magenta()),
            );
        }

        result
    }

    fn render_markdown_line(&self, line: &str) -> String {
        // Headers
        if line.starts_with("### ") {
            return format!("{}", line[4..].bright_yellow().bold());
        }
        if line.starts_with("## ") {
            return format!("{}", line[3..].bright_cyan().bold());
        }
        if line.starts_with("# ") {
            return format!("{}", line[2..].bright_white().bold().underline());
        }

        // Code blocks
        if line.starts_with("```") {
            return format!("{}", line.dimmed());
        }

        // Inline code
        let mut result = line.to_string();
        while let Some(start) = result.find('`') {
            if let Some(end) = result[start + 1..].find('`') {
                let code = &result[start + 1..start + 1 + end];
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    code.bright_green(),
                    &result[start + 2 + end..]
                );
            } else {
                break;
            }
        }

        // Bold
        while let Some(start) = result.find("**") {
            if let Some(end) = result[start + 2..].find("**") {
                let bold_text = &result[start + 2..start + 2 + end];
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    bold_text.bold(),
                    &result[start + 4 + end..]
                );
            } else {
                break;
            }
        }

        // Lists
        if line.starts_with("- ") || line.starts_with("* ") {
            return format!("  {} {}", "•".bright_cyan(), &line[2..]);
        }

        result
    }
}
