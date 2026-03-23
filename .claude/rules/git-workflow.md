# Git & PR Workflow Standards

## Worktree Safety

- **ALWAYS confirm the current worktree before making any changes.** Run `git worktree list` and `pwd` to verify.
- Planning docs are the **source of truth** for which worktree a task uses. Every planning doc MUST include a `Worktree` field in the header.
- **NEVER switch worktrees without explicit user approval.**

## Commits

- Use conventional commit prefixes: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `test:`
- Include PR number (`#N`) when working on a PR
- Never amend published commits — create new commits instead
- Never skip hooks (`--no-verify`) or force push without explicit user approval

## Pushing & Merging

- **Only push when the user explicitly says to.** Don't auto-push after commits.
- **Run `/check` before every push.** All checks must pass. Don't push with known failures.
- **Never push or merge to main without explicit user approval.**
- `/check` runs all checks below. Or run manually from **project root**:
  1. `cargo fmt --all` then `cargo fmt --all -- --check` (format first — `--check` has caching that misses files)
  2. `flox activate -- cargo clippy --all --all-targets -- -D warnings [flags from ci.yml]`
  3. `cd frontend && npx tsc --noEmit` (from `frontend/`)
  4. `cd frontend && npx eslint . --ext ts,tsx` (from `frontend/`)
  5. `npm run generate-types:check` (from **project root**, not `frontend/`)

## Pull Requests

- Always write a planning file to `planning/` before implementing large features
- Planning file format: `YYYY-MM-DD--<type>--<topic>.md`
- PR descriptions must include a test plan with checkboxes
- Run `/qa-review` before requesting merge to main
- For frontend changes: also run `/playwright-smoke` on affected pages
- For backend changes: also run `cargo test --workspace` inside flox
- Branch-specific deployment instructions go in the planning file, not the project deployment guide

## Planning Doc Requirements

Every planning doc MUST include a **Worktree** field:

```markdown
**Date**: 2026-03-18
**Branch**: `refactor/my-feature`
**Worktree**: `/Users/mediamonsters/topos/pcg-cc-mcp` (root)
**Base**: `main`
```

- At session start, **read the planning doc first** and confirm worktree matches
- If the worktree no longer exists, update the doc and confirm with the user

## Branch Hygiene

- Keep individual commit history on feature branches
- Squash merge to main via PR
- After squash merge, sync branch with `git merge origin/main` (not rebase)

## Tracking Failures

- Never skip or ignore test failures without tracking them
- Pre-existing failures go in `planning/BACKLOG--remaining-work.md` with resolution recommendations
- Categorize: code bug vs env dependency vs seed data vs infrastructure
