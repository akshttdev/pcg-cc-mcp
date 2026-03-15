# Planning Directory

## File Naming Convention

**Format:** `YYYY-MM-DD--<type>--<topic-slug>.md`

**Types:**
| Type | Purpose | Example |
|------|---------|---------|
| `plan` | Implementation plans, roadmaps, architecture decisions | `2026-03-12--plan--agent-task-mcp-wiring.md` |
| `review` | QA findings, usability audits, regression checks | `2026-03-12--review--sprint1-qa-results.md` |
| `tracker` | Issue/status trackers with open/resolved items | `2026-03-12--tracker--sprint1-issues.md` |
| `analysis` | One-time investigations (merge diffs, regressions) | `2026-03-11--analysis--pr15-merge-conflict.md` |
| `release` | Release notes, changelogs | `2026-03-12--release--seed-db-migration-update.md` |
| `reference` | Persistent reference docs (not date-specific) | `reference--api-patterns.md` |

**Rules:**
- Date is when the document was created (not last modified)
- Topic slug uses kebab-case, max 4-5 words
- Active docs live in `planning/` root — only move to subdirectories when status changes

## Subdirectories

| Directory | Purpose | When to move files here |
|-----------|---------|------------------------|
| `archive/` | Completed work — all findings resolved or investigation finished | After verifying all items are done in the codebase. No open action items remain. |
| `deferred/` | Considered but not actively developed — may revisit later | When a plan is shelved intentionally, not abandoned. Development hasn't started or is paused. |
| `notes/` | Standalone reference docs not tied to sprints or timelines | Persistent documentation (e.g., APN architecture notes) that doesn't follow the date-based naming convention. |

**Archive rules:**
- Verify completion in the codebase before archiving — don't archive based on assumptions
- Files keep their original names when moved (the date records when they were created, not archived)
- Archived files are read-only context — don't update them after archiving
- If an archived topic needs new work, create a new file in `planning/` root, don't move it back

**Deferred rules:**
- Add a note at the top of the file explaining why it was deferred and any conditions for resuming
- Deferred files can be moved back to `planning/` root when development resumes

## Browsing

Files sort chronologically by default. To find active work:
```bash
ls planning/*--plan--* planning/*--tracker--*    # Active plans & trackers
ls planning/*--review--*                         # QA findings
ls planning/archive/                             # Completed work
ls planning/deferred/                            # Shelved plans
ls planning/notes/                               # Reference docs
```

## Status Markers in Active Docs

When updating docs with resolution status, use these inline markers:
- `[RESOLVED]` — item addressed, with brief description of the fix
- `~~strikethrough~~` — for resolved text in tables/lists
- `**STILL OPEN**` — explicitly mark items that remain unresolved
- Add a `> **Status Update (date):**` blockquote near the top summarizing what changed
