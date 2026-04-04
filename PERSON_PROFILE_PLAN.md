PHASE 1 SCOUT — PERSON PROFILE & INTEL ENHANCEMENT PLAN
==========================================================
2026-03-26


═══════════════════════════════════════════════════════════════
SECTION 1: CURRENT STATE vs. TARGET STATE
═══════════════════════════════════════════════════════════════

  CURRENT PERSON PROFILE PAGE (person-profile.tsx)
  ─────────────────────────────────────────────────
  ┌──────────────────────────────────────────────────────────┐
  │  [Avatar initials]  Full Name                            │
  │  Job Title · Company Name                                │
  │  [lead_score badge] [lifecycle_stage] [research_depth]  │
  │  ─────────────────────────────────────────────────────   │
  │  Tabs: Overview │ Notes │ Activity │ Deals               │
  │                                                          │
  │  Overview:                                               │
  │  Contact Info: email, phone                              │
  │  Profile: financial_role, business_stage, lifecycle      │
  │  Intel Snapshot: status badge, confidence %, summary     │
  │  Key Facts: positioning, specialization (from intel_raw) │
  │  Company Roles: name, role, dates                        │
  └──────────────────────────────────────────────────────────┘

  PROBLEMS:
  • No photo — just initials in a grey circle
  • No social media links/handles/follower counts
  • No media appearances, press mentions, podcast features
  • Intel summary buried in a small card, 3-line clamp
  • Company affiliations show as a flat list with no context
  • No content strategy or personal brand preview
  • No geographic/location data
  • No PCG opportunity assessment visible at a glance
  • Intel page is a raw research pass log — not a wiki

  CURRENT INTEL PAGE (person-intel.tsx)
  ──────────────────────────────────────
  ┌──────────────────────────────────────────────────────────┐
  │  Person: [name] · Intel Score [confidence%]             │
  │  ─────────────────────────────────────────────────────   │
  │  Tabs: Intelligence │ Research Passes │ Reports          │
  │                                                          │
  │  Intelligence tab:                                       │
  │    Executive Summary (text block)                        │
  │    Background / Market Positioning / Expertise           │
  │    Target Clients / Network / Strategy                   │
  │    Comm Style / Opportunities / Risks                    │
  │                                                          │
  │  Research Passes tab:                                    │
  │    Pass timeline: pass# · focus · summary                │
  │    key_findings list · confidence_delta                  │
  │                                                          │
  │  Reports tab:                                            │
  │    Business report cards                                 │
  └──────────────────────────────────────────────────────────┘

  PROBLEMS:
  • No sources / citations displayed
  • No social presence section
  • No media appearances
  • No business affiliations depth
  • Not visually rich — walls of text
  • No PCG-specific opportunity scoring
  • Research passes show raw data, not synthesized narrative
  • No comparison / diff between passes


═══════════════════════════════════════════════════════════════
SECTION 2: SCOUT WORKFLOW ENHANCEMENT — WHAT TO GATHER
═══════════════════════════════════════════════════════════════

  CURRENT PHASE 1 PASS STRUCTURE (5 passes, web_search tool):
  ─────────────────────────────────────────────────────────────
  Pass 1 → identity
  Pass 2 → market_position
  Pass 3 → competitors
  Pass 4 → target_clients
  Pass 5+ → deep_strategy

  ENHANCED PHASE 1 PASS STRUCTURE:
  ─────────────────────────────────────────────────────────────
  Pass 1 → identity + social_discovery
           NEW: extract photo_url, all social handles,
                follower counts, platform bios, location

  Pass 2 → market_position + business_affiliations
           NEW: extract all companies (past + present),
                roles, dates, board seats, advisory roles

  Pass 3 → media_presence + content_strategy
           NEW: podcasts appeared on, press mentions,
                articles written, YouTube/speaking events,
                content themes, posting cadence

  Pass 4 → audience + competitive
           NEW: audience demographics, engagement rates,
                competitor relationships, industry standing

  Pass 5+ → deep_strategy + pcg_opportunity
            NEW: pain points specific to PCG services,
                 budget signals, decision-making style,
                 recommended PCG engagement strategy

  NEW FIELDS TO EXTRACT INTO intelligence_raw JSON:
  ─────────────────────────────────────────────────────────────
  {
    // Identity
    "photo_url": "https://...",
    "location": "Houston, TX",
    "location_verified": true,

    // Social Platforms (array)
    "social_profiles": [
      {
        "platform": "instagram",
        "handle": "@stephie.b.fit",
        "url": "https://instagram.com/stephie.b.fit",
        "follower_count": 12400,
        "following_count": 892,
        "bio": "...",
        "verified": false,
        "engagement_rate": 4.2,
        "post_count": 318,
        "content_themes": ["fitness", "strength training", "mindset"]
      },
      {
        "platform": "tiktok",
        "handle": "@badgirlstrengthclub",
        "url": "https://tiktok.com/@badgirlstrengthclub",
        "follower_count": 8900,
        "bio": "...",
        "avg_views": 15000
      }
    ],

    // Business Affiliations (array)
    "business_affiliations": [
      {
        "company_name": "Bad Girl Strength Club",
        "role": "Founder & CEO",
        "start_date": "2021",
        "end_date": null,
        "current": true,
        "description": "Online fitness coaching platform...",
        "company_url": "https://badgirlstrength.club",
        "company_type": "fitness_coaching"
      }
    ],

    // Media Appearances (array)
    "media_appearances": [
      {
        "type": "podcast",
        "title": "Episode title",
        "publication": "Podcast name",
        "date": "2024-08-01",
        "url": "https://...",
        "summary": "Discussed..."
      }
    ],

    // Existing fields (enhanced)
    "background": "...",
    "positioning": "...",
    "specialization": "...",
    "expertise": [...],
    "target_clients": [...],
    "strategy": "...",
    "communication_style": "...",
    "opportunities": [...],
    "risks": [...],
    "network": [...],
    "competitors": [...],

    // NEW: PCG Opportunity Assessment
    "pcg_opportunity": {
      "score": 8.5,
      "recommended_tier": "Tier 2",
      "recommended_services": ["Brand Kit", "VSL", "Social Strategy"],
      "pain_points": ["No cohesive brand", "Manual content creation"],
      "budget_signals": "Sells $197-497/mo programs, estimated $15k/mo revenue",
      "engagement_style": "Direct, values authenticity, dislikes corporate feel",
      "best_approach": "Lead with brand identity ROI, show before/after case studies"
    },

    // NEW: Content Strategy Profile
    "content_strategy": {
      "primary_platforms": ["instagram", "tiktok"],
      "posting_frequency": "Daily on IG, 3x/week TikTok",
      "content_types": ["reels", "stories", "transformation posts"],
      "brand_voice": "Motivational, direct, no-nonsense",
      "hashtag_strategy": "#badgirlstrengthclub #getbadasfast ...",
      "best_performing_content": "Transformation videos"
    }
  }

  NEW BACKEND CHANGES NEEDED:
  ─────────────────────────────────────────────────────────────
  1. person_social_profiles TABLE enhancement
     ADD: follower_count INT
     ADD: following_count INT
     ADD: bio TEXT
     ADD: verified INT (bool)
     ADD: engagement_rate REAL
     ADD: post_count INT
     ADD: content_themes TEXT (JSON array)
     ADD: avg_views REAL
     ADD: last_synced_at TEXT

  2. person_business_affiliations TABLE (new or use existing person_company_roles)
     - Check if PersonCompanyRole has enough fields
     - ADD: description TEXT
     - ADD: company_type TEXT
     - ADD: company_url TEXT

  3. person_media_appearances TABLE (new)
     person_id FK, type, title, publication, date, url, summary

  4. Run Scout prompts enhanced (intelligence.rs)
     - Pass 1 prompt explicitly asks for photo_url + all socials
     - Pass 2 prompt explicitly asks for all business affiliations
     - Pass 3 prompt explicitly asks for media appearances
     - All passes output structured JSON with above new fields


═══════════════════════════════════════════════════════════════
SECTION 3: NEW PERSON PROFILE PAGE — WIREFRAME
═══════════════════════════════════════════════════════════════

  /persons/:id  (person-profile.tsx — full redesign)

  ┌─────────────────────────────────────────────────────────────────────────┐
  │  ← Back to People                                    [Run Scout ▶]     │
  │                                                       [View Intel  ]    │
  │                                                       [Create Deal ]    │
  ├─────────────────────────────────────────────────────────────────────────┤
  │                                                                         │
  │  ┌──────────────────────────────────────────────────────────────────┐  │
  │  │  HERO SECTION                                                    │  │
  │  │                                                                  │  │
  │  │  ┌────────────┐   Stephanie Bruno                                │  │
  │  │  │            │   Founder & CEO · Bad Girl Strength Club         │  │
  │  │  │  [PHOTO]   │   📍 Houston, TX                                 │  │
  │  │  │  120×120   │                                                  │  │
  │  │  │  rounded   │   [● client] [★ 84] [◎ Tier 2] [⚡ deep intel]  │  │
  │  │  └────────────┘                                                  │  │
  │  │                                                                  │  │
  │  │  ──── SOCIAL PLATFORMS ────────────────────────────────────────  │  │
  │  │                                                                  │  │
  │  │  [📷 Instagram]  @stephie.b.fit      12.4K followers  4.2% eng  │  │
  │  │  [▶  TikTok   ]  @badgirlstrength     8.9K followers            │  │
  │  │  [in LinkedIn ]  /in/stephanieb       2.1K connections          │  │
  │  │  [🔗 Website  ]  badgirlstrength.club                           │  │
  │  │  [🔗 Linktree ]  linktr.ee/badgirlstrengthclub                  │  │
  │  │                                                                  │  │
  │  └──────────────────────────────────────────────────────────────────┘  │
  │                                                                         │
  │  ┌────────────────────────────────┐  ┌──────────────────────────────┐  │
  │  │  BUSINESS AFFILIATIONS         │  │  PCG OPPORTUNITY             │  │
  │  │  ──────────────────────────    │  │  ─────────────────────────   │  │
  │  │                                │  │                              │  │
  │  │  ● Bad Girl Strength Club      │  │  Score:  ████████░░  8.5/10  │  │
  │  │    Founder & CEO  · 2021–now   │  │                              │  │
  │  │    Online fitness coaching     │  │  Tier:   Tier 2              │  │
  │  │    badgirlstrength.club  [↗]   │  │  Budget: ~$15K/mo est.       │  │
  │  │                                │  │                              │  │
  │  │  ○ Passion.io Platform         │  │  Recommended:                │  │
  │  │    Content Creator · 2022–now  │  │  • Brand Kit                 │  │
  │  │    App-based training programs │  │  • VSL + Landing Page        │  │
  │  │                                │  │  • Social Strategy           │  │
  │  │  [+ Add Affiliation]           │  │                              │  │
  │  │                                │  │  [View Full Analysis →]      │  │
  │  └────────────────────────────────┘  └──────────────────────────────┘  │
  │                                                                         │
  │  ┌─────────────────────────────────────────────────────────────────┐   │
  │  │  BACKGROUND & NARRATIVE                                         │   │
  │  │  ─────────────────────────────────────────────────────────────  │   │
  │  │                                                                 │   │
  │  │  Stephanie Bruno is a Houston-based fitness entrepreneur and    │   │
  │  │  founder of Bad Girl Strength Club, an online coaching platform │   │
  │  │  focused on strength training for women. She built her          │   │
  │  │  audience primarily on Instagram (@stephie.b.fit, 12.4K) and   │   │
  │  │  TikTok, positioning her brand around the tagline "Get Badass   │   │
  │  │  Fast." Her program suite includes a Passion.io app with        │   │
  │  │  tiered training programs ($197–$497/mo) and a growing          │   │
  │  │  community of 800+ active members.                              │   │
  │  │                                                                 │   │
  │  │  [Read More ↓]                                                  │   │
  │  │                                                                 │   │
  │  └─────────────────────────────────────────────────────────────────┘   │
  │                                                                         │
  │  ┌────────────────────────────────┐  ┌──────────────────────────────┐  │
  │  │  CONTENT STRATEGY              │  │  MEDIA APPEARANCES           │  │
  │  │  ──────────────────────────    │  │  ─────────────────────────   │  │
  │  │                                │  │                              │  │
  │  │  Primary:  Instagram · TikTok  │  │  🎙 Strong Women Podcast     │  │
  │  │  Voice:    Direct · No-nonsense│  │     Aug 2024  [↗]            │  │
  │  │  Cadence:  Daily IG, 3×/wk TT  │  │                              │  │
  │  │                                │  │  📰 Houston Biz Journal      │  │
  │  │  Top themes:                   │  │     Mar 2024  [↗]            │  │
  │  │  • Strength training           │  │                              │  │
  │  │  • Mindset & confidence        │  │  🎥 YouTube Feature          │  │
  │  │  • Transformations             │  │     Jan 2024  [↗]            │  │
  │  │  • Community building          │  │                              │  │
  │  │                                │  │  [See All →]                 │  │
  │  └────────────────────────────────┘  └──────────────────────────────┘  │
  │                                                                         │
  │  TABS:  [Overview ●]  [Notes]  [Activity]  [Deals]  [Intel]            │
  │                                                                         │
  └─────────────────────────────────────────────────────────────────────────┘


═══════════════════════════════════════════════════════════════
SECTION 4: NEW INTEL / WIKI PAGE — WIREFRAME
═══════════════════════════════════════════════════════════════

  /persons/:id/intel  (person-intel.tsx — full redesign)

  ┌─────────────────────────────────────────────────────────────────────────┐
  │  ← Stephanie Bruno                                                      │
  │                                                                         │
  │  INTELLIGENCE WIKI                     Confidence: ████████░░ 84%      │
  │                                        Passes: 3/5 complete             │
  │                                        Last run: 2026-03-14             │
  │                                        [▶ Run Next Pass]                │
  ├─────────────────────────────────────────────────────────────────────────┤
  │                                                                         │
  │  LEFT SIDEBAR          MAIN CONTENT                                     │
  │  ────────────────       ────────────────────────────────────────────    │
  │  § Overview            ┌──────────────────────────────────────────┐    │
  │  § Social Presence     │  EXECUTIVE SUMMARY                       │    │
  │  § Business Profile    │  ─────────────────────────────────────   │    │
  │  § Background          │  Stephanie Bruno is a fitness-first      │    │
  │  § Market Position     │  entrepreneur who built Bad Girl Strength │    │
  │  § Content Strategy    │  Club from zero into a $15K/mo recurring  │    │
  │  § Audience            │  revenue business. Her platform combines  │    │
  │  § Media Presence      │  app-based programming (Passion.io) with  │    │
  │  § Competitive Intel   │  high-touch coaching for women seeking    │    │
  │  § PCG Opportunity     │  strength-based transformation. She is    │    │
  │  § Research Log        │  the sole decision-maker and responds     │    │
  │  § Business Reports    │  well to direct, results-first pitches.   │    │
  │                        └──────────────────────────────────────────┘    │
  │                                                                         │
  │                        ┌──────────────────────────────────────────┐    │
  │                        │  SOCIAL PRESENCE                         │    │
  │                        │  ─────────────────────────────────────   │    │
  │                        │                                          │    │
  │                        │  ┌─────────────────┐  ┌──────────────┐  │    │
  │                        │  │ 📷 INSTAGRAM     │  │ ▶ TIKTOK    │  │    │
  │                        │  │ @stephie.b.fit   │  │ @badgirl...  │  │    │
  │                        │  │ 12,400 followers │  │ 8,900 follow │  │    │
  │                        │  │ 318 posts        │  │ ~15K avg view│  │    │
  │                        │  │ 4.2% engagement  │  │              │  │    │
  │                        │  │                  │  │              │  │    │
  │                        │  │ Content Themes:  │  │ Content:     │  │    │
  │                        │  │ • Strength       │  │ • Short-form │  │    │
  │                        │  │ • Mindset        │  │ • Transforms │  │    │
  │                        │  │ • Community      │  │ • Workouts   │  │    │
  │                        │  └─────────────────┘  └──────────────┘  │    │
  │                        │                                          │    │
  │                        │  ┌─────────────────┐  ┌──────────────┐  │    │
  │                        │  │ in LINKEDIN      │  │ 🔗 WEBSITE   │  │    │
  │                        │  │ 2,100 conn.      │  │ badgirl...   │  │    │
  │                        │  │ Engaged          │  │ Wix platform │  │    │
  │                        │  └─────────────────┘  └──────────────┘  │    │
  │                        └──────────────────────────────────────────┘    │
  │                                                                         │
  │                        ┌──────────────────────────────────────────┐    │
  │                        │  BUSINESS PROFILE                        │    │
  │                        │  ─────────────────────────────────────   │    │
  │                        │                                          │    │
  │                        │  Primary Venture:                        │    │
  │                        │  ┌────────────────────────────────────┐  │    │
  │                        │  │ Bad Girl Strength Club              │  │    │
  │                        │  │ Founder & CEO  · 2021 → Present    │  │    │
  │                        │  │                                    │  │    │
  │                        │  │ Type:     Online Fitness Coaching  │  │    │
  │                        │  │ Revenue:  ~$15K/mo (est.)          │  │    │
  │                        │  │ Members:  800+ active              │  │    │
  │                        │  │ Platform: Passion.io app           │  │    │
  │                        │  │ Programs: $197–$497/mo             │  │    │
  │                        │  │ Web:      badgirlstrength.club     │  │    │
  │                        │  └────────────────────────────────────┘  │    │
  │                        │                                          │    │
  │                        │  Additional Roles:                       │    │
  │                        │  • Content Creator on Passion.io         │    │
  │                        │  • Community host (Discord/Group)        │    │
  │                        └──────────────────────────────────────────┘    │
  │                                                                         │
  │                        ┌──────────────────────────────────────────┐    │
  │                        │  MARKET POSITION                         │    │
  │                        │  ─────────────────────────────────────   │    │
  │                        │  Stage:     Customer Acquisition         │    │
  │                        │  Category:  Fitness · Coaching · D2C     │    │
  │                        │  Geography: Houston TX + Online          │    │
  │                        │  ICP:       Women 25-45 seeking          │    │
  │                        │             strength transformation       │    │
  │                        │                                          │    │
  │                        │  Competitors:                            │    │
  │                        │  ┌──────────────────────────────────┐   │    │
  │                        │  │ Competitor     Threat  Notes     │   │    │
  │                        │  │ ─────────────  ──────  ───────── │   │    │
  │                        │  │ Sweat Program  Medium  App-based │   │    │
  │                        │  │ BBG / Kayla I  High    Scale/brand│   │    │
  │                        │  │ Local gyms     Low     In-person │   │    │
  │                        │  └──────────────────────────────────┘   │    │
  │                        └──────────────────────────────────────────┘    │
  │                                                                         │
  │                        ┌──────────────────────────────────────────┐    │
  │                        │  PCG OPPORTUNITY ASSESSMENT              │    │
  │                        │  ─────────────────────────────────────   │    │
  │                        │                                          │    │
  │                        │  Overall Score:  ████████░░  8.5/10      │    │
  │                        │                                          │    │
  │                        │  Pain Points Identified:                 │    │
  │                        │  ● No cohesive brand system              │    │
  │                        │  ● Manual content creation bottleneck    │    │
  │                        │  ● No high-converting VSL                │    │
  │                        │  ● Sales funnel is direct DM only        │    │
  │                        │  ● No automated lead nurture             │    │
  │                        │                                          │    │
  │                        │  Budget Signals:                         │    │
  │                        │  Sells $197–497/mo programs              │    │
  │                        │  ~800 active members → ~$15K/mo ARR      │    │
  │                        │  Previously invested in Passion.io app   │    │
  │                        │                                          │    │
  │                        │  Engagement Style:                       │    │
  │                        │  Direct, values authenticity             │    │
  │                        │  Dislikes corporate/polished feel        │    │
  │                        │  Responds to before/after ROI framing    │    │
  │                        │                                          │    │
  │                        │  Recommended Approach:                   │    │
  │                        │  Lead with brand identity ROI, show      │    │
  │                        │  transformation case studies, reference  │    │
  │                        │  her existing aesthetic as a strength    │    │
  │                        └──────────────────────────────────────────┘    │
  │                                                                         │
  │                        ┌──────────────────────────────────────────┐    │
  │                        │  RESEARCH LOG                            │    │
  │                        │  ─────────────────────────────────────   │    │
  │                        │                                          │    │
  │                        │  Pass 3 · deep_strategy · 84% conf ▼    │    │
  │                        │  ─────────────────────────────────────   │    │
  │                        │  Summary: Discovered active Passion.io   │    │
  │                        │  presence with 800+ members. VSL + sales │    │
  │                        │  funnel identified as primary gap...     │    │
  │                        │  Sources: [badgirlstrength.club] [IG]    │    │
  │                        │  Key findings: [3 chips]                 │    │
  │                        │  Confidence delta: +12%                  │    │
  │                        │                                          │    │
  │                        │  Pass 2 · market_position · +18% ▼      │    │
  │                        │  Pass 1 · identity · +54% ▼             │    │
  │                        │                                          │    │
  │                        │  [▶ Run Pass 4: Audience Research]       │    │
  │                        └──────────────────────────────────────────┘    │
  └─────────────────────────────────────────────────────────────────────────┘


═══════════════════════════════════════════════════════════════
SECTION 5: DATA FLOW — ENHANCED SCOUT PIPELINE
═══════════════════════════════════════════════════════════════

  TRIGGER:
  User clicks [Run Scout] on Person Profile  OR  deal enters Intel stage
       │
       ▼
  POST /api/persons/:id/research-passes/next
       │
       ▼
  ┌──────────────────────────────────────────────────────────────────┐
  │  intelligence.rs: trigger_next_research_pass()                   │
  │                                                                  │
  │  1. Get next pass_number                                         │
  │  2. Assign focus:                                                │
  │     Pass 1 → "identity_social"    (NEW combined focus)          │
  │     Pass 2 → "business_profile"   (NEW: affiliations + revenue) │
  │     Pass 3 → "media_content"      (NEW: appearances + strategy) │
  │     Pass 4 → "audience_market"    (enhanced)                    │
  │     Pass 5 → "pcg_opportunity"    (NEW: pain points + approach) │
  │  3. Build prompt (see below)                                     │
  │  4. Spawn run_research_pass() async                              │
  └──────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌──────────────────────────────────────────────────────────────────┐
  │  run_research_pass() — enhanced prompt structure                 │
  │                                                                  │
  │  System: "You are Scout, a professional intelligence analyst     │
  │   for a creative agency. Research [name] thoroughly.            │
  │   Use web_search to find REAL data. Return structured JSON."    │
  │                                                                  │
  │  Pass 1 prompt asks for:                                         │
  │  - Full name, confirmed location                                 │
  │  - Best available photo URL (LinkedIn, IG, website)             │
  │  - ALL social media handles with follower counts                 │
  │  - Website(s), Linktree, booking pages                          │
  │  - Short bio / personal brand statement                          │
  │  - Primary business name and role                               │
  │                                                                  │
  │  Pass 2 prompt asks for:                                         │
  │  - All business affiliations (current + past)                   │
  │  - Revenue estimates with basis                                  │
  │  - Pricing model / program tiers                                │
  │  - Team size / organizational structure                          │
  │  - Platform stack (website, app, tools used)                    │
  │                                                                  │
  │  Pass 3 prompt asks for:                                         │
  │  - Podcast appearances (as guest or host)                       │
  │  - Press mentions / news articles                               │
  │  - YouTube features / speaking events                           │
  │  - Content themes and brand voice analysis                      │
  │  - Posting cadence per platform                                 │
  │  - Best performing content types                                │
  │                                                                  │
  │  Pass 4 prompt asks for:                                         │
  │  - Target audience demographics                                 │
  │  - Competitor names + positioning                               │
  │  - Market category and industry standing                        │
  │  - Audience engagement signals                                  │
  │                                                                  │
  │  Pass 5 prompt asks for:                                         │
  │  - Pain points mappable to PCG services                         │
  │  - Budget signals from pricing and public data                  │
  │  - Decision-making style and communication preferences          │
  │  - Recommended PCG engagement strategy                          │
  │  - Overall opportunity score (0-10) with rationale             │
  └──────────────────────────────────────────────────────────────────┘
       │
       ▼
  ┌──────────────────────────────────────────────────────────────────┐
  │  JSON RESPONSE PARSED INTO:                                      │
  │                                                                  │
  │  persons.intelligence_raw (JSON):                                │
  │  ├── photo_url                                                   │
  │  ├── location                                                    │
  │  ├── social_profiles[]                                           │
  │  ├── business_affiliations[]                                     │
  │  ├── media_appearances[]                                         │
  │  ├── content_strategy{}                                          │
  │  ├── pcg_opportunity{}                                           │
  │  ├── background                                                  │
  │  ├── positioning                                                 │
  │  ├── specialization                                              │
  │  ├── expertise[]                                                 │
  │  ├── target_clients[]                                            │
  │  ├── competitors[]                                               │
  │  ├── strategy                                                    │
  │  ├── communication_style                                         │
  │  ├── opportunities[]                                             │
  │  └── risks[]                                                     │
  │                                                                  │
  │  persons.intelligence_summary  (cumulative narrative)            │
  │  persons.intelligence_confidence  (0-1 float)                    │
  │  person_research_passes row  (pass log)                          │
  │  person_social_profiles rows  (upsert per platform)             │
  └──────────────────────────────────────────────────────────────────┘


═══════════════════════════════════════════════════════════════
SECTION 6: IMPLEMENTATION PHASES
═══════════════════════════════════════════════════════════════

  PHASE A — Backend: Enhanced Scout Prompts (intelligence.rs)
  ─────────────────────────────────────────────────────────────
  A1. Rename/restructure pass focuses:
      identity → identity_social
      market_position → business_profile
      competitors → media_content   (new)
      target_clients → audience_market
      deep_strategy → pcg_opportunity

  A2. Rewrite prompt for each pass focus with explicit
      structured output schema (JSON with all new fields above)

  A3. Parse new fields from response:
      - Extract social_profiles array → upsert person_social_profiles
      - Extract business_affiliations → upsert PersonCompanyRole
      - Extract photo_url → store in intelligence_raw.photo_url
      - Extract media_appearances → store in intelligence_raw
      - Extract pcg_opportunity → store in intelligence_raw
      - Extract content_strategy → store in intelligence_raw

  PHASE B — DB: person_social_profiles Enhancement
  ─────────────────────────────────────────────────────────────
  B1. Migration: add follower_count, following_count, bio,
      verified, engagement_rate, post_count, content_themes,
      avg_views, last_synced_at to person_social_profiles

  B2. Check PersonCompanyRole has description, company_url fields
      Add if missing

  PHASE C — Frontend: Person Profile Page Redesign
  ─────────────────────────────────────────────────────────────
  C1. Hero section with photo (from intelligence_raw.photo_url,
      fallback to clearbit/initials avatar)

  C2. Social platforms row with platform icons, handles,
      follower counts, engagement rates, links

  C3. Business affiliations card with current/past,
      description, revenue estimates

  C4. PCG Opportunity card (score bar + services + pain points)

  C5. Background narrative (full text, expandable)

  C6. Content strategy card

  C7. Media appearances list

  PHASE D — Frontend: Intel Wiki Page Redesign
  ─────────────────────────────────────────────────────────────
  D1. Left sidebar navigation (anchor links to sections)

  D2. Social presence section with platform cards

  D3. Business profile section with affiliation cards

  D4. Market position with competitors table

  D5. PCG Opportunity assessment section

  D6. Research log as collapsible pass history
      with sources cited, confidence delta

  D7. Business reports section (already exists, keep + enhance)


═══════════════════════════════════════════════════════════════
SECTION 7: WORK ORDER
═══════════════════════════════════════════════════════════════

  1. [ ] Phase A — intelligence.rs: rewrite pass prompts
                   add social_profiles + photo_url parsing
                   add pcg_opportunity + content_strategy parsing
                   upsert social_profiles table on pass complete

  2. [ ] Phase B — DB migration: person_social_profiles fields
                   person_company_roles: add description + url fields

  3. [ ] Phase C — person-profile.tsx: full redesign
                   Hero with photo + socials
                   Business affiliations + PCG opportunity cards
                   Background narrative + content strategy

  4. [ ] Phase D — person-intel.tsx: wiki redesign
                   Sidebar nav
                   Social presence section
                   Business profile section
                   PCG opportunity section
                   Research log (enhanced)

  START: Phase A (backend prompt rewrite) — this is the data source
         for everything else. Without richer data, the UI
         improvements are empty shells.
