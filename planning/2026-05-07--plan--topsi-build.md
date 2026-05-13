# Topsi — Detailed Build Plan

**Voice-First Solana Mobile App + SPL Gas Token for the Sovereign Stack Hackathon**

Date: 2026-05-07
Author: drafted with Bodhi
Status: Plan, not yet approved for build
Target devices: Solana Seeker (flagship), Android phones (broad), iPhone (broad)

---

## Table of Contents

0. Executive Summary
1. The Topsi Trinity (agent + app + token)
2. Strategic Positioning & Hackathon Win Theory
3. System Architecture (topology diagrams)
4. Stack & Dependencies
5. Module Structure
6. Screen Architecture (3-screen wireframes)
7. Auth Flow (JWT + SGT progressive enhancement)
8. Agent Chat Flow (SSE streaming + voice)
9. Gas Metering Flow (the strategic core)
10. $TOPSI Token Lifecycle (state diagram)
11. API Contracts
12. Configuration & Secrets
13. Phased Roadmap (0.1 → 0.2 → 0.3 → 1.0)
14. dApp Store Submission Checklist
15. Token Mint Sequence (last phase)
16. Hackathon Demo Storyboard
17. Risk Register
18. Open Decisions
19. Verification Tasks (must-confirm-in-code before build)
20. Tonight's Concrete First Move

---

## 0 · Executive Summary

**Topsi** is a unified product brand (agent + app + token) being built for submission to a Solana Mobile hackathon. The agent is already live inside `pcg-cc-mcp`; the app is a React Native + Expo voice terminal that ships to Android, iOS, and the Solana Seeker; the token is an SPL on Solana mainnet that acts as on-chain gas — every message to the Topsi agent costs $TOPSI at 2× the existing VIBE-per-message rate.

Sequencing locked by Bodhi:
1. Build the app
2. Push the app to the Solana dApp Store
3. Mint $TOPSI on mainnet last (after the product exists to point at)

Until the token is minted, the app meters via a **server-side virtual $TOPSI balance** with the same UI it will use post-mint. At mint time, the data source flips from server counter to on-chain RPC reads with no UI change.

**Why this wins the hackathon:** most Solana Mobile submissions are wallet UX or speculative tokens. Topsi is a real utility product — metered AI conversation native to Solana, gated to verified Seeker holders via Seeker Genesis Token (SGT), and demonstrating the full sovereign stack story (hardware identity + voice + agent + token) in one ~5-minute demo.

---

## 1 · The Topsi Trinity

```
        AGENT                  APP                    TOKEN
     ┌────────────┐       ┌──────────────┐        ┌────────────┐
     │   Topsi    │       │    Topsi     │        │   $TOPSI   │
     │  (agent)   │ ────► │  (RN/Expo)   │ ─────► │   (SPL)    │
     │            │ ◄──── │              │ ◄───── │            │
     └────────────┘       └──────────────┘        └────────────┘
        speaks               voice terminal           burned per
        listens              for ORCHA                conversation
        Pythia's offspring   Android/iOS/Seeker       2× VIBE rate
```

The trinity is the marketing surface. One name carries three things and the three reinforce each other: you open Topsi (the app), you talk to Topsi (the agent), and your Topsi (the balance) goes down. There is no friction in the story — no "wait, the app is called X, the token is called Y, the agent is called Z."

> Cipher remains the Stage-2 brand for when ORCHA opens to external clients who don't know the PCG-internal names. For now, everything is Topsi.

---

## 2 · Strategic Positioning & Hackathon Win Theory

A hackathon judge sees ~30 demos. Most blur. The five things that make Topsi unforgettable, ordered by how rare each is in the typical Solana Mobile submission:

| # | Feature                              | How rare in submissions   | Why it lands     |
|---|--------------------------------------|---------------------------|------------------|
| 1 | SGT-gated access                     | Almost nobody does this   | Anti-Sybil; "you can only use this if you really hold a Seeker" — proves Solana Mobile is the paved road, not a sticker |
| 2 | $TOPSI as on-chain gas for AI calls  | Nobody does this          | Real economic loop, not speculation. Token = utility from minute one |
| 3 | SIWS auth                            | Some do                   | Proper Web3 auth, not just JWT |
| 4 | Native voice (in + out)              | Few attempt, fewer ship   | Voice is what mobile is FOR; PWA wrappers can't really do it |
| 5 | Cross-platform with Seeker as flagship | Rare                    | Same APK works on iPhone for the team, but Seeker users get more — that's narrative gold |

**The 5-minute pitch:** "I open Topsi on my Seeker. The app verifies my Seeker Genesis Token. I sign in with my wallet. I see my $TOPSI balance — 1.25 million tokens. I hold the mic, ask Topsi what's in my pipeline this week. Topsi answers in voice, my balance ticks down by the metered amount. That's the loop. The agent is paid in $TOPSI; $TOPSI is gas; gas is verified; the device proves you. Same app on iPhone for my team — JWT auth only, no SGT path."

Each sentence in that pitch is one feature shipped.

---

## 3 · System Architecture

### 3.1 Topology overview

```
   ┌──────────────────────────────────────────────────────────────────────┐
   │                     CLIENT TIER (cross-platform)                     │
   │                                                                      │
   │   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                │
   │   │   Seeker    │   │   iPhone    │   │  Android    │                │
   │   │ Topsi APK   │   │ Topsi IPA   │   │ Topsi APK   │                │
   │   │             │   │             │   │             │                │
   │   │  + SGT      │   │   JWT only  │   │  JWT only   │                │
   │   │  + SIWS     │   │             │   │             │                │
   │   │  + Seed     │   │             │   │             │                │
   │   │    Vault    │   │             │   │             │                │
   │   └──────┬──────┘   └──────┬──────┘   └──────┬──────┘                │
   └──────────┼─────────────────┼─────────────────┼──────────────────────┘
              │                 │                 │
              │  HTTPS          │  HTTPS          │  HTTPS
              │  Bearer JWT     │  Bearer JWT     │  Bearer JWT
              │  (+SIWS proof)  │                 │
              ▼                 ▼                 ▼
   ┌──────────────────────────────────────────────────────────────────────┐
   │                  EDGE / API: dashboard.powerclubglobal.com           │
   │                       (pcg-cc-mcp Rust/Axum, deployed)               │
   │                                                                      │
   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │
   │   │  /api/auth   │  │  /api/agents │  │ /api/agents/ │               │
   │   │  /login      │  │              │  │ {id}/chat    │               │
   │   │  /me         │  │              │  │ /stream (SSE)│               │
   │   └──────────────┘  └──────────────┘  └──────────────┘               │
   │                                                                      │
   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │
   │   │  /api/topsi  │  │  /api/vibe   │  │  /api/billing│               │
   │   │  /voice      │  │  /balance    │  │  /rate (NEW) │               │
   │   │  (TTS)       │  │              │  │              │               │
   │   └──────────────┘  └──────────────┘  └──────────────┘               │
   │                                                                      │
   │   gas-metering middleware (NEW: per-msg $TOPSI debit)                │
   │   nora crate → Anthropic / OpenAI / Ollama                           │
   │   SQLite (sessions, users, agents, conversations, balances)          │
   └──────────────────────────────────────────────────────────────────────┘
              │                                              │
              │ peer sync (NATS)                             │ on-chain reads
              ▼                                              ▼
   ┌──────────────────────────┐                ┌─────────────────────────┐
   │   APN (peer overlay)     │                │ Solana mainnet          │
   │   Pythia Master Node     │                │  - $TOPSI mint          │
   │   sync across nodes      │                │  - SGT NFT lookup       │
   │                          │                │  - VIBE balance reads   │
   └──────────────────────────┘                │    (Aptos via bridge or │
                                                │     mirrored read API) │
                                                └─────────────────────────┘
```

### 3.2 Why this shape

- **Single backend.** No new server tier. Topsi the app is just a second client of pcg-cc-mcp, alongside the existing dashboard. Backend changes are additive (gas-metering middleware, billing rate endpoint), not architectural.
- **Bearer JWT works for all platforms.** The session cookie is hard cross-origin; Bearer tokens are not. We use the existing localStorage-fallback path the dashboard already supports.
- **SGT is progressive.** Same Topsi binary on every platform; Seeker just unlocks an extra auth+token-balance path.
- **Token reads are direct from Solana.** No backend proxy for $TOPSI balance — the app reads directly from a mainnet RPC (Helius / Triton / public). Faster and trustless.

### 3.3 Network boundaries

```
   Trust zone                  | Boundary                  | Crossing rule
   --------------------------- | ------------------------- | ----------------------
   Topsi app on phone          | TLS to API                | Bearer JWT in header
   pcg-cc-mcp                  | TLS to LLM provider       | Provider API key
   pcg-cc-mcp                  | NATS to APN peers         | Peer identity (mTLS)
   Topsi app on phone          | TLS to Solana RPC         | None (read-only)
   Topsi app on Seeker         | local IPC to Seed Vault   | Android permission
   Topsi app on Seeker         | local IPC to MWA wallet   | User consent
```

---

## 4 · Stack & Dependencies

### 4.1 Mobile app (Topsi)

```
 Layer            Package                                          Why
 ---------------- ------------------------------------------------ ----------
 Framework        react-native 0.76                                Mobile core
 Runtime          expo 52                                          Build, OTA, dev client
 Language         typescript 5.x                                   Static checks
 UI               react-native-paper 5.12                          Material design components
 Nav              @react-navigation/native 6                       Three-screen stack
 State            @tanstack/react-query 5                          Server state
 State (local)    zustand                                          Tiny global store
 Storage          @react-native-async-storage/async-storage 1.23   JWT, prefs
 HTTP             axios + axios-retry                              Bearer interceptor
 Streaming        react-native-sse                                 SSE on RN
 Voice in         @react-native-voice/voice                        STT
 Voice out        expo-speech (fallback) + remote TTS              Native + ElevenLabs
 Audio playback   expo-av                                          MP3 from server TTS
 Solana web3      @solana/web3.js 1.95                             Mainnet reads
 SPL token        @solana/spl-token 0.4                            $TOPSI ATA reads
 MWA              @solana-mobile/mobile-wallet-adapter-protocol-web3js 2.1   Wallet adapter
 Seed Vault       @solana-mobile/seed-vault-lib (as available)     Hardware keys, Seeker only
 SIWS             @solana/wallet-standard-features                 Sign-in with Solana
 Polyfills        react-native-get-random-values, buffer           web3.js needs these
 Push             expo-notifications                               Async agent results
 Permissions      expo-permissions / inline                        Mic, notifications
```

Bootstrap: `yarn create expo-app topsi-sol-dapp --template @solana-mobile/solana-mobile-expo-template` (gives us most of the Solana plumbing already wired: MWA, polyfills, web3.js, Paper). Repo: **Soverign-Stack/topsi-sol-dapp** (locked). Package id: **global.powerclubglobal.topsi** (locked).

### 4.2 Backend (pcg-cc-mcp) — additive only

```
 Change                            Impact
 --------------------------------- ----------------------------------------
 Add /api/billing/rate             Returns current per-message rates in
                                   VIBE and $TOPSI. App uses this to display
                                   accurate gas cost before each message.
 Add gas-metering middleware       Wraps agent chat handlers; debits the
                                   user's $TOPSI virtual balance per message.
 Add /api/topsi/balance            Server-side virtual $TOPSI balance per
                                   user. Pre-mint, this is canonical.
                                   Post-mint, may be deprecated in favor of
                                   on-chain reads.
 Add /api/topsi/balance/airdrop    Adds N to a user's virtual balance. Used
                                   during demo + by /claim funnel.
 ALLOWED_ORIGINS                   Add Topsi app dev origins (Expo dev URL)
                                   for browser-Web preview if we ship one.
 SGT verification endpoint         /api/auth/sgt-verify — accepts a SIWS-
                                   signed challenge and SGT mint address,
                                   confirms ownership server-side.
```

No deletions. No schema migrations. No deployment of a second service.

### 4.3 Token (post-build)

```
 Item             Choice                           Why
 ---------------- -------------------------------- --------------------------
 Standard         SPL Token                        Native Solana
 Decimals         9                                Convention; matches SOL
 Supply           1,000,000,000 (1B fixed)         Locked by Bodhi
 Mint authority   Revoked post-initial-mint        No more $TOPSI ever
 Freeze authority Revoked post-initial-mint        No account freezes
 Metadata         Metaplex Token Metadata          Required for wallets
 Image hosting    Arweave (or IPFS / pinned)       Persistent
 LP at launch     None                             Defer to post-hackathon
 Treasury         Fresh hardware-backed wallet     Not Bodhi's daily driver
```

---

## 5 · Module Structure

```
   topsi/
   ├── app.config.ts                ← Expo config, EAS, deep links
   ├── app.json                     ← legacy Expo config
   ├── package.json
   ├── tsconfig.json
   ├── babel.config.js
   ├── metro.config.js
   ├── eas.json                     ← EAS build profiles
   │
   ├── src/
   │   ├── App.tsx                  ← root, providers, navigator
   │   │
   │   ├── lib/                     ← cross-cutting infra
   │   │   ├── api.ts               ← axios client, Bearer interceptor
   │   │   ├── env.ts               ← typed env vars
   │   │   ├── storage.ts           ← AsyncStorage helpers
   │   │   ├── solana.ts            ← Connection, RPC, token reads
   │   │   └── platform.ts          ← isSeeker(), isAndroid(), isIOS()
   │   │
   │   ├── auth/
   │   │   ├── api.ts               ← login/me/logout
   │   │   ├── store.ts             ← zustand auth slice
   │   │   ├── interceptor.ts       ← inject Bearer
   │   │   ├── siws.ts              ← SIWS challenge + verify
   │   │   └── sgt.ts               ← Seeker Genesis Token check
   │   │
   │   ├── agents/
   │   │   ├── api.ts               ← list, chat, stream
   │   │   ├── types.ts             ← Agent, ChatChunk, etc.
   │   │   └── hooks.ts             ← useAgents(), useAgentChat()
   │   │
   │   ├── billing/
   │   │   ├── api.ts               ← rate, balance
   │   │   ├── source.ts            ← swappable: server | onchain
   │   │   ├── debit.ts             ← optimistic UI debit
   │   │   └── format.ts            ← display 1,234,567 $TOPSI
   │   │
   │   ├── voice/
   │   │   ├── stt.ts               ← @react-native-voice/voice wrapper
   │   │   ├── tts.ts               ← server TTS playback (expo-av)
   │   │   ├── permissions.ts       ← mic permission flow
   │   │   └── PushToTalk.tsx       ← UI component
   │   │
   │   ├── screens/
   │   │   ├── LoginScreen.tsx
   │   │   ├── AgentSelectScreen.tsx
   │   │   └── ChatScreen.tsx
   │   │
   │   ├── components/
   │   │   ├── BalancePill.tsx      ← header: 1,247,500 $TOPSI
   │   │   ├── AgentTile.tsx        ← Topsi / Nora cards
   │   │   ├── MessageBubble.tsx
   │   │   ├── StreamingText.tsx    ← animates SSE chunks
   │   │   ├── DebitToast.tsx       ← "−2 $TOPSI" floating
   │   │   ├── SeekerBadge.tsx      ← "Seeker verified"
   │   │   └── DesignSystem/        ← theme, colors, type
   │   │
   │   ├── theme/
   │   │   ├── colors.ts            ← Sovereign Stack palette
   │   │   ├── typography.ts
   │   │   └── paper.ts             ← Paper theme override
   │   │
   │   └── navigation/
   │       ├── AuthStack.tsx
   │       └── AppStack.tsx
   │
   ├── android/                     ← Expo prebuild output
   ├── ios/                         ← Expo prebuild output (later)
   ├── assets/
   │   ├── icon.png                 ← 1024×1024
   │   ├── splash.png
   │   ├── adaptive-icon.png
   │   └── topsi-mark.png           ← reused as $TOPSI metadata image
   │
   └── docs/
       ├── PRIVACY.md               ← required for dApp Store
       └── TERMS.md                 ← required for dApp Store
```

---

## 6 · Screen Architecture

### 6.1 LoginScreen

```
   ┌──────────────────────────────────────────────┐
   │                                              │
   │             [ Topsi mark, large ]            │
   │                                              │
   │             T O P S I                        │
   │      voice terminal for ORCHA                │
   │                                              │
   │   ┌──────────────────────────────────┐       │
   │   │  Email                           │       │
   │   └──────────────────────────────────┘       │
   │                                              │
   │   ┌──────────────────────────────────┐       │
   │   │  Password                  ●●●●● │       │
   │   └──────────────────────────────────┘       │
   │                                              │
   │   ┌──────────────────────────────────┐       │
   │   │           Sign In                │       │
   │   └──────────────────────────────────┘       │
   │                                              │
   │   ─────────── on Seeker only ───────────     │
   │                                              │
   │   ┌──────────────────────────────────┐       │
   │   │   ◆  Sign in with your Seeker    │       │
   │   └──────────────────────────────────┘       │
   │                                              │
   └──────────────────────────────────────────────┘
```

Behaviour:
- iPhone / regular Android: only the email + password form is shown.
- Seeker: extra "Sign in with your Seeker" button uses MWA → SIWS → SGT verify, in one tap.
- Email-password is offered as fallback even on Seeker for the case where the user wants to log in as someone else.

### 6.2 AgentSelectScreen

```
   ┌──────────────────────────────────────────────┐
   │  Bodhi · ◆ Seeker verified · 1,247,500       │
   │                                       $TOPSI │
   ├──────────────────────────────────────────────┤
   │                                              │
   │   Who do you want to talk to?                │
   │                                              │
   │   ┌────────────────────────────────────┐     │
   │   │                                    │     │
   │   │   🤖   Topsi                       │     │
   │   │        platform default            │     │
   │   │        2 $TOPSI / message          │     │
   │   │                                    │     │
   │   └────────────────────────────────────┘     │
   │                                              │
   │   ┌────────────────────────────────────┐     │
   │   │                                    │     │
   │   │   🎙   Nora                        │     │
   │   │        executive (admin)           │     │
   │   │        4 $TOPSI / message          │     │
   │   │                                    │     │
   │   └────────────────────────────────────┘     │
   │                                              │
   │              [ Sign out ]                    │
   │                                              │
   └──────────────────────────────────────────────┘
```

Behaviour:
- Header pills (Seeker verified, $TOPSI balance) are read on mount and on focus.
- Per-agent gas rate is shown — transparency builds trust in the metering.
- Nora tile is dimmed/hidden if the user is not an admin.
- Tapping a tile navigates to ChatScreen.

### 6.3 ChatScreen

```
   ┌──────────────────────────────────────────────┐
   │  ‹  Topsi                          1,247,498 │
   │     platform default                  $TOPSI │
   ├──────────────────────────────────────────────┤
   │                                              │
   │   ┌─────────────────────────┐                │
   │   │  Hi Bodhi. What do you  │                │
   │   │  want to do?            │                │
   │   └─────────────────────────┘                │
   │       Topsi · just now                       │
   │                                              │
   │              ┌─────────────────────────────┐ │
   │              │  what's in my pipeline this │ │
   │              │  week                       │ │
   │              └─────────────────────────────┘ │
   │                                  you · now   │
   │                                              │
   │   ┌─────────────────────────────────────┐    │
   │   │  You have 4 active deals            │    │
   │   │  totalling $2.4M. Two are blocked   │    │
   │   │  on your reply, one is▎             │    │
   │   └─────────────────────────────────────┘    │
   │       Topsi · streaming…       −2 $TOPSI ↘   │
   │                                              │
   ├──────────────────────────────────────────────┤
   │   ┌────────────────────────────────────┐     │
   │   │  ●  Hold to speak     or type ↓    │     │
   │   └────────────────────────────────────┘     │
   └──────────────────────────────────────────────┘
```

Behaviour:
- Streaming text animates as SSE chunks arrive (cursor █ visible while in flight).
- TTS plays interleaved as chunks arrive (chunked playback feels natural).
- Each completed assistant message triggers a `−N $TOPSI` micro-toast and animates the header balance down.
- Push-to-talk: hold the mic button → STT runs locally → on release, the recognized text is auto-submitted.
- Text input is a fallback (always present below the mic for accessibility / silent contexts).

---

## 7 · Auth Flow

### 7.1 Standard JWT (all devices)

```
   Topsi App                       pcg-cc-mcp
   ─────────                       ──────────
        │
        │ 1. POST /api/auth/login
        │    { email, password }
        │ ─────────────────────────►
        │                              bcrypt verify
        │                              create 7-day session
        │
        │ 2. 200 { user, session_id }
        │ ◄─────────────────────────
        │
        │ 3. AsyncStorage.setItem(
        │      'jwt', session_id)
        │
        │ 4. axios interceptor adds
        │    Authorization: Bearer ...
        │    on every subsequent call
        │
        │ 5. GET /api/auth/me  ─────►
        │ ◄── { user profile } ─────
        ▼
   navigate to AgentSelect
```

### 7.2 SGT progressive enhancement (Seeker only)

```
   Topsi App on Seeker             MWA wallet           Solana RPC          pcg-cc-mcp
   ────────────────────             ──────────           ──────────          ──────────
        │
        │ "Sign in with your Seeker" tap
        │
        │ requestAuthorization() ─────►
        │ ◄───── pubkey + auth token ─
        │
        │ build SIWS challenge:
        │   "Sign in to Topsi at
        │    {timestamp} ({nonce})"
        │
        │ signMessage(challenge) ─────►
        │ ◄──── signature ────────────
        │
        │ POST /api/auth/sgt-verify ────────────────────────────────────────►
        │   { pubkey, challenge, signature }                                  │
        │                                                                    │
        │                                       backend queries              │
        │                                       Solana RPC for SGT NFT       │
        │                                       at this pubkey               │
        │                                       ─────────────────────────►   │
        │                                                                    │
        │                                       ◄───── SGT mint found ──     │
        │                                                                    │
        │                                       verify signature             │
        │                                       lookup or create user        │
        │                                       issue session                │
        │                                                                    │
        │ ◄─────── 200 { user, session_id, sgt_verified: true } ────────────
        ▼
   navigate to AgentSelect with Seeker badge
```

Notes:
- The session that comes out the other side is the **same JWT shape** as the email/password path. The downstream code doesn't care which path produced the token; the only difference is `user.sgt_verified === true` which is what unlocks the airdrop claim and the Seeker badge on the screen.
- We don't store the wallet keypair anywhere in the app; the MWA / Seed Vault interaction is per-session.
- The challenge has a short TTL (60s) and a server-side nonce to prevent replay.

---

## 8 · Agent Chat Flow

### 8.1 Streaming text

```
   ChatScreen                   pcg-cc-mcp                    LLM provider
   ──────────                   ──────────                    ────────────
        │
        │ user submits message
        │
        │ POST /api/agents/{id}/chat/stream
        │   Accept: text/event-stream
        │   body: { message, session_id }
        │ ─────────────────────────────►
        │                                 (1) gas-metering middleware
        │                                     check user has balance
        │                                     return 402 if not
        │                                 (2) reserve N $TOPSI
        │                                     (atomic decrement)
        │                                 (3) call provider streaming
        │                                 ─────────────────────────►
        │                                 ◄────── chunks ─────────
        │
        │ ◄─── data: {"content":"You "}
        │ ◄─── data: {"content":"have "}
        │ ◄─── data: {"content":"4 deals..."}
        │ ◄─── data: [DONE]
        │
        │ accumulate → animate
        │ play TTS chunked
        │ animate −N $TOPSI debit toast
        │ refresh header balance
        ▼
```

### 8.2 Streaming + voice round-trip

```
   user voice          STT                  agent             TTS              speaker
   ──────────          ────                 ─────             ───              ───────
       │ hold mic        │                    │                 │                 │
       │────────────────►│ transcribe partial │                 │                 │
       │                 │  on-device         │                 │                 │
       │ release         │                    │                 │                 │
       │────────────────►│ final transcript   │                 │                 │
       │                 │───────────────────►│ stream chunks   │                 │
       │                 │                    │────────────────►│  TTS chunks      │
       │                 │                    │                 │────────────────►│
       │                 │                    │                 │  audio plays    │
       │                 │                    │ DONE            │                 │
```

For Phase 0.2 (voice), we have two TTS choices:
- **expo-speech (on-device)** — instant, free, uniform voice across devices. Lower production value.
- **Server-side TTS via topsi/voice.rs** — 638 lines of voice routes already exist in pcg-cc-mcp. Likely streams ElevenLabs / Chatterbox MP3 chunks back, which Topsi plays via `expo-av`. Better personality match per agent.

We default to server-side and fall back to expo-speech if the server endpoint is offline.

---

## 9 · Gas Metering Flow (the strategic core)

This is the diagram that wins the hackathon if we get it right.

### 9.1 Lifecycle of a single message

```
   t = 0     user sends message
             ─────────────────────────────────►
             POST /api/agents/{id}/chat/stream

   t = 1     gas middleware intercepts
             ─────────────────────────────────►
             read /api/billing/rate   →   N $TOPSI per message
             read user balance        →   B
             if B < N    return 402 Insufficient Balance
                         (app shows "low balance" + airdrop hint)
             else         decrement B by N (atomic, transactional)

   t = 2     LLM provider stream begins
             ─────────────────────────────────►
             chunks flow back over SSE

   t = 3     stream completes
             ─────────────────────────────────►
             commit decrement
             write transaction record:
                { user_id, agent_id, tokens_spent, message_id, ts }

   t = 4     response complete on client
             ─────────────────────────────────►
             app shows debit toast
             app refetches /api/topsi/balance
             header BalancePill animates down
```

### 9.2 Pre-mint vs post-mint

```
                 PRE-MINT (server-side virtual)              POST-MINT (on-chain)
                 ────────────────────────────────             ────────────────────────
 Source of       SQLite table: user_topsi_balance             Solana RPC: getBalance()
 truth           in pcg-cc-mcp                                 on user's $TOPSI ATA

 Debit method    UPDATE user_topsi_balance                    spl-token transferChecked
                 SET balance = balance - N                    from user ATA →
                 WHERE user_id = ?                            treasury ATA (or burn)

 Authority       backend has full control                     user signs each debit via
                                                              MWA on each message? OR
                                                              we maintain a "Topsi
                                                              gas escrow" pattern: user
                                                              tops up an escrow wallet,
                                                              backend signs debits from
                                                              the escrow.

 Verification    none (client trusts server)                  user can verify on-chain

 Implementation  ~2 hours backend + 1 hour client             ~1 day, depends on the
                                                              escrow design
```

The **swappable interface** in `src/billing/source.ts`:

```typescript
   // src/billing/source.ts (sketch)

   interface BalanceSource {
     read(userId: string): Promise<bigint>;
     subscribe(userId: string, cb: (b: bigint) => void): Unsubscribe;
   }

   class ServerBalanceSource implements BalanceSource { ... }
   class OnChainBalanceSource implements BalanceSource { ... }

   export const balanceSource: BalanceSource =
     env.TOPSI_TOKEN_MINTED
       ? new OnChainBalanceSource(env.TOPSI_MINT, env.RPC_URL)
       : new ServerBalanceSource(api);
```

UI never sees the difference. We ship the entire app on the server source, demo it that way, then flip the env flag at mint time.

### 9.3 Pricing math

Given the user's spec — "$TOPSI rate is 2× the VIBE-per-message rate, which is already inflated value over the LLM cost" — we need to confirm in code:

| Value                              | Where it lives    | Verification needed |
|------------------------------------|-------------------|---------------------|
| Underlying LLM cost / message      | provider invoices | n/a                 |
| VIBE rate per message              | pcg-cc-mcp config | **VERIFY in code**  |
| $TOPSI rate per message            | derived (2× VIBE) | **DEFINE**          |

The plan: expose both rates via `GET /api/billing/rate` returning:

```json
{
  "underlying_llm_cost_usd_per_msg": 0.012,
  "vibe_rate_per_msg": 0.05,
  "vibe_markup_multiple": 4.16,
  "topsi_rate_per_msg": 0.10,
  "topsi_markup_multiple": 8.33,
  "agent_overrides": {
    "topsi": { "topsi_rate_per_msg": 2 },
    "nora":  { "topsi_rate_per_msg": 4 }
  }
}
```

Topsi the app reads this on AgentSelect and ChatScreen mount; rates are shown to the user. Rate changes are instant — no client deploy needed. Every value here is **verifiable in code before we commit** to specifics; see Section 19.

---

## 10 · $TOPSI Token Lifecycle

```
                          ┌──────────────────────────┐
                          │  Phase 0: not minted yet │
                          │  app uses server-virtual │
                          │  balances exclusively    │
                          └────────────┬─────────────┘
                                       │
                                       │  app reaches Phase 1.0
                                       │  (dApp Store-ready)
                                       ▼
                          ┌──────────────────────────┐
                          │  Phase 1: devnet mint    │
                          │  same script will run on │
                          │  mainnet — verify shape  │
                          │  + metadata + transfers  │
                          └────────────┬─────────────┘
                                       │
                                       │  devnet looks correct
                                       ▼
                          ┌──────────────────────────┐
                          │  Phase 2: mainnet mint   │
                          │  fresh treasury wallet   │
                          │  receives full 1B        │
                          └────────────┬─────────────┘
                                       │
                       authorities revoked next
                                       ▼
                          ┌──────────────────────────┐
                          │  Phase 3: metadata live  │
                          │  Metaplex on-chain       │
                          │  image hosted            │
                          │  wallets render correctly│
                          └────────────┬─────────────┘
                                       │
                       data source flip in app
                                       ▼
                          ┌──────────────────────────┐
                          │  Phase 4: on-chain reads │
                          │  app reads balances      │
                          │  directly from RPC       │
                          │  server-virtual gone     │
                          └────────────┬─────────────┘
                                       │
                       /claim funnel goes live
                                       ▼
                          ┌──────────────────────────┐
                          │  Phase 5: distribution   │
                          │  SGT holders claim       │
                          │  airdrop allocation      │
                          │  team allocation visible │
                          └────────────┬─────────────┘
                                       │
                       (post-hackathon, optional)
                                       ▼
                          ┌──────────────────────────┐
                          │  Phase 6: liquidity      │
                          │  Raydium / pump.fun pool │
                          │  $TOPSI starts trading   │
                          └──────────────────────────┘
```

Per Bodhi's sequencing: Phases 0, 1, 2, 3, 4, 5 are tonight-through-launch. Phase 6 is post-hackathon.

---

## 11 · API Contracts

### 11.1 Auth

```
 POST /api/auth/login
   request:  { email: string, password: string }
   response: 200 { user: User, session_id: string }
   errors:   401 Invalid credentials

 GET  /api/auth/me
   request:  Authorization: Bearer <session_id>
   response: 200 User

 POST /api/auth/logout
   request:  Authorization: Bearer <session_id>
   response: 204

 POST /api/auth/sgt-verify        [NEW]
   request:  Authorization: Bearer <session_id>  (optional — issues new session if absent)
             body: { pubkey, challenge, signature }
   response: 200 { user, session_id, sgt_verified: true, sgt_mint: string }
   errors:   400 Invalid signature
             404 No SGT found at pubkey
             409 SGT already linked to a different user
```

### 11.2 Agents

```
 GET  /api/agents
   request:  Authorization: Bearer <session_id>
   response: 200 Agent[]
                  filtered by user tier:
                  admin     → all agents
                  regular   → system-tier + user's own

 POST /api/agents/{id}/chat/stream
   request:  Authorization: Bearer <session_id>
             Accept: text/event-stream
             body: { message: string, session_id?, project_id?, model?, provider? }
   response: text/event-stream of:
                  data: {"content": "..."}
                  ...
                  data: [DONE]
   errors:   402 Insufficient $TOPSI balance     [NEW behaviour]
             403 Forbidden (agent tier)
             429 Rate limited
```

### 11.3 Billing & Balance [NEW]

```
 GET  /api/billing/rate
   request:  Authorization: Bearer <session_id>
   response: 200 {
                  underlying_llm_cost_usd_per_msg: number,
                  vibe_rate_per_msg: number,
                  vibe_markup_multiple: number,
                  topsi_rate_per_msg: number,
                  topsi_markup_multiple: number,
                  agent_overrides: { [agent_id]: { topsi_rate_per_msg: number } }
                }

 GET  /api/topsi/balance
   request:  Authorization: Bearer <session_id>
   response: 200 { balance: string, source: "server" | "onchain", as_of: timestamp }

 POST /api/topsi/balance/airdrop      [admin / claim funnel]
   request:  Authorization: Bearer <session_id>
             body: { user_id, amount }
   response: 200 { user_id, balance, last_credit }
```

### 11.4 Voice

```
 POST /api/topsi/voice/tts
   request:  Authorization: Bearer <session_id>
             body: { text, voice_id, agent_id }
   response: 200 audio/mpeg (chunked)
   notes:    existing — verify shape via Explore (see Section 19)
```

---

## 12 · Configuration & Secrets

### 12.1 Mobile app (Expo env)

```
   EXPO_PUBLIC_API_BASE         = https://dashboard.powerclubglobal.com
   EXPO_PUBLIC_RPC_URL          = https://api.mainnet-beta.solana.com
                                   (or Helius URL for hackathon polish)
   EXPO_PUBLIC_TOPSI_MINT       = (filled at Phase 4)
   EXPO_PUBLIC_TOPSI_TOKEN_MINTED = false → flips to true at Phase 4
   EXPO_PUBLIC_SGT_COLLECTION   = (Solana Mobile Genesis Token collection ID)
   EXPO_PUBLIC_PUSH_PROJECT_ID  = expo push project id
```

### 12.2 Backend (pcg-cc-mcp env additions)

```
   TOPSI_GAS_ENABLED            = true                 (master switch)
   TOPSI_DEFAULT_RATE_PER_MSG   = 2                    (in $TOPSI)
   TOPSI_NORA_RATE_PER_MSG      = 4
   TOPSI_AIRDROP_DEFAULT        = 1_000_000            (granted to new SGT-verified users)
   SGT_COLLECTION_ID            = ...
   SOLANA_RPC_URL               = ...
   ALLOWED_ORIGINS              = ..., (Expo dev URL if web preview), (orcha host)
```

---

## 13 · Phased Roadmap

### 13.1 Timeline

```
   Phase 0.1 — Foundation                     ETA: 1-2 days
   ──────────────────────────────────         ─────────────
   • Expo scaffold from solana-mobile-template
   • Strip wallet demo screens
   • Theme + design system primitives
   • LoginScreen (email/password) → JWT
   • AgentSelectScreen (filtered list)
   • ChatScreen with streaming text
   • Sideload to Seeker via `adb install`

   Phase 0.2 — Voice                          ETA: +1-2 days
   ──────────────────────────────────         ─────────────
   • PushToTalk component
   • STT via @react-native-voice/voice
   • Server-side TTS via /api/topsi/voice/tts
   • Audio playback via expo-av
   • Push notifications (expo-notifications)

   Phase 0.3 — Sovereign layer                ETA: +2 days
   ──────────────────────────────────         ─────────────
   • Backend: /api/billing/rate
   • Backend: gas-metering middleware
   • Backend: /api/topsi/balance + /airdrop
   • Backend: /api/auth/sgt-verify
   • App: BalancePill in header
   • App: DebitToast on each message
   • App: SIWS + SGT path on Seeker
   • App: SeekerBadge after SGT verify

   Phase 0.4 — Polish + dApp Store prep       ETA: +1-2 days
   ──────────────────────────────────         ─────────────
   • Privacy + terms docs
   • Screenshots (Seeker form factor)
   • App icon final
   • Signed release APK
   • Rehearsal demo

   Phase 1.0 — dApp Store submission          ETA: +1 day
   ──────────────────────────────────         ─────────────
   • dapp-publishing manifest
   • dapp-store-cli submit
   • Review: 1-2 weeks (out of our hands)

   Phase 2.0 — Token mint                     ETA: 1-2 hours
   ──────────────────────────────────         ─────────────
   (after dApp Store approval, OR sooner if hackathon judging requires)
   • Devnet dry-run
   • Treasury wallet setup
   • Mainnet mint 1B
   • Metaplex metadata
   • Revoke authorities
   • Flip EXPO_PUBLIC_TOPSI_TOKEN_MINTED = true
   • Build new APK, push OTA via EAS Update

   Phase 2.1 — /claim funnel live             ETA: +1 day
   ──────────────────────────────────         ─────────────
   • Web claim page
   • SGT verify → eligibility
   • Sub-allocate from treasury
   • Waitlist for non-SGT
```

### 13.2 Sequencing rationale

- **Voice (0.2) before sovereign layer (0.3)** because voice is what makes the demo memorable; gas metering is the hidden depth that comes through on close inspection. We want the surface working first.
- **dApp Store submission (1.0) before token mint (2.0)** because the store reviews the binary and the manifest, not the on-chain token. We can ship a perfectly working app whose token doesn't exist yet — gas-metering uses server-virtual balances that look identical to on-chain in the UI.
- **/claim funnel last (2.1)** because it can't run without a real token to disburse.

### 13.3 Critical path

```
   Phase 0.1  →  Phase 0.2  →  Phase 0.3  →  Phase 0.4  →  Phase 1.0
                                       ↓
                                  Phase 2.0  →  Phase 2.1
```

- 0.2 and 0.3 are independent — we could parallelize voice and sovereign-layer work if we had two streams.
- 2.0 is gated on 1.0 by Bodhi's sequencing call (mint after Store push).
- 2.1 is gated on 2.0.

---

## 14 · dApp Store Submission Checklist

```
   ASSET                                      STATUS    NOTES
   ────────────────────────────────────────   ──────    ────────────────────────────
   Signed release APK                         TODO      EAS build profile production
   App icon 512×512                           TODO      from topsi-mark.png
   Splash screen                              TODO
   Screenshots (Seeker, 1080×2400 minimum)    TODO      LoginScreen, AgentSelect,
                                                        ChatScreen with streaming
   Banner image 1024×500                      TODO
   App name + short description               TODO      "Topsi · voice terminal..."
   Long description                           TODO      ~500 chars
   Categories                                 TODO      "AI", "Productivity"
   Permissions disclosure                     TODO      Mic, Notifications, Internet
   Privacy policy URL                         TODO      docs/PRIVACY.md → hosted
   Terms URL                                  TODO      docs/TERMS.md → hosted
   Solana wallets used                        FILLED    @solana-mobile/MWA
   Token contract addresses                   N/A       (until Phase 2.0)
   dapp-publishing manifest (JSON)            TODO
```

Submission via `dapp-store-cli`. Review window: typically 1-2 weeks but variable.

---

## 15 · Token Mint Sequence (Phase 2.0)

```
   Step 1.  Solana CLI / Metaplex setup on a clean wallet
            solana-keygen new --outfile ./treasury.json
            solana config set --url devnet
            solana airdrop 2

   Step 2.  Devnet mint
            spl-token create-token --decimals 9 → MINT_DEV
            spl-token create-account MINT_DEV → ATA_DEV
            spl-token mint MINT_DEV 1_000_000_000 ATA_DEV

   Step 3.  Devnet metadata via Metaplex
            mpl-token-metadata create-metadata
              --name "Topsi"
              --symbol "TOPSI"
              --uri https://.../metadata.json
              --mint MINT_DEV

   Step 4.  Verify on devnet
            - Phantom on Seeker (devnet) shows "Topsi" with image
            - spl-token transfer to a test wallet → shows correctly
            - Topsi app pointed at devnet RPC reads correct balance

   Step 5.  Mainnet — same script, change RPC + new keypair
            solana config set --url mainnet-beta
            solana-keygen new --outfile ./treasury-mainnet.json
            (fund with ~0.5 SOL for fees)
            spl-token create-token ... → MINT_MAIN
            spl-token create-account ...
            spl-token mint ... 1_000_000_000

   Step 6.  Mainnet metadata
            mpl-token-metadata create-metadata
              --name "Topsi"
              --symbol "TOPSI"
              --uri https://.../metadata.json
              --mint MINT_MAIN

   Step 7.  Revoke authorities
            spl-token authorize MINT_MAIN mint --disable
            spl-token authorize MINT_MAIN freeze --disable

   Step 8.  Record in project_topsi_token.md memory file:
            - mint address
            - treasury wallet
            - metadata URI
            - mint-authority-revoke tx sig
            - freeze-authority-revoke tx sig

   Step 9.  Flip app config
            EXPO_PUBLIC_TOPSI_MINT=...
            EXPO_PUBLIC_TOPSI_TOKEN_MINTED=true
            eas update --branch production --message "Topsi token live"

   Step 10. Verify on Seeker
            - open Topsi → header shows real on-chain balance
            - send a message → debit reflected on chain
```

Total Step 5-10 active time: ~1-2 hours.

---

## 16 · Hackathon Demo Storyboard

A 5-minute pitch, scene by scene.

```
   ┌──────────┐
   │ Scene 1  │  "Meet Topsi" (30s)
   ├──────────┤
   │  - On stage: Bodhi holding the Seeker
   │  - Slide: the Topsi trinity diagram
   │  - Voice over: "Topsi is the Baby Pythia. Your AI orchestrator,
   │    on a phone, paid in $TOPSI. The agent, the app, the token —
   │    one product, one identity."
   └──────────┘

   ┌──────────┐
   │ Scene 2  │  "Sign in with your Seeker" (30s)
   ├──────────┤
   │  - Open Topsi on the Seeker (mirrored to projector via scrcpy)
   │  - Tap "Sign in with your Seeker"
   │  - MWA prompt → user approves
   │  - Screen flashes "Seeker verified" badge
   │  - Voice over: "Hardware identity. The Seeker's Genesis Token
   │    proves you're real. No Sybils. No bots."
   └──────────┘

   ┌──────────┐
   │ Scene 3  │  "1.25M $TOPSI" (15s)
   ├──────────┤
   │  - Header pill animates in: "1,250,000 $TOPSI"
   │  - Voice over: "Each $TOPSI is gas. Two cents of premium gas
   │    over the underlying LLM cost. You don't pay OpenAI, you
   │    pay Topsi."
   └──────────┘

   ┌──────────┐
   │ Scene 4  │  "Voice" (90s)
   ├──────────┤
   │  - Bodhi taps Topsi tile → ChatScreen
   │  - Holds mic, speaks: "What's in my pipeline this week?"
   │  - Topsi responds in voice (ElevenLabs)
   │  - Header balance ticks down 2 → 1,249,998
   │  - Floating toast: "−2 $TOPSI"
   │  - Voice over: "Real native voice. Real on-chain meter.
   │    Watch the balance. Every word costs."
   └──────────┘

   ┌──────────┐
   │ Scene 5  │  "Different agent, different rate" (45s)
   ├──────────┤
   │  - Back to AgentSelect → tap Nora (admin only)
   │  - Bodhi: "Nora, draft me a follow-up to the HORSE deal"
   │  - Nora responds → balance drops 4
   │  - Voice over: "Nora is executive-tier. She costs more.
   │    The market for AI on Solana, priced per agent, paid
   │    in $TOPSI."
   └──────────┘

   ┌──────────┐
   │ Scene 6  │  "Sovereign Stack" (45s)
   ├──────────┤
   │  - Show on slide: Pythia node + APN + Seeker + Topsi app
   │  - Voice over: "Topsi is one face of a sovereign stack:
   │    your hardware, your nodes, your data, your AI, your token.
   │    All on Solana. All on this phone. Tomorrow it ships
   │    via the dApp Store."
   └──────────┘

   ┌──────────┐
   │ Scene 7  │  "Try it" (15s)
   ├──────────┤
   │  - QR code → /claim funnel
   │  - Voice over: "If you brought your Seeker, scan this.
   │    Topsi airdrops to you. Talk to her tonight."
   └──────────┘

   Total runtime: ~4:30, leaves 30s buffer.
```

---

## 17 · Risk Register

```
   RISK                              LIKELIHOOD   IMPACT     MITIGATION
   ─────────────────────────────────  ──────────  ─────────  ─────────────────────────
   VIBE metering not actually live    medium      high       Section 19 verification.
   in pcg-cc-mcp despite expectation               If aspirational, build it tonight
                                                  alongside Topsi gas layer

   /api/topsi/voice/tts shape         medium      medium     Verify before client wiring;
   doesn't match expected stream                              fall back to expo-speech
                                                              if needed

   SGT collection ID unknown          low         medium     Look up on Solana Mobile docs;
                                                              hardcode for v1; configurable

   solana-mobile-expo-template        low         medium     We saw it pushed Jun 2025;
   stale on RN/Expo versions                                  may need yarn upgrade pass

   MWA module breaks in Expo Go       n/a         n/a        Always use custom dev builds
                                                              (template's docs cover this)

   dApp Store review rejects on       low         high       Submit as soon as 0.4 ready;
   permissions / disclosure                                   mic+net+notifs only

   Mainnet mint with wrong decimals   medium      catastrophic  Devnet dry-run with
                                                                exact same script

   Treasury keypair compromised       low         catastrophic  Hardware wallet from
                                                                day one; multisig if time

   $TOPSI metadata image not          medium      medium     Pin to Arweave (not IPFS
   persistent → "Unknown Token"                              gateway URLs); upload twice

   Network outage during demo         medium      high       Practice with airplane mode,
                                                              cache last balance,
                                                              have offline copy of slides

   Hackathon explicitly bans          low         catastrophic  Read rules before mint;
   mainnet token launches during                                if banned, mint after
   the event                                                    submission deadline

   Provider rate limits during demo   low         high       Pre-warm sessions; have
   (Anthropic / OpenAI 429)                                   backup keys ready

   pcg-cc-mcp deploy goes down        low         high       Bodhi monitors;
   mid-demo                                                   fallback slides ready

   $TOPSI rate set too high for       low         medium     Make rate live-configurable
   demo (1.25M depleted in N msgs)                            via /api/billing/rate
                                                              before demo starts
```

---

## 18 · Open Decisions

```
   #   DECISION                                       OPTIONS
   ─── ─────────────────────────────────────────────  ────────────────────────────────
   1   GitHub repo location                            ✓ RESOLVED 2026-05-07:
                                                       Soverign-Stack/topsi-sol-dapp

   2   Package id (locked at first publish)            ✓ RESOLVED 2026-05-07:
                                                       global.powerclubglobal.topsi

   3   Server-side TTS choice                          ElevenLabs (paid, premium voice)
                                                       Chatterbox (in-house? verify)
                                                       expo-speech only (fallback)

   4   Topsi voice personality                         Pre-pick a voice ID for hackathon

   5   $TOPSI default rate per message                 2 (matches plan above)
                                                       1
                                                       configurable per env

   6   Nora rate multiplier vs Topsi                   2× (4 if Topsi=2)
                                                       3×

   7   Airdrop default for SGT-verified                1,000,000 (1M)
                                                       100,000
                                                       Variable based on SGT mint number

   8   Custom domain for hosting Topsi assets          topsi.sovereignstack.dev
                                                       topsi.powerclubglobal.com

   9   Web preview build                               Yes (Cipher Preview vibe — sales tool)
                                                       No (mobile-only is purer)

   10  RPC provider                                    Helius (premium, paid)
                                                       Triton (premium, paid)
                                                       Public mainnet-beta (rate-limited)

   11  $TOPSI mainnet trade-out path post-hackathon    LP on Raydium (controlled)
                                                       Pump.fun (viral)
                                                       Just hold, no LP (most boring)

   12  Cipher rebrand timing                           When ORCHA opens to external
                                                       clients (Stage 2)
```

---

## 19 · Verification Tasks — RESOLVED 2026-05-07

Full report: `planning/2026-05-07--analysis--topsi-verification.md`. Summary:

```
   #   QUESTION                                         RESOLUTION
   ─── ─────────────────────────────────────────────── ────────────────────────────
   V1  VIBE-per-message metering enforced today?       ✓ ENFORCED
                                                        ensure_vibe_balance() in
                                                        billing.rs:56 (pre-check),
                                                        record_llm_vibe_usage() at
                                                        billing.rs:88 (post-charge).
                                                        Currency: i64 VIBE.

   V2  Canonical streaming chat endpoint?               ✓ /api/agents/{id}/chat/stream
                                                        (agent_chat.rs:505) is the
                                                        SSE-streaming path that works
                                                        for ALL agents including Topsi.
                                                        /api/topsi/chat is JSON-only
                                                        — do NOT use it for streaming.

   V3  /api/topsi/voice/synthesize shape?               ✓ Base64 JSON blob, NOT chunked.
                                                        Play with expo-av.
                                                        WAV/MP3/FLAC/OGG. No agent_id
                                                        param — voice_profile string.

   V4  Existing SGT support?                            ✗ Not found. Fresh-build for
                                                        Topsi 0.3. Adds /api/auth/sgt-
                                                        verify to backend.

   V5  Cross-origin auth posture?                       ✓ ALLOWED_ORIGINS + localStorage
                                                        Bearer fallback unchanged.

   V6  VIBE balance endpoint?                           ✓ /api/projects/{project_id}/
                                                        vibe/balance — PROJECT-scoped,
                                                        not user-scoped. Topsi must
                                                        use default project_id.

   V7  /api/agents admin filter?                        ✓ admin → all; user → system+own.
                                                        Nora visibility "just works"
                                                        via owner_id filter.

   V8  Existing dApp-publishing artifacts?              ✗ Not found. All Store assets
                                                        ship fresh from topsi-sol-dapp.
```

**Key decisions driven by verification:**
- Use `/api/agents/{id}/chat/stream` as the canonical chat endpoint (NOT per-agent paths).
- Gas-metering layer becomes "expose existing VIBE billing logic with a 2× $TOPSI multiplier" — not a fresh build.
- Voice playback uses single base64 decode + expo-av, not chunked stream.
- Add `/api/auth/sgt-verify` (new) and `/api/billing/rate` (new) endpoints to pcg-cc-mcp during 0.3.
- Default project_id must be returned by `/api/auth/me` (verify this) so client knows which project's VIBE balance to read.

---

## 20 · Tonight's Concrete First Move

When you approve this plan:

```
   1.  Run verification Explore agent against pcg-cc-mcp
       → resolves Section 19 V1-V8 in 30 minutes
       → updates this plan with concrete file:line citations

   2.  Bootstrap the Topsi project
       cd /Users/sirakstudios/topos/pcg/github
       yarn create expo-app topsi-sol-dapp --template @solana-mobile/solana-mobile-expo-template
       cd topsi-sol-dapp
       git init && gh repo create Soverign-Stack/topsi-sol-dapp --private --source . --push
       cp -r /tmp/sm-skills/{mwa,genesis-token} ~/.claude/skills/
       # set package id global.powerclubglobal.topsi in app.config.ts

   3.  Strip the wallet demo screens
       (template ships with wallet UI — repurpose nav, gut content)

   4.  Wire LoginScreen → /api/auth/login
       end-state for tonight: tap login on the Seeker, see your name and
       the Topsi/Nora tiles populated from real /api/agents data

   5.  Stop. Sleep. Sell the hackathon judges tomorrow.
```

That's the plan.

---

## Appendix A — Naming changelog

```
   2026-05-07  Initial brief proposed "Cipher" — voice terminal for ORCHA
   2026-05-07  Bodhi pivoted to "Topsi" for hackathon submission;
               Cipher pocketed for Stage-2 public product.
   2026-05-07  Decision: app + agent + token unified under Topsi name
```

## Appendix B — File locations

```
   This plan:           pcg-cc-mcp/planning/2026-05-07--plan--topsi-build.md
   Memory:              ~/.claude/projects/.../memory/project_topsi.md
                        ~/.claude/projects/.../memory/project_topsi_token.md
   Verification report: pcg-cc-mcp/planning/2026-05-07--analysis--topsi-verification.md
                        (created after Section 19 Explore pass)
   Future repo:         /Users/sirakstudios/topos/pcg/github/topsi-sol-dapp/
                        Soverign-Stack/topsi-sol-dapp (locked 2026-05-07)
                        Package id: global.powerclubglobal.topsi (locked 2026-05-07)
```
