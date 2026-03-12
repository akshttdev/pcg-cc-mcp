/**
 * Check Nora's Zoho inbox for Google Meet links and join the first one found.
 * Run: node scripts/check-nora-email.js
 */

const tls = require('tls');
const { spawn } = require('child_process');
const path = require('path');

if (!process.env.SMTP_USERNAME || !process.env.SMTP_PASSWORD) {
  console.error('Fatal: SMTP_USERNAME and SMTP_PASSWORD env vars are required.');
  process.exit(1);
}

const IMAP = {
  host: 'imap.zoho.com',
  port: 993,
  user: process.env.SMTP_USERNAME,
  pass: process.env.SMTP_PASSWORD,
};

const SERVER  = `http://127.0.0.1:${process.env.SERVER_PORT || '3000'}`;
const BOT     = path.join(__dirname, 'meet-bot.js');

// ─── Minimal IMAP client ───────────────────────────────────────────────────

function imapConnect(host, port, user, pass) {
  return new Promise((resolve, reject) => {
    const socket = tls.connect(port, host, { rejectUnauthorized: false });
    let buf = '';
    let tag = 1;
    const pending = new Map();

    socket.on('error', reject);

    function send(cmd, callback) {
      const t = `A${String(tag++).padStart(3, '0')}`;
      pending.set(t, { buf: '', cb: callback });
      socket.write(`${t} ${cmd}\r\n`);
    }

    function sendLiteral(cmd, literal, callback) {
      const t = `A${String(tag++).padStart(3, '0')}`;
      pending.set(t, { buf: '', cb: callback, literalCb: callback });
      socket.write(`${t} ${cmd} {${Buffer.byteLength(literal)}}\r\n`);
      // Wait for continuation then send literal
      const origHandler = socket.listeners('data')[0];
      socket.once('data', () => {
        socket.write(literal + '\r\n');
      });
    }

    socket.on('data', d => {
      buf += d.toString();
      const lines = buf.split('\r\n');
      buf = lines.pop();

      for (const line of lines) {
        // Check for tagged responses
        for (const [t, entry] of pending.entries()) {
          if (line.startsWith(t + ' ')) {
            entry.buf += line + '\n';
            const cb = entry.cb;
            pending.delete(t);
            cb(null, entry.buf);
            break;
          } else {
            entry.buf += line + '\n';
          }
        }
      }
    });

    socket.once('data', greeting => {
      // Send LOGIN
      send(`LOGIN "${user}" "${pass}"`, (err, resp) => {
        if (err || resp.includes('NO') || resp.includes('BAD')) {
          reject(new Error('Login failed: ' + resp));
          return;
        }
        resolve({ send, socket });
      });
    });
  });
}

async function fetchRecentEmails() {
  console.log('[IMAP] Connecting to', IMAP.host, '...');
  const { send, socket } = await imapConnect(IMAP.host, IMAP.port, IMAP.user, IMAP.pass);

  return new Promise((resolve, reject) => {
    // Select INBOX
    send('SELECT INBOX', (err, resp) => {
      if (err) return reject(err);
      console.log('[IMAP] INBOX selected');

      // Search for recent unseen emails (last 7 days)
      send('SEARCH UNSEEN', (err2, resp2) => {
        if (err2) return reject(err2);
        const uids = resp2.match(/\* SEARCH([\d ]*)/)?.[1]?.trim().split(' ').filter(Boolean) || [];
        console.log('[IMAP] Unread UIDs:', uids.length > 0 ? uids.join(', ') : '(none)');

        if (uids.length === 0) {
          // Try ALL recent instead
          send('SEARCH ALL', (err3, resp3) => {
            const allUids = resp3.match(/\* SEARCH([\d ]*)/)?.[1]?.trim().split(' ').filter(Boolean) || [];
            const recent = allUids.slice(-20); // last 20
            console.log('[IMAP] Recent UIDs:', recent.join(', '));
            fetchEmails(send, socket, recent, resolve, reject);
          });
        } else {
          fetchEmails(send, socket, uids, resolve, reject);
        }
      });
    });
  });
}

function fetchEmails(send, socket, uids, resolve, reject) {
  if (uids.length === 0) { socket.destroy(); return resolve([]); }

  const uidList = uids.join(',');
  send(`FETCH ${uidList} (BODY[TEXT] BODY[HEADER.FIELDS (FROM SUBJECT DATE)])`, (err, resp) => {
    socket.write('A999 LOGOUT\r\n');
    socket.destroy();
    if (err) return reject(err);
    resolve(resp);
  });
}

function extractMeetLinks(text) {
  const matches = [...text.matchAll(/https?:\/\/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/gi)];
  return [...new Set(matches.map(m => `https://meet.google.com/${m[1]}`))];
}

async function joinMeet(meetUrl) {
  console.log('\n[JOIN] Launching Nora into:', meetUrl);
  const sessionId = require('crypto').randomUUID();
  return new Promise((resolve, reject) => {
    const child = spawn('node', [BOT, meetUrl, sessionId, SERVER], {
      detached: false,
      stdio: 'inherit',
      env: { ...process.env },
      cwd: path.join(__dirname, '..'),
    });
    child.on('error', reject);
    child.on('exit', code => { console.log('[JOIN] Exited:', code); resolve(code); });
  });
}

async function main() {
  console.log('[START] Checking nora@powerclubglobal.com for meeting invites...');

  let rawData;
  try {
    rawData = await fetchRecentEmails();
  } catch (e) {
    console.error('[IMAP] Error:', e.message);
    process.exit(1);
  }

  const links = extractMeetLinks(rawData);

  if (links.length === 0) {
    console.log('[IMAP] No Google Meet links found in recent emails.');
    console.log('[IMAP] Raw snippet:', rawData.slice(0, 500));
    process.exit(1);
  }

  console.log('[IMAP] Found meet links:', links);

  // Kill any existing scheduler
  try { require('child_process').execSync('kill $(pgrep -f join-meet-at-10pm) 2>/dev/null'); } catch (_) {}

  // Join the first/most relevant link
  await joinMeet(links[0]);
}

main().catch(e => { console.error('Fatal:', e.message); process.exit(1); });
