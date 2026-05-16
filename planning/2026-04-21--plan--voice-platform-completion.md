# Nora Voice Platform — Live Developer Handoff & Completion Plan

A complete runbook for a new developer picking up P2–P5 of the voice-optimization roadmap. Everything you need: current production state, target architecture, per-phase implementation steps, build/deploy procedures, and risk mitigations.

---

## Part 1 — Current Production State (as of 2026-04-21)

### 1.1 Live topology

```
                               ┌──────────────────────────────────┐
                               │   dashboard.powerclubglobal.com  │
                               └────────────────┬─────────────────┘
                                                │ cloudflared tunnel
                                                ▼
                                     ┌──────────────────────┐
                                     │   pcg-cloudflared    │ docker
                                     └──────────┬───────────┘
                                                │
                                                ▼
                                     ┌──────────────────────┐
                                     │   pcg-nginx :8080    │ docker (serves /app/frontend/dist)
                                     │   proxies /api/ →    │
                                     └──────────┬───────────┘
                                                │
                                                ▼
           ┌──────────────────────────────────────────────────────────────┐
           │                      pcg-cc-mcp :3001                        │ docker
           │                                                              │
           │  axum server (Rust)                                          │
           │  ├── routes/twilio      Phone + SMS webhooks                │
           │  ├── routes/meet        Google Meet bot API + MEET-WATCHER  │
           │  ├── routes/nora        Agent chat + voice config           │
           │  └── workers/           NORA_INBOX, agent flow engine        │
           │                                                              │
           │  In-container processes:                                     │
           │  ├── server (main, tini PID1 supervised)                     │
           │  ├── chatterbox (Python TTS, :8102 → host)                   │
           │  └── ollama (:11434 → host)                                  │
           └─────────┬────────────────────────────────────────────────────┘
                     │ bind-mounted DB /app/dev_assets/db.sqlite
                     ▼
              ┌────────────────────────────────────────────────┐
              │ /home/pythia/pcg-cc-mcp/dev_assets/db.sqlite   │  HOST filesystem
              └────────────────────────────────────────────────┘

HOST-SIDE processes (NOT in Docker):
├── discord-bot-js (plain node, manages NORA#1216 + Topsi#4247 via 2 tokens)
├── meet-bot.js    (Playwright + Chrome, spawned per meeting by supervisor)
├── host-capture.sh (parec audio backup, spawned per meeting by supervisor)
├── meet-bot-supervisor.service (systemd --user, watches docker logs)
├── whisper_server.py (GPU :8101)
├── chatterbox_server.py (GPU :8102) — host copy, Docker also has one
└── pcg-updater (docker, polls GitHub releases, inert without GITHUB_TOKEN)
```

### 1.2 What is already deployed

| Area | Change | Status |
|---|---|---|
| Zoho OAuth | Stale-row dedupe + `ORDER BY updated_at DESC` + persist rotated refresh_token | live |
| Phone → KG | `handle_call_status` upserts `ProjectKnowledgeSource` on transcription_status=completed; Deal scope if `crm_deal_id` present | live |
| Meet/Discord → KG | `handle_end_meeting` upserts Project + Deal scope (resolves `linked_person_ids` → active deals) | live |
| Meet bot stealth | `meet-bot.js` uses `playwright-extra` + `puppeteer-extra-plugin-stealth` | live |
| Chrome | `google-chrome-stable 147` installed at `/usr/bin`; supervisor passes `CHROMIUM_PATH` | live |
| Bot supervisor | `/home/pythia/meet-bot-supervisor/supervisor.sh` + systemd user service; auto-spawns bot + host-capture on invite detection | live |
| Transcript safety net | `host-capture.sh` parec + dual-tee WAV; silence alarm in meet-bot; auto-reroute pactl loop | live |
| Voice instrumentation | `nora_voice_turn_duration_seconds{channel, stage}` histogram at `/api/metrics` | live |
| Phrase cache | 128-entry LRU at `VoiceEngine::synthesize_speech_with_format`, keyed `(text, voice_id)` | live |
| Discord fast path | silence 1000→400ms + classifier awaited→fire-and-forget | live |

### 1.3 Baseline latency (live, from instrumentation)

Current voice turn (end-of-speech → user hears response):

```
                 Stage breakdown (approximate, from NORA_VOICE_TURN histogram)
                 ┌──────────────────────────────────────────────────────────┐
  Twilio phone   │ Gather   LLM (Haiku 4.5) TTS                             │
                 │  ~1s       ~1-2s            ~2-6s        Total: 4-9s    │
                 ├──────────────────────────────────────────────────────────┤
  Google Meet    │ 2s chunk  Whisper  LLM (gpt-4o) TTS       Poll (500ms)  │
                 │  ~2s        ~0.5s   ~1-2s       ~2-6s        0-0.5s      │
                 │                                         Total: 5-11s    │
                 ├──────────────────────────────────────────────────────────┤
  Discord        │ 400ms silence + Whisper  LLM    TTS                     │
                 │  ~0.4s            ~0.5s   ~1-2s   ~2-6s  Total: 4-9s    │
                 └──────────────────────────────────────────────────────────┘

                 Cached greeting (after phrase cache HIT): ~50ms
```

Target after P2–P4:

```
                 ┌──────────────────────────────────────────────────┐
  Any channel    │ VAD  Stream-STT  Stream-LLM  Stream-TTS          │
                 │ 200ms  200ms       400ms       300ms TTFA        │
                 │                                    Total: ~1.1s  │
                 └──────────────────────────────────────────────────┘
                 TTFA = time-to-first-audio
```

### 1.4 Critical gotchas (learned the hard way)

- **Binary swap survives `docker restart` but NOT `docker compose up -d` or `compose build`** — recreate wipes `/usr/local/bin/server` back to image contents. Always re-apply swap after any compose operation.
- **Rust server in Docker cannot spawn `node`** — no Node.js in the container. All host-side automation must go through the supervisor (which already handles it).
- **`pactl` inside the Docker container can't see host PipeWire** — any audio-routing logic must be on the host (moved to meet-bot.js).
- **Meet rejects Playwright + stock Chromium** regardless of sign-in — must use Chrome + stealth plugin.
- **Google can flag an account after a burst of failed joins** — test incrementally and wait for flags to clear after any suspicious burst.
- **Zoho rotates refresh tokens every refresh** — code must persist the rotated value (fixed; don't regress).
- **SingletonLock left over after Chrome crashes** at `/home/pythia/nora-chrome-profile/SingletonLock` — clear before respawn.
- **Moved values** — variables like `transcript` can be moved into `CallLog::update`; downstream uses need `.clone()`.
- **SQLX_OFFLINE=1** is set for builds — new `query!` macros need `.sqlx/` metadata regen OR use runtime `sqlx::query()`/`query_as()`.
- **`pcg-updater`** polls GitHub releases but has no auth token — it will NEVER trigger deploys right now. Don't remove it, but don't trust it either.

---

## Part 2 — Target Architecture

### 2.1 Unified voice pipeline (where we're going)

```
                 ┌───────────────────────────────────────────────────────────┐
                 │                 NORA VOICE CORE                           │
                 │                                                           │
                 │   ┌───────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐    │
 User audio ────►│──►│ VAD   │──►│Streaming│──►│Streaming│──►│Streaming│──► │
                 │   │(webrtc│   │  STT    │   │   LLM   │   │   TTS   │    │
                 │   │  vad) │   │(whisper │   │  (SSE   │   │ (EL-11  │    │
                 │   │       │   │ stream) │   │ tokens) │   │ stream) │    │
                 │   └───┬───┘   └─────────┘   └────┬────┘   └────┬────┘    │
                 │       │                          │             │         │
                 │       │ barge-in interrupt       │             │         │
                 │       │ signal                   │             │         │
                 │       │                          ▼             │         │
                 │       │                   ┌──────────────┐    │         │
                 │       │                   │ Phrase Cache │    │         │
                 │       │                   │  (in-mem +   │    │         │
                 │       │                   │   disk)      │    │         │
                 │       │                   └──────────────┘    │         │
                 │       │                                       │         │
                 │       ▼                                       ▼         │
                 │  [stop playback]                       [sentence chunks │
                 │                                         flow to trans-  │
                 │                                         port adapter]   │
                 │                                                          │
                 │   Metrics: per-stage histogram {channel, stage}         │
                 │   Spans:   tracing::instrument on every stage            │
                 └──────────────┬────────────────────────────────────────────┘
                                │ ChannelAdapter trait
                 ┌──────────────┼──────────────┬─────────────────┐
                 ▼              ▼              ▼                 ▼
        ┌───────────────┐ ┌───────────┐ ┌──────────────┐ ┌──────────────┐
        │ TwilioAdapter │ │MeetAdapter│ │DiscordAdapter│ │  SmsAdapter  │
        │ (Media        │ │ (Playwrt +│ │ (@discordjs/ │ │(text only —  │
        │  Streams WS)  │ │  pactl    │ │  voice)      │ │ future       │
        │               │ │  sinks +  │ │              │ │  adapter)    │
        │               │ │  SSE)     │ │              │ │              │
        └───────────────┘ └───────────┘ └──────────────┘ └──────────────┘
```

### 2.2 Phase dependency DAG

```
       ┌──────────────────────────────────────────────────────┐
       │                                                      │
       │   P5.1 NORA_LLM_MODEL alignment      ◄── do first    │
       │   (trivial config; unlocks consistent measurement)   │
       │                                                      │
       └──────────────────────┬───────────────────────────────┘
                              │
              ┌───────────────┴────────────────┐
              ▼                                ▼
   ┌──────────────────────┐          ┌──────────────────────┐
   │  P2 Streaming TTS    │          │  P3 Streaming STT    │
   │                      │          │  + VAD               │
   │                      │          │                      │
   │  Independent — both  │          │  Provides VAD signal │
   │  can be in parallel  │          │  that P4 barge-in    │
   │  tracks              │          │  consumes            │
   └─────────┬────────────┘          └─────────┬────────────┘
             │                                 │
             └─────────────┬───────────────────┘
                           ▼
              ┌──────────────────────────┐
              │ P4 Barge-in + Phrase     │
              │ cache expansion          │
              │                          │
              │ Needs: VAD signal (P3),  │
              │ streaming TTS output to  │
              │ interrupt (P2)           │
              └────────────┬─────────────┘
                           ▼
              ┌──────────────────────────┐
              │ P5 Consolidation         │
              │                          │
              │ Refactor on top of all   │
              │ the plumbing. Do last.   │
              └──────────────────────────┘

Critical path:  P5.1 → P2 → P4 → P5
Parallel path:  P3 can run alongside P2 with a second developer
```

---

## Part 3 — Per-Phase Implementation Plans

### Phase P5.1 — Align voice LLM model (do first)

**Goal.** All voice paths use `claude-haiku-4-5-20251001`. Eliminates the gpt-4o vs Haiku inconsistency so voice metrics are comparable across channels.

**Non-goals.** Non-voice Nora paths (chat UI, agent flows) stay on whatever `NORA_LLM_MODEL` is set to — this is a voice-only override.

**Changes.**

| File | Change |
|---|---|
| `crates/server/src/routes/meet.rs` (line ~411 in `receive_audio`) | After building `NoraRequest`, add `req.llm_model_override = Some("claude-haiku-4-5-20251001".to_string());` — requires adding that field to `NoraRequest` struct |
| `crates/nora/src/agent.rs` (struct `NoraRequest`) | Add `pub llm_model_override: Option<String>` with `#[serde(default)]` |
| `crates/nora/src/brain.rs` (LLM dispatch) | If `llm_model_override` is set, use it instead of env `NORA_LLM_MODEL` |
| `crates/server/src/routes/nora/chat.rs` (agent_channels) | Keep unchanged — non-voice |
| `discord-bot-js/src/audio.js` (callAgent) | Pass `llm_model_override: "claude-haiku-4-5-20251001"` in request body |

**Sequence diagram.**

```
  MeetHandler      NoraRequest     NoraAgent        Brain         Claude API
      │                │              │               │                │
      ├──construct────►│              │               │                │
      │ llm_model_     │              │               │                │
      │ override="..." │              │               │                │
      │                ├──process────►│               │                │
      │                │              ├──chat_stream─►│                │
      │                │              │ (model=       │                │
      │                │              │  override ??  │                │
      │                │              │  env default) │                │
      │                │              │               ├──POST /messages│
      │                │              │               │  model=haiku───►
```

**Test.**

```
curl -s http://localhost:3001/api/metrics | grep nora_llm_duration_seconds
# After changes, {model="claude-haiku-4-5-20251001"} should appear for all voice turns
```

**Rollback.** Revert the 3 edits; rebuild; redeploy.

---

### Phase P2 — Streaming TTS (headline latency win)

**Goal.** Time-to-first-audio (TTFA) drops from 4–10s to ~1.2s by pipelining LLM → TTS → transport as sentences land instead of batching.

**Non-goals.** Sub-sentence streaming (word-by-word TTS). Modern TTS has poor quality at sub-sentence granularity — sentence-chunking is the sweet spot.

**Architecture.**

```
Current (batch, per channel):
  ──[STT complete]──►[LLM produces FULL response 1-3s]──►[TTS FULL audio 2-6s]──►[play]
                                                              ▲
                                                     4–9s to first byte out

Target:
  ──[STT]──►[LLM stream]──► sentence chunker ──► [TTS stream]──► [play frag]
                │             │                    │               │
                │             ▼                    ▼               ▼
                │         "Got it,"             (audio)         (plays 300ms in)
                │             │                    │
                │         "one moment."          (audio)
                │             │                    │
                └─►complete──►"Let me check." ►(audio)

  TTFA = LLM_first_sentence_latency + TTS_first_chunk_latency ≈ 500ms + 300ms = ~800ms
```

**File-by-file changes.**

```
crates/nora/src/brain.rs
  - New method: chat_stream_sentences() -> impl Stream<Item = SentenceEvent>
  - Wraps existing chat_stream() SSE, buffers tokens, flushes on sentence boundary
    (regex: /[.!?][\s"]|\n\n/)
  - Emits SentenceEvent { text, is_final }

crates/nora/src/voice/tts.rs
  - New trait method: synthesize_speech_stream(req) -> impl Stream<Item = AudioChunk>
  - ElevenLabsTTS impl uses /text-to-speech/{id}/stream endpoint
  - Reads response.bytes_stream() and forwards chunks
  - Fallback: ChatterboxTTS uses non-stream (buffers to single chunk)

crates/nora/src/voice/engine.rs
  - New method: synthesize_stream(text_stream) -> impl Stream<Item = AudioChunk>
  - Consumes SentenceEvents, synthesizes each via TTS stream
  - Concatenates audio chunks downstream

crates/server/src/routes/twilio/nora_integration.rs
crates/server/src/routes/twilio/audio.rs
  - Replace process_with_nora + generate_and_cache_audio with a streaming path
  - Use Twilio Media Streams (WebSocket) instead of Gather + <Play>
  - NEW: routes/twilio/media.rs websocket handler — already exists, needs wiring

crates/server/src/routes/meet.rs
  - Change /nora/meet/{id}/next-tts from long-poll to SSE (text/event-stream)
  - Emit audio chunks as they arrive instead of buffering to tts_queue
  - scripts/meet-bot.js: switch from polling /next-tts to consuming SSE

discord-bot-js/src/audio.js  (or new module)
  - Replace synthesizeTts() call with streamed variant
  - Feed Opus-encoded chunks progressively to @discordjs/voice AudioPlayer
  - AudioPlayer.play(resource) with Readable stream (already supported)
```

**Sequence diagram — Meet channel.**

```
meet-bot.js       Server /audio      Brain           TTS          Server /next-tts
    │                  │                │              │                │
    ├──POST wav────────►│                │              │                │
    │                  ├──STT──►whisper  │              │                │
    │                  ├──chat_stream_sentences──►      │                │
    │                  │                │ sentence1────►│                │
    │                  │                │              │─audio_chunks──►│ (SSE)
    │                  │                │              │                │─►client
    │                  │                │ sentence2────►│                │
    │                  │                │              │─audio_chunks──►│
    │◄─SSE: chunk1─────┼────────────────┼──────────────┼────────────────┤
    │◄─SSE: chunk2─────┼────────────────┼──────────────┼────────────────┤
    │ paplay chunk1 (starts playing ~800ms after user spoke)
    │ paplay chunk2 (seamless continuation)
```

**Transport-specific complexity.**

```
┌────────────┬───────────────────────────────────────────────────┐
│ Transport  │ How to emit streaming audio                       │
├────────────┼───────────────────────────────────────────────────┤
│ Meet       │ SSE replaces poll; meet-bot paplay consumes bytes │
│ Discord    │ @discordjs/voice accepts Readable stream directly │
│ Twilio     │ Media Streams WS (bidirectional, 8kHz mulaw) —    │
│            │ existing media.rs has skeleton; needs:            │
│            │  • MP3 → mulaw 8kHz transcode (ffmpeg in-process) │
│            │  • 20ms frame pacing (Twilio rejects bursts)      │
│            │  • proper mark events for playback tracking       │
└────────────┴───────────────────────────────────────────────────┘

Suggested sequencing: Meet (easiest) → Discord → Twilio (most complex).
```

**Test plan.**

```
# Instrumented A/B: add new stage name "tts_first_byte" to NORA_VOICE_TURN
# Before: nora_voice_turn_duration_seconds{stage="tts"} p50 ≈ 3s
# After:  nora_voice_turn_duration_seconds{stage="tts_first_byte"} p50 ≈ 300ms

# Manual test, all channels:
# 1. Place test call to +15803301101, say "hello Nora"
# 2. Time with stopwatch from end of "Nora" to first syllable of response
# 3. Target: <1.5s

# Discord test: /nora-join + voice ping
# Meet test: send invite to nora@powerclubglobal.com, wait for auto-join
```

**Rollback.** Feature-flag: `VOICE_STREAMING_ENABLED=false` in Docker env skips the stream path and falls back to current batch code. Keep both paths for a validation period.

---

### Phase P3 — Streaming STT + VAD

**Goal.** End-of-speech detection drops from 1–2s to ~300ms. Whisper switches to a streaming variant that emits partial transcripts so the LLM call can start before the user is fully done speaking.

**Architecture.**

```
Today (Meet path shown):
  audio ──► 2s fixed-chunk buffer ──► Whisper full transcribe ──► single transcript
            (can't detect end-of-speech; wastes CPU on silence)

Target:
  audio ──► webrtc-vad ──► speech_start event
     │        │               │
     │        │         start Whisper streaming session
     │        │               │
     │        ▼          partial_transcript events (every ~150ms)
     │    silence detection     │
     │      (>300ms)            │
     │        │          final_transcript event
     │        ▼               │
     │    end-of-speech ──────┼──► fire LLM call (partials gave us a head start)
     │                        │
     └────────────────────────┘
```

**Components.**

| Component | Swap | Rationale |
|---|---|---|
| Chunking (meet-bot.js) | Fixed 2s PCM windows → webrtc-vad segmentation | Ends chunk at silence, not arbitrary timer |
| Whisper server | openai-whisper → **faster-whisper** (2-4x faster) | Needed for partial transcripts |
| STT endpoint | `/transcribe/file` → `/transcribe/stream` (WebSocket) | Emits partial results |
| Discord silence | Already 400ms | No change |
| Twilio | Gather handles its own VAD | No change in P3 |

**File-by-file.**

```
scripts/whisper_server.py
  - pip install faster-whisper
  - Replace OpenAI whisper model with faster_whisper.WhisperModel
  - Add new endpoint: WS /transcribe/stream
    Client sends raw PCM frames
    Server emits: {type: "partial", text}, {type: "final", text}

scripts/meet-bot.js
  - npm install node-webrtcvad
  - Replace fixed-chunk loop with VAD-gated segmentation
  - Open WS to /transcribe/stream, send PCM frames in real-time
  - On "final" event, POST to /api/nora/meet/{id}/audio with transcript
    (bypass server-side Whisper call — server just needs the text now)
  
crates/server/src/routes/meet.rs
  - Accept pre-transcribed POST variant: body {transcript: "...", audio_ref: "..."}
  - If transcript present, skip call_whisper, use provided text

crates/nora/voice/stt.rs
  - Update WhisperSTT to use /transcribe/stream for the Rust-driven paths
    (Twilio calls, etc. if applicable)
```

**Sequence diagram.**

```
  parec ──► meet-bot.js (VAD)     whisper-server (WS)     server /audio
    │          │                        │                      │
    │ frames──►│                        │                      │
    │          │ idle (silence)         │                      │
    │ frames──►│                        │                      │
    │          │ vad:speech_start       │                      │
    │          ├──────WS open──────────►│                      │
    │ frames──►│─────frames────────────►│                      │
    │          │                        │ partial: "hey..."    │
    │          │◄──partial──────────────┤                      │
    │ frames──►│─────frames────────────►│                      │
    │          │                        │ partial: "hey nora"  │
    │          │◄──partial──────────────┤                      │
    │          │ vad:silence (300ms)    │                      │
    │          ├──────end────────────► │                      │
    │          │                        │ final: "hey nora"    │
    │          │◄──final────────────────┤                      │
    │          ├──POST transcript──────────────────────────────►│
    │          │                        │                      │─► LLM (stream)
```

**Test.**

```
# A/B against silence:
# Before: 2s fixed chunk even for 50ms "yes" utterance
# After:  "yes" → end-of-speech in ~350ms

# nora_voice_turn_duration_seconds{stage="stt"} histogram should show:
#   p50 drop from ~500ms to ~200ms
#   p99 drop from ~30s (!) to ~1s (the invalid-WAV-timeout case)

# Latency to LLM start, measured end-to-end:
#   Before: 2.5s (chunk + whisper)
#   After:  450ms (VAD end + final transcript)
```

---

### Phase P4 — Barge-in + expanded phrase cache

**Goal.** User can interrupt Nora mid-sentence. Common phrases (greetings, acks, fillers) are pre-rendered to disk so they play instantly even on first synthesis.

**Barge-in state machine.**

```
                    ┌───────────────┐
                    │   LISTENING   │◄─────────────────────┐
                    │ (waiting for  │                      │
                    │  user speech) │                      │
                    └───────┬───────┘                      │
                     VAD    │                              │
                     detects│                              │
                     speech │                              │
                            ▼                              │
                    ┌───────────────┐                      │
                    │   USER_TURN   │                      │
                    │  processing   │                      │
                    │  transcribe   │                      │
                    └───────┬───────┘                      │
                     final  │                              │
                     trans- │                              │
                     cript  │                              │
                            ▼                              │
                    ┌───────────────┐                      │
                    │  NORA_TURN    │                      │
                    │  speaking     │──VAD detects user    │
                    │  (streaming   │  speech──────┐       │
                    │   TTS out)    │              │       │
                    └───────┬───────┘              │       │
                     TTS    │                      ▼       │
                     done   │              ┌───────────────┐
                            │              │  INTERRUPTED  │
                            │              │ stop TTS      │
                            │              │ drop queue    │
                            │              │ return to     │
                            │              │ USER_TURN     │
                            │              └───────┬───────┘
                            │                      │
                            ▼                      ▼
                    ┌───────────────┐              │
                    │   LISTENING   │◄─────────────┘
                    └───────────────┘
```

**Implementation.**

```
scripts/meet-bot.js + discord-bot-js/src/index.js
  - Add VAD on OUTPUT side (monitor nora's own TTS playback)
    NOT on input mic — we already have that
  - Actually: monitor input VAD while Nora's playing; if speech detected
    > 500ms DURING TTS playback, send {event: "barge-in"} to server
  - On barge-in:
      paplay.kill() / audioPlayer.stop()
      clear tts queue / discard pending sentences

crates/server/src/routes/meet.rs
  - New endpoint: POST /nora/meet/{id}/interrupt
    Cancels in-flight TTS synthesis for this session
    Drops any queued audio chunks
    (Streaming TTS from P2 must support cancellation tokens)

crates/nora/src/voice/engine.rs
  - synthesize_stream returns (AudioStream, AbortHandle)
  - Caller can drop AbortHandle to cancel mid-synthesis

Phrase cache expansion:
  crates/nora/src/voice/engine.rs
  - Add startup pre-render of ~20 common phrases to disk:
      "Hello", "One moment", "Got it", "Let me check on that",
      "Could you repeat that?", "Certainly", "Thanks for holding",
      "I'll transfer you", "Is there anything else?", "Goodbye"
  - Disk path: /app/nora-phrase-cache/{hash}.{mp3|wav}
  - Bind-mount /home/pythia/nora-phrase-cache → /app/nora-phrase-cache
  - In-memory LRU stays as hot-cache layer in front of disk
```

**Test.**

```
# Barge-in test (Meet):
# 1. Ask Nora a long question
# 2. While she's responding, say "actually, wait"
# 3. She should stop speaking within ~500ms of your interruption starting

# Phrase cache test:
# curl http://localhost:3001/api/metrics | grep nora_phrase_cache_hits_total
# After ~10 conversational turns, hits_total should grow (common phrases repeat)
```

---

### Phase P5 — Consolidation (do last, informed by P2–P4 learnings)

**Goal.** Refactor the working voice plumbing behind a clean abstraction. Reduces per-channel special cases and makes adding a 4th/5th channel (WhatsApp voice, SIP, browser WebRTC) a low-friction task.

**Component diagram.**

```
            ┌──────────────────────────────────────┐
            │           VoiceEngine                │
            │  (stt, tts, phrase_cache, metrics)  │
            └───────────────┬──────────────────────┘
                            │
            ┌───────────────▼──────────────────────┐
            │         ChannelAdapter trait         │
            │  async fn enter_session()            │
            │  async fn on_audio_chunk(AudioRef)   │
            │  async fn emit_response(TtsStream)   │
            │  async fn on_interrupt()             │
            │  async fn leave_session()            │
            └─┬──────────┬──────────┬──────────┬───┘
              │          │          │          │
              ▼          ▼          ▼          ▼
       ┌──────────┐ ┌────────┐ ┌─────────┐ ┌──────┐
       │ Twilio   │ │  Meet  │ │ Discord │ │ SMS  │
       │ Media-   │ │ adapter│ │ adapter │ │ (no  │
       │ Stream   │ │        │ │         │ │ stream)│
       └──────────┘ └────────┘ └─────────┘ └──────┘
```

**Prompts → YAML.**

```
Before (current):
  crates/server/src/routes/twilio/mod.rs:198
    const NORA_PCG_TEAM_SYSTEM: &str = "You are Nora...";

After:
  prompts/
    nora_pcg_team.yaml
    nora_client.yaml
    nora_meet_prefix.yaml
    nora_discord_prefix.yaml
    topsi_system.yaml

  YAML format:
    id: nora_pcg_team
    description: System prompt for Nora on PCG team phone calls
    version: 3
    body: |
      You are Nora, an Executive AI Assistant...
      
  Rust side: crates/nora/src/prompts.rs
    struct PromptRegistry {
      prompts: HashMap<String, String>,
      file_watcher: RecommendedWatcher,  // hot-reload on yaml change
    }
    
  Hot reload: edit YAML, changes apply on next request (no restart)
```

**Wake-word upgrade.**

```
Current (Meet): transcript.to_lowercase().contains("nora")
  → false triggers on "know about", "a notary", etc.

Target:
  - Whisper's word timestamps available in faster-whisper output
  - Check isolated-word match: is there a token "nora" or "norah"?
    Not just substring.
  - Phonetic fallback regex (steal from Topsi's detectWakeWord): 
    /\bn[aeo]r[aeh]\b/i
  - Log false-positive rate via a metric; tune threshold
```

**Rust discord-bots crate retirement.**

```
crates/discord-bot/ and crates/discord-bots/ exist but are disabled
  (see crates/server/src/main.rs:138 comment).
They conflict with the JS bot using same tokens.

Action: 
  - Delete both crates from workspace
  - Remove from Cargo.toml workspace members
  - Saves compile time per release build
  - Single source of truth: discord-bot-js/
```

**LLM model alignment (finishes P5.1).**

```
In addition to P5.1's per-request override, introduce a central config:

crates/nora/src/config.rs
  struct VoiceLlmConfig {
    phone:   String,  // "claude-haiku-4-5-20251001"
    meet:    String,  // "claude-haiku-4-5-20251001"
    discord: String,  // "claude-haiku-4-5-20251001"
    chat:    String,  // "gpt-4o" or whatever
  }
  Loaded from voice_llm_config.yaml; channel-specific override.
```

---

## Part 4 — Build & Deploy Playbook

### 4.1 Local build (fast path)

```
cd /home/pythia/pcg-cc-mcp && \
set -a && source .env && set +a && \
CMAKE_POLICY_VERSION_MINIMUM=3.5 \
SQLX_OFFLINE=1 \
RUST_ENV=development \
OPENSSL_NO_VENDOR=1 \
OPENSSL_DIR=/usr \
OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu \
OPENSSL_INCLUDE_DIR=/usr/include \
cargo build --release -p server
```

### 4.2 Deploy (binary swap — fastest, survives restart but NOT recreate)

```
docker cp target/release/server pcg-cc-mcp:/usr/local/bin/server.new && \
docker exec -u root pcg-cc-mcp sh -c '
  chmod +x /usr/local/bin/server.new && \
  cp /usr/local/bin/server /usr/local/bin/server.bak && \
  mv /usr/local/bin/server.new /usr/local/bin/server
' && \
docker restart pcg-cc-mcp
```

### 4.3 Deploy (full rebuild — use when image layer needs refresh, e.g. new pnpm deps)

```
cd /home/pythia/pcg-cc-mcp && \
docker compose build --no-cache app && \
docker compose up -d
# WIPES any binary swap. Re-apply after.
```

### 4.4 Rollback procedure

```
# Within container:
docker exec -u root pcg-cc-mcp sh -c '
  mv /usr/local/bin/server.bak /usr/local/bin/server
' && \
docker restart pcg-cc-mcp

# If container itself is borked:
docker compose down && docker compose up -d
# (uses last-built image from local docker registry)
```

### 4.5 Verification checklist after every deploy

```
# 1. Health endpoints
curl -sf http://localhost:3001/api/health | jq .
curl -sf http://localhost:3001/api/twilio/health | jq .

# 2. Metric families present
curl -s http://localhost:3001/api/metrics | grep -c '^# TYPE'
# Should be >15 after some voice activity

# 3. Zoho working (no 400 spam)
docker logs pcg-cc-mcp --since 2m 2>&1 | grep -c '400 Bad Request'
# Should be 0

# 4. Meet auto-join path (synthetic)
sqlite3 dev_assets/db.sqlite "SELECT MAX(created_at) FROM meeting_segments"

# 5. Supervisor healthy
systemctl --user status meet-bot-supervisor.service --no-pager
tail /tmp/meet-supervisor.log
```

---

## Part 5 — Risk Matrix

```
┌─────────────────────────────────────┬──────┬────────┬──────────────────────────────────┐
│ Risk                                │ Prob │ Impact │ Mitigation                       │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ Google bot-detection catches up to  │ MED  │ HIGH   │ Stealth versioning; rotate       │
│ stealth plugin again                │      │        │ accounts; fall back to raw audio │
│                                     │      │        │ capture (host-capture.sh live)   │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ ElevenLabs stream endpoint rate     │ LOW  │ MED    │ Phrase cache absorbs repeats;    │
│ limits under load                   │      │        │ Chatterbox fallback configured   │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ Twilio Media Streams WS drops       │ MED  │ MED    │ Close call gracefully; fall back │
│ mid-call                            │      │        │ to <Play> chain; log the event   │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ Container recreate wipes binary     │ HIGH │ MED    │ Document + script binary swap    │
│ swap during unrelated work          │      │        │ as post-deploy-hook; pcg-updater │
│                                     │      │        │ is inert so low risk today       │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ Zoho refresh-token rotation breaks  │ LOW  │ HIGH   │ Already patched — get_zoho_token │
│ again after a pattern change        │      │        │ persists rotations now           │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ Process-local in-memory maps        │ LOW  │ HIGH   │ Multi-instance deploy is NOT     │
│ (CALL_DB_CONTEXTS) split across     │      │        │ supported today — single         │
│ replicas                            │      │        │ instance only. Flag before any   │
│                                     │      │        │ Fly.io scale-out.                │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ faster-whisper GPU memory conflict  │ MED  │ MED    │ Chatterbox uses ~2GB, whisper    │
│ with Chatterbox on same RTX 3080 Ti │      │        │ small ~2GB, faster-whisper       │
│                                     │      │        │ ~4GB → total 8GB / 12GB. Fine.   │
│                                     │      │        │ Test under concurrent load.      │
├─────────────────────────────────────┼──────┼────────┼──────────────────────────────────┤
│ Barge-in false positives (user's    │ MED  │ LOW    │ Require 500ms+ speech before    │
│ own breath / background noise       │      │        │ interrupting; tune threshold     │
│ interrupts Nora)                    │      │        │ per-channel via metrics          │
└─────────────────────────────────────┴──────┴────────┴──────────────────────────────────┘
```

---

## Part 6 — Essential References

- Working directory: `/home/pythia/pcg-cc-mcp`
- Live DB: `/home/pythia/pcg-cc-mcp/dev_assets/db.sqlite` (bind-mounted to Docker)
- Supervisor: `/home/pythia/meet-bot-supervisor/supervisor.sh` + systemd user service
- Chrome profile: `/home/pythia/nora-chrome-profile` (signed in as `powerclubglobalmgmt@gmail.com`)
- Recordings: `/home/pythia/meet-recordings/` (dual-tee WAVs from host-capture)
- Supervisor log: `/tmp/meet-supervisor.log`
- Metrics endpoint: `http://localhost:3001/api/metrics`
- Public health: `https://dashboard.powerclubglobal.com/api/health`
- Twilio webhook base: `https://dashboard.powerclubglobal.com/api/twilio/*`
- ElevenLabs voice ID: `ZtcPZrt9K4w8e1OB9M6w` (Mia Moore, British female)
- Session memory: `/home/pythia/.claude/projects/-home-pythia-topos/memory/voice-pipeline.md`

---

This plan is concrete enough for a new developer to start on P5.1 without additional context. Each phase can be picked up and shipped independently as long as the dependency DAG in Part 2.2 is respected.
