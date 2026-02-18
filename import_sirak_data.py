#!/usr/bin/env python3
"""
Import Sirak's local database content into the central network database

This script:
1. Connects to both databases (local source, network destination)
2. Identifies Sirak's user ID in the network database
3. Imports all content owned by Sirak from local to network
4. Handles ID conflicts and foreign key relationships
5. Updates ownership to Sirak's network user ID
"""

import sqlite3
import sys
from pathlib import Path

# Database paths
NETWORK_DB = "/home/pythia/.local/share/duck-kanban/db.sqlite"
# LOCAL_DB will be provided as command line argument

# Tables to import (in order, respecting foreign key dependencies)
TABLES_TO_IMPORT = [
    # Core entities first
    'projects',
    'tasks',
    'task_attempts',
    'agents',

    # Project-related
    'project_assets',
    'project_boards',
    'project_members',
    'project_pods',
    'project_brand_profiles',

    # Task-related
    'task_artifacts',
    'task_comments',
    'task_tags',
    'task_images',

    # Execution
    'execution_processes',
    'execution_process_logs',
    'executor_sessions',

    # Media
    'images',
    'media_files',
    'media_batches',

    # Workflows
    'workflow_executions',
    'workflow_events',

    # Agent-related
    'agent_flows',
    'agent_conversations',
    'agent_conversation_messages',
    'agent_wallets',

    # Other content
    'time_entries',
    'activity_logs',
]

def get_table_schema(cursor, table_name):
    """Get column information for a table"""
    cursor.execute(f"PRAGMA table_info({table_name})")
    return cursor.fetchall()

def has_owner_column(cursor, table_name):
    """Check if table has owner_id, created_by, or user_id column"""
    schema = get_table_schema(cursor, table_name)
    columns = [col[1] for col in schema]
    return any(col in columns for col in ['owner_id', 'created_by', 'created_by_user_id', 'user_id'])

def get_owner_column_name(cursor, table_name):
    """Get the name of the ownership column"""
    schema = get_table_schema(cursor, table_name)
    columns = [col[1] for col in schema]

    for col in ['owner_id', 'created_by_user_id', 'created_by', 'user_id']:
        if col in columns:
            return col
    return None

def import_data(local_db_path, local_username='admin'):
    """Import data from local database to network database"""

    # Verify files exist
    if not Path(local_db_path).exists():
        print(f"❌ Local database not found: {local_db_path}")
        return False

    if not Path(NETWORK_DB).exists():
        print(f"❌ Network database not found: {NETWORK_DB}")
        return False

    print("="*70)
    print("PCG Dashboard - Data Import Tool")
    print("="*70)
    print(f"Source (local):  {local_db_path}")
    print(f"Target (network): {NETWORK_DB}")
    print("="*70)

    # Connect to both databases
    local_conn = sqlite3.connect(local_db_path)
    network_conn = sqlite3.connect(NETWORK_DB)

    local_cursor = local_conn.cursor()
    network_cursor = network_conn.cursor()

    # Get Sirak's user ID in network database
    network_cursor.execute("SELECT id FROM users WHERE username = 'Sirak'")
    sirak_network_id = network_cursor.fetchone()

    if not sirak_network_id:
        print("❌ Sirak user not found in network database!")
        return False

    sirak_network_id = sirak_network_id[0]
    print(f"\n✅ Found Sirak in network database (ID: {sirak_network_id.hex() if isinstance(sirak_network_id, bytes) else sirak_network_id})")

    # Get local user ID (default admin or specified user)
    local_cursor.execute(f"SELECT id FROM users WHERE username = ?", (local_username,))
    local_user_id = local_cursor.fetchone()

    if not local_user_id:
        print(f"❌ User '{local_username}' not found in local database!")
        return False

    local_user_id = local_user_id[0]
    print(f"✅ Found '{local_username}' in local database")

    print(f"\n📊 Starting import process...")
    print(f"   All content from '{local_username}' will be assigned to 'Sirak' in network")
    print()

    stats = {
        'tables_processed': 0,
        'rows_imported': 0,
        'rows_skipped': 0,
        'errors': 0
    }

    # Import each table
    for table_name in TABLES_TO_IMPORT:
        # Check if table exists in both databases
        local_cursor.execute(f"SELECT name FROM sqlite_master WHERE type='table' AND name=?", (table_name,))
        if not local_cursor.fetchone():
            print(f"⏭️  Skipping {table_name} (not in local db)")
            continue

        network_cursor.execute(f"SELECT name FROM sqlite_master WHERE type='table' AND name=?", (table_name,))
        if not network_cursor.fetchone():
            print(f"⏭️  Skipping {table_name} (not in network db)")
            continue

        print(f"\n📦 Processing table: {table_name}")

        # Get table schema
        schema = get_table_schema(local_cursor, table_name)
        columns = [col[1] for col in schema]

        # Get ownership column if exists
        owner_col = get_owner_column_name(local_cursor, table_name)

        # Build query to get data from local db
        if owner_col:
            # Only import rows owned by the local user
            query = f"SELECT * FROM {table_name} WHERE {owner_col} = ?"
            local_cursor.execute(query, (local_user_id,))
        else:
            # Import all rows if no ownership column
            print(f"   ℹ️  No ownership column - importing all rows")
            local_cursor.execute(f"SELECT * FROM {table_name}")

        rows = local_cursor.fetchall()

        if not rows:
            print(f"   ⏭️  No rows to import")
            continue

        print(f"   📥 Found {len(rows)} rows to import")

        imported = 0
        skipped = 0

        for row in rows:
            try:
                # Convert row to dict
                row_dict = dict(zip(columns, row))

                # Update ownership to Sirak's network ID
                if owner_col and owner_col in row_dict:
                    row_dict[owner_col] = sirak_network_id

                # Build insert query
                cols = ', '.join(row_dict.keys())
                placeholders = ', '.join(['?' for _ in row_dict])
                insert_query = f"INSERT OR IGNORE INTO {table_name} ({cols}) VALUES ({placeholders})"

                # Execute insert
                network_cursor.execute(insert_query, list(row_dict.values()))

                if network_cursor.rowcount > 0:
                    imported += 1
                else:
                    skipped += 1

            except Exception as e:
                print(f"   ⚠️  Error importing row: {e}")
                stats['errors'] += 1

        print(f"   ✅ Imported: {imported} rows")
        if skipped > 0:
            print(f"   ⏭️  Skipped: {skipped} rows (already exist)")

        stats['tables_processed'] += 1
        stats['rows_imported'] += imported
        stats['rows_skipped'] += skipped

    # Commit changes
    network_conn.commit()

    # Close connections
    local_conn.close()
    network_conn.close()

    # Print summary
    print("\n" + "="*70)
    print("Import Complete!")
    print("="*70)
    print(f"Tables processed: {stats['tables_processed']}")
    print(f"Rows imported:    {stats['rows_imported']}")
    print(f"Rows skipped:     {stats['rows_skipped']}")
    print(f"Errors:           {stats['errors']}")
    print("="*70)

    return True

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 import_sirak_data.py <path_to_local_db.sqlite> [local_username]")
        print("\nExample:")
        print("  python3 import_sirak_data.py /path/to/sirak_local.sqlite admin")
        print("\nThis will import all content from 'admin' user in local db")
        print("and assign it to 'Sirak' user in the network database.")
        sys.exit(1)

    local_db = sys.argv[1]
    local_user = sys.argv[2] if len(sys.argv) > 2 else 'admin'

    success = import_data(local_db, local_user)
    sys.exit(0 if success else 1)
