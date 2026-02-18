#!/usr/bin/env python3
"""
Storage Provider Server

Runs on storage provider nodes (Pythia Master)
Receives encrypted databases from mobile nodes
Serves data when mobile nodes offline
Earns VIBE for service

VIBE Pricing (1 VIBE = $0.01 USD):
- Storage: 2 VIBE/GB/month ($0.02/GB)
- Transfer: 0.5 VIBE/GB ($0.005/GB)
- Uptime bonus: 1.5x for 99.9% availability
"""

import sqlite3
import json
import base64
import asyncio
import hashlib
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional

try:
    from nats.aio.client import Client as NATS
except ImportError:
    print("Error: nats-py not installed")
    print("Install with: pip install nats-py")
    exit(1)


class StorageProviderServer:
    """
    Provides storage service for mobile sovereign nodes
    Stores encrypted databases and serves them when owners offline
    """

    def __init__(
        self,
        device_id: str,
        db_path: str,
        storage_dir: str = "/home/pythia/.sovereign_storage",
        nats_url: str = "nats://nonlocal.info:4222"
    ):
        self.device_id = device_id
        self.db_path = db_path
        self.storage_dir = Path(storage_dir)
        self.storage_dir.mkdir(parents=True, exist_ok=True)
        self.nats_url = nats_url
        self.nc = None

        # In-memory index of stored databases
        self.replicas: Dict[str, Dict] = {}

    async def connect(self):
        """Connect to NATS"""
        if not self.nc:
            self.nc = NATS()
            await self.nc.connect(self.nats_url)
            print(f"✅ Connected to {self.nats_url}")

    async def handle_sync_request(self, msg):
        """Handle sync request from mobile node"""
        try:
            data = json.loads(msg.data.decode())

            # Log the message structure for debugging
            print(f"\n📨 Received message: {json.dumps(data, indent=2)}")

            # Check if this is the new metadata-only sync (v0.2.0+)
            if "from_device" in data and "projects" in data:
                print("\n✅ Metadata sync received (v0.2.0)")
                print(f"   From: {data.get('from_device')}")
                print(f"   DB Size: {data.get('db_size_bytes', 0):,} bytes")
                print(f"   Projects: {data.get('projects', 0)}")
                print(f"   Tasks: {data.get('tasks', 0)}")
                print(f"   Agents: {data.get('agents', 0)}")
                # TODO: Store metadata, not full database
                return

            # Handle old full-database sync format
            client_device_id = data.get("from") or data.get("device_id") or data.get("source_device_id")
            version = data.get("version", 0)
            checksum = data.get("checksum", "")
            size = data.get("size", 0)
            encrypted_size = data.get("encrypted_size", 0)
            encrypted_data = base64.b64decode(data.get("data", ""))

            print("\n" + "="*70)
            print("📥 Sync Request Received")
            print("="*70)
            print(f"From: {client_device_id}")
            print(f"Version: {version}")
            print(f"Size: {size:,} bytes ({size / 1024 / 1024:.2f} MB)")
            print(f"Encrypted: {encrypted_size:,} bytes")
            print(f"Checksum: {checksum[:16]}...")

            # Store encrypted database
            replica_path = self.storage_dir / f"{client_device_id}.db.encrypted"

            print(f"💾 Storing to: {replica_path}")

            with open(replica_path, 'wb') as f:
                f.write(encrypted_data)

            # Verify storage
            stored_checksum = hashlib.sha256(encrypted_data).hexdigest()

            print(f"✅ Stored successfully")
            print(f"   Stored checksum: {stored_checksum[:16]}...")

            # Update index
            self.replicas[client_device_id] = {
                "path": str(replica_path),
                "version": version,
                "checksum": checksum,
                "encrypted_checksum": stored_checksum,
                "size": size,
                "encrypted_size": encrypted_size,
                "last_updated": datetime.utcnow().isoformat()
            }

            # Record metrics in database
            self._record_storage_metrics(client_device_id, size, encrypted_size)

            # Send acknowledgment
            ack = {
                "success": True,
                "version": version,
                "checksum": stored_checksum,
                "timestamp": datetime.utcnow().isoformat()
            }

            await self.nc.publish(
                f"apn.storage.ack.{client_device_id}",
                json.dumps(ack).encode()
            )

            print(f"✅ Acknowledgment sent to {client_device_id}")

        except Exception as e:
            print(f"❌ Error handling sync request: {e}")
            import traceback
            traceback.print_exc()

    def _record_storage_metrics(self, client_device_id: str, size: int, encrypted_size: int):
        """Record storage metrics for billing"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # Get or create contract
        cursor.execute("""
            SELECT id FROM storage_contracts
            WHERE provider_device_id = ?
            AND status = 'active'
            LIMIT 1
        """, (self.device_id,))

        contract = cursor.fetchone()

        if contract:
            contract_id = contract[0]

            # Update contract usage
            size_gb = encrypted_size / 1024 / 1024 / 1024

            cursor.execute("""
                UPDATE storage_contracts
                SET actual_storage_used_gb = ?
                WHERE id = ?
            """, (size_gb, contract_id))

            conn.commit()

        conn.close()

    async def serve_data_request(self, msg):
        """Serve data when client device is offline"""
        try:
            data = json.loads(msg.data.decode())

            client_device_id = data["device_id"]
            requester = data["requester"]

            print(f"\n📤 Data request from {requester} for {client_device_id}")

            # Check if we have this replica
            if client_device_id not in self.replicas:
                print(f"❌ No replica available for {client_device_id}")
                await self.nc.publish(
                    f"apn.storage.response.{requester}",
                    json.dumps({"error": "No replica available"}).encode()
                )
                return

            # Get replica info
            replica_info = self.replicas[client_device_id]
            replica_path = Path(replica_info["path"])

            # Read encrypted database
            with open(replica_path, 'rb') as f:
                encrypted_data = f.read()

            # Send encrypted data (client will decrypt)
            response = {
                "success": True,
                "device_id": client_device_id,
                "version": replica_info["version"],
                "data": base64.b64encode(encrypted_data).decode(),
                "checksum": replica_info["encrypted_checksum"],
                "source": "storage_provider",
                "provider_id": self.device_id
            }

            await self.nc.publish(
                f"apn.storage.response.{requester}",
                json.dumps(response).encode()
            )

            print(f"✅ Sent {replica_info['encrypted_size']:,} bytes to {requester}")

            # Record transfer for billing
            self._record_transfer(client_device_id, replica_info["encrypted_size"])

        except Exception as e:
            print(f"❌ Error serving data: {e}")
            import traceback
            traceback.print_exc()

    def _record_transfer(self, client_device_id: str, bytes_transferred: int):
        """Record data transfer for billing"""
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()

        # Get contract
        cursor.execute("""
            SELECT id FROM storage_contracts
            WHERE provider_device_id = ?
            AND status = 'active'
            LIMIT 1
        """, (self.device_id,))

        contract = cursor.fetchone()

        if contract:
            contract_id = contract[0]
            gb_transferred = bytes_transferred / 1024 / 1024 / 1024

            # Update contract
            cursor.execute("""
                UPDATE storage_contracts
                SET total_data_transferred_gb = total_data_transferred_gb + ?
                WHERE id = ?
            """, (gb_transferred, contract_id))

            conn.commit()

        conn.close()

    def get_storage_stats(self) -> Dict:
        """Get storage provider statistics"""
        total_clients = len(self.replicas)
        total_size = sum(r["encrypted_size"] for r in self.replicas.values())

        conn = sqlite3.connect(self.db_path)
        conn.row_factory = sqlite3.Row
        cursor = conn.cursor()

        # Get active contracts
        cursor.execute("""
            SELECT
                COUNT(*) as active_contracts,
                SUM(actual_storage_used_gb) as total_storage_gb,
                SUM(total_data_transferred_gb) as total_transferred_gb,
                SUM(monthly_rate_vibe) as monthly_revenue_vibe
            FROM storage_contracts
            WHERE provider_device_id = ?
            AND status = 'active'
        """, (self.device_id,))

        stats = dict(cursor.fetchone())
        conn.close()

        return {
            "replicas_count": total_clients,
            "total_storage_bytes": total_size,
            "total_storage_gb": total_size / 1024 / 1024 / 1024,
            "active_contracts": stats.get("active_contracts", 0) or 0,
            "contracted_storage_gb": stats.get("total_storage_gb", 0) or 0,
            "total_transferred_gb": stats.get("total_transferred_gb", 0) or 0,
            "monthly_revenue_vibe": stats.get("monthly_revenue_vibe", 0) or 0
        }

    async def run(self):
        """Main run loop"""
        await self.connect()

        print("\n" + "="*70)
        print("💾 Storage Provider Server Started")
        print("="*70)
        print(f"Device ID: {self.device_id}")
        print(f"Storage Directory: {self.storage_dir}")
        print(f"Database: {self.db_path}")
        print("="*70)

        # Subscribe to sync requests
        await self.nc.subscribe(
            f"apn.storage.sync.{self.device_id}",
            cb=self.handle_sync_request
        )
        print(f"📡 Subscribed to: apn.storage.sync.{self.device_id}")

        # Subscribe to data serve requests
        await self.nc.subscribe(
            f"apn.storage.serve.{self.device_id}",
            cb=self.serve_data_request
        )
        print(f"📡 Subscribed to: apn.storage.serve.{self.device_id}")

        print("\n🎧 Listening for storage requests...\n")

        # Keep running
        while True:
            await asyncio.sleep(60)

            # Print stats every minute
            stats = self.get_storage_stats()
            print(f"\n📊 Storage Stats:")
            print(f"   Replicas: {stats['replicas_count']}")
            print(f"   Storage: {stats['total_storage_gb']:.2f} GB")
            print(f"   Contracts: {stats['active_contracts']}")
            print(f"   Revenue: {stats['monthly_revenue_vibe']} VIBE/month")


# CLI
if __name__ == "__main__":
    import sys

    if len(sys.argv) < 2:
        print("Usage:")
        print("  python storage_provider_server.py <device_id>")
        print("\nExample:")
        print("  python storage_provider_server.py pythia_master")
        sys.exit(1)

    device_id = sys.argv[1]
    db_path = "/home/pythia/.local/share/duck-kanban/db.sqlite"

    async def main():
        server = StorageProviderServer(
            device_id=device_id,
            db_path=db_path
        )

        try:
            await server.run()
        except KeyboardInterrupt:
            print("\n\n👋 Shutting down...")
            await server.nc.close()

    asyncio.run(main())
