# Phase D & E: Personal Social Calendar + Knowledge Graph UI
**Date**: 2026-05-18  
**Sprint context**: Stage 1 / Sirak Studios Pilot — Priority 10  
**Prerequisite**: Phases A + B + C complete ✅

---

## 1. What We're Building

Three concrete deliverables that complete the individual user layer:

| ID | Deliverable | Where it surfaces |
|----|-------------|------------------|
| D2 | Personal social accounts in calendar | `/calendar` — new "My Social" row |
| D3/E2 | My Intel — personal knowledge graph UI | `/calendar` as a second tab + `/settings/my-intel` |
| E3 | KG seed on client → org upgrade | Backend only — runs inside `client_to_org()` |

---

## 2. System Context: Three Ownership Layers (current state)

```
┌──────────────────────────────────────────────────────────────────────┐
│                         social_accounts                              │
│                                                                      │
│   project_id ──────── Project-scoped    (✅ wired: A, B, C done)    │
│   organization_id ─── Org brand channel (✅ wired: Phase B done)    │
│   user_id ─────────── Personal account  (❌ column exists, not wired)│
└──────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────┐
│                          Knowledge Graph                             │
│                                                                      │
│   project_knowledge_sources                                         │
│     owner_type='project'      ── Project KG     (✅ works)          │
│     owner_type='organization' ── Org KG         (✅ API + UI done)  │
│     client_visible=1          ── Client KG gate (✅ done)           │
│                                                                      │
│   user_knowledge_sources                                            │
│     (per-user private KG)     ── My Intel       (❌ API exists, no UI)│
└──────────────────────────────────────────────────────────────────────┘
```

---

## 3. Deliverable D2: Personal Social Accounts in Calendar

### 3a. Current Calendar Data Flow

```
CalendarPage (/calendar)
│
├── useProjectList() → [project_id_1, project_id_2, ...]
│
├── for each project_id:
│     socialApi.listPostsFiltered({ projectId }) → SocialPostRecord[]
│
└── renders: tasks + project social posts on monthly grid
                                 ↑
                      (personal posts NOT included)
```

### 3b. Target Calendar Data Flow

```
CalendarPage (/calendar)
│
├── useProjectList() → project posts (unchanged)
│
├── socialApi.listPostsFiltered({ userId: 'me' })   ← NEW
│     → personal social posts WHERE user_id = current_user
│
├── socialApi.getPersonalAccounts()                  ← NEW
│     → social_accounts WHERE user_id = current_user
│
└── renders:
      Project posts  (blue chips — existing)
      Personal posts (purple chips — new, labeled with platform icon)
      ↕
      "My Social" sidebar panel: connected personal accounts
        + "Connect personal account" button
```

### 3c. Backend Changes Required

**1. OAuth connect — add `user` owner type**

File: `crates/server/src/routes/social_oauth.rs`

`ConnectQuery` currently accepts `project_id` or `org_id`. Add `user_id: bool` flag (or derive from JWT when neither is provided).

```
GET /api/social/oauth/:platform/connect
  ?project_id=...    → "project" owner
  ?org_id=...        → "org" owner
  (no params)        → "user" owner (JWT user_id used)   ← NEW
```

State string extended: `{platform}:user:{user_id_hex}`

Callback `upsert_social_account` extended: when `owner_type == "user"`, sets `user_id` column and leaves `project_id` and `organization_id` null.

**2. Personal posts endpoint**

File: `crates/server/src/routes/social_posts.rs`

`ListPostsQuery` already has `organization_id`. Add `user_id: Option<String>` — when `user_id == "me"` resolve from JWT claims; when it's a UUID string use it directly.

```
GET /api/social/posts?user_id=me   → posts WHERE user_id = $current_user
```

**3. Personal accounts endpoint**

File: `crates/server/src/routes/social_accounts.rs`

```
GET /api/social/accounts?user_id=me   → accounts WHERE user_id = $current_user
```

`SocialAccount::find_by_user(pool, user_id)` already exists in the model (check; if not, add).

### 3d. Frontend Changes

**Calendar page** (`frontend/src/pages/calendar.tsx`):

```
┌─────────────────────────────────────────────────┐
│  ◀ May 2026  ▶          [+ New Post]  [⚙ Scope] │
│                                                 │
│  Mon  Tue  Wed  Thu  Fri  Sat  Sun              │
│   ┌──────────────────────────────────────────┐  │
│   │ 18 │ ...project posts (blue)...          │  │
│   │    │ ...personal posts (purple) ← NEW    │  │
│   └──────────────────────────────────────────┘  │
│                                                 │
│  MY SOCIAL ACCOUNTS  ← NEW sidebar chip row     │
│  [💼 LinkedIn: @sirak]  [+ Connect]             │
└─────────────────────────────────────────────────┘
```

Changes:
1. Add `useQuery` for `socialApi.listPostsFiltered({ userId: 'me' })` → personal posts
2. Add `useQuery` for `socialApi.getPersonalAccounts()` → personal account chips
3. Merge personal posts into calendar grid with `type: 'personal-social'` distinguisher
4. Personal post chips styled in purple (`bg-violet-500`) vs. project blue (`bg-blue-500`)
5. "My Social" chip row above calendar header — shows connected accounts + connect button (no `projectId`/`orgId` → triggers user OAuth flow)
6. New post composer: when creating from personal context, sets `user_id` instead of `project_id`

**`frontend/src/lib/api/social.ts`**:
```ts
getPersonalAccounts: () =>
  apiRequest<SocialAccountRecord[]>('/social/accounts?user_id=me'),
listPostsFiltered already exists — add userId?: string to params
```

**`ConnectAccountButton.tsx`**: when rendered without `projectId` or `orgId`, emit connect URL with no owner params → triggers `user` owner type OAuth.

---

## 4. Deliverable D3/E2: My Intel — Personal Knowledge Graph UI

### 4a. Placement Decision

The personal KG sits at the junction of two natural homes:

```
Option A: Calendar tabs (recommended)
  /calendar?tab=intel
  ┌─────────────────────────────────┐
  │ [📅 Calendar] [🧠 My Intel]    │  ← tab strip
  │                                 │
  │  My Intel tab content here      │
  └─────────────────────────────────┘
  PRO: My Intel is contextually a personal workspace view
  CON: calendar.tsx is already 1,082 lines

Option B: Settings route
  /settings/my-intel
  PRO: consistent with settings layout pattern
  CON: feels buried in settings; not a "workspace" surface

Option C: Both — settings route, aliased from calendar
  /settings/my-intel   (canonical)
  /calendar → "My Intel" link in sidebar → navigates to settings page
```

**Decision: Option A (calendar tab)** — new `?tab=` param; My Intel renders as a lazy-loaded sub-panel. Also add `/settings/my-intel` as a standalone route for deep links.

### 4b. API Response Shape

`GET /api/knowledge/mine` already returns:
```json
{
  "by_type": {
    "company": [UserKnowledgeSource, ...],
    "person":  [UserKnowledgeSource, ...],
    "project": [UserKnowledgeSource, ...]
  },
  "total": 12,
  "active": 10
}
```

`UserKnowledgeSource` fields: `id, user_id, source_type, source_id, source_title, source_summary, related_company_id, related_person_id, related_project_id, coverage_score, is_active, is_stale`.

### 4c. My Intel UI Layout

```
┌──────────────────────────────────────────────────────────────────┐
│  🧠 My Intel                                     [+ Add Source]  │
│  Personal knowledge graph — 12 sources · 83% coverage            │
│                                                                  │
│  Filter: [All] [Companies] [Persons] [Projects]                  │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │ 🏢 Sirak Studios           company · 94% coverage  ● live │  │
│  │    Media production company — music videos, brand...       │  │
│  │    Related: Sirak Fekadu (person)                          │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │ 👤 Jordan Lee              person · 71% coverage  ⚠ stale │  │
│  │    Music producer, 12k followers on Instagram              │  │
│  │    Related: Sirak Studios (company) · Project X            │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │ 📁 Brand Video Q2          project · 88% coverage ● live  │  │
│  │    Brand identity video series for Q2 campaign             │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                  │
│  Coverage ring: ████████░░ 83%                                   │
└──────────────────────────────────────────────────────────────────┘
```

### 4d. Files to Create/Modify

**New component**: `frontend/src/components/intel/MyIntelPanel.tsx`
- `useQuery` → `GET /api/knowledge/mine`
- Type filter pill buttons (All / Companies / Persons / Projects)
- Coverage ring: SVG circle (r=40, strokeDasharray = coverage% of circumference)
- Source cards: icon by type, title, summary excerpt, coverage bar, stale badge
- Stale: amber dot + "Refresh" button → `POST /api/knowledge/mine/{id}/refresh` (add this endpoint)

**Modified**: `frontend/src/pages/calendar.tsx`
- Add `tab` state: `'calendar' | 'intel'`, driven by `?tab=` search param
- Tab strip: `[📅 Calendar] [🧠 My Intel]`
- Lazy-render `<MyIntelPanel />` when tab === 'intel'

**New settings page**: `frontend/src/pages/settings/MyIntelSettings.tsx`
- Identical to MyIntelPanel, wrapped in settings layout
- Route: `/settings/my-intel`

**New API method** in `frontend/src/lib/api/knowledge.ts` (or existing):
```ts
getMyKnowledge: () =>
  apiRequest<MyKnowledgeResponse>('/knowledge/mine'),
```

**Backend — add refresh endpoint**:
```
POST /api/knowledge/mine/{id}/refresh
  → sets is_stale = 0, last_refreshed_at = now on user_knowledge_source
```

---

## 5. Deliverable E3: KG Seed on Client → Org Upgrade

### 5a. Current `client_to_org` Flow

```
client_to_org(pool, client_id)
  1. SELECT name/slug from clients
  2. INSERT INTO organizations (new_org_id)
  3. UPDATE projects SET organization_id = new_org_id WHERE client_id = old_client_id
  4. migrate_client_members_to_org(...)
  5. Soft-delete client
  ← KG not seeded ← GAP
```

### 5b. Target Flow

```
client_to_org(pool, client_id)
  1–4. (unchanged)
  5. Collect all project IDs that were moved to the new org:
       SELECT id FROM projects WHERE organization_id = new_org_id
  6. For each project, copy client_visible=1 entries to the org KG:
       INSERT INTO project_knowledge_sources
         (id, owner_type, owner_id, source_type, source_id,
          source_title, source_summary, coverage_score, project_id,
          client_visible, created_at, updated_at)
       SELECT
         uuid(), 'organization', new_org_id, source_type, source_id,
         source_title, source_summary, coverage_score, project_id,
         1, datetime('now','subsec'), datetime('now','subsec')
       FROM project_knowledge_sources
       WHERE project_id IN (moved_project_ids)
         AND client_visible = 1
         AND is_active = 1
       ON CONFLICT DO NOTHING
  7. Soft-delete client (moved from step 5)
```

### 5c. Why This Matters

```
Before upgrade:
  Client KG = project_knowledge_sources WHERE client_visible=1 AND project.client_id=X
                      ↓ (upgrade)
After upgrade:
  New Org KG = project_knowledge_sources WHERE owner_type='organization' AND owner_id=new_org_id
  
  Without seeding: new org KG is empty on day 1 — all the delivered work is invisible.
  With seeding:    new org KG bootstrapped with everything already delivered to them.
```

File: `crates/db/src/models/entity_conversion.rs`, function `client_to_org`.

---

## 6. Data Flow: Full Personal Social Account Journey

```
User clicks "Connect personal account" on /calendar
  │
  ▼
ConnectAccountButton (no projectId, no orgId)
  → GET /api/social/oauth/linkedin/connect   (no query params)
  │
  ▼ (server)
connect() handler: owner_type = "user", owner_id = jwt.user_id
  → state = "linkedin:user:{user_id_hex}"
  → LinkedIn consent screen
  │
  ▼ (LinkedIn redirects back)
callback() handler:
  → parse state → owner_type="user", owner_id=user_id_hex
  → upsert_social_account(user_id=user_id_hex)
  → social_accounts row: user_id=X, project_id=NULL, organization_id=NULL
  → redirect → /calendar?connected=linkedin
  │
  ▼ (calendar page)
getPersonalAccounts() picks up new account → renders chip
listPostsFiltered({ userId: 'me' }) → any scheduled/published personal posts
```

---

## 7. File Change Map

### Backend

| File | Change |
|------|--------|
| `crates/server/src/routes/social_oauth.rs` | Add `user` owner type in `connect()` + `callback()` + `upsert_social_account()` |
| `crates/server/src/routes/social_posts.rs` | Add `user_id=me` filter to `ListPostsQuery` |
| `crates/server/src/routes/social_accounts.rs` | Add `user_id=me` filter to list accounts |
| `crates/server/src/routes/knowledge.rs` | Add `POST /knowledge/mine/{id}/refresh` |
| `crates/db/src/models/entity_conversion.rs` | Seed org KG from `client_visible=1` entries in `client_to_org()` |

### Frontend

| File | Change |
|------|--------|
| `frontend/src/lib/api/social.ts` | Add `getPersonalAccounts()`, add `userId` to `listPostsFiltered` |
| `frontend/src/lib/api/knowledge.ts` | Add `getMyKnowledge()`, `refreshMySource(id)` |
| `frontend/src/lib/query-keys.ts` | Add `myKnowledge` key |
| `frontend/src/pages/calendar.tsx` | Tab strip, personal posts query, "My Social" account chips |
| `frontend/src/components/intel/MyIntelPanel.tsx` | NEW — My Intel UI |
| `frontend/src/components/social/ConnectAccountButton.tsx` | Support no-owner mode (personal) |
| `frontend/src/pages/settings/MyIntelSettings.tsx` | NEW — settings wrapper |
| `frontend/src/App.tsx` | Add `/settings/my-intel` route |

---

## 8. Execution Order

```
Step 1 (Backend — parallel):
  ├── social_oauth.rs: user owner type
  ├── social_posts.rs: user_id=me filter
  ├── social_accounts.rs: user_id=me filter
  ├── knowledge.rs: POST /knowledge/mine/:id/refresh
  └── entity_conversion.rs: KG seed in client_to_org

Step 2 (Frontend — parallel, after step 1 routes exist):
  ├── api/social.ts + api/knowledge.ts: new methods
  ├── MyIntelPanel.tsx component
  ├── calendar.tsx: tab strip + personal posts + account chips
  ├── ConnectAccountButton.tsx: personal mode
  └── MyIntelSettings.tsx + App.tsx route

Step 3: Build + deploy
```

---

## 9. What Is NOT in This Plan

- **Personal post composer**: Can create personal posts via the calendar's existing "New Post" dialog — just scoped to `user_id` instead of `project_id`. Not a new UI; just a new target option.
- **Token refresh worker for personal accounts**: Will be handled by the existing token refresh mechanism once it's built (separate concern).
- **Individual analytics**: Personal account analytics (impressions, engagement) are out of scope for Phase D — that requires platform API calls per account and is a Stage 2 feature.
- **Phase E3 UI**: The client → org upgrade UI already exists (`entity_conversion` route + frontend conversion button). We only add the KG seeding logic to the backend function.

---

*Last updated: 2026-05-18 by Claude Code*
