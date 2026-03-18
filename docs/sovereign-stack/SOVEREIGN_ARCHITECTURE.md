# Sovereign Stack Architecture for PCG Dashboard

## Philosophy: Local-First, Federation Over Centralization

Instead of syncing all data to a central server, we maintain **data sovereignty** where:
- 🏠 Your data lives on YOUR device
- 🌐 Server is just a coordinator/index
- 🔒 You control your data's availability
- ⚡ Direct peer-to-peer when possible
- 💾 Optional caching for offline scenarios

---

## Core Principles

### 1. Data Sovereignty
**You own your data, you serve your data**
- Data stays on your device by default
- You decide if/when to share
- You can go offline anytime
- No forced synchronization

### 2. Federation Over Centralization
**Devices are equal peers, server is just an index**
- No single source of truth
- Server doesn't own content
- Devices talk directly when possible
- Server routes queries to owners

### 3. Privacy by Default
**Minimal data exposure**
- Only metadata goes to server
- Actual content served from owner
- End-to-end encryption possible
- Audit trails stay local

### 4. Graceful Degradation
**System works even when devices offline**
- Online devices → full access
- Offline devices → cached views (if enabled)
- Mixed mode → partial availability
- Clear offline indicators

---

## Architecture Layers

### Layer 1: Local Sovereign Node

Each device runs its own stack:

```
┌─────────────────────────────────────────┐
│  Sirak's Laptop (Sovereign Node)        │
├─────────────────────────────────────────┤
│                                          │
│  📁 Local SQLite Database                │
│     - Full data (projects, tasks, etc)  │
│     - Full history and audit trail      │
│     - Private keys and credentials      │
│                                          │
│  🌐 APN Data Server                      │
│     - Serves own data over APN           │
│     - Responds to queries from network   │
│     - Access control (who can query)     │
│                                          │
│  📡 APN Node                             │
│     - Publishes presence/availability    │
│     - Subscribes to data requests        │
│     - Direct P2P when possible           │
│                                          │
│  🔒 Sovereignty Guardian                 │
│     - Controls what data is shared       │
│     - Enforces access policies           │
│     - Audit logging                      │
│                                          │
└─────────────────────────────────────────┘
```

**What runs locally:**
- Full PCG Dashboard backend (Rust server)
- Complete database with all your data
- APN node for network communication
- Data server to respond to queries
- Access control layer

### Layer 2: Central Index Node

Server acts as coordinator, NOT data owner:

```
┌─────────────────────────────────────────┐
│  Network Server (Index Node)            │
├─────────────────────────────────────────┤
│                                          │
│  👤 User Registry                        │
│     - Usernames, emails, auth           │
│     - Public keys for encryption         │
│     - Device associations                │
│                                          │
│  📇 Metadata Index                       │
│     - What data exists                   │
│     - Who owns it                        │
│     - Where to find it (device ID)       │
│     - When last seen                     │
│                                          │
│  🔍 Query Router                         │
│     - Routes queries to owner devices    │
│     - Aggregates results from peers      │
│     - Handles offline scenarios          │
│                                          │
│  💾 Cache Layer (Optional)               │
│     - Thumbnails and previews            │
│     - Metadata snapshots                 │
│     - Recent activity summaries          │
│     - NOT full data replication          │
│                                          │
└─────────────────────────────────────────┘
```

**What server stores:**
- User accounts (authentication)
- Metadata index (pointers to data)
- Device registry (who's online)
- Optional caches (thumbnails, summaries)
- NOT the actual project/task data

### Layer 3: Federation Protocol

Devices communicate via APN:

```
Query Flow:
1. User opens dashboard → connects to server
2. Server shows: "Sirak's projects (3 online, 2 offline)"
3. User clicks project → server routes to Sirak's device
4. Sirak's device serves data directly
5. Results stream back through APN

Device Offline Flow:
1. Server checks: Is Sirak's device online?
2. NO → Check cache policy
3. If cached: Serve snapshot + "⚠️ Offline data"
4. If not cached: "❌ Device offline, data unavailable"
```

---

## Implementation Components

### Component 1: Local Data Server

**File:** `local_data_server.py` (runs on each device)

```python
# Serves YOUR data from YOUR device
class LocalDataServer:
    """
    Serves local database over APN network
    Only responds to authorized queries
    """

    def handle_query(self, query, requester):
        # Check if requester is authorized
        if not self.sovereignty_guardian.authorize(requester, query):
            return {"error": "Unauthorized"}

        # Execute query on local database
        result = self.local_db.execute(query)

        # Log the access
        self.audit_log.record(requester, query, timestamp)

        return result
```

**Key Features:**
- Serves data only when device is online
- Access control per query
- Audit logging
- Rate limiting
- Can go offline anytime

### Component 2: Metadata Index

**File:** `metadata_index.py` (runs on server)

```python
# Tracks what data exists and where
class MetadataIndex:
    """
    Central index of data locations
    Does NOT store actual data
    """

    def register_content(self, owner_id, content_type, content_id):
        # Record that data exists
        self.index[content_id] = {
            "owner": owner_id,
            "type": content_type,
            "device": owner_device_id,
            "online": True,
            "last_seen": timestamp
        }

    def query(self, content_id):
        # Return pointer to data, not data itself
        metadata = self.index.get(content_id)
        if metadata["online"]:
            return {"location": metadata["device"]}
        else:
            return {"error": "Device offline", "cache_available": has_cache}
```

**What it stores:**
```json
{
  "project_abc123": {
    "owner": "sirak",
    "type": "project",
    "device": "sirak_laptop",
    "online": true,
    "last_seen": "2026-02-09T15:30:00Z"
  }
}
```

### Component 3: Query Router

**File:** `federated_query_router.py` (runs on server)

```python
class FederatedQueryRouter:
    """
    Routes queries to appropriate devices
    Handles online/offline scenarios
    """

    async def get_project(self, project_id, requester):
        # Look up where this project lives
        metadata = self.index.query(project_id)

        if metadata["online"]:
            # Device is online - query directly
            return await self.apn.query(
                device=metadata["device"],
                query={"type": "get_project", "id": project_id},
                requester=requester
            )
        else:
            # Device offline - check cache
            if self.cache.has(project_id):
                return {
                    "data": self.cache.get(project_id),
                    "warning": "Offline data (cached)",
                    "last_updated": metadata["last_seen"]
                }
            else:
                return {
                    "error": "Device offline",
                    "owner": metadata["owner"],
                    "last_seen": metadata["last_seen"]
                }
```

### Component 4: Sovereignty Guardian

**File:** `sovereignty_guardian.py` (runs on each device)

```python
class SovereigntyGuardian:
    """
    Controls what data can be accessed from your device
    Privacy and access control
    """

    def __init__(self):
        self.policies = {
            "public": ["projects.name", "projects.created_at"],
            "authenticated": ["projects.*", "tasks.title"],
            "owner_only": ["tasks.description", "agent_conversations.*"]
        }

    def authorize(self, requester, query):
        # Check requester's permission level
        level = self.get_permission_level(requester)

        # Check if query accesses allowed fields
        fields = self.extract_fields(query)
        for field in fields:
            if field not in self.policies[level]:
                self.audit_log.denied(requester, field)
                return False

        return True
```

---

## Data Tiering Strategy

### Tier 1: Always Local (Sovereign)
**Never leaves your device unless you explicitly share**

- Agent conversations (private AI chats)
- Activity logs (your local audit trail)
- Private notes and drafts
- Credentials and secrets
- Full media files (videos, large images)

### Tier 2: Indexed Metadata (Federated)
**Metadata goes to index, content stays local**

- Projects (metadata indexed, data local)
- Tasks (metadata indexed, data local)
- Media files (thumbnails indexed, files local)
- Workflows (metadata indexed, data local)

### Tier 3: Shared/Collaborative (Replicated)
**Optionally replicated for collaboration**

- Shared projects (co-owned)
- Team tasks (multi-user)
- Public content (intentionally shared)

### Tier 4: Global (Centralized)
**Makes sense to centralize**

- User accounts (authentication)
- Network topology (who's connected)
- VIBE transactions (blockchain anyway)
- Public marketplace content

---

## Dashboard UX with Federation

### User's View

**Online Device:**
```
┌─────────────────────────────────────┐
│ Sirak's Projects                    │
├─────────────────────────────────────┤
│ ✅ Website Redesign    [Sirak's Mac]│
│ ✅ Mobile App          [Sirak's Mac]│
│ ✅ API Integration     [Sirak's Mac]│
└─────────────────────────────────────┘
```

**Offline Device:**
```
┌─────────────────────────────────────┐
│ Sirak's Projects                    │
├─────────────────────────────────────┤
│ ⚠️  Website Redesign   [Offline]    │
│     Last seen: 2 hours ago          │
│     Cached data available           │
│ ❌ Mobile App          [Offline]    │
│     No cached data                  │
└─────────────────────────────────────┘
```

**Mixed (Multiple Devices):**
```
┌─────────────────────────────────────┐
│ Sirak's Projects                    │
├─────────────────────────────────────┤
│ ✅ Website Redesign    [Mac - Online]│
│ ✅ Mobile App          [Mac - Online]│
│ ⚠️  Server Setup       [Laptop - Offline]│
│ ✅ Dashboard v2        [Desktop - Online]│
└─────────────────────────────────────┘
```

---

## Cache Policy Options

### Option 1: No Cache (Pure Sovereign)
- ✅ Maximum sovereignty
- ✅ No data on server
- ❌ No offline access
- **Use case:** Maximum privacy

### Option 2: Metadata Only (Lightweight)
- ✅ High sovereignty
- ✅ Minimal server data
- ⚠️ Basic offline access (metadata only)
- **Use case:** Balanced approach

### Option 3: Snapshot Cache (Hybrid)
- ⚠️ Reduced sovereignty
- ✅ Full offline access
- ⚠️ Server has data copy
- **Use case:** Mobile/traveling users

### Option 4: Full Replication (Traditional)
- ❌ No sovereignty
- ✅ Maximum availability
- ❌ Centralized
- **Use case:** Traditional SaaS model

**Recommendation:** Start with Option 2, let users choose.

---

## Migration Path

### Phase 1: Dual Mode (Current + Sovereign)

Keep current centralized system, add sovereign option:

```
Server Database (Centralized)
    ↓
    ├─ Users who sync to server (traditional)
    └─ Users who serve locally (sovereign)
```

### Phase 2: Hybrid Architecture

```
Index Node (Metadata)
    ↓
    ├─ Sovereign Nodes (most data local)
    ├─ Cached Snapshots (optional)
    └─ Shared Projects (collaborative)
```

### Phase 3: Full Federation

```
Peer Network
    ├─ Device 1 (sovereign)
    ├─ Device 2 (sovereign)
    ├─ Device 3 (sovereign)
    └─ Index Node (routing only)
```

---

## Implementation Roadmap

### Week 1: Local Data Server
- [ ] Create local data server component
- [ ] APN integration for queries
- [ ] Access control layer
- [ ] Audit logging

### Week 2: Metadata Index
- [ ] Design metadata schema
- [ ] Implement index on server
- [ ] Device registration
- [ ] Presence tracking

### Week 3: Query Router
- [ ] Federated query routing
- [ ] Online/offline handling
- [ ] Cache layer (optional)
- [ ] Error handling

### Week 4: Dashboard Integration
- [ ] Update frontend to show device status
- [ ] Online/offline indicators
- [ ] Cache policy settings
- [ ] Multi-device view

---

## Technical Specifications

### APN Protocol Extensions

**New Message Types:**

```
apn.data.query.<device_id>       - Query device for data
apn.data.response.<requester_id> - Response with data
apn.index.register               - Register content metadata
apn.index.heartbeat              - Device presence
apn.cache.request                - Request cache update
```

### Data Query Protocol

```json
{
  "type": "DATA_QUERY",
  "query_id": "uuid",
  "requester": "user_id",
  "target": "device_id",
  "query": {
    "type": "get_project",
    "id": "project_id",
    "fields": ["name", "tasks", "created_at"]
  },
  "auth_token": "jwt_token"
}
```

### Metadata Schema

```sql
CREATE TABLE content_index (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL,  -- 'project', 'task', etc
    owner_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    online BOOLEAN DEFAULT FALSE,
    last_seen TIMESTAMP,
    metadata JSON,  -- Searchable metadata
    cache_policy TEXT,  -- 'none', 'metadata', 'snapshot'
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);

CREATE INDEX idx_owner ON content_index(owner_id);
CREATE INDEX idx_device ON content_index(device_id);
CREATE INDEX idx_online ON content_index(online);
```

---

## Benefits of Sovereign Architecture

### For Users
- ✅ **Privacy**: Your data stays on your device
- ✅ **Control**: You decide what to share
- ✅ **Offline**: Works without server
- ✅ **Speed**: Direct device-to-device
- ✅ **Ownership**: True data ownership

### For Network
- ✅ **Scalability**: No central bottleneck
- ✅ **Resilience**: No single point of failure
- ✅ **Bandwidth**: Distributed load
- ✅ **Cost**: Less server storage needed
- ✅ **Legal**: No data custody liability

### For APN
- ✅ **Decentralization**: True P2P network
- ✅ **Sovereignty**: Aligns with APN values
- ✅ **Innovation**: New use case for APN
- ✅ **Differentiation**: Unique vs traditional SaaS

---

## Security Considerations

### Authentication
- User authenticates with server (JWT)
- Devices authenticate with each other (APN keys)
- End-to-end encryption for queries

### Authorization
- Server controls who can query
- Devices control what data to serve
- Fine-grained access policies

### Data Integrity
- Checksums for all data transfers
- Signatures for mutations
- Audit trails on all devices

### Privacy
- Encrypted APN communication
- Optional E2E encryption
- No plaintext data on server
- Local audit logs

---

## Comparison with Current Architecture

| Aspect | Current (Centralized) | Sovereign (Proposed) |
|--------|----------------------|---------------------|
| Data Location | Server | Local devices |
| Offline Access | Via cache | Native (own data) |
| Privacy | Trust server | Self-sovereign |
| Scalability | Server bottleneck | Distributed |
| Complexity | Simple | More complex |
| Latency | Medium | Variable (P2P=fast, offline=none) |
| Cost | High storage | Low storage |

---

## Conclusion

The sovereign architecture:
- ✅ Aligns with APN's decentralization values
- ✅ Gives users true data ownership
- ✅ Scales better than centralized
- ✅ More resilient to failures
- ⚠️ More complex to implement
- ⚠️ Requires user education

**Recommendation:** Implement hybrid model where users can choose between:
1. **Sovereign mode** (data stays local, serve over APN)
2. **Sync mode** (replicate to server for 24/7 availability)
3. **Hybrid mode** (metadata synced, data local)

Let users decide based on their needs!

---

**Status:** Design complete, ready for implementation
**Next Steps:** Build local data server + metadata index
**Timeline:** 4 weeks for MVP
