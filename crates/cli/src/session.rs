//! Development session management
//!
//! Tracks development sessions including git stats, token usage, and task linkage.

use std::{
    io::Write,
    path::Path,
    sync::atomic::{AtomicI32, AtomicI64, Ordering},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{ApiClient, SessionReport, StartSessionRequest};

// ─── Conversation Log ────────────────────────────────────────────────────────

/// A single message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationEntry {
    pub timestamp: DateTime<Utc>,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tokens: i64,
    #[serde(default)]
    pub tools_used: Vec<String>,
}

/// Persists every exchange to ~/.pcg/sessions/<date>-<id>.jsonl
pub struct ConversationLog {
    pub session_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub project_name: Option<String>,
    entries: Vec<ConversationEntry>,
}

impl ConversationLog {
    pub fn new(project_name: Option<String>) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            started_at: Utc::now(),
            project_name,
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, role: &str, content: &str, tokens: i64, tools: Vec<String>) {
        self.entries.push(ConversationEntry {
            timestamp: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            tokens,
            tools_used: tools,
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn total_tokens(&self) -> i64 {
        self.entries.iter().map(|e| e.tokens).sum()
    }

    /// Write JSONL to ~/.pcg/sessions/
    pub fn save(&self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        let dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".pcg")
            .join("sessions");
        std::fs::create_dir_all(&dir)?;

        let filename = format!(
            "{}-{}.jsonl",
            self.started_at.format("%Y-%m-%d-%H%M"),
            &self.session_id.to_string()[..8]
        );
        let mut file = std::fs::File::create(dir.join(&filename))?;

        // Line 1: header
        writeln!(
            file,
            "{}",
            serde_json::json!({
                "type": "header",
                "session_id": self.session_id,
                "started_at": self.started_at.to_rfc3339(),
                "project": self.project_name,
                "message_count": self.entries.len(),
                "total_tokens": self.total_tokens(),
            })
        )?;

        // Remaining lines: conversation entries
        for entry in &self.entries {
            writeln!(file, "{}", serde_json::to_string(entry)?)?;
        }

        Ok(())
    }

    /// List saved sessions from disk, most recent first
    pub fn list_saved(limit: usize) -> Vec<SessionSummary> {
        let dir = dirs::home_dir()
            .unwrap_or_default()
            .join(".pcg")
            .join("sessions");

        let Ok(read_dir) = std::fs::read_dir(&dir) else {
            return vec![];
        };

        let mut summaries: Vec<SessionSummary> = read_dir
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "jsonl"))
            .filter_map(|e| {
                let path = e.path();
                let content = std::fs::read_to_string(&path).ok()?;
                let first_line = content.lines().next()?;
                let h: serde_json::Value = serde_json::from_str(first_line).ok()?;
                Some(SessionSummary {
                    session_id: h["session_id"].as_str().unwrap_or("").to_string(),
                    started_at: h["started_at"].as_str().unwrap_or("").to_string(),
                    project: h["project"].as_str().map(|s| s.to_string()),
                    message_count: h["message_count"].as_u64().unwrap_or(0) as usize,
                    total_tokens: h["total_tokens"].as_i64().unwrap_or(0),
                    path: path.to_string_lossy().to_string(),
                })
            })
            .collect();

        summaries.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        summaries.truncate(limit);
        summaries
    }
}

/// Metadata for a saved session (one per file)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SessionSummary {
    pub session_id: String,
    pub started_at: String,
    pub project: Option<String>,
    pub message_count: usize,
    pub total_tokens: i64,
    pub path: String,
}

/// Development session state
#[allow(dead_code)]
pub struct DevSession {
    pub id: Uuid,
    pub project_id: Uuid,
    pub project_name: String,
    pub title: String,
    pub started_at: DateTime<Utc>,
    pub git_start_sha: Option<String>,
    pub git_branch: Option<String>,

    // Real-time metrics (thread-safe)
    total_tokens: AtomicI64,
    total_vibe_cost: AtomicI64,
    tasks_created: AtomicI32,
    tasks_completed: AtomicI32,

    // Linked tasks
    linked_tasks: std::sync::Mutex<Vec<Uuid>>,
}

impl DevSession {
    /// Start a new development session
    pub async fn start(
        api: &ApiClient,
        project_id: Uuid,
        project_name: &str,
        title: &str,
        work_dir: &Path,
    ) -> Result<Self> {
        // Capture git info
        let (git_sha, git_branch) = get_git_info(work_dir);

        // Create session via API (or locally if API not available)
        let request = StartSessionRequest {
            project_id,
            title: title.to_string(),
            git_branch: git_branch.clone(),
            git_start_sha: git_sha.clone(),
        };

        let api_session = api.start_session(&request).await?;

        Ok(Self {
            id: api_session.id,
            project_id,
            project_name: project_name.to_string(),
            title: title.to_string(),
            started_at: api_session.started_at,
            git_start_sha: git_sha,
            git_branch,
            total_tokens: AtomicI64::new(0),
            total_vibe_cost: AtomicI64::new(0),
            tasks_created: AtomicI32::new(0),
            tasks_completed: AtomicI32::new(0),
            linked_tasks: std::sync::Mutex::new(Vec::new()),
        })
    }

    /// Resume an existing session
    pub async fn resume(api: &ApiClient, session_id: Uuid) -> Result<Self> {
        // Note: Would need an API endpoint to fetch session details
        // For now, create a placeholder
        let _ = api;
        Ok(Self {
            id: session_id,
            project_id: Uuid::nil(),
            project_name: "Unknown".to_string(),
            title: "Resumed Session".to_string(),
            started_at: Utc::now(),
            git_start_sha: None,
            git_branch: None,
            total_tokens: AtomicI64::new(0),
            total_vibe_cost: AtomicI64::new(0),
            tasks_created: AtomicI32::new(0),
            tasks_completed: AtomicI32::new(0),
            linked_tasks: std::sync::Mutex::new(Vec::new()),
        })
    }

    /// Complete the session and generate report
    pub async fn complete(&self, api: &ApiClient, work_dir: &Path) -> Result<SessionReport> {
        // Capture final git stats
        let git_stats = get_git_diff_stats(work_dir, self.git_start_sha.as_deref());

        // Complete via API
        let mut report = api.complete_session(self.id).await?;

        // Update report with local data
        report.total_tokens = self.total_tokens.load(Ordering::SeqCst);
        report.total_vibe_cost = self.total_vibe_cost.load(Ordering::SeqCst);
        report.tasks_created = self.tasks_created.load(Ordering::SeqCst);
        report.tasks_completed = self.tasks_completed.load(Ordering::SeqCst);
        report.files_changed = git_stats.files_changed;
        report.lines_added = git_stats.lines_added;
        report.lines_removed = git_stats.lines_removed;

        // Calculate duration
        let duration = Utc::now() - self.started_at;
        report.duration_minutes = duration.num_minutes();

        Ok(report)
    }

    /// Update cost metrics
    pub fn update_cost(&self, tokens: i64, vibe: i64) {
        self.total_tokens.fetch_add(tokens, Ordering::SeqCst);
        self.total_vibe_cost.fetch_add(vibe, Ordering::SeqCst);
    }

    /// Record task creation
    pub fn record_task_created(&self, task_id: Uuid) {
        self.tasks_created.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut tasks) = self.linked_tasks.lock() {
            tasks.push(task_id);
        }
    }

    /// Record task completion
    pub fn record_task_completed(&self) {
        self.tasks_completed.fetch_add(1, Ordering::SeqCst);
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> SessionMetrics {
        SessionMetrics {
            total_tokens: self.total_tokens.load(Ordering::SeqCst),
            total_vibe_cost: self.total_vibe_cost.load(Ordering::SeqCst),
            tasks_created: self.tasks_created.load(Ordering::SeqCst),
            tasks_completed: self.tasks_completed.load(Ordering::SeqCst),
        }
    }

    /// Get duration since session start
    pub fn duration(&self) -> chrono::Duration {
        Utc::now() - self.started_at
    }

    /// Format duration as human-readable string
    pub fn duration_string(&self) -> String {
        let duration = self.duration();
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;

        if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }
}

/// Session metrics snapshot
#[derive(Debug, Clone)]
pub struct SessionMetrics {
    pub total_tokens: i64,
    pub total_vibe_cost: i64,
    pub tasks_created: i32,
    pub tasks_completed: i32,
}

/// Git diff statistics
#[derive(Debug, Default)]
pub struct GitDiffStats {
    pub files_changed: i32,
    pub lines_added: i32,
    pub lines_removed: i32,
    pub commits: i32,
}

/// Get current git info (SHA and branch)
fn get_git_info(work_dir: &Path) -> (Option<String>, Option<String>) {
    let repo = match git2::Repository::discover(work_dir) {
        Ok(r) => r,
        Err(_) => return (None, None),
    };

    // Get HEAD commit SHA
    let sha = repo.head().ok().and_then(|head| {
        head.peel_to_commit()
            .ok()
            .map(|c| c.id().to_string()[..8].to_string())
    });

    // Get branch name
    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));

    (sha, branch)
}

/// Get git diff stats since a given commit
fn get_git_diff_stats(work_dir: &Path, start_sha: Option<&str>) -> GitDiffStats {
    let repo = match git2::Repository::discover(work_dir) {
        Ok(r) => r,
        Err(_) => return GitDiffStats::default(),
    };

    let mut stats = GitDiffStats::default();

    // Get start commit
    let start_commit = start_sha.and_then(|sha| {
        git2::Oid::from_str(sha)
            .ok()
            .and_then(|oid| repo.find_commit(oid).ok())
    });

    // Get HEAD commit
    let head_commit = repo.head().ok().and_then(|head| head.peel_to_commit().ok());

    if let (Some(start), Some(end)) = (start_commit, head_commit.as_ref()) {
        // Get diff between start and HEAD
        let start_tree = start.tree().ok();
        let end_tree = end.tree().ok();

        if let (Some(st), Some(et)) = (start_tree, end_tree) {
            if let Ok(diff) = repo.diff_tree_to_tree(Some(&st), Some(&et), None) {
                if let Ok(diff_stats) = diff.stats() {
                    stats.files_changed = diff_stats.files_changed() as i32;
                    stats.lines_added = diff_stats.insertions() as i32;
                    stats.lines_removed = diff_stats.deletions() as i32;
                }
            }
        }

        // Count commits between start and HEAD
        if let Ok(mut revwalk) = repo.revwalk() {
            let _ = revwalk.push(end.id());
            let _ = revwalk.hide(start.id());
            stats.commits = revwalk.count() as i32;
        }
    }

    stats
}
