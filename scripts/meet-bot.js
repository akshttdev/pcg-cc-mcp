#!/usr/bin/env node
/**
 * Nora Google Meet Bot
 *
 * Joins a Google Meet as Nora, routes audio through PipeWire virtual devices,
 * sends participant audio to the PCG server for STT → Nora LLM → TTS,
 * and plays Nora's voice responses back into the meeting.
 *
 * Usage:
 *   node meet-bot.js <meet_url> <session_id> <server_url> [profile_dir]
 *
 * Audio routing (PipeWire/PulseAudio):
 *   - Virtual null sink  "nora-meet-out"  ← Chrome plays meeting audio here
 *   - Monitor of above   "nora-meet-out.monitor" → parec → Whisper STT
 *   - Virtual null sink  "nora-meet-tts"  ← paplay sends TTS audio here
 *   - Virtual source     "nora-meet-mic"  = monitor of nora-meet-tts → Chrome mic
 */

const { chromium } = require('playwright');
const { execSync, spawn } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');
const https = require('https');
const http = require('http');

// ── Args ────────────────────────────────────────────────────────────────────
const [,, meetUrl, sessionId, serverUrl = 'http://localhost:3000', profileDir] = process.argv;

if (!meetUrl || !sessionId) {
  console.error(JSON.stringify({ type: 'error', message: 'Usage: node meet-bot.js <meet_url> <session_id> <server_url> [profile_dir]' }));
  process.exit(1);
}

const SERVER = serverUrl.replace(/\/$/, '');
const PROFILE_DIR = profileDir || path.join(os.homedir(), 'nora-chrome-profile');
const CHUNK_DIR = path.join(os.tmpdir(), `meet-audio-${sessionId}`);
const TTS_DIR   = path.join(os.tmpdir(), `meet-tts-${sessionId}`);

// Audio device names (unique per session to avoid conflicts)
const OUT_SINK  = `nora-meet-out-${sessionId.slice(0, 8)}`;
const TTS_SINK  = `nora-meet-tts-${sessionId.slice(0, 8)}`;
const MIC_SRC   = `nora-meet-mic-${sessionId.slice(0, 8)}`;

// Suppress EPIPE on stdout — parent process may close the pipe before we finish cleanup
process.stdout.on('error', (err) => { if (err.code !== 'EPIPE') throw err; });

// Crash detection — log unhandled errors before dying
process.on('unhandledRejection', (reason) => {
  try { process.stdout.write(JSON.stringify({ type: 'error', message: 'Unhandled rejection', detail: String(reason) }) + '\n'); } catch (_) {}
  process.exit(1);
});
process.on('uncaughtException', (err) => {
  try { process.stdout.write(JSON.stringify({ type: 'error', message: 'Uncaught exception', detail: err.message, stack: err.stack }) + '\n'); } catch (_) {}
  process.exit(1);
});

// ── Logging ─────────────────────────────────────────────────────────────────
function emit(obj) {
  try {
    process.stdout.write(JSON.stringify(obj) + '\n');
  } catch (_) { /* stdout closed — ignore */ }
}

function log(message, data = {}) {
  emit({ type: 'log', message, ...data });
}

function error(message, err = {}) {
  emit({ type: 'error', message, error: err.message || String(err) });
}

// ── HTTP helpers ─────────────────────────────────────────────────────────────
function post(url, body, opts = {}) {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const lib = parsed.protocol === 'https:' ? https : http;
    // Handle Buffer (binary) bodies directly; stringify everything else
    const payload = Buffer.isBuffer(body) ? body
      : (typeof body === 'string' ? body : JSON.stringify(body));
    const req = lib.request({
      hostname: parsed.hostname,
      port: parsed.port || (parsed.protocol === 'https:' ? 443 : 80),
      path: parsed.pathname + parsed.search,
      method: 'POST',
      headers: { 'Content-Type': opts.contentType || 'application/json', 'Content-Length': Buffer.byteLength(payload), ...opts.headers },
    }, res => {
      let data = '';
      res.on('data', d => data += d);
      res.on('end', () => resolve({ status: res.statusCode, body: data }));
    });
    req.on('error', reject);
    req.write(payload);
    req.end();
  });
}

function get(url, opts = {}) {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const lib = parsed.protocol === 'https:' ? https : http;
    const req = lib.request({
      hostname: parsed.hostname,
      port: parsed.port || (parsed.protocol === 'https:' ? 443 : 80),
      path: parsed.pathname + parsed.search,
      method: 'GET',
      headers: opts.headers || {},
    }, res => {
      const chunks = [];
      res.on('data', d => chunks.push(d));
      res.on('end', () => resolve({ status: res.statusCode, body: Buffer.concat(chunks) }));
    });
    req.on('error', reject);
    req.end();
  });
}

// ── PipeWire/PulseAudio audio setup ─────────────────────────────────────────
let outSinkModule  = null;
let ttsSinkModule  = null;
let micSrcModule   = null;

function pactl(...args) {
  try {
    return execSync(['pactl', ...args].join(' '), { stdio: ['pipe', 'pipe', 'pipe'] }).toString().trim();
  } catch (e) {
    return null;
  }
}

function setupAudio() {
  log('Setting up PipeWire virtual audio devices');

  // Meeting output: Chrome plays meeting participants audio here
  const outResult = pactl(`load-module module-null-sink sink_name=${OUT_SINK} sink_properties=device.description="Nora-Meet-Output"`);
  if (outResult) outSinkModule = outResult;

  // TTS input: we play Nora's TTS audio here, Chrome picks it up as mic
  const ttsResult = pactl(`load-module module-null-sink sink_name=${TTS_SINK} sink_properties=device.description="Nora-TTS-Sink"`);
  if (ttsResult) ttsSinkModule = ttsResult;

  // Virtual source from TTS sink monitor → Chrome microphone
  const micResult = pactl(`load-module module-virtual-source source_name=${MIC_SRC} master=${TTS_SINK}.monitor source_properties=device.description="Nora-Meet-Mic"`);
  if (micResult) micSrcModule = micResult;

  // Set as PulseAudio server defaults so Chrome picks them up automatically.
  // Virtual sources are registered by PipeWire as "output.<name>", not "<name>".
  pactl(`set-default-sink ${OUT_SINK}`);
  pactl(`set-default-source output.${MIC_SRC}`);

  log('Audio devices created', { outSink: OUT_SINK, ttsSink: TTS_SINK, micSrc: MIC_SRC });
}

function teardownAudio() {
  log('Tearing down audio devices');
  if (micSrcModule)  pactl(`unload-module ${micSrcModule}`);
  if (ttsSinkModule) pactl(`unload-module ${ttsSinkModule}`);
  if (outSinkModule) pactl(`unload-module ${outSinkModule}`);
}

// ── Audio capture (parec → chunk files → server) ────────────────────────────
let parec = null;
let chunkIndex = 0;
let chunkBuffer = Buffer.alloc(0);
const SAMPLE_RATE  = 16000;
const CHANNELS     = 1;
const BYTES_SAMPLE = 2; // s16le
// 2 seconds of audio per chunk
const CHUNK_SAMPLES  = SAMPLE_RATE * 2;
const CHUNK_BYTES    = CHUNK_SAMPLES * CHANNELS * BYTES_SAMPLE;

function wavHeader(dataLength) {
  const buf = Buffer.alloc(44);
  buf.write('RIFF', 0);
  buf.writeUInt32LE(36 + dataLength, 4);
  buf.write('WAVE', 8);
  buf.write('fmt ', 12);
  buf.writeUInt32LE(16, 16);
  buf.writeUInt16LE(1, 20);   // PCM
  buf.writeUInt16LE(CHANNELS, 22);
  buf.writeUInt32LE(SAMPLE_RATE, 24);
  buf.writeUInt32LE(SAMPLE_RATE * CHANNELS * BYTES_SAMPLE, 28);
  buf.writeUInt16LE(CHANNELS * BYTES_SAMPLE, 32);
  buf.writeUInt16LE(16, 34);  // bits per sample
  buf.write('data', 36);
  buf.writeUInt32LE(dataLength, 40);
  return buf;
}

async function sendAudioChunk(pcmData) {
  const wavData = Buffer.concat([wavHeader(pcmData.length), pcmData]);
  const tmpFile = path.join(CHUNK_DIR, `chunk_${chunkIndex++}.wav`);
  fs.writeFileSync(tmpFile, wavData);

  try {
    const result = await post(`${SERVER}/api/nora/meet/${sessionId}/audio`, wavData, {
      contentType: 'audio/wav',
    });
    if (result.status === 200) {
      // Server returns TTS audio directly if Nora has a response
      const body = JSON.parse(result.body);
      if (body.transcript) {
        emit({ type: 'transcript', text: body.transcript, speaker: 'participant' });
      }
      if (body.tts_url) {
        playTTS(body.tts_url);
      }
    }
  } catch (e) {
    // Non-blocking — audio chunks are fire-and-forget
  } finally {
    try { fs.unlinkSync(tmpFile); } catch (_) {}
  }
}

function startCapture() {
  log('Starting audio capture from meeting output monitor');

  fs.mkdirSync(CHUNK_DIR, { recursive: true });
  fs.mkdirSync(TTS_DIR,   { recursive: true });

  parec = spawn('parec', [
    '--device', `${OUT_SINK}.monitor`,
    '--format=s16le',
    `--rate=${SAMPLE_RATE}`,
    '--channels=1',
    '--latency-msec=100',
  ]);

  parec.stdout.on('data', async (data) => {
    chunkBuffer = Buffer.concat([chunkBuffer, data]);
    while (chunkBuffer.length >= CHUNK_BYTES) {
      const chunk = chunkBuffer.slice(0, CHUNK_BYTES);
      chunkBuffer = chunkBuffer.slice(CHUNK_BYTES);
      // Send in background
      sendAudioChunk(chunk).catch(() => {});
    }
  });

  parec.stderr.on('data', d => log(`[parec] ${d.toString().trim()}`));
  parec.on('exit', (code) => log(`parec exited with code ${code}`));
}

function stopCapture() {
  if (parec) {
    parec.kill('SIGTERM');
    parec = null;
  }
}

// ── TTS playback ─────────────────────────────────────────────────────────────
let ttsQueue = [];
let ttsPlaying = false;

async function playTTS(ttsUrl) {
  ttsQueue.push(ttsUrl);
  if (!ttsPlaying) processTTSQueue();
}

async function processTTSQueue() {
  if (ttsQueue.length === 0) { ttsPlaying = false; return; }
  ttsPlaying = true;
  const url = ttsQueue.shift();

  try {
    // Download TTS audio
    const res = await get(url);
    if (res.status === 200) {
      const tmpFile = path.join(TTS_DIR, `tts_${Date.now()}.wav`);
      fs.writeFileSync(tmpFile, res.body);
      emit({ type: 'speaking', message: 'Nora is speaking' });
      await new Promise((resolve) => {
        const proc = spawn('paplay', ['--device', TTS_SINK, tmpFile]);
        proc.on('exit', () => { try { fs.unlinkSync(tmpFile); } catch (_) {} resolve(); });
      });
      emit({ type: 'speaking_done' });
    }
  } catch (e) {
    error('TTS playback error', e);
  }

  processTTSQueue();
}

// ── TTS polling (for responses queued by server) ─────────────────────────────
let pollingActive = false;

async function pollForTTS() {
  pollingActive = true;
  while (pollingActive) {
    try {
      const res = await get(`${SERVER}/api/nora/meet/${sessionId}/next-tts`, {
        headers: { 'X-Timeout': '20000' },
      });
      if (res.status === 200 && res.body.length > 44) {
        // Got WAV audio directly
        const tmpFile = path.join(TTS_DIR, `poll_${Date.now()}.wav`);
        fs.writeFileSync(tmpFile, res.body);
        emit({ type: 'speaking', message: 'Nora is speaking (polled)' });
        await new Promise(resolve => {
          const proc = spawn('paplay', ['--device', TTS_SINK, tmpFile]);
          proc.on('exit', () => { try { fs.unlinkSync(tmpFile); } catch (_) {} resolve(); });
        });
        emit({ type: 'speaking_done' });
      }
    } catch (_) {}
    await new Promise(r => setTimeout(r, 500));
  }
}

// ── Google Meet page joining ──────────────────────────────────────────────────
async function joinMeet(page) {
  log('Navigating to Google Meet as guest', { url: meetUrl });

  await page.goto(meetUrl, { waitUntil: 'domcontentloaded', timeout: 30000 });
  await page.waitForTimeout(3000);

  // Handle cookie/consent dialogs
  try {
    const acceptBtn = page.locator('button:has-text("Accept all"), button:has-text("Accept"), button:has-text("I agree"), button:has-text("Reject all")').first();
    if (await acceptBtn.isVisible({ timeout: 3000 })) {
      await acceptBtn.click();
      await page.waitForTimeout(1000);
    }
  } catch (_) {}

  // If redirected to Google sign-in, navigate to guest join URL
  const currentUrl = page.url();
  log('Current URL', { url: currentUrl });
  if (currentUrl.includes('accounts.google.com')) {
    log('Sign-in redirect detected — going back to Meet to join as guest');
    await page.goto(meetUrl, { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(2000);
  }

  // Step 1: Click "Continue without signing in" / "Use without an account" if shown
  const guestSelectors = [
    'button:has-text("Continue without signing in")',
    'button:has-text("Use without an account")',
    'a:has-text("Use without an account")',
    'button:has-text("Join as guest")',
  ];
  for (const sel of guestSelectors) {
    try {
      const btn = page.locator(sel).first();
      if (await btn.isVisible({ timeout: 2000 })) {
        log('Clicking guest option', { selector: sel });
        await btn.click();
        await page.waitForTimeout(1500);
        break;
      }
    } catch (_) {}
  }

  // Step 2: Fill in name field (guest join)
  try {
    const nameSelectors = [
      'input[placeholder*="name" i]',
      'input[aria-label*="name" i]',
      'input[aria-label*="Your name" i]',
      'input[type="text"]',
    ];
    for (const sel of nameSelectors) {
      try {
        const input = page.locator(sel).first();
        if (await input.isVisible({ timeout: 2000 })) {
          await input.fill('Nora');
          log('Entered guest name: Nora');
          await page.waitForTimeout(500);
          break;
        }
      } catch (_) {}
    }
  } catch (_) {}

  // Step 3: Turn off mic and camera before joining (Nora controls these via PipeWire)
  const muteSelectors = [
    '[aria-label*="Turn off microphone"]',
    '[aria-label*="microphone"][aria-pressed="true"]',
    '[data-is-muted="false"][aria-label*="mic" i]',
  ];
  for (const sel of muteSelectors) {
    try {
      const btn = page.locator(sel).first();
      if (await btn.isVisible({ timeout: 1000 })) {
        await btn.click();
        log('Pre-muted microphone');
        break;
      }
    } catch (_) {}
  }

  // Handle "note taker" / recording notice dialog if shown
  try {
    const noticeSelectors = [
      'button:has-text("Got it")',
      'button:has-text("OK")',
      'button:has-text("Accept")',
      'button:has-text("Continue")',
      '[aria-label*="Got it"]',
    ];
    for (const sel of noticeSelectors) {
      try {
        const btn = page.locator(sel).first();
        if (await btn.isVisible({ timeout: 2000 })) {
          log('Dismissing notice dialog', { selector: sel });
          await btn.click();
          await page.waitForTimeout(1000);
          break;
        }
      } catch (_) {}
    }
  } catch (_) {}

  // Step 4: Click "Join now" / "Ask to join"
  const joinSelectors = [
    'button:has-text("Join now")',
    'button:has-text("Ask to join")',
    'button:has-text("Join")',
    '[aria-label*="Join now"]',
    '[aria-label*="Ask to join"]',
  ];

  let joined = false;
  for (let attempt = 0; attempt < 5 && !joined; attempt++) {
    for (const sel of joinSelectors) {
      try {
        const btn = page.locator(sel).first();
        if (await btn.isVisible({ timeout: 2000 })) {
          log('Clicking join button', { selector: sel });
          await btn.click();
          joined = true;
          break;
        }
      } catch (_) {}
    }
    if (!joined) await page.waitForTimeout(2000);
  }

  if (!joined) {
    log('Could not find join button — taking screenshot for debug');
    try {
      await page.screenshot({ path: '/tmp/meet-debug.png', fullPage: true });
      emit({ type: 'debug_screenshot', path: '/tmp/meet-debug.png', url: page.url() });
    } catch (_) {}
    return false;
  }

  // Step 5: Wait for meeting to fully load (or lobby admission)
  // If we're in the lobby ("Ask to join"), wait up to 5 min for host to admit us
  log('In lobby — waiting for admission or direct join...');
  const lobbyStart = Date.now();
  let admitted = false;
  while (Date.now() - lobbyStart < 300000) { // 5 min
    try {
      await page.waitForTimeout(3000);
    } catch (_) {
      log('Page closed during lobby wait — was not admitted');
      return false;
    }

    let url;
    try { url = page.url(); } catch (_) { return false; }

    // Check for "You can't join" / redirect away — try dismissing or re-navigating
    if (url.includes('workspace.google.com') || url.includes('google.com/sorry')) {
      log('Redirected away from meeting', { url });
      return false;
    }

    // Dismiss any dialogs that popped up in lobby (recording notice, etc.)
    for (const sel of ['button:has-text("Got it")', 'button:has-text("OK")', 'button:has-text("Accept")', 'button:has-text("Continue")']) {
      try {
        const btn = page.locator(sel).first();
        if (await btn.isVisible({ timeout: 400 })) {
          await btn.click();
          log('Dismissed lobby dialog', { selector: sel });
          break;
        }
      } catch (_) {}
    }

    if (url.includes('meet.google.com/')) {
      // Look for in-meeting indicators
      const inMeeting = await page.locator(
        '[aria-label*="Leave call"], [aria-label*="Leave meeting"], [data-tooltip*="Leave"]'
      ).first().isVisible({ timeout: 1000 }).catch(() => false);
      if (inMeeting) {
        log('Admitted to meeting — now live');
        admitted = true;
        break;
      }
    }
    log('Still in lobby, waiting...', { elapsed: Math.round((Date.now() - lobbyStart) / 1000) });
  }
  if (!admitted) return false;

  // Dismiss any dialogs that appear immediately after being admitted
  await page.waitForTimeout(1500);
  const postJoinDismiss = [
    'button:has-text("Got it")',
    'button:has-text("OK")',
    'button:has-text("Accept")',
    'button:has-text("Continue")',
    'button:has-text("I understand")',
    'button:has-text("Dismiss")',
  ];
  for (const sel of postJoinDismiss) {
    try {
      const btn = page.locator(sel).first();
      if (await btn.isVisible({ timeout: 800 })) {
        await btn.click();
        log('Dismissed post-join dialog', { selector: sel });
        await page.waitForTimeout(500);
      }
    } catch (_) {}
  }

  emit({ type: 'joined', message: 'Nora has joined the meeting' });
  return true;
}

// ── Main ─────────────────────────────────────────────────────────────────────
async function main() {
  emit({ type: 'starting', sessionId, meetUrl });

  // Setup audio routing
  setupAudio();

  let browser;
  try {
    log('Launching Chromium browser');

    // Build launch environment with audio device overrides
    const env = {
      ...process.env,
      DISPLAY: process.env.DISPLAY || ':1',
      PULSE_SINK: OUT_SINK,
      PULSE_SOURCE: `output.${MIC_SRC}`,
    };

    const chromiumExec = process.env.CHROMIUM_PATH
      || '/home/pythia/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome';

    // Clear Chrome crash-recovery state so the "Restore pages?" dialog never appears.
    // Chrome writes Last Session / Last Tabs when it exits uncleanly (e.g. SIGKILL).
    // On next launch it shows a blocking modal; deleting these files prevents it.
    const defaultDir = path.join(PROFILE_DIR, 'Default');
    for (const f of ['Last Session', 'Last Tabs', 'Last Browser']) {
      try { fs.unlinkSync(path.join(defaultDir, f)); } catch (_) { /* doesn't exist, fine */ }
    }

    const launchArgs = [
      '--no-sandbox',
      '--disable-setuid-sandbox',
      '--disable-dev-shm-usage',
      '--disable-blink-features=AutomationControlled',
      '--no-first-run',
      '--no-default-browser-check',
      '--autoplay-policy=no-user-gesture-required',
      '--disable-background-timer-throttling',
      '--disable-renderer-backgrounding',
      '--allow-running-insecure-content',
      '--disable-gpu',
      '--disable-session-crashed-bubble',
      '--disable-infobars',
      '--restore-last-session=false',
      `--display=${env.DISPLAY}`,
    ];

    // launchPersistentContext is the correct API for a user-data-dir
    const context = await chromium.launchPersistentContext(PROFILE_DIR, {
      headless: false,
      executablePath: chromiumExec,
      env,
      args: launchArgs,
      permissions: ['microphone', 'camera', 'notifications'],
      viewport: { width: 1280, height: 720 },
      userAgent: 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36',
    });

    // Grant media permissions for Meet
    await context.grantPermissions(['microphone', 'camera'], { origin: 'https://meet.google.com' });

    browser = context; // treat context as the thing to close on cleanup

    const page = context.pages()[0] || await context.newPage();

    // Inject display name "Nora" for guest join + disable noise suppression so TTS passes through
    await page.addInitScript(() => {
      // Override name prompt
      const origPrompt = window.prompt;
      window.prompt = (msg) => {
        if (msg && msg.toLowerCase().includes('name')) return 'Nora';
        return origPrompt(msg);
      };
      // Disable WebRTC noise suppression, echo cancellation, and AGC so Nora's TTS voice
      // is not classified as noise and filtered before transmission.
      const origGUM = navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);
      navigator.mediaDevices.getUserMedia = function(constraints) {
        if (constraints && constraints.audio && typeof constraints.audio === 'object') {
          constraints.audio = {
            ...constraints.audio,
            echoCancellation: false,
            noiseSuppression: false,
            autoGainControl: false,
          };
        } else if (constraints && constraints.audio === true) {
          constraints.audio = { echoCancellation: false, noiseSuppression: false, autoGainControl: false };
        }
        return origGUM(constraints);
      };
    });

    // Join the meeting
    const joined = await joinMeet(page);

    if (joined) {
      // Unmute microphone so Nora's TTS voice transmits into the meeting
      try {
        const unmuteSelectors = [
          '[aria-label*="Turn on microphone"]',
          '[aria-label*="Unmute microphone"]',
          'button[aria-label*="mic" i][aria-pressed="false"]',
          '[data-is-muted="true"][aria-label*="mic" i]',
        ];
        let unmuted = false;
        for (const sel of unmuteSelectors) {
          try {
            const btn = page.locator(sel).first();
            if (await btn.isVisible({ timeout: 2000 })) {
              await btn.click();
              log('Unmuted microphone for TTS output');
              unmuted = true;
              break;
            }
          } catch (_) {}
        }
        if (!unmuted) log('Could not find unmute button — mic may already be on or selector changed');
      } catch (_) {}

      // Start audio capture from meeting
      startCapture();

      // Start polling for TTS responses
      pollForTTS();

      // Keep alive — monitor page state
      log('Nora is active in the meeting. Monitoring...');

      // Continuously dismiss any modal dialogs that appear during the meeting
      // (e.g. AI note taker consent, recording notice, captions prompt, etc.)
      const dialogDismissSelectors = [
        'button:has-text("Got it")',
        'button:has-text("OK")',
        'button:has-text("Accept")',
        'button:has-text("Continue")',
        'button:has-text("Dismiss")',
        'button:has-text("I understand")',
        '[aria-label="Dismiss"]',
      ];
      const dialogPoller = setInterval(async () => {
        try {
          for (const sel of dialogDismissSelectors) {
            const btn = page.locator(sel).first();
            if (await btn.isVisible({ timeout: 300 }).catch(() => false)) {
              await btn.click().catch(() => {});
              log('Dismissed in-meeting dialog', { selector: sel });
              break;
            }
          }
        } catch (_) {}
      }, 2000);

      // Monitor meeting state — only exit if page is fully gone (browser closed etc)
      const keepAlive = setInterval(async () => {
        try {
          const url = page.url();
          // Google can briefly navigate away — only bail if clearly kicked out
          if (url.includes('workspace.google.com/products/meet') ||
              url.includes('google.com/sorry') ||
              url === 'about:blank') {
            log('Ejected from meeting', { url });
            clearInterval(keepAlive);
            clearInterval(dialogPoller);
            await cleanup(browser);
          }
        } catch (err) {
          error('Browser/page error in keepAlive — Chrome may have crashed', err instanceof Error ? err : new Error(String(err)));
          clearInterval(keepAlive);
          clearInterval(dialogPoller);
          await cleanup(browser);
        }
      }, 10000);

      // Handle process signals
      process.on('SIGTERM', async () => {
        log('Received SIGTERM — leaving meeting');
        clearInterval(keepAlive);
        await cleanup(browser);
      });

      process.on('SIGINT', async () => {
        clearInterval(keepAlive);
        await cleanup(browser);
      });

    } else {
      await cleanup(browser);
    }

  } catch (e) {
    error('Browser error', e);
    if (browser) await browser.close().catch(() => {});
    teardownAudio();
    process.exit(1);
  }
}

async function cleanup(browser) {
  emit({ type: 'leaving', message: 'Nora is leaving the meeting' });
  pollingActive = false;
  stopCapture();

  // Cleanup temp dirs
  try { fs.rmSync(CHUNK_DIR, { recursive: true, force: true }); } catch (_) {}
  try { fs.rmSync(TTS_DIR,   { recursive: true, force: true }); } catch (_) {}

  if (browser) await browser.close().catch(() => {});

  teardownAudio();

  // Notify server
  try {
    await post(`${SERVER}/api/nora/meet/${sessionId}/left`, {});
  } catch (_) {}

  emit({ type: 'done', message: 'Nora has left the meeting' });
  process.exit(0);
}

main();
