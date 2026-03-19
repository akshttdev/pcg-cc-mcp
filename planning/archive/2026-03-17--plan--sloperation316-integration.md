# Sloperation316 Branch Integration — QA Complete, Ready for Review

**Date**: 2026-03-17
**PR**: [#45](https://github.com/KingBodhi/pcg-cc-mcp/pull/45)
**Branch**: `integration/sloperation316`
**Status**: All critical/security issues resolved. Ready for merge.

## What Was Integrated

### Phase 1 — Pipeline Progress (4 commits)
Cherry-picked from `sloperation316-pipeline-progress`:
- `10da5c097` feat(crm): Phase 1 pipeline workflow — person/company links, research tasks, BA report
- `63d8df9df` feat(crm): People/Companies/Pipeline three-view tab + Phase 2 intel improvements
- `81dd3d7f9` fix(crm): People/Companies from org contacts + business-reports open to all users
- `fd397c36e` feat(pipeline): company profile links, /people route consolidation, BLOB UUID fix

**Key additions**: 3 DB migrations (BLOB→TEXT, cleanup, dealflow v2 with 9 stages), CRM deal enrichment with auto-triggers, proposal/deck/invoice endpoints, person/company profile redesigns, E2E test suite.

### Phase 2 — Database Sync (3 commits)
Cherry-picked from `spleration316-database-sync`:
- `333277af9` feat: Sovereign Stack — org data hosting via master node
- `a416ec0eb` feat: sovereign stack only — remove legacy volumes, enhance preview
- `2a87fe85a` feat: Intelligence page, APN Cloud branding, data separation & roadmap

**Key additions**: org_cloud migration (cloud_files, cloud_contributions, org_cloud_settings tables), sovereign stack background scraper, cloud file indexer, REST API for browse/upload/download, intelligence page.

### Phase 3 — Skipped
`sloperation316` had 0 unique commits — pure subset of the other two branches.

## Conflicts Resolved

| File | Type | Resolution |
|------|------|------------|
| `navbar.tsx` | Import merge | Kept both: DevBanner + useViewStore |
| `person-profile.tsx` | Import + query keys | Kept extended types + entityKeys pattern |
| `nora.rs`, `topsi.rs`, `twilio.rs` | Modify/delete | Accepted deletion — files split into dirs by modularity sprints #42/#43 |
| `access_control.rs` (9 regions) | BLOB vs string binding | Kept HEAD's BLOB byte approach consistently |
| `auth_sqlite.rs` (3 regions) | OrgRow naming | Kept HEAD's OrgRow2 rename |
| `mod.rs` | Module declarations | Kept both: communications + org_cloud |
| `brand.rs` (2 regions) | Duplicate functions | Discarded incoming — already in modular dir |
| `api/index.ts` | Export list | Kept all exports from both sides |
| `query-keys.ts` (2 regions) | Key definitions | Merged all key objects |
| `organization-profile/index.tsx` | Lazy imports | Kept both: OrgWikiTab + CloudTab |
| `person.rs` (2 regions) | UUID conversion | Kept HEAD's DbUuid::into_string() approach |

## Build Fixes Applied

- `access_control.rs`: `user_id_str` → `user_id_bytes` (3 sites — BLOB column binding)
- `crm.ts`: `CrmDealRich.company_id: string | null` → `?: string` (match base type)
- `crm.ts`: `CrmDealRich.company_intelligence_summary: string | null` → `?: string`
- `ActivityTab.tsx`: `'completed'` → `'done'` (invalid TaskStatus value)
- `person-intel.tsx`: `personsApi.getById()` → `personsApi.get()`
- `ClientProjectPanel.tsx`: `tasksApi.getByProject()` → `tasksApi.getAll()`
- `ClientProjectPanel.tsx`: Removed missing `project_status`/`description` fields
- Commented out unused code in ContactsTab.tsx
- Cleaned up unused imports across 8 files

## Migration Notes

- Renamed `20260408000000_org_cloud.sql` → `20260409000000_org_cloud.sql` to avoid timestamp collision with `20260408000000_dealflow_pipeline_v2.sql`

## QA Review — Issues Found & Resolved

### Critical (fixed)

| Issue | Fix |
|-------|-----|
| `storage_volume` CHECK constraint missing `sovereign_personal`/`sovereign_org` | Added to `20260409000000_org_cloud.sql` CHECK constraint |
| `browse_files` read `projects.id` as `Vec<u8>` — fails silently if TEXT | Changed to `IdRow { id: String }` with `CASE WHEN typeof(p.id) = 'blob' THEN lower(hex(p.id)) ELSE p.id END` |

### Security (fixed)

| Issue | Fix |
|-------|-----|
| Path traversal: `resolve_volume_path` only checked `..` literally | Added canonicalize verification, null byte rejection, encoded `..` check |
| Content-Disposition header injection via malicious filenames | Sanitize `"` and `\` in `file.file_name` |
| Hardcoded Windows paths in `org_cloud.rs`, `org_cloud_indexer.rs`, `data_sources.rs` | Replaced with `SOVEREIGN_STACK_ROOT`/`SOVEREIGN_STACK_ORG_NAME`/`SOVEREIGN_STORAGE_ROOT` env vars |
| Hardcoded upload path in `contribute_file` | Replaced with `resolve_volume_path("sovereign_org", "Uploads")` |
| TOCTOU race in `update_file` — org check after mutation | Moved ownership check before `CloudFile::update` call |
| LIKE search injection (`%`, `_` not escaped) | Added `ESCAPE '\'` clause + input escaping in `CloudFile::browse` |
| Sync `std::fs` in async handler (`contribute_file`) | Replaced with `tokio::fs::create_dir_all` / `tokio::fs::write` |

### Deferred (not blocking)

| Issue | Reason |
|-------|--------|
| Communications + CRM deal handlers lack org-level authorization | All routes behind `require_auth` (authenticated), need fine-grained authz — tracked |
| `~20+ any` types in new TS code | Pre-existing pattern, not introduced by this PR — tracked in backlog |
| `Company::find_by_id` uses `hex(id)` workaround | Blocks on DbUuid Phase C (`users.id` BLOB→TEXT migration) |
| Business reports route changed to `ProtectedRoute` | Intentional per commit `81dd3d7f9` |
| Sovereign stack no graceful shutdown (CancellationToken) | Acceptable for background scraper |

## DbUuid TODO Comments

Added `// TODO(dbuuid)` annotations to 16 Rust files with heaviest Uuid conversion boilerplate, pointing to `planning/2026-03-17--plan--dbuuid-migration.md`. Top hotspots:

| File | Conversion count |
|------|-----------------|
| `topsi/platform_data.rs` | 42 |
| `routes/tasks.rs` | 16 |
| `routes/org_invitations.rs` | 11 |
| `routes/crm_deals.rs` | 9 |
| `routes/invitations.rs` | 8 |

## Commit History

| Hash | Message |
|------|---------|
| `0b4d617fe` | feat(crm): Phase 1 pipeline workflow |
| `f9a3a6ee1` | feat(crm): People/Companies/Pipeline three-view tab |
| `e429f7cd1` | fix(crm): People/Companies from org contacts |
| `cc2cbee8c` | feat(pipeline): company profile links, BLOB UUID fix |
| `47bfbc0ca` | feat: Sovereign Stack — org data hosting |
| `097252ed0` | chore: rename org_cloud migration |
| `661e72cf4` | feat: sovereign stack only — remove legacy volumes |
| `7127e30ab` | feat: Intelligence page, APN Cloud branding |
| `418cacadb` | fix: resolve build errors from cherry-pick integration |
| `1fb03dbd5` | feat: replace window.confirm with showConfirm dialog |
| `3cb61d165` | refactor: centralize query keys in hooks |
| `a47994a82` | docs: add integration tracker and DbUuid migration plan |
| `61990c363` | fix: harden org cloud — path traversal, env vars, header sanitization |
| `b564de793` | docs: update integration tracker with QA results and backlog |
| `ea9541131` | fix: org cloud — TOCTOU race, hardcoded upload path, LIKE injection, sync fs |
