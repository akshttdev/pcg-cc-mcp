# NORA DIRECTIVE: Sirak Studios — Foundation Build Engagement

**Issued**: 2026-03-08
**Priority**: Executive
**Client**: Sirak Studios (Aaren Sirak)
**Engagement Type**: Foundation Build
**Template ID**: `foundation_build`

---

## OBJECTIVE

Execute the Foundation Build workflow for Sirak Studios through the PCG dashboard. This is a website redesign engagement — the client has an existing Framer-based site at sirakstudios.com and an undeployed Next.js 15 codebase at `/home/spaceterminal/topos/sirak-studios/GitHub/`. The redesign will build on the existing Next.js codebase and deploy it as the production site.

---

## STEP 1: CLIENT & CRM SETUP

### 1a. Ensure Sirak Studios Client Record Exists
- **Organization**: PowerClub Global (use existing org ID)
- **Client Name**: Sirak Studios
- **Primary Contact**: Aaren Sirak
- **Email**: aaren@sirakstudios.com
- **Location**: Miami, FL
- **Website**: sirakstudios.com
- **Social**: @sirakstudios on Instagram, YouTube, Facebook, Twitter

If the Sirak client record does not exist, create it under the PCG organization.

### 1b. Create CRM Contact
- **Full Name**: Aaren Sirak
- **Company**: Sirak Studios
- **Email**: aaren@sirakstudios.com
- **Website**: sirakstudios.com
- **Lifecycle Stage**: customer
- **Lead Score**: 100 (existing relationship)

### 1c. Create CRM Deal
- **Deal Name**: Sirak Studios — Website Redesign
- **Stage**: signed (skip pipeline — engagement approved)
- **Description**: Full website redesign. Migrating from Framer to Next.js. Existing codebase at `/home/spaceterminal/topos/sirak-studios/GitHub/` with 110 TSX files, comprehensive site config, and service/pricing data. Current live site is a Framer build with dark aesthetic, red/purple accents, scroll-based narrative UX.
- **Tags**: website-redesign, foundation-build, creative-studio
- **Contact**: Link to Aaren Sirak contact record

---

## STEP 2: DEAL CONVERSION

Convert the deal to a project using the Foundation Build template:

**Endpoint**: `POST /api/crm/deals/:deal_id/convert`

```json
{
  "template_id": "foundation_build",
  "project_name": "Sirak Studios — Foundation Build",
  "git_repo_path": "/home/spaceterminal/topos/sirak-studios/GitHub"
}
```

**Expected Result**: This auto-scaffolds:
- 4 boards (Discovery & Research, Brand Guide, Website & Dealflow, Social Media Setup)
- 19 tasks with proper dependencies and gate reviews
- Knowledge sources registered at 0% coverage
- Deal context seeded into project knowledge

**Verify on dashboard**: All 4 phase boards appear. All 19 tasks visible. Dependency chains intact.

---

## STEP 3: PHASE 1 — DISCOVERY & RESEARCH

This phase has 5 tasks. The first 4 run via scout agents. The 5th is a human review gate.

### Task 1: Industry & Market Research (scout, high priority)

**Seed this research into the task**:

**Miami Creative Studio Market — 2026**
- Market is saturated with video/photo studios but very few offer the full stack (photography + videography + music production + studio rentals) under one roof
- Price range spans from accessible ($80-85/hr for space-only rentals at Miami Film House) to enterprise-tier (M3 Studios, Temple House — no public rates, quote-only)
- Most competitors are either venue-only rental operations OR production-only agencies — rarely both
- Sirak's positioning as a full-service creative production studio with transparent, published pricing is a genuine market differentiator
- Key market segments: music artists, brands/agencies, real estate, events, product shoots
- Miami's creative economy continues to grow post-pandemic with influx of tech/media companies

**Knowledge outputs**: `market_research`, `industry_context`

### Task 2: Competitor Analysis (scout, high priority)

**Seed this research into the task**:

| Competitor | Type | Strengths | Weaknesses |
|-----------|------|-----------|------------|
| **Temple House Studios** | Venue rental | 360° projection mapping (unique), celebrity clients (Shakira, Chainsmokers), converted Art Deco synagogue, 15K+ sq ft, dual indoor/outdoor | No pricing anywhere, no booking system, venue-only (not full production), Squarespace platform |
| **M3 Studios** | Enterprise facility | 122K sq ft, 7 sound stages, stunt/transport services, in-house set construction, city permitting liaison, operating since 2003 | Blog last updated 2020, no pricing, no online booking, WordPress/Visual Composer bloat, enterprise-only positioning |
| **Miami Film House** | Creator-focused rental | Transparent rates ($80-85/hr), 10 unique sets with RGB lighting, Rain Room feature, external booking via as.me | Minimal portfolio displayed, no production pricing, production services secondary to rentals, limited equipment specs |
| **CC Studios Miami** | Photography rental | 3K sq ft cyclorama, rotating styled backdrops, professional gear | Weak site (Divi/WordPress), no portfolio, no pricing, no booking system, minimal content |
| **Chroma House** | Production company | Full-service production (pre through post), founded 2012, crewing + equipment | No studio facility of their own, service-only model |
| **Global Filmz** | Production company | Music video focus, multi-city (Miami/NYC/SLC), corporate + film | Production-only, no studio rentals, no music production |

**Sirak competitive advantages**:
1. Only studio offering photo + video + music production + rentals under one roof
2. Transparent, tiered pricing (competitors hide rates behind quote requests)
3. High-end equipment inventory (ARRI Alexa Mini, Phantom Flex 4K, RED Komodo, Sony FX6, Neumann U87, Dolby Atmos)
4. Existing client portal and dashboard infrastructure (Next.js codebase)
5. Comprehensive package structure (basic through premium, events, products, real estate, music videos)
6. Add-on ecosystem (drone, HMUA, casting, graphic design, BTS video)

**Knowledge outputs**: `competitor_analysis`, `competitive_landscape`

### Task 3: Target Audience Research (scout, high priority)

**Depends on**: Task 1 (market_research)

Execute audience research covering:
- **Primary segments**: Independent musicians/artists, brands needing content, real estate agents/brokers, event promoters, product companies
- **Demographics**: Miami-Dade creatives, ages 22-45, digitally native, social-media-forward
- **Psychographics**: Value transparent pricing, want professional quality without enterprise gatekeeping, active on Instagram/TikTok/YouTube
- **Pain points**: Hidden pricing from competitors, difficulty finding full-service studios, booking friction, inconsistent quality from freelancers
- **Where they are online**: Instagram (primary discovery), YouTube, TikTok, Google Maps/reviews, music industry networks

**Knowledge outputs**: `audience_personas`, `audience_insights`

### Task 4: Client Brand Audit (scout, medium priority)

**Seed this audit into the task**:

**Current sirakstudios.com — Live Site Audit**

| Area | Current State | Issues |
|------|---------------|--------|
| **Platform** | Built on Framer (design-to-code) | Limited dev flexibility, poor SEO control, vendor lock-in |
| **Design** | Dark mode (#0d0d0d), red (#fe0100) + purple (#b25af9) accents | Visually polished but scroll-heavy UX, sticky sections may confuse navigation |
| **HTML/SEO** | No semantic headings, div-heavy layout, no Open Graph tags | Major gap — poor crawlability, no social sharing previews |
| **Content** | Scroll-based narrative with ~9 portfolio cards (320x369px) | Content buried in animations, not easily scannable |
| **Forms** | Email + textarea with 4-column checkbox grid | No booking system integration, basic lead capture only |
| **Mobile** | Responsive breakpoints at 720px, 1100px, 1440px | Adjusts layout but removes elements rather than reorganizing |
| **Performance** | Extensive inline CSS, design tokens via --token- variables | Framer-generated code is bloated, not optimized for Core Web Vitals |

**Existing Next.js Codebase** (`/home/spaceterminal/topos/sirak-studios/GitHub/`):
- Next.js 15.5.12, React 19, Tailwind CSS 3, TypeScript
- 110 TSX files covering all pages (services, studio, work, pricing, contact, about, book, press, events, dashboard, portal)
- Comprehensive config at `config/site.ts` (439 lines) with all services, pricing tiers, packages, equipment lists, inquiry types, nav structure, booking config
- Client portal features specced: project dashboard, asset uploads, review/approval workflow, direct messaging, invoices, session booking
- **This codebase is NOT deployed** — the Framer version is live

**Gap analysis**:
1. Framer site → Next.js migration (codebase exists but needs design update)
2. No booking system integration on either version
3. No SEO optimization on live site
4. Portfolio showcase is weak — needs proper project case studies
5. No client portal deployed yet (specced but not built)
6. Missing: testimonials, social proof, Google reviews integration

**Knowledge outputs**: `brand_audit`, `current_state`

### Task 5: Review Research Findings (human_review, critical, GATE)

**Depends on**: Tasks 1, 2, 3, 4

**Action for Nora**: When all 4 research tasks are complete, compile findings and flag for human review. This is the Phase 1 gate — no Phase 2 work begins until this is approved.

**Checklist for review**:
- [ ] Market research validates Sirak's positioning
- [ ] Competitor analysis identifies clear differentiation points
- [ ] Audience personas align with Sirak's actual client base
- [ ] Brand audit accurately captures current state and gaps
- [ ] Research foundation is sufficient to inform brand strategy

---

## KNOWN POTENTIAL ISSUES — MONITOR & ESCALATE

### Platform Issues
1. **Database state**: The dev seed DB may not have the Sirak client/org records. If CRM operations fail, check that the organization and user records exist. The migration `20260213100000_add_sirak_user.sql` should have created the user but client/deal records may need manual creation.
2. **Backend must be running**: Deal conversion requires the Axum backend on port 3002. If not running, start with `pnpm run dev` from the PCG project root.
3. **Task dependency enforcement**: The workflow template creates cross-phase dependencies (Phase N first task depends on Phase N-1 gate). If tasks appear unblocked when they shouldn't be, check that dependency records were created during conversion.

### Workflow Issues
4. **Knowledge source coverage**: All sources start at 0.0 coverage. As research tasks complete, update coverage via `update_knowledge_source` MCP tool. Don't proceed to Phase 2 if knowledge coverage is below 0.5 on core domains.
5. **Agent role assignment**: Tasks specify agent_role (scout, astra, creative) but actual agent assignment may need manual mapping if agents aren't registered. Verify agent records exist in the system.
6. **Gate task approval**: Task 5 (and all gate tasks) have `requires_approval: true`. The approval_status must be explicitly set — don't auto-approve. Flag for human decision.

### Content Issues
7. **Existing codebase conflicts**: The Next.js repo may have stale dependencies (Next 15 vs latest). During Phase 3, audit `package.json` and update before development begins.
8. **Framer → Next.js content parity**: Ensure all content visible on the live Framer site is captured in the Next.js codebase. The `config/site.ts` is comprehensive but may not cover all Framer page content.
9. **Asset availability**: Portfolio images, studio photos, equipment photos — verify these exist in the repo or need to be sourced from Aaren.

---

## PHASE 2-4 PREVIEW (Do not execute until Phase 1 gate approved)

### Phase 2: Brand Guide
- Brand strategy & positioning based on validated research
- Voice & messaging framework
- Visual identity system (evolve from current red/purple or rebrand)
- Compile brand guide document
- **Gate**: Brand guide review & approval

### Phase 3: Website & Dealflow
- Site architecture & content strategy
- Website copy aligned with brand voice
- Design & development (Next.js codebase, apply new brand)
- Dealflow engine (booking integration, lead capture, CRM connection)
- **Gate**: Website review & QA

### Phase 4: Social Media Setup
- Social landscape analysis
- Account optimization across platforms
- 30-day content calendar
- Initial content library production
- **Gate**: Content review & launch approval

---

## ESCALATION PROTOCOL

- **Blocked by platform bug** → Flag to dev team, do not attempt workarounds that bypass data integrity
- **Missing data/records** → Document what's missing, request creation, do not fabricate placeholder data
- **Gate task ready for review** → Notify via executive alert with summary of deliverables and coverage metrics
- **Cross-phase dependency conflict** → Do not override. Report the conflict and wait for resolution
- **Agent execution failure** → Log the error, attempt once with adjusted context. If second failure, escalate with full error trace

---

## SUCCESS CRITERIA

Phase 1 is complete when:
1. All 4 research tasks have status `done` with knowledge outputs populated
2. Knowledge coverage ≥ 0.5 on: `market_research`, `competitor_analysis`, `audience_personas`, `brand_audit`
3. Gate task (Task 5) has been reviewed and `approval_status` set to `approved`
4. All research is visible on the dashboard with proper board/task organization

Full engagement is complete when:
1. All 4 phases pass their gate reviews
2. Redesigned website is deployed on Next.js (not Framer)
3. Dealflow engine captures leads and connects to PCG CRM
4. Social media accounts optimized with 30-day content library ready
5. Brand guide document delivered to client
