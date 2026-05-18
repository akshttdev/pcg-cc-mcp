# Session Handoff

**Date**: 2026-04-21
**Branch**: `feature/graph-viz-enhancement`
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp`
**Last pushed commit**: `4bccf85e` — feat(graph-viz): phase 3 — Intelligence › Topology tab + drive-by fixes
**Reason for handoff**: Bodhi stepping away on an errand; wants to resume even if this terminal is lost.

## Quick-start for the next Claude

1. Read this file.
2. Read `planning/2026-04-21--plan--graph-viz-integration-roadmap.md`
   **§8 in particular** — it has the full Q&A clarification round from
   today's session. That's the most up-to-date thinking.
3. Ask Bodhi which of the three next moves he wants:
   - (a) Answer the pending clarification questions listed in §8.13
   - (b) Confirm the integration plan and start Phase 4
   - (c) Something else
4. **Do not re-interrogate Bodhi** on things already answered in §8 —
   that's a lot of answered questions. Scan §8.1–§8.13 first.

## What was done this session

### Shipped (Phases 0–3, 4 commits, all pushed to origin)

- **Phase 0 — foundation**: `entity_graph.rs` readers (`global_subgraph`,
  `org_subgraph`, `subgraph_focused`), `NodeActionRegistry`
  (`node_actions.rs`), `filter_visible_nodes` (`graph_access.rs`),
  5 new graph routes.
- **Phase 1 — materialization**: 7 `sync_*_graph` functions +
  migration `20260428100000_proposals_lead_contact_id.sql`. Smoke
  example at `crates/db/examples/sync_graph_smoke.rs` verifies 348
  nodes / 271 edges against `dev_assets/db.sqlite`, idempotent.
- **Phase 2 — frontend canvas**: `frontend/src/components/topology/`
  stack (TopologyShell, TopologyCanvas, useForceLayout,
  useTopologyData, toolbar, previews). `lib/graph/adapter.ts` and
  `lib/api/topology.ts`. `d3-force` + `@types/d3-force` installed.
- **Phase 3 — intel tab mount**: `TopologyIntel.tsx` renders shell +
  legacy Archive sub-tab. `ViewInTopologyButton` on company profile.
  Deep-link params (`focus_type`, `focus_id`, `depth`, `archive=1`).
  Fullscreen toggle.

### Drive-by fixes landed in commit 4

- `Organization` struct SELECT drift — added `wallet_address` +
  `vibe_budget_total` to 4 queries in `crates/db/src/models/user.rs`.
  Fixed `/api/organizations/:id` 500 error.
- `/perlin-noise.png` missing — replaced with runtime
  `buildNoiseTexture()` DataTexture in `OrbVisualizer.tsx`.
- React duplicate-copy error from errant `npm install d3-force` —
  recovered via `rm -rf frontend/node_modules && pnpm install` +
  `rm -rf frontend/node_modules/.vite`.

### Live-verified via headed Playwright

- Logged in as `admin` / `admin123`
- Powerclub Global → Intelligence → Topology shows 55 nodes / 71 edges
- All 7 testids present (`topology-canvas`, etc.)
- Pan/zoom, 2D/3D toggle, filter pane, search, Archive sub-tab all
  working.

### Planning work — THIS IS CRITICAL

- **`planning/2026-04-21--plan--graph-viz-integration-roadmap.md`** —
  new integration plan aligning Phases 4–10 with the 5-year roadmap.
  ~1000 lines, 5 diagrams.
- Added **§8 Use-case clarifications** capturing a rich Q&A round
  with Bodhi on 2026-04-21. This rewrites the product framing:
  - Nora ≡ Topsi (same engine, scoped roles)
  - APN / Pythia hive-mind is first-class
  - VIBE tokens are the currency
  - Workflow taxonomy = Dev/Social/Sales/Client
  - Bodhi's "I love this" moment = voice-replaces-CLI
- Added **§9 Plan impact** — new phases forced by the clarifications:
  Phase 1b (external ingest), 5c (VIBE wallet), 5d (deliverable
  approval gate), 11 (APN integration).

### Memory updates

- `/Users/sirakstudios/.claude/projects/-Users-sirakstudios-topos/memory/reference_nora_topsi_agents.md`
  rewritten. Old version said they were distinct agents with
  different personas. **New version**: they're the SAME engine with
  different `scope` fields. Memory notes this supersedes.

## What remains (uncommitted as of handoff)

**Two files not yet committed** — these need to be committed + pushed
before Bodhi can resume from another machine:

1. `planning/2026-04-21--plan--graph-viz-integration-roadmap.md`
2. `planning/SESSION-HANDOFF.md` (this file)

The memory file at `~/.claude/.../reference_nora_topsi_agents.md` is
**local to this machine** — it does NOT travel to another dev box. If
Bodhi resumes on a different machine, the memory system there won't
have the updated Nora/Topsi clarification. The next Claude should
re-save the memory on any new machine after reading §8.2 of the plan.

## Pending clarification questions

Listed in plan §8.13. None block Phase 4 start — they're
Phase-5-and-beyond concerns:

- **APN-1**: current RustDesk flow details + aspirational MCP-daemon model
- **APN-2**: peer enrollment mechanics
- **APN-3**: do peers appear as graph nodes
- **VIBE-1**: user → VIBE origination (earn / allocate / purchase)
- **VIBE-2**: cost-deduction timing (pre / post / confirm)
- **Orch-1**: Madhav's orchestration layer status
- **Orch-2**: Nora-vs-orchestration boundary
- **WF-1**: workflow taxonomy as perspective / nav / tags
- **Del-1**: deliverable approval UX specifics
- **Disc-1**: past-content discovery cache vs live
- **Sec-1**: Topsi visibility when Bodhi/Admin read through it

## Next action when Bodhi returns

Default recommendation: **commit + push the two uncommitted files
FIRST** so they survive terminal loss. Then let Bodhi choose between:

- (a) Answering the 11 pending clarifications (clears Phase 5+ runway)
- (b) Starting Phase 4 implementation (perspective framework +
      aggregation/collapse — §9 moved aggregation forward)
- (c) Something else Bodhi wants

## Servers (as of handoff)

- **Backend**: port 3001 (running, PID 11075, target/debug/server)
- **Frontend**: port 3000 (running, PID 21016, Vite dev)
- **Cargo-watch**: on `crates/` (auto-rebuilds backend)
- **Vite started with**: `BACKEND_PORT=3001 npx vite --port 3000 --host`
  (this is important — vite's proxy defaults to port 58297 unless
  BACKEND_PORT is set, which is what caused ECONNREFUSED earlier.)

## Branch state

- Branch: `feature/graph-viz-enhancement`
- Remote: `origin/feature/graph-viz-enhancement` (up to date through
  `4bccf85e`)
- Main: NOT touched. Bodhi explicitly directed "do not merge anything
  into main."
- Repo remote moved from `KingBodhi/pcg-cc-mcp` →
  `Powerclub-Global/pcg-cc-mcp`. Push still works against old URL via
  redirect; worth updating `origin` when convenient:
  `git remote set-url origin https://github.com/Powerclub-Global/pcg-cc-mcp.git`

## Key context for the next Claude

- **Bodhi's directive throughout this session**: "no commit, keep
  building locally" — until explicitly told to commit. When he said
  "commit and push to branch but do NOT merge into main" that was
  the opening to commit. Phases 0–3 landed; §8/§9 plan updates are
  the remaining commit owed.
- **Nora and Topsi are NOT two separate agents** — this changed in
  today's session. If an earlier memory entry says they're distinct,
  trust §8.2 of the integration plan and the updated memory file.
- **Bodhi asked "ask as many questions as needed"** — he answered 20+
  clarification questions today. Respect that investment. Don't
  re-litigate resolved things.
- **APN is a vision component, not vaporware** — 3 PCs exist today
  running peer nodes; Bodhi uses RustDesk to drive them. Dashboard
  becomes the cockpit. This is why Phase 11 (APN integration) is
  real, not speculative.
- **Orchestration layer boundary**: Madhav is building a separate
  planning/dispatch layer. Our Phase 5 should be an adapter, not
  duplicate logic. If Bodhi asks about orchestration, defer to
  Madhav's work unless explicitly told otherwise.

## To resume from a fresh session on this machine

```bash
cd /Users/sirakstudios/topos/pcg/github/pcg-cc-mcp
git status                 # should show this handoff file + the new plan file uncommitted
cat planning/SESSION-HANDOFF.md   # this file
```

Then tell the new Claude: "Read `planning/SESSION-HANDOFF.md` and
continue."

## To resume from a DIFFERENT machine

After commit + push (which will happen next):

```bash
git clone https://github.com/Powerclub-Global/pcg-cc-mcp.git
cd pcg-cc-mcp
git checkout feature/graph-viz-enhancement
cat planning/SESSION-HANDOFF.md
```

**Note**: the memory update at
`~/.claude/.../reference_nora_topsi_agents.md` is local to the
original machine. On a different machine, the next Claude should
read §8.2 of the integration plan and re-save the memory entry.

---

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
