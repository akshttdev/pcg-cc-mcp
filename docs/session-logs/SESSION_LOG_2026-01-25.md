# PCG Session Log - January 25, 2026

## Session Goal
Test conference workflows using Ollama (open source models) instead of OpenAI due to API access changes.

## Configuration
- **EXA_API_KEY**: `ba180d56-aa1b-486a-af76-908488069a8f` (for real web search)
- **OLLAMA_MODEL**: `deepseek-r1` (5.2GB, faster than gpt-oss:20b at 13GB)
- **Available Ollama Models**:
  - deepseek-r1:latest (5.2 GB)
  - gpt-oss:20b (13 GB) - default but slow
  - llama3.2:1b (1.3 GB)
  - llama3.2:latest (2.0 GB)

## Code Changes Made (from previous session)

### 1. Task Scheduler - Workflow Stage Gating
**File**: `crates/server/src/task_scheduler.rs`
- Added SQL query that checks `workflow_stage_required` in task custom_properties
- Tasks now wait for appropriate workflow stage before executing
- Stage order: intake(0) < conference_intel(1) < speaker_research(2) < brand_research(3) < production_team(4) < competitive_intel(5) < side_events(6) < research_complete(7)

### 2. Conference Tasks - Stage Requirements
**File**: `crates/nora/src/conference.rs`
- Added `workflow_stage_required` to task custom_properties:
  - Research task → "intake"
  - Speakers Article → "speaker_research"
  - Side Events Article → "side_events"
  - Press Release → "research_complete"

### 3. Research Tools - Exa Logging
**File**: `crates/nora/src/execution/research.rs`
- Added logging to show whether Exa API is configured
- Provider auto-detection: Ollama (if running) → OpenAI (if key) → Anthropic (if key)
- Model selection via `OLLAMA_MODEL` env var (defaults to gpt-oss:20b)

## Server Startup Command
```bash
export EXA_API_KEY=ba180d56-aa1b-486a-af76-908488069a8f
export OLLAMA_MODEL=deepseek-r1
pnpm run dev
```

## Current State
- Backend compiled and started on dynamic port (46583 in last run)
- Frontend on port 3000
- API requires authentication (401 on direct curl)
- Need to use `pnpm run dev` for proper dev environment setup

## Next Steps
1. Start server with `pnpm run dev` (handles auth and ports correctly)
2. Create fresh Real Vision Crypto Gathering 2026 workflow
3. Monitor conference_intel stage with Ollama + Exa
4. Verify workflow stage gating works correctly

## Known Issues
- OpenAI rate limits (429) when using OpenAI provider
- `generate_openai()` method lacks retry/fallback logic (only `generate_openai_with_tools_and_history()` has it)
- EXA_API_KEY needs explicit export (dotenv not loading from correct directory)

## Conference Details
- **Event**: Real Vision Crypto Gathering 2026
- **Dates**: January 22-25, 2026
- **Location**: Loews Miami Beach
