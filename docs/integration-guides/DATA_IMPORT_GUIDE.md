# Importing Sirak's Local Data to Network Database

## Overview

This guide explains how to import content from Sirak's local PCG Dashboard instance into the central network database at `dashboard.powerclubglobal.com`.

---

## What Gets Imported

The import process will transfer:
- ✅ **Projects** and project settings
- ✅ **Tasks** and task attempts
- ✅ **Agents** and agent configurations
- ✅ **Media files** (images, videos, etc.)
- ✅ **Workflows** and executions
- ✅ **Time entries** and activity logs
- ✅ **Agent conversations** and flows
- ✅ **All other content** owned by the user

**Important:** Only content owned by the specified user will be imported. Ownership will be transferred to Sirak's network account.

---

## Prerequisites

### On Sirak's Local Device

1. **Locate the database file**
   ```bash
   # The database is usually at:
   ~/.local/share/duck-kanban/db.sqlite

   # Or in the project directory:
   /path/to/pcg-cc-mcp/dev_assets/db.sqlite
   ```

2. **Copy the database to a USB drive or transfer via network**
   ```bash
   # Option 1: Copy to USB drive
   cp ~/.local/share/duck-kanban/db.sqlite /media/usb/sirak_local.sqlite

   # Option 2: Transfer via SCP (if accessible)
   scp ~/.local/share/duck-kanban/db.sqlite pythia@192.168.1.77:/tmp/sirak_local.sqlite

   # Option 3: Upload to cloud storage
   # Upload to Dropbox/Google Drive, then download on network server
   ```

### On the Network Server (This Machine)

Ensure you have:
- ✅ Python 3 installed
- ✅ Access to `/home/pythia/pcg-cc-mcp/`
- ✅ Import script: `import_sirak_data.py`
- ✅ Sirak's local database file transferred here

---

## Import Process

### Step 1: Transfer Sirak's Local Database

Get the database file from Sirak's device to this server:

```bash
# Example locations where you might place it:
/tmp/sirak_local.sqlite
# or
/home/pythia/pcg-cc-mcp/sirak_local.sqlite
```

### Step 2: Check Current Network Database

Before importing, check what's currently in the network database:

```bash
cd /home/pythia/pcg-cc-mcp

# Count projects by user
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite \
  "SELECT u.username, COUNT(p.id) as projects
   FROM users u
   LEFT JOIN projects p ON p.owner_id = u.id
   GROUP BY u.id"

# Count tasks
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite \
  "SELECT COUNT(*) FROM tasks"
```

### Step 3: Check Sirak's Local Database

Preview what will be imported:

```bash
# Replace /tmp/sirak_local.sqlite with actual path
LOCAL_DB="/tmp/sirak_local.sqlite"

# Count projects in local database
sqlite3 $LOCAL_DB \
  "SELECT u.username, COUNT(p.id) as projects
   FROM users u
   LEFT JOIN projects p ON p.owner_id = u.id
   GROUP BY u.id"

# Count tasks
sqlite3 $LOCAL_DB "SELECT COUNT(*) FROM tasks"

# List projects
sqlite3 $LOCAL_DB "SELECT name FROM projects"
```

### Step 4: Run the Import Script

```bash
cd /home/pythia/pcg-cc-mcp

# Basic usage (imports from 'admin' user by default)
python3 import_sirak_data.py /tmp/sirak_local.sqlite

# Or specify the local username if different
python3 import_sirak_data.py /tmp/sirak_local.sqlite sirak

# The script will show progress like:
# ====================================================================
# PCG Dashboard - Data Import Tool
# ====================================================================
# Source (local):  /tmp/sirak_local.sqlite
# Target (network): /home/pythia/.local/share/duck-kanban/db.sqlite
# ====================================================================
#
# ✅ Found Sirak in network database
# ✅ Found 'admin' in local database
#
# 📊 Starting import process...
#
# 📦 Processing table: projects
#    📥 Found 5 rows to import
#    ✅ Imported: 5 rows
#
# 📦 Processing table: tasks
#    📥 Found 23 rows to import
#    ✅ Imported: 23 rows
# ...
```

### Step 5: Verify Import

After the import completes, verify the data:

```bash
# Check Sirak's projects in network database
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite \
  "SELECT p.name, p.created_at
   FROM projects p
   JOIN users u ON p.owner_id = u.id
   WHERE u.username = 'Sirak'"

# Check Sirak's tasks
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite \
  "SELECT COUNT(*) FROM tasks t
   JOIN projects p ON t.project_id = p.id
   JOIN users u ON p.owner_id = u.id
   WHERE u.username = 'Sirak'"
```

### Step 6: Test in Dashboard

1. Login to https://dashboard.powerclubglobal.com
2. Use credentials: `Sirak` / `Sirak123`
3. Verify projects, tasks, and content appear
4. Check that everything is accessible and working

---

## Advanced Options

### Dry Run (Preview Only)

To see what would be imported without actually importing:

```bash
# Modify the script to add a dry_run parameter
# Or manually check:
sqlite3 /tmp/sirak_local.sqlite \
  "SELECT name FROM sqlite_master WHERE type='table'" | \
  while read table; do
    echo "Table: $table"
    sqlite3 /tmp/sirak_local.sqlite "SELECT COUNT(*) FROM $table"
  done
```

### Import Specific Tables Only

Edit `import_sirak_data.py` and modify the `TABLES_TO_IMPORT` list:

```python
TABLES_TO_IMPORT = [
    'projects',
    'tasks',
    # Comment out tables you don't want to import
    # 'images',
    # 'media_files',
]
```

### Backup Before Import

Always backup the network database before importing:

```bash
# Create backup
cp /home/pythia/.local/share/duck-kanban/db.sqlite \
   /home/pythia/pcg-cc-mcp/backups/pre_import_$(date +%Y%m%d_%H%M%S).sqlite

# Verify backup exists
ls -lh /home/pythia/pcg-cc-mcp/backups/
```

---

## Troubleshooting

### "Database is locked"

If you get a database lock error:

```bash
# Stop the backend server
pkill -f "target/release/server"

# Run import
python3 import_sirak_data.py /tmp/sirak_local.sqlite

# Restart backend
cd /home/pythia/pcg-cc-mcp
./start-backend-with-whisper.sh > /tmp/backend.log 2>&1 &
```

### Foreign Key Constraint Errors

If you see foreign key errors:

1. The script uses `INSERT OR IGNORE` to skip conflicts
2. Some related data might be missing (e.g., referenced users)
3. Check the error messages and import order

### Duplicate Content

The script uses `INSERT OR IGNORE` which will skip rows that already exist (based on primary key). This is safe and prevents duplicates.

### Missing Tables

If a table doesn't exist in either database:

```bash
# Check if table exists
sqlite3 /tmp/sirak_local.sqlite ".tables" | grep projects
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite ".tables" | grep projects
```

### Verify Ownership Changed

Check that imported content is owned by Sirak:

```bash
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite \
  "SELECT p.name, u.username as owner
   FROM projects p
   LEFT JOIN users u ON p.owner_id = u.id"
```

---

## What Happens During Import

1. **Connects to both databases** (local source, network target)
2. **Finds Sirak's user ID** in the network database
3. **For each table:**
   - Checks if table exists in both databases
   - Finds rows owned by the local user
   - Updates `owner_id` to Sirak's network user ID
   - Inserts rows into network database
   - Skips duplicates (based on primary key)
4. **Commits all changes** atomically
5. **Reports statistics** (rows imported, skipped, errors)

---

## Security Notes

### Database File Security

- ✅ The local database may contain sensitive data
- ✅ Transfer securely (encrypted USB, SCP, or secure cloud)
- ✅ Delete temporary files after import
- ✅ Backup both databases before and after import

### Cleanup After Import

```bash
# Delete temporary local database copy
rm /tmp/sirak_local.sqlite

# Verify it's gone
ls -la /tmp/sirak_local.sqlite  # Should show "No such file"
```

---

## Alternative: Manual Database Merge

If you prefer SQL, you can manually merge:

```sql
-- Attach local database
ATTACH DATABASE '/tmp/sirak_local.sqlite' AS local;

-- Get Sirak's network user ID
-- (Use the UUID from earlier: e76d96ca-f08e-481d-8149-f40ceca43506)

-- Import projects
INSERT OR IGNORE INTO projects
SELECT
  id,
  name,
  git_repo_path,
  -- ... other columns
  X'e76d96caf08e481d8149f40ceca43506' AS owner_id  -- Sirak's network ID
FROM local.projects
WHERE owner_id = (SELECT id FROM local.users WHERE username = 'admin');

-- Repeat for other tables...
```

---

## Post-Import Checklist

- [ ] Backup created before import
- [ ] Import script executed successfully
- [ ] No errors reported
- [ ] Data verified in network database
- [ ] Sirak can login and see imported content
- [ ] Projects, tasks, agents accessible
- [ ] Media files accessible (if imported)
- [ ] Temporary local database deleted
- [ ] Success documented

---

## Quick Reference

### File Locations

```
Network Database:  /home/pythia/.local/share/duck-kanban/db.sqlite
Import Script:     /home/pythia/pcg-cc-mcp/import_sirak_data.py
Backups:           /home/pythia/pcg-cc-mcp/backups/
```

### Key Commands

```bash
# Run import
cd /home/pythia/pcg-cc-mcp
python3 import_sirak_data.py /path/to/local.sqlite [username]

# Backup database
cp /home/pythia/.local/share/duck-kanban/db.sqlite backups/backup_$(date +%Y%m%d_%H%M%S).sqlite

# Check Sirak's data
sqlite3 /home/pythia/.local/share/duck-kanban/db.sqlite \
  "SELECT COUNT(*) FROM projects p JOIN users u ON p.owner_id = u.id WHERE u.username = 'Sirak'"
```

---

## Support

If you encounter issues:

1. **Check the script output** for error messages
2. **Verify database file paths** are correct
3. **Ensure Sirak user exists** in network database
4. **Check foreign key relationships** for missing dependencies
5. **Review backups** before and after import

---

**Last Updated:** 2026-02-09
**Location:** `/home/pythia/pcg-cc-mcp/`
**Network Database:** `/home/pythia/.local/share/duck-kanban/db.sqlite`
**Sirak's Network User ID:** `e76d96ca-f08e-481d-8149-f40ceca43506`
