# British Voice Configuration for Nora

## Overview

The dashboard is now configured to use **OpenAI TTS** as the primary voice provider with British-style voices. Your OpenAI API key is already configured and ready to use.

## Available British Voices (OpenAI)

### Female Voices

1. **Fable** (Recommended for British Executive Female)
   - Most British-leaning female voice
   - Clear, expressive, professional
   - Best for executive assistant role

2. **Nova** (Professional Female)
   - Warm, professional tone
   - Good for friendly but formal communication
   - Alternative British female option

3. **Shimmer** (Softer Female)
   - Gentle, softer tone
   - Less formal than Fable/Nova

### Male Voices

1. **Echo** (British Executive Male)
   - Clear articulation
   - Authoritative, professional
   - Best male British voice

2. **Onyx** (Professional Male)
   - Deep, professional tone
   - Strong executive presence

### Neutral

- **Alloy** - Balanced, neutral voice

## Current Configuration

The system is pre-configured with:
- **Provider**: OpenAI (using your existing API key)
- **Default Voice**: Fable (British Female Executive)
- **Quality**: High (tts-1-hd model)
- **Speed**: 1.0 (normal)
- **Volume**: 0.85

## How to Change Voice Settings

### Option 1: Via UI (Recommended)

1. Open the Nora Assistant interface
2. Click the voice settings/configuration panel
3. Under "Text-to-Speech Settings":
   - **TTS Provider**: Select "OpenAI" (should already be selected)
   - **Voice**: Choose from the dropdown:
     - Fable (British Female Executive) ← Default
     - Nova (Professional Female)
     - Echo (British Male Executive)
     - Onyx (Professional Male)
     - Shimmer (Softer Female)
     - Alloy (Neutral)
4. Click "Save Configuration"

### Option 2: Via API

Update the voice configuration programmatically:

```bash
curl -X PUT http://localhost:3000/api/nora/voice/config \
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
      }
    }
  }'
```

## Testing the Voice

### Quick Test via API

Test the British voice synthesis:

```bash
curl -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Good morning. This is Nora, your British executive assistant. How may I assist you today?",
    "voiceProfile": "BritishExecutiveFemale"
  }' \
  | jq -r '.audioData' | base64 -d > test_voice.mp3

# Play the audio (macOS)
afplay test_voice.mp3
```

### Test Different Voices

```bash
# Test British Female (Fable)
curl -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "This is the Fable voice, designed for British executive communication.",
    "voiceProfile": "BritishExecutiveFemale"
  }' | jq -r '.audioData' | base64 -d > fable.mp3

# Test British Male (Echo)
curl -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "This is the Echo voice, a clear authoritative British male tone.",
    "voiceProfile": "BritishExecutiveMale"
  }' | jq -r '.audioData' | base64 -d > echo.mp3
```

## Voice Profile Mapping

When you select a voice profile, it automatically maps to the best OpenAI voice:

| Profile | OpenAI Voice | Description |
|---------|--------------|-------------|
| BritishExecutiveFemale | fable | British-leaning, professional female |
| BritishProfessionalFemale | nova | Warm, professional female |
| BritishExecutiveMale | echo | Clear, authoritative male |
| BritishProfessionalMale | onyx | Deep, professional male |
| SystemDefault | fable | Default British voice |

## Environment Configuration

Your `.env` file already has:
```bash
OPENAI_API_KEY=sk-proj-...
```

This is used for:
- ✅ Text-to-Speech (TTS) - British voices
- ✅ Speech-to-Text (STT) - Whisper with British dialect support
- ✅ LLM chat - GPT-4/Claude integration

## Advanced Configuration

### Adjusting Speech Parameters

Fine-tune the voice output:

```typescript
{
  tts: {
    provider: "openAI",
    voiceId: "fable",
    speed: 1.0,      // 0.25 to 4.0 (1.0 = normal)
    volume: 0.85,    // 0.0 to 1.0
    pitch: 1.0,      // Not used by OpenAI (included for compatibility)
    quality: "high"  // Uses tts-1-hd model
  }
}
```

### British Accent Enhancement

The system also applies British vocabulary normalization:
- "Good morning" instead of "Hey"
- "Whilst" instead of "While"
- "Schedule" pronounced correctly
- British spellings (colour, favour, etc.)

## Upgrading to Premium British Voices (Optional)

If you want even more authentic British RP accent:

### ElevenLabs (Premium)
```bash
# Add to .env
ELEVENLABS_API_KEY=your_key_here
```

Then change provider to "elevenLabs" and select:
- Rachel (British RP Female)
- Charlie (British RP Male)

### Azure Speech
```bash
# Add to .env
AZURE_SPEECH_KEY=your_key_here
AZURE_SPEECH_REGION=uksouth
```

British voices:
- en-GB-SoniaNeural (Female)
- en-GB-RyanNeural (Male)

## Troubleshooting

### Voice not working?

1. Check OpenAI API key:
```bash
echo $OPENAI_API_KEY
# Should show: sk-proj-...
```

2. Check backend logs:
```bash
npm run backend:dev:watch
# Look for: "Synthesizing speech with OpenAI voice: fable"
```

3. Verify API endpoint:
```bash
curl http://localhost:3000/api/nora/voice/config
```

### Audio not playing?

The synthesize endpoint returns base64-encoded audio. Decode it first:
```bash
echo "BASE64_STRING" | base64 -d > output.mp3
afplay output.mp3
```

## Next Steps

1. **Start the dashboard**: `pnpm run dev`
2. **Open the voice settings**: Navigate to Nora → Voice Controls
3. **Select "Fable" voice**: This is the most British-sounding option
4. **Test the voice**: Use the test button in the UI
5. **Save configuration**: Your settings are persisted to the database

## Cost Information

OpenAI TTS pricing (tts-1-hd model):
- $15.00 per 1 million characters
- ~7.5 characters per word
- Example: 1000 words ≈ 7,500 chars ≈ $0.11

Very affordable for personal use!

---

**Your dashboard is now configured with British voices and ready to use!** 🇬🇧
