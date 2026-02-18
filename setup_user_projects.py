#!/usr/bin/env python3
"""
Setup User Projects for ORCHA
Creates initial projects for each user in their respective Topsi databases
"""

import sqlite3
import uuid
from datetime import datetime

# Database paths
ADMIN_DB = "/home/pythia/.local/share/pcg/data/admin/topsi.db"
SHARED_DB = "/home/pythia/.local/share/duck-kanban/db.sqlite"  # For user lookup

def get_user_id(cursor, username):
    """Get user ID from shared database"""
    cursor.execute("SELECT id FROM users WHERE username = ?", (username,))
    result = cursor.fetchone()
    if result:
        return result[0]
    return None

def create_project(cursor, project_name, git_repo_path, owner_id):
    """Create a project in Topsi database"""
    project_id = uuid.uuid4().bytes
    created_at = datetime.utcnow().isoformat()

    cursor.execute("""
        INSERT OR IGNORE INTO projects (
            id, name, git_repo_path, owner_id, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?)
    """, (project_id, project_name, git_repo_path, owner_id, created_at, created_at))

    print(f"   ✅ Created project: {project_name}")
    print(f"      Path: {git_repo_path}")
    print(f"      ID: {uuid.UUID(bytes=project_id)}")

    return project_id

def add_project_member(cursor, project_id, user_id, role='owner'):
    """Add user as project member"""
    member_id = uuid.uuid4().bytes
    granted_at = datetime.utcnow().isoformat()

    cursor.execute("""
        INSERT OR IGNORE INTO project_members (
            id, project_id, user_id, role, granted_at
        ) VALUES (?, ?, ?, ?, ?)
    """, (member_id, project_id, user_id, role, granted_at))

def setup_admin_projects():
    """Setup projects for admin user"""
    print("=" * 70)
    print("Setting up Admin's Projects (Pythia Master Node)")
    print("=" * 70)
    print("")

    # Get admin user ID from shared DB
    shared_conn = sqlite3.connect(SHARED_DB)
    shared_cursor = shared_conn.cursor()
    admin_id = get_user_id(shared_cursor, 'admin')
    shared_conn.close()

    if not admin_id:
        print("❌ Admin user not found in shared database!")
        return

    print(f"📋 Admin user ID: {admin_id.hex() if isinstance(admin_id, bytes) else admin_id}")
    print("")

    # Connect to admin's Topsi database
    admin_conn = sqlite3.connect(ADMIN_DB)
    admin_cursor = admin_conn.cursor()

    # Create admin's projects
    projects = [
        {
            "name": "Pythia Oracle Agent",
            "path": "/home/pythia/oracle-agent-pythia",
            "description": "Main Pythia spatial intelligence system"
        },
        {
            "name": "ORCHA (PCG Dashboard)",
            "path": "/home/pythia/pcg-cc-mcp",
            "description": "Orchestration application and dashboard"
        },
        {
            "name": "Alpha Protocol Web",
            "path": "/home/pythia/alpha-protocol-web",
            "description": "Alpha Protocol website and interface"
        },
        {
            "name": "Power Club Global Website",
            "path": "/home/pythia/powerclub-global-website",
            "description": "Power Club Global marketing website"
        },
        {
            "name": "APN Core",
            "path": "/home/pythia/apn-core",
            "description": "Alpha Protocol Network core infrastructure"
        }
    ]

    print("📁 Creating projects in admin's Topsi:")
    print("-" * 70)

    for proj in projects:
        project_id = create_project(
            admin_cursor,
            proj["name"],
            proj["path"],
            admin_id
        )

        # Add admin as project owner
        add_project_member(admin_cursor, project_id, admin_id, 'owner')
        print("")

    admin_conn.commit()

    # Show project summary
    print("=" * 70)
    print("📋 Admin's Projects Summary")
    print("=" * 70)

    admin_cursor.execute("""
        SELECT name, git_repo_path, created_at
        FROM projects
        ORDER BY created_at DESC
    """)

    for row in admin_cursor.fetchall():
        name, path, created_at = row
        print(f"  📦 {name}")
        print(f"     {path}")
        print("")

    admin_conn.close()

    print("=" * 70)
    print("✅ Admin's Projects Setup Complete!")
    print("=" * 70)
    print("")
    print("Next: Set up projects for Sirak and Bonomotion on their devices")
    print("")

def setup_placeholder_projects_for_others():
    """Add placeholder entries for Sirak and Bonomotion projects"""
    print("=" * 70)
    print("Setting up placeholder project references for other users")
    print("=" * 70)
    print("")

    shared_conn = sqlite3.connect(SHARED_DB)
    shared_cursor = shared_conn.cursor()

    sirak_id = get_user_id(shared_cursor, 'Sirak')
    bonomotion_id = get_user_id(shared_cursor, 'Bonomotion')

    if sirak_id:
        print("📋 Sirak's projects (to be initialized on sirak-studios-laptop-001):")
        print("   - Sirak Studios")
        print("   - Prime")
        print("")

    if bonomotion_id:
        print("📋 Bonomotion's projects (to be initialized on bonomotion-device-001):")
        print("   - Bonomotion")
        print("")

    shared_conn.close()

    print("💡 These projects will be created when Sirak and Bonomotion")
    print("   initialize their Topsi databases on their respective devices.")
    print("")

if __name__ == "__main__":
    setup_admin_projects()
    setup_placeholder_projects_for_others()
