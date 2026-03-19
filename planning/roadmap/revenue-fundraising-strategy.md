# ORCHA Revenue & Fundraising Strategy

**Day 3 Deliverable — 5-Year Product Roadmap Research Sprint**
**Date**: 2026-03-18
**Status**: Draft
**Author**: PCG Strategy

---

## 1. Executive Summary

ORCHA is positioned to execute a **hybrid agency-to-SaaS transition**, the highest-ROI path for a 2-3 person founding team with an existing agency client network. The recommended sequencing is:

1. **Immediate (Q2 2026)**: Generate agency revenue using ORCHA as internal delivery infrastructure — zero additional sales cycle, validates the platform under production load.
2. **Near-term (Q3-Q4 2026)**: Convert 2-5 agency clients into managed platform pilots at premium pricing ($500-2K/month), building toward a pre-seed raise of $250-500K.
3. **Medium-term (Q1-Q2 2027)**: Launch self-service SaaS with hybrid seat + usage pricing, targeting $25-50K MRR and a $1-3M seed round.
4. **Growth (2028-2029)**: Scale to $100K+ MRR, raise Series A ($5-15M) to fund go-to-market and APN network expansion.

This approach de-risks the fundraising path by generating revenue from day one, validating product-market fit through real agency delivery, and building a referral pipeline from satisfied clients before scaling self-service.

**Key insight**: PCG's existing billing relationships eliminate the cold-start problem that kills most developer tools. Every agency client is a potential platform customer who already trusts the team.

---

## 2. PCG's Existing Client Network as Distribution Channel

### The Agency-to-SaaS Transition Model

PCG operates as a creative/tech agency with an active deal pipeline managed through a 9-stage CRM (Lead, Qualified, Proposal, Negotiation, Pilot, Contract, Onboarding, Active, Closed Won/Lost). This pipeline represents a ready-made distribution channel for ORCHA.

| Asset | Strategic Value |
|-------|----------------|
| Active deal pipeline | Warm leads who already understand PCG's capabilities |
| Sirak Studios partnership | First external org validates multi-tenant architecture |
| Existing billing relationships | No new procurement cycle needed for platform upsell |
| Agency delivery track record | Social proof and case studies from real work |
| Client trust | Reduces adoption friction vs. cold SaaS outreach |

### Distribution Flywheel

```
PCG wins agency deal
  → Delivers using ORCHA internally (higher margins)
    → Client sees results, asks "how did you do that?"
      → Offer managed platform access (platform revenue)
        → Client refers other agencies/teams
          → Self-service SaaS pipeline grows
```

This is the same playbook executed by:
- **Basecamp** (37signals consulting → project management SaaS)
- **HubSpot** (agency services → marketing automation platform)
- **Figma** (design agency relationships → design tool standard)

### Advantage Over Pure-Play Startups

A pure SaaS startup building ORCHA would need to:
1. Raise capital before generating any revenue
2. Build the product without real-world feedback loops
3. Cold-sell to agencies with no existing trust
4. Burn cash on marketing to acquire early adopters

PCG skips steps 1-3 entirely by dogfooding the platform and leveraging existing relationships.

---

## 3. Revenue Model Analysis

### Model A: Agency Services First (Hybrid)

| Dimension | Assessment |
|-----------|------------|
| **Mechanism** | Use ORCHA internally for PCG agency delivery; bill clients for services |
| **Timeline to revenue** | 0-2 months (already doing agency work) |
| **Revenue type** | Project-based, hourly, or retainer |
| **Scalability** | Linear (hours x rate) |
| **Valuation multiple** | 1-3x revenue (services multiple) |

**Pros**:
- Immediate revenue from existing client relationships
- Validates platform through real production use
- No additional engineering needed for billing/onboarding
- Platform efficiency translates directly to higher agency margins (deliver $10K of work in 60% of the time)

**Cons**:
- Revenue scales linearly with headcount
- Services companies command lower valuation multiples
- Risk of becoming "just an agency" and never transitioning to product

### Model B: SaaS-First

| Dimension | Assessment |
|-----------|------------|
| **Mechanism** | Build billing, onboarding, multi-tenant isolation; launch subscription product |
| **Timeline to revenue** | 3-6 months |
| **Revenue type** | Recurring subscriptions |
| **Scalability** | Exponential (marginal cost near zero) |
| **Valuation multiple** | 10-30x ARR (SaaS multiple) |

**Target customers**: Agencies, consulting firms, creative studios, and technical teams needing AI orchestration with data sovereignty.

**Pros**:
- Scalable, recurring revenue
- Higher valuation multiples (10-30x ARR vs. 1-3x for services)
- Network effects from workflow marketplace and APN mesh
- Defensible moat from data gravity and switching costs

**Cons**:
- 8-12 weeks minimum to revenue readiness (billing, onboarding, docs)
- Needs marketing/sales motion with no existing pipeline
- Zero revenue during build phase burns runway
- Multi-tenant isolation and security are table stakes, not differentiators

### Model C: Hybrid (Recommended)

| Phase | Timeline | Focus | Revenue Source |
|-------|----------|-------|----------------|
| Phase 1 | Q2-Q3 2026 | Dogfood + agency delivery | Agency services ($5-20K/mo) |
| Phase 2 | Q3-Q4 2026 | Managed platform pilots | Agency + pilot licenses ($15-30K/mo) |
| Phase 3 | Q1-Q2 2027 | Self-service SaaS launch | SaaS + agency ($25-50K/mo) |

**Pros**:
- Revenue from day one funds continued development
- Real-world validation before scaling
- De-risks SaaS launch with proven workflows and case studies
- Natural customer development pipeline (agency client → pilot → SaaS subscriber → reference)

**Cons**:
- Requires discipline to not become services-only
- Must allocate engineering time to platform features, not just client deliverables
- Dual revenue model adds operational complexity

**Mitigation for services trap**: Set a firm rule — no more than 60% of engineering time on client work. Remaining 40% is always platform development. Track this weekly.

---

## 4. Pricing Model Options

### Option 1: Seat-Based (Linear Model)

| Tier | Price | Includes |
|------|-------|----------|
| Free | $0 | 1 org, 3 users, 100 tasks |
| Pro | $15/user/month | Unlimited tasks, workflows, 5 agents |
| Team | $30/user/month | Unlimited agents, priority support, API access |
| Enterprise | Custom | SSO, dedicated infrastructure, SLA |

**Comparable**: Linear ($10-15/user/month, $400M valuation with notably low $35K lifetime marketing spend — a testament to product-led growth).

**Strengths**: Predictable revenue, easy to understand, aligns with team growth.
**Weaknesses**: Penalizes adoption (users resist adding seats), doesn't capture AI value, commoditized pricing model.

### Option 2: Usage-Based (AI Platform Model)

| Tier | Price | Includes |
|------|-------|----------|
| Base | $49/org/month | $20 AI credits included |
| AI credits | $0.01-0.10/action | Based on model tier (local vs. cloud) |
| Overage | Pay-as-you-go | Same rate as credits |
| Enterprise | Custom | Volume discounts, committed use |

**Comparable**: Cursor ($20-200/user/month, reached $1B ARR, Anysphere valued at $29.3B — fastest to $100M ARR in history, per The Information, Feb 2025).

**Strengths**: Revenue scales with value delivered, captures AI compute margins, low barrier to entry.
**Weaknesses**: Unpredictable bills frustrate customers, hard to budget for, revenue volatility.

### Option 3: Hybrid Seat + Usage (Recommended)

| Tier | Monthly Price | Users | AI Actions/Month |
|------|--------------|-------|-------------------|
| Free | $0 | 2 | 500 |
| Starter | $29 | 5 | 2,000 |
| Pro | $79 | 15 | 10,000 |
| Scale | $199 | 50 | 50,000 |
| Enterprise | Custom | Unlimited | Custom |

**Overage AI actions**: $0.005-0.05 each (model-dependent; local APN actions at low end, cloud LLM actions at high end).

**Why this works for ORCHA**:
1. Base subscription covers PM/CRM value (seats)
2. Usage component captures AI orchestration value (actions)
3. APN mesh creates a cost advantage — local compute actions are cheaper than cloud, improving margins as the network grows
4. Free tier is generous enough for evaluation but constrained enough to drive conversion

**Unit economics target**: Cost of AI per action (CAPO) < $0.01 average, enabling healthy margins at $0.005-0.05 pricing.

---

## 5. Market Comparables

### Developer & AI Tool Valuations

| Company | Pricing | Key Metric | Valuation | Multiple |
|---------|---------|------------|-----------|----------|
| Cursor (Anysphere) | $20-200/user/mo | $1B ARR (2025) | $29.3B | ~29x ARR |
| Linear | $10-15/user/mo | Est. $20-40M ARR | $400M | ~10-20x ARR |
| Windsurf (Codeium) | $15-30/user/mo | Acquired by Cognition AI | $50-100M | N/A |
| CrewAI | $99/mo enterprise | OSS + managed | Est. $50-100M | Early stage |
| Plane | Self-hosted + cloud | OSS + paid | Est. $20-50M | Pre-revenue |
| LangGraph/LangSmith | Free OSS + paid monitoring | $25M+ ARR est. | $3B (LangChain) | ~100x+ ARR |

Sources: The Information (Cursor ARR, Feb 2025), TechCrunch (Linear valuation), Cognition AI acquisition reports (2025).

### AI Agent Market Size

| Year | Market Size | Growth |
|------|-------------|--------|
| 2025 | $7.8B | — |
| 2026 | $10.9B | 45% YoY |
| 2028 | $22.9B | 45% CAGR |
| 2030 | $52.6B | 45% CAGR |
| 2034 | $251.4B | 48% CAGR (accelerating) |

Source: MarketsandMarkets, "AI Agents Market" report (2024); Grand View Research, "Autonomous AI Agents" forecast (2025).

### MCP Ecosystem Traction

- **6,400+ registered MCP servers** as of Q1 2026
- Major enterprise adoption: Block, Bloomberg, Amazon, multiple Fortune 500 companies
- Full protocol standardization expected by end of 2026
- ORCHA's MCP-native architecture is a structural advantage — competitors must retrofit

Source: Anthropic MCP Registry, Block engineering blog (2025), Bloomberg Tech (2025).

### Relevant Insight: Linear's Capital Efficiency

Linear reached a $400M valuation with only **$35K in lifetime marketing spend** (per Karri Saarinen, Linear co-founder, 2024). This demonstrates that exceptional product quality in the project management space can drive organic growth with minimal marketing budget — a model aligned with PCG's resource constraints.

---

## 6. Fundraising Milestone Targets

### Pre-Seed: $250K-500K — Target Q3-Q4 2026

**Trigger milestones (all required)**:
- Working ORCHA demo with 3+ client deployments
- Platform dogfooded internally for 3+ months of PCG agency delivery
- Agent orchestration engine functional with measurable success rates
- Unit economics validated: CAPO < $0.01 per agent action
- 2-3 paying pilot clients (even at reduced/introductory rates)

**Use of funds**:

| Category | Monthly | 12-Month Total |
|----------|---------|----------------|
| Salaries (1 additional engineer) | $8-12K | $96-144K |
| Founder stipends | $5-10K | $60-120K |
| Infrastructure (hosting, APIs) | $500-2K | $6-24K |
| AI API costs | $500-1K | $6-12K |
| Legal/incorporation | — | $10-20K |
| Buffer | — | $22-80K |
| **Total** | **$15-25K** | **$200-400K** |

**Target investors**: Angel investors, pre-seed funds (Pear VC, Hustle Fund, Precursor Ventures), AI-focused micro-funds, agency-background angels.

**Narrative**: "We built an AI orchestration platform, dogfooded it for our agency, and our clients want to buy it. We need capital to productize what's already working."

### Seed: $1M-3M — Target Q2-Q3 2027

**Trigger milestones**:
- $5-10K MRR from platform subscriptions (excluding agency services)
- 20+ active organizations on the platform
- Agent success rate > 85% across workflow types
- Workflow marketplace with 10+ reusable templates
- PostgreSQL migration complete (enterprise readiness)
- APN Phase 2 operational (distributed compute proof-of-concept)

**Use of funds**:

| Category | Monthly | 18-Month Total |
|----------|---------|----------------|
| Salaries (5-8 people) | $50-80K | $900K-1.44M |
| Infrastructure | $2-5K | $36-90K |
| AI compute costs | $2-10K | $36-180K |
| Marketing/content | $5-10K | $90-180K |
| Office/operations | $2-5K | $36-90K |
| Legal/compliance | $1-2K | $18-36K |
| **Total** | **$62-112K** | **$1.1-2.0M** |

**Target investors**: Seed-stage funds (Y Combinator, a16z Seed, First Round Capital), strategic investors from the agency/creative tools space.

### Series A: $5M-15M — Target Q1-Q2 2029

**Trigger milestones**:
- $50-100K MRR
- 200+ active organizations
- Net revenue retention > 120% (expansion revenue exceeds churn)
- Clear product-market fit signal (organic growth > 30% of new revenue)
- APN network with 50+ active nodes
- Full sovereignty stack operational (on-prem deployment option)

**Use of funds**:
- Scale team to 15-25 people (engineering, sales, customer success)
- Build go-to-market engine (content, partnerships, conference presence)
- International expansion (EU data sovereignty market)
- APN incentive program to bootstrap network effects
- Enterprise compliance certifications (SOC 2, ISO 27001)

---

## 7. Burn Rate Analysis

### Current State (2-3 Person Team, Bootstrapped)

| Category | Low | High | Notes |
|----------|-----|------|-------|
| Founder salaries | $0 | $8,000 | Deferred or reduced |
| Infrastructure | $200 | $500 | Cloudflare, domain, basic hosting |
| AI API costs | $100 | $500 | Development and testing usage |
| Tools/subscriptions | $100 | $300 | GitHub, design tools, etc. |
| **Total monthly burn** | **$400** | **$9,300** | |

**Runway at current burn**: Effectively indefinite if founders are taking minimal salary and agency revenue covers costs. This is the key advantage of the hybrid model.

### Post-Pre-Seed (3-4 People)

| Category | Low | High |
|----------|-----|------|
| Salaries | $15,000 | $25,000 |
| Infrastructure | $500 | $2,000 |
| AI API costs | $500 | $2,000 |
| Marketing | $500 | $1,000 |
| Legal/accounting | $500 | $500 |
| **Total monthly burn** | **$17,000** | **$30,500** |

**Runway calculation**:
- At $250K raise: 8-15 months
- At $500K raise: 16-29 months
- With $10K/month agency revenue offset: extends runway by 33-59%

### Post-Seed (5-8 People)

| Category | Low | High |
|----------|-----|------|
| Salaries | $50,000 | $80,000 |
| Infrastructure | $2,000 | $5,000 |
| AI compute costs | $2,000 | $10,000 |
| Marketing/sales | $5,000 | $10,000 |
| Office/operations | $2,000 | $5,000 |
| **Total monthly burn** | **$61,000** | **$110,000** |

**Runway calculation**:
- At $1M raise: 9-16 months
- At $3M raise: 27-49 months
- With $30K/month platform revenue offset: extends runway by 27-49%

### Key Insight: Revenue-Offset Runway

The hybrid model's greatest financial advantage is **revenue-offset burn**. A bootstrapped startup with $20K/month agency revenue and $17K/month burn is effectively cash-flow positive before raising a single dollar. This transforms fundraising from a survival necessity into a growth accelerator.

---

## 8. Revenue Projections by Stage

### Conservative Scenario (Agency-Funded Bootstrap)

Assumes: No external funding, slow organic growth, cautious client acquisition.

| Period | Monthly Revenue | Cumulative ARR | Primary Source |
|--------|----------------|----------------|----------------|
| Q2 2026 | $5-10K | $60-120K | PCG agency services |
| Q3 2026 | $8-15K | $96-180K | Agency + 1-2 pilot platform clients |
| Q4 2026 | $10-20K | $120-240K | Agency + 3-5 pilots |
| Q1 2027 | $15-25K | $180-300K | Agency + early SaaS conversions |
| Q2-Q4 2027 | $20-40K | $240-480K | Growing SaaS + agency |
| 2028 | $40-80K | $480K-960K | SaaS primary, agency secondary |
| 2029 | $80-150K | $960K-1.8M | SaaS dominant |

**5-year outcome**: $1-2M ARR, profitable, no dilution. Valued at $10-40M (10-20x ARR).

### Moderate Scenario (Pre-Seed Funded)

Assumes: $250-500K pre-seed in Q3 2026, seed round in Q2 2027, steady execution.

| Period | Monthly Revenue | Cumulative ARR | Primary Source |
|--------|----------------|----------------|----------------|
| Q2 2026 | $5-10K | $60-120K | Agency services |
| Q3 2026 | $10-20K | $120-240K | Agency + pilots + pre-seed capital |
| Q4 2026 | $15-30K | $180-360K | Agency + 5-10 pilot clients |
| Q1-Q2 2027 | $25-50K | $300-600K | Early SaaS launch |
| Q3-Q4 2027 | $50-100K | $600K-1.2M | SaaS growth, seed-funded |
| 2028 | $100-200K | $1.2-2.4M | Scaling with seed capital |
| 2029 | $200-500K | $2.4-6M | Series A trajectory |

**5-year outcome**: $2.4-6M ARR, ~20-30% dilution, strong growth trajectory. Valued at $25-60M.

### Aggressive Scenario (Strong Product-Market Fit)

Assumes: Rapid organic adoption, Cursor-like growth dynamics, favorable market timing.

| Period | Monthly Revenue | Cumulative ARR | Primary Source |
|--------|----------------|----------------|----------------|
| Q3 2026 | $15-25K | $180-300K | Agency + 5 paying pilots |
| Q4 2026 | $30-50K | $360-600K | 10-15 paying organizations |
| Q1 2027 | $50-100K | $600K-1.2M | SaaS launch + seed round |
| Q2-Q4 2027 | $100-250K | $1.2-3M | Rapid organic adoption |
| 2028 | $250-500K | $3-6M | Category leadership emerging |
| 2029 | $500K-1M | $6-12M | Series A, market expansion |

**5-year outcome**: $6-12M ARR, ~35-45% dilution, category-defining position. Valued at $60-180M.

---

## 9. Competitive Positioning

### ORCHA's Structural Differentiators

| Differentiator | What It Means | Who It Beats |
|----------------|---------------|--------------|
| **Sovereign data platform** | All data stays on-premises or in customer-controlled infrastructure | Cursor, Linear, CrewAI (all cloud-dependent) |
| **Full-stack agency tool** | CRM + PM + AI orchestration + workflows in one platform | Point solutions (Linear for PM, HubSpot for CRM, CrewAI for agents) |
| **MCP-native architecture** | Built on Model Context Protocol from day one, not retrofitted | LangGraph, CrewAI (proprietary agent protocols) |
| **APN mesh network** | Decentralized compute reduces costs and increases resilience | All centralized cloud competitors |
| **Categorical architecture** | Mathematical foundation (Topos theory) enables formal composability | Ad-hoc integration approaches |

### Competitive Risk Matrix

| Risk | Severity | Probability | Mitigation |
|------|----------|-------------|------------|
| Cursor/Windsurf add PM features | High | Medium | Focus on agency workflow, not just code |
| Linear adds AI agent orchestration | High | Low | Linear's DNA is simplicity; orchestration is complex |
| CrewAI adds CRM/PM | Medium | Low | CrewAI is agent-only; full-stack is hard to bolt on |
| New well-funded entrant | Medium | Medium | First-mover in sovereign + full-stack + MCP niche |
| Open-source alternative emerges | Medium | High | Community + APN network effects create moat |

### Strategic Positioning Statement

ORCHA is **not** competing head-to-head with Cursor on code editing, Linear on task management, or HubSpot on CRM. ORCHA competes on the **integration layer** — the value of having AI agents that can read your CRM, update your tasks, execute your workflows, and deliver results through a sovereign compute mesh, all without data leaving your control.

The target buyer is a **technical agency leader** who currently uses 4-7 separate tools and wants:
1. Fewer tools, less context-switching
2. AI automation that understands their full business context
3. Confidence that client data stays sovereign
4. A platform that gets smarter as their team uses it

---

## 10. Recommended Strategy

### Phase 1 — Dogfood & Agency Revenue (Q2-Q3 2026)

**Objective**: Generate $10-20K/month through PCG agency delivery using ORCHA as the operating system for all client work.

**Actions**:
- Route all PCG proposals, research, content, and design work through ORCHA workflows
- Track time savings and quality improvements as case study data
- Build 3-5 reusable workflow templates from real agency engagements
- Identify 2-3 clients who would benefit from direct platform access
- Begin pre-seed investor conversations (warm intros, not formal raise)

**Success criteria**:
- ORCHA used daily for all PCG agency operations
- 3+ workflow templates validated through production use
- $10K+ monthly agency revenue
- 2+ clients expressing interest in platform access

### Phase 2 — Pilot Platform Sales (Q3-Q4 2026)

**Objective**: Convert 2-5 agency clients into managed ORCHA deployments at premium pricing.

**Actions**:
- Offer white-glove onboarding to select clients ($500-2K/month per org)
- Build minimal multi-tenant isolation (org-scoped data, separate API keys)
- Gather structured product feedback through weekly check-ins
- Validate unit economics (CAPO, support cost per org, infrastructure cost per org)
- Close pre-seed round ($250-500K) based on pilot traction

**Success criteria**:
- 3+ paying pilot organizations
- CAPO < $0.01 per agent action
- Pre-seed term sheet signed
- Agent success rate > 80%

### Phase 3 — SaaS Launch (Q1-Q2 2027)

**Objective**: Launch self-service signup with hybrid seat + usage billing, targeting $25-50K MRR.

**Actions**:
- Build billing infrastructure (Stripe integration, usage metering)
- Create self-service onboarding flow
- Launch content marketing (blog, case studies from pilot clients, workflow library)
- Open workflow marketplace for community contributions
- Begin seed round conversations

**Success criteria**:
- 20+ active organizations
- $25K+ MRR from platform subscriptions
- Self-service conversion rate > 5% (free → paid)
- Net promoter score > 50

### Phase 4 — Scale (Q3 2027 - Q4 2028)

**Objective**: Grow to $100K+ MRR with seed funding, build enterprise readiness.

**Actions**:
- Hire 3-5 additional team members (engineering, customer success, marketing)
- Ship enterprise features: SSO, audit logs, compliance certifications, role-based access
- Launch APN Phase 2 with node incentive program
- Expand workflow marketplace to 50+ templates
- Build partner channel (agency referrals, technology integrations)

**Success criteria**:
- $100K+ MRR
- 200+ active organizations
- Net revenue retention > 120%
- APN network with 20+ nodes

### Phase 5 — Growth (2029+)

**Objective**: Raise Series A, build go-to-market engine, expand internationally.

**Actions**:
- Raise $5-15M Series A
- Scale team to 15-25 people
- Enter EU market (data sovereignty regulations create natural demand)
- Launch enterprise sales motion
- Expand APN to 50+ nodes with formalized incentive economics

**Success criteria**:
- $500K+ MRR
- 1,000+ active organizations
- APN network self-sustaining
- Category recognition (analyst reports, conference keynotes)

---

## Appendix A: Key Assumptions

1. **AI agent market grows at 45%+ CAGR through 2030** — supported by MarketsandMarkets and Grand View Research forecasts.
2. **MCP becomes the dominant agent protocol** — based on current adoption trajectory (6,400+ servers, Fortune 500 adoption).
3. **Data sovereignty regulations increase** — EU AI Act, state-level US privacy laws create demand for on-premises AI.
4. **PCG maintains 2-3 active agency clients** — provides baseline revenue of $5-15K/month.
5. **APN mesh compute is 30-60% cheaper than cloud** — based on distributed compute economics (no data center overhead).

## Appendix B: Valuation Benchmarks

| Stage | Revenue Multiple | ORCHA Implied Valuation (Moderate) |
|-------|------------------|------------------------------------|
| Pre-seed | 50-100x MRR | $500K-2M |
| Seed | 30-50x MRR | $3-10M |
| Series A | 20-30x ARR | $25-60M |
| Series B+ | 15-25x ARR | $50-150M |

Note: AI-native companies currently command premium multiples. Cursor's 29x ARR multiple and LangChain's 100x+ ARR multiple suggest the market rewards AI infrastructure plays. ORCHA's sovereign + full-stack positioning could command multiples at the higher end of these ranges.

## Appendix C: Fundraising Timeline Decision Tree

```
Q2 2026: Agency revenue > $10K/month?
  ├─ YES → Begin pre-seed conversations
  │         Q3 2026: 3+ pilot clients paying?
  │           ├─ YES → Close pre-seed ($250-500K)
  │           └─ NO  → Continue agency model, re-evaluate Q4
  └─ NO  → Focus on agency pipeline, defer fundraising

Q1 2027: Platform MRR > $15K?
  ├─ YES → Begin seed conversations
  │         Q2-Q3 2027: 20+ orgs, >$30K MRR?
  │           ├─ YES → Close seed ($1-3M)
  │           └─ NO  → Extend runway with agency revenue, re-evaluate Q4
  └─ NO  → Evaluate product-market fit; consider pivot or niche focus

Q4 2028: MRR > $80K, NRR > 120%?
  ├─ YES → Begin Series A process
  └─ NO  → Profitable at current scale?
              ├─ YES → Continue bootstrapped growth
              └─ NO  → Strategic options (acqui-hire, merge, wind down)
```
