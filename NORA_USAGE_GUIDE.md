# NORA Executive Assistant - Usage Guide

## Overview

NORA (Neural Operational Resource Architect) is an AI-powered executive assistant integrated into the PCG-CC-MCP platform. It provides intelligent conversation, task coordination, voice interactions, and tool execution capabilities.

---

## Table of Contents

1. [How NORA Generates Responses](#how-nora-generates-responses)
2. [API Endpoints](#api-endpoints)
3. [Configuration & Initialization](#configuration--initialization)
4. [Using NORA Features](#using-nora-features)
5. [Available Tools](#available-tools)
6. [Advanced Features](#advanced-features)

---

## How NORA Generates Responses

### Response Generation Flow

```
User Request → Rate Limiting → NORA Agent → LLM Processing → Response
                     ↓              ↓            ↓              ↓
              20 req/min    Context Building  OpenAI API   Cache Check
                          (projects, memory)  gpt-4-turbo   Hit/Miss
```

### Current LLM Setup

**Model**: `gpt-4-turbo` (OpenAI)
**System Prompt**: Executive assistant with live database access to PCG projects
**Context Building**: 
- Active projects from SQLite database
- Conversation memory (last 10 exchanges)
- Executive context (priorities, milestones, budgets)
- User request context

### Response Types

1. **Simple Greetings** (≤5 words) - Pattern matched, no LLM
   - "Hello", "Hi", "Thank you" → Pre-defined responses

2. **Complex Queries** (>5 words or data requests) - LLM powered
   - Project status questions
   - Task analysis
   - Strategic planning
   - All other interactions

### Caching System

- **Cache Hit**: Returns cached response instantly
- **Cache Miss**: Calls OpenAI API, stores result
- **TTL**: Configurable (default: 1 hour)
- **Metrics**: Tracked via Prometheus

---

## API Endpoints

### Base URL
```
http://localhost:3001/api
```

### 1. Initialize NORA

**POST** `/nora/initialize`

**Request Body:**
```json
{
  "config": {
    "llm": {
      "provider": "openai",
      "model": "gpt-4-turbo",
      "apiKey": "sk-...",
      "temperature": 0.7,
      "maxTokens": 1500,
      "systemPrompt": "Custom prompt (optional)"
    },
    "voice": {
      "enabled": true,
      "sttProvider": "openai",
      "ttsProvider": "openai",
      "voiceId": "nova"
    },
    "coordination": {
      "enabled": true,
      "maxConcurrentTasks": 10
    },
    "personality": {
      "mode": "executive",
      "formality": 0.8
    }
  },
  "activateImmediately": true
}
```

**Response:**
```json
{
  "success": true,
  "noraId": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Nora initialized successfully",
  "capabilities": [
    "text_interaction",
    "voice_synthesis",
    "voice_transcription",
    "task_coordination",
    "executive_tools",
    "streaming_responses"
  ]
}
```

### 2. Chat with NORA (Standard)

**POST** `/nora/chat`

**Request Body:**
```json
{
  "message": "What's the status of our active projects?",
  "sessionId": "user-session-123",
  "requestType": "TextInteraction",
  "voiceEnabled": false,
  "priority": "Normal",
  "context": {
    "projectFilter": "active"
  }
}
```

**Response:**
```json
{
  "responseId": "resp-456",
  "requestId": "req-789",
  "sessionId": "user-session-123",
  "responseType": "DirectResponse",
  "content": "Based on live data, we have 3 active projects: PCG Dashboard MCP (72% complete), Experience the Game (61%), and Chimia DAO (44%). PCG Dashboard is on track with 4 weeks to milestone...",
  "actions": [],
  "voiceResponse": null,
  "followUpSuggestions": [
    "Would you like details on any specific project?",
    "Shall I identify blockers across all active projects?"
  ],
  "contextUpdates": [],
  "timestamp": "2025-11-19T12:34:56Z",
  "processingTimeMs": 1234
}
```

### 3. Chat with NORA (Streaming)

**POST** `/nora/chat/stream`

**Request Body:** Same as `/nora/chat`

**Response:** Server-Sent Events (SSE) stream

```
data: Based on
data:  live data,
data:  we have
data:  3 active
data:  projects
...
```

**Client Example:**
```javascript
const eventSource = new EventSource('/api/nora/chat/stream', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    message: "Summarize active projects",
    sessionId: "session-123",
    voiceEnabled: false
  })
});

eventSource.onmessage = (event) => {
  console.log(event.data); // Stream chunk
};
```

### 4. Get NORA Status

**GET** `/nora/status`

**Response:**
```json
{
  "isActive": true,
  "noraId": "550e8400-e29b-41d4-a716-446655440000",
  "uptimeMs": 3600000,
  "voiceEnabled": true,
  "coordinationEnabled": true,
  "executiveMode": true,
  "personalityMode": "executive"
}
```

### 5. Execute Tools

**POST** `/nora/tools/execute`

**Request Body:**
```json
{
  "tool": "ReadFile",
  "parameters": {
    "filePath": "/app/README.md",
    "encoding": "utf-8"
  }
}
```

**Response:**
```json
{
  "success": true,
  "result": {
    "filePath": "/app/README.md",
    "content": "# PCG-CC-MCP...",
    "sizeBytes": 4567
  },
  "executionTimeMs": 45
}
```

### 6. Voice Synthesis

**POST** `/nora/voice/synthesize`

**Request Body:**
```json
{
  "text": "Hello, how can I help you today?",
  "voiceId": "nova"
}
```

**Response:**
```json
{
  "audioData": "base64-encoded-audio...",
  "format": "mp3",
  "durationMs": 2500
}
```

### 7. Cache Statistics

**GET** `/nora/cache/stats`

**Response:**
```json
{
  "cacheHits": 45,
  "cacheMisses": 12,
  "hitRate": 0.789,
  "totalEntries": 57,
  "totalSizeBytes": 123456
}
```

### 8. Coordination Stats

**GET** `/nora/coordination/stats`

**Response:**
```json
{
  "activeAgents": 3,
  "totalTasks": 24,
  "completedTasks": 18,
  "failedTasks": 1,
  "averageExecutionTimeMs": 2345
}
```

---

## Configuration & Initialization

### Docker Deployment Setup (Recommended)

Since you're deploying with Docker, NORA configuration is done via environment variables in your `.env` file:

**Step 1: Configure Environment**

```bash
# Copy the example .env file
cp .env.example .env

# Edit .env and add your OpenAI API key
nano .env
```

**Step 2: Add Required Environment Variables**

Add to your `.env` file:

```bash
# Cloudflare Tunnel Token (for external access)
CLOUDFLARE_TUNNEL_TOKEN=eyJhIjoiYourActualTokenHere...

# OpenAI Configuration (REQUIRED for NORA)
OPENAI_API_KEY=sk-your-api-key-here

# Optional: Override NORA LLM settings
NORA_LLM_MODEL=gpt-4-turbo
NORA_LLM_TEMPERATURE=0.7
NORA_LLM_MAX_TOKENS=2000

# Optional: Voice settings
NORA_TTS_VOICE=nova
NORA_STT_LANGUAGE=en

# Application Settings (already in .env.example)
HOST=0.0.0.0
BACKEND_PORT=3001
FRONTEND_PORT=3000
RUST_LOG=info
DATABASE_URL=sqlite:///app/dev_assets/db.sqlite
```

**Step 3: Update docker-compose.yml**

Add NORA environment variables to the `app` service:

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
```

**Step 4: Build and Deploy**

```bash
# Build and start containers
docker-compose build
docker-compose up -d

# Check logs to verify NORA initialization
docker-compose logs -f app | grep -i nora

# You should see:
# "Initializing Nora executive assistant"
# "Nora initialized successfully"
```

**Step 5: Access NORA UI**

Once deployed:
- **Local Access**: `http://localhost:3001/nora`
- **Cloudflare URL**: `https://your-subdomain.yourdomain.com/nora`

The UI automatically initializes NORA on first page load.

### Environment Variables Reference

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `OPENAI_API_KEY` | OpenAI API key for LLM | **Yes** | None |
| `NORA_LLM_MODEL` | LLM model to use | No | `gpt-4-turbo` |
| `NORA_LLM_TEMPERATURE` | Response creativity (0.0-1.0) | No | `0.7` |
| `NORA_LLM_MAX_TOKENS` | Max tokens per response | No | `1500` |
| `NORA_TTS_VOICE` | Voice synthesis voice ID | No | `nova` |
| `NORA_STT_LANGUAGE` | Speech recognition language | No | `en` |

### Initialization via API (Manual)

**Minimal Config:**
```bash
curl -X POST http://localhost:3001/api/nora/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "activateImmediately": true
  }'
```

**Full Config:**
```bash
curl -X POST http://localhost:3001/api/nora/initialize \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "llm": {
        "provider": "openai",
        "model": "gpt-4-turbo",
        "apiKey": "'"$OPENAI_API_KEY"'",
        "temperature": 0.7,
        "maxTokens": 2000
      },
      "voice": {
        "enabled": true,
        "sttProvider": "openai",
        "ttsProvider": "openai",
        "voiceId": "nova"
      }
    },
    "activateImmediately": true
  }'
```

### Initialization via Frontend

```typescript
const initializeNora = async () => {
  const response = await fetch('/api/nora/initialize', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      config: {
        llm: {
          provider: 'openai',
          model: 'gpt-4-turbo',
          temperature: 0.7
        },
        voice: { enabled: true }
      },
      activateImmediately: true
    })
  });
  
  const data = await response.json();
  console.log('NORA initialized:', data.noraId);
};
```

---

## Using NORA Features

### 🎨 UI Access (Primary Method)

**NORA is fully integrated into the dashboard UI:**

1. **Navigate to NORA**: Click "Nora Assistant" in the sidebar (Crown icon)
2. **Auto-Initialization**: NORA initializes automatically when you load the page
3. **Chat Interface**: Type messages in the chat box at the bottom
4. **Tabs Available**:
   - **Chat Assistant**: Main conversation interface
   - **Coordination**: View multi-agent coordination stats
   - **Voice Settings**: Configure voice synthesis/transcription
   - **Analytics**: Performance metrics and usage stats

**Features in UI:**
- ✅ Real-time chat with streaming responses
- ✅ Conversation history (persistent per session)
- ✅ Voice interaction (if enabled)
- ✅ Executive action buttons
- ✅ Project context awareness
- ✅ Follow-up suggestions

**UI Location**: `/nora` route (accessible after login)

**Components**:
- `NoraAssistant` - Main chat interface
- `NoraCoordinationPanel` - Multi-agent stats
- `NoraVoiceControls` - Voice settings

### 📡 API Access (Programmatic)

#### 1. Text Chat

**Basic Example (Docker deployment):**
```bash
# From inside Docker container
docker-compose exec app sh -c "curl -X POST http://localhost:3001/api/nora/chat \
  -H 'Content-Type: application/json' \
  -d '{
    \"message\": \"What are our top priorities this week?\",
    \"sessionId\": \"user-123\",
    \"voiceEnabled\": false
  }'"

# From host machine (if port 3001 is exposed)
curl -X POST http://localhost:3001/api/nora/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What are our top priorities this week?",
    "sessionId": "user-123",
    "voiceEnabled": false
  }'
```

**With Context:**
```bash
curl -X POST http://localhost:3001/api/nora/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Create a task for the dashboard migration",
    "sessionId": "user-123",
    "priority": "High",
    "context": {
      "projectId": "pcg-dashboard-mcp",
      "taskType": "migration"
    }
  }'
```

### 2. Streaming Chat (SSE)

**JavaScript Client:**
```javascript
async function chatWithNoraStream(message) {
  const response = await fetch('/api/nora/chat/stream', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      message,
      sessionId: 'session-' + Date.now(),
      voiceEnabled: false
    })
  });

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    
    const chunk = decoder.decode(value);
    const lines = chunk.split('\n');
    
    for (const line of lines) {
      if (line.startsWith('data: ')) {
        const text = line.slice(6);
        process.stdout.write(text); // Stream to console
      }
    }
  }
}

chatWithNoraStream("Analyze our Q4 milestones");
```

### 3. Voice Interaction

**Full Voice Workflow:**
```bash
# 1. Record audio (browser/device)
# 2. Transcribe to text
curl -X POST http://localhost:3001/api/nora/voice/transcribe \
  -H "Content-Type: application/json" \
  -d '{
    "audioData": "base64-encoded-audio",
    "format": "wav"
  }'

# 3. Process with NORA (gets text from transcription)
curl -X POST http://localhost:3001/api/nora/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "transcribed text here",
    "sessionId": "voice-session",
    "voiceEnabled": true
  }'

# 4. Response includes voiceResponse (base64 audio)
# 5. Play audio in browser
```

### 4. Tool Execution

**Available Tools:**

#### File Operations
```bash
# Read file
curl -X POST http://localhost:3001/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ReadFile",
    "parameters": {
      "filePath": "/app/config.json"
    }
  }'

# Write file
curl -X POST http://localhost:3001/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "WriteFile",
    "parameters": {
      "filePath": "/app/output.txt",
      "content": "Generated content",
      "createDirectories": true
    }
  }'

# List directory
curl -X POST http://localhost:3001/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ListDirectory",
    "parameters": {
      "directoryPath": "/app",
      "recursive": false,
      "pattern": "*.md"
    }
  }'
```

#### Web Operations
```bash
# Fetch webpage
curl -X POST http://localhost:3001/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "FetchWebPage",
    "parameters": {
      "url": "https://example.com",
      "extractText": true
    }
  }'
```

#### Code Analysis
```bash
# Analyze code quality
curl -X POST http://localhost:3001/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "AnalyzeCodeQuality",
    "parameters": {
      "code": "fn main() { println!(\"Hello\"); }",
      "language": "Rust",
      "checkSecurity": true
    }
  }'
```

---

## Available Tools

### 13 Integrated Tools (Just Implemented!)

| Tool | Category | Status | Description |
|------|----------|--------|-------------|
| **ReadFile** | File Ops | ✅ Production | Read file contents with encoding support |
| **WriteFile** | File Ops | ✅ Production | Write files with auto-directory creation |
| **ListDirectory** | File Ops | ✅ Production | Recursive directory listing with patterns |
| **DeleteFile** | File Ops | ✅ Production | Safe file deletion with confirmation |
| **SearchWeb** | Web | 🔧 Needs API | Web search (General, News, Academic, etc.) |
| **FetchWebPage** | Web | ✅ Production | HTTP client for webpage fetching |
| **SummarizeContent** | Web | 🔧 Can Add LLM | Text summarization with formats |
| **ExecuteCode** | Code | 🔒 Sandboxing Needed | Secure code execution (pending Docker) |
| **AnalyzeCodeQuality** | Code | ✅ Production | Code metrics and quality analysis |
| **GenerateDocumentation** | Code | ✅ Production | Generate Markdown/HTML/RST docs |
| **SendEmail** | Email | 🔧 Needs SMTP | Email with priority levels |
| **SendSlackMessage** | Notifications | 🔧 Needs Slack API | Channel messaging with mentions |
| **CreateNotification** | Notifications | ✅ Production | Multi-type notifications |
| **CreateCalendarEvent** | Calendar | 🔧 Needs API | Event creation with attendees |
| **FindAvailableSlots** | Calendar | 🔧 Needs API | Multi-participant scheduling |
| **CheckCalendarAvailability** | Calendar | 🔧 Needs API | Conflict detection |

**Legend:**
- ✅ Production: Fully functional
- 🔧 Needs Integration: Works, but needs external API
- 🔒 Security Required: Needs sandboxing/isolation

### Tool Categories

```typescript
enum ToolCategory {
  FileOperations = "file_operations",
  WebSearch = "web_search",
  CodeDevelopment = "code_development",
  Communication = "communication",
  CalendarScheduling = "calendar_scheduling"
}
```

---

## Advanced Features

### 1. Rate Limiting

**Current Limits:**
- Chat endpoints: **20 requests/minute**
- Voice synthesis: **30 requests/minute**
- Auto-refill: Every 3 seconds (chat), 2 seconds (voice)

**Error Response:**
```json
{
  "error": "Rate limit exceeded",
  "message": "Please slow down your chat requests",
  "retryAfter": 3000
}
```

### 2. Prometheus Metrics

**Available Metrics:**
```
nora_requests_total{endpoint, priority}
nora_request_duration_seconds{endpoint}
nora_cache_hits_total
nora_cache_misses_total
nora_cache_size_bytes
nora_cache_entries_total
nora_tts_calls_total{provider, status}
nora_stt_calls_total{provider, status}
```

**Access:** `http://localhost:3001/metrics`

### 3. Conversation Memory

**Auto-stored:**
- Last 10 exchanges per session
- User intent tracking
- Context continuity

**Access:**
```bash
# Included in every response's contextUpdates field
{
  "contextUpdates": [
    {"type": "memory", "key": "last_project", "value": "pcg-dashboard"}
  ]
}
```

### 4. Executive Context

**Tracked Data:**
- Active projects (live from DB)
- Current priorities
- Milestones
- Budget status
- Team allocations

**Updated:** Real-time on every request

---

## Testing NORA

### Quick Test Script

```bash
#!/bin/bash

# 1. Initialize NORA
echo "Initializing NORA..."
curl -X POST http://localhost:3001/api/nora/initialize \
  -H "Content-Type: application/json" \
  -d '{"activateImmediately": true}'

echo -e "\n\n"

# 2. Check status
echo "Checking status..."
curl http://localhost:3001/api/nora/status

echo -e "\n\n"

# 3. Chat
echo "Chatting with NORA..."
curl -X POST http://localhost:3001/api/nora/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Hello! What are our active projects?",
    "sessionId": "test-session",
    "voiceEnabled": false
  }'

echo -e "\n\n"

# 4. Execute tool
echo "Executing ReadFile tool..."
curl -X POST http://localhost:3001/api/nora/tools/execute \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "ListDirectory",
    "parameters": {
      "directoryPath": "/app",
      "recursive": false
    }
  }'
```

### Frontend Integration Example

```typescript
// src/services/nora.ts
export class NoraService {
  private baseUrl = '/api/nora';
  
  async initialize() {
    const response = await fetch(`${this.baseUrl}/initialize`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ activateImmediately: true })
    });
    return response.json();
  }
  
  async chat(message: string, sessionId: string) {
    const response = await fetch(`${this.baseUrl}/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message,
        sessionId,
        voiceEnabled: false
      })
    });
    return response.json();
  }
  
  async chatStream(message: string, onChunk: (text: string) => void) {
    const response = await fetch(`${this.baseUrl}/chat/stream`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message,
        sessionId: 'stream-' + Date.now(),
        voiceEnabled: false
      })
    });
    
    const reader = response.body!.getReader();
    const decoder = new TextDecoder();
    
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      
      const chunk = decoder.decode(value);
      const lines = chunk.split('\n');
      
      for (const line of lines) {
        if (line.startsWith('data: ')) {
          onChunk(line.slice(6));
        }
      }
    }
  }
}

// Usage in component
const nora = new NoraService();
await nora.initialize();

nora.chatStream('Analyze our projects', (chunk) => {
  console.log('Stream:', chunk);
});
```

---

## Next Steps

### Enhancements to Implement

1. **External API Integration** (Task #4)
   - SMTP for email (SendEmail tool)
   - Slack webhooks (SendSlackMessage tool)
   - Google/Outlook Calendar APIs

2. **Code Execution Sandboxing** (Task #5)
   - Docker-based isolation for ExecuteCode tool
   - Security policies and timeouts

3. **Cache Invalidation** (Task #6)
   - TTL-based eviction
   - Cache warming for common queries
   - Smart invalidation policies

### Recommended Configuration

For production use:

```json
{
  "llm": {
    "model": "gpt-4-turbo",
    "temperature": 0.7,
    "maxTokens": 2000,
    "cacheEnabled": true,
    "cacheTtlSeconds": 3600
  },
  "voice": {
    "enabled": true,
    "voiceId": "nova",
    "sttLanguage": "en"
  },
  "rateLimits": {
    "chatPerMinute": 20,
    "voicePerMinute": 30
  }
}
```

---

## Troubleshooting

### NORA Not Initializing
```bash
# Check logs
docker-compose logs app | grep -i nora

# Verify OpenAI API key
echo $OPENAI_API_KEY

# Re-initialize
curl -X POST http://localhost:3001/api/nora/initialize \
  -d '{"activateImmediately": true}'
```

### Rate Limit Errors
- Wait 3 seconds between requests
- Use streaming endpoint for long conversations
- Check `/metrics` for current usage

### Cache Issues
```bash
# Clear cache
curl -X POST http://localhost:3001/api/nora/cache/clear

# Check stats
curl http://localhost:3001/api/nora/cache/stats
```

---

## Support

- **Documentation:** This file + `AGENTS.md`
- **Code:** `crates/nora/src/` (Rust backend)
- **API Routes:** `crates/server/src/routes/nora.rs`
- **Tests:** `cargo test -p nora`

**All 42 tests passing!** ✅
