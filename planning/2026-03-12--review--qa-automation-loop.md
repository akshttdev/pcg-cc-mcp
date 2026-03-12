# Project, Task & Automation Capability Review

**Date:** 2026-03-12
**Method:** Browser-based QA via Playwright Firefox MCP (accessibility snapshots)
**Scope:** Project creation, task management UI, workflow builder, MCP task server, AI agent automation
**Reviewer perspective:** Expert UI designer + Expert app architect

---

## Executive Summary

The platform has a rich feature surface for project management, task creation, workflow orchestration, and agent automation. However, critical gaps in the end-to-end automation flow — particularly around VIBE budget blocking, task-to-agent handoff visibility, and workflow discoverability — prevent users from completing the core loop of "create project → assign tasks → automate with agents."

---

## 1. USABILITY GAPS (UI/UX Design)

### 1.1 [P0] No Viable Path to Fund VIBE Wallet in Dev Mode
- **Location:** Settings → Wallet (`/settings/wallet`)
- **Issue:** VIBE balance shows **0 VIBE** on Testnet. "Send" is disabled, "Receive" and "Buy VIBE" buttons exist but require external blockchain interaction. Agent task execution fails with insufficient balance error, with no dev-mode bypass or faucet.
- **Impact:** Completely blocks the agent automation loop in local development.
- **Recommendation:** Add a "Dev Faucet" button (visible only in dev mode) that credits test VIBE tokens, or add a `SKIP_VIBE_CHECK` env var for local development.

### 1.2 [P0] Task Detail Panel Missing Description, Completion Criteria & Output Format
- **Location:** Task detail Overview tab (`/projects/:id/tasks/:taskId`)
- **Issue:** The Overview tab shows Recent Artifacts, Agent Terminal, and Recent Activity — but no **Description**, **Completion Criteria**, or **Output Format** fields. These are the primary inputs an agent needs to understand what to do.
- **Impact:** Users set completion criteria in the task form, but can't verify them after creation. Agents have no visible context about what "done" looks like.
- **Recommendation:** Add Description, Completion Criteria, and Output Format sections at the top of the Overview tab, above Recent Artifacts. The `EnhancedTaskDetailsPanel` has these fields but they're only shown in Enhanced view mode, not in the default view.

### 1.3 [P1] Project Detail Page Has No Direct Tasks Tab
- **Location:** Project page (`/projects/:id`)
- **Issue:** The project detail shows Overview, Brand Identity, Integrations Hub, Boards, Assets sections — but no "Tasks" tab or prominent link. Users must know to navigate to `/projects/:id/tasks` via URL or sidebar.
- **Impact:** The most common action (view/manage tasks) requires non-obvious navigation.
- **Recommendation:** Add a "Tasks" tab to the project detail page, or at minimum a prominent "View Tasks" card/link in the Overview section.

### 1.4 [P1] Task Cards Don't Show Agent Assignment
- **Location:** Kanban board task cards
- **Issue:** Task cards show title, priority badge, and board tag — but NOT which agent is assigned. Users managing multiple agent-delegated tasks can't see at a glance who's working on what.
- **Impact:** Reduces visibility into agent workload distribution.
- **Recommendation:** Add a small agent avatar/name badge to task cards when `assigned_agent` is set.

### 1.5 [P1] My Tasks Page Renders Blank
- **Location:** `/my-tasks`
- **Issue:** Page loads with sidebar and header but the main content area is empty — no empty state message, no loading indicator after initial load.
- **Impact:** Users navigating to "My Tasks" see a broken-looking page.
- **Recommendation:** Show an empty state ("No tasks assigned to you yet") or fix the query that populates this view.

### 1.6 [P2] Template Table Missing New Fields
- **Location:** Settings → General → Task Templates section
- **Issue:** Template list table only shows Template Name, Title, Description columns. The 6 new fields (Priority, Completion Criteria, Output Format, Assigned Agent, Tags) are hidden — only visible when editing.
- **Impact:** Users can't compare templates at a glance or see which templates have agent assignments.
- **Recommendation:** Add at minimum Priority and Assigned Agent columns to the table, or show a summary badge row.

### 1.7 [P2] MCP Carousel Shows Only 3 of 4 Servers
- **Location:** Settings → MCP Servers
- **Issue:** Popular servers carousel shows Duck Kanban, Context7, Playwright — but not the 4th server (ORCHA). Previous/Next buttons are disabled, suggesting the carousel thinks all items are visible.
- **Impact:** ORCHA MCP server, a key platform integration, is hidden.
- **Recommendation:** Verify `default_mcp.json` has 4 entries and fix carousel item count / responsiveness to show all items.

### 1.8 [P2] Workflow Triggers Panel Missing Schedule Option in UI
- **Location:** Workflow Builder → triggers configuration
- **Issue:** While the WorkflowTriggersPanel code now supports `schedule` type with interval selectors, there's no way to access it from the workflow editor — the triggers panel isn't exposed in the Builder tab UI.
- **Impact:** Users can't create or manage schedule triggers from the workflow editor.
- **Recommendation:** Add a "Triggers" section/button to the workflow editor that opens the WorkflowTriggersPanel.

### 1.9 [P3] Agent Terminal UX Friction
- **Location:** Task detail → Agent Terminal
- **Issue:** The agent terminal input field says "Message the agent..." with a disabled send button. There's no indication of what agents can do, what message format to use, or that VIBE tokens are required before sending.
- **Impact:** Users type messages and hit a confusing VIBE error with no recovery path.
- **Recommendation:** Show agent capabilities inline, display VIBE cost estimate before sending, and add a helpful placeholder that sets expectations.

### 1.10 [P3] i18next Missing Keys Flooding Console
- **Location:** All Settings pages
- **Issue:** Dozens of `i18next::translator: missingKey` console warnings on every settings page load, indicating untranslated keys in the `settings` namespace.
- **Impact:** Console noise, potential UI showing raw translation keys instead of labels.
- **Recommendation:** Add the missing translation keys to the i18n settings namespace, or suppress warnings for the settings namespace.

---

## 2. FUNCTIONALITY GAPS (Architecture)

### 2.1 [P0] System Workflows Not Seeded in Dev Database
- **Location:** Workflows page (`/workflows`)
- **Issue:** Only 1 workflow visible (Data Source Analysis). The 4 system workflows (Bug Triage Pipeline, Sprint Planning, Client Onboarding, Content Pipeline) seeded by migration `20260325000000` are not present in the running dev database.
- **Root cause:** Migration was created but not applied to `dev_assets/db.sqlite`. The seed DB in `dev_assets_seed/` also doesn't have them.
- **Impact:** Users see a nearly-empty workflow builder with no templates to start from.
- **Fix:** Run `sqlx migrate run` against the dev database, AND update `dev_assets_seed/duck_kanban.db` to include the seeded workflows.

### 2.2 [P0] Agent Execution Blocked by VIBE Balance Check
- **Location:** Task attempt creation flow
- **Issue:** When a user assigns an agent to a task and sends a message, the backend checks VIBE balance and rejects with insufficient funds. In dev/local mode, there's no way to acquire VIBE tokens.
- **Impact:** The entire agent automation pipeline is non-functional in development.
- **Recommendation:** Add a server-side `BYPASS_VIBE_CHECK=true` env var for development, or implement a testnet faucet endpoint.

### 2.3 [P1] WebSocket Connection Failures
- **Location:** All pages with real-time features
- **Issue:** Consistent WebSocket errors: "Firefox can't establish a connection to the server at ws://..." from `useExecutionEvents.ts` and `useJsonPatchWsStream.ts`.
- **Root cause:** Backend WebSocket endpoints may not be running, or the Vite proxy isn't forwarding WS connections.
- **Impact:** No real-time updates for task status changes, execution events, or Mission Control live coordination.
- **Recommendation:** Verify WebSocket proxy configuration in `vite.config.ts` (needs `ws: true` on proxy), and ensure backend WebSocket handlers are registered.

### 2.4 [P1] Workflow Editor "New Workflow" Creates Only Default Analysis
- **Location:** Workflows → Builder → New Workflow
- **Issue:** Only 1 workflow type available. No ability to start from system workflow templates (Bug Triage, Sprint Planning, etc.) even if they were seeded.
- **Impact:** Users must build workflows from scratch with no guidance.
- **Recommendation:** Add a "Create from Template" flow that lists system workflows as starting points, or show system workflows in the Builder list with a "Clone" action.

### 2.5 [P1] No Task-to-Workflow Linking
- **Location:** Task detail → Workflow tab
- **Issue:** The Workflow tab exists on the task detail panel but there's no visible way to connect a task to a workflow — no dropdown, no search, no "Run workflow on this task" button.
- **Impact:** Workflows and tasks are siloed. Users can't trigger a workflow from a task context.
- **Recommendation:** Add a "Run Workflow" action on tasks that lets users select a workflow definition and execute it with the task as context.

### 2.6 [P2] Agent Profile API Returns Zeros
- **Location:** Agent detail dialog (Settings → Agents → click agent)
- **Issue:** Execution Stats show 0/0/0/— for all agents. The `/api/agents/:id/profile` endpoint returns data but with zero stats because no task attempts have been executed (blocked by VIBE).
- **Impact:** Agent profiles look unpopulated and don't demonstrate the system's capabilities.
- **Recommendation:** Seed some historical execution data in dev database for demo purposes.

### 2.7 [P2] Task Template "Assigned Agent" is Free Text
- **Location:** Template edit dialog → Assigned Agent field
- **Issue:** The Assigned Agent field is a plain text input (`e.g., Nora, Maci, Editron`) rather than a dropdown populated from the agents API.
- **Impact:** Typos in agent names will silently fail. Users must know agent names.
- **Recommendation:** Replace with a dropdown/combobox populated from `GET /api/agents` with search.

### 2.8 [P2] Automations Tab Not Editable
- **Location:** Workflows → Automations tab
- **Issue:** Shows 5 automation rules (Waiting-on-Client Follow-up, Churn Overdue Leads, etc.) as read-only cards. No create, edit, enable/disable, or delete actions.
- **Impact:** Users can see automations but can't modify or create new ones.
- **Recommendation:** Add CRUD actions for automations: create new, toggle enable/disable, edit trigger conditions, delete.

### 2.9 [P2] Project Scaffolding Templates Don't Set Completion Criteria
- **Location:** Project creation → template scaffolding
- **Issue:** The 4 project templates (Software Dev, Research, Marketing, Client Onboarding) create 5 tasks each, but scaffolded tasks don't have completion_criteria or output_format set.
- **Impact:** Agent-assigned tasks from templates have no success criteria for self-evaluation.
- **Recommendation:** Update `ProjectFormDialog` scaffolding to include completion_criteria and output_format for each template task.

### 2.10 [P3] Mission Control Empty State Lacks Guidance
- **Location:** `/mission-control`
- **Issue:** Shows "No active executions" with "Agent runs will appear here in real time." but no guidance on how to trigger an execution.
- **Impact:** New users don't know how to get started with agent automation from this page.
- **Recommendation:** Add a "Quick Start" card with steps: 1) Create a task, 2) Assign an agent, 3) Send a message to trigger execution.

---

## 3. POSITIVE FINDINGS (What Works Well)

- **Agent Directory** (Settings → Agents): 13 agents with rich profiles, search/filter/sort, avatar images. Agent detail dialog shows cost estimation, personality traits, capabilities, tools, and execution stats.
- **Workflow Editor**: Visual node builder with configurable parameters, prompt templates with variable insertion (`{{content}}`), model override per node, input connection management.
- **Task Form**: Comprehensive fields including Completion Criteria, Output Format, Agent Assignment dropdown with all 13 agents, MCP server assignment, Tags, Images, Template selector.
- **Project Templates**: 4 scaffolding templates that auto-create 5 tasks each — great onboarding experience.
- **Kanban Board**: Clean 5-column layout (To Do, In Progress, In Review, Done, Cancelled) with filter, priority sort, import/export, enhanced view toggle.
- **Settings Architecture**: Well-organized sidebar with 14 settings categories, each with clear descriptions.
- **Automations Display**: 5 pre-built automation rules with clear trigger/action descriptions.

---

## 4. RECOMMENDED PRIORITY ORDER

### Immediate (blocks core automation loop)
1. Dev VIBE faucet or bypass (`BYPASS_VIBE_CHECK`)
2. Apply system workflow seed migration to dev DB
3. Show Description/Completion Criteria/Output Format in default task detail view

### Short-term (improves usability)
4. Add Tasks tab to project detail page
5. Show agent assignment on kanban task cards
6. Fix My Tasks empty state
7. Fix WebSocket proxy for real-time updates

### Medium-term (enhances automation capability)
8. Task-to-workflow linking ("Run Workflow" action on tasks)
9. Agent assignment dropdown in template editor (replace free text)
10. Workflow trigger management in editor UI
11. Automations CRUD
12. Template scaffolding with completion criteria

### Polish
13. Fix MCP carousel to show all 4 servers
14. Mission Control quick-start guidance
15. Suppress i18next missing key warnings
16. Seed demo execution data for agent profiles
