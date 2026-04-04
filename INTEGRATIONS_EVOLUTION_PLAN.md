# Integrations Evolution Plan — sloperation327-integrations
_Created: 2026-03-27_

---

## Overview

This document maps the full evolution of integrations across **Individuals**, **AI Agents**, and **Organizations** — with special focus on **social integrations** as both inbound data sources (intelligence) and outbound distribution channels (publishing).

Three phases:
- **Phase 1 (Current)** — Project-scoped social accounts, flat publishing, Nora email/inbox
- **Phase 2 (Near-term)** — Polymorphic ownership, agent-native integrations, social intelligence loop
- **Phase 3 (Vision)** — Full mesh: every entity owns its integrations, cross-entity syndication, AI-driven content lifecycle

---

## Current State Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                        PHASE 1 — CURRENT STATE                                  │
└─────────────────────────────────────────────────────────────────────────────────┘

  INTEGRATIONS TODAY
  ══════════════════

  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  OWNER: projects (single owner type)                                        │
  │                                                                             │
  │  social_accounts                    email_accounts                          │
  │  ─────────────                      ──────────────                          │
  │  project_id ──► Project             owner_type: project|agent|user|org  ◄── POLYMORPHIC (partial)
  │  platform: twitter|linkedin|ig|tiktok|threads                              │
  │  access_token (plaintext)           smtp_host, imap_host                   │
  │  refresh_token (plaintext)          credentials (plaintext)                │
  │  status: active|inactive                                                    │
  └────────────────────────────┬────────────────────────────────────────────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
              ▼                ▼                ▼
     social_posts        social_mentions    email_messages
     ──────────────       ──────────────    ──────────────
     project_id           project_id        account_id
     platform             platform          from/to/subject
     content              content           body
     status:              mention_type      is_read
       draft|scheduled|   sentiment         created_at
       published|failed   author_handle
     published_at         engagement_metrics
     metrics JSON
       (likes/retweets/
        impressions)

              │
              ▼
     services/social/
     ───────────────
     publisher.rs  ──► Twitter API v2
                   ──► LinkedIn API
                   ──► Instagram Graph API
                   ──► TikTok API
                   ──► Threads API
                   ──► Facebook [STUBBED]
                   ──► YouTube [STUBBED]
     scheduler.rs  ──► background worker (polls published_at)
     analytics.rs  ──► metrics sync


  INBOUND INTEGRATIONS (data sources)
  ════════════════════════════════════

  email (Zoho IMAP)          airtable          fireflies.ai        google docs
  ─────────────────          ────────          ────────────        ──────────────
  nora@powerclubglobal.com   airtable_sources  [source links       [source links
  ──► NoraInboxPoller        airtable_syncs     extracted from       extracted from
  ──► call-intake pipeline   data_source        email body, no       email body, no
  ──► person/company extract  _workflows         native OAuth]        native OAuth]
  ──► crm_deal creation                        ──► intake item      ──► intake item
                                                  metadata.            metadata.
                                                  source_links[]       source_links[]


  INTEL PIPELINE (no social listening yet)
  ════════════════════════════════════════

  call_intake_items
    │  auto_pipeline=true
    │  source_links: [{type, url}]
    │
    ├──► Scout (web search — anthropic beta tool)
    │      ──► person_research_passes
    │      ──► company.intelligence_status = done
    │
    └──► Astra (Claude deep research + Claude report generation)
           ──► business_reports
           ──► tasks (Review: {company} Business Report)


  GAPS (Phase 1 → Phase 2)
  ════════════════════════
  ✗  social_accounts: project-scoped only (no user/agent/org owner)
  ✗  no social listening (mentions, competitor signals, audience data)
  ✗  no Facebook / YouTube publishing
  ✗  no Fireflies native OAuth (only URL extraction from email)
  ✗  no Google Docs OAuth (only URL extraction from email)
  ✗  no Dropbox / Drive file sync
  ✗  tokens stored plaintext
  ✗  no integration registry / plugin system
  ✗  no cross-entity content syndication
  ✗  no A/B testing (schema exists, logic missing)
```

---

## Ownership Evolution

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    POLYMORPHIC OWNERSHIP EVOLUTION                               │
└─────────────────────────────────────────────────────────────────────────────────┘

  PHASE 1 — Project-Scoped (current)
  ────────────────────────────────────────────────────────────────────────────────

    projects ──► social_accounts (project_id FK)
    projects ──► email_accounts  (owner_type='project', owner_id)
               ┗ partial polymorphism on email, none on social

  PHASE 2 — Polymorphic Social Accounts
  ────────────────────────────────────────────────────────────────────────────────

    Migrate social_accounts to match email_accounts pattern:

    ┌─────────────────────────────────────────────────────────────┐
    │  social_accounts                                            │
    │  ─────────────────────────────────────────────────────────  │
    │  owner_type  TEXT  ──► 'project' | 'user' | 'agent' | 'org'│
    │  owner_id    TEXT  ──► UUID of the owning entity            │
    │  platform    TEXT  ──► twitter | linkedin | instagram | ... │
    │  handle      TEXT  ──► @username / page slug                │
    │  access_token_enc  ──► encrypted (KMS or env-key AES-256)  │
    │  refresh_token_enc ──► encrypted                            │
    │  scopes      TEXT  ──► JSON array of OAuth scopes           │
    │  status      TEXT  ──► active | inactive | revoked          │
    │  follower_count INT                                         │
    │  bio         TEXT                                           │
    └─────────────────────────────────────────────────────────────┘

    Ownership map:
    ┌──────────────┐   owner_type='user'    ┌───────────────────┐
    │   users      │ ──────────────────────►│  social_accounts  │
    │  (Sirak,     │                        │  (personal X,     │
    │   Nora,etc.) │                        │   LinkedIn)       │
    └──────────────┘                        └───────────────────┘
    ┌──────────────┐   owner_type='agent'   ┌───────────────────┐
    │   agents     │ ──────────────────────►│  social_accounts  │
    │  (Nora,Scout │                        │  (Nora's Twitter  │
    │   Astra,Topsi│                        │   handle)         │
    └──────────────┘                        └───────────────────┘
    ┌──────────────┐   owner_type='org'     ┌───────────────────┐
    │organizations │ ──────────────────────►│  social_accounts  │
    │  (PCG,Sirak  │                        │  (brand IG/TW/LI) │
    │   Studios)   │                        └───────────────────┘
    └──────────────┘
    ┌──────────────┐   owner_type='project' ┌───────────────────┐
    │   projects   │ ──────────────────────►│  social_accounts  │
    │  (campaign,  │                        │  (campaign-level  │
    │   content)   │                        │   accounts)       │
    └──────────────┘                        └───────────────────┘

  PHASE 3 — Entity Integration Hub
  ────────────────────────────────────────────────────────────────────────────────

    Each entity type gets a unified integration hub:

    ┌───────────────────────────────────────────────────────────────────────────┐
    │  integration_connections  (new table — replaces fragmented tables)        │
    │  ─────────────────────────────────────────────────────────────────────── │
    │  owner_type    TEXT  ──► user|agent|org|project|person|company            │
    │  owner_id      TEXT  ──► UUID                                             │
    │  provider      TEXT  ──► twitter|linkedin|ig|tiktok|threads|fb|yt|        │
    │                           fireflies|gdocs|notion|airtable|dropbox|        │
    │                           quickbooks|zoho|gmail|slack|discord             │
    │  credential_enc TEXT ──► encrypted JSON (access/refresh/api_key)         │
    │  scopes        TEXT ──► JSON array                                        │
    │  status        TEXT ──► active|inactive|revoked|error                     │
    │  last_synced_at                                                           │
    │  capabilities  TEXT ──► JSON: ["read","write","listen","publish"]         │
    └───────────────────────────────────────────────────────────────────────────┘
```

---

## Social as Data Source (Intelligence Loop)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    SOCIAL → INTELLIGENCE FLOW                                    │
└─────────────────────────────────────────────────────────────────────────────────┘

  PHASE 2 — Social Listening + Person/Company Intel
  ══════════════════════════════════════════════════

                           INBOUND SIGNALS
                           ───────────────

  Twitter/X ──────────────────────────────────────────────────────────────────────
    mentions of @PCG / brand      ──► social_mentions table
    engagement on published posts ──► social_posts.metrics (updated)
    competitor content            ──► [audience_signals table — Phase 3]
    target person's public tweets ──► Scout pass: identity_social

  LinkedIn ────────────────────────────────────────────────────────────────────────
    profile views (org page)      ──► org analytics
    post engagement               ──► social_posts.metrics
    target person's activity      ──► Scout pass: business_profile
    company page following        ──► company intel signals

  Instagram / TikTok ─────────────────────────────────────────────────────────────
    post reach/saves/shares       ──► social_posts.metrics
    follower demographics         ──► audience_intelligence table (Phase 3)
    target creator metrics        ──► Scout pass: media_content, audience_market

  Fireflies.ai ────────────────────────────────────────────────────────────────────
    transcript URLs (from email)  ──► call_intake_items.metadata.source_links
    [Phase 2: native OAuth]       ──► fireflies_transcripts table
    speaker identification        ──► persons table (name + company match)
    action items extracted        ──► tasks table

  Google Docs ─────────────────────────────────────────────────────────────────────
    doc URLs (from email)         ──► call_intake_items.metadata.source_links
    [Phase 2: native OAuth]       ──► gdocs_content table
    meeting notes                 ──► Phase II (Astra) call_context


  SCOUT INTELLIGENCE PASSES (enhanced Phase 2)
  ═════════════════════════════════════════════

  Person research now collects:

  pass 1: identity_social
  ┌──────────────────────────────────────────────────────────────────────────┐
  │  Query: full name + company → social handles across all platforms        │
  │  Stores: person_social_profiles []                                       │
  │    { platform, handle, url, follower_count, engagement_rate, bio }       │
  │  Extracts: verified accounts, primary content themes, posting cadence    │
  └──────────────────────────────────────────────────────────────────────────┘

  pass 2: business_profile
  ┌──────────────────────────────────────────────────────────────────────────┐
  │  Query: professional background, company affiliations, funding news      │
  │  Stores: business_affiliations [], funding_context                       │
  └──────────────────────────────────────────────────────────────────────────┘

  pass 3: media_content
  ┌──────────────────────────────────────────────────────────────────────────┐
  │  Query: podcasts, interviews, press mentions, YouTube appearances        │
  │  Stores: media_appearances [] { source, title, url, date, summary }     │
  └──────────────────────────────────────────────────────────────────────────┘

  pass 4: audience_market
  ┌──────────────────────────────────────────────────────────────────────────┐
  │  Query: audience size signals, target market, brand positioning          │
  │  Stores: content_strategy, audience_description                          │
  └──────────────────────────────────────────────────────────────────────────┘

  pass 5: pcg_opportunity
  ┌──────────────────────────────────────────────────────────────────────────┐
  │  Query: brand needs, recent launches, gaps, PCG service alignment        │
  │  Stores: pcg_opportunity { score, rationale, recommended_services [] }   │
  └──────────────────────────────────────────────────────────────────────────┘

  Company intel similarly extended:
  ┌──────────────────────────────────────────────────────────────────────────┐
  │  social_handles [] (all platforms)                                       │
  │  follower_counts JSON                                                    │
  │  content_themes []                                                       │
  │  recent_campaigns []                                                     │
  │  competitor_mentions []                                                  │
  └──────────────────────────────────────────────────────────────────────────┘


  PHASE 3 — Real-Time Social Intelligence
  ════════════════════════════════════════

  ┌─────────────────────────────────────────────────────────────────────────────┐
  │                    SOCIAL INTELLIGENCE MESH                                  │
  │                                                                             │
  │  audience_signals                    competitor_signals                     │
  │  ────────────────                    ──────────────────                     │
  │  company_id ──► companies            company_id                            │
  │  platform                            competitor_company_id                 │
  │  signal_type:                        platform                              │
  │    follower_growth                   content_type                          │
  │    engagement_rate                   engagement                            │
  │    top_content_themes                detected_campaign                     │
  │    audience_demographics             signal_date                           │
  │  recorded_at                                                               │
  │                                                                            │
  │  mention_alerts                                                            │
  │  ────────────────                                                          │
  │  entity_type: company|person|org                                           │
  │  entity_id                                                                 │
  │  platform                                                                  │
  │  content                                                                   │
  │  author_handle                                                             │
  │  sentiment: positive|neutral|negative                                      │
  │  reach_estimate                                                            │
  │  routed_to_task: bool ──► auto-create Nora task for high-reach mentions   │
  └─────────────────────────────────────────────────────────────────────────────┘
```

---

## Social as Distribution (Content Pipeline)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    CONTENT DISTRIBUTION PIPELINE                                 │
└─────────────────────────────────────────────────────────────────────────────────┘

  PHASE 1 — Manual Draft → Publish (current)
  ═══════════════════════════════════════════

  User creates post → social_posts (draft)
    │  content: TEXT
    │  platform: twitter|linkedin|...
    │  scheduled_for: datetime
    │  project_id
    │
    ▼
  scheduler.rs polls every N min
    │  WHERE status='scheduled' AND scheduled_for <= now()
    │
    ▼
  publisher.rs → platform API
    │  POST tweet / LinkedIn share / IG media / TikTok video
    │
    ▼
  social_posts.status = 'published'
  social_posts.published_at = now()
  social_posts.metrics = { impressions: 0, likes: 0, ... }  (populated on sync)


  PHASE 2 — AI-Assisted Multi-Platform Content Pipeline
  ══════════════════════════════════════════════════════

                        content_source
                        ──────────────
  business_report ──────────────────────────────────────────────────► content_brief
  deliverable (done) ───────────────────────────────────────────────► content_brief
  meeting_outcome (intake) ─────────────────────────────────────────► content_brief
  manual prompt ────────────────────────────────────────────────────► content_brief

                              │
                              ▼
                    ┌─────────────────────┐
                    │   TOPSI / EDITRON   │  ◄── AI content generation agent
                    │   content drafting  │      adapts to platform voice/format
                    └────────────┬────────┘
                                 │
                    ┌────────────┴────────────────────────────────┐
                    │             platform variants               │
                    │                                             │
                    ▼            ▼            ▼         ▼        ▼
                Twitter/X    LinkedIn     Instagram   TikTok   Threads
                ─────────    ────────     ─────────   ──────   ───────
                280 chars    long-form    visual      video    casual
                thread       thought      carousel    script   short
                hooks        leadership   caption     hook     relay
                             article      hashtags    caption  from IG

                    │
                    ▼
              content_calendar
              ────────────────
              org_id / project_id
              scheduled_for
              platforms: [] (multi-platform batch)
              status: draft|approved|queued|published
              approval_required: bool
              approved_by: user_id
              ab_variant_group: uuid  ──► A/B test grouping (schema exists)

                    │
                    ▼ (on approval / auto if trusted)
              publisher.rs (enhanced)
              ───────────────────────
              fan-out to each platform in platforms[]
              respect rate limits per platform
              retry with exponential backoff
              update social_posts.status per platform
              aggregate cross-platform metrics → content_calendar.metrics


  PHASE 3 — Autonomous Content Lifecycle
  ═══════════════════════════════════════

  ┌────────────────────────────────────────────────────────────────────────────┐
  │                     CONTENT LIFECYCLE (agent-driven)                       │
  │                                                                            │
  │  Trigger signals ──────────────────────────────────────────────────────   │
  │  ─────────────────                                                         │
  │  • New business_report ready         ──► auto-draft thought leadership    │
  │  • Deliverable approved by client    ──► auto-draft case study teaser     │
  │  • Intake lead from high-profile co. ──► auto-draft discovery post        │
  │  • High-engagement mention detected  ──► auto-draft reply / amplification │
  │  • Competitor campaign detected      ──► alert + optional counter-post    │
  │                                                                            │
  │  Nora orchestrates:                                                        │
  │  ──────────────────                                                        │
  │  Nora.content_pipeline_agent                                               │
  │    ├── reads: business_reports, deliverables, social_mentions              │
  │    ├── calls: Editron for copy, Topsi for brand-voice check               │
  │    ├── creates: social_posts (draft) with ab_variant_group set            │
  │    ├── routes: to operator for approval (task in Review column)           │
  │    └── on approval: schedules via content_calendar                        │
  │                                                                            │
  │  Performance loop:                                                         │
  │  ─────────────────                                                         │
  │  published → metrics synced (24h, 7d, 28d windows)                        │
  │           → Scout analyzes: what performed, why                            │
  │           → Topsi updates: brand voice refinement model                   │
  │           → next drafts: informed by performance data                     │
  └────────────────────────────────────────────────────────────────────────────┘
```

---

## AI Agent Integration Flows

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    AGENT INTEGRATION ARCHITECTURE                                │
└─────────────────────────────────────────────────────────────────────────────────┘

  NORA — Executive Assistant
  ═════════════════════════════════════════════════════════════════════════════

  INBOUND channels Nora consumes:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  email (Zoho IMAP)  ──► NoraInboxPoller ──► call-intake pipeline          │
  │  phone (Twilio)     ──► /api/twilio/voice ──► real-time conversation      │
  │  Discord (JS bot)   ──► pcg-discord-bot.service ──► Nora agent            │
  │  web chat           ──► /api/nora/chat ──► Nora agent                     │
  │  [Phase 2] Slack    ──► SlackEventConsumer ──► Nora agent                 │
  │  [Phase 2] Twitter DM ──► TwitterDMPoller ──► Nora agent                  │
  └────────────────────────────────────────────────────────────────────────────┘

  OUTBOUND channels Nora uses:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  email reply        ──► SMTP (Zoho)                                        │
  │  phone reply        ──► Twilio TwiML TTS                                  │
  │  task creation      ──► tasks table ──► kanban board                      │
  │  CRM updates        ──► crm_deals, persons, companies                     │
  │  [Phase 2] Slack DM ──► Slack API (notify deal owner)                     │
  │  [Phase 2] calendar ──► Google Calendar (schedule discovery calls)        │
  └────────────────────────────────────────────────────────────────────────────┘

  SCOUT — Web Research Agent
  ═════════════════════════════════════════════════════════════════════════════

  INBOUND data sources Scout uses:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  web search (anthropic beta tool)  ──► public web                         │
  │  call_intake_items.raw_content     ──► meeting notes, transcripts         │
  │  call_intake_items.source_links    ──► Fireflies URLs, Google Doc URLs    │
  │  persons table                     ──► existing profile data              │
  │  companies table                   ──► existing intel                     │
  │  [Phase 2] Fireflies OAuth         ──► full transcripts natively          │
  │  [Phase 2] Google Docs OAuth       ──► doc content natively               │
  │  [Phase 2] LinkedIn (read scopes)  ──► company/person verified data       │
  └────────────────────────────────────────────────────────────────────────────┘

  OUTBOUND artifacts Scout produces:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  person_research_passes            ──► persons profile fields updated     │
  │  company intel fields              ──► companies.intelligence_*           │
  │  person_social_profiles [Phase 2]  ──► social handles + metrics           │
  │  company_social_presence [Phase 2] ──► brand social footprint             │
  └────────────────────────────────────────────────────────────────────────────┘

  ASTRA — Deep Research / Report Generation
  ═════════════════════════════════════════════════════════════════════════════

  INBOUND data sources Astra uses:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  Scout output (company intel)      ──► base layer for deep dive           │
  │  call_intake_items (raw_content)   ──► discovery call meeting notes       │
  │  business_reports (prior)          ──► avoid duplication                  │
  │  [Phase 2] social_posts analytics  ──► brand content performance          │
  │  [Phase 2] audience_signals        ──► market positioning context         │
  └────────────────────────────────────────────────────────────────────────────┘

  OUTBOUND artifacts:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  business_reports                  ──► PDF-ready structured report        │
  │  tasks (Review: {co} Business Report) ──► assigned to deal owner         │
  │  [Phase 2] proposal drafts         ──► Cash AI → proposal.content        │
  │  [Phase 2] content briefs          ──► Editron → social post drafts      │
  └────────────────────────────────────────────────────────────────────────────┘

  TOPSI — Brand / Platform Agent
  ═════════════════════════════════════════════════════════════════════════════

  INBOUND:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  web chat (public-facing)          ──► /api/topsi/chat                    │
  │  VIBE payment events               ──► vibe_transactions                  │
  │  [Phase 2] social_mentions         ──► brand monitoring feed              │
  │  [Phase 2] audience_signals        ──► platform performance feed          │
  └────────────────────────────────────────────────────────────────────────────┘

  OUTBOUND:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  chat responses                    ──► web visitors                       │
  │  brand voice guidance              ──► Editron drafts                     │
  │  [Phase 2] social post approval    ──► content calendar queue             │
  │  [Phase 2] brand alert tasks       ──► Nora → deal owner                 │
  └────────────────────────────────────────────────────────────────────────────┘

  EDITRON — Content / Copy Agent  [Phase 2]
  ═════════════════════════════════════════════════════════════════════════════

  INBOUND:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  business_reports                  ──► source material for content        │
  │  brand_profiles                    ──► voice + tone constraints           │
  │  social_posts.metrics (past)       ──► performance-informed writing       │
  │  content_briefs                    ──► Astra/Nora-generated briefs        │
  └────────────────────────────────────────────────────────────────────────────┘

  OUTBOUND:
  ┌────────────────────────────────────────────────────────────────────────────┐
  │  social_posts (draft)              ──► per-platform variants              │
  │  proposal.content sections         ──► Cash AI polish pass               │
  │  deliverable copy drafts           ──► projects workflow                  │
  └────────────────────────────────────────────────────────────────────────────┘
```

---

## Organization Integration Hub

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    ORGANIZATION INTEGRATION HUB                                  │
└─────────────────────────────────────────────────────────────────────────────────┘

  Per-organization integration surface (Phase 2 target):

  organizations
    │  id
    │  domain ──► used for OAuth callback routing
    │  company_id ──► companies (brand entity)
    │
    ├──► email_accounts (owner_type='org', owner_id=org.id)
    │      nora@powerclubglobal.com (inbound intake)
    │      hello@sirakstudios.com   (client org)
    │
    ├──► social_accounts (owner_type='org', owner_id=org.id)  [Phase 2]
    │      @PCG_official      (Twitter/X)
    │      PCG                (LinkedIn Company Page)
    │      @pcgglobal         (Instagram)
    │      @pcgglobal         (TikTok)
    │
    ├──► integration_connections [Phase 3]
    │      Airtable           ──► project_data_sources
    │      QuickBooks         ──► invoices sync (stub)
    │      Zoho CRM           ──► contacts sync (stub)
    │      Slack workspace    ──► team notifications
    │      Google Workspace   ──► calendar + docs OAuth
    │
    └──► brand_profiles
           voice_guidelines
           logo_url
           color_palette
           approved_hashtags []


  Cross-org publishing (Phase 2 — client org integration):
  ═════════════════════════════════════════════════════════

  Sirak Studios org
    │  social_accounts (owner_type='org', owner_id=sirak_org_id)
    │    @sirakmedia (Instagram)
    │    @sirakmedia (TikTok)
    │    linkedin.com/company/sirak-studios
    │
    └──► CONTENT DELIVERY FLOW:
         PCG creates deliverable ──► deliverable.status = 'done'
         ──► client review portal ──► client approves
         ──► approved content pushed to client's social_accounts
         ──► publisher.rs uses client org's tokens (not PCG's)
         ──► metrics reported back to PCG project dashboard


  OAuth flow per org (Phase 2):
  ═══════════════════════════════

  ┌─────────────────────────────────────────────────────────────────────────────┐
  │  GET /api/integrations/oauth/start?provider=twitter&org_id=...              │
  │                         │                                                   │
  │                         ▼                                                   │
  │  redirect → Twitter OAuth 2.0 PKCE                                         │
  │                         │                                                   │
  │                         ▼ callback                                          │
  │  GET /api/integrations/oauth/callback?provider=twitter&code=...&state=...   │
  │                         │                                                   │
  │                         ▼                                                   │
  │  exchange code → access_token + refresh_token                               │
  │  encrypt tokens → store in social_accounts / integration_connections        │
  │  set status = 'active'                                                      │
  │                         │                                                   │
  │                         ▼                                                   │
  │  POST /api/integrations/oauth/complete  (redirect to settings page)         │
  └─────────────────────────────────────────────────────────────────────────────┘
```

---

## Phase Roadmap

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                         INTEGRATION PHASE ROADMAP                                │
└─────────────────────────────────────────────────────────────────────────────────┘

  PHASE 1 — FOUNDATION (current / sloperation327)
  ─────────────────────────────────────────────────────────────────────────────────

  ✅  Nora inbox (Zoho IMAP) → intake pipeline → CRM deals
  ✅  Source link extraction (Fireflies + Google Doc URLs from email)
  ✅  Scout web research → person/company intel
  ✅  Astra report with call context (meeting notes pulled from intake)
  ✅  Twitter / LinkedIn / Instagram / TikTok / Threads publishing
  ✅  social_posts + scheduler + publisher services
  ✅  social_mentions CRUD
  ✅  Airtable data source workflows (60% complete)
  ✅  email_accounts polymorphic (project/agent/user/org)
  ✅  brand_profiles + brand research (PCG KG + Exa multi-pass)

  IN PROGRESS (sloperation327-integrations)
  ─────────────────────────────────────────────────────────────────────────────────

  🔧  Track A — Prospect client on intake (pipeline gaps)
  🔧  Track B — Sirak auto-pipeline (3-phase Scout→Astra→Review)
  🔧  Person profile redesign (photo, social rail, PCG opportunity card)
  🔧  Company intel page redesign (SWOT grid, key people, research history)

  PHASE 2 — POLYMORPHIC SOCIAL + NATIVE OAUTH (next sprint)
  ─────────────────────────────────────────────────────────────────────────────────

  Priority   Task
  ────────── ─────────────────────────────────────────────────────────────────────
  HIGH       Migrate social_accounts → polymorphic (owner_type + owner_id)
             Migration: add owner_type TEXT DEFAULT 'project', owner_id TEXT
             Backfill: UPDATE social_accounts SET owner_type='project', owner_id=project_id
             Add: scopes TEXT, follower_count INTEGER, bio TEXT, handle TEXT

  HIGH       OAuth flow for social platforms
             Routes: GET /api/integrations/oauth/start, GET /api/integrations/oauth/callback
             Providers: Twitter PKCE, LinkedIn OAuth 2.0, Instagram Graph, TikTok
             Token encryption: AES-256-GCM with INTEGRATION_SECRET env key

  HIGH       Enhanced Scout passes (5 focuses: identity_social, business_profile,
             media_content, audience_market, pcg_opportunity)
             New table: person_social_profiles (handle, platform, follower_count, bio)
             New fields on persons: photo_url, social_profiles JSON, pcg_opportunity JSON

  MEDIUM     Fireflies native OAuth
             OAuth 2.0 → access transcript list → fetch full transcript
             Table: fireflies_transcripts (intake_item_id, transcript_id, content, speakers)
             Match speakers → persons table

  MEDIUM     Google Docs native OAuth
             OAuth 2.0 → read doc content → feed into intake pipeline
             Table: gdoc_sources (doc_id, doc_title, content, last_synced_at)

  MEDIUM     Facebook + YouTube publishing
             Extend publisher.rs with Graph API (Facebook) + YouTube Data API v3
             Add platforms to social_accounts enum

  LOW        Social analytics dashboard
             Route: GET /api/social/analytics?period=7d|30d
             Aggregate: social_posts.metrics across platforms per org/project
             Display: reach, engagement_rate, top posts, platform comparison

  PHASE 3 — FULL MESH (Q3 2026 vision)
  ─────────────────────────────────────────────────────────────────────────────────

  Priority   Task
  ────────── ─────────────────────────────────────────────────────────────────────
  HIGH       Integration connections registry (unified table replacing fragmented)
             Providers: Slack, Discord webhooks, Notion, Dropbox, Drive, QuickBooks
             Webhook manager: ngrok/Cloudflare webhook receivers per org

  HIGH       Social listening engine
             mention_alerts table + real-time Twitter filtered stream
             Sentiment analysis on mentions (Claude or local model)
             Auto-route high-reach mentions → Nora task

  HIGH       Audience intelligence
             audience_signals table (follower_growth, engagement_rate, demographics)
             Competitor signals table (rival campaigns, content gaps)
             Scout enriches company intel with live audience data

  HIGH       Content lifecycle agent (Editron)
             Trigger: business_report → content brief → platform variants
             A/B testing: ab_variant_group on social_posts (schema exists)
             Performance loop: metrics → Scout analysis → next draft improvement

  MEDIUM     Cross-org content delivery
             Client org social_accounts (Sirak's handles)
             Approved deliverable → publish to client's channels using client tokens
             Metrics reported back to PCG project dashboard

  MEDIUM     Calendar integrations
             Google Calendar OAuth → Nora schedules discovery calls
             Outlook / iCal feed → project timeline sync

  LOW        Bluesky / Pinterest (emerging platforms)
             AT Protocol for Bluesky
             Pinterest Ads API

  LOW        A/B test analytics loop
             ab_variant_group → compare engagement per variant
             Topsi updates brand voice model based on winning variants
```

---

## Implementation Priority (Next Sprint)

| # | Task | File(s) | Phase |
|---|------|---------|-------|
| 1 | social_accounts migration → polymorphic | `crates/db/migrations/20260428100000_social_accounts_polymorphic.sql` | 2 |
| 2 | OAuth flow routes (start + callback) | `crates/server/src/routes/integrations_oauth.rs` | 2 |
| 3 | Token encryption util | `crates/utils/src/token_encryption.rs` | 2 |
| 4 | Enhanced Scout passes (5 focuses) | `crates/server/src/routes/intelligence.rs` | 2 |
| 5 | person_social_profiles table + model | migration + `crates/db/src/models/person_social_profile.rs` | 2 |
| 6 | Fireflies OAuth + transcript fetch | `crates/server/src/routes/integrations_fireflies.rs` | 2 |
| 7 | Facebook + YouTube publisher | `crates/services/src/social/publisher.rs` | 2 |
| 8 | Person Profile page redesign | `frontend/src/pages/person-detail.tsx` | 2 |
| 9 | Social analytics dashboard | `frontend/src/pages/social-analytics.tsx` | 2 |
| 10 | Editron content agent | `crates/server/src/routes/editron.rs` | 3 |
| 11 | mention_alerts + social listening | `crates/server/src/workers/social_listener.rs` | 3 |
| 12 | Cross-org content delivery | `crates/server/src/routes/content_delivery.rs` | 3 |
