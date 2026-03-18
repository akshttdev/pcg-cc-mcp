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

# Edit .env and add your tokens
nano .env
```

Add your required tokens and API keys:

```env
# Cloudflare Tunnel Token (for external access)
CLOUDFLARE_TUNNEL_TOKEN=eyJhIjoiYourActualTokenHere...

# OpenAI API Key (REQUIRED for NORA AI Assistant)
OPENAI_API_KEY=sk-your-openai-api-key-here

# Optional: NORA Configuration
NORA_LLM_MODEL=gpt-4-turbo
NORA_LLM_TEMPERATURE=0.7
NORA_LLM_MAX_TOKENS=2000

# Application Settings (already in .env.example)
HOST=0.0.0.0
BACKEND_PORT=3001
FRONTEND_PORT=3000
RUST_LOG=info
DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
```

**Note**: The OpenAI API key is required if you want to use the NORA AI Assistant feature. Without it, NORA won't be able to generate responses.

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

**Available Routes:**
- **Dashboard**: `/` - Main project dashboard
- **Projects**: `/projects` - Project management
- **NORA Assistant**: `/nora` - AI Executive Assistant (requires OpenAI API key)
- **Settings**: `/settings` - Application settings

**NORA AI Assistant:**
- Navigate to `/nora` in your browser
- NORA auto-initializes on first page load
- Start chatting immediately - no additional setup needed!
- Features: Chat, Voice, Coordination, Analytics tabs

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

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `CLOUDFLARE_TUNNEL_TOKEN` | Cloudflare tunnel token | - | Yes (for external access) |
| `OPENAI_API_KEY` | OpenAI API key for NORA Assistant | - | **Yes (for NORA)** |
| `HOST` | Bind address | `0.0.0.0` | No |
| `BACKEND_PORT` | Backend port | `3001` | No |
| `FRONTEND_PORT` | Frontend port | `3000` | No |
| `RUST_LOG` | Log level | `info` | No |
| `DATABASE_URL` | SQLite database path | `sqlite:///app/dev_assets/db.sqlite` | No |
| `NORA_LLM_MODEL` | LLM model for NORA | `gpt-4-turbo` | No |
| `NORA_LLM_TEMPERATURE` | NORA response creativity (0.0-1.0) | `0.7` | No |
| `NORA_LLM_MAX_TOKENS` | Max tokens per NORA response | `1500` | No |
| `TWILIO_ACCOUNT_SID` | Twilio Account SID | - | No (for phone) |
| `TWILIO_AUTH_TOKEN` | Twilio Auth Token | - | No (for phone) |
| `TWILIO_PHONE_NUMBER` | Twilio virtual phone number | - | No (for phone) |
| `TWILIO_WEBHOOK_BASE_URL` | Your server's public URL | - | No (for phone) |
| `TWILIO_SPEECH_LANGUAGE` | Speech recognition language | `en-GB` | No |
| `TWILIO_TTS_VOICE` | TTS voice for responses | `Polly.Amy` | No |

### Getting API Keys

**OpenAI API Key** (for NORA):
1. Go to [platform.openai.com](https://platform.openai.com)
2. Sign up or log in
3. Navigate to API Keys
4. Create new secret key
5. Copy and add to `.env` as `OPENAI_API_KEY`

**Cloudflare Tunnel Token** (for external access):
1. Go to [Cloudflare Zero Trust Dashboard](https://one.dash.cloudflare.com/)
2. Navigate to Networks > Tunnels
3. Create a tunnel
4. Copy the token
5. Add to `.env` as `CLOUDFLARE_TUNNEL_TOKEN`

## Twilio Phone Integration (Optional)

Enable users to call NORA on a real phone number using Twilio.

### Setting Up Twilio

1. **Create a Twilio Account**
   - Go to [console.twilio.com](https://console.twilio.com/)
   - Sign up for a free trial or paid account
   - Note your **Account SID** and **Auth Token** from the dashboard

2. **Get a Phone Number**
   - In Twilio Console, go to **Phone Numbers** > **Manage** > **Buy a Number**
   - Choose a number with Voice capability
   - Note the phone number (e.g., `+15551234567`)

3. **Configure Webhooks**
   
   In the Twilio Console, configure your phone number's Voice settings:
   
   | Setting | Value |
   |---------|-------|
   | **A CALL COMES IN** | Webhook: `https://your-domain.com/api/twilio/voice` (POST) |
   | **PRIMARY HANDLER FAILS** | Webhook: `https://your-domain.com/api/twilio/fallback` (POST) |
   | **CALL STATUS CHANGES** | Webhook: `https://your-domain.com/api/twilio/status` (POST) |

4. **Update Environment Variables**
   
   Add to your `.env` file:
   ```env
   TWILIO_ACCOUNT_SID=ACxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
   TWILIO_AUTH_TOKEN=your_auth_token_here
   TWILIO_PHONE_NUMBER=+15551234567
   TWILIO_WEBHOOK_BASE_URL=https://your-domain.com
   ```

5. **Restart the Application**
   ```bash
   docker-compose down
   docker-compose up -d
   ```

### Testing Phone Calls

1. Call your Twilio phone number
2. NORA will greet you: "Hello, this is Nora, your Executive AI Assistant. How may I assist you today?"
3. Speak your question or request
4. NORA will respond using Twilio's text-to-speech (British Amy voice)
5. Say "goodbye" or "thank you" to end the call

### Twilio Configuration Options

| Variable | Description | Default |
|----------|-------------|---------|
| `TWILIO_SPEECH_LANGUAGE` | Speech recognition language | `en-GB` (British English) |
| `TWILIO_TTS_VOICE` | Text-to-speech voice | `Polly.Amy` (British female) |
| `TWILIO_MAX_CALL_DURATION` | Maximum call length (seconds) | `3600` (1 hour) |
| `TWILIO_RECORDING_ENABLED` | Enable call recording | `false` |
| `TWILIO_GREETING_MESSAGE` | Custom greeting message | Default NORA greeting |

### Available Voices

Twilio supports Amazon Polly voices. Recommended British voices:
- `Polly.Amy` - British female (default)
- `Polly.Emma` - British female
- `Polly.Brian` - British male

### Troubleshooting Phone Calls

**Calls go to voicemail or fail:**
- Verify your Cloudflare tunnel is running and accessible
- Check webhook URLs are correctly configured in Twilio
- Ensure `TWILIO_WEBHOOK_BASE_URL` matches your public URL

**NORA doesn't respond:**
- Check that NORA is initialized: `curl https://your-domain.com/api/nora/status`
- Verify OpenAI API key is configured

**Check Twilio health:**
```bash
curl https://your-domain.com/api/twilio/health
```

**View call logs:**
- Twilio Console > Monitor > Logs > Calls
- Application logs: `docker-compose logs app | grep -i twilio`

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
