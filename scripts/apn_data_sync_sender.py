#!/usr/bin/env python3
"""
APN Data Sync - Sender (Sirak's Local Device)

Sends local database to network server over APN/NATS for import.

Usage:
    python3 apn_data_sync_sender.py [database_path] [username]

Example:
    python3 apn_data_sync_sender.py ~/.local/share/duck-kanban/db.sqlite admin
"""

import asyncio
import base64
import hashlib
import json
import os
import sys
from datetime import datetime
from pathlib import Path

try:
    from nats.aio.client import Client as NATS
except ImportError:
    print("❌ Error: nats-py not installed")
    print("Install with: pip install nats-py")
    sys.exit(1)

# Configuration
NATS_SERVER = "nats://nonlocal.info:4222"
CHUNK_SIZE = 256 * 1024  # 256KB chunks

# Subjects
SYNC_REQUEST_SUBJECT = "apn.sync.request"
SYNC_CHUNK_SUBJECT = "apn.sync.chunk"
SYNC_COMPLETE_SUBJECT = "apn.sync.complete"
SYNC_STATUS_SUBJECT = "apn.sync.status"


class APNDataSyncSender:
    """Sends database chunks over APN"""

    def __init__(self, db_path, username="admin"):
        self.db_path = Path(db_path).expanduser()
        self.username = username
        self.nc = NATS()
        self.node_id = f"apn_sender_{os.getpid()}"

        if not self.db_path.exists():
            raise FileNotFoundError(f"Database not found: {self.db_path}")

    async def connect(self):
        """Connect to NATS relay"""
        print("🔌 Connecting to NATS relay...")
        await self.nc.connect(NATS_SERVER)
        print(f"✅ Connected to {NATS_SERVER}")

    async def calculate_metadata(self):
        """Calculate database metadata"""
        print("📊 Calculating database metadata...")

        # Get file size
        file_size = self.db_path.stat().st_size

        # Calculate checksum
        with open(self.db_path, 'rb') as f:
            checksum = hashlib.sha256(f.read()).hexdigest()

        # Calculate total chunks
        total_chunks = (file_size + CHUNK_SIZE - 1) // CHUNK_SIZE

        metadata = {
            "sender_node_id": self.node_id,
            "username": self.username,
            "db_name": self.db_path.name,
            "db_path": str(self.db_path),
            "total_size": file_size,
            "total_chunks": total_chunks,
            "chunk_size": CHUNK_SIZE,
            "checksum": checksum,
            "timestamp": datetime.utcnow().isoformat()
        }

        print(f"✅ Metadata calculated:")
        print(f"   Database: {metadata['db_name']}")
        print(f"   Size: {file_size:,} bytes ({file_size / 1024 / 1024:.2f} MB)")
        print(f"   Chunks: {total_chunks}")
        print(f"   Checksum: {checksum[:16]}...")

        return metadata

    async def send_sync_request(self, metadata):
        """Send sync request to network"""
        print("\n📤 Sending sync request to network...")

        await self.nc.publish(
            SYNC_REQUEST_SUBJECT,
            json.dumps(metadata).encode()
        )

        print("✅ Sync request sent")

        # Wait for acknowledgment
        print("⏳ Waiting for acknowledgment...")
        ack_received = False

        async def ack_handler(msg):
            nonlocal ack_received
            data = json.loads(msg.data.decode())
            if data.get('status') == 'ready':
                ack_received = True
                print("✅ Network is ready to receive")

        sub = await self.nc.subscribe(SYNC_STATUS_SUBJECT, cb=ack_handler)

        # Wait up to 10 seconds for ack
        for i in range(10):
            if ack_received:
                break
            await asyncio.sleep(1)

        await self.nc.unsubscribe(sub)

        if not ack_received:
            raise TimeoutError("No acknowledgment received from network")

    async def send_chunks(self, metadata):
        """Send database chunks"""
        print("\n📦 Sending database chunks...")

        total_chunks = metadata['total_chunks']
        chunk_num = 0

        with open(self.db_path, 'rb') as f:
            while True:
                chunk_data = f.read(CHUNK_SIZE)
                if not chunk_data:
                    break

                # Calculate chunk hash
                chunk_hash = hashlib.sha256(chunk_data).hexdigest()

                # Encode chunk
                chunk_msg = {
                    "chunk_num": chunk_num,
                    "data": base64.b64encode(chunk_data).decode(),
                    "chunk_hash": chunk_hash,
                    "timestamp": datetime.utcnow().isoformat()
                }

                # Send chunk
                await self.nc.publish(
                    SYNC_CHUNK_SUBJECT,
                    json.dumps(chunk_msg).encode()
                )

                # Progress
                progress = ((chunk_num + 1) / total_chunks) * 100
                print(f"   Chunk {chunk_num + 1}/{total_chunks} sent ({progress:.1f}%)", end='\r')

                chunk_num += 1

                # Small delay to avoid overwhelming the network
                await asyncio.sleep(0.1)

        print(f"\n✅ All {total_chunks} chunks sent")

    async def send_complete_signal(self):
        """Send transfer complete signal"""
        print("\n📡 Sending completion signal...")

        complete_msg = {
            "type": "TRANSFER_COMPLETE",
            "sender_node_id": self.node_id,
            "timestamp": datetime.utcnow().isoformat()
        }

        await self.nc.publish(
            SYNC_COMPLETE_SUBJECT,
            json.dumps(complete_msg).encode()
        )

        print("✅ Completion signal sent")

    async def wait_for_import_complete(self):
        """Wait for import to complete on network"""
        print("\n⏳ Waiting for network to import data...")

        import_complete = False

        async def complete_handler(msg):
            nonlocal import_complete
            data = json.loads(msg.data.decode())
            if data.get('type') == 'IMPORT_COMPLETE':
                import_complete = True
                print(f"\n✅ Import completed on network at {data.get('timestamp')}")

        sub = await self.nc.subscribe(SYNC_STATUS_SUBJECT, cb=complete_handler)

        # Wait up to 60 seconds for import to complete
        for i in range(60):
            if import_complete:
                break
            await asyncio.sleep(1)
            if i % 5 == 0 and i > 0:
                print(f"   Still waiting... ({i}s elapsed)", end='\r')

        await self.nc.unsubscribe(sub)

        if not import_complete:
            print("\n⚠️  Import status unknown (timeout)")
        else:
            print("\n🎉 Data sync and import complete!")

    async def sync(self):
        """Main sync process"""
        try:
            # Calculate metadata
            metadata = await self.calculate_metadata()

            # Send sync request
            await self.send_sync_request(metadata)

            # Send chunks
            await self.send_chunks(metadata)

            # Send complete signal
            await self.send_complete_signal()

            # Wait for import
            await self.wait_for_import_complete()

            return True

        except Exception as e:
            print(f"\n❌ Sync failed: {e}")
            import traceback
            traceback.print_exc()
            return False


async def main():
    if len(sys.argv) < 2:
        print("Usage: python3 apn_data_sync_sender.py <database_path> [username]")
        print("\nExample:")
        print("  python3 apn_data_sync_sender.py ~/.local/share/duck-kanban/db.sqlite admin")
        print("\nThis will send your local database to the network over APN")
        print("and import all content from 'admin' user to your network account.")
        sys.exit(1)

    db_path = sys.argv[1]
    username = sys.argv[2] if len(sys.argv) > 2 else 'admin'

    print("="*70)
    print("APN Data Sync - Sender")
    print("="*70)
    print(f"Database: {db_path}")
    print(f"Username: {username}")
    print(f"NATS Server: {NATS_SERVER}")
    print("="*70)

    sender = APNDataSyncSender(db_path, username)

    try:
        await sender.connect()
        success = await sender.sync()
        await sender.nc.close()

        if success:
            print("\n" + "="*70)
            print("✅ Sync Complete!")
            print("="*70)
            print("\nYour data is now available on the network at:")
            print("https://dashboard.powerclubglobal.com")
            print("\nLogin with your credentials to access your projects and content.")
            print("="*70)
        else:
            print("\n" + "="*70)
            print("❌ Sync Failed")
            print("="*70)
            sys.exit(1)

    except KeyboardInterrupt:
        print("\n\n👋 Sync cancelled by user")
        await sender.nc.close()
    except Exception as e:
        print(f"\n❌ Fatal error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    asyncio.run(main())
