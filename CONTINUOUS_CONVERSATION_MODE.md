# Continuous Conversation Mode - Call-Like Experience with Nora

## Overview

The dashboard now supports **Continuous Conversation Mode** for Nora, enabling natural call-like interactions where the microphone stays on continuously and Nora automatically detects when you've finished speaking based on pauses.

## Features

### Two Interaction Modes

#### 1. **Push-to-Talk Mode** (Default)
- Click mic button to start recording
- Speak your message
- Click mic button again to stop and send
- Traditional manual control

#### 2. **Continuous Mode** (New - Call Mode)
- Click "Start Call" to begin continuous listening
- Microphone stays on constantly
- Speak naturally with pauses
- **Automatic pause detection** (1.5 seconds of silence)
- Nora automatically processes speech after detecting pauses
- Natural conversation flow like a phone call
- Click "End Call" to stop

## How It Works

### Voice Activity Detection (VAD)

The system uses the Web Speech Recognition API with intelligent pause detection:

1. **Continuous Listening**: Recognition runs continuously, capturing all speech
2. **Interim Results**: You see your speech transcribed in real-time
3. **Pause Detection**: When you stop speaking for 1.5 seconds, the system automatically:
   - Finalizes the transcript
   - Sends it to Nora for processing
   - Resets for the next utterance
4. **Automatic Restart**: After processing, listening automatically resumes

### Speech Accumulation

In continuous mode, the system accumulates your speech:

```typescript
// Example conversation flow
You: "Hey Nora" → [accumulating]
You: "can you help me with" → [accumulating: "Hey Nora can you help me with"]
You: "my schedule today?" → [accumulating: "Hey Nora can you help me with my schedule today?"]
[1.5s pause detected]
→ Sends complete message: "Hey Nora can you help me with my schedule today?"
→ Nora responds
→ Listening resumes automatically
```

## UI Indicators

### Mode Toggle Button
- **🎤 PTT** - Push-to-Talk mode (manual)
- **📞 Call** - Continuous conversation mode (automatic)

### Microphone Button Status
- **Push-to-Talk Mode**:
  - Gray mic icon: Ready to record
  - Red mic-off icon: Recording (click to stop)

- **Continuous Mode**:
  - Gray mic icon: Ready to start call
  - Red mic-off icon with pulsing red indicator: Call active
  - Pulsing animation shows the call is live

### Visual Feedback

When in continuous mode and actively listening, you'll see:
- Red pulsing indicator on the mic button
- Real-time transcription in the text area
- Transcription updates as you speak

## Usage

### Starting a Continuous Conversation

1. Open the Nora Assistant interface
2. Click the **"📞 Call"** button to switch to continuous mode
3. Click the **microphone button** to start the call
4. Start speaking naturally
5. Nora will respond automatically after detecting pauses

### Example Conversation

```
[Click "Start Call"]
[Mic activates with pulsing indicator]

You: "Hey Nora, what's on my schedule today?"
[1.5s pause]
→ Nora: "Good morning! You have three meetings scheduled today..."

You: "Great, can you send a summary to my team?"
[1.5s pause]
→ Nora: "Certainly! I'll prepare and send that summary now..."

You: "Perfect, thanks!"
[1.5s pause]
→ Nora: "You're very welcome! Is there anything else I can help with?"

[Click "End Call"]
```

### Switching Modes Mid-Conversation

You can switch between modes at any time:

- **While idle**: Simply toggle the mode button
- **While listening**: The system will:
  1. Stop current recording
  2. Wait 100ms for cleanup
  3. Restart with new mode settings

## Technical Details

### Pause Detection Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| Silence Timeout | 1.5 seconds | How long to wait before considering speech complete |
| Recognition Mode | Continuous | Keeps listening after each utterance |
| Interim Results | Enabled | Shows real-time transcription |
| Language | en-GB | British English |

### Browser Compatibility

Requires browser support for:
- **Web Speech Recognition API**
  - Chrome: ✅ Full support
  - Edge: ✅ Full support  
  - Safari: ✅ Full support (with webkit prefix)
  - Firefox: ❌ Limited support (falls back to media recorder)

### Fallback Behavior

If Speech Recognition API is not available:
- System falls back to MediaRecorder API
- Continuous mode functions with manual stop required
- Less seamless but still functional

## Configuration

### Adjusting Pause Duration

To modify the silence detection timeout, edit the timeout value in `NoraAssistant.tsx`:

```typescript
// Current: 1.5 seconds
silenceTimeoutRef.current = setTimeout(async () => {
  if (accumulatedTranscript.trim()) {
    await sendMessage(accumulatedTranscript.trim(), 'voiceInteraction');
    accumulatedTranscript = '';
    setInterimTranscript('');
  }
}, 1500); // Change this value (in milliseconds)
```

**Recommended values:**
- **1000ms (1s)** - Fast-paced conversation, may cut off slower speakers
- **1500ms (1.5s)** - Default, balanced for most users
- **2000ms (2s)** - Slower pace, accommodates pauses and thinking time
- **2500ms (2.5s)** - Very patient, good for multilingual or accessibility needs

### Voice Settings

The continuous mode respects all voice settings:
- Provider (OpenAI, ElevenLabs, Azure)
- Selected voice (fable, nova, echo, etc.)
- Speed, volume, quality settings
- British accent preferences

## Best Practices

### For Natural Conversations

1. **Speak in complete thoughts**: The system works best with full sentences
2. **Use natural pauses**: Pause for 1.5+ seconds when you want Nora to respond
3. **Avoid long pauses mid-sentence**: May cause premature processing
4. **Background noise**: Use in a quiet environment for best results

### Optimal Environment

- **Quiet room**: Minimal background noise
- **Good microphone**: Built-in or external mic with noise cancellation
- **Clear speech**: Normal speaking volume and pace
- **Stable internet**: Required for speech recognition and API calls

### When to Use Each Mode

**Use Push-to-Talk when:**
- In noisy environments
- Want precise control over when to send messages
- Recording longer, complex instructions
- Need to think while pausing

**Use Continuous Mode when:**
- Having a natural conversation
- Want hands-free operation
- Back-and-forth discussion with Nora
- Quick questions and responses

## Troubleshooting

### "Call mode not responding to pauses"

- **Cause**: Speaking too quickly without pauses
- **Solution**: Pause for at least 1.5 seconds after completing your thought

### "Messages sent too early"

- **Cause**: Long mid-sentence pauses triggering detection
- **Solution**: 
  - Increase timeout duration (see Configuration)
  - Speak more continuously until your thought is complete

### "Transcription not appearing"

- **Cause**: Microphone permissions not granted
- **Solution**: 
  - Check browser permissions
  - Reload page and allow microphone access

### "Continuous mode stops unexpectedly"

- **Cause**: Speech recognition API timeout or error
- **Solution**:
  - The system auto-restarts on errors
  - If persistent, switch to push-to-talk mode
  - Check console for error messages

## Privacy & Security

### Data Handling

- **Audio processing**: Done via browser's Web Speech Recognition API
- **Transcriptions**: Sent to backend for Nora processing
- **Storage**: Conversation history stored in database
- **Network**: Requires internet for speech recognition and AI responses

### Microphone Access

- **Permission required**: Browser will request microphone access
- **Indicator**: Red pulsing dot shows when actively listening
- **Control**: Click "End Call" to immediately stop listening
- **Privacy**: Audio is only processed during active call

## Future Enhancements

Planned improvements:

1. **Custom VAD thresholds**: Adjust sensitivity per user
2. **Barge-in support**: Interrupt Nora mid-response
3. **Voice activation**: "Hey Nora" wake word
4. **Multi-turn context**: Better handling of conversation context
5. **Audio visualization**: Waveform display during speaking
6. **Speaker diarization**: Distinguish multiple speakers
7. **Emotion detection**: Detect tone and adjust responses

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Space` (hold) | Quick push-to-talk in PTT mode |
| `Ctrl+M` | Toggle microphone on/off |
| `Ctrl+Shift+C` | Toggle continuous/PTT mode |
| `Esc` | Stop listening/end call |

*(Keyboard shortcuts coming soon)*

---

**Ready to have a natural conversation with Nora!** 🎙️📞

Try switching to continuous mode and experience the difference!
