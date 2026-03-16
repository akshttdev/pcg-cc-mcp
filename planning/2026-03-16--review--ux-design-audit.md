# UX Design Audit — Gaps & Shortfalls

**Date**: 2026-03-16
**Type**: review
**Method**: Live Playwright MCP walkthrough of all major workflows
**Screenshots**: `ux-review/01-12*.png`

---

## Executive Summary

The platform has strong bones — clear information architecture, consistent sidebar navigation, good use of color coding for status/priority. However, there are significant UX gaps that would block or confuse real users, particularly around empty states, navigation clarity, information density, and first-run experience.

---

## Critical UX Gaps (P0)

### 1. No Onboarding or First-Run Experience
**Where**: Every page after login
**Problem**: A new user lands on the Projects page with zero guidance. No welcome wizard, no tooltips, no "get started" checklist. The platform is complex (CRM, Workflows, Intelligence, Agents, Kanban, Pipeline) but offers no progressive disclosure.
**Impact**: Users will bounce or feel overwhelmed.
**Recommendation**: Add a dismissable onboarding checklist widget (similar to Linear's onboarding) that guides through: create org → create project → create first task → explore CRM pipeline.

### 2. Empty Pipeline Board Is a Dead End
**Where**: CRM > Pipeline (`ux-review/04-crm-pipeline.png`)
**Problem**: 8 empty pipeline columns with "No deals" and tiny "+ Add deal" buttons. The board looks broken/incomplete rather than ready to use. No visual hierarchy distinguishing the stages — they all look identical.
**Impact**: Users see a wall of empty gray columns and don't understand the pipeline progression or what the stages mean.
**Recommendation**:
- Show a single-card onboarding prompt in the first column: "Add your first deal to start tracking prospects"
- Add subtle stage descriptions or tooltips (e.g., "Lead: Initial contact, Nora auto-researches")
- Differentiate stages visually — e.g., gradient background intensity from left (cold) to right (hot)
- The "Nora" / "Account Manager" / "Topsi + PM" labels under stages are cryptic — add a tooltip: "This stage is managed by [agent]. They will automatically [action]."

### 3. Task Cards Are Too Uniform — No Visual Differentiation
**Where**: Kanban board (`ux-review/03-kanban-board.png`)
**Problem**: All 12 task cards look identical — same code icon, same layout. No assignee avatars, no tags visible, no due dates, no description preview. The only differentiator is title text and a small priority badge.
**Impact**: Users can't scan the board to find what matters. The board feels like a flat list rather than a visual workflow tool.
**Recommendation**:
- Show assignee avatar (or "Unassigned" placeholder) on each card
- Show tags as colored chips
- Show due date (or "overdue" warning) when present
- Add a subtle description preview (first line, truncated)
- Consider color-coding the left border by priority (red=critical, yellow=medium, etc.)

### 4. Notification Dropdown Is Minimal
**Where**: Notification bell → dropdown (`ux-review/08-notifications.png`)
**Problem**: Dropdown shows "Activity: Recent activity across your projects" header and "No recent activity." There's no way to filter, no tabs (All/Unread/Mentions), no settings link, no "mark all read" button. Even with activity, the dropdown is a simple text list with no action buttons.
**Impact**: Notifications are a primary engagement driver. The current implementation doesn't support quick triage or action-taking.
**Recommendation**:
- Add tabs: All | Unread | Mentions
- Add "Mark all read" button
- Add quick action buttons per notification (from backlog: "Approve", "Review PR", "Mark Done")
- Add a "View all" link to a full notifications page
- Show notification count badge on the bell icon

---

## Major UX Issues (P1)

### 5. Sidebar Information Overload
**Where**: All pages — left sidebar
**Problem**: The sidebar tries to serve too many masters: My Workspace (6 items) + Management (9 items) + Global Views (3 items) + Organizations + Internal Projects + Clients. On the org page, it also expands CRM sub-nav, Social sub-nav, Intelligence sub-nav. This creates a sidebar that can easily exceed viewport height.
**Impact**: Users must scroll the sidebar to find items. Mental model is unclear — "Management" with 9 items and "Global Views" with 3 items are collapsed with counts but no hint of contents.
**Recommendation**:
- Collapse My Workspace by default after first visit (user has already seen it)
- Use a flyout/hover-expand pattern for Management and Global Views instead of inline expansion
- Move VIBELAND and VIBE out of the primary workspace nav — they're distinct product areas

### 6. Task Detail Drawer Competes with Kanban for Space
**Where**: Task detail view (`ux-review/11-task-detail-drawer.png`)
**Problem**: The drawer opens on the right, pushing the kanban to ~40% width. The kanban becomes nearly unusable — you can see one partial column. The drawer itself packs in 5 tabs (Overview, Artifacts, Workflow, Logs, Vibe), an agent terminal, agent reviewers, and activity feed.
**Impact**: Users lose kanban context when viewing task details. The drawer tries to be a full page in a half-page space.
**Recommendation**:
- Default to a wider drawer (70/30 split) or allow the drawer to go fullscreen
- The "Expand" button exists on Agent Terminal but not on the drawer itself — add a prominent fullscreen toggle in the drawer header
- Consider a slide-over pattern that covers the kanban entirely (with a visible back button) rather than cramming both

### 7. "Source of truth: ." in Brand Identity
**Where**: Project detail page (`ux-review/02-project-detail.png`)
**Problem**: The brand identity card shows `Source of truth: .` and `Repository: .` — these are clearly placeholder/unset values being displayed as literal periods.
**Impact**: Looks broken. Users will question data integrity.
**Recommendation**: Show "Not configured" or hide the field entirely when the value is empty or "."

### 8. Test/E2E Data Pollutes the UI
**Where**: Kanban board, My Tasks
**Problem**: All visible tasks are E2E test artifacts: "[E2E] Kanban Test 1773352134117", "API Health Check 1773345781303". There's no way for a user to tell these are test data vs. real tasks.
**Impact**: This is a dev environment issue, but it highlights a missing feature: no bulk delete, no "clean up test data" action, no way to filter by tag prefix.
**Recommendation**:
- Add a "development mode" banner on pages when `RUST_ENV=development` (partially exists as the orange banner, but it doesn't affect data)
- Add bulk select + delete on kanban board
- Consider auto-prefixing E2E-created tasks with a hidden tag for easy cleanup

### 9. Integrations Hub Shows All Missing — No Prioritization
**Where**: Project detail → Integrations Hub (`ux-review/02-project-detail.png`)
**Problem**: 6 integration categories, all showing "0/N connected" with red "Missing" badges. GitHub, Gmail, Zoho Mail, Instagram, Facebook, LinkedIn, X, YouTube, TikTok, Airtable, Stripe, Google Analytics — all listed as missing. This is demoralizing.
**Impact**: A wall of red "Missing" badges creates anxiety rather than motivation. User doesn't know which integrations are essential vs. optional.
**Recommendation**:
- Prioritize: show "Recommended" integrations first (GitHub, maybe Gmail)
- Collapse optional integrations (Social × 6, Analytics) into expandable sections
- Use amber/gray instead of red for optional missing integrations
- Add a completion percentage or "Quick Start: connect GitHub to unlock code workflows"

### 10. Org Overview KPI Cards Are Static
**Where**: Org overview page (`ux-review/10-org-overview.png`)
**Problem**: The top stat cards (Total Tasks: 12, Total Deals: 0, Pipeline Value: $0, Contacts: 0, Active Projects: 1) are just numbers with no trends, sparklines, or period selectors. The quick-nav cards below (Pipelines, Contacts, Projects, Intelligence, Members, Integrations) are just text links styled as cards.
**Impact**: The overview page doesn't tell a story. Users can't see "are things getting better or worse?"
**Recommendation**:
- Add simple trend arrows (↑12% this week) or sparkline charts
- Add time period selector (7d / 30d / 90d)
- The quick-nav cards could show a preview metric (e.g., "Pipelines: 2 active, $45K in pipeline")

---

## Medium UX Issues (P2)

### 11. Workflow Builder Cards Lack Status Indicators
**Where**: My Workflows page (`ux-review/06-workflows.png`)
**Problem**: 5 workflow definition cards show name, description, node tags, and play/edit buttons. But there's no indication of: last run time, success/failure rate, whether triggers are configured, or if the workflow is active/paused.
**Impact**: Users can't tell which workflows are working and which need attention.
**Recommendation**: Add a small status indicator (last run: 2h ago ✓ / last run: failed ✗ / never run)

### 12. My Tasks List View Lacks Batch Actions
**Where**: My Tasks page (`ux-review/07-my-tasks.png`)
**Problem**: The list shows all 12 tasks with status and priority badges, but no checkboxes for bulk operations. The only interaction is clicking into a task. No way to bulk-change status, reassign, or archive.
**Impact**: Task triage is one-at-a-time only.
**Recommendation**: Add a checkbox column, show a floating action bar when items are selected (Move to..., Change status, Archive, Delete)

### 13. Settings Page Has Unintuitive Tab Layout
**Where**: Settings (`ux-review/09-settings.png`)
**Problem**: Settings has a top-level tab bar (User / Admin / Org / Client) and then a vertical side menu (General, Wallet, Profile, Privacy, Activity Log, API Keys, Topsi Preferences, Integrations "Coming soon"). The two-axis navigation creates confusion.
**Impact**: Users may not discover all settings. The "Coming soon" integrations tab feels unprofessional.
**Recommendation**:
- Remove "Coming soon" items or move them to a roadmap page
- Consider a single-level nav (flat list in left column) rather than tabs + sidebar

### 14. Topsi Chat Widget Feels Disconnected
**Where**: Topsi floating widget (`ux-review/12-topsi-assistant.png`)
**Problem**: The chat widget opens as a small floating card in the bottom-right. It has "Start a conversation with Topsi" placeholder, a push-to-talk mic button, text input, and "Start call" button. But there's no context about what Topsi can do, no suggested prompts, no indication of whether it's connected/ready.
**Impact**: Users don't know what to ask. Voice features (push-to-talk, call) may not work without additional setup but there's no feedback about that.
**Recommendation**:
- Add 3-4 suggested prompt pills: "Summarize my tasks", "What deals need attention?", "Create a task for..."
- Show connection status (online/offline indicator)
- Hide voice features if the voice backend isn't configured, rather than showing non-functional buttons

### 15. Breadcrumb Navigation Inconsistencies
**Where**: Various pages
**Problem**: Breadcrumbs change format between pages:
- Project detail: `Home > Powerclub Global > ORCHA Platform`
- Task kanban: `Home > Powerclub Global > ORCHA Platform > Tasks`
- CRM Pipeline: `Home > Powerclub Global > CRM > Pipeline`
- My Tasks: `Home > Jungleverse > My Tasks`
The "Jungleverse" entity appears in some breadcrumbs but not others. The org name vs. entity name switching is confusing.
**Impact**: Users lose their place in the hierarchy. "Jungleverse" appearing randomly is disorienting.
**Recommendation**: Standardize breadcrumb pattern. Always start with org name, then path. Hide "Jungleverse" or explain what it is.

### 16. Login Page Shows "Session Expired" on First Visit
**Where**: Login page
**Problem**: Navigating to the root URL redirects to `/login?expired=1` with a yellow "Your session has expired" alert — even on first visit or after clearing cookies.
**Impact**: Creates false alarm. Users who haven't logged in before see an error message.
**Recommendation**: Only show the expiry message when there was actually a prior session (check for a cookie/token before showing).

---

## Low Priority / Polish (P3)

### 17. Development Mode Banner Takes Valuable Space
The orange "Development Mode - This is a development build" banner uses ~32px of vertical space on every page. Consider making it a small badge or corner indicator in development builds.

### 18. Project Cards Could Show More Context
On the Projects page, each card shows name, status badge, creation date, boards count, and tasks count. Missing: last activity timestamp, assignee avatars, progress indicator (% tasks done).

### 19. CRM Sub-nav Duplication
CRM appears in both the sidebar (under org) and as a sub-nav item under the project. The project-level CRM and org-level CRM point to the same org CRM page, which is confusing.

### 20. Kanban Column Headers Need Add-Task Button
The kanban columns have headers with status name and count, but no "+" button to add a task directly to that column. Users must use the top-bar "Create new task" button and then manually set status.

### 21. No Dark Mode Preview
The Settings page shows a Theme selector (System/Light/Dark) but the screenshots show only light mode. Verify dark mode renders correctly across all major screens.

---

## Summary by Priority

| Priority | Count | Theme |
|----------|-------|-------|
| P0 Critical | 4 | Onboarding, empty states, task card density, notifications |
| P1 Major | 6 | Sidebar overload, drawer layout, placeholder data, integrations wall, KPIs, test data |
| P2 Medium | 6 | Workflow status, batch actions, settings layout, Topsi guidance, breadcrumbs, login |
| P3 Polish | 5 | Dev banner, project cards, CRM duplication, kanban add, dark mode |

---

## Recommended Sprint Priorities

**Sprint A (Quick wins — 2-3 days):**
- Fix "Source of truth: ." placeholder display (#7)
- Fix login expired message on first visit (#16)
- Add suggested prompts to Topsi widget (#14)
- Add "+" button to kanban column headers (#20)

**Sprint B (High-impact — 1 week):**
- Enhance task cards with assignee, tags, dates (#3)
- Add notification tabs + mark-all-read + count badge (#4)
- Add pipeline stage descriptions/tooltips (#2)
- Add batch select to My Tasks (#12)

**Sprint C (Architecture — 1-2 weeks):**
- First-run onboarding checklist (#1)
- Reorganize sidebar navigation (#5)
- Rethink integrations hub presentation (#9)
- Add trend indicators to org overview KPIs (#10)
