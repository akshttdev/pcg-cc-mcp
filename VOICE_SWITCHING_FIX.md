# Voice Switching Fix - Technical Details

## Problem

When users changed the voice settings via the UI (e.g., switching from "fable" to "nova"), the audio synthesis continued using the old voice. The configuration was being updated in the database and frontend, but the actual voice didn't change.

## Root Cause

The voice synthesis system had a disconnect between configuration and execution:

1. **Configuration Layer**: The `TTSConfig` stores `voice_id` as a string (e.g., "fable", "nova", "echo")
2. **Execution Layer**: The `VoiceEngine.synthesize_speech()` method was hardcoding `VoiceProfile::BritishExecutiveFemale`
3. **OpenAI TTS**: Was mapping the `VoiceProfile` enum to voice names, ignoring the actual `voice_id` from config

```rust
// Before (crates/nora/src/voice/engine.rs:67)
let request = SpeechRequest {
    text: text.to_string(),
    voice_profile: VoiceProfile::BritishExecutiveFemale,  // ❌ Hardcoded!
    speed: self.config.tts.speed,
    volume: self.config.tts.volume,
    // ...
};
```

This meant that even when users changed `config.tts.voice_id` from "fable" to "nova", the system always used the hardcoded `BritishExecutiveFemale` profile.

## Solution

### 1. Dynamic Voice Profile Mapping (engine.rs)

Added a helper method to convert the `voice_id` string from config into the appropriate `VoiceProfile` enum:

```rust
fn voice_id_to_profile(voice_id: &str) -> VoiceProfile {
    match voice_id {
        // OpenAI voices
        "fable" | "nova" | "shimmer" => VoiceProfile::BritishExecutiveFemale,
        "echo" | "onyx" => VoiceProfile::BritishExecutiveMale,
        "alloy" => VoiceProfile::SystemDefault,
        
        // ElevenLabs voices
        "Rachel" | "british_executive_female" => VoiceProfile::BritishExecutiveFemale,
        "british_executive_male" => VoiceProfile::BritishExecutiveMale,
        
        // Azure voices
        "en-GB-SoniaNeural" | "en-GB-LibbyNeural" => VoiceProfile::BritishProfessionalFemale,
        "en-GB-RyanNeural" | "en-GB-ThomasNeural" => VoiceProfile::BritishProfessionalMale,
        
        // Default fallback
        _ => VoiceProfile::BritishExecutiveFemale,
    }
}
```

Updated `synthesize_speech()` to use this mapping:

```rust
// After
let voice_profile = Self::voice_id_to_profile(&self.config.tts.voice_id);

let request = SpeechRequest {
    text: text.to_string(),
    voice_profile,  // ✅ Dynamic based on config
    speed: self.config.tts.speed,
    volume: self.config.tts.volume,
    // ...
};
```

### 2. Direct Voice ID Usage (tts.rs)

Updated the OpenAI TTS implementation to use the actual `voice_id` from config when it's a valid OpenAI voice:

```rust
// Before
let voice = match request.voice_profile {
    VoiceProfile::BritishExecutiveFemale => "fable",
    VoiceProfile::BritishProfessionalFemale => "nova",
    // ...
};

// After
let valid_voices = ["alloy", "echo", "fable", "onyx", "nova", "shimmer"];
let voice = if valid_voices.contains(&self.config.voice_id.as_str()) {
    self.config.voice_id.as_str()  // ✅ Use config directly
} else {
    // Fall back to profile mapping for non-OpenAI voices
    match request.voice_profile {
        VoiceProfile::BritishExecutiveFemale => "fable",
        // ...
    }
};
```

This ensures that when a user selects "nova" in the UI:
1. Config is updated: `voice_id = "nova"`
2. Voice engine reloaded with new config
3. OpenAI TTS uses "nova" directly from `self.config.voice_id`
4. Correct voice is synthesized

### 3. Enhanced Logging

Added detailed logging to track voice selection:

```rust
info!("Synthesizing speech with OpenAI voice: {} (config voice_id: {}, profile: {:?})", 
      voice, self.config.voice_id, request.voice_profile);
```

## Data Flow

### Before Fix
```
User selects "nova" in UI
    ↓
Frontend sends PUT /api/nora/voice/config
    ↓
Backend updates config.tts.voice_id = "nova"
    ↓
Backend creates new VoiceEngine with updated config
    ↓
User requests synthesis
    ↓
engine.synthesize_speech() hardcodes BritishExecutiveFemale
    ↓
OpenAI TTS maps BritishExecutiveFemale → "fable"
    ↓
❌ Wrong voice! User hears "fable" instead of "nova"
```

### After Fix
```
User selects "nova" in UI
    ↓
Frontend sends PUT /api/nora/voice/config
    ↓
Backend updates config.tts.voice_id = "nova"
    ↓
Backend creates new VoiceEngine with updated config
    ↓
User requests synthesis
    ↓
engine.synthesize_speech() calls voice_id_to_profile("nova")
    ↓
Returns VoiceProfile::BritishExecutiveFemale
    ↓
OpenAI TTS checks valid_voices.contains("nova") → true
    ↓
Uses config.voice_id directly: "nova"
    ↓
✅ Correct voice! User hears "nova"
```

## Testing

Run the test script to verify voice switching works:

```bash
./scripts/test-voice-switching.sh
```

This will:
1. Get current voice configuration
2. Switch to "nova" voice
3. Synthesize audio with nova
4. Switch to "echo" voice
5. Synthesize audio with echo
6. Switch back to "fable"
7. Verify final configuration

You should hear three different voices in the generated audio files.

## Files Changed

1. **crates/nora/src/voice/engine.rs**
   - Added `voice_id_to_profile()` helper method
   - Updated `synthesize_speech()` to use dynamic voice profile

2. **crates/nora/src/voice/tts.rs**
   - Removed `#[allow(dead_code)]` from `config` field
   - Updated OpenAI TTS to check `config.voice_id` first
   - Enhanced logging with config and profile info

3. **scripts/test-voice-switching.sh** (new)
   - Automated test for voice switching functionality

## Verification

After restarting the backend (`pnpm run dev`), verify the fix:

### Via UI
1. Open http://localhost:5173
2. Navigate to Nora → Voice Controls
3. Change voice from "Fable" to "Nova"
4. Click a button to trigger speech synthesis
5. Verify you hear the nova voice (warm female)

### Via API
```bash
# Switch to echo
curl -X PUT http://localhost:3000/api/nora/voice/config \
  -H "Content-Type: application/json" \
  -d '{"config": {"tts": {"voiceId": "echo", ...}}}'

# Synthesize with echo
curl -X POST http://localhost:3000/api/nora/voice/synthesize \
  -H "Content-Type: application/json" \
  -d '{"text": "This is the echo voice"}' \
  | jq -r '.audioData' | base64 -d > test.mp3

afplay test.mp3  # Should hear echo voice (male)
```

### Check Logs
Look for this in backend logs:
```
INFO synthesizing speech with OpenAI voice: nova (config voice_id: nova, profile: BritishExecutiveFemale)
```

The voice name should match what you selected in the UI.

## Future Improvements

1. **Voice Previews**: Add "Test Voice" button in UI to preview each voice
2. **Voice Metadata**: Store voice characteristics (gender, accent, tone) in database
3. **Voice Favorites**: Let users save favorite voices for quick switching
4. **Voice History**: Track which voices were used and when
5. **Custom Voice Mappings**: Allow users to define their own profile→voice_id mappings

## Compatibility

This fix is backward compatible:
- Existing configs with `voice_id = "british_executive_female"` still work
- ElevenLabs and Azure voices map correctly via profile
- System falls back to BritishExecutiveFemale for unknown voices

## Performance Impact

Minimal:
- `voice_id_to_profile()` is a simple string match (O(1))
- No additional API calls
- Voice engine is only recreated when config changes (not on every synthesis)
