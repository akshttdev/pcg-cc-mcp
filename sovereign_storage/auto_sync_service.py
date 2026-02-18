#!/usr/bin/env python3
"""
Automatic Sync Service

Runs alongside the PCG Dashboard
Automatically syncs database to storage provider every 5 minutes
Integrates with dashboard via configuration file

Usage:
  python3 auto_sync_service.py [config_file]

Config file format (JSON):
{
  "database_path": "~/.local/share/duck-kanban/db.sqlite",
  "device_id": "c5ff23d2-1bdc-4479-b614-dfc103e8aa67",
  "provider_device_id": "d903c0e7-f3a6-4967-80e7-0da9d0fe7632",
  "encryption_password": "your_password_here",
  "sync_interval_minutes": 5,
  "enabled": true
}
"""

import asyncio
import json
import sys
import signal
from pathlib import Path
from datetime import datetime
from typing import Optional
import os

# Import the replication client
import sys
sys.path.insert(0, str(Path(__file__).parent))
from storage_replication_client import StorageReplicationClient


class AutoSyncService:
    """
    Automatic database sync service
    Runs in background and syncs database to storage provider
    """

    def __init__(self, config_path: Optional[str] = None):
        self.config_path = config_path or os.path.expanduser("~/.config/pcg-dashboard/sovereign-sync.json")
        self.config = self.load_config()
        self.client = None
        self.running = False

    def load_config(self) -> dict:
        """Load configuration from file or create default"""
        config_file = Path(self.config_path).expanduser()

        if config_file.exists():
            with open(config_file, 'r') as f:
                config = json.load(f)
                print(f"✅ Loaded config from {config_file}")
                return config
        else:
            # Create default config
            default_config = {
                "enabled": False,
                "database_path": "~/.local/share/duck-kanban/db.sqlite",
                "device_id": "",
                "provider_device_id": "",
                "encryption_password": "",
                "sync_interval_minutes": 5,
                "nats_url": "nats://nonlocal.info:4222"
            }

            # Create config directory
            config_file.parent.mkdir(parents=True, exist_ok=True)

            with open(config_file, 'w') as f:
                json.dump(default_config, f, indent=2)

            print(f"⚠️  Created default config at {config_file}")
            print(f"⚠️  Please edit the config file and set enabled=true")
            return default_config

    def save_config(self):
        """Save current configuration"""
        config_file = Path(self.config_path).expanduser()
        config_file.parent.mkdir(parents=True, exist_ok=True)

        with open(config_file, 'w') as f:
            json.dump(self.config, f, indent=2)

        print(f"✅ Saved config to {config_file}")

    def validate_config(self) -> bool:
        """Validate configuration"""
        required = ["database_path", "device_id", "provider_device_id", "encryption_password"]

        for field in required:
            if not self.config.get(field):
                print(f"❌ Missing required config field: {field}")
                return False

        # Check if database exists
        db_path = Path(self.config["database_path"]).expanduser()
        if not db_path.exists():
            print(f"❌ Database not found: {db_path}")
            return False

        return True

    async def setup_client(self):
        """Setup replication client"""
        self.client = StorageReplicationClient(
            local_db_path=os.path.expanduser(self.config["database_path"]),
            device_id=self.config["device_id"],
            provider_device_id=self.config["provider_device_id"],
            encryption_password=self.config["encryption_password"],
            nats_url=self.config.get("nats_url", "nats://nonlocal.info:4222")
        )

        await self.client.connect()
        print(f"✅ Connected to NATS")

    async def sync_once(self):
        """Perform a single sync"""
        try:
            await self.client.sync_to_provider()
            print(f"✅ Sync completed at {datetime.now().isoformat()}")
        except Exception as e:
            print(f"❌ Sync failed: {e}")

    async def run(self):
        """Main run loop"""
        if not self.config.get("enabled", False):
            print("❌ Auto-sync is disabled in config")
            print(f"   Edit {self.config_path} and set enabled=true")
            return

        if not self.validate_config():
            print("❌ Invalid configuration")
            return

        self.running = True

        print("=" * 70)
        print("🔄 PCG Dashboard Auto-Sync Service")
        print("=" * 70)
        print(f"Database: {self.config['database_path']}")
        print(f"Device ID: {self.config['device_id']}")
        print(f"Provider: {self.config['provider_device_id']}")
        print(f"Interval: {self.config['sync_interval_minutes']} minutes")
        print("=" * 70)
        print()

        await self.setup_client()

        # Perform initial sync
        print("📤 Performing initial sync...")
        await self.sync_once()

        # Start periodic sync loop
        interval_seconds = self.config["sync_interval_minutes"] * 60

        print(f"\n🔄 Auto-sync enabled (every {self.config['sync_interval_minutes']} minutes)")
        print("Press Ctrl+C to stop\n")

        while self.running:
            await asyncio.sleep(interval_seconds)

            if not self.running:
                break

            print(f"\n⏰ Starting scheduled sync...")
            await self.sync_once()

    async def stop(self):
        """Stop the service"""
        print("\n\n🛑 Stopping auto-sync service...")
        self.running = False

        if self.client and self.client.nc:
            await self.client.nc.close()

        print("✅ Stopped")


async def main():
    """Main entry point"""
    config_path = sys.argv[1] if len(sys.argv) > 1 else None

    service = AutoSyncService(config_path)

    # Setup signal handlers
    def signal_handler(sig, frame):
        asyncio.create_task(service.stop())

    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)

    try:
        await service.run()
    except KeyboardInterrupt:
        await service.stop()


if __name__ == "__main__":
    asyncio.run(main())
