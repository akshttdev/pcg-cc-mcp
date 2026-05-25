//! Halt-report writer.
//!
//! Writes one `.md` and one `.json` to the configured report directory on a
//! graceful halt. Format is deliberately small — the goal is "what should
//! a human look at first?", not a full audit log.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::circuit_breaker::{CircuitSnapshot, TripReason};
use crate::events::LoopEvent;

const MAX_HISTORY_LINES: usize = 50;

#[derive(Debug, Serialize, Deserialize)]
pub struct HaltReport {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub trip_reason: Option<TripReason>,
    pub snapshot: CircuitSnapshot,
    pub recent_events: Vec<HistoryEntry>,
    pub per_agent_summary: Vec<AgentSummary>,
    pub top_causes: Vec<(String, u32)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub at: chrono::DateTime<chrono::Utc>,
    pub kind: String,
    pub summary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentSummary {
    pub agent: String,
    pub commits: u32,
    pub tests_run: u32,
    pub ux_reports: u32,
}

impl HaltReport {
    pub fn build(
        trip_reason: Option<TripReason>,
        snapshot: CircuitSnapshot,
        history: Vec<LoopEvent>,
    ) -> Self {
        let recent = history
            .iter()
            .rev()
            .take(MAX_HISTORY_LINES)
            .map(|event| HistoryEntry {
                at: event.at(),
                kind: event.kind().to_string(),
                summary: summarise(event),
            })
            .collect::<Vec<_>>();

        let mut top_causes = snapshot.same_cause_counts.clone();
        top_causes.sort_by(|a, b| b.1.cmp(&a.1));
        top_causes.truncate(5);

        let per_agent_summary = summarise_per_agent(&history);

        HaltReport {
            generated_at: Utc::now(),
            trip_reason,
            snapshot,
            recent_events: recent,
            per_agent_summary,
            top_causes,
        }
    }

    /// Write the report into `dir` as `loop-<UTC>.md` and `loop-<UTC>.json`.
    /// Returns the path to the markdown file.
    pub async fn write(&self, dir: &Path) -> Result<PathBuf, anyhow::Error> {
        tokio::fs::create_dir_all(dir).await?;
        let stem = format!("loop-{}", self.generated_at.format("%Y%m%dT%H%M%SZ"));
        let md_path = dir.join(format!("{stem}.md"));
        let json_path = dir.join(format!("{stem}.json"));

        let md = self.render_markdown();
        let json = serde_json::to_string_pretty(self)?;

        tokio::fs::write(&md_path, md).await?;
        tokio::fs::write(&json_path, json).await?;

        Ok(md_path)
    }

    fn render_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Loop Engine Halt Report\n\n");
        out.push_str(&format!(
            "**Generated**: {}\n\n",
            self.generated_at.to_rfc3339()
        ));
        match &self.trip_reason {
            Some(reason) => {
                out.push_str(&format!("**Trip reason**: `{:?}`\n\n", reason));
            }
            None => {
                out.push_str("**Trip reason**: external cancel (not a trip)\n\n");
            }
        }

        out.push_str("## Top causes\n\n");
        if self.top_causes.is_empty() {
            out.push_str("_None recorded._\n\n");
        } else {
            for (cause, count) in &self.top_causes {
                out.push_str(&format!("- `{cause}` — {count}\n"));
            }
            out.push('\n');
        }

        out.push_str("## Per-agent activity\n\n");
        out.push_str("| Agent | Commits | Tests run | UX reports |\n");
        out.push_str("|-------|---------|-----------|------------|\n");
        for summary in &self.per_agent_summary {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                summary.agent, summary.commits, summary.tests_run, summary.ux_reports
            ));
        }
        out.push('\n');

        out.push_str("## Recent events (newest first)\n\n");
        for entry in &self.recent_events {
            out.push_str(&format!(
                "- `{}` **{}** — {}\n",
                entry.at.to_rfc3339(),
                entry.kind,
                entry.summary
            ));
        }
        out.push('\n');

        out.push_str("## What to look at first\n\n");
        if let Some((cause, _)) = self.top_causes.first() {
            out.push_str(&format!(
                "Repeated cause `{cause}` dominates the failure log. Start there.\n"
            ));
        } else {
            out.push_str("No dominant cause — likely an external halt or a quiet run.\n");
        }
        out
    }
}

fn summarise(event: &LoopEvent) -> String {
    match event {
        LoopEvent::CommitCompleted { sha, author, summary, .. } => {
            format!("{} @ {} — {}", author.as_str(), shorten_sha(sha), summary)
        }
        LoopEvent::TestsCompleted { sha, passed, failures } => {
            format!(
                "tests {} @ {} ({} failures)",
                if *passed { "passed" } else { "failed" },
                shorten_sha(sha),
                failures.len()
            )
        }
        LoopEvent::UxReport { sha, blockers } => {
            format!("ux @ {} — {} blocker(s)", shorten_sha(sha), blockers.len())
        }
        LoopEvent::FailureDetected { source, detail, .. } => {
            format!("from {}: {}", source.as_str(), shorten(detail, 80))
        }
        LoopEvent::CircuitTripped { reason, .. } => {
            format!("breaker tripped: {:?}", reason)
        }
        LoopEvent::LoopHalted { final_report_path } => {
            format!("halted, report at {}", final_report_path.display())
        }
    }
}

fn shorten_sha(sha: &str) -> String {
    sha.chars().take(8).collect()
}

fn shorten(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

fn summarise_per_agent(history: &[LoopEvent]) -> Vec<AgentSummary> {
    use std::collections::HashMap;
    let mut by_agent: HashMap<String, AgentSummary> = HashMap::new();
    for event in history {
        match event {
            LoopEvent::CommitCompleted { author, .. } => {
                by_agent
                    .entry(author.as_str().to_string())
                    .or_insert_with(|| AgentSummary {
                        agent: author.as_str().to_string(),
                        commits: 0,
                        tests_run: 0,
                        ux_reports: 0,
                    })
                    .commits += 1;
            }
            LoopEvent::TestsCompleted { .. } => {
                by_agent
                    .entry("playwright".to_string())
                    .or_insert_with(|| AgentSummary {
                        agent: "playwright".to_string(),
                        commits: 0,
                        tests_run: 0,
                        ux_reports: 0,
                    })
                    .tests_run += 1;
            }
            LoopEvent::UxReport { .. } => {
                by_agent
                    .entry("fake_user".to_string())
                    .or_insert_with(|| AgentSummary {
                        agent: "fake_user".to_string(),
                        commits: 0,
                        tests_run: 0,
                        ux_reports: 0,
                    })
                    .ux_reports += 1;
            }
            _ => {}
        }
    }
    let mut out: Vec<_> = by_agent.into_values().collect();
    out.sort_by(|a, b| a.agent.cmp(&b.agent));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::AgentId;
    use crate::circuit_breaker::CircuitSnapshot;
    use crate::events::{CauseHash, LoopEvent};

    #[tokio::test]
    async fn writes_md_and_json() {
        let tmp = tempfile::tempdir().expect("tmp");
        let history = vec![
            LoopEvent::CommitCompleted {
                at: Utc::now(),
                sha: "abc12345".into(),
                author: AgentId::Auri,
                summary: "fix bug".into(),
                loop_generated: false,
            },
            LoopEvent::FailureDetected {
                at: Utc::now(),
                sha: None,
                cause: CauseHash::from_normalised("E1"),
                detail: "tests failed".into(),
                source: AgentId::Playwright,
            },
        ];
        let snapshot = CircuitSnapshot {
            same_cause_counts: vec![("aa".into(), 3)],
            rolling_failure_count: 3,
            rolling_window_seconds: 1800,
        };
        let report = HaltReport::build(Some(TripReason::RepeatedSameCause), snapshot, history);

        let md_path = report.write(tmp.path()).await.expect("write");
        assert!(md_path.exists());
        let json_path = md_path.with_extension("json");
        assert!(json_path.exists());

        let md = tokio::fs::read_to_string(&md_path).await.expect("read md");
        assert!(md.contains("Halt Report"));
        assert!(md.contains("RepeatedSameCause"));
    }
}
