/**
 * meet-watcher.js — Auto-join Google Meet invitations received in Nora's inbox.
 *
 * Polls Nora's Zoho inbox via the Zoho Mail API every 60 seconds.
 * When an email containing a Google Meet link is found that hasn't been
 * joined yet, it automatically launches meet-bot.js to join the call.
 *
 * Requires: ZOHO_ACCESS_TOKEN or ZOHO_REFRESH_TOKEN + ZOHO_CLIENT_ID/SECRET in env.
 * Falls back to SMTP-password login via webmail scraping if tokens unavailable.
 */

import { spawn } from 'child_process';
import { createRequire } from 'module';
import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs';
import https from 'https';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const BOT_SCRIPT = path.join(__dirname, '../../scripts/meet-bot.js');
const SERVER = `http://127.0.0.1:${process.env.SERVER_PORT || '3000'}`;
const JOINED_FILE = '/tmp/nora-joined-meets.json';

const POLL_INTERVAL_MS = 30_000; // check every 30s

// Track which meet URLs we've already joined this session
const joinedUrls = new Set(loadJoined());

function loadJoined() {
  try { return JSON.parse(fs.readFileSync(JOINED_FILE, 'utf8')); } catch (_) { return []; }
}
function saveJoined() {
  fs.writeFileSync(JOINED_FILE, JSON.stringify([...joinedUrls]), 'utf8');
}

// ─── Zoho Mail API ────────────────────────────────────────────────────────────

async function getZohoAccessToken() {
  const refreshToken = process.env.ZOHO_REFRESH_TOKEN;
  if (!refreshToken) return process.env.ZOHO_ACCESS_TOKEN || null;

  return new Promise((resolve, reject) => {
    const params = new URLSearchParams({
      refresh_token: refreshToken,
      client_id: process.env.ZOHO_CLIENT_ID,
      client_secret: process.env.ZOHO_CLIENT_SECRET,
      grant_type: 'refresh_token',
    }).toString();

    const req = https.request({
      hostname: 'accounts.zoho.com',
      path: '/oauth/v2/token',
      method: 'POST',
      headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Content-Length': params.length },
    }, res => {
      let data = '';
      res.on('data', d => data += d);
      res.on('end', () => {
        try {
          const json = JSON.parse(data);
          resolve(json.access_token || null);
        } catch (_) { resolve(null); }
      });
    });
    req.on('error', () => resolve(null));
    req.write(params);
    req.end();
  });
}

async function fetchInboxViaApi(accessToken) {
  return new Promise((resolve, reject) => {
    const req = https.request({
      hostname: 'mail.zoho.com',
      path: '/api/accounts/self/messages/view?folderId=inbox&limit=20&sortBy=date&sortOrder=desc',
      headers: { Authorization: `Zoho-oauthtoken ${accessToken}` },
    }, res => {
      let data = '';
      res.on('data', d => data += d);
      res.on('end', () => {
        try { resolve(JSON.parse(data)); } catch (_) { resolve(null); }
      });
    });
    req.on('error', () => resolve(null));
    req.end();
  });
}

// ─── Playwright fallback ──────────────────────────────────────────────────────

async function fetchInboxViaWebmail() {
  const { chromium } = createRequire(import.meta.url)('playwright');
  const profileDir = path.join(process.env.HOME, 'nora-zoho-profile');

  const browser = await chromium.launchPersistentContext(profileDir, {
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });

  const page = await browser.newPage();
  try {
    await page.goto('https://mail.zoho.com/zm/#mail/folder/inbox', {
      waitUntil: 'domcontentloaded', timeout: 20000,
    });
    await page.waitForTimeout(4000);
    const content = await page.content();
    await browser.close();
    return content;
  } catch (e) {
    try { await browser.close(); } catch (_) {}
    return '';
  }
}

// ─── Meet link extraction ─────────────────────────────────────────────────────

function extractMeetLinks(text) {
  const matches = [...text.matchAll(/https?:\/\/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/gi)];
  return [...new Set(matches.map(m => `https://meet.google.com/${m[1]}`))];
}

// ─── Join a meet ──────────────────────────────────────────────────────────────

function joinMeet(meetUrl) {
  if (joinedUrls.has(meetUrl)) {
    console.log('[WATCHER] Already joined (or attempted):', meetUrl);
    return;
  }

  joinedUrls.add(meetUrl);
  saveJoined();

  console.log('[WATCHER] AUTO-JOINING meet:', meetUrl);
  const sessionId = Math.random().toString(36).slice(2);

  const child = spawn('node', [BOT_SCRIPT, meetUrl, sessionId, SERVER], {
    detached: true,
    stdio: 'inherit',
    env: { ...process.env },
    cwd: path.join(__dirname, '../../'),
  });

  child.on('error', err => console.error('[WATCHER] meet-bot error:', err.message));
  child.unref();
  console.log('[WATCHER] meet-bot launched, session:', sessionId);
}

// ─── Poll loop ────────────────────────────────────────────────────────────────

async function poll() {
  console.log('[WATCHER] Checking inbox for meet invites...');

  let meetLinks = [];

  // Try Zoho API first
  const token = await getZohoAccessToken();
  if (token) {
    const data = await fetchInboxViaApi(token);
    if (data?.data) {
      const combined = data.data.map(m => (m.subject || '') + ' ' + (m.content || '')).join('\n');
      meetLinks = extractMeetLinks(combined);
    }
  }

  // Fallback to webmail
  if (meetLinks.length === 0) {
    const html = await fetchInboxViaWebmail();
    meetLinks = extractMeetLinks(html);
  }

  if (meetLinks.length > 0) {
    console.log('[WATCHER] Found meet links:', meetLinks);
    for (const url of meetLinks) {
      joinMeet(url);
    }
  } else {
    console.log('[WATCHER] No meet links found this cycle.');
  }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

console.log('[WATCHER] Nora meet-watcher started. Polling every', POLL_INTERVAL_MS / 1000, 's');
poll(); // run immediately
setInterval(poll, POLL_INTERVAL_MS);
