# QA Regression Review — PR #50

**Date**: 2026-03-18
**Branch**: `integration/sloperation317-quality` vs `origin/main`
**Reviewer**: Claude (automated)
**Commits**: 43 commits, 52 files changed, +7857 / -436 lines

## Summary

| Category | Files Changed | Risk | Status |
|----------|--------------|------|--------|
| Backend: crm_deals.rs | 1 | HIGH | ISSUES FOUND |
| Backend: companies.rs | 1 | MEDIUM | PASS with notes |
| Backend: intelligence.rs | 1 | MEDIUM | PASS with notes |
| Backend: db/lib.rs | 1 | LOW | PASS |
| Backend: models/ | 2 | LOW | PASS |
| Frontend: App.tsx | 1 | LOW | PASS |
| Frontend: CRM components | 5 | LOW | PASS |
| Frontend: call-intake.tsx | 1 | MEDIUM | ISSUES FOUND |
| Frontend: company-profile | 1 | LOW | PASS |
| Frontend: brand-guide | 1 (new) | LOW | PASS |
| Frontend: query-keys.ts | 1 | LOW | PASS |
| Frontend: api/business.ts | 1 | LOW | PASS |
| Migrations | 3 (new) | HIGH | ISSUES FOUND |
| Config: playwright.config.ts | 1 | LOW | PASS |
| Config: CLAUDE.md | 1 | LOW | PASS |
| E2E tests | 10 (new) | LOW | PASS (quarantined) |
| Docs/Planning | 8 | N/A | PASS |

## Detailed Findings

### REGRESSION: Access Control Removed from `get_kanban_data` (crm_deals.rs)

**Severity: HIGH**

The `get_kanban_data` handler (`GET /crm/deals/kanban/:pipeline_id`) had its `AccessContext` and `require_org_membership` check **completely removed**. On main, the handler:
1. Extracted `Extension<AccessContext>`
2. Loaded the pipeline to find its `organization_id`
3. Called `require_org_membership` to verify the caller belongs to that org

Now the handler accepts any authenticated user and returns kanban data for any pipeline by ID. This is an **authorization bypass** -- any logged-in user can view any organization's pipeline data.

**Recommendation**: Restore org-scoped authorization, using `DbUuid::parse` for the pipeline_id.

### REGRESSION: `uuid::Uuid` Used in New Code (crm_deals.rs)

**Severity: LOW** (backlog debt, not a runtime bug)

Two instances of `uuid::Uuid` in new code within `trigger_deep_research_pass2`:
- `uuid::Uuid::new_v4()` (line ~693 of diff) -- should use `DbUuid::new().to_uuid()`
- `uuid::Uuid::parse_str(deal_id.as_str())` -- should use `DbUuid::parse`

These are in a background task writing to BLOB columns (`business_reports.id`, `crm_deal_id`), so functionally correct, but violates the DbUuid standard.

### REGRESSION: Raw `fetch()` in call-intake.tsx

**Severity: MEDIUM**

New "Create Deal" button in `call-intake.tsx` uses a raw `fetch('/api/crm/deals', ...)` instead of the established `makeRequest` / `crmDealsApi` pattern. Issues:
1. Bypasses the `makeRequest` wrapper (which handles auth, base URL, error formatting)
2. Manually reads `localStorage.getItem('session_id')` for auth header
3. Has `as any` cast: `(user as any)?.organizations?.[0]?.id`

**Recommendation**: Use `crmDealsApi.createDeal()` and properly type the user object.

### ISSUE: Migration Uses BLOB for New Table (20260412)

**Severity: MEDIUM**

The `company_brand_profiles` migration creates `id` and `company_id` columns as `BLOB`:
```sql
id         BLOB PRIMARY KEY NOT NULL DEFAULT (randomblob(16)),
company_id BLOB NOT NULL UNIQUE REFERENCES companies(id) ON DELETE CASCADE,
```

Meanwhile, the entire purpose of migration `20260406` was to convert CRM tables from BLOB to TEXT. The new brand profiles table goes back to BLOB. The Rust `CompanyBrandProfile` struct uses `DbUuid` which handles both, but this is inconsistent with the TEXT direction and will need a future migration.

Additionally, the route handler inserts with `randomblob(16)` (BLOB) but queries with `DbUuid` (TEXT-compatible). This works because `DbUuid` decodes both, but creates mixed-format data.

### ISSUE: Hardcoded Operator Assignment (crm_deals.rs)

**Severity: LOW** (business logic concern)

The `create_review_task_if_needed` function has hardcoded username-to-org mapping:
```rust
Some(n) if n.contains("Sirak") => Some("Sirak"),
Some(n) if n.contains("PowerClub") || n.contains("PCG") => Some("Bodhi"),
```

This is a hardcoded business rule that will break if org names or usernames change. Should be configurable.

### ISSUE: `CAST(id AS TEXT)` Pattern in Companies Updates

**Severity: LOW**

Several UPDATE queries in `companies.rs` and `intelligence.rs` use `WHERE CAST(id AS TEXT) = ?` to work around BLOB/TEXT mismatch. This is functionally correct but:
1. Cannot use indexes (CAST defeats index lookup)
2. Indicates the `companies` table still has BLOB IDs (wasn't covered by the BLOB-to-TEXT migration)

### NOTE: Role Relaxation in App.tsx

**Severity: LOW** (intentional)

Multiple routes changed from `minRole="platform_member"` to `minRole="org_viewer"`:
- `/people`, `/people/:personId`, `/people/:personId/intel`
- `/proposals`, `/companies`, `/companies/:companyId`
- `/command-center`, `/invoices`

This broadens access to these pages. Appears intentional (CRM features should be accessible to org viewers), but should be verified against product requirements.

### NOTE: `generate_proposal` Refactored -- Auth Removed from Core

**Severity: LOW** (not a regression)

The `generate_proposal` handler was refactored into `generate_proposal_core` (no auth) + a handler wrapper. The handler wrapper still has `require_deal_org_access`. The core function is also called from background tasks (`generate_proposal_background`), which correctly don't need auth since they run server-side.

## Items Verified Clean

1. **No deleted files** from main -- `git diff --diff-filter=D` returned empty
2. **No `Path<Uuid>` additions** -- all new handlers use `Path<String>` + `DbUuid::parse`
3. **No hardcoded `localhost:3000`** -- none in diff
4. **No inline query keys** in new frontend code -- new key uses `entityKeys.companyBrandProfile()`
5. **No new `: any` types** in frontend (except the pre-existing `as any` in call-intake noted above)
6. **No unwrap() in non-test route handlers** -- all use `.map_err()`, `.unwrap_or()`, or `.ok()`
7. **No sensitive data committed** -- API keys accessed via `std::env::var()` (correct pattern)
8. **DATABASE_URL change in db/lib.rs** -- safe; falls back to same default path if env var not set
9. **Company model change** (`find_by_name` instead of `find_by_id` after create) -- correct; avoids BLOB/TEXT hex mismatch
10. **CrmDeal model change** (stage name lookup from DB instead of hardcoded "qualification") -- correct improvement
11. **Playwright config** -- `testIgnore` now also skips `quarantine/` -- correct, quarantine tests need separate runs
12. **LLM fallback pattern** (OpenAI-first, Anthropic-fallback) -- consistent across intelligence.rs and crm_deals.rs

## Pre-existing Quality Debt Not Addressed (Acceptable)

- 84 `Path<Uuid>` remaining in non-CRM routes (tracked in backlog)
- 29 `: any` types remaining in RJSF/DiffCard/conversation code
- 15 inline query keys in niche domain code
- 27 Rust warnings in nora/alpha-protocol crates
- `companies` table still uses BLOB IDs (not covered by BLOB-to-TEXT migration)

## Sign-off

- [x] **Backend: crm_deals.rs** -- FIXED: `get_kanban_data` access control restored (commit `8a77952`)
- [x] **Backend: companies.rs** -- no regressions (DbUuid migration clean)
- [x] **Backend: intelligence.rs** -- no regressions (LLM fallback logic sound)
- [x] **Backend: db models** -- no regressions
- [x] **Frontend: call-intake.tsx** -- FIXED: raw fetch() → makeRequest (commit `8a77952`)
- [x] **Frontend: all other** -- no regressions
- [x] **Migrations** -- FIXED: company_brand_profiles BLOB → TEXT (commit `8a77952`)
- [x] **Tests** -- 104/104 pass, quarantined appropriately, promoted test passes
- [x] **Docs** -- updated accurately

## Issues Found & Fixed

1. ~~**MUST FIX**: Restore org authorization on `get_kanban_data` endpoint~~ → **FIXED** (commit `8a77952`)
2. ~~**SHOULD FIX**: Replace raw `fetch()` in call-intake.tsx with `crmDealsApi`~~ → **FIXED** (commit `8a77952`)
3. ~~**NICE TO HAVE**: Use TEXT columns in `company_brand_profiles` migration~~ → **FIXED** (commit `8a77952`)

## Remaining Low-Effort Items Not Addressed

These are pre-existing or intentional — acceptable to defer:
