# Dashboard Exploratory Bug Audit — 2026-05-18

Session: sirak@sirakstudios.com on dashboard.powerclubglobal.com

## Bugs Found

### B1 — Org root page fires 168x HTTP 403 on /api/tasks
- **URL:** /organizations/02020202-0202-0202-0202-020202020202
- **Behavior:** Page loads then issues 30+ `GET /api/tasks?project_id={uuid}` calls, ALL returning 403 Forbidden. Each project gets requested 3 times.
- **Impact:** ~200 console errors per page load. Probably means task-count badges are silently failing.
- **Hypothesis:** Authorization rule rejects task listing for projects the user isn't a member of. Should either be silent or filtered server-side. Currently every project queries are fired client-side then 403s.

### B2 — Invalid UUID project ID `a1aca00c-0000-0000-0000-000000000p01`
- Literal `p` in the UUID causes 400 Bad Request on every list endpoint that accepts it.
- Appears in TIACA — Strategic Growth Partnership project.
- Already documented in social-posts bug report.


### B3 — Project cards on /projects show "Forbidden" text in their body 🔴
- **URL:** /projects
- **Behavior:** Every project card on the global "Projects" page renders the literal string "Forbidden" where description/health info should be.
- **Cause:** The 403 from /api/tasks (Bug B1) is being surfaced into the card. Error response leaks into UI instead of being swallowed or surfaced as a state.
- **Visible to:** Every user. Page is unusable as a project list.


### B4 — Calendar page title is just "Powerclub Global" (no page-specific prefix)
- **URL:** /calendar
- **Behavior:** Browser tab title reads "Powerclub Global" while h1 says "Calendar". Other pages like /my-tasks correctly say "My Tasks — Powerclub Global".
- **Impact:** Multi-tab UX, browser history, screen-reader navigation.

### B5 — Calendar fires same 403 /api/tasks fanout (88 errors on load)
- **URL:** /calendar
- **Behavior:** Same as B1 — even though the calendar isn't a project-task surface, it still fans out 30+ task queries on load.
- **Possible cause:** Sidebar tree pre-fetches task counts. Should be a single aggregated endpoint.


### B6 — Org Admin sees 44 projects but can only open 1 🔴🔴 (CRITICAL)
- **URL:** /projects/{id}/tasks (any project)
- **User:** sirak@sirakstudios.com (Org Admin per UI)
- **Behavior:**
  - `GET /api/projects` returns 44 projects → all rendered in sidebar with counts
  - `GET /api/projects/{id}` returns **403 Forbidden** for 43 of the 44
  - The only accessible project is `sirak-studios` (the user's own project)
- **UI symptom:** Click any client project → page renders "Failed to load project" and "Loading boards…" infinite spinner
- **Root cause hypothesis:** List endpoint scoped to org-membership (lenient), detail endpoint scoped to project-membership (strict). Org Admin role does not auto-grant project access. Probably needs an `is_org_admin OR is_project_member` check in the detail route.
- **Severity:** This is the dashboard's main job. Org admins can navigate to projects but can't open them. Effectively the app is broken for them.

### B7 — Sidebar fans out per-project task counts to a permission-checked endpoint → 403 storm on every page
- **Behavior:** Every page load triggers 30+ `GET /api/tasks?project_id={id}` requests (one per visible sidebar project). Each 403s for projects the user isn't a member of (i.e., almost all of them).
- **Fix:** Either move the task-count to an aggregated endpoint server-side, or make the per-project endpoint return 0 instead of 403 when the user isn't a member.


### B8 — New Company creates orphan record (not auto-linked to org)
- **URL:** /organizations/{org}/crm/companies
- **Reproduction:** Click "New Company", fill name, hit Create. POST returns 200 with company id. Company DOES NOT appear in the org's Companies table or sidebar Clients.
- **Cause:** Creating a Company doesn't auto-create the Company↔Organization relationship. The UI implies it would.
- **Fix:** Either auto-link on creation (org_id from URL context) OR add a "Link to org" step after creation.

### B9 — No DELETE endpoint for companies
- **Reproduction:** `DELETE /api/companies/{id}`, `DELETE /api/crm/companies/{id}`, `DELETE /api/organizations/{org}/companies/{id}` all return 404.
- **Impact:** Companies are immortal once created. Mistakes can't be undone via UI/API.
- **Workaround:** PATCH to rename, but can't truly delete.

### B10 — API envelope inconsistency
- `/api/organizations/{org}/companies` and `/api/organizations/{org}/clients` return RAW ARRAYS — no `{success, data, error_data, message}` wrapper.
- All other endpoints (`/api/social/posts`, `/api/companies`, `/api/projects`, etc) return the wrapped envelope.
- **Impact:** Client code has to special-case. Easy to get null/empty/error handling wrong.


### B11 — Quick Actions palette shows "C" keyboard shortcut but pressing C does nothing
- **URL:** any page (⌘K palette)
- **Behavior:** "Create Task" row in the ⌘K palette shows the `C` shortcut next to it. Pressing C with the palette open does nothing (typed into search input instead). Pressing Enter on the highlighted row also does not trigger.
- **Workaround:** Must mouse-click the row.

### B12 — Client/project tab "Pipeline" is a stub redirect, not a pipeline view
- **URL:** /organizations/{org}/clients/{id} (Pipeline tab)
- **Behavior:** Tab renders an empty card with "Open Pipeline" CTA that navigates away to the org-level pipeline. No client-scoped pipeline view exists.
- **UX:** Wasted tab. Either inline a filtered pipeline or remove the tab.

### B13 — Brand Guide page browser tab title is generic "Company" (no brand/page-specific title)
- **URL:** /companies/{id}/brand-guide
- **Behavior:** Tab title reads "Company — Powerclub Global"; should be "Brand Guide — {company} — Powerclub Global" or similar.

### B14 — Tabs duplicated on client detail page
- **URL:** /organizations/{org}/clients/{id}
- **Behavior:** "Members", "Intelligence", "Intel", "Social" appear both in the small header chips (Intel | Brand Guide | Members | Convert to…) AND as full-size tabs (Overview | Projects | People | Members | Pipeline | Social | Intelligence | Intel | Integrations). Two surfaces, partly overlapping. Confusing.

### B15 — Members page h1 shows org name, not "Members"
- **URL:** /organizations/{org}/members
- **Behavior:** Browser tab title says "Members — Powerclub Global" but the page h1 says "Sirak Studios". h1 should match the page (Members), not the org.


### B16 — Topsi assistant errors with no detail ("I encountered an issue. Please try again.")
- **URL:** /interface
- **Reproduction:** Type any message ("What is 2 + 2?") in the Topsi input → press Enter.
- **Behavior:** Status pulses "THINKING / Consulting knowledge graph…" for ~8s, then resolves to "I encountered an issue. Please try again." with no error code, no logs surfaced to the user.
- **Impact:** The agent surface is unusable. Brand promise ("your platform intelligence agent") fails on first interaction.

### B17 — Topsi greeting hardcoded to "Good morning"
- Greeting reads "Good morning, Sirak…" regardless of time of day. (Tested at ~01:23 local / ~05:23 UTC.)

### B18 — /interface page tab title is generic "Powerclub Global"
- Should be "Topsi — Powerclub Global" or similar.


### B19 — Notifications bell shows count, but records lack user-facing fields
- **URL:** /api/notifications?limit=30
- **Response shape:** `[{ id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp, actor_name }]`
- **Missing:** `title`, `body`, `message`, `read_at`. This is an event log, not a notifications feed. The bell shows "5" but the panel can't render meaningful text.
- **Fix:** Either (a) project events into user-facing notification rows with title/body, or (b) change the bell to show a different count source.

### B20 — `/api/notifications/inbox` returns 0; `/api/notifications` returns 5 (different data)
- Two endpoints, different shapes, different counts. Pick one canonical source.

### B21 — Personal Intelligence page is empty for new users with no guidance
- **URL:** /intelligence
- "No personal files / Personal files from your Sovereign Stack will appear here."
- No CTA to upload, no link to Sovereign Stack docs, no example. User sees an empty page with no path forward.

### B22 — Settings "Save" button label appears truncated ("Save S…")
- **URL:** /settings/general (bottom-right)
- Button width seems to be clipping the label. May be a viewport/CSS issue.


### B23 — REST routing inconsistency / many 404 endpoints
- **Tested at top-level:** `/api/deals`, `/api/pipelines`, `/api/decks`, `/api/people`, `/api/contacts`, `/api/workflows`, `/api/clients` → all return 404.
- **Tested under /api/crm/:** `/api/crm/pipelines`, `/api/crm/deals` → 400. `/api/crm/companies` → 404.
- **Tested under /api/organizations/{id}/:** `/contacts` works (200), but `/pipelines`, `/decks`, `/workflows` → 404.
- **Impact:** No consistent REST root. Discovering or scripting against the API requires reading the UI bundle to find each endpoint individually. Easy to write broken integrations.

### B24 — `/api/social/posts` (no filter) returns 500 with DB decode error
- **Response:** `{ success: false, message: "DatabaseError: error occurred while decoding column 'id': in..." }`
- **Same family** as the social-posts list bug (B3 in main report). Some rows in the `social_posts` table have IDs that don't decode as `uuid::Uuid`. Likely the same legacy corrupt rows.
