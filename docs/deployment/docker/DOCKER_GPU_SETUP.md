# 🚀 GPU-Accelerated Docker Setup

## Overview

This Docker setup includes GPU acceleration for:
- **Ollama** - Local LLM inference (GPT-OSS, DeepSeek)
- **Chatterbox TTS** - Local voice synthesis with GPU acceleration

## Prerequisites

### 1. NVIDIA GPU & Drivers
- NVIDIA GPU with CUDA support
- NVIDIA drivers installed on host
- Verify: `nvidia-smi`

### 2. NVIDIA Container Toolkit
```bash
# Install NVIDIA Container Toolkit
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
  sudo tee /etc/apt/sources.list.d/nvidia-docker.list

sudo apt-get update && sudo apt-get install -y nvidia-container-toolkit
sudo systemctl restart docker
```

### 3. Verify GPU Access
```bash
# Test GPU access in Docker
docker run --rm --gpus all nvidia/cuda:12.1.0-base-ubuntu22.04 nvidia-smi
```

## Pre-Installed Models

The Docker image comes with these models pre-pulled:

### Ollama Models
- **gpt-oss:20b** - 20B parameter open source GPT (default)
- **deepseek-chat** - DeepSeek general purpose chat model

### Chatterbox TTS
- **chatterbox-tts** - Installed and ready with Python 3.11

## Quick Start

### Build with GPU Support
```bash
# Build the GPU-enabled image
docker-compose build

# Start all services (Ollama + Chatterbox + Main App)
docker-compose up -d
```

### Check Service Status
```bash
# Check all containers
docker-compose ps

# View logs
docker-compose logs -f app

# Check Ollama service
docker-compose exec app curl http://localhost:11434/api/tags

# Check Chatterbox service
docker-compose exec app curl http://localhost:8100/health
```

## Service Ports

- **3001** - Main application API
- **11434** - Ollama API (local LLM)
- **8100** - Chatterbox TTS API (local voice)

## Configuration

### Environment Variables

Add to your `.env` file:

```bash
# Ollama Configuration
OLLAMA_BASE_URL=http://localhost:11434
OLLAMA_MODEL=gpt-oss:20b  # or deepseek-chat

# Chatterbox Configuration
CHATTERBOX_PORT=8100
CHATTERBOX_DEVICE=cuda  # or cpu for non-GPU
CHATTERBOX_VOICE_REF=  # Optional: path to reference audio for voice cloning

# NORA LLM Model (use Ollama models)
NORA_LLM_MODEL=gpt-oss:20b  # or deepseek-chat
```

## Using Different Models

### Switch to DeepSeek
```bash
# Update .env
NORA_LLM_MODEL=deepseek-chat

# Restart services
docker-compose restart app
```

### Pull Additional Ollama Models
```bash
# Enter container
docker-compose exec app bash

# Pull models
ollama pull deepseek-coder      # For coding tasks
ollama pull deepseek-reasoner   # For reasoning tasks
ollama pull llama3.3            # Meta's Llama
ollama pull mistral             # Mistral AI
ollama pull gpt-oss:120b        # Larger GPT-OSS variant

# List available models
ollama list
```

## Testing Local Services

### Test Ollama
```bash
# Test GPT-OSS
curl http://localhost:11434/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-oss:20b",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'

# Test DeepSeek
curl http://localhost:11434/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "deepseek-chat",
    "messages": [{"role": "user", "content": "Hello!"}]
  }'
```

### Test Chatterbox TTS
```bash
# Generate speech
curl -X POST http://localhost:8100/tts \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello, this is a test of Chatterbox TTS"}' \
  --output test.wav

# Check health
curl http://localhost:8100/health
```

## GPU Memory Management

### Monitor GPU Usage
```bash
# Inside container
nvidia-smi

# Watch continuously
watch -n 1 nvidia-smi
```

### Model Memory Requirements

| Model | VRAM Required | Notes |
|-------|---------------|-------|
| gpt-oss:20b | ~12-16 GB | Default, balanced |
| gpt-oss:120b | ~60-80 GB | High VRAM needed |
| deepseek-chat | ~16-20 GB | Good for general use |
| deepseek-coder | ~16-20 GB | Optimized for code |
| Chatterbox TTS | ~2-4 GB | Runs alongside LLM |

### Running Without GPU

If GPU is unavailable, the services will fall back to CPU:

```bash
# Update docker-compose.yml - remove deploy section:
# deploy:
#   resources:
#     reservations:
#       devices:
#         - driver: nvidia
#           count: all
#           capabilities: [gpu]

# Set CPU mode
CHATTERBOX_DEVICE=cpu
```

## Troubleshooting

### GPU Not Detected
```bash
# Check NVIDIA runtime
docker run --rm --gpus all ubuntu nvidia-smi

# Check docker daemon configuration
cat /etc/docker/daemon.json
# Should include:
# {
#   "runtimes": {
#     "nvidia": {
#       "path": "nvidia-container-runtime",
#       "runtimeArgs": []
#     }
#   }
# }
```

### Ollama Not Starting
```bash
# Check logs
docker-compose logs app | grep -i ollama

# Manually test
docker-compose exec app ollama serve

# Check if port is bound
docker-compose exec app netstat -tulpn | grep 11434
```

### Chatterbox Not Starting
```bash
# Check logs
docker-compose logs app | grep -i chatterbox
cat /tmp/chatterbox.log  # inside container

# Test Python 3.11
docker-compose exec app python3 --version  # Should be 3.11.x

# Check dependencies
docker-compose exec app python3 -m pip list | grep chatterbox
```

### Out of Memory Errors
- Use smaller models (gpt-oss:20b instead of 120b)
- Close other GPU-intensive applications
- Increase swap space
- Consider running Ollama on CPU while Chatterbox uses GPU

## Performance Tips

1. **Keep models loaded** - First inference is slow (model loading), subsequent calls are fast
2. **Use appropriate model sizes** - Balance quality vs. speed
3. **Monitor VRAM** - Don't overload GPU memory
4. **Batch requests** - Ollama handles concurrent requests efficiently
5. **Adjust temperature** - Lower = faster, more deterministic

## Advanced: Voice Cloning with Chatterbox

```bash
# 1. Add reference audio to container
docker cp your_voice_sample.wav pcg-cc-mcp:/app/voice_ref.wav

# 2. Update environment
CHATTERBOX_VOICE_REF=/app/voice_ref.wav

# 3. Restart Chatterbox
docker-compose restart app

# 4. Test cloned voice
curl -X POST http://localhost:8100/tts \
  -H "Content-Type: application/json" \
  -d '{"text": "This should sound like the reference voice", "exaggeration": 0.5}' \
  --output cloned_voice.wav
```

## Support & Resources

- **Ollama Docs**: https://ollama.ai/docs
- **Chatterbox GitHub**: https://github.com/resemble-ai/chatterbox
- **NVIDIA Container Toolkit**: https://docs.nvidia.com/datacenter/cloud-native/container-toolkit/

---

🎯 **Ready to use local LLM and TTS with GPU acceleration!**
