# Social Intelligence Engine — Full Architecture Plan
**Date:** 2026-05-18  
**Status:** Planning  
**Roadmap alignment:** Stage 0 Month 2 Priority 4 — "Social Media Pipeline Migration" → "Dashboard for cross-platform analytics"

---

## 1. Vision

Transform the social calendar from a scheduling tool into a **closed-loop intelligence platform**. The pulse engine already collects signals from the world. The social calendar already publishes content. This plan builds the missing layer between them: signal processing, opportunity detection, auto-drafting, and performance feedback — all surfaced through Topsi for the team and through curated reports for clients.

The competitive moat is the loop itself:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    THE INTELLIGENCE LOOP                             │
│                                                                      │
│  WORLD                                                               │
│    ↓                                                                 │
│  ┌──────────────────────────────────────────────────┐               │
│  │              SIGNAL INGEST LAYER                 │               │
│  │  Competitors · KOLs · Thought Leaders ·          │               │
│  │  Trade Press · Hashtag Feeds · Own Mentions      │               │
│  └──────────────────┬───────────────────────────────┘               │
│                     ↓                                                │
│  ┌──────────────────────────────────────────────────┐               │
│  │           SIGNAL PROCESSING LAYER                │               │
│  │  Trend Detector · Competitor Analyzer ·          │               │
│  │  Audience Miner · Performance Feedback ·         │               │
│  │  KOL Signal Extractor · News Event Detector      │               │
│  └──────────────────┬───────────────────────────────┘               │
│                     ↓                                                │
│  ┌──────────────────────────────────────────────────┐               │
│  │         CONTENT OPPORTUNITY PIPELINE             │               │
│  │  detected → briefed → draft_created →            │               │
│  │  approved → scheduled                            │               │
│  └──────────────────┬───────────────────────────────┘               │
│                     ↓                                                │
│  ┌──────────────────────────────────────────────────┐               │
│  │           SOCIAL CALENDAR (existing)             │               │
│  │  Schedule · Publish · Track Performance          │               │
│  └──────────────────┬───────────────────────────────┘               │
│                     ↓                                                │
│  ┌──────────────────────────────────────────────────┐               │
│  │           PERFORMANCE FEEDBACK LAYER             │               │
│  │  Snapshot Analytics → Scheduler Tuning ·         │               │
│  │  Post Performance → Format Weighting ·           │               │
│  │  Comment Clusters → New Tracking Keywords        │               │
│  └──────────────────────────────────────────────────┘               │
│                     ↓                                                │
│  CLIENT REPORTS (curated, read-only)                                 │
│  TOPSI QUERIES   (team-facing intelligence interface)                │
└─────────────────────────────────────────────────────────────────────┘
```

---

## 2. Signal Source Taxonomy

The existing `PulseSource` model has `source_type` (transport: RSS, Twitter, YouTube, Reddit, Web).
We add `source_category` (intent) as a new column.

### Source Categories

```
source_category       intent / downstream processing
─────────────────     ──────────────────────────────────────────────────
competitor            Gap analysis, format benchmarking, share of voice,
                      posting cadence tracking
kol_influencer        Trending format detection, early-signal topics,
                      collaboration opportunity surfacing, CRM lead hook
thought_leader        Topic authority signals, emerging ideas,
                      credibility alignment, content brief inspiration
trade_press           Industry news → timely response windows,
                      event coverage opportunities, authority content
keyword_feed          Hashtag/keyword conversation velocity,
                      sentiment drift, emerging formats
owned_mentions        Comment/DM topic clustering, FAQ surface,
                      crisis detection, audience insight mining
```

Each category triggers a **different signal processor** when a `PulseContentItem` is ingested.
A single tracked entity (e.g., a KOL) can have multiple PulseSources across platforms —
the category stays constant, the source_type (transport) varies.

### Example Source Configuration

```
KOL "Jamie Oliver" (hypothetical hospitality client):
  PulseSource { source_category: kol_influencer, source_type: youtube, url: RSS feed }
  PulseSource { source_category: kol_influencer, source_type: twitter, url: @jamieoliver }
  PulseSource { source_category: kol_influencer, source_type: rss, url: jamieoliver.com/feed }

Trade Press:
  PulseSource { source_category: trade_press, source_type: rss, url: eater.com/rss }
  PulseSource { source_category: trade_press, source_type: rss, url: fsrmagazine.com/rss }

Hashtag Feed:
  PulseSource { source_category: keyword_feed, source_type: twitter, config: {"query": "#hospitalitylife #barculture"} }
```

---

## 3. New Data Models

### 3.1 `pulse_sources` — ADD COLUMN `source_category`

```sql
ALTER TABLE pulse_sources ADD COLUMN source_category TEXT NOT NULL DEFAULT 'keyword_feed';
-- values: competitor | kol_influencer | thought_leader | trade_press | keyword_feed | owned_mentions
```

### 3.2 `social_tracked_entities` (new table)

Represents a named entity being tracked across multiple pulse sources.
Separates the "who we're tracking" from the "how we're tracking them."

```sql
CREATE TABLE social_tracked_entities (
    id          TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id  TEXT REFERENCES projects(id) ON DELETE SET NULL,
    entity_type TEXT NOT NULL, -- competitor | kol_influencer | thought_leader | trade_press
    name        TEXT NOT NULL,
    description TEXT,
    -- Platform handles (for display + CRM linking)
    instagram_handle TEXT,
    twitter_handle   TEXT,
    linkedin_url     TEXT,
    tiktok_handle    TEXT,
    youtube_channel  TEXT,
    website_url      TEXT,
    -- CRM link (KOLs may become collaboration leads)
    crm_contact_id  TEXT REFERENCES crm_contacts(id) ON DELETE SET NULL,
    -- Metadata
    follower_estimate INTEGER, -- rough order of magnitude for priority weighting
    niche_relevance   REAL DEFAULT 0.5, -- 0.0-1.0, manually set or AI-scored
    is_active         INTEGER NOT NULL DEFAULT 1,
    notes             TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX idx_tracked_entities_org ON social_tracked_entities(organization_id);
CREATE INDEX idx_tracked_entities_type ON social_tracked_entities(entity_type);
```

### 3.3 `social_content_opportunities` (new table)

Central record for a detected signal → content draft pipeline.

```sql
CREATE TABLE social_content_opportunities (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,

    -- Signal origin
    opportunity_type TEXT NOT NULL,
    -- values: trending_topic | competitor_gap | kol_signal | news_event |
    --         audience_question | performance_pattern | thought_leader_angle

    signal_source_type TEXT, -- pulse_content_item | social_mention_cluster | snapshot_analysis | manual
    signal_source_id   TEXT, -- FK to the originating record (flexible)
    tracked_entity_id  TEXT REFERENCES social_tracked_entities(id) ON DELETE SET NULL,

    -- Content brief
    title               TEXT NOT NULL, -- short: "New cocktail trend: low-ABV highballs"
    brief               TEXT,          -- AI-generated: why timely, what angle, what to say
    suggested_format    TEXT,          -- post | carousel | reel | story | thread
    suggested_platforms TEXT,          -- JSON array: ["instagram","linkedin"]
    suggested_slot      TEXT,          -- ISO datetime recommendation
    suggested_hashtags  TEXT,          -- JSON array

    -- Draft linkage
    draft_post_id TEXT REFERENCES social_posts(id) ON DELETE SET NULL,

    -- Lifecycle
    status          TEXT NOT NULL DEFAULT 'detected',
    -- detected → briefed → draft_created → approved → rejected → scheduled | expired
    relevance_score REAL DEFAULT 0.5,
    expires_at      TEXT, -- NULL = no expiry; trending topics get ~48h windows

    -- Review
    reviewed_by   TEXT REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at   TEXT,
    review_notes  TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX idx_opportunities_org ON social_content_opportunities(organization_id);
CREATE INDEX idx_opportunities_status ON social_content_opportunities(status);
CREATE INDEX idx_opportunities_type ON social_content_opportunities(opportunity_type);
CREATE INDEX idx_opportunities_expires ON social_content_opportunities(expires_at)
    WHERE expires_at IS NOT NULL;
```

### 3.4 `social_audience_insights` (new table)

Aggregated insights from mention + comment clustering.

```sql
CREATE TABLE social_audience_insights (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    social_account_id TEXT REFERENCES social_accounts(id) ON DELETE CASCADE,

    insight_type TEXT NOT NULL,
    -- faq | objection | praise | feature_request | topic_interest | sentiment_shift

    topic           TEXT NOT NULL,  -- extracted topic label
    mention_count   INTEGER NOT NULL DEFAULT 1,
    sample_mentions TEXT,           -- JSON array of example texts (anonymized)
    sentiment       TEXT,           -- positive | neutral | negative | mixed
    sentiment_score REAL,           -- -1.0 to 1.0

    -- Content action
    suggested_content_angle TEXT,   -- AI-generated: "Post an FAQ about X"
    opportunity_id TEXT REFERENCES social_content_opportunities(id) ON DELETE SET NULL,

    period_start TEXT NOT NULL,
    period_end   TEXT NOT NULL,
    actioned     INTEGER NOT NULL DEFAULT 0,
    dismissed    INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX idx_audience_insights_org ON social_audience_insights(organization_id);
CREATE INDEX idx_audience_insights_account ON social_audience_insights(social_account_id);
```

### 3.5 `social_performance_benchmarks` (new table)

Per-account learned optimal posting patterns derived from snapshot history.
Replaces hardcoded times in the scheduler.

```sql
CREATE TABLE social_performance_benchmarks (
    id                TEXT PRIMARY KEY,
    social_account_id TEXT NOT NULL REFERENCES social_accounts(id) ON DELETE CASCADE,
    day_of_week       INTEGER NOT NULL, -- 0=Sunday, 6=Saturday
    hour_of_day       INTEGER NOT NULL, -- 0-23 UTC
    avg_engagement_rate REAL,
    avg_reach           REAL,
    avg_saves           REAL,
    avg_impressions     REAL,
    sample_count        INTEGER NOT NULL DEFAULT 0,
    confidence          REAL DEFAULT 0.0, -- 0.0-1.0, rises with sample_count
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(social_account_id, day_of_week, hour_of_day)
);

CREATE INDEX idx_benchmarks_account ON social_performance_benchmarks(social_account_id);
```

### 3.6 `social_share_of_voice` (new table)

Weekly share of voice snapshots per org, per niche/keyword set.

```sql
CREATE TABLE social_share_of_voice (
    id              TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_start    TEXT NOT NULL,
    period_end      TEXT NOT NULL,
    keyword_set     TEXT NOT NULL, -- JSON array of tracked keywords/hashtags
    platform        TEXT,          -- NULL = cross-platform aggregate
    -- Volume
    own_mention_count        INTEGER NOT NULL DEFAULT 0,
    total_niche_mention_count INTEGER NOT NULL DEFAULT 0,
    share_of_voice_pct       REAL,  -- own / total * 100
    -- Sentiment breakdown
    own_positive_pct  REAL,
    own_neutral_pct   REAL,
    own_negative_pct  REAL,
    niche_positive_pct REAL,
    niche_neutral_pct  REAL,
    niche_negative_pct REAL,
    -- Top content signals
    top_keywords      TEXT, -- JSON: [{keyword, count, velocity}]
    competitor_summary TEXT, -- JSON: [{entity_id, mention_count, share_pct}]
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
```

---

## 4. Signal Processing Services

New service module: `crates/services/src/services/social/intelligence/`

```
intelligence/
├── mod.rs
├── snapshot_analyzer.rs      — daily metrics → benchmark updates + fatigue detection
├── mention_clusterer.rs      — groups mentions by topic → audience_insights
├── trend_detector.rs         — PulseContentItem velocity → trending topic opportunities
├── kol_signal_extractor.rs   — KOL/thought leader content → format + topic signals
├── news_event_detector.rs    — trade press items → time-sensitive content windows
├── competitor_analyzer.rs    — competitor content → gap analysis opportunities
├── opportunity_drafter.rs    — opportunity record → AI-generated draft post
└── share_of_voice_calculator.rs — weekly SOV rollup
```

### 4.1 SnapshotAnalyzer

**Trigger:** After each `social_account_snapshots` record is written (daily).

**Logic:**
1. Load last 90 days of snapshots for the account.
2. Calculate average engagement rate per (day_of_week, hour_of_day) bucket.
   - A post published at 7pm Tuesday → its metrics go into bucket (Tue, 19).
   - Use `posts_published` + engagement metrics from snapshot to approximate.
3. Upsert `social_performance_benchmarks` — raise confidence as sample_count grows.
4. Detect **content fatigue**: 3-week rolling engagement decline > 20% → create
   `social_content_opportunities` of type `performance_pattern` briefing a format refresh.
5. Detect **growth spikes**: follower delta > 2σ above 30-day mean → flag for analysis.

**Scheduler output:** Updates `Scheduler::get_optimal_times()` to query benchmarks
instead of hardcoded category times (when confidence > 0.6).

### 4.2 MentionClusterer

**Trigger:** Hourly batch on unprocessed `social_mentions` (status = unread, no cluster yet).

**Logic:**
1. Pull up to 200 recent unprocessed mentions per org.
2. LLM call: "Group these into topics. For each group: topic label, sentiment, count,
   content angle suggestion."
3. Upsert `social_audience_insights` records.
4. For groups with > 5 mentions on same topic → create `social_content_opportunities`
   of type `audience_question`.
5. **Crisis detection:** If negative sentiment cluster grows > 15 mentions/hour →
   create `pulse_alert` with priority = urgent.

### 4.3 TrendDetector

**Trigger:** After each `PulseContentItem` is ingested, for items from
`source_category IN (keyword_feed, kol_influencer, thought_leader)`.

**Logic:**
1. Count mentions of extracted entities/keywords in a 24h rolling window.
2. Compare to 7-day baseline for same window.
3. Velocity threshold: current_rate > 3× baseline → trending signal.
4. Check: does client have a post on this topic scheduled in the next 7 days?
   - No → create `social_content_opportunities` of type `trending_topic`.
   - Yes → skip (already covered).
5. For KOL sources specifically: if KOL post is getting high engagement (estimated
   from comment velocity on the source) → create `kol_signal` opportunity noting
   the format and topic.

### 4.4 NewsEventDetector

**Trigger:** After each `PulseContentItem` from `source_category = trade_press`.

**Logic:**
1. LLM extracts: event type, date window, relevance to org's niche.
2. If event is within 14 days and relevance > 0.6:
   - Create `social_content_opportunities` of type `news_event`.
   - Set `expires_at` = event date + 48h (content after is stale).
   - Brief includes: "Industry event [X] happening [date]. Timely content window."

### 4.5 CompetitorAnalyzer

**Trigger:** Weekly batch, per org.

**Logic:**
1. Pull all `PulseContentItems` from competitor sources in the last 7 days.
2. Extract topics per competitor using LLM entity extraction.
3. Compare competitor topic set to client's published topics (last 30 days).
4. Gaps (topics they cover, you don't) with > 2 competitor posts → `competitor_gap` opportunity.
5. Format analysis: if competitors are posting 3× more carousels than client →
   `performance_pattern` opportunity noting format gap.
6. Calculate share of voice contribution for `social_share_of_voice` table.

### 4.6 OpportunityDrafter

**Trigger:** When a `social_content_opportunity` record is created (status = detected).

**Logic:**
1. Pull opportunity context: type, brief, tracked entity, originating content item.
2. Pull client's top 5 performing posts (by saves + reach) as style reference.
3. LLM call → generate:
   - Caption (platform-appropriate length)
   - Hashtag set (10-15, mix of niche + broad)
   - Format recommendation with rationale
   - Suggested publish slot (from benchmarks if available, else category defaults)
4. Create `social_posts` record with `status = draft`, link to opportunity.
5. Update opportunity `status = draft_created`, set `draft_post_id`.

---

## 5. API Routes (New)

### Intelligence Routes

```
GET    /api/organizations/:id/intelligence/opportunities
       ?status=detected|draft_created|approved
       ?type=trending_topic|kol_signal|...
       ?limit=20

PATCH  /api/social/opportunities/:id
       body: { status, review_notes }
       — approve/reject/request edits

GET    /api/organizations/:id/intelligence/audience-insights
       ?period=7d|30d
       ?type=faq|sentiment_shift|...

GET    /api/organizations/:id/intelligence/share-of-voice
       ?period=7d|30d|90d
       ?platform=instagram|linkedin|...

GET    /api/organizations/:id/intelligence/tracked-entities
POST   /api/organizations/:id/intelligence/tracked-entities
PATCH  /api/social/tracked-entities/:id
DELETE /api/social/tracked-entities/:id

GET    /api/social/accounts/:id/analytics
       ?period=7d|30d|90d
       — returns: snapshot time series, benchmark heatmap, top posts

GET    /api/organizations/:id/social-report
       ?project_id=...
       — client-facing curated report (follower growth, top posts, engagement rate)
```

---

## 6. Frontend UI

### 6.1 Org Social Tab — Intelligence View (new sub-view)

Add `sv=intelligence` to the existing `?sv=` routing system.

```
┌─────────────────────────────────────────────────────┐
│  Content Intelligence              [+ Add Source]   │
│                                                     │
│  ┌─ OPPORTUNITIES ──────────────────────────────┐   │
│  │  🔥 Trending: Low-ABV cocktails     [Draft→] │   │
│  │  📰 News event: Tales of Cocktail   [Draft→] │   │
│  │  💬 Audience Q: "What's your house  [Draft→] │   │
│  │     margarita recipe?"                       │   │
│  │  📉 Competitor gap: Sunday brunch   [Draft→] │   │
│  │     content (4 competitors posting)          │   │
│  │                                    [View all]│   │
│  └──────────────────────────────────────────────┘   │
│                                                     │
│  ┌─ SIGNAL SOURCES ──────────────────────────────┐  │
│  │  Competitors (3)  KOLs (7)  Press (5)         │  │
│  │  [Manage Sources]                             │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ┌─ SHARE OF VOICE ─────────────────────────────┐   │
│  │  Your brand: 23% │ Competitor A: 31% │       │   │
│  │  Competitor B: 18% │ Other: 28%      │       │   │
│  │  [7d ▼]  [instagram ▼]               [Full→] │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### 6.2 Org Social Tab — Analytics View (new sub-view `sv=analytics`)

```
┌─────────────────────────────────────────────────────┐
│  Account Performance        [30d ▼]  [instagram ▼]  │
│                                                     │
│  Followers ↑ 847   Reach 42K   Avg. Eng. 4.2%      │
│  ──────────────────────────────────────────────     │
│  [Engagement heatmap: day × hour grid]              │
│                                                     │
│  TOP POSTS (by saves + reach)                       │
│  ┌─────────────────────────────────────────────┐    │
│  │ [img] "Happy hour starts at 4..." ↑saves 82 │    │
│  │ [img] "Behind the bar: meet..."   ↑reach 8K │    │
│  │ [img] "This week's special..."    ↑eng. 6.1%│    │
│  └─────────────────────────────────────────────┘    │
│                                                     │
│  FORMAT PERFORMANCE                                 │
│  Carousel 5.1% · Reel 3.8% · Static 2.4%           │
└─────────────────────────────────────────────────────┘
```

### 6.3 Tracked Entities Manager

Modal or side-panel for adding/editing tracked entities.
Fields: name, entity_type (dropdown), platform handles, niche relevance slider,
notes, CRM contact link (for KOLs).

### 6.4 Client Report View (`/clients/:id?sv=report`)

Read-only. Curated data only — no pulse intel, no competitor data.

```
┌─────────────────────────────────────────────────────┐
│  [Client Name] Social Performance — May 2026        │
│                                                     │
│  GROWTH           REACH          ENGAGEMENT         │
│  +847 followers   42K/week       4.2% avg           │
│                                                     │
│  TOP PERFORMING CONTENT THIS MONTH                  │
│  [top 3 posts with thumbnail, caption, metrics]     │
│                                                     │
│  CONTENT PUBLISHED: 12 posts across 3 platforms     │
│  [bar chart by platform]                            │
│                                                     │
│  AUDIENCE HIGHLIGHTS                                │
│  Most engaged topics: cocktails, events, behind-    │
│  the-scenes. Growing follower segment: 25-34 F.     │
└─────────────────────────────────────────────────────┘
```

---

## 7. Topsi Intelligence Tools

New file: `crates/server/src/mcp/task_server/intelligence_ops.rs`

Tools available to Topsi (team-facing, not client-facing):

```
get_content_opportunities(org_id, status?, type?, limit?)
  → list of open opportunities with brief + suggested format

approve_content_opportunity(opportunity_id, notes?)
  → marks approved, triggers draft to enter calendar review queue

get_social_performance_summary(org_id?, project_id?, period?)
  → follower growth, top posts, engagement rate, format breakdown

get_audience_insights(org_id?, project_id?, period?)
  → clustered FAQ/sentiment/topic insights from mentions

get_share_of_voice(org_id, period?, platform?)
  → own % vs. competitors in the niche

get_kol_signals(org_id, limit?)
  → recent high-signal KOL/thought leader content worth responding to

get_tracked_entities(org_id, entity_type?)
  → list of competitors/KOLs/press being monitored

add_tracked_entity(org_id, name, entity_type, handles...)
  → add a new competitor/KOL/press source to the watch list
```

**Topsi query examples this enables:**
- "What content opportunities do we have for the Bodega client this week?"
- "How did our hospitality clients perform last month?"
- "What are KOLs talking about in the bar scene right now?"
- "Add Eater Denver as a trade press source for the Bodega account."
- "Show me our share of voice vs. competitors for the last 30 days."
- "What is our audience asking about most in comments?"

---

## 8. KOL → CRM Collaboration Pipeline

When a KOL is tracked and shows strong signal alignment with a client's niche,
the system creates a soft link to the CRM. This surfaces as a note in the
person/contact record and can be promoted to an outreach deal.

```
KOL tracked in pulse
    ↓ (high niche_relevance + recent high-engagement content)
social_tracked_entities.crm_contact_id = [linked contact]
    ↓
CRM person record gets tag: "KOL — content signal active"
    ↓
Topsi surfaces: "Jamie X has been posting about [topic] in your space.
                 High engagement. Consider collaboration outreach."
    ↓
One-click: create CRM activity / deal for outreach
```

This is a unique differentiator. Most social tools have no CRM bridge for KOL discovery.

---

## 9. Performance Feedback → Scheduler Tuning

The scheduler currently uses hardcoded times by category. After benchmarks accumulate:

```rust
// In Scheduler::get_optimal_times(), after Phase 1:
async fn get_optimal_times_learned(&self, account_id: Uuid) -> Vec<String> {
    // Query social_performance_benchmarks for this account
    // Filter: confidence > 0.6, order by avg_saves DESC
    // Return top 4 slots as "HH:MM" strings
    // Fallback to category defaults if confidence too low
}
```

The feedback loop means the scheduler gets measurably smarter per account over time.

---

## 10. Implementation Phases

### Phase 1 — Analytics Foundation (2-3 days)
**Goal:** Real metrics visible, scheduler starts learning.

- Migration: add `source_category` to `pulse_sources`
- Migration: create `social_performance_benchmarks`
- SnapshotAnalyzer service: benchmark upsert logic
- Analytics API: `/api/social/accounts/:id/analytics`
- Client report API: `/api/organizations/:id/social-report`
- Frontend: `sv=analytics` sub-view in org social tab
- Client report view in client-overview

### Phase 2 — Signal Sources + Tracked Entities (2-3 days)
**Goal:** Team can configure who/what to monitor.

- Migration: create `social_tracked_entities`
- Tracked entities CRUD API + Topsi tools
- Frontend: Tracked Entities Manager in org social tab
- Wire `source_category` through existing PulseSource CRUD
- Update pulse collection to tag items with `source_category`

### Phase 3 — Audience Intelligence (2-3 days)
**Goal:** Comment/mention clusters surface content ideas.

- Migration: create `social_audience_insights`
- MentionClusterer service (batch, LLM-powered)
- Audience insights API + Topsi tool
- Frontend: audience insights section in `sv=intelligence`

### Phase 4 — Content Opportunity Pipeline (3-4 days)
**Goal:** The closed loop. Signals → drafts in the calendar.

- Migration: create `social_content_opportunities`
- TrendDetector, NewsEventDetector, CompetitorAnalyzer services
- OpportunityDrafter service (auto-draft on detection)
- Opportunity API: list + approve/reject
- Frontend: opportunities card view in `sv=intelligence`
- Topsi tools: get/approve opportunities

### Phase 5 — Share of Voice + KOL Pipeline (2-3 days)
**Goal:** Agency-grade competitive intelligence.

- Migration: create `social_share_of_voice`
- ShareOfVoiceCalculator (weekly batch)
- SOV API + Topsi tool
- KOL → CRM bridge logic
- Frontend: SOV widget, KOL signal feed
- Topsi tool: `get_kol_signals`

---

## 11. Roadmap Alignment

```
Stage 0 Month 2 Priority 4: "Social Media Pipeline Migration"
  Phase 1 (Analytics) ✓ — "Dashboard for cross-platform analytics"
  Phase 2 (Sources)   ✓ — "Wire content pipeline to ORCHA workflows"
  Phase 3 (Audience)  ✓ — "Template library for recurring content types" (FAQ → series)

Stage 1 Priority 10: "Sirak Studios Pilot"
  Phase 4 (Opportunity Pipeline) — this IS the pilot feature set
  Phase 5 (SOV + KOL) — agency differentiator for client pitches

Stage 1 Priority 9: "Multi-Tenant Hardening"
  All tables are properly org/project scoped
  Client report API enforces client_visible gate
```

---

## 12. Open Questions / Deferred

- **Platform API access for metrics**: Instagram Insights requires Business account with
  specific OAuth scopes. LinkedIn has content analytics. TikTok Business API is restrictive.
  Phase 1 analytics work from `social_account_snapshots` (which we populate from syncs).
  Real-time metrics ingestion is a separate platform API integration sprint.

- **KOL follower count estimation**: For public Twitter/Instagram accounts, follower count
  is accessible via public API (or can be scraped). This feeds `niche_relevance` scoring.

- **Client report delivery**: Phase 1 is dashboard-only. PDF generation or email digest
  is a Phase 2 enhancement (could use Topsi to trigger generation).

- **Competitor public metrics**: Engagement rate on competitor posts requires either
  a social listening API (Brandwatch, Keyhole) or estimation from comment/like counts
  on public posts. Phase 4 uses estimation; paid API is an upgrade path.
