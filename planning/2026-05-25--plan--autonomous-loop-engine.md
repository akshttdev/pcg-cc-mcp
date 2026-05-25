# Autonomous Dev Loop Engine — Implementation Plan

**Date**: 2026-05-25
**Branch**: `auri/58004041`
**Worktree**: `/home/pythia/auri-workspaces/58004041`
**Base**: `main`
**Status**: PLANNING → IMPLEMENTING (skeleton)
**Inputs**: Master Bodhi (vision), Madhav (reactive architecture refinement)

---

## Vision

Build an autonomous development loop where three sub-agents — **Auri** (coder),
**Playwright** (test runner), and **Fake User** (UX stress) — operate as a
single self-reinforcing system. Rather than a fixed 6–8 hour timed loop, the
default mode is **reactive**: every commit or failure triggers all three
sub-agents in parallel, and they compound on each other's outputs.

A **sustained loop mode** is also supported for overnight sprints. Both modes
share the same circuit breaker so the system halts cleanly instead of burning
credits on the Ralph Wiggum failure pattern
(https://www.reddit.com/r/ClaudeCode/comments/1q2qvta/).

## Why this lives here

`crates/loop-engine` joins the existing orchestration stack:

| Crate                 | Role                                                |
|-----------------------|-----------------------------------------------------|
| `nora`                | Executive AI assistant (voice, planning)            |
| `executors`           | Pluggable AI coding-agent backends                  |
| `services`            | Business logic, git ops, GitHub                     |
| `loop-engine` *(new)* | Reactive harness that wires agents into a feedback loop |

The engine consumes the existing executor abstraction — it does not replace it.

## Non-Goals (this sprint)

- **No** new database tables. State for the first cut is in-memory; persistence
  comes later once we know the shape we want.
- **No** new HTTP routes. The harness runs as a library; the server crate can
  expose it later via an SSE endpoint mirroring `/api/events/processes/...`.
- **No** binding to a specific LLM. Agents are traits; Hermes / Claude / Gemini
  back-ends slot in behind the trait.

## Deliverables for this sprint

1. `LOOP_ENGINE_ARCHITECTURE.md` — full spec with Mermaid diagrams (reactive
   mode + sustained loop mode).
2. `crates/loop-engine/` — workspace crate with:
   - `CircuitBreaker` (max retry, consecutive-failure detection, graceful halt)
   - `Orchestrator` (reactive event-driven harness)
   - `Agent` trait + three skeleton impls (Auri, Playwright, FakeUser)
   - `EventBus` (in-memory broadcast channel)
   - `sustained_loop()` helper for the time-boxed mode
3. Planning file (this document).

A working test PR against Model Pickleball / Powerclub Global web is **next
sprint** — it depends on the executor back-end being wired and on Nora signing
off on the circuit-breaker thresholds.

## Architecture Snapshot

See `LOOP_ENGINE_ARCHITECTURE.md` for the full picture, including diagrams.
The TL;DR:

- `Orchestrator` owns an `EventBus` and a `CircuitBreaker`.
- An external trigger (a commit hook, a CI failure webhook, or a manual call)
  publishes `LoopEvent::CommitCompleted` or `LoopEvent::FailureDetected`.
- Subscribed agents react in parallel via `tokio::spawn`. Each agent emits its
  own follow-up events; the orchestrator forwards them back through the bus.
- The circuit breaker tracks consecutive failures per "cause hash" (so retrying
  the *same* broken logic trips the breaker faster than diverse failures do).
- On trip → `LoopEvent::CircuitTripped` → orchestrator halts and writes a
  `HaltReport`.

## Open Questions for Nora

1. **Circuit breaker thresholds.** Default proposed: 3 consecutive failures on
   the same cause hash, OR 8 total failures in any 30-minute window. Adjust?
2. **Sustained-loop default duration.** Proposed: 6 hours, hard cap 8.
3. **Agent back-end for Auri inside the loop.** The host process is already
   `claude-code`; do we want the loop to invoke a *separate* coding agent
   (e.g. Hermes) for isolation, or feed back into the parent?
4. **Where do reports go?** Proposed: `planning/reports/` as Markdown +
   `report.json`. Confirm.

## Milestones

| #  | Milestone                                                  | Owner    | Status |
|----|------------------------------------------------------------|----------|--------|
| M1 | Planning doc + architecture spec                           | Auri     | DONE   |
| M2 | Crate skeleton compiles, unit tests for circuit breaker    | Auri     | DONE   |
| M3 | Orchestrator wiring + in-memory event bus                  | Auri     | DONE   |
| M4 | Agent trait + three stub impls                             | Auri     | DONE   |
| M5 | Hermes back-end behind `Agent` trait                       | Auri     | NEXT   |
| M6 | Working test PR against Powerclub Global web               | Auri+Nora| NEXT   |

## Risks

- **Credit burn.** Loops that retry against unrealistic failure modes can
  spend large amounts of API credit. The circuit breaker is the primary
  mitigation. A daily $ budget is a future addition.
- **Worktree collisions.** Auri commits trigger the loop, but if the loop's
  own commits re-trigger it, we infinite-loop. Mitigation: tag loop-generated
  commits with a header (`Loop-Engine: auri`) and skip them in the trigger.
- **Test flakiness.** Playwright flakes will look like real failures. The
  circuit breaker's cause-hash distinguishes "always same error" (real bug)
  from "different errors each retry" (likely flake), but isn't a complete
  fix.

## File List

| Path                                                  | What                                   |
|-------------------------------------------------------|----------------------------------------|
| `planning/2026-05-25--plan--autonomous-loop-engine.md`| This doc                               |
| `LOOP_ENGINE_ARCHITECTURE.md`                         | Full spec + Mermaid diagrams           |
| `crates/loop-engine/Cargo.toml`                       | New crate manifest                     |
| `crates/loop-engine/src/lib.rs`                       | Public surface                         |
| `crates/loop-engine/src/circuit_breaker.rs`           | Ralph Wiggum prevention                |
| `crates/loop-engine/src/events.rs`                    | LoopEvent + EventBus                   |
| `crates/loop-engine/src/orchestrator.rs`              | Reactive harness                       |
| `crates/loop-engine/src/sustained.rs`                 | Time-boxed loop helper                 |
| `crates/loop-engine/src/agents/mod.rs`                | `Agent` trait + registry               |
| `crates/loop-engine/src/agents/auri.rs`               | Coding-agent adapter (skeleton)        |
| `crates/loop-engine/src/agents/playwright.rs`         | Test agent (skeleton)                  |
| `crates/loop-engine/src/agents/fake_user.rs`          | UX stress agent (skeleton)             |
| `crates/loop-engine/src/report.rs`                    | `HaltReport` writer                    |
| `Cargo.toml`                                          | Workspace member entry                 |
