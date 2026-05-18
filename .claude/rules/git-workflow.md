# Git & PR Workflow Standards

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

## Branch Hygiene

- Keep individual commit history on feature branches (user preference)
- Squash merge to main via PR
- After squash merge, sync branch with `git merge origin/main` (not rebase)
