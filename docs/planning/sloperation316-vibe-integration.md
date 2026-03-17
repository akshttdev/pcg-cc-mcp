# VIBE Unified Tokenomics — Sovereign Stack Architecture & Evolution Plan

**Branch:** `sloperation316-vibe-integration`
**Created:** 2026-03-17
**Status:** Planning

---

## Overview

Unified billing hierarchy across Users, Organizations, Projects, and Agents so every LLM call is charged at 2x market cost against a proper wallet. Current state only charges at project/agent level — org and user wallets are disconnected. This plan defines the schema changes, service layer, route wiring, and dashboard UI needed for a complete and seamless integration.

---

## Current State

```
LLM Call
   │
   ▼
[nora.rs / agent_chat.rs / topsi.rs / twilio.rs]
   │
   ▼
VibePricingService::record_llm_usage()
   │
   ├──→ vibe_transactions (source_type: "project" | "agent")
   │         ✅ 216 rows, works
   │
   └──→ Project::adjust_vibe_spent()
             ✅ updates projects.vibe_spent_amount

DISCONNECTED:
• users.vibe_balance ──── ✗ never charged, no spend history
• organizations ────────── ✗ no VIBE columns at all
• token_usage table ────── ✗ table exists but is empty
• PCG Router ───────────── ✗ estimates cost but doesn't bill
• Budget gate ──────────── ✅ exists but only project-scoped
```

---

## Target Architecture — The 4-Tier Wallet Hierarchy

```
┌──────────────────────────────────────────────────────────────────────────┐
│                    TARGET STATE — Unified Tokenomics                      │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                           │
│  ┌───────────────────────────────────────────────────────────────┐       │
│  │                   ORGANIZATION WALLET (Tier 1)                │       │
│  │  vibe_balance  vibe_spent  vibe_deposited  vibe_withdrawn      │       │
│  │  budget_limit  (top-up via faucet / on-chain deposit)         │       │
│  └───────────────────────┬───────────────────────────────────────┘       │
│                           │ allocates to                                  │
│           ┌───────────────┼───────────────┐                              │
│           │               │               │                              │
│           ▼               ▼               ▼                              │
│  ┌────────────┐   ┌───────────┐   ┌──────────────────────┐             │
│  │  USER      │   │  PROJECT  │   │  AGENT WALLET        │             │
│  │  WALLET    │   │  WALLET   │   │  (Tier 3)            │             │
│  │ (Tier 2)   │   │ (Tier 2)  │   │  ✅ already has:     │             │
│  │            │   │           │   │  vibe_budget_limit   │             │
│  │vibe_bal    │   │vibe_budget│   │  vibe_spent_amount   │             │
│  │vibe_spent  │   │vibe_spent │   └──────────────────────┘             │
│  │vibe_limit  │   │deposited  │                                          │
│  └────────────┘   │withdrawn  │                                          │
│                   └───────────┘                                          │
│                                                                           │
│  BILLING CASCADE (every LLM call):                                        │
│                                                                           │
│  PCG Router / Nora / Agent Chat / Twilio / Topsi                          │
│       │                                                                   │
│       ▼                                                                   │
│  UnifiedBillingService::charge(context)                                   │
│       │                                                                   │
│       ├─1st─→ Project wallet  (if project_id present & has balance)      │
│       │           └─ fallback ↓                                           │
│       ├─2nd─→ Org wallet      (parent org of project or user)            │
│       │           └─ fallback ↓                                           │
│       ├─3rd─→ User wallet     (if personal / no project context)         │
│       └─FAIL─→ HTTP 402 (no funds anywhere in cascade)                  │
│                                                                           │
│  Every charge writes:                                                     │
│    → vibe_transactions  (source_type: project|user|org|agent)            │
│    → token_usage        (granular tokens per task/agent/project)          │
│    → balance update on charged entity                                     │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## Schema Gap Analysis

| Entity | Has Now | Needs Added |
|---|---|---|
| `users` | `vibe_balance` (f64) | `vibe_spent_amount` (i64), `vibe_budget_limit` (i64?), `total_vibe_deposited` (i64) |
| `organizations` | nothing | `vibe_balance` (i64), `vibe_spent_amount` (i64), `vibe_budget_limit` (i64?), `total_vibe_deposited` (i64), `total_vibe_withdrawn` (i64) |
| `projects` | `vibe_budget_limit`, `vibe_spent_amount`, `total_vibe_deposited`, `total_vibe_withdrawn` | ✅ complete |
| `agent_wallets` | `vibe_budget_limit`, `vibe_spent_amount` | ✅ complete |
| `vibe_transactions` | `source_type`: agent\|project | add `user` and `org` variants; add `billed_org_id`, `billed_user_id` columns for cascade tracing |
| `token_usage` | table exists, 0 rows | wire to all LLM callers via UnifiedBillingService |

---

## Phase 1 — Schema Unification

**Migration:** `20260317100000_unified_vibe_wallets.sql`

```sql
-- Extend users with spending tracking
-- (vibe_balance already exists as REAL; add INTEGER counterparts)
ALTER TABLE users ADD COLUMN vibe_spent_amount INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN vibe_budget_limit INTEGER;       -- NULL = unlimited
ALTER TABLE users ADD COLUMN total_vibe_deposited INTEGER NOT NULL DEFAULT 0;

-- Add full wallet to organizations
ALTER TABLE organizations ADD COLUMN vibe_balance INTEGER NOT NULL DEFAULT 0;
ALTER TABLE organizations ADD COLUMN vibe_spent_amount INTEGER NOT NULL DEFAULT 0;
ALTER TABLE organizations ADD COLUMN vibe_budget_limit INTEGER;
ALTER TABLE organizations ADD COLUMN total_vibe_deposited INTEGER NOT NULL DEFAULT 0;
ALTER TABLE organizations ADD COLUMN total_vibe_withdrawn INTEGER NOT NULL DEFAULT 0;

-- Add cascade tracing to vibe_transactions
-- source_type: "project" | "agent" | "user" | "org"
ALTER TABLE vibe_transactions ADD COLUMN billed_org_id BLOB;
ALTER TABLE vibe_transactions ADD COLUMN billed_user_id BLOB;
```

**Rust model changes required:**
- `Organization`: add wallet fields + `adjust_vibe_spent()`, `adjust_vibe_balance()` methods
- `User`: add `vibe_spent_amount`, `vibe_budget_limit`, `total_vibe_deposited` + `adjust_vibe_spent()` method
- `VibeSourceType` enum: add `User` and `Organization` variants

---

## Phase 2 — UnifiedBillingService

**New file:** `crates/services/src/services/unified_billing.rs`

```rust
pub struct BillingContext {
    pub project_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub task_id: Option<Uuid>,
    pub task_attempt_id: Option<Uuid>,
    pub operation_type: String,  // "chat" | "voice" | "agent_task" | "research"
}

pub struct VibeCharge {
    pub billed_entity_type: String,  // "project" | "org" | "user" | "agent"
    pub billed_entity_id: Uuid,
    pub amount_vibe: i64,
    pub cost_usd: f64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub transaction_id: Uuid,
}

pub struct WalletBalance {
    pub entity_type: String,
    pub entity_id: Uuid,
    pub available_vibe: i64,
    pub spent_vibe: i64,
    pub deposited_vibe: i64,
    pub budget_limit: Option<i64>,
    pub available_usd: f64,
    pub spent_usd: f64,
}

impl UnifiedBillingService {
    // Main entry point — replaces all direct VibePricingService calls
    pub async fn charge(&self, ctx: BillingContext) -> Result<VibeCharge>

    // Pre-flight check before executing expensive calls
    pub async fn pre_check(&self, ctx: &BillingContext) -> Result<PreflightResult>

    // Funding: credit VIBE to any entity (admin/faucet use)
    pub async fn fund_org(org_id, amount_vibe, description) -> Result<()>
    pub async fn fund_user(user_id, amount_vibe, description) -> Result<()>
    pub async fn fund_project(project_id, amount_vibe, description) -> Result<()>

    // Balance queries
    pub async fn org_balance(org_id) -> Result<WalletBalance>
    pub async fn user_balance(user_id) -> Result<WalletBalance>
    pub async fn project_balance(project_id) -> Result<WalletBalance>
}
```

**Cascade logic:**
```
charge(ctx):
    cost = model_pricing.calculate(model, tokens)  // always 2x market

    if ctx.project_id AND project.available_balance >= cost:
        bill → project
    else if ctx.org_id AND org.vibe_balance >= cost:
        bill → org
    else if ctx.user_id AND user.vibe_balance >= cost:
        bill → user
    else:
        return Err(InsufficientFunds)  // → HTTP 402

    write vibe_transactions (source_type = chosen entity)
    write token_usage       (always, even if billing fails for dev mode)
    update balance on charged entity
```

---

## Phase 3 — Wire All LLM Callers

Replace `VibePricingService::record_llm_usage()` with `UnifiedBillingService::charge()` in:

| File | Status |
|---|---|
| `crates/server/src/routes/nora.rs` | upgrade from VibePricingService |
| `crates/server/src/routes/topsi.rs` | upgrade from VibePricingService |
| `crates/server/src/routes/agent_chat.rs` | upgrade from VibePricingService |
| `crates/server/src/routes/twilio.rs` | upgrade from VibePricingService |
| `crates/server/src/routes/pcg_router.rs` | **NOT WIRED** — add after `route_completion()` |

**PCG Router integration pattern:**
```rust
// After route_completion() returns a successful response:
let billing_ctx = BillingContext {
    project_id: headers.get("X-Project-Id").and_then(|v| Uuid::parse_str(v).ok()),
    org_id: jwt_claims.org_id,
    user_id: jwt_claims.user_id,
    model: metadata.model_used.clone(),
    input_tokens: metadata.input_tokens,
    output_tokens: metadata.output_tokens,
    operation_type: "chat".to_string(),
    ..Default::default()
};
// Non-blocking — don't slow the response
tokio::spawn(async move { billing.charge(billing_ctx).await });
```

---

## Phase 4 — Token Usage Dual-Write

Every `UnifiedBillingService::charge()` call writes BOTH `vibe_transactions` AND `token_usage`:

```rust
// Always write granular token record (even in dev/free mode)
TokenUsage::create(pool, TokenUsageCreate {
    task_attempt_id: ctx.task_attempt_id,
    agent_id: ctx.agent_id,
    project_id: ctx.project_id.unwrap_or(org_default_project),
    model: ctx.model.clone(),
    provider: ModelPricing::infer_provider(&ctx.model),
    input_tokens: ctx.input_tokens,
    output_tokens: ctx.output_tokens,
    total_tokens: ctx.input_tokens + ctx.output_tokens,
    cost_cents: Some(cost_cents),
    operation_type: Some(ctx.operation_type.clone()),
    metadata: None,
}).await?;
```

This populates the `/api/token-usage/*` analytics endpoints which currently return empty data.

---

## Phase 5 — Admin Funding Endpoints

Extend faucet / deposit APIs to fund orgs and users (not just projects):

```
POST /api/vibe/fund                   (X-Admin-Key required)
  Body: { entity_type: "org"|"user"|"project", entity_id: UUID, amount_vibe: i64, description: String }

GET  /api/vibe/wallets/org/{org_id}   → WalletBalance
GET  /api/vibe/wallets/user/{user_id} → WalletBalance
GET  /api/vibe/wallets/project/{id}   → WalletBalance  (already exists via vibe_treasury.rs)
```

---

## Phase 6 — Dashboard UI

### 6a. Command Center Widget — VIBE Economy

```
┌──────────────────────────────────────────────────┐
│  VIBE Economy              Today: 892 ◈  $8.92   │
├──────────────────────────────────────────────────┤
│  Org Balance: 9,108 ◈  ($91.08)                  │
│  ████████████████████░░░░░  91% remaining         │
│                                                   │
│  Today's Usage                                    │
│  ┌──────────────────────────────────────────┐    │
│  │ Nora Chat      412 ◈   Claude Sonnet 4.6 │    │
│  │ Agent Tasks    280 ◈   Claude Opus 4.6   │    │
│  │ Twilio Voice   114 ◈   ElevenLabs        │    │
│  │ Research        86 ◈   Claude Haiku      │    │
│  └──────────────────────────────────────────┘    │
│                                                   │
│  7-Day Trend  ▁▂▃▅▆▄█░░                          │
│  [Top Up Org]  [View History]                     │
└──────────────────────────────────────────────────┘
```

### 6b. Per-Project Wallet Tab

Add "Wallet" tab to project detail pages:

```
[Overview] [Deliverables] [Media] [Knowledge] [Wallet]

Project Wallet
  Balance:   2,340 ◈  ($23.40)
  Spent:     1,660 ◈  ($16.60)   ← this billing period
  Limit:     4,000 ◈  (no limit if blank)

  Recent Transactions
  ├ -42 ◈  Nora Chat — claude-sonnet  2h ago
  ├ -180 ◈ Research Pass — claude-opus  5h ago
  ├ +1000 ◈ Admin Top-up              yesterday
  └ -38 ◈  Agent Task                 yesterday

  [Top Up Project]  [Set Limit]
```

### 6c. User Wallet (Profile / Settings)

```
Your VIBE Wallet
  Balance:  500 ◈  ($5.00)
  Spent:    100 ◈  ($1.00)    ← all time
  Limit:    Unlimited

  [Add Funds]  [Transaction History]
```

### 6d. Org Admin Wallet Page (`/settings/wallet`)

```
Organization Wallet — Sirak Studios

  Balance:       9,108 ◈  ($91.08)
  Total Funded: 21,000 ◈
  Total Spent:  11,892 ◈

  ┌─── Spending by Project ────────────────────────┐
  │  Brand Research        3,200 ◈  26.9%          │
  │  Deliverables AI       2,800 ◈  23.5%          │
  │  Nora (Unprovisioned)  2,100 ◈  17.6%          │
  │  Other                 3,792 ◈  31.9%          │
  └────────────────────────────────────────────────┘

  ┌─── Top-up History ─────────────────────────────┐
  │  +10,000 ◈  Admin Seed    2026-03-11            │
  │  +10,000 ◈  Admin Seed    2026-03-05            │
  │  +1,000 ◈   Faucet test   2026-03-03            │
  └────────────────────────────────────────────────┘

  [Fund Org Wallet]  [Download CSV]  [Set Budget Limit]
```

---

## Phase 7 — On-Chain Settlement (Future)

All 216 existing `vibe_transactions` have `aptos_tx_status = 'pending'` and `on_chain_synced = 0`. Phase 7 closes the loop:

```
Plan:
  • Batch settlement job: runs every 24h via existing automations.rs pattern
  • Aggregates pending vibe_transactions per org/project
  • Submits single Aptos transaction per entity per day (not per API call)
  • Updates on_chain_synced = 1 and aptos_tx_hash on each batch member
  • Revenue flows to PLATFORM_REVENUE_ADDRESS on-chain
  • Marketplace split: 85% provider / 15% platform (already exists in marketplace logic)
```

---

## Implementation Priority

| Priority | Phase | Effort | Impact |
|---|---|---|---|
| **P0** | Phase 1 — Schema migration | ~1h | Unlocks everything |
| **P0** | Phase 2 — UnifiedBillingService | ~3h | Core billing logic |
| **P1** | Phase 3 — Wire all callers | ~2h | Actual charging begins |
| **P1** | Phase 4 — Token usage dual-write | ~1h | Analytics populated |
| **P1** | Phase 5 — Admin funding endpoints | ~1h | Fund orgs/users |
| **P2** | Phase 6a-b — Command Center + Project wallet UI | ~3h | Dashboard visibility |
| **P2** | Phase 6c-d — User + Org wallet UI | ~3h | Full self-service |
| **P3** | Phase 7 — On-chain settlement | TBD | Blockchain finality |

---

## Key Invariants (Non-Negotiable)

1. **2x markup always** — All LLM costs charged at 2x market rate via `model_pricing` table. Never hardcode or bypass.
2. **Dual-write always** — Every LLM call writes BOTH `vibe_transactions` AND `token_usage`. No exceptions.
3. **Cascade order** — project → org → user → HTTP 402. Never skip a tier.
4. **Non-blocking billing** — Always `tokio::spawn` billing after returning HTTP response. Never block the user.
5. **Idempotency** — `task_attempt_id` on `vibe_transactions` prevents double-billing on retries.
6. **Balance is derived** — `available = deposited − withdrawn − spent`. Running counters (`vibe_spent_amount`) are denormalized for performance only — treat `vibe_transactions` as the source of truth.

---

## Current Live Data (2026-03-17)

| Metric | Value |
|---|---|
| Total vibe_transactions | 216 |
| Total VIBE spent | ~21,000 ◈ (~$210) |
| Top model | claude-sonnet-4-20250514 (197 calls, $88.72) |
| PCG Router models registered | 31 (5 enabled) |
| token_usage rows | 0 (not yet wired) |
| Orgs with VIBE wallet | 0 (not yet built) |
| Users actively billed | 0 (not yet built) |
