# SudoLang & AIDD Framework — Deep Research

**Date:** 2026-03-18
**Type:** Research / Reference
**Purpose:** Extract principles, tools, and techniques from SudoLang and the AIDD (AI-Driven Development) framework applicable to our ORCHA agent orchestration system and task management processes.

---

## What is SudoLang?

[SudoLang](https://github.com/paralleldrive/sudolang) is a **pseudocode programming language designed for LLM collaboration**, created by Eric Elliott. It combines natural language expressiveness with programming precision — typed interfaces, constraint-based programming, pattern matching, and function composition — all interpreted natively by advanced LLMs without needing to paste the specification.

The key innovation: SudoLang treats **constraints as first-class citizens**. Instead of imperative "do this, then that" instructions, you declare *what must be true* and the LLM's inference engine maintains those invariants. This is inspired by Ivan Sutherland's Sketchpad (1963) — the first constraint-based programming system.

**Performance data:** SudoLang prompts use 20-30% fewer tokens than equivalent natural language, and pseudocode prompting shows 12-38% response score improvements over plain language (O'Reilly research).

## What is AIDD?

[AIDD](https://github.com/paralleldrive/aidd) (AI-Driven Development) is the **workflow framework** built on SudoLang. It structures the full development lifecycle — from discovery through testing — into composable, AI-executable workflow commands. It uses SudoLang as the specification language for skills, prompts, and agent instructions.

---

## SudoLang Language Specification (v2.0)

### Core Syntax

```sudolang
# Variables & Assignment
name = "Atlas"
$minimumSalary = $100,000    # $ prefix disambiguates from natural language

# Template strings
greeting = "Hello, $name"

# Conditional expressions (evaluate to values)
status = if (age >= 18) "adult" else "minor"

# Range operator (inclusive)
1..20

# Pipe operator (function composition, left-to-right)
1..20 |> fibonacci |> log
data |> validate |> transform |> save

# Pattern matching with destructuring
result = match (shape) {
  case {type: "circle", radius} => "Circle: $radius";
  case {type: "rect", width, height} => "${width}x${height}";
  default => "Unknown";
}
```

### Interfaces, State & Constraints

This is the most powerful and applicable part of SudoLang:

```sudolang
interface TaskOrchestrator {
  State {
    activeProject
    taskQueue[]
    agentPool {
      available[]
      busy[]
    }
  }

  Constraints {
    # Declarative invariants — LLM maintains these automatically
    Each task must have completion_requirements before entering "in_progress"
    Agent assignments must match agent capabilities to task requirements
    No agent may work on more than 3 concurrent tasks
    When a task is blocked, notify the orchestrator immediately
  }

  /dispatch [task] => assign to best-fit available agent
  /status => list all active tasks with agent assignments
  /escalate [task] => move to higher-tier agent with context briefing
}
```

**Key insight:** Constraints are **continuously respected** by the AI. They're not one-time checks — they're invariants that the LLM enforces across all subsequent interactions. When state changes, constraints synchronize automatically.

```sudolang
# Constraint with onChange handler
constraint SalaryFloor {
  for each employee {
    employee.salary >= $minimumSalary;
    onChange {
      emit({ constraint: 'SalaryFloor', employee, raise: constraintDifference })
    }
  }
}

# When minimumSalary changes, ALL employee salaries auto-adjust
minimumSalary = $120,000;  # joe.salary automatically becomes 120,000
```

### Modifiers

Customize function behavior inline:

```sudolang
explain(topic):length=short, detail=simple;
analyze(codebase):depth=comprehensive, format=json;
summarize(findings):length=concise, audience=technical;
```

**Application to ORCHA:** Modifiers could parameterize agent behavior per-task without changing the agent definition. An analysis agent could run `:depth=surface` for triage or `:depth=comprehensive` for full review.

### Referential Omnipotence & Inference

SudoLang leverages the LLM's world knowledge. Functions don't need explicit definitions — the AI infers behavior from context:

```sudolang
# These work without definitions because the LLM infers meaning
welcome()
sort(users, by: "last_active")
getTopicList("machine learning", n=5)
```

**Style guide:** "Favor natural language. Lean into inference. Limit code to bare minimum."

### Semantic Pattern Matching

Pattern matching can use natural language conditions:

```sudolang
match (input) {
  (post contains harmful content) => explain(content policy);
  (user asks about pricing) => show(pricing table);
  (task is overdue by > 7 days) => escalate(task, reason="overdue");
}
```

**Application to ORCHA:** Agent routing decisions could use semantic pattern matching — the orchestrator matches task characteristics to agent capabilities using natural language conditions, not rigid if/else chains.

### Program Structure (PICS Framework)

SudoLang programs follow four sections:

1. **Preamble** — Role definition, context, background (markdown is code — comments are meaningful)
2. **Interface** — State, constraints, methods, commands
3. **Components** — Supporting functions and data structures
4. **Start** — Initialization sequence

```sudolang
# Preamble
You are TaskRouter, an intelligent orchestrator that dispatches
development tasks to specialized AI agents based on task type,
complexity, and agent availability.

# Interface
interface TaskRouter {
  State { ... }
  Constraints { ... }
  /route [task] => ...
  /rebalance => ...
}

# Components
function assessComplexity(task) => ...
function matchAgent(requirements) => ...

# Start
Welcome. Show current queue status and await commands.
```

---

## AIDD Framework Architecture

### Project Structure

```
project-root/
├── ai/
│   ├── skills/              # Reusable agent behaviors
│   │   └── aidd-please/    # Main orchestrator skill
│   ├── commands/            # Workflow templates
│   ├── rules/               # Development guidelines
│   └── system/              # Core prompts
├── plan/
│   ├── story-map/           # User journey mappings from /discover
│   ├── epics/               # Task definitions from /task
│   └── testing-scripts/     # Test scenarios from /user-test
├── vision.md                # Source of truth for all agents
├── AGENTS.md                # Agent configuration & roles
└── aidd-custom/             # Project-specific overrides
```

### Vision Document (vision.md)

The single source of truth containing:
- Project identity, purpose, competitive positioning
- User personas with motivations and pain points
- Core features with user benefit statements
- Technical architecture decisions and rationale
- Design principles
- Success metrics (measurable)
- Constraints (business, technical, scope)

**Key insight:** The vision document prevents agent drift. Before creating or running any task, agents **must read vision.md first**. When detecting contradictions between a requested task and the vision, agents must identify the conflict and request clarification before proceeding.

**Application to ORCHA:** Our agent orchestration should have an equivalent "source of truth" document that all agents reference before starting work. This prevents agents from optimizing locally while violating project-level constraints.

### Workflow Commands

| Command | Phase | Output | Location |
|---------|-------|--------|----------|
| `/discover` | Research | User journey maps, personas, feature priorities | `plan/story-map/` |
| `/task` | Planning | Structured epics with requirements + acceptance criteria | `plan/epics/` |
| `/execute` | Implementation | Code with passing tests (TDD: Red→Green→Refactor) | Source tree |
| `/review` | Quality | Duplication elimination, coupling analysis, simplifications | Code review |
| `/log` | Documentation | Change records | `activity-log.md` |
| `/commit` | Version control | Staged + committed changes | Git |
| `/user-test` | Validation | Human scripts + AI agent test scripts | `plan/testing-scripts/` |

**Standard workflow:**
```
branch → /discover → /task → /review → /execute → push → /user-test
```

**Application to ORCHA:** This structured workflow — discover what to build, plan it as a structured epic, review the plan, execute with TDD, then validate — maps directly to our agent task lifecycle. Each phase could be a distinct agent mode/role.

### Skills System

Skills are reusable, composable agent behaviors:

```
ai/skills/skill-name/
├── SKILL.md                # Documentation + metadata
├── prompt.sudolang         # SudoLang specification
├── system-instructions.md  # Context guidelines
├── examples/               # Usage examples
└── tests/                  # Validation scenarios
```

Skills have frontmatter metadata enabling auto-generated index files. Agents use **progressive discovery** — they only read the root index until they need subfolder contents, minimizing token consumption.

**Application to ORCHA:** Agent capabilities could be defined as composable skills with SudoLang-like specifications. Skills have explicit interfaces, constraints, and examples — making agent behavior inspectable and testable.

### AGENTS.md Configuration

Specifies how AI agents should operate:
- Agent roles and responsibilities
- Model selection per role
- Custom instructions and constraints
- Integration points for external tools
- Persona-based behavior definitions

### TDD Integration

AIDD enforces Test-Driven Development:
1. **Red:** Write failing tests from task requirements
2. **Green:** Implement minimum code to pass
3. **Refactor:** Improve quality while maintaining tests
4. Maintain **100% test coverage minimum**

This counters documented AI code quality issues: GitClear tracked 8x more code duplication with AI adoption; DORA reports show 9% higher bug rates. TDD prevents agents from generating code that runs but fails at scale.

### Dual User Testing

`/user-test` generates two complementary formats:
- **Human scripts:** Think-aloud protocols with video recording guidance
- **AI agent scripts:** Executable tests with persona-based behavior assertions

Research: testing with 3-5 users reveals 65-85% of usability problems.

---

## Core Design Principles

### 1. Constraints Over Instructions

SudoLang's most powerful idea: **declare what must be true, not what to do**. Constraints are maintained across all interactions, automatically synchronizing state when conditions change. This is fundamentally different from step-by-step instructions that the LLM may forget mid-conversation.

**Application to ORCHA:** Agent orchestration rules should be expressed as constraints ("No agent may work on more than 3 concurrent tasks") rather than procedural logic ("Before assigning a task, check if the agent has fewer than 3 tasks"). Constraints survive context window limits; procedures get lost.

### 2. Interface-Oriented Composition

SudoLang favors `interface` over `class`, composition over inheritance. Interfaces define state + constraints + methods + commands in a single cohesive unit. This is the PICS pattern: Preamble, Interface, Components, Start.

**Application to ORCHA:** Agent definitions should be interface-like: state they manage, constraints they enforce, commands they accept, and components they compose. Not monolithic instruction sets.

### 3. Vision-First Alignment

All agents must read `vision.md` before starting work. Contradictions between task and vision require explicit clarification. The vision document prevents drift toward speculative features or locally-optimal-but-globally-wrong decisions.

**Application to ORCHA:** Our orchestration system needs an authoritative "project vision" document that all agents reference. This is stronger than just CLAUDE.md — it's a formal constraint anchor.

### 4. Progressive Discovery (Token Efficiency)

Agents consume only what they need. Root index first, subfolder contents only when relevant. This minimizes context window consumption for large projects.

**Application to ORCHA:** Agent context loading should be lazy — load project overview first, drill into specifics only when the task requires it. Don't front-load the full project state.

### 5. Structured Workflows Prevent Quality Decay

AIDD's /discover → /task → /review → /execute → /user-test pipeline enforces quality at every stage. Without this structure, AI agents generate code that compiles but accumulates technical debt.

**Application to ORCHA:** Our agent task pipeline should have explicit quality gates between phases (planning → implementation → review → testing), not just status transitions.

### 6. Modifiers as Behavioral Parameters

SudoLang's modifier system (`:length=short, detail=high`) lets you parameterize behavior without redefining the function. The same agent can operate at different depths/speeds.

**Application to ORCHA:** Agent dispatching should support behavioral modifiers — same agent, different operating parameters per task (e.g., analysis depth, output format, verbosity level).

### 7. Semantic Pattern Matching for Routing

Instead of rigid if/else routing, SudoLang uses natural language pattern matching. The LLM infers the best match from context.

**Application to ORCHA:** Task routing to agents could use semantic matching ("task requires deep code analysis" → code-review agent) rather than keyword-based or type-based routing.

---

## Comparison: SudoLang/AIDD vs ORCHA Patterns

| Dimension | SudoLang/AIDD | ORCHA (Current) | Gap/Opportunity |
|-----------|---------------|-----------------|-----------------|
| **Agent specification** | SudoLang interfaces with state + constraints + commands | Natural language prompts | Formalize agent specs as interface-like definitions |
| **Constraints** | First-class, continuously enforced, auto-synchronizing | Ad-hoc rules in prompts | Promote constraints to structured, persistent declarations |
| **Workflow phases** | /discover → /task → /execute → /review → /user-test | Task status transitions | Add explicit quality gates between phases |
| **Vision alignment** | vision.md is mandatory first-read for all agents | CLAUDE.md per project | Formalize a "project vision" constraint anchor |
| **Agent skills** | Composable skills with SKILL.md + prompt.sudolang + tests | Executor pattern (pluggable but not skill-based) | Define agent capabilities as testable, composable skills |
| **Token efficiency** | Progressive discovery, 20-30% fewer tokens | Full context loading | Lazy context loading, lean agent specifications |
| **Task planning** | Structured epics with acceptance criteria + TDD specs | Freeform task descriptions | Add structured requirements + completion criteria |
| **Quality enforcement** | TDD mandatory, 100% coverage, automated review | Optional testing | Enforce test generation as part of agent task lifecycle |
| **Behavioral parameters** | Modifiers (`:length=short, depth=comprehensive`) | Fixed agent behavior | Support per-task behavioral modifiers |
| **Routing** | Semantic pattern matching | Type-based or manual assignment | Add semantic matching for agent-task routing |
| **Source of truth** | vision.md prevents agent drift | No formal vision anchor | Create authoritative project vision document |

---

## Actionable Recommendations for ORCHA

### High Priority

1. **Constraint-Based Agent Specifications** — Define agent behavior as interfaces with explicit state, constraints, commands, and components (the PICS pattern). Constraints should be structured declarations that survive context limits, not buried in freeform prompts. Example: "This agent must never modify files outside its assigned worktree" is a constraint, not an instruction.

2. **Vision/Mission Anchor Document** — Create a formal project vision document that all agents must reference before starting work. Include: project goals, user personas, architectural decisions, design principles, success metrics, and scope constraints. Agents that detect contradictions between their task and the vision must escalate, not proceed.

3. **Structured Task Epics with Acceptance Criteria** — When decomposing work, generate structured epics that include: requirements with acceptance criteria, completion criteria, TDD test specifications, and dependency mapping. This is AIDD's `/task` output format — every task has explicit "done" criteria before work begins.

### Medium Priority

4. **Phased Workflow with Quality Gates** — Structure agent task lifecycle as explicit phases (discover → plan → execute → review → test) with quality gates between each. Don't let agents skip from "assigned" to "done" without review and testing phases. AIDD's six-command pipeline enforces this.

5. **Agent Skills as Composable Units** — Define agent capabilities as composable skills with: documentation (SKILL.md), specification (SudoLang-like), examples, and tests. Skills can be mixed and matched per task. This makes agent behavior inspectable, testable, and reusable across different orchestration flows.

6. **Behavioral Modifiers for Agent Dispatch** — When dispatching tasks, support per-task behavioral parameters (depth, format, verbosity, scope) that customize agent behavior without changing the agent definition. Same analysis agent can do a surface triage or deep audit based on modifiers.

7. **Progressive Context Loading** — Agents should load context lazily: project overview first, then drill into specifics only when the task requires it. Don't front-load entire project state. This reduces token consumption and improves focus.

### Lower Priority

8. **Semantic Pattern Matching for Task Routing** — Replace rigid type-based or keyword-based agent routing with semantic matching. The orchestrator describes task characteristics in natural language, and the routing system matches to the best-fit agent based on capability semantics.

9. **Dual Testing Output (Human + AI)** — When agents complete implementation tasks, generate both human-readable test scripts (think-aloud protocols) and AI-executable test scripts. This enables both manual QA and automated regression.

10. **Constraint onChange Events** — When agent constraints are violated or state changes trigger constraint re-evaluation, emit structured events that the orchestrator can observe and react to. This enables reactive orchestration based on constraint state.

---

## Key Takeaways

1. **Constraints > instructions** — Declarative constraints that the LLM maintains across all interactions are more reliable than step-by-step instructions that degrade over long conversations. Express rules as invariants, not procedures.

2. **Interface-oriented agent design** — Agents should be defined as interfaces (state + constraints + commands + components), not monolithic instruction blobs. This is inspectable, composable, and testable.

3. **Vision-first prevents drift** — A formal vision document that all agents must reference before starting work prevents locally-optimal-but-globally-wrong decisions. Contradictions require escalation.

4. **Structured workflows enforce quality** — Without explicit phases and gates (discover → plan → execute → review → test), AI agents accumulate technical debt. The pipeline IS the quality assurance.

5. **Modifiers parameterize behavior** — Same agent, different operating parameters per task. Don't create separate agents for "quick analysis" and "deep analysis" — use behavioral modifiers.

6. **Progressive discovery saves tokens** — Load context lazily. Root index first, details on demand. 20-30% token savings translate directly to cost reduction and faster responses.

7. **Semantic matching > rigid routing** — Natural language pattern matching for task-to-agent routing is more flexible than type-based or keyword-based systems.

---

## Sources

- [SudoLang GitHub Repository](https://github.com/paralleldrive/sudolang)
- [SudoLang Language Specification (v2.0)](https://github.com/paralleldrive/sudolang-llm-support/blob/main/sudolang.sudo.md)
- [AIDD Framework GitHub](https://github.com/paralleldrive/aidd)
- [AIDD AGENTS.md](https://github.com/paralleldrive/aidd/blob/main/AGENTS.md)
- [O'Reilly: Unlocking AI-Driven Development with SudoLang](https://www.oreilly.com/radar/unlocking-the-power-of-ai-driven-development-with-sudolang/)
- [SudoLang: A Powerful Pseudocode Programming Language for LLMs — Eric Elliott](https://medium.com/javascript-scene/sudolang-a-powerful-pseudocode-programming-language-for-llms-d64d42aa719b)
- [Anatomy of a SudoLang Program — Eric Elliott](https://medium.com/javascript-scene/anatomy-of-a-sudolang-program-prompt-engineering-by-example-f7a7b65263bc)
- [The Art of Effortless Programming — Eric Elliott](https://medium.com/javascript-scene/the-art-of-effortless-programming-3e1860abe1d3)
- [SudoLang on Hacker News](https://news.ycombinator.com/item?id=37791060)
