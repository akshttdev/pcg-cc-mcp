# Session handoff — 2026-05-16

**Read this first**, then delete it before starting new work.

## Where we are

The Avatar Profile Engine is **production-ready as far as security, correctness, billing, and asset isolation are concerned**. All critical gates from the original PR-#68 code review are closed, plus the production-rollout gates from `planning/2026-05-16--plan--avatar-engine-hardening.md` Phase 4.1 (cost control) and Phase 4.4 (asset isolation).

Three PRs are in flight (stacked):

| PR | State | Branch | Worktree |
|---|---|---|---|
| **#68** | **Ready for review** | `feat/avatar-profile-engine` | `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp-avatar-engine` |
| **#69** | Open, stacked on #68 | `feat/avatar-engine-billing` | same worktree |
| **#70** | Open, stacked on #69 | `feat/avatar-engine-asset-isolation` | same worktree |

URLs:
- https://github.com/Powerclub-Global/pcg-cc-mcp/pull/68
- https://github.com/Powerclub-Global/pcg-cc-mcp/pull/69
- https://github.com/Powerclub-Global/pcg-cc-mcp/pull/70

The current worktree HEAD is `feat/avatar-engine-asset-isolation`. Working tree should be clean.

## What's been verified working

- All 3 branches pushed, no unpushed commits
- Backend binary built fresh and running on port 3022
- Frontend running on port 3020 (vite, May 13 build — frontend code unchanged this session)
- Migration `20260516120000_org_member_vibe_limits` applied to `dev_assets/db.sqlite`
- 10/10 e2e tests passing on `--project=api`
- 5/5 unit tests in `crates/services/src/services/avatar_pricing`
- 5/5 unit tests in `crates/db/src/models/org_member_vibe_limit`
- All 13 🔴 blockers from the original code review are addressed

## What the next session should do

In priority order (see hardening plan's "Execution log" section for full detail):

1. **Wait for PR #68 review feedback** — that's the gating PR. Address comments before continuing the stack.
2. **Phase 4.2 — Recovery worker** (~2–3h) for orphaned `generating` rows after server crashes.
3. **Phase 4.3 — Observability metrics** (~2h) — counters, histograms, traces.
4. **Phase 4.5 — Image-of-likeness consent records** (~half day) — legal coverage before public rollout.
5. **Phase 3.1** — `render_motion` decomposition (~3–4h), unblocks render_motion billing.
6. Cleanup items in Phase 2.2 / 3.x (mechanical, no urgency).

## Critical dev-environment gotchas the next Claude should know about

### 1. flox rustfmt ≠ team rustfmt
Every `cargo build` / `cargo check` regenerates ~30 fmt-only diffs in `crates/services/`. They're import-ordering changes (`use foo::{Capitalized, lowercase}` vs `use foo::{lowercase, Capitalized}`). The team's tip commits aren't `cargo fmt --check`-clean against the flox-pinned rustfmt.

**Always run `git checkout -- crates/services/` before committing** to avoid committing fmt noise. Don't try to "fix" this by formatting and pushing — it loops.

### 2. `/login` route returns 404
The dev frontend's `/login` route 404s when rendered by React. This breaks `e2e/auth.setup.ts`, which the main playwright project depends on. Workaround: tests live in the `api` project (`--project=api`) which doesn't require auth setup. To run the full e2e suite browser-side, fix the `/login` route first.

### 3. Migration version drift on dev DB
The dev SQLite has migration rows applied from feature branches that aren't on disk in this branch (20260417*, 20260418*, 20260419*, 20260428*). `cargo sqlx migrate run` fails with "previously applied but missing in resolved migrations." Workaround used this session: apply new migrations via raw `sqlite3` and let SQLx auto-record on next boot (CREATE TABLE IF NOT EXISTS makes it idempotent). The 20260516120000 migration is currently tracked correctly.

### 4. PR #68 has one empty commit (`9f6e043dd`)
Leftover from an aborted `cargo fmt` attempt. Harmless noise. Don't try to rewrite history to remove it — the branch is published, force-push would invalidate review.

### 5. The Vite proxy from frontend → backend doesn't preserve some path edge cases
Playwright `request` defaulted to baseURL `http://127.0.0.1:3020` (frontend), but path-traversal tests need to hit the backend directly. Playwright config's `api` project overrides baseURL to `http://127.0.0.1:${BACKEND_PORT}`. Don't break this.

### 6. Backend binary doesn't auto-rebuild
The May-13 `target/debug/server` binary was the original PR #68 build. To pick up new code: kill `pid_for(3022)`, run `flox activate -- cargo build -p server`, then `flox activate -- bash -c 'source .env; target/debug/server'` in background.

### 7. Generate-types deletes `shared/testids.ts`
Running `npm run generate-types` deleted `shared/testids.ts` this session (probably ts-rs reaping non-`#[ts(export)]` files in `shared/`). Workaround: `git checkout HEAD -- shared/testids.ts` after running generate-types.

## How to pick up from here

```bash
cd /Users/sirakstudios/topos/pcg/github/pcg-cc-mcp-avatar-engine
git status                     # should be clean
git log --oneline -5           # confirm you're at the right HEAD
# read planning/2026-05-16--plan--avatar-engine-hardening.md (especially "Execution log")
# delete this file before starting work
rm planning/SESSION-HANDOFF.md
```

If a reviewer leaves comments on PR #68, do those first (`gh pr view 68 --comments`). If they want changes to the stacked PRs (#69 / #70), check them out and address inline.
