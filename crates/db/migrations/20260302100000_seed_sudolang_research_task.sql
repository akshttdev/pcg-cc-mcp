-- Seed SudoLang Research task on PCG Development Board
-- Created by Fraze, assigned to Madhav + Admin for review
-- Includes research artifacts for shared knowledge base
-- All inserts use INSERT OR IGNORE with deterministic IDs — safe to re-run.

-- ============================================================================
-- 1. USERS — ensure Fraze and Madhav exist
-- ============================================================================

INSERT OR IGNORE INTO users (id, username, full_name, email, password_hash, is_active, is_admin, created_at, updated_at)
VALUES (
    X'F4A2E00000004000800000000000F4A2',
    'fraze',
    'Fraze',
    'fraze@powerclub.global',
    '$2b$12$placeholder.hash.for.seed.data.only',
    1,
    0,
    '2026-03-02T12:00:00.000',
    '2026-03-02T12:00:00.000'
);

INSERT OR IGNORE INTO users (id, username, full_name, email, password_hash, is_active, is_admin, created_at, updated_at)
VALUES (
    X'3AD4AE00000040008000000000003AD4',
    'madhav',
    'Madhav',
    'madhav@powerclub.global',
    '$2b$12$placeholder.hash.for.seed.data.only',
    1,
    0,
    '2026-03-02T12:00:00.000',
    '2026-03-02T12:00:00.000'
);

-- ============================================================================
-- 2. PCG DEVELOPMENT BOARD on sirak-studios project
-- ============================================================================

INSERT OR IGNORE INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
VALUES (
    X'9C6DE800000040008000000000000001',
    X'B0B1B2B3B4B5B6B7B8B9BABBBCBDBEBF',  -- sirak-studios
    'PCG Development',
    'pcg-development',
    'custom',
    'Platform development tasks, research spikes, and integration planning for the PCG system',
    datetime('now', 'subsec'),
    datetime('now', 'subsec')
);

-- ============================================================================
-- 3. TASK — SudoLang Research & Integration Planning
-- ============================================================================

INSERT OR IGNORE INTO tasks (
    id, project_id, board_id, title, description, status,
    priority, assignee_id, created_by, created_by_user_id,
    requires_approval, tags, collaborators,
    created_at, updated_at
) VALUES (
    X'2EC0B1FA000040008000000000000020',
    X'B0B1B2B3B4B5B6B7B8B9BABBBCBDBEBF',  -- sirak-studios
    X'9C6DE800000040008000000000000001',    -- PCG Development board
    'SudoLang Research & Integration Planning for PCG Agentic Workflows',
    'Review the SudoLang pseudocode language and AIDD (AI-Driven Development) framework for integration into PCG''s agentic workflows (Topsi, NORA, Editron).

## Objective
Establish shared understanding of SudoLang across the team so we can evaluate its use in our agent orchestration, prompt engineering, and workflow definition layers.

## Background
SudoLang is a pseudocode language designed for LLM interaction that combines natural language with structured programming constructs (interfaces, constraints, pipe operators, pattern matching). The AIDD framework builds on SudoLang to provide a complete AI-Driven Development methodology with workflow commands (/discover, /task, /execute, /review).

Key advantages identified:
- 20-30% fewer tokens than natural language prompts
- Improved reasoning performance vs prose prompts
- Constraint-based programming aligns with our agent behavior definitions
- Interface/composition patterns map to our artifact type system

## Action Items
1. **Review** the attached research artifacts (SudoLang spec, AIDD framework analysis, integration strategy)
2. **Evaluate** applicability to Topsi/NORA agent prompt structures
3. **Identify** 2-3 concrete pilot use cases for our next sprint
4. **Discuss** findings in next team sync

## References
- SudoLang spec: https://github.com/paralleldrive/sudolang-llm-support
- AIDD framework: https://github.com/paralleldrive/aidd
- Contributed by Fraze for team review',
    'inreview',
    'high',
    'madhav',           -- primary assignee
    'fraze',            -- created by
    X'F4A2E00000004000800000000000F4A2',  -- fraze user id
    1,                  -- requires approval (review gate)
    '["sudolang","aidd","research","agentic-workflows","prompt-engineering"]',
    '[{"user_id":"madhav","role":"reviewer","added_at":"2026-03-02T12:00:00Z"},{"user_id":"admin","role":"reviewer","added_at":"2026-03-02T12:00:00Z"}]',
    '2026-03-02T12:00:00.000',
    '2026-03-02T12:00:00.000'
);

-- ============================================================================
-- 4. EXECUTION ARTIFACTS — SudoLang Research
-- ============================================================================

-- 4a. Research Report: SudoLang Language Specification & Core Concepts
INSERT OR IGNORE INTO execution_artifacts (
    id, execution_process_id, artifact_type, title, content, metadata, phase, review_status
) VALUES (
    X'2EC0B1FA000040008000000000000021',
    NULL,  -- standalone research, no execution process
    'research_report',
    'SudoLang v2.0 — Language Specification & Core Concepts',
    '{
  "summary": "Comprehensive analysis of SudoLang v2.0 pseudocode language for LLM interaction",
  "source": "https://github.com/paralleldrive/sudolang-llm-support",
  "language_overview": {
    "definition": "A pseudolanguage designed for interacting with LLMs that combines natural language with programming constructs. All sufficiently advanced language models understand it without special prompting.",
    "file_extensions": [".sudo", ".sudo.md", ".mdc"],
    "token_efficiency": "20-30% fewer tokens than equivalent natural language prompts"
  },
  "core_features": {
    "variables_and_assignment": {
      "operators": ["=", "+=", "-=", "*=", "/="],
      "template_strings": "$variable interpolation, \\\\$ to escape"
    },
    "control_flow": {
      "conditionals": "if/else expressions that evaluate to assignable values",
      "loops": ["for each item, action", "while (condition) { action }", "loop { action }"],
      "pattern_matching": "match (value) { case condition => result; default => fallback }"
    },
    "operators": {
      "logical": ["&& (AND)", "|| (OR)", "xor", "! (NOT)"],
      "math": ["+", "-", "*", "/", "^ (exponent)", "% (remainder)"],
      "set": ["union", "intersection"],
      "pipe": "|> chains function outputs as first arguments to next function",
      "range": ".. creates inclusive number ranges (e.g., 1..3)"
    },
    "functions": {
      "inferred": "fn name; — AI infers implementation (referential omnipotence)",
      "full_definition": "fn name() { constraints }",
      "lambda": "x => x + 1",
      "composition": "Via pipe operator: input |> transform |> output"
    },
    "interfaces": {
      "purpose": "Define structure, behavior, and types for organizing data and logic",
      "properties": "Support composition, reusability, and modular design",
      "type_inference": "Types can be explicitly stated or inferred by AI"
    },
    "constraints": {
      "purpose": "Declarative rules that guide AI behavior — describe WHAT should happen, not HOW",
      "behavior": "Continuously respected by AI, synchronize state automatically",
      "syntax": "constraint [name] { rules } or natural language descriptions"
    },
    "commands": {
      "syntax": "/command | shortcut - description",
      "builtins": ["ask", "explain", "run", "log", "transpile", "convert", "list", "emit"]
    },
    "semantic_pattern_matching": "AI can infer program states and match patterns like: (post contains harmful content) => explain(content policy)",
    "markdown_integration": "Full markdown support — comments are meaningful, documentation is executable"
  },
  "design_principles": [
    "Favor natural language and inference over explicit code",
    "Minimize code to essential flow control",
    "Prioritize readability and clarity",
    "Use composition and factories instead of constructors/inheritance",
    "Constraint-based over imperative programming"
  ],
  "pcg_relevance": {
    "agent_prompts": "SudoLang interfaces could formalize Topsi/NORA agent behavior definitions",
    "constraint_system": "Maps directly to our agent autonomy_level and approval_required patterns",
    "pipe_operator": "Natural fit for our multi-stage pipeline architecture (ingest |> analyze |> edit |> render)",
    "commands": "Could standardize our /slash-command patterns across agents",
    "pattern_matching": "Useful for agent routing and task classification logic"
  }
}',
    '{"source_url":"https://github.com/paralleldrive/sudolang-llm-support","research_type":"language_specification","contributed_by":"fraze"}',
    'planning',
    'pending'
);

-- 4b. Research Report: AIDD Framework Analysis
INSERT OR IGNORE INTO execution_artifacts (
    id, execution_process_id, artifact_type, title, content, metadata, phase, review_status
) VALUES (
    X'2EC0B1FA000040008000000000000022',
    NULL,
    'research_report',
    'AIDD Framework — AI-Driven Development Methodology Analysis',
    '{
  "summary": "Analysis of the AIDD (AI-Driven Development) framework built on SudoLang for automated software development workflows",
  "source": "https://github.com/paralleldrive/aidd",
  "framework_overview": {
    "definition": "A methodology where AI systems take primary responsibility for generating, testing, and documenting code",
    "core_problem": "GitClear analysis of 211M lines (2020-2024) found 8x more code duplication with AI adoption. Google DORA report shows AI correlates with 9% higher bug rates.",
    "solution": "Specification-driven development + systematic TDD + automated code review"
  },
  "workflow_commands": {
    "/discover": "Product discovery — identify user journeys, create story maps (saved to plan/story-map/)",
    "/task": "Epic planning from user stories — structured task breakdowns",
    "/execute": "TDD-based implementation — one requirement at a time",
    "/review": "Code review and duplication elimination",
    "/log": "Activity log documentation",
    "/commit": "Version control operations",
    "/user-test": "Generate dual testing scripts (human think-aloud + AI agent executable tests)",
    "/run-test": "Execute AI agent test scripts"
  },
  "agent_runtime": {
    "persona": "Aiden — AI assistant operating as senior engineer, PM, and technical writer",
    "thinking_framework": "Reflective Thought Composition (RTC): restate |> ideate |> reflectCritically |> expandOrthogonally |> scoreRankEvaluate |> respond",
    "depth_levels": "1-10 scale for reasoning depth",
    "safety_constraints": [
      "Do not modify files unless command explicitly requires it",
      "Execute ONE THING at a time with user approval",
      "Verify API confidence before use",
      "Cite documentation before attempting external APIs"
    ]
  },
  "project_structure": {
    "ai/commands/": "Workflow command templates (help, plan, review, task, etc.)",
    "ai/rules/": "Agent orchestration rules (please.mdc is main SudoLang orchestrator)",
    "plan/story-map/": "User journey maps and personas (YAML)",
    "plan/": "Testing scripts (human + agent)",
    "vision.md": "Source of truth for AI agents — prevents architectural conflicts"
  },
  "server_framework": {
    "description": "Lightweight Node/Next.js backend alternative to Express",
    "features": ["createRoute — middleware composition", "createWithConfig — fail-fast config validation", "withRequestId — CUID2 request tracking", "createWithCors — explicit origin validation", "createWithAuth — better-auth session validation"],
    "auth_options": ["Email/password via better-auth", "Passkey/passwordless authentication"]
  },
  "testing_methodology": {
    "approach": "Dual testing from user journeys",
    "human_scripts": "Think-aloud protocol with video recording",
    "ai_agent_scripts": "Executable tests with screenshots and persona-based behavior",
    "research_basis": "Nielsen Norman Group: 3-5 users reveal 65-85% of usability problems"
  },
  "pcg_relevance": {
    "workflow_mapping": "AIDD /discover → /task → /execute → /review maps to our task lifecycle (todo → inprogress → inreview → done)",
    "vision_pattern": "vision.md as source of truth mirrors our project description + knowledge sheaf",
    "testing_dual_track": "Human + AI testing aligns with our requires_approval + agent verification pattern",
    "rtc_thinking": "Reflective Thought Composition could enhance Topsi/NORA reasoning chains",
    "agent_safety": "Their safety constraints parallel our autonomy_level system"
  }
}',
    '{"source_url":"https://github.com/paralleldrive/aidd","research_type":"framework_analysis","contributed_by":"fraze"}',
    'planning',
    'pending'
);

-- 4c. Strategy Document: SudoLang Integration Strategy for PCG
INSERT OR IGNORE INTO execution_artifacts (
    id, execution_process_id, artifact_type, title, content, metadata, phase, review_status
) VALUES (
    X'2EC0B1FA000040008000000000000023',
    NULL,
    'strategy_document',
    'SudoLang Integration Strategy — PCG Agentic Workflows',
    '{
  "summary": "Proposed strategy for integrating SudoLang into PCG platform agent orchestration and prompt engineering",
  "status": "draft_for_review",
  "prepared_by": "fraze",
  "review_requested": ["madhav", "admin"],
  "integration_areas": [
    {
      "area": "Agent Behavior Definitions",
      "current_state": "Agent personalities and capabilities defined as JSON blobs in agents table",
      "proposed": "Define agent behaviors as SudoLang interfaces with constraints, enabling structured yet natural-language-readable behavior specs",
      "example": "interface TopsiAgent {\n  constraints {\n    Always verify task scope before execution\n    Require approval for budget > 50 VIBE\n    Route media tasks to Editron\n  }\n  /plan |> ideate |> evaluate |> propose\n  /execute task |> verify_budget |> run |> report\n}",
      "effort": "medium",
      "impact": "high"
    },
    {
      "area": "Prompt Templates",
      "current_state": "Agent prompts are hardcoded strings in Rust agent modules (nora/src/agent.rs, topsi/src/agent.rs)",
      "proposed": "Extract prompt logic into .sudo.md files that can be versioned, composed, and shared across agents. Use SudoLang pipe operator for multi-stage prompt chains.",
      "example": "user_request |> classify_intent |> route_to_agent |> execute_with_constraints |> verify_output",
      "effort": "medium",
      "impact": "high"
    },
    {
      "area": "Workflow Orchestration",
      "current_state": "Agent flows defined in execution_profiles with JSON config",
      "proposed": "Use AIDD-style workflow commands (/discover, /task, /execute, /review) as first-class workflow phases in our execution pipeline. Map to our existing phases (planning → execution → verification).",
      "effort": "low",
      "impact": "medium"
    },
    {
      "area": "Knowledge Graph Enrichment",
      "current_state": "project_knowledge_sources tracks artifacts via trigger",
      "proposed": "SudoLang constraint definitions could auto-register as knowledge sources, making agent behavior rules queryable through the knowledge sheaf",
      "effort": "low",
      "impact": "medium"
    },
    {
      "area": "Task Description Language",
      "current_state": "Task descriptions are freeform markdown",
      "proposed": "Allow optional SudoLang blocks in task descriptions for structured requirements that agents can parse deterministically. Constraints become acceptance criteria.",
      "effort": "low",
      "impact": "medium"
    }
  ],
  "pilot_use_cases": [
    {
      "name": "Topsi Agent Behavior Spec",
      "description": "Rewrite Topsi agent prompt as a SudoLang interface with explicit constraints, commands, and reasoning depth. Compare token usage and output quality vs current prose prompt.",
      "success_criteria": "Measurably fewer tokens, equivalent or better task routing accuracy"
    },
    {
      "name": "Editron Pipeline as SudoLang Flow",
      "description": "Express the editron ingest |> analyze |> edit |> render pipeline as a SudoLang program with constraints at each stage. Test if LLM can better reason about pipeline state.",
      "success_criteria": "Pipeline decisions (clip selection, timing) are more explainable and debuggable"
    },
    {
      "name": "AIDD-style Code Review for PCG PRs",
      "description": "Adopt the /review command pattern from AIDD to create a structured PR review workflow that checks for duplication, test coverage, and security.",
      "success_criteria": "Catch issues that current manual review misses, reduce review turnaround time"
    }
  ],
  "risks_and_mitigations": [
    {
      "risk": "Team unfamiliarity with SudoLang syntax",
      "mitigation": "SudoLang is intentionally close to natural language — low learning curve. Start with pilot use cases before broad adoption."
    },
    {
      "risk": "Vendor lock-in to pseudocode approach",
      "mitigation": "SudoLang works with all major LLMs (Claude, GPT, Gemini, Llama). It is open source (MIT licensed)."
    },
    {
      "risk": "Over-engineering prompt layer",
      "mitigation": "Start with the 3 pilot use cases only. Measure before expanding."
    }
  ],
  "next_steps": [
    "Team reviews this document and attached research artifacts",
    "Madhav + Admin provide feedback on integration priorities",
    "Select 1 pilot use case for next sprint",
    "Fraze creates proof-of-concept SudoLang agent spec"
  ]
}',
    '{"research_type":"integration_strategy","contributed_by":"fraze","review_requested":["madhav","admin"]}',
    'planning',
    'pending'
);

-- ============================================================================
-- 5. TASK-ARTIFACT LINKS
-- ============================================================================

-- Strategy document: primary, pinned (this is the actionable deliverable)
INSERT OR IGNORE INTO task_artifacts (
    task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by
) VALUES (
    X'2EC0B1FA000040008000000000000020',
    X'2EC0B1FA000040008000000000000023',
    'primary', 1, 1, '2026-03-02T12:00:00', 'fraze'
);

-- AIDD framework analysis: primary
INSERT OR IGNORE INTO task_artifacts (
    task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by
) VALUES (
    X'2EC0B1FA000040008000000000000020',
    X'2EC0B1FA000040008000000000000022',
    'primary', 2, 0, '2026-03-02T12:00:00', 'fraze'
);

-- SudoLang spec: reference material
INSERT OR IGNORE INTO task_artifacts (
    task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by
) VALUES (
    X'2EC0B1FA000040008000000000000020',
    X'2EC0B1FA000040008000000000000021',
    'reference', 3, 0, '2026-03-02T12:00:00', 'fraze'
);
