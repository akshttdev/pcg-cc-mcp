# Voice Configuration Guide

## Current Voice Settings

### Text-to-Speech (TTS) Configuration

**Provider**: OpenAI TTS
- **Voice ID**: `ZtcPZrt9K4w8e1OB9M6w` (ElevenLabs voice ID, but using OpenAI)
- **Speed**: 1.0 (normal speed)
- **Volume**: 0.8 (80% volume)
- **Pitch**: 0.0 (default pitch)
- **Quality**: High
- **Fallback Providers**: System TTS

**British Voice Preferences**:
- Primary: ZtcPZrt9K4w8e1OB9M6w

### Speech-to-Text (STT) Configuration

**Provider**: Whisper (OpenAI)
- **Model**: system_stt
- **Language**: en-GB (British English)
- **British Dialect Support**: Enabled
- **Executive Vocabulary**: Enabled
- **Real-time Processing**: Disabled
- **Noise Reduction**: Enabled

### Audio Configuration

- **Sample Rate**: 44,100 Hz
- **Channels**: 2 (Stereo)
- **Bit Depth**: 16-bit
- **Buffer Size**: 1024
- **Noise Suppression**: Enabled
- **Echo Cancellation**: Enabled
- **Auto Gain Control**: Enabled

### British Accent Configuration

- **Accent Strength**: 0.8 (Strong)
- **Regional Variant**: Received Pronunciation (RP)
- **Formality Level**: Professional
- **Vocabulary Preferences**: Executive

### Executive Mode

- **Enabled**: Yes
- **Proactive Communication**: Enabled
- **Executive Summary Style**: Enabled
- **Formal Address**: Enabled
- **Business Vocabulary**: Enabled

## Available TTS Providers

### 1. **ElevenLabs** (Premium British Voices)
- **Best For**: Professional, high-quality British executive voices
- **Requires**: `ELEVENLABS_API_KEY` environment variable
- **Voices**:
  - Rachel (British Executive Female)
  - Brian (British Executive Male)
  - Bella (British Professional Female)
  - Charlie (British Professional Male)

**Configuration**:
```json
{
  "provider": "elevenLabs",
  "voiceId": "Rachel",
  "quality": "high"
}
```

### 2. **OpenAI TTS** (Current)
- **Best For**: Good quality, reliable, cost-effective
- **Requires**: `OPENAI_API_KEY` environment variable (already configured)
- **Voices**:
  - alloy
  - echo
  - fable
  - onyx
  - nova
  - shimmer

**Configuration**:
```json
{
  "provider": "openAI",
  "voiceId": "nova",
  "quality": "high"
}
```

### 3. **Azure Cognitive Services**
- **Best For**: Enterprise deployments, multiple languages
- **Requires**: `AZURE_SPEECH_KEY` and `AZURE_SPEECH_REGION` environment variables
- **British Voices**:
  - en-GB-SoniaNeural
  - en-GB-RyanNeural
  - en-GB-LibbyNeural

**Configuration**:
```json
{
  "provider": "azure",
  "voiceId": "en-GB-SoniaNeural",
  "quality": "high"
}
```

### 4. **Google Cloud TTS**
- **Best For**: WaveNet voices, natural sounding
- **Requires**: Google Cloud credentials
- **British Voices**:
  - en-GB-Wavenet-A (Female)
  - en-GB-Wavenet-B (Male)
  - en-GB-Wavenet-C (Female)
  - en-GB-Wavenet-D (Male)

**Configuration**:
```json
{
  "provider": "google",
  "voiceId": "en-GB-Wavenet-A",
  "quality": "high"
}
```

### 5. **System TTS** (Fallback)
- **Best For**: Development, testing, no API key required
- **Quality**: Basic
- **Note**: Returns dummy audio data for now

## Available STT Providers

### 1. **Whisper (OpenAI)** (Current)
- **Best For**: High accuracy, multiple languages
- **Requires**: `OPENAI_API_KEY` environment variable (already configured)
- **Models**: base, small, medium, large
- **Languages**: 99 languages including en-GB

### 2. **Azure Speech**
- **Best For**: Real-time transcription, enterprise
- **Requires**: `AZURE_SPEECH_KEY` and `AZURE_SPEECH_REGION`
- **Features**: Real-time, diarization, profanity filtering

### 3. **Google Speech-to-Text**
- **Best For**: Streaming recognition, word-level timestamps
- **Requires**: Google Cloud credentials

### 4. **System STT** (Fallback)
- **Note**: Returns dummy transcriptions for development

## How to Change Voice Settings

### Via Frontend UI

1. Navigate to **Nora** → **Voice Settings** tab
2. Adjust the following:
   - **TTS Provider**: Select from dropdown (elevenLabs, openAI, azure, google, system)
   - **Voice ID**: Enter voice identifier or name
   - **Speed**: Adjust slider (0.5 - 2.0)
   - **Volume**: Adjust slider (0.0 - 1.0)
   - **Pitch**: Adjust slider (0.5 - 2.0)
   - **STT Provider**: Select from dropdown
   - **Language**: Select from available languages

3. Click "Test Voice" to hear the current settings
4. Changes are **automatically saved** to the database

### Via API

**Update Voice Configuration**:
```bash
curl -X PUT http://localhost:3005/api/nora/voice/config \
  -H "Content-Type: application/json" \
  -d '{
    "config": {
      "tts": {
        "provider": "openAI",
        "voiceId": "nova",
        "speed": 1.0,
        "volume": 0.8,
        "pitch": 0.0,
        "quality": "high",
        "britishVoicePreferences": ["nova"],
        "fallbackProviders": ["system"]
      },
      "stt": {
        "provider": "whisper",
        "model": "medium",
        "language": "en-GB",
        "britishDialectSupport": true,
        "executiveVocabulary": true,
        "realTime": false,
        "noiseReduction": true
      }
    }
  }'
```

**Get Current Configuration**:
```bash
curl http://localhost:3005/api/nora/voice/config
```

### Via Environment Variables

Set the following in your `.env` file:

```env
# Required for OpenAI TTS/STT (already configured)
OPENAI_API_KEY=sk-proj-...

# Optional: For premium ElevenLabs voices
ELEVENLABS_API_KEY=your_elevenlabs_key

# Optional: For Azure Speech services
AZURE_SPEECH_KEY=your_azure_key
AZURE_SPEECH_REGION=eastus

# Optional: For NORA LLM configuration
NORA_LLM_MODEL=gpt-4-turbo
NORA_LLM_TEMPERATURE=0.7
NORA_LLM_MAX_TOKENS=2000
```

## Testing Voice Synthesis

### Using the Frontend

1. Go to **Nora** → **Voice Settings**
2. Scroll to **Voice Testing** section
3. Click **"Test Voice Synthesis"**
4. Listen to the generated audio

### Using the API

```bash
curl -X POST http://localhost:3005/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Good afternoon. This is a test of my British executive voice.",
    "britishAccent": true,
    "executiveTone": true
  }'
```

Response:
```json
{
  "audioData": "base64_encoded_audio...",
  "durationMs": 5000,
  "sampleRate": 22050,
  "format": "Mp3",
  "processingTimeMs": 234
}
```

## Testing Speech Recognition

### Using the Frontend

1. Go to **Nora** → **Voice Settings**
2. Scroll to **Speech Recognition Test** section
3. Click **"Start Recording"**
4. Speak clearly
5. Click **"Stop Recording"**
6. View the transcription result

### Using the API

```bash
# Record audio and convert to base64, then:
curl -X POST http://localhost:3005/api/nora/voice/transcribe \
  -H "Content-Type: application/json" \
  -d '{
    "audioData": "base64_encoded_audio...",
    "language": "en-GB",
    "britishDialect": true
  }'
```

Response:
```json
{
  "text": "Your transcribed text here"
}
```

## Troubleshooting

### Issue: "InvalidCharacterError" when testing voice synthesis

**Cause**: Mismatch between backend response format (camelCase) and frontend expectation (snake_case)

**Solution**: Already fixed in the codebase. The frontend now correctly handles the `audioData` field.

### Issue: STT returns dummy transcriptions

**Cause**: Missing `OPENAI_API_KEY` or provider fallback to SystemSTT

**Solution**: 
1. Check that `OPENAI_API_KEY` is set in `.env`
2. Restart the backend: `pnpm run backend:dev`
3. Check logs for "Creating Whisper STT with OpenAI API key"

### Issue: TTS returns silent/dummy audio

**Cause**: Using System TTS provider which is a placeholder

**Solution**:
1. Set `OPENAI_API_KEY` (already configured)
2. Change TTS provider to "openAI" in Voice Settings
3. Or add `ELEVENLABS_API_KEY` for premium voices

### Issue: Poor audio quality

**Recommendations**:
1. **Use ElevenLabs** for best British voices (requires API key)
2. **Use OpenAI TTS** for good quality, reliable service
3. **Increase sample rate** to 48000 Hz
4. **Enable noise suppression** in audio config
5. **Use "high" quality setting** for TTS

## Database Storage

Voice configuration is persisted in the SQLite database:

**Table**: `nora_voice_config`
- **id**: Always 1 (singleton)
- **config_json**: JSON serialized configuration
- **created_at**: Initial creation timestamp
- **updated_at**: Last modification timestamp

**Query Configuration**:
```sql
SELECT config_json FROM nora_voice_config WHERE id = 1;
```

**Update Configuration**:
```sql
UPDATE nora_voice_config 
SET config_json = '<json>', updated_at = datetime('now') 
WHERE id = 1;
```

## Voice Configuration Schema

Complete TypeScript/Rust schema:

```typescript
interface VoiceConfig {
  tts: {
    provider: 'elevenLabs' | 'openAI' | 'azure' | 'google' | 'system';
    voiceId: string;
    speed: number;        // 0.5 - 2.0
    volume: number;       // 0.0 - 1.0
    pitch: number;        // 0.5 - 2.0
    quality: 'low' | 'medium' | 'high';
    britishVoicePreferences: string[];
    fallbackProviders: string[];
  };
  stt: {
    provider: 'whisper' | 'azure' | 'google' | 'system';
    model: string;
    language: string;
    britishDialectSupport: boolean;
    executiveVocabulary: boolean;
    realTime: boolean;
    noiseReduction: boolean;
  };
  audio: {
    sampleRate: number;
    channels: number;
    bitDepth: number;
    bufferSize: number;
    noiseSuppression: boolean;
    echoCancellation: boolean;
    autoGainControl: boolean;
  };
  britishAccent: {
    accentStrength: number;
    regionalVariant: string;
    formalityLevel: string;
    vocabularyPreferences: string;
  };
  executiveMode: {
    enabled: boolean;
    proactiveCommunication: boolean;
    executiveSummaryStyle: boolean;
    formalAddress: boolean;
    businessVocabulary: boolean;
  };
}
```

## API Endpoints

- `GET /api/nora/voice/config` - Get current voice configuration
- `PUT /api/nora/voice/config` - Update voice configuration
- `POST /api/nora/voice/synthesize` - Synthesize speech from text
- `POST /api/nora/voice/transcribe` - Transcribe audio to text
- `POST /api/nora/voice/interaction` - Full voice interaction (STT → LLM → TTS)
