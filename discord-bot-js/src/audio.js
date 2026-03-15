/**
 * Audio helpers: WAV encoding, STT (Whisper), TTS (ElevenLabs/OpenAI), wake word detection
 */

// ─── WAV encoding ─────────────────────────────────────────────────────────────

/**
 * Convert 48kHz stereo PCM (i16 interleaved) captured from Discord voice into
 * a 16kHz mono WAV buffer suitable for Whisper.
 */
export function downsampleAndEncodeWav(pcmBuffer) {
  // pcmBuffer is a Node.js Buffer of raw i16 LE stereo 48kHz samples
  const sampleCount = pcmBuffer.length / 2; // number of i16 samples
  const monoDownsampled = [];

  // stereo pairs are L, R, L, R...
  // Downsample 48kHz→16kHz: take every 3rd pair (ratio = 48000/16000 = 3)
  for (let i = 0; i < sampleCount; i += 6) {
    // i and i+1 are one stereo pair; step by 6 to get every 3rd pair
    const l = pcmBuffer.readInt16LE(i * 2);
    const r = pcmBuffer.readInt16LE((i + 1) * 2);
    monoDownsampled.push((l + r) >> 1);
  }

  const pcm16k = Buffer.allocUnsafe(monoDownsampled.length * 2);
  for (let i = 0; i < monoDownsampled.length; i++) {
    pcm16k.writeInt16LE(monoDownsampled[i], i * 2);
  }

  return buildWavHeader(pcm16k, 16000, 1);
}

function buildWavHeader(pcmData, sampleRate, channels) {
  const dataSize = pcmData.length;
  const header = Buffer.allocUnsafe(44);
  header.write('RIFF', 0);
  header.writeUInt32LE(36 + dataSize, 4);
  header.write('WAVE', 8);
  header.write('fmt ', 12);
  header.writeUInt32LE(16, 16);                               // fmt chunk size
  header.writeUInt16LE(1, 20);                                // PCM format
  header.writeUInt16LE(channels, 22);
  header.writeUInt32LE(sampleRate, 24);
  header.writeUInt32LE(sampleRate * channels * 2, 28);        // byte rate
  header.writeUInt16LE(channels * 2, 32);                     // block align
  header.writeUInt16LE(16, 34);                               // bits per sample
  header.write('data', 36);
  header.writeUInt32LE(dataSize, 40);
  return Buffer.concat([header, pcmData]);
}

// ─── STT ──────────────────────────────────────────────────────────────────────

/**
 * Transcribe WAV audio via local Whisper server or OpenAI Whisper API.
 * Returns null if audio is too short or no provider is configured.
 */
export async function transcribeWav(wavBuffer) {
  // Require at least 0.5 seconds: 16kHz mono i16 = 16000*2 bytes + 44 header
  const MIN_BYTES = 16_044;
  if (wavBuffer.length < MIN_BYTES) {
    console.debug(`[STT] Audio too short (${wavBuffer.length} bytes < ${MIN_BYTES}), skipping`);
    return null;
  }

  console.log(`[STT] Transcribing ${wavBuffer.length} byte WAV (~${((wavBuffer.length - 44) / 32000).toFixed(1)}s audio)`);

  const whisperUrl = process.env.WHISPER_URL;
  if (whisperUrl) {
    try {
      // Try /transcribe/file (whisper_server.py) then /inference (whispercpp/other servers)
      for (const path of ['/transcribe/file', '/inference']) {
        const form = new FormData();
        form.append('file', new Blob([wavBuffer], { type: 'audio/wav' }), 'audio.wav');
        form.append('language', 'en');
        try {
          const res = await fetch(`${whisperUrl.replace(/\/$/, '')}${path}`, {
            method: 'POST',
            body: form,
            signal: AbortSignal.timeout(30_000),
          });
          if (res.ok) {
            const json = await res.json();
            const text = (json.text ?? json.transcript)?.trim();
            if (text) {
              console.log(`[STT] Local Whisper result: "${text.slice(0, 80)}"`);
              return text;
            }
            // Empty transcription — nothing heard, stop here
            return null;
          }
        } catch (pathErr) {
          if (path === '/transcribe/file') continue; // try next path
          throw pathErr;
        }
      }
    } catch (e) {
      console.warn('[STT] Local Whisper failed:', e.message);
    }
  }

  // OpenAI Whisper fallback — only if no local provider configured
  if (!whisperUrl) {
    const apiKey = process.env.OPENAI_API_KEY;
    if (apiKey) {
      try {
        const form = new FormData();
        form.append('file', new Blob([wavBuffer], { type: 'audio/wav' }), 'audio.wav');
        form.append('model', 'whisper-1');
        form.append('language', 'en');
        const res = await fetch('https://api.openai.com/v1/audio/transcriptions', {
          method: 'POST',
          headers: { Authorization: `Bearer ${apiKey}` },
          body: form,
        });
        if (res.ok) {
          const json = await res.json();
          const text = json.text?.trim() || null;
          console.log(`[STT] OpenAI Whisper result: "${text?.slice(0, 80) ?? '(empty)'}"`);
          return text;
        } else {
          const body = await res.text();
          console.warn(`[STT] OpenAI Whisper returned ${res.status}: ${body.slice(0, 200)}`);
        }
      } catch (e) {
        console.warn('[STT] OpenAI Whisper failed:', e.message);
      }
    }
  }

  return null;
}

// ─── Wake word detection ──────────────────────────────────────────────────────

const WAKE_WORDS = {
  nora: ['hey nora', 'ok nora', 'nora'],
  topsi: ['hey topsi', 'ok topsi', 'topsi'],
};

// Regex for phonetic Topsi variants — Whisper consistently mishears it as
// tulsi, tofsi, topsy, tapsi, topci, topzy, topsei, topsie, etc.
// Pattern: starts with T, 4-7 chars, ends with s+vowel/y or si/sy
const TOPSI_REGEX = /\bt[aou][a-z]{0,2}s[aeiouy]\b/i;

/**
 * Checks whether the transcript addresses the given agent.
 * Returns the text after the wake word (the actual command), or null if not addressed.
 */
export function detectWakeWord(text, agentName) {
  const lower = text.toLowerCase();

  // Regex fallback for Topsi — catches all phonetic mishearings automatically
  if (agentName.toLowerCase() === 'topsi') {
    const m = lower.match(TOPSI_REGEX);
    if (m) {
      const idx = m.index + m[0].length;
      const after = text.slice(idx).replace(/^[^a-z0-9]+/i, '').trim();
      return after || text.trim();
    }
  }

  const words = WAKE_WORDS[agentName.toLowerCase()] ?? [];
  for (const w of words) {
    const idx = lower.indexOf(w);
    if (idx !== -1) {
      const after = text.slice(idx + w.length).replace(/^[^a-z0-9]+/i, '').trim();
      return after || text.trim();
    }
  }
  return null;
}

// ─── Agent call ───────────────────────────────────────────────────────────────

export async function callAgent(serverPort, agentName, message, projectId, sessionId, participantCtx = '') {
  const endpoint = agentName.toLowerCase() === 'topsi' ? 'topsi' : 'nora';
  const identities = {
    nora: `You are Nora, PowerClub Global's Executive AI Agent. You are speaking via Discord voice. British English.`,
    topsi: `You are Topsi, PowerClub Global's technical AI agent. You are speaking via Discord voice.`,
  };
  const identity = identities[agentName.toLowerCase()] ?? `You are ${agentName}, a PowerClub Global AI agent.`;
  const contextPart = participantCtx ? ` ${participantCtx}` : '';
  const lengthGuide = agentName.toLowerCase() === 'topsi'
    ? 'No markdown. Respond at whatever length the user requests — if asked to read aloud, read the full text without summarising.'
    : 'Keep your final spoken response to 1-3 sentences, no markdown.';
  const prefix = `[DISCORD VOICE CALL — ${identity}${contextPart} Speaking aloud — ${lengthGuide} You MAY and SHOULD still call tools (search_web, fetch_web_page, render_page, etc.) when the user asks you to look something up, fetch a URL, or search the web — tool use is encouraged.]`;

  const res = await fetch(`http://127.0.0.1:${serverPort}/api/internal/${endpoint}/chat`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-Admin-Key': process.env.ADMIN_API_KEY ?? '',
    },
    body: JSON.stringify({
      message: `${prefix}\n\n${message}`,
      sessionId,
    }),
  });

  if (!res.ok) {
    const body = await res.text();
    throw new Error(`Agent returned ${res.status}: ${body}`);
  }

  const json = await res.json();
  const response = json.response ?? json.message ?? json.content ?? 'Understood.';
  console.log(`[${agentName}] Response: ${response.slice(0, 80)}`);
  return response;
}

// ─── TTS ──────────────────────────────────────────────────────────────────────

/**
 * Synthesize text to an MP3 buffer.
 * Tries ElevenLabs first, falls back to OpenAI TTS.
 */
// Nora uses ElevenLabs (Mia Moore — British Studio Presenter)
// Topsi uses Chatterbox (local, matches dashboard config)
const ELEVENLABS_NORA_VOICE = process.env.ELEVENLABS_NORA_VOICE_ID ?? 'ZtcPZrt9K4w8e1OB9M6w';
const CHATTERBOX_URL = process.env.CHATTERBOX_URL ?? 'http://localhost:8102';

export async function synthesizeTts(text, agentName = 'nora') {
  const agent = agentName.toLowerCase();

  // ── Topsi: Piper TTS (en_GB-semaine-medium, prudence speaker) ────────────
  // Matches the dashboard voice: computer-sounding semaine female voice
  if (agent === 'topsi') {
    try {
      const res = await fetch(`${CHATTERBOX_URL.replace(/\/$/, '')}/tts`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          text,
          speaker_id: 'prudence',
          speed: 1.0,
        }),
        signal: AbortSignal.timeout(30_000),
      });
      if (res.ok) {
        console.log('[TTS] Piper semaine/prudence (Topsi)');
        return Buffer.from(await res.arrayBuffer());
      }
      console.warn(`[TTS] Piper returned ${res.status}`);
    } catch (e) {
      console.warn('[TTS] Piper failed:', e.message);
    }
    throw new Error('Piper TTS unavailable for Topsi — is it running on port 8102?');
  }

  // ── Nora: ElevenLabs (Mia Moore) ─────────────────────────────────────────
  const elKey = process.env.ELEVENLABS_API_KEY;
  if (elKey) {
    try {
      const res = await fetch(`https://api.elevenlabs.io/v1/text-to-speech/${ELEVENLABS_NORA_VOICE}`, {
        method: 'POST',
        headers: {
          'xi-api-key': elKey,
          'Content-Type': 'application/json',
          Accept: 'audio/mpeg',
        },
        body: JSON.stringify({
          text,
          model_id: 'eleven_monolingual_v1',
          voice_settings: { stability: 0.5, similarity_boost: 0.75 },
        }),
      });
      if (res.ok) {
        console.log('[TTS] ElevenLabs Mia Moore (Nora)');
        return Buffer.from(await res.arrayBuffer());
      }
      console.warn(`[TTS] ElevenLabs returned ${res.status}`);
    } catch (e) {
      console.warn('[TTS] ElevenLabs failed:', e.message);
    }
  }

  throw new Error('No TTS provider available for Nora — check ELEVENLABS_API_KEY');
}
