#!/bin/bash
# Test script for British voice configuration

set -e

echo "🎙️  Testing British Voice Configuration"
echo "========================================"
echo ""

# Check if backend is running
if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
    echo "❌ Backend is not running. Start it with: pnpm run dev"
    exit 1
fi

echo "✅ Backend is running"
echo ""

# Get current voice configuration
echo "📋 Current Voice Configuration:"
echo "------------------------------"
curl -s http://localhost:3000/api/nora/voice/config | jq '.config.tts' || echo "Failed to fetch config"
echo ""

# Test British Female voice (Fable)
echo "🎵 Testing British Female Voice (Fable)..."
curl -s -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Good morning. This is Nora, your British executive assistant.",
    "voiceProfile": "BritishExecutiveFemale"
  }' > /tmp/voice_test_response.json

if [ $? -eq 0 ]; then
    # Extract audio and play
    jq -r '.audioData' /tmp/voice_test_response.json | base64 -d > /tmp/british_female.mp3
    echo "✅ Generated audio: /tmp/british_female.mp3"
    
    if command -v afplay > /dev/null 2>&1; then
        echo "🔊 Playing audio..."
        afplay /tmp/british_female.mp3
    else
        echo "ℹ️  Audio saved. Play with: afplay /tmp/british_female.mp3"
    fi
else
    echo "❌ Failed to synthesize speech"
fi

echo ""

# Test British Male voice (Echo)
echo "🎵 Testing British Male Voice (Echo)..."
curl -s -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "This is the Echo voice, a clear authoritative British male tone.",
    "voiceProfile": "BritishExecutiveMale"
  }' > /tmp/voice_test_response.json

if [ $? -eq 0 ]; then
    # Extract audio and play
    jq -r '.audioData' /tmp/voice_test_response.json | base64 -d > /tmp/british_male.mp3
    echo "✅ Generated audio: /tmp/british_male.mp3"
    
    if command -v afplay > /dev/null 2>&1; then
        echo "🔊 Playing audio..."
        afplay /tmp/british_male.mp3
    else
        echo "ℹ️  Audio saved. Play with: afplay /tmp/british_male.mp3"
    fi
else
    echo "❌ Failed to synthesize speech"
fi

echo ""
echo "✅ British voice test complete!"
echo ""
echo "Next steps:"
echo "1. Open http://localhost:5173 (frontend)"
echo "2. Navigate to Nora → Voice Controls"
echo "3. Select OpenAI provider with 'Fable' voice"
echo "4. Click 'Save Configuration'"
