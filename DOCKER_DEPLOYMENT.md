# Docker Deployment Guide

This guide explains how to deploy the PCG-CC-MCP application using Docker with Cloudflare Tunnel for secure port forwarding.

## Prerequisites

- Docker and Docker Compose installed
- A Cloudflare account (free tier works)
- Basic command line knowledge

## Quick Start

### 1. Set Up Cloudflare Tunnel

1. Go to [Cloudflare Zero Trust Dashboard](https://one.dash.cloudflare.com/)
2. Navigate to **Networks** > **Tunnels**
3. Click **Create a tunnel**
4. Choose **Cloudflared** as the connector
5. Name your tunnel (e.g., `pcg-cc-mcp`)
6. Copy the tunnel token (starts with `eyJ...`)
7. Configure the tunnel:
   - **Public Hostname**: Choose your subdomain (e.g., `pcg.yourdomain.com`)
   - **Service**: `http://app:3001`
8. Save the tunnel

### 2. Configure Environment

```bash
# Copy the example environment file
cp .env.example .env

# Edit .env and add your Cloudflare tunnel token
nano .env
```

Add your tunnel token:
```env
CLOUDFLARE_TUNNEL_TOKEN=eyJhIjoiYourActualTokenHere...
```

### 3. Build and Run

```bash
# Build the Docker image
docker-compose build

# Start the services
docker-compose up -d

# Check logs
docker-compose logs -f
```

### 4. Access Your Application

Your application will be accessible at the URL you configured in Cloudflare Tunnel (e.g., `https://pcg.yourdomain.com`).

## Commands

### Start Services
```bash
docker-compose up -d
```

### Stop Services
```bash
docker-compose down
```

### View Logs
```bash
# All services
docker-compose logs -f

# Just the app
docker-compose logs -f app

# Just cloudflared
docker-compose logs -f cloudflared
```

### Rebuild After Changes
```bash
docker-compose down
docker-compose build --no-cache
docker-compose up -d
```

### Access Database
```bash
docker-compose exec app sh
sqlite3 /app/dev_assets/db.sqlite
```

## Volumes

The following data is persisted:

- **Database**: `./dev_assets/db.sqlite` (mounted from host)
- **Repositories**: `repos_data` (Docker volume)

## Backup

### Backup Database
```bash
docker-compose exec app sh -c "sqlite3 /app/dev_assets/db.sqlite .dump" > backup.sql
```

### Restore Database
```bash
cat backup.sql | docker-compose exec -T app sh -c "sqlite3 /app/dev_assets/db.sqlite"
```

## Ports

- **3001**: Backend API (internal)
- External access is handled by Cloudflare Tunnel (no ports exposed)

## Security Notes

1. **Never commit `.env`** - It contains your tunnel token
2. **Use strong passwords** for admin users
3. **Keep Docker images updated**: `docker-compose pull && docker-compose up -d`
4. **Database backups** should be automated
5. **Cloudflare Access** policies can add extra authentication

## Troubleshooting

### Application won't start
```bash
# Check logs
docker-compose logs app

# Verify database permissions
ls -la dev_assets/
```

### Cloudflared connection issues
```bash
# Check cloudflared logs
docker-compose logs cloudflared

# Verify tunnel token
echo $CLOUDFLARE_TUNNEL_TOKEN
```

### Cannot access via Cloudflare URL
1. Verify tunnel is running: Check Cloudflare dashboard
2. Ensure tunnel points to `http://app:3001`
3. Check app health: `docker-compose ps`

### Database locked errors
```bash
# Stop all containers
docker-compose down

# Start fresh
docker-compose up -d
```

## Production Recommendations

1. **Use a reverse proxy** (nginx) if not using Cloudflare Tunnel
2. **Set up monitoring** (Prometheus + Grafana)
3. **Implement log rotation**
4. **Regular backups** (automated daily)
5. **SSL certificates** (Cloudflare Tunnel handles this)
6. **Resource limits** in docker-compose.yml:
   ```yaml
   deploy:
     resources:
       limits:
         cpus: '2'
         memory: 4G
   ```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `CLOUDFLARE_TUNNEL_TOKEN` | Cloudflare tunnel token | Required |
| `HOST` | Bind address | `0.0.0.0` |
| `BACKEND_PORT` | Backend port | `3001` |
| `FRONTEND_PORT` | Frontend port | `3000` |
| `RUST_LOG` | Log level | `info` |
| `DATABASE_URL` | SQLite database path | `sqlite:///app/dev_assets/db.sqlite` |

## Docker Hub Publishing (Optional)

If you want to publish the image to Docker Hub:

```bash
# Build for multiple platforms
docker buildx build --platform linux/amd64,linux/arm64 -t yourusername/pcg-cc-mcp:latest --push .

# Users can then pull:
docker pull yourusername/pcg-cc-mcp:latest
```

## Support

For issues, check:
- Application logs: `docker-compose logs app`
- Cloudflare tunnel status: Cloudflare dashboard
- Health check: `docker-compose ps`
