#!/usr/bin/env python3
"""
Device Registry Manager

Tracks all devices in the sovereign network:
- Always-on servers (Bonomotion's studio)
- Mobile nodes (Sirak's laptop)
- Storage providers (Pythia)
"""

import sqlite3
import json
import uuid
from datetime import datetime, timedelta
from typing import Optional, List, Dict
import asyncio

try:
    from nats.aio.client import Client as NATS
except ImportError:
    print("Warning: nats-py not installed")


class DeviceRegistry:
    """Manages device registration and heartbeats"""

    def __init__(self, db_path: str, nats_url: str = "nats://nonlocal.info:4222"):
        self.db_path = db_path
        self.nats_url = nats_url
        self.nc = None

    async def connect(self):
        """Connect to NATS"""
        if not self.nc:
            self.nc = NATS()
            await self.nc.connect(self.nats_url)

    def register_device(
        self,
        owner_id: bytes,
        device_name: str,
        device_type: str,
        apn_node_id: str,
        public_key: str,
        storage_capacity_gb: int = 0,
        serves_data: bool = False,
        accepts_storage_contracts: bool = False
    ) -> str:
        """Register a new device"""

        device_id = str(uuid.uuid4())

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute("""
            INSERT INTO device_registry (
                id, owner_id, device_name, device_type, apn_node_id, public_key,
                storage_capacity_gb, serves_data, accepts_storage_contracts,
                is_online, last_seen
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            device_id,
            owner_id,
            device_name,
            device_type,
            apn_node_id,
            public_key,
            storage_capacity_gb,
            serves_data,
            accepts_storage_contracts,
            True,  # is_online
            datetime.utcnow().isoformat()
        ))

        conn.commit()
        conn.close()

        return device_id

    def update_heartbeat(self, device_id: str):
        """Update device heartbeat (marks as online)"""

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute("""
            UPDATE device_registry
            SET is_online = TRUE,
                last_heartbeat = ?,
                last_seen = ?
            WHERE id = ?
        """, (
            datetime.utcnow().isoformat(),
            datetime.utcnow().isoformat(),
            device_id
        ))

        conn.commit()
        conn.close()

    def mark_offline(self, device_id: str):
        """Mark device as offline"""

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute("""
            UPDATE device_registry
            SET is_online = FALSE
            WHERE id = ?
        """, (device_id,))

        conn.commit()
        conn.close()

    def get_device(self, device_id: str) -> Optional[Dict]:
        """Get device information"""

        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        cursor.execute("""
            SELECT * FROM device_registry WHERE id = ?
        """, (device_id,))

        row = cursor.fetchone()
        conn.close()

        if row:
            return dict(row)
        return None

    def get_user_devices(self, owner_id: bytes) -> List[Dict]:
        """Get all devices for a user"""

        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        cursor.execute("""
            SELECT * FROM device_registry
            WHERE owner_id = ?
            ORDER BY last_seen DESC
        """, (owner_id,))

        rows = cursor.fetchall()
        conn.close()

        return [dict(row) for row in rows]

    def get_online_devices(self) -> List[Dict]:
        """Get all online devices"""

        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        cursor.execute("""
            SELECT * FROM device_registry
            WHERE is_online = TRUE
            ORDER BY last_heartbeat DESC
        """)

        rows = cursor.fetchall()
        conn.close()

        return [dict(row) for row in rows]

    def get_storage_providers(self) -> List[Dict]:
        """Get devices that accept storage contracts"""

        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        cursor.execute("""
            SELECT * FROM device_registry
            WHERE accepts_storage_contracts = TRUE
            AND is_online = TRUE
            ORDER BY storage_capacity_gb DESC
        """)

        rows = cursor.fetchall()
        conn.close()

        return [dict(row) for row in rows]

    def check_stale_devices(self, timeout_minutes: int = 5):
        """Mark devices as offline if haven't seen heartbeat"""

        cutoff = datetime.utcnow() - timedelta(minutes=timeout_minutes)

        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        cursor.execute("""
            UPDATE device_registry
            SET is_online = FALSE
            WHERE last_heartbeat < ?
            AND is_online = TRUE
        """, (cutoff.isoformat(),))

        affected = cursor.rowcount
        conn.commit()
        conn.close()

        return affected

    async def heartbeat_listener(self):
        """Listen for device heartbeats over APN"""

        await self.connect()

        async def heartbeat_handler(msg):
            data = json.loads(msg.data.decode())
            device_id = data.get("device_id")

            if device_id:
                self.update_heartbeat(device_id)

        await self.nc.subscribe("apn.device.heartbeat", cb=heartbeat_handler)

        print("📡 Listening for device heartbeats...")

        # Periodic stale check
        while True:
            await asyncio.sleep(60)  # Check every minute
            stale_count = self.check_stale_devices()
            if stale_count > 0:
                print(f"⚠️  Marked {stale_count} devices as offline (stale heartbeat)")

    async def publish_heartbeat(self, device_id: str):
        """Publish heartbeat for a device"""

        await self.connect()

        await self.nc.publish(
            "apn.device.heartbeat",
            json.dumps({
                "device_id": device_id,
                "timestamp": datetime.utcnow().isoformat()
            }).encode()
        )


# CLI for device registration
if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print("Usage:")
        print("  Register device:")
        print("    python device_registry.py register <owner_username> <device_name> <device_type> <apn_node_id>")
        print("  List devices:")
        print("    python device_registry.py list <owner_username>")
        sys.exit(1)

    DB_PATH = "/home/pythia/.local/share/duck-kanban/db.sqlite"

    registry = DeviceRegistry(DB_PATH)

    command = sys.argv[1]

    if command == "register":
        owner_username = sys.argv[2]
        device_name = sys.argv[3]
        device_type = sys.argv[4]
        apn_node_id = sys.argv[5]

        # Get owner ID
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        cursor.execute("SELECT id FROM users WHERE username = ?", (owner_username,))
        owner_id = cursor.fetchone()[0]
        conn.close()

        # Generate dummy public key (in production, use actual key)
        public_key = f"pubkey_{uuid.uuid4().hex[:16]}"

        # Set capabilities based on device type
        storage_capacity = 100 if device_type == "storage_provider" else 0
        serves_data = device_type in ["always_on", "storage_provider"]
        accepts_contracts = device_type == "storage_provider"

        device_id = registry.register_device(
            owner_id=owner_id,
            device_name=device_name,
            device_type=device_type,
            apn_node_id=apn_node_id,
            public_key=public_key,
            storage_capacity_gb=storage_capacity,
            serves_data=serves_data,
            accepts_storage_contracts=accepts_contracts
        )

        print(f"✅ Device registered: {device_id}")
        print(f"   Owner: {owner_username}")
        print(f"   Name: {device_name}")
        print(f"   Type: {device_type}")

    elif command == "list":
        owner_username = sys.argv[2]

        # Get owner ID
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()
        cursor.execute("SELECT id FROM users WHERE username = ?", (owner_username,))
        owner_id = cursor.fetchone()[0]
        conn.close()

        devices = registry.get_user_devices(owner_id)

        print(f"\nDevices for {owner_username}:")
        print("=" * 70)
        for device in devices:
            status = "🟢 Online" if device["is_online"] else "🔴 Offline"
            print(f"{status} {device['device_name']}")
            print(f"   ID: {device['id']}")
            print(f"   Type: {device['device_type']}")
            print(f"   Last seen: {device['last_seen']}")
            print()
