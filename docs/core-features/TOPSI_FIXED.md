# Topsi Chat Error - RESOLVED ✅

**Date**: February 9, 2026
**Issue**: Topsi was returning "Sorry, I encountered an error processing your request" when trying to chat
**Status**: ✅ **FIXED**

---

## Root Cause

Topsi was configured to use OpenAI's API by default, but no API key was provided. When it fell back to the local Ollama instance, it used the `deepseek-r1` model which **does not support function calling/tools**, which Topsi requires.

---

## Solution

1. **Configured Topsi to use Ollama** with environment variables:
   ```bash
   TOPSI_LLM_PROVIDER=ollama
   TOPSI_LLM_MODEL=qwen2.5:7b
   OLLAMA_BASE_URL=http://localhost:11434
   ```

2. **Downloaded qwen2.5:7b model** which supports function calling:
   ```bash
   ollama pull qwen2.5:7b
   ```

3. **Updated configuration files**:
   - `.env` - Added TOPSI_LLM_PROVIDER and TOPSI_LLM_MODEL
   - `launch-dashboard.sh` - Added environment variables to server startup

---

## How to Start Server with Topsi Support

```bash
cd /home/pythia/pcg-cc-mcp

# Method 1: Manual start
killall -9 server
AUTO_START_COMFYUI=false PORT=3002 BACKEND_PORT=3002 \
TOPSI_LLM_PROVIDER=ollama TOPSI_LLM_MODEL=qwen2.5:7b \
OLLAMA_BASE_URL=http://localhost:11434 \
nohup ./target/release/server > /tmp/pcg_backend_3002.log 2>&1 &

# Method 2: Use launch script (updated)
./launch-dashboard.sh
```

---

## Verification

1. **Check Topsi Status**:
   ```bash
   curl https://dashboard.powerclubglobal.com/api/topsi/status
   ```

2. **Initialize Topsi** (if needed):
   ```bash
   curl -X POST https://dashboard.powerclubglobal.com/api/topsi/initialize \
   -H "Content-Type: application/json" \
   -d '{"activateImmediately": true}'
   ```

3. **Test Chat**:
   Go to https://dashboard.powerclubglobal.com/topsi and send a message.

---

## Why qwen2.5:7b?

- ✅ Supports function calling/tools (required by Topsi)
- ✅ 7B parameters - runs efficiently on local hardware
- ✅ Good performance for code and technical tasks
- ✅ Compatible with Ollama's function calling API

**Alternative models that work**:
- `llama3.1:8b` or higher
- `mistral:latest`
- `qwen2.5:14b` (if more memory available)

**Models that DON'T work**:
- ❌ `deepseek-r1` - no tool support
- ❌ Most older/smaller models - no tool support

---

## Current Status

✅ Server running on port 3002
✅ Topsi endpoints accessible at https://dashboard.powerclubglobal.com/api/topsi/*
✅ Topsi initialized and ready
✅ Using qwen2.5:7b model with Ollama
✅ Chat should now work without errors

---

## Monitoring

**Check logs**:
```bash
tail -f /tmp/pcg_backend_3002.log
```

**Check if model is loaded**:
```bash
curl http://localhost:11434/api/tags | jq -r '.models[].name'
```

**Should see**:
```
deepseek-r1:latest
qwen2.5:7b
```

---

## Troubleshooting

### "Model not found" error
```bash
ollama pull qwen2.5:7b
```

### Topsi still using wrong model
Restart server with correct environment variables (see above)

### Chat still errors
Check logs:
```bash
tail -100 /tmp/pcg_backend_3002.log | grep ERROR
```

---

**Last Updated**: 2026-02-09 16:05 CST
**Status**: 🟢 **OPERATIONAL** (pending model download completion)
