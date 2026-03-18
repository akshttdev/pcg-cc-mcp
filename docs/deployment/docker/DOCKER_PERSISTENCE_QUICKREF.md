# Docker Data Persistence - Quick Reference

## 🎯 TL;DR

Your database **WILL persist** across:
- ✅ Container restarts (`docker-compose restart`)
- ✅ Image rebuilds (`docker-compose build`)
- ✅ Container recreation (`docker-compose up --force-recreate`)
- ✅ System reboots

Your database **WILL BE LOST** if:
- ❌ You run `docker-compose down -v` (deletes volumes)
- ❌ You manually delete `./dev_assets/` directory
- ❌ You delete the named volume: `docker volume rm pcg-cc-mcp_repos_data`

---

## 📍 Where Is My Data?

### Database
```bash
Location: ./dev_assets/db.sqlite
Type: Bind mount (host filesystem)
Access: Direct file access from host
Backup: Just copy the file!
```

### Git Repositories  
```bash
Location: Docker volume 'repos_data'
Type: Named volume (Docker-managed)
Access: Via container only
Backup: Use docker volume backup commands
```

---

## 🤖 Automated Backups

**Your database is automatically backed up!** The `db-backup` service runs in the background.

### Backup Configuration

```bash
# In .env file (or use defaults):
BACKUP_INTERVAL=86400        # Backup every 24 hours (in seconds)
BACKUP_RETENTION_DAYS=7      # Keep last 7 days of backups
```

**Common intervals:**
- Hourly: `3600`
- Every 6 hours: `21600`
- Daily: `86400` (default)
- Weekly: `604800`

### View Automated Backups

```bash
# List all backups
ls -lh backups/

# Use backup manager script
./scripts/backup-manager.sh list

# View latest backup
./scripts/backup-manager.sh latest

# Check backup service status
docker-compose logs db-backup
```

### Backup Manager Commands

```bash
# Create manual backup now
./scripts/backup-manager.sh backup

# List all backups with details
./scripts/backup-manager.sh list

# Restore from specific backup
./scripts/backup-manager.sh restore backups/backup_20251119_143000.sqlite

# Verify database integrity
./scripts/backup-manager.sh verify

# Clean old backups (older than 30 days)
./scripts/backup-manager.sh clean 30

# Show backup service status
./scripts/backup-manager.sh status
```

---

## ⚡ Common Operations

### Manual Backup (Quick)
```bash
# Using backup manager (recommended)
./scripts/backup-manager.sh backup

# Simple file copy
cp dev_assets/db.sqlite backups/manual_$(date +%Y%m%d).sqlite

# Or SQL dump (cross-platform compatible)
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite .dump" > backups/dump_$(date +%Y%m%d).sql
```

### Restore Database
```bash
# Stop app first (important!)
docker-compose stop app

# From file backup
cp dev_assets/db.backup.20251119.sqlite dev_assets/db.sqlite

# From SQL dump
cat backup_20251119.sql | \
  docker-compose exec -T app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite"

# Restart app
docker-compose start app
```

### Verify Data Persists
```bash
# 1. Create test data
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite \"SELECT COUNT(*) FROM projects\""

# 2. Note the count, then restart
docker-compose restart app

# 3. Verify count is the same
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite \"SELECT COUNT(*) FROM projects\""
```

### Check Permissions
```bash
# On host
ls -la dev_assets/db.sqlite

# In container
docker-compose exec app sh -c "ls -la /app/dev_assets/db.sqlite"

# Should both show readable/writable
```

### Fix Permission Issues
```bash
# Development (match your user)
chown -R $(id -u):$(id -g) dev_assets/

# Production (match container user)
chown -R 1001:1001 dev_assets/

# Nuclear option (less secure)
chmod 777 dev_assets/
chmod 666 dev_assets/db.sqlite
```

---

## 🚨 Danger Zone

### Commands That DELETE Data

```bash
# ❌ DANGER: Deletes all volumes including database!
docker-compose down -v

# ❌ DANGER: Deletes specific volume
docker volume rm pcg-cc-mcp_repos_data

# ❌ DANGER: Deletes database file
rm dev_assets/db.sqlite

# ✅ SAFE: Normal shutdown (keeps volumes)
docker-compose down
```

### Before Risky Operations

```bash
# ALWAYS backup before:
# - Schema migrations
# - Major version upgrades
# - Production deployments
# - Manual database edits

cp dev_assets/db.sqlite dev_assets/db.backup.$(date +%Y%m%d_%H%M%S).sqlite
```

---

## 🔍 Troubleshooting

### "Database is locked"
```bash
# Someone else is using the database
# Stop the app first
docker-compose stop app

# Then perform your operation
# Then restart
docker-compose start app
```

### "Unable to open database file"
```bash
# Permission issue - fix ownership
chown -R 1001:1001 dev_assets/

# Or make writable
chmod 777 dev_assets/
```

### "Database not persisting after rebuild"
```bash
# Check volume mount in docker-compose.yml
docker-compose config | grep -A 5 volumes

# Should see: ./dev_assets:/app/dev_assets

# Verify mount inside container
docker-compose exec app sh -c "mount | grep dev_assets"
```

### "Lost data after docker-compose down"
```bash
# If you used -v flag, data is gone
# Restore from backup:
cp backups/db_backup_latest.sqlite dev_assets/db.sqlite

# If no backup, data is unrecoverable
# This is why automated backups are critical!
```

---

## ✅ Daily Checklist

- [ ] Database file exists: `ls -la dev_assets/db.sqlite`
- [ ] Permissions correct: `ls -la dev_assets/` (should be writable)
- [ ] Recent backup exists: `ls -la backups/` (< 24 hours old)
- [ ] Volume mount working: `docker-compose exec app sh -c "ls /app/dev_assets"`
- [ ] No "read-only" errors in logs: `docker-compose logs app | grep -i "readonly"`

---

## 📦 Automated Backup Setup

Add to `docker-compose.yml`:

```yaml
services:
  db-backup:
    image: alpine:latest
    restart: unless-stopped
    volumes:
      - ./dev_assets:/data:ro
      - ./backups:/backups
    command: >
      sh -c '
        while true; do
          sleep 86400  # Daily
          cp /data/db.sqlite /backups/db_backup_$$(date +%Y%m%d_%H%M%S).sqlite
          find /backups -name "db_backup_*.sqlite" -mtime +7 -delete
          echo "Backup created"
        done
      '
```

Then:
```bash
# Create backup directory
mkdir -p backups

# Start backup service
docker-compose up -d db-backup

# Check backups
ls -la backups/
```

---

## 🎓 Understanding Docker Persistence

### Bind Mounts (what you're using for database)
```yaml
volumes:
  - ./dev_assets:/app/dev_assets
  #  ^^^^^^^^^^^^  ^^^^^^^^^^^^^^^
  #  Host path     Container path
```
**Pros:** Easy backup, direct access, familiar  
**Cons:** Permissions can be tricky  

### Named Volumes (what you're using for repos)
```yaml
volumes:
  - repos_data:/repos
  #  ^^^^^^^^^^  ^^^^^
  #  Volume name Container path
```
**Pros:** Docker manages it, better performance  
**Cons:** Harder to access directly  

---

## 📚 Learn More

- Full guide: `DOCKER_DATA_PERSISTENCE.md`
- SQLite backup docs: https://www.sqlite.org/backup.html
- Docker volumes: https://docs.docker.com/storage/volumes/

---

## 💡 Pro Tips

1. **Backup before every deploy:** `cp dev_assets/db.sqlite backups/pre_deploy_$(date +%Y%m%d).sqlite`
2. **Test your backups:** Restore to a test database monthly
3. **Monitor disk space:** Database can grow quickly
4. **Use transactions:** For bulk operations
5. **Never use `-v` flag:** Unless you want to delete everything

---

**Remember:** Your data is only as safe as your last backup! 💾
