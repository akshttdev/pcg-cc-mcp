# TIACA Executive Summit 2026 — Warsaw Landing Page & Conversion Funnel
**Type**: plan  
**Date**: 2026-04-22  
**Client**: The International Air Cargo Association (TIACA)  
**Project**: Executive Summit 2026 — Dedicated Landing Page & Conversion Funnel  
**Proposal value**: $7,400 (pending Glyn approval)  
**Timeline**: 1 week from receipt of content + access  
**Event dates**: June 1–3, 2026 · Warsaw, Poland (hosted by LOT Cargo)  
**Event URL**: Currently lives under TIACA.org — new dedicated page/URL to be created  
**Primary contact**: Glyn Hughes `ghughes@tiaca.org` · Event coordination: Riley

---

## 1. Strategic Context

The Executive Summit is **the most urgent deliverable in the entire engagement**. It is 5–6 weeks away and is TIACA's first major event in the Sirak retainer window. It has two strategic functions:

1. **Near-term revenue**: Sponsorship packages and partner placement at a closed-door executive event are high-value conversions. The current Summit presence is under-built for this.
2. **Content seed**: Warsaw is the first major content-capture opportunity in the 12-month cycle. Every asset produced there feeds ACF promotion and TNN episodes through summer and fall. If no dedicated conversion presence exists going in, the event underperforms commercially.

This site is **not** a general information page — it is a **sales and conversion funnel** for a high-level industry event with a specific attendee profile (senior decision-makers, C-suite, VPs). It should feel premium, closed-door, and authoritative.

**Relationship to TIACA.org**: The main site's Executive Summit page links HERE. This site does the conversion work; the main site provides the context.

---

## 2. Scope of Work

Per the Executive Summit Landing Page & Funnel Proposal ($7,400):

- Strategic funnel architecture for Summit positioning + conversion
- Copywriting and messaging (tone: premium, exclusive, executive-level)
- Full design direction + page layout (hi-fi, brand-aligned)
- Build of a high-conversion single-page or short-page funnel
- Sponsor visibility and structured placement
- Registration / CTA integration (linking to Glue Up / existing registration system)
- Mobile optimization
- Integration of existing Summit materials and approved assets

**Not in scope**: Multi-page event website, ticketing platform rebuild, custom registration backend, sponsor outreach, paid media, ongoing maintenance post-launch.

---

## 3. Page Architecture

Single long-form landing page (or 2–3 page microsite if Glyn prefers). Sections:

### Section 1 — Hero
- Event name: **TIACA Executive Summit 2026**
- Location + dates: **Warsaw, Poland · June 1–3, 2026**
- Host: LOT Cargo (acknowledge prominently)
- Tagline: Premium headline from messaging framework — e.g. *"Where the decisions shaping air cargo's future begin"*
- Hero image: Warsaw / LOT Cargo / air cargo executive imagery (source from client library or licensed)
- **Primary CTA**: "Register Now" (→ Glue Up registration) + "Become a Sponsor" (→ inquiry form or contact)

### Section 2 — What is the Executive Summit?
- 3–4 sentence positioning paragraph
- NOT a description of TIACA generally — assumes the reader already knows TIACA
- Focus: what makes this event unique (closed-door, senior-only, format, conversations)
- Glyn's emphasis: youngest-ever presenter lineup (22 & 24 yrs old) — differentiation narrative worth including
- 3 icons/stats: attendee seniority level, number of countries, sponsor exposure

### Section 3 — Who Attends
- Attendee profile: C-suite, VPs, Directors across airlines, airports, freight forwarders, handlers, tech providers
- Framing: "The room you want to be in"
- Short copy only — does not need full segmentation here

### Section 4 — Agenda / Program Highlights
- High-level agenda (morning/afternoon blocks, not granular session times)
- Speaker placeholders if full lineup not yet confirmed (e.g. "Industry Leader, TBA — full program announced May 2026")
- Keynote themes aligned to TIACA pillars: Digitalization · Sustainability · Safety · Next-Gen Leadership
- Evening function highlights if applicable (networking dinner, venue details)

### Section 5 — Sponsorship
- 2–3 sponsor tiers (or "Contact us for packages" if tiers aren't finalized)
- What sponsors get: logo placement, speaking opportunity, networking access, content capture exposure
- Visual: clean tier comparison table or benefit cards
- **CTA**: "Request Sponsorship Info" → inquiry form → routes to Glyn or Riley

### Section 6 — Warsaw / LOT Cargo Host Feature
- Short acknowledgment of LOT Cargo as host
- Warsaw as a destination (logistics hub, European gateway angle)
- Practical: link to venue, accommodation recommendations if available

### Section 7 — Registration
- Two-track CTA block:
  - Attendee: "Register to Attend" → Glue Up or direct link
  - Sponsor/Partner: "Become a Sponsor" → form or email
- Contact: Riley's email (once confirmed) for practical queries

### Section 8 — Footer
- TIACA logo + copyright
- Link back to tiaca.org
- Social: LinkedIn, Twitter/X
- "Powered by" if applicable

---

## 4. Technical Approach

### Platform

**Option A — Webflow (recommended)**
- Fastest high-fidelity build for a single conversion page
- No CMS handover needed — Riley/Rachel don't need to edit this frequently
- Clean responsive grid, easy sponsor section layout
- Can be hosted on its own subdomain: `summit.tiaca.org` or `events.tiaca.org/warsaw-2026`

**Option B — WordPress page (on existing site)**
- Only if TIACA.org redesign is also on WordPress
- Avoids separate hosting setup
- Less design flexibility for a premium single-page layout
- Risk: existing WP theme constraints may limit the visual premium feel

**Option C — Static site (React + Tailwind, Vercel/Netlify deploy)**
- Fastest build performance, best Core Web Vitals scores
- No client editing capability post-launch (fine for a 6-week event page)
- Could be served from `tiaca.org` subdirectory via reverse proxy if needed
- **Recommended if timeline is very tight** — Sirak dev team can build in 2–3 days

**Decision**: Confirm with Glyn/Riley. Lean toward Option A (Webflow) or Option C (static) given the 1-week build window.

### URL strategy
- Preferred: `executive-summit.tiaca.org` (clean, permanent, reusable next year)
- Fallback: `tiaca.org/executive-summit-2026` (subdirectory if subdomain setup is slow)
- DNS: coordinate with Rachel / hosting provider

### Registration integration
- Primary: link to existing Glue Up registration event page (Rachel provides the URL)
- Sponsor inquiry: form → email to Riley or Glyn (confirm routing)
- Form tool: Webflow native / Typeform / Gravity Forms — whatever fits the build platform

### Analytics
- Google Analytics 4 with UTM-tagged links from TIACA.org and email campaigns
- Conversion events: Registration CTA click, Sponsor inquiry form submission
- Heatmapping: Hotjar or Microsoft Clarity (free tier) for quick optimization insight post-launch

---

## 5. Brand & Visual Direction

| Element | Spec |
|---|---|
| Primary color | TIACA blue `#005A9B` |
| Accent | Dark navy or charcoal for premium contrast (confirm from Brandbook) |
| Typography | Per Brandbook — confirm with Rachel before design begins |
| Logo | TIACA master wordmark |
| Sub-event identity | "Executive Summit 2026" lockup — consistent with TIACA master brand |
| Photography | Warsaw / aviation executive / high-end corporate event imagery |
| Tone | Premium, closed-door, high-stakes. NOT mass-market. NOT generic conference. |
| Visual feel | Dark/deep blue hero with clean white content areas — authoritative and modern |

**Design reference brief for Figma**: Think `Davos / TED Summit` energy, filtered through air cargo B2B. Premium whitespace, confident typography, subtle motion on scroll.

---

## 6. Asset Requirements (needed before build)

**From Riley** (event manager, primary source for this project):
- [ ] Official Summit event registration link (Glue Up URL)
- [ ] Confirmed attendee seniority / expected headcount
- [ ] Full agenda or working agenda (even placeholder session titles)
- [ ] Confirmed speakers (names, titles, companies) — even partial list
- [ ] Sponsor tiers and packages (or confirmation that Sirak will develop these)
- [ ] Any existing Warsaw/LOT Cargo event branding or visuals
- [ ] Evening event details (venue, format)
- [ ] Riley's email address for sponsor inquiry routing

**From Rachel**:
- [ ] DNS access or subdomain creation capability (`executive-summit.tiaca.org`)
- [ ] Confirmation of Google Analytics account to add the new property to
- [ ] Any existing Summit photography or past Summit assets

**From Glyn**:
- [ ] Proposal approval ($7,400) — gates all build work
- [ ] Confirmation of spokesperson for sponsor inquiries (Glyn? Riley?)
- [ ] Any Board-endorsed Summit messaging or talking points

---

## 7. Dashboard Setup (pcg-cc-mcp)

### New project board: **Executive Summit 2026 — Warsaw**

Tasks:

| Task | Status | Owner | Blocked on |
|---|---|---|---|
| Proposal approved by Glyn | `todo` | Aaren | Glyn call |
| Asset collection from Riley | `todo` | Sirak | Proposal approved |
| DNS / URL strategy confirmed | `todo` | Sirak + Rachel | Proposal approved |
| Platform decision (Webflow/WP/Static) | `todo` | Aaren | Asset call |
| Figma design — full page | `todo` | Design | Assets received |
| Internal design review | `todo` | Aaren + Alicia | Figma complete |
| Client design review (Glyn + Riley) | `todo` | Sirak | Internal review done |
| Build — all sections | `todo` | Dev | Design approved |
| Registration + form integration | `todo` | Dev | Build complete |
| Analytics setup (GA4 + events) | `todo` | Dev | Build complete |
| Mobile QA | `todo` | QA | Build complete |
| Cross-browser test | `todo` | QA | Build complete |
| Staging sent to Riley + Glyn | `todo` | Sirak | QA passed |
| Final approval + DNS cutover | `todo` | Glyn | Feedback resolved |
| **Page live** | `todo` | Sirak | Final approval |

### Deliverable review workflow
- Figma design shared via dashboard deliverable link → Riley reviews first (event accuracy), Glyn reviews second (positioning + approval)
- Staging link shared as dashboard deliverable → 2-business-day feedback window (urgent — event is weeks away)
- All change requests logged in dashboard comments, not email threads

---

## 8. Phase Breakdown

### Day 0 — Proposal approval + asset kickoff
- Glyn approves $7,400 proposal on the Glyn call
- Sirak sends Riley the asset request list immediately after
- DNS/URL strategy initiated with Rachel

### Days 1–2 — Design
- Figma hi-fi design: full page all sections
- Internal Aaren + Alicia review and approval
- Design submitted to Riley + Glyn as dashboard deliverable
- **Note**: No wireframe stage for this project — timeline is too tight. Go straight to hi-fi based on the brief above.

### Day 3 — Design review (client)
- Riley reviews for event accuracy (agenda, speakers, registration link)
- Glyn reviews for positioning and sign-off
- Feedback consolidated same day

### Days 4–5 — Build
- Platform setup + page build
- All sections built per approved design
- Registration CTA integrated (Glue Up link from Riley)
- Sponsor inquiry form wired to Riley/Glyn email
- GA4 property created, UTM links prepared

### Day 6 — QA
- Mobile responsiveness (iPhone, Android, iPad)
- Cross-browser (Chrome, Firefox, Safari, Edge)
- All CTAs and forms tested — confirm emails arrive at correct inboxes
- Load speed check
- Staging URL sent to Glyn + Riley

### Day 7 — Launch
- Feedback from staging resolved (same day)
- DNS cutover (subdomain) or URL finalized
- Page live
- Glyn/Riley confirmation
- TIACA.org Executive Summit page updated to link HERE

**Hard deadline**: Live by **May 9, 2026** (3.5 weeks before Jun 1) — allows 3+ weeks of promotional runway for Warsaw.

---

## 9. Content Rules

- **Tone**: This is an executive-level event. No promotional fluff. Confident, specific, outcome-focused copy.
- **Avoid**: generic "join us for a great experience" language. Replace with specifics: who attends, what decisions get made, what access you get.
- **Sponsor framing**: Sponsorship = business access, not logo placement. Emphasize the room, the relationships, the exclusivity.
- **Glyn's transition**: Do NOT feature Glyn personally. Institutional voice only. He may appear in a quote if he approves, but not as the identity of the event.
- **Youth angle**: Include the 22 & 24-year-old presenter story — it's a differentiation signal and shows TIACA's forward-facing positioning.
- **LOT Cargo**: Acknowledge as host partner prominently. They funded/supported hosting — recognizing them appropriately is a relationship asset.

---

## 10. Post-Launch — Content from Warsaw

Once the page is live, it serves a second purpose during and after the event:

- **During** (Jun 1–3): Riley posts live LinkedIn stories. This page receives the traffic from those posts.
- **After** (Jun 4+): Page updated with recap content (aftermovie embed, photo gallery, key quotes) — this content is produced under the Warsaw Event Coverage proposal ($9,750 + travel). The landing page becomes a persistent post-event asset that feeds ACF promotion.

The Warsaw coverage proposal and this site proposal are **architecturally linked**. Winning both is the correct commercial outcome.

---

## 11. Open Questions

- [ ] Exact registration URL from Glue Up (Riley)
- [ ] Confirmed speaker list (even partial) for credibility section
- [ ] Sponsor tier structure (existing or does Sirak build from scratch?)
- [ ] Evening event venue in Warsaw — include or keep mystery?
- [ ] Expected attendee count and seniority split
- [ ] Preferred page URL: subdomain or subdirectory?
- [ ] Who handles post-event page updates: Riley or Sirak?
