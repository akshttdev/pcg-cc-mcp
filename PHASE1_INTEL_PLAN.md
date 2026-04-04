# Phase 1 Intelligence — Full Design & Implementation Plan
# Power Club Global — PCG CRM
> Date: 2026-03-20 | Status: PLAN

---

# PART 1 — PERSON PROFILE PAGE (Complete Makeover)

## Current State Problems
- Intel snapshot is a truncated text blob buried in the 3rd column card
- Social tab is always blank (research never writes to person_social_profiles)
- Vibe, online presence quality — columns don't exist
- [Intel] button is a small ghost button in the corner — no hierarchy
- The profile reads like a contact card, not a person brief

---

## New Person Profile Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Back to People                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  ┌──────────────────────────────────────────────────────────────────────┐    │
│  │  PERSON HEADER CARD                                                   │    │
│  │                                                                       │    │
│  │   ┌─────────┐   Stephanie Bruno                                       │    │
│  │   │         │   Business Owner & Founder · @Bad Girl Strength Club ↗  │    │
│  │   │  AVATAR │                                                         │    │
│  │   │         │   ● LEAD   ◆ GIVER   ◇ early-stage                     │    │
│  │   └─────────┘                                                         │    │
│  │                ╔══════════════════════════════════╗                   │    │
│  │                ║  Direct · Empowering · Raw        ║  ← VIBE CHIPS   │    │
│  │                ╚══════════════════════════════════╝                   │    │
│  │                                                                       │    │
│  │   Online Presence: ● moderate    Last Activity: Mar 14, 2026          │    │
│  │                                                                       │    │
│  │   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐  │    │
│  │   │  🧠  INTEL       │  │  📊  RESEARCH    │  │  📄  REPORT      │  │    │
│  │   │  [PRIMARY BTN]   │  │  [SECONDARY]     │  │  [SECONDARY]     │  │    │
│  │   └──────────────────┘  └──────────────────┘  └──────────────────┘  │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  INTEL SUMMARY STRIP  (teaser — not the full page)                  │     │
│  │                                                                      │     │
│  │  Confidence ████████░░ 72%   · 1 research pass · Scout              │     │
│  │                                                                      │     │
│  │  "Stephanie Bruno is the founder of Bad Girl Strength Club, a        │     │
│  │   women's fitness subscription brand transitioning from 1-on-1       │     │
│  │   coaching to a scalable app model. She leads with raw authenticity  │     │
│  │   and anti-diet-culture positioning..."                              │     │
│  │                                                                      │     │
│  │                               [ View Full Intel Page → ]            │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌────────────────────────┐  ┌────────────────────────┐  ┌────────────────┐  │
│  │  CONTACT               │  │  PROFILE               │  │  SOCIAL RAIL   │  │
│  │                        │  │                        │  │                │  │
│  │  ✉ stephanie@...       │  │  Lifecycle    LEAD     │  │  IG @stephie.. │  │
│  │  📱 +1 (555) ...       │  │  Fin. Role    giver    │  │     3.4k       │  │
│  │  🌐 badgirlstr...      │  │  Biz Stage    early    │  │                │  │
│  │                        │  │  Lead Score   ░░░░ 0   │  │  in linkedin.. │  │
│  │  via  Instagram        │  │                        │  │                │  │
│  │  pref Email            │  │  Bad Girl Strength ↗   │  │  🌐 badgirl..  │  │
│  └────────────────────────┘  └────────────────────────┘  └────────────────┘  │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  TABS: Overview  │  Social  │  Notes  │  Research  │  Reports       │     │
│  ├─────────────────────────────────────────────────────────────────────┤     │
│  │  [tab content]                                                       │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### What changes on the Person Profile:

| Section | Currently | New |
|---------|-----------|-----|
| Header | Name + small badge row | Name + company link + badge row + **VIBE CHIPS** |
| Action buttons | Small ghost "Research" + tiny "Intel" | **[INTEL] primary** + [Research] + [Report] |
| Intel snapshot | Truncated text in 3rd column card | Full-width **INTEL SUMMARY STRIP** with confidence, excerpt, → link |
| Online presence | Not shown | Dot indicator (strong/moderate/minimal/none) in header |
| Social | 3rd column is generic Intel card | Dedicated **SOCIAL RAIL** card — platform icons, handles, follower counts |
| Contact card | Email, phone, website | Same + onboarding channel + preferred contact |

---

# PART 2 — PERSON INTEL PAGE (Full Wiki)

## Reference: business-reports.tsx visual language
The person intel page should use identical card/section components as the business analysis review page.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Profile     Intelligence Profile · Mar 20, 2026     [Run Pass] [Report] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  INTELLIGENCE PROFILE · STEPHANIE BRUNO                                      │
│  Business Owner & Founder · Bad Girl Strength Club                           │
│  ─────────────────────────────────────────────────                           │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  📋 EXECUTIVE SUMMARY                                                │     │
│  │                                                                      │     │
│  │  "Stephanie Bruno is the founder of Bad Girl Strength Club, a        │     │
│  │   women's fitness subscription brand pivoting from 1-on-1 personal   │     │
│  │   coaching to a scalable digital app. Operating on Passion.io, the   │     │
│  │   brand occupies anti-diet-culture, empowerment-first positioning    │     │
│  │   with strong community appeal but currently limited digital         │     │
│  │   infrastructure and marketing reach."                               │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  ✦ VIBE                                                              │     │
│  │                                                                      │     │
│  │   ┌──────────────┐  ┌────────────────┐  ┌────────────────────────┐  │     │
│  │   │   Direct     │  │  Empowering    │  │  Anti-establishment     │  │     │
│  │   └──────────────┘  └────────────────┘  └────────────────────────┘  │     │
│  │                                                                      │     │
│  │  Communication:  Raw, peer-to-peer. No corporate polish.             │     │
│  │                  Speaks to women who are done with toxic gym culture. │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌──────────────────────────────────┐  ┌───────────────────────────────┐     │
│  │  💼 PROFESSIONAL BACKGROUND      │  │  📍 MARKET POSITIONING        │     │
│  │                                  │  │                               │     │
│  │  Founder and operator of Bad     │  │  Women's strength & fitness   │     │
│  │  Girl Strength Club. Career      │  │  niche. Anti-diet-culture.    │     │
│  │  built around fitness coaching,  │  │  Targets women 25–45 who      │     │
│  │  entrepreneurship, and strength  │  │  rejected traditional gym     │     │
│  │  training philosophy. No prior   │  │  culture. Currently local +   │     │
│  │  major employer history found.   │  │  early-stage digital.         │     │
│  └──────────────────────────────────┘  └───────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  🌐 ONLINE PRESENCE                     Presence: ● moderate        │     │
│  │                                                                      │     │
│  │  Platform     Handle              Followers  Notes                   │     │
│  │  ─────────────────────────────────────────────────────────────────  │     │
│  │  Instagram    @stephie.b.fit       ~3,400    Primary channel.       │     │
│  │                                              Workout clips, wins,   │     │
│  │                                              community posts.       │     │
│  │  Website      badgirlstrength.com    —       Passion.io hosted.     │     │
│  │                                              Sparse landing page.   │     │
│  │  TikTok       Not found              —       Opportunity gap.       │     │
│  │  LinkedIn     Not found              —       Low priority.          │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌──────────────────────────────────┐  ┌───────────────────────────────┐     │
│  │  🧠 EXPERTISE                    │  │  👥 TARGET AUDIENCE           │     │
│  │                                  │  │                               │     │
│  │  · Strength & resistance         │  │  · Women 25–45                │     │
│  │    training programming          │  │  · Anti-diet mindset          │     │
│  │  · Women's fitness coaching      │  │  · Gym-hesitant beginners     │     │
│  │  · Subscription app content      │  │  · App-native fitness         │     │
│  │  · Community building            │  │    consumers                  │     │
│  │  · Personal brand operations     │  │                               │     │
│  └──────────────────────────────────┘  └───────────────────────────────┘     │
│                                                                               │
│  ┌──────────────────────────────────┐  ┌───────────────────────────────┐     │
│  │  💡 OPPORTUNITIES                │  │  ⚠ RISK SIGNALS               │     │
│  │                                  │  │                               │     │
│  │  · App subscription at scale     │  │  · Founder-dependent brand    │     │
│  │  · TikTok acquisition funnel     │  │  · Personal life disruption   │     │
│  │    (untapped)                    │  │    (divorce, relocation)       │     │
│  │  · Merch + community events      │  │  · Platform lock-in           │     │
│  │  · Brand partnership potential   │  │    (Passion.io limits)        │     │
│  │  · PCG brand + content stack     │  │  · Budget sensitivity         │     │
│  └──────────────────────────────────┘  └───────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  🎯 COMPETITIVE LANDSCAPE                                            │     │
│  │                                                                      │     │
│  │  · Barre3, Obé Fitness, Peloton (established digital fitness)       │     │
│  │  · Local women's gyms and boutique studios                          │     │
│  │  · Free YouTube fitness creators (@growwithjo, @sydneycummings)     │     │
│  │                                                                      │     │
│  │  Differentiator: Raw, unpolished authenticity + community intimacy. │     │
│  │  Cannot win on production value — must win on brand depth + tribe.  │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  🔬 INTEL SOURCES — Research History                                 │     │
│  │                                                                      │     │
│  │  ① Pass 1 · identity · Mar 20, 2026 · confidence +72%              │     │
│  │    Scout identified Stephanie as fitness founder with Instagram      │     │
│  │    primary channel. Limited public footprint — brand is grassroots. │     │
│  │                                                                      │     │
│  │  [Run Next Pass →]                                                   │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  📄 REPORTS LINKED                                                   │     │
│  │                                                                      │     │
│  │  Business Audit: Bad Girl Strength Club · Mar 14, 2026 · ready →    │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Person Intel Page Sections (in order):

| # | Section | Data source | Icon |
|---|---------|-------------|------|
| 1 | Executive Summary | `intelligence_raw.summary` | BookOpen |
| 2 | Vibe | `person.vibe` + `intelligence_raw.communication_style` | Star/Fingerprint |
| 3 | Professional Background | `intelligence_raw.background` | Briefcase |
| 4 | Market Positioning | `intelligence_raw.positioning` | Target |
| 5 | Online Presence | `intelligence_raw.social_profiles[]` + `person.online_presence_quality` | Globe |
| 6 | Expertise | `intelligence_raw.expertise[]` | Brain |
| 7 | Target Audience | `intelligence_raw.target_clients[]` | Users |
| 8 | Opportunities | `intelligence_raw.opportunities[]` | Lightbulb |
| 9 | Risk Signals | `intelligence_raw.risks[]` | AlertTriangle |
| 10 | Competitive Landscape | `intelligence_raw.competitors[]` | Target |
| 11 | Network & Relationships | `intelligence_raw.network` | Network |
| 12 | Strategy | `intelligence_raw.strategy` | TrendingUp |
| 13 | Intel Sources / Research History | `person_research_passes[]` | Fingerprint |
| 14 | Linked Reports | `business_reports[]` | FileText |

---

# PART 3 — COMPANY PROFILE PAGE (Consistent with Org Structure)

## Design principle: mirror how Organization profile pages are structured
The company profile should feel like a "brand entity card" — one level above a person, one level below an org.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Back to Companies                                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  ┌──────────────────────────────────────────────────────────────────────┐    │
│  │  COMPANY HEADER CARD                                                  │    │
│  │                                                                       │    │
│  │   ┌──────────┐   Bad Girl Strength Club                               │    │
│  │   │  [LOGO]  │   Health & Fitness · badgirlstrength.com ↗            │    │
│  │   │  or icon │                                                        │    │
│  │   └──────────┘   📍 United States  ·  👥 1–10  ·  ⭐ —              │    │
│  │                                                                       │    │
│  │   ┌────────────────────┐  ┌────────────────────┐  ┌────────────────┐ │    │
│  │   │  🧠  INTEL         │  │  🎨  BRAND GUIDE   │  │  🔬 RESEARCH   │ │    │
│  │   │  [PRIMARY BTN]     │  │  [PRIMARY BTN]     │  │  [SECONDARY]   │ │    │
│  │   └────────────────────┘  └────────────────────┘  └────────────────┘ │    │
│  └──────────────────────────────────────────────────────────────────────┘    │
│                                                                               │
│  ┌──────────────────────────────────┐  ┌───────────────────────────────┐     │
│  │  CONTACT & PRESENCE              │  │  SOCIAL HANDLES               │     │
│  │                                  │  │                               │     │
│  │  🌐 badgirlstrength.com          │  │  IG  @badgirlstrengthclub     │     │
│  │  📱 —                            │  │  TW  —                        │     │
│  │  ✉  —                            │  │  LI  —                        │     │
│  │  📍 United States                │  │  FB  —                        │     │
│  │                                  │  │  TT  —                        │     │
│  │  GMB Rating: —  (0 reviews)      │  │                               │     │
│  │  Founded:    —                   │  │  Online Presence: ● moderate  │     │
│  └──────────────────────────────────┘  └───────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  BRAND VOICE STRIP                                                   │     │
│  │  "Bold, raw, anti-diet-culture. Speaks directly to women who are     │     │
│  │   done with toxic fitness culture. No corporate polish."             │     │
│  │                                 [ View Brand Guide → ]              │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  TABS:  Overview  │  People  │  Deals  │  Intel  │  Brand Guide     │     │
│  ├─────────────────────────────────────────────────────────────────────┤     │
│  │  [tab content]                                                       │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Company Profile Tabs:

| Tab | Content |
|-----|---------|
| **Overview** | Intel summary excerpt, contact/presence cards, brand voice strip, associated people list |
| **People** | All persons with `company_id = this_company` — linked to their profiles |
| **Deals** | CRM deals linked to this company |
| **Intel** | Intel button → opens full company intel page |
| **Brand Guide** | Brand Guide button → opens brand guide page (existing route) |

### Key UI differences from current company list page:
- Logo prominently displayed (from `companies.logo_url` seeded by research)
- Social handles shown as clickable platform links
- Brand voice shown as a strip (not buried in a tab)
- [INTEL] and [BRAND GUIDE] are co-equal primary actions
- GMB-style stats row: location, employee count, rating, founded year

---

# PART 4 — COMPANY INTEL PAGE (Full Wiki)

## Same visual language as Person Intel + Business Analysis review page

```
┌─────────────────────────────────────────────────────────────────────────────┐
│  ← Company Profile     Company Intelligence · Mar 20, 2026   [Run] [Guide] │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                               │
│  COMPANY INTELLIGENCE WIKI                                                   │
│  Bad Girl Strength Club · Health & Fitness                                   │
│  ─────────────────────────────────────────                                   │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  📋 EXECUTIVE SUMMARY                                                │     │
│  │                                                                      │     │
│  │  "Bad Girl Strength Club is an early-stage women's fitness brand     │     │
│  │   built around strength training and body empowerment. Operating     │     │
│  │   as a Passion.io subscription app, it is transitioning from a      │     │
│  │   personal coaching model to scalable digital delivery. The brand    │     │
│  │   speaks boldly to women rejecting toxic diet culture but currently  │     │
│  │   lacks the production infrastructure to match its ambition."        │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌──────────────────────────────────┐  ┌───────────────────────────────┐     │
│  │  🏢 COMPANY OVERVIEW             │  │  🌐 DIGITAL PRESENCE          │     │
│  │                                  │  │                               │     │
│  │  Type:      Subscription App     │  │  Website  badgirlstrength.com │     │
│  │  Stage:     Early / Pre-revenue  │  │  IG       @badgirlstrengthclub│     │
│  │  Platform:  Passion.io           │  │           ~3,400 followers    │     │
│  │  Founded:   ~2024                │  │  TikTok   ✗ not found        │     │
│  │  Team:      Solo founder         │  │  Twitter  ✗ not found        │     │
│  │  Revenue:   Pre-scale            │  │  LinkedIn ✗ not found        │     │
│  │  Location:  USA (remote)         │  │                               │     │
│  │                                  │  │  Presence quality: moderate   │     │
│  └──────────────────────────────────┘  └───────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  🎙 BRAND VOICE & LANGUAGE                                           │     │
│  │                                                                      │     │
│  │  Tone:       Bold · Raw · Anti-establishment · Empowering            │     │
│  │  Language:   Peer-to-peer. "We" not "our clients." Profanity-        │     │
│  │              friendly. Anti-diet, pro-strength.                      │     │
│  │  Archetype:  The Rebel — disrupts traditional fitness culture        │     │
│  │  Avoid:      Corporate speak, diet language, before/after framing    │     │
│  │                                                                      │     │
│  │                              [ Open Brand Guide → ]                 │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  📊 MARKET POSITION                                                  │     │
│  │                                                                      │     │
│  │  Niche:         Women's strength & fitness subscription              │     │
│  │  TAM:           Large — women's fitness is $X B globally             │     │
│  │  Differentiator: Anti-diet positioning + founder authenticity        │     │
│  │  Stage:         Pre-product-market fit, building audience first      │     │
│  │  Geography:     US-based, digital-first potential                    │     │
│  │                                                                      │     │
│  │  STRENGTHS                        WEAKNESSES                        │     │
│  │  ─────────────────────            ──────────────────────            │     │
│  │  ✓ Unique brand positioning        ✗ Founder-dependent              │     │
│  │  ✓ Authentic community feel        ✗ Platform-constrained           │     │
│  │  ✓ Underserved niche               ✗ No acquisition funnel          │     │
│  │  ✓ App-native content potential    ✗ Amateur brand assets           │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌──────────────────────────────────┐  ┌───────────────────────────────┐     │
│  │  🎯 SERVICES THEY NEED           │  │  ⚡ PAIN POINTS               │     │
│  │                                  │  │                               │     │
│  │  · Brand identity development    │  │  HIGH                         │     │
│  │  · Content strategy + production │  │  · App content too text-heavy │     │
│  │  · Sales funnel architecture     │  │  · No cohesive brand identity │     │
│  │  · Video sales landing page      │  │  · Zero acquisition funnel    │     │
│  │  · Marketing automation setup    │  │                               │     │
│  │  · App UX improvement plan       │  │  MEDIUM                       │     │
│  │                                  │  │  · Platform tech limitations  │     │
│  │  Recommended Tier: Tier 2        │  │  · Founder bandwidth          │     │
│  └──────────────────────────────────┘  └───────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  🏁 COMPETITIVE LANDSCAPE                                            │     │
│  │                                                                      │     │
│  │  Direct competitors (digital fitness subscriptions):                │     │
│  │  · Peloton App · Obé Fitness · Sweat (Kayla Itsines)               │     │
│  │  · Barre3 · Ladder · Tone It Up                                     │     │
│  │                                                                      │     │
│  │  Content creators in adjacent space:                                │     │
│  │  · @growwithjo (YouTube) · @sydneycummings · @heatherrobertson      │     │
│  │                                                                      │     │
│  │  Advantage: None can offer BGSC's raw founder authenticity.         │     │
│  │  Threat: They have production budgets. BGSC needs to close gap fast.│     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  👥 KEY PEOPLE                                                       │     │
│  │                                                                      │     │
│  │  Stephanie Bruno  ·  Founder & CEO  →  [View Profile]              │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
│                                                                               │
│  ┌─────────────────────────────────────────────────────────────────────┐     │
│  │  🔬 RESEARCH HISTORY                                                 │     │
│  │                                                                      │     │
│  │  ① Pass 1 · identity · Mar 20, 2026 · confidence 72%               │     │
│  │    Scout conducted initial scan. Limited public footprint.           │     │
│  │    Instagram confirmed as primary channel.                          │     │
│  │                                                                      │     │
│  │  [Run Company Research →]                                            │     │
│  └─────────────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Company Intel Page Sections (in order):

| # | Section | Data source |
|---|---------|-------------|
| 1 | Executive Summary | `companies.intelligence_summary` |
| 2 | Company Overview | `companies.*` (type, stage, platform, team size) |
| 3 | Digital Presence | `companies.instagram_handle/twitter/linkedin` + presence quality |
| 4 | Brand Voice & Language | `company_brand_profiles.brand_voice` + tone chips |
| 5 | Market Position | `companies.intelligence_raw.positioning` — SWOT mini-grid |
| 6 | Services They Need | `business_reports.recommended_services[]` if exists |
| 7 | Pain Points | `business_reports.pain_points[]` with severity badges |
| 8 | Competitive Landscape | `companies.intelligence_raw.competitors[]` |
| 9 | Key People | All persons with `company_id = this_company` |
| 10 | Research History | `companies.intelligence_raw` passes |

---

# PART 5 — DATA FLOW (Research → All Pages)

```
  POST /api/persons/:id/research
            │
            ▼
  ┌─────────────────────┐
  │  LLM (JSON schema)  │
  │  Scout / GPT-4o     │
  └──────────┬──────────┘
             │  returns structured IntelJson
             ▼
  ┌──────────────────────────────────────────────────────────┐
  │  write_intelligence_results()  — THE FAN-OUT             │
  │                                                          │
  │  ① persons                                              │
  │     .intelligence_summary  ← json.summary               │
  │     .intelligence_raw      ← full json string           │
  │     .vibe                  ← json.vibe                  │
  │     .online_presence_quality ← json.online_presence_q   │
  │     .intelligence_status   = 'done'                     │
  │     .intelligence_confidence ← json.confidence          │
  │                                                          │
  │  ② person_social_profiles  (upsert per platform)        │
  │     platform, handle, profile_url, follower_count, bio  │
  │                                                          │
  │  ③ companies               (COALESCE — never overwrite) │
  │     instagram_handle, twitter_handle, linkedin_url       │
  │     facebook_url, logo_url, website                      │
  │     description, industry                                │
  │                                                          │
  │  ④ company_brand_profiles  (INSERT OR IGNORE + UPDATE)  │
  │     brand_voice, website_url                            │
  │     social_instagram/twitter/linkedin/facebook           │
  │     logo_url, industry                                   │
  │     research_status = 'done'                            │
  └──────────────────────────────────────────────────────────┘
             │
     ┌───────┴──────────────────────────────────────┐
     ▼               ▼               ▼              ▼
  Person         Social          Company        Brand Guide
  Profile        Tab             Profile        (seeded)
  ─────────      ──────          ───────        ────────────
  vibe badge     platform cards  social rail    brand_voice
  intel strip    handles         logo           social handles
  online dot     followers       industry       logo_url
  ↓              bio             presence dot   research_status
  Intel Page     ↓               ↓
  (all sections  person-intel    Company
  populated)     social section  Intel Page
```

---

# PART 6 — IMPLEMENTATION FILES

## Total scope: 1 new file, 7 modified

```
crates/db/migrations/
  └── 20260421000000_person_intel_fields.sql        [NEW]
        ALTER TABLE persons ADD COLUMN vibe TEXT;
        ALTER TABLE persons ADD COLUMN online_presence_quality TEXT;

crates/db/src/models/
  └── person.rs                                     [MODIFY]
        + vibe: Option<String>
        + online_presence_quality: Option<String>
        (Person struct only — no UpdatePerson change needed)

crates/server/src/routes/
  └── intelligence.rs                               [MODIFY — largest change]
        + IntelJson struct
        + IntelSocialProfile struct
        + parse_intel_json_typed() helper
        + rewrite run_research_direct() prompt → JSON schema
        + rewrite write_intelligence_results() → fan-out (4 writes)
        + import PersonSocialProfile + UpsertPersonSocialProfile

frontend/src/lib/api/
  └── communication.ts                              [MODIFY]
        PersonRecord interface:
          + vibe?: string
          + online_presence_quality?: string

frontend/src/pages/
  ├── person-profile.tsx                            [MODIFY]
  │     Header: + vibe chips, + online presence dot, + [INTEL] primary button
  │     New: INTEL SUMMARY STRIP (full-width between header and 3-col row)
  │     OverviewTab: + social mini-rail replacing intel 3rd card
  │
  ├── person-intel.tsx                              [MODIFY]
  │     + Vibe section (after executive summary)
  │     + Online Presence section with table layout (after vibe)
  │     All existing sections now populate from JSON keys
  │
  └── company-profile/
        ├── index.tsx                               [MODIFY]
        │     Header: + logo display, + [INTEL] + [BRAND GUIDE] primary buttons
        │     + brand voice strip
        │     + social handles card
        │     + GMB stats row
        │
        └── tabs/IntelligenceTab.tsx               [MODIFY — or new CompanyIntelPage]
              Full wiki layout matching person-intel.tsx
              + SWOT mini-grid in Market Position
              + Brand Voice chips
              + Key People linked list
```

---

# PART 7 — DEFINITION OF DONE

### Research pipeline
- [ ] `intelligence_raw` stored as valid JSON (not plain text)
- [ ] `persons.vibe` populated after research
- [ ] `persons.online_presence_quality` populated
- [ ] `person_social_profiles` rows created per platform found
- [ ] `companies.instagram_handle` (and others) updated if company linked
- [ ] `company_brand_profiles.brand_voice` set + `research_status = 'done'`

### Person Profile page
- [ ] Vibe chips display in header
- [ ] [INTEL] button is primary / prominent
- [ ] Intel summary strip visible between header and 3-col row
- [ ] Social rail card shows platform links + follower counts
- [ ] Online presence quality dot visible
- [ ] Social tab shows real data (not blank)

### Person Intel page
- [ ] Vibe section renders with chips
- [ ] Online Presence section shows table with platform rows
- [ ] All JSON keys render (background, positioning, expertise, etc.)
- [ ] Research history section shows pass cards with confidence delta
- [ ] Linked reports section shows business audit card

### Company Profile page
- [ ] Logo shown in header (from logo_url)
- [ ] [INTEL] and [BRAND GUIDE] are co-equal primary buttons
- [ ] Social handles card shows all platforms found
- [ ] Brand voice strip visible
- [ ] People tab lists associated persons

### Company Intel page
- [ ] Executive summary renders
- [ ] Digital Presence section shows all social handles
- [ ] Brand Voice section renders with tone chips
- [ ] Market Position section has SWOT mini-grid
- [ ] Key People linked to profiles
- [ ] Research history shown

### Build
- [ ] `cargo build -p server` exits 0
- [ ] `npm run build` exits 0 (no TS errors)
- [ ] Server restarts clean
