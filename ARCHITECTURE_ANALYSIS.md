# PCG CRM/Pipeline Architecture Analysis
> Generated: 2026-03-25 | Based on live schema PRAGMA table_info + live data audit

---

## 1. Core Entity Relationship Diagram

```mermaid
erDiagram
    %% ─── IDENTITY LAYER (triplication problem) ───
    users {
        BLOB id PK
        TEXT username
        TEXT email
        TEXT full_name
        TEXT password_hash
        INT  is_admin
        TEXT user_role
        REAL vibe_balance
    }
    persons {
        BLOB   id PK
        TEXT   full_name
        TEXT   email
        TEXT   user_id FK
        TEXT   company_id FK
        TEXT   company_org_id FK
        TEXT   person_type
        TEXT   financial_role
        TEXT   assigned_to FK
        INT    research_pass_count
        TEXT   research_depth
        TEXT   intelligence_summary
        TEXT   intelligence_updated_at
    }
    crm_contacts {
        BLOB id PK
        TEXT person_id FK
        TEXT crm_pipeline_id FK
        TEXT contact_name
        TEXT contact_email
        TEXT contact_phone
        TEXT company
        TEXT stage
        TEXT notes
        TEXT tags
    }

    users ||--o| persons : "user_id (soft TEXT FK, undeclared)"
    persons ||--o| crm_contacts : "person_id (soft TEXT FK, undeclared)"

    %% ─── ORGANIZATION LAYER ───
    organizations {
        BLOB id PK
        TEXT name
        TEXT company_id FK
        TEXT invite_token
        TEXT pending_owner_email
        TEXT created_by_org_id FK
        TEXT address
    }
    companies {
        BLOB id PK
        TEXT name
        TEXT website
        TEXT logo_url
        TEXT created_by_org_id FK
        TEXT intelligence_summary
        TEXT intelligence_score
        TEXT headquarters
    }
    clients {
        BLOB id PK
        BLOB organization_id FK
        BLOB crm_contact_id FK
        TEXT name
        TEXT email
        TEXT phone
        TEXT status
    }
    projects {
        BLOB id PK
        TEXT name
        BLOB organization_id FK
        BLOB client_id FK
        TEXT slug
        TEXT project_status
        TEXT waiting_on_client
        REAL client_budget_vibe
        BLOB account_manager_id FK
        BLOB pm_ep_id FK
    }

    organizations ||--o{ projects : "organization_id"
    organizations }o--o| companies : "company_id (org's brand entity)"
    companies ||--o{ persons : "company_id"
    organizations ||--o{ clients : "organization_id"
    clients }o--o| crm_contacts : "crm_contact_id"
    projects }o--o| clients : "client_id ⚠️ NULL on all 38 live deals"

    %% ─── CRM PIPELINE LAYER ───
    crm_pipelines {
        BLOB id PK
        BLOB organization_id FK
        BLOB project_id FK
        TEXT name
        TEXT pipeline_type
        TEXT status
    }
    crm_pipeline_stages {
        BLOB id PK
        BLOB pipeline_id FK
        TEXT name
        INT  position
        TEXT stage_type
        TEXT stage_config
        TEXT color
    }
    crm_deals {
        BLOB id PK
        BLOB crm_stage_id FK
        BLOB project_id FK
        BLOB client_id FK
        TEXT stage
        REAL amount
        TEXT currency
        TEXT title
        TEXT description
        TEXT won_at
        TEXT actual_close_date
        TEXT proposal_status
        TEXT deck_url
        TEXT win_reason
        TEXT loss_reason
    }
    proposals {
        BLOB   id PK
        BLOB   project_id FK
        BLOB   organization_id FK
        TEXT   contact_ids
        TEXT   title
        TEXT   status
        REAL   amount
        TEXT   currency
        TEXT   tier
        TEXT   signed_at
        TEXT   sent_at
    }

    crm_pipelines }o--o| organizations : "organization_id"
    crm_pipelines }o--o| projects : "project_id (org-level pipelines have NULL)"
    crm_pipeline_stages ||--o{ crm_deals : "crm_stage_id"
    crm_pipeline_stages }|--|| crm_pipelines : "pipeline_id"
    crm_deals }o--o| projects : "project_id"
    crm_deals }o--o| clients : "client_id ⚠️ NULL on all live deals"
    proposals }o--o| projects : "project_id"
    proposals }o--o| organizations : "organization_id ⚠️ NULL in live data"

    %% ─── TASK / BOARD LAYER ───
    project_boards {
        BLOB id PK
        BLOB project_id FK
        TEXT name
        TEXT board_type
        TEXT description
    }
    tasks {
        BLOB id PK
        BLOB board_id FK
        BLOB crm_deal_id FK
        TEXT title
        TEXT description
        TEXT status
        TEXT priority
        BLOB assigned_to FK
        TEXT due_date
        INT  position
    }

    project_boards }|--|| projects : "project_id"
    tasks }|--|| project_boards : "board_id"
    tasks }o--o| crm_deals : "crm_deal_id ⚠️ never populated in live data"
    tasks }o--o| users : "assigned_to"

    %% ─── DELIVERABLES LAYER ───
    deliverables {
        BLOB id PK
        BLOB project_id FK
        BLOB proposal_id FK
        TEXT title
        TEXT deliverable_type
        TEXT status
        TEXT review_stage
        INT  revision_count
        TEXT due_date
        BLOB assigned_to FK
    }

    deliverables }|--|| projects : "project_id"
    deliverables }o--o| proposals : "proposal_id ⚠️ mostly NULL"
    deliverables }o--o| users : "assigned_to"

    %% ─── KNOWLEDGE LAYER ───
    project_knowledge_sources {
        BLOB id PK
        TEXT owner_type
        TEXT owner_id FK
        TEXT source_type
        TEXT source_id
        TEXT source_title
        TEXT source_summary
        REAL coverage_score
        INT  is_active
        INT  is_stale
    }

    %% ─── INTAKE LAYER ───
    call_intake_items {
        BLOB id PK
        TEXT source
        TEXT raw_content
        TEXT extracted_data
        TEXT status
        TEXT created_at
    }
    deal_transcripts {
        BLOB id PK
        BLOB deal_id FK
        BLOB intake_item_id FK
        TEXT transcript_type
    }
    business_reports {
        BLOB id PK
        BLOB company_id FK
        BLOB person_id FK
        BLOB crm_deal_id FK
        TEXT report_type
        TEXT content
        TEXT generated_at
    }

    deal_transcripts }|--|| crm_deals : "deal_id"
    deal_transcripts }|--|| call_intake_items : "intake_item_id"
    business_reports }o--o| companies : "company_id"
    business_reports }o--o| persons : "person_id"
    business_reports }o--o| crm_deals : "crm_deal_id"

    %% ─── BRAND LAYER ───
    company_brand_profiles {
        BLOB id PK
        TEXT company_id FK
        TEXT primary_color
        TEXT secondary_color
        TEXT accent_color
        TEXT typography_heading
        TEXT typography_body
        TEXT logo_url
        TEXT tagline
        TEXT mission
        TEXT vision
    }
    organization_brand_profiles {
        BLOB id PK
        TEXT organization_id FK
        TEXT primary_color
        TEXT secondary_color
        TEXT accent_color
        TEXT typography_heading
        TEXT typography_body
        TEXT logo_url
        TEXT tagline
        TEXT mission
        TEXT vision
    }

    company_brand_profiles }|--|| companies : "company_id"
    organization_brand_profiles }|--|| organizations : "organization_id"
```

---

## 2. Intended vs. Actual Pipeline Flow

```mermaid
flowchart TD
    subgraph INTENDED["🎯 INTENDED DEAL FLOW"]
        direction TB
        I1[Intake Email / Call] --> I2[call_intake_items]
        I2 --> I3[Person Created / Matched]
        I3 --> I4[Company Entity Created]
        I4 --> I5[Client Record Created\nclients table]
        I5 --> I6[CRM Contact Created\nlinked to Person + Client]
        I6 --> I7[CRM Deal Created\nproject_id + client_id + person link]
        I7 --> I8[Intel Stage\nResearch Pass]
        I8 --> I9[Business Analysis Artifact]
        I9 --> I10[Discovery Stage\nProposal Created\norg_id + project_id + contact_ids]
        I10 --> I11[Polish Stage\nDeliverables auto-generated from Proposal]
        I11 --> I12[Won Stage\nDeliverables linked to Proposal\nProject Board auto-created\nTasks from Deliverables]
        I12 --> I13[Knowledge Graph updated\nall artifacts registered]
    end

    subgraph ACTUAL["⚠️ ACTUAL DATA FLOW (live DB audit)"]
        direction TB
        A1[Email to Nora] --> A2[call_intake_items row created]
        A2 --> A3[Person row created\n✅ full intel fields populated]
        A3 --> A4[Company row created\n✅ brand profile added]
        A4 -->|❌ client record NOT created| A5[CRM Deal Created\nclient_id = NULL\nproject_id = project BLOB]
        A5 --> A6[Pipeline stages advanced manually\nno auto-triggers firing]
        A6 --> A7[Proposal created separately\norganization_id = NULL\nnot FK-linked to deal]
        A7 -->|❌ deliverables not auto-generated| A8[Deliverables created manually\nproposal_id = NULL on most]
        A8 -->|❌ tasks not linked to deal| A9[Tasks created on board\ncrm_deal_id = NULL]
        A9 -->|❌ KG registered via direct SQL only| A10[Knowledge sources in DB\nbut not queryable from deal card UI]
    end

    style INTENDED fill:#1a2e1a,stroke:#2d5a2d,color:#90ee90
    style ACTUAL fill:#2e1a1a,stroke:#5a2d2d,color:#ff9999
```

---

## 3. Identity Layer: Triplication Problem

```mermaid
flowchart LR
    subgraph AUTH["🔐 Auth Layer"]
        U[users\nid BLOB\nusername\nemail\npassword_hash\nuser_role\nvibe_balance]
    end

    subgraph UNIVERSAL["🌐 Universal Identity"]
        P[persons\nid BLOB\nfull_name\nemail\ncompany_id → companies\nuser_id → users TEXT\nassigned_to → users\nperson_type\nfinancial_role\nintelligence fields\nresearch_pass_count]
    end

    subgraph CRM["📋 CRM Layer"]
        C[crm_contacts\nid BLOB\nperson_id → persons TEXT\ncrm_pipeline_id\ncontact_name\ncontact_email\ncompany\nstage\nnotes\ntags]
    end

    subgraph CLIENTS["🏢 Client Records"]
        CL[clients\nid BLOB\norganization_id\ncrm_contact_id → crm_contacts\nname\nemail\nphone\nstatus]
    end

    U -.->|"soft bridge\nno FK declared\nuser_id TEXT"| P
    P -.->|"soft bridge\nno FK declared\nperson_id TEXT"| C
    C -.->|"crm_contact_id BLOB\n✅ declared FK"| CL

    P -->|"company_id TEXT"| CO[companies]
    P -->|"company_org_id TEXT"| ORG[organizations]

    PROB1["⚠️ PROBLEM: Same contact exists\nin 3 tables with no enforced link.\nDeleting a person does NOT cascade\nto crm_contacts or clients."]
    PROB2["⚠️ PROBLEM: crm_contacts has its own\ncontact_name/email fields duplicating\npersons.full_name/email with no sync."]
    PROB3["⚠️ PROBLEM: proposals.contact_ids\nis a JSON array of crm_contact IDs\nbut the UI needs person intelligence\nwhich is on the persons table."]

    style AUTH fill:#1a1a2e,stroke:#2d2d5a,color:#9999ff
    style UNIVERSAL fill:#1a2e2e,stroke:#2d5a5a,color:#99ffff
    style CRM fill:#2e2e1a,stroke:#5a5a2d,color:#ffff99
    style CLIENTS fill:#2e1a2e,stroke:#5a2d5a,color:#ff99ff
    style PROB1 fill:#3a1a1a,stroke:#aa3333,color:#ff9999
    style PROB2 fill:#3a1a1a,stroke:#aa3333,color:#ff9999
    style PROB3 fill:#3a1a1a,stroke:#aa3333,color:#ff9999
```

---

## 4. Knowledge Graph Connection Map

```mermaid
flowchart TD
    subgraph KG["🧠 project_knowledge_sources (polymorphic)"]
        KGS["owner_type: 'project'|'company'|'user'|'organization'\nowner_id: TEXT UUID\nsource_type: conversation|artifact|entity|context_injection|topology_snapshot|pulse_content\nsource_id: TEXT\nsource_title: TEXT\ncoverage_score: REAL"]
    end

    subgraph OWNERS["Queryable Owners"]
        PR[projects\nowner_type='project']
        CO[companies\nowner_type='company']
        ORG[organizations\nowner_type='organization']
        US[users\nowner_type='user']
    end

    subgraph MISSING["❌ NOT Queryable from KG"]
        CD[crm_deals]
        CP[crm_pipelines]
        DE[deliverables]
        TA[tasks]
    end

    PR --> KGS
    CO --> KGS
    ORG --> KGS
    US --> KGS

    CD -.->|"❌ no owner_type='deal'\nDeal card UI cannot\nquery its own context"| KGS
    DE -.->|"❌ deliverable artifacts\nonly queryable via project"| KGS
    TA -.->|"❌ tasks have no KG link"| KGS

    PROB["⚠️ The CRM Deal card is the PRIMARY\ncontext for an account manager.\nBut deal-specific knowledge must be\nqueried by project_id, not deal_id.\nIf a project has multiple deals,\nall deals share the same KG context —\nno deal-level knowledge isolation."]

    style KG fill:#1a2e1a,stroke:#2d5a2d,color:#90ee90
    style OWNERS fill:#1a1a2e,stroke:#2d2d5a,color:#9999ff
    style MISSING fill:#2e1a1a,stroke:#5a2d2d,color:#ff9999
    style PROB fill:#3a1a1a,stroke:#aa3333,color:#ff9999
```

---

## 5. Gap Analysis: Critical Issues

### GAP-01: `client_id` Never Populated on Deals
| Field | Location | Live Value |
|-------|----------|------------|
| `crm_deals.client_id` | BLOB FK → clients | `NULL` on all 38 deals |
| `projects.client_id` | BLOB FK → clients | `NULL` on all projects |

**Impact:** The `clients` table is entirely orphaned from the pipeline. Client billing, history, and portal access cannot be linked to live deals. The `clients` table has `crm_contact_id` but `crm_contacts` don't cascade to `persons` intelligence.

**Fix:** When a deal is created, auto-populate `client_id` from the `clients` table using `crm_contact.id → clients.crm_contact_id`. If no client row exists, create it.

---

### GAP-02: Identity Bridges Are Undeclared (Soft TEXT FKs)
| Bridge | Declared? | Risk |
|--------|-----------|------|
| `persons.user_id → users.id` | ❌ TEXT, no FK | Delete user → orphaned person |
| `crm_contacts.person_id → persons.id` | ❌ TEXT, no FK | Delete person → orphaned CRM contact |
| `persons.company_id → companies.id` | ❌ TEXT, no FK | Delete company → orphaned persons |
| `crm_deals.project_id → projects.id` | ✅ BLOB FK | OK |

**Impact:** No CASCADE behavior. Deleting a person doesn't remove their CRM contacts. Searches by person ID across the CRM contact table fail unless the caller knows to join on TEXT equality.

**Fix (short-term):** Add `PRAGMA foreign_keys = ON` enforcement + application-level cascade on delete. Long-term: normalize all person bridges to a single identity table.

---

### GAP-03: `proposals.organization_id = NULL` in Live Data
| Field | Live Value |
|-------|------------|
| `proposals.organization_id` | `NULL` on all live proposals |
| `proposals.project_id` | Populated |

**Impact:** Proposals cannot be queried at the org level for agency reporting. Revenue attribution by client organization is impossible from the proposals table. Tier/contract data is not visible in org-level dashboards.

**Fix:** Set `organization_id` on proposal creation from `projects.organization_id`.

---

### GAP-04: `deliverables.proposal_id = NULL` (Broken Chain)
The intended chain is: **Proposal → Won → Deliverables → Tasks → Done**

| Link | Status |
|------|--------|
| `proposals` → create `deliverables` | ❌ NOT auto-triggered |
| `deliverables.proposal_id` | NULL on most rows |
| `deliverables` → create `tasks` | ❌ NOT auto-triggered |
| `tasks.crm_deal_id` | NULL on all live tasks |

**Impact:** Winning a deal does not automatically scaffold the delivery workflow. PMs must manually create boards, tasks, and deliverables with no linkage back to the proposal scope/value.

**Fix:** On `crm_deals` stage transition to `won`:
1. Look up linked `proposal` via `project_id`
2. Auto-create `deliverables` from proposal line items (or a deliverable template)
3. Set `deliverables.proposal_id`
4. Auto-create `project_boards` entry (if none)
5. Create `tasks` from deliverable types with `crm_deal_id` set

---

### GAP-05: Deal Card Has No Direct Knowledge Graph Scope
| Problem | Detail |
|---------|--------|
| `project_knowledge_sources` uses `owner_type='project'` | All deals for a project share the same KG |
| No `owner_type='deal'` | Deal-specific context (transcripts, proposals) not isolated |
| `deal_transcripts` exists | But transcripts are not registered as KG sources with `owner_id = deal_id` |

**Impact:** An account manager viewing a deal card cannot query "what do we know about this specific deal?" — only "what do we know about this project?" If a project has multiple client deals (e.g., upsell + renewal), their knowledge contexts bleed together.

**Fix:** Add `owner_type='deal'` support to `project_knowledge_sources`. Register deal-specific artifacts (transcripts, proposals, contracts) under `owner_id = crm_deal.id`.

---

### GAP-06: Stage Auto-Triggers Never Fire
The `crm_pipeline_stages.stage_config` JSON has `on_enter_actions` and `auto_trigger` fields, but:
- No server-side observer watches stage transitions
- `routes/automations.rs` runs 5 hourly jobs — none trigger on stage change events
- Nora's "Cash" (Proposal) and "Lux" (Polish) agent auto-triggers are defined in config but have no handler

**Impact:** Every pipeline stage advance is purely a status column update. No intelligence gathering, proposal drafting, or brand research is automatically initiated.

**Fix:** Add a `handle_stage_transition(deal_id, from_stage, to_stage)` function called from the deal PATCH handler that executes `on_enter_actions` from the new stage's `stage_config`.

---

### GAP-07: `crm_contacts` Duplicates `persons` Data
`crm_contacts` has: `contact_name`, `contact_email`, `contact_phone`, `company`
`persons` has: `full_name`, `email`, `phones` (JSON), `company_id`

These fields are populated independently with no sync. A phone number update on `persons` does not update `crm_contacts`. The CRM contact card displayed in the pipeline shows stale data.

**Fix:** Remove `contact_name`, `contact_email`, `contact_phone`, `company` from `crm_contacts` and JOIN to `persons` at query time via `person_id`. The `crm_contacts` table should be a thin pipeline-card entity, not a contact data store.

---

### GAP-08: `task_boards` / `task_cards` Don't Exist (Memory Drift)
Internal documentation and some code comments reference `task_boards` and `task_cards`. Actual tables are `project_boards` and `tasks` (with `tasks.board_id`). This naming drift causes confusion in new feature development.

**Fix:** Update all MEMORY.md entries and any frontend references to use `project_boards` / `tasks`.

---

## 6. Priority Fix Roadmap

```mermaid
gantt
    title Architecture Fix Priority
    dateFormat  YYYY-MM-DD
    section Critical (Break Data Integrity)
        Populate client_id on deals + projects       :crit, a1, 2026-03-25, 3d
        Add FK declarations for person bridges        :crit, a2, 2026-03-25, 2d
        Set proposals.organization_id on creation     :crit, a3, 2026-03-26, 1d
    section High (Break Pipeline Flow)
        Stage auto-trigger handler                    :active, b1, 2026-03-27, 5d
        Won→ deliverables + tasks auto-scaffold       :b2, after b1, 3d
        Deal-level KG scope (owner_type=deal)         :b3, 2026-03-28, 4d
    section Medium (Data Quality)
        Strip duplicate fields from crm_contacts      :c1, 2026-04-01, 2d
        Link deliverables.proposal_id on create       :c2, 2026-04-01, 2d
        Register deal transcripts as KG sources       :c3, 2026-04-02, 2d
    section Low (Housekeeping)
        Update MEMORY.md table name references        :d1, 2026-04-01, 1d
        Add proposals monthly revenue view            :d2, 2026-04-03, 2d
```

---

## 7. Recommended Schema Additions

```sql
-- 1. Add FK declarations (migration)
ALTER TABLE persons ADD COLUMN user_id_blob BLOB REFERENCES users(id);
-- (migrate user_id TEXT → user_id_blob BLOB for FK enforcement)

-- 2. Add owner_type='deal' support (already works — just use it)
-- Register deal artifacts:
INSERT INTO project_knowledge_sources (owner_type, owner_id, source_type, source_id, ...)
VALUES ('deal', hex(crm_deal_id), 'conversation', intake_item_id, ...);

-- 3. Auto-set client_id on deal creation trigger
CREATE TRIGGER set_deal_client_id AFTER INSERT ON crm_deals
WHEN NEW.client_id IS NULL AND NEW.project_id IS NOT NULL
BEGIN
  UPDATE crm_deals SET client_id = (
    SELECT id FROM clients WHERE organization_id = (
      SELECT organization_id FROM projects WHERE id = NEW.project_id
    ) LIMIT 1
  ) WHERE id = NEW.id;
END;

-- 4. Stage transition event log (enables auto-trigger handler)
CREATE TABLE IF NOT EXISTS deal_stage_transitions (
  id         BLOB PRIMARY KEY,
  deal_id    BLOB NOT NULL REFERENCES crm_deals(id),
  from_stage TEXT,
  to_stage   TEXT NOT NULL,
  triggered_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
  actions_fired TEXT DEFAULT '[]'
);
```

---

*End of Architecture Analysis — PCG CRM/Pipeline System v0.1*
