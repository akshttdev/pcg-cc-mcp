# Sloperation316 Branch Integration — Completed

**Date**: 2026-03-17
**PR**: #45
**Branch**: `integration/sloperation316`

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
