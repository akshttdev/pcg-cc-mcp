# Docker Data Persistence & Database Consistency Guide

## 🎯 Problem Statement

Ensuring data and databases stay consistent across:
- ✅ Container restarts (`docker-compose restart`)
- ✅ Image rebuilds (`docker-compose build`)
- ✅ Container recreation (`docker-compose down && docker-compose up`)
- ✅ System reboots
- ✅ Docker version upgrades

---

## 📊 Current Setup Analysis

### Your Current Configuration

**docker-compose.yml:**
```yaml
volumes:
  # Bind mount - persists on host filesystem
  - ./dev_assets:/app/dev_assets
  
  # Named volume - managed by Docker
  - repos_data:/repos

volumes:
  repos_data:
    driver: local
```

**Dockerfile:**
```dockerfile
# Copies database into image (problematic!)
COPY --from=builder /app/dev_assets /app/dev_assets

# Creates directories
RUN mkdir -p /repos /app/dev_assets && \
    chown -R appuser:appgroup /repos /app
```

**Database location:**
```bash
DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
```

### ✅ What Works Well

1. **Bind mount for database** (`./dev_assets:/app/dev_assets`)
   - Database persists on host at `./dev_assets/db.sqlite`
   - Survives container deletion
   - Can be backed up directly from host
   - Easy to inspect and restore

2. **Named volume for repos** (`repos_data:/repos`)
   - Git repositories persist across rebuilds
   - Managed by Docker (automatic cleanup when needed)
   - Good for data that doesn't need host access

3. **Migrations via SQLx**
   - Schema migrations in `crates/db/migrations/`
   - Automatic schema versioning
   - `CREATE TABLE IF NOT EXISTS` for safety

### ⚠️ Issues & Risks

#### 1. Database Copied Into Image (Anti-pattern)

**Problem:**
```dockerfile
COPY --from=builder /app/dev_assets /app/dev_assets
```

This creates a **stale database snapshot** inside the image:
- Image contains database from build time
- Volume mount overrides it at runtime
- Wastes image space (~5-50MB per image)
- Confusing during debugging
- If volume mount fails, old database is used!

**Impact:**
- 🔴 **High risk**: If volume mount path is wrong, old data is used silently
- 🟡 **Medium waste**: Every image contains outdated database copy
- 🟡 **Confusion**: Developers wonder why database in image != runtime database

#### 2. No Database Initialization Strategy

**Problem:** First run behavior undefined
- What if `dev_assets/` doesn't exist?
- What if `db.sqlite` is missing?
- Who runs migrations?

**Current behavior:**
```bash
# First run on new machine:
docker-compose up
# ERROR: /app/dev_assets/db.sqlite not found (if dev_assets/ is empty)
```

#### 3. Permission Mismatches

**Problem:**
- Container runs as `appuser` (UID 1001, GID 1001)
- Host files may be owned by different user (e.g., UID 501 on macOS)
- SQLite needs write access to both database file AND directory

**Symptoms:**
```
Error: unable to open database file
Error: attempt to write a readonly database
```

#### 4. No Backup Strategy

**Current state:**
- ✅ Data persists via volume
- ❌ No automated backups
- ❌ No backup restoration docs
- ❌ No disaster recovery plan

#### 5. Migration Safety

**Risk:** Schema migrations could fail mid-transaction:
```sql
-- If this succeeds but next line fails...
ALTER TABLE tasks ADD COLUMN new_field TEXT;
-- Database is in broken state
INSERT INTO migration_log VALUES (...);
```

---

## ✅ Recommended Solutions

### Solution 1: Fix Dockerfile (Remove DB Copy)

**Update Dockerfile to only copy config template, not database:**

```dockerfile
# Build stage
FROM node:24-alpine AS builder
# ... existing build steps ...

# Runtime stage
FROM alpine:latest AS runtime

# Install runtime dependencies
RUN apk add --no-cache \
    ca-certificates \
    tini \
    libgcc \
    wget

# Create app user for security
RUN addgroup -g 1001 -S appgroup && \
    adduser -u 1001 -S appuser -G appgroup

# Copy binary and frontend from builder
COPY --from=builder /app/target/release/server /usr/local/bin/server
COPY --from=builder /app/frontend/dist /app/frontend/dist

# ONLY copy config template, NOT database!
COPY --from=builder /app/dev_assets/config.json /app/dev_assets_seed/config.json

# Create necessary directories with correct permissions
RUN mkdir -p /repos /app/dev_assets && \
    chown -R appuser:appgroup /repos /app /app/dev_assets

# Switch to non-root user
USER appuser

# ... rest of Dockerfile ...
```

**Why this works:**
- ✅ No stale database in image
- ✅ Image size reduced
- ✅ Clear separation: image = code, volume = data
- ✅ Config template available for first-run initialization

### Solution 2: Add Database Initialization Script

**Create initialization entrypoint:**

```bash
#!/bin/sh
# scripts/docker-entrypoint.sh

set -e

echo "🚀 Starting PCG-CC-MCP..."

# Check if database exists
if [ ! -f "/app/dev_assets/db.sqlite" ]; then
    echo "📦 First run detected - initializing database..."
    
    # Copy config template if needed
    if [ ! -f "/app/dev_assets/config.json" ]; then
        echo "📝 Copying default config..."
        cp /app/dev_assets_seed/config.json /app/dev_assets/config.json
    fi
    
    # Database will be created by migrations on first run
    echo "✅ Database will be initialized by application"
else
    echo "✅ Database found at /app/dev_assets/db.sqlite"
fi

# Check permissions
if [ ! -w "/app/dev_assets" ]; then
    echo "⚠️  WARNING: /app/dev_assets is not writable!"
    echo "   This may cause database errors."
    echo "   Fix with: chown -R 1001:1001 ./dev_assets"
fi

# Run migrations and start server
echo "🔄 Running database migrations..."
exec "$@"
```

**Update Dockerfile:**

```dockerfile
# Copy entrypoint script
COPY scripts/docker-entrypoint.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Use entrypoint
ENTRYPOINT ["/sbin/tini", "--", "docker-entrypoint.sh"]
CMD ["server"]
```

### Solution 3: Fix Permission Issues

**Option A: Match container UID to host UID (Development)**

```yaml
# docker-compose.yml
services:
  app:
    user: "${UID:-1001}:${GID:-1001}"
    environment:
      - DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
    volumes:
      - ./dev_assets:/app/dev_assets
```

```bash
# .env
UID=1001  # Set to your user ID (run `id -u`)
GID=1001  # Set to your group ID (run `id -g`)
```

**Option B: Fix ownership before starting (Production)**

```yaml
# docker-compose.yml
services:
  app:
    volumes:
      - dev_assets_data:/app/dev_assets  # Named volume instead of bind mount
      - repos_data:/repos

volumes:
  dev_assets_data:
    driver: local
  repos_data:
    driver: local
```

**Option C: Use init container pattern**

```yaml
services:
  init:
    image: alpine:latest
    volumes:
      - ./dev_assets:/data
    command: >
      sh -c "
        mkdir -p /data && 
        chown -R 1001:1001 /data &&
        echo 'Permissions fixed'
      "

  app:
    depends_on:
      init:
        condition: service_completed_successfully
    # ... rest of config
```

### Solution 4: Implement Backup Strategy

**Add backup service to docker-compose.yml:**

```yaml
services:
  # Automated backup service
  backup:
    image: alpine:latest
    volumes:
      - ./dev_assets:/data:ro  # Read-only
      - ./backups:/backups
    environment:
      - BACKUP_INTERVAL=86400  # Daily (in seconds)
    command: >
      sh -c '
        while true; do
          TIMESTAMP=$$(date +%Y%m%d_%H%M%S)
          echo "📦 Creating backup: backup_$$TIMESTAMP.sqlite"
          cp /data/db.sqlite /backups/backup_$$TIMESTAMP.sqlite
          
          # Keep only last 7 days
          find /backups -name "backup_*.sqlite" -mtime +7 -delete
          
          echo "✅ Backup complete. Next backup in $$BACKUP_INTERVAL seconds"
          sleep $$BACKUP_INTERVAL
        done
      '
    restart: unless-stopped
```

**Manual backup commands:**

```bash
# Backup database
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite '.backup /app/dev_assets/backup_$(date +%Y%m%d).sqlite'"

# Export to SQL dump
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite .dump" > backup.sql

# Restore from backup
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite < /app/dev_assets/backup_20251119.sqlite"
```

### Solution 5: Add Volume Backup to CI/CD

**GitHub Actions example:**

```yaml
# .github/workflows/backup.yml
name: Database Backup

on:
  schedule:
    - cron: '0 2 * * *'  # Daily at 2 AM
  workflow_dispatch:  # Manual trigger

jobs:
  backup:
    runs-on: ubuntu-latest
    steps:
      - name: Backup database
        run: |
          ssh ${{ secrets.SERVER_HOST }} \
            "docker-compose exec -T app \
             sqlite3 /app/dev_assets/db.sqlite .dump" > backup_$(date +%Y%m%d).sql
      
      - name: Upload to S3
        run: |
          aws s3 cp backup_$(date +%Y%m%d).sql \
            s3://my-backups/pcg-cc-mcp/
```

### Solution 6: Migration Safety Improvements

**Add migration testing:**

```rust
// crates/db/src/lib.rs

pub async fn run_migrations_safe(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // Create backup before migrations
    let backup_path = format!("/app/dev_assets/db.backup.{}.sqlite", 
        chrono::Utc::now().timestamp());
    
    sqlx::query(&format!("VACUUM INTO '{}'", backup_path))
        .execute(pool)
        .await?;
    
    tracing::info!("📦 Created migration backup: {}", backup_path);
    
    // Run migrations in transaction
    let mut tx = pool.begin().await?;
    
    match sqlx::migrate!("./migrations")
        .run(&mut *tx)
        .await
    {
        Ok(_) => {
            tx.commit().await?;
            tracing::info!("✅ Migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            tx.rollback().await?;
            tracing::error!("❌ Migration failed, rolled back: {}", e);
            tracing::info!("💾 Backup available at: {}", backup_path);
            Err(e)
        }
    }
}
```

---

## 🚀 Complete Recommended Setup

### Updated docker-compose.yml

```yaml
version: '3.8'

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
    container_name: pcg-cc-mcp
    restart: unless-stopped
    
    # Match container user to host user (development)
    user: "${UID:-1001}:${GID:-1001}"
    
    environment:
      - HOST=0.0.0.0
      - BACKEND_PORT=3001
      - FRONTEND_PORT=3000
      - RUST_LOG=info
      - DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
      
      # NORA Configuration
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - NORA_LLM_MODEL=${NORA_LLM_MODEL:-gpt-4-turbo}
      - NORA_LLM_TEMPERATURE=${NORA_LLM_TEMPERATURE:-0.7}
      - NORA_LLM_MAX_TOKENS=${NORA_LLM_MAX_TOKENS:-2000}
      
      # Voice Configuration
      - ELEVENLABS_API_KEY=${ELEVENLABS_API_KEY:-}
      - AZURE_SPEECH_KEY=${AZURE_SPEECH_KEY:-}
      - AZURE_SPEECH_REGION=${AZURE_SPEECH_REGION:-eastus}
    
    volumes:
      # Bind mount for database (easy backup)
      - ./dev_assets:/app/dev_assets
      
      # Named volume for repositories
      - repos_data:/repos
      
      # Optional: Backup directory
      - ./backups:/backups
    
    ports:
      - "3001:3001"
    
    healthcheck:
      test: ["CMD", "wget", "--quiet", "--tries=1", "--spider", "http://127.0.0.1:3001/api/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    
    networks:
      - app-network

  # Optional: Automated backup service
  db-backup:
    image: alpine:latest
    container_name: pcg-db-backup
    restart: unless-stopped
    depends_on:
      - app
    volumes:
      - ./dev_assets:/data:ro
      - ./backups:/backups
    command: >
      sh -c '
        while true; do
          sleep 86400  # Daily
          TIMESTAMP=$$(date +%Y%m%d_%H%M%S)
          cp /data/db.sqlite /backups/db_backup_$$TIMESTAMP.sqlite
          find /backups -name "db_backup_*.sqlite" -mtime +7 -delete
          echo "✅ Backup created: db_backup_$$TIMESTAMP.sqlite"
        done
      '

  cloudflared:
    image: cloudflare/cloudflared:latest
    container_name: pcg-cloudflared
    restart: unless-stopped
    command: tunnel --no-autoupdate run --token ${CLOUDFLARE_TUNNEL_TOKEN}
    environment:
      - TUNNEL_TOKEN=${CLOUDFLARE_TUNNEL_TOKEN}
    depends_on:
      app:
        condition: service_healthy
    networks:
      - app-network

volumes:
  repos_data:
    driver: local
    # Optional: Add labels for better organization
    labels:
      - "com.pcg-cc-mcp.description=Git repository storage"
      - "com.pcg-cc-mcp.backup=true"

networks:
  app-network:
    driver: bridge
```

### Updated .env.example

```bash
# User ID matching (for development)
# Get your UID: id -u
# Get your GID: id -g
UID=1001
GID=1001

# Cloudflare Tunnel
CLOUDFLARE_TUNNEL_TOKEN=your_token_here

# OpenAI API (REQUIRED for NORA)
OPENAI_API_KEY=sk-your-key-here

# NORA Configuration
NORA_LLM_MODEL=gpt-4-turbo
NORA_LLM_TEMPERATURE=0.7
NORA_LLM_MAX_TOKENS=2000

# Voice/TTS/STT (Optional)
# ELEVENLABS_API_KEY=your-key-here
# AZURE_SPEECH_KEY=your-key-here
# AZURE_SPEECH_REGION=eastus

# Application Settings
HOST=0.0.0.0
BACKEND_PORT=3001
FRONTEND_PORT=3000
RUST_LOG=info

# Database
DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
```

---

## 📖 Best Practices

### 1. Development Workflow

```bash
# First time setup
cp .env.example .env
# Edit .env with your keys

# Set UID/GID to match your user
echo "UID=$(id -u)" >> .env
echo "GID=$(id -g)" >> .env

# Start containers
docker-compose up -d

# Check database
ls -la dev_assets/
# -rw-r--r-- 1 youruser yourgroup ... db.sqlite

# Verify permissions
docker-compose exec app sh -c "ls -la /app/dev_assets/"
# -rw-r--r-- 1 appuser appgroup ... db.sqlite
```

### 2. Backup Before Risky Operations

```bash
# Before schema migrations
cp dev_assets/db.sqlite dev_assets/db.backup.$(date +%Y%m%d).sqlite

# Before major upgrades
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite .dump" > full_backup_$(date +%Y%m%d).sql
```

### 3. Testing Data Persistence

```bash
# 1. Create test data
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite \"INSERT INTO projects (name) VALUES ('Test Project')\""

# 2. Verify data exists
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite \"SELECT * FROM projects WHERE name='Test Project'\""

# 3. Restart container
docker-compose restart app

# 4. Verify data persists
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite \"SELECT * FROM projects WHERE name='Test Project'\""

# 5. Rebuild image
docker-compose build app
docker-compose up -d app

# 6. Verify data STILL persists
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite \"SELECT * FROM projects WHERE name='Test Project'\""

# 7. Complete teardown
docker-compose down
docker-compose up -d

# 8. Verify data STILL persists
docker-compose exec app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite \"SELECT * FROM projects WHERE name='Test Project'\""
```

### 4. Disaster Recovery

**Restore from backup:**

```bash
# Stop application
docker-compose stop app

# Restore database
cp backups/db_backup_20251119_120000.sqlite dev_assets/db.sqlite

# Or restore from SQL dump
cat full_backup_20251119.sql | \
  docker-compose exec -T app sh -c \
  "sqlite3 /app/dev_assets/db.sqlite"

# Restart application
docker-compose start app
```

### 5. Volume Management

```bash
# List volumes
docker volume ls

# Inspect volume
docker volume inspect pcg-cc-mcp_repos_data

# Backup named volume
docker run --rm \
  -v pcg-cc-mcp_repos_data:/source:ro \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/repos_backup_$(date +%Y%m%d).tar.gz -C /source .

# Restore named volume
docker run --rm \
  -v pcg-cc-mcp_repos_data:/target \
  -v $(pwd)/backups:/backup \
  alpine tar xzf /backup/repos_backup_20251119.tar.gz -C /target
```

---

## ✅ Checklist: Data Persistence

### Before Production Deployment

- [ ] Database NOT copied into Docker image
- [ ] Bind mount or named volume configured for `/app/dev_assets`
- [ ] Named volume configured for `/repos`
- [ ] Permissions set correctly (UID/GID match or world-writable)
- [ ] Backup strategy implemented
- [ ] Backup restoration tested
- [ ] Migration rollback tested
- [ ] Data persists across container restart
- [ ] Data persists across image rebuild
- [ ] Data persists across `docker-compose down`
- [ ] `.env` file not committed to git
- [ ] `dev_assets/db.sqlite` not committed to git (in .gitignore)
- [ ] Backup directory exists and is writable
- [ ] Documentation updated with backup procedures

### Monitoring

- [ ] Database size monitored (alert at >1GB)
- [ ] Backup success/failure logged
- [ ] Permission errors logged and alerted
- [ ] Disk space monitored (alert at >90% full)
- [ ] Migration failures trigger alerts

---

## 🔍 Troubleshooting

### Database Permission Denied

**Error:**
```
Error: unable to open database file
Error: attempt to write a readonly database
```

**Fix:**
```bash
# Check ownership
ls -la dev_assets/

# Fix ownership (development)
sudo chown -R $(id -u):$(id -g) dev_assets/

# Fix ownership (production, container UID)
sudo chown -R 1001:1001 dev_assets/

# Or make world-writable (less secure)
chmod 777 dev_assets/
chmod 666 dev_assets/db.sqlite
```

### Database Not Persisting

**Check volume mount:**
```bash
docker-compose exec app sh -c "ls -la /app/dev_assets/"
docker-compose exec app sh -c "echo test > /app/dev_assets/test.txt"
ls -la dev_assets/  # Should see test.txt
```

**Check volume configuration:**
```bash
docker volume inspect pcg-cc-mcp_repos_data
docker-compose config  # View resolved configuration
```

### Migration Failures

**Rollback:**
```bash
# Stop app
docker-compose stop app

# Restore backup
cp dev_assets/db.backup.*.sqlite dev_assets/db.sqlite

# Start app
docker-compose start app
```

### Data Loss After docker-compose down -v

**Prevention:**
```bash
# NEVER use -v flag unless you want to delete volumes!
docker-compose down -v  # ❌ DELETES VOLUMES!

# Safe shutdown
docker-compose down     # ✅ KEEPS VOLUMES
```

**Recovery:**
```bash
# Restore from backup
cp backups/db_backup_latest.sqlite dev_assets/db.sqlite
```

---

## 📚 Additional Resources

- **SQLite Backup**: https://www.sqlite.org/backup.html
- **Docker Volumes**: https://docs.docker.com/storage/volumes/
- **Docker Compose Volumes**: https://docs.docker.com/compose/compose-file/07-volumes/
- **Database Migration Best Practices**: https://www.prisma.io/docs/guides/migrate

---

## 🎯 Summary

**Key Takeaways:**

1. ✅ **Use volumes** for all persistent data (database, repos, uploads)
2. ✅ **Never bake data into images** (only code and dependencies)
3. ✅ **Match UID/GID** for development to avoid permission issues
4. ✅ **Backup regularly** (automated daily + manual before changes)
5. ✅ **Test restoration** regularly (backup is useless if restore doesn't work)
6. ✅ **Use transactions** for schema migrations
7. ✅ **Monitor disk usage** and database growth

**Your data is safe when:**
- Volume mounts are configured correctly ✅
- Backups are automated and tested ✅
- Permissions are set correctly ✅
- Images don't contain data ✅
