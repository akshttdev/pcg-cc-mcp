# Social Calendar & Knowledge Graph Architecture Plan
**Date**: 2026-05-18  
**Sprint context**: Stage 0 / Month 2 — Priority 4 "Social Media Pipeline Migration"  
**Roadmap ref**: `planning/roadmap/5-year-product-roadmap.md`

---

## 1. Problem Statement

The current social calendar is flat: every `social_post` and `social_account` is pinned to a `project_id`. There is no concept of organization-owned content, individual-owned content, or a clear knowledge graph boundary between what an org knows vs. what a client is permitted to know. This blocks both Stage 0 (PCG dogfooding) and Stage 1 (Sirak Studios as a managed pilot client).

---

## 2. Entity Ownership Hierarchy

The platform has three principal types that can own social accounts, post content, and accumulate knowledge:

```
┌─────────────────────────────────────────────────────────────────┐
│                        ORCHA Platform                           │
│                                                                 │
│  ┌──────────────┐                                               │
│  │  Individual  │  e.g. Sirak personally                        │
│  │   (User)     │  ← personal IG, X, LinkedIn                  │
│  └──────┬───────┘                                               │
│         │ owns / is_member_of                                   │
│  ┌──────▼───────────────────────────────────────────────────┐   │
│  │              Organization  (e.g. Sirak Studios)           │   │
│  │                                                           │   │
│  │  ┌─────────────────────┐  ┌────────────────────────────┐ │   │
│  │  │  Org-owned Accounts │  │     Org Knowledge Graph    │ │   │
│  │  │  (brand channels)   │  │  (everything — workflow,   │ │   │
│  │  └─────────────────────┘  │   deliverables, analytics) │ │   │
│  │                           └────────────────────────────┘ │   │
│  │                                                           │   │
│  │  Projects (org's own work — no client_id)                │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │  project social posts  │  project KG               │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  │                                                           │   │
│  │  Clients (external companies who hire the org)           │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │  Client                                             │ │   │
│  │  │  ┌──────────────────────────────────────────────┐   │ │   │
│  │  │  │ Client Projects (work done FOR the client)   │   │ │   │
│  │  │  │  ├─ client-owned social accounts             │   │ │   │
│  │  │  │  ├─ client content calendar                  │   │ │   │
│  │  │  │  └─ client KG ← DELIVERABLES ONLY            │   │ │   │
│  │  │  └──────────────────────────────────────────────┘   │ │   │
│  │  │  (Client can upgrade → Organization in future)       │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  └───────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Knowledge Graph Boundaries

This is the critical data governance boundary. The org retains control of workflow data; the client receives only what is explicitly delivered to them.

```
┌─────────────────────────────────────────────────────┐
│              Org Knowledge Graph                    │
│           (project_knowledge_sources                │
│            WHERE owner_type = 'organization')       │
│                                                     │
│  ✓ All deliverables (published to client or not)   │
│  ✓ All social analytics (org + client accounts)    │
│  ✓ Internal workflow data, briefs, drafts           │
│  ✓ CRM intel, proposals, deal history               │
│  ✓ Client communications and meeting transcripts   │
│  ✓ Brand research, market intelligence              │
└────────────────────────┬────────────────────────────┘
                         │ explicit deliverable publish
                         │ (status = 'done' + client_visible = true)
                         ▼
┌─────────────────────────────────────────────────────┐
│              Client Knowledge Graph                 │
│           (project_knowledge_sources                │
│            WHERE owner_type = 'project'             │
│            AND project.client_id IS NOT NULL        │
│            AND client_visible = true)               │
│                                                     │
│  ✓ Finalized deliverables explicitly published     │
│  ✓ Approved content calendar posts                 │
│  ✓ Published analytics reports                     │
│  ✗ Internal drafts and workflow state              │
│  ✗ Internal briefs and pricing data                │
│  ✗ Other clients' data                             │
│                                                     │
│  → Client can eventually UPGRADE to Organization   │
│    (their KG becomes the seed for their new org KG)│
└─────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────┐
│             Individual Knowledge Graph              │
│              (user_knowledge_sources)               │
│                                                     │
│  ✓ Personal notes and research                     │
│  ✓ Personal social account analytics               │
│  ✓ Contacts the user has personally researched     │
│  ✗ Org workflow data (that's org-scoped)           │
│  ✗ Client data the user has access to via role     │
└─────────────────────────────────────────────────────┘
```

### Current KG state vs. target

| KG | Table today | Status | Gap |
|----|-------------|--------|-----|
| Individual | `user_knowledge_sources` | ✅ Table exists | No UI, no routes exposed to frontend |
| Organization | `project_knowledge_sources` WHERE `owner_type='organization'` | ⚠️ Polymorphic field exists but no dedicated routes | Org KG has no distinct API surface or UI |
| Project | `project_knowledge_sources` WHERE `owner_type='project'` | ✅ Works | Functions correctly |
| Client | `project_knowledge_sources` WHERE `owner_type='project'` and `client_visible=true` | ❌ `client_visible` column doesn't exist | No visibility gate between org and client KG |

---

## 4. Social Calendar Views

Three distinct calendar surfaces, each with different default scope and filter options:

```
┌──────────────────────────────────────────────────────────────────────┐
│              MY WORKSPACE → Calendar  (/calendar)                    │
│                                                                      │
│  DEFAULT VIEW:  Tasks + posts from user's personal social accounts  │
│  FILTER ON:     + posts from projects I'm a member of               │
│                 + posts from org accounts I manage                   │
│                                                                      │
│  Data source:   user_social_accounts (future)                        │
│                 + social_posts WHERE assignee_id = me               │
│  Stage:         DEFERRED (Phase D — no personal accounts wired yet) │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│         ORG SOCIAL  (/organizations/:id/social?sv=content)           │
│                                                                      │
│  DEFAULT VIEW:  Org-owned accounts + posts                          │
│                 (social_accounts WHERE organization_id = :org_id     │
│                  AND project_id IS NULL)                             │
│                                                                      │
│  FILTER: ☐ Show client content                                      │
│          └─ adds posts WHERE project.organization_id = :org_id       │
│          ☐ Filter by client: [dropdown]                             │
│          ☐ Filter by platform: [multi-select]                       │
│                                                                      │
│  Stage:  PHASE B — implement after DB migration                     │
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│      CLIENT SOCIAL  (/organizations/:id/clients/:cid?sv=content)     │
│                                                                      │
│  DEFAULT VIEW:  Posts from all projects WHERE client_id = :cid       │
│                 Shows only client-owned social accounts              │
│                                                                      │
│  FILTER: ☐ Filter by project: [dropdown]                            │
│          ☐ Filter by platform: [multi-select]                       │
│          ☐ Filter by status: [multi-select]                         │
│                                                                      │
│  Stage:  Already mostly correct — fix sv= param, add filters        │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 5. Data Model Changes

### 5a. Social Accounts & Posts — new ownership columns

```
CURRENT:
  social_accounts (id, project_id BLOB NOT NULL, platform, ...)
  social_posts    (id, project_id BLOB NOT NULL, ...)

TARGET:
  social_accounts (
    id,
    project_id      BLOB NULL,          -- client project account
    organization_id TEXT NULL,          -- org brand account  ← NEW
    user_id         TEXT NULL,          -- personal account   ← NEW (Phase D)
    platform,
    ...
    CHECK (
      (project_id IS NOT NULL) +
      (organization_id IS NOT NULL) +
      (user_id IS NOT NULL) = 1
    )
  )

  social_posts (
    id,
    project_id      BLOB NULL,          -- client/project post
    organization_id TEXT NULL,          -- org brand post      ← NEW
    user_id         TEXT NULL,          -- personal post       ← NEW (Phase D)
    ...
    CHECK (
      (project_id IS NOT NULL) +
      (organization_id IS NOT NULL) +
      (user_id IS NOT NULL) = 1
    )
  )
```

Migration path: all existing rows retain their `project_id`. New org-owned posts and accounts set `organization_id` and leave `project_id = NULL`. The CHECK constraint is enforced going forward.

### 5b. KG — add client visibility gate

```
CURRENT:
  project_knowledge_sources (
    id, project_id BLOB, owner_type TEXT DEFAULT 'project',
    owner_id TEXT, source_type, source_id, source_title,
    source_summary, coverage_score, is_active, is_stale, ...
  )

  user_knowledge_sources (
    id, user_id BLOB, source_type, source_id, source_title,
    source_summary, related_company_id, related_person_id,
    related_project_id, coverage_score, is_active, is_stale, ...
  )

CHANGES NEEDED:
  ALTER TABLE project_knowledge_sources
    ADD COLUMN client_visible INTEGER NOT NULL DEFAULT 0;
  -- When 0: visible only to org members (workflow data)
  -- When 1: deliverable, visible to the client

  -- No new table needed. Org KG = owner_type='organization'
  -- Client KG = owner_type='project' AND client_visible=1
  --             AND project.client_id IS NOT NULL
```

### 5c. Social analytics snapshots — add org scope

```
CURRENT:
  social_account_snapshots (id, account_id, project_id, ...)

ADD:
  ALTER TABLE social_account_snapshots
    ADD COLUMN organization_id TEXT NULL REFERENCES organizations(id);
  -- org-level snapshots aggregate across all org accounts
```

---

## 6. Implementation Phases

### Phase A — DB Migration & Backend  *(this sprint)*
Priority: Gate for everything else.

1. **Migration**: Add `organization_id` (nullable) to `social_accounts`, `social_posts`, `social_account_snapshots`
2. **Migration**: Add `client_visible INTEGER DEFAULT 0` to `project_knowledge_sources`
3. **Backend routes**: Update `GET /social/accounts` and `GET /social/posts` to accept `organization_id` as an alternative to `project_id`
4. **Backend routes**: Add `GET /social/accounts?organization_id=:id` and `GET /social/posts?organization_id=:id`
5. **Backend**: Update publisher/scheduler to handle `organization_id`-owned posts
6. **KG route**: Expose org KG via `GET /api/organizations/:id/knowledge` (mirrors existing project knowledge route)
7. **KG route**: Add `PATCH /api/knowledge-sources/:id/visibility` to toggle `client_visible`

### Phase B — Org Social Calendar  *(this sprint)*
Priority: Sirak Studios pilot readiness.

1. **Org social default filter**: Pass `organization_id` to `SocialTab` for the "own content" default view
2. **Filter toggle**: "Show client content" checkbox in `SocialContentView` that expands the query to include project posts
3. **Account connect**: Wire "Connect Account" button on org social tab to create an org-owned (not project-owned) account
4. **New Post composer**: Add "Posting as" selector — org brand vs. specific project/client
5. **Analytics**: Org analytics aggregates org accounts + optionally client accounts

### Phase C — Client Social Fixes  *(this sprint)*
Priority: Minor cleanup, already mostly correct.

1. **Fix `sv=` param**: Wire `sv` query param to tab state on `ClientOverview` page so deep-links work
2. **Client filters**: Add project and platform filter dropdowns to client content view
3. **Client visibility gate**: When a post is published for a client, optionally mark the deliverable as `client_visible=true` in the KG

### Phase D — Individual Calendar  *(Stage 1 / Sirak Studios pilot)*
Deferred — no personal social accounts are integrated yet. Design the data model now, build the UI when accounts are ready.

1. **Data model**: `user_id` column already planned in Phase A migration (added but nullable)
2. **My Workspace calendar**: When user has personal social accounts, show them in `/calendar` view
3. **Individual KG UI**: Surface `user_knowledge_sources` in a "My Intel" section accessible from My Workspace

### Phase E — KG UI  *(Stage 1)*
1. **Org KG page**: `/organizations/:id/knowledge` — shows all org knowledge sources with type filter (deliverables, social, intel, artifacts), visibility status per item, and the ability to mark items as `client_visible`
2. **Individual KG page**: `/settings/my-knowledge` or within My Workspace — shows `user_knowledge_sources`
3. **Client upgrade path**: When a client is promoted to an Organization, migrate their `client_visible=true` KG entries into their new org's `project_knowledge_sources` as the seed dataset

---

## 7. Alignment with 5-Year Roadmap

| Roadmap Item | Stage | This Plan |
|---|---|---|
| Priority 4: Social Media Pipeline Migration | S0 M2 | Phases A + B implement this |
| Priority 10: Sirak Studios Pilot | S1 M4 | Phase B + C enable Sirak Studios as first pilot |
| Priority 9: Multi-Tenant Hardening | S1 M4 | Phase A's org-scoped accounts are the social component of tenant isolation |
| Priority 21: Database Strategy | S2 | `organization_id` column now avoids the multi-tenant data leakage risk flagged in `GAP-S1-06` (tasks lack direct org_id) |
| Stage 3: ATLAS Graph Dependencies | S3 | KG `client_visible` gate + org/client KG separation is the data governance layer that makes the knowledge graph trustworthy at scale |
| Stage 5: Client → Org Upgrade Path | S5 | Phase E upgrade path seeds the new org's KG from the client's deliverables |

**Key alignment note**: The roadmap flags "data isolation audit — ensure org A can never see org B's data" as a critical prerequisite for pilot onboarding. The `organization_id` column and the `client_visible` KG gate directly address this. Without them, a client who eventually upgrades to an Organization would have no clean separation between "what they delivered" and "workflow internals."

---

## 8. What Is NOT Changing

- The `project_knowledge_sources` table is **not** being split into a separate `organization_knowledge_sources` table. The polymorphic `owner_type / owner_id` pattern already handles this correctly and is consistent with the existing architecture.
- The `user_knowledge_sources` table structure is **not** changing — it's correct, it just needs UI exposure (Phase D/E).
- Client social (project-scoped posts) continues to work exactly as it does today. Only new org-owned posts use `organization_id`.
- No changes to the social post FSM (draft → pending_review → approved → scheduled → publishing → published/failed).
- No changes to analytics, approval queue, caption generator, or queue manager built in this session — those work at the `project_id` level and remain correct for client work.

---

## 9. Execution Order

```
Phase A (migration + backend) ──→ Phase B (org social UI) ──→ Phase C (client fixes)
       │                                                              │
       │ runs in parallel once migration lands                        │
       └──────────────────────────────────────────────────────────────┘
              │
              ▼ (Stage 1 — after Sirak Studios pilot launched)
Phase D (individual calendar) ──→ Phase E (KG UI)
```

Phases A + B + C constitute the **current sprint deliverable**.  
Phases D + E are **Stage 1 deliverables**, scoped to the Sirak Studios pilot milestone (Priority 10).

---

*Last updated: 2026-05-18 by Claude Code*
