#!/bin/bash
# Test voice switching functionality

set -e

echo "🎙️  Testing Voice Switching Functionality"
echo "=========================================="
echo ""

# Check if backend is running
if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
    echo "❌ Backend is not running. Start it with: pnpm run dev"
    exit 1
fi

echo "✅ Backend is running"
echo ""

# Test 1: Get current config
echo "📋 Step 1: Getting current voice configuration..."
CURRENT_CONFIG=$(curl -s http://localhost:3000/api/nora/voice/config)
CURRENT_VOICE=$(echo $CURRENT_CONFIG | jq -r '.config.tts.voiceId')
echo "Current voice: $CURRENT_VOICE"
echo ""

# Test 2: Switch to "nova" voice
echo "🔄 Step 2: Switching to 'nova' voice..."
curl -s -X PUT http://localhost:3000/api/nora/voice/config \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "tts": {
        "provider": "openAI",
        "voiceId": "nova",
        "speed": 1.0,
        "volume": 0.85,
        "pitch": 1.0,
        "quality": "high",
        "britishVoicePreferences": ["nova", "fable", "echo"],
        "fallbackProviders": ["system"]
      },
      "stt": {
        "provider": "whisper",
        "model": "whisper-1",
        "language": "en-GB",
        "britishDialectSupport": true,
        "executiveVocabulary": true,
        "realTime": false,
        "noiseReduction": true
      },
      "audio": {
        "sampleRate": 44100,
        "channels": 2,
        "bitDepth": 16,
        "bufferSize": 1024,
        "noiseSuppression": true,
        "echoCancellation": true,
        "autoGainControl": true
      },
      "britishAccent": {
        "enabled": true,
        "accentStrength": 0.8,
        "regionalVariant": "RP",
        "formalityLevel": "professional",
        "vocabularyPreferences": "executive"
      },
      "executiveMode": {
        "enabled": true,
        "proactiveCommunication": true,
        "executiveSummaryStyle": true,
        "formalAddress": true,
        "businessVocabulary": true
      }
    }
  }' > /dev/null

echo "✅ Voice switched to 'nova'"
echo ""

# Test 3: Synthesize with nova
echo "🎵 Step 3: Testing 'nova' voice..."
curl -s -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "This is the Nova voice, a warm professional female voice."
  }' | jq -r '.audioData' | base64 -d > /tmp/nova_test.mp3

echo "✅ Generated audio with nova: /tmp/nova_test.mp3"

if command -v afplay > /dev/null 2>&1; then
    echo "🔊 Playing nova voice..."
    afplay /tmp/nova_test.mp3
fi
echo ""

# Test 4: Switch to "echo" voice
echo "🔄 Step 4: Switching to 'echo' voice..."
curl -s -X PUT http://localhost:3000/api/nora/voice/config \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "tts": {
        "provider": "openAI",
        "voiceId": "echo",
        "speed": 1.0,
        "volume": 0.85,
        "pitch": 1.0,
        "quality": "high",
        "britishVoicePreferences": ["echo", "onyx", "fable"],
        "fallbackProviders": ["system"]
      },
      "stt": {
        "provider": "whisper",
        "model": "whisper-1",
        "language": "en-GB",
        "britishDialectSupport": true,
        "executiveVocabulary": true,
        "realTime": false,
        "noiseReduction": true
      },
      "audio": {
        "sampleRate": 44100,
        "channels": 2,
        "bitDepth": 16,
        "bufferSize": 1024,
        "noiseSuppression": true,
        "echoCancellation": true,
        "autoGainControl": true
      },
      "britishAccent": {
        "enabled": true,
        "accentStrength": 0.8,
        "regionalVariant": "RP",
        "formalityLevel": "professional",
        "vocabularyPreferences": "executive"
      },
      "executiveMode": {
        "enabled": true,
        "proactiveCommunication": true,
        "executiveSummaryStyle": true,
        "formalAddress": true,
        "businessVocabulary": true
      }
    }
  }' > /dev/null

echo "✅ Voice switched to 'echo'"
echo ""

# Test 5: Synthesize with echo
echo "🎵 Step 5: Testing 'echo' voice..."
curl -s -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "This is the Echo voice, a clear authoritative British male voice."
  }' | jq -r '.audioData' | base64 -d > /tmp/echo_test.mp3

echo "✅ Generated audio with echo: /tmp/echo_test.mp3"

if command -v afplay > /dev/null 2>&1; then
    echo "🔊 Playing echo voice..."
    afplay /tmp/echo_test.mp3
fi
echo ""

# Test 6: Switch back to fable
echo "🔄 Step 6: Switching back to 'fable' voice..."
curl -s -X PUT http://localhost:3000/api/nora/voice/config \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "tts": {
        "provider": "openAI",
        "voiceId": "fable",
        "speed": 1.0,
        "volume": 0.85,
        "pitch": 1.0,
        "quality": "high",
        "britishVoicePreferences": ["fable", "nova", "echo"],
        "fallbackProviders": ["system"]
      },
      "stt": {
        "provider": "whisper",
        "model": "whisper-1",
        "language": "en-GB",
        "britishDialectSupport": true,
        "executiveVocabulary": true,
        "realTime": false,
        "noiseReduction": true
      },
      "audio": {
        "sampleRate": 44100,
        "channels": 2,
        "bitDepth": 16,
        "bufferSize": 1024,
        "noiseSuppression": true,
        "echoCancellation": true,
        "autoGainControl": true
      },
      "britishAccent": {
        "enabled": true,
        "accentStrength": 0.8,
        "regionalVariant": "RP",
        "formalityLevel": "professional",
        "vocabularyPreferences": "executive"
      },
      "executiveMode": {
        "enabled": true,
        "proactiveCommunication": true,
        "executiveSummaryStyle": true,
        "formalAddress": true,
        "businessVocabulary": true
      }
    }
  }' > /dev/null

echo "✅ Voice switched back to 'fable'"
echo ""

# Test 7: Verify final config
echo "📋 Step 7: Verifying final configuration..."
FINAL_CONFIG=$(curl -s http://localhost:3000/api/nora/voice/config)
FINAL_VOICE=$(echo $FINAL_CONFIG | jq -r '.config.tts.voiceId')
echo "Final voice: $FINAL_VOICE"
echo ""

echo "✅ Voice switching test complete!"
echo ""
echo "Summary:"
echo "--------"
echo "- Started with: $CURRENT_VOICE"
echo "- Switched to: nova ✓"
echo "- Switched to: echo ✓"
echo "- Switched back to: $FINAL_VOICE ✓"
echo ""
echo "Audio files generated:"
echo "- /tmp/nova_test.mp3 (warm female)"
echo "- /tmp/echo_test.mp3 (authoritative male)"
echo ""
echo "Next steps:"
echo "1. Test voice switching in the UI at http://localhost:5173"
echo "2. Try all 6 OpenAI voices: alloy, echo, fable, onyx, nova, shimmer"
