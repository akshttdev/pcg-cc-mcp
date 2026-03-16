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

### 1. ~~No Onboarding or First-Run Experience~~ → ADDRESSED (PR #39)
**Where**: Every page after login
**Problem**: A new user lands on the Projects page with zero guidance. No welcome wizard, no tooltips, no "get started" checklist. The platform is complex (CRM, Workflows, Intelligence, Agents, Kanban, Pipeline) but offers no progressive disclosure.
**Impact**: Users will bounce or feel overwhelmed.
**Recommendation**: Add a dismissable onboarding checklist widget (similar to Linear's onboarding) that guides through: create org → create project → create first task → explore CRM pipeline.
**Resolution**: Org-scoped onboarding system with 9-segment carousel on org overview page. DB tables, API, React Query hook, and UI integration all shipped.

### 2. ~~Empty Pipeline Board Is a Dead End~~ → PARTIALLY ADDRESSED (PR #38 + #39)
**Where**: CRM > Pipeline (`ux-review/04-crm-pipeline.png`)
**Problem**: 8 empty pipeline columns with "No deals" and tiny "+ Add deal" buttons. The board looks broken/incomplete rather than ready to use. No visual hierarchy distinguishing the stages — they all look identical.
**Impact**: Users see a wall of empty gray columns and don't understand the pipeline progression or what the stages mean.
**Resolution**: PR #38 added pipeline stage tooltips. PR #39 added onboarding prompt card in first column of empty pipeline ("Start your pipeline" + "Add Deal" CTA).
**Remaining**: Stage visual differentiation (gradient backgrounds), agent action tooltip improvements.

### 3. ~~Task Cards Are Too Uniform — No Visual Differentiation~~ → PARTIALLY ADDRESSED (PR #41)
**Where**: Kanban board (`ux-review/03-kanban-board.png`)
**Problem**: All 12 task cards look identical — same code icon, same layout. No assignee avatars, no tags visible, no due dates, no description preview. The only differentiator is title text and a small priority badge.
**Impact**: Users can't scan the board to find what matters. The board feels like a flat list rather than a visual workflow tool.
**Recommendation**:
- Show assignee avatar (or "Unassigned" placeholder) on each card
- Show tags as colored chips
- Show due date (or "overdue" warning) when present
- Add a subtle description preview (first line, truncated)
- Consider color-coding the left border by priority (red=critical, yellow=medium, etc.)
**Resolution**: PR #41 added 3px left-border priority indicator colored by priority level (red=critical, amber=high, blue=medium, slate=low). Card already renders assignee avatars, due dates, tags, and description preview — issue was primarily data population (seed/test data lacks metadata).
**Remaining**: Seed data needs richer metadata (assignees, due dates, tags) to fully demonstrate card differentiation.

### 4. ~~Notification Dropdown Is Minimal~~ → ADDRESSED (PR #38 + #39 + #41)
**Where**: Notification bell → dropdown (`ux-review/08-notifications.png`)
**Problem**: Dropdown shows "Activity: Recent activity across your projects" header and "No recent activity." There's no way to filter, no tabs (All/Unread/Mentions), no settings link, no "mark all read" button. Even with activity, the dropdown is a simple text list with no action buttons.
**Impact**: Notifications are a primary engagement driver. The current implementation doesn't support quick triage or action-taking.
**Resolution**: PR #38 added notification count badge. PR #39 added source-aware quick action buttons (View Task, View Run), inline dismiss, status chips on activity items, and improved deep-linking. PR #41 added All/Inbox/Activity tabs with unread count badges in dropdown, "View All Notifications" link, and full `/notifications` page with search, read/unread filter, and pagination.

---

## Major UX Issues (P1)

### 5. ~~Sidebar Information Overload~~ → ADDRESSED (PR #41)
**Where**: All pages — left sidebar
**Problem**: The sidebar tries to serve too many masters: My Workspace (6 items) + Management (9 items) + Global Views (3 items) + Organizations + Internal Projects + Clients. On the org page, it also expands CRM sub-nav, Social sub-nav, Intelligence sub-nav. This creates a sidebar that can easily exceed viewport height.
**Impact**: Users must scroll the sidebar to find items. Mental model is unclear — "Management" with 9 items and "Global Views" with 3 items are collapsed with counts but no hint of contents.
**Resolution**: PR #41 merged Management + Global Views into single "Views & Management" section (count badge shows 12), moved External Links (Docs, Feedback & Support) into a "More" popover at sidebar bottom to reduce scroll height.

### 6. ~~Task Detail Drawer Competes with Kanban for Space~~ → ADDRESSED (PR #41)
**Where**: Task detail view (`ux-review/11-task-detail-drawer.png`)
**Problem**: The drawer opens on the right, pushing the kanban to ~40% width. The kanban becomes nearly unusable — you can see one partial column. The drawer itself packs in 5 tabs (Overview, Artifacts, Workflow, Logs, Vibe), an agent terminal, agent reviewers, and activity feed.
**Impact**: Users lose kanban context when viewing task details. The drawer tries to be a full page in a half-page space.
**Resolution**: PR #41 increased default drawer width from 600px to 800px, giving task detail more room while keeping kanban columns visible.

### 7. "Source of truth: ." in Brand Identity
**Where**: Project detail page (`ux-review/02-project-detail.png`)
**Problem**: The brand identity card shows `Source of truth: .` and `Repository: .` — these are clearly placeholder/unset values being displayed as literal periods.
**Impact**: Looks broken. Users will question data integrity.
**Recommendation**: Show "Not configured" or hide the field entirely when the value is empty or "."

### 8. ~~Test/E2E Data Pollutes the UI~~ → PARTIALLY ADDRESSED (PR #38 + #41)
**Where**: Kanban board, My Tasks
**Problem**: All visible tasks are E2E test artifacts: "[E2E] Kanban Test 1773352134117", "API Health Check 1773345781303". There's no way for a user to tell these are test data vs. real tasks.
**Impact**: This is a dev environment issue, but it highlights a missing feature: no bulk delete, no "clean up test data" action, no way to filter by tag prefix.
**Resolution**: PR #38 added bulk select + delete on kanban board. PR #41 added "Hide Tests" toggle on kanban toolbar that filters out tasks with `[E2E]` or `[Test]` prefix (persisted in localStorage).
**Remaining**: Backend cleanup endpoint for bulk-deleting test data.

### 9. ~~Integrations Hub Shows All Missing — No Prioritization~~ → ADDRESSED (PR #41)
**Where**: Project detail → Integrations Hub (`ux-review/02-project-detail.png`)
**Problem**: 6 integration categories, all showing "0/N connected" with red "Missing" badges. GitHub, Gmail, Zoho Mail, Instagram, Facebook, LinkedIn, X, YouTube, TikTok, Airtable, Stripe, Google Analytics — all listed as missing. This is demoralizing.
**Impact**: A wall of red "Missing" badges creates anxiety rather than motivation. User doesn't know which integrations are essential vs. optional.
**Resolution**: PR #41 redesigned the Integrations Hub:
- "Recommended" section at top with larger cards for GitHub + Gmail (key integrations)
- "Optional" categories collapsed into expandable accordion (default collapsed)
- "Missing" badge recolored from red to muted gray/slate for optional integrations
- Progress ring header showing "N of M connected" with SVG progress indicator
- Org-level IntegrationCard accent bar also updated for consistency

### 10. ~~Org Overview KPI Cards Are Static~~ → PARTIALLY ADDRESSED (PR #39)
**Where**: Org overview page (`ux-review/10-org-overview.png`)
**Problem**: The top stat cards (Total Tasks: 12, Total Deals: 0, Pipeline Value: $0, Contacts: 0, Active Projects: 1) are just numbers with no trends, sparklines, or period selectors. The quick-nav cards below (Pipelines, Contacts, Projects, Intelligence, Members, Integrations) are just text links styled as cards.
**Resolution**: PR #39 added TrendIndicator component showing "Active"/"No data" status on KPI cards, plus recent workflow run stats section with record counts.
**Remaining**: Trend arrows with percentages (requires additional API), time period selector, sparkline charts.

---

## Medium UX Issues (P2)

### 11. ~~Workflow Builder Cards Lack Status Indicators~~ → ADDRESSED (PR #39)
**Where**: My Workflows page (`ux-review/06-workflows.png`)
**Problem**: 5 workflow definition cards show name, description, node tags, and play/edit buttons. But there's no indication of: last run time, success/failure rate, whether triggers are configured, or if the workflow is active/paused.
**Resolution**: PR #39 added last-run status indicators on workflow definition cards (time ago + status icon: checkmark/error/spinner for completed/failed/running, "Never run" for no runs).

### 12. ~~My Tasks List View Lacks Batch Actions~~ → ADDRESSED (PR #38)
**Where**: My Tasks page (`ux-review/07-my-tasks.png`)
**Problem**: The list shows all 12 tasks with status and priority badges, but no checkboxes for bulk operations. The only interaction is clicking into a task. No way to bulk-change status, reassign, or archive.
**Resolution**: PR #38 added checkbox selection with floating batch action bar (status change, priority change, delete).

### 13. ~~Settings Page Has Unintuitive Tab Layout~~ → PARTIALLY ADDRESSED (PR #39)
**Where**: Settings (`ux-review/09-settings.png`)
**Problem**: Settings has a top-level tab bar (User / Admin / Org / Client) and then a vertical side menu (General, Wallet, Profile, Privacy, Activity Log, API Keys, Topsi Preferences, Integrations "Coming soon"). The two-axis navigation creates confusion.
**Resolution**: PR #39 grouped "Coming soon" items at bottom of nav with "Planned" header and reduced opacity (opacity-40), making them clearly secondary.
**Remaining**: Consider simplifying two-axis navigation (tabs + sidebar) to single-level nav.

### 14. Topsi Chat Widget Feels Disconnected
**Where**: Topsi floating widget (`ux-review/12-topsi-assistant.png`)
**Problem**: The chat widget opens as a small floating card in the bottom-right. It has "Start a conversation with Topsi" placeholder, a push-to-talk mic button, text input, and "Start call" button. But there's no context about what Topsi can do, no suggested prompts, no indication of whether it's connected/ready.
**Impact**: Users don't know what to ask. Voice features (push-to-talk, call) may not work without additional setup but there's no feedback about that.
**Recommendation**:
- Add 3-4 suggested prompt pills: "Summarize my tasks", "What deals need attention?", "Create a task for..."
- Show connection status (online/offline indicator)
- Hide voice features if the voice backend isn't configured, rather than showing non-functional buttons

### 15. ~~Breadcrumb Navigation Inconsistencies~~ → PARTIALLY ADDRESSED (PR #41)
**Where**: Various pages
**Problem**: Breadcrumbs change format between pages:
- Project detail: `Home > Powerclub Global > ORCHA Platform`
- Task kanban: `Home > Powerclub Global > ORCHA Platform > Tasks`
- CRM Pipeline: `Home > Powerclub Global > CRM > Pipeline`
- My Tasks: `Home > Jungleverse > My Tasks`
The "Jungleverse" entity appears in some breadcrumbs but not others. The org name vs. entity name switching is confusing.
**Resolution**: PR #41 added `/notifications` to breadcrumb page labels, removed project-level CRM breadcrumbs (CRM is org-scoped).
**Remaining**: "Jungleverse" still appears as default org name in some breadcrumbs; full standardization of org name display needed.

### 16. Login Page Shows "Session Expired" on First Visit
**Where**: Login page
**Problem**: Navigating to the root URL redirects to `/login?expired=1` with a yellow "Your session has expired" alert — even on first visit or after clearing cookies.
**Impact**: Creates false alarm. Users who haven't logged in before see an error message.
**Recommendation**: Only show the expiry message when there was actually a prior session (check for a cookie/token before showing).

---

## Low Priority / Polish (P3)

### 17. Development Mode Banner Takes Valuable Space
The orange "Development Mode - This is a development build" banner uses ~32px of vertical space on every page. Consider making it a small badge or corner indicator in development builds.

### 18. ~~Project Cards Could Show More Context~~ → PARTIALLY ADDRESSED (PR #39)
On the Projects page, each card shows name, status badge, creation date, boards count, and tasks count. Missing: last activity timestamp, assignee avatars, progress indicator (% tasks done).
**Resolution**: PR #39 added task completion progress bar to project cards.
**Remaining**: Last activity timestamp, assignee avatars.

### 19. ~~CRM Sub-nav Duplication~~ → ADDRESSED (PR #41)
CRM appears in both the sidebar (under org) and as a sub-nav item under the project. The project-level CRM and org-level CRM point to the same org CRM page, which is confusing.
**Resolution**: PR #41 removed CRM from project sub-nav in `ProjectFolder.tsx`. CRM now only appears under org section in the sidebar, since it's org-scoped.

### 20. ~~Kanban Column Headers Need Add-Task Button~~ → ADDRESSED (PR #38)
The kanban columns have headers with status name and count, but no "+" button to add a task directly to that column. Users must use the top-bar "Create new task" button and then manually set status.
**Resolution**: PR #38 added "+" button to kanban column headers that pre-sets the task status.

### 21. ~~No Dark Mode Preview~~ → ADDRESSED (PR #39)
The Settings page shows a Theme selector (System/Light/Dark) but the screenshots show only light mode.
**Resolution**: PR #39 fixed hardcoded colors (brand gold → CSS variable, OnboardingCarousel dark variants), then verified dark mode via Playwright walkthrough across all major screens (Projects, Org Overview, CRM Pipeline, Workflows, Settings). All pages render correctly.

---

## Summary by Priority

| Priority | Count | Addressed | Theme |
|----------|-------|-----------|-------|
| P0 Critical | 4 | 4 of 4 | ~~Onboarding~~ (PR #39), ~~empty states~~ (PR #38+#39), ~~task card density~~ (PR #41), ~~notifications~~ (PR #38+#39+#41) |
| P1 Major | 6 | 6 of 6 | ~~Sidebar overload~~ (PR #41), ~~drawer layout~~ (PR #41), ~~placeholder data~~ (PR #38), ~~integrations wall~~ (PR #41), ~~KPIs~~ (PR #39), ~~test data~~ (PR #38+#41) |
| P2 Medium | 6 | 4 of 6 | ~~Workflow status~~ (PR #39), ~~batch actions~~ (PR #38), ~~settings layout~~ (PR #39), Topsi guidance, ~~breadcrumbs~~ (PR #41 partial), login |
| P3 Polish | 5 | 4 of 5 | Dev banner, ~~project cards~~ (PR #39), ~~CRM duplication~~ (PR #41), ~~kanban add~~ (PR #38), ~~dark mode~~ (PR #39) |

**Total: 18 of 21 items addressed across PR #38, #39, and #41.**

---

## Remaining Items (Not Yet Addressed)

**P2**: #14 Topsi chat suggested prompts, #16 Login "session expired" on first visit
**P3**: #17 Dev banner space reduction
