# Sprint Plan — sloperation327 continuation
_Created: 2026-03-27_

---

## Track A — Fix Phase I for New Leads (Pipeline Gaps)

Current gap: intake pipeline creates persons + companies + crm_deals but NOT a client/prospect record,
so client card KG inheritance and the review-task trigger are unreachable for fresh leads.

### A1 — Create prospect record on intake
**File:** `crates/server/src/routes/intake/pipeline.rs`
- After company + crm_deal are created, also INSERT into `clients` with:
  - `name` = company name
  - `organization_id` = resolved org
  - `company_id` = UUID of the matched/created company (TEXT with dashes)
  - `prospect_at` = now
  - `client_since` = NULL (set on Won)
- Upsert semantics: `INSERT OR IGNORE` on `(organization_id, name)`

### A2 — Wire company_id on the prospect client
**File:** same, after client upsert
- After inserting/finding the client, run:
  ```sql
  UPDATE clients SET company_id = ? WHERE id = ?
  ```
- This ensures `auto_create_intel_review_tasks()` can find the deal→client→company chain

### A3 — Fix review-task query to fallback to crm_deal directly
**File:** `crates/server/src/routes/intelligence.rs` → `auto_create_intel_review_tasks()`
- Current query: `crm_deals JOIN clients ON cl.company_id = ?` — fails if no client yet
- Fix: UNION with a second path: `crm_deals WHERE company_id = ?` (if deals have direct company ref)
  OR broaden: query crm_contacts → persons → company match

---

## Track B — Sirak Discovery Call Automation (Workflow Builder)

### Overview

When `sirak@sirakstudios.com` emails/shares a discovery call with Nora, the system runs a
**fully automated 3-phase pipeline with no human checkpoints** and produces exactly 3 tasks:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SIRAK DISCOVERY CALL → PIPELINE                         │
└─────────────────────────────────────────────────────────────────────────────┘

  [Email/Upload from sirak@sirakstudios.com]
           │
           ▼
  ┌─────────────────┐
  │  INTAKE INGEST  │  Extract: persons, companies, action items
  │  (existing)     │  Create: crm_contact, crm_deal, client prospect
  └────────┬────────┘
           │  spawns
           ▼
  ╔═════════════════╗   creates Task #1
  ║  PHASE I TASK   ║ ──────────────────► [Task: "Scout: {Company}"]
  ║  Scout Research ║                      status: in_progress → done
  ║  (auto-trigger) ║                      assignee: Nora (agent)
  ╚════════╦════════╝
           ║  on completion, auto-triggers (no human gate)
           ▼
  ╔═════════════════╗   creates Task #2
  ║  PHASE II TASK  ║ ──────────────────► [Task: "Astra Report: {Company}"]
  ║  Astra Deep     ║                      status: in_progress → done
  ║  Research       ║                      assignee: Nora (agent)
  ╚════════╦════════╝
           ║  on completion
           ▼
  ╔═════════════════╗   creates Task #3
  ║  PHASE III TASK ║ ──────────────────► [Task: "Review: {Company} Report"]
  ║  Human Review   ║                      status: pending (in_review column)
  ║  (ready state)  ║                      assignee: Sirak (operator)
  ╚═════════════════╝                      linked: business_report_id
```

### B1 — Sirak Auto-Trigger Detection
**File:** `crates/server/src/routes/intake/pipeline.rs`
- After org/assignee resolution: detect `sirak@sirakstudios.com` sender
- Set flag `auto_pipeline: bool = true` on the intake item
- Store as `metadata` JSON on `call_intake_items`

### B2 — Phase I Task Creation (on intake)
**File:** `crates/server/src/routes/intake/pipeline.rs`
- When `auto_pipeline = true`: insert Task #1
  ```
  title: "Scout: {company_name}"
  description: "Phase I — Automated Scout research for {company_name} discovery call"
  status: "in_progress"
  assignee_id: Nora agent user_id (or NULL for agent tasks)
  metadata: { phase: 1, company_id: "...", intake_item_id: "...", auto_pipeline: true }
  ```
- Store `phase1_task_id` on the intake item

### B3 — Phase I Completion → Auto-trigger Phase II (no human gate)
**File:** `crates/server/src/routes/intelligence.rs` → `run_company_research_direct()`
- After research completes, check if any intake item linked to this company has `auto_pipeline = true`
- If yes: skip `auto_create_intel_review_tasks()` → directly call `run_phase2_from_company_intel()`
- Mark Phase I task → `done`
- Create Phase II task (Task #2):
  ```
  title: "Astra Report: {company_name}"
  status: "in_progress"
  metadata: { phase: 2, company_id: "...", auto_pipeline: true }
  ```

### B4 — Phase II Completion → Create Review Task (Task #3)
**File:** `crates/server/src/routes/intake/report.rs` → `run_phase2_from_company_intel()`
- After `BusinessReport` is created and marked ready:
- Mark Phase II task → `done`
- Create Task #3:
  ```
  title: "Review: {company_name} Business Report"
  description: "Phase III — Human review of Astra deep research report"
  status: "todo"  (renders in "In Review" column via kanban config)
  assignee_id: resolved operator (deal owner, or Sirak's user_id for Sirak org)
  metadata: { phase: 3, business_report_id: "...", auto_pipeline: true }
  ```

### B5 — Workflow Builder Config (no-code definition)
**File:** `crates/db/migrations/20260428000000_sirak_auto_pipeline_workflow.sql`
- Insert a row into `agent_workflows` (or new `crm_workflows` table) defining the trigger:
  ```json
  {
    "id": "sirak-discovery-auto-pipeline",
    "trigger": { "type": "intake_email", "from_domain": "sirakstudios.com" },
    "steps": [
      { "id": "phase1", "action": "scout_research", "auto": true },
      { "id": "phase2", "action": "astra_report",   "auto": true, "depends_on": "phase1" },
      { "id": "phase3", "action": "human_review",    "auto": false, "assignee": "deal_owner" }
    ]
  }
  ```

---

## Task Kanban Column Mapping

```
┌───────────┬──────────────┬──────────────┬───────────┐
│  Backlog  │  In Progress │  In Review   │   Done    │
├───────────┼──────────────┼──────────────┼───────────┤
│           │  Task #1     │              │  Task #1  │
│           │  Scout:Co... │              │  ✓ done   │
│           │  ──────────► │              │           │
│           │  Task #2     │              │  Task #2  │
│           │  Astra:Co... │              │  ✓ done   │
│           │  ──────────► │              │           │
│           │              │  Task #3     │           │
│           │              │  Review:Co.. │           │
│           │              │  (assigned   │           │
│           │              │   to Sirak)  │           │
└───────────┴──────────────┴──────────────┴───────────┘
```

---

## Implementation Order

| # | Task | File(s) | Priority |
|---|------|---------|----------|
| 1 | Create prospect client on intake (A1+A2) | `pipeline.rs` | High |
| 2 | Fix review-task fallback query (A3) | `intelligence.rs` | High |
| 3 | Detect Sirak auto-pipeline flag (B1) | `pipeline.rs` | High |
| 4 | Phase I task creation on intake (B2) | `pipeline.rs` | High |
| 5 | Phase I done → auto-spawn Phase II (B3) | `intelligence.rs` | High |
| 6 | Phase II done → create review task #3 (B4) | `report.rs` | High |
| 7 | Workflow definition in DB/migration (B5) | migration + model | Medium |
| 8 | Frontend: pipeline status view (3 tasks) | `client-overview.tsx` | Medium |

---

## Data Flow Diagram

```
call_intake_items
  │  organization_id ──► organizations (sirakstudios.com)
  │  auto_pipeline = true
  │  phase1_task_id ──────────────────────► tasks (Scout)
  │  phase2_task_id ──────────────────────► tasks (Astra)
  │
  ├─► persons ──► person_research_passes
  │
  └─► companies ──► company_knowledge_sources
        │                  │
        │            intelligence_status: done
        │                  │
        ▼                  ▼
      clients         business_reports ──► tasks (Review, assigned to Sirak)
        │  company_id       │
        │  prospect_at      │ client_id
        │                   │ review_status: ready
        └───────────────────┘
```
