# Claude Code Platform Standards & Tech Debt Prevention

**Branch:** `feat/claude-code-standards`
**PR:** [#32](https://github.com/KingBodhi/pcg-cc-mcp/pull/32)
**Status:** Implementing

---

## Problem

Tech debt review (2026-03-15) identified recurring patterns that could be prevented at authoring time:
- 100+ `.unwrap()` calls in production Rust code
- 257 `any` type instances in frontend TypeScript
- Monolithic files (api.ts 6.6K, org-profile 7K lines)
- Inconsistent error handling, state management, and git workflow

Currently no automated guardrails exist to prevent these patterns from being introduced in new code.

## Solution

Leverage Claude Code's configuration system to enforce standards automatically:

1. **Rules** (`.claude/rules/`) — path-scoped instructions auto-loaded when editing matching files
2. **Skills** (`.claude/skills/`) — reusable slash commands for code review and checks
3. **Hooks** (`.claude/hooks/`) — PostToolUse scripts that warn on violations in real-time
4. **Settings** (`.claude/settings.json`) — wire hooks to Edit/Write tool calls

## Files Added

| File | Purpose |
|------|---------|
| `.claude/rules/rust-standards.md` | Ban unwrap in prod, error handling, concurrency, DB, validation |
| `.claude/rules/frontend-standards.md` | Ban `any`, 500-line limit, error boundaries, state separation |
| `.claude/rules/git-workflow.md` | Conventional commits, PR process, planning files |
| `.claude/skills/check/SKILL.md` | `/check` — run all project checks (tsc, lint, clippy, fmt, tests) |
| `.claude/skills/review-rust/SKILL.md` | `/review-rust` — audit Rust changes for violations |
| `.claude/skills/review-frontend/SKILL.md` | `/review-frontend` — audit TS changes for violations |
| `.claude/hooks/check-rust-unwraps.sh` | Warn on `.unwrap()`/`.expect()` in edited .rs files |
| `.claude/hooks/check-frontend-any.sh` | Warn on `any` type in edited .ts/.tsx files |
| `.claude/settings.json` | Wire PostToolUse hooks to Edit/Write |
| `.gitignore` | Track .claude/{rules,skills,hooks,settings.json}, ignore local overrides |
| `planning/notes/2026-03-15--analysis--tech-debt-review.md` | Full tech debt report |

## Sharing Strategy

- `.claude/*` ignored by default (keeps worktrees, local state out of git)
- Selective un-ignore for rules/, skills/, hooks/, settings.json, CLAUDE.md
- `.claude/settings.local.json` stays gitignored for personal overrides
- Anyone cloning the repo gets standards automatically

## Verification

- [ ] `.claude/` files appear in git status after clone
- [ ] Rules auto-load when editing matching paths
- [ ] `/check` skill runs all checks successfully
- [ ] `/review-rust` catches unwrap in test file
- [ ] `/review-frontend` catches any type in test file
- [ ] Hooks fire warnings after Edit/Write on .rs and .ts files
- [ ] `.claude/settings.local.json` stays out of git
