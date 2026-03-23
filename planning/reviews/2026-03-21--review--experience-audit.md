# Experience Audit Report

**Date**: 2026-03-21
**Scope**: Tier 1 Phase 0 sprint features — AI Usage, Friction Reporting, Agent Flows, CRM Pipeline + general IA/navigation
**Branch**: `feature/2026-03-19--tier1-phase0`
**Viewport**: Desktop (1440×900) + Mobile (375×812)

## Executive Summary

The app is functionally rich but overwhelms new users — the sidebar alone has 50+ items across 8 groups, all expanded by default. Sprint-specific features (AI Usage, friction reporting, agent flows) are buried behind 3+ clicks in non-obvious locations. Interactive testing revealed strong execution quality: **friction report submission works end-to-end** (form → submit → toast → closes cleanly), and **CRM deal creation is excellent** (create → instant kanban update → data persists after navigation → pipeline totals update). The weakest journey is AI Usage which shows all zeros (old API still wired). The CRM Pipeline sets the gold standard — other pages should follow its patterns.

## Findings by Severity

### BLOCKERS

None found. All journeys can be completed.

### PAIN POINTS

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| P1 | Dashboard / sidebar | **Sidebar has 50+ items across 8 groups, all org blocks expanded by default.** 5 organizations × 6 sub-items each = 30 org items alone. Violates Miller's law (7±2). New user would be paralyzed. | `ux-audit-dashboard.png` | Collapse all org blocks except the currently active one. Add a "collapse all" control. Consider progressive disclosure — show only active org's sub-items until user expands others. | STRUCTURAL |
| P2 | AI Usage journey | **"Views & Management" is a dumping ground.** 12 unrelated items (All Tasks, CRM Admin, Discord Voice, AI Usage) under one label. AI Usage is item 12 of 12 — a user looking for cost data would never think to look here. | `ux-audit-ai-usage.png` | Split into logical groups: "Admin Views" (All Tasks, CRM Admin, All Social), "Business" (Proposals, Invoices, Reports), "Tools" (Command Center, Discord, AI Usage). Or promote AI Usage to My Workspace. | INVESTMENT |
| P3 | AI Usage page | **Dashboard shows all zeros** — still reads from old `tokenUsageApi` instead of new `costsApi`. User navigates to AI Usage expecting cost breakdown and finds nothing useful. Sprint marked this DONE but frontend was never wired. | `ux-audit-ai-usage.png` | Wire `costsApi.orgSummary()`, `.orgByModel()`, `.orgByProject()` into the AI Usage page components. (Also tracked in functionality audit as Must Fix.) | QUICK WIN |
| P4 | Every page | **Notifications API returns 500 on every page load.** Console shows `Failed to load resource: /api/notifications/inbox` error. No user-facing indication, but indicates a broken backend endpoint and pollutes console — masks real errors during development. | — | Fix the notifications endpoint or add a graceful fallback that doesn't spam 500s. At minimum, don't retry failed notification fetches every page navigation. | QUICK WIN |

### FRICTION

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| F1 | Agent Flows | **Agent Flows hidden behind 3 clicks**: Workflows → tab bar "More" dropdown → "Agent Flows". The "More" button looks like an action button, not a tab overflow indicator. No visual hint that more tabs exist. | `ux-audit-workflows.png` | Either: (a) add "Agent Flows" as a visible tab, or (b) add a count indicator to the "More" button (e.g., "More +3") so users know there's hidden content. | QUICK WIN |
| F2 | Friction report | **Friction reporting requires 3 clicks**: sidebar More → Feedback & Support → select "Friction Report" type. No keyboard shortcut, no persistent entry point. For a dogfooding tool, friction reporting should be effortless. | — | Add a keyboard shortcut (e.g., `Shift+F`) that opens the feedback dialog directly in friction mode. Or add a small "Report Issue" button in the bottom bar next to Topsi. | QUICK WIN |
| F3 | Dashboard | **No home/dashboard view.** Landing page goes straight to Projects list. No activity feed, no "here's what needs attention today", no pending tasks summary. A user returning after a day doesn't know what changed. | `ux-audit-dashboard.png` | Create a dashboard home page with: recent activity, assigned tasks requiring action, agent flow status, and a "quick actions" card. Projects list becomes a sub-page. | INVESTMENT |
| F4 | Navigation | **Breadcrumb inconsistency.** On Projects page: shows "Home > Jungleverse" (no page name). On CRM Pipeline: shows "Jungleverse > CRM > Pipeline" (correct). User can't always tell where they are. | — | Always include the current page name as the last breadcrumb segment. | QUICK WIN |
| F5 | Mobile (375px) | **"Create Project" button overlaps description text.** The button and subtitle "Manage your projects and track their progress" compete for the same horizontal space. | `ux-audit-mobile-dashboard.png` | Stack the button below the subtitle on mobile, or move it to a floating action button. | QUICK WIN |
| F6 | Sidebar | **Empty org sections expand with just "+" buttons.** "Internal Projects" and "Clients" sections show expanded but empty for every org, creating visual noise (10 empty expandable sections across 5 orgs). | `ux-audit-dashboard.png` | Collapse empty sections by default. Only show expand arrow if the section has content. Or hide empty sections entirely with a "Show empty sections" toggle. | QUICK WIN |
| F7 | CRM Pipeline | **10 tabs in content area truncate on smaller viewports.** Last tabs show as "Integr..." with no way to see the full name without hovering. | `ux-audit-crm-pipeline.png` | Use a scrollable tab bar with left/right arrows, or group less-used tabs (Wiki, Cloud, Integrations) into a "More" overflow. | QUICK WIN |
| F8 | Workflow cards | **Icon-only action buttons** (play, edit, "...") on workflow cards have no visible labels or tooltips in the accessibility snapshot. User must hover to discover what each does. | `ux-audit-workflows.png` | Add `aria-label` to all icon-only buttons. The "Run workflow" and "Edit in visual builder" have labels but the third ("...") button does not. | QUICK WIN |

### POLISH

| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|
| PO1 | Sidebar | **"VIBELAND" and "VIBE" are jargon.** No tooltip, description, or icon context explaining what these are. A new user has no mental model for them. | — | Add tooltips: "VIBELAND — 3D virtual environment" and "VIBE — token & treasury management". | QUICK WIN |
| PO2 | Workflows | **All workflow cards show "Never run"** — correct empty state but no guidance on how to trigger. | — | Link "Never run" text to docs or add "Run your first workflow →" CTA on the first card. | QUICK WIN |
| PO3 | CRM Pipeline | **Excellent empty state** — "Start your pipeline: Add your first deal to start tracking prospects through your sales process" with orange "Add Deal" CTA. **This is the gold standard.** Other pages should follow this pattern. | `ux-audit-crm-pipeline.png` | Use as template for other empty states across the app. | — |
| PO4 | All pages | **Topsi floating button always visible** in bottom-right. Could overlap content on narrow viewports or during scrolling. | — | Allow user to dismiss/minimize Topsi to a smaller indicator. Or auto-hide when scrolling. | POLISH |

## Journey Scorecard

| Journey | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|------------|----------|------|----------|-------|
| J1: View AI spending | 4 clicks (expand Views & Mgmt → AI Usage) | Page renders but shows $0 — old API still wired | 0 | 2 (P2, P3) | 0 | **D** — finds page but data is wrong |
| J2: Report friction | 3 clicks + 5 form fields + submit | **FULLY TESTED**: Selected friction type → filled all fields (intent, friction, expected, frustration=4) → filled title+description → submitted → toast "Thank you!" → dialog closed. Validation: submit disabled when required fields empty. | 0 | 0 | 1 (F2) | **A** — works perfectly, just buried |
| J3: Monitor agent flows | 3 clicks (Workflows → More → Agent Flows) | Empty state correct ("No Agent Flows"). Cannot test interaction — no create button, no seed data. | 0 | 0 | 1 (F1) | **B** — untestable without data |
| J4: CRM pipeline | 2 clicks + deal creation + persistence check | **FULLY TESTED**: Created "UX Audit Test Deal - Acme Corp" ($25k) → instant kanban card with "Research needed" badge → 2 toasts ("Deal created" + "Deal added to pipeline") → pipeline header updated to "1 deals $25,000" → navigated away → came back → deal persisted. Validation: submit disabled without name. | 0 | 0 | 1 (F7) | **A** — excellent end-to-end |
| General IA | — | — | 0 | 1 (P1) | 3 (F3, F4, F6) | **C** — overwhelming sidebar |

## Quick Wins (do first)

Ranked by impact-to-effort ratio:

1. **P4** — Fix notifications endpoint 500 or add retry backoff — 1-2 hours
2. **F6** — Collapse empty org sections (Internal Projects/Clients) by default — 30 min
3. **F4** — Add current page name to breadcrumb on all pages — 1 hour
4. **F1** — Add count indicator to Workflows "More" button or promote Agent Flows to visible tab — 30 min
5. **F2** — Add keyboard shortcut (Shift+F) to open feedback dialog in friction mode — 1 hour
6. **PO1** — Add tooltips to VIBELAND and VIBE sidebar items — 15 min
7. **F5** — Stack "Create Project" button below subtitle on mobile — 30 min
8. **F8** — Add aria-labels to icon-only workflow card buttons — 30 min
9. **F7** — Add scrollable overflow to CRM tab bar — 1-2 hours

## Investments (plan for)

1. **P3** — Wire `costsApi` into AI Usage dashboard — 1-2 days — Users currently see $0 for all costs; this was the main deliverable of the cost bridge sprint item
2. **P2** — Restructure "Views & Management" into logical sub-groups — 2-3 days — Current dumping ground makes 12 features unfindable
3. **F3** — Create a proper dashboard/home page with activity feed — 3-5 days — Transforms the first-login experience from "wall of projects" to "here's what matters today"

## Structural Recommendations (roadmap discussion)

1. **Sidebar information architecture** — The sidebar tries to be everything: workspace nav, org switcher, org drill-down, admin tools, and settings. With 5 orgs × 6 sub-items + 3 nav groups + 12 management views, it's ~50 items. **What good looks like**: One active org context at a time (like Slack workspaces), with a lightweight org switcher. Non-active orgs collapse to just their name. Admin/management views move to a separate admin panel or top-level route. — Scope: 1-2 week redesign
2. **Empty state consistency** — CRM Pipeline has the best empty state in the app (contextual message + CTA). Most other pages (Projects, Workflows, Agent Flows) show minimal empty states ("No items"). **What good looks like**: Every empty state follows the CRM Pipeline pattern — icon, contextual message explaining what belongs here, and a primary CTA to create the first item. — Scope: 2-3 days to template + apply

## Accessibility Summary

| Check | Status | Issues |
|-------|--------|--------|
| Keyboard navigation | PARTIAL | ⌘K command palette works. ⌘B collapse works. Tab order not fully tested. |
| Mobile responsive | PASS | Sidebar collapses to hamburger. Content stacks. Touch targets adequate. Minor button overlap on Projects page (F5). |
| Color/contrast | PASS | Status badges use color + text labels. Pipeline stages use colored dots + text. Dark mode not tested this session. |
| Screen reader basics | PARTIAL | Most buttons labeled. Icon-only workflow card buttons lack aria-labels (F8). Form fields have labels. |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | 2/5 pages tested have good empty states | CRM Pipeline: excellent (icon + message + CTA). Agent Flows: acceptable ("No Agent Flows"). AI Usage: shows zeros not "no data" — misleading. Projects: cards show "0 boards · 0 tasks" — adequate. Workflows: "Never run" — adequate but no CTA. |
| Loading states | All pages show spinner | Generic spinner with "Loading..." text. No page-specific skeletons. Content flashes from spinner → full page (layout shift). |
| Error states | Not tested (no errors triggered) | Notifications endpoint returns 500 silently — no user-facing error. Error boundary exists but not triggered during testing. |
| Validation feedback | Feedback dialog: Submit button disabled until required fields filled | Good pattern — inline validation with character counter (0/200). |
