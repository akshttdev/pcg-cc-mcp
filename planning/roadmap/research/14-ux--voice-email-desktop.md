# Research: Voice, Email, SMS & Desktop Capabilities

**Date**: 2026-03-19
**Type**: Research
**Author**: Claude Research Sprint
**Status**: Complete

---

## Table of Contents

1. [Voice/Audio Pipeline](#1-voiceaudio-pipeline)
2. [Email Integration](#2-email-integration)
3. [SMS/Messaging](#3-smsmessaging)
4. [Desktop Application](#4-desktop-application)
5. [Push Notifications](#5-push-notifications)
6. [Existing Codebase Inventory](#6-existing-codebase-inventory)
7. [Consolidated Recommendations](#7-consolidated-recommendations)

---

## 1. Voice/Audio Pipeline

### Existing State

ORCHA already has a substantial voice pipeline:
- **STT**: WhisperSTT (OpenAI API), LocalWhisperSTT (self-hosted Python server), AzureSTT, GoogleSTT (stub), SystemSTT (fallback)
- **TTS**: ElevenLabsTTS, AzureTTS, OpenAITTS, ChatterboxTTS (local), SystemTTS
- **VoiceEngine**: Full orchestrator with session management, British executive personality
- **VoiceGateway**: Stub module (commented out in `crates/nora/src/voice/mod.rs`)
- **Voice routes**: WebSocket endpoint, SSE events, synthesize/transcribe REST endpoints (in `crates/server/src/routes/voice.rs` — references VoiceGateway/BrowserChannel/GlassesChannel that are not yet compiled)
- **Twilio integration**: Full call handler, TwiML builder, audio cache, SMS sender
- **Architecture supports**: Browser (WebSocket), Glasses (APN mesh), Phone (Twilio), Desktop (local mic)

The primary gap is **real-time browser voice** via WebRTC and **local inference** without Python sidecar dependencies.

---

### 1.1 WebRTC Libraries

#### str0m (Rust, Sans-I/O)

- **Repository**: [github.com/algesten/str0m](https://github.com/algesten/str0m)
- **License**: MIT / Apache-2.0
- **Architecture**: Sans-I/O — the `Rtc` instance does no network I/O, no internal threads, no async runtime dependency. All operations happen through public API calls. This is the idiomatic Rust approach.
- **Performance**: Benchmarks show ~218k messages/sec on data channels vs ~138k for webrtc-rs
- **Maturity**: libp2p migrated from webrtc-rs to str0m ([issue #3659](https://github.com/libp2p/rust-libp2p/issues/3659))

**Pros**:
- No async runtime coupling — works with tokio, async-std, or sync code
- Significantly better performance than webrtc-rs
- Cleaner Rust-idiomatic API (no callback hell)
- MIT license, no commercial restrictions
- Active maintenance

**Cons**:
- Smaller community than webrtc-rs
- Less documentation and examples
- No built-in TURN server support (must integrate separately)
- Sans-I/O requires more boilerplate to wire up networking

**Roadmap Stage**: Stage 1 (First Pilots) — browser voice is a differentiator for demos
**Budget Impact**: 2-3 engineer-weeks to integrate with existing VoiceGateway architecture
**Recommendation**: **PRIMARY CHOICE** for Rust-side WebRTC

#### webrtc-rs

- **Repository**: [github.com/webrtc-rs/webrtc](https://github.com/webrtc-rs/webrtc)
- **License**: MIT / Apache-2.0
- **Architecture**: Async-based, ported from Pion (Go). v0.20.0 in development with new Sans-I/O architecture.
- **Performance**: ~138k messages/sec data channel throughput

**Pros**:
- Larger community, more examples
- API mirrors JavaScript WebRTC (familiar to web developers)
- Full TURN/STUN support built-in

**Cons**:
- Fundamental design issues: async callbacks require locks, prevent idiomatic `&mut` usage
- Lower performance than str0m
- v0.20.0 rewrite is in-progress — current API may change
- Has been losing momentum (libp2p moved away)

**Roadmap Stage**: Stage 1 alternative
**Recommendation**: **SKIP** — str0m is superior for ORCHA's architecture

#### LiveKit (Server + Rust SDK)

- **Repository**: [github.com/livekit/rust-sdks](https://github.com/livekit/rust-sdks)
- **License**: Apache-2.0 (both server and SDK)
- **Architecture**: Full SFU (Selective Forwarding Unit) server written in Go, with Rust client SDK

**Pros**:
- Production-grade SFU with recording, streaming, room management
- Rust SDK is well-maintained (Zed IDE uses it)
- Handles scaling, TURN, authentication out of the box
- Built-in AI agent framework for voice agents
- Apache-2.0 — fully permissive

**Cons**:
- Requires running a separate Go server (not pure Rust)
- Adds operational complexity
- Overkill for 1:1 voice interactions (designed for multi-party)
- Dependency on external infrastructure

**Roadmap Stage**: Stage 2-3 — when multi-party meetings become a requirement
**Budget Impact**: $50-200/mo for hosted LiveKit Cloud, or self-hosted on existing infra
**Recommendation**: **STAGE 2 UPGRADE** — use for Topsi meeting rooms and multi-agent voice conferences

#### Janus / mediasoup

- **Janus**: C-based SFU, GPL-3.0 license
- **mediasoup**: Node.js SFU, ISC license

**Recommendation**: **SKIP BOTH** — Janus is GPL (license contamination risk), mediasoup requires Node.js runtime. LiveKit is the better managed SFU option.

---

### 1.2 Voice Activity Detection (VAD)

#### Silero VAD (via Rust bindings)

- **Crates**: `silero-vad-rs`, `silero-vad-rust`, `vad-silero-rs`
- **License**: MIT (Rust bindings), MIT (original model)
- **Architecture**: Pre-trained ONNX model, runs via `ort` crate (ONNX Runtime)

**Pros**:
- Enterprise-grade accuracy, trained on 6000+ languages
- Multiple Rust crate options, all using ONNX Runtime
- ~5ms inference time per frame
- No GPU required

**Cons**:
- Requires ONNX Runtime dependency
- Model file bundled in crate (~2MB)
- Community-maintained Rust bindings (not official)

**Roadmap Stage**: Stage 0-1 — critical for real-time voice pipeline
**Budget Impact**: Free, 1 engineer-day to integrate
**Recommendation**: **IMMEDIATE ADOPTION** — use `silero-vad-rs` crate

#### RNNoise

- **License**: BSD-3-Clause (original), various Rust bindings
- **Architecture**: Recurrent neural network for noise suppression + VAD

**Pros**:
- Extremely lightweight (~100KB model)
- Combines noise suppression AND voice detection
- Real-time capable on embedded devices

**Cons**:
- Lower accuracy than Silero for VAD specifically
- Primary purpose is denoising, not VAD
- Fewer Rust bindings available

**Roadmap Stage**: Stage 1 — as noise suppression layer
**Recommendation**: **COMPLEMENT** — use alongside Silero VAD for noise reduction

---

### 1.3 Real-Time STT

#### whisper.cpp (via whisper-rs)

- **Crate**: `whisper-rs` (v0.16.0, March 2026), `whisper-cpp-plus` (streaming + VAD)
- **License**: MIT (whisper.cpp), MIT (bindings)
- **Architecture**: C++ library with Rust FFI bindings

**Pros**:
- Best open-source STT quality (Whisper large-v3)
- Runs fully local — no API costs
- `whisper-cpp-plus` adds real-time PCM streaming and VAD
- ORCHA already has `scripts/whisper_server.py` — this replaces Python dependency
- GPU acceleration via Metal (macOS) / CUDA / Vulkan

**Cons**:
- C++ build dependency (cmake required — already in flox)
- Large model files (base=142MB, medium=1.5GB, large=3GB)
- Latency: ~200ms for base model, ~2s for large on CPU
- Not true streaming — processes chunks

**Roadmap Stage**: Stage 0 — replace Python whisper_server.py with native Rust
**Budget Impact**: Free, 3-5 engineer-days to replace Python server
**Recommendation**: **HIGH PRIORITY** — eliminates Python sidecar dependency

#### Vosk

- **Rust bindings**: `vosk-sys`
- **License**: Apache-2.0
- **Architecture**: Kaldi-based, C API with Rust FFI

**Pros**:
- True streaming recognition (word-by-word)
- Very small models (50MB for basic English)
- 20+ language support
- Runs on Raspberry Pi — very lightweight

**Cons**:
- Lower accuracy than Whisper (especially for accented speech)
- Less active development than Whisper ecosystem
- British English accuracy may be weaker

**Roadmap Stage**: Stage 2 — as lightweight fallback for resource-constrained deployments
**Recommendation**: **OPTIONAL FALLBACK** — Whisper.cpp is better for ORCHA's quality needs

#### Deepgram SDK

- **License**: Proprietary (cloud API)
- **Architecture**: REST/WebSocket API

**Pros**:
- Best-in-class real-time streaming STT
- True word-level streaming with <300ms latency
- Excellent accuracy for business English
- Built-in speaker diarization

**Cons**:
- Cloud-only — violates sovereign stack principle
- $0.0043/min (Nova-2) — costs add up at scale
- No Rust SDK (HTTP/WS client only)

**Roadmap Stage**: Stage 1-2 — as premium cloud option alongside local Whisper
**Budget Impact**: ~$25/mo for 100 hours of transcription
**Recommendation**: **OPTIONAL CLOUD TIER** — offer as premium STT provider

---

### 1.4 Real-Time TTS (Local)

#### Piper (Local Neural TTS)

- **Rust bindings**: `piper-rs`, `piper-tts-rust`
- **License**: MIT (original, now archived). **WARNING**: Active development moved to GPL-3.0 fork (`piper1-gpl` by Open Home Foundation)
- **Architecture**: ONNX-based VITS models, extremely fast

**Pros**:
- Extremely fast — real-time on Raspberry Pi
- Many pre-trained voices including British English
- Small model files (~60MB per voice)
- Rust bindings available

**Cons**:
- **LICENSE RISK**: Original MIT repo archived Oct 2025, active fork is GPL-3.0
- Quality below ElevenLabs/Chatterbox for natural speech
- No voice cloning capability
- Rust bindings are community-maintained

**Roadmap Stage**: Stage 1 — as lightweight local TTS for low-latency responses
**Budget Impact**: Free, 2 engineer-days to integrate
**Recommendation**: **USE MIT VERSION ONLY** — pin to archived MIT release, do not use GPL fork. Use for ultra-low-latency responses, Chatterbox for quality.

#### Kokoro

- **License**: Apache-2.0
- **Architecture**: 82M parameter model, lightweight VITS-based

**Pros**:
- Apache-2.0 — fully permissive, commercially safe
- Only 82M parameters — fast inference
- Quality comparable to much larger models
- Active development

**Cons**:
- No native Rust bindings yet (Python/ONNX)
- Newer project, less battle-tested
- Limited voice variety

**Roadmap Stage**: Stage 1-2 — as Apache-2.0 alternative to GPL Piper
**Recommendation**: **WATCH** — promising Apache-2.0 replacement for Piper

#### Chatterbox (Resemble AI)

- **License**: MIT
- **Architecture**: Python library, HTTP server wrapper

**Pros**:
- MIT license, commercially safe
- High quality voice cloning from short samples
- Multilingual support
- Already integrated in ORCHA (`ChatterboxTTS`, `SystemTTS`)

**Cons**:
- Python dependency (requires sidecar process)
- Higher latency than Piper
- GPU strongly recommended for real-time use

**Roadmap Stage**: Stage 0 (already integrated)
**Recommendation**: **KEEP AS PRIMARY** — already working, MIT license

#### Bark (Suno AI)

- **License**: MIT
- **Architecture**: Transformer-based, generates speech + non-verbal sounds

**Pros**:
- MIT license
- Can generate laughter, sighs, music
- Highly expressive

**Cons**:
- Slow — not suitable for real-time
- High GPU requirements
- No streaming support

**Recommendation**: **SKIP** for real-time voice; consider for pre-generated content

#### edge-tts

- **License**: MIT (Python package)
- **Architecture**: Wrapper around Microsoft Edge's online TTS service

**Pros**:
- Free to use (no API key needed)
- High quality Microsoft Neural voices
- Many British English voices

**Cons**:
- **Requires internet** — not sovereign
- Uses Microsoft's service without explicit API agreement (grey area)
- Could stop working at any time
- Python dependency

**Recommendation**: **SKIP** — legally grey, not sovereign-compatible

---

### 1.5 Opus Codec Integration

ORCHA already has `libopus` in the flox dev environment. Opus is the standard codec for WebRTC voice.

- **Rust crates**: `opus` (safe bindings), `audiopus` (higher-level API)
- **License**: BSD-3-Clause (libopus), MIT (Rust bindings)
- **Integration pattern**: Encode audio at source (browser/client), decode at server, process, re-encode for TTS output

**Roadmap Stage**: Stage 0-1 — needed when WebRTC is added
**Recommendation**: Already available via flox. Use `audiopus` crate for Rust-side encoding/decoding.

---

### Voice Pipeline Summary

| Component | Recommended Tool | License | Stage | Priority |
|-----------|-----------------|---------|-------|----------|
| WebRTC (browser) | str0m | MIT/Apache-2.0 | 1 | High |
| WebRTC (multi-party) | LiveKit | Apache-2.0 | 2-3 | Medium |
| VAD | silero-vad-rs | MIT | 0-1 | Critical |
| Noise suppression | RNNoise | BSD-3 | 1 | Medium |
| Local STT | whisper-rs (whisper.cpp) | MIT | 0 | Critical |
| Cloud STT | Deepgram | Proprietary | 1-2 | Low |
| Local TTS (fast) | Piper (MIT archived) | MIT | 1 | Medium |
| Local TTS (quality) | Chatterbox | MIT | 0 (done) | N/A |
| Local TTS (future) | Kokoro | Apache-2.0 | 1-2 | Watch |
| Codec | libopus + audiopus | BSD-3/MIT | 0-1 | High |

---

## 2. Email Integration

### Existing State

ORCHA already has:
- `EmailAccount` model with Gmail, Zoho, IMAP providers (`crates/db/src/models/email_account.rs`)
- `EmailMessage` model with sentiment analysis, priority, threading (`crates/db/src/models/email_message.rs`)
- Email account routes (`crates/server/src/routes/email_accounts.rs`)
- Email message routes (`crates/server/src/routes/email_messages.rs`)

The gap is **actual email sending/receiving** — the models exist but the transport layer is not wired.

---

### 2.1 Sending Email

#### lettre

- **Crate**: `lettre`
- **License**: MIT / Apache-2.0
- **Architecture**: Pure Rust email client supporting SMTP, with async support via tokio

**Pros**:
- De facto standard Rust email library
- SMTP with STARTTLS/TLS support
- Async (tokio) and sync APIs
- DKIM signing support
- Well-maintained, mature (v0.11+)
- Builds email messages with MIME support

**Cons**:
- SMTP only — no transactional email API integration
- Must manage SMTP credentials
- Deliverability depends on your SMTP server reputation

**Roadmap Stage**: Stage 0-1 — for system notifications and direct SMTP
**Budget Impact**: Free, 1-2 engineer-days
**Recommendation**: **ADOPT** — use for system emails via SMTP relay

#### Resend

- **License**: Proprietary (cloud API)
- **API**: REST, no official Rust SDK (simple HTTP calls)
- **Pricing**: Free tier: 100 emails/day, Paid: $20/mo for 50k emails

**Pros**:
- Best developer experience (founded by React Email creators)
- Excellent deliverability
- Simple REST API — trivial to call from Rust via `reqwest`
- React Email template compatibility (matches ORCHA's React frontend)

**Cons**:
- No Rust SDK (but API is trivial)
- Cloud-only
- Relatively new (2023)

**Roadmap Stage**: Stage 1 — for client-facing transactional emails
**Budget Impact**: $20/mo for 50k emails
**Recommendation**: **PRIMARY TRANSACTIONAL** — best DX, easy integration

#### Postmark

- **License**: Proprietary (cloud API)
- **Pricing**: $15/mo for 10k emails

**Pros**:
- Industry-best deliverability rates
- Inbound email processing (webhooks for received emails)
- Message streams for transactional vs broadcast
- 45-day message retention

**Cons**:
- More expensive per-email than Resend
- No Rust SDK
- Stricter sending policies

**Roadmap Stage**: Stage 2 — when deliverability SLAs matter
**Recommendation**: **ALTERNATIVE** — consider if Resend deliverability is insufficient

#### SendGrid (Twilio)

- **License**: Proprietary (cloud API)
- **Pricing**: Free tier: 100 emails/day, Paid: $19.95/mo

**Pros**:
- Already using Twilio for SMS — account consolidation
- Massive scale capability
- Official SDK for 8+ languages
- Email analytics and A/B testing

**Cons**:
- Complex API, heavy abstraction
- Twilio acquisition has led to degraded developer experience
- Known deliverability issues post-Twilio merger

**Roadmap Stage**: Stage 2-3
**Recommendation**: **SKIP** — Resend is better for ORCHA's needs

---

### 2.2 Receiving Email (IMAP)

#### imap crate

- **Crate**: `imap` (v3.0.0-alpha.15)
- **License**: MIT / Apache-2.0
- **Architecture**: Async IMAP client for Rust

**Pros**:
- Full IMAP protocol support (RFC 3501 + extensions)
- IDLE support for push notifications
- TLS/STARTTLS support
- Maintained by Jon Gjengset (well-known Rust developer)

**Cons**:
- Still in alpha for v3 (v2.4.1 is stable)
- Complex API (IMAP protocol is inherently complex)
- OAuth2 for Gmail requires additional setup

**Roadmap Stage**: Stage 1 — for email-to-task automation
**Budget Impact**: Free, 3-5 engineer-days for Gmail/Zoho integration
**Recommendation**: **ADOPT** — essential for email monitoring

#### mailparse

- **Crate**: `mailparse` (v0.16.1)
- **License**: MIT / Apache-2.0
- **Architecture**: MIME email parser

**Pros**:
- Parses real-world email data (MIME, attachments, headers)
- Handles multipart messages
- Stable, well-maintained
- Works with data from IMAP FETCH

**Cons**:
- Parser only — does not send/receive
- No HTML-to-text conversion built-in

**Roadmap Stage**: Stage 1 — pair with `imap` crate
**Recommendation**: **ADOPT** — needed for email processing

---

### 2.3 Email-to-Task Automation Pattern

```
IMAP IDLE (monitor inbox)
  -> mailparse (extract content)
  -> LLM classification (urgency, category, CRM contact matching)
  -> Auto-create task OR CRM activity
  -> Notify user via push/SSE
  -> Auto-draft reply (optional, human-in-loop)
```

**Roadmap Stage**: Stage 1 (basic), Stage 2 (LLM-powered classification)
**Budget Impact**: 1-2 engineer-weeks for basic flow, 1 week for LLM classification

---

### Email Summary

| Component | Recommended Tool | License | Stage | Priority |
|-----------|-----------------|---------|-------|----------|
| SMTP sending | lettre | MIT/Apache-2.0 | 0-1 | High |
| Transactional API | Resend | Proprietary | 1 | High |
| IMAP receiving | imap crate | MIT/Apache-2.0 | 1 | High |
| Email parsing | mailparse | MIT/Apache-2.0 | 1 | High |
| Email-to-task | Custom (LLM) | N/A | 1-2 | Medium |

---

## 3. SMS/Messaging

### Existing State

ORCHA already has:
- `TwilioSmsSender` with rate limiting, CRM activity logging (`crates/server/src/twilio_sms.rs`)
- `SmsMessage` model in database
- SignalWire compatibility (LaML endpoint fallback)
- Twilio call handling (inbound/outbound voice, TwiML, speech recognition)
- SMS routes (`crates/server/src/routes/twilio/sms.rs`)

The SMS foundation is solid. Gaps are in **encrypted messaging** and **decentralized communication**.

---

### 3.1 Twilio SMS (Already Implemented)

- **Status**: Fully functional
- **Features**: Send SMS, Pulse alerts, content notifications, rate limiting, CRM activity tracking
- **Cost**: ~$0.0079/SMS outbound, $0.0075/SMS inbound
- **Enhancement opportunities**:
  - MMS support (image/video messages)
  - WhatsApp Business API via Twilio
  - Conversation API for stateful threads

**Roadmap Stage**: Stage 0 (already done)
**Enhancement Stage**: Stage 1 (WhatsApp, MMS)

---

### 3.2 Signal Protocol

- **License**: GPL-3.0 (libsignal), AGPL-3.0 (Signal server)
- **Rust crate**: `libsignal-protocol` (official, by Signal Foundation)

**Pros**:
- Gold standard for E2E encryption
- Double Ratchet algorithm, forward secrecy
- Used by WhatsApp, Facebook Messenger

**Cons**:
- **GPL-3.0 license** — viral, contaminates linked code
- Complex key management
- Cannot use Signal network directly (no API)
- Overkill for most business messaging use cases

**Roadmap Stage**: Stage 3-4 — only if E2E encrypted messaging becomes a requirement
**Recommendation**: **DEFER** — GPL license is problematic. If E2E encryption needed, use Matrix protocol instead.

---

### 3.3 Matrix Protocol

- **Rust SDK**: [github.com/matrix-org/matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk)
- **License**: Apache-2.0 (SDK and protocol)
- **Architecture**: Decentralized, federated messaging protocol

**Pros**:
- Apache-2.0 — commercially safe
- First-class Rust SDK (used by Element X, Zed)
- E2E encryption built-in (Vodozemac, also Rust)
- Federation — users keep their own servers
- Government adoption (10+ national governments, ICC)
- Bridges to Slack, Discord, Telegram, WhatsApp, IRC
- Rich features: rooms, threads, reactions, file sharing, voice/video
- Active development (Matrix Community Summit 2026 in May)

**Cons**:
- Complex protocol — significant integration effort
- Running a homeserver (Synapse or Conduit) adds ops burden
- Conduit (Rust homeserver) is not yet production-stable
- Federation complicates compliance (data sovereignty)

**Roadmap Stage**: Stage 2-3 — as internal messaging backbone and external bridge
**Budget Impact**: 3-4 engineer-weeks for basic integration, $20-50/mo for homeserver
**Recommendation**: **STRATEGIC INVESTMENT** — aligns with sovereign stack vision. Use for:
  - Internal team communication within ORCHA
  - Agent-to-user messaging
  - Bridge to external platforms (Slack, Discord)
  - Decentralized alternative to proprietary chat

---

### SMS/Messaging Summary

| Component | Recommended Tool | License | Stage | Priority |
|-----------|-----------------|---------|-------|----------|
| SMS outbound | Twilio (done) | Proprietary | 0 (done) | N/A |
| WhatsApp | Twilio WhatsApp API | Proprietary | 1 | Medium |
| E2E encryption | Matrix SDK | Apache-2.0 | 2-3 | Medium |
| Decentralized chat | Matrix + Conduit | Apache-2.0 | 2-3 | Medium |
| Signal Protocol | libsignal | GPL-3.0 | Skip | N/A |

---

## 4. Desktop Application

### Existing State

ORCHA has **two Tauri applications**:
1. **Main ORCHA desktop** (`src-tauri/`): Tauri 2.0 with backend sidecar, system info, health checks. Has merge conflict markers (needs cleanup).
2. **APN App** (`apn-app/src-tauri/`): Separate Tauri app for Alpha Protocol Network node management, mesh networking, VIBE balance tracking.

Both use Tauri 2.0 with `tauri-plugin-shell`, `tauri-plugin-log`, `sysinfo`.

---

### 4.1 Tauri 2.0

- **License**: MIT / Apache-2.0
- **Architecture**: Rust core + WebView (system webview, not Electron's Chromium)

**Pros**:
- Already integrated in ORCHA (two apps)
- ~10MB binary vs ~150MB Electron
- Uses system webview — no bundled Chromium
- Rust backend with full access to OS APIs
- Sidecar pattern for backend server
- Mobile support (iOS/Android) in Tauri 2.0
- MIT/Apache-2.0 — commercially safe

**Cons**:
- WebView differences across platforms (especially Windows WebView2)
- Smaller ecosystem than Electron
- Some plugins less mature
- Current ORCHA Tauri code has merge conflicts (needs cleanup)

**Roadmap Stage**: Stage 0 (cleanup), Stage 1 (polish for demos)
**Recommendation**: **CONTINUE** — already invested, strong fit

---

### 4.2 System Tray Integration

- **Plugin**: Built into Tauri 2.0 (`tray-icon` feature, renamed from `system-tray`)
- **API**: `app.tray_icon()` for creating/managing tray icons

**Capabilities**:
- Custom tray icon with dynamic updates
- Context menu with actions
- Tooltip text
- Click/double-click handlers
- Show/hide window on tray click

**Integration pattern for ORCHA**:
```
System tray icon shows:
- Green: all agents healthy
- Yellow: tasks need attention
- Red: agent failures
Right-click menu:
- Quick task creation
- View active agents
- Voice input (hold to talk)
- Open dashboard
- Quit
```

**Roadmap Stage**: Stage 1
**Budget Impact**: 1-2 engineer-days
**Recommendation**: **ADOPT** — low effort, high UX value

---

### 4.3 Auto-Update (tauri-plugin-updater)

- **Plugin**: `tauri-plugin-updater`
- **Architecture**: Checks a JSON endpoint for new versions, downloads and applies updates

**Update flow**:
1. App checks update endpoint on startup (or periodically)
2. Endpoint returns JSON with version, platform-specific download URLs, signatures
3. App downloads update bundle, verifies signature
4. Applies update (restart required)

**Hosting options**:
- CrabNebula Cloud (Tauri-native, managed)
- Self-hosted JSON endpoint + file server (S3, Cloudflare R2)
- GitHub Releases (free, integrates with CI/CD)

**Roadmap Stage**: Stage 1
**Budget Impact**: Free (GitHub Releases) or $10-50/mo (CrabNebula)
**Recommendation**: **ADOPT** — use GitHub Releases for Stage 1, CrabNebula for Stage 2+

---

### 4.4 Local LLM Integration

#### candle (Hugging Face)

- **Repository**: [github.com/huggingface/candle](https://github.com/huggingface/candle)
- **License**: MIT / Apache-2.0
- **Architecture**: Pure Rust ML framework

**Pros**:
- Pure Rust — no C++ dependencies
- Supports LLaMA, Mistral, Phi, Gemma, etc.
- LoRA fine-tuning support
- Metal (macOS) and CUDA acceleration
- Quantization support (GGUF, GPTQ)

**Cons**:
- Slower than llama.cpp for inference
- Smaller model ecosystem
- Less community tooling

#### mistral.rs (on candle)

- **License**: MIT / Apache-2.0
- **Architecture**: High-performance inference engine built on candle

**Pros**:
- Best pure-Rust LLM inference performance
- OpenAI-compatible API server
- Supports quantized models
- ISQ (In-Situ Quantization) for on-the-fly quantization

**Cons**:
- Focused on Mistral/LLaMA family
- Newer project

#### llama-cpp-2 (llama.cpp bindings)

- **Crate**: `llama-cpp-2`
- **License**: MIT (bindings), MIT (llama.cpp)
- **Architecture**: Thin Rust FFI wrapper around llama.cpp C API

**Pros**:
- Best inference performance (llama.cpp is the gold standard)
- Widest model support via GGUF format
- Active maintenance
- Metal/CUDA/Vulkan acceleration

**Cons**:
- C++ build dependency
- FFI boundary complexity
- Less Rust-idiomatic

#### Crane

- **Repository**: [github.com/lucasjinreal/Crane](https://github.com/lucasjinreal/Crane)
- **License**: MIT
- **Architecture**: Pure Rust inference engine on candle, supports LLM/VLM/TTS/OCR

**Pros**:
- Pure Rust alternative to llama.cpp
- Comparable performance
- No GGUF conversion needed
- Cleaner codebase

**Cons**:
- Smaller community
- Less battle-tested

**Recommendation for ORCHA**:
- **Stage 1**: Use `llama-cpp-2` bindings — widest model support, best performance
- **Stage 2-3**: Migrate to `mistral.rs` or `Crane` as pure Rust matures
- **Use case**: Offline agent capabilities, local classification, email triage, quick commands

**Budget Impact**: Free (models are open-weight), 2-3 engineer-weeks for integration

---

### 4.5 Offline-First Patterns with Sync

**Architecture for ORCHA desktop offline mode**:

```
Desktop App (Tauri)
  -> Local SQLite DB (already used by ORCHA)
  -> Background sync service
  -> Conflict resolution (CRDT or last-write-wins)
  -> Server reconciliation on reconnect
```

**Key patterns**:
1. **Local-first DB**: ORCHA already uses SQLite — same DB engine locally
2. **Sync protocol options**:
   - `cr-sqlite` (CRDT-based SQLite extension, MIT) — automatic conflict resolution
   - Custom sync via ORCHA's existing SSE events + REST API
   - Electric SQL (Postgres-based, Apache-2.0) — if ORCHA migrates to Postgres
3. **Offline queue**: Queue mutations locally, replay on reconnect
4. **Selective sync**: Only sync user's active projects/tasks

**Roadmap Stage**: Stage 2-3 — requires careful data model design
**Budget Impact**: 2-4 engineer-weeks
**Recommendation**: Start with simple offline queue in Stage 1, full CRDT sync in Stage 3

---

### Desktop Summary

| Component | Recommended Tool | License | Stage | Priority |
|-----------|-----------------|---------|-------|----------|
| Desktop app | Tauri 2.0 (done) | MIT/Apache-2.0 | 0 (cleanup) | High |
| System tray | Tauri tray-icon | MIT/Apache-2.0 | 1 | Medium |
| Auto-update | tauri-plugin-updater | MIT/Apache-2.0 | 1 | High |
| Local LLM | llama-cpp-2 | MIT | 1-2 | Medium |
| Local LLM (pure Rust) | mistral.rs / Crane | MIT | 2-3 | Low |
| Offline sync | cr-sqlite + custom | MIT | 2-3 | Medium |

---

## 5. Push Notifications

### Existing State

ORCHA has:
- `NotificationService` with sound alerts (cross-platform: macOS afplay, Linux paplay/aplay, WSL PowerShell)
- Push notification placeholder in `NotificationService::send_push_notification()`
- SSE for real-time frontend updates (already working)

---

### 5.1 Web Push API

- **Standard**: W3C Push API + VAPID authentication
- **Rust crate**: `web-push` (MIT)
- **Architecture**: Server generates VAPID keys, browser registers service worker, server pushes via Web Push protocol

**Pros**:
- Standards-based — works in all modern browsers
- No third-party dependency
- Free (no per-message cost)
- Works when browser tab is closed (via service worker)

**Cons**:
- Safari support was late (iOS 16.4+, 2023)
- Requires HTTPS
- Service worker complexity on frontend
- No delivery guarantees

**Roadmap Stage**: Stage 1
**Budget Impact**: Free, 2-3 engineer-days (backend + frontend service worker)
**Recommendation**: **ADOPT** — standard approach for web notifications

---

### 5.2 ntfy (Self-Hosted Push)

- **Repository**: [github.com/binwiederhier/ntfy](https://github.com/binwiederhier/ntfy)
- **License**: Apache-2.0 / GPL-2.0 (dual license)
- **Rust crates**: `ntfy-sdk` (client library)
- **Architecture**: HTTP pub-sub with mobile apps (iOS/Android) and PWA

**Pros**:
- Self-hostable — aligns with sovereign stack
- Simple HTTP API (PUT/POST to send)
- Native iOS and Android apps
- PWA for desktop
- Free tier at ntfy.sh for development
- Supports attachments, actions, priorities

**Cons**:
- GPL-2.0 for server (Apache-2.0 for library) — server license is restrictive
- Requires running another service
- No Rust server (Go-based)
- Mobile apps require ntfy.sh relay for push (FCM/APNs)

**Roadmap Stage**: Stage 1-2
**Budget Impact**: Free (self-hosted), 1-2 engineer-days to integrate
**Recommendation**: **ADOPT FOR INTERNAL** — great for agent alerts, ops notifications. Use Web Push for end-user browser notifications.

---

### 5.3 FCM/APNs (Mobile Push)

- **FCM** (Firebase Cloud Messaging): Free, Google's push service for Android
- **APNs** (Apple Push Notification Service): Free, Apple's push service for iOS

**Rust crates**:
- `a2` (APNs client, MIT)
- `fcm` / `firebase-messaging` (FCM client)

**Pros**:
- Required for native mobile push
- Free (no per-message cost)
- Reliable delivery

**Cons**:
- Requires Firebase/Apple developer accounts
- Platform-specific setup
- Only relevant when ORCHA has native mobile apps

**Roadmap Stage**: Stage 3 — when Tauri mobile builds are production-ready
**Recommendation**: **DEFER** — not needed until mobile app ships

---

### Push Notification Summary

| Component | Recommended Tool | License | Stage | Priority |
|-----------|-----------------|---------|-------|----------|
| Browser push | Web Push API + `web-push` crate | MIT | 1 | High |
| Self-hosted push | ntfy | Apache-2.0/GPL-2.0 | 1-2 | Medium |
| Mobile push (Android) | FCM | Proprietary (free) | 3 | Low |
| Mobile push (iOS) | APNs | Proprietary (free) | 3 | Low |

---

## 6. Existing Codebase Inventory

### What Already Works

| Component | Status | Location |
|-----------|--------|----------|
| STT (5 providers) | Working | `crates/nora/src/voice/stt.rs` |
| TTS (5 providers) | Working | `crates/nora/src/voice/tts.rs` |
| VoiceEngine | Working | `crates/nora/src/voice/engine.rs` |
| VoiceConfig | Working | `crates/nora/src/voice/config.rs` |
| Diarization | Working | `crates/nora/src/voice/diarization.rs` |
| Twilio calls | Working | `crates/nora/src/twilio/` |
| Twilio SMS | Working | `crates/server/src/twilio_sms.rs` |
| Email models | Schema only | `crates/db/src/models/email_account.rs`, `email_message.rs` |
| Tauri desktop | Partially working (merge conflicts) | `src-tauri/` |
| APN desktop app | Working | `apn-app/src-tauri/` |
| Notification service | Sound only | `crates/services/src/services/notification.rs` |
| serenity-voice-model | Vendor stub | `vendor/serenity-voice-model/` |
| libopus | In flox | `.flox/env/manifest.toml` |

### What Is Commented Out / Stubbed

| Component | Status | Location |
|-----------|--------|----------|
| VoiceGateway | Stub module | `crates/nora/src/voice/gateway.rs` |
| VoiceCommandRouter | Not compiled | `crates/nora/src/voice/mod.rs` (commented) |
| BrowserChannel | Not compiled | `crates/nora/src/voice/mod.rs` (commented) |
| GlassesChannel | Not compiled | `crates/nora/src/voice/mod.rs` (commented) |
| Voice routes (full) | Not compiled | `crates/server/src/routes/voice.rs` (references uncompiled types) |
| Push notifications | Placeholder | `NotificationService::send_push_notification()` |

---

## 7. Consolidated Recommendations

### Stage 0 (Dogfood — Now)

| Action | Effort | Impact |
|--------|--------|--------|
| Integrate `silero-vad-rs` for voice activity detection | 1 day | Enables real-time voice pipeline |
| Replace Python whisper_server.py with `whisper-rs` native | 3-5 days | Eliminates Python sidecar |
| Clean up Tauri merge conflicts in `src-tauri/` | 1 day | Unblocks desktop app |
| Add `lettre` for SMTP email sending | 1-2 days | System notification emails |

### Stage 1 (First Pilots — Q3 2026)

| Action | Effort | Impact |
|--------|--------|--------|
| Integrate str0m for browser WebRTC voice | 2-3 weeks | Browser voice input for demos |
| Wire `imap` + `mailparse` for email monitoring | 1 week | Email-to-task automation |
| Integrate Resend for transactional emails | 2-3 days | Client-facing emails |
| Add Web Push API (`web-push` crate + service worker) | 3 days | Browser notifications |
| Tauri system tray integration | 2 days | Desktop UX polish |
| Tauri auto-updater (GitHub Releases) | 2-3 days | Desktop distribution |
| Piper TTS (MIT version) for low-latency local TTS | 2 days | Ultra-fast voice responses |
| ntfy for ops/agent notifications | 1-2 days | Internal alerting |

### Stage 2 (SaaS Launch — Q4 2026+)

| Action | Effort | Impact |
|--------|--------|--------|
| LiveKit integration for multi-party voice | 2-3 weeks | Meeting rooms, agent conferences |
| Matrix protocol integration | 3-4 weeks | Decentralized messaging |
| Local LLM via llama-cpp-2 | 2-3 weeks | Offline agent capabilities |
| WhatsApp via Twilio API | 1 week | Business messaging channel |
| Deepgram as premium STT option | 3-5 days | Real-time streaming STT |

### Stage 3+ (Growth)

| Action | Effort | Impact |
|--------|--------|--------|
| FCM/APNs for mobile push | 1-2 weeks | Native mobile notifications |
| Offline-first sync (cr-sqlite) | 2-4 weeks | Desktop offline mode |
| Pure Rust LLM (mistral.rs/Crane) | 2-3 weeks | Remove C++ dependency |
| Kokoro TTS (Apache-2.0) | 1 week | Replace Piper if GPL fork is needed |

---

### License Risk Register

| Tool | License | Risk Level | Mitigation |
|------|---------|-----------|------------|
| Piper TTS (active fork) | GPL-3.0 | **HIGH** | Pin to archived MIT release only |
| Signal Protocol | GPL-3.0 | **HIGH** | Skip — use Matrix instead |
| ntfy server | GPL-2.0 | **MEDIUM** | Run as separate service, not linked |
| Janus SFU | GPL-3.0 | **HIGH** | Skip — use LiveKit |
| edge-tts | MIT (grey area usage) | **MEDIUM** | Skip — uses MS service without API agreement |
| Coqui XTTS-v2 | Coqui Public Model License | **MEDIUM** | Non-commercial only — use Chatterbox MIT instead |

### Budget Summary (Incremental Monthly Costs)

| Stage | Service | Monthly Cost |
|-------|---------|-------------|
| 0 | All local/open-source | $0 |
| 1 | Resend (transactional email) | $20 |
| 1 | Twilio SMS (existing) | ~$10-50 |
| 2 | LiveKit Cloud (optional) | $50-200 |
| 2 | Deepgram STT (optional) | $25 |
| 2 | Matrix homeserver | $20-50 |
| **Total Stage 2** | | **~$125-345/mo** |

---

## Sources

### WebRTC / Voice
- [str0m - Sans I/O WebRTC for Rust](https://github.com/algesten/str0m)
- [webrtc-rs - Async WebRTC for Rust](https://github.com/webrtc-rs/webrtc)
- [LiveKit Rust SDKs](https://github.com/livekit/rust-sdks)
- [LiveKit Server (Apache-2.0)](https://github.com/livekit/livekit)
- [Rust vs Go WebRTC Benchmark](https://miuda.ai/blog/webrtc-datachannel-benchmark/)
- [libp2p migration to str0m](https://github.com/libp2p/rust-libp2p/issues/3659)

### VAD / STT
- [Silero VAD](https://github.com/snakers4/silero-vad)
- [silero-vad-rs crate](https://crates.io/crates/silero-vad-rs)
- [Best VAD Comparison 2025](https://picovoice.ai/blog/best-voice-activity-detection-vad-2025/)
- [whisper-rs (whisper.cpp bindings)](https://github.com/tazz4843/whisper-rs)
- [whisper-cpp-plus (streaming + VAD)](https://github.com/operator-kit/whisper-cpp-plus-rs)
- [Vosk Speech Recognition](https://github.com/alphacep/vosk-api)

### TTS
- [Piper TTS (MIT, archived)](https://github.com/rhasspy/piper)
- [Piper GPL fork](https://github.com/OHF-Voice/piper1-gpl)
- [piper-rs (Rust bindings)](https://github.com/thewh1teagle/piper-rs)
- [Chatterbox (Resemble AI, MIT)](https://www.resemble.ai/best-open-source-ai-voice-cloning-tools/)
- [Kokoro TTS (Apache-2.0)](https://smallest.ai/blog/open-source-tts-alternatives-compared)
- [Open Source TTS Comparison 2026](https://www.bentoml.com/blog/exploring-the-world-of-open-source-text-to-speech-models)

### Email
- [lettre and imap crates overview](https://blog.logrocket.com/email-crates-for-rust-lettre-and-imap/)
- [mailparse crate](https://github.com/staktrace/mailparse)
- [imap crate](https://github.com/jonhoo/rust-imap)
- [Resend](https://resend.com)
- [Transactional Email APIs Compared 2025](https://www.pingram.io/blog/transactional-email-apis)

### Messaging
- [Matrix Rust SDK](https://github.com/matrix-org/matrix-rust-sdk)
- [Matrix gaining ground in government IT](https://www.theregister.com/2026/02/09/matrix_element_secure_chat/)
- [This Week in Matrix (March 2026)](https://matrix.org/blog/2026/03/13/this-week-in-matrix-2026-03-13/)

### Desktop
- [Tauri 2.0 Stable Release](https://v2.tauri.app/blog/tauri-20/)
- [Tauri System Tray](https://v2.tauri.app/learn/system-tray/)
- [Tauri Auto-Updater](https://v2.tauri.app/plugin/updater/)
- [Tauri + Local LLM Guide](https://codenote.net/en/posts/tauri-cross-platform-app-with-local-llm/)

### Local LLM
- [candle (Hugging Face)](https://github.com/huggingface/candle)
- [llama-cpp-2 crate](https://crates.io/crates/llama-cpp-2)
- [Crane (pure Rust LLM engine)](https://github.com/lucasjinreal/Crane)
- [Building LLM Apps with Rust](https://dasroot.net/posts/2026/01/building-llm-applications-rust-candle-llm-crates/)

### Push Notifications
- [ntfy (self-hosted push)](https://github.com/binwiederhier/ntfy)
- [ntfy Rust library](https://github.com/shadowylab/ntfy)
- [ntfy self-hosted setup guide](https://www.vanwerkhoven.org/blog/2025/my-ntfy-self-hosted-push-notification-setup/)
