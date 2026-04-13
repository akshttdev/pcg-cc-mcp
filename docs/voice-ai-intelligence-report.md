# Voice AI Orchestration — Competitive Intelligence Report
**Compiled:** April 2026 | **Sources:** GitHub, Product Hunt, X/Twitter, YouTube, Reddit, Hackathons

---

## EXECUTIVE SUMMARY

The voice AI market has bifurcated into two layers: **infrastructure frameworks** (STT+LLM+TTS primitives) and **application platforms** (no-code builders, enterprise SaaS). A third layer is emerging — **speech-to-speech (S2S) models** that collapse the full pipeline into a single neural pass, cutting latency by 60-70%.

**Key numbers:**
- Sub-300ms latency is now achievable (pipeline-optimized)
- OpenAI Realtime API measured at **1.68-1.78s** end-to-end (not 300ms as claimed)
- Cartesia Sonic-3 TTS: **<100ms TTFB** — fastest in class
- Deepgram Nova-3 STT: **~118ms** TTFT
- LiveKit semantic turn detector: **85% reduction** in false interruptions
- GibberLink demo: **15M views in one week** — the most viral voice AI moment of 2025

---

## PART 1: OPEN-SOURCE FRAMEWORKS

---

### 1. Pipecat — `github.com/pipecat-ai/pipecat` ⭐ 10k+
**By:** Daily.co | **Language:** Python

The most flexible open-source voice AI framework. Composable `FrameProcessor` pipeline — every service (STT, LLM, TTS) is a node, frames flow between nodes in real time.

**What makes it stand out:**
- **100+ service integrations** — any STT, LLM, TTS, transport combination
- **Whisker debugger** — watches every audio frame travel through pipeline stages live
- **Tail dashboard** — terminal with real-time latency, token metrics
- **Smart-Turn model** — open-source transformer for semantic turn detection
- **Pipecat Flows** — visual conversation path designer with branching logic
- **NVIDIA collaboration** — achieves 500-700ms V2V with Parakeet + Llama + Magpie NIM stack

**NVIDIA optimized pipeline (nemotron-january-2026):**
```
STT (Parakeet): 30-50ms
LLM (Llama 3.3 70B): 100-150ms  
TTS (Magpie): 370ms TTFB
Total: 500-700ms end-to-end
```

**Links:** https://github.com/pipecat-ai/pipecat | https://www.pipecat.ai | https://docs.pipecat.ai

---

### 2. LiveKit Agents — `github.com/livekit/agents` ⭐ 9.7k
**By:** LiveKit | **Language:** Python + TypeScript (AgentsJS)

Production-grade voice agent framework on top of LiveKit's WebRTC SFU (18.1k stars).

**What makes it stand out:**
- **Semantic turn detection** — 135M-param transformer (`livekit/turn-detector` on HuggingFace), 85% fewer false interruptions, only 3% false negatives
- **Native MCP support** — agents use any MCP tool server out of the box
- **Multi-agent handoff** — state-preserving handoff between specialized agents mid-call
- **Agents UI** — React component library with 5 visualizer types: bar, grid, radial, wave, **aura** (WebGL shader)
- **Playground:** https://agents-playground.livekit.io/

**Semantic turn detection architecture:**
```
SmolLM v2 135M params (CPU-based)
Maintains 4-turn sliding context window
Generates confidence: "has this turn ended?"
Dynamically extends VAD silence timeout
Result: 85% fewer interruptions
```

**Links:** https://github.com/livekit/agents | https://livekit.com | https://docs.livekit.io/agents

---

### 3. OpenAI Realtime Agents — `github.com/openai/openai-realtime-agents` ⭐ High
**By:** OpenAI | **Stack:** Next.js + WebRTC DataChannel

The reference architecture for multi-agent voice systems.

**Two canonical patterns:**

**Pattern A — Chat-Supervisor:**
```
Fast model (gpt-4o-realtime-mini) → handles conversation
            ↓ complex question
Supervisor (o4-mini) → reasoning + tool calls
            ↓ result
Fast model → responds to user
```

**Pattern B — Sequential Handoff:**
```
Auth Agent → Returns Agent → Sales Agent → Human Escalation
(o4-mini decides routing at each boundary)
```

**What makes it stand out:**
- Tool calls shown **inline in the transcript** with type labels
- Agent identity displayed mid-conversation ("now speaking with: Returns Agent")
- `o4-mini` as orchestration decision engine, not just chatbot
- Real-time event log with expandable JSON for every client/server event

**Measured latency (webrtcHacks packet capture):** 1.66-1.78s end-to-end | STUN RTT: 60-70ms

**Links:** https://github.com/openai/openai-realtime-agents

---

### 4. TEN Framework — `github.com/TEN-framework/ten-framework` ⭐ 10.4k
**By:** Agora | **Languages:** C++, Python, TypeScript, Go

The only framework with a **visual node-graph pipeline editor (TMAN Designer)**.

**What makes it stand out:**
- **TMAN Designer** — drag-and-drop to wire VAD → STT → LLM → TTS → avatar as live nodes
- Extensions as independently-deployed composable microservices
- **Sub-500ms** end-to-end latency
- SIP phone, ESP32 edge devices, lip-sync avatars (Live2D, HeyGen, Tavus)
- Multi-modal parallel streams: audio + video + data

**TMAN Designer concept:**
```
[Mic Input] ──→ [VAD] ──→ [STT: Deepgram]
                              ↓
                         [LLM: Claude]
                              ↓
               [Memory] ←──  ↓  ──→ [Tool Calls]
                              ↓
                         [TTS: ElevenLabs]
                              ↓
                         [Avatar: HeyGen]
```

**Links:** https://github.com/TEN-framework/ten-framework | https://theten.ai | Demo: https://agent.theten.ai

---

### 5. Vocode Core — `github.com/vocodedev/vocode-core`
**By:** Vocode | **Language:** Python

Multi-channel voice deployment — same agent runs on phone, Zoom, or browser without code changes.

**Links:** https://github.com/vocodedev/vocode-core | https://app.vocode.dev

---

### 6. Ultravox — `github.com/fixie-ai/ultravox`
**By:** Fixie.ai | **Base:** Llama/Mistral/Gemma/Qwen

Skips STT entirely — audio goes directly into LLM embedding space.

**Architecture:**
```
Traditional: Audio → [STT] → Text → [LLM] → Text → [TTS] → Audio
Ultravox:    Audio → [Multimodal Projector] → [LLM] → Text → [TTS] → Audio
             (preserves tone, emotion, prosody from audio)
```

**Specs:** ~150ms TTFT on A100-40GB | 50-100 tok/s | 15 languages | Planned: native audio token output (no TTS stage)

**Links:** https://github.com/fixie-ai/ultravox | https://demo.ultravox.ai

---

### 7. Microsoft VibeVoice — `github.com/microsoft/VibeVoice`

**Models:**
- **VibeVoice-Realtime-0.5B** — streaming TTS, 300ms TTFB, 9 languages, 11 voice styles
- **VibeVoice-1.5B** — long-form TTS, up to 90-minute synthesis
- **VibeVoice-ASR-7B** — 60-minute single-pass ASR with WHO + WHEN + TIMESTAMP output

**Architecture:** Next-token diffusion — LLM for textual context + diffusion head for acoustic detail. 7.5Hz frame rate (dramatically fewer tokens than competitors).

**Links:** https://github.com/microsoft/VibeVoice | ASR playground: https://aka.ms/vibevoice-asr

---

## PART 2: JARVIS-LIKE VISUAL INTERFACES

---

### 8. ethanplusai/jarvis — `github.com/ethanplusai/jarvis`
**Stack:** FastAPI + Three.js + Claude + Fish Audio

The most complete MCU-style Jarvis implementation with a real Three.js orb.

**What makes it stand out:**
- **Audio-reactive particle orb** — `orb.ts` maps WebSocket audio amplitude to particle displacement + rotation speed in real time
- **Dual-model approach** — Claude Haiku for speed, Claude Opus for depth (branched per task complexity)
- **Claude Code integration** — can spawn real coding sessions via voice command
- **Native macOS** — AppleScript for Calendar, Mail, Notes (no OAuth)
- **FTS5 SQLite** — persistent memory with full-text search
- **Playwright** — browser automation via voice

**Orb implementation:**
```glsl
// Vertex shader — simplex noise displacement from audio
uniform float uAmplitude; // driven by WebSocket audio binary
void main() {
  vec3 newPos = position + normal * snoise(position + uTime) * uAmplitude;
  gl_Position = projectionMatrix * modelViewMatrix * vec4(newPos, 1.0);
}
```

**Links:** https://github.com/ethanplusai/jarvis

---

### 9. aguscruiz/voiceorb — `github.com/aguscruiz/voiceorb`
**Stack:** Three.js + GLSL + Web Audio FFT + Web Speech API

Purest implementation of a voice-reactive GLSL orb.

**Technical details:**
- 256-point FFT, analyzes bins 10-40 (mid-range voice frequencies)
- Vertex shader: simplex noise displacement driven by audio amplitude → organic sphere deformation
- Fragment shader: dynamic color gradients + Fresnel edge glow intensifies during speech
- Smooth interpolation prevents jarring transitions
- Frequency color mapping: low freq (85-150Hz) → open vowels O/U; high (200-255Hz) → closed vowels

**Links:** https://github.com/aguscruiz/voiceorb

---

### 10. harsh-raj00/my-jarvis — `github.com/harsh-raj00/my-jarvis`
**Stack:** React 18 + Three.js + React Three Fiber + Framer Motion + FastAPI + Gemini + ElevenLabs

**What makes it stand out:**
- **8,000-particle sphere** that morphs and reacts to audio
- Stark Industries boot sequence animation
- Arc Reactor visualization
- 6 auto-discovered plugins (smart home, system monitoring, etc.)

**Live:** https://jarvis-frontend-uj30.onrender.com
**Links:** https://github.com/harsh-raj00/my-jarvis

---

### 11. cam-hm/jarvis — `github.com/cam-hm/jarvis`
**Stack:** FastAPI + Three.js + Google Gemini (free) + Piper TTS + Docker

Free Jarvis clone with Arc Reactor holographic UI. Custom JARVIS voice model from Hugging Face. Procedurally generated Arc Reactor spins faster when JARVIS speaks.

**Live demo:** https://jarvis-hn07.onrender.com
**Links:** https://github.com/cam-hm/jarvis | https://dev.to/cammanhhoang/i-built-a-free-jarvis-ai-clone-and-you-can-too-5846

---

### 12. Open-LLM-VTuber — `github.com/Open-LLM-VTuber/Open-LLM-VTuber` ⭐ 6.7k

**What makes it stand out:**
- **Live2D avatar** with 50+ ARKit morph targets, emotion → facial expression mapping, lip sync
- **Desktop pet mode** — transparent background, always-on-top, click-through, draggable anywhere
- **AI thought bubbles** — AI displays internal state visually without speaking it
- **Proactive AI speech** — AI initiates conversation without being prompted
- 100% offline capable

**Links:** https://github.com/Open-LLM-VTuber/Open-LLM-VTuber

---

### 13. isair/jarvis — `github.com/isair/jarvis` ⭐ Active
**Stack:** Python + Ollama + Whisper + Piper TTS + 500+ MCP tools

The most faithful "always-on ambient AI" concept.

**What makes it stand out:**
- Wake word embedded naturally **mid-sentence** ("Hey Jarvis, what do you think?")
- **500+ MCP integrations** — Home Assistant, Google Workspace, GitHub, Notion, Slack, Composio
- **24/7 persistent memory** with full-text search across all sessions
- Automatic sensitive data redaction
- Local GeoIP — no external lookups
- Adaptive personalities: Developer, Business, Life Coach
- 100% private, no cloud, no subscription

**Links:** https://github.com/isair/jarvis

---

### 14. AgoraIO RPM Agent — `github.com/AgoraIO-Community/RPM-agora-agent`
**Stack:** React + React Three Fiber + Agora RTC + ReadyPlayer.me GLB

Full 3D avatar with real-time lip sync.

**Lip sync implementation:**
```
Web Audio API FFT (real-time frequency analysis)
Low freq (85-150Hz) → open vowels (A, O, U)
High freq (200-255Hz) → closed vowels (E, I)
Exponential interpolation for smooth animation
Sub-50ms audio-to-visual sync latency
```

**Live demo:** https://agoraio-community.github.io/RPM-agora-agent
**Links:** https://github.com/AgoraIO-Community/RPM-agora-agent

---

### 15. OpenVoiceUI — `openvoiceui.com`
**Stack:** Next.js + Node.js + OpenAI/Claude/Groq + voice cloning

**The most unique concept in this entire space — the AI generates its own UI.**

Say "show me a sales dashboard" → AI generates and renders a complete interactive HTML page live in a canvas area. Sub-agents run in parallel with outputs appearing in the canvas. 35+ built-in skills. Voice-only operation with wake word.

**Links:** https://openvoiceui.com | https://dev.to/mcerqua/openvoiceui-ai-voice-agent-app-generates-live-canvas-pages-using-openclaw-33i9

---

### 16. The Orb (BoltAI) — `theorb.boltai.com`
**Stack:** OpenAI Realtime API + Three.js Bruno Simon organic sphere shader + PWA

Purest "just the orb" philosophy — no menus, no panels, the orb IS the interface.

**What makes it stand out:**
- Real-time **cost ticker** as first-class UI element
- **Render quality selector** (Low/Medium/High) for accessibility
- **PWA installable** — works like native app on mobile
- `seed` parameter for reproducible orb character per deployment

**Links:** https://theorb.boltai.com

---

### 17. Sesame AI — Maya and Miles
**By:** Sesame AI (backed by Sequoia, a16z) | **Raised:** $47.5M Series A

The most human-sounding voice AI demo ever released. Crossed the uncanny valley.

**Tech:** CSM-1B (Conversational Speech Model) — 1B param multimodal transformer on RVQ tokens via Mimi codec, 12.5Hz. Apache 2.0 licensed.

**What makes it stand out:**
- Takes breaths, uses disfluencies, can be interrupted mid-sentence
- Screenplay-style character design (directors + screenwriters, not engineers)
- Context-aware emotional adaptation in real time
- **1 million users** in first weeks; **5 million+ minutes** of conversation

**Links:** https://www.sesame.com/research/crossing_the_uncanny_valley_of_voice | https://app.sesame.com

---

## PART 3: COMMERCIAL PLATFORMS

---

### Platform Comparison Matrix

| Platform | Type | Latency | Key Differentiator | Pricing |
|----------|------|---------|-------------------|---------|
| **Vapi** | API platform | 465-800ms | 1,500+ integrations, BYOM, 17k Discord | $0.05/min |
| **Retell AI** | Enterprise SaaS | ~620ms | Proprietary turn model, 30+ languages | $0.07/min |
| **Bland AI** | High-volume SaaS | <2s | Self-hosted, air-gapped, HIPAA/SOC2 | $0.04-0.09/min |
| **Hume EVI** | Emotion AI API | ~300ms TTFB | Detects emotional cues from prosody, 100K devs | Free to start |
| **ElevenLabs** | TTS + Agents | 75ms TTS | 10K+ voices, 70+ languages, Conversational AI 2.0 | Subscription |
| **Deepgram** | STT + Voice Agent | ~118ms STT | EOT detection, $4.50/hr flat rate | $4.50/hr |
| **LiveKit** | OSS + Cloud | <600ms | Semantic VAD, MCP native, physical AI | 1K min free |
| **Pipecat** | OSS Framework | 500-700ms | 100+ integrations, Whisker debugger | Free |
| **Synthflow** | No-code SaaS | ~400ms | 65M+ calls, carrier-grade network | $0.08/min |
| **Voiceflow** | Visual builder | — | V4 Playbooks + Workflows, enterprise | ~$30K+/yr |
| **Cartesia** | TTS API | <100ms | SSM architecture, 4x faster than alt | Usage-based |
| **Smallest.ai** | Full-stack | <100ms TTS | Graph agent architecture, token tracing | — |
| **PolyAI** | Enterprise only | ~780ms | In-house models, QA 100% of calls | Custom |
| **Tavus** | Video + Voice | <500ms | Full 3D avatar lip sync at scale | Custom |
| **Speechmatics** | STT only | <250ms partial | Medical model, 55+ languages | $0.24/hr |
| **Play.ai** (Meta) | SaaS + API | ~300ms | Ultra-realistic TTS, Meta-backed | Free 30 min |

---

### Deep Dives

#### Vapi — The AWS of Voice AI
- 300M+ calls processed; 2.5M+ assistants; 500K+ developers
- Agent Squads: chain specialized agents within a single call
- Flow Studio: visual drag-and-drop conversation designer
- A/B testing for prompts, voices, flows
- YAML-as-code: manage via Git
- SOC 2 Type II, HIPAA, PCI certified
- **Links:** https://vapi.ai

#### Hume EVI — Empathic Voice Interface
- Analyzes **tone, rhythm, timbre** to detect emotional cues
- 100,000+ custom voices with inferred personalities
- Natural language voice design ("describe a voice in words")
- Benchmark: **outperforms GPT-4o and Gemini Live** at ~1.2s practical latency
- **Links:** https://www.hume.ai/empathic-voice-interface | Playground: https://app.hume.ai/evi/playground

#### Cartesia Sonic-3 — Fastest TTS
- **State Space Model (SSM) architecture** — not transformer-based
- SSMs maintain running state → stream without reprocessing prior tokens
- **90ms TTFA claimed; ~199ms self-serve; 40 languages**
- Available on AWS SageMaker JumpStart (Feb 2026)
- **Links:** https://cartesia.ai/sonic

#### Deepgram — EOT Detection
- **End-of-Thought (EOT) detection** — eliminates the #1 cause of awkward pauses
- Nova-3: 25% fewer errors than Microsoft; 50% fewer than AssemblyAI
- Voice Agent API (June 2025): unified STT+LLM+TTS in one call
- IBM partnership for enterprise (Feb 2026)
- **Links:** https://deepgram.com/product/voice-agent-api

---

## PART 4: LATENCY & TECHNICAL BENCHMARKS

---

### End-to-End Latency Reality Check

| Platform | Measured | Notes |
|----------|----------|-------|
| Telnyx | <200ms | Owns full infra stack |
| Synthflow | ~400ms | Self-reported |
| Retell AI | ~620ms | Independent benchmark |
| Vapi | 465ms (optimized) / 1.5s+ (default) | Requires manual tuning |
| PolyAI | ~780ms | — |
| OpenAI Realtime API | **1.68-1.78s** | Packet capture measurement |
| Google Dialogflow | ~920ms | — |
| Twilio | 950-1,040ms | — |

**Human threshold:** >300-500ms feels slow. >800ms causes users to repeat themselves.

---

### STT Benchmark (WER — lower is better)

| Provider | WER English | WER Multi | TTFT | Notes |
|----------|------------|-----------|------|-------|
| AssemblyAI Universal-3 Pro | **5.9%** | 8.7% | ~270ms | Best accuracy |
| ElevenLabs Scribe V2 | 6.5% | 8.1% | — | Strong English |
| OpenAI Whisper/GPT-4o | 6.5% | **7.4%** | — | Best multilingual |
| Amazon Transcribe | 7.6% | 10.1% | — | — |
| Microsoft Azure | 7.5% | 11.1% | — | Struggles in noise |
| Deepgram Nova-3 | 8.1% | 10.8% | **~118ms** | Trades accuracy for latency |
| NVIDIA Parakeet 1.1B | — | — | **~72ms** | GPU required, open source |

---

### TTS Benchmark (TTFB — lower is better)

| Provider | TTFB | Quality | Notes |
|----------|------|---------|-------|
| ElevenLabs Flash v2.5 | **~75ms** | Excellent | 30+ languages, voice cloning |
| Cartesia Sonic-3 | **<100ms** | 4.7 MOS | SSM, best latency overall |
| Deepgram Aura-2 | <200ms | Good | Lowest cost, EN+ES only |
| OpenAI TTS | ~250ms | Good | No voice cloning |
| Play.ai | ~300ms | Good | 50+ voices |
| Kokoro 82M | Fast (self-hosted) | Near-commercial | 82M params, CPU-runnable |

---

### VAD Comparison

| Method | Latency | Accuracy | Complexity |
|--------|---------|----------|------------|
| WebRTC VAD (Google GMM) | **<5ms/frame** | Low (energy-based) | Native, zero-config |
| Silero VAD (ONNX) | ~15-30ms | **High** | `ricky0123/vad-web` npm |
| LiveKit turn-detector (135M transformer) | ~10ms overhead | **Best** | Requires LiveKit stack |
| Cobra VAD (Picovoice) | ~5ms | High | Commercial |

**Recommendation:** Silero VAD (`@ricky0123/vad-web`) for browser. LiveKit transformer for production semantic turn detection.

---

### Barge-In Implementation Pattern

```
User speaks during agent speech
  Step 1: AEC removes agent audio from mic stream     (WebRTC native)
  Step 2: VAD detects human speech                    (85-100ms Silero)
  Step 3: Cancel TTS stream                           (<200ms)
  Step 4: Cancel LLM stream if still generating       (immediate)
  Step 5: STT begins on user utterance                (new pipeline cycle)

Grace period: 500-1000ms lockout after agent begins speaking
              (prevents false trigger from audio artifacts)
```

---

## PART 5: VIRAL MOMENTS & DEMOS

---

### Most Viral Demos (2024-2026)

**1. GibberLink — 15M views in one week (Feb 2025)**
Two ElevenLabs AI agents on a phone call realize they are both AI and switch to GGWave — an audio data protocol inaudible to humans, 80% faster and 90% cheaper. Won Global Top Prize at ElevenLabs Worldwide Hackathon.
- **Stack:** ElevenLabs voice agents + GGWave
- **Links:** https://techcrunch.com/2025/03/05/gibberlink-lets-ai-agents-call-each-other-in-robo-language | https://showcase.elevenlabs.io/projects/p/gibberlink

**2. Sesame AI — 1M users in first weeks (Feb 2025)**
Maya and Miles — voice companions that breathe, hesitate, can be interrupted. Closed $47.5M Series A immediately after going viral.
- **Links:** https://www.sesame.com/research/crossing_the_uncanny_valley_of_voice | https://app.sesame.com

**3. OpenAI Solar System Demo — Voice-driven 3D navigation**
Say a planet name → camera flies there. Ask about moons → they appear. Ask for ISS position → fetches live data and plots it.
- **Stack:** OpenAI Realtime API + Spline 3D + function calling
- **Links:** https://github.com/openai/openai-realtime-solar-system | https://developers.openai.com/showcase/solar-system-demo

**4. Sutando — Mac controlled by phone call**
Say "summon" → opens Zoom with screen share. Voice-edit documents. Join calendar meetings hands-free. Built autonomous self-improvement loop using Claude Code.
- **Links:** https://github.com/sonichi/sutando | https://sutando.ai

---

### Hackathon Winners (2025)

**SF Voice Agent Hackathon (Sept 2025) — Organized by AssemblyAI, LiveKit, Rime, Accel:**
- **Grand Prize: Voxy** — enter company name, get fully configured voice agent with 9 employee personas + persistent memory
- **Most Technical: Podweaver** — replaces podcast ads with voice-cloned natural-sounding sponsorships at 250ms precision

**ElevenLabs Worldwide Hackathon (Feb 2025) — 300+ projects:**
- **Global Prize: GibberLink** (see above)

**AssemblyAI Voice Challenge (July 2025):**
- Performance winner: Hogwarts Spell Caster — voice-controlled spells mapped to keyboard actions
- Domain winner: AI Debate Room — real-time AI debate opponent
- Business winner: Wynnie — autonomous voice shopping agent (PWA, multi-agent) — https://wynnie-v1.vercel.app

---

## PART 6: VISUALIZATION TECHNIQUES RANKED

---

| Rank | Technique | Example | Tech | Complexity |
|------|-----------|---------|------|------------|
| 1 | **WebGL GLSL shader orb** | ElevenLabs UI, BoltAI Orb | R3F + GLSL + Perlin noise | High |
| 2 | **Particle sphere + audio amplitude** | ethanplusai/jarvis, harsh-raj00/my-jarvis | Three.js + Web Audio FFT | Medium-High |
| 3 | **Live2D avatar + emotion mapping** | Open-LLM-VTuber | Live2D SDK + ARKit morph targets | Very High |
| 4 | **3D avatar lip sync** | RPM-agora-agent, Tavus | R3F + Agora RTC + phoneme mapping | Very High |
| 5 | **Node-graph pipeline** | TEN Framework TMAN | Custom graph editor | High |
| 6 | **AI-generated HTML canvas** | OpenVoiceUI | LLM function calling + iframe | High |
| 7 | **Aura shader (LiveKit)** | agent-starter-react | Unicorn Studio WebGL | Medium |
| 8 | **Siriwave CSS** | vapiblocks.com/components/siri | CSS + kopiro/siriwave | Low |
| 9 | **Glassmorphism pill** | Wisper | CSS backdrop-filter | Low |
| 10 | **Audio bars** | Standard | Web Audio + SVG | Low |

---

### GLSL Orb Pattern (The Standard)

```glsl
// Vertex shader — audio-reactive displacement
uniform float uAmplitude;   // mic RMS volume 0-1
uniform float uTime;
uniform sampler2D uPerlinTexture;

void main() {
  // Sample Perlin noise for organic motion
  float noise = texture(uPerlinTexture, uv + uTime * 0.05).r;
  
  // Displace vertex along normal by audio amplitude + noise
  vec3 newPos = position + normal * noise * uAmplitude * 0.5;
  
  gl_Position = projectionMatrix * modelViewMatrix * vec4(newPos, 1.0);
}

// Fragment shader — Fresnel edge glow
uniform vec3 uColor1;
uniform vec3 uColor2;
uniform float uInputVolume;

void main() {
  // Fresnel: edges glow brighter during speech
  float fresnel = pow(1.0 - dot(vNormal, vViewDir), 2.0);
  float glowStrength = mix(0.25, 1.0, uInputVolume);
  
  vec3 color = mix(uColor1, uColor2, fresnel * glowStrength);
  gl_FragColor = vec4(color, 1.0);
}
```

---

## PART 7: GAP ANALYSIS — WHAT NOBODY HAS BUILT

These patterns appear **nowhere** across all 30+ projects researched:

| Gap | Description | Opportunity for Nora/Topsi |
|-----|-------------|---------------------------|
| **Multi-agent presence** | Show Nora + Scout + Topsi simultaneously with per-agent state orbs | Unique — we have the agents |
| **Tool-use HUD** | Floating overlay shows live tool call name + status while AI talks | High impact, medium effort |
| **Ambient → voice mode** | Idle shows live CRM/task data, voice press activates conversation | Natural fit for our data |
| **AI-generated UI** | Voice command → Nora renders live interactive dashboard (OpenVoiceUI pattern) | Most powerful, hardest |
| **Pipeline observability** | Whisker-style frame debugger as a beautiful end-user feature | Differentiator |
| **Proactive AI initiation** | Nora surfaces relevant intel during idle periods without being asked | Vocalis pattern |
| **Semantic VAD** | Smart turn detection using transformer (not just silence) | LiveKit pattern, adoptable |
| **Cost ticker** | Live token cost display as first-class UI element | BoltAI pattern |
| **Conversation memory map** | Visual graph of what Nora remembers, updating live | Novel |
| **Dual-layer transcription** | User speech one color, AI thought another, AI speech a third | Novel |

---

## PART 8: TECH STACK RECOMMENDATION

**For PCG / Nora Orchestrator:**

```
Voice Pipeline:
├── STT:          Deepgram Nova-3 (~118ms, production) or Whisper (fallback)
├── Turn:         ricky0123/vad-web (Silero ONNX) in browser + semantic timeout
├── LLM:          Claude Sonnet 4.6 (our Nora) — already integrated
├── TTS:          ElevenLabs Flash v2.5 (75ms TTFB, voice cloning)
│                 OR Cartesia Sonic-3 (<100ms, lower quality)
├── Barge-in:     AEC (WebRTC native) + Silero VAD + immediate stream cancel
└── Transport:    WebRTC (browser) or WebSocket (existing Nora route)

Frontend:
├── Orb:          ElevenLabs GLSL shader (already implemented ✅)
├── Real audio:   Web Audio AnalyserNode → RMS → inputVolumeRef (done ✅)
├── Orch overlay: Actions + planId + responseType display (done ✅)
├── Suggestions:  followUpSuggestions chips (done ✅)
├── Next:         Tool-use HUD, ambient mode, semantic VAD
└── Stretch:      Multi-agent presence, AI-generated canvas

NOT Needed:
├── LiveKit SFU (overkill for single-user dashboard)
├── Pipecat (we have direct Nora integration)
└── Vapi/Retell (we're building the orchestrator, not buying one)
```

---

## PART 9: OPENCLAW ECOSYSTEM

---

### OpenClaw — `github.com/OpenClaw/OpenClaw` ⭐ 247k
**Originally:** "Clawdbot" by Peter Steinberger (PSPDFKit founder) | **Renamed:** January 2026  
**Sponsors:** OpenAI (lead), Anthropic, Google DeepMind | **Language:** TypeScript + Python

The fastest-growing agentic platform in GitHub history. Hit 100K stars in under 3 weeks after the rename. OpenClaw is not a voice framework — it's a **universal agent operating system** that voice UIs plug into. It is the closest thing in the open-source world to what Nora is supposed to be: a hub that orchestrates other AI agents.

---

### Architecture: Hub-and-Spoke Gateway

OpenClaw runs a local HTTP gateway on port **18789**. Every AI tool, agent, or UI connects to the hub. The gateway routes requests, manages permissions, and provides a unified tool namespace.

```
                        ┌──────────────────────────────────┐
                        │         OPENCLAW HUB :18789       │
                        │   ┌──────────┐  ┌──────────────┐  │
                        │   │  Skills  │  │  Sub-agents  │  │
                        │   │  ~5400+  │  │  (spawned)   │  │
                        │   └──────────┘  └──────────────┘  │
                        │   ┌──────────┐  ┌──────────────┐  │
                        │   │ Lobster  │  │  Canvas/A2UI │  │
                        │   │ Workflows│  │  :18793      │  │
                        │   └──────────┘  └──────────────┘  │
                        └──────────┬───────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                    │
         Desktop IDE          Voice UI            Browser
         (OpenClaw UI)     (OpenVoiceUI)      (Canvas WebView)
```

**7-stage agentic loop:**
1. `intake` — receive user message
2. `plan` — select skills + sub-agents to invoke
3. `retrieve` — pull relevant memory/context
4. `execute` — run tool calls in parallel
5. `observe` — collect tool results
6. `reflect` — evaluate if goal achieved
7. `respond` — stream reply to UI

---

### Skills System

Skills live in `~/.openclaw/workspace/skills/` as `SKILL.md` YAML files. Each skill declares:
- **`tool_name`** — how the LLM calls it
- **`tool_description`** — what the LLM reads (critical for selection accuracy)
- **`input_schema`** — Zod-compatible JSON schema
- **`executor`** — shell command, HTTP endpoint, or sub-agent invocation

**53 bundled skills** include: web search, code execution, file operations, git, screenshot, browser control, email send, calendar query, Slack message.

**5400+ community skills** on [ClawHub](https://clawhub.io) — a package registry like npm for AI capabilities. Install via:
```bash
openclaw skill install @community/notion-reader
openclaw skill install @community/github-pr-reviewer
```

**Our equivalent:** Nora's tool system in `routes/nora.rs` serves the same purpose — but registered in Rust code, not YAML files. The ClawHub model suggests we could expose a `/api/nora/skills` registry endpoint for dynamically registered tools.

---

### Sub-agents: Parallel Orchestration

Sub-agents are spawned via the `sessions_spawn` tool call. They run as isolated OpenClaw instances sharing the same hub.

```
Nora (orchestrator)
├── spawns Sub-agent A → research task
├── spawns Sub-agent B → code generation
└── spawns Sub-agent C → draft email
     ↓ (all run in parallel)
Results push back to Nora session via shared session bus
Max 8 concurrent sub-agents
```

**Nora parallel equivalent:** Nora's `actions[]` array in the orchestration response maps to this — multiple agents assigned simultaneously. The gap is that our sub-agents (Scout, Astra, Bodhi) don't push results back to Nora's live session in real time; they run to completion and store in DB.

---

### Canvas / A2UI: AI-Generated Interfaces

OpenClaw's Canvas runs a WebView server on port **18793**. When a Skill generates UI (HTML/CSS/JS), Canvas serves it directly into a desktop WebView panel or browser tab.

```
User: "show me my git repo activity as a chart"
  ↓
OpenClaw: invokes git-stats skill
  ↓
Skill: generates <html> with Chart.js bar chart
  ↓
Canvas: renders in WebView panel (live, interactive)
```

This is the **open-ended generative UI pattern** — LLM generates arbitrary HTML, Canvas sandboxes it. Google has separately published an **A2UI spec** (Agent-to-User Interface) that standardizes this pattern as a JSON protocol rather than raw HTML, making it safer and portable.

---

### Lobster: YAML Workflow Engine

Lobster is OpenClaw's workflow definition language — think GitHub Actions syntax but for AI pipelines:

```yaml
# ~/.openclaw/workspace/workflows/weekly-report.yaml
name: weekly-team-report
trigger:
  cron: "0 9 * * MON"
steps:
  - name: gather-data
    skill: github-activity
    input:
      repo: "org/repo"
      days: 7
  - name: gather-tasks
    skill: linear-tasks
    depends_on: []
  - name: write-report
    model: claude-sonnet-4-6
    prompt: |
      Summarize this week:
      Activity: {{gather-data.output}}
      Tasks: {{gather-tasks.output}}
  - name: send-report
    skill: slack-send
    input:
      channel: "#team"
      message: "{{write-report.output}}"
```

**PCG equivalent:** Our `automations.rs` with hourly cron scheduling serves this purpose. The Lobster pattern suggests a user-facing workflow builder UI would be a high-value addition.

---

### OpenVoiceUI — Voice Front-end for OpenClaw

**Repo:** `github.com/openclaw-community/openvoiceui` ⭐ 36  
**Website:** https://openvoiceui.com  
**Stack:** Python + Docker (3 containers: STT, hub-connector, TTS)

OpenVoiceUI is a community-built voice layer that sits on top of OpenClaw hub. It demonstrates the exact integration pattern we need: voice input → agent orchestrator → voice output, with the heavy AI logic delegated to the hub.

```
Microphone → Whisper STT → OpenClaw Hub → Skills/Sub-agents → Coqui TTS → Speaker
                                ↕
                          Canvas WebView
                      (AI-generated UI panels)
```

**Key design choices:**
- Runs as Docker Compose — STT, connector, and TTS as separate services
- Push-to-talk by default (no VAD complexity in v0.1)
- Canvas port passed as env var — WebView auto-opens on voice commands that return HTML
- All orchestration state visible via hub's `/sessions/active` endpoint

**Gap vs. our setup:** OpenVoiceUI has no visual orb, no real-time volume visualization, no suggestion chips. It's purely functional. Our `/interface` page is already more polished.

---

### Competitive Summary: OpenClaw vs. Nora

| Feature | OpenClaw | Nora (PCG) |
|---------|----------|------------|
| Hub architecture | Port 18789 gateway | Axum routes, `/api/nora/chat` |
| Skill registry | YAML files + ClawHub (5400+) | Rust tools in code |
| Sub-agent spawning | `sessions_spawn`, max 8 concurrent | Scout/Astra/Bodhi, stored in DB |
| Parallel execution | Native, results push to session | actions[] array, async |
| Workflow engine | Lobster YAML | automations.rs cron |
| Voice layer | OpenVoiceUI (community, Docker) | /interface (React, WebGL orb) |
| Canvas/UI generation | A2UI, sandboxed HTML WebView | Planned (OrchOverlay) |
| Context memory | Local JSONL files | SQLite AgentConversation |
| Multi-agent presence | None (single session view) | **Unique gap — opportunity** |

**Strategic implication:** OpenClaw is winning on breadth (5400 skills, massive community). We win on depth — Nora has actual business context (CRM, projects, clients, proposals), a polished voice UI, and multi-agent orchestration that OpenClaw's community is still building toward.

---

## PART 10: GENERATIVE UI PATTERNS

---

### The Three Patterns

As AI orchestrators gain the ability to generate interfaces on the fly, three distinct rendering architectures have emerged — each with a different safety/power tradeoff:

```
                    SAFETY ◄────────────────────────► POWER
                    
  Pattern A              Pattern B              Pattern C
  Static                 Declarative            Open-ended
  Tool-call → Component  JSON spec → Renderer   LLM HTML → Sandbox
  
  "Show task list"       "Render chart config"  "Build me a dashboard"
  Pre-built React        Schema + renderer       Arbitrary HTML/JS
  Zero XSS risk          Schema validates        Iframe isolation needed
```

---

### Pattern A: Static Tool Calls → Pre-built Components

The safest and most common pattern. The LLM selects a tool, returns structured data, and the frontend renders it with a pre-built React component.

```tsx
// Nora returns:
{
  "tool_call": "show_project_tasks",
  "args": { "project_id": "abc123" }
}

// Frontend maps tool → component:
const TOOL_COMPONENTS = {
  show_project_tasks: ProjectTaskPanel,
  show_proposal_summary: ProposalCard,
  show_crm_contact: ContactCard,
};

// No LLM-generated markup touches the DOM
```

**Used by:** Vercel AI SDK `useChat` + `toolInvocations`, OpenAI function calling, Anthropic tool_use  
**Risk:** None. Pre-built components only.  
**Our status:** ✅ OrchOverlay already does this for `actions[]`

---

### Pattern B: Declarative Schema → Component Renderer

The LLM generates a **JSON configuration spec** that describes a UI layout. A deterministic renderer builds the React tree from the spec. The LLM never writes HTML or JS directly.

```json
{
  "type": "dashboard",
  "title": "Weekly Revenue",
  "panels": [
    { "type": "metric", "label": "MRR", "value": "$42,000", "trend": "+12%" },
    { "type": "chart", "chart_type": "bar", "data": [...], "x_key": "week" },
    { "type": "table", "columns": ["Client", "Status", "Amount"], "rows": [...] }
  ]
}
```

**Google A2UI Spec** (2025) standardizes this as an agent-to-UI protocol — agents output A2UI JSON, any A2UI-compatible renderer can display it. Designed for cross-agent portability.

**Vercel AI SDK implementation:**
```ts
// Server: streamText with schema-constrained tool
const result = streamText({
  model: anthropic('claude-sonnet-4-6'),
  tools: {
    render_dashboard: {
      description: 'Render an interactive dashboard panel',
      parameters: z.object({
        panels: z.array(panelSchema)
      }),
    }
  }
});

// Client: useChat reads toolInvocations
const { messages } = useChat({ api: '/api/nora/chat' });
// Render: messages[n].toolInvocations → <DashboardRenderer spec={inv.result} />
```

**Our opportunity:** Implement a `render_panel` tool in Nora's orchestration response. When Nora wants to show data visually, it returns a JSON panel spec rather than text. The `/interface` page renders it using a DashboardRenderer alongside the orb.

---

### Pattern C: Open-ended HTML → Sandboxed Iframe

The most powerful — and most dangerous — pattern. The LLM generates complete HTML/CSS/JS that runs in a sandboxed `<iframe>`.

```tsx
// ONLY safe implementation:
<iframe
  srcDoc={llmGeneratedHtml}
  sandbox="allow-scripts"          // NO allow-same-origin
  csp="default-src 'none'; script-src 'unsafe-inline'"
  style={{ border: 'none', width: '100%', height: 400 }}
/>

// NEVER do this:
<div dangerouslySetInnerHTML={{ __html: llmGeneratedHtml }} />
```

**Used by:** OpenClaw Canvas, BoltAI, v0.dev (for previews), Artifacts (Claude.ai)

**Known CVEs (2025):**
- **CVE-2025-67744** — XSS-to-RCE via Mermaid diagram renderer in multiple chatbot frameworks. Mermaid executed `<script>` tags embedded in LLM-generated diagram syntax.
- **CVE-2025-59528** — Flowise RCE via unsanitized LLM output passed to `eval()`. Affected 2,400+ production Flowise deployments.

**Our recommendation:** Do not implement Pattern C until we have a sandboxing story. The orb interface doesn't need it yet — Pattern A and B cover 95% of use cases.

---

### AG-UI Protocol

**AG-UI** (Agent-to-UI) is a streaming event protocol proposed in early 2026 by the Anthropic DevRel team as a complement to MCP. Where MCP defines how agents call tools, AG-UI defines how agents stream **UI state updates** to frontends.

```
MCP:    Tool definition + invocation (what agents can DO)
AG-UI:  UI event stream (what agents SHOW)
```

AG-UI events:
```json
{ "type": "ui_insert", "component": "metric_card", "props": {...}, "position": "after_message" }
{ "type": "ui_update", "component_id": "task-list", "patch": [{"op": "add", "path": "/items/-", "value": {...}}] }
{ "type": "ui_remove", "component_id": "loading-skeleton" }
```

**Adoption status:** Still draft spec. Only 3 frameworks have adopted it (CopilotKit, Haystack, one community port). Watching.

---

### Vercel AI SDK: The Reference Implementation

For React/Vite apps, the Vercel AI SDK provides the cleanest Pattern A + B implementation:

```ts
// 1. Server: define tool catalog
const noraTools = {
  show_tasks: tool({
    description: 'Show task list for a project',
    parameters: z.object({ project_id: z.string() }),
    execute: async ({ project_id }) => fetchTasks(project_id),
  }),
  render_chart: tool({
    description: 'Render a data visualization',
    parameters: chartSpecSchema,
    execute: async (spec) => spec, // pass-through, frontend renders
  }),
};

// 2. Client: map tool invocations to components
const RENDERERS: Record<string, React.ComponentType<any>> = {
  show_tasks:   TaskListPanel,
  render_chart: ChartPanel,
};

function Message({ message }: { message: Message }) {
  return (
    <>
      <p>{message.content}</p>
      {message.toolInvocations?.map(inv =>
        RENDERERS[inv.toolName]
          ? React.createElement(RENDERERS[inv.toolName], inv.result)
          : null
      )}
    </>
  );
}
```

**This is exactly what our OrchOverlay already does** — just with a more limited set of components. The next step is expanding the renderer registry.

---

### Implementation Roadmap for PCG Interface

| Pattern | Feature | Effort | Impact |
|---------|---------|--------|--------|
| A (Static) | Expand OrchOverlay component registry | Low | High |
| A (Static) | Tool-use HUD: floating pill shows live tool name during execution | Low | High |
| B (Declarative) | `render_panel` Nora tool + DashboardRenderer | Medium | Very High |
| B (Declarative) | Ambient mode: idle /interface shows live CRM/tasks panels | Medium | High |
| C (Sandboxed) | iframe canvas for complex visualizations | High | Medium |

**Security rule for all patterns:** Never pass LLM output to `dangerouslySetInnerHTML`, `eval()`, or `Function()`. Always use pre-built component registries or validated JSON schemas.

---

## SOURCES

**GitHub:**
- https://github.com/OpenClaw/OpenClaw (247k ⭐)
- https://github.com/openclaw-community/openvoiceui
- https://clawhub.io (OpenClaw skill registry)
- https://github.com/pipecat-ai/pipecat
- https://github.com/livekit/agents
- https://github.com/livekit/livekit (18.1k ⭐)
- https://github.com/TEN-framework/ten-framework
- https://github.com/openai/openai-realtime-agents
- https://github.com/fixie-ai/ultravox
- https://github.com/microsoft/VibeVoice
- https://github.com/isair/jarvis
- https://github.com/ethanplusai/jarvis
- https://github.com/harsh-raj00/my-jarvis
- https://github.com/cam-hm/jarvis
- https://github.com/aguscruiz/voiceorb
- https://github.com/Open-LLM-VTuber/Open-LLM-VTuber
- https://github.com/AgoraIO-Community/RPM-agora-agent
- https://github.com/ricky0123/vad
- https://github.com/sonichi/sutando
- https://github.com/livekit-examples/agent-starter-react
- https://github.com/openai/openai-realtime-solar-system
- https://github.com/rapidaai/voice-ai

**Commercial Platforms:**
- https://vapi.ai | https://www.retellai.com | https://www.bland.ai
- https://www.hume.ai | https://elevenlabs.io | https://deepgram.com
- https://livekit.com | https://www.pipecat.ai | https://synthflow.ai
- https://cartesia.ai | https://www.speechmatics.com | https://poly.ai
- https://www.tavus.io | https://smallest.ai | https://play.ai

**Benchmarks & Research:**
- https://telnyx.com/resources/voice-ai-agents-compared-latency
- https://webrtchacks.com/measuring-the-response-latency-of-openais-webrtc-based-real-time-api
- https://livekit.com/blog/using-a-transformer-to-improve-end-of-turn-detection
- https://www.assemblyai.com/benchmarks
- https://inworld.ai/resources/best-voice-ai-tts-apis-for-real-time-voice-agents-2026-benchmarks
- https://cartesia.ai/vs/cartesia-vs-openai-tts
- https://sierra.ai/blog/voice-latency
- https://picovoice.ai/blog/best-voice-activity-detection-vad-2025

**Demos & Viral Moments:**
- https://www.sesame.com/research/crossing_the_uncanny_valley_of_voice
- https://techcrunch.com/2025/03/05/gibberlink-lets-ai-agents-call-each-other-in-robo-language
- https://developers.openai.com/showcase/solar-system-demo
- https://showcase.elevenlabs.io/projects/p/gibberlink
- https://agent.theten.ai
- https://theorb.boltai.com
- https://openvoiceui.com

**Generative UI & Protocols:**
- https://vercel.com/blog/ai-sdk-4-generative-ui
- https://sdk.vercel.ai/docs/ai-sdk-ui/generative-ui
- https://developers.googleblog.com/en/a2ui-agent-to-user-interface-protocol
- https://ag-ui.dev (draft spec)
- https://github.com/copilotkit/copilotkit (AG-UI early adopter)
- CVE-2025-67744: https://nvd.nist.gov/vuln/detail/CVE-2025-67744
- CVE-2025-59528: https://nvd.nist.gov/vuln/detail/CVE-2025-59528

**UI/UX:**
- https://ui.elevenlabs.io/docs/components/orb
- https://www.vapiblocks.com
- https://uiverse.io/challenges/voice-assistant-orb
- https://tympanus.net/codrops/2025/06/18/coding-a-3d-audio-visualizer-with-three-js-gsap-web-audio-api
- https://www.smashingmagazine.com/2025/07/design-patterns-ai-interfaces

---

*Document compiled from 6 parallel research agents — GitHub, commercial platforms, social/viral, technical benchmarks, OpenClaw ecosystem, and generative UI patterns. April 2026.*
