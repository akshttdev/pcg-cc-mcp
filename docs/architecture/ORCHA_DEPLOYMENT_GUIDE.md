# ORCHA Deployment Guide

**Updated**: February 13, 2026
**Branch**: `sirak`
**Status**: Production Ready

---

## 🚀 Quick Start

### Start ORCHA Server

```bash
cd /home/pythia/pcg-cc-mcp

# Set ORCHA configuration
export ORCHA_CONFIG=orcha_config.toml

# Start the server
cargo run --release --bin server
```

The server will:
1. Load ORCHA configuration from `orcha_config.toml`
2. Connect to the shared authentication database
3. Route authenticated users to their Topsi instances
4. Serve on `http://localhost:3000` (or configured port)

---

## 📋 Prerequisites

### 1. Database Setup

**Shared Authentication Database** (required):
```bash
# Location: ~/.local/share/duck-kanban/db.sqlite
# Contains: users, sessions, device_registry
```

**Per-User Topsi Databases**:
```bash
# Admin
/home/pythia/.local/share/pcg/data/admin/topsi.db ✅

# Sirak (on remote device)
/home/sirak/topos/.topsi/db.sqlite

# Bonomotion (on remote device)
/home/bonomotion/.local/share/pcg/data/bonomotion/topsi.db
```

### 2. Environment Variables

```bash
# ORCHA Configuration
export ORCHA_CONFIG=orcha_config.toml

# APN Network (if using)
export APN_RELAY_URL=nats://nonlocal.info:4222
export APN_ENABLED=true

# Optional: Override database paths
export DATABASE_URL=sqlite:///home/pythia/.local/share/duck-kanban/db.sqlite
```

### 3. Device Registry

Ensure devices are registered:
```bash
cd /home/pythia/pcg-cc-mcp
python3 setup_device_registry.py
```

Expected devices:
- pythia-master-node-001 (admin primary)
- space-terminal-001 (admin secondary)
- sirak-studios-laptop-001 (Sirak primary)
- apn-cloud-sirak-001 (Sirak backup)
- bonomotion-device-001 (Bonomotion primary)

---

## 🔧 Configuration

### orcha_config.toml

**Location**: `/home/pythia/pcg-cc-mcp/orcha_config.toml`

**Structure**:
```toml
[orcha]
name = "ORCHA"
version = "1.0.0"

[apn]
relay_url = "nats://nonlocal.info:4222"
enabled = true

[[users]]
username = "admin"
primary_device = "pythia-master-node-001"
secondary_devices = ["space-terminal-001"]
topsi_db_path = "/home/pythia/.local/share/pcg/data/admin/topsi.db"
projects_path = "/home/pythia/topos"
```

**Modify for your environment**:
- Update database paths
- Change APN relay URL if needed
- Add/remove users and devices
- Configure routing strategies

---

## 🖥️ Systemd Service (Recommended)

### Create Service File

```bash
sudo nano /etc/systemd/system/orcha.service
```

```ini
[Unit]
Description=ORCHA Orchestration Application
After=network.target

[Service]
Type=simple
User=pythia
WorkingDirectory=/home/pythia/pcg-cc-mcp
Environment="ORCHA_CONFIG=orcha_config.toml"
Environment="RUST_LOG=info"
ExecStart=/home/pythia/pcg-cc-mcp/target/release/server
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Enable and Start

```bash
sudo systemctl daemon-reload
sudo systemctl enable orcha
sudo systemctl start orcha
sudo systemctl status orcha
```

### View Logs

```bash
# Follow logs
sudo journalctl -u orcha -f

# View recent logs
sudo journalctl -u orcha -n 100
```

---

## 🌐 Nginx Reverse Proxy (Optional)

### Configuration

```nginx
# /etc/nginx/sites-available/orcha

upstream orcha_backend {
    server 127.0.0.1:3000;
}

server {
    listen 80;
    server_name orcha.local dashboard.local;

    location / {
        proxy_pass http://orcha_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

```bash
sudo ln -s /etc/nginx/sites-available/orcha /etc/nginx/sites-enabled/
sudo nginx -t
sudo systemctl reload nginx
```

---

## 👥 User Management

### Add New User

1. **Add to shared database**:
```bash
python3 add_users.py  # or use SQL directly
```

2. **Register their device**:
```bash
python3 setup_device_registry.py  # Update script with new device
```

3. **Update orcha_config.toml**:
```toml
[[users]]
username = "newuser"
primary_device = "newuser-device-001"
topsi_db_path = "/home/newuser/.local/share/pcg/data/newuser/topsi.db"
projects_path = "/home/newuser/topos"
```

4. **Initialize their Topsi database**:
```bash
# On their device
./init_user_topsi_databases.sh  # Modify for new user
```

5. **Restart ORCHA**:
```bash
sudo systemctl restart orcha
```

---

## 🔍 Testing & Verification

### Test Routing

```bash
cd /home/pythia/pcg-cc-mcp
./test_orcha_routing.sh
```

### Manual Verification

```bash
# Check ORCHA config loads
cargo test --package server --lib orcha_routing::tests

# Check device registry
sqlite3 ~/.local/share/duck-kanban/db.sqlite \
  "SELECT device_name, device_type FROM device_registry;"

# Check admin's Topsi
sqlite3 /home/pythia/.local/share/pcg/data/admin/topsi.db \
  "SELECT name FROM projects;"

# Test authentication
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'
```

---

## 🐛 Troubleshooting

### ORCHA won't start

**Check configuration**:
```bash
# Verify TOML syntax
cat orcha_config.toml | grep -v "^#" | grep -v "^$"

# Check database paths exist
ls -lh /home/pythia/.local/share/pcg/data/admin/topsi.db
ls -lh ~/.local/share/duck-kanban/db.sqlite
```

**Check logs**:
```bash
sudo journalctl -u orcha -n 50
```

### User can't authenticate

**Verify user exists**:
```bash
sqlite3 ~/.local/share/duck-kanban/db.sqlite \
  "SELECT username, is_active FROM users WHERE username='admin';"
```

**Check password hash**:
```bash
python3 test_login.py  # Verify bcrypt hashes
```

### Routing not working

**Check device online status**:
```bash
sqlite3 ~/.local/share/duck-kanban/db.sqlite \
  "SELECT device_name, is_online FROM device_registry;"
```

**Update device status**:
```bash
sqlite3 ~/.local/share/duck-kanban/db.sqlite \
  "UPDATE device_registry SET is_online=1 WHERE device_name='Pythia Master Node';"
```

### Topsi database not found

**Initialize database**:
```bash
./init_user_topsi_databases.sh
```

**Check path in config**:
```bash
grep "topsi_db_path" orcha_config.toml
```

---

## 🔐 Security

### Database Permissions

```bash
# Topsi databases should be user-readable only
chmod 600 /home/pythia/.local/share/pcg/data/admin/topsi.db
chown pythia:pythia /home/pythia/.local/share/pcg/data/admin/topsi.db

# Projects directory
chmod 755 /home/pythia/topos
chown -R pythia:pythia /home/pythia/topos
```

### Firewall (if exposing externally)

```bash
# Allow HTTP (Nginx)
sudo ufw allow 80/tcp

# Allow HTTPS (if using SSL)
sudo ufw allow 443/tcp

# Block direct access to ORCHA port
sudo ufw deny 3000/tcp
```

### SSL/TLS (Production)

```bash
# Install certbot
sudo apt install certbot python3-certbot-nginx

# Get certificate
sudo certbot --nginx -d orcha.yourdomain.com

# Auto-renew
sudo systemctl enable certbot.timer
```

---

## 📊 Monitoring

### Health Check

```bash
# Check server is running
curl http://localhost:3000/api/health

# Check routing status
curl http://localhost:3000/api/orcha/status
```

### Performance Monitoring

```bash
# CPU/Memory usage
htop -p $(pgrep -f "server")

# Database size
du -sh ~/.local/share/duck-kanban/db.sqlite
du -sh /home/pythia/.local/share/pcg/data/admin/topsi.db

# Connection pool stats
# Check logs for database connection info
grep "database" /var/log/syslog | tail -20
```

---

## 🔄 Updates & Maintenance

### Update ORCHA

```bash
cd /home/pythia/pcg-cc-mcp

# Pull latest changes
git pull origin sirak

# Rebuild
cargo build --release --bin server

# Restart service
sudo systemctl restart orcha
```

### Database Migrations

```bash
# Backup before migration
cp ~/.local/share/duck-kanban/db.sqlite{,.backup}
cp /home/pythia/.local/share/pcg/data/admin/topsi.db{,.backup}

# Run migrations (automatic on startup)
# Or manually:
sqlx migrate run --source ./crates/db/migrations
```

### Backup Strategy

```bash
# Daily backup script
cat > /home/pythia/backup_orcha.sh <<'EOF'
#!/bin/bash
BACKUP_DIR=/home/pythia/backups/orcha
mkdir -p $BACKUP_DIR
DATE=$(date +%Y%m%d_%H%M%S)

# Backup databases
cp ~/.local/share/duck-kanban/db.sqlite \
   $BACKUP_DIR/shared_db_$DATE.sqlite

cp /home/pythia/.local/share/pcg/data/admin/topsi.db \
   $BACKUP_DIR/admin_topsi_$DATE.db

# Keep only last 7 days
find $BACKUP_DIR -name "*.sqlite" -mtime +7 -delete
find $BACKUP_DIR -name "*.db" -mtime +7 -delete
EOF

chmod +x /home/pythia/backup_orcha.sh

# Add to crontab
crontab -e
# Add: 0 2 * * * /home/pythia/backup_orcha.sh
```

---

## 📞 Support

**Documentation**:
- `ORCHA_IMPLEMENTATION_COMPLETE.md` - Full technical details
- `ORCHA_DEPLOYMENT_SUMMARY.md` - Quick reference
- `SOVEREIGN_ARCHITECTURE.md` - Architecture design

**GitHub**: https://github.com/KingBodhi/pcg-cc-mcp/tree/sirak

**Logs**:
```bash
# System logs
sudo journalctl -u orcha

# Application logs
tail -f /var/log/orcha.log  # If configured
```

---

**Last Updated**: February 13, 2026
**Author**: Claude Sonnet 4.5
**Branch**: `sirak`
