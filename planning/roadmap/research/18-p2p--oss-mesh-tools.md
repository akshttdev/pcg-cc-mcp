# Research: Open Source P2P, Mesh Networking, and Decentralized Compute Tools

**Date:** 2026-03-19
**Type:** research
**Status:** complete
**Scope:** License-verified OSS tools for ORCHA AI orchestration platform (Rust/Axum + React)

---

## Executive Summary

This report evaluates open-source tools across P2P networking, messaging, desktop packaging, container orchestration, cryptography, and network discovery for integration into ORCHA's multi-stage roadmap. All tools are verified as permissively licensed (MIT, Apache 2.0, BSD, ISC). The report also covers MCP/A2A protocol compatibility considerations.

**Key recommendations:**
- **Stage 0-1 (Dogfood/Pilots):** NATS + bollard + rustls + mdns-sd
- **Stage 2-3 (SaaS/PMF):** Add iroh for P2P, Tauri for desktop, WebRTC (str0m) for browser-agent
- **Stage 4-5 (Scale/Sovereignty):** Full mesh with libp2p, WireGuard tunnels, decentralized identity

---

## Protocol Landscape: MCP, A2A, and Agent Communication

Before evaluating individual tools, it is important to understand the protocol context.

### MCP (Model Context Protocol)
- **License:** MIT (specification and reference implementations)
- **Status:** Active; 2026 roadmap targets stateless Streamable HTTP transport, session migration, and Server Cards for discovery (spec release tentatively June 2026)
- **Relevance:** MCP is the standard for LLM-to-tool communication. ORCHA already uses MCP servers. The 2026 roadmap's move toward remote, stateless MCP servers aligns with ORCHA's distributed compute goals.

### A2A (Agent-to-Agent Protocol)
- **License:** Apache 2.0 (Linux Foundation)
- **Status:** Active; v0.3 released July 2025 with gRPC support. ACP (IBM) merged into A2A.
- **Relevance:** A2A covers agent-to-agent communication (JSON-RPC 2.0 over HTTP/SSE/push). Complementary to MCP (which is LLM-to-tool). ORCHA should plan for A2A adoption at Stage 2+ for multi-agent orchestration across organizational boundaries.

### Compatibility Note
MCP and A2A are complementary, not competing. MCP handles tool integration; A2A handles agent interoperability. P2P transport layers (iroh, libp2p) can carry either protocol. The tools below are evaluated as transport/infrastructure layers that sit beneath these application protocols.

---

## 1. rust-libp2p

**Category:** P2P networking library
**License:** MIT / Apache 2.0 (dual)
**Repo:** https://github.com/libp2p/rust-libp2p
**Latest:** v0.56.0 (actively maintained, last commit ~March 2026, 142+ contributors)

### Pros
- Most mature and battle-tested P2P stack in Rust (used by IPFS, Filecoin, Polkadot)
- Modular: pick transports (TCP, QUIC, WebSocket, WebRTC), protocols (Kademlia DHT, GossipSub, mDNS), and security (Noise, TLS)
- Built-in peer discovery, NAT traversal (AutoNAT, hole punching ~70% success), relay protocol
- Active maintainer team with regular community calls
- libp2p-webrtc transport enables browser connectivity
- Large ecosystem of examples and documentation

### Cons
- Steep learning curve; extensive configuration surface
- NAT hole-punching success rate (~70%) lower than iroh
- Heavy dependency tree for full feature set
- Maximally decentralized design adds complexity when some centralization is acceptable
- Compile times increase significantly with many protocol features enabled

### Roadmap Fit
- **Stage 4-5 (Series A / Sovereignty):** Full decentralized mesh networking where no central relay is acceptable
- Could be introduced earlier (Stage 3) if ORCHA needs DHT-based peer discovery at scale

### Budget/Timeline Impact
- **Integration effort:** 4-6 weeks for basic P2P transport, 8-12 weeks for production mesh with DHT + relay
- **Ongoing cost:** Moderate maintenance burden due to frequent releases and API evolution
- **Infrastructure:** Requires relay/bootstrap nodes for NAT traversal

### Rationale vs. Alternatives
Use libp2p when you need maximum decentralization and protocol flexibility. For most ORCHA use cases through Stage 3, iroh is simpler and more effective. Consider libp2p for Stage 4-5 when sovereignty requirements demand zero central dependencies.

---

## 2. NATS

**Category:** Cloud-native messaging system
**License:** Apache 2.0
**Repo:** https://github.com/nats-io/nats.rs (Rust client)
**Server:** https://github.com/nats-io/nats-server (Go, Apache 2.0)
**Latest Rust client:** async-nats (actively maintained, supports NATS 2.12 server features)

### Pros
- Extremely lightweight, high-performance pub/sub and request/reply
- JetStream provides built-in persistence, exactly-once delivery, stream replay
- Async-native Rust client (async-nats) with Tokio integration — perfect fit for Axum
- Supports clustering, leaf nodes, and superclusters for multi-region deployment
- Built-in authentication (NKeys, JWT), subject-based authorization
- Embeddable server option for single-binary deployments
- Request/reply pattern maps naturally to MCP tool calls
- Subject-based routing enables per-agent, per-org, per-task message isolation

### Cons
- Server is written in Go (not Rust), adding a deployment dependency
- JetStream adds operational complexity vs. core NATS
- Not truly peer-to-peer — requires NATS server infrastructure
- Leaf node topology requires planning for multi-site deployments
- async-nats API still on 0.x versioning

### Roadmap Fit
- **Stage 0-1 (Dogfood / Pilots):** Core message bus for agent-to-orchestrator communication
- **Stage 2-3 (SaaS / PMF):** JetStream for durable task queues, event sourcing, audit trails
- **Stage 4+:** Supercluster for multi-region, leaf nodes for edge/on-prem hybrid

### Budget/Timeline Impact
- **Integration effort:** 1-2 weeks for basic pub/sub, 3-4 weeks for JetStream with task queuing
- **Ongoing cost:** Low — NATS server is lightweight (~50MB RAM for moderate workloads)
- **Infrastructure:** Single NATS server for Stage 0-1, 3-node cluster for Stage 2+

### Rationale vs. Alternatives
NATS is the strongest choice for ORCHA's messaging backbone. It provides the reliability and patterns needed for production agent orchestration without the complexity of Kafka or the limitations of Redis pub/sub. The subject-based routing model maps cleanly to ORCHA's organization/project/task hierarchy.

**Recommendation: Adopt immediately (Stage 0).**

---

## 3. Tauri

**Category:** Desktop application framework
**License:** MIT / Apache 2.0 (dual)
**Repo:** https://github.com/tauri-apps/tauri
**Latest:** Tauri v2 (stable since October 2024, includes iOS/Android support)

### Pros
- Build desktop + mobile apps from existing React frontend with Rust backend
- Much smaller binary size than Electron (~600KB vs ~150MB)
- Native OS WebView (no bundled Chromium) — reduces attack surface and memory usage
- Tauri v2 adds mobile (iOS/Android), plugin system, inter-process communication improvements
- Rust backend can embed ORCHA agent logic, NATS client, P2P networking directly
- Strong security model with fine-grained API permissions
- Active community and corporate backing (CrabNebula)

### Cons
- WebView rendering differences across platforms (especially Linux/Windows edge cases)
- Some web APIs not available in WebView contexts
- Plugin ecosystem less mature than Electron
- Mobile support (v2) is newer and less battle-tested than desktop
- Requires platform-specific build toolchains for each target

### Roadmap Fit
- **Stage 2-3 (SaaS / PMF):** Desktop agent that runs locally, connects to ORCHA cloud
- **Stage 4-5 (Sovereignty):** On-premise agent node with local compute, P2P mesh participation

### Budget/Timeline Impact
- **Integration effort:** 2-3 weeks to wrap existing React frontend in Tauri shell
- **Desktop agent features:** 4-6 weeks for system tray, background tasks, local model serving
- **Ongoing cost:** Low — leverages existing frontend codebase

### Rationale vs. Alternatives
Tauri is the clear choice over Electron for a Rust-native platform. The existing React frontend ports directly, and the Rust backend can embed ORCHA's agent runtime, NATS client, and P2P networking in a single binary. Defer to Stage 2 since web-first is sufficient for dogfood/pilots.

---

## 4. iroh

**Category:** P2P connectivity library
**License:** MIT / Apache 2.0 (dual)
**Repo:** https://github.com/n0-computer/iroh
**Latest:** Actively maintained (February 2026 releases on lib.rs)

### Pros
- Dramatically simpler API than libp2p — dial by public key, iroh handles the rest
- Built on QUIC (via quinn) — encrypted, multiplexed, low-latency by default
- Superior NAT traversal: Tailscale-inspired hole punching with relay fallback (higher success than libp2p's ~70%)
- Relay servers are lightweight and open-ecosystem (anyone can run one)
- Pure Rust, async-native, Tokio-compatible
- `libp2p-iroh` bridge crate exists for gradual migration between the two
- Focused scope reduces integration complexity and dependency weight

### Cons
- Assumes QUIC only — no TCP, WebSocket, or WebRTC transport options
- Some centralization required (relay servers for fallback)
- Smaller ecosystem and community than libp2p
- No built-in DHT for fully decentralized peer discovery
- Younger project with less production mileage than libp2p
- No browser support (QUIC not available in browser contexts)

### Roadmap Fit
- **Stage 2-3 (SaaS / PMF):** P2P connectivity between ORCHA desktop agents and cloud
- **Stage 3-4 (PMF / Series A):** Direct agent-to-agent communication, bypassing central relay when possible

### Budget/Timeline Impact
- **Integration effort:** 1-2 weeks for basic P2P connectivity, 3-4 weeks for production with relay infrastructure
- **Infrastructure:** Need to run at least one relay server (lightweight)
- **Ongoing cost:** Low — simple API means fewer bugs, less maintenance

### Rationale vs. Alternatives
iroh is the recommended P2P library for ORCHA through Stage 3-4. It provides 80% of libp2p's value at 20% of the complexity. The Tailscale-proven NAT traversal approach is more reliable in real-world networks. The `libp2p-iroh` bridge crate provides an upgrade path to full libp2p if needed at Stage 5.

**Recommendation: Adopt at Stage 2, evaluate for Stage 1 pilot use cases.**

---

## 5. bollard

**Category:** Docker API client for Rust
**License:** Apache 2.0
**Repo:** https://github.com/fussybeaver/bollard
**Latest:** v0.19.2 (January 2026 build, actively maintained)

### Pros
- Full async Docker API via Hyper + Tokio — native fit for Axum stack
- Supports container lifecycle (create, start, stop, exec), image management, networks, volumes
- Windows Named Pipe support + optional Rustls for HTTPS
- Generated from OpenAPI/protobuf specs — stays current with Docker API
- Buildkit integration for advanced image builds
- Enables sandboxed agent execution in containers

### Cons
- Docker dependency means containers must be available on target machines
- No Podman/containerd support out of the box (Docker socket only)
- Some advanced Docker features may lag behind the latest Docker releases
- Container overhead for lightweight agent tasks

### Roadmap Fit
- **Stage 0-1 (Dogfood / Pilots):** Sandboxed execution of agent tasks (code execution, tool use)
- **Stage 2-3 (SaaS / PMF):** Multi-tenant workload isolation, GPU container scheduling
- **Stage 4+:** Kubernetes orchestration likely supersedes direct Docker API usage

### Budget/Timeline Impact
- **Integration effort:** 1-2 weeks for basic container lifecycle management
- **Sandboxed execution:** 3-4 weeks for full agent sandbox with networking, resource limits, output capture
- **Infrastructure:** Docker daemon required on compute nodes

### Rationale vs. Alternatives
bollard is the standard Rust Docker client with no serious competitors. Essential for ORCHA's sandboxed agent execution model. The async Tokio integration makes it trivial to use alongside Axum.

**Recommendation: Adopt immediately (Stage 0).**

---

## 6. ZeroMQ / nanomsg (NNG)

### ZeroMQ (zmq.rs — native Rust)

**License:** MIT (zmq.rs native Rust implementation)
**Repo:** https://github.com/zeromq/zmq.rs
**Note:** The C libzmq library is MPL-2.0 (weak copyleft). The native Rust `zmq.rs` is MIT.

#### Pros
- Battle-tested messaging patterns (PUB/SUB, REQ/REP, PUSH/PULL, ROUTER/DEALER)
- zmq.rs is a pure Rust async implementation — no C dependency
- Broker-less architecture reduces infrastructure
- Well-understood patterns for distributed systems

#### Cons
- zmq.rs is incomplete — not all ZeroMQ patterns fully implemented
- Smaller community around the Rust native port
- No built-in persistence or exactly-once delivery (unlike NATS JetStream)
- No built-in auth/encryption (must layer on top)

### NNG (nanomsg-next-generation)

**License:** MIT
**Repo:** https://github.com/nanomsg/nng (C library), Rust bindings via `nng` crate
**Rust crate:** https://crates.io/crates/nng

#### Pros
- MIT licensed, lightweight, broker-less
- Simpler API than ZeroMQ
- Supports survey/respondent pattern (useful for agent capability discovery)
- FFI bindings available; `anng` crate provides async Rust bindings

#### Cons
- Rust bindings are FFI wrappers around C library (not native Rust)
- Smaller ecosystem than ZeroMQ or NATS
- Last significant NNG C library update unclear — may have stalled
- No built-in persistence

### Roadmap Fit
- **Not recommended for primary messaging** — NATS covers all use cases better with persistence, auth, and clustering
- **Potential niche use:** NNG survey pattern for local agent discovery in Stage 3+

### Budget/Timeline Impact
- Low integration cost but duplicates NATS functionality
- Not worth the additional dependency unless a specific pattern is needed

### Rationale
NATS supersedes both ZeroMQ and NNG for ORCHA's use cases. NATS provides the same messaging patterns plus persistence, clustering, authentication, and a mature async Rust client. The only scenario for ZeroMQ/NNG would be extreme low-latency IPC between co-located processes, which is a niche case ORCHA can address later if needed.

---

## 7. Cryptography Libraries

### rustls

**License:** MIT / Apache 2.0 / ISC (triple-licensed)
**Repo:** https://github.com/rustls/rustls
**Latest:** Actively maintained, production-grade

#### Pros
- Pure Rust TLS implementation — no OpenSSL dependency
- Memory-safe by default, no C code in the TLS stack
- Since v0.22: pluggable crypto backends (aws-lc-rs recommended, ring as fallback)
- Already widely used in the Rust ecosystem (reqwest, hyper, etc.)
- Post-quantum algorithm support via aws-lc-rs backend
- Excellent fit for ORCHA's Axum stack (likely already a transitive dependency)

#### Cons
- Slightly lower throughput than OpenSSL for bulk data transfer
- aws-lc-rs backend has more complex build requirements than ring

#### Recommendation
**Already in use (transitive via Axum/hyper). Ensure aws-lc-rs backend for post-quantum readiness at Stage 3+.**

### ring

**License:** ISC (new code) / Apache 2.0 (BoringSSL-derived code) — both permissive
**Repo:** https://github.com/briansmith/ring
**Latest:** v0.17.14, actively maintained

#### Pros
- High-performance, well-audited cryptographic primitives
- Hybrid Rust/C/assembly for optimal performance
- Used by rustls as default crypto backend
- Small, focused API surface

#### Cons
- Contains C and assembly code (not pure Rust)
- Build can be tricky on some platforms (cross-compilation)
- Single maintainer (Brian Smith) — bus factor concern
- No post-quantum algorithm support (unlike aws-lc-rs)

#### Recommendation
**Use via rustls. Do not depend on ring directly — prefer rustls's abstraction layer which allows switching to aws-lc-rs.**

### libsodium (via sodiumoxide / libsodium-sys)

**License:** Apache 2.0 / MIT (Rust bindings); ISC (libsodium C library)
**Repo:** https://github.com/sodiumoxide/sodiumoxide (Rust), https://github.com/jedisct1/libsodium (C)

#### Pros
- Excellent API design — hard to misuse
- Complete suite: encryption, signatures, key exchange, hashing, password hashing
- Well-audited C library with broad platform support
- Good for application-layer crypto (message signing, key management)

#### Cons
- FFI wrapper around C library (not pure Rust)
- `sodiumoxide` crate maintenance has been inconsistent
- Overlaps with ring/rustls for TLS use cases

#### Recommendation
**Consider for Stage 3+ application-layer crypto (agent identity, message signing). For TLS, rustls is sufficient. Evaluate pure-Rust alternatives (e.g., `ed25519-dalek`, `x25519-dalek`) before committing to libsodium FFI.**

### Crypto Library Summary

| Library | Use Case | Stage | Priority |
|---------|----------|-------|----------|
| rustls | TLS for all network communication | 0 (already present) | Required |
| ring | Low-level crypto (via rustls) | 0 (already present) | Transitive |
| aws-lc-rs | Post-quantum crypto backend for rustls | 3+ | High |
| libsodium / dalek | Agent identity, message signing | 3+ | Medium |

---

## 8. WebRTC for Browser-to-Agent Communication

### str0m (Recommended)

**License:** MIT / Apache 2.0 (dual)
**Repo:** https://github.com/algesten/str0m
**Latest:** v0.16.2 (March 2026, actively maintained)

#### Pros
- Sans-I/O design: no threads, no async runtime dependency, no locks — composable with any async runtime
- Clean Rust ownership model (no Arc, Mutex, Rc)
- Lightweight and focused
- rust-libp2p is migrating from webrtc-rs to str0m (strong ecosystem signal)
- Actively maintained with frequent releases

#### Cons
- Lower-level API — requires building signaling and session management on top
- Smaller community than webrtc-rs
- Data channels only (no media — fine for ORCHA's use case)

### webrtc-rs (Alternative)

**License:** MIT / Apache 2.0 (dual)
**Repo:** https://github.com/webrtc-rs/webrtc
**Latest:** v0.17.x (bug fixes), v0.20.0 (active development with Sans-I/O rewrite)

#### Pros
- Full WebRTC implementation (media + data channels)
- Pion-inspired API familiar to Go WebRTC developers
- Actively being rewritten with Sans-I/O architecture (converging with str0m's approach)

#### Cons
- v0.17.x is Tokio-locked
- v0.20.0 (new architecture) is not yet stable
- Heavier dependency tree than str0m

### Roadmap Fit
- **Stage 2-3 (SaaS / PMF):** Browser-to-agent data channels for real-time collaboration
- **Stage 3-4:** P2P browser-to-browser agent communication without server relay

### Budget/Timeline Impact
- **str0m integration:** 3-5 weeks (signaling server + data channel management)
- **Infrastructure:** Signaling server (can be built on NATS), STUN/TURN servers

### Recommendation
**Use str0m for data-channel WebRTC.** Its Sans-I/O design aligns with ORCHA's Rust architecture, and the libp2p migration signal validates the choice. webrtc-rs v0.20.0 may be worth re-evaluating at Stage 3 if media channels become needed.

---

## 9. mDNS / DNS-SD for Local Network Discovery

### mdns-sd

**License:** MIT / Apache 2.0 (dual)
**Repo:** https://github.com/keepsimple1/mdns-sd
**Latest:** v0.17.2 (actively maintained, tested with Avahi/Bonjour)

#### Pros
- Pure Rust implementation of mDNS (RFC 6762) and DNS-SD (RFC 6763)
- Supports both querier (client) and responder (server) modes
- Works with sync and async code via flume channels
- Tested cross-platform: Linux (Avahi), macOS (dns-sd), iOS (Bonjour)
- Small dependency footprint

#### Cons
- Spawns a dedicated mDNS daemon thread (not purely async)
- Beta-quality — focused on common use cases, edge cases may have issues
- LAN-only discovery (by design)

### Alternatives
- **simple-mdns:** Pure Rust, sync+async, but smaller community
- **libmdns:** Responder-only (no querier), used by librespot

### Roadmap Fit
- **Stage 1-2 (Pilots / SaaS):** Auto-discover ORCHA agents on local network
- **Stage 4-5 (Sovereignty):** On-premise mesh agent discovery without cloud dependency

### Budget/Timeline Impact
- **Integration effort:** 1 week for service advertisement and discovery
- **Infrastructure:** None (zero-config by design)

### Recommendation
**Adopt mdns-sd at Stage 1 for LAN agent discovery.** Extremely low cost, high value for on-premise and developer experience. Pairs naturally with iroh for P2P connectivity after discovery.

---

## 10. WireGuard for Encrypted Tunnels

### boringtun

**License:** BSD 3-Clause
**Repo:** https://github.com/cloudflare/boringtun
**Maintained by:** Cloudflare (deployed on millions of devices)

#### Pros
- Userspace WireGuard implementation in pure Rust
- Production-proven at Cloudflare scale (millions of iOS/Android devices)
- No kernel module required — runs in userspace
- C ABI bindings for cross-language integration
- BSD 3-Clause is fully permissive

#### Cons
- Library-level API (provides WireGuard protocol, not networking stack)
- Requires building tunnel management, key exchange, and peer configuration on top
- Cloudflare has deprioritized open-source updates in favor of internal fork
- firezone/boringtun fork may be more actively maintained

### Roadmap Fit
- **Stage 4-5 (Series A / Sovereignty):** Encrypted site-to-site tunnels between ORCHA nodes
- **Stage 5:** Zero-trust networking between sovereign compute nodes

### Budget/Timeline Impact
- **Integration effort:** 6-10 weeks for tunnel management, key distribution, peer configuration
- **Infrastructure:** Key management service, peer registry

### Recommendation
**Defer to Stage 4-5.** WireGuard tunnels are essential for sovereignty but overkill for earlier stages. iroh's QUIC transport provides encryption by default for P2P communication. When WireGuard is needed, evaluate the firezone/boringtun fork vs. the Cloudflare original.

**Alternative consideration:** At Stage 4-5, evaluate if iroh's relay + QUIC encryption is sufficient, potentially avoiding WireGuard entirely.

---

## Consolidated Roadmap Integration

### Stage 0 — Dogfood (Q2 2026)

| Tool | Purpose | Effort |
|------|---------|--------|
| **NATS** (async-nats) | Agent message bus, task queuing | 2-3 weeks |
| **bollard** | Sandboxed agent execution in Docker | 2-3 weeks |
| **rustls** | TLS (already present via Axum) | 0 weeks |

**Total new integration effort: ~4-6 weeks**

### Stage 1 — Pilots (Q3-Q4 2026)

| Tool | Purpose | Effort |
|------|---------|--------|
| **mdns-sd** | LAN agent discovery for on-prem pilots | 1 week |
| **NATS JetStream** | Durable task queues, event audit trail | 2-3 weeks |

**Total new integration effort: ~3-4 weeks**

### Stage 2 — SaaS (2027)

| Tool | Purpose | Effort |
|------|---------|--------|
| **iroh** | P2P agent connectivity, NAT traversal | 3-4 weeks |
| **Tauri** | Desktop agent application | 3-4 weeks |
| **str0m** (WebRTC) | Browser-to-agent data channels | 4-5 weeks |

**Total new integration effort: ~10-13 weeks**

### Stage 3 — PMF (2028)

| Tool | Purpose | Effort |
|------|---------|--------|
| **A2A protocol** | Cross-org agent interoperability | 4-6 weeks |
| **aws-lc-rs** | Post-quantum crypto for rustls | 1-2 weeks |
| **ed25519-dalek** | Agent identity and message signing | 2-3 weeks |
| **NATS supercluster** | Multi-region messaging | 2-3 weeks |

**Total new integration effort: ~9-14 weeks**

### Stage 4 — Series A (2029)

| Tool | Purpose | Effort |
|------|---------|--------|
| **libp2p** (or iroh at scale) | Full decentralized mesh | 8-12 weeks |
| **boringtun** (WireGuard) | Site-to-site encrypted tunnels | 6-10 weeks |

**Total new integration effort: ~14-22 weeks**

### Stage 5 — Sovereignty (2030-2031)

| Tool | Purpose | Effort |
|------|---------|--------|
| **libp2p DHT** | Fully decentralized peer discovery | 4-6 weeks |
| Decentralized identity layer | Agent identity without central authority | 8-12 weeks |

**Total new integration effort: ~12-18 weeks**

---

## License Verification Summary

| Tool | License | Permissive? | Verified |
|------|---------|-------------|----------|
| rust-libp2p | MIT / Apache 2.0 | Yes | Yes |
| NATS (server + client) | Apache 2.0 | Yes | Yes |
| Tauri | MIT / Apache 2.0 | Yes | Yes |
| iroh | MIT / Apache 2.0 | Yes | Yes |
| bollard | Apache 2.0 | Yes | Yes |
| zmq.rs (native Rust) | MIT | Yes | Yes |
| libzmq (C library) | MPL-2.0 | Weak copyleft — AVOID | Yes |
| NNG | MIT | Yes | Yes |
| rustls | MIT / Apache 2.0 / ISC | Yes | Yes |
| ring | ISC / Apache 2.0 | Yes | Yes |
| libsodium-sys | Apache 2.0 / MIT | Yes | Yes |
| str0m (WebRTC) | MIT / Apache 2.0 | Yes | Yes |
| webrtc-rs | MIT / Apache 2.0 | Yes | Yes |
| mdns-sd | MIT / Apache 2.0 | Yes | Yes |
| boringtun (WireGuard) | BSD 3-Clause | Yes | Yes |
| A2A protocol | Apache 2.0 | Yes | Yes |

**No copyleft (GPL/AGPL), SSPL, or BSL licenses in any recommended tool.**

---

## Decision Matrix

| Criteria | NATS | iroh | libp2p | Tauri | bollard |
|----------|------|------|--------|-------|---------|
| Time to integrate | 2-3w | 3-4w | 8-12w | 3-4w | 2-3w |
| Axum/Tokio fit | Excellent | Excellent | Good | Good | Excellent |
| MCP compatibility | High (req/rep maps to tool calls) | Medium (transport layer) | Medium (transport layer) | N/A | N/A |
| Production readiness | High | Medium-High | High | High | High |
| Maintenance activity | High | High | High | High | Medium-High |
| Complexity | Low | Low | High | Medium | Low |

---

## Sources

- [rust-libp2p GitHub](https://github.com/libp2p/rust-libp2p)
- [NATS Rust Client](https://github.com/nats-io/nats.rs)
- [iroh by n0-computer](https://github.com/n0-computer/iroh)
- [Comparing Iroh & Libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [Tauri v2](https://v2.tauri.app/)
- [bollard Docker API](https://github.com/fussybeaver/bollard)
- [zmq.rs Native Rust ZeroMQ](https://github.com/zeromq/zmq.rs)
- [NNG - nanomsg-NG](https://nng.nanomsg.org/)
- [ZeroMQ License](https://zeromq.org/license/)
- [rustls](https://github.com/rustls/rustls)
- [ring](https://github.com/briansmith/ring)
- [str0m WebRTC](https://github.com/algesten/str0m)
- [webrtc-rs](https://github.com/webrtc-rs/webrtc)
- [mdns-sd](https://github.com/keepsimple1/mdns-sd)
- [boringtun WireGuard](https://github.com/cloudflare/boringtun)
- [2026 MCP Roadmap](http://blog.modelcontextprotocol.io/posts/2026-mcp-roadmap/)
- [A2A Protocol](https://github.com/a2aproject/A2A)
- [A2A Protocol Specification](https://a2a-protocol.org/latest/specification/)
- [MCP Transport Future](https://blog.modelcontextprotocol.io/posts/2025-12-19-mcp-transport-future/)
- [P2P Networking: WebRTC vs libp2p vs Iroh](https://medium.com/@ark-builders/the-deceptive-complexity-of-p2p-connections-and-the-solution-we-found-d2b5cbeddbaf)
