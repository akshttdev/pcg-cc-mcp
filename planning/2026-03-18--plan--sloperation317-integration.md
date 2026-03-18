# Sloperation317 Integration + Quality Sprint

**Date**: 2026-03-18
**Branch**: `integration/sloperation317-quality` (from main `445e6d7b8`)
**Status**: Planning
**Depends on**: sloperation317 branch (11 commits, 300 files)

## Context

`sloperation317` has 11 unique commits with CRM pipeline automation, company profiles, brand guides, Dockerfile fixes, and VIBE tokenomics planning. It's 3 PRs behind main (PRs #46, #47, #48) and will have conflicts with:

- **DbUuid::parse** (PR #48): ~200 `Uuid::parse_str` → `DbUuid::parse` in route files
- **Path\<String\>** (PR #48): 231 `Path<Uuid>` → `Path<String>` conversions
- **Query key factories** (PR #48): 74 inline keys migrated to factories
- **File splits** (PRs #46, #47): virtual-environment, data-sources, meeting-mode, project-tasks directories
- **Onboarding wizard** (PR #48): AppShell rewrite, new WelcomeWizard component
- **Dead code deletion** (PR #48): MeetingMode.tsx and project-tasks.tsx removed

## Scope

### Phase 1: Merge & Conflict Resolution
1. Merge `origin/sloperation317` into our branch
2. Resolve all conflicts applying main's standards:
   - Use `DbUuid::parse` not `Uuid::parse_str` in route handlers
   - Use `Path<String>` not `Path<Uuid>`
   - Use query key factories not inline strings
   - Keep file split directory structures
3. Verify `cargo check --workspace` and `tsc --noEmit` pass

### Phase 2: Quality Pass on Sloperation Code
Apply PR #48 standards to new sloperation code:
- Replace any new `Uuid::parse_str` with `DbUuid::parse`
- Replace any new `Path<Uuid>` with `Path<String>`
- Replace any new `: any` types with proper interfaces
- Replace any new inline query keys with factory functions
- Fix any new `catch(e: any)` → `catch(e: unknown)`
- Run eslint + prettier on changed frontend files

### Phase 3: Followup Quality Work (B+D+E from sprint proposal)
After sloperation integration is stable:
- **DbUuid Phase C**: Migrate `users.id` BLOB → TEXT (if feasible)
- **E2E test infrastructure**: Fix GITHUB_TOKEN requirement, pipeline stage count, env var pre-flight
- **Remaining quality debt**: File splits for largest remaining files, remaining `: any` types

## Risk Assessment

- **High conflict density**: 43 route files changed in sloperation317, many overlap with PR #48's DbUuid work
- **Migration files**: sloperation317 may have DB migrations that collide with main's numbering
- **Dockerfile changes**: sloperation317 has Docker fixes that may conflict with main's structure

## Verification

- `flox activate -- cargo check --workspace`
- `cd frontend && npx tsc --noEmit`
- `cd frontend && npx eslint src/ --max-warnings=0` on changed files
- Playwright smoke tests: login → CRM pipeline → company profiles → data sources

## Out of Scope

- Agent Flow Orchestration Engine (P0 — separate sprint)
- DbUuid Phase D (batch model conversion — separate sprint)
- VIBE tokenomics implementation (planning only in sloperation317)
