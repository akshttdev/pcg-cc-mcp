# Alpha Protocol Network (APN)

A decentralized mesh networking system for distributed compute sharing, resource pooling, and economic settlement via VIBE tokens.

## Overview

APN enables peer-to-peer communication across devices (desktops, servers, IoT), allowing nodes to discover each other, share compute resources, distribute tasks, and earn VIBE tokens for contributions. It is structured in three layers:

```
┌─────────────────────────────────────────┐
│  Layer 2: Application (Dashboard)       │
│  MeshPanel UI, NetworkSettings page     │
│  Task distribution, resource monitoring │
└────────────────┬────────────────────────┘
                 │ HTTP / SSE
┌────────────────▼────────────────────────┐
│  Layer 1: Integration (Bridge)          │
│  apn_bridge_server.py (FastAPI :8000)   │
│  apn-client Rust crate                  │
│  apn_data_service (NATS consumer)       │
└────────────────┬────────────────────────┘
                 │ NATS / libp2p
┌────────────────▼────────────────────────┐
│  Layer 0: Core Network (apn_node)       │
│  libp2p mesh (gossipsub + kad + mdns)   │
│  NATS relay client                      │
│  Crypto, identity, resource collection  │
└─────────────────────────────────────────┘
```

## Technology Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| P2P Transport | libp2p 0.54 | Mesh networking substrate |
| Encrypted Transport | Noise Protocol (libp2p) | Point-to-point encrypted connections |
| Symmetric Encryption | ChaCha20-Poly1305 (AEAD) | Message payload encryption |
| Key Exchange | X25519 ECDH | Ephemeral session key derivation |
| Peer Discovery | mDNS (local), Kademlia DHT (wide-area) | Finding peers |
| Message Broadcasting | Gossipsub | Pub/sub across mesh |
| Identity | Ed25519 keypairs + BIP39 mnemonics | Cryptographic node identity |
| NAT Traversal | NATS (async-nats 0.37) | Relay at nonlocal.info:4222 |
| Hashing | BLAKE3, SHA256 | Fast hashing and key derivation |
| Runtime | Tokio 1.45 | Async Rust execution |
| Bridge | Python FastAPI | HTTP bridge for dashboard |
| Blockchain | Aptos (testnet) | VIBE token settlement |

## Key Modules

### Core Library (`crates/alpha-protocol-core/`)

| File | Role |
|------|------|
| `src/node.rs` | AlphaNode — high-level API combining identity, mesh, and relay |
| `src/mesh.rs` | MeshNode — libp2p swarm with gossipsub, kad, mdns, identify, ping |
| `src/relay.rs` | NatsRelay — NATS fallback for NAT traversal |
| `src/identity.rs` | Ed25519 + BIP39 identity generation, persistence to `~/.apn/node_identity.json` |
| `src/crypto.rs` | X25519 key exchange + ChaCha20-Poly1305 AEAD encryption |
| `src/wire.rs` | Wire protocol — 32-byte header, message types (JSON serialization) |
| `src/resources.rs` | System resource detection (CPU, RAM, disk, GPU) |
| `src/economics.rs` | VIBE reward calculation with weighted scoring |
| `src/reward_tracker.rs` | Listens to NATS heartbeats, calculates and records rewards |
| `src/reward_distributor.rs` | Batches and distributes VIBE to wallets |
| `src/mining.rs` | Mining pool integration framework (stub) |

### Binaries (`crates/alpha-protocol-core/src/bin/`)

| Binary | Purpose |
|--------|---------|
| `apn_node` | CLI node — runs libp2p mesh + NATS relay + heartbeat loop |
| `apn_api_server` | REST API for APN Core (port 8000) |
| `apn_send` | Test message CLI |
| `apn_reward_tracker` | Track node contributions |
| `apn_reward_distributor` | Batch VIBE distribution |

### Integration Layer

| File | Role |
|------|------|
| `crates/apn-client/` | Rust HTTP client for APN Core API |
| `apn_bridge_server.py` | FastAPI bridge — translates mesh protocol to REST JSON |
| `crates/server/src/apn_data_service.rs` | Bidirectional data sync over NATS (provider/consumer) |
| `crates/server/src/routes/mesh.rs` | Dashboard mesh API (`GET /api/mesh/stats`) |
| `crates/server/src/routes/apn_data.rs` | Data sync endpoints |

### Frontend

| File | Role |
|------|------|
| `frontend/src/components/mesh/MeshPanel.tsx` | Main mesh UI |
| `frontend/src/components/mesh/TransactionLog.tsx` | VIBE transaction history |
| `frontend/src/pages/settings/NetworkSettings.tsx` | APN node control |
| `frontend/src/pages/mesh.tsx` | Mesh network page |

### Process Management

`crates/utils/src/external_services.rs` handles the full lifecycle:
- `kill_existing_apn_nodes()` — prevents zombie processes via PID files and `pgrep`
- `initialize_external_services()` — spawns apn_node and bridge on server startup
- PID files at `/tmp/apn_node.pid`, `/tmp/apn_bridge.pid`
- Logs at `/tmp/apn_node.log`, `/tmp/apn_bridge.log`

## Gossipsub Topics

| Topic | Purpose |
|-------|---------|
| `apn.peers` | Peer announcements |
| `apn.tasks` | Task distribution and execution |
| `apn.heartbeat` | Status broadcasts (every 30s) |
| `apn.topology` | Network topology updates |
| `apn.mining` | Mining pool shares |
| `apn.pythia` | Federated learning gradients |

## Wire Protocol Message Types

`HANDSHAKE (0x01)`, `DATA (0x02)`, `ACK (0x03)`, `PEER_ANNOUNCE (0x04)`, `TOPOLOGY_UPDATE (0x05)`, `TASK_BROADCAST (0x06)`, `PYTHIA_GRADIENT (0x07)`, `HEARTBEAT (0x08)`, `TASK_CLAIM (0x09)`, `TASK_RESULT (0x0A)`, `PAYMENT_CONFIRM (0x0B)`

## Economics (VIBE Tokens)

- **Heartbeat uptime**: 0.1 VIBE base per 30s heartbeat
- **Resource multipliers**: GPU = 2x, >16 CPU cores = 1.5x, >32GB RAM = 1.3x
- **Task execution**: Per-task rewards negotiated in TaskBid
- **Marketplace**: Listings, subscriptions, and gateway request routing (DB schema in `20260315200000_apn_marketplace.sql`)
- **Payout**: Batched every 5 minutes via `apn_reward_distributor` to Aptos testnet

## Phase 1 Status (as of 2026-03-11)

### Working

- libp2p mesh (Swarm, Gossipsub, mDNS, Kademlia DHT, Ping)
- NATS relay (publish, subscribe, peer announcements)
- Identity (Ed25519 keypair generation, BIP39 mnemonics, persistence)
- Encryption (X25519 ECDH + ChaCha20-Poly1305 AEAD)
- Wire protocol (JSON serialization — functional, not yet optimized to binary)
- Resource detection (CPU, RAM, disk, GPU via sysinfo and CLI tools)
- Heartbeat (30s timer broadcasting over NATS)
- Reward tracker (NATS heartbeat listener, VIBE calculation, DB writes)
- Economics model (contribution scoring, reputation, stake pool)
- Process management (spawn, PID tracking, zombie cleanup)
- Data sync service (bidirectional NATS sync with access control)
- Dashboard routes (APN Core API calls with log-parsing fallback)
- Bridge server (FastAPI + NATS, graceful degradation to mock mode)

### Partial

- **Reward distributor**: Tracking and batching work, but `simulate_aptos_transfer()` generates fake tx hashes — no real Aptos blockchain signing
- **Bandwidth estimation**: `estimate_bandwidth()` returns `None` (TODO)

### Stub / Not Working

- **Mining**: `CpuMiner::start()` is a no-op. No Stratum protocol, no pool connection, no hashing.

## Running

```bash
# Build the node binary
flox activate -- cargo build --release --bin apn_node

# Start a node manually
./target/release/apn_node \
  --port 4001 \
  --relay nats://nonlocal.info:4222 \
  --heartbeat-interval 30

# Start with bootstrap peer
./target/release/apn_node \
  --port 4002 \
  --relay nats://nonlocal.info:4222 \
  --bootstrap /ip4/192.168.1.77/tcp/4001/p2p/<PEER_ID>

# Quick setup for peer nodes
chmod +x setup-peer-node.sh && ./setup-peer-node.sh

# Monitor
tail -f /tmp/apn_node.log
curl http://localhost:8000/api/network/stats
curl http://localhost:8000/api/identity
```

## Planned Phases

1. **Phase 1** (current): Foundation — networking, crypto, peer discovery, heartbeats, reward tracking, dashboard
2. **Phase 2**: Task Distribution Bridge
3. **Phase 3**: Remote Execution Streaming
4. **Phase 4**: Resource Accounting
5. **Phase 5**: Full Economic Layer (production VIBE integration, real Aptos transfers)
