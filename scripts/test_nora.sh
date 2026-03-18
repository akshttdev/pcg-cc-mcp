#!/bin/bash

# Test Nora Agent Integration
BASE_URL="http://127.0.0.1:3006"

echo "🔍 Testing Nora Agent Integration..."
echo ""

# Test 1: Initialize Nora
echo "1️⃣  Initializing Nora..."
INIT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/nora/initialize" \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "personality": {
        "accentStrength": 0.8,
        "formalityLevel": "professional",
        "warmthLevel": "warm"
      },
      "executiveMode": true,
      "llm": {
        "provider": "OpenAI",
        "model": "gpt-4o-mini",
        "temperature": 0.2,
        "maxTokens": 600
      }
    },
    "activateImmediately": true
  }')

echo "$INIT_RESPONSE" | jq '.'
NORA_ID=$(echo "$INIT_RESPONSE" | jq -r '.noraId')
echo "✅ Nora initialized with ID: $NORA_ID"
echo ""

# Test 2: Check Status
echo "2️⃣  Checking Nora status..."
STATUS_RESPONSE=$(curl -s "$BASE_URL/api/nora/status")
echo "$STATUS_RESPONSE" | jq '.'
echo ""

# Test 3: Simple chat
echo "3️⃣  Testing simple chat..."
CHAT_RESPONSE=$(curl -s -X POST "$BASE_URL/api/nora/chat" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Hello Nora, please introduce yourself",
    "sessionId": "test-session-123",
    "voiceEnabled": false,
    "priority": "Normal"
  }')

echo "$CHAT_RESPONSE" | jq '.'
echo ""

# Test 4: Executive query
echo "4️⃣  Testing executive query with LLM..."
EXEC_RESPONSE=$(curl -s -X POST "$BASE_URL/api/nora/chat" \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What are the top 3 priorities I should focus on today?",
    "sessionId": "test-session-123",
    "requestType": "strategyPlanning",
    "voiceEnabled": false,
    "priority": "Executive"
  }')

echo "$EXEC_RESPONSE" | jq '.'
echo ""

# Test 5: Get available tools
echo "5️⃣  Getting available executive tools..."
TOOLS_RESPONSE=$(curl -s "$BASE_URL/api/nora/tools/available")
echo "$TOOLS_RESPONSE" | jq '.tools | length' | xargs -I {} echo "Available tools: {}"
echo ""

echo "✅ Nora integration test complete!"
