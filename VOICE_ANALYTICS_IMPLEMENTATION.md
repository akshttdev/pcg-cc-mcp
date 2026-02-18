# Voice Conversation Logging & Analytics Implementation

## Overview

This implementation adds persistent conversation logging and analytics for Nora voice interactions with wearable devices. All voice conversations are now stored in the database and can be queried for analytics and history retrieval.

## Changes Made

### 1. Extended VoiceInteraction Struct

**File:** `crates/nora/src/voice/mod.rs`

Added user/device tracking fields to the `VoiceInteraction` struct:

```rust
pub struct VoiceInteraction {
    // ... existing fields ...

    // NEW: User/device tracking
    pub user_id: Option<String>,
    pub device_id: Option<String>,
    pub device_type: Option<String>, // "glasses", "phone", "browser"

    // ... rest of fields ...
}
```

### 2. Database Persistence

**File:** `crates/server/src/routes/nora.rs`

Modified the `voice_interaction` handler to persist conversations:

- Gets or creates conversation using `session_id`
- Updates conversation with `user_id` if provided
- Saves user message (transcription) to database
- Saves assistant response with processing time
- Gracefully handles errors (logs warnings but doesn't fail request)

**Key Implementation Details:**
- Uses existing `AgentConversation::get_or_create()` method
- Session continuity maintained across interactions
- Backward compatible - works without user_id
- Non-blocking - database errors don't fail voice interactions

### 3. Analytics Endpoints

Added three new REST endpoints:

#### GET `/nora/voice/analytics/users/:user_id`

Get analytics for a specific user.

**Response:**
```json
{
  "userId": "user123",
  "totalInteractions": 42,
  "totalMessages": 84,
  "averageResponseTimeMs": 1234.5,
  "firstInteraction": "2024-01-01T00:00:00Z",
  "lastInteraction": "2024-01-15T12:30:00Z",
  "totalInputTokens": 15000,
  "totalOutputTokens": 8000,
  "uniqueSessions": 5
}
```

#### GET `/nora/voice/analytics/users`

Get analytics for all users, grouped by user_id.

**Response:**
```json
[
  {
    "userId": "user123",
    "totalInteractions": 42,
    // ... same structure as above
  },
  {
    "userId": "user456",
    "totalInteractions": 20,
    // ... same structure as above
  }
]
```

Sorted by most recent interaction (descending).

#### GET `/nora/voice/conversations/:session_id`

Get full conversation history for a specific session.

**Response:**
```json
[
  {
    "id": "uuid",
    "conversationId": "uuid",
    "role": "user",
    "content": "What's the weather today?",
    "createdAt": "2024-01-15T10:00:00Z",
    // ... other message fields
  },
  {
    "id": "uuid",
    "conversationId": "uuid",
    "role": "assistant",
    "content": "The weather is sunny with a high of 75°F.",
    "latencyMs": 1234,
    "createdAt": "2024-01-15T10:00:01Z",
    // ... other message fields
  }
]
```

## Database Schema

Uses existing tables (no migrations required):

### `agent_conversations`

- `id` - Unique conversation ID
- `agent_id` - Nora instance ID
- `session_id` - Session identifier (groups related interactions)
- `user_id` - User who made the interaction (nullable)
- `status` - 'active', 'archived', or 'expired'
- `message_count` - Number of messages in conversation
- `total_input_tokens` - Sum of input tokens
- `total_output_tokens` - Sum of output tokens
- `created_at` - First interaction timestamp
- `updated_at` - Last interaction timestamp
- `last_message_at` - Timestamp of most recent message

### `agent_conversation_messages`

- `id` - Unique message ID
- `conversation_id` - Foreign key to conversation
- `role` - 'user', 'assistant', 'system', or 'tool'
- `content` - Message text
- `input_tokens` - Tokens in message (nullable)
- `output_tokens` - Tokens in response (nullable)
- `model` - LLM model used (nullable)
- `provider` - LLM provider (nullable)
- `latency_ms` - Processing time (nullable)
- `created_at` - Message timestamp

## Usage Examples

### Sending a Voice Interaction with User Tracking

```bash
curl -X POST http://localhost:3002/nora/voice/interaction \
  -H "Content-Type: application/json" \
  -d '{
    "interactionId": "unique-id",
    "sessionId": "session-123",
    "userId": "alice",
    "deviceId": "glasses-001",
    "deviceType": "glasses",
    "interactionType": "Conversation",
    "audioInput": "<base64-encoded-audio>",
    "responseText": "",
    "processingTimeMs": 0,
    "timestamp": "2024-01-15T10:00:00Z"
  }'
```

### Get User Analytics

```bash
curl http://localhost:3002/nora/voice/analytics/users/alice
```

### Get All Users Analytics

```bash
curl http://localhost:3002/nora/voice/analytics/users
```

### Get Conversation History

```bash
curl http://localhost:3002/nora/voice/conversations/session-123
```

### Query Database Directly

```bash
sqlite3 dev_assets/db.sqlite <<EOF
SELECT
  user_id,
  session_id,
  message_count,
  total_input_tokens,
  total_output_tokens,
  created_at
FROM agent_conversations
WHERE user_id = 'alice'
ORDER BY updated_at DESC;
EOF
```

## Testing

Run the provided test script:

```bash
cd /home/pythia/pcg-cc-mcp
./test_voice_analytics.sh
```

This script will:
1. Create test voice interactions
2. Query user-specific analytics
3. Query all users analytics
4. Retrieve conversation history
5. Check database directly

## TypeScript Integration

All new types have TypeScript bindings via `ts-rs`:

```typescript
interface VoiceAnalyticsSummary {
  userId?: string;
  totalInteractions: number;
  totalMessages: number;
  averageResponseTimeMs: number;
  firstInteraction: string;
  lastInteraction: string;
  totalInputTokens: number;
  totalOutputTokens: number;
  uniqueSessions: number;
}

interface VoiceInteraction {
  interactionId: string;
  sessionId: string;
  interactionType: VoiceInteractionType;
  // NEW fields
  userId?: string;
  deviceId?: string;
  deviceType?: string;
  // ... other fields
}
```

## Backward Compatibility

✅ **Fully backward compatible**
- All new fields are optional (`Option<String>`)
- Voice interactions without `user_id` still work
- Database persistence failures are logged but don't break requests
- Existing in-memory conversation system unaffected

## Performance Considerations

- **Database writes per interaction:** 3 queries
  - 1 conversation upsert (get_or_create)
  - 1 user message insert
  - 1 assistant message insert
- **Expected latency:** ~5ms total for DB operations
- **Index usage:** All queries use existing indexes on `agent_id`, `session_id`, `user_id`
- **Non-blocking:** Database operations don't slow down voice response

## Analytics Metrics Explained

- **Total Interactions:** Number of distinct conversations
- **Total Messages:** Sum of all messages across conversations
- **Average Response Time:** Average time from conversation start to last message
- **Total Input/Output Tokens:** Sum of LLM token usage
- **Unique Sessions:** Number of distinct session_ids

## Future Enhancements

### Voice Authentication (Not Implemented)

Potential additions for voice biometric authentication:

```rust
pub struct VoiceInteraction {
    // ... existing fields ...
    pub voice_biometric_hash: Option<String>,
    pub voice_confidence_score: Option<f64>, // 0.0-1.0
}
```

Implementation would require:
- Voice feature extraction from audio input
- Speaker verification against registered profiles
- New `user_voice_profiles` table
- Confidence threshold enforcement
- Recommended libraries: `resemblyzer` or `pyannote.audio`

## Troubleshooting

### Issue: Analytics endpoint returns empty data

**Solution:** Ensure voice interactions include `user_id` field:
```json
{
  "userId": "alice",
  // ... other fields
}
```

### Issue: Conversation not persisting

**Check:**
1. Database connection is active
2. Check server logs for warnings: `grep "Failed to persist" logs.txt`
3. Verify database permissions

### Issue: Session continuity broken

**Solution:** Use consistent `session_id` across related interactions:
```json
{
  "sessionId": "session-alice-glasses-20240115",
  // ... other fields
}
```

## Files Modified

1. `crates/nora/src/voice/mod.rs` - Extended VoiceInteraction struct
2. `crates/server/src/routes/nora.rs` - Added persistence and analytics endpoints

## Files Created

1. `test_voice_analytics.sh` - Test script for verification
2. `VOICE_ANALYTICS_IMPLEMENTATION.md` - This documentation

## Verification Checklist

- [x] VoiceInteraction struct extended with user/device fields
- [x] Database persistence added to voice_interaction handler
- [x] Analytics endpoint for specific user
- [x] Analytics endpoint for all users
- [x] Conversation history endpoint
- [x] Routes registered in router
- [x] TypeScript bindings generated
- [x] Code compiles without errors
- [x] Backward compatibility maintained
- [x] Test script created
- [x] Documentation written

## Next Steps

1. Start the server: `cargo run --release`
2. Run test script: `./test_voice_analytics.sh`
3. Integrate with wearable client to send `user_id`, `device_id`, `device_type`
4. Monitor database growth and set up archival policies if needed
5. Consider implementing voice authentication for production use

## Support

For issues or questions:
- Check server logs: `tail -f logs/server.log`
- Query database directly: `sqlite3 dev_assets/db.sqlite`
- Review API responses for error messages
