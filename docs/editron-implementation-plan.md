# Editron Implementation Plan

## Executive Summary

This document outlines the implementation plan for Editron, the Post-Production Architect agent. Editron will automate video editing workflows for event footage, creating recaps (59s) and highlight reels (10-30s) from batch content sources.

## Current Status ✅

### Successfully Completed

1. **Merge Resolution** - Unified architecture combining:
   - Workflow orchestrator pattern (scalable multi-agent system)
   - Graph orchestrator (task dependencies)
   - Conversation histories (persistent interaction)
   - Agent profiles system (replicable agent model)

2. **Editron Agent Profile** - Already defined in `crates/nora/src/profiles.rs`:
   - Agent ID: `editron-post`
   - Codename: "Editron"
   - Two workflows defined:
     - `event-recap-forge` - 60-90 second recaps
     - `highlight-reel-loop` - Platform-specific short-form content
   - Capabilities: batch ingestion, story synthesis, motion orchestration, render control

3. **Infrastructure in Place**:
   - Workflow orchestrator (`crates/nora/src/workflow/orchestrator.rs`)
   - Workflow router for matching requests to agents
   - SSE event streaming for real-time updates
   - Project board task tracking
   - Database models for workflow executions

## What Needs to Be Built

### Phase 1: Core Tools & Integration (Week 1-2)

#### 1.1 Dropbox Auto-Detection
**Location**: `crates/server/src/webhooks/dropbox.rs` (new)

- Implement Dropbox webhook endpoint
- Verify webhook signatures
- Parse incoming batch metadata
- Auto-create tasks on project boards
- Trigger Nora with batch information

**Implementation**:
```rust
pub async fn handle_dropbox_webhook(
    payload: DropboxWebhookPayload
) -> Result<()> {
    // 1. Validate webhook signature
    // 2. Parse folder metadata
    // 3. Create project/task if needed
    // 4. Trigger Nora with context:
    //    "New event footage from {event_name}: {dropbox_url}"
}
```

#### 1.2 Executive Tools for Editron
**Location**: `crates/nora/src/tools.rs` (extend NoraExecutiveTool enum)

Add these tools:
- `IngestMediaBatch` - Download and validate media from Dropbox
- `AnalyzeMediaBatch` - Run video analysis (see Phase 2)
- `GenerateVideoEdits` - Orchestrate edit assembly
- `RenderVideoDeliverables` - Export final videos
- `RecordEditronFeedback` - Store approval/rejection data

**Example**:
```rust
pub enum NoraExecutiveTool {
    // ... existing tools ...

    IngestMediaBatch {
        source_url: String,
        reference_name: Option<String>,
        storage_tier: String,
        checksum_required: bool,
        project_id: Option<String>,
    },

    AnalyzeMediaBatch {
        batch_id: String,
        scope: Vec<String>, // ["recap", "highlight", "social"]
        model_hint: Option<String>,
        max_iterations: u32,
    },

    // ... more tools
}
```

#### 1.3 Media Batch Database Models
**Location**: `crates/db/src/models/media_batch.rs` (new)

```rust
pub struct MediaBatch {
    pub id: Uuid,
    pub project_id: Uuid,
    pub source_url: String,
    pub reference_name: Option<String>,
    pub storage_tier: String,
    pub status: BatchStatus,
    pub file_count: i32,
    pub total_size_bytes: i64,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

pub struct MediaFile {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub filename: String,
    pub file_path: String,
    pub checksum: String,
    pub duration_seconds: Option<f64>,
    pub resolution: Option<String>,
    pub codec: Option<String>,
    pub analysis_results: Option<serde_json::Value>,
}
```

### Phase 2: Video Analysis Pipeline (Week 2-3)

#### 2.1 Local AI Models Integration
**Location**: `crates/services/src/services/video_analysis.rs` (new)

Implement using:
- **CLIP** for visual scene understanding
- **Whisper** for audio transcription
- **Shot detection** using scene change algorithms
- **Energy scoring** based on motion and audio levels

**Architecture**:
```
Input: MediaBatch
  ↓
Shot Detection → Segment video into clips
  ↓
CLIP Analysis → Tag visual content per shot
  ↓
Whisper Transcription → Extract dialogue/audio
  ↓
Energy Scoring → Rate crowd response, music intensity
  ↓
Output: AnalysisResults with hero shots, timestamps, tags
```

#### 2.2 Analysis Results Schema
```rust
pub struct AnalysisResults {
    pub hero_shots: Vec<HeroShot>,
    pub transcript: Vec<TranscriptSegment>,
    pub energy_curve: Vec<EnergyPoint>,
    pub sponsor_moments: Vec<SponsorCue>,
    pub recommended_cuts: Vec<CutSuggestion>,
}
```

### Phase 3: Premiere Pro Integration (Week 3-4)

#### 3.1 ExtendScript Bridge
**Location**: `assets/premiere_scripts/` (new)

Create AppleScript/ExtendScript automation:
- `create_project.jsx` - Initialize Premiere project
- `import_clips.jsx` - Import analyzed footage
- `apply_template.jsx` - Apply editing template
- `export_deliverables.jsx` - Render final outputs

**Note**: Initially use iMovie with AppleScript (simpler), migrate to Premiere later.

#### 3.2 Render Queue Service
**Location**: `crates/services/src/services/render_queue.rs` (new)

- Priority-based render queue
- Multiple aspect ratio exports (16:9, 9:16, 1:1)
- Quality profiles per destination
- Progress tracking via SSE events

### Phase 4: Story Blueprint Engine (Week 4-5)

#### 4.1 Blueprint Generator
**Location**: `crates/services/src/services/story_blueprint.rs` (new)

- Combine analysis results with creative brief
- Generate timeline structure
- Suggest music/transition points
- Create multiple variants for review

#### 4.2 Creative Memory System
**Location**: `crates/services/src/services/editron_memory.rs` (new)

- Store approved edits per brand/event type
- Learn pacing preferences
- Remember successful transition patterns
- Auto-apply historical preferences

### Phase 5: Frontend Integration (Week 5-6)

#### 5.1 Editron Dashboard Component
**Location**: `frontend/src/components/editron/EditronDashboard.tsx` (new)

Features:
- View active batches and their status
- Review analysis results
- Approve/reject edit variants
- Trigger renders
- Monitor render queue progress

#### 5.2 Video Player Integration
**Location**: `frontend/src/components/editron/VideoPlayer.tsx` (new)

- Preview generated edits
- Timeline scrubbing
- Side-by-side variant comparison
- Annotation tools for feedback

## Testing Strategy

### 1. Local Development Testing
**Test Source**: https://www.dropbox.com/scl/fo/t1txylfh2i4r5valhbjfv/AMTf4O6HfnqyiqJKTIVKqwc/25Jan25%20Footage

Steps:
1. Manually trigger ingest via NORA command
2. Verify files downloaded and checksummed
3. Run analysis pipeline
4. Review analysis results
5. Generate edit blueprint
6. Create draft export (manual for now)

### 2. End-to-End Workflow Test
```
User: "Nora, we have new event footage at [dropbox link]"
  ↓
NORA routes to Editron workflow
  ↓
Editron ingests batch
  ↓
Analysis runs automatically
  ↓
Blueprint generated
  ↓
User reviews variants
  ↓
Approved edits render
  ↓
Deliverables uploaded
```

### 3. Integration Points to Validate
- [ ] Dropbox webhook triggers correctly
- [ ] Tasks created on project boards
- [ ] SSE events stream to frontend
- [ ] Workflow stages update coordination grid
- [ ] Analysis results persisted correctly
- [ ] Render queue processes exports
- [ ] Frame.io review links work (optional)

## Environment Configuration

### Required .env Variables
```bash
# Dropbox Integration
DROPBOX_APP_KEY=your_app_key
DROPBOX_APP_SECRET=your_app_secret
DROPBOX_WEBHOOK_SECRET=your_webhook_secret

# Video Analysis
WHISPER_MODEL_PATH=/path/to/whisper/models
CLIP_MODEL_PATH=/path/to/clip/models

# Premiere Pro (future)
PREMIERE_LICENSE_KEY=your_license_key

# Storage
MEDIA_STORAGE_PATH=/path/to/media/storage
PROXY_CACHE_PATH=/path/to/proxy/cache
```

## Success Criteria

### MVP (Minimum Viable Product)
1. ✅ Editron agent profile defined
2. ✅ NORA executive tools wired up (IngestMediaBatch, AnalyzeMediaBatch, GenerateVideoEdits, RenderVideoDeliverables)
3. ✅ MediaPipelineService integrated and initialized
4. ✅ OpenAI tool schemas for LLM function calling
5. ⏳ Dropbox link triggers automatic task creation (webhook needed)
6. ⏳ Videos downloaded and analyzed locally (basic download works, needs real analysis)
7. ⏳ Analysis results visible in frontend
8. ⏳ Manual edit workflow using iMovie
9. ⏳ Renders exported to multiple aspect ratios

### Production Ready
1. Full automation (Dropbox → render)
2. Premiere Pro integration
3. AI-assisted cut suggestions
4. Frame.io review integration
5. Creative memory learning from feedback
6. Multi-project batch processing

## Next Immediate Steps

1. **Implement Dropbox Webhook** (1-2 days)
   - Create webhook endpoint
   - Register with Dropbox
   - Test with sample folder

2. **Create IngestMediaBatch Tool** (2-3 days)
   - Download files from Dropbox
   - Verify checksums
   - Store in local cache
   - Update database

3. **Set Up CLIP + Whisper** (3-4 days)
   - Install model dependencies
   - Create analysis service
   - Test with sample videos
   - Store results in database

4. **Wire Up Workflow Execution** (2-3 days)
   - Connect Nora request → workflow router
   - Execute Editron workflow stages
   - Emit SSE events for frontend
   - Create tasks on project boards

5. **Create Frontend Dashboard** (3-5 days)
   - Batch list view
   - Analysis results viewer
   - Video player for previews
   - Render status tracking

## Risk Mitigation

### Technical Risks
1. **Video analysis performance** - Use GPU acceleration, process in background
2. **Storage costs** - Implement tiered storage (hot/cold), cleanup policies
3. **Premiere Pro automation** - Start with iMovie, migrate gradually
4. **Model quality** - Fine-tune CLIP/Whisper on event footage samples

### Operational Risks
1. **Dropbox rate limits** - Implement retry logic, respect API limits
2. **Manual approval bottleneck** - Create confidence scoring for auto-approval
3. **Render queue congestion** - Priority system, capacity planning

## Timeline Estimate

- **Weeks 1-2**: Core infrastructure (tools, dropbox, database)
- **Weeks 3-4**: Video analysis pipeline
- **Weeks 5-6**: Premiere/iMovie integration
- **Weeks 7-8**: Frontend & workflow polishing
- **Week 9**: Testing & refinement
- **Week 10**: Production deployment

**MVP Target**: 6 weeks
**Production Ready**: 10 weeks

## Resources & Dependencies

### Team
- Backend Engineer (Rust, video processing)
- Frontend Engineer (React, video player)
- Creative Director (style guides, templates)
- DevOps (storage, GPU setup)

### External Services
- Dropbox API
- Frame.io (optional review platform)
- GPU compute (local or cloud)

### Assets Needed
- iMovie/Premiere templates per brand
- LUT packs for color grading
- Typography/motion graphics packs
- Audio transition library

---

**Status**: Architecture designed, implementation ready to begin
**Last Updated**: 2025-12-22
**Owner**: Claude + Team
