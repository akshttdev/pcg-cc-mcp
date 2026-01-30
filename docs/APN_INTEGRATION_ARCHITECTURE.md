# Alpha Protocol Network - Dashboard Integration Architecture

## Overview

The Alpha Protocol Network (APN) serves as the distributed compute and communication layer for the PCG Dashboard. This document outlines how workflow execution, resource sharing, and economic settlement integrate between the application layer (PCG Dashboard) and the transport layer (APN).

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────────┐
│  PCG Dashboard (Application Layer)                              │
│  ├── Nora Orchestrator - Workflow coordination                  │
│  ├── Task Executor - Spawns execution processes                 │
│  ├── Coordination Manager - Cross-agent events                  │
│  └── Mesh Panel - Network status & transaction logs             │
├─────────────────────────────────────────────────────────────────┤
│  APN Bridge (Integration Layer)                                 │
│  ├── TaskDistributor - Routes tasks to capable nodes            │
│  ├── ExecutionRelay - Streams logs/progress from remote nodes   │
│  ├── ResourceAccounting - Tracks bandwidth/compute usage        │
│  └── SettlementEngine - Vibe token credits/debits               │
├─────────────────────────────────────────────────────────────────┤
│  Alpha Protocol Network (Transport Layer)                       │
│  ├── NATS Relay - NAT traversal & message routing               │
│  ├── libp2p Mesh - Direct P2P when available                    │
│  ├── Peer Discovery - Kademlia DHT + mDNS                       │
│  └── Encrypted Transport - Noise protocol + ChaCha20            │
└─────────────────────────────────────────────────────────────────┘
```

## Message Types (Application-Level)

Replace raw `MeshMessage` with application-meaningful types:

```rust
/// Application-level mesh messages
pub enum APNMessage {
    // === Task Distribution ===
    /// Request to execute a task on the mesh
    TaskDistributionRequest {
        task_id: Uuid,
        task_attempt_id: Uuid,
        executor_profile: String,      // CLAUDE_CODE, AMP, etc.
        prompt: String,
        project_context: ProjectContext,
        resource_requirements: ResourceRequirements,
        reward_vibe: f64,              // Offered payment
    },

    /// Bid to execute a task
    TaskBid {
        task_id: Uuid,
        bidder_node: String,
        estimated_time_ms: u64,
        bid_vibe: f64,                 // Requested payment
        capabilities: Vec<String>,
    },

    /// Task assigned to a node
    TaskAssigned {
        task_id: Uuid,
        assigned_node: String,
        agreed_vibe: f64,
    },

    // === Execution Progress ===
    /// Execution started on remote node
    ExecutionStarted {
        task_id: Uuid,
        task_attempt_id: Uuid,
        execution_process_id: Uuid,
        executor_node: String,
        timestamp: DateTime<Utc>,
    },

    /// Progress update during execution
    ExecutionProgress {
        execution_process_id: Uuid,
        stage: String,                 // "setup", "coding", "testing", etc.
        progress_percent: u8,
        current_action: String,
        files_modified: u32,
        timestamp: DateTime<Utc>,
    },

    /// Log chunk from remote execution
    ExecutionLogs {
        execution_process_id: Uuid,
        log_type: LogType,             // Stdout, Stderr, Agent
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Execution completed
    ExecutionCompleted {
        task_id: Uuid,
        execution_process_id: Uuid,
        executor_node: String,
        summary: ExecutionSummaryData,
        git_diff: Option<String>,
        timestamp: DateTime<Utc>,
    },

    /// Execution failed
    ExecutionFailed {
        task_id: Uuid,
        execution_process_id: Uuid,
        executor_node: String,
        error: String,
        stage: String,
        timestamp: DateTime<Utc>,
    },

    // === Resource Accounting ===
    /// Bandwidth contribution record
    BandwidthContribution {
        from_node: String,
        to_node: String,
        bytes_transferred: u64,
        purpose: String,               // "task_relay", "log_stream", "git_sync"
        timestamp: DateTime<Utc>,
    },

    /// Compute contribution record
    ComputeContribution {
        executor_node: String,
        task_id: Uuid,
        cpu_seconds: f64,
        memory_gb_seconds: f64,
        timestamp: DateTime<Utc>,
    },

    // === Settlement ===
    /// Vibe credit transaction
    VibeTransaction {
        tx_id: Uuid,
        from_node: String,
        to_node: String,
        amount: f64,
        reason: TransactionReason,     // TaskExecution, BandwidthShare, Relay
        task_id: Option<Uuid>,
        timestamp: DateTime<Utc>,
    },
}
```

## Integration Flow: Distributed Task Execution

### Phase 1: Task Distribution

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│  Nora/Dashboard  │     │   APN Bridge     │     │   Mesh Network   │
└────────┬─────────┘     └────────┬─────────┘     └────────┬─────────┘
         │                        │                        │
         │ create_task_attempt()  │                        │
         │───────────────────────>│                        │
         │                        │                        │
         │                        │ Check local resources  │
         │                        │────────┐               │
         │                        │<───────┘               │
         │                        │                        │
         │                        │ [If insufficient]      │
         │                        │                        │
         │                        │ TaskDistributionRequest│
         │                        │───────────────────────>│
         │                        │                        │ Broadcast to
         │                        │                        │ capable peers
         │                        │                        │
         │                        │            TaskBid     │
         │                        │<───────────────────────│
         │                        │                        │
         │                        │ Select best bid        │
         │                        │────────┐               │
         │                        │<───────┘               │
         │                        │                        │
         │                        │         TaskAssigned   │
         │                        │───────────────────────>│
         │                        │                        │
```

### Phase 2: Remote Execution

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│    Dashboard     │     │   APN Bridge     │     │  Remote Node     │
└────────┬─────────┘     └────────┬─────────┘     └────────┬─────────┘
         │                        │                        │
         │                        │                        │ Receive task
         │                        │                        │────────┐
         │                        │                        │<───────┘
         │                        │                        │
         │                        │       ExecutionStarted │
         │                        │<───────────────────────│
         │                        │                        │
         │  Update UI: "Running   │                        │
         │   on remote node"      │                        │
         │<───────────────────────│                        │
         │                        │                        │
         │                        │      ExecutionProgress │
         │                        │<───────────────────────│ (periodic)
         │                        │                        │
         │  Stream to SSE         │                        │
         │<───────────────────────│                        │
         │                        │                        │
         │                        │        ExecutionLogs   │
         │                        │<───────────────────────│ (streaming)
         │                        │                        │
         │  /api/events/.../logs  │                        │
         │<───────────────────────│                        │
         │                        │                        │
         │                        │     ExecutionCompleted │
         │                        │<───────────────────────│
         │                        │                        │
         │  Update task status    │                        │
         │<───────────────────────│                        │
         │                        │                        │
         │                        │ ComputeContribution    │
         │                        │<───────────────────────│
         │                        │                        │
         │                        │ VibeTransaction        │
         │                        │───────────────────────>│
         │                        │                        │
```

## Mesh Panel: Transaction Log Display

The Mesh Panel should display meaningful transaction logs, not raw messages:

### Transaction Log Types

```typescript
interface TransactionLog {
  id: string;
  timestamp: string;
  type: TransactionType;
  details: TransactionDetails;
}

type TransactionType =
  | 'task_distributed'      // Task sent to remote node
  | 'task_received'         // Task received from network
  | 'execution_started'     // Execution began
  | 'execution_completed'   // Execution finished
  | 'execution_failed'      // Execution error
  | 'bandwidth_contributed' // Relayed data for network
  | 'compute_contributed'   // Processed task for network
  | 'vibe_earned'          // Credits received
  | 'vibe_spent'           // Credits paid

interface TaskDistributedLog {
  type: 'task_distributed';
  task_id: string;
  task_title: string;
  target_node: string;
  reward_vibe: number;
}

interface ExecutionCompletedLog {
  type: 'execution_completed';
  task_id: string;
  task_title: string;
  executor_node: string;
  execution_time_ms: number;
  files_modified: number;
  vibe_paid: number;
}

interface BandwidthContributedLog {
  type: 'bandwidth_contributed';
  bytes: number;
  purpose: string;
  peer_node: string;
  vibe_earned: number;
}
```

### Mesh Panel UI Sections

```
┌─────────────────────────────────────────────────────────────┐
│  Alpha Protocol Network                        [● ONLINE]   │
├─────────────────────────────────────────────────────────────┤
│  Node: apn_9c47c2fb    Peers: 3    Uptime: 2h 34m          │
│  Relay: Connected      Vibe Balance: 1,247.50               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Resource Contribution                                      │
├─────────────────────────────────────────────────────────────┤
│  Bandwidth: ████████░░ 12.5 / 100 Mbps                     │
│  ↑ Contributing: 8.2 Mbps    ↓ Consuming: 4.3 Mbps         │
│                                                             │
│  Compute: ████░░░░░░ 2 / 8 tasks                           │
│  Active: 2 tasks    Completed today: 14                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Transaction Log                                    [Live]  │
├─────────────────────────────────────────────────────────────┤
│  14:32:15  ✓ Task completed on apn_09465b95                │
│            "Fix authentication bug" - 3 files modified      │
│            Paid: 15.00 VIBE                                 │
│                                                             │
│  14:28:41  → Task distributed to apn_09465b95              │
│            "Fix authentication bug" - Reward: 15.00 VIBE   │
│                                                             │
│  14:15:22  ↑ Bandwidth contributed                         │
│            Relayed 52MB for apn_a1b2c3d4                    │
│            Earned: 0.52 VIBE                                │
│                                                             │
│  14:02:08  ◆ Execution started locally                     │
│            "Update API endpoints" from apn_e5f6g7h8        │
│            Earning: ~25.00 VIBE                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Connected Peers (3)                                        │
├─────────────────────────────────────────────────────────────┤
│  ● apn_09465b95  [compute, relay]     45ms    100 Mbps     │
│  ● apn_a1b2c3d4  [compute, storage]   120ms   50 Mbps      │
│  ● apn_e5f6g7h8  [relay]              85ms    75 Mbps      │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Foundation (Current)
- [x] APN core networking (libp2p + NATS relay)
- [x] Node identity and encryption
- [x] Basic Mesh Panel with system resources
- [ ] Remove raw message display

### Phase 2: Task Distribution Bridge
- [ ] Create `APNBridge` service in `crates/services/`
- [ ] Implement `TaskDistributor` to route tasks
- [ ] Add task distribution messages to mesh
- [ ] Update Mesh Panel with transaction log

### Phase 3: Remote Execution
- [ ] Implement `ExecutionRelay` for log streaming
- [ ] Add progress tracking over mesh
- [ ] Handle git diff transfer
- [ ] Update Dashboard to show remote execution

### Phase 4: Resource Accounting
- [ ] Track bandwidth contribution
- [ ] Track compute contribution
- [ ] Implement `ResourceAccounting` service
- [ ] Add metrics to Mesh Panel

### Phase 5: Economic Layer
- [ ] Design Vibe token integration
- [ ] Implement `SettlementEngine`
- [ ] Add transaction history
- [ ] Implement balance tracking

## File Structure

```
crates/
├── alpha-protocol-core/        # Network layer (existing)
│   ├── src/
│   │   ├── mesh.rs            # P2P networking
│   │   ├── relay.rs           # NATS relay
│   │   └── messages.rs        # APNMessage types (new)
│
├── services/
│   └── src/
│       └── apn_bridge/        # Integration layer (new)
│           ├── mod.rs
│           ├── task_distributor.rs
│           ├── execution_relay.rs
│           ├── resource_accounting.rs
│           └── settlement.rs
│
└── nora/
    └── src/
        └── mesh_integration.rs # Nora ↔ APN bridge (new)

frontend/
└── src/
    └── components/
        └── mesh/
            ├── MeshPanel.tsx       # Main panel
            ├── TransactionLog.tsx  # Transaction display (new)
            ├── ResourceStats.tsx   # Resource contribution (new)
            └── PeerList.tsx        # Connected peers (new)
```

## Configuration

```toml
# config/apn.toml

[node]
port = 4001
capabilities = ["compute", "relay", "storage"]

[resources]
max_bandwidth_mbps = 100
max_concurrent_tasks = 4
storage_gb = 50

[economics]
min_task_reward = 1.0
bandwidth_rate_per_gb = 0.01
compute_rate_per_cpu_hour = 0.50

[relay]
url = "nats://nonlocal.info:4222"
auto_reconnect = true
```

## Security Considerations

1. **Task Verification**: Tasks must be signed by the originating node
2. **Result Verification**: Execution results should include proof of work
3. **Economic Security**: Escrow system for task rewards
4. **Privacy**: Task content encrypted end-to-end
5. **Reputation**: Track node reliability for task routing

## Next Steps

1. Review and approve this architecture
2. Start Phase 2: Task Distribution Bridge
3. Update Mesh Panel to show transaction logs instead of raw messages
