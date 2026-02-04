#!/usr/bin/env python3
"""
APN Bridge Server - HTTP API bridge to Alpha Protocol Network

Provides REST API endpoints for the PCG Dashboard to interact with the APN mesh network.
"""

import asyncio
import json
import subprocess
from datetime import datetime
from typing import Optional, List, Dict, Any
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
import uvicorn

app = FastAPI(
    title="APN Bridge Server",
    description="HTTP API bridge to Alpha Protocol Network",
    version="0.1.0"
)

# Add CORS middleware to allow dashboard access
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # In production, specify actual origins
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# Store node information
NODE_INFO = {
    "node_id": "apn_09465b95",
    "wallet_address": "0x09465b9572fb354fdf4e34040386f180d1ff0c2a3a668333bedee17b266a4b74",
    "p2p_port": 4001,
    "relay_url": "nats://nonlocal.info:4222",
    "status": "running",
    "started_at": datetime.utcnow().isoformat()
}

DISCOVERED_PEERS = []
MESSAGES = []

# Request/Response Models
class NodeStatus(BaseModel):
    node_id: str
    wallet_address: str
    p2p_port: int
    relay_url: str
    status: str
    started_at: str
    peer_count: int

class PeerInfo(BaseModel):
    node_id: str
    wallet_address: str
    capabilities: List[str]
    discovered_at: str

class SendMessageRequest(BaseModel):
    recipient: str
    message: str
    message_type: str = "direct_message"

class MessageInfo(BaseModel):
    from_node: str
    to_node: str
    message: str
    timestamp: str
    message_type: str

# API Endpoints
@app.get("/")
async def root():
    """Root endpoint - API info"""
    return {
        "service": "APN Bridge Server",
        "version": "0.1.0",
        "node_id": NODE_INFO["node_id"],
        "status": "online"
    }

@app.get("/health")
async def health_check():
    """Health check endpoint"""
    return {
        "status": "healthy",
        "timestamp": datetime.utcnow().isoformat()
    }

@app.get("/api/node/status", response_model=NodeStatus)
async def get_node_status():
    """Get current node status and information"""
    return NodeStatus(
        node_id=NODE_INFO["node_id"],
        wallet_address=NODE_INFO["wallet_address"],
        p2p_port=NODE_INFO["p2p_port"],
        relay_url=NODE_INFO["relay_url"],
        status=NODE_INFO["status"],
        started_at=NODE_INFO["started_at"],
        peer_count=len(DISCOVERED_PEERS)
    )

@app.get("/api/peers", response_model=List[PeerInfo])
async def get_peers():
    """List all discovered peers"""
    return DISCOVERED_PEERS

@app.post("/api/messages/send")
async def send_message(request: SendMessageRequest):
    """Send a message to another node in the mesh"""
    # In a full implementation, this would communicate with the apn_node binary
    # For now, return success
    message = MessageInfo(
        from_node=NODE_INFO["node_id"],
        to_node=request.recipient,
        message=request.message,
        timestamp=datetime.utcnow().isoformat(),
        message_type=request.message_type
    )
    MESSAGES.append(message)

    return {
        "success": True,
        "message_id": len(MESSAGES),
        "timestamp": message.timestamp
    }

@app.get("/api/messages", response_model=List[MessageInfo])
async def get_messages(limit: int = 50):
    """Get recent messages"""
    return MESSAGES[-limit:]

@app.get("/api/network/stats")
async def get_network_stats():
    """Get network statistics"""
    return {
        "total_peers": len(DISCOVERED_PEERS),
        "total_messages": len(MESSAGES),
        "node_uptime_seconds": (datetime.utcnow() - datetime.fromisoformat(NODE_INFO["started_at"])).total_seconds(),
        "connected_to_relay": True
    }

@app.post("/api/tasks/distribute")
async def distribute_task(task_data: Dict[str, Any]):
    """Distribute a task to the mesh network"""
    # Placeholder for task distribution
    return {
        "success": True,
        "task_id": task_data.get("task_id"),
        "status": "distributed",
        "timestamp": datetime.utcnow().isoformat()
    }

@app.get("/api/node/capabilities")
async def get_capabilities():
    """Get node capabilities"""
    return {
        "capabilities": ["compute", "relay", "storage"],
        "resources": {
            "cpu_cores": 24,
            "ram_mb": 0,
            "storage_gb": 0,
            "gpu_available": False
        }
    }

if __name__ == "__main__":
    print(f"🚀 Starting APN Bridge Server")
    print(f"📡 Node ID: {NODE_INFO['node_id']}")
    print(f"🔗 Wallet: {NODE_INFO['wallet_address']}")
    print(f"🌐 Listening on http://0.0.0.0:8000")
    print(f"📚 API docs at http://localhost:8000/docs")

    uvicorn.run(
        app,
        host="0.0.0.0",
        port=8000,
        log_level="info"
    )
