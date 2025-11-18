# Automated Database Backup - Quick Start

## 🎉 Setup Complete!

Automated periodic backups have been added to your Docker deployment!

---

## 🚀 Getting Started

### 1. Start the Backup Service

```bash
# Start all services (including db-backup)
docker-compose up -d

# Or start just the backup service
docker-compose up -d db-backup
```

### 2. Verify It's Running

```bash
# Check service status
docker-compose ps db-backup

# View logs
docker-compose logs db-backup

# Should see:
# 🔄 Backup service started
# 📅 Backup interval: 86400 seconds
# 🗑️  Retention: 7 days
# 📦 Creating initial backup: backup_20251119_143052.sqlite
# ✅ Initial backup complete
```

### 3. Check Your First Backup

```bash
# List backups
ls -lh backups/

# Or use the backup manager
./scripts/backup-manager.sh list
```

---

## ⚙️ Configuration

### Default Settings (in `.env`)

```bash
BACKUP_INTERVAL=86400        # Backup every 24 hours
BACKUP_RETENTION_DAYS=7      # Keep last 7 days
```

### Common Backup Intervals

```bash
# Hourly backups
BACKUP_INTERVAL=3600

# Every 6 hours
BACKUP_INTERVAL=21600

# Daily (default)
BACKUP_INTERVAL=86400

# Weekly
BACKUP_INTERVAL=604800
```

### Apply New Configuration

```bash
# Edit .env file
nano .env

# Restart backup service
docker-compose restart db-backup
```

---

## 🛠️ Using the Backup Manager

The backup manager script provides convenient commands:

```bash
# Create manual backup
./scripts/backup-manager.sh backup

# List all backups with details
./scripts/backup-manager.sh list

# Show latest backup info
./scripts/backup-manager.sh latest

# Restore from backup (interactive)
./scripts/backup-manager.sh restore backup_20251119_143000.sqlite

# Verify database integrity
./scripts/backup-manager.sh verify

# Clean old backups (older than 30 days)
./scripts/backup-manager.sh clean 30

# Show backup service status
./scripts/backup-manager.sh status

# Help
./scripts/backup-manager.sh help
```

---

## 📋 What Gets Backed Up

- **Database**: `dev_assets/db.sqlite` (SQLite database)
- **Frequency**: Every 24 hours by default (configurable)
- **Location**: `backups/` directory
- **Format**: `backup_YYYYMMDD_HHMMSS.sqlite`
- **Latest**: Symlinked as `backup_latest.sqlite`

**Example backups:**
```
backups/
├── backup_20251119_020000.sqlite
├── backup_20251120_020000.sqlite
├── backup_20251121_020000.sqlite
├── backup_latest.sqlite -> backup_20251121_020000.sqlite
└── README.md
```

---

## 🔍 Monitoring

### View Live Backup Activity

```bash
# Follow logs in real-time
docker-compose logs -f db-backup
```

### Check Last Backup

```bash
# Show latest backup info
./scripts/backup-manager.sh latest

# Quick check
ls -lh backups/backup_latest.sqlite
```

### Verify Backups Are Working

```bash
# Check backup count
ls backups/*.sqlite | wc -l

# Check total size
du -sh backups/

# List with timestamps
ls -lt backups/
```

---

## 🔄 Restore Process

### Using Backup Manager (Recommended)

```bash
# List available backups
./scripts/backup-manager.sh list

# Restore from specific backup
./scripts/backup-manager.sh restore backup_20251119_143000.sqlite
```

This will automatically:
1. ✅ Create safety backup of current database
2. ✅ Stop the application
3. ✅ Replace database with backup
4. ✅ Restart application

### Manual Restore

```bash
# Stop app
docker-compose stop app

# Copy backup
cp backups/backup_20251119_143000.sqlite dev_assets/db.sqlite

# Restart app
docker-compose start app
```

---

## 📊 Backup Service Architecture

```
┌─────────────────────────────────────────────────┐
│                 Docker Host                     │
│                                                 │
│  ┌─────────────┐         ┌──────────────────┐  │
│  │   App       │         │   db-backup      │  │
│  │ Container   │         │   Container      │  │
│  │             │         │                  │  │
│  │  ┌───────┐  │         │  ┌────────────┐  │  │
│  │  │  DB   │◄─┼─────────┼─►│ Read-only  │  │  │
│  │  │ SQLite│  │         │  │   Access   │  │  │
│  │  └───────┘  │         │  └────────────┘  │  │
│  └──────┬──────┘         └────────┬─────────┘  │
│         │                         │            │
│         │                         │            │
│  ┌──────▼───────────────┬─────────▼─────────┐  │
│  │  dev_assets/         │   backups/        │  │
│  │  ├── db.sqlite       │   ├── backup_...  │  │
│  │  └── config.json     │   ├── backup_...  │  │
│  │                      │   └── latest →    │  │
│  └──────────────────────┴───────────────────┘  │
│          Host Filesystem                       │
└─────────────────────────────────────────────────┘
```

**Key Points:**
- ✅ Read-only access prevents backup corruption
- ✅ Runs in separate container (isolated)
- ✅ No impact on app performance
- ✅ Automatic cleanup of old backups

---

## ⚠️ Important Notes

### Backup Service Features

- ✅ **Initial backup on startup** - Immediate backup when service starts
- ✅ **Periodic backups** - Automated at configured interval
- ✅ **Automatic cleanup** - Removes backups older than retention days
- ✅ **Latest symlink** - Always points to newest backup
- ✅ **Read-only access** - Cannot modify original database
- ✅ **Restart-safe** - Survives container restarts

### What Backups DON'T Include

- ❌ Git repositories (in `repos_data` volume)
- ❌ Application logs
- ❌ Config files (backed up separately if needed)

For complete system backup, also backup:
```bash
# Backup repos volume
docker run --rm \
  -v pcg-cc-mcp_repos_data:/source:ro \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/repos_$(date +%Y%m%d).tar.gz -C /source .

# Backup config
cp dev_assets/config.json backups/config_$(date +%Y%m%d).json
```

---

## 🎯 Best Practices

### For Development

```bash
BACKUP_INTERVAL=3600         # Hourly
BACKUP_RETENTION_DAYS=3      # 3 days
```

### For Production

```bash
BACKUP_INTERVAL=21600        # Every 6 hours
BACKUP_RETENTION_DAYS=30     # 30 days

# Plus off-site backups!
```

### Before Risky Operations

```bash
# Always create manual backup first
./scripts/backup-manager.sh backup

# Then perform operation
# ...

# Verify it worked
./scripts/backup-manager.sh verify
```

### Regular Maintenance

```bash
# Weekly: Check disk space
du -sh backups/

# Monthly: Test restore procedure
./scripts/backup-manager.sh list
./scripts/backup-manager.sh verify

# Quarterly: Test full disaster recovery
```

---

## 🐛 Troubleshooting

### Service Not Running

```bash
# Check status
docker-compose ps db-backup

# View logs
docker-compose logs db-backup

# Restart service
docker-compose restart db-backup
```

### No Backups Appearing

**Check database exists:**
```bash
ls -la dev_assets/db.sqlite
```

**Check logs for errors:**
```bash
docker-compose logs db-backup | grep -i error
```

**Verify volume mounts:**
```bash
docker-compose exec db-backup ls -la /data
docker-compose exec db-backup ls -la /backups
```

### Permission Errors

```bash
# Make backups directory writable
chmod 777 backups/

# Check permissions
ls -ld backups/
ls -la dev_assets/
```

### Disk Space Issues

```bash
# Check space usage
du -sh backups/
df -h .

# Manually clean old backups
./scripts/backup-manager.sh clean 3

# Or reduce retention
# Edit .env: BACKUP_RETENTION_DAYS=3
docker-compose restart db-backup
```

---

## 📚 Documentation

- **Backup Manager Help**: `./scripts/backup-manager.sh help`
- **Backup Directory README**: `backups/README.md`
- **Full Persistence Guide**: `DOCKER_DATA_PERSISTENCE.md`
- **Quick Reference**: `DOCKER_PERSISTENCE_QUICKREF.md`

---

## ✅ Verification Checklist

After setup, verify:

- [ ] Backup service is running: `docker-compose ps db-backup`
- [ ] Initial backup created: `ls -lh backups/`
- [ ] Logs show success: `docker-compose logs db-backup`
- [ ] Latest symlink exists: `ls -l backups/backup_latest.sqlite`
- [ ] Backup manager works: `./scripts/backup-manager.sh status`
- [ ] Can list backups: `./scripts/backup-manager.sh list`
- [ ] Manual backup works: `./scripts/backup-manager.sh backup`

---

## 🎉 You're All Set!

Your database is now automatically backed up every 24 hours (or your configured interval).

**Next steps:**
1. ✅ Start services: `docker-compose up -d`
2. ✅ Verify first backup: `./scripts/backup-manager.sh list`
3. ✅ Test restore: `./scripts/backup-manager.sh restore <backup-file>`
4. ✅ Set up off-site backup (S3, external drive, etc.)

**Questions?** Check the documentation or run:
```bash
./scripts/backup-manager.sh help
```
