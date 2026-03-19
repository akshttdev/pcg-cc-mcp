---
name: expand-plan
description: Research the codebase to expand and validate a planning file — finds gaps, contradictions, missing files, and modularity opportunities
user-invocable: true
allowed-tools: Agent, Bash, Read, Write, Edit, Glob, Grep
---

# Expand & Validate Planning File

Given a planning file path (or the most recent `planning/*.md` on the current branch), perform a multi-phase research expansion:

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

For each sprint item, check for:

1. **Missing build steps**: Does the item modify SQL queries (needs `cargo sqlx prepare`)? Add Rust types with `#[derive(TS)]` (needs `npm run generate-types`)? Change `package.json` (needs `npm install`)?
2. **Missing files**: Are all files that need changing listed in the files table?
3. **Contradictions**: Do any two items or decisions conflict?
4. **Access control gaps**: Are there route files touched by the sprint that lack access checks?
5. **Test coverage**: Does the verification plan cover all items?
6. **Dependency ordering**: Does the PR dependency chain match the execution schedule?

## Phase 4: Modularity Scan (use Agent tool, 3 concurrent)

Launch agents to find low-effort extraction opportunities in files touched by the sprint:

- **Backend**: Duplicate patterns, shared helpers, file split candidates (>1,000 lines)
- **Frontend**: Duplicate hooks, components, constants, shared patterns
- **Cross-cutting**: Repeated access control, error handling, config loading patterns

## Phase 5: Update the Plan

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
