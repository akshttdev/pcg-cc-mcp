# Planning Directory

## File Naming Convention

**Format:** `YYYY-MM-DD--<type>--<topic-slug>.md`

**Types:**
| Type | Purpose | Example |
|------|---------|---------|
| `plan` | Implementation plans, roadmaps, architecture decisions | `2026-03-12--plan--agent-task-integration.md` |
| `review` | QA findings, usability audits, regression checks | `2026-03-12--review--sprint-qa.md` |
| `tracker` | Issue/status trackers with open/resolved items | `2026-03-12--tracker--sprint-implementation-issues.md` |
| `analysis` | One-time investigations (merge diffs, regressions) | `2026-03-11--analysis--branch-merge-pr15.md` |
| `release` | Release notes, changelogs | `2026-03-12--release--seed-db-update.md` |
| `reference` | Persistent reference docs (not date-specific) | `reference--api-patterns.md` |

**Rules:**
- Date is when the document was created (not last modified)
- Topic slug uses kebab-case, max 4-5 words
- Historical/completed docs stay in place for context — don't delete
- `notes/` subdirectory is for standalone reference docs not tied to sprints

## Browsing

Files sort chronologically by default. To find active work:
```
ls planning/*--plan--* planning/*--tracker--*    # Active plans & trackers
ls planning/*--review--*                         # QA findings
ls planning/*--analysis--*                       # Historical investigations
```
