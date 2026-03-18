# 🐳 Docker Deployment - Quick Reference

## What's Been Added

Your PCG-CC-MCP project is now fully containerized with the following files:

### Core Files
- **`Dockerfile`** - Multi-stage build (Node + Rust → Alpine runtime)
- **`docker-compose.yml`** - Production setup with Cloudflare Tunnel
- **`docker-compose.local.yml`** - Local testing without tunnel
- **`.env.example`** - Environment template with all required variables

### Documentation
- **`DOCKER_DEPLOYMENT.md`** - Complete deployment guide
- **`CLOUDFLARE_TUNNEL.md`** - Step-by-step Cloudflare Tunnel setup
- **`README.md`** - Updated with Docker deployment section

### Scripts
- **`deploy.sh`** - Interactive deployment script
- **`health-check.sh`** - Health monitoring script

### CI/CD
- **`.github/workflows/docker-publish.yml`** - Auto-build and push to Docker Hub

## Quick Start Commands

### 1. Local Testing (No Cloudflare)
```bash
# Build and run locally
docker-compose -f docker-compose.local.yml build
docker-compose -f docker-compose.local.yml up -d

# Access at http://localhost:3001
```

### 2. Production Deployment with Cloudflare Tunnel

```bash
# Step 1: Get Cloudflare Tunnel token
# Visit: https://one.dash.cloudflare.com/
# Create tunnel, copy token

# Step 2: Configure
cp .env.example .env
# Edit .env and add your CLOUDFLARE_TUNNEL_TOKEN

# Step 3: Deploy
./deploy.sh
# Select option 1

# Step 4: Verify
./health-check.sh
```

### 3. Pull from Docker Hub (When Published)

```bash
# Pull the latest image
docker pull kingbodhi/pcg-cc-mcp:latest

# Or update docker-compose.yml to use the image:
# services:
#   app:
#     image: kingbodhi/pcg-cc-mcp:latest
#     # ... rest of config
```

## Container Architecture

```
┌─────────────────────────────────────────┐
│         Cloudflare Tunnel               │
│    (Secure public access - HTTPS)       │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│         Docker Network                   │
│                                          │
│  ┌────────────────┐  ┌───────────────┐ │
│  │  cloudflared   │  │  app:3001     │ │
│  │  container     │─▶│  PCG-CC-MCP   │ │
│  └────────────────┘  └───┬───────────┘ │
│                          │              │
│                          ▼              │
│                  ┌───────────────┐      │
│                  │  SQLite DB    │      │
│                  │  (Volume)     │      │
│                  └───────────────┘      │
└─────────────────────────────────────────┘
```

## Environment Variables

Required in `.env`:

```env
# Required
CLOUDFLARE_TUNNEL_TOKEN=eyJhIjoiYourTokenHere...

# Optional (defaults provided)
HOST=0.0.0.0
BACKEND_PORT=3001
FRONTEND_PORT=3000
RUST_LOG=info
DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
```

## Data Persistence

Two important volumes:

1. **Database** - Mounted from host: `./dev_assets/db.sqlite`
2. **Repositories** - Docker volume: `repos_data`

### Backup Your Data

```bash
# Backup database
docker-compose exec app sh -c "sqlite3 /app/dev_assets/db.sqlite .dump" > backup-$(date +%Y%m%d).sql

# Backup repositories
docker run --rm -v pcg-cc-mcp_repos_data:/data -v $(pwd):/backup alpine tar czf /backup/repos-backup.tar.gz /data
```

## Ports

- **3001** - Backend API (can be customized via `BACKEND_PORT`)
- External access via Cloudflare Tunnel (no ports exposed to internet)

## Health Checks

The application includes built-in health checks:

```bash
# Check with script
./health-check.sh

# Or manually
curl http://localhost:3001/api/auth/me

# Docker health status
docker-compose ps
```

## Useful Commands

```bash
# View logs
docker-compose logs -f app          # Application logs
docker-compose logs -f cloudflared  # Tunnel logs

# Restart services
docker-compose restart app
docker-compose restart cloudflared

# Stop everything
docker-compose down

# Remove everything (including volumes)
docker-compose down -v

# Rebuild from scratch
docker-compose down
docker-compose build --no-cache
docker-compose up -d

# Access container shell
docker-compose exec app sh

# Check resource usage
docker stats
```

## Troubleshooting

### Container won't start
```bash
docker-compose logs app
docker-compose ps
```

### Cloudflare tunnel not connecting
```bash
docker-compose logs cloudflared
# Verify token in .env
```

### Database permission errors
```bash
chmod -R 755 dev_assets/
docker-compose restart app
```

### Out of disk space
```bash
# Clean up Docker
docker system prune -a
```

## Publishing to Docker Hub

To publish your own image:

1. **Login to Docker Hub**
   ```bash
   docker login
   ```

2. **Build for multiple platforms**
   ```bash
   docker buildx create --use
   docker buildx build --platform linux/amd64,linux/arm64 \
     -t yourusername/pcg-cc-mcp:latest \
     --push .
   ```

3. **Or use GitHub Actions**
   - Add `DOCKER_USERNAME` and `DOCKER_PASSWORD` to GitHub Secrets
   - Push to main branch or create a tag
   - Workflow will auto-build and publish

## Security Checklist

- [ ] Changed default admin password
- [ ] Set strong `CLOUDFLARE_TUNNEL_TOKEN`
- [ ] `.env` file is in `.gitignore` (it is by default)
- [ ] Enabled Cloudflare Access policies (optional but recommended)
- [ ] Set up automated backups
- [ ] Configured log rotation
- [ ] Running as non-root user (default in container)
- [ ] Keep Docker images updated regularly

## Next Steps

1. ✅ Test locally: `docker-compose -f docker-compose.local.yml up`
2. ✅ Set up Cloudflare Tunnel: See `CLOUDFLARE_TUNNEL.md`
3. ✅ Deploy to production: `./deploy.sh`
4. ✅ Configure backups: Set up cron job for database dumps
5. ✅ Monitor health: Add `health-check.sh` to monitoring system
6. ✅ Set up CI/CD: Push to GitHub to trigger auto-builds

## Support & Resources

- **Deployment Guide**: `DOCKER_DEPLOYMENT.md`
- **Cloudflare Setup**: `CLOUDFLARE_TUNNEL.md`
- **Main README**: `README.md`
- **Docker Docs**: https://docs.docker.com/
- **Cloudflare Tunnel Docs**: https://developers.cloudflare.com/cloudflare-one/

---

**Questions?** Check the detailed guides or open an issue on GitHub!
