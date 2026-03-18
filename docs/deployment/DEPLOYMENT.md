# PCG-CC-MCP Dashboard - Docker Deployment Guide

Complete guide for deploying the PCG-CC-MCP dashboard using Docker with Cloudflare Tunnel integration.

## Table of Contents

- [Quick Start](#quick-start)
- [Architecture](#architecture)
- [Prerequisites](#prerequisites)
- [Configuration](#configuration)
- [Deployment](#deployment)
- [Access](#access)
- [Management](#management)
- [Troubleshooting](#troubleshooting)
- [Advanced Configuration](#advanced-configuration)

---

## Quick Start

Get up and running in 3 commands:

```bash
# 1. Setup environment
./deploy.sh setup

# 2. Deploy application
./deploy.sh deploy

# 3. Check status
./deploy.sh status
```

Or using Make:

```bash
make setup
make deploy
make status
```

---

## Architecture

The deployment consists of 5 Docker services:

```
┌─────────────────────────────────────────┐
│      CLOUDFLARE TUNNEL (cloudflared)    │
│  Secure external access via tunnel      │
└─────────────────────────────┬───────────┘
                              │
┌─────────────────────────────▼───────────┐
│      NGINX (Reverse Proxy)              │
│  - Routes traffic to services           │
│  - Serves frontend static files         │
│  - WebSocket/SSE support                │
│  - Security headers & compression       │
└──────┬──────────────────────┬───────────┘
       │                      │
┌──────▼──────┐      ┌────────▼────────┐
│ APP Service │      │ APN Bridge      │
│ (Rust)      │      │ (Python FastAPI)│
│ - Main API  │      │ - Alpha Proto   │
│ - Ollama    │      │ - Mesh network  │
│ - Chatterbox│      │                 │
└──────┬──────┘      └─────────────────┘
       │
┌──────▼──────────────────────────────┐
│  DB Backup Service (Alpine)         │
│  - Automated daily backups          │
│  - 7-day retention                  │
└─────────────────────────────────────┘
```

### Services

1. **app** - Main application (Rust + Axum)
   - Port 3001: REST API
   - Port 11434: Ollama (Local LLM)
   - Port 8100: Chatterbox TTS
   - GPU acceleration support (CUDA)

2. **nginx** - Reverse proxy
   - Port 8080: Public HTTP interface
   - Routes to app and apn-bridge
   - Serves React frontend

3. **apn-bridge** - Alpha Protocol Network bridge
   - Port 8000: APN HTTP API
   - NATS relay connection

4. **cloudflared** - Cloudflare Tunnel
   - Secure external access
   - No port forwarding required

5. **db-backup** - Automated backup service
   - Daily database backups
   - 7-day retention

---

## Prerequisites

### Required

- Docker 20.10+
- Docker Compose 2.0+
- 8GB RAM minimum (16GB recommended)
- 20GB disk space
- OpenAI API key (for NORA AI Assistant)

### Optional

- NVIDIA GPU + nvidia-docker (for GPU acceleration)
- Cloudflare account (for external access)

### Installation

**Docker (Ubuntu/Debian):**
```bash
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER
```

**Docker Compose:**
```bash
sudo apt-get install docker-compose-plugin
```

**NVIDIA Docker Runtime (for GPU):**
```bash
curl -fsSL https://nvidia.github.io/libnvidia-container/gpgkey | sudo gpg --dearmor -o /usr/share/keyrings/nvidia-container-toolkit-keyring.gpg
curl -s -L https://nvidia.github.io/libnvidia-container/stable/deb/nvidia-container-toolkit.list | \
  sed 's#deb https://#deb [signed-by=/usr/share/keyrings/nvidia-container-toolkit-keyring.gpg] https://#g' | \
  sudo tee /etc/apt/sources.list.d/nvidia-container-toolkit.list
sudo apt-get update
sudo apt-get install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker
```

---

## Configuration

### Step 1: Get Cloudflare Tunnel Token

1. Go to [Cloudflare Zero Trust Dashboard](https://one.dash.cloudflare.com/)
2. Navigate to: **Zero Trust** > **Networks** > **Tunnels**
3. Click **Create a tunnel**
4. Choose **Cloudflared** connector
5. Name your tunnel (e.g., `pcg-cc-mcp`)
6. Copy the tunnel token
7. Configure public hostname:
   - **Subdomain**: your-app
   - **Domain**: your-domain.com
   - **Service**: `http://nginx:80`
8. Save the configuration

### Step 2: Get OpenAI API Key

1. Go to [OpenAI Platform](https://platform.openai.com/api-keys)
2. Create a new API key
3. Copy the key (starts with `sk-`)

### Step 3: Run Setup

```bash
./deploy.sh setup
```

This will:
- Create `.env` from `.env.example`
- Prompt for Cloudflare tunnel token
- Prompt for OpenAI API key
- Validate configuration

Or manually edit `.env`:

```bash
cp .env.example .env
nano .env
```

Required settings:
```bash
CLOUDFLARE_TUNNEL_TOKEN=your_token_here
OPENAI_API_KEY=sk-your-key-here
```

---

## Deployment

### Option 1: Interactive Menu

```bash
./deploy.sh
```

This shows an interactive menu with all options.

### Option 2: Command Line

```bash
# Setup
./deploy.sh setup

# Deploy
./deploy.sh deploy

# Or individually
./deploy.sh start    # Start services
./deploy.sh stop     # Stop services
./deploy.sh restart  # Restart services
./deploy.sh status   # Show status
./deploy.sh logs     # View logs
```

### Option 3: Using Make

```bash
make setup
make deploy
make status
make logs
```

### First Deployment

```bash
# Check prerequisites
./deploy.sh check

# Setup environment
./deploy.sh setup

# Deploy (builds and starts)
./deploy.sh deploy
```

Build duration: 10-15 minutes (first time)

---

## Access

### Local Access

After deployment, access the application locally:

- **Web Interface**: http://localhost:8080
- **Main API**: http://localhost:3001
- **Ollama API**: http://localhost:11434
- **Chatterbox TTS**: http://localhost:8100

### External Access

If Cloudflare Tunnel is configured:

1. Check your tunnel status: https://one.dash.cloudflare.com/
2. Your app is accessible at: `https://your-app.your-domain.com`

---

## Management

### Common Operations

```bash
# View status
./deploy.sh status
make status

# View logs (all services)
./deploy.sh logs
make logs

# View specific service logs
make logs-app
make logs-nginx
make logs-apn
make logs-cloudflared

# Restart services
./deploy.sh restart
make restart

# Create database backup
./deploy.sh backup
make backup

# Open shell in container
./deploy.sh shell
make shell

# Update application
./deploy.sh update
make update
```

### Monitoring

```bash
# Container status
docker-compose ps

# Resource usage
docker stats

# Health checks
make health

# Database size
make db-size
```

### Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f app

# Last 100 lines
docker-compose logs --tail=100 -f

# With timestamps
docker-compose logs -f --timestamps
```

### Database

```bash
# Manual backup
./deploy.sh backup

# Open SQLite shell
make db-shell

# Check database size
make db-size

# Restore from backup
docker-compose down
cp backups/backup_20240101_120000.sqlite dev_assets/db.sqlite
docker-compose up -d
```

### Updates

```bash
# Pull latest code and rebuild
./deploy.sh update

# Or manually
git pull
docker-compose down
docker-compose build
docker-compose up -d
```

### Clean Rebuild

```bash
# Complete rebuild from scratch
./deploy.sh clean

# Or with Make
make clean
```

---

## Troubleshooting

### Services Not Starting

Check logs:
```bash
docker-compose logs -f
```

Check service health:
```bash
docker-compose ps
```

Restart services:
```bash
docker-compose restart
```

### Database Issues

**Corrupted database:**
```bash
# Restore from latest backup
docker-compose down
cp backups/backup_latest.sqlite dev_assets/db.sqlite
docker-compose up -d
```

**Check database integrity:**
```bash
docker-compose exec app sqlite3 /app/dev_assets/db.sqlite "PRAGMA integrity_check;"
```

### GPU Issues

**Check GPU availability:**
```bash
nvidia-smi
```

**Test GPU in Docker:**
```bash
docker run --rm --gpus all nvidia/cuda:12.1.0-base-ubuntu22.04 nvidia-smi
```

**If GPU not working:**
```bash
# Install nvidia-container-toolkit
sudo apt-get install nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Rebuild containers
docker-compose down
docker-compose up -d
```

### Cloudflare Tunnel Issues

**Check tunnel status:**
```bash
docker-compose logs cloudflared
```

**Verify token:**
```bash
grep CLOUDFLARE_TUNNEL_TOKEN .env
```

**Test tunnel connection:**
1. Go to: https://one.dash.cloudflare.com/
2. Check tunnel status (should be "Healthy")
3. Verify public hostname configuration

### Port Conflicts

**Port 8080 already in use:**

Edit `docker-compose.yml`:
```yaml
nginx:
  ports:
    - "9080:80"  # Change to different port
```

**Port 3001 already in use:**

Edit `docker-compose.yml`:
```yaml
app:
  ports:
    - "3002:3001"  # Change external port
```

### Out of Disk Space

**Check disk usage:**
```bash
df -h
docker system df
```

**Clean up:**
```bash
# Remove old backups
find backups/ -name "backup_*.sqlite" -mtime +7 -delete

# Prune Docker
make prune

# Deep clean (WARNING: removes everything)
make deep-clean
```

### Build Failures

**Clear Docker cache and rebuild:**
```bash
docker-compose down
docker-compose build --no-cache
docker-compose up -d
```

**Check available disk space:**
```bash
df -h
```

**Check Docker logs:**
```bash
journalctl -u docker -n 100
```

---

## Advanced Configuration

### Custom Environment Variables

Edit `.env` to configure:

**Application Settings:**
```bash
HOST=0.0.0.0
BACKEND_PORT=3001
FRONTEND_PORT=3000
RUST_LOG=info  # debug, info, warn, error
```

**Ollama Configuration:**
```bash
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=deepseek-r1
```

**Chatterbox TTS:**
```bash
CHATTERBOX_PORT=8100
CHATTERBOX_DEVICE=cuda  # or cpu
```

**Backup Settings:**
```bash
BACKUP_INTERVAL=86400      # 24 hours in seconds
BACKUP_RETENTION_DAYS=7    # Keep 7 days
```

**Voice Services (Optional):**
```bash
ELEVENLABS_API_KEY=your_key
AZURE_SPEECH_KEY=your_key
AZURE_SPEECH_REGION=eastus
```

**Email Integration (Optional):**
```bash
GOOGLE_CLIENT_ID=your_id
GOOGLE_CLIENT_SECRET=your_secret
ZOHO_CLIENT_ID=your_id
ZOHO_CLIENT_SECRET=your_secret
```

### Nginx Configuration

Edit `nginx/default.conf` to customize:

- Security headers
- Compression settings
- Timeouts
- Buffer sizes
- Custom routes

After editing:
```bash
docker-compose restart nginx
```

### Resource Limits

Edit `docker-compose.yml` to add resource limits:

```yaml
app:
  deploy:
    resources:
      limits:
        cpus: '4'
        memory: 8G
      reservations:
        cpus: '2'
        memory: 4G
```

### SSL/TLS with Custom Domain

**Using Let's Encrypt:**

1. Install certbot on host
2. Obtain certificate
3. Mount certificate in nginx
4. Update nginx config

Or use Cloudflare Tunnel (automatic SSL/TLS).

### Multiple Environments

Create environment-specific files:

```bash
# Production
docker-compose -f docker-compose.yml --env-file .env.production up -d

# Staging
docker-compose -f docker-compose.yml --env-file .env.staging up -d
```

---

## Security Best Practices

1. **Keep secrets secure**
   - Never commit `.env` to version control
   - Use `.env.example` as template
   - Rotate API keys regularly

2. **Update regularly**
   ```bash
   ./deploy.sh update
   ```

3. **Monitor logs**
   ```bash
   ./deploy.sh logs
   ```

4. **Enable backups**
   - Automated daily backups enabled by default
   - Store backups off-site for disaster recovery

5. **Use Cloudflare Tunnel**
   - No open ports required
   - DDoS protection
   - Automatic SSL/TLS

6. **Limit resource usage**
   - Set memory limits in docker-compose.yml
   - Monitor with `docker stats`

---

## Support

### Documentation

- Main README: `README.md`
- API Documentation: Check `/api/docs` endpoint
- Environment Configuration: `.env.example`

### Logs

Always include logs when reporting issues:

```bash
docker-compose logs > logs.txt
```

### Useful Commands

```bash
# Show all available commands
make help

# Interactive menu
./deploy.sh

# Quick status check
./deploy.sh status

# Health check
make health
```

---

## License

See main project LICENSE file.
