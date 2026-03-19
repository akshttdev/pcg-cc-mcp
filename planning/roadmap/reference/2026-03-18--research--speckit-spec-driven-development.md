# Spec Kit & Spec-Driven Development — Deep Research

**Date:** 2026-03-18
**Type:** Research / Reference
**Purpose:** Extract principles, tools, and techniques from GitHub's Spec Kit and the broader spec-driven development (SDD) ecosystem applicable to our ORCHA agent orchestration system and task management processes.

---

## What is Spec Kit?

[Spec Kit](https://github.com/github/spec-kit) is GitHub's open-source toolkit (78k+ stars) for **Spec-Driven Development** — a methodology where specifications become living, executable artifacts that directly generate implementations. Built as a Python CLI, it works with 20+ AI coding agents (Claude Code, Copilot, Gemini CLI, Cursor, Windsurf, etc.) through a pluggable template system.

The core insight: **"vibe coding" (freeform prompting) fails at scale** because agents excel at pattern completion but not mind-reading. Specs provide the missing intent, constraints, and acceptance criteria that agents need to produce correct code.

---

## Architecture & Project Structure

### .specify Directory Layout

```
.specify/
├── templates/              # Core SDD templates
│   └── overrides/         # Project-local overrides (highest priority)
├── extensions/            # Installed extensions (add capabilities)
│   └── <ext-id>/templates/
├── presets/               # Active presets (customize behavior)
│   └── <preset-id>/templates/
├── constitution.md        # Immutable architectural principles
├── specification.md       # Feature requirements (the "what")
├── implementation-plan.md # Technical architecture (the "how")
└── tasks.md              # Actionable task breakdown
```

### Per-Spec Artifact Structure

Each spec generates ~8 files:
- `specification.md` — User stories, requirements, boundary conditions
- `implementation-plan.md` — Architecture, tech stack, integration patterns
- `tasks.md` — Numbered, sequential work items with "done" criteria
- `data-model.md` — Schema definitions
- `api.md` — API contract definitions
- `component.md` — Component architecture
- `research.md` — Research notes and findings
- `constitution.md` — Immutable project principles

### Template Resolution (Precedence Order)

1. **Project-local overrides** (`.specify/templates/overrides/`) — Highest priority
2. **Active presets** (`.specify/presets/<preset-id>/templates/`)
3. **Installed extensions** (`.specify/extensions/<ext-id>/templates/`)
4. **Core templates** (`.specify/templates/`) — Lowest priority

**Application to ORCHA:** This layered override system — where project-specific configs take precedence over organization-level defaults, which take precedence over platform defaults — is a clean pattern for our agent configuration hierarchy.

---

## Five-Phase Workflow

### Phase 0: Constitution (`/speckit.constitution`)

Establishes **immutable architectural principles** and development guidelines. The constitution is the constraint anchor — it doesn't change per feature.

Example content: "All API endpoints must use REST conventions," "Database migrations must be reversible," "No direct DOM manipulation — use React abstractions."

**Application to ORCHA:** This maps to our need for a Vision/Mission anchor document (from SudoLang research). The constitution is the "laws" that no agent may violate.

### Phase 1: Specify (`/speckit.specify`)

Captures **what** to build — user journeys, business outcomes, boundary conditions. No technical decisions yet. Output: `specification.md`.

Key questions answered:
- Who will use this?
- What problem does it solve?
- How will they interact with it?
- What outcomes matter?

**Application to ORCHA:** Before any agent starts implementation, force a specification phase that captures user intent separate from technical approach.

### Phase 2: Plan (`/speckit.plan`)

Captures **how** to build it — architecture, stack, compliance requirements, integration patterns. Output: `implementation-plan.md`.

The plan bridges abstract requirements and concrete tasks. Developers provide constraints (preferred stack, existing patterns, compliance rules), and the agent generates a comprehensive technical plan.

**Application to ORCHA:** Our workflow planning should explicitly separate "what" (spec) from "how" (plan). Agents can generate multiple plan variations for comparison.

### Phase 3: Tasks (`/speckit.tasks`)

Decomposes spec + plan into **discrete, testable work units**. Each task has:
- Sequential numbering with dependency ordering
- Definition of "done" criteria
- Integration checkpoints
- Phase grouping with `[P]` markers for parallelizable tasks

Output: `tasks.md`.

**Application to ORCHA:** Our task decomposition should produce structured task lists with explicit done criteria, dependency ordering, and parallel-execution markers.

### Phase 4: Implement (`/speckit.implement`)

Agents execute tasks one-by-one (or in parallel via team-implement). Each task produces a focused, reviewable change — not thousand-line dumps.

The agent knows:
- **What** to build (specification)
- **How** to build it (plan)
- **What specifically** to work on (task)

### Optional Enhancement Commands

| Command | Purpose |
|---------|---------|
| `/speckit.clarify` | Resolve underspecified areas before planning |
| `/speckit.analyze` | Cross-artifact consistency validation |
| `/speckit.checklist` | Quality validation (specs as testable requirements) |

---

## Multi-Agent Parallel Execution (`/speckit.team-implement`)

This is Spec Kit's most sophisticated pattern, directly applicable to ORCHA.

### File-Conflict Analysis Algorithm

Tasks are grouped into parallel work streams based on **file paths** (not keywords):

1. Extract file paths from task descriptions via regex
2. Group by two-segment path prefix (`backend/src/`, `frontend/src/`)
3. Build conflict graph — connected components = independent streams
4. Tasks touching the same files are forced into the same stream

> "File paths are unambiguous" — keyword detection creates false matches.

**Application to ORCHA:** When parallelizing agent work, use file-path analysis to determine safe parallel boundaries. Two agents editing the same file = guaranteed conflicts.

### Team Composition

- **Implementation agents** — One per detected work stream (backend-dev, frontend-dev)
- **QA/Security gatekeeper** — Always present, catches blind spots implementers miss
- **Lead coordinator** — Main session that assigns tasks, monitors progress, gates phases

> "Who watches the watcher?" — A dedicated reviewer with fresh context catches placeholder code, security violations, and quality gate failures that implementers ignore.

**Application to ORCHA:** Every orchestrated workflow should include a mandatory review agent, separate from the implementing agents.

### Self-Organizing Execution

Agents autonomously claim tasks from a shared list via file locking. After finishing a task, agents pick the next unassigned, unblocked task automatically. This eliminates micro-management and enables natural load balancing.

**Application to ORCHA:** Agent task claiming should be self-organizing where possible, not centrally dispatched for every item.

### Coordination Mechanisms

- **Phase gating** — Phase N+1 blocked until Phase N complete + QA verified
- **Story dependencies** — Frontend `[US1]` waits for backend `[US1]`
- **E2E test dependencies** — E2E tests depend on relevant implementation completion
- **Atomic commits per task** — One commit per completed task (not batches)

### Optimal Team Size

**2-4 agents** is the sweet spot. More creates coordination overhead without proportional gains. ~5-6 tasks per agent is ideal.

### Spec Quality Amplification

> "With sequential execution, you can course-correct ambiguity mid-stream. With parallel agents working independently, vague specs lead to inconsistent assumptions across teammates."

Investment in specification quality pays **exponential** dividends with parallel execution. This is the strongest argument for the SDD approach.

---

## Multi-Agent Orchestration (speckit-agents)

[speckit-agents](https://github.com/sbhavani/speckit-agents) demonstrates a PM + Developer agent pair:

### Agent Roles

| Agent | Responsibilities |
|-------|-----------------|
| **PM Agent** | Reads PRD, prioritizes features, answers clarification questions, records learnings to `.agent/product-manager.md` |
| **Dev Agent** | Executes full Spec Kit workflow, creates isolated worktrees, queries PM Agent during implementation, creates PRs |

### Orchestration State Machine

```
PM suggests feature from PRD
  → Human approves/rejects (Mattermost)
  → Dev executes: /speckit.specify → /speckit.plan → /speckit.tasks → /speckit.implement
  → During implementation: Dev asks questions → PM responds contextually
  → Human can intervene at any checkpoint
  → Dev creates PR from isolated worktree
  → Cycle repeats
```

**Key pattern:** Specs serve as the **communication substrate** between agents. The PM's PRD informs feature suggestions; Spec Kit's structured outputs (SPEC.md, PLAN.md, TASKS.md) guide the Dev Agent. Document-centric collaboration enables async coordination without tight coupling.

**Application to ORCHA:** Agent coordination via structured documents (specs, plans, task lists) is more robust than conversational hand-off. Documents persist across sessions and are inspectable by humans.

---

## Critical Analysis & Limitations

### The Waterfall Criticism (Scott Logic)

A [practical evaluation](https://blog.scottlogic.com/2025/11/26/putting-spec-kit-through-its-paces-radical-idea-or-reinvented-waterfall.html) found:

- **Excessive overhead for simple tasks** — 189-line constitution + 230-line spec + 2000+ lines of plan for a simple feature. 33 min agent time + 3.5 hours review.
- **A "simple and very obvious bug"** despite all specification work
- **10x faster** using traditional iterative prompting without SDD, with zero defects
- **Waterfall regression** — Sequential constitution → spec → plan → implement mirrors waterfall despite industry's move toward agile

### Martin Fowler's Analysis

[Fowler's comparison](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html) of Spec Kit vs Kiro vs Tessl:

- **Scalability issues** — Kiro applied 16-criteria workflows to trivial bug fixes ("sledgehammer to crack a nut")
- **Non-determinism** — Agents sometimes ignored research notes or misinterpreted existing patterns
- **Spec/implementation confusion** — Weak discipline separating requirements from technical details
- **MDD parallel** — Specs at "awkward abstraction level" create overhead without business value

### Resolution: SDD Scales with Complexity

The critics are right for simple tasks. But the parallel execution findings tell the opposite story for complex, multi-agent work:

> Investment in specification quality pays exponential dividends with parallel execution.

**Our takeaway:** Don't apply SDD uniformly. Use it for complex, multi-agent, multi-phase work. Skip it for simple bug fixes and single-file changes. Match ceremony to complexity.

---

## Comparison: Spec Kit vs ORCHA Patterns

| Dimension | Spec Kit | ORCHA (Current) | Gap/Opportunity |
|-----------|----------|-----------------|-----------------|
| **Spec artifacts** | 8 structured files per feature | Freeform task descriptions | Add structured spec + plan artifacts |
| **Constitution** | Immutable project principles | CLAUDE.md (conventions) | Formalize immutable architectural constraints |
| **What/how separation** | Spec (what) separate from Plan (how) | Mixed in task descriptions | Separate user intent from technical approach |
| **Task decomposition** | Sequential numbering, done criteria, `[P]` parallel markers | Flat task list | Add dependency ordering + parallel markers |
| **Parallel execution** | File-conflict analysis, stream detection, team-implement | Serial agent execution | Add file-path-based parallel boundary detection |
| **QA agent** | Mandatory separate reviewer per workflow | No dedicated QA agent | Add mandatory review agent to orchestration |
| **Human checkpoints** | Approve/reject between phases | Ad hoc review | Formalize human gates between spec/plan/implement |
| **Template system** | 4-tier override hierarchy | No template system | Add layered config: platform → org → project → task |
| **Cross-artifact validation** | `/speckit.analyze` checks consistency | No cross-artifact checks | Add consistency validation between related artifacts |
| **Multi-agent coordination** | Document-centric (specs as communication substrate) | Conversational | Use structured documents for inter-agent communication |
| **Complexity scaling** | Full SDD for complex, skip for simple | Uniform process | Match ceremony to complexity |

---

## Actionable Recommendations for ORCHA

### High Priority

1. **Structured Specification Artifacts** — For complex agent tasks, generate a structured spec (what to build, who it's for, acceptance criteria) and a separate plan (how to build it, architecture, constraints) before implementation begins. These serve as both the "contract" for the implementing agent AND the communication substrate for multi-agent coordination.

2. **File-Path-Based Parallel Boundary Detection** — When orchestrating parallel agent work, analyze task descriptions for file paths to determine safe parallel boundaries. Tasks touching overlapping files must be serialized or assigned to the same agent. This prevents merge conflicts without manual coordination.

3. **Mandatory Review Agent** — Every multi-agent workflow should include a dedicated QA/review agent that is separate from implementing agents. "Who watches the watcher?" — a fresh-context reviewer catches placeholder code, security violations, and quality gate failures that implementers miss.

### Medium Priority

4. **Layered Configuration Hierarchy** — Implement a 4-tier template/config system: platform defaults → organization overrides → project overrides → task-specific overrides. Higher-specificity configs take precedence. This enables standardization without rigidity.

5. **Complexity-Scaled Ceremony** — Don't apply full SDD to every task. Simple bug fixes get minimal ceremony; complex multi-agent features get full spec → plan → tasks → implement → review pipeline. Match process weight to task complexity.

6. **Document-Centric Agent Coordination** — Use structured documents (specs, plans, task lists) as the communication substrate between agents, not conversational hand-off. Documents persist across sessions, are inspectable by humans, and enable async collaboration.

7. **Cross-Artifact Consistency Validation** — Add a validation step that checks consistency between related artifacts (spec requirements match plan architecture, plan components map to tasks, tasks cover all spec requirements). Spec Kit's `/speckit.analyze` catches drift between artifacts.

### Lower Priority

8. **What/How Separation in Task Definitions** — Separate user intent ("what") from technical approach ("how") in task metadata. The spec captures requirements; the plan captures architecture. Changing the "how" shouldn't require rewriting the "what."

9. **Atomic Commits Per Task** — When agents execute multi-task workflows, enforce one commit per completed task (not batches). Enables clean git history, easy rollbacks, and per-task review.

10. **Self-Organizing Task Claiming** — For parallel agent teams, let agents autonomously claim unblocked tasks from a shared queue rather than centrally dispatching every assignment. Natural load balancing with file-lock-based claiming.

---

## Key Takeaways

1. **Specs as executable contracts** — Specifications are not documentation — they're the source of truth that agents use to generate, test, and validate code. They bridge intent and implementation.

2. **Separate what from how** — User intent (spec) should be distinct from technical approach (plan). This enables changing architecture without rewriting requirements, and vice versa.

3. **File paths determine parallel boundaries** — The most reliable way to parallelize agent work is analyzing which files each task touches. File ownership must be a hard guarantee, not a guideline.

4. **Mandatory review agent** — "Who watches the watcher?" Every multi-agent workflow needs a dedicated reviewer with fresh context. Implementers miss their own blind spots.

5. **Spec quality amplifies with parallelism** — Vague specs are tolerable for serial execution (you course-correct mid-stream). With parallel agents, vague specs cause inconsistent assumptions across teammates. Investment in spec quality pays exponential dividends.

6. **Match ceremony to complexity** — Don't apply SDD to trivial tasks. Full spec → plan → tasks → implement → review for complex features; minimal ceremony for bug fixes. The waterfall criticism is valid for simple work but wrong for complex orchestration.

7. **Document-centric coordination > conversation** — Specs, plans, and task lists as structured documents enable async, inspectable, persistent inter-agent communication. Conversations are ephemeral and opaque.

---

## Sources

- [Spec Kit — GitHub Repository](https://github.com/github/spec-kit)
- [Spec Kit — Official Website](https://speckit.org/)
- [Spec-Driven Development with AI — GitHub Blog](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)
- [Spec Kit AGENTS.md](https://github.com/github/spec-kit/blob/main/AGENTS.md)
- [Diving Into Spec-Driven Development — Microsoft Developer Blog](https://developer.microsoft.com/blog/spec-driven-development-spec-kit)
- [Understanding SDD Tools: Spec-kit, Kiro, and Tessl — Martin Fowler](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)
- [Adding Parallel Agent Teams to Spec Kit — Dominic Böttger](https://dominic-boettger.com/blog/speckit-team-implement-parallel-ai-agent-teams/)
- [speckit-agents: Multi-Agent Orchestration — GitHub](https://github.com/sbhavani/speckit-agents)
- [Putting Spec Kit Through Its Paces — Scott Logic](https://blog.scottlogic.com/2025/11/26/putting-spec-kit-through-its-paces-radical-idea-or-reinvented-waterfall.html)
- [SDD Ecosystem Comparison — Vishal Mysore](https://medium.com/@visrow/spec-driven-development-is-eating-software-engineering-a-map-of-30-agentic-coding-frameworks-6ac0b5e2b484)
- [Spec-Driven AI Coding: Spec-kit, BMAD, Agent OS — Tim Wang](https://medium.com/@tim_wang/spec-kit-bmad-and-agent-os-e8536f6bf8a4)
