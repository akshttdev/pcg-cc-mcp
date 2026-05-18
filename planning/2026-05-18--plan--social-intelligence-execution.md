# Social Intelligence Engine — Execution Plan
**Date:** 2026-05-18  
**Status:** Ready to Execute  
**Supersedes:** `2026-05-18--plan--social-intelligence-engine.md` (research doc)

---

## Current State Audit Summary

```
WHAT EXISTS TODAY
─────────────────
✅  social_posts / social_accounts — org/user ownership columns added (Phase A)
✅  social_account_snapshots — daily account metrics (follower, reach, engagement)
✅  social_post_metrics_snapshots — per-post time-series metrics
✅  social_mentions — full inbox with sentiment, priority, author signals
✅  pulse_sources / pulse_content_items / pulse_alerts — intel collection stack
✅  Publisher service — retry loop, 5/9 platform connectors live
✅  Scheduler service — runs, uses hardcoded optimal times (needs learning)
✅  MCP tools — 6 social tools in task_server (create, publish, list)
✅  client_visible gate — column added to project_knowledge_sources
✅  Phase A struct fixes — all CreateSocialPost/Account literals compile-ready

⬜  source_category — PulseSource.category exists but not semantically typed
⬜  social_tracked_entities — no table; no way to define competitors/KOLs/press
⬜  social_content_opportunities — no table; no signal→draft pipeline
⬜  social_audience_insights — no table; no mention clustering output
⬜  social_performance_benchmarks — no table; scheduler can't learn
⬜  social_share_of_voice — no table; no competitive share tracking
⬜  intelligence services — no signal processors yet
⬜  analytics API routes — snapshot data exists, no query endpoints
⬜  client report endpoint — no curated client-facing report
⬜  Intelligence sub-view in frontend — not built
⬜  Topsi intelligence tools — social_ops.rs has creation tools only
```

---

## Full System Architecture

```
╔══════════════════════════════════════════════════════════════════════╗
║                   ORCHA SOCIAL INTELLIGENCE PLATFORM                ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║  ┌─────────────────────────────────────────────────────────────┐    ║
║  │                   SIGNAL INGEST LAYER                        │    ║
║  │                                                              │    ║
║  │  COMPETITORS    KOLs/INFLUENCERS    THOUGHT LEADERS         │    ║
║  │      │               │                    │                 │    ║
║  │  TRADE PRESS    KEYWORD FEEDS       OWN MENTIONS            │    ║
║  │      │               │                    │                 │    ║
║  │  └───────────────────┴────────────────────┘                 │    ║
║  │                       │                                      │    ║
║  │              PulseSources (transport)                        │    ║
║  │         source_category = intent layer (NEW)                │    ║
║  │                       │                                      │    ║
║  │              PulseContentItems                               │    ║
║  │         (enriched, entity-extracted, scored)                │    ║
║  └──────────────────────┬──────────────────────────────────────┘    ║
║                         │                                            ║
║  ┌──────────────────────▼──────────────────────────────────────┐    ║
║  │              SIGNAL PROCESSING LAYER (NEW)                   │    ║
║  │                                                              │    ║
║  │   ┌─────────────────┐   ┌─────────────────────────────┐    │    ║
║  │   │ TrendDetector   │   │  NewsEventDetector           │    │    ║
║  │   │ (keyword velo-  │   │  (trade press → time        │    │    ║
║  │   │  city vs base)  │   │   window detection)          │    │    ║
║  │   └────────┬────────┘   └──────────────┬──────────────┘    │    ║
║  │            │                            │                    │    ║
║  │   ┌────────▼────────┐   ┌──────────────▼──────────────┐    │    ║
║  │   │CompetitorAnalyz │   │  MentionClusterer            │    │    ║
║  │   │ (gap + format + │   │  (LLM topic grouping →      │    │    ║
║  │   │  cadence gaps)  │   │   audience insights)         │    │    ║
║  │   └────────┬────────┘   └──────────────┬──────────────┘    │    ║
║  │            │                            │                    │    ║
║  │   ┌────────▼────────┐   ┌──────────────▼──────────────┐    │    ║
║  │   │ KOLSignalExtract│   │  SnapshotAnalyzer            │    │    ║
║  │   │ (format signal  │   │  (benchmark learning +       │    │    ║
║  │   │  + collab hook) │   │   fatigue detection)         │    │    ║
║  │   └────────┬────────┘   └──────────────┬──────────────┘    │    ║
║  │            └────────────────┬───────────┘                   │    ║
║  └─────────────────────────────│───────────────────────────────┘    ║
║                                │                                     ║
║  ┌─────────────────────────────▼───────────────────────────────┐    ║
║  │         CONTENT OPPORTUNITY PIPELINE (NEW)                   │    ║
║  │                                                              │    ║
║  │  detected → briefed → draft_created → approved → scheduled  │    ║
║  │                                                              │    ║
║  │  ┌──────────────────────────────────────────────────────┐   │    ║
║  │  │  OpportunityDrafter (LLM)                            │   │    ║
║  │  │  • Pull client's top 5 posts as style reference      │   │    ║
║  │  │  • Generate: caption + hashtags + format + slot      │   │    ║
║  │  │  • Create social_posts draft, link to opportunity    │   │    ║
║  │  └──────────────────────────────────────────────────────┘   │    ║
║  └─────────────────────────────┬───────────────────────────────┘    ║
║                                │                                     ║
║  ┌─────────────────────────────▼───────────────────────────────┐    ║
║  │           SOCIAL CALENDAR (existing, enhanced)               │    ║
║  │                                                              │    ║
║  │  Draft → Review → Approved → Scheduled → Published          │    ║
║  │              ↑                                               │    ║
║  │        Topsi review interface                                │    ║
║  └─────────────────────────────┬───────────────────────────────┘    ║
║                                │                                     ║
║  ┌─────────────────────────────▼───────────────────────────────┐    ║
║  │          PERFORMANCE FEEDBACK LAYER                          │    ║
║  │                                                              │    ║
║  │  social_account_snapshots ──────→ social_performance_       │    ║
║  │  (daily metrics)                   benchmarks               │    ║
║  │                                    (learned optimal times)  │    ║
║  │  social_post_metrics_snapshots ──→ Scheduler.get_optimal_   │    ║
║  │  (per-post time-series)            times_learned()          │    ║
║  │                                                              │    ║
║  │  SocialMentions ────────────────→ social_audience_insights  │    ║
║  │  (comments, DMs, mentions)         (FAQ / sentiment clusters│    ║
║  │                                     → new opportunities)    │    ║
║  └─────────────────────────────────────────────────────────────┘    ║
║                                                                      ║
║  ┌──────────────────┐    ┌─────────────────────────────────────┐    ║
║  │  CLIENT REPORTS  │    │   TOPSI INTELLIGENCE INTERFACE      │    ║
║  │  (curated,       │    │   (team-facing, Sirak/Alicai)       │    ║
║  │   read-only)     │    │   • Content opportunities           │    ║
║  │  • Growth delta  │    │   • Performance summaries           │    ║
║  │  • Top 3 posts   │    │   • KOL signals                     │    ║
║  │  • Eng. rate     │    │   • Audience insights               │    ║
║  │  • Posts sent    │    │   • Share of voice                  │    ║
║  └──────────────────┘    └─────────────────────────────────────┘    ║
╚══════════════════════════════════════════════════════════════════════╝
```

---

## Data Model Overview

```
social_tracked_entities          social_content_opportunities
──────────────────────           ────────────────────────────
id                               id
organization_id ─────────┐       organization_id ────────────┐
entity_type               │       project_id                  │
  competitor               │       opportunity_type            │
  kol_influencer           │       signal_source_type          │
  thought_leader           │       signal_source_id            │
  trade_press              │       tracked_entity_id ──────────┘
name                       │       title / brief
instagram_handle           │       suggested_format
twitter_handle             │       suggested_platforms
linkedin_url               │       suggested_slot
crm_contact_id ────────→ CRM       suggested_hashtags
niche_relevance            │       draft_post_id ──────────→ social_posts
follower_estimate          │       status: detected→approved
                           │       relevance_score
                           │       expires_at
                           │
         pulse_sources ────┘
         ──────────────
         source_category (renamed from category)
           competitor
           kol_influencer
           thought_leader
           trade_press
           keyword_feed
           owned_mentions


social_performance_benchmarks     social_audience_insights
─────────────────────────────     ────────────────────────
social_account_id                 organization_id
day_of_week (0-6)                 social_account_id
hour_of_day (0-23)                insight_type: faq|objection|...
avg_engagement_rate               topic
avg_reach                         mention_count
avg_saves                         sample_mentions (JSON)
avg_impressions                   sentiment
sample_count                      sentiment_score
confidence (0.0-1.0)              suggested_content_angle
                                  opportunity_id ──→ opportunities


social_share_of_voice
─────────────────────
organization_id
period_start / period_end
keyword_set (JSON)
platform (nullable = cross-platform)
own_mention_count
total_niche_mention_count
share_of_voice_pct
sentiment breakdown (own + niche)
top_keywords (JSON)
competitor_summary (JSON)
```

---

## Signal Flow Detail

```
SIGNAL: Trending topic detected
────────────────────────────────
PulseContentItem ingested
    │
    ├── source_category = kol_influencer
    │       │
    │       └── KOLSignalExtractor
    │               Extract: topic, format type, engagement estimate
    │               Check: does client have post on this topic in next 7 days?
    │               No → CREATE social_content_opportunities
    │                       type = kol_signal
    │                       brief = "KOL @handle posted about [topic] getting
    │                                high engagement. Their format was [X]."
    │
    ├── source_category = keyword_feed
    │       │
    │       └── TrendDetector
    │               Count keyword mentions in 24h rolling window
    │               Compare to 7-day baseline
    │               If velocity > 3× baseline AND no client post on topic:
    │               CREATE social_content_opportunities
    │                       type = trending_topic
    │                       expires_at = now + 48h
    │
    └── source_category = trade_press
            │
            └── NewsEventDetector
                    LLM: extract event, date, relevance to niche
                    If event within 14 days AND relevance > 0.6:
                    CREATE social_content_opportunities
                            type = news_event
                            expires_at = event_date + 48h

    ↓ (any opportunity created)

OpportunityDrafter (async, triggered by DB insert)
    Load: opportunity brief + type + tracked entity
    Load: client's top 5 posts by (saves + reach) as style reference
    LLM call →
        caption (platform-appropriate length)
        hashtags (10-15, niche + broad mix)
        format recommendation + rationale
        suggested publish slot (from benchmarks if confidence > 0.6)
    CREATE social_posts (status = draft)
    UPDATE opportunity: status = draft_created, draft_post_id = new post id

    ↓

Topsi surfaces: "New content opportunity for [client]: [title]
                 Draft ready for review. [Approve/Edit/Reject]"

    ↓ (Alicai or Topsi approves)

Opportunity status = approved
Post status = pending_review → approved → scheduled
```

```
SIGNAL: Audience question cluster
──────────────────────────────────
SocialMentions collected (hourly batch, up to 200 per org)
    │
    └── MentionClusterer
            LLM: "Group these by topic. For each:
                  - topic label
                  - sentiment (positive/neutral/negative/mixed)
                  - count
                  - content angle suggestion"
            │
            ├── UPSERT social_audience_insights per cluster
            │
            ├── If cluster > 5 mentions on same topic:
            │   CREATE social_content_opportunities (type = audience_question)
            │
            └── CRISIS DETECTION:
                If negative sentiment cluster grows > 15 mentions/hour:
                CREATE pulse_alert (priority = urgent)
                → Topsi immediately surfaced alert
```

```
FEEDBACK: Scheduler learns from performance
────────────────────────────────────────────
SnapshotAnalyzer (runs after each daily snapshot write)
    │
    ├── Load last 90 days of social_account_snapshots
    │   + social_post_metrics_snapshots for same period
    │
    ├── For each published post: bucket by (day_of_week, hour_of_day)
    │   Calculate per-bucket: avg engagement, avg reach, avg saves
    │
    ├── UPSERT social_performance_benchmarks
    │   Increment sample_count, recalculate averages
    │   Set confidence = min(sample_count / 20, 1.0)
    │
    ├── CONTENT FATIGUE DETECTION:
    │   3-week rolling engagement decline > 20% in same format category:
    │   CREATE social_content_opportunities (type = performance_pattern)
    │   brief = "Carousel engagement declining 23% over 3 weeks.
    │            Consider switching to Reels or static for this content type."
    │
    └── Scheduler.get_optimal_times_learned(account_id):
        If benchmarks exist AND confidence > 0.6:
            Return top 4 slots by avg_saves DESC
        Else:
            Return category-based hardcoded defaults (existing behavior)
```

---

## Sprint-by-Sprint Execution Plan

---

### SPRINT 0 — Finish Phase A + Deploy
**Estimated time:** 2–4 hours  
**Goal:** Get the current session's work live in Docker.

```
Files changed this session (already done):
┌─────────────────────────────────────────────────────────────────┐
│  crates/db/migrations/20260518400000_social_org_scope.sql   ✅  │
│  crates/db/src/models/social_account.rs                     ✅  │
│  crates/db/src/models/social_post.rs                        ✅  │
│  crates/services/src/services/social/scheduler.rs           ✅  │
│  crates/server/src/mcp/task_server/social_ops.rs            ✅  │
│  crates/server/src/mcp/nora_server.rs                       ✅  │
│  crates/nora/src/conference_workflow/social.rs              ✅  │
│  crates/server/src/routes/social_accounts.rs                ✅  │
│  crates/server/src/routes/social_posts.rs                   ✅  │
└─────────────────────────────────────────────────────────────────┘

Steps:
1. cargo check (verify no remaining compile errors)
2. cargo build -p server (release build with OpenSSL env vars)
3. Apply migration to Docker DB:
   docker exec pcg-cc-mcp sqlite3 /app/db.sqlite < migration_file
   OR let server auto-apply on start (sqlx runs pending migrations)
4. docker cp target/release/server pcg-cc-mcp:/usr/local/bin/server.new
5. Swap binary and restart container
```

---

### SPRINT 1 — Analytics Foundation
**Estimated time:** 2–3 days  
**Goal:** Real performance data visible. Scheduler starts learning. Client report exists.

#### 1A — Migrations

```sql
-- Migration: 20260519200000_social_analytics_foundation.sql

-- Learned optimal times per account
CREATE TABLE social_performance_benchmarks (
    id                TEXT PRIMARY KEY,
    social_account_id TEXT NOT NULL REFERENCES social_accounts(id) ON DELETE CASCADE,
    day_of_week       INTEGER NOT NULL,  -- 0=Sunday, 6=Saturday
    hour_of_day       INTEGER NOT NULL,  -- 0-23 UTC
    avg_engagement_rate REAL DEFAULT 0.0,
    avg_reach           REAL DEFAULT 0.0,
    avg_saves           REAL DEFAULT 0.0,
    avg_impressions     REAL DEFAULT 0.0,
    sample_count        INTEGER NOT NULL DEFAULT 0,
    confidence          REAL DEFAULT 0.0,  -- 0.0-1.0
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(social_account_id, day_of_week, hour_of_day)
);
CREATE INDEX idx_benchmarks_account ON social_performance_benchmarks(social_account_id);
```

#### 1B — SnapshotAnalyzer Service

New file: `crates/services/src/services/social/intelligence/snapshot_analyzer.rs`

```
SnapshotAnalyzer::run_for_account(pool, account_id)
  1. Fetch all social_post_metrics_snapshots for account (last 90 days)
     JOIN social_posts to get published_at timestamp
  2. Bucket by (day_of_week, hour_of_day) using strftime on published_at
  3. Calculate averages per bucket
  4. UPSERT social_performance_benchmarks (rolling update formula)
  5. Check fatigue: 3-week rolling decline → create opportunity (Sprint 3+)

Called from:
  - After snapshot write in social_metrics_sync route
  - Daily cron in main.rs automation loop
```

#### 1C — Scheduler Enhancement

Update `crates/services/src/services/social/scheduler.rs`:

```rust
// Add to Scheduler:
pub async fn get_optimal_times_learned(
    &self,
    account_id: Uuid,
) -> Vec<String> {
    // Query social_performance_benchmarks
    // WHERE social_account_id = account_id
    //   AND confidence > 0.6
    // ORDER BY avg_saves DESC
    // LIMIT 4
    // Map to "HH:MM" format
    // Fallback to get_optimal_times(category) if no benchmarks
}

// Update find_next_available_slot() to call get_optimal_times_learned()
// when post.social_account_id is Some
```

#### 1D — Analytics API Routes

New route handlers in `crates/server/src/routes/social_accounts.rs`:

```
GET /social/accounts/:id/analytics?days=30
  Returns: AnalyticsSummary {
    total_impressions, total_reach, total_likes, total_comments,
    total_shares, total_saves, total_clicks, posts_published,
    avg_engagement_rate,
    daily: Vec<DailyMetrics>,    // snapshot time series
    by_format: Vec<FormatMetrics>, // post | carousel | reel | story
    top_posts: Vec<TopPost>,     // by saves + reach combined
    benchmark_heatmap: Vec<BenchmarkSlot>  // day×hour grid
  }

GET /api/organizations/:id/social-report?project_id=&period=30d
  Returns: ClientReport {
    period: { start, end },
    accounts: [{
      platform, username, avatar_url,
      follower_growth: { start, end, delta, pct_change },
      reach_total, impressions_total,
      avg_engagement_rate,
      posts_published,
      top_posts: [{ thumbnail, caption_preview, likes, saves, reach }],
      best_performing_format
    }]
  }
```

#### 1E — Frontend: Analytics Sub-View

New file: `frontend/src/pages/organization-profile/tabs/social/SocialAnalyticsView.tsx`

```
Route: ?sv=analytics

┌────────────────────────────────────────────────────────┐
│  Analytics              [30d ▼]  [All Accounts ▼]      │
├────────────────────────────────────────────────────────┤
│                                                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────┐ │
│  │+847       │  │  42.3K   │  │  4.2%    │  │  12   │ │
│  │followers  │  │  reach   │  │  eng.    │  │ posts │ │
│  └──────────┘  └──────────┘  └──────────┘  └───────┘ │
│                                                        │
│  BEST TIMES TO POST (heatmap: day × hour)              │
│  ┌─────────────────────────────────────────────────┐   │
│  │     Mon  Tue  Wed  Thu  Fri  Sat  Sun            │   │
│  │ 6am  ·    ·    ·    ·    ·    ○    ○             │   │
│  │12pm  ●    ○    ●    ○    ●    ●    ●             │   │
│  │ 6pm  ●    ●    ●    ●    ●    ·    ○             │   │
│  │ 9pm  ○    ·    ○    ·    ●    ●    ●             │   │
│  │  ● = high  ○ = medium  · = low                   │   │
│  └─────────────────────────────────────────────────┘   │
│                                                        │
│  TOP POSTS                    FORMAT BREAKDOWN         │
│  ┌─────────────────────┐  ┌─────────────────────────┐ │
│  │[img] Happy hour...  │  │ Carousel    ████  5.1%  │ │
│  │      saves:82 r:8K  │  │ Reel        ███   3.8%  │ │
│  │[img] Meet the team  │  │ Static      ██    2.4%  │ │
│  │      saves:61 r:6K  │  │ Story       █     1.9%  │ │
│  └─────────────────────┘  └─────────────────────────┘ │
└────────────────────────────────────────────────────────┘
```

#### 1F — Frontend: Client Report View

Update `frontend/src/pages/client-overview.tsx`:
- Wire `?sv=` from `useSearchParams` (fixes existing bug)
- Add `report` tab → `ClientReportView` component

```
Route: /clients/:id?sv=report

┌────────────────────────────────────────────────────────┐
│  [Client Name] — Social Performance Report             │
│  May 2026                            [Generate PDF]    │
├────────────────────────────────────────────────────────┤
│  INSTAGRAM @handle                                     │
│  ┌───────────┬───────────┬───────────┬──────────────┐  │
│  │+847        │42K/wk     │4.2% avg   │12 posts      │  │
│  │followers   │reach      │engagement │published     │  │
│  └───────────┴───────────┴───────────┴──────────────┘  │
│                                                        │
│  TOP CONTENT THIS PERIOD                               │
│  [post thumbnail] "Happy hour starts..." saves:82      │
│  [post thumbnail] "Meet the bar team..." reach:8.2K    │
│  [post thumbnail] "This week's special" eng:6.1%       │
│                                                        │
│  AUDIENCE                                              │
│  Top engaged topics: cocktails · events · behind-bar   │
└────────────────────────────────────────────────────────┘
```

---

### SPRINT 2 — Signal Sources + Tracked Entities
**Estimated time:** 1–2 days  
**Goal:** Team can define who/what to monitor. Intelligence pipeline has its input config.

#### 2A — Migrations

```sql
-- Migration: 20260519300000_social_tracked_entities.sql

-- Rename pulse_sources.category → source_category
-- (SQLite: add new column, copy data, no drop needed since it's nullable)
ALTER TABLE pulse_sources ADD COLUMN source_category TEXT NOT NULL DEFAULT 'keyword_feed';
UPDATE pulse_sources SET source_category = COALESCE(category, 'keyword_feed');
-- category column kept for backwards compat

-- Named tracked entities
CREATE TABLE social_tracked_entities (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    entity_type     TEXT NOT NULL,
    -- competitor | kol_influencer | thought_leader | trade_press
    name            TEXT NOT NULL,
    description     TEXT,
    instagram_handle TEXT,
    twitter_handle   TEXT,
    linkedin_url     TEXT,
    tiktok_handle    TEXT,
    youtube_channel  TEXT,
    website_url      TEXT,
    crm_contact_id  TEXT REFERENCES crm_contacts(id) ON DELETE SET NULL,
    follower_estimate INTEGER,
    niche_relevance   REAL DEFAULT 0.5,
    is_active         INTEGER NOT NULL DEFAULT 1,
    notes             TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_tracked_entities_org  ON social_tracked_entities(organization_id);
CREATE INDEX idx_tracked_entities_type ON social_tracked_entities(entity_type);
```

#### 2B — Model + Routes

New file: `crates/db/src/models/social_tracked_entity.rs`  
New routes in `crates/server/src/routes/social_intelligence.rs`:

```
GET    /api/organizations/:id/intelligence/tracked-entities?type=
POST   /api/organizations/:id/intelligence/tracked-entities
PATCH  /api/social/tracked-entities/:id
DELETE /api/social/tracked-entities/:id
```

#### 2C — Topsi Tool

Add to `crates/server/src/mcp/task_server/intelligence_ops.rs` (new file):

```
add_tracked_entity(org_id, name, entity_type, handles..., notes?)
get_tracked_entities(org_id, entity_type?)
```

#### 2D — Frontend: Tracked Entities Manager

Add to `SocialContentView.tsx` or new panel in `sv=intelligence`:

```
┌────────────────────────────────────────────────────┐
│  Signal Sources             [+ Add Source]         │
├────────────────────────────────────────────────────┤
│  COMPETITORS (3)                                   │
│  ┌──────────────────────────────────────────────┐  │
│  │ 🏢 Rival Bar Co    IG:@rivalbar  TW:@rival  │  │
│  │ 🏢 The Local       IG:@thelocal             │  │
│  │ 🏢 Craft House     IG:@crafthouse           │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  KOLs / INFLUENCERS (7)                            │
│  ┌──────────────────────────────────────────────┐  │
│  │ 👤 @cocktailkween  450K · relevance: ●●●●○  │  │
│  │ 👤 @barlife_daily  120K · relevance: ●●●○○  │  │
│  └──────────────────────────────────────────────┘  │
│                                                    │
│  TRADE PRESS (5)                                   │
│  ┌──────────────────────────────────────────────┐  │
│  │ 📰 Eater         (RSS)                       │  │
│  │ 📰 Punch Drink   (RSS)                       │  │
│  │ 📰 Bar Business  (RSS + Twitter)             │  │
│  └──────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────┘
```

---

### SPRINT 3 — Content Opportunity Pipeline
**Estimated time:** 3–4 days  
**Goal:** Signals automatically become draft posts for Alicai to approve.  
**This is the core competitive moat.**

#### 3A — Migration

```sql
-- Migration: 20260519400000_social_content_opportunities.sql

CREATE TABLE social_content_opportunities (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    opportunity_type TEXT NOT NULL,
    -- trending_topic | kol_signal | news_event | audience_question
    -- competitor_gap | performance_pattern | thought_leader_angle
    signal_source_type TEXT,
    -- pulse_content_item | social_mention_cluster | snapshot_analysis | manual
    signal_source_id   TEXT,
    tracked_entity_id  TEXT REFERENCES social_tracked_entities(id) ON DELETE SET NULL,
    title               TEXT NOT NULL,
    brief               TEXT,
    suggested_format    TEXT,
    suggested_platforms TEXT,  -- JSON array
    suggested_slot      TEXT,  -- ISO datetime
    suggested_hashtags  TEXT,  -- JSON array
    draft_post_id TEXT REFERENCES social_posts(id) ON DELETE SET NULL,
    status          TEXT NOT NULL DEFAULT 'detected',
    -- detected → briefed → draft_created → approved → rejected → scheduled | expired
    relevance_score REAL DEFAULT 0.5,
    expires_at      TEXT,
    reviewed_by   TEXT REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at   TEXT,
    review_notes  TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_opportunities_org     ON social_content_opportunities(organization_id);
CREATE INDEX idx_opportunities_status  ON social_content_opportunities(status);
CREATE INDEX idx_opportunities_expires ON social_content_opportunities(expires_at)
    WHERE expires_at IS NOT NULL;
```

#### 3B — Signal Processor Services

New directory: `crates/services/src/services/social/intelligence/`

```
intelligence/
├── mod.rs          — IntelligenceEngine::new(pool), run_all_processors()
├── trend_detector.rs
├── news_event_detector.rs
├── competitor_analyzer.rs
├── kol_signal_extractor.rs
└── opportunity_drafter.rs
```

**TrendDetector — trigger on PulseContentItem ingest:**
```
Input: new PulseContentItem (source_category = keyword_feed OR kol_influencer)
1. Count keyword mentions in 24h window (from PulseContentItems)
2. Calculate 7-day hourly baseline for same keywords
3. If velocity > 3× baseline:
   a. Check: any social_post for org with matching topic in next 7 days?
   b. No → create social_content_opportunities (type = trending_topic, expires = +48h)
```

**NewsEventDetector — trigger on trade_press ingest:**
```
Input: new PulseContentItem (source_category = trade_press)
LLM call: extract { event_name, event_date, relevance_score, niche_match }
If event_date within 14 days AND relevance_score > 0.6:
  create social_content_opportunities (type = news_event, expires = event_date + 48h)
```

**KOLSignalExtractor — trigger on kol_influencer ingest:**
```
Input: new PulseContentItem (source_category = kol_influencer | thought_leader)
LLM call: extract { topic, format_type, engagement_signals, content_angle }
If engagement_signals suggest high performance:
  create social_content_opportunities (type = kol_signal)
  brief includes: KOL name, topic, format, why it's performing
```

**CompetitorAnalyzer — weekly batch:**
```
For each org:
  Pull PulseContentItems from competitor sources (last 7 days)
  LLM: extract topic list per competitor
  Compare to client's published topics (last 30 days)
  Gaps (competitor covers, client doesn't, 2+ posts) → competitor_gap opportunity
  Format gaps → performance_pattern opportunity
  Calculate: social_share_of_voice record (Sprint 5)
```

**OpportunityDrafter — triggered on opportunity creation:**
```
Input: social_content_opportunities record (status = detected)
1. Fetch opportunity brief + type + entity context
2. Fetch client's top 5 posts (by saves+reach) as style reference
3. LLM call with prompt:
   "You are a social media strategist for [client niche].
    Opportunity: [brief]
    Style reference: [top post captions]
    Generate: caption, 12 hashtags, format (post/carousel/reel), rationale"
4. Apply benchmark slot (if confidence > 0.6) or category default
5. CREATE social_posts (status = draft, category = opportunity type)
6. UPDATE opportunity: status = draft_created, draft_post_id = new id
```

**Automation hook in main.rs:**
```rust
// Add to hourly automation loop:
tokio::spawn(async move {
    let engine = IntelligenceEngine::new(pool.clone());
    engine.run_periodic_processors().await;
    // Runs: CompetitorAnalyzer (weekly), MentionClusterer, SnapshotAnalyzer
});

// Triggered per-item (in pulse collection handler):
engine.process_new_content_item(item_id).await;
// Runs: TrendDetector, NewsEventDetector, KOLSignalExtractor per item
```

#### 3C — Opportunity API Routes

Add to `social_intelligence.rs`:

```
GET  /api/organizations/:id/intelligence/opportunities
     ?status=detected|draft_created|approved
     ?type=trending_topic|kol_signal|...
     ?limit=20

GET  /api/social/opportunities/:id

PATCH /api/social/opportunities/:id
  body: { status: "approved"|"rejected", review_notes? }
  — On approved: advances draft_post status to pending_review
  — Validates status transition
```

#### 3D — Frontend: Opportunities View

New file: `frontend/src/pages/organization-profile/tabs/social/SocialIntelligenceView.tsx`  
Route: `?sv=intelligence`

```
┌─────────────────────────────────────────────────────────────┐
│  Content Intelligence                      [View Sources]   │
├─────────────────────────────────────────────────────────────┤
│  OPEN OPPORTUNITIES (6)            [Filter: All types ▼]   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 🔥 TRENDING · Expires in 31h                        │   │
│  │ Low-ABV cocktails dominating bar conversation       │   │
│  │ "3× velocity spike on #lowaбv. KOL @spiritsnerds   │   │
│  │  posted a reel getting 14K saves."                  │   │
│  │ Suggested: Reel · Instagram + TikTok · Tonight 7pm  │   │
│  │                        [View Draft] [Approve] [✕]  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 📰 NEWS EVENT · Expires in 6d                       │   │
│  │ Tales of the Cocktail - Denver edition announced    │   │
│  │ "Industry event Jun 3. 4 competitor venues already  │   │
│  │  posting about it. Timely content window."          │   │
│  │ Suggested: Carousel · Instagram · Jun 1 12pm        │   │
│  │                        [View Draft] [Approve] [✕]  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ 💬 AUDIENCE QUESTION · No expiry                    │   │
│  │ "What's your house margarita recipe?" (8 mentions)  │   │
│  │ "Recurring question in comments across 3 posts.     │   │
│  │  An FAQ post or Reel would perform well."           │   │
│  │ Suggested: Post · Instagram · Fri 5pm               │   │
│  │                        [View Draft] [Approve] [✕]  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  [Load 3 more]                                             │
└─────────────────────────────────────────────────────────────┘
```

#### 3E — Topsi Intelligence Tools

New file: `crates/server/src/mcp/task_server/intelligence_ops.rs`

```
get_content_opportunities(org_id, status?, type?, limit?)
  → Returns opportunity cards with brief, format suggestion, draft link

approve_content_opportunity(opportunity_id, notes?)
  → Marks approved, advances draft to pending_review queue

reject_content_opportunity(opportunity_id, reason?)
  → Marks rejected

get_social_performance_summary(org_id?, project_id?, period?)
  → Follower growth, top posts, engagement rate, format breakdown

get_audience_insights(org_id?, project_id?, period?)
  → Clustered FAQ/sentiment topics from mentions
```

---

### SPRINT 4 — Audience Intelligence
**Estimated time:** 2–3 days  
**Goal:** Comment and mention clusters surface content ideas and flag crisis signals.

#### 4A — Migration

```sql
-- Migration: 20260519500000_social_audience_insights.sql

CREATE TABLE social_audience_insights (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    social_account_id TEXT REFERENCES social_accounts(id) ON DELETE CASCADE,
    insight_type TEXT NOT NULL,
    -- faq | objection | praise | topic_interest | sentiment_shift
    topic           TEXT NOT NULL,
    mention_count   INTEGER NOT NULL DEFAULT 1,
    sample_mentions TEXT,  -- JSON array (anonymized text)
    sentiment       TEXT,  -- positive | neutral | negative | mixed
    sentiment_score REAL,  -- -1.0 to 1.0
    suggested_content_angle TEXT,
    opportunity_id TEXT REFERENCES social_content_opportunities(id) ON DELETE SET NULL,
    period_start TEXT NOT NULL,
    period_end   TEXT NOT NULL,
    actioned     INTEGER NOT NULL DEFAULT 0,
    dismissed    INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_audience_insights_org     ON social_audience_insights(organization_id);
CREATE INDEX idx_audience_insights_account ON social_audience_insights(social_account_id);
```

#### 4B — MentionClusterer Service

New file: `crates/services/src/services/social/intelligence/mention_clusterer.rs`

```
MentionClusterer::run_for_org(pool, org_id)
  1. Pull up to 200 unread mentions for org (last 24h)
  2. LLM batch call:
     "Group into topics. Per group: label, sentiment, count, content angle"
  3. UPSERT social_audience_insights (period = last 24h)
  4. For groups > 5 mentions: create social_content_opportunities (audience_question)
  5. Crisis check: if negative group growing > 15/hour:
     CREATE pulse_alert (priority = urgent, message = "Reputation spike detected")
```

#### 4C — API + Frontend

```
GET /api/organizations/:id/intelligence/audience-insights
    ?period=7d|30d&type=faq|sentiment_shift|...

Frontend: "Audience Signals" section in SocialIntelligenceView

┌────────────────────────────────────────────────────┐
│  AUDIENCE SIGNALS (last 30 days)                   │
├────────────────────────────────────────────────────┤
│  💬 FAQ (3 topics)                                 │
│   • "What's your house margarita?" — 8 mentions    │
│   • "Do you do private events?"    — 6 mentions    │
│   • "Happy hour times?"            — 5 mentions    │
│                                                    │
│  ✨ PRAISE                                         │
│   • "Vibe/atmosphere" — 31 positive mentions       │
│   • "Staff friendliness" — 18 positive mentions    │
│                                                    │
│  📉 WATCH                                          │
│   • "Wait times" — 7 neutral/negative mentions     │
└────────────────────────────────────────────────────┘
```

---

### SPRINT 5 — Share of Voice + KOL Pipeline
**Estimated time:** 2–3 days  
**Goal:** Agency-grade competitive intelligence. KOL tracking bridges to CRM.

#### 5A — Migration

```sql
-- Migration: 20260519600000_social_share_of_voice.sql

CREATE TABLE social_share_of_voice (
    id              TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_start    TEXT NOT NULL,
    period_end      TEXT NOT NULL,
    keyword_set     TEXT NOT NULL,  -- JSON array
    platform        TEXT,           -- NULL = cross-platform
    own_mention_count         INTEGER NOT NULL DEFAULT 0,
    total_niche_mention_count INTEGER NOT NULL DEFAULT 0,
    share_of_voice_pct        REAL,
    own_positive_pct  REAL,
    own_neutral_pct   REAL,
    own_negative_pct  REAL,
    niche_positive_pct REAL,
    niche_neutral_pct  REAL,
    niche_negative_pct REAL,
    top_keywords      TEXT,  -- JSON [{keyword, count, velocity}]
    competitor_summary TEXT, -- JSON [{entity_id, name, mention_count, share_pct}]
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);
```

#### 5B — ShareOfVoiceCalculator + CompetitorAnalyzer

```
ShareOfVoiceCalculator::calculate_weekly(pool, org_id)
  1. Get org's keyword_set from PulseTrackingConfig
  2. Count PulseContentItems matching keywords where source is org's mentions
     → own_mention_count
  3. Count all PulseContentItems matching keywords (all sources)
     → total_niche_mention_count
  4. Per competitor: count their mentions → competitor_summary
  5. INSERT social_share_of_voice record

CompetitorAnalyzer::analyze_weekly(pool, org_id)
  1. Pull competitor PulseContentItems (last 7 days)
  2. LLM: topic extraction per competitor
  3. Compare to client topic coverage → gaps → opportunities
  4. Call ShareOfVoiceCalculator
```

#### 5C — KOL → CRM Bridge

```
KOLSignalExtractor (enhancement):
  When creating kol_signal opportunity:
  IF tracked_entity.crm_contact_id IS NOT NULL:
    Create CrmActivity on the contact:
      type = "kol_signal"
      note = "High-engagement content detected. Topic: [X]. Format: [Y]."
  ELSE IF niche_relevance > 0.8:
    Add to opportunity brief:
      "Consider outreach: [entity.name] is in your space and performing well."
```

#### 5D — API + Frontend

```
GET /api/organizations/:id/intelligence/share-of-voice?period=30d&platform=

Frontend: SOV widget in SocialIntelligenceView

┌──────────────────────────────────────────┐
│  Share of Voice — Last 30 Days           │
│  [instagram ▼]                           │
├──────────────────────────────────────────┤
│                                          │
│  ████████░░░░░░░░░░░░░░░  Your brand 23% │
│  ██████████████░░░░░░░░░  Rival Bar  31% │
│  ████████░░░░░░░░░░░░░░░  The Local  18% │
│  ░░░░░░░░░░░░░░░░░░░░░░░  Other      28% │
│                                          │
│  Trending in niche: #lowaabv · #craftbar │
│  Your coverage: ●●○○○  Competitor: ●●●●○ │
└──────────────────────────────────────────┘
```

Additional Topsi tools:
```
get_share_of_voice(org_id, period?, platform?)
get_kol_signals(org_id, limit?)
  → Recent high-signal KOL/thought leader content worth responding to
```

---

### SPRINT 6 — Topsi Intelligence Interface (Full)
**Estimated time:** 1–2 days  
**Goal:** Topsi can fully brief and manage the content pipeline in conversation.

Complete `intelligence_ops.rs` with all tools:

```
get_content_opportunities(org_id, status?, type?, limit?)
approve_content_opportunity(opportunity_id, notes?)
reject_content_opportunity(opportunity_id, reason?)
get_social_performance_summary(org_id?, project_id?, period?)
get_audience_insights(org_id?, project_id?, period?)
get_share_of_voice(org_id, period?, platform?)
get_kol_signals(org_id, limit?)
get_tracked_entities(org_id, entity_type?)
add_tracked_entity(org_id, name, entity_type, handles..., notes?)
generate_client_report(org_id, project_id, period?)
```

**Topsi conversations this enables:**
```
Team: "What should we post for the Bodega account this week?"
Topsi: "You have 3 open opportunities:
        1. Low-ABV trend — draft ready, expires tomorrow night
        2. Tales of the Cocktail event — draft ready, deadline Jun 1
        3. House margarita FAQ — no expiry, 8 audience requests
        Want me to approve any of these?"

Team: "What's our share of voice on Instagram this month?"
Topsi: "Bodega is at 23% share of voice in the local bar niche,
        up from 18% last month. Main competitor Rival Bar Co
        leads at 31%. Their edge is posting frequency — 4×/week
        vs your 2×/week. Want me to surface some quick-win
        opportunities to close that gap?"

Team: "Add Punch magazine as a press source for Bodega."
Topsi: "Added Punch (punchdrink.com) as a trade press source for
        Bodega. RSS feed configured. First collection will run in
        the next hourly cycle."
```

---

## Frontend Navigation Updates

```
Organization Social Tab sub-views (sv= param):
──────────────────────────────────────────────
sv=content     (existing) — content calendar + scheduler
sv=accounts    (existing) — connected platform accounts
sv=intelligence (NEW Sprint 3) — opportunities + signals + SOV
sv=analytics   (NEW Sprint 1) — performance metrics + heatmap
sv=sources     (NEW Sprint 2) — tracked entities manager

New tab navigation row:
┌──────────┬──────────┬───────────────┬───────────┬──────────┐
│ Content  │ Accounts │ Intelligence  │ Analytics │ Sources  │
└──────────┴──────────┴───────────────┴───────────┴──────────┘

Client Overview sub-views (sv= param, read-only):
─────────────────────────────────────────────────
sv=social  (existing) — client's content calendar
sv=report  (NEW Sprint 1) — curated performance report
Fix: wire useSearchParams to initialize tab state (existing bug)
```

---

## File Creation Map

```
NEW FILES BY SPRINT:

Sprint 0 (complete existing work):
  No new files — all changes already made

Sprint 1 (analytics):
  crates/db/migrations/20260519200000_social_analytics_foundation.sql
  crates/db/src/models/social_performance_benchmark.rs
  crates/services/src/services/social/intelligence/mod.rs
  crates/services/src/services/social/intelligence/snapshot_analyzer.rs
  frontend/src/pages/organization-profile/tabs/social/SocialAnalyticsView.tsx
  frontend/src/components/social/ClientReportView.tsx
  frontend/src/lib/api/social-analytics.ts

Sprint 2 (tracked entities):
  crates/db/migrations/20260519300000_social_tracked_entities.sql
  crates/db/src/models/social_tracked_entity.rs
  frontend/src/components/social/TrackedEntitiesManager.tsx

Sprint 3 (opportunity pipeline):
  crates/db/migrations/20260519400000_social_content_opportunities.sql
  crates/db/src/models/social_content_opportunity.rs
  crates/services/src/services/social/intelligence/trend_detector.rs
  crates/services/src/services/social/intelligence/news_event_detector.rs
  crates/services/src/services/social/intelligence/kol_signal_extractor.rs
  crates/services/src/services/social/intelligence/opportunity_drafter.rs
  crates/server/src/routes/social_intelligence.rs
  crates/server/src/mcp/task_server/intelligence_ops.rs
  frontend/src/pages/organization-profile/tabs/social/SocialIntelligenceView.tsx

Sprint 4 (audience intelligence):
  crates/db/migrations/20260519500000_social_audience_insights.sql
  crates/db/src/models/social_audience_insight.rs
  crates/services/src/services/social/intelligence/mention_clusterer.rs

Sprint 5 (share of voice):
  crates/db/migrations/20260519600000_social_share_of_voice.sql
  crates/db/src/models/social_share_of_voice.rs
  crates/services/src/services/social/intelligence/competitor_analyzer.rs
  crates/services/src/services/social/intelligence/share_of_voice_calculator.rs

Sprint 6 (Topsi full wiring):
  intelligence_ops.rs enhanced (all 10 tools complete)

MODIFIED FILES BY SPRINT:

Sprint 0:
  (build + deploy only — all edits already done)

Sprint 1:
  crates/db/src/models/mod.rs              — add social_performance_benchmark
  crates/services/src/services/social/scheduler.rs — add get_optimal_times_learned()
  crates/services/src/services/social/mod.rs — add intelligence module
  crates/server/src/routes/social_accounts.rs — add analytics endpoint
  crates/server/src/routes/mod.rs          — add social_intelligence routes
  frontend/src/pages/organization-profile/tabs/social/index.tsx — add sv=analytics
  frontend/src/pages/client-overview.tsx   — fix sv= param + add report tab

Sprint 2:
  crates/db/src/models/mod.rs              — add social_tracked_entity
  crates/server/src/routes/mod.rs          — register intelligence routes
  crates/server/src/mcp/task_server/mod.rs — register intelligence_ops

Sprint 3:
  crates/db/src/models/mod.rs              — add social_content_opportunity
  crates/server/src/main.rs (or automations) — wire intelligence engine to hourly loop

Sprint 4:
  crates/db/src/models/mod.rs              — add social_audience_insight
  crates/services/src/services/social/intelligence/mod.rs — add mention_clusterer

Sprint 5:
  crates/db/src/models/mod.rs              — add social_share_of_voice
  crates/server/src/mcp/task_server/intelligence_ops.rs — add SOV + KOL tools
```

---

## Dependency Chain

```
Sprint 0 (Phase A deploy)
    │
    └── Sprint 1 (analytics + benchmarks)
            │
            ├── Sprint 2 (tracked entities + source_category)
            │       │
            │       └── Sprint 3 (opportunity pipeline)  ← CORE MOAT
            │               │
            │               └── Sprint 4 (audience intelligence)
            │                       │
            │                       └── Sprint 5 (SOV + KOL bridge)
            │                               │
            │                               └── Sprint 6 (Topsi full)
            │
            └── (Sprint 1 client report is independently shippable)
```

Sprints 1 and 2 can be parallelized (no shared code dependencies).  
Sprint 3 requires Sprint 2's tracked_entities table for signal attribution.  
Sprints 4, 5, 6 can each be shipped incrementally.

---

## Roadmap Alignment

```
Stage 0 Month 2 Priority 4: "Social Media Pipeline Migration"
  Sprint 0  → Phase A multi-owner architecture shipped
  Sprint 1  → "Dashboard for cross-platform analytics" ✓
  Sprint 2  → Pulse configured as intelligence input ✓
  Sprint 3  → "Wire content pipeline to ORCHA workflows" ✓
  Sprint 4  → "Template library for recurring content types" (FAQ → series) ✓

Stage 1 Priority 10: "Sirak Studios Pilot"
  Sprint 3+4 → The opportunity pipeline IS the pilot product
  Sprint 5   → Agency-grade SOV, the billable differentiator

Stage 1 Priority 9: "Multi-Tenant Hardening"
  All tables properly org/project scoped
  client_visible gate enforced on KG
  Client report API shows zero pulse/intel data
```
