# PCG-CC-MCP Quick Start Guide

Get your dashboard running in 5 minutes!

## One-Line Install

```bash
./quick-deploy.sh
```

This interactive script will:
1. Check prerequisites
2. Configure environment
3. Build and deploy
4. Show access URLs

## Manual Quick Start

### 1. Setup

```bash
./deploy.sh setup
```

### 2. Deploy

```bash
./deploy.sh deploy
```

### 3. Access

Open http://localhost:8080 in your browser.

## What You Need

### Required
- Docker & Docker Compose
- OpenAI API key ([get one](https://platform.openai.com/api-keys))

### Optional
- Cloudflare Tunnel token (for external access)
- NVIDIA GPU (for faster AI inference)

## Key Commands

```bash
# Start
./deploy.sh start

# Stop
./deploy.sh stop

# Status
./deploy.sh status

# Logs
./deploy.sh logs

# Help
./deploy.sh
make help
```

## Access Points

After deployment:

- **Dashboard**: http://localhost:8080
- **API**: http://localhost:3001
- **Ollama**: http://localhost:11434
- **Chatterbox TTS**: http://localhost:8100

## External Access (Optional)

### Setup Cloudflare Tunnel

1. Go to [Cloudflare Dashboard](https://one.dash.cloudflare.com/)
2. Navigate to: **Zero Trust** > **Networks** > **Tunnels**
3. Create tunnel, copy token
4. Configure hostname:
   - Service: `http://nginx:80`
5. Add token to `.env`:
   ```bash
   CLOUDFLARE_TUNNEL_TOKEN=your_token_here
   ```
6. Restart: `./deploy.sh restart`

## Troubleshooting

### Services not starting?

```bash
docker-compose logs -f
```

### Need to rebuild?

```bash
./deploy.sh clean
```

### Check health?

```bash
make health
```

## Next Steps

- Read [DEPLOYMENT.md](./DEPLOYMENT.md) for full documentation
- Check [README.md](./README.md) for application features
- View logs: `./deploy.sh logs`
- Create backup: `./deploy.sh backup`

## Support

For issues:
1. Check logs: `./deploy.sh logs`
2. Verify config: `./deploy.sh check`
3. See full guide: [DEPLOYMENT.md](./DEPLOYMENT.md)

---

**Ready to deploy?** Run `./quick-deploy.sh` now!
