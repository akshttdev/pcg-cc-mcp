# Cipher — Sovereign AI Companion App
## Detailed Build Plan v1.0

**Date**: 2026-05-07  
**Status**: Planning — Ready for Execution  
**Author**: PCG Founding Team + Claude  
**Target Reader**: Any execution terminal picking this up fresh

---

## 0. Brief for the Execution Terminal

This document is a self-contained briefing. Before reading the plan, read this section so you understand the world you're building inside.

**What is PCG?**  
Powerclub Global is a creative agency turned AI orchestration platform company. The platform is called **ORCHA** (Orchestration + CHA). It is a full-stack Rust/React dashboard that runs the entire business: CRM, task management, AI agent workflows, social media publishing, client delivery, and a VIBE token economy. It lives at `dashboard.powerclubglobal.com`.

**Who are the agents?**  
Two agents are relevant to Cipher:
- **Nora** — executive AI assistant. Bodhi's (CEO/founder) personal orchestrator. Handles strategic queries, CRM context, and executive decisions. British personality. Voice-enabled.  
- **Topsi** — platform orchestrator. Team-facing. Handles project work, workflow triggers, task coordination. The agent most users will talk to.

**What is the Sovereign Stack?**  
The Sovereign Stack is the layered architecture principle that unifies all PCG's infrastructure decisions. Every layer answers the question: *"who controls this?"* — and the answer is always PCG or the individual user, never a third-party cloud.

```
┌─────────────────────────────────────────────────────────────────────┐
│  LAYER 5: Identity & Access                                         │
│  PCG JWT credentials + Seeker Genesis Token (SGT) hardware proof   │
│  "You are who you say you are, proven by something you ARE"         │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 4: Intelligence (ORCHA)                                      │
│  Agents: Nora, Topsi, Scout, Cash, Lux, Maci, Editron              │
│  MCP Servers: TaskServer, TopsiServer, NoraServer, PulseServer      │
│  9 executor backends: Claude, Gemini, Qwen, OpenCode, ...          │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 3: Economy (VIBE)                                            │
│  Aptos-based token. 1 VIBE = $0.01 USD. 2x markup on AI costs.     │
│  Internal ledger (VibeTransaction) → future on-chain settlement.    │
│  Marketplace: 85% to providers, 15% platform.                      │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 2: Messaging (Pulse Engine / NATS)                           │
│  NATS relay at nats://nonlocal.info:4222                            │
│  Subjects: apn.service.*, apn.storage.*, apn.agent.*               │
│  PCG→Pulse publisher (global singleton in server binary)            │
│  Pulse→PCG consumer (background worker)                             │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 1: Mesh (Alpha Protocol Network / APN)                       │
│  libp2p + NATS relay. X25519 encryption. Ed25519 keys.              │
│  BIP39 mnemonics for node identity. Kademlia DHT peer discovery.    │
│  Each PCG server = master node. Cipher app = mobile edge node.      │
├─────────────────────────────────────────────────────────────────────┤
│  LAYER 0: Compute                                                   │
│  Self-hosted Docker (pcg-cc-mcp container, port 3001 prod)          │
│  Nginx reverse proxy on port 8080.                                  │
│  Cloudflare tunnel → dashboard.powerclubglobal.com                 │
│  SQLite (dev) → PostgreSQL (Stage 2+)                               │
└─────────────────────────────────────────────────────────────────────┘
```

**What is Cipher?**  
Cipher is the mobile voice terminal for ORCHA. It is NOT a mobile version of the dashboard. The dashboard handles workflows, CRM, social calendars, deliverables — all visual/complex work. Cipher handles one thing: **conversation with your orchestrator, from anywhere, by voice or text**.

The name is intentional: a cipher is both an encryption key (your sovereign identity token) and a personal monogram (this is your window into your orchestration layer, not someone else's).

---

## 1. Where Cipher Lives in the Stack

```
                        ╔══════════════════════════════════╗
                        ║    dashboard.powerclubglobal.com ║
                        ║         ORCHA Dashboard           ║
                        ║                                   ║
                        ║  CRM · Tasks · Social · Delivery  ║
                        ║  Workflows · Analytics · Admin    ║
                        ╚══════════════════════════════════╝
                                        │
                              REST + SSE + WebSocket
                                        │
                        ╔══════════════════════════════════╗
                        ║        ORCHA API Server           ║
                        ║   (Rust/Axum, port 3001 prod)    ║
                        ║                                   ║
                        ║  /api/auth/*                      ║
                        ║  /api/topsi/*    ← Cipher uses   ║
                        ║  /api/nora/*     ← Cipher uses   ║
                        ║  /api/agents/*   ← Cipher uses   ║
                        ║  /api/vibe/*     ← Cipher reads  ║
                        ║  /api/social/*                    ║
                        ║  /api/crm/*                       ║
                        ║  ... 120 route modules total      ║
                        ╚══════════════════════════════════╝
                          │                         │
                    NATS relay               direct HTTPS
                (apn.agent.*)             (dashboard URL)
                          │                         │
              ┌───────────┴───────┐         ┌───────┴──────┐
              │                   │         │              │
      ╔═══════╧═══════╗   ╔══════╧══════╗  │   Browser    │
      ║    CIPHER      ║   ║   CIPHER   ║  │  (desktop)   │
      ║   (Android)    ║   ║   (iOS)    ║  └──────────────┘
      ║                ║   ║            ║
      ║ ┌────────────┐ ║   ║            ║
      ║ │  + Seeker  │ ║   ║            ║
      ║ │  MWA + SGT │ ║   ║            ║
      ║ └────────────┘ ║   ║            ║
      ╚════════════════╝   ╚════════════╝
         Hardware-bound       JWT only
         identity proof       (still secure)
```

**Key architectural decision**: Cipher talks to the same API server as the dashboard. No separate mobile backend. No BFF layer. The server already speaks SSE for streaming chat. The app connects to `dashboard.powerclubglobal.com/api/` with a Bearer JWT token.

---

## 2. Full Sovereign Stack + Cipher Architecture Diagram

```
 ╔══════════════════════════════════════════════════════════════════════╗
 ║                     SOVEREIGN STACK — FULL VIEW                     ║
 ╠══════════════════════════════════════════════════════════════════════╣
 ║                                                                      ║
 ║  IDENTITY LAYER                                                      ║
 ║  ─────────────                                                       ║
 ║                                                                      ║
 ║    Standard Device           Seeker Device (Solana Mobile)          ║
 ║    ┌─────────────┐           ┌──────────────────────────┐           ║
 ║    │ email+pass  │           │ email+pass               │           ║
 ║    │     ↓       │           │    + SIWS signature       │           ║
 ║    │ POST /login │           │    + SGT ownership proof  │           ║
 ║    │     ↓       │           │         ↓                 │           ║
 ║    │  JWT token  │           │    JWT token              │           ║
 ║    │ (1 factor)  │           │  (2 factor: know + ARE)  │           ║
 ║    └─────────────┘           └──────────────────────────┘           ║
 ║                                                                      ║
 ║    SGT = Seeker Genesis Token. Non-transferable. Bound to device.   ║
 ║    Steal password → still need the physical Seeker.                  ║
 ║    Steal Seeker → still need PCG credentials.                        ║
 ║                                                                      ║
 ╠══════════════════════════════════════════════════════════════════════╣
 ║                                                                      ║
 ║  CIPHER APP (mobile)                                                 ║
 ║  ──────────────────                                                  ║
 ║                                                                      ║
 ║  ┌─────────────────────────────────────────────────────────────┐    ║
 ║  │                                                             │    ║
 ║  │  LoginScreen        AgentSelectScreen      ChatScreen       │    ║
 ║  │  ───────────        ────────────────       ──────────       │    ║
 ║  │  • email/password   • Topsi card           • message thread │    ║
 ║  │  • JWT → storage    • Nora card            • streaming SSE  │    ║
 ║  │  • Seeker: SIWS     • (future: more)       • voice input    │    ║
 ║  │                                            • voice output   │    ║
 ║  │                                            • VIBE balance   │    ║
 ║  │                                                             │    ║
 ║  │  State: Zustand slice  |  HTTP: axios + interceptor         │    ║
 ║  │  SSE: react-native-sse |  Voice: expo-av + ElevenLabs TTS  │    ║
 ║  │  Auth: AsyncStorage    |  Nav: React Navigation v6          │    ║
 ║  └─────────────────────────────────────────────────────────────┘    ║
 ║                │                                                     ║
 ║          HTTPS + SSE                                                 ║
 ║                │                                                     ║
 ╠══════════════════════════════════════════════════════════════════════╣
 ║                                                                      ║
 ║  ORCHA API (crates/server)                                           ║
 ║  ─────────────────────────                                           ║
 ║                                                                      ║
 ║  Auth routes (/api/auth/*)                                           ║
 ║  ┌─────────────────────────────────────────────────┐                ║
 ║  │ POST /api/auth/login  → { token, user }         │                ║
 ║  │ POST /api/auth/logout                           │                ║
 ║  │ GET  /api/auth/me     → { user, org, roles }    │                ║
 ║  └─────────────────────────────────────────────────┘                ║
 ║                                                                      ║
 ║  Agent routes (/api/topsi/*, /api/nora/*)                           ║
 ║  ┌─────────────────────────────────────────────────┐                ║
 ║  │ POST /api/topsi/chat         → SSE stream       │                ║
 ║  │ POST /api/topsi/voice        → audio response   │                ║
 ║  │ POST /api/nora/chat          → SSE stream       │                ║
 ║  │ POST /api/nora/voice         → audio response   │                ║
 ║  │ GET  /api/agents             → agent list       │                ║
 ║  └─────────────────────────────────────────────────┘                ║
 ║                                                                      ║
 ║  Economy (/api/vibe/*)                                               ║
 ║  ┌─────────────────────────────────────────────────┐                ║
 ║  │ GET /api/vibe/balance  → { balance_vibe }       │                ║
 ║  └─────────────────────────────────────────────────┘                ║
 ║                                                                      ║
 ╠══════════════════════════════════════════════════════════════════════╣
 ║                                                                      ║
 ║  INTELLIGENCE LAYER (behind API)                                     ║
 ║  ──────────────────────────────                                      ║
 ║                                                                      ║
 ║  ┌──────────────┐   ┌──────────────┐   ┌──────────────────────────┐ ║
 ║  │  NoraAgent   │   │  TopsiAgent  │   │  9 LLM Executor backends │ ║
 ║  │  Arc<RwLock> │   │  Arc<RwLock> │   │  Claude, Gemini, Qwen,   │ ║
 ║  │  • Memory    │   │  • Memory    │   │  OpenCode, Cursor, Duck, │ ║
 ║  │  • Voice     │   │  • MCP tools │   │  ACP, AMP, Codex         │ ║
 ║  │  • Persona   │   │  • Workflows │   │                          │ ║
 ║  └──────────────┘   └──────────────┘   └──────────────────────────┘ ║
 ║          │                  │                                        ║
 ║          └──────────────────┘                                        ║
 ║                      │                                               ║
 ║             MCP Servers (4 running)                                  ║
 ║    TaskServer · TopsiServer · NoraServer · PulseServer               ║
 ║                                                                      ║
 ╠══════════════════════════════════════════════════════════════════════╣
 ║                                                                      ║
 ║  ECONOMY LAYER (VIBE)                                                ║
 ║  ─────────────────────                                               ║
 ║                                                                      ║
 ║  • Every chat message costs VIBE (debited from project balance)      ║
 ║  • 1 VIBE = $0.01 USD. AI costs marked up 2x.                       ║
 ║  • VibePricingService.record_llm_usage() called on each response.    ║
 ║  • Cipher shows balance — user sees their "fuel gauge."              ║
 ║  • HTTP 402 Payment Required returned if balance = 0.               ║
 ║                                                                      ║
 ╠══════════════════════════════════════════════════════════════════════╣
 ║                                                                      ║
 ║  MESH LAYER (APN)                                                    ║
 ║  ─────────────────                                                   ║
 ║                                                                      ║
 ║  NATS relay: nats://nonlocal.info:4222                               ║
 ║                                                                      ║
 ║  PCG Master Node ──── NATS ──── Remote APN Nodes                    ║
 ║  (server binary)      relay     (compute contributors)              ║
 ║        │                                                             ║
 ║        │  (future: Cipher 0.3)                                       ║
 ║        └──── Cipher as mobile edge node                              ║
 ║              announces presence on apn.presence.*                   ║
 ║              receives push payloads via apn.agent.notify.*          ║
 ║                                                                      ║
 ╠══════════════════════════════════════════════════════════════════════╣
 ║                                                                      ║
 ║  COMPUTE LAYER                                                       ║
 ║  ─────────────                                                       ║
 ║                                                                      ║
 ║  Docker containers (prod):                                           ║
 ║    pcg-cc-mcp     — Rust server binary, port 3001                   ║
 ║    pcg-nginx      — Nginx reverse proxy, port 8080                  ║
 ║    pcg-cloudflared — Cloudflare tunnel                               ║
 ║    + db-backup, apn-bridge services                                  ║
 ║                                                                      ║
 ║  Local dev: ./target/debug/server on port 3000                       ║
 ║  DB: SQLite at dev_assets/db.sqlite (dev)                            ║
 ║                                                                      ║
 ╚══════════════════════════════════════════════════════════════════════╝
```

---

## 3. Authentication Architecture

### 3A. Standard Auth (All Devices)

```
 Cipher App                     ORCHA API                    DB
 ──────────                     ─────────                    ──
                                                             
  [Login Form]                                              
  email + password                                          
       │                                                    
       │  POST /api/auth/login                              
       │  { email, password }                               
       │ ──────────────────────────────────────────────→   
       │                                                    
       │              validate credentials                  
       │              AuthService::verify_password()        
       │              generate JWT (7d expiry)              
       │                                          SELECT    
       │                                       ←────────── 
       │                                                    
       │  ← 200 { token: "eyJ...", user: { id, name, ... }}
       │                                                    
  AsyncStorage.setItem('cipher_token', token)               
  axios.defaults.headers.Authorization = `Bearer ${token}`  
       │                                                    
  [Navigate → AgentSelectScreen]                            
```

### 3B. Seeker Hardware Auth (Seeker Device Only — Progressive Enhancement)

```
 Cipher on Seeker          Seed Vault              ORCHA API
 ────────────────          ──────────              ─────────
                                                   
  [Login Form]                                    
  email + password                                
       │                                          
       │  1. Standard JWT login (same as above)   
       │ ────────────────────────────────────→    
       │  ← JWT token                             
       │                                          
       │  2. Request SIWS signature from Seed Vault
       │ ──────────────────────────────→          
       │       sign({ message, nonce })           
       │  ← { signature, publicKey, sgtProof }    
       │                                          
       │  3. Register SGT proof with backend       
       │  POST /api/auth/sovereign-proof           
       │  { token, signature, publicKey, sgtNFT } 
       │ ──────────────────────────────────────→  
       │       verify signature                   
       │       verify SGT ownership on-chain       
       │       attach sgt_verified: true to user  
       │  ← 200 { sgt_verified: true }            
       │                                          
  [Mark session as hardware-verified]             
  [Show "Sovereign" badge in header]              
```

**What SGT verification unlocks (future gates):**
- Access to higher-privilege agent commands
- Increased VIBE spending limits
- Sovereign identity badge visible to agents
- Future: cross-org collaboration without data exposure

---

## 4. Screen Architecture — Detailed

### Screen 1: LoginScreen

```
┌────────────────────────────────────────┐
│                                        │
│                                        │
│                                        │
│         ░░░░░░░░░░░░░░░░░              │
│         ░  [cipher logo]  ░            │
│         ░░░░░░░░░░░░░░░░░              │
│                                        │
│      C I P H E R                      │
│      by Powerclub Global               │
│                                        │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │  email@domain.com                │  │
│  └──────────────────────────────────┘  │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │  ••••••••••                      │  │
│  └──────────────────────────────────┘  │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │         Sign In                  │  │
│  └──────────────────────────────────┘  │
│                                        │
│                                        │
│  ──────── or ────────                  │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │  🔐 Sign In with Seeker          │  │  ← only shown if MWA available
│  └──────────────────────────────────┘  │
│                                        │
│                                        │
└────────────────────────────────────────┘

State:
  email: string
  password: string
  loading: boolean
  error: string | null
  seekerAvailable: boolean  ← detected via useMobileWallet()

On sign-in success:
  → store JWT in AsyncStorage
  → fetch /api/auth/me for user context
  → navigate to AgentSelectScreen

On HTTP 402 after login:
  → show "VIBE balance depleted" message
  → link to dashboard to top up
```

### Screen 2: AgentSelectScreen

```
┌────────────────────────────────────────┐
│  ←  Cipher           [VIBE: 2,450 ⚡]  │  ← header: back + balance
├────────────────────────────────────────┤
│                                        │
│  Good morning, Bodhi.                  │  ← user.name from /api/auth/me
│  Who would you like to speak with?     │
│                                        │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │  🤖                              │  │
│  │  Topsi                           │  │
│  │  Platform Orchestrator           │  │
│  │                                  │  │
│  │  Tasks · Workflows · Projects    │  │
│  │  Team coordination               │  │
│  │                                  │  │
│  │  ● Online                        │  │
│  └──────────────────────────────────┘  │
│                                        │
│  ┌──────────────────────────────────┐  │
│  │  🎙                              │  │
│  │  Nora                            │  │
│  │  Executive Assistant             │  │
│  │                                  │  │
│  │  CRM · Strategy · Intelligence  │  │
│  │  Voice-first conversations       │  │
│  │                                  │  │
│  │  ● Online                        │  │
│  └──────────────────────────────────┘  │
│                                        │
│  (agents sourced from GET /api/agents) │
│                                        │
└────────────────────────────────────────┘

Navigation: tap card → ChatScreen(agentId)
Agents fetched from: GET /api/agents
Filtered to: short_name IN ('Topsi', 'Nora')
```

### Screen 3: ChatScreen

```
┌────────────────────────────────────────┐
│  ← Topsi             [VIBE: 2,450 ⚡]  │
├────────────────────────────────────────┤
│                                        │
│  ┌──────────────────────────────────┐  │  ← today separator
│  │           Today                  │  │
│  └──────────────────────────────────┘  │
│                                        │
│  ┌──────────────────────────────────┐  │  ← user bubble (right)
│  │  What's the status on TIACA?    ░│  │
│  └──────────────────────────────────┘  │
│  10:23 AM                              │
│                                        │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │  ← agent bubble (left)
│  ░ [T]                               ░  │
│  ░ TIACA ES2026 is tracking well.   ░  │  ← streaming: renders
│  ░ 14 posts are scheduled from      ░  │     token by token
│  ░ May 26 through July 14. The      ░  │     as SSE arrives
│  ░ social publish loop is active    ░  │
│  ░ and 3 posts are in pending      ░  │
│  ░ review. Want me to send a        ░  │
│  ░ review link for any of them?     ░  │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│  10:23 AM  ✓✓                         │
│                                        │
│                                        │
│                                        │
│                                        │
├────────────────────────────────────────┤
│                                        │
│  ┌────────────────────────┐  ┌──────┐  │
│  │  Message Topsi...      │  │  🎤  │  │
│  └────────────────────────┘  └──────┘  │
│                              hold to   │
│                               speak    │
└────────────────────────────────────────┘

Voice flow:
  [Hold 🎤] → expo-av records audio → release
          → STT (on-device or Whisper API)
          → populate text input → auto-send

OR:
  [Hold 🎤] → stream audio directly to 
          POST /api/nora/voice or /api/topsi/voice
          ← audio response (play via expo-av)
```

---

## 5. Data Flow — Chat Message (Text)

```
 Cipher                         ORCHA Server                  LLM
 ──────                         ────────────                  ───
                                                              
  user types message                                         
  taps send                                                  
        │                                                    
        │  POST /api/topsi/chat                              
        │  Authorization: Bearer <jwt>                       
        │  Content-Type: application/json                    
        │  {                                                 
        │    "message": "What's the TIACA status?",         
        │    "session_id": "uuid",                          
        │    "project_id": "optional"                       
        │  }                                                 
        │ ──────────────────────────────────────────────→   
        │                                                    
        │                  load TopsiAgent context          
        │                  build system prompt              
        │                  check VIBE balance               
        │                  if balance=0 → 402              
        │                                                   
        │                        stream to Claude API       
        │                        ──────────────────────→   
        │                                                   
        │  ← HTTP 200                                       
        │    Content-Type: text/event-stream                
        │    Transfer-Encoding: chunked                     
        │                                                   
        │  event: delta                                     
        │  data: {"text": "TIACA ES2026 is "}              
        │                                                   
        │  event: delta                                     
        │  data: {"text": "tracking well. "}               
        │                                                   
        │  event: delta                                     
        │  data: {"text": "14 posts are "}                 
        │                                                   
        │  event: done                                      
        │  data: {"usage": {"input":120,"output":85}}       
        │                                                   
  renderStreamingBubble(text)                               
  append token by token to message                          
        │                                                   
        │                  record_llm_usage()               
        │                  debit VIBE from balance          
        │                  update conversation history      
        │                                                   
  update VIBE balance display                               
```

---

## 6. Data Flow — Voice Message

```
 Cipher                ORCHA Server            TTS (ElevenLabs)
 ──────                ────────────            ────────────────
                                               
  [Hold 🎤 button]                            
  expo-av starts recording                    
                                              
  [Release button]                            
  audio buffer captured                       
                                              
  Option A — STT on device (preferred):       
    expo-speech or on-device Whisper          
    transcribed text → same as text flow      
                                              
  Option B — send raw audio:                  
    POST /api/nora/voice                      
    Content-Type: multipart/form-data         
    { audio: <wav>, session_id }              
    ──────────────────────────────────────→   
                                              
                   Whisper STT transcribes    
                   agent processes text       
                   TTS requested              
                   ─────────────────────→    
                   ←─────────────────────    
                   audio bytes returned       
                                              
    ←── 200 { text: "...", audio_url: "..." } 
                                              
  expo-av.Sound.loadAsync(audio_url)          
  sound.playAsync()                           
  ← agent speaks through earpiece            
```

---

## 7. Technical Stack

### Core

```
Framework:    React Native 0.76 via Expo SDK 52
Template:     @solana-mobile/solana-mobile-expo-template
Language:     TypeScript (strict mode)
Navigation:   React Navigation v6 (stack + bottom tabs)
State:        Zustand (matches dashboard pattern — same store shape)
HTTP:         axios with JWT interceptor (auto-attach + 401 refresh)
Streaming:    react-native-sse (10-line wrapper around SSE)
Storage:      @react-native-async-storage/async-storage (JWT persistence)
```

### Voice

```
Recording:    expo-av (microphone access)
Playback:     expo-av (TTS audio playback)
STT:          on-device via @react-native-voice/voice (offline-capable)
              fallback: Whisper API endpoint
TTS output:   ElevenLabs via /api/nora/voice route (already wired)
```

### Solana / Seeker (Progressive Enhancement)

```
Wallet:       @solana-mobile/mobile-wallet-adapter-protocol
Wallet hook:  useMobileWallet() from solana-mobile Expo template
SIWS:         sign-in-with-solana package
SGT check:    solana-mobile-dev-skill → /skills/genesis-token/
              (installs Claude Code skill for SGT verification)
Detection:    useMobileWallet().wallets.length > 0 → show Seeker button
```

### Build

```
Dev:          expo start --android (or --ios)
APK:          eas build --profile development --platform android
Seeker:       eas build --profile production --platform android
              → submit to dApp Store
iOS:          eas build --profile production --platform ios
              → TestFlight → App Store
```

---

## 8. Project Structure

```
cipher/
├── app/                          ← Expo Router file-based routing
│   ├── _layout.tsx               ← Root layout (auth guard)
│   ├── index.tsx                 ← Redirects to /login or /agents
│   ├── login.tsx                 ← LoginScreen
│   ├── agents.tsx                ← AgentSelectScreen
│   └── chat/
│       └── [agentId].tsx         ← ChatScreen (dynamic route)
│
├── components/
│   ├── MessageBubble.tsx          ← Reusable message + streaming
│   ├── VoiceButton.tsx            ← Hold-to-speak control
│   ├── AgentCard.tsx              ← Agent selection card
│   ├── VibeBalance.tsx            ← Balance display widget
│   └── StreamingText.tsx          ← Token-by-token render
│
├── hooks/
│   ├── useChat.ts                 ← SSE chat logic
│   ├── useVoice.ts                ← Record/play logic
│   ├── useAuth.ts                 ← JWT + AsyncStorage
│   └── useSeeker.ts               ← MWA detection + SIWS
│
├── store/
│   ├── auth.ts                    ← Zustand: user, token
│   ├── chat.ts                    ← Zustand: messages per agent
│   └── vibe.ts                    ← Zustand: balance
│
├── lib/
│   ├── api.ts                     ← axios instance + interceptors
│   ├── sse.ts                     ← react-native-sse wrapper
│   └── types.ts                   ← shared TypeScript types
│
├── constants/
│   ├── agents.ts                  ← Agent config (Topsi, Nora)
│   └── theme.ts                   ← Colors, spacing, typography
│
└── assets/
    ├── icon.png                   ← Cipher app icon
    ├── splash.png                 ← Splash screen
    └── fonts/                     ← Brand typography
```

---

## 9. API Contracts

All requests to: `https://dashboard.powerclubglobal.com/api/`  
Dev target: `http://[YOUR_LOCAL_IP]:3000/api/`  
Auth header: `Authorization: Bearer <jwt>`

### Auth

```typescript
// POST /api/auth/login
// ⚠️ MOBILE NOTE: Server uses session cookies (Set-Cookie: session_id).
// React Native: use axios cookieJar OR store session_id from response body
// and send as Cookie header on subsequent requests.
Request:  { username: string, password: string }   // ← username, NOT email
Response: { 
  user: UserProfile,
  session_id: string                               // also in Set-Cookie header
}
// Set-Cookie: session_id=<value>; HttpOnly; SameSite=Lax; Max-Age=2592000

// Mobile auth strategy:
// Option A: react-native-cookies + axios-cookiejar-support (auto-handles Set-Cookie)
// Option B: extract session_id from response body → store in AsyncStorage
//           → manually add Cookie: session_id=<value> to every request
// Option B is simpler for mobile. Verify backend accepts Cookie header (it should).
```

### Agents

```typescript
// GET /api/agents
// Returns all agents; regular users see system-tier + own agents
Response: Array<{
  id: string,
  short_name: string,        // "Topsi" | "Nora" | etc.
  display_name: string,
  description: string,
  agent_type: string,
  status: string,            // "active"
  model_config: object | null
}>
```

### Chat (Topsi — Blocking)

```typescript
// POST /api/topsi/chat  ← blocking response, NOT streaming
Request: {
  message: string,
  session_id: string,        // conversation session UUID
  project_id?: string,       // optional project context (UUID string)
  context?: object
}
Response: {
  message: string,           // full response text
  input_tokens?: number,
  output_tokens?: number
}
// No SSE for Topsi. Show typing indicator, await full response.
```

### Chat (Nora — Streaming SSE)

```typescript
// POST /api/nora/chat          ← blocking (same shape as Topsi)
// POST /api/nora/chat/stream   ← SSE streaming ✓

// Streaming request:
Request: {
  message: string,
  session_id: string,
  stream?: true,
  voice_enabled?: boolean,
  priority?: "Low" | "Normal" | "High" | "Urgent" | "Executive",
  project_id?: string
}

// Streaming response (text/event-stream):
// Axum sends: Event::default().data(chunk_text)
// Each SSE data line is a text chunk (NOT JSON-wrapped)
// Connection closes when stream ends.
// react-native-sse listener:
//   source.addEventListener('message', (e) => appendToken(e.data))
//   source.addEventListener('error', () => finalizeMessage())
```

### Voice

```typescript
// POST /api/nora/voice/synthesize   ← text → speech
Request:  { text: string, voice_profile?: string, speed?: number }
Response: { audio_data: string,    // base64-encoded MP3
            duration_ms: number,
            sample_rate: number,
            format: "Mp3" }

// POST /api/nora/voice/transcribe   ← audio → text
Request:  { audio_data: string,    // base64-encoded audio
            language?: string }
Response: { text: string }

// POST /api/nora/voice/interaction  ← audio in, audio out
// (combines transcribe + chat + synthesize in one round-trip)

// Same routes exist for Topsi:
// POST /api/topsi/voice/synthesize
// POST /api/topsi/voice/transcribe
// POST /api/topsi/voice/interaction
```

### VIBE

```typescript
// GET /api/projects/{project_id}/vibe/balance
Response: {
  project_id: string,
  total_deposited: number,
  total_withdrawn: number,
  total_spent: number,
  available_balance: number      // this is what to display
}

// HTTP 402 from any chat endpoint means:
{ error: "Insufficient VIBE balance" }
// App shows: "VIBE balance empty. Top up at dashboard.powerclubglobal.com"
```

---

## 10. State Management Schema

```typescript
// store/auth.ts
interface AuthState {
  token: string | null
  user: {
    id: string
    username: string
    full_name: string
    email: string
  } | null
  sgt_verified: boolean     // Seeker hardware proof attached
  isLoading: boolean
  error: string | null
  
  login: (email: string, password: string) => Promise<void>
  logout: () => void
  verifySGT: (signature: string, publicKey: string) => Promise<void>
  hydrate: () => Promise<void>   // load JWT from AsyncStorage on boot
}

// store/chat.ts
interface ChatState {
  sessions: Record<string, ChatSession>    // keyed by agentId
  activeAgentId: string | null
  
  sendMessage: (agentId: string, text: string) => Promise<void>
  appendStreamToken: (agentId: string, token: string) => void
  finalizeMessage: (agentId: string) => void
  clearSession: (agentId: string) => void
}

interface ChatSession {
  agentId: string
  sessionId: string           // UUID, stable per app session
  messages: ChatMessage[]
  isStreaming: boolean
  streamingText: string       // accumulates during SSE
}

interface ChatMessage {
  id: string
  role: 'user' | 'assistant'
  text: string
  timestamp: Date
  tokenUsage?: { input: number, output: number }
}

// store/vibe.ts
interface VibeState {
  balance: number | null
  isLoading: boolean
  refresh: () => Promise<void>
}
```

---

## 11. Build Phases

### Phase 0.1 — Core Shell (1-2 days)
*Goal: Login → pick agent → text chat working on device*

```
[ ] Scaffold: yarn create expo-app cipher --template @solana-mobile/solana-mobile-expo-template
[ ] Strip demo wallet screens, keep navigation shell
[ ] Install dependencies: axios, zustand, react-native-sse, async-storage
[ ] Implement auth store + LoginScreen
[ ] Implement AgentSelectScreen (fetch /api/agents, show Topsi + Nora)
[ ] Implement ChatScreen with text input
[ ] Wire SSE streaming via react-native-sse
[ ] Show VIBE balance in header (poll every 30s)
[ ] Test on Android device: login → Topsi → ask "what are my open tasks?"
[ ] Handle 402 gracefully (VIBE empty message)
[ ] Handle 401 gracefully (re-login redirect)
```

### Phase 0.2 — Voice (2-3 days)
*Goal: Hold button → speak → hear Nora/Topsi respond*

```
[ ] Add expo-av and @react-native-voice/voice
[ ] Implement VoiceButton (hold-to-record UX, waveform animation)
[ ] On-device STT transcription → auto-populate text → send
[ ] Implement audio playback for TTS responses from /api/nora/voice
[ ] Voice mode toggle: tap 🎤 for voice session, keyboard for text
[ ] Test: "Nora, what deals are in the pipeline?" → hear her respond
[ ] Silence detection to auto-stop recording
```

### Phase 0.3 — Sovereign Layer (2-3 days)
*Goal: Seeker hardware auth + APN presence + VIBE display*

```
[ ] Detect MWA availability (useMobileWallet().wallets.length > 0)
[ ] Add "Sign In with Seeker" button on LoginScreen (conditional)
[ ] SIWS flow: request signature from Seed Vault → POST /api/auth/sovereign-proof
[ ] Show "Sovereign ✓" badge when SGT verified
[ ] Add Cipher as APN edge node (announce presence on NATS)
[ ] Push notifications via Expo Notifications (agent async responses)
[ ] VIBE balance with "Top Up" deep link to dashboard
[ ] dApp Store submission prep (store listing, screenshots, description)
```

### Phase 0.4 — Polish (1-2 days)
*Goal: Production-ready, ready for team use*

```
[ ] Dark theme (matches ORCHA dashboard aesthetic)
[ ] Haptic feedback on voice button press
[ ] Conversation history persistence (AsyncStorage per agent)
[ ] Connection status indicator (online/offline/reconnecting)
[ ] Error boundary with retry for SSE disconnects
[ ] EAS build configured for both dev and production profiles
[ ] iOS build (same codebase, EAS handles it)
[ ] App icon + splash screen (Cipher brand assets)
```

---

## 12. Design Language

```
Colors:
  Background:     #0A0A0F   (near black, sovereign depth)
  Surface:        #13131A   (card/sheet background)
  Border:         #1E1E2E   (subtle separator)
  Accent:         #5B4FE8   (PCG purple — matches dashboard)
  Text Primary:   #FFFFFF
  Text Secondary: #8888AA
  VIBE Gold:      #F5A623   (balance indicator)
  Online Green:   #22D55A
  Topsi Blue:     #3B82F6   (agent accent)
  Nora Purple:    #8B5CF6   (agent accent)
  
Typography:
  Headings:  Cinzel (matches dashboard brand font — already on Google Fonts)
  Body:      System font (-apple-system / Roboto)
  Monospace: JetBrains Mono (for any code in responses)

Spacing: 8pt grid (8, 16, 24, 32, 48)

Motion:
  Stream cursor blink: 500ms pulse
  Voice waveform: real-time amplitude bars
  Message appear: 150ms fade+slide up
  Agent card press: scale 0.97
```

---

## 13. Environment Configuration

```bash
# .env (cipher app root)
EXPO_PUBLIC_API_URL=https://dashboard.powerclubglobal.com
EXPO_PUBLIC_API_URL_DEV=http://192.168.X.X:3000   # replace with local IP
EXPO_PUBLIC_APP_NAME=Cipher
EXPO_PUBLIC_VERSION=0.1.0

# EAS project config (eas.json)
{
  "build": {
    "development": {
      "developmentClient": true,
      "distribution": "internal",
      "env": { "EXPO_PUBLIC_API_URL": "http://192.168.X.X:3000" }
    },
    "production": {
      "env": { "EXPO_PUBLIC_API_URL": "https://dashboard.powerclubglobal.com" }
    }
  }
}
```

**Important dev note**: When testing on a physical device, `localhost` won't resolve to your dev machine. Use your machine's LAN IP address (`ip addr show | grep 192.168`). The ORCHA server must be started with `HOST=0.0.0.0` to accept connections from the device.

---

## 14. What NOT to Build in Cipher

This list exists because scope creep is the #1 risk. If the following comes up, push it back to the dashboard:

| Feature | Why Not | Where It Lives |
|---------|---------|----------------|
| Workflow builder | Complex graph UI, needs canvas | ORCHA Dashboard |
| CRM pipeline view | Kanban/list views, lots of data | ORCHA Dashboard |
| Social calendar | Calendar grid + drag-drop | ORCHA Dashboard |
| Deliverable review | File previews, multi-step | ORCHA Dashboard |
| Admin settings | Complex forms, dangerous ops | ORCHA Dashboard |
| Agent configuration | Developer-facing, complex | ORCHA Dashboard |
| File uploads | Large files, no camera needed yet | ORCHA Dashboard |
| Analytics dashboards | Charts, complex data viz | ORCHA Dashboard |

The test: *"Can this be done entirely in one voice sentence?"* If yes, it might belong in Cipher. If it requires more than one screen of visual structure, it belongs in the dashboard.

---

## 15. Testing Checklist (Pre-Ship)

```
Auth
[ ] Login with valid credentials → lands on AgentSelectScreen
[ ] Login with invalid credentials → shows error, stays on LoginScreen
[ ] Token expiry (after 7 days) → redirected to LoginScreen
[ ] VIBE balance = 0 → 402 handled gracefully

Topsi Chat
[ ] "What are my open tasks?" → streams a coherent response
[ ] Long response (100+ words) → all tokens render correctly
[ ] SSE disconnects mid-stream → reconnect + show partial message
[ ] Consecutive messages → conversation context maintained

Nora Chat
[ ] "What deals are in the pipeline?" → Nora responds in context
[ ] Nora voice response plays through speaker

Voice (Phase 0.2)
[ ] Hold button → mic activates (waveform shows)
[ ] Release button → transcription appears in text box
[ ] Auto-send after transcription → response streams

Seeker (Phase 0.3, Seeker device only)
[ ] Seeker button appears only on Seeker device
[ ] SIWS flow completes → "Sovereign ✓" badge shown
[ ] Non-Seeker device → Seeker button hidden, standard auth only

Network
[ ] App works on 4G (not just WiFi)
[ ] Airplane mode → offline indicator shown, graceful error
[ ] Reconnect after offline → session resumes
```

---

## Appendix: Key Backend File Locations

For the execution terminal, these are the files you will most likely need to reference or modify:

```
Auth:
  crates/server/src/routes/auth.rs           ← login/logout/me endpoints
  crates/db/src/models/user.rs               ← User model

Agent chat:
  crates/server/src/routes/topsi.rs          ← Topsi chat + voice routes
  crates/server/src/routes/nora.rs           ← Nora chat + voice routes
  crates/nora/src/agent.rs                   ← NoraAgent implementation

Agent list:
  crates/server/src/routes/agents.rs         ← GET /api/agents
  crates/db/src/models/agent.rs              ← Agent model

VIBE balance:
  crates/server/src/routes/vibe.rs           ← balance endpoints
  crates/db/src/models/vibe_transaction.rs   ← transaction model

Streaming:
  Look for axum SSE pattern: Sse<impl Stream>
  Search for: use axum::response::sse

Voice pipeline:
  crates/nora/src/voice/gateway.rs           ← VoiceGateway (638 lines)
  crates/server/src/routes/topsi/voice.rs    ← voice route handlers

MCP servers:
  crates/server/src/routes/topsi_mcp.rs      ← TopsiServer
  crates/server/src/routes/nora_mcp.rs       ← NoraServer

Pulse/NATS:
  crates/server/src/pulse_publisher.rs       ← global NATS publisher
  crates/server/src/pulse_consumer.rs        ← NATS consumer

APN:
  docs/APN_INTEGRATION_ARCHITECTURE.md       ← full APN architecture
```

---

*Cipher is deliberately small. A few screens, a voice button, a real-time stream. The power is not in the app — it's in what it connects to. The Sovereign Stack is the product. Cipher is the key.*
