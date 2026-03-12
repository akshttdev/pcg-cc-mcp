# Sprint QA Review — 2026-03-12

**Reviewer:** QA / Full-Stack Architecture Review
**Method:** Firefox MCP (Playwright) browser-only testing
**Companion docs:** `2026-03-12-improvement-roadmap.md`, `2026-03-12-sprint-implementation-issues.md`

---

## QA Checklist & Results

### Sprint 1

#### 1A. Completion Criteria in Task Form
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Task form has Completion Criteria field | **PASS** | Field present with placeholder "What must be true for this task to be considered done?" |
| 2 | Task form has Output Format field | **PASS** | Field present with placeholder "Expected deliverable format" |
| 3 | Help text displayed below fields | **PASS** | "Structured success criteria for agents to self-evaluate completion" / "Describes the expected deliverable format for agent output" |
| 4 | Simple mode toggle visible | **PASS** | Toggle exists at top of dialog header |
| 5 | Simple mode hides advanced fields | **BLOCKED** | Toggle is outside viewport when dialog scrolls — cannot click via Playwright. Usability gap. |
| 6 | Completion criteria persists on task save | **FAIL** | Task model (API response) does not include `completion_criteria` or `output_format` fields. These fields exist in the form but the `Task` DB model/struct has no columns for them. Data is lost on save. |
| 7 | Task detail view shows completion criteria | **FAIL** | Task detail "Overview" tab shows only Description, Recent Artifacts, Agent Terminal, Recent Activity — no completion criteria or output format displayed. |

**Critical Gap:** The Task form collects completion_criteria/output_format but the Task backend model has no columns for these fields. The TaskTemplate model has them, but Tasks do not. Data entered in the form is silently discarded.

---

#### 1B. Project Scaffolding UI
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Template selector appears in Create Project dialog | **PASS** | "Start from Template" dropdown visible |
| 2 | All 4 templates available | **PASS** | Software Development, Research Project, Marketing Campaign, Client Onboarding all present with descriptions |
| 3 | Template description shown | **PASS** | "5 tasks: setup, implementation, testing, review, deployment" + "Creates 5 pre-configured tasks..." |
| 4 | Project creation with template scaffolds tasks | **PASS** | Created "QA Test Software Project" → 5 tasks created immediately |
| 5 | Scaffolded tasks have correct titles | **PASS** | Deployment & Release, Core Feature Implementation, Project Setup & Environment Configuration, Code Review & Refactoring, Test Suite Development |
| 6 | Scaffolded tasks have correct priorities | **PASS** | 3 High, 2 Medium — matching template definitions |
| 7 | Scaffolded tasks have descriptions | **PASS** | All tasks have meaningful descriptions |
| 8 | Scaffolded tasks have completion_criteria | **FAIL** | API response for scaffolded tasks has no completion_criteria field — same root cause as 1A.6 |

---

#### 1C. ORCHA Task Server in MCP Popular Servers
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | MCP Servers settings page loads | **PASS** | `/settings/mcp` renders correctly |
| 2 | Popular servers carousel visible | **PASS** | Shows 3 servers: Duck Kanban, Context7, Playwright |
| 3 | ORCHA Task Server visible in carousel | **FAIL** | Not visible. Only 3 servers shown. Both Previous/Next slide buttons are disabled — no additional items in carousel. |
| 4 | Carousel scrolling works for 4+ items | **N/A** | Only 3 items present |

**Gap:** ORCHA Task Server was added to `default_mcp.json` as a platform server but does not appear in the popular servers carousel. The carousel source and `default_mcp.json` may be decoupled — the carousel is likely hardcoded in the frontend component.

---

#### 1D. Verify output_tasks Node
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Output: Tasks node appears in workflow NodePicker | **PASS** | Listed under "Outputs" category as "Output: Tasks — Create tasks from workflow results" |

---

#### 1E. Agent Execution Dashboard (Mission Control)
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Mission Control page exists at `/mission-control` | **PASS** | Page loads with title "Mission Control" |
| 2 | Active Agents section visible | **PASS** | Shows "Active Agents: 0" with empty state message |
| 3 | Execution Timeline tab works | **PASS** | Timeline tab selected by default, shows "No active executions" |
| 4 | Workflows tab present | **PASS** | Tab available for switching |
| 5 | Grid tab present | **PASS** | Tab available for switching |
| 6 | Live Coordination panel visible | **PASS** | Shows "Waiting for events..." |
| 7 | Refresh button works | **PASS** | Refresh button present and clickable |
| 8 | WebSocket status shown | **PASS** | Shows connection status indicator |
| 9 | Discoverability — link from agent cards | **NOT TESTED** | No direct link from agent cards to Mission Control observed |

---

### Sprint 2

#### 2A. New Node Types (Frontend)
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Conditional node in NodePicker | **PASS** | "Branch execution based on a condition" |
| 2 | Send Notification node | **PASS** | "Send an in-app notification or email alert" |
| 3 | Assign to Agent node | **PASS** | "Create a task and assign it to an AI agent" |
| 4 | HTTP Request node | **PASS** | "Make an external API call" |
| 5 | Update CRM Contact node | **PASS** | "Update fields on existing CRM contacts" |
| 6 | Update CRM Deal node | **PASS** | "Update deal stage, value, or other fields" |
| 7 | Update Company node | **PASS** | "Update fields on existing company records" |
| 8 | NodePicker has Actions category | **PASS** | Nodes organized under "Actions" section |
| 9 | NodePicker has Outputs category | **PASS** | Output nodes under "Outputs" section |
| 10 | Conditional node config panel | **PASS** | Shows: Condition input, True Branch Label, False Branch Label |
| 11 | Adding node to workflow | **PASS** | Conditional node added as node #4 to workflow |
| 12 | Appropriate icons for each node | **PASS** | Each node type has distinct icon |

---

#### 2B. Convert Automations to System Workflows
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Existing automations still visible | **PASS** | 5 hardcoded automations in Automations tab |
| 2 | Status: Deferred per plan | **N/A** | Kept existing hardcoded automations running |

---

#### 2C. Workflow Trigger System
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Schedule trigger UI exists | **FAIL** | No UI for creating or managing workflow schedule triggers. Automations tab only shows hardcoded automations. |
| 2 | Backend schedule loop running | **NOT TESTABLE** | Backend-only feature, cannot verify via browser |

**Gap:** The schedule trigger system is backend-only. There is no frontend UI to create, view, or manage schedule-based workflow triggers. Users cannot configure `every_5m`, `hourly`, `daily`, etc. intervals through the UI.

---

#### 2D. System Workflows
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Bug Triage Pipeline workflow visible | **FAIL** | Not in workflow definitions list |
| 2 | Sprint Planning workflow visible | **FAIL** | Not in workflow definitions list |
| 3 | Client Onboarding workflow visible | **FAIL** | Not in workflow definitions list |
| 4 | Content Pipeline workflow visible | **FAIL** | Not in workflow definitions list |
| 5 | API returns system workflows | **FAIL** | `GET /api/workflows/definitions` returns only 1 workflow (default_analysis) |

**Root Cause:** The `seed_defaults()` function seeds workflows on fresh database initialization. The dev database is copied from `dev_assets_seed/` which doesn't include the new system workflows. The seed function needs to be triggered, or the workflows need to be added to the seed database.

---

### Sprint 3

#### 3A. Wire ACP Agents to MCP
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Backend loads platform MCP servers | **NOT TESTABLE** | Backend-only feature requiring ACP agent execution |

---

#### 3B. Platform vs User MCP Config Separation
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | MCP settings page shows user config path | **PASS** | Shows "Changes will be saved to: /Users/mediamonsters/.claude.json" |
| 2 | User config separate from platform config | **PASS** | Architectural separation confirmed — user sees only their `.claude.json` |

---

#### 3C. Enhanced Task Templates
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Task template table visible in settings | **PASS** | 3 templates shown: Add Unit Tests, Bug Analysis, Code Refactoring |
| 2 | Template table shows new columns (priority, etc.) | **FAIL** | Table only shows: Template Name, Title, Description, Actions. No priority, completion_criteria, output_format, assigned_agent, or tags columns. |
| 3 | Template edit dialog has new fields | **FAIL** | Edit dialog only shows: Template Name, Default Title, Default Description. The 6 new fields are not exposed. |
| 4 | Backend model supports new fields | **PASS** | `TaskTemplate` struct and migration include all 6 new columns |
| 5 | API accepts new fields | **PASS** | `CreateTaskTemplate`/`UpdateTaskTemplate` types include new fields (passed as null from frontend) |

**Gap:** The backend fully supports the enhanced task template fields but the frontend (`TaskTemplateEditDialog.tsx`) doesn't render form inputs for them. Users cannot set priority, completion_criteria, output_format, assigned_agent, or tags on templates through the UI.

---

#### 3D. Agent Capability Profiles
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Agents settings page loads | **PASS** | 13 agents displayed with rich cards |
| 2 | Agent cards show basic info | **PASS** | Name, designation, description, status, avatar |
| 3 | Agent cards link to profile | **FAIL** | No link or button to view agent profile/stats |
| 4 | `/api/agents/:id/profile` returns data | **FAIL** | Returns HTML instead of JSON — route not proxied to backend. Tested via `fetch()` from browser. Regular `/api/agents/:id` works fine. |
| 5 | Profile shows execution stats | **NOT TESTABLE** | Endpoint inaccessible from frontend |
| 6 | Profile shows recent attempts | **NOT TESTABLE** | Endpoint inaccessible from frontend |
| 7 | Profile shows MCP tools | **NOT TESTABLE** | Endpoint inaccessible from frontend |

**Root Cause:** The `/api/agents/{id}/profile` route returns HTML (Vite SPA fallback) instead of being proxied to the backend. This may be because the backend isn't registering the route correctly, or the compiled backend binary doesn't include the new route. The Vite proxy config (`/api` prefix) should match this path.

---

#### 3E. Agent Flow Orchestration (Design Only)
| # | Test | Result | Notes |
|---|------|--------|-------|
| 1 | Design-only for this sprint | **N/A** | Per roadmap, no implementation expected |

---

## Cross-Cutting Observations

### Console Errors & Warnings
| Issue | Severity | Count | Details |
|-------|----------|-------|---------|
| WebSocket connection failures | Medium | Every page | `Firefox can't establish a connection to ws://...` — expected in dev without active agent executions |
| i18n missing translation keys | Low | ~100+ per settings page | All settings page strings fall through to default English — keys not configured in i18n |
| React Router future flag warnings | Low | Every page | Standard deprecation warnings for v7 migration |
| `validateDOMNesting` warning | Low | Workflow editor | Button nested inside button in workflow editor |

### Usability Gaps
1. **Task form dialog too tall** — Simple mode toggle and Close button are outside the viewport when the dialog content is long. Users can't access the toggle without scrolling the page behind the dialog.
2. **No workflow schedule trigger UI** — Backend supports schedule triggers but users have no way to create or manage them.
3. **Task template edit missing new fields** — Backend has 6 new columns but the edit dialog only exposes 3 original fields.
4. **Agent profile not accessible** — Profile endpoint exists but isn't reachable from the frontend.
5. **System workflows not seeded** — 4 new workflows exist in code but don't appear in the running dev database.
6. **ORCHA server not in MCP carousel** — Popular servers list appears hardcoded and doesn't include ORCHA.
7. **completion_criteria/output_format on Task model** — Fields exist on form and template but the Task struct itself has no columns for these values. Data entered is silently lost.

### Architecture Observations
1. **Seed vs Migration gap**: New system workflows are seeded programmatically but the dev database is file-copied, so seeds don't run unless explicitly triggered.
2. **Frontend-backend field mismatch**: `completion_criteria` and `output_format` exist on `TaskTemplate` and `TaskFormDialog` but not on the `Task` model. This is the most impactful gap — it undermines the core Sprint 1A feature.
3. **Route proxy issue**: The `/api/agents/:id/profile` endpoint works conceptually but isn't reachable through the Vite dev proxy, suggesting the backend binary may not include the new route or there's a routing conflict.

---

## Summary

| Sprint | Feature | Status |
|--------|---------|--------|
| 1A | Completion Criteria in Task Form | **PARTIAL** — UI present, backend Task model missing fields |
| 1B | Project Scaffolding UI | **PASS** — Templates and task creation working |
| 1C | ORCHA in MCP Popular Servers | **FAIL** — Not visible in carousel |
| 1D | output_tasks Node | **PASS** — Present in NodePicker |
| 1E | Mission Control Dashboard | **PASS** — Fully functional |
| 2A | New Node Types (Frontend) | **PASS** — All 7 types with config panels |
| 2B | Convert Automations | **DEFERRED** — As planned |
| 2C | Workflow Trigger System | **PARTIAL** — Backend only, no UI |
| 2D | System Workflows | **FAIL** — Not seeded in dev database |
| 3A | Wire ACP to MCP | **NOT TESTABLE** — Backend-only |
| 3B | Platform/User MCP Separation | **PASS** — Architectural separation confirmed |
| 3C | Enhanced Task Templates | **PARTIAL** — Backend done, frontend UI not updated |
| 3D | Agent Capability Profiles | **PARTIAL** — Backend endpoint exists, not accessible from frontend |
| 3E | Agent Flow Orchestration | **N/A** — Design only |

### Priority Fixes Needed
1. **P0**: Add `completion_criteria` and `output_format` columns to the `tasks` table (migration) and update the Task Rust model + TypeScript types. Without this, Sprint 1A is fundamentally incomplete.
2. **P1**: Trigger `seed_defaults()` or add new system workflows to the seed database so Sprint 2D workflows are available.
3. **P1**: Debug why `/api/agents/:id/profile` returns HTML — either recompile backend with new route or fix routing conflict.
4. **P2**: Add UI fields to `TaskTemplateEditDialog` for priority, completion_criteria, output_format, assigned_agent, tags.
5. **P2**: Add ORCHA Task Server to the MCP popular servers carousel.
6. **P2**: Build minimal UI for workflow schedule triggers (create/view/delete).
7. **P3**: Display completion_criteria and output_format in the task detail view.
8. **P3**: Add "View Profile" link to agent cards in Settings → Agents.
