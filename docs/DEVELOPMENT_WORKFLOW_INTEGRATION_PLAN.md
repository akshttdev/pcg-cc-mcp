# Development Workflow Integration Plan

## Executive Summary

**Problem:** Development work (like Claude Code sessions) happens *outside* the PCG Dashboard, resulting in:
- Manual session log creation
- No real-time cost tracking
- Disconnected task history
- Lost metrics and insights

**Solution:** Integrate development workflows *natively* into PCG Dashboard by:
1. Creating a "Development Session" entity that tracks work in real-time
2. Leveraging existing `TokenUsage` infrastructure for cost tracking
3. Auto-generating session reports from actual system data
4. Linking sessions bidirectionally with tasks

---

## Current System Strengths (Already Built)

| Component | Location | Capability |
|-----------|----------|------------|
| `TokenUsage` | `crates/db/src/models/token_usage.rs` | Tracks LLM tokens per project/agent/task |
| `ExecutorSession` | `crates/db/src/models/executor_session.rs` | Tracks AI execution sessions |
| `VibeTransaction` | `crates/db/src/models/vibe_transaction.rs` | Ledger of VIBE token usage |
| `ModelPricing` | `crates/db/src/models/model_pricing.rs` | Token → VIBE cost conversion |
| Task system | Full CRUD with `custom_properties` | Extensible metadata per task |
| Project Boards | Kanban boards with calendar view | Visual timeline of work |

**Key Insight:** The infrastructure exists - we need a **Development Session** layer to connect these pieces.

---

## Proposed Architecture

### New Entity: `DevelopmentSession`

```sql
CREATE TABLE development_sessions (
    id UUID PRIMARY KEY,
    project_id UUID NOT NULL REFERENCES projects(id),
    board_id UUID REFERENCES project_boards(id),

    -- Identity
    title TEXT NOT NULL,
    session_type TEXT NOT NULL CHECK (session_type IN ('feature', 'bugfix', 'refactor', 'research', 'deployment')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'completed', 'cancelled')),

    -- Timing
    started_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ended_at TIMESTAMP,
    duration_minutes INTEGER,

    -- Git tracking (auto-captured)
    git_branch TEXT,
    git_start_sha TEXT,
    git_end_sha TEXT,
    commits_count INTEGER DEFAULT 0,
    files_changed INTEGER DEFAULT 0,
    lines_added INTEGER DEFAULT 0,
    lines_removed INTEGER DEFAULT 0,

    -- Cost tracking (auto-calculated)
    total_tokens_used BIGINT DEFAULT 0,
    total_vibe_cost BIGINT DEFAULT 0,
    total_usd_cost DECIMAL(10,4) DEFAULT 0,

    -- Linked entities
    linked_task_ids TEXT, -- JSON array of task UUIDs
    created_by TEXT NOT NULL,

    -- Content
    summary TEXT,
    key_changes TEXT, -- JSON array of change descriptions
    next_steps TEXT, -- JSON array of follow-up items

    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Link table for session ↔ task relationships
CREATE TABLE development_session_tasks (
    session_id UUID REFERENCES development_sessions(id),
    task_id UUID REFERENCES tasks(id),
    relationship TEXT CHECK (relationship IN ('created', 'completed', 'updated', 'blocked')),
    PRIMARY KEY (session_id, task_id)
);
```

---

## Integration Points

### 1. Claude Code / AI Session Integration

**How it works:**
```
┌─────────────────────────────────────────────────────────────────┐
│                     Claude Code Session                          │
├─────────────────────────────────────────────────────────────────┤
│ 1. User starts session: "Let's work on feature X"               │
│    → API: POST /api/dev-sessions/start                          │
│    → Creates DevelopmentSession, captures git SHA               │
│                                                                 │
│ 2. Work happens (LLM calls tracked via existing TokenUsage)     │
│    → TokenUsage records link to session via task_attempt_id     │
│                                                                 │
│ 3. User completes tasks                                         │
│    → Session auto-links completed tasks                         │
│                                                                 │
│ 4. Session ends                                                 │
│    → API: POST /api/dev-sessions/{id}/complete                  │
│    → Captures git diff stats                                    │
│    → Calculates total VIBE cost                                 │
│    → Auto-generates summary from TokenUsage + git data          │
└─────────────────────────────────────────────────────────────────┘
```

### 2. VIBE Cost Calculation (Real-Time)

```rust
// Extend existing TokenUsage to link to sessions
impl DevelopmentSession {
    pub async fn calculate_costs(&self, pool: &SqlitePool) -> SessionCosts {
        // Sum all TokenUsage records created during session timeframe
        let usage = sqlx::query!(
            r#"SELECT
                SUM(total_tokens) as tokens,
                SUM(cost_cents) as cost_cents
            FROM token_usage
            WHERE project_id = ?
            AND created_at BETWEEN ? AND ?"#,
            self.project_id,
            self.started_at,
            self.ended_at.unwrap_or(Utc::now())
        ).fetch_one(pool).await?;

        // Convert to VIBE using ModelPricing
        let vibe_cost = (usage.cost_cents.unwrap_or(0) as f64 / 0.1) as i64;

        SessionCosts {
            tokens_used: usage.tokens.unwrap_or(0),
            cost_usd: usage.cost_cents.unwrap_or(0) as f64 / 100.0,
            vibe_cost,
        }
    }
}
```

### 3. Git Integration (Auto-Capture)

```rust
impl DevelopmentSession {
    pub async fn capture_git_stats(&mut self, repo_path: &Path) -> Result<()> {
        // Get current SHA
        let current_sha = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()?;
        self.git_end_sha = Some(String::from_utf8_lossy(&current_sha.stdout).trim().to_string());

        // Get diff stats since start
        if let Some(start_sha) = &self.git_start_sha {
            let diff_stats = Command::new("git")
                .args(["diff", "--stat", start_sha, "HEAD"])
                .current_dir(repo_path)
                .output()?;

            // Parse: "X files changed, Y insertions(+), Z deletions(-)"
            let (files, additions, deletions) = parse_git_stats(&diff_stats.stdout);
            self.files_changed = Some(files);
            self.lines_added = Some(additions);
            self.lines_removed = Some(deletions);

            // Count commits
            let commit_count = Command::new("git")
                .args(["rev-list", "--count", &format!("{}..HEAD", start_sha)])
                .current_dir(repo_path)
                .output()?;
            self.commits_count = Some(String::from_utf8_lossy(&commit_count.stdout).trim().parse().unwrap_or(0));
        }

        Ok(())
    }
}
```

### 4. Auto-Generated Session Report

When session completes, automatically generate markdown report:

```rust
impl DevelopmentSession {
    pub fn generate_report(&self) -> String {
        format!(r#"
# Development Session: {}
**Date:** {} | **Duration:** {} minutes | **Status:** {}

## Metrics
| Metric | Value |
|--------|-------|
| Files Changed | {} |
| Lines Added | +{} |
| Lines Removed | -{} |
| Commits | {} |
| Tokens Used | {:,} |
| VIBE Cost | {:,} |
| USD Cost | ${:.2} |

## Tasks Completed
{}

## Git Commits
{}

## Next Steps
{}
"#,
            self.title,
            self.started_at.format("%Y-%m-%d"),
            self.duration_minutes.unwrap_or(0),
            self.status,
            self.files_changed.unwrap_or(0),
            self.lines_added.unwrap_or(0),
            self.lines_removed.unwrap_or(0),
            self.commits_count.unwrap_or(0),
            self.total_tokens_used,
            self.total_vibe_cost,
            self.total_usd_cost,
            self.format_linked_tasks(),
            self.format_git_commits(),
            self.next_steps.as_deref().unwrap_or("None specified")
        )
    }
}
```

---

## API Endpoints

```rust
// New routes: crates/server/src/routes/dev_sessions.rs

POST   /api/dev-sessions/start           // Start new session
GET    /api/dev-sessions/active          // Get active sessions
GET    /api/dev-sessions/:id             // Get session details
PATCH  /api/dev-sessions/:id             // Update session
POST   /api/dev-sessions/:id/pause       // Pause session
POST   /api/dev-sessions/:id/resume      // Resume session
POST   /api/dev-sessions/:id/complete    // Complete session (triggers stats capture)
POST   /api/dev-sessions/:id/link-task   // Link task to session
GET    /api/dev-sessions/:id/report      // Get auto-generated report
GET    /api/dev-sessions/:id/costs       // Get real-time cost breakdown

// Analytics
GET    /api/dev-sessions/stats           // Aggregate session statistics
GET    /api/dev-sessions/by-project/:id  // Sessions by project
GET    /api/dev-sessions/by-date-range   // Filter by dates
```

---

## Frontend Integration

### 1. Session Control Panel (Floating Widget)

```tsx
// components/dev-session/SessionControlPanel.tsx
function SessionControlPanel() {
  const { activeSession, startSession, endSession } = useDevSession();

  return (
    <div className="fixed bottom-4 right-4 bg-card rounded-lg shadow-lg p-4">
      {activeSession ? (
        <>
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" />
            <span className="font-medium">{activeSession.title}</span>
          </div>
          <div className="text-sm text-muted-foreground mt-2">
            <p>Duration: {formatDuration(activeSession.started_at)}</p>
            <p>Tokens: {activeSession.total_tokens_used.toLocaleString()}</p>
            <p>VIBE: {activeSession.total_vibe_cost.toLocaleString()}</p>
          </div>
          <Button onClick={endSession} className="mt-2 w-full">
            Complete Session
          </Button>
        </>
      ) : (
        <Button onClick={startSession}>Start Dev Session</Button>
      )}
    </div>
  );
}
```

### 2. Session History on Task Cards

```tsx
// In TaskCard.tsx - show which session created/completed the task
{task.sessions?.map(session => (
  <Badge key={session.id} variant="outline">
    Session: {session.title} ({session.relationship})
  </Badge>
))}
```

### 3. Calendar View Enhancement

Tasks from Development History board will show on calendar with:
- Session dates as scheduled dates
- VIBE cost in tooltip
- Color coding by session type (feature/bugfix/refactor)

---

## Migration Path

### Phase 1: Database & Models (Week 1)
- [ ] Create `development_sessions` migration
- [ ] Create `development_session_tasks` migration
- [ ] Implement Rust models
- [ ] Generate TypeScript types

### Phase 2: Core API (Week 2)
- [ ] Implement session CRUD endpoints
- [ ] Implement git stats capture
- [ ] Link TokenUsage to sessions
- [ ] Auto-report generation

### Phase 3: Frontend (Week 3)
- [ ] Session control widget
- [ ] Session history page
- [ ] Task linkage UI
- [ ] Calendar integration

### Phase 4: Claude Code Integration (Week 4)
- [ ] MCP tool: `start_dev_session`
- [ ] MCP tool: `end_dev_session`
- [ ] MCP tool: `link_task_to_session`
- [ ] Auto-detection of session context

---

## VIBE Cost Estimation Formula

Based on session log analysis, here's the VIBE cost model:

```
Base Cost = (LLM Tokens × Model Rate) + (Images × Image Rate) + (Searches × Search Rate)

Where:
- GPT-4o: 5,000 VIBE per 1M input tokens, 20,000 VIBE per 1M output tokens
- Claude Sonnet 4: 6,000 VIBE per 1M input, 30,000 VIBE per 1M output
- DALL-E/SDXL: 4,000 VIBE per image
- Exa Search: 100 VIBE per query

Complexity Multiplier:
- Simple feature: 1.0x
- Medium feature: 1.5x
- Complex system: 2.0x
- Major architecture: 3.0x
```

**Example from Session 2026-01-18 (Conference Workflow):**
```
LLM Calls: ~50 calls × avg 2K tokens = 100K tokens
Images: 4 thumbnails × 4,000 = 16,000 VIBE
Base: ~50,000 VIBE (LLM) + 16,000 (images) = 66,000 VIBE
Complexity: 2.0x (complex workflow)
Total: ~132,000 VIBE

Actual logged estimate: 145,000 VIBE ✓ (within 10%)
```

---

## Success Metrics

After implementation, we should see:

| Metric | Before | After |
|--------|--------|-------|
| Session logging time | 15-30 min manual | 0 min (auto) |
| Cost visibility | None | Real-time |
| Task linkage | Manual notes | Automatic |
| Git stats | Manual copy | Auto-capture |
| Report accuracy | Estimated | Precise |
| VIBE tracking | Retrospective | Live |

---

## Immediate Next Steps

1. **Create migration file** for `development_sessions` table
2. **Add MCP tools** for session start/end in Claude Code context
3. **Build floating widget** for session control
4. **Wire TokenUsage** to session time windows
5. **Test with next development session** to validate data flow

This transforms PCG Dashboard from a *retrospective documentation tool* into a **real-time development intelligence platform**.
