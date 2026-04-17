# SESSION HANDOFF — TIACA Client Onboarding (Phase 1 complete)

**Created**: 2026-04-16
**Worktree**: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp` (root)
**Branch at handoff**: `feature/lux-creator-studio` (a9e0eab6) — **behind origin/main by 38 commits, plan is to rebase before continuing**
**Reason for handoff**: Branch is stale relative to origin/main (major new work: agent orchestration framework, sloperation410 integration, DispatchAgentsParallel tool, native peer manager, Nora phone fix). User asked to save context so we can resume cleanly post-rebase.

---

## 1. Engagement context (the *why*)

The user (Sirak Studios org, operated by Aaren Sirak) just closed a new client: **TIACA — The International Air Cargo Association**. The signed **Strategic Growth Partnership Agreement** (archived in Topos — see §3) has two phases:

- **Phase 1 (fixed fee, $9,800)** — strategic foundation + 10-page website redesign, ~30 business days
- **Phase 2 (retainer, $3,000/mo × 12mo, 6-mo min)** — monthly advisory starting **2026-04-01**

Total contract value: ~$45,800 / minimum ~$27,800.

The ask is to go through the app's onboarding pipeline phase-by-phase to **build a TIACA client profile** end-to-end. Phase 1 (identity/"who-is" research) is complete. Phase 2 (DB + KG wiring to visualize on the dashboard) is next.

## 2. Confirmed facts (verified 2026-04-16)

### TIACA
- Legal name: The International Air Cargo Association. Domain: `tiaca.org`
- HQ: 5600 NW 36 St, Ste 5100, Miami FL 33122
- Type: not-for-profit global trade association, founded ~1990, Board-governed
- Taglines: **"NETWORKING | KNOWLEDGE | ADVOCACY"** + **"Uniting Air Cargo"**
- **Primary blue**: `#005A9B` (pixel-extracted from master logo, >99% dominance)
- Typography: not publicly specified — pending client supply
- Membership: full value chain (airlines, forwarders, airports, shippers, ground handlers, manufacturers, IT vendors, financial institutions, educators); tiers = voting **Trustees** + 4 non-voting categories
- Key 2026 events (inside retainer term):
  - **TIACA Executive Summit 2026** — Warsaw, **May 26–28 2026**, hosted by LOT Cargo
  - **Air Cargo Forum 2026 (ACF)** — Miami Beach Convention Center, **Oct 26–29 2026**, flagship biennial, 300+ booths, 3,500+ attendees, home-city event → peak campaign anchor

### Leadership
- **Director General (CEO)** — **Glyn Hughes** (first-ever DG, Feb 2021 → end-2026). Announced departure in March 2026. Applications open for successor. 36+ yrs industry experience. Prior: IATA for ~30 years, Global Head of Cargo 2014–2021. Born Crawley UK, raised Florida (journalist father at National Enquirer).
- **Chair (current)** — Roos Bakker
- Historical: Steven Polmans (Chair, 2023 bios), Emir Pineda (Vice Chair/Secretary, 2023 bios)
- Recent board addition: Adrien Thominet (Exec Chairman, ECS Group)

### Point of contact for this engagement
- **Glyn Hughes** — confirmed by user. User spelled "Glen" — actual is **Glyn** (pronounced "Glinn").
- Contract Exhibit A was blank. **Still need from user**: Glyn's direct email (pattern `@tiaca.org`), phone, LinkedIn URL, preferred title during transition.

### Parties to the agreement
- **Agency**: Sirak Studios LLC, 4990 SW 63rd Ave, Miami FL 33155 — Aaren Sirak, Managing Director, `sirak@sirakstudios.com`
- **Client**: TIACA (address above). Authorized approver domain: `@tiaca.org`; Written Approval = "APPROVED" in email body

## 3. Artifacts saved to Topos (on disk — the "hard drive")

User rule: **"the database is our hard drive — all artifacts we collect or generate live in Topos."**

Per-client KB convention documented in memory at `reference_topos_kb_layout.md`.

```
/Users/sirakstudios/topos/tiaca/
├── README.md                                              canonical facts + folder map
├── brand/
│   ├── brand-guide-v1.md                                  research-seeded brand guide draft (primary blue #005A9B, taglines, pillars, gaps/asks)
│   └── logos/tiaca-logo-blue-2018.png                     3222×3222 RGBA master (from tiaca.org/wp-content/uploads)
├── contracts/
│   └── 2026-04-16_tiaca-strategic-growth-partnership-agreement.docx
├── intel/
│   ├── organization/01-identity.md                         full "who-is" pass — leadership, events, membership, programs, digital footprint, strategic signals
│   └── contacts/glyn-hughes.md                             verbatim TIACA bio + career arc + messaging priorities + missing-data checklist
├── press/                                                  (reserved — empty)
└── sources/
    └── tiaca-corporate-biographies-2023.pdf                cached primary source (official TIACA PDF)
```

**Nothing is registered in the app's knowledge graph yet** — that is Phase 2 work.

## 4. Memory saved (for future Claude sessions)

`~/.claude/projects/-Users-sirakstudios-topos/memory/`:

- `MEMORY.md` — index
- `workspace_layout.md` — Topos workspace layout
- `project_pcg_cc_mcp.md` — tech stack and core entities
- `project_sirak_studios.md` — Sirak Studios org, TIACA as new client
- `project_tiaca_facts.md` — TIACA verified identity, leadership, events, brand
- `reference_client_dashboard.md` — `client-overview.tsx` at `/organizations/:orgId/clients/:clientId`
- `reference_topos_kb_layout.md` — per-client KB convention

## 5. Repo/DB state at handoff

### Branch
- Current: `feature/lux-creator-studio` @ `a9e0eab6` — **4 commits ahead, 38 commits behind origin/main**
- Branch's 4 commits (Lux creator studio — phases 0 → 1.1a):
  - `a9e0eab6` feat(lux): Phase 1.1a — first three deck authoring tools + type safety fixes
  - `f35a10ff` feat(lux): Phase 1.0 — link crm_deals to deck documents
  - `51a03863` feat(lux): Phase 0 — deck document schema + typed domain model
  - `e008fc39` docs: plan for Lux creator studio — Illustrator-compatible deck authoring
- Untracked files present:
  - `crates/db/bindings/DealDataSource.ts`
  - `crates/db/bindings/LinkDataSourceRequest.ts`
  - These are ts-rs generated bindings from a prior Rust type change — decide whether to commit or regenerate post-rebase.

### Dev servers
- **Frontend (vite)** — running on `:3000` (pid 31153) from this repo
- **Backend (cargo)** — **NOT running** (no process on `:3001`). `apn_node` on `:4001` is unrelated (Alpha Protocol Node).
- Backend MUST be started for the dashboard to load data: `flox activate -- pnpm run backend:dev`

### Database (dev — `dev_assets/db.sqlite`)
- Organization `Sirak Studios` exists: id `02020202-0202-0202-0202-020202020202`
- `clients` table: **TWO TIACA rows under Sirak Studios** (needs dedupe):
  - `5303d1cd-2845-43c7-bbbf-e39cf8ca08bb` (clean UUID — keep)
  - `35333033-6431-6364-2d32-3834352d3433` (legacy-format id — soft-delete candidate)
- `crm_pipelines` — **0 rows** (Sirak has no pipelines at all)
- `crm_pipeline_stages` — **1967 orphaned rows** referencing pipeline IDs that no longer exist (from a prior wipe). Schema is solid; just empty on active pipelines.
- `crm_deals` — **0 rows** (no deal exists for TIACA; agreement signed but not in CRM)
- `crm_contacts` — no Glyn Hughes row
- `company_research_passes`, `contact_research_passes` — no TIACA/Glyn entries
- `organization_brand_profiles`, `company_brand_profiles` — no TIACA entry
- `entity_graph_nodes`, `entity_graph_edges` — no TIACA nodes

## 6. Phase 2 plan — DB + KG wiring (NOT yet executed)

User confirmed: wire TIACA under the **Sirak Studios org**. Work queued:

1. **Rebase `feature/lux-creator-studio` onto `origin/main`** — resolve conflicts, regenerate TS bindings (`npm run generate-types`), then `cargo sqlx prepare --workspace` if Rust types changed. Run `/check` before continuing.
2. **Dedupe TIACA clients** — keep `5303d1cd-…`, soft-delete the legacy-id row (preserve linkages first — check if anything references it).
3. **Seed Sirak Studios pipelines** — user to confirm:
   - **Sales pipeline** (Lead → Proposal → Presented → **Won** → Lost) + create TIACA closed-won deal for history
   - **Client Delivery pipeline** (Onboarding → **Brand Guide** → Online Presence → Social Stack → Monthly Retainer → Active Retainer → Completed) + TIACA card at **Brand Guide** stage
   - Likely want both — standard acquisition+delivery pattern
4. **Create Glyn Hughes crm_contact** — needs his email (blocker: user to supply).
5. **Create "Brand Guide v1" task** on the Client Delivery pipeline board, status = `inreview` — maps to the user's "Task Card on the Board in Review" request.
6. **Seed `company_research_passes`** for TIACA (focus=`identity`, status=`done`) — body points to `/tiaca/intel/organization/01-identity.md`.
7. **Seed `contact_research_passes`** for Glyn (focus=`identity`, status=`done`) — body points to `/tiaca/intel/contacts/glyn-hughes.md`.
8. **Populate `organization_brand_profiles` for TIACA** — primary_color `#005A9B`, logo_url `/tiaca/brand/logos/tiaca-logo-blue-2018.png`, taglines, mission (from `brand-guide-v1.md`). Schema check: may need a TIACA `companies` row if using `company_brand_profiles` instead — verify during Phase 2 which table fits a Client entity.
9. **Register Topos artifacts in `project_knowledge_sources`** with `owner_type='organization'`, `owner_id=<sirak-studios-org-id>`:
   - README.md, brand-guide-v1.md, 01-identity.md, glyn-hughes.md, tiaca-corporate-biographies-2023.pdf, agreement .docx, logo .png
10. **Register entity_graph_nodes/edges**:
    - Node: `organization` Sirak Studios
    - Node: `company` TIACA (may need a companies row)
    - Node: `person` Glyn Hughes
    - Edges: Sirak Studios —`manages`→ TIACA; TIACA —`employs`→ Glyn Hughes
11. **Start backend** (`flox activate -- pnpm run backend:dev`) so the dashboard can render data; reload `http://localhost:3000/organizations/02020202-0202-0202-0202-020202020202/clients/5303d1cd-2845-43c7-bbbf-e39cf8ca08bb` to verify TIACA card + brand + intel render.
12. **Run `/check`** and commit in logical chunks.

## 7. Open questions (need user answer)

- Glyn Hughes direct email / LinkedIn URL / phone?
- Both pipelines (Sales + Client Delivery) or just Client Delivery?
- Brand guide to target `organization_brand_profiles` (attached to Sirak Studios, representing "our guide for TIACA") or a new TIACA `companies` row with its own `company_brand_profiles`?
- Okay to soft-delete the legacy-id duplicate TIACA client (`35333033-…`) after verifying no references?

## 8. Suggested re-prompt (after rebase)

> Read `planning/SESSION-HANDOFF.md`, confirm the branch is now on a clean rebase of `origin/main`, then execute Phase 2 from §6. Answer my open questions from §7 first (Glyn's email is `______`; use both pipelines; target `organization_brand_profiles`; soft-delete the legacy client row). Start backend, wire DB + KG, and open the TIACA client page in the browser to verify.

---

## 9. Source-of-truth pointers

- Memory index: `~/.claude/projects/-Users-sirakstudios-topos/memory/MEMORY.md`
- TIACA KB root: `/Users/sirakstudios/topos/tiaca/`
- App repo: `/Users/sirakstudios/topos/pcg/github/pcg-cc-mcp/`
- Contract original: `~/Downloads/TIACA Strategic Growth Partnership Agreement.docx` (also archived in KB)
- Extended dropbox artifacts (mentioned by user, not fetched — auth-gated): https://www.dropbox.com/scl/fo/w82dszyo8xn3fb70ggk8t/AFw2cKQWfdeTfNai-fyr8tU?rlkey=iqkhoop2od8ctbr4p5oslas74&e=1&dl=0
