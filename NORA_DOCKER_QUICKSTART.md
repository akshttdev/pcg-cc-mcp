# NORA AI Assistant - Docker Quick Start

## 🚀 5-Minute Setup Guide

This guide gets NORA AI Assistant running in your Docker deployment.

---

## Prerequisites

- Docker and Docker Compose installed
- OpenAI API key ([Get one here](https://platform.openai.com))
- Cloudflare account (optional, for external access)

---

## Step 1: Configure Environment

```bash
# Navigate to project directory
cd pcg-cc-mcp

# Copy environment template
cp .env.example .env

# Edit environment file
nano .env  # or use your preferred editor
```

**Add these REQUIRED variables to `.env`:**

```bash
# Required: OpenAI API Key for NORA
OPENAI_API_KEY=sk-your-openai-api-key-here

# Optional: Cloudflare Tunnel (for external access)
CLOUDFLARE_TUNNEL_TOKEN=eyJhIjoiYourCloudflareToken...

# Optional: NORA Configuration (uses defaults if not set)
NORA_LLM_MODEL=gpt-4-turbo
NORA_LLM_TEMPERATURE=0.7
NORA_LLM_MAX_TOKENS=2000

# These are already in .env.example (no changes needed)
HOST=0.0.0.0
BACKEND_PORT=3001
FRONTEND_PORT=3000
RUST_LOG=info
DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
```

---

## Step 2: Update docker-compose.yml

Add NORA environment variables to your `docker-compose.yml`:

```yaml
services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
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
    volumes:
      - ./dev_assets:/app/dev_assets
      - repos_data:/repos
    ports:
      - "3001:3001"
```

---

## Step 3: Build and Deploy

```bash
# Build Docker images
docker-compose build

# Start containers
docker-compose up -d

# Verify containers are running
docker-compose ps

# Check logs (look for "Nora initialized")
docker-compose logs -f app | grep -i nora
```

**Expected output:**
```
app | INFO  Initializing Nora executive assistant
app | INFO  Nora initialized successfully
app | INFO  LLM client configured: gpt-4-turbo
```

---

## Step 4: Access NORA

### Local Access (Development)

```
http://localhost:3001/nora
```

### External Access (Production with Cloudflare)

```
https://your-subdomain.yourdomain.com/nora
```

---

## Using NORA

### Via Web UI (Recommended)

1. **Open your browser**: Navigate to `/nora`
2. **Auto-initialization**: NORA initializes automatically
3. **Start chatting**: Type in the chat box at the bottom
4. **Features available**:
   - **Chat Tab**: Main conversation interface
   - **Coordination Tab**: Multi-agent stats
   - **Voice Tab**: Voice settings (if enabled)
   - **Analytics Tab**: Usage metrics

### Via API (Programmatic)

```bash
# Test NORA initialization
docker-compose exec app sh -c "curl -X POST http://localhost:3001/api/nora/initialize \
  -H 'Content-Type: application/json' \
  -d '{\"activateImmediately\": true}'"

# Send a chat message
docker-compose exec app sh -c "curl -X POST http://localhost:3001/api/nora/chat \
  -H 'Content-Type: application/json' \
  -d '{
    \"message\": \"What are our active projects?\",
    \"sessionId\": \"test-123\",
    \"voiceEnabled\": false
  }'"
```

---

## Verification Checklist

✅ **Environment Variables Set**
```bash
# Check if OPENAI_API_KEY is set
docker-compose exec app sh -c 'echo $OPENAI_API_KEY'
# Should output: sk-...
```

✅ **NORA Initialized**
```bash
# Check NORA status
curl http://localhost:3001/api/nora/status
# Should return: {"isActive": true, ...}
```

✅ **UI Accessible**
- Open `http://localhost:3001/nora` in browser
- Should see "Nora Executive Assistant" page
- Chat interface should be visible

✅ **Chat Working**
- Type "Hello" in chat box
- Should receive response within 2-3 seconds
- Response should be conversational and British-styled

---

## Troubleshooting

### NORA Not Responding

**Problem**: Chat sends but no response

**Solution**:
```bash
# Check OpenAI API key
docker-compose exec app sh -c 'echo $OPENAI_API_KEY'

# Check logs for errors
docker-compose logs app | grep -i "openai\|nora\|error"

# Restart containers
docker-compose restart app
```

### "Nora not initialized" Error

**Solution**:
```bash
# Manually initialize NORA
curl -X POST http://localhost:3001/api/nora/initialize \
  -H "Content-Type: application/json" \
  -d '{"activateImmediately": true}'

# Or refresh the /nora page in browser
```

### OpenAI API Quota Exceeded

**Problem**: Error message about quota/billing

**Solution**:
1. Check OpenAI account billing: https://platform.openai.com/account/billing
2. Verify API key has quota available
3. Consider using GPT-3.5-turbo for lower costs:
   ```bash
   # In .env file
   NORA_LLM_MODEL=gpt-3.5-turbo
   ```

### Slow Responses

**Problem**: NORA takes >10 seconds to respond

**Check**:
```bash
# View response times in logs
docker-compose logs app | grep "processing_time_ms"

# Check cache stats
curl http://localhost:3001/api/nora/cache/stats
```

**Solutions**:
- Cache is warming up (responses will get faster)
- Check internet connection
- Verify OpenAI API is not rate-limiting

---

## Advanced Configuration

### Change LLM Model

```bash
# In .env file
NORA_LLM_MODEL=gpt-4-turbo      # Best quality
NORA_LLM_MODEL=gpt-4             # High quality
NORA_LLM_MODEL=gpt-3.5-turbo    # Faster, cheaper
```

### Adjust Response Style

```bash
# More creative responses (0.0 = deterministic, 1.0 = creative)
NORA_LLM_TEMPERATURE=0.9

# Shorter responses
NORA_LLM_MAX_TOKENS=500

# Longer, detailed responses
NORA_LLM_MAX_TOKENS=3000
```

### Enable Voice Features

**Voice is FULLY WORKING but disabled by default.**

#### Option 1: Use OpenAI Voice (Included with your API key!)

**Already configured!** Your `OPENAI_API_KEY` enables:
- ✅ OpenAI TTS (Text-to-Speech)
- ✅ OpenAI Whisper STT (Speech-to-Text)

**Test TTS endpoint:**
```bash
curl -X POST http://localhost:3001/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Good morning, this is Nora speaking.",
    "voice_profile": "BritishExecutiveFemale"
  }' > response.json

# Extract audio from response
cat response.json | jq -r '.audio_data' | base64 -d > speech.mp3

# Play audio (macOS)
afplay speech.mp3
```

**Test STT endpoint:**
```bash
# Record audio or use existing .wav file
# Convert to base64
AUDIO_BASE64=$(base64 -i audio.wav)

curl -X POST http://localhost:3001/nora/voice/transcribe \
  -H "Content-Type: application/json" \
  -d "{\"audio_data\": \"$AUDIO_BASE64\"}" | jq .
```

#### Option 2: Premium British Voices (ElevenLabs)

For the **best British executive voices** (Rachel, Brian, Bella, Charlie):

1. **Get ElevenLabs API key**: https://elevenlabs.io/
2. **Update `.env`**:
   ```bash
   ELEVENLABS_API_KEY=your-elevenlabs-key-here
   ```
3. **Update `docker-compose.yml`** (already done if you copied from Step 2):
   ```yaml
   - ELEVENLABS_API_KEY=${ELEVENLABS_API_KEY:-}
   ```
4. **Restart**:
   ```bash
   docker-compose restart app
   ```

Voice will automatically use ElevenLabs (higher quality) if API key is present!

#### Option 3: Enterprise Azure Speech

For **enterprise deployments**:

1. **Get Azure credentials**: https://portal.azure.com/
2. **Update `.env`**:
   ```bash
   AZURE_SPEECH_KEY=your-azure-key-here
   AZURE_SPEECH_REGION=eastus
   ```
3. **Restart**:
   ```bash
   docker-compose restart app
   ```

#### Voice Endpoints

Once configured, these endpoints work:

- **POST `/nora/voice/synthesize`** - Text-to-Speech
- **POST `/nora/voice/transcribe`** - Speech-to-Text
- **POST `/nora/voice/interaction`** - Full voice conversation
- **GET `/nora/voice/config`** - Voice configuration

**See `NORA_VOICE_STATUS_REPORT.md` for complete voice documentation!**

---

## Production Recommendations

### Security

```bash
# Never commit .env to git
echo ".env" >> .gitignore

# Rotate API keys regularly
# Use Cloudflare Access for authentication
```

### Performance

```yaml
# In docker-compose.yml, add resource limits
deploy:
  resources:
    limits:
      cpus: '2'
      memory: 4G
```

### Monitoring

```bash
# Check cache hit rate
curl http://localhost:3001/api/nora/cache/stats

# View Prometheus metrics
curl http://localhost:3001/metrics | grep nora

# Monitor logs
docker-compose logs -f app --tail=100
```

### Cost Optimization

1. **Use cache effectively**: Hit rate should be >60%
2. **Choose right model**: GPT-3.5-turbo is 10x cheaper
3. **Limit token usage**: Set `NORA_LLM_MAX_TOKENS=1000`
4. **Monitor usage**: Check OpenAI dashboard daily

---

## Quick Commands Reference

```bash
# Start everything
docker-compose up -d

# Stop everything
docker-compose down

# Restart NORA
docker-compose restart app

# View logs
docker-compose logs -f app

# Check NORA status
curl http://localhost:3001/api/nora/status

# Clear NORA cache
curl -X POST http://localhost:3001/api/nora/cache/clear

# Access database
docker-compose exec app sh
sqlite3 /app/dev_assets/db.sqlite

# Check environment
docker-compose exec app env | grep NORA
```

---

## What You Get

### NORA Features (All Working)

✅ **Conversational AI** - GPT-4 powered British executive assistant  
✅ **Project Awareness** - Live database access to all projects  
✅ **Tool Execution** - 13 integrated tools (file, web, code, email, calendar)  
✅ **Response Streaming** - Real-time SSE streaming  
✅ **Smart Caching** - LRU cache with Prometheus metrics  
✅ **Rate Limiting** - 20 req/min chat, 30 req/min voice  
✅ **Voice Synthesis** - OpenAI TTS (if enabled)  
✅ **Voice Recognition** - OpenAI Whisper (if enabled)  
✅ **Multi-Agent Coordination** - Task orchestration  
✅ **Executive Context** - Priorities, milestones, budgets tracking  

### UI Components

- **Chat Interface**: Full conversation history, markdown support
- **Coordination Panel**: Multi-agent stats, task tracking
- **Voice Controls**: TTS/STT settings
- **Analytics Dashboard**: Usage metrics, cache stats

---

## Support & Documentation

- **Full Guide**: See `NORA_USAGE_GUIDE.md`
- **Docker Guide**: See `DOCKER_DEPLOYMENT.md`
- **Code**: `crates/nora/src/` (Rust backend)
- **Frontend**: `frontend/src/components/nora/` (React components)
- **API Routes**: `crates/server/src/routes/nora.rs`

**Tests Passing**: All 42 tests ✅

---

## Next Steps

Once NORA is running:

1. **Explore the UI**: Try different tabs and features
2. **Test Chat**: Ask about projects, tasks, milestones
3. **Try Tools**: Use file operations, code analysis
4. **Monitor Metrics**: Check `/metrics` endpoint
5. **Customize**: Adjust temperature, model, max tokens

**Need Help?** Check logs: `docker-compose logs -f app | grep -i nora`

---

**Ready to deploy? Follow the 4 steps above and NORA will be live in 5 minutes!** 🚀
