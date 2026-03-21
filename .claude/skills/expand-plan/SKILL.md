---
name: expand-plan
description: Research the codebase to expand and validate a planning file — finds gaps, contradictions, missing files, and modularity opportunities
user-invocable: true
allowed-tools: Agent, Bash, Read, Write, Edit, Glob, Grep, WebSearch, WebFetch, mcp__sequential-thinking__sequentialthinking
---

# Expand & Validate Planning File

Given a planning file path (or the most recent `planning/*.md` on the current branch), perform a multi-phase research expansion.

## Available MCP Tools

Use these when available to enhance research quality:

- **Sequential Thinking** (`mcp__sequential-thinking__sequentialthinking`): Use for complex reasoning steps — analyzing contradictions, dependency ordering, gap identification. Use **5-20 thoughts** depending on complexity (simple validations: 5, multi-PR dependency analysis: 15-20). Start a thinking session before Phase 3 (Gap Analysis) to reason through dependencies and conflicts systematically.
- **Context7** (via Agent tool): If the plan references external libraries or frameworks, agents can use Context7 MCP to fetch up-to-date documentation.
- **GitHub** (via Agent tool): If the plan references PRs, issues, or CI workflows, agents can use GitHub MCP to verify PR status, check CI results, or read issue context.

## Phase 0: Readiness Check

Before expanding feature work, assess baseline health:

1. **CI status**: Check last 3 CI runs on the target branch. If red, add a fix-CI item before feature work.
2. **Merge distance**: How far is the branch from main? Large gaps = merge conflicts. Factor time accordingly.
3. **Unplanned work buffer**: Add 15-25% buffer for regression analysis, PR reviews, merge conflicts, and CI churn. Plans that fill 100% of available time always slip.

## Phase 1: Parse the Plan

1. Read the planning file completely
2. Extract: sprint items, files to create/modify, architectural patterns, PR strategy, dependencies
3. Identify claims that can be verified against the codebase (line numbers, function names, file existence, enum variants, table columns)

## Phase 2: Parallel Codebase Audit (use Agent tool, max 8 concurrent)

Launch exploration agents to verify each major claim:

- **Access control audit**: For each route file mentioned, verify which endpoints have/lack access checks
- **File existence**: Verify all "Create" files don't already exist, all "Modify" files do exist
- **Function/type verification**: Confirm referenced functions, structs, enums, and methods exist at stated line numbers
- **Dependency check**: Verify crate dependencies mentioned are/aren't in Cargo.toml
- **Migration numbering**: Check latest migration number, confirm proposed numbers don't conflict
- **Dead code check**: For any "reuse" claims, verify the code actually exists and is unused
- **Pattern verification**: For architectural patterns referencing existing code, confirm the patterns match
- **Frontend hook/component audit**: Verify referenced hooks, components, and API clients exist

## Phase 3: Gap Analysis

Use Sequential Thinking MCP to reason through dependencies and conflicts before writing findings.

For each sprint item, check for:

1. **Missing build steps**: Does the item modify SQL queries (needs `cargo sqlx prepare`)? Add Rust types with `#[derive(TS)]` (needs `npm run generate-types`)? Change `package.json` (needs `npm install`)?
2. **Missing files**: Are all files that need changing listed in the files table?
3. **Contradictions**: Do any two items or decisions conflict?
4. **Access control gaps**: Are there route files touched by the sprint that lack access checks?
5. **Test coverage**: Does the verification plan cover all items?
6. **Dependency ordering**: Does the PR dependency chain match the execution schedule?

## Phase 4: E2E Test & Frontend Verification Audit

Review existing E2E tests and demos for patterns relevant to the sprint:

1. **Scan `e2e/` directory** for tests covering affected pages/features — note patterns, helpers, and fixtures used
2. **Scan `e2e/demos/`** for demo scripts that exercise similar workflows
3. **For any frontend UI changes or new implementations**: add Playwright MCP verification instructions to the plan's test section — specify pages to navigate, snapshots to take, console errors to check
4. **UX feedback**: Note any UX issues discovered during Playwright walkthroughs and add them to the planning file under a "UX Observations" section
5. **Test gaps**: Flag sprint items that modify frontend but have no corresponding E2E coverage

## Phase 5: Modularity Scan (use Agent tool, 3 concurrent)

Launch agents to find low-effort extraction opportunities in files touched by the sprint:

- **Backend**: Duplicate patterns, shared helpers, file split candidates (>1,000 lines)
- **Frontend**: Duplicate hooks, components, constants, shared patterns
- **Cross-cutting**: Repeated access control, error handling, config loading patterns

## Phase 6: Acceptance Criteria Audit

For each sprint item, verify the plan includes:

1. **Definition of done** — not "code exists" but "user can do X and see Y". If the plan only describes backend changes without a user-facing verification, flag it.
2. **Stub vs working** — if an item ships scaffolding without real behavior (e.g., a background worker that logs but doesn't dispatch), call it out explicitly so the sprint tracker reflects reality.
3. **Dependency hints** — if items seem coupled (shared types, shared routes), note likely merge order. Don't over-specify — just flag where parallel work may conflict.
4. **Build step suggestions** — if items touch types, schemas, or dependencies, suggest likely build steps (e.g., `cargo sqlx prepare`, `npm run generate-types`, `pnpm install`). Be specific when the steps are known.

## Phase 7: Update the Plan

Apply findings to the planning file:

1. **Fix** any incorrect line numbers, function names, or file references
2. **Add** missing build steps, files, and dependencies
3. **Flag** contradictions with `[CONTRADICTION]` markers
4. **Add** a "Decisions Needed" section for unresolved questions (max 10)
5. **Add** modularity wins (in-sprint vs deferred) with effort estimates
6. **Move** any deferred/out-of-scope items to `BACKLOG--remaining-work.md`

## Output Format

Present findings as:

```
## Expansion Summary

### Verified (N items)
- [list of confirmed claims]

### Corrections (N items)
- [list of fixes applied with before/after]

### Gaps Found (N items)
- [list of missing items added]

### Questions for User (max 10)
1. [question with context]

### Modularity Wins
- In-sprint: [list with effort]
- Deferred: [list, added to backlog]
```

## Important Rules

- Do NOT mark anything as "RESOLVED" — items are resolved when code ships
- Do NOT modify sprint scope without flagging as a question
- Do NOT run destructive git operations
- Do NOT start implementation — this is research only
- Always confirm worktree before making changes
- Keep the planning file focused on actionable content — move reference material to backlog
- Target: planning file should be <600 lines after expansion
