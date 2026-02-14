#!/usr/bin/env python3
"""
Setup Device Registry for ORCHA Federated Architecture
Registers all devices and maps them to their owner users
"""

import sqlite3
import uuid
from datetime import datetime

DB_PATH = "/home/pythia/.local/share/duck-kanban/db.sqlite"

def get_user_id(cursor, username):
    """Get user ID by username"""
    cursor.execute("SELECT id FROM users WHERE username = ?", (username,))
    result = cursor.fetchone()
    if result:
        return result[0]
    return None

def register_device(cursor, device_id, owner_id, device_name, device_type, apn_node_id,
                   hardware_info=None, serves_data=True, storage_capacity_gb=0):
    """Register or update a device in the registry"""

    # Check if device exists
    cursor.execute("SELECT id FROM device_registry WHERE id = ?", (device_id,))
    exists = cursor.fetchone()

    if exists:
        # Update existing device
        cursor.execute("""
            UPDATE device_registry SET
                owner_id = ?,
                device_name = ?,
                device_type = ?,
                apn_node_id = ?,
                hardware_info = ?,
                serves_data = ?,
                storage_capacity_gb = ?,
                updated_at = datetime('now')
            WHERE id = ?
        """, (owner_id, device_name, device_type, apn_node_id, hardware_info,
              serves_data, storage_capacity_gb, device_id))
        print(f"✅ Updated device: {device_name}")
    else:
        # Insert new device
        cursor.execute("""
            INSERT INTO device_registry (
                id, owner_id, device_name, device_type, apn_node_id,
                public_key, is_online, serves_data, storage_capacity_gb,
                hardware_info, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
        """, (device_id, owner_id, device_name, device_type, apn_node_id,
              'placeholder-key', True, serves_data, storage_capacity_gb, hardware_info))
        print(f"✅ Registered new device: {device_name}")

def setup_devices():
    """Setup all devices in the registry"""
    conn = sqlite3.connect(DB_PATH)
    cursor = conn.cursor()

    print("=" * 70)
    print("ORCHA Device Registry Setup")
    print("=" * 70)

    # Get user IDs
    admin_id = get_user_id(cursor, 'admin')
    sirak_id = get_user_id(cursor, 'Sirak')
    bonomotion_id = get_user_id(cursor, 'Bonomotion')

    if not admin_id:
        print("❌ Admin user not found!")
        return

    print(f"\n📋 User IDs:")
    print(f"   admin: {admin_id.hex() if isinstance(admin_id, bytes) else admin_id}")
    print(f"   Sirak: {sirak_id.hex() if isinstance(sirak_id, bytes) else sirak_id}")
    print(f"   Bonomotion: {bonomotion_id.hex() if isinstance(bonomotion_id, bytes) else bonomotion_id}")

    print(f"\n🖥️  Registering Devices:")
    print("-" * 70)

    # Device 1: Pythia Master Node (admin's primary)
    pythia_hw = {
        "cpu": "AMD Ryzen 9 5900X (24 threads)",
        "ram": "32GB DDR4",
        "gpu": "NVIDIA RTX 3080 Ti (12GB)",
        "storage": "3.6TB NVMe SSD",
        "role": "Primary Master Node - Always On"
    }
    register_device(
        cursor,
        device_id="pythia-master-node-001",
        owner_id=admin_id,
        device_name="Pythia Master Node",
        device_type="always_on",
        apn_node_id="apn_pythia_master",
        hardware_info=str(pythia_hw),
        serves_data=True,
        storage_capacity_gb=3600
    )

    # Device 2: Space Terminal (admin's secondary)
    space_terminal_hw = {
        "cpu": "16 cores",
        "ram": "15.7GB",
        "gpu": "NVIDIA RTX 3070 Laptop",
        "role": "Secondary Compute Node"
    }
    register_device(
        cursor,
        device_id="space-terminal-001",
        owner_id=admin_id,
        device_name="Space Terminal",
        device_type="always_on",
        apn_node_id="apn_space_terminal",
        hardware_info=str(space_terminal_hw),
        serves_data=False,  # Accesses data from Pythia
        storage_capacity_gb=0
    )

    # Device 3: Sirak Studios Laptop (Sirak's primary)
    if sirak_id:
        sirak_hw = {
            "role": "Mobile Workstation",
            "uptime": "<100% - Uses APN Cloud for backup"
        }
        register_device(
            cursor,
            device_id="sirak-studios-laptop-001",
            owner_id=sirak_id,
            device_name="Sirak Studios Laptop",
            device_type="mobile",
            apn_node_id="apn_sirak_laptop",
            hardware_info=str(sirak_hw),
            serves_data=True,
            storage_capacity_gb=500
        )

        # Device 3b: APN Cloud Storage (Sirak's backup)
        register_device(
            cursor,
            device_id="apn-cloud-sirak-001",
            owner_id=sirak_id,
            device_name="APN Cloud (Sirak Backup)",
            device_type="storage_provider",
            apn_node_id="apn_cloud_sirak",
            hardware_info='{"role": "Cloud backup for Sirak when laptop offline"}',
            serves_data=True,
            storage_capacity_gb=1000
        )

    # Device 4: Bonomotion Device
    if bonomotion_id:
        bonomotion_hw = {
            "role": "Bonomotion Studio Workstation"
        }
        register_device(
            cursor,
            device_id="bonomotion-device-001",
            owner_id=bonomotion_id,
            device_name="Bonomotion Studio Desktop",
            device_type="always_on",
            apn_node_id="apn_bonomotion",
            hardware_info=str(bonomotion_hw),
            serves_data=True,
            storage_capacity_gb=1000
        )

    conn.commit()

    # Display device registry
    print("\n" + "=" * 70)
    print("📋 Device Registry Summary")
    print("=" * 70)

    cursor.execute("""
        SELECT d.device_name, u.username, d.device_type, d.is_online, d.serves_data
        FROM device_registry d
        JOIN users u ON d.owner_id = u.id
        ORDER BY u.username, d.device_name
    """)

    current_user = None
    for row in cursor.fetchall():
        device_name, username, device_type, is_online, serves_data = row

        if username != current_user:
            print(f"\n👤 {username}:")
            current_user = username

        online_badge = "🟢" if is_online else "🔴"
        serves_badge = "📡" if serves_data else "💻"
        type_badge = {
            'always_on': '🏠',
            'mobile': '📱',
            'storage_provider': '☁️'
        }.get(device_type, '❓')

        print(f"   {type_badge} {device_name:<30} {online_badge} {serves_badge} [{device_type}]")

    conn.close()

    print("\n" + "=" * 70)
    print("✅ Device Registry Setup Complete!")
    print("=" * 70)
    print("\n📌 Legend:")
    print("   🏠 Always-on device")
    print("   📱 Mobile device")
    print("   ☁️  Storage provider")
    print("   🟢 Online")
    print("   📡 Serves data")
    print("   💻 Compute only (accesses data from primary)")
    print("=" * 70)

if __name__ == "__main__":
    setup_devices()
