#!/bin/bash
# Test script for voice conversation logging and analytics

BASE_URL="http://localhost:3002"

echo "=== Testing Voice Analytics Implementation ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to make a test voice interaction
test_voice_interaction() {
    local user_id=$1
    local session_id=$2
    local device_type=$3

    echo -e "${YELLOW}Testing voice interaction for user: $user_id${NC}"

    # Note: In real usage, audio_input would be base64-encoded audio
    # For testing without actual audio, you can omit it or provide dummy data
    curl -s -X POST "$BASE_URL/nora/voice/interaction" \
      -H "Content-Type: application/json" \
      -d "{
        \"interactionId\": \"test-$(date +%s)\",
        \"sessionId\": \"$session_id\",
        \"userId\": \"$user_id\",
        \"deviceId\": \"device-$user_id\",
        \"deviceType\": \"$device_type\",
        \"interactionType\": \"Conversation\",
        \"responseText\": \"Test response\",
        \"processingTimeMs\": 100,
        \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
      }" | jq '.'

    echo ""
}

# Function to test user analytics
test_user_analytics() {
    local user_id=$1

    echo -e "${YELLOW}Fetching analytics for user: $user_id${NC}"
    curl -s "$BASE_URL/nora/voice/analytics/users/$user_id" | jq '.'
    echo ""
}

# Function to test all users analytics
test_all_users_analytics() {
    echo -e "${YELLOW}Fetching analytics for all users${NC}"
    curl -s "$BASE_URL/nora/voice/analytics/users" | jq '.'
    echo ""
}

# Function to test conversation history
test_conversation_history() {
    local session_id=$1

    echo -e "${YELLOW}Fetching conversation history for session: $session_id${NC}"
    curl -s "$BASE_URL/nora/voice/conversations/$session_id" | jq '.'
    echo ""
}

# Function to check database
check_database() {
    echo -e "${YELLOW}Checking database for voice conversations${NC}"
    sqlite3 dev_assets/db.sqlite <<EOF
SELECT
    user_id,
    session_id,
    message_count,
    created_at,
    updated_at
FROM agent_conversations
WHERE user_id IS NOT NULL
ORDER BY updated_at DESC
LIMIT 5;
EOF
    echo ""
}

# Main test flow
echo "1. Creating test voice interactions..."
test_voice_interaction "alice" "session-alice-1" "glasses"
sleep 1
test_voice_interaction "alice" "session-alice-1" "glasses"
sleep 1
test_voice_interaction "bob" "session-bob-1" "phone"
sleep 1

echo ""
echo "2. Testing user-specific analytics..."
test_user_analytics "alice"

echo ""
echo "3. Testing all users analytics..."
test_all_users_analytics

echo ""
echo "4. Testing conversation history..."
test_conversation_history "session-alice-1"

echo ""
echo "5. Checking database directly..."
check_database

echo -e "${GREEN}=== Test script complete ===${NC}"
echo ""
echo "To run individual tests:"
echo "  ./test_voice_analytics.sh"
echo ""
echo "API Endpoints:"
echo "  GET  $BASE_URL/nora/voice/analytics/users/:user_id"
echo "  GET  $BASE_URL/nora/voice/analytics/users"
echo "  GET  $BASE_URL/nora/voice/conversations/:session_id"
echo "  POST $BASE_URL/nora/voice/interaction"
