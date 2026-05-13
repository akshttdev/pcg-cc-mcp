# Topsi Backend Verification Report

Date: 2026-05-07
Source: Explore agent run against /Users/sirakstudios/topos/pcg/github/pcg-cc-mcp
Companion to: 2026-05-07--plan--topsi-build.md (§19 Verification Tasks)

## Summary

| #  | Question                          | Status                | Note                                                             |
|----|-----------------------------------|-----------------------|------------------------------------------------------------------|
| V1 | VIBE metering enforced today?     | **Yes — enforced**    | Pre-chat balance check + post-chat ledger write. Bypass toggle.  |
| V2 | Canonical chat endpoints?         | **Three live**        | Topsi JSON-only; Nora has /stream SSE; agents has /stream SSE.   |
| V3 | Voice TTS shape?                  | **Base64 JSON**       | Not chunked-MP3. WAV / MP3 / FLAC / OGG. No agent_id parameter.  |
| V4 | SGT / SIWS / Solana wallet auth?  | **Not found**         | Wallet support is Aptos-only. Fresh build for Topsi 0.3.         |
| V5 | Cross-origin auth posture?        | **Confirmed**         | ALLOWED_ORIGINS + localStorage Bearer fallback unchanged.        |
| V6 | VIBE balance endpoint?            | **Live, project-scoped** | `GET /api/projects/{project_id}/vibe/balance`                 |
| V7 | Agent list admin gating?          | **Confirmed**         | admin → all; user → system-tier + own. Nora is owner-scoped.    |
| V8 | dApp-publishing artifacts?        | **Not found**         | Fresh manifest + 1024×1024 icons needed for store submission.   |

## V1 — VIBE-per-message metering

**Status: Enforced (with graceful degradation)**

- Bypass toggle via system setting: `crates/server/src/helpers/vibe_check.rs` (`VIBE_BYPASS`)
- Pre-chat balance check: `ensure_vibe_balance()` at `crates/server/src/helpers/billing.rs:56`
- Post-chat charge: `record_llm_vibe_usage()` at `crates/server/src/helpers/billing.rs:88`
- Currency unit: VIBE as `i64` (stored in `vibe_transactions.amount_vibe`)
- Enforcement flow inside Topsi chat handler: `topsi/chat.rs:32-34` (balance check) → `topsi/chat.rs:70-91` (record cost)
- Failure semantics: billing failures DO NOT block responses (per `billing.rs:87`) — fail-open behaviour

**Implication for Topsi (app):** the gas-metering layer in §9 of the build plan is no longer "build from scratch." We expose existing logic via a new `/api/billing/rate` endpoint and add a 2× multiplier to derive `topsi_rate_per_msg` from the existing VIBE rate. The pre-mint virtual balance pattern can mirror `vibe_transactions` directly.

## V2 — Canonical chat endpoints

| Route                                  | Handler                              | Streaming     | Status |
|----------------------------------------|--------------------------------------|---------------|--------|
| `POST /api/topsi/chat`                 | `chat_with_topsi`  (topsi/chat.rs:6) | JSON only     | Live   |
| `POST /api/nora/chat`                  | `chat_with_nora`   (nora/chat.rs:6)  | JSON only     | Live   |
| `POST /api/nora/chat/stream`           | (nora/chat.rs:231)                   | SSE           | Live   |
| `POST /api/agents/{id}/chat`           | `agent_chat`        (agent_chat.rs:100)| JSON only   | Live   |
| `POST /api/agents/{id}/chat/stream`    | `agent_chat_stream` (agent_chat.rs:505)| SSE         | Live   |

**Implication for Topsi (app):** use `POST /api/agents/{id}/chat/stream` as the canonical streaming path for **all agents including Topsi**. Topsi-specific `/api/topsi/chat` is JSON-only and would force a non-streaming UX; the generic agent endpoint streams over SSE for any agent id, which is what we want.

## V3 — Voice TTS endpoint shape

Public routes (registered at `topsi/mod.rs:329-331`):
- `POST /api/topsi/voice/synthesize` → `synthesize_speech()`  (`voice.rs:328`)
- `POST /api/topsi/voice/transcribe`  → `transcribe_speech()` (`voice.rs:364`)
- `POST /api/topsi/voice/interaction` → `voice_interaction()` (`voice.rs:396`)

Synthesize request: `VoiceSynthesisRequest { text, voice_profile?, speed?, executive_tone? }`
Synthesize response: `SpeechResponse { audio_data: String (base64), duration_ms, sample_rate, format, processing_time_ms }`
Audio formats: WAV / MP3 / FLAC / OGG
Content-Type: application/json (NOT chunked MP3)
No agent_id parameter — global voice engine via Chatterbox + OpenAI fallback (`voice.rs:195-200`).

**Implication for Topsi (app):** use `expo-av` to play the decoded base64 blob in one shot. Simpler than chunked streaming. Voice differentiation per agent (Topsi vs Nora) goes via `voice_profile` string parameter, not agent_id.

## V4 — SGT / SIWS / Solana wallet auth

**Not found.** No Seeker Genesis Token, Sign-in-with-Solana, or generic Solana wallet auth path exists. Existing wallet endpoints are Aptos:
- `POST /api/vibe/send` — Aptos VIBE transfer (`aptos.rs:26`)
- `GET /api/vibe/balance/{address}` — Aptos balance read (`aptos.rs:25`)

**Implication for Topsi (app):** Phase 0.3 (SGT progressive enhancement) is fresh-build territory on both client AND backend. Adds `/api/auth/sgt-verify` to pcg-cc-mcp.

## V5 — Cross-origin auth posture

- CORS: `ALLOWED_ORIGINS` env var → `CorsLayer` (`routes/mod.rs:293-330`)
- Credentials: `allow_credentials(true)` + COOKIE header allowed (`routes/mod.rs:322, 328`)
- Frontend stores session_id: `localStorage.setItem('session_id', ...)` (`auth-api.ts:69`)
- Frontend Bearer fallback: `Authorization: Bearer <session_id>` from localStorage (`topsi/MeetingHistory.tsx:85-87`, `meeting-mode/types.ts:61-63`)

**Implication for Topsi (app):** mobile uses Bearer-only path (no cookie). No backend change required for Topsi mobile to authenticate cross-origin. **One caveat:** `ALLOWED_ORIGINS` may still need updating to include the Topsi web preview build's origin if we ship one.

## V6 — VIBE balance endpoint

Endpoint: `GET /api/projects/{project_id}/vibe/balance` (`vibe_treasury.rs:71`)
Handler: `get_project_balance()` (`vibe_treasury.rs:147`)
Response: `ProjectVibeBalance { project_id, total_deposited, total_withdrawn, total_spent, available_balance }` (`vibe_treasury.rs:25-31`)
Currency: VIBE as `i64`
Source: DB ledger (deposits − withdrawals − spent)

**Implication for Topsi (app):** balance is project-scoped, NOT user-scoped. Topsi the app needs a default project_id (Bodhi's primary) — likely set via `/api/auth/me` returning `default_project_id`. Verify this in code before client wiring.

## V7 — Agent list admin gating

Route: `GET /api/agents` (`agents.rs:52`)
Filter (`agents.rs:58-71`):
- Admin → `Agent::find_all(pool)`
- Regular → `Agent::find_visible_for_user(pool, user_id)` → `agent_tier = 'system' OR owner_id = $1`
- No-auth fallback → `find_all()`
Tier rule definition: `db/models/agent.rs:820`

Nora is special-cased via `/nora/initialize` (`nora/mod.rs:228`) — not a standard agent_tier filter.

**Implication for Topsi (app):** Topsi the app calls `GET /api/agents` and trusts the server-side filter. Nora visibility for Bodhi will Just Work because the agent list naturally returns owner-scoped + system agents. We do NOT need to ship a client-side admin check.

## V8 — dApp-publishing artifacts

**None found.** No `dapp-publishing` config, no Solana Mobile manifests, no 1024×1024 PNG icons in the asset tree. Existing assets in `pcg-cc-mcp/assets/` are fonts + `.wav` audio.

(Conference workflow does generate 1024×1024 graphics dynamically at `nora/src/conference_workflow/graphics.rs:31-35` — image generation infrastructure exists, just not pre-baked icons.)

**Implication for Topsi (app):** all dApp Store assets ship fresh from the Topsi project — icon, splash, screenshots, manifest JSON. No reusable artifact in pcg-cc-mcp.
