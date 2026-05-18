# APN Infrastructure Advancement Plan
## From Current State to Sovereign Data Network

### Context

The APN (Alpha Protocol Network) has a solid Layer 0 (networking) with libp2p mesh, NATS relay, Ed25519 identity, and heartbeat tracking all working. However, the data layer is incomplete — files don't actually move between peers, the Aptos blockchain integration is simulated, and cross-node access relies on shared Dropbox rather than the mesh. The PCG Dashboard (Powerclub Global) serves as the frontend for this network, with the Org Cloud and Intelligence tabs already functional for local sovereign stack browsing.

**Goal:** Advance the APN so that organizations can store, serve, and transmit data between peers with the PCG Dashboard as the user-facing experience — including shareable links for external clients.

---

## Current Architecture (What's Real)

```
┌─────────────────────────────────────────────────────────────┐
│                    PCG DASHBOARD (Frontend)                   │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Cloud    │  │ Intel    │  │ Projects │  │ CRM      │    │
│  │ Browser  │  │ Data Src │  │ Tasks    │  │ Pipeline │    │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘    │
│       │              │              │              │          │
│       └──────────────┴──────────────┴──────────────┘          │
│                          REST API                             │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────┴──────────────────────────────────┐
│                  PCG BACKEND (Rust/Axum)                      │
│                                                              │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐       │
│  │ org_cloud   │  │ sovereign    │  │ org_cloud     │       │
│  │ routes      │  │ _stack.rs    │  │ _indexer.rs   │       │
│  │ (browse/    │  │ (Dropbox     │  │ (filesystem   │       │
│  │  download/  │  │  scraper)    │  │  → cloud_files│       │
│  │  upload)    │  │              │  │  → data_src)  │       │
│  └──────┬──────┘  └──────┬──────┘  └───────┬───────┘       │
│         │                │                  │                │
│  ┌──────┴────────────────┴──────────────────┴──────┐        │
│  │              SQLite (dev_assets/db.sqlite)       │        │
│  │  cloud_files | data_sources | org_cloud_settings │        │
│  └─────────────────────────────────────────────────┘        │
│                                                              │
│  ┌─────────────────────────────────────────────────┐        │
│  │           E: Drive (Sovereign Stack)             │        │
│  │  sovereign_stack/Sirak Studios/                  │        │
│  │    ├── ACTIVE CLIENTS/  ├── PROJECTS/            │        │
│  │    ├── ARCHIVE/         ├── RESOURCES/           │        │
│  │    ├── PROPOSALS/       ├── SALES/               │        │
│  │    └── SOCIAL MEDIA/    └── Uploads/             │        │
│  │  sovereign_stack/Personal/                       │        │
│  └─────────────────────────────────────────────────┘        │
└──────────────────────────────────────────────────────────────┘

                    ┌──────────────┐
                    │  NATS Relay  │
                    │ nonlocal.info│
                    │    :4222     │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
     ┌────────┴───┐  ┌────┴────┐  ┌───┴────────┐
     │ Sirak Node │  │ Pythia  │  │ Other Peers│
     │ (Master)   │  │ (Master)│  │            │
     │            │  │         │  │            │
     │ DB sync ◄──┼──► DB sync │  │  DB sync   │
     │ (rows only)│  │(rows)   │  │  (rows)    │
     └────────────┘  └─────────┘  └────────────┘

     ❌ Files do NOT transfer between peers
     ❌ Aptos rewards are simulated (fake tx hashes)
     ❌ No share links for external users
     ❌ Consumer nodes can't pull files from provider
```

---

## Target Architecture (Where We're Going)

```
┌─────────────────────────────────────────────────────────────────┐
│              PCG DASHBOARD (Powerclub Global Frontend)            │
│                                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────┐      │
│  │ Cloud    │  │ Intel    │  │ Share    │  │ APN        │      │
│  │ Browser  │  │ Data Src │  │ Links   │  │ Network    │      │
│  │ + Preview│  │ + Preview│  │ Manager │  │ Dashboard  │      │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └─────┬──────┘      │
│       └──────────────┴─────────────┴──────────────┘              │
│                        REST API + WebSocket                      │
└────────────────────────────┬─────────────────────────────────────┘
                             │
┌────────────────────────────┴─────────────────────────────────────┐
│                   PCG BACKEND (Rust/Axum)                         │
│                                                                   │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐       │
│  │ org_cloud   │  │ share_links  │  │ apn_data_service  │       │
│  │ routes      │  │ routes       │  │ (provider mode)   │       │
│  │ (auth'd)    │  │ (token auth) │  │                   │       │
│  └──────┬──────┘  └──────┬───────┘  └─────────┬─────────┘       │
│         │                │                     │                  │
│  ┌──────┴────────────────┴─────────────────────┴──────────┐      │
│  │              SQLite + File System                       │      │
│  │  cloud_files | shared_links | data_sources              │      │
│  │  E:/topos/sovereign_stack/ (canonical file store)       │      │
│  └────────────────────────────────────────────────────────┘      │
│                                                                   │
│  ┌────────────────────────────────────────────────────────┐      │
│  │              APN Integration Layer                      │      │
│  │  ┌──────────────┐  ┌─────────────┐  ┌──────────────┐  │      │
│  │  │ File Transfer│  │ DB Sync     │  │ Contribution │  │      │
│  │  │ (chunked     │  │ (rows +     │  │ Tracker      │  │      │
│  │  │  over NATS)  │  │  manifests) │  │ (VIBE token) │  │      │
│  │  └──────┬───────┘  └──────┬──────┘  └──────┬───────┘  │      │
│  │         └─────────────────┼─────────────────┘          │      │
│  └───────────────────────────┼────────────────────────────┘      │
└──────────────────────────────┼───────────────────────────────────┘
                               │
                    ┌──────────┴──────────┐
                    │    NATS Relay       │
                    │  nonlocal.info:4222 │
                    │                    │
                    │  + libp2p mesh     │
                    │  (direct P2P when  │
                    │   on same LAN)     │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
     ┌────────┴────────┐  ┌───┴──────────┐  ┌──┴───────────┐
     │  SIRAK STUDIOS  │  │   PYTHIA     │  │ CLIENT NODE  │
     │  Master Node    │  │  Master Node │  │ (Org Member) │
     │                 │  │              │  │              │
     │  Provider Mode  │  │ Provider +   │  │ Consumer     │
     │  ─────────────  │  │ Consumer     │  │ Mode         │
     │  ✓ Hosts files  │  │ ───────────  │  │ ────────────│
     │  ✓ Serves peers │  │ ✓ Own files  │  │ ✓ Browse    │
     │  ✓ Share links  │  │ ✓ Pulls from │  │   remote    │
     │  ✓ DB authority │  │   Sirak      │  │ ✓ Download  │
     │  ✓ VIBE rewards │  │ ✓ Caches     │  │   on demand │
     │                 │  │   locally    │  │ ✓ Upload    │
     │  E: 21TB        │  │              │  │   artifacts │
     └─────────────────┘  └──────────────┘  └──────────────┘

     ✅ Files transfer via NATS (chunked, compressed)
     ✅ Aptos rewards for storage/bandwidth contribution
     ✅ Share links for external clients (token-based)
     ✅ Consumer nodes pull files on demand from provider
```

---

## Implementation Phases

### Phase 1: Share Links (MVP — PCG Dashboard Only)
**Priority: HIGH | Effort: 1-2 weeks | No APN changes needed**

This phase works entirely within the PCG Dashboard, no mesh networking required.

```
External Client                    PCG Dashboard (Sirak Master Node)
─────────────                     ─────────────────────────────────

  Admin generates share link:
  POST /api/org-cloud/{org}/share-links
    { folder: "ACTIVE CLIENTS/Cino",
      permissions: ["view", "download", "upload"],
      expires_in: "30d",
      password: null }
    → returns: { token: "sk_abc123...", url: "localhost:3000/shared/sk_abc123" }

  Client opens link:
  GET /shared/sk_abc123
    ↓
  Lightweight file browser (no sidebar, no login required)
    ↓
  Browse: GET /api/shared/sk_abc123/browse
  Download: GET /api/shared/sk_abc123/files/{id}/download
  Upload: POST /api/shared/sk_abc123/contribute (if permitted)
    ↓
  Files land in sovereign_stack → indexed → visible in Cloud tab
```

**Database:**
```sql
CREATE TABLE shared_links (
    id TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id),
    token TEXT NOT NULL UNIQUE,           -- "sk_" + secure random
    created_by TEXT NOT NULL REFERENCES users(id),

    -- Scope
    target_type TEXT NOT NULL DEFAULT 'folder',  -- folder | file | volume
    target_path TEXT,                     -- e.g. "ACTIVE CLIENTS/Cino"
    storage_volume TEXT DEFAULT 'sovereign_org',

    -- Permissions
    can_view INTEGER DEFAULT 1,
    can_download INTEGER DEFAULT 1,
    can_upload INTEGER DEFAULT 0,
    password_hash TEXT,                   -- bcrypt, optional

    -- Limits
    expires_at TEXT,
    max_downloads INTEGER,
    download_count INTEGER DEFAULT 0,

    -- Metadata
    label TEXT,                           -- "Cino Product Photos"
    created_at TEXT DEFAULT (datetime('now')),
    revoked_at TEXT
);
```

**Backend routes** (no auth middleware):
- `GET /api/shared/{token}/browse` — list files in scoped folder
- `GET /api/shared/{token}/files/{id}/download` — download (respects can_download)
- `GET /api/shared/{token}/files/{id}/preview` — inline preview
- `POST /api/shared/{token}/contribute` — upload (respects can_upload)

**Frontend**: New page at `/shared/{token}` — minimal CloudBrowser without sidebar/auth.

**For authenticated users**: If an org member opens a share link while logged in, merge their session permissions with the link permissions (higher of the two wins).

---

### Phase 2: Cross-Node File Transfer (APN Data Layer)
**Priority: HIGH | Effort: 2-3 weeks | Core APN advancement**

Enable actual file bytes to move between peers over NATS.

```
Consumer (Pythia)                NATS Relay              Provider (Sirak Master)
────────────────                ──────────              ───────────────────────

1. User browses Cloud tab
   → sees Sirak Studios files
   (from synced cloud_files rows)

2. User clicks Download
   → local file NOT found
   → triggers remote fetch:

   FileRequest ──────────────► apn.files.request.{sirak_id}
   { file_id, cloud_file_id,                    │
     requesting_device }                         │
                                                 ▼
                                          Provider receives request
                                          → looks up cloud_files
                                          → reads file from E: drive
                                          → chunks into 256KB pieces
                                          → compresses each chunk (gzip)

   apn.files.chunk.{pythia_id} ◄──────── FileChunk { transfer_id,
                                           chunk_index, total_chunks,
                                           data: Vec<u8>, hash }

   (repeat for all chunks)

   apn.files.complete.{pythia_id} ◄───── TransferComplete { transfer_id,
                                           total_size, file_hash }

3. Consumer reassembles chunks
   → verifies SHA-256 hash
   → writes to local cache
   → serves to frontend

4. Contribution recorded
   → Provider earns VIBE for bandwidth
```

**NATS Subjects:**
```
apn.files.request.{provider_id}     — Consumer requests file
apn.files.chunk.{consumer_id}       — Provider sends chunk
apn.files.complete.{consumer_id}    — Transfer complete signal
apn.files.cancel.{transfer_id}      — Either side cancels
apn.files.manifest.{provider_id}    — Request file listing (optimization)
```

**Key design decisions:**
- **Chunk size**: 256KB (balances NATS message limits vs round-trip overhead)
- **Compression**: gzip per-chunk (most media files won't compress much, but docs will)
- **Caching**: Consumer caches downloaded files locally in `E:/topos/apn_cache/`
- **Deduplication**: Content-hash based — if consumer already has a file with matching SHA-256, skip transfer
- **Concurrency**: Max 4 concurrent chunk transfers per peer pair

---

### Phase 3: Sovereign Storage Sync v2 (Full Data Sync)
**Priority: MEDIUM | Effort: 2 weeks**

Upgrade the current DB-row-only sync to include file manifests so consumer nodes know what's available.

```
Provider (Sirak)                              Consumer (Pythia)
────────────────                              ────────────────

Every 5 minutes:

  Build manifest:
  { org_id, device_id, timestamp,
    files: [
      { id, path, hash, size, mime,
        volume, updated_at }
    ],
    total_size, file_count }

  Publish to:
  apn.storage.manifest.{org_id} ──────────►  Receive manifest
                                              → Diff against local cloud_files
                                              → Mark remote-only files as
                                                "available_from: sirak_device"
                                              → UI shows remote files with
                                                cloud icon badge
                                              → Click to download triggers
                                                Phase 2 file transfer
```

**cloud_files schema addition:**
```sql
ALTER TABLE cloud_files ADD COLUMN source_device TEXT;    -- device that hosts file
ALTER TABLE cloud_files ADD COLUMN is_local INTEGER DEFAULT 1;  -- 0 = remote only
```

---

### Phase 4: VIBE Token Settlement (Aptos Blockchain)
**Priority: MEDIUM | Effort: 2-3 weeks**

Replace simulated transfers with actual on-chain Aptos transactions.

```
Contribution Lifecycle:

  Peer heartbeat (every 30s)
       │
       ▼
  Reward Tracker creates peer_rewards row
  (base_amount × multiplier = final_amount)
       │
       ▼
  Reward Distributor batches every 5 min
  (groups by wallet, creates reward_batch)
       │
       ▼
  ┌─── Currently: simulate_aptos_transfer() → fake hash
  │
  └─── Target: aptos_sdk::transfer()
         │
         ▼
       Aptos Testnet/Mainnet
       ─────────────────────
       VIBE Token Contract (Move module)
       ├── mint(to, amount)        — treasury mints rewards
       ├── transfer(from, to, amt) — peer-to-peer
       ├── balance(addr)           — check balance
       └── stake(amount, duration) — future: staking

       Transaction confirmed → update reward status
       peer_rewards.status = 'confirmed'
       peer_rewards.aptos_tx_hash = real hash
```

**Prerequisites:**
- Deploy VIBE token Move module to Aptos testnet
- Fund treasury wallet with initial VIBE supply
- Implement `actual_aptos_transfer()` in reward_distributor.rs
- Add balance checking to peer dashboard

---

### Phase 5: External Access via alphaprotocol.network
**Priority: LOW | Effort: 3-4 weeks | Future**

Migrate share links from PCG Dashboard URLs to `alphaprotocol.network` domain.

```
Today (Phase 1):
  https://dashboard.powerclubglobal.com/shared/sk_abc123

Future (Phase 5):
  https://alphaprotocol.network/share/sk_abc123
    ↓
  APN Gateway (edge proxy)
    ↓
  Routes to correct master node based on token lookup
    ↓
  File served from provider's sovereign stack
```

This requires:
- APN Gateway service (lightweight Rust proxy)
- DNS + TLS for alphaprotocol.network
- Token-to-node routing table (which org/node owns this share?)
- CDN caching layer for popular shared files

---

## Summary: What Gets Built When

| Phase | Feature | Effort | Dependencies | Impact |
|-------|---------|--------|-------------|--------|
| **1** | Share Links (PCG) | 1-2 wk | None | Client uploads directly into org intelligence |
| **2** | Cross-Node File Transfer | 2-3 wk | Phase 1 | Peers can actually download files from each other |
| **3** | Manifest Sync v2 | 2 wk | Phase 2 | Consumer nodes see provider's files in their UI |
| **4** | VIBE Token (Aptos) | 2-3 wk | None | Real rewards for storage/bandwidth contribution |
| **5** | APN Gateway | 3-4 wk | Phase 1+2 | Share links served by alphaprotocol.network |

**Recommended execution order:** Phase 1 → Phase 2 → Phase 3 → Phase 4 (parallel with 2/3) → Phase 5

Phase 1 (Share Links) is the immediate MVP — it solves the client use case today with zero APN changes. Phases 2-3 make the mesh actually useful for multi-device orgs. Phase 4 can run in parallel since it's blockchain-only. Phase 5 is the long-term vision.

---

## Key Files to Modify Per Phase

**Phase 1 (Share Links):**
- NEW: `crates/db/migrations/XXXXXX_shared_links.sql`
- NEW: `crates/server/src/routes/shared_links.rs`
- NEW: `frontend/src/pages/shared/index.tsx`
- MODIFY: `crates/server/src/routes/org_cloud.rs` (add share-link CRUD)
- MODIFY: `frontend/src/pages/organization-profile/tabs/cloud/CloudBrowser.tsx` (share button)

**Phase 2 (File Transfer):**
- MODIFY: `crates/server/src/apn_data_service.rs` (implement request/response handlers)
- NEW: `crates/alpha-protocol-core/src/file_transfer.rs` (chunked transfer protocol)
- MODIFY: `crates/server/src/routes/org_cloud.rs` (remote file proxy)
- MODIFY: `crates/server/src/routes/mesh.rs` (file transfer status endpoints)

**Phase 3 (Manifest Sync):**
- MODIFY: `crates/server/src/sovereign_storage.rs` (add manifest publishing)
- MODIFY: `crates/db/migrations/` (add source_device, is_local to cloud_files)
- MODIFY: `frontend/src/pages/organization-profile/tabs/cloud/CloudBrowser.tsx` (remote badge)

**Phase 4 (VIBE Token):**
- MODIFY: `crates/alpha-protocol-core/src/reward_distributor.rs` (real Aptos transfers)
- NEW: `aptos/vibe_token/` (Move module for VIBE token)
- MODIFY: `crates/server/src/routes/peer_rewards.rs` (balance checking)
