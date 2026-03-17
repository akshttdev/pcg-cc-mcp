# Demo Workflow UX Audit — Usability & Functionality Gaps

**Date**: 2026-03-16
**Type**: review
**Method**: Live Playwright MCP walkthrough + E2E demo spec code review + UI component code analysis
**Screenshots**: `ux-review/01-14*.png`
**Demos reviewed**: `workflow-crm-pipeline`, `workflow-spanish-pipeline`, `pipeline-intelligence-workflow`, `bug-report-lifecycle`, `manual-qa-trigger`, `notification-center`, `dogfood-workflows`, `workflow-pipeline`

**Related**: See `2026-03-16--review--ux-design-audit.md` for general platform UX findings (21 items, 20 addressed via PRs #38, #39, #41). This document focuses exclusively on **demo workflow UX** — the workflow builder, staging review, run management, data source → workflow → CRM flow, and demo script observations. No overlap with the prior audit.

---

## Executive Summary

The demo workflows showcase powerful capabilities (LLM extraction, CRM pipeline, agent-driven QA, staging review) but the **user experience connecting these capabilities is fragmented, unintuitive, and relies on users already understanding the system's mental model**. A first-time viewer watching these demos would struggle to understand: what workflows are, why they'd use them, how pieces connect, and what they should do next at each step.

The core issue is **the workflow system was built capability-first, not journey-first**. Each component works, but the connective tissue between them — navigation, feedback, progressive disclosure, contextual guidance — is weak or missing.

---

## CRITICAL FINDINGS (P0) — Demo-Blocking Issues

### 1. Workflow Builder: No Visual Connection Feedback
**Where**: New Workflow dialog → Graph view (`ux-review/06-graph-view.png`)
**Problem**: The graph view shows disconnected boxes stacked vertically. There are **no connection lines, arrows, or visual indicators of data flow between nodes**. The LLM Extract node shows "No inputs" even though Data Source exists above it. The user must manually open each node config, click "Add input connection...", and select from a dropdown — but the graph never reflects these connections visually.
**Impact**: The entire value proposition of a "visual workflow builder" is undermined. Users can't see the pipeline they're building. The e2e demo helpers (`addExtractNode`, `addOutputNode`) work around this by using the list view and dropdowns, but a human user would be lost.
**Recommendation**:
- Draw SVG connection lines between connected nodes in graph view
- Add drag-to-connect handles on node edges
- Show connection state (connected/disconnected) on each node
- Auto-connect new nodes to the selected node when added

### 2. Node Picker Has No Category Headers or Search
**Where**: Add Node panel (`ux-review/03-node-picker.png`)
**Problem**: 18 node types in a single scrollable list. Only two subtle gray "ACTIONS" and "OUTPUTS" section headers separate them. No search, no filtering, no visual grouping. A user scanning for "Output: CRM Contacts" must scroll past 14 items.
**Impact**: Node selection is slow and error-prone. Demo scripts use exact regex selectors (`/^Data Source Marks this/`) because there's no better way to target nodes.
**Recommendation**:
- Add a search/filter input at the top
- Group into collapsible sections: Input, Processing (LLM), Transform, Actions, Outputs
- Show which nodes are already in the workflow (grayed out or badged)
- Add "Recently used" section for repeat builders

### 3. Workflow ID Field Is Confusing and Error-Prone
**Where**: New Workflow dialog → Workflow ID field (`ux-review/02-new-workflow-dialog.png`)
**Problem**: Users must manually enter a "Workflow ID" (snake_case, auto-converted) separately from the display "Name". The placeholder `my_workflow` gives no guidance on naming conventions. The ID silently transforms input (e.g., `My Workflow` → `my_workflow`) as you type, which is disorienting. The ID cannot be changed after creation.
**Impact**: Technical debt UX — exposes internal identifiers to users who don't need them. Demo scripts generate IDs like `e2e_crm_extract_${Date.now()}` to avoid collisions — a real user would have no idea what to put here.
**Recommendation**:
- Auto-generate the ID from the name (like slug generation)
- Hide the ID field behind an "Advanced" toggle
- Show the generated ID as read-only text below the name field

### 4. "Run Workflow" Flow Is Split Across Multiple Disconnected Pages
**Where**: Data Source detail → Workflows section → Run → Staging tab
**Problem**: Running a workflow requires navigating to a data source detail page, selecting a workflow from a dropdown, clicking Run, then being redirected to `/workflows?tab=staging`. The user loses all context about which data source they were looking at. The staging tab shows records from *all* workflows, not just the one they ran.
**Impact**: The demo scripts navigate between 4+ different URL paths to complete one end-to-end flow. The `workflow-crm-pipeline` demo has 8 parts spanning data source creation, workflow building, execution, staging review, CRM verification — each on a different page.
**Recommendation**:
- Add "Run Workflow" button directly on workflow definition cards (already exists but opens a dialog that requires selecting a data source — reverse the entry point)
- After workflow execution, show staging results inline or in a contextual panel, not a redirect
- Add breadcrumb trail: "Data Source → Workflow Run → Staging Review" so user can navigate back
- Show which data source produced the staged records in the staging tab

### 5. Staging Tab Empty State Gives No Next Steps
**Where**: Workflows → Staging tab (`ux-review/08-staging-tab-empty.png`)
**Problem**: Shows "No pending records — Records extracted by workflow runs will appear here for review." with a clipboard icon. No link to run a workflow, no explanation of what staging is, no guidance on how to get records here.
**Impact**: A user exploring the workflows page would see this dead end and not understand the connection between running a workflow and seeing results here.
**Recommendation**:
- Add "Run a Workflow" CTA button in the empty state
- Add a brief explanation: "When you run a workflow against a data source, extracted records appear here for review before being committed to your CRM."
- Link to the Builder tab with a "Create your first workflow" option

---

## MAJOR FINDINGS (P1) — Significant Usability Gaps

### 6. Runs Tab Shows Raw IDs Instead of Data Source Names
**Where**: Workflows → Runs tab (`ux-review/07-runs-tab.png`)
**Problem**: Three runs show truncated UUIDs ("4131d8cb...", "7d7db6d9...", "8fba31ec...") instead of data source names. Console shows `[API Error] {message: Data source not found}` — the data sources were deleted after test cleanup but the run history still references them.
**Impact**: Users can't identify which data source a run processed. The demo cleanup deletes data sources but not runs, leaving orphaned references.
**Recommendation**:
- Store data source title snapshot in the `workflow_runs` record at execution time (not just ID)
- Show "(Deleted)" or "Unknown source" for orphaned references instead of raw UUID fragments
- Add data source name as a prominent column in the runs list

### 7. Data Source Detail → Workflow Section UX Is Dense
**Where**: Data source detail page → Workflows panel (`ux-review/11-data-source-detail.png`)
**Problem**: The workflows section shows: a workflow dropdown (defaulting to "Client Onboarding Pipeline"), a model dropdown ("Claude Sonnet 4.6"), Builder, Run History, and Run buttons all in one compact row. Below that, a static numbered list of the workflow's nodes. No explanation of what these controls do or how they relate.
**Impact**: Too many choices presented at once with no progressive disclosure. A user doesn't know whether to select a workflow first, what the model selector does, or what "Builder" vs "Run History" vs "Run" means in this context.
**Recommendation**:
- Default to showing a simpler "Run Workflow" action with workflow selection
- Move "Builder" link to the workflow name (as a link to the workflow editor)
- Move "Run History" to an expandable section below
- Add inline help text: "Select a workflow to extract structured data from this source"
- Show model selection only after workflow is selected, with explanation

### 8. Automations Tab Has No Create/Edit Actions
**Where**: Workflows → Automations tab (`ux-review/09-automations-tab.png`)
**Problem**: Shows 5 automation cards (Waiting-on-Client Follow-up, Churn Overdue Leads, etc.) but there's **no "Create Automation" button, no edit controls, no enable/disable toggles**. The cards are purely informational — read-only cards showing Trigger/Action pairs. The user can view but not interact.
**Impact**: The automations look like they could be configured but can't be. This creates a "glass ceiling" where the user sees capabilities but can't act on them.
**Recommendation**:
- Add "Create Automation" button in the header
- Add enable/disable toggle per automation card
- Add edit/delete actions to each card
- Add status indicator (active/paused/error)

### 9. Workflow Builder Node Config Panel Is Too Wide
**Where**: Workflow editor → Node config (`ux-review/05-llm-extract-node-config.png`)
**Problem**: The right panel (node config) takes ~65% of the dialog width. The left panel (node list) is compressed to ~35%, showing only small cards. When multiple nodes are added, the list scrolls but the config panel's prompt template textarea is enormous, with most space being empty placeholder text.
**Impact**: The layout prioritizes configuration over the workflow overview. Users lose sight of their workflow structure when configuring a single node.
**Recommendation**:
- Make the node config panel collapsible or resizable
- Start with a balanced 50/50 split
- Collapse the metadata header ("Untitled Workflow" + ID/Name/Description) after first save
- Use a drawer/slide-over for node config instead of a fixed panel

### 10. Demo Flow Requires Jumping Between Org Intelligence and Global Workflows
**Where**: Throughout all workflow demos
**Problem**: Data sources live under Organization → Intelligence → Data Sources. Workflows live under global My Workflows. CRM results live under Organization → CRM. The user must navigate between three completely separate navigation trees to complete one workflow:
1. Create data source at `/organizations/{id}/intelligence/data-sources`
2. Build workflow at `/workflows`
3. Run from data source detail at `/organizations/{id}/data-sources/{id}`
4. Review staging at `/workflows?tab=staging`
5. Verify CRM at `/organizations/{id}/crm/contacts`

**Impact**: The workflow journey is spread across 5+ page contexts with no persistent breadcrumb connecting them. Demo scripts hard-code navigation between these paths.
**Recommendation**:
- Add a "Workflow Hub" that combines: select data source → choose/build workflow → run → review → commit in a single guided flow
- Add cross-links: from staging records, link back to the source data and forward to the CRM entry
- Show a workflow run progress indicator that persists across navigation

### 11. Pipeline Board Column Headers Lack Context for New Users
**Where**: CRM Pipeline → Acquisition tab (`ux-review/12-crm-pipeline.png`)
**Problem**: The 8-stage pipeline (Lead → Business Analysis → Discovery → Build Proposal → Polish → Proposal Meeting → Closed Won → Closed Lost) shows role labels under each column ("Nora", "Account Manager", "Topsi + PM", "PM / EP") but no explanation of what each stage means, what happens there, or what the exit criteria are.
**Impact**: The pipeline intelligence demo (`pipeline-intelligence-workflow.spec.ts`) programmatically advances deals through stages, but a human user watching wouldn't understand what triggers the advancement or what "Business Analysis" vs "Discovery" means in practice.
**Recommendation**:
- Add hover tooltips explaining each stage's purpose and exit criteria
- Show stage-specific actions (e.g., "Research triggered automatically" for Business Analysis)
- Add a small info icon linking to pipeline documentation

---

## MEDIUM FINDINGS (P2) — Polish & Clarity Gaps

### 12. Prompt Template Placeholder Is Overwhelming
**Where**: LLM Extract node → Prompt Template textarea (`ux-review/05-llm-extract-node-config.png`)
**Problem**: The placeholder text is a 7-line multi-paragraph example covering contacts AND companies. It's too much text for a placeholder — users might think they need to write something equally complex.
**Recommendation**: Shorten to one line: "Describe what to extract (e.g., 'Extract all people with name, email, company')". Move examples to a "Show examples" expandable section.

### 13. Output Schema Field Has No Validation or Help
**Where**: LLM Extract node → Output Schema field
**Problem**: Freeform text field with placeholder "e.g. companies[], contacts[], opportunities[]". No validation, no explanation of syntax, no link to available schemas. User could type anything.
**Recommendation**: Make it a dropdown of available target types (CRM Contacts, Companies, Deals, Tasks) or a structured builder.

### 14. Graph View Doesn't Support Editing
**Where**: Workflow editor graph toggle
**Problem**: Graph view is read-only visualization. Clicking a node in graph view navigates back to list view. Users can't drag nodes, create connections, or edit properties in graph view.
**Recommendation**: At minimum, allow clicking a node in graph view to open its config panel without switching views.

### 15. "Tests Hidden" Toggle Creates Confusion in Demos
**Where**: Kanban board toolbar (`ux-review/13-kanban-board.png`)
**Problem**: The yellow "Tests Hidden" pill is enabled by default, filtering out `[E2E]` prefixed tasks. During a demo, this means newly created test tasks might be invisible until the filter is toggled off. The demo scripts don't interact with this toggle.
**Recommendation**: Auto-disable "Tests Hidden" when running in demo mode, or make it more obvious that tasks may be filtered.

### 16. Feedback Dialog → Task → Kanban Flow Has No Visual Continuity
**Where**: Bug report lifecycle demo
**Problem**: User submits feedback via dialog → toast appears → dialog closes → user navigates to kanban → searches for the task. There's no deep link, no highlight animation, no "View your submission" button in the success toast.
**Recommendation**: Add "View Task" action button in the success toast that navigates directly to the new task on the kanban board.

### 17. Agent Reviewers Panel UX Is Unclear
**Where**: Task detail drawer → Agent Reviewers section (`ux-review/14-task-detail-drawer.png`)
**Problem**: Shows "AGENT REVIEWERS" header with "+ Add" button and "No agent reviewers assigned". The term "Agent Reviewers" is jargon — users don't know what agents are available, what they review, or why they'd add one. The Add button opens a list of agents but doesn't explain what each agent does.
**Recommendation**:
- Rename to "Automated Reviewers" or "AI Code Review"
- Add subtitle: "Add AI agents to automatically review PRs for this task"
- Show agent descriptions in the selection dropdown

### 18. Notification Dropdown → Task Navigation Is Unreliable
**Where**: Notification center demo, Step 7
**Problem**: The demo clicks the first notification item expecting navigation to a task, but verifies with a very loose assertion (`url.includes("/tasks/") || url.includes("/my-tasks") || url.includes("/projects/")`). The notification items use `.cursor-pointer` CSS but may not all have working deep links.
**Recommendation**: Ensure every notification item has a clear, working deep link. Add visual affordance (arrow icon, "View" text) to indicate clickability.

---

## WORKFLOW-SPECIFIC DEMO OPTIMIZATION OPPORTUNITIES

### A. CRM Pipeline Demo (workflow-crm-pipeline.spec.ts)
**Current**: 8 sequential parts across 4+ page contexts
**Issues**:
- Part 5 has a 50-line API fallback for when auto-approve fails (confidence < 0.7) — this means the UI flow doesn't reliably work without API intervention
- Cleanup runs in `afterAll` but is fragile (searches by email, name keywords)
- No visual indication when workflow execution completes — user just watches for URL change

**Optimization**:
- Combine Parts 3-5 (run + verify + commit) into a guided wizard flow
- Show real-time execution progress (node-by-node status updates) instead of waiting for redirect
- Auto-select the just-created workflow in the run dialog
- Show confidence threshold explanation when auto-approve fails

### B. Pipeline Intelligence Demo (pipeline-intelligence-workflow.spec.ts)
**Current**: 5 parts, mostly API-driven stage advancement
**Issues**:
- Parts 3-4 advance through 5 stages entirely via API — no UI interaction for the core feature
- Review tasks are auto-completed via API to unblock advancement — user never sees the gate
- The "gate" concept (complete review task before advancing) is the key value prop but it's invisible

**Optimization**:
- Show the review task creation as a visible step in the UI
- Add a "Complete & Advance" button on the deal detail panel
- Show stage transition animation on the pipeline board
- Display auto-created delivery deal when closing as Won

### C. Bug Report Lifecycle Demo (bug-report-lifecycle.spec.ts)
**Current**: 8 steps, agent interactions simulated via API
**Issues**:
- Steps 4-6 simulate agent work + QA verdict via API, bypassing the actual agent execution system
- The `waitForToast` helper has multiple fallback patterns (`/moved to In Review|In Review/i`) suggesting unreliable feedback
- Step 6 requires a `page.reload()` because the watcher panel doesn't update in real-time

**Optimization**:
- Fix WebSocket/SSE updates so watcher status reflects in real-time (no reload needed)
- Add a visual "agent working" indicator (animated border, progress dots) on the task card
- Show the PR link prominently in the task detail when linked

### D. Spanish Pipeline Demo (workflow-spanish-pipeline.spec.ts)
**Current**: Exact duplicate of CRM pipeline demo with Spanish content
**Issues**:
- Identical 6-part structure, same node types, same review flow
- Only value-add over English demo is proving multilingual extraction works
- Could be reduced to a variant/config of the CRM demo

**Optimization**:
- Merge with CRM pipeline demo as a "language variant" parameter
- Or: make it showcase something unique (e.g., cross-language entity resolution, accent handling)

---

## CROSS-CUTTING DESIGN RECOMMENDATIONS

### I. Add a "Demo Mode" Overlay
When running demos (`E2E_DEMO_PACE` env var is set), show:
- Step counter ("Step 3 of 8: Running workflow...")
- Explanatory subtitles for what's happening
- Highlight the active UI element with a spotlight effect
- Pause points with "Press any key to continue"

### II. Create a Guided Workflow Wizard
Replace the current fragmented flow with a 4-step wizard:
1. **Select Source** — Pick or create a data source
2. **Build Pipeline** — Choose/edit workflow (embedded editor)
3. **Run & Review** — Execute, show progress, review staging
4. **Commit** — Approve records, see them in CRM/Tasks

### III. Add Contextual Help System
Each major section needs a small `(?)` icon linking to:
- What this section does
- When to use it
- How it connects to other features
- Example use case

### IV. Improve Real-Time Feedback
- Workflow execution: show node-by-node progress bar
- Staging commit: show per-record success/failure animation
- Agent watcher: live status updates via WebSocket (not page reload)
- Pipeline advancement: stage transition animation

### V. Reduce Navigation Hops
Current: 5+ page navigations for one workflow execution
Target: 2-3 at most (select → run → review)
Method: In-context panels, contextual drawers, persistent workflow run context

---

## Summary by Priority

| Priority | Count | Theme |
|----------|-------|-------|
| P0 Critical | 5 | Visual connections, node picker, workflow ID, run flow fragmentation, staging empty state |
| P1 Major | 6 | Runs data resolution, data source UX density, automations read-only, layout balance, navigation fragmentation, pipeline context |
| P2 Medium | 7 | Prompt UX, output schema, graph editing, test toggle, feedback continuity, agent reviewers, notification links |
| Optimization | 4 | Demo-specific improvements for CRM, pipeline, bug lifecycle, Spanish demos |
| Cross-cutting | 5 | Demo mode, guided wizard, contextual help, real-time feedback, navigation reduction |

**Total: 27 findings + 5 cross-cutting recommendations**

---

## Files Referenced

| Component | Path |
|-----------|------|
| Workflow Editor | `frontend/src/components/workflows/editor/index.tsx` |
| Node Config Panel | `frontend/src/components/workflows/editor/NodeConfigPanel.tsx` |
| Node Picker | `frontend/src/components/workflows/editor/NodePicker.tsx` |
| Graph View | `frontend/src/components/workflows/editor/WorkflowGraphView.tsx` |
| Staging Review | `frontend/src/components/workflows/StagingReviewPanel.tsx` |
| Runs Panel | `frontend/src/components/workflows/WorkflowRunsPanel.tsx` |
| Triggers Panel | `frontend/src/components/workflows/WorkflowTriggersPanel.tsx` |
| Builder Tab | `frontend/src/pages/workflows/tabs/BuilderTab.tsx` |
| Staging Tab | `frontend/src/pages/workflows/tabs/StagingTab.tsx` |
| Runs Tab | `frontend/src/pages/workflows/tabs/RunsTab.tsx` |
| CRM Pipeline Demo | `e2e/demos/workflow-crm-pipeline.spec.ts` |
| Spanish Pipeline Demo | `e2e/demos/workflow-spanish-pipeline.spec.ts` |
| Pipeline Intelligence Demo | `e2e/demos/pipeline-intelligence-workflow.spec.ts` |
| Bug Report Demo | `e2e/demos/bug-report-lifecycle.spec.ts` |
| QA Trigger Demo | `e2e/demos/manual-qa-trigger.spec.ts` |
| Notification Demo | `e2e/demos/notification-center.spec.ts` |
| Workflow Builder Helpers | `e2e/helpers/workflow-builder.ts` |
| Demo Simulation Helpers | `e2e/helpers/demo/simulation.ts` |
