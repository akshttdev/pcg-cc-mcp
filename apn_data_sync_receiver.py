#!/usr/bin/env python3
"""
APN Data Sync - Receiver (Network Server)

Receives database chunks from a peer over APN/NATS and reconstructs the database.
Then imports the data into the central network database.

Usage:
    python3 apn_data_sync_receiver.py
"""

import asyncio
import base64
import hashlib
import json
import os
import sqlite3
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
NETWORK_DB = "/home/pythia/.local/share/duck-kanban/db.sqlite"
TEMP_DIR = "/tmp/apn_sync"
CHUNK_SIZE = 256 * 1024  # 256KB chunks

# Subjects
SYNC_REQUEST_SUBJECT = "apn.sync.request"
SYNC_CHUNK_SUBJECT = "apn.sync.chunk"
SYNC_COMPLETE_SUBJECT = "apn.sync.complete"
SYNC_STATUS_SUBJECT = "apn.sync.status"


class APNDataSyncReceiver:
    """Receives database chunks over APN and imports data"""

    def __init__(self):
        self.nc = NATS()
        self.chunks = {}
        self.metadata = None
        self.temp_db_path = None

    async def connect(self):
        """Connect to NATS relay"""
        print("🔌 Connecting to NATS relay...")
        await self.nc.connect(NATS_SERVER)
        print(f"✅ Connected to {NATS_SERVER}")

    async def subscribe(self):
        """Subscribe to sync channels"""
        print("📡 Subscribing to sync channels...")

        await self.nc.subscribe(SYNC_REQUEST_SUBJECT, cb=self.handle_sync_request)
        await self.nc.subscribe(SYNC_CHUNK_SUBJECT, cb=self.handle_chunk)
        await self.nc.subscribe(SYNC_COMPLETE_SUBJECT, cb=self.handle_complete)

        print(f"✅ Subscribed to:")
        print(f"   - {SYNC_REQUEST_SUBJECT}")
        print(f"   - {SYNC_CHUNK_SUBJECT}")
        print(f"   - {SYNC_COMPLETE_SUBJECT}")

    async def handle_sync_request(self, msg):
        """Handle sync request from peer"""
        try:
            data = json.loads(msg.data.decode())
            print("\n" + "="*70)
            print("📥 Sync Request Received")
            print("="*70)
            print(f"From: {data.get('sender_node_id')}")
            print(f"Username: {data.get('username')}")
            print(f"Database: {data.get('db_name')}")
            print(f"Size: {data.get('total_size'):,} bytes")
            print(f"Chunks: {data.get('total_chunks')}")
            print(f"Checksum: {data.get('checksum')[:16]}...")

            # Store metadata
            self.metadata = data

            # Reset chunks
            self.chunks = {}

            # Create temp directory
            Path(TEMP_DIR).mkdir(parents=True, exist_ok=True)
            self.temp_db_path = f"{TEMP_DIR}/{data.get('db_name')}"

            # Send acknowledgment
            ack = {
                "type": "SYNC_ACK",
                "status": "ready",
                "timestamp": datetime.utcnow().isoformat()
            }
            await self.nc.publish(SYNC_STATUS_SUBJECT, json.dumps(ack).encode())
            print("✅ Sent acknowledgment - ready to receive chunks")

        except Exception as e:
            print(f"❌ Error handling sync request: {e}")

    async def handle_chunk(self, msg):
        """Handle incoming chunk"""
        try:
            data = json.loads(msg.data.decode())
            chunk_num = data['chunk_num']
            chunk_data = base64.b64decode(data['data'])
            chunk_hash = data['chunk_hash']

            # Verify chunk hash
            actual_hash = hashlib.sha256(chunk_data).hexdigest()
            if actual_hash != chunk_hash:
                print(f"❌ Chunk {chunk_num} hash mismatch!")
                return

            # Store chunk
            self.chunks[chunk_num] = chunk_data

            # Progress
            total = self.metadata['total_chunks'] if self.metadata else '?'
            progress = (len(self.chunks) / int(total) * 100) if self.metadata else 0
            print(f"📦 Chunk {chunk_num + 1}/{total} received ({progress:.1f}%)", end='\r')

            # Send ack for this chunk
            ack = {
                "type": "CHUNK_ACK",
                "chunk_num": chunk_num,
                "status": "received"
            }
            await self.nc.publish(SYNC_STATUS_SUBJECT, json.dumps(ack).encode())

        except Exception as e:
            print(f"\n❌ Error handling chunk: {e}")

    async def handle_complete(self, msg):
        """Handle transfer complete signal"""
        try:
            print("\n\n" + "="*70)
            print("📦 Transfer Complete - Reconstructing Database")
            print("="*70)

            # Reconstruct database file
            with open(self.temp_db_path, 'wb') as f:
                for i in sorted(self.chunks.keys()):
                    f.write(self.chunks[i])

            print(f"✅ Database reconstructed: {self.temp_db_path}")

            # Verify checksum
            with open(self.temp_db_path, 'rb') as f:
                actual_checksum = hashlib.sha256(f.read()).hexdigest()

            if actual_checksum != self.metadata['checksum']:
                print("❌ Checksum mismatch! Database may be corrupted.")
                return

            print(f"✅ Checksum verified: {actual_checksum[:16]}...")

            # Import data
            await self.import_data()

            # Cleanup
            print(f"\n🧹 Cleaning up temporary files...")
            os.remove(self.temp_db_path)
            print("✅ Cleanup complete")

            # Send completion ack
            ack = {
                "type": "IMPORT_COMPLETE",
                "status": "success",
                "timestamp": datetime.utcnow().isoformat()
            }
            await self.nc.publish(SYNC_STATUS_SUBJECT, json.dumps(ack).encode())

        except Exception as e:
            print(f"❌ Error handling completion: {e}")

    async def import_data(self):
        """Import data from received database to network database"""
        print("\n" + "="*70)
        print("📥 Importing Data to Network Database")
        print("="*70)

        username = self.metadata.get('username', 'admin')

        # Run the import script
        import subprocess
        result = subprocess.run(
            ['python3', '/home/pythia/pcg-cc-mcp/import_sirak_data.py', self.temp_db_path, username],
            capture_output=True,
            text=True
        )

        print(result.stdout)
        if result.stderr:
            print("Errors:", result.stderr)

        if result.returncode == 0:
            print("✅ Import completed successfully!")
        else:
            print(f"❌ Import failed with code {result.returncode}")

    async def run(self):
        """Main run loop"""
        await self.connect()
        await self.subscribe()

        print("\n" + "="*70)
        print("🎧 Listening for sync requests...")
        print("="*70)
        print("\nWaiting for Sirak's device to initiate sync...\n")

        # Keep running
        while True:
            await asyncio.sleep(1)


async def main():
    print("="*70)
    print("APN Data Sync - Receiver")
    print("="*70)
    print(f"NATS Server: {NATS_SERVER}")
    print(f"Network DB:  {NETWORK_DB}")
    print("="*70)

    receiver = APNDataSyncReceiver()

    try:
        await receiver.run()
    except KeyboardInterrupt:
        print("\n\n👋 Shutting down...")
        await receiver.nc.close()
    except Exception as e:
        print(f"\n❌ Fatal error: {e}")
        import traceback
        traceback.print_exc()


if __name__ == "__main__":
    asyncio.run(main())
