# PCG Acquisition Pipeline — Rebuild Plan
**Date:** 2026-05-18  
**Status:** Ready for sprint execution

---

## Table of Contents

1. [Diagnosis Summary](#1-diagnosis-summary)
2. [Intended Happy Path](#2-intended-happy-path)
3. [Bug Breakdown](#3-bug-breakdown)
4. [Sprint Plan](#4-sprint-plan)
5. [Validation Checklist](#5-validation-checklist)

---

## 1. Diagnosis Summary

The 9-stage acquisition pipeline is currently broken at every automated stage
transition. The root failure is a **stage name mismatch**: 31 deals were
bulk-seeded on 2026-03-15 with `stage = 'qualification'`, a name that does not
exist in the live pipeline definition. Because stage automation keys off the
string `"intel"` (in `stage_transition.rs:554`), those 31 deals are completely
invisible to every agent trigger.

Secondary failures compound the damage: even for the handful of deals that did
reach `Intel` correctly, the Scout deduplication guard only checks for
`Phase 1 Research:` task prefixes — but tasks seeded by the intake pipeline use
the title format `Scout: <company>` and `workflow_type = 'phase1_scout'`. The
Astra → Cash chain fires inside a bare `tokio::spawn` with no status task, so
Cash failures are silent. And the Won provisioning `INSERT INTO projects` omits
`client_id`, producing internal-looking projects the Command Center AR counter
misses.

### Where the pipeline currently breaks

```
  [Intake / Manual creation]
           |
           v
  crm_deals.stage = 'qualification'   <-- BUG-1: not a real stage
           |
           | (Intel-stage trigger never fires:
           |  stage_transition.rs:554 checks "intel", not "qualification")
           |
           X  Scout never runs
           X  Phase 1 tasks never created  <-- BUG-3: boards stay empty
           X  Astra never runs
           X  Cash never runs
           X  Lux never runs

  Separately, for the few deals that DO reach Intel correctly:
           |
           v
  trigger_who_is_research() called twice (double-hit or stage rollback)
           |
           v
  Phase 1 tasks created a second time  <-- BUG-2: no cross-prefix dedup guard
           |
           v
  Astra Pass 2 completes
           |
           v
  generate_proposal_background() called inside tokio::spawn
           |
           v
  Cash fails silently  <-- BUG-5: no visibility task, no retry, deal stuck
           |
           v
  (if Cash succeeds) deal reaches Won
           |
           v
  provision_won_deal() INSERT INTO projects omits client_id  <-- BUG-6
           |
           v
  Project appears as internal. AR counter misses it.

  Orthogonal:
  E2E spec → navigateToFirstProjectTasks() → projects[0]
  → creates "[E2E] Kanban Test <timestamp>" tasks on Legendary Forms board
  → cleanupTestData() races or fails silently  <-- BUG-4
  → 6 leftover test artifacts on live board
```

### Current DB snapshot (audit 2026-05-18)

```
  crm_deals
  ---------
  31 deals  stage = 'qualification'     ← BUG-1, never move
   3 deals  stage = 'Intel'
   1 deal   stage = 'Polish'
   1 deal   stage = 'Lead'
   1 deal   stage = 'Won'   (Bad Girl Strength Club — Stephanie Bruno)

  projects
  --------
  30 acquisition project boards
  22 boards  task_count = 0             ← BUG-3, dead weight

  tasks (scout)
  -------------
  ~10 scout tasks, mostly status = 'todo', no completion
   3 duplicate "Scout: Competitor Deep Dive" on Bad Girl Strength Club  ← BUG-2
   6 "[E2E] Kanban Test" tasks on Legendary Forms  ← BUG-4
```

---

## 2. Intended Happy Path

### Full working flow

```
  [Lead enters — via Intake, manual, or API]
                    |
                    v
           +------------------+
           |   Lead (stage)   |  auto_skip → immediately advances to Intel
           +------------------+
                    |
                    v
           +------------------+
           |   Intel (stage)  |  ENTRY: trigger_who_is_research()
           +------------------+    ├─ SET contact.intelligence_status = 'queued'
                    |              ├─ spawn → run_contact_research_direct()
                    |              ├─ create Phase 1 visibility tasks (dedup guard)
                    |              └─ trigger_company_research_if_idle()
                    |
                    |  [Human reviews Scout output, advances deal]
                    v
  +---------------------------+
  |  Business Analysis        |  ENTRY: generate_phase1_business_report()
  +---------------------------+    ├─ dedup: skip if report already exists
                    |              ├─ compile person + company intel
                    |              └─ Astra Pass 1 → INSERT business_reports
                    |
                    |  [Human reviews Astra report, advances deal]
                    v
           +------------------+
           | Discovery        |  ENTRY: create review task (gated)
           +------------------+  Human conducts discovery call, links transcript
                    |
                    |  [Human advances deal after transcript linked]
                    v
           +------------------+
           | Proposal         |  ENTRY: trigger_deep_research_pass2()
           +------------------+    ├─ Astra Pass 2: enhance report with transcript
                    |              ├─ UPDATE business_reports.status = 'enhanced'
                    |              ├─ CREATE "Cash: Writing Proposal" task (inprogress)
                    |              └─ generate_proposal_background()
                    |                    ├─ success → UPDATE crm_deals.proposal_text
                    |                    │            UPDATE task → done
                    |                    └─ failure → UPDATE task → todo + error note
                    |                                 log crm_activities agent_failure
                    |
                    |  [Human reviews & approves proposal]
                    v
           +------------------+
           | Polish           |  ENTRY: generate_deck_background()
           +------------------+    ├─ only if deck_url IS NULL & proposal_text IS NOT NULL
                    |              ├─ CREATE "Lux: Building Deck" task (inprogress)
                    |              ├─ success → UPDATE crm_deals.deck_url
                    |              └─ failure → UPDATE task → todo + error note
                    |
                    |  [Human reviews deck, advances deal]
                    v
  +-------------------------------+
  | Present & Invoice             |  ENTRY: create review task
  +-------------------------------+  Human sends invoice → crm_deals.invoice_id set
                    |
                    |  [Client pays / human confirms Won or Lost]
                    v
        +-----------+-----------+
        |                       |
        v                       v
  +-----------+          +----------+
  |    Won    |          |   Lost   |
  +-----------+          +----------+
       |
       v
  provision_won_deal()
       ├─ find/create clients row
       ├─ INSERT INTO projects (... client_id = clients.id ...)  ← BUG-6 fix
       ├─ UPDATE crm_deals SET project_id = new_project.id
       ├─ create tasks from deliverables (or fallback setup task)
       ├─ INSERT INTO vibe_transactions
       ├─ INSERT INTO crm_activities
       └─ crm_deals delivery pipeline deal (dedup by contact_id)
```

### Agent roles

```
  Scout  — Intel stage
           contact + company intelligence
           updates crm_contacts.intelligence_status

  Astra  — Business Analysis (Pass 1) + Proposal (Pass 2)
           generates / enhances business_reports rows

  Cash   — Proposal stage (chained after Astra Pass 2)
           writes crm_deals.proposal_text

  Lux    — Polish stage
           writes crm_deals.deck_url
```

---

## 3. Bug Breakdown

### BUG-1: Stage Name Mismatch — 31 deals stuck at "qualification"

**Root cause.** Deals were bulk-created on 2026-03-15 with `stage = 'qualification'`.
The live pipeline starts at `Intel`. `stage_transition.rs:554` checks:

```rust
if stage_name_lower == "intel" || stage_type_lower == "intel" {
```

`"qualification"` matches neither branch. All 31 deals are invisible to
every automated trigger.

**Fix.** SQL migration:

```sql
-- crates/db/migrations/20260518500000_fix_qualification_stage.sql
UPDATE crm_deals
SET
  crm_stage_id = (
    SELECT s.id
    FROM crm_pipeline_stages s
    JOIN crm_pipelines p ON p.id = s.pipeline_id
    WHERE p.pipeline_type = 'sales'
      AND lower(s.name) = 'intel'
    LIMIT 1
  ),
  stage = 'Intel',
  updated_at = datetime('now', 'subsec')
WHERE stage = 'qualification'
  AND crm_pipeline_id IN (
    SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales'
  );
```

After migration, Scout backfill must be triggered manually for each newly-unblocked deal.

---

### BUG-2: Scout Task Deduplication Missing

**Location.** `crates/server/src/routes/crm_deal_automations.rs` lines 162-196.

**Root cause.** `trigger_who_is_research()` dedup guard at line 162:

```rust
"SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ?
 AND title LIKE 'Phase 1 Research:%' AND deleted_at IS NULL"
```

Only catches `Phase 1 Research:` prefix. Intake pipeline seeds tasks with
`Scout: <company>` prefix and `workflow_type = 'phase1_scout'`. The two
systems don't share a dedup key — double-triggering creates duplicate tasks.

**Evidence.** Bad Girl Strength Club: 3 identical `Scout: Competitor Deep Dive` tasks.

**Fix (code).** Expand the dedup query at line 162:

```rust
let has_phase1_tasks: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM tasks WHERE crm_deal_id = ?
     AND (title LIKE 'Phase 1 Research:%' OR workflow_type = 'phase1_scout')
     AND deleted_at IS NULL",
)
.bind(&deal_id)
.fetch_one(pool)
.await
.unwrap_or(0);
```

**Fix (DB backstop).** Migration:

```sql
-- crates/db/migrations/20260518530000_uq_phase1_scout_per_deal.sql
CREATE UNIQUE INDEX IF NOT EXISTS uq_tasks_phase1_scout_per_deal
ON tasks (crm_deal_id, workflow_type)
WHERE workflow_type = 'phase1_scout' AND deleted_at IS NULL;
```

---

### BUG-3: Empty Boards — 22/30 Acquisition Projects Have No Tasks

**Root cause.** Downstream consequence of BUG-1. Projects and CRM deals exist,
but deals are at `qualification` so Intel-stage automation never fired and
no Phase 1 tasks were created.

**Fix.** After BUG-1 migration runs, identify and backfill:

```sql
-- Deals that need Scout backfill
SELECT d.id, d.name, d.crm_contact_id
FROM crm_deals d
WHERE d.stage = 'Intel'
  AND d.crm_contact_id IS NOT NULL
  AND NOT EXISTS (
    SELECT 1 FROM tasks t
    WHERE t.crm_deal_id = d.id
      AND (t.title LIKE 'Phase 1 Research:%' OR t.workflow_type = 'phase1_scout')
      AND t.deleted_at IS NULL
  );
```

For each row: call `POST /api/crm/deals/:id/trigger-scout` (or invoke
`trigger_who_is_research()` directly via admin script).

---

### BUG-4: E2E Test Pollution in Legendary Forms

**Location.** `e2e/helpers/navigation.ts` lines 12-18 (`navigateToFirstProjectTasks`).

**Root cause.** E2E tests call `GET /api/projects` and take `projects[0]`.
Currently returns Legendary Forms. Test creates `[E2E] Kanban Test <timestamp>`
tasks. Cleanup in `e2e/helpers/cleanup.ts` is fire-and-forget (wrapped in
`.catch(() => {})`) — interrupted test runs leave orphan tasks.

**Fix A (immediate DB cleanup):**

```sql
-- crates/db/migrations/20260518510000_cleanup_e2e_test_tasks.sql
DELETE FROM tasks
WHERE title LIKE '[E2E]%' AND deleted_at IS NULL;
```

**Fix B (prevent recurrence).** In `navigation.ts`, replace `projects[0]` with
a dedicated `[E2E] Test Project` — create it in `beforeAll`, delete in
`afterAll`. Never touch named client/acquisition boards.

Fix `cleanup.ts`: replace silent `.catch(() => {})` with `.catch(e => console.warn('[E2E cleanup failed]', e))`.

---

### BUG-5: Astra → Cash Chain — Silent Failures at Proposal Stage

**Location.** `crates/server/src/routes/crm_deal_automations.rs` lines 856-868.

**Root cause.** After Astra Pass 2, `generate_proposal_background()` runs inside
a `tokio::spawn`. On failure it logs at `warn` level only:

```rust
Err(e) => tracing::warn!("Cash auto-generation failed for deal {}: {}", deal_id, e),
```

No visibility task. No activity log. No way for an operator to know Cash failed.
Deal stalls at Proposal with `proposal_text = NULL` indefinitely.

Same pattern in `generate_deck_background()` for Lux (line 1124).

**Fix.** Add visibility task + structured failure path to both functions:

```rust
async fn generate_proposal_background(pool: &sqlx::SqlitePool, deal_id: DbUuid) {
    let task_id = create_agent_visibility_task(
        pool, &deal_id,
        "Cash: Writing Proposal",
        "Cash is generating the proposal from Astra's enhanced briefing.",
        "inprogress",
    ).await;

    match generate_proposal_core(pool, &deal_id).await {
        Ok(_) => {
            if let Some(tid) = task_id { update_task_status(pool, &tid, "done").await; }
        }
        Err(e) => {
            tracing::error!("[Cash] Proposal failed for deal {}: {}", deal_id, e);
            if let Some(tid) = task_id {
                update_task_status_with_note(pool, &tid, "todo",
                    &format!("Cash failed: {}. Retry via /generate-proposal.", e)).await;
            }
            log_deal_activity(pool, &deal_id, "agent_failure",
                &format!("Cash proposal generation failed: {}", e)).await;
        }
    }
}
```

Apply identical pattern to `generate_deck_background()`.

---

### BUG-6: Won → Delivery Project Missing `client_id`

**Location.** `crates/server/src/routes/crm_deal_automations.rs` line 1407.

**Root cause.** `provision_won_deal()` computes `client_id` (line 1331-1377) but
the subsequent `INSERT INTO projects` omits it from the column list. Projects
with `NULL client_id` are treated as internal work. The Command Center AR
counter filters `WHERE client_id IS NOT NULL`.

**Fix (code).** Add `client_id` to the INSERT at line 1406:

```rust
sqlx::query(
    "INSERT INTO projects
     (id, name, git_repo_path, organization_id, client_id,
      created_at, updated_at)
     VALUES (?, ?, ?, ?, ?, datetime('now','subsec'), datetime('now','subsec'))"
)
.bind(&pid)
.bind(&project_name)
.bind(&repo_path)
.bind(&deal.organization_id)
.bind(&client_id)   // ← the fix
.execute(pool).await
```

**Fix (DB backfill).** Migration for existing Won deal:

```sql
-- crates/db/migrations/20260518520000_backfill_project_client_id.sql
UPDATE projects
SET
  client_id = (
    SELECT cl.id
    FROM clients cl
    JOIN crm_deals d ON d.project_id = projects.id
    JOIN crm_contacts cc ON cc.id = d.crm_contact_id
    WHERE cl.organization_id = projects.organization_id
      AND (
        lower(cl.name) = lower(COALESCE(cc.company_name, ''))
        OR lower(cl.name) = lower(d.name)
      )
    LIMIT 1
  ),
  updated_at = datetime('now', 'subsec')
WHERE client_id IS NULL
  AND id IN (
    SELECT project_id FROM crm_deals
    WHERE won_at IS NOT NULL AND project_id IS NOT NULL
  );
```

---

## 4. Sprint Plan

### Sprint 1 — DB Repair (SQL only, no code changes)

**Goal:** Unblock 31 stuck deals. Clean test pollution. Backfill client_id.  
**Risk:** Low — SQL only, fully reversible.  
**Duration:** 1 day.

```
  S1-1  Backup db.sqlite
        cp db.sqlite backups/pre-pipeline-repair-$(date +%Y%m%d).sqlite

  S1-2  20260518500000_fix_qualification_stage.sql       (BUG-1)
        → moves 31 deals to Intel
        → verify: SELECT COUNT(*) FROM crm_deals WHERE stage='qualification' = 0

  S1-3  20260518510000_cleanup_e2e_test_tasks.sql        (BUG-4)
        → deletes [E2E] Kanban Test artifacts
        → verify: SELECT COUNT(*) FROM tasks WHERE title LIKE '[E2E]%' = 0

  S1-4  20260518520000_backfill_project_client_id.sql    (BUG-6 partial)
        → backfills client_id on Won deal projects
        → verify: all rows in result set have non-NULL client_id

  S1-5  Scout backfill for 31 now-unblocked deals
        → run query from BUG-3 fix section to identify targets
        → call POST /api/crm/deals/:id/trigger-scout for each
```

---

### Sprint 2 — Deduplication + Scout Chain Hardening

**Goal:** Fix BUG-2 (Scout dedup). Fix BUG-4 (E2E test isolation).  
**Risk:** Low — additive guards only.  
**Duration:** 1-2 days.

```
  S2-1  Expand dedup query in trigger_who_is_research()         (BUG-2)
        crm_deal_automations.rs lines 162-169
        Add: OR workflow_type = 'phase1_scout' to the COUNT query

  S2-2  20260518530000_uq_phase1_scout_per_deal.sql              (BUG-2)
        DB-level unique index as dedup backstop

  S2-3  Delete 2 duplicate Scout tasks on Bad Girl Strength Club (BUG-2)
        SQL: DELETE FROM tasks WHERE title = 'Scout: Competitor Deep Dive'
             AND id NOT IN (
               SELECT id FROM tasks
               WHERE title = 'Scout: Competitor Deep Dive'
               ORDER BY created_at ASC LIMIT 1
             )

  S2-4  Fix e2e/helpers/navigation.ts — replace projects[0]     (BUG-4)
        with dedicated [E2E] Test Project (create in beforeAll, delete in afterAll)

  S2-5  Fix e2e/helpers/cleanup.ts — log failures instead of    (BUG-4)
        silently swallowing them
```

---

### Sprint 3 — Cash/Lux Visibility + Failure Recovery

**Goal:** Fix BUG-5. Make agent failures visible and recoverable.  
**Risk:** Medium — changes automation hot paths. Needs staging smoke test.  
**Duration:** 2-3 days.

```
  S3-1  Extract create_agent_visibility_task() helper
        crm_deal_automations.rs — private async fn
        Creates task linked to crm_deal_id, returns Option<DbUuid>

  S3-2  Extract update_task_status() / update_task_status_with_note() helpers
        Same file — thin wrappers over UPDATE tasks SET status = ?, description = ?

  S3-3  Extract log_deal_activity() helper
        Thin wrapper over INSERT INTO crm_activities

  S3-4  Wrap generate_proposal_background() with visibility task + error path
        crm_deal_automations.rs lines 862-868  (see BUG-5 fix above)

  S3-5  Apply same pattern to generate_deck_background() for Lux
        crm_deal_automations.rs lines 1124-1129

  S3-6  Smoke test: trigger Proposal stage →
        verify "Cash: Writing Proposal" task appears →
        verify task status = 'done' after success →
        verify crm_activities row exists

  S3-7  Failure simulation: mock LLM to return error →
        verify task status = 'todo' with error note →
        verify agent_failure activity row created
```

---

### Sprint 4 — Won → Delivery Provisioning Fix

**Goal:** Fix BUG-6 code fix. Verify Won deal creates a proper client project.  
**Risk:** Low — isolated INSERT change.  
**Duration:** 1 day.

```
  S4-1  Add client_id to INSERT INTO projects in provision_won_deal()
        crm_deal_automations.rs line 1407  (see BUG-6 fix above)

  S4-2  Add assertion in pipeline E2E: after mark_deal_won →
        projects row has client_id = <expected client id>

  S4-3  Verify Command Center AR counter picks up Bad Girl Strength Club
        after Sprint 1 backfill migration ran

  S4-4  Verify delivery pipeline deal was created (pipeline_type='delivery')
        for Stephanie Bruno / Bad Girl Strength Club
```

---

## 5. Validation Checklist

### DB state

```
[ ] SELECT COUNT(*) FROM crm_deals WHERE stage = 'qualification'
    → 0

[ ] SELECT COUNT(*) FROM tasks WHERE title LIKE '[E2E]%' AND deleted_at IS NULL
    → 0

[ ] SELECT COUNT(*) FROM tasks
      WHERE title = 'Scout: Competitor Deep Dive'
        AND crm_deal_id = '<bad-girl-strength-club-deal-id>'
    → 1 (not 3)

[ ] SELECT client_id FROM projects
      WHERE id IN (SELECT project_id FROM crm_deals WHERE won_at IS NOT NULL)
    → all rows have non-NULL client_id

[ ] SELECT COUNT(*) FROM crm_deals
      WHERE stage = 'Intel' AND crm_contact_id IS NOT NULL
    → 31+ deals ready for Scout
```

### Stage transition automation

```
[ ] Create test deal at Intel with valid crm_contact_id.
    Within 60s verify:
    - crm_contacts.intelligence_status: idle → queued → complete
    - Exactly 1 "Phase 1 Research: <contact>" task for this deal
    - Exactly 1 "Phase 1 Research: <company>" task for this deal
    - Calling Intel trigger a SECOND time creates NO additional tasks

[ ] Advance to Business Analysis. Within 60s verify:
    - business_reports row exists with crm_deal_id = deal.id
    - business_reports.status = 'draft' or 'complete'

[ ] Advance to Proposal. Within 120s verify:
    - "Cash: Writing Proposal" task exists, status = 'inprogress'
    - business_reports.status = 'enhanced' (Astra Pass 2 ran)
    - crm_deals.proposal_text IS NOT NULL
    - "Cash: Writing Proposal" task status = 'done'

[ ] Simulate Cash LLM failure. Verify:
    - "Cash: Writing Proposal" task status = 'todo' with error note
    - crm_activities row: activity_type = 'agent_failure'
    - Deal is NOT silently stuck (operator can see the failure)

[ ] Advance to Polish. Within 120s verify:
    - "Lux: Building Deck" task exists, status = 'inprogress'
    - crm_deals.deck_url IS NOT NULL after success
    - "Lux: Building Deck" task status = 'done'
```

### Won provisioning

```
[ ] Mark test deal Won. Immediately verify:
    - crm_deals.won_at IS NOT NULL
    - clients row exists for contact's company
    - projects row: client_id = clients.id (BUG-6 fix)
    - projects row: organization_id IS NOT NULL
    - crm_deals.project_id = new project's id
    - tasks exist under that project_id
    - vibe_transactions row: transaction_type = 'deal_won'
    - delivery pipeline deal created (pipeline_type = 'delivery')

[ ] Command Center → AR tab shows Won deal's company in AR list
```

### E2E isolation

```
[ ] Run health-check.spec.ts to completion.
    After suite: zero "[E2E]" tasks on Legendary Forms or any acquisition board.
    The [E2E] Test Project itself deleted in afterAll.

[ ] Kill health-check.spec.ts mid-run. Re-run cleanup manually.
    Zero orphan "[E2E]" tasks remain on any live board.
```

### Board completeness

```
[ ] After Scout backfill completes:
    SELECT COUNT(*) FROM projects p
    WHERE p.id IN (SELECT project_id FROM crm_deals WHERE stage = 'Intel')
      AND (SELECT COUNT(*) FROM tasks t WHERE t.project_id = p.id) = 0
    → 0  (no empty boards among Intel-stage deals)
```

---

## Key File Locations

| Component | File | Lines |
|---|---|---|
| Scout research trigger | `crates/server/src/routes/crm_deal_automations.rs` | 33–202 |
| Phase 1 task dedup guard | `crates/server/src/routes/crm_deal_automations.rs` | 162–196 |
| Phase 1 business report (Astra) | `crates/server/src/routes/crm_deal_automations.rs` | 343–600 |
| Astra Pass 2 + Cash chain | `crates/server/src/routes/crm_deal_automations.rs` | 675–868 |
| Cash background fn (no visibility task) | `crates/server/src/routes/crm_deal_automations.rs` | 862–868 |
| Lux background fn (no visibility task) | `crates/server/src/routes/crm_deal_automations.rs` | 1124–1129 |
| Won provisioning + missing client_id | `crates/server/src/routes/crm_deal_automations.rs` | 1299–1551 |
| Project INSERT (missing client_id) | `crates/server/src/routes/crm_deal_automations.rs` | 1406–1416 |
| Intel-stage auto-trigger | `crates/server/src/stage_transition.rs` | 554–562 |
| Stage entry actions | `crates/server/src/stage_transition.rs` | 544–638 |
| Won transition handler | `crates/server/src/stage_transition.rs` | 641–738 |
| Deal advance validation | `crates/server/src/routes/crm_deal_transitions.rs` | 80–348 |
| Intake → CRM deal creation | `crates/server/src/routes/intake/pipeline.rs` | 884–1019 |
| Workflow task seeding | `crates/server/src/routes/intake/pipeline.rs` | 1207–1397 |
| E2E cleanup helper | `frontend/tests/e2e/helpers/cleanup.ts` | 39–72 |
| E2E navigation helper (projects[0] bug) | `frontend/tests/e2e/helpers/navigation.ts` | 6–21 |
