# PCG Router & External Services Audit — Report + Fix Plan

**Date**: 2026-04-22
**Request**: Audit all API usage to ensure routing through PCG Router and service facades
**Scope**: All external HTTP calls (LLM + non-LLM services)
**Action**: Report + fix plan
**Branch**: TBD
**Worktree**: `/Users/madhav/development/GitHub/pcg-cc-mcp`

---

## Executive Summary

**LLM Calls**: 12+ locations bypass PCG Router, calling Anthropic/OpenAI directly
**Non-LLM Services**: Well-structured facades exist, but 3 critical issues found
**Recommendation**: Consolidate LLM calls through WorkflowLLMService; fix security issues in external services

---

## Part 1: LLM Provider Audit

### Current State

| Component | Uses PCG Router? | Direct Calls | Impact |
|-----------|-----------------|--------------|--------|
| WorkflowLLMService | Partial (DB config) | Yes | Medium |
| Nora Brain Providers | No | Yes | High |
| Server Routes (brand, intake, CRM) | No | Yes | High |
| TopicClips | No | Yes | Medium |
| Voice Services (STT/TTS) | N/A | Yes | Low |

### Direct Anthropic Calls (Bypassing PCG Router)

| File | Line | Feature |
|------|------|---------|
| `crates/nora/src/brain/providers/anthropic.rs` | 40 | Nora brain - direct `api.anthropic.com` |
| `crates/server/src/routes/organizations/brand.rs` | ~350+ | Brand research (2 calls) |
| `crates/server/src/routes/crm_deal_automations.rs` | 768 | Deal automation |
| `crates/server/src/routes/intake/report.rs` | 61, 164, 264, 739 | Intake reports (4 calls) |
| `crates/topiclips/src/lib.rs` | Multiple | TopicClips generation |
| `crates/topiclips/src/artistic_interpreter.rs` | Multiple | Artistic interpretation |

### Direct OpenAI Calls (Bypassing PCG Router)

| File | Line | Feature |
|------|------|---------|
| `crates/nora/src/brain/providers/openai.rs` | 37 | Nora brain - direct `api.openai.com` |
| `crates/nora/src/brain/mod.rs` | 335 | LLMClient fallback |
| `crates/nora/src/execution/research.rs` | Multiple | Research agents |
| `crates/nora/src/voice/stt.rs` | 99 | Whisper STT |
| `crates/server/src/routes/crm_deal_automations.rs` | 721 | Deal automation fallback |

### What WorkflowLLMService Does Right

Located at `crates/services/src/services/workflow_llm.rs`:
- ✅ Database-driven provider selection via `PcgRouterModel`
- ✅ Automatic fallback on failure
- ✅ Cost tracking per request
- ✅ Supports all providers: Anthropic, OpenAI, Gemini, xAI, Mistral, Groq, etc.

**However**: It still makes direct HTTP calls to providers, not through the PCG Router HTTP endpoint.

---

## Part 2: Non-LLM External Services Audit

### Services with Good Facades ✅

| Service | File | Pattern | Retry | Error Handling |
|---------|------|---------|-------|----------------|
| Airtable | `services/airtable_service.rs` | Excellent | ✅ backon | ✅ Stratified |
| GitHub | `services/github_service.rs` | Excellent | ✅ backon | ✅ Stratified |
| Twitter | `services/social/connectors/twitter.rs` | Good | ❌ | ✅ |
| LinkedIn | `services/social/connectors/linkedin.rs` | Good | ❌ | ✅ |
| TikTok | `services/social/connectors/tiktok.rs` | Good | ❌ | ✅ |
| Threads | `services/social/connectors/threads.rs` | Good | ❌ | ✅ |
| Artlist | `services/editron/artlist.rs` | Excellent | ✅ backon | ✅ |
| Epidemic Sound | `services/editron/epidemic.rs` | Excellent | ✅ backon | ✅ |
| Soundstripe | `services/editron/soundstripe.rs` | Good | ✅ backon | ✅ |

### Critical Issues Found 🔴

#### Issue 1: Instagram Credentials in URL (Security)
**File**: `crates/services/src/services/social/connectors/instagram.rs:143-149`
**Problem**: `client_secret` passed in query parameter instead of header
```rust
// CURRENT (INSECURE):
let url = format!("{}?client_id={}&client_secret={}&redirect_uri={}&code={}",
    META_TOKEN_URL, &self.client_id, &self.client_secret, ...);
```
**Fix**: Use POST with header-based auth or form body

#### Issue 2: HeyGen Creates Client Per Request (Performance)
**File**: `crates/video_gen/src/heygen.rs`
**Problem**: `reqwest::Client::new()` called for every API request
**Fix**: Create `HeyGenClient` struct with shared client instance

#### Issue 3: ElevenLabs Creates Client Per Request (Performance)
**File**: `crates/video_gen/src/tts.rs`
**Problem**: Same as HeyGen - no client reuse
**Fix**: Create `ElevenLabsClient` struct with shared client instance

### Services Missing Proper Facades ⚠️

| Service | Location | Issue |
|---------|----------|-------|
| HeyGen | `video_gen/heygen.rs` | No facade, client per request |
| ElevenLabs TTS | `video_gen/tts.rs` | No facade, client per request |
| Exa Search | `nora/execution/research.rs` | Inline HTTP calls |

---

## Part 3: Fix Plan

### Phase 1: Security Fixes (Priority: Critical)

**1.1 Fix Instagram Credentials Exposure**
- File: `crates/services/src/services/social/connectors/instagram.rs`
- Change: Move `client_secret` from URL to request body or header
- Effort: 1 hour
- Test: OAuth flow still works

### Phase 2: LLM Routing Consolidation (Priority: High)

**2.1 Route Nora Brain Through WorkflowLLMService**

Option A (Recommended): Modify `LLMClient` to use `WorkflowLLMService`
- Files to modify:
  - `crates/nora/src/brain/mod.rs` - Change `create_client_for_agent()`
  - `crates/nora/src/brain/providers/anthropic.rs` - Deprecate direct calls
  - `crates/nora/src/brain/providers/openai.rs` - Deprecate direct calls
- Effort: 4-6 hours
- Benefit: Unified cost tracking, provider switching via DB

Option B: Keep brain providers, add cost tracking hooks
- Less invasive but doesn't consolidate routing
- Effort: 2-3 hours

**2.2 Migrate Direct Route Calls to WorkflowLLMService**

| File | Calls | Effort |
|------|-------|--------|
| `routes/organizations/brand.rs` | 2 | 1 hour |
| `routes/crm_deal_automations.rs` | 2 | 1 hour |
| `routes/intake/report.rs` | 4 | 2 hours |
| `topiclips/lib.rs` | Multiple | 2 hours |
| `topiclips/artistic_interpreter.rs` | Multiple | 1 hour |

**2.3 Create WorkflowLLMService Wrapper for Non-Chat LLM Calls**
- STT (Whisper) - Keep direct (specialized endpoint)
- TTS (ElevenLabs) - Keep direct (specialized endpoint)
- Exa Search - Keep direct (not LLM)

### Phase 3: Service Facade Improvements (Priority: Medium)

**3.1 Create HeyGenClient Facade**
```rust
// New file: crates/video_gen/src/heygen_client.rs
pub struct HeyGenClient {
    client: reqwest::Client,
    api_key: String,
}

impl HeyGenClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: api_key.to_string(),
        }
    }

    pub async fn upload_audio(&self, ...) -> Result<String, HeyGenError> { ... }
    pub async fn generate_video(&self, ...) -> Result<String, HeyGenError> { ... }
    pub async fn poll_status(&self, ...) -> Result<VideoStatus, HeyGenError> { ... }
}
```
- Effort: 2 hours

**3.2 Create ElevenLabsClient Facade**
- Similar pattern to HeyGenClient
- Effort: 2 hours

**3.3 Add Retry Logic to Social Connectors**
- Add `backon` retry wrapper to Twitter, LinkedIn, TikTok, Threads, Instagram
- Use Airtable pattern as template
- Effort: 3 hours

### Phase 4: Observability (Priority: Low)

**4.1 Add Request Logging Middleware**
- Log all external HTTP calls with duration, status, provider
- Use `tracing` spans for correlation

**4.2 Add Cost Tracking to All LLM Calls**
- Ensure WorkflowLLMService tracks costs
- Add dashboard for cost monitoring

---

## Implementation Order

| Phase | Task | Priority | Effort | Dependencies |
|-------|------|----------|--------|--------------|
| 1.1 | Fix Instagram credentials | Critical | 1h | None |
| 2.1 | Route Nora brain via WorkflowLLM | High | 4-6h | None |
| 2.2 | Migrate route direct calls | High | 7h | 2.1 |
| 3.1 | HeyGen facade | Medium | 2h | None |
| 3.2 | ElevenLabs facade | Medium | 2h | None |
| 3.3 | Social connector retries | Medium | 3h | None |
| 4.1 | Request logging | Low | 2h | None |
| 4.2 | Cost tracking dashboard | Low | 4h | 2.1, 2.2 |

**Total Effort**: ~25-30 hours

---

## Files to Modify

### Critical (Security)
- `crates/services/src/services/social/connectors/instagram.rs`

### High Priority (LLM Routing)
- `crates/nora/src/brain/mod.rs`
- `crates/nora/src/brain/providers/anthropic.rs`
- `crates/nora/src/brain/providers/openai.rs`
- `crates/server/src/routes/organizations/brand.rs`
- `crates/server/src/routes/crm_deal_automations.rs`
- `crates/server/src/routes/intake/report.rs`
- `crates/topiclips/src/lib.rs`
- `crates/topiclips/src/artistic_interpreter.rs`

### Medium Priority (Service Facades)
- `crates/video_gen/src/heygen.rs` → create `heygen_client.rs`
- `crates/video_gen/src/tts.rs` → create `elevenlabs_client.rs`
- `crates/services/src/services/social/connectors/*.rs` (add retry)

---

## Reusable Patterns (Templates)

### HTTP Client with Retry (from Airtable)
```rust
(|| async { self.internal_method().await })
    .retry(&ExponentialBuilder::default()
        .with_min_delay(Duration::from_secs(1))
        .with_max_delay(Duration::from_secs(30))
        .with_max_times(3)
        .with_jitter())
    .when(|e| e.should_retry())
    .notify(|err, dur| tracing::warn!("Retrying after {:.2}s: {}", dur.as_secs_f64(), err))
    .await
```

### Error Type with Retry Intelligence
```rust
impl ServiceError {
    pub fn should_retry(&self) -> bool {
        matches!(self, Self::RateLimited | Self::NetworkError(_))
    }
}
```

### Service Facade Structure
```rust
pub struct ExternalClient {
    client: reqwest::Client,
    config: ClientConfig,
}

impl ExternalClient {
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;
        Ok(Self { client, config })
    }
}
```

---

## Verification

1. **Security**: Run Instagram OAuth flow, verify credentials not in URL
2. **LLM Routing**: Check `pcg_router_usage` logs show all calls
3. **Performance**: Measure connection reuse in HeyGen/ElevenLabs
4. **Cost Tracking**: Verify all LLM calls appear in cost dashboard
5. **Retries**: Simulate 429 responses, verify exponential backoff

---

## Notes

- Voice services (STT/TTS) use specialized endpoints - keep direct calls
- Exa Search is not an LLM - keep direct calls
- PCG Router HTTP endpoint exists but is underutilized - consider deprecating in favor of WorkflowLLMService direct integration

---

## Implementation Status (2026-05-19)

### Completed

1. **PCGRouterVoiceEngine** (`crates/nora/src/voice/pcg_router_engine.rs`)
   - New voice engine that routes TTS/STT through `WorkflowTTSService` and `WorkflowSTTService`
   - Provides database-driven provider selection, automatic fallback, and cost tracking
   - Exported as `nora::voice::PCGRouterVoiceEngine`

2. **Database Migration** (`crates/db/migrations/20260519000000_pcg_router_voice_providers.sql`)
   - Created `pcg_router_tts_providers` table with ElevenLabs, OpenAI, Azure, Chatterbox
   - Created `pcg_router_stt_providers` table with Whisper, LocalWhisper, Azure
   - Added priority-based indexing for provider selection

3. **LLMClient Routing Verification**
   - Confirmed that `LLMClient::with_pool()` properly routes through `PcgRouterAdapter`
   - All public methods (`generate`, `generate_stream`, `generate_with_tools_and_history`) check `custom_provider` first
   - Direct `generate_anthropic*` methods are correctly used only as fallback when no DB pool is provided

4. **Deprecation Notices**
   - Added `#[deprecated]` to `ElevenLabsTTS`, `AzureTTS`, `OpenAITTS` in `nora/voice/tts.rs`
   - Added `#[deprecated]` to `WhisperSTT`, `AzureSTT` in `nora/voice/stt.rs`
   - Added documentation note to `VoiceEngine` recommending `PCGRouterVoiceEngine`

5. **Voice Engine Migration** (completed 2026-05-20)
   - Created `UnifiedVoiceEngine` enum that wraps both legacy and router engines
   - Updated `NoraAgent` to use `UnifiedVoiceEngine` (auto-upgrades in `with_database()`)
   - Updated Topsi voice routes to use `get_pcg_router_voice_engine(pool)`
   - Updated Topsi meeting routes to use PCG Router voice engine
   - Updated Nora voice routes to use `UnifiedVoiceEngine`
   - Deprecated `get_or_init_voice_engine()` in favor of `get_pcg_router_voice_engine()`

### Remaining Work

- **Enable Providers**: TTS/STT providers seeded as disabled by default; enable via admin UI
- **HeyGen/ElevenLabs Facades**: Create proper client facades with connection reuse
- **Social Connector Retries**: Add retry logic to Twitter, LinkedIn, TikTok, Threads
