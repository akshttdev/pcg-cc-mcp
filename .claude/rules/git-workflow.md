# Git & PR Workflow Standards

## Worktree Safety

- **ALWAYS confirm the current worktree before making any changes.** Run `git worktree list` and `pwd` to verify you are in the correct worktree for the task.
- Planning docs are the **source of truth** for which worktree a task uses. Every planning doc MUST include a `Worktree` field in the header (see Planning section below).
- Before starting work in a session, read the planning doc and confirm the worktree path matches your current directory.
- **NEVER switch worktrees without explicit user approval.** If you suspect you're in the wrong worktree, stop and ask.
- When multiple worktrees exist, be aware that staged/unstaged changes in one worktree are invisible to others — do not assume files staged by another process belong to your task.

## Commits

- Always include PR number (`#N`) in commit messages when working on a PR
- Use conventional commit prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`
- Never amend published commits — create new commits instead
- Never skip hooks (`--no-verify`) or force push without explicit user approval

## Pull Requests

- Always write a planning file to `planning/` before implementing large features
- Planning file format: `YYYY-MM-DD--<type>--<topic>.md` (types: plan, review, tracker, analysis, release, reference)
- PR descriptions must include a test plan with checkboxes
- Never push or merge to main without explicit user approval

## Planning Doc Worktree Requirements

Every planning doc MUST include a **Worktree** field in the header metadata, immediately after **Branch**:

```markdown
**Date**: 2026-03-18
**Branch**: `refactor/my-feature`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Base**: `main`
```

- For root worktree: `(root)` suffix
- For agent/isolated worktrees: full path, e.g., `.claude/worktrees/main-worktree`
- At the start of any session continuing a plan, **read the planning doc first** and confirm you are in the documented worktree before making changes
- If the worktree no longer exists or has changed, update the planning doc and confirm with the user

## Branch Hygiene

- Keep individual commit history on feature branches (user preference)
- Squash merge to main via PR
- After squash merge, sync branch with `git merge origin/main` (not rebase)

## Pushing

- **Only push when the user explicitly says to.** Don't auto-push after commits.
- Before pushing, run the local CI checks below. Don't push with known failures.

## Local CI Checks (run before push/merge)

All of these must pass before pushing or requesting merge:

```bash
# 1. Rust formatting
cargo fmt --all -- --check

# 2. Rust linting (use exact flags from .github/workflows/ci.yml)
flox activate -- cargo clippy --all --all-targets -- -D warnings [CI -A flags]

# 3. TypeScript
cd frontend && npx tsc --noEmit

# 4. ESLint
cd frontend && npx eslint . --ext ts,tsx --max-warnings [CI threshold]

# 5. Type generation
npm run generate-types:check
```

The `/check` skill runs all of these. Use it as shorthand.

**Important**: Use the **exact** clippy `-A` flags from `ci.yml` — the flag list changes over time. If a lint name is invalid (e.g., renamed in newer Rust), clippy treats it as a hard error.

## QA Before Merging

- Run `/check` (local CI) before creating PR — all checks must pass
- Run `/qa-review` before requesting merge to main
- For frontend changes: run `/playwright-smoke` on affected pages
- For backend changes: run `cargo test --workspace` inside flox
- Run E2E demo tests: `FRONTEND_PORT=<port> npx playwright test e2e/demos/ --reporter=list`
- Playwright config auto-loads `.env` (GITHUB_TOKEN, etc.) — no need to export manually
- Reference: existing E2E tests in `e2e/`

## Tracking Failures

- Never skip or ignore test failures without tracking them
- Pre-existing failures that can't be fixed in the current sprint go in `planning/BACKLOG--remaining-work.md` with resolution recommendations
- Include actionable fix suggestions (not just "this is broken")
- Categorize: code bug vs env dependency vs seed data vs infrastructure
