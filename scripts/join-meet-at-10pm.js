#!/usr/bin/env node
/**
 * Schedule Nora to join the Business Fusion Fashion Week Google Meet
 * at 10:00 PM EDT (21:00 CDT server time).
 *
 * Usage: node scripts/join-meet-at-10pm.js <meet_url>
 */

const { spawn } = require('child_process');
const path = require('path');

const meetUrl = process.argv[2];
if (!meetUrl) {
  console.error('Usage: node join-meet-at-10pm.js <meet_url>');
  process.exit(1);
}

const SERVER = `http://127.0.0.1:${process.env.SERVER_PORT || '3000'}`;
const BOT_SCRIPT = path.join(__dirname, 'meet-bot.js');

// 10:00 PM EDT = 21:00 CDT (server is in CDT/UTC-5)
const target = new Date();
target.setHours(21, 0, 0, 0); // 9:00 PM CDT = 10:00 PM EDT

const now = new Date();
let diffMs = target.getTime() - now.getTime();

if (diffMs < 0) {
  // Already past — try next natural run or join immediately if within last hour
  const minsAgo = Math.round(-diffMs / 60000);
  if (minsAgo < 60) {
    console.log(`[SCHEDULE] 10 PM EDT passed ${minsAgo} min ago — joining now`);
    diffMs = 0;
  } else {
    console.error('[SCHEDULE] Meeting time has passed by more than 1 hour. Exiting.');
    process.exit(0);
  }
}

const diffMin = Math.round(diffMs / 60000);
if (diffMs > 0) {
  const t = new Date(target);
  console.log(`[SCHEDULE] Will join at ${t.toLocaleTimeString()} CDT (${diffMin} min from now)`);
  console.log(`[SCHEDULE] Meet URL: ${meetUrl}`);
}

setTimeout(() => {
  console.log('[JOIN] Launching meet-bot.js...');
  const sessionId = require('crypto').randomUUID();

  const child = spawn('node', [BOT_SCRIPT, meetUrl, sessionId, SERVER], {
    detached: false,
    stdio: 'inherit',
    env: { ...process.env },
    cwd: path.join(__dirname, '..'),
  });

  child.on('error', err => {
    console.error('[JOIN] Error launching meet-bot:', err.message);
    process.exit(1);
  });

  child.on('exit', (code) => {
    console.log('[JOIN] meet-bot.js exited with code', code);
    process.exit(code || 0);
  });
}, diffMs);

// Keep alive with a status ticker
if (diffMs > 0) {
  const interval = setInterval(() => {
    const remaining = Math.round((target.getTime() - Date.now()) / 1000);
    if (remaining <= 0) {
      clearInterval(interval);
      return;
    }
    const m = Math.floor(remaining / 60);
    const s = remaining % 60;
    process.stdout.write(`\r[SCHEDULE] Joining in ${m}m ${s}s...  `);
  }, 1000);
}
