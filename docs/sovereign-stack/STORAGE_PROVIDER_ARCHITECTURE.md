# APN Storage Provider Architecture

## Overview: Sovereign Cloud Storage Marketplace

A decentralized storage marketplace where:
- **Data owners** (Sirak) pay VIBE for backup/availability
- **Storage providers** (Pythia) earn VIBE for hosting data
- **Always-on nodes** (Bonomotion) serve shared projects
- **Access control** via APN identity system

---

## Node Types

### Type 1: Always-On Sovereign Server
**Example:** Bonomotion's Studio Device

**Characteristics:**
- Runs 24/7 at fixed location
- Hosts projects and serves them reliably
- Can have collaborators (multi-user projects)
- Full database locally
- Acts as project server for team

**Use Cases:**
- Studio/office computers
- Home servers
- Raspberry Pi nodes
- Desktop workstations

**Economics:**
- No storage fees (owns own data)
- May earn VIBE if storing others' data
- Pays electricity but earns from collaboration features

### Type 2: Mobile Sovereign Node
**Example:** Sirak's Laptop

**Characteristics:**
- Goes offline frequently
- Full local database when online
- Needs backup for offline periods
- Syncs to storage provider when online

**Use Cases:**
- Laptops
- Mobile devices
- Traveling workers
- Part-time nodes

**Economics:**
- Pays VIBE for storage service
- Pays based on data size + transfers
- Cost scales with usage

### Type 3: Storage Provider Node
**Example:** Pythia Master Node

**Characteristics:**
- Always online (high uptime)
- Large storage capacity
- Serves data when owners offline
- Earns VIBE for service

**Use Cases:**
- Data centers
- Home servers with excess capacity
- Always-on desktop computers
- Dedicated storage nodes

**Economics:**
- Earns VIBE per GB stored
- Earns VIBE per GB transferred
- Bonus for high uptime (99%+)
- Costs: electricity + hardware

---

## Three-Device Scenario

### Device Roles in Our Setup

```
┌──────────────────────────────────────────────────────┐
│  BONOMOTION'S STUDIO DEVICE                          │
│  Role: Always-On Sovereign Server                    │
├──────────────────────────────────────────────────────┤
│  - Hosts Bonomotion's projects                       │
│  - Shared with Sirak (collaborator)                  │
│  - Always online (studio desktop)                    │
│  - Serves own data 24/7                              │
│                                                       │
│  Revenue: None (owns own data)                       │
│  Cost: Electricity                                   │
└──────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────┐
│  SIRAK'S LAPTOP                                      │
│  Role: Mobile Sovereign Node                         │
├──────────────────────────────────────────────────────┤
│  - Hosts Sirak's personal projects                   │
│  - Goes offline frequently                           │
│  - Syncs to Pythia when online                       │
│  - Local-first when available                        │
│                                                       │
│  Revenue: None (consuming storage)                   │
│  Cost: ~9 VIBE/month to Pythia                       │
└──────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────┐
│  PYTHIA MASTER NODE                                  │
│  Role: Storage Provider                              │
├──────────────────────────────────────────────────────┤
│  - Stores replica of Sirak's data                    │
│  - Serves when Sirak offline                         │
│  - High availability (99.9% uptime)                  │
│  - Earns VIBE for service                            │
│                                                       │
│  Revenue: ~9 VIBE/month from Sirak                   │
│  Cost: Electricity + minimal storage                 │
└──────────────────────────────────────────────────────┘
```

### Data Distribution

| Data | Bonomotion Studio | Sirak's Laptop | Pythia Node | Who Can Access |
|------|-------------------|----------------|-------------|----------------|
| Bonomotion's Projects | ✅ Primary | ❌ | ❌ | Bonomotion, Sirak (collaborator) |
| Shared Boards | ✅ Primary | Cached | ❌ | Bonomotion, Sirak |
| Sirak's Projects | ❌ | ✅ Primary | ✅ Replica | Sirak only |
| User Accounts | ❌ | ❌ | ✅ Index | Everyone (public keys) |

---

## Storage Replication Protocol

### Real-Time Sync (Laptop Online)

```
Sirak's Laptop (online)
  ↓
1. Detects changes to local database
2. Calculates delta (what changed since last sync)
3. Encrypts delta with Sirak's key
4. Sends to Pythia via APN (apn.storage.sync)
5. Pythia acknowledges receipt
6. Pythia applies changes to replica
7. Pythia sends confirmation
```

**Protocol:**
```json
{
  "type": "STORAGE_SYNC",
  "from": "sirak_laptop",
  "to": "pythia_master",
  "data": {
    "delta": "base64_encrypted_changes",
    "version": 123,
    "checksum": "sha256_hash"
  },
  "signature": "ed25519_signature"
}
```

### Conflict Resolution

**When Sirak edits offline then comes online:**

```
Laptop comes online
  ↓
Checks version with Pythia
  ↓
Laptop version: 123
Pythia version: 125 (newer!)
  ↓
Conflict detected
  ↓
Resolution strategies:
  1. Last-write-wins (simple)
  2. Merge with CRDTs (complex)
  3. Ask user (safe)
```

**Recommendation:** Last-write-wins for MVP, CRDTs for v2

---

## Access Control & Collaboration

### Bonomotion's Projects (Shared with Sirak)

**Access Control List:**
```json
{
  "project_id": "bonomotion_website",
  "owner": "bonomotion",
  "collaborators": [
    {
      "user": "sirak",
      "role": "editor",
      "permissions": ["read", "write", "comment"]
    }
  ],
  "visibility": "private"
}
```

**Query Flow:**
```
Sirak requests Bonomotion's project
  ↓
Query routes to Bonomotion's studio device
  ↓
Studio device checks ACL
  ↓
Is Sirak in collaborators? ✅
Does Sirak have "read" permission? ✅
  ↓
Serve project data to Sirak
```

### Sirak's Projects (Private on Laptop/Pythia)

**Access Control:**
```json
{
  "project_id": "sirak_personal",
  "owner": "sirak",
  "collaborators": [],
  "visibility": "private",
  "storage_provider": "pythia_master",
  "encrypted": true
}
```

**Storage Provider Access:**
- Pythia stores **encrypted** data
- Pythia cannot read content (Sirak's key)
- Pythia only serves encrypted blobs
- Client (Sirak) decrypts locally

---

## Storage Provider Payment System

### Contract Setup

When Sirak enables cloud backup:

```
1. Sirak creates storage contract
2. Specifies: Pythia as provider
3. Deposits: 100 VIBE (escrow)
4. Terms: 5 GB, 99% uptime, 1 month
5. Contract published to blockchain
```

**Smart Contract (Aptos Move):**
```move
module storage_contract {
    struct StorageContract {
        client: address,           // Sirak
        provider: address,         // Pythia
        data_size_gb: u64,        // 5 GB
        monthly_rate: u64,        // 9 VIBE
        escrow_amount: u64,       // 100 VIBE
        start_date: u64,
        end_date: u64,
        uptime_requirement: u8,   // 99%
    }
}
```

### Payment Flow

**Monthly Billing:**
```
Day 1:
  - Sirak deposits 100 VIBE to escrow
  - Pythia starts storing data

Day 30:
  - System calculates actual usage:
    - Storage: 5.2 GB × 1 VIBE = 5.2 VIBE
    - Transfers: 12 GB × 0.1 VIBE = 1.2 VIBE
    - Uptime: 99.8% → 1.5x bonus
    - Total: (5.2 + 1.2) × 1.5 = 9.6 VIBE
  - Smart contract releases 9.6 VIBE to Pythia
  - Escrow balance: 90.4 VIBE

Month 2-10:
  - Continues same pattern

Month 11:
  - Escrow below threshold
  - System notifies Sirak to top up
  - Sirak deposits another 100 VIBE
```

### Uptime Verification

**Proof of Storage:**
```
Every hour:
  - Sirak's wallet sends challenge to Pythia
  - Challenge: "Prove you have block #123"
  - Pythia responds with merkle proof
  - If valid: uptime ++
  - If invalid: uptime violation

End of month:
  - Uptime = successful_proofs / total_challenges
  - 99.8% = 717/720 successful
  - Multiplier applied to payment
```

---

## Implementation Components

### Component 1: Storage Replication Client

**File:** `storage_replication_client.py` (runs on Sirak's laptop)

```python
class StorageReplicationClient:
    """
    Syncs local database to storage provider
    Runs on mobile sovereign nodes
    """

    def __init__(self, provider_node_id, encryption_key):
        self.provider = provider_node_id  # "pythia_master"
        self.key = encryption_key
        self.last_sync_version = 0

    async def sync(self):
        """Sync changes to storage provider"""
        # Get changes since last sync
        delta = self.db.get_changes_since(self.last_sync_version)

        # Encrypt delta
        encrypted = self.encrypt(delta, self.key)

        # Send to provider
        await self.apn.publish(
            f"apn.storage.sync.{self.provider}",
            {
                "type": "STORAGE_SYNC",
                "delta": encrypted,
                "version": self.db.version,
                "checksum": sha256(delta)
            }
        )

        # Wait for confirmation
        ack = await self.wait_for_ack()
        if ack["success"]:
            self.last_sync_version = self.db.version
```

### Component 2: Storage Provider Server

**File:** `storage_provider_server.py` (runs on Pythia)

```python
class StorageProviderServer:
    """
    Provides storage service for mobile nodes
    Earns VIBE for hosting data
    """

    def __init__(self):
        self.replicas = {}  # {client_id: encrypted_database}
        self.contracts = {}  # {client_id: StorageContract}

    async def handle_sync(self, msg):
        """Handle sync request from client"""
        client_id = msg["from"]
        delta = msg["delta"]

        # Store encrypted delta
        if client_id not in self.replicas:
            self.replicas[client_id] = EncryptedDatabase()

        self.replicas[client_id].apply_delta(delta)

        # Record metrics for billing
        self.record_storage(client_id, len(delta))

        # Send confirmation
        await self.apn.publish(
            f"apn.storage.ack.{client_id}",
            {"success": True, "version": msg["version"]}
        )

    async def serve_data(self, query):
        """Serve data when client offline"""
        client_id = query["owner"]

        # Check if we have replica
        if client_id not in self.replicas:
            return {"error": "No replica available"}

        # Serve encrypted data
        data = self.replicas[client_id].query(query)

        # Record transfer for billing
        self.record_transfer(client_id, len(data))

        return {"data": data, "source": "storage_provider"}
```

### Component 3: Project Collaboration Server

**File:** `project_collaboration_server.py` (runs on Bonomotion's studio)

```python
class ProjectCollaborationServer:
    """
    Serves shared projects to collaborators
    Runs on always-on sovereign servers
    """

    def __init__(self):
        self.projects = {}  # Local database
        self.acl = {}  # Access control lists

    def add_collaborator(self, project_id, user_id, role):
        """Add collaborator to project"""
        if project_id not in self.acl:
            self.acl[project_id] = {"collaborators": []}

        self.acl[project_id]["collaborators"].append({
            "user": user_id,
            "role": role,
            "permissions": self.get_permissions(role)
        })

    async def handle_query(self, query, requester):
        """Handle query from collaborator"""
        project_id = query["project_id"]

        # Check access
        if not self.has_access(requester, project_id):
            return {"error": "Access denied"}

        # Serve project data
        project = self.projects[project_id]
        return {"project": project, "source": "owner_device"}
```

---

## Dashboard UX Updates

### Project List View

```
┌─────────────────────────────────────────────────────┐
│  Sirak's Dashboard                                  │
├─────────────────────────────────────────────────────┤
│                                                      │
│  MY PROJECTS (3)                                    │
│  ✅ Website Redesign       [Your Laptop - Online]   │
│  ⚠️  Mobile App            [Pythia - Laptop Offline]│
│  ✅ API Integration        [Your Laptop - Online]   │
│                                                      │
│  SHARED WITH ME (2)                                 │
│  ✅ Bonomotion Website     [Studio Device - Online] │
│  ✅ Marketing Campaign     [Studio Device - Online] │
│                                                      │
│  STORAGE STATUS                                     │
│  📊 Using: 5.2 GB / 10 GB                           │
│  💰 Cost: 9.6 VIBE/month                            │
│  📡 Provider: Pythia Master (99.8% uptime)          │
│  💾 Last sync: 2 minutes ago                        │
│                                                      │
└─────────────────────────────────────────────────────┘
```

### File Download from Studio

```
Sirak clicks "Download" on file in Bonomotion's project
  ↓
Dashboard routes to Bonomotion's studio device
  ↓
Studio device checks: Is Sirak allowed? ✅
  ↓
Studio device streams file over APN
  ↓
File downloads directly to Sirak's browser
  ↓
No intermediate server storage
```

---

## Pricing Model

### Storage Tiers

| Tier | Storage | Monthly Cost | Provider Earnings |
|------|---------|-------------|-------------------|
| Free | 1 GB | 0 VIBE | 0 VIBE (subsidized) |
| Basic | 10 GB | 15 VIBE | 15 VIBE |
| Pro | 50 GB | 60 VIBE | 60 VIBE |
| Team | 200 GB | 200 VIBE | 200 VIBE |

### Transfer Costs

| Type | Cost | Provider Earnings |
|------|------|-------------------|
| Upload | 0 VIBE | 0 VIBE |
| Download | 0.1 VIBE/GB | 0.1 VIBE/GB |
| Sync | Included | Included |

### Uptime Bonuses

| Uptime | Multiplier | Example |
|--------|-----------|---------|
| 99.9%+ | 1.5x | 15 VIBE → 22.5 VIBE |
| 99-99.9% | 1.2x | 15 VIBE → 18 VIBE |
| 95-99% | 1.0x | 15 VIBE → 15 VIBE |
| <95% | 0.5x | 15 VIBE → 7.5 VIBE (penalty) |

---

## Security & Encryption

### Client-Side Encryption

**All data encrypted before leaving Sirak's laptop:**

```python
# Sirak's laptop encrypts before syncing
encryption_key = derive_key(sirak_password, sirak_wallet_seed)
encrypted_data = encrypt(project_data, encryption_key)
send_to_pythia(encrypted_data)

# Pythia stores encrypted blob (cannot read)
pythia.store(encrypted_data)

# When serving back to Sirak
encrypted_data = pythia.retrieve()
project_data = decrypt(encrypted_data, encryption_key)
```

**Pythia never has the key** - true zero-knowledge storage

### Access Control

**For shared projects (Bonomotion → Sirak):**

```python
# Bonomotion encrypts with shared key
shared_key = generate_shared_key(bonomotion_key, sirak_public_key)
encrypted_project = encrypt(project, shared_key)

# Sirak can decrypt
sirak_shared_key = generate_shared_key(sirak_key, bonomotion_public_key)
project = decrypt(encrypted_project, sirak_shared_key)
```

---

## Summary

**Perfect!** I understand the architecture:

1. **Bonomotion's Studio Device**
   - Always-on sovereign server
   - Hosts shared projects
   - Sirak is collaborator

2. **Sirak's Laptop**
   - Mobile sovereign node
   - Syncs to Pythia when online
   - Pays ~9 VIBE/month for storage

3. **Pythia Master Node**
   - Storage provider
   - Serves Sirak's data when laptop offline
   - Earns ~9 VIBE/month for service

This is a **sovereign cloud marketplace** - decentralized storage with VIBE economics!

Ready to implement?
