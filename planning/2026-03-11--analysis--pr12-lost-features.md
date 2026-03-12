# Feature Regressions: PR #12 → Current Branch

**Status:** ALL RESTORED (verified via Playwright QA 2026-03-12, PR #15)

Comparing commit `1d87eda0c` (PR #12 merge) to `feature/fraze-2026-03-11` HEAD.

## Organization Profile (`frontend/src/pages/organization-profile.tsx`)

### Complete Removals

1. **Workflow Runs in Overview Tab**
   - Old: Fetched `workflowsApi.listRecentRuns({ organization_id, limit: 10 })`, displayed each run with status color-coding (completed/failed/running), workflow name, records staged/committed/duplicates, model used, timestamp
   - Impact: No visibility into recent workflow activity from org overview

2. **Quick Links Grid**
   - Old: 6 navigation cards (Pipelines, Contacts, Projects, Intelligence, Members, Integrations) with icons, counts, and summary text
   - Impact: Users must use tabs or sidebar instead of overview shortcuts

3. **Contact Detail Modal**
   - Old: `ContactDetailModal` component — full modal with inline edit form (first_name, last_name, email, phone, company, job_title, department, linkedin_url), delete with confirmation, associated deals list, recent activities, metadata footer
   - New: Contact cards link to `/crm/contacts/:id` (navigates away from org page)
   - Impact: Lost in-page editing/viewing; context switch when clicking a contact

4. **Toast Notifications**
   - Old: `toast.success`/`toast.error` on contact create/update/delete, company create
   - New: `sonner` import removed entirely — no user feedback on CRUD operations
   - Impact: Silent success/failure on all CRM mutations from this page

5. **Company Creation Dialog**
   - Old: `CreateCompanyInlineDialog` with name/website/industry fields, "Add Company" button in Companies tab
   - New: Companies merged into ContactsTab as sub-toggle, no creation UI
   - Impact: Cannot create companies from org profile page

6. **Deliverables Tab**
   - Old: `DeliverablesTab` fetched deliverables across all projects, grouped by status (draft/in_progress/review/delivered), showed title, description, type badge, project name, due date
   - New: Tab and route entry completely removed
   - Impact: No aggregated deliverables view from org profile

### Severe Degradation

7. **Member Management (read-only)**
   - Old: Full management — add member (user dropdown + role selector), remove member, change role (inline Select), invite link generation (dialog with role selection + copy), member assignments (expandable: projects, clients, tasks with assign/unassign)
   - New: Read-only list showing name, email, role badge, join date
   - Removed API calls: `addMember`, `removeMember`, `changeMemberRole`, `createInvitation`, `getMemberAssignments`, `assignMember`, `unassignProject`, `unassignClient`
   - Impact: All team management must be done elsewhere (if it exists)

### Partial Degradation

8. **Contact Card Badges**
   - Missing: `person_type` badge, context badge (client/vendor/partner/prospect from `PersonOrgContact`), "Workflow" import badge (checked `custom_fields.source_workflow_run_id`)
   - Gained: `intelligence_status` dot, `research_depth` badge, `research_pass_count`
   - Impact: Less CRM context on contact cards

9. **URL Path-Based Deep-Linking**
   - Old: `PATH_TO_TAB`/`PATH_TO_VIEW` mappings, `useNavigate`/`useLocation` for URL-path routing (e.g., `/organizations/:id/crm/contacts`)
   - New: Query-parameter only (`?tab=contacts`)
   - Impact: Bookmarks/links using path-based routes broken

10. **Intelligence Sub-View URL Routing**
    - Old: Parent passed `view` from URL path to `KnowledgeTab` for sub-view routing
    - New: `IntelligenceTab` manages views internally; no URL deep-linking to specific sub-views
    - Impact: Cannot link directly to e.g. `/intelligence/data-sources`

---

## Sidebar (`frontend/src/components/layout/sidebar.tsx`)

11. **"Social" Nav Item Replaced**
    - Old: "Social" link to `/social-command` in My Workspace
    - New: Replaced with "Calendar" link to `/calendar`
    - Impact: `/social-command` not accessible from My Workspace (still in admin-only Global Views)

12. **Client Name Click Navigation**
    - Old: Clicking client name navigated to `/organizations/:id/clients/:clientId`
    - New: Click toggles expand/collapse; "Client Overview" link inside expanded content
    - Impact: UX regression — extra click to reach client detail

13. **Client-Scoped Pipeline Link**
    - Old: Deliverables link → `?tab=pipelines&client=X` (client-filtered pipeline)
    - New: Link → `/organizations/:id/crm/pipeline` (generic, no client filter)
    - Impact: Lost client-scoped pipeline view

14. **Client Projects Filter Link**
    - Old: Client Overview → `?tab=projects&client=X`
    - New: → `/organizations/:id/clients/:id` (dedicated route)
    - Impact: Different behavior; old filtered-projects view gone

15. **Admin/Management Gating**
    - Old: `isAdmin` boolean check per item
    - New: `roleInfo.canSeeManagement` / `canSeeAdminPlatforms` from `useEffectiveRole()`
    - Impact: Low risk but potential visibility differences if semantics differ

---

## CRM Page (`frontend/src/pages/crm.tsx`) — No Regressions
## API Client (`frontend/src/lib/api.ts`) — No Regressions

---

## Restoration Priority

1. ~~**Contact Detail Modal**~~ — RESTORED: modal with view/edit/delete, deals, activities, metadata
2. ~~**Member Management**~~ — RESTORED: add/remove members, change roles, invite links, member assignments
3. ~~**Workflow Runs in Overview**~~ — RESTORED: recent runs with status, records staged/committed/duplicates
4. ~~**Company Creation Dialog**~~ — RESTORED: Add Company button + dialog (name/website/industry) in Companies sub-view
5. ~~**Toast Notifications**~~ — RESTORED: sonner toast on all CRUD operations
6. **Deliverables Tab** — SKIPPED: separate OrgDeliverablesPage at `/crm/deliverables` covers same need with richer UI (6 statuses, clickable items, type icons)
7. ~~**Contact Card Badges**~~ — RESTORED: workflow import badge; source badge kept
8. ~~**Quick Links Grid**~~ — RESTORED: 6 navigation cards in overview
9. ~~**Sidebar client navigation**~~ — RESTORED: client name click navigates directly, chevron toggles expand/collapse, pipeline link passes `?client=X` filter, fixed `useLocation` bug
10. ~~**URL deep-linking**~~ — RESTORED: tab clicks navigate to proper URL paths (e.g. `/members` not `?tab=members`), intelligence sub-views navigate to `/intelligence/data-sources` etc.
