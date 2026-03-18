#!/usr/bin/env python3
"""
APN Bridge Server - HTTP API bridge to Alpha Protocol Network

Provides REST API endpoints for the PCG Dashboard to interact with the APN mesh network.
Connects to NATS relay for peer discovery and message routing.
"""

import asyncio
import json
import os
import subprocess
from datetime import datetime, timezone
from typing import Optional, List, Dict, Any
from contextlib import asynccontextmanager
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

# Try to import nats, gracefully handle if not installed
try:
    import nats
    from nats.aio.client import Client as NATS
    NATS_AVAILABLE = True
except ImportError:
    NATS_AVAILABLE = False
    print("⚠️  NATS library not installed. Running in mock mode.")
    print("   Install with: pip install nats-py")

# ============================================================================
# Configuration from Environment
# ============================================================================

# Node configuration
NODE_ID = os.getenv("NODE_ID", "apn_dashboard")
DEVICE_NAME = os.getenv("DEVICE_NAME", "APN Node")
WALLET_ADDRESS = os.getenv("WALLET_ADDRESS", "0x09465b9572fb354fdf4e34040386f180d1ff0c2a3a668333bedee17b266a4b74")
P2P_PORT = int(os.getenv("P2P_PORT", "4001"))

# NATS Relay configuration
RELAY_URL = os.getenv("RELAY_URL", "nats://nonlocal.info:4222")

# Master nodes for bootstrap (comma-separated list)
# Format: node_id:wallet_address or just node_id
MASTER_NODES = os.getenv("MASTER_NODES", "").split(",") if os.getenv("MASTER_NODES") else []

# Node capabilities
CAPABILITIES = os.getenv("CAPABILITIES", "relay,dashboard").split(",")

# ============================================================================
# Global State
# ============================================================================

# NATS client
nats_client: Optional[Any] = None
nats_connected = False

# Discovered peers from the mesh
discovered_peers: Dict[str, Dict[str, Any]] = {}

# Message statistics
stats = {
    "messages_sent": 0,
    "messages_received": 0,
    "bytes_sent": 0,
    "bytes_received": 0,
}

# Store node information
NODE_INFO = {
    "node_id": NODE_ID,
    "wallet_address": WALLET_ADDRESS,
    "p2p_port": P2P_PORT,
    "relay_url": RELAY_URL,
    "master_nodes": MASTER_NODES,
    "capabilities": CAPABILITIES,
    "status": "initializing",
    "started_at": datetime.now(timezone.utc).isoformat()
}

# ============================================================================
# NATS Connection Management
# ============================================================================

async def connect_to_relay():
    """Connect to the NATS relay server."""
    global nats_client, nats_connected, NODE_INFO

    if not NATS_AVAILABLE:
        print("⚠️  NATS not available, running in mock mode")
        NODE_INFO["status"] = "mock_mode"
        return False

    try:
        nats_client = NATS()
        await nats_client.connect(RELAY_URL)
        nats_connected = True
        NODE_INFO["status"] = "connected"
        print(f"✅ Connected to NATS relay: {RELAY_URL}")

        # Subscribe to APN topics
        await subscribe_to_topics()

        # Announce ourselves
        await announce_node()

        return True
    except Exception as e:
        print(f"❌ Failed to connect to NATS relay: {e}")
        NODE_INFO["status"] = "relay_error"
        nats_connected = False
        return False

async def subscribe_to_topics():
    """Subscribe to APN gossip topics."""
    if not nats_client or not nats_connected:
        return

    topics = [
        "apn.discovery",      # Peer discovery announcements
        "apn.peers",          # Peer status updates
        "apn.heartbeat",      # Node heartbeats
        "apn.tasks",          # Task announcements
        f"apn.dm.{NODE_ID}",  # Direct messages to this node
    ]

    for topic in topics:
        try:
            await nats_client.subscribe(topic, cb=handle_message)
            print(f"📡 Subscribed to: {topic}")
        except Exception as e:
            print(f"⚠️  Failed to subscribe to {topic}: {e}")

async def handle_message(msg):
    """Handle incoming NATS messages."""
    global stats, discovered_peers

    stats["messages_received"] += 1
    stats["bytes_received"] += len(msg.data)

    try:
        data = json.loads(msg.data.decode())
        subject = msg.subject

        # Handle peer announcements
        if subject == "apn.discovery" or subject == "apn.peers":
            if "node_id" in data:
                peer_id = data["node_id"]
                discovered_peers[peer_id] = {
                    "peer_id": peer_id,
                    "device_name": data.get("device_name", data.get("hostname", "")),
                    "wallet_address": data.get("wallet_address", ""),
                    "capabilities": data.get("capabilities", []),
                    "resources": data.get("resources"),
                    "last_seen": datetime.now(timezone.utc).isoformat(),
                    "source": subject,
                }
                name = data.get("device_name", data.get("hostname", peer_id))
                print(f"👋 Discovered peer: {name} ({peer_id})")

        # Handle heartbeats
        elif subject == "apn.heartbeat":
            peer_id = data.get("node_id")
            if peer_id:
                if peer_id in discovered_peers:
                    discovered_peers[peer_id]["last_seen"] = datetime.now(timezone.utc).isoformat()
                    if "resources" in data:
                        discovered_peers[peer_id]["resources"] = data["resources"]
                    if data.get("hostname"):
                        discovered_peers[peer_id]["device_name"] = data["hostname"]
                else:
                    discovered_peers[peer_id] = {
                        "peer_id": peer_id,
                        "device_name": data.get("hostname", ""),
                        "wallet_address": data.get("wallet_address", ""),
                        "capabilities": data.get("capabilities", []),
                        "resources": data.get("resources"),
                        "last_seen": datetime.now(timezone.utc).isoformat(),
                        "source": "apn.heartbeat",
                    }
                    name = data.get("hostname", peer_id)
                    print(f"👋 Discovered peer via heartbeat: {name} ({peer_id})")

        # Handle direct messages
        elif subject.startswith("apn.dm."):
            print(f"📨 Direct message from {data.get('from', 'unknown')}: {data.get('content', '')[:100]}")

    except json.JSONDecodeError:
        print(f"⚠️  Invalid JSON on {msg.subject}")
    except Exception as e:
        print(f"⚠️  Error handling message: {e}")

async def announce_node():
    """Announce this node to the network."""
    if not nats_client or not nats_connected:
        return

    announcement = {
        "node_id": NODE_ID,
        "device_name": DEVICE_NAME,
        "hostname": DEVICE_NAME,
        "wallet_address": WALLET_ADDRESS,
        "capabilities": CAPABILITIES,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "version": "0.1.0",
    }

    try:
        await nats_client.publish("apn.discovery", json.dumps(announcement).encode())
        stats["messages_sent"] += 1
        print(f"📢 Announced node to network: {NODE_ID}")
    except Exception as e:
        print(f"⚠️  Failed to announce: {e}")

async def sync_with_master_nodes():
    """Sync state with configured master nodes."""
    if not MASTER_NODES or not nats_client or not nats_connected:
        return {"synced": 0, "failed": 0}

    synced = 0
    failed = 0

    for master in MASTER_NODES:
        if not master.strip():
            continue

        master_id = master.split(":")[0].strip()
        try:
            # Request sync from master node
            sync_request = {
                "type": "sync_request",
                "from": NODE_ID,
                "timestamp": datetime.now(timezone.utc).isoformat(),
            }
            await nats_client.publish(f"apn.dm.{master_id}", json.dumps(sync_request).encode())
            stats["messages_sent"] += 1
            synced += 1
            print(f"🔄 Sync request sent to master: {master_id}")
        except Exception as e:
            print(f"⚠️  Failed to sync with {master_id}: {e}")
            failed += 1

    return {"synced": synced, "failed": failed}

# ============================================================================
# FastAPI Lifespan
# ============================================================================

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Manage application lifecycle."""
    # Startup
    print("🚀 Starting APN Bridge Server")
    await connect_to_relay()

    # Start heartbeat task
    heartbeat_task = asyncio.create_task(heartbeat_loop())

    yield

    # Shutdown
    heartbeat_task.cancel()
    if nats_client and nats_connected:
        await nats_client.close()
    print("👋 APN Bridge Server shutting down")

async def heartbeat_loop():
    """Send periodic heartbeats to the network."""
    while True:
        try:
            await asyncio.sleep(30)  # Every 30 seconds
            if nats_client and nats_connected:
                heartbeat = {
                    "node_id": NODE_ID,
                    "timestamp": datetime.now(timezone.utc).isoformat(),
                    "peers": len(discovered_peers),
                }
                await nats_client.publish("apn.heartbeat", json.dumps(heartbeat).encode())
                stats["messages_sent"] += 1
        except asyncio.CancelledError:
            break
        except Exception as e:
            print(f"⚠️  Heartbeat error: {e}")

app = FastAPI(
    title="APN Bridge Server",
    description="HTTP API bridge to Alpha Protocol Network",
    version="0.2.0",
    lifespan=lifespan
)

# Add CORS middleware to allow dashboard access
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # In production, specify actual origins
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Request/Response models
class MessageRequest(BaseModel):
    recipient: str
    content: str
    message_type: str = "text"

class PeerInfo(BaseModel):
    peer_id: str
    wallet_address: Optional[str] = None
    capabilities: List[str] = []
    last_seen: str

class NetworkStats(BaseModel):
    total_peers: int
    active_connections: int
    messages_sent: int
    messages_received: int
    uptime_seconds: float

# ============================================================================
# Health & Status Endpoints
# ============================================================================

@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "service": "apn-bridge",
        "timestamp": datetime.now(timezone.utc).isoformat()
    }

@app.get("/")
async def root():
    """Root endpoint with service info"""
    return {
        "service": "APN Bridge Server",
        "version": "0.1.0",
        "docs": "/docs",
        "health": "/health"
    }

@app.post("/register")
async def register_node(node_data: dict):
    """Register a new node in the mesh network"""
    return {
        "success": True,
        "message": "Node registration received",
        "node_id": node_data.get("node_id", "unknown")
    }

# ============================================================================
# Node Operations
# ============================================================================

@app.get("/api/node/status")
async def get_node_status():
    """Get current node status and information"""
    uptime = (datetime.now(timezone.utc) - datetime.fromisoformat(NODE_INFO["started_at"])).total_seconds()
    return {
        "success": True,
        "data": {
            **NODE_INFO,
            "node_uptime_seconds": uptime,
            "nats_available": NATS_AVAILABLE,
            "connections": {
                "relay": "connected" if nats_connected else "disconnected",
                "peers": len(discovered_peers)
            },
            "stats": stats,
        }
    }

@app.post("/api/node/start")
async def start_node():
    """Start/connect the APN node"""
    if nats_connected:
        return {
            "success": True,
            "message": "Node already connected"
        }

    success = await connect_to_relay()
    return {
        "success": success,
        "message": "Node connected" if success else "Failed to connect",
        "status": NODE_INFO["status"]
    }

@app.post("/api/node/stop")
async def stop_node():
    """Stop/disconnect the APN node"""
    global nats_connected

    if nats_client and nats_connected:
        try:
            await nats_client.close()
            nats_connected = False
            NODE_INFO["status"] = "disconnected"
        except Exception as e:
            return {
                "success": False,
                "message": f"Error disconnecting: {e}"
            }

    return {
        "success": True,
        "message": "Node disconnected",
        "status": NODE_INFO["status"]
    }

@app.post("/api/node/restart")
async def restart_node():
    """Restart the APN node connection"""
    # Stop first
    await stop_node()

    # Then start
    success = await connect_to_relay()
    return {
        "success": success,
        "message": "Node restarted" if success else "Failed to restart",
        "status": NODE_INFO["status"]
    }

# ============================================================================
# Peer Discovery & Management
# ============================================================================

@app.get("/api/peers")
async def list_peers():
    """List all discovered peers in the mesh network"""
    peers = list(discovered_peers.values())
    return {
        "success": True,
        "data": {
            "peers": peers,
            "total": len(peers),
            "relay_connected": nats_connected,
        }
    }

@app.get("/api/peers/{peer_id}")
async def get_peer_info(peer_id: str):
    """Get detailed information about a specific peer"""
    return {
        "success": True,
        "data": {
            "peer_id": peer_id,
            "wallet_address": f"0x{peer_id}...",
            "capabilities": ["relay"],
            "last_seen": datetime.now(timezone.utc).isoformat(),
            "connection_quality": "good"
        }
    }

# ============================================================================
# Mesh Network Operations
# ============================================================================

@app.get("/api/mesh/topology")
async def get_mesh_topology():
    """Get the current mesh network topology"""
    peer_count = len(discovered_peers)
    return {
        "success": True,
        "data": {
            "nodes": peer_count + 1,  # Include self
            "connections": peer_count,
            "topology_type": "mesh",
            "self": {
                "node_id": NODE_ID,
                "wallet_address": WALLET_ADDRESS,
                "capabilities": CAPABILITIES,
            },
            "peers": list(discovered_peers.keys()),
        }
    }

@app.post("/api/mesh/discover")
async def trigger_discovery():
    """Trigger peer discovery process"""
    if not nats_connected:
        return {
            "success": False,
            "message": "Not connected to relay"
        }

    # Re-announce ourselves to trigger peer responses
    await announce_node()

    return {
        "success": True,
        "message": "Peer discovery initiated",
        "current_peers": len(discovered_peers)
    }

@app.get("/api/mesh/stats")
async def get_mesh_stats():
    """Get mesh network statistics (alias for /api/network/stats)"""
    uptime = (datetime.now(timezone.utc) - datetime.fromisoformat(NODE_INFO["started_at"])).total_seconds()
    peer_count = len(discovered_peers)
    health = "healthy" if nats_connected and peer_count > 0 else ("degraded" if nats_connected else "disconnected")

    return {
        "success": True,
        "data": {
            "total_nodes": peer_count + 1,  # Include self
            "total_peers": peer_count,
            "active_connections": 1 if nats_connected else 0,
            "messages_sent": stats["messages_sent"],
            "messages_received": stats["messages_received"],
            "uptime_seconds": uptime,
            "network_health": health,
            "relay_connected": nats_connected,
            "relay_url": RELAY_URL,
            "master_nodes": MASTER_NODES,
            "topology": {
                "nodes": peer_count + 1,
                "connections": peer_count,
                "type": "mesh"
            }
        }
    }

# ============================================================================
# Message Operations
# ============================================================================

@app.post("/api/messages/send")
async def send_message(message: MessageRequest):
    """Send a message to a peer in the mesh network"""
    return {
        "success": True,
        "message_id": f"msg_{datetime.now(timezone.utc).timestamp()}",
        "recipient": message.recipient,
        "status": "sent"
    }

@app.get("/api/messages/inbox")
async def get_inbox():
    """Get received messages"""
    return {
        "success": True,
        "data": {
            "messages": [],
            "total": 0
        }
    }

@app.get("/api/messages/outbox")
async def get_outbox():
    """Get sent messages"""
    return {
        "success": True,
        "data": {
            "messages": [],
            "total": 0
        }
    }

# ============================================================================
# Network Statistics
# ============================================================================

@app.get("/api/network/stats")
async def get_network_stats():
    """Get mesh network statistics"""
    uptime = (datetime.now(timezone.utc) - datetime.fromisoformat(NODE_INFO["started_at"])).total_seconds()
    peer_count = len(discovered_peers)
    health = "healthy" if nats_connected and peer_count > 0 else ("degraded" if nats_connected else "disconnected")

    return {
        "success": True,
        "data": {
            "total_peers": peer_count,
            "active_connections": 1 if nats_connected else 0,
            "messages_sent": stats["messages_sent"],
            "messages_received": stats["messages_received"],
            "uptime_seconds": uptime,
            "network_health": health,
            "relay_connected": nats_connected,
        }
    }

@app.get("/api/network/bandwidth")
async def get_bandwidth_stats():
    """Get network bandwidth statistics"""
    uptime = (datetime.now(timezone.utc) - datetime.fromisoformat(NODE_INFO["started_at"])).total_seconds()
    mps = stats["messages_sent"] / uptime if uptime > 0 else 0

    return {
        "success": True,
        "data": {
            "bytes_sent": stats["bytes_sent"],
            "bytes_received": stats["bytes_received"],
            "messages_per_second": round(mps, 3)
        }
    }

# ============================================================================
# Master Node Sync
# ============================================================================

@app.post("/api/sync/master")
async def sync_with_masters():
    """Trigger sync with configured master nodes"""
    if not MASTER_NODES:
        return {
            "success": False,
            "error": "No master nodes configured. Set MASTER_NODES environment variable."
        }

    if not nats_connected:
        return {
            "success": False,
            "error": "Not connected to relay. Cannot sync with master nodes."
        }

    result = await sync_with_master_nodes()
    return {
        "success": result["synced"] > 0,
        "data": {
            "master_nodes": MASTER_NODES,
            "synced": result["synced"],
            "failed": result["failed"],
        }
    }

@app.get("/api/sync/status")
async def get_sync_status():
    """Get current sync status with master nodes"""
    return {
        "success": True,
        "data": {
            "relay_connected": nats_connected,
            "relay_url": RELAY_URL,
            "master_nodes": MASTER_NODES,
            "master_nodes_configured": len([m for m in MASTER_NODES if m.strip()]),
            "discovered_peers": len(discovered_peers),
            "node_id": NODE_ID,
            "status": NODE_INFO["status"],
        }
    }

@app.post("/api/sync/reconnect")
async def reconnect_relay():
    """Reconnect to the NATS relay"""
    global nats_connected

    if nats_connected and nats_client:
        try:
            await nats_client.close()
        except:
            pass

    nats_connected = False
    success = await connect_to_relay()

    return {
        "success": success,
        "data": {
            "relay_url": RELAY_URL,
            "connected": nats_connected,
            "status": NODE_INFO["status"],
        }
    }

# ============================================================================
# Task Distribution
# ============================================================================

@app.post("/api/tasks/submit")
async def submit_task(task_data: dict):
    """Submit a task for distributed processing"""
    return {
        "success": True,
        "task_id": f"task_{datetime.now(timezone.utc).timestamp()}",
        "status": "queued"
    }

@app.get("/api/tasks/{task_id}")
async def get_task_status(task_id: str):
    """Get status of a submitted task"""
    return {
        "success": True,
        "data": {
            "task_id": task_id,
            "status": "completed",
            "result": None
        }
    }

# ============================================================================
# Secure Operations
# ============================================================================

@app.post("/api/secure/encrypt")
async def encrypt_data(data: dict):
    """Encrypt data for secure transmission"""
    return {
        "success": True,
        "encrypted": "base64_encrypted_data_here"
    }

@app.post("/api/secure/decrypt")
async def decrypt_data(data: dict):
    """Decrypt received encrypted data"""
    return {
        "success": True,
        "decrypted": "decrypted_data_here"
    }

# ============================================================================
# Main Entry Point
# ============================================================================

if __name__ == "__main__":
    print("🚀 Starting APN Bridge Server")
    print(f"📡 Node ID: {NODE_INFO['node_id']}")
    print(f"🔗 Wallet: {NODE_INFO['wallet_address']}")
    print("🌐 Listening on http://0.0.0.0:8000")
    print("📚 API docs at http://localhost:8000/docs")

    uvicorn.run(
        app,
        host="0.0.0.0",
        port=8000,
        log_level="info"
    )
