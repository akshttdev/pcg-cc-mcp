#!/usr/bin/env python3
"""
Direct NATS sync for execution_artifacts and task_artifacts.

Publishes a sovereign storage v0.5.0 sync payload containing all project data
INCLUDING execution_artifacts and task_artifacts. This allows the production
server (once updated to v0.5.0) to receive and import document artifacts.

Can also be used standalone to sync just the artifacts once the production
server has been updated.

Usage:
    python3 sync_artifacts_nats.py [--db dev_assets/db.sqlite]
"""

import asyncio
import json
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

try:
    from nats.aio.client import Client as NATS
except ImportError:
    print("Error: nats-py not installed. Run: pip install nats-py")
    sys.exit(1)

NATS_SERVER = "nats://nonlocal.info:4222"
DEVICE_ID = "pythia-master-814d37f4"
PROVIDER_ID = "pythia-master-814d37f4"

DB_PATH = Path("dev_assets/db.sqlite")


def hex_blob(val):
    """Convert a blob to hex string, or return None."""
    if val is None:
        return None
    if isinstance(val, bytes):
        return val.hex().upper()
    return str(val)


def query_table(cursor, query):
    """Execute query and return list of dicts with bytes converted to hex."""
    cursor.execute(query)
    columns = [desc[0] for desc in cursor.description]
    rows = []
    for row in cursor.fetchall():
        d = {}
        for col, val in zip(columns, row):
            if isinstance(val, bytes):
                d[col] = val.hex().upper()
            else:
                d[col] = val
        rows.append(d)
    return rows


def build_payload(db_path):
    """Build a sovereign storage v0.5.0 sync payload from local DB."""
    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row
    c = conn.cursor()

    db_size = db_path.stat().st_size

    # Core tables (same as sovereign_storage.rs)
    projects = query_table(c,
        "SELECT hex(id) as id, name, git_repo_path, hex(organization_id) as organization_id, "
        "hex(client_id) as client_id, hex(folder_id) as folder_id, hex(owner_id) as owner_id, "
        "created_at, updated_at FROM projects WHERE deleted_at IS NULL LIMIT 500")

    tasks = query_table(c,
        "SELECT hex(id) as id, hex(project_id) as project_id, title, description, status, priority, "
        "assigned_agent, custom_properties, hex(board_id) as board_id, assignee_id, tags, "
        "due_date, created_by, created_at, updated_at FROM tasks WHERE deleted_at IS NULL LIMIT 2000")

    agents = query_table(c,
        "SELECT id, short_name, designation, description, status, capabilities, autonomy_level, created_at "
        "FROM agents LIMIT 100")

    # Org hierarchy
    users = query_table(c,
        "SELECT hex(id) as id, username, email, full_name, avatar_url, is_active, is_admin, created_at, updated_at "
        "FROM users WHERE deleted_at IS NULL")

    organizations = query_table(c,
        "SELECT hex(id) as id, name, slug, description, avatar_url, hex(owner_id) as owner_id, settings, "
        "is_active, created_at, updated_at FROM organizations WHERE deleted_at IS NULL")

    organization_members = query_table(c,
        "SELECT hex(id) as id, hex(organization_id) as organization_id, hex(user_id) as user_id, role, joined_at as granted_at "
        "FROM organization_members")

    clients = query_table(c,
        "SELECT hex(id) as id, hex(organization_id) as organization_id, name, slug, description, logo_url, "
        "website, is_active, created_at, updated_at FROM clients WHERE deleted_at IS NULL")

    project_folders = query_table(c,
        "SELECT hex(id) as id, hex(organization_id) as organization_id, hex(client_id) as client_id, "
        "name, sort_order, is_active, created_at, updated_at FROM project_folders")

    project_boards = query_table(c,
        "SELECT hex(id) as id, hex(project_id) as project_id, name, slug, board_type, description, "
        "created_at, updated_at FROM project_boards")

    board_shares = query_table(c,
        "SELECT hex(id) as id, hex(board_id) as board_id, hex(source_organization_id) as source_organization_id, "
        "hex(target_organization_id) as target_organization_id, permission, share_type, hex(shared_by) as shared_by, "
        "is_active, created_at, updated_at FROM board_shares")

    project_members = query_table(c,
        "SELECT hex(id) as id, hex(project_id) as project_id, hex(user_id) as user_id, role, permissions, "
        "hex(granted_by) as granted_by, granted_at FROM project_members")

    # v0.5.0: Execution artifacts and task linking
    execution_artifacts = query_table(c,
        "SELECT hex(id) as id, hex(execution_process_id) as execution_process_id, artifact_type, "
        "title, content, file_path, metadata, phase, hex(created_by_agent_id) as created_by_agent_id, "
        "review_status, hex(parent_artifact_id) as parent_artifact_id, created_at "
        "FROM execution_artifacts LIMIT 10000")

    task_artifacts = query_table(c,
        "SELECT hex(task_id) as task_id, hex(artifact_id) as artifact_id, artifact_role, "
        "display_order, pinned, added_at, added_by FROM task_artifacts LIMIT 50000")

    # v0.5.0: Execution history
    task_attempts = query_table(c,
        "SELECT hex(id) as id, hex(task_id) as task_id, executor, "
        "created_at, updated_at, base_branch, branch "
        "FROM task_attempts WHERE deleted_at IS NULL LIMIT 5000")

    execution_processes = query_table(c,
        "SELECT hex(id) as id, hex(task_attempt_id) as task_attempt_id, status, "
        "exit_code, started_at, completed_at, created_at, updated_at, run_reason, "
        "executor_action "
        "FROM execution_processes LIMIT 10000")

    activity_logs = query_table(c,
        "SELECT id, task_id, actor_id, actor_type, action, "
        "previous_state, new_state, metadata, timestamp "
        "FROM activity_logs LIMIT 50000")

    # Empty arrays for tables we don't need to sync right now
    empty = []

    conn.close()

    payload = {
        "from_device": DEVICE_ID,
        "to_provider": PROVIDER_ID,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "version": "0.5.0",
        "db_size_bytes": db_size,
        "projects": projects,
        "tasks": tasks,
        "agents": agents,
        "workflow_executions": empty,
        "media_batches": empty,
        "media_files": empty,
        "edit_sessions": empty,
        "media_batch_analyses": empty,
        "agent_flows": empty,
        "agent_flow_events": empty,
        "users": users,
        "organizations": organizations,
        "organization_members": organization_members,
        "clients": clients,
        "project_folders": project_folders,
        "project_boards": project_boards,
        "board_shares": board_shares,
        "project_members": project_members,
        "execution_artifacts": execution_artifacts,
        "task_artifacts": task_artifacts,
        "task_attempts": task_attempts,
        "execution_processes": execution_processes,
        "activity_logs": activity_logs,
    }

    return payload


async def main():
    db_path = DB_PATH
    if len(sys.argv) > 1 and sys.argv[1] == "--db":
        db_path = Path(sys.argv[2])

    if not db_path.exists():
        print(f"Database not found: {db_path}")
        sys.exit(1)

    print("=" * 70)
    print("Sovereign Storage v0.5.0 — NATS Artifact Sync")
    print("=" * 70)

    # Build payload
    print("Building sync payload from local DB...")
    payload = build_payload(db_path)

    print(f"  Projects: {len(payload['projects'])}")
    print(f"  Tasks: {len(payload['tasks'])}")
    print(f"  Users: {len(payload['users'])}")
    print(f"  Organizations: {len(payload['organizations'])}")
    print(f"  Clients: {len(payload['clients'])}")
    print(f"  Boards: {len(payload['project_boards'])}")
    print(f"  Board Shares: {len(payload['board_shares'])}")
    print(f"  Execution Artifacts: {len(payload['execution_artifacts'])}")
    print(f"  Task Artifacts: {len(payload['task_artifacts'])}")
    print(f"  Task Attempts: {len(payload['task_attempts'])}")
    print(f"  Execution Processes: {len(payload['execution_processes'])}")
    print(f"  Activity Logs: {len(payload['activity_logs'])}")

    # Serialize
    data = json.dumps(payload).encode()
    print(f"\n  Payload size: {len(data):,} bytes ({len(data) / 1024:.1f} KB)")

    # Connect to NATS
    print(f"\nConnecting to {NATS_SERVER}...")
    nc = NATS()
    await nc.connect(NATS_SERVER)
    print("Connected.")

    # Publish to sovereign storage sync channel
    subject = f"apn.storage.sync.{PROVIDER_ID}"
    print(f"Publishing to: {subject}")
    await nc.publish(subject, data)
    await nc.flush()
    print("Published successfully.")

    await nc.close()

    print("\n" + "=" * 70)
    print("Sync payload sent to NATS.")
    print("")
    print("If the production server is running v0.5.0 code, it will")
    print("automatically import the execution_artifacts and task_artifacts.")
    print("")
    print("Core data (projects, tasks, boards, clients, users) will be")
    print("imported even by v0.4.0 servers.")
    print("=" * 70)


if __name__ == "__main__":
    asyncio.run(main())
