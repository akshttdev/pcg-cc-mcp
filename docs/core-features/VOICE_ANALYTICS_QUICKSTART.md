# Voice Analytics Quick Start Guide

## TL;DR

Voice conversations are now persisted to the database with user tracking and analytics.

## New Fields in VoiceInteraction

```json
{
  "userId": "optional-user-id",
  "deviceId": "optional-device-id",
  "deviceType": "glasses|phone|browser"
}
```

## New API Endpoints

```bash
# Get analytics for a user
GET /nora/voice/analytics/users/:user_id

# Get analytics for all users
GET /nora/voice/analytics/users

# Get conversation history
GET /nora/voice/conversations/:session_id
```

## Quick Test

```bash
# 1. Start server
cargo run --release

# 2. Run test script (in another terminal)
./test_voice_analytics.sh

# 3. Check results
curl http://localhost:3002/nora/voice/analytics/users
```

## Integration Example

Update your wearable client to include user tracking:

```javascript
// Before (still works)
const interaction = {
  interactionId: uuid(),
  sessionId: "session-123",
  interactionType: "Conversation",
  // ...
};

// After (with tracking)
const interaction = {
  interactionId: uuid(),
  sessionId: "session-123",
  userId: currentUser.id,        // NEW
  deviceId: device.serialNumber, // NEW
  deviceType: "glasses",         // NEW
  interactionType: "Conversation",
  // ...
};
```

## Analytics Dashboard Example

```javascript
// Fetch all users analytics
const response = await fetch('/nora/voice/analytics/users');
const analytics = await response.json();

analytics.forEach(user => {
  console.log(`
    User: ${user.userId}
    Total Interactions: ${user.totalInteractions}
    Total Messages: ${user.totalMessages}
    Avg Response Time: ${user.averageResponseTimeMs}ms
    Sessions: ${user.uniqueSessions}
  `);
});
```

## Database Query Example

```sql
-- View recent conversations
SELECT
  user_id,
  session_id,
  message_count,
  created_at,
  updated_at
FROM agent_conversations
WHERE user_id IS NOT NULL
ORDER BY updated_at DESC
LIMIT 10;

-- View messages for a conversation
SELECT
  role,
  content,
  latency_ms,
  created_at
FROM agent_conversation_messages
WHERE conversation_id = 'your-conversation-uuid'
ORDER BY created_at ASC;
```

## Key Points

✅ **Backward Compatible** - All new fields optional
✅ **Non-Blocking** - DB errors don't break voice interactions
✅ **Session Continuity** - Use same `session_id` for related interactions
✅ **Analytics Ready** - Metrics available immediately

## Files Changed

- `crates/nora/src/voice/mod.rs` - VoiceInteraction struct
- `crates/server/src/routes/nora.rs` - Handler + endpoints

## Files Added

- `test_voice_analytics.sh` - Test script
- `VOICE_ANALYTICS_IMPLEMENTATION.md` - Full docs
- `VOICE_ANALYTICS_QUICKSTART.md` - This file

## Support

- Full docs: `VOICE_ANALYTICS_IMPLEMENTATION.md`
- Test script: `./test_voice_analytics.sh`
- Database: `sqlite3 dev_assets/db.sqlite`
