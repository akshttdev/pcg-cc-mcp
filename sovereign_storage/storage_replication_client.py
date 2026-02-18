#!/usr/bin/env python3
"""
Storage Replication Client

Runs on mobile sovereign nodes (Sirak's laptop)
Syncs local database to storage provider (Pythia)

Cost (1 VIBE = $0.01 USD):
- Storage: 2 VIBE/GB/month ($0.02/GB)
- Transfer: 0.5 VIBE/GB ($0.005/GB)
- Example: 5GB = ~10 VIBE/month ($0.10/month)
"""

import sqlite3
import json
import hashlib
import base64
import asyncio
from datetime import datetime
from pathlib import Path
from typing import Optional

try:
    from nats.aio.client import Client as NATS
    from cryptography.fernet import Fernet
    from cryptography.hazmat.primitives import hashes
    from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2
except ImportError:
    print("Error: Missing dependencies")
    print("Install with: pip install nats-py cryptography")
    exit(1)


class StorageReplicationClient:
    """
    Syncs local database to storage provider
    Encrypts data before sending
    """

    def __init__(
        self,
        local_db_path: str,
        device_id: str,
        provider_device_id: str,
        encryption_password: str,
        nats_url: str = "nats://nonlocal.info:4222"
    ):
        self.local_db_path = Path(local_db_path)
        self.device_id = device_id
        self.provider_device_id = provider_device_id
        self.nats_url = nats_url
        self.nc = None

        # Setup encryption
        self.cipher = self._setup_encryption(encryption_password)

        # Replication state
        self.last_sync_version = 0
        self.is_syncing = False

    def _setup_encryption(self, password: str) -> Fernet:
        """Setup encryption cipher from password"""
        kdf = PBKDF2(
            algorithm=hashes.SHA256(),
            length=32,
            salt=b'sovereign_storage_salt',  # In production, use unique salt per user
            iterations=100000,
        )
        key = base64.urlsafe_b64encode(kdf.derive(password.encode()))
        return Fernet(key)

    async def connect(self):
        """Connect to NATS"""
        if not self.nc:
            self.nc = NATS()
            await self.nc.connect(self.nats_url)
            print(f"✅ Connected to {self.nats_url}")

    def calculate_db_checksum(self) -> str:
        """Calculate checksum of entire database"""
        with open(self.local_db_path, 'rb') as f:
            return hashlib.sha256(f.read()).hexdigest()

    def get_db_version(self) -> int:
        """Get current database version (timestamp-based)"""
        conn = sqlite3.connect(self.local_db_path)
        cursor = conn.cursor()

        # Use the most recent updated_at timestamp as version
        cursor.execute("""
            SELECT MAX(updated_at) FROM (
                SELECT updated_at FROM projects
                UNION ALL
                SELECT updated_at FROM tasks
                UNION ALL
                SELECT updated_at FROM users
            )
        """)

        result = cursor.fetchone()[0]
        conn.close()

        if result:
            # Convert timestamp to int (epoch)
            dt = datetime.fromisoformat(result)
            return int(dt.timestamp())
        return 0

    def export_database_chunk(self) -> bytes:
        """Export database as encrypted chunk"""
        # Read entire database file
        with open(self.local_db_path, 'rb') as f:
            db_data = f.read()

        # Encrypt
        encrypted = self.cipher.encrypt(db_data)

        return encrypted

    async def sync_to_provider(self):
        """Sync local database to storage provider"""

        if self.is_syncing:
            print("⚠️  Sync already in progress")
            return

        self.is_syncing = True

        try:
            print("\n" + "="*70)
            print("📤 Starting sync to storage provider")
            print("="*70)

            await self.connect()

            # Get current state
            current_version = self.get_db_version()
            current_checksum = self.calculate_db_checksum()
            db_size = self.local_db_path.stat().st_size

            print(f"Database version: {current_version}")
            print(f"Database size: {db_size:,} bytes ({db_size / 1024 / 1024:.2f} MB)")
            print(f"Checksum: {current_checksum[:16]}...")

            # Check if sync needed
            if current_version == self.last_sync_version:
                print("✅ Already in sync (no changes)")
                return

            # Export and encrypt database
            print("🔒 Encrypting database...")
            encrypted_data = self.export_database_chunk()
            encrypted_size = len(encrypted_data)

            print(f"Encrypted size: {encrypted_size:,} bytes ({encrypted_size / 1024 / 1024:.2f} MB)")

            # Send sync request
            print("📡 Sending to storage provider...")

            sync_msg = {
                "type": "STORAGE_SYNC",
                "from": self.device_id,
                "to": self.provider_device_id,
                "version": current_version,
                "checksum": current_checksum,
                "size": db_size,
                "encrypted_size": encrypted_size,
                "data": base64.b64encode(encrypted_data).decode(),
                "timestamp": datetime.utcnow().isoformat()
            }

            # Publish to provider-specific subject
            await self.nc.publish(
                f"apn.storage.sync.{self.provider_device_id}",
                json.dumps(sync_msg).encode()
            )

            # Wait for acknowledgment
            print("⏳ Waiting for confirmation...")

            ack_received = False

            async def ack_handler(msg):
                nonlocal ack_received
                data = json.loads(msg.data.decode())
                if data.get("success"):
                    ack_received = True
                    print(f"✅ Sync confirmed by provider")
                    print(f"   Provider version: {data.get('version')}")

            sub = await self.nc.subscribe(
                f"apn.storage.ack.{self.device_id}",
                cb=ack_handler
            )

            # Wait up to 30 seconds
            for i in range(30):
                if ack_received:
                    break
                await asyncio.sleep(1)

            await self.nc.unsubscribe(sub)

            if ack_received:
                self.last_sync_version = current_version
                print("✅ Sync complete!")

                # Update replication state in local database
                self._update_replication_state(current_version, current_checksum)
            else:
                print("❌ Sync failed (no acknowledgment)")

        except Exception as e:
            print(f"❌ Sync error: {e}")
            import traceback
            traceback.print_exc()

        finally:
            self.is_syncing = False

    def _update_replication_state(self, version: int, checksum: str):
        """Update local replication state"""
        conn = sqlite3.connect(self.local_db_path)
        cursor = conn.cursor()

        cursor.execute("""
            INSERT OR REPLACE INTO data_replication_state (
                id, source_device_id, destination_device_id, data_owner_id,
                data_type, last_sync_timestamp, last_sync_version, current_version,
                sync_status, source_checksum
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            f"{self.device_id}_{self.provider_device_id}",
            self.device_id,
            self.provider_device_id,
            None,  # Will be set by actual user
            "full_db",
            datetime.utcnow().isoformat(),
            version,
            version,
            "in_sync",
            checksum
        ))

        conn.commit()
        conn.close()

    async def auto_sync_loop(self, interval_minutes: int = 5):
        """Automatically sync at regular intervals"""
        print(f"🔄 Auto-sync enabled (every {interval_minutes} minutes)")

        while True:
            await asyncio.sleep(interval_minutes * 60)

            try:
                await self.sync_to_provider()
            except Exception as e:
                print(f"⚠️  Auto-sync error: {e}")


# CLI
if __name__ == "__main__":
    import sys

    if len(sys.argv) < 4:
        print("Usage:")
        print("  python storage_replication_client.py <db_path> <device_id> <provider_device_id> [password]")
        print("\nExample:")
        print("  python storage_replication_client.py ~/.local/share/duck-kanban/db.sqlite sirak_laptop_001 pythia_master my_password")
        sys.exit(1)

    db_path = sys.argv[1]
    device_id = sys.argv[2]
    provider_id = sys.argv[3]
    password = sys.argv[4] if len(sys.argv) > 4 else "default_password"

    async def main():
        client = StorageReplicationClient(
            local_db_path=db_path,
            device_id=device_id,
            provider_device_id=provider_id,
            encryption_password=password
        )

        # One-time sync
        await client.sync_to_provider()

        # Keep connection alive
        await client.nc.close()

    asyncio.run(main())
