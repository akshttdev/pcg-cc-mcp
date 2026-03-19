# RooRoo Agent Orchestration Framework — Deep Research

**Date:** 2026-03-18
**Type:** Research / Reference
**Purpose:** Extract principles, tools, and techniques from the RooRoo framework applicable to our ORCHA agent orchestration system and task management processes.

---

## What is RooRoo?

RooRoo (如如, "rú rú" — Buddhist concept of "Thusness") is a **minimalist AI orchestration framework** built on top of Roo Code's custom modes system. Created by [marv1nnnnn on GitHub](https://github.com/marv1nnnnn/rooroo), it turns Roo Code's VS Code extension into a structured multi-agent team with clear roles, communication protocols, and task management.

The name reflects its design philosophy: focus on the essential, specialized nature of each agent — minimalism over complexity.

---

## Architecture Overview

### Agent Hierarchy & Cost Tiers

RooRoo assigns agents to **LLM cost tiers** based on cognitive demand:

| Agent | Role | Recommended Tier |
|-------|------|-----------------|
| **Navigator** | Central orchestrator — triage, dispatch, logging, user interaction | Cheap/Fast |
| **Planner** | Decomposes goals into sub-tasks, creates context.md briefings | Smart/Expensive |
| **Developer** | Executes coding tasks per context.md | Custom (varies) |
| **Analyzer** | Performs analysis, generates structured reports | Cheap/Fast |
| **Documenter** | Creates/updates documentation | Cheap/Fast |
| **Idea Sparker** | Strategic brainstorming facilitator, produces handoff documents | Smart/Expensive |

**Key insight:** Only planning/creative work needs expensive models. Mechanical orchestration, analysis, and documentation can use cheaper/faster models. This is directly applicable to our multi-model routing strategy.

### Workflow Phases

```
Phase 0 (Optional): User brainstorms with Idea Sparker
Phase 1: Navigator receives goal → triages:
         - Complex/uncertain → Planner breaks into sub-tasks
         - Simple/clear → Direct execution or queuing
         - Ambiguous → Navigator requests clarification
Phase 2: Navigator dispatches queued tasks to appropriate expert via context.md
Phase 3: Expert executes, returns JSON Output Envelope (status, message, artifacts)
Phase 4: Navigator processes envelope, logs events, updates queue, prompts next steps
```

---

## Task Management System

### Task ID Scheme

RooRoo uses prefixed IDs for traceability:

- `ROO#PLAN_[timestamp]_[description]` — Planner-initiated planning tasks
- `ROO#SUB_[parent]_[sequence]` — Sub-tasks created by Planner
- `ROO#TASK_[timestamp]_[description]` — Direct Navigator-initiated tasks
- `ROO#IDEA_[timestamp]_[description]` — Brainstorming sessions

**Applicable principle:** Typed, prefixed task IDs enable instant recognition of task origin, type, and hierarchy without needing to look up metadata.

### Directory Structure

```
.rooroo/
├── queue.jsonl              # Pending tasks (append-friendly JSONL)
├── logs/activity.jsonl      # Event log with severity levels
├── rules/                   # Workspace custom instructions
├── tasks/
│   ├── ROO#PLAN_*.../context.md
│   ├── ROO#SUB_*.../
│   │   ├── context.md
│   │   └── [artifacts]
│   └── ROO#TASK_*.../
│       ├── context.md
│       └── [outputs]
├── plans/                   # Planner overview documents
└── brainstorming/           # Idea Sparker outputs
```

### v0.4.0 State Management Evolution

RooRoo v0.4.0 made a critical architectural shift: **from monolithic JSON to JSONL**.

**Before:** Single `project_overview.json` housed task definitions, statuses, and configs — required complex `apply_diff`-like operations, prone to corruption.

**After:**
- `task_queue.jsonl` — One JSON object per line per task. Simple append/remove operations.
- `task_log.jsonl` — Append-only audit trail for events.

**Why this matters:** JSONL eliminates state mutation brittleness. No complex document rewrites. Reduces corruption risk during concurrent or interrupted operations. Simpler file operations = more robust agent interactions.

---

## Communication Protocol

### Context.md Briefings

Each task gets a `context.md` file serving as the comprehensive briefing for the assigned agent:
- Detailed goals and requirements
- **Links prioritized over embedded content** ("Link, Don't Embed")
- Workspace-relative file paths
- Expert-specific execution guidance

**Principle: "Concise Context File Preparation"** — Keep context files lean by linking to existing docs/code rather than duplicating content. This reduces token usage and keeps briefings focused.

### Output Envelope (JSON)

Experts return structured responses:

```json
{
  "status": "Done|NeedsClarification|Failed",
  "message": "Description of outcome",
  "artifacts": ["list", "of", "generated", "files"],
  "task_metadata": { ... }
}
```

The Navigator parses this envelope:
- `NeedsClarification` → Relay questions to user
- `Done` → Log events, update queue, proceed
- `Failed` → Log failure, escalate or retry

**Applicable principle:** Structured inter-agent communication with explicit status codes eliminates ambiguity. The orchestrator doesn't need to "interpret" freeform text from sub-agents.

### Activity Logging

```jsonl
{"timestamp": "...", "agent_slug": "...", "severity": "INFO|ERROR", "task_id": "ROO#...", "event_type": "EXPERT_REPORT", "details": {...}}
```

Append-only JSONL log enables debugging, recovery, and audit trails.

---

## Core Design Principles

### 1. Principle of Least Assumption
Agents **clarify ambiguities rather than guessing**. When an agent encounters uncertainty, it returns `NeedsClarification` rather than making assumptions that could cascade into larger errors.

**Application to ORCHA:** Our agents should have explicit "I don't know" / "I need clarification" pathways rather than hallucinating solutions.

### 2. Navigator-Led Orchestration
One central agent (Navigator) owns the workflow. It doesn't execute work — it triages, dispatches, and processes results. This separation keeps the orchestrator lightweight and focused.

**Application to ORCHA:** Mirrors our existing pattern but RooRoo takes it further by making the Navigator deliberately cheap/fast — it's a router, not a thinker.

### 3. Minimalism & Specialization
Each agent has focused, non-overlapping responsibilities. The Orchestrator in Roo Code's Boomerang Tasks system intentionally **lacks direct file/command capabilities** to maintain strategic focus.

**Application to ORCHA:** Consider whether our orchestrating agent should be stripped of direct execution tools to prevent it from "getting in the weeds."

### 4. Cost-Tiered Model Routing
Not all agent work requires the same intelligence level. Routing mechanical tasks to cheaper models and reserving expensive models for planning/creative work reduces costs significantly.

**Application to ORCHA:** We already have multi-model support. RooRoo's tier assignments provide a concrete template for which tasks need which tier.

### 5. Link, Don't Embed
Context files link to source material rather than embedding it. This keeps token counts manageable and ensures agents reference the authoritative source.

**Application to ORCHA:** Our workflow context/briefings should reference file paths and URLs rather than inlining entire documents.

### 6. Structured Communication Over Freeform
JSON Output Envelopes with explicit status codes beat freeform text responses. The orchestrator can mechanically parse results rather than interpreting natural language.

**Application to ORCHA:** Our agent communication protocol should enforce structured responses with typed status fields.

---

## Roo Code Foundation: Boomerang Tasks

RooRoo builds on Roo Code's **Boomerang Tasks** pattern (built-in Orchestrator Mode):

- **Parent-child task delegation** — Orchestrator spawns subtasks via `new_task` tool
- **Isolation model** — Each subtask has its own conversation history, no automatic context inheritance
- **Information flows downward** through instructions, **upward** through completion summaries only
- **The orchestrator receives only the summary**, not the full execution trace — keeps parent context clean

**Key insight:** Context isolation between parent and child tasks prevents context window pollution. The parent doesn't need to know *how* the work was done, just *what was accomplished*.

---

## Comparison: RooRoo vs ORCHA Patterns

| Dimension | RooRoo | ORCHA (Current) | Gap/Opportunity |
|-----------|--------|-----------------|-----------------|
| **Orchestration** | Navigator (cheap model) routes work | Single agent handles orchestration + execution | Consider separating orchestration from execution |
| **Task decomposition** | Planner agent with dedicated context.md briefings | Planning files in `planning/` dir | Formalize per-task briefing documents |
| **Inter-agent comms** | Structured JSON Output Envelopes | Freeform conversation | Add structured status/artifact responses |
| **State management** | JSONL queues (append-only, corruption-resistant) | SQLite + in-memory | JSONL pattern useful for agent-specific state |
| **Cost optimization** | Explicit tier assignments per agent role | Multi-model but not role-tiered | Map agent roles to model tiers |
| **Task IDs** | Typed prefixes (PLAN, SUB, TASK, IDEA) | UUID-based | Consider adding semantic prefixes |
| **Context isolation** | Subtasks get own conversation, parent gets summary only | Shared context | Enforce context boundaries between orchestrated tasks |
| **Clarification protocol** | Explicit NeedsClarification status | Agent decides ad hoc | Formalize "I need help" as a first-class response type |
| **Audit trail** | Append-only JSONL activity log | Execution artifacts in DB | Consider lightweight event log alongside artifacts |

---

## Actionable Recommendations for ORCHA

### High Priority

1. **Structured Agent Response Protocol** — Define a standard response envelope (status, message, artifacts, clarification_needed) for all agent-to-orchestrator communication. Eliminates ambiguity in result parsing.

2. **Cost-Tiered Model Routing** — Map each agent role to a model tier:
   - Orchestration/routing → fast/cheap (Haiku-class)
   - Analysis/documentation → mid-tier (Sonnet-class)
   - Planning/creative/complex coding → expensive (Opus-class)

3. **Context Isolation Between Sub-tasks** — When an orchestrator dispatches work, the sub-agent should operate in isolation and return only a summary. Prevents context window bloat in the parent.

### Medium Priority

4. **Formalize Clarification as First-Class Status** — Agents should have an explicit "NeedsClarification" response type rather than ad-hoc questions embedded in freeform text.

5. **Task Briefing Documents** — For each dispatched task, generate a lean `context.md` that links to relevant files/docs rather than embedding content. This is the "contract" between orchestrator and executor.

6. **Append-Only Event Logging** — Add a lightweight JSONL event log for agent orchestration events (task dispatched, completed, failed, clarification requested). Useful for debugging and replay.

### Lower Priority

7. **Typed Task ID Prefixes** — Consider prefixing task IDs with their type (PLAN, EXEC, REVIEW, etc.) for instant human readability.

8. **Orchestrator Tool Restriction** — Evaluate whether the orchestrating agent should be stripped of direct execution tools (file edit, command run) to keep it focused on coordination.

9. **Brainstorming/Ideation Agent** — RooRoo's Idea Sparker pattern (structured exploration → handoff document) could be useful for our workflow definition process.

---

## Key Takeaways

1. **Minimalism wins** — RooRoo succeeds by doing less per agent, not more. Each agent has a sharp, narrow role.
2. **Cheap orchestration** — The coordinator doesn't need to be smart, just reliable. Route expensive thinking to where it matters.
3. **Structured > freeform** — JSON envelopes, typed IDs, and explicit status codes beat natural language for inter-agent communication.
4. **JSONL > monolithic JSON** — Append-only formats are more robust for agent state management than complex document mutations.
5. **Link, don't embed** — Keep context lean by referencing rather than duplicating.
6. **Clarify, don't assume** — First-class support for "I need help" prevents cascading errors from bad assumptions.

---

## Appendix A: Roo Code Foundation (Underlying Platform)

RooRoo is built on Roo Code, which provides the runtime substrate. Understanding Roo Code's internals reveals additional patterns applicable to ORCHA.

### Agent Loop Architecture

Roo Code's core execution is `recursivelyMakeClineRequests()`:

1. **Build API request** — assembles system prompt + full message history
2. **Stream response** from configured LLM provider
3. **Parse tool calls** via `NativeToolCallParser`
4. **Approval workflow** — read ops auto-approve; write/command ops require user consent
5. **Execute tool** — file operations, commands, or MCP tools (with file path validation against mode restrictions)
6. **Format result** → add to history → **loop** until `attempt_completion` is called

**Key detail:** The system prompt is generated once per task, NOT regenerated between turns. This is a performance optimization but means mode-specific instructions are static within a task.

### System Prompt Construction Pipeline

Built in order by `generatePrompt()`:

1. Role definition (from active mode's `roleDefinition`)
2. Markdown formatting guidelines
3. Shared tool use instructions
4. Tool-specific guidelines (only for tools available in current mode)
5. Capabilities documentation
6. Available modes enumeration
7. Loaded skills catalog
8. Operational rules
9. System information (OS, shell, workspace path)
10. Task objective
11. Custom user instructions (appended last)

**Application to ORCHA:** This layered prompt assembly pattern — where each layer is conditionally included based on active mode/context — is a clean architecture for our agent system prompts.

### Tool System (7 Groups)

| Group | Tools |
|-------|-------|
| **Read** | `read_file`, `list_files`, `read_command_output` |
| **Search** | `search_files`, `codebase_search` |
| **Edit** | `apply_diff`, `apply_patch`, `edit`, `write_to_file` |
| **Image** | `generate_image` |
| **Command** | `execute_command`, `run_slash_command` |
| **MCP** | `use_mcp_tool`, `access_mcp_resource` |
| **Workflow** | `switch_mode`, `new_task`, `ask_followup_question`, `attempt_completion`, `update_todo_list` |

Four tools are **always available** regardless of mode: `ask_followup_question`, `attempt_completion`, `switch_mode`, `new_task`.

**Mode-gated tool access:** If a tool attempts to write a file outside a mode's allowed `fileRegex` patterns, a `FileRestrictionError` is thrown. This is how Architect mode can only edit `.md` files.

**Application to ORCHA:** Tool gating per agent role prevents scope creep. An analysis agent shouldn't be able to edit code; a documentation agent shouldn't run commands.

### Context Window Management

1. **Token Counting** — Each API handler implements `countTokens()`, using native API endpoints when available, tiktoken as fallback
2. **Context Window Reservation** — 30% reserved (20% output, 10% safety buffer), leaving 70% for history
3. **Intelligent Context Condensing** — When approaching threshold, AI summarizes earlier conversation segments. **Non-destructive:** original messages preserved internally; rewinding to checkpoint restores full history
4. **Auto Error Recovery** — Detects context window errors, reduces context by 25%, retries

**Application to ORCHA:** Our long-running agent conversations need similar condensation. The non-destructive approach (summarize but preserve originals for replay) is particularly valuable for debugging.

### Custom Modes Configuration

```yaml
customModes:
  - slug: docs-writer
    name: "Documentation Writer"
    roleDefinition: >-
      You are a technical writer with expertise in
      clear, comprehensive documentation practices.
    whenToUse: "Use for writing and editing documentation"
    customInstructions: |-
      Focus on clarity and completeness.
    groups:
      - read
      - - edit
        - fileRegex: \.(md|mdx)$
          description: "Markdown files only"
      - command
      - mcp
```

**Key patterns:**
- `whenToUse` guides the Orchestrator's mode selection decisions
- Tool groups can be unrestricted (string) or restricted (array with fileRegex)
- Project-level `.roomodes` overrides global config
- File-based rules per mode: `.roo/rules-{slug}/` directory

### MCP Integration

Roo Code is a first-class MCP client supporting STDIO, Streamable HTTP, and SSE transports. MCP tools are conditionally included in the system prompt only when the current mode's tool groups include `"mcp"`.

**Application to ORCHA:** Our MCP server integration should be mode/role-aware — not every agent needs access to every MCP tool.

### Sticky Models Per Mode

Each mode remembers the last LLM model used. Different models can be assigned per mode (e.g., o3 for Architect, Claude Sonnet for Code). This persists across sessions.

**Application to ORCHA:** Model assignment should be a property of the agent role, not a global setting.

---

## Sources

- [RooRoo GitHub Repository](https://github.com/marv1nnnnn/rooroo)
- [RooRoo v0.4.0 Changelog](https://github.com/marv1nnnnn/rooroo/blob/main/v0.4.0.md)
- [Roo Code Documentation](https://docs.roocode.com/)
- [Boomerang Tasks Documentation](https://docs.roocode.com/features/boomerang-tasks)
- [Roo Code Modes Documentation](https://docs.roocode.com/basic-usage/using-modes)
- [Roo Code Custom Modes](https://docs.roocode.com/features/custom-modes)
- [Roo Code Context Condensing](https://docs.roocode.com/features/intelligent-context-condensing)
- [Roo Code Tool Use Overview](https://docs.roocode.com/advanced-usage/available-tools/tool-use-overview)
- [Roo Code vs Cline Comparison (Qodo)](https://www.qodo.ai/blog/roo-code-vs-cline/)
- [Roo Code GitHub Repository](https://github.com/RooCodeInc/Roo-Code)
- [DeepWiki: Roo Code Architecture](https://deepwiki.com/RooCodeInc/Roo-Code)
