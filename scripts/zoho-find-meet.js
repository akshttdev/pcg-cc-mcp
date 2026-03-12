/**
 * Directly query Zoho Mail API with stored tokens to find Google Meet links.
 */
const https = require('https');
const { spawn, execSync } = require('child_process');
const path = require('path');

const DB = process.env.DB_PATH || '/home/pythia/pcg-cc-mcp/dev_assets/db.sqlite';
const SERVER = `http://127.0.0.1:${process.env.SERVER_PORT || '3000'}`;
const BOT = path.join(__dirname, 'meet-bot.js');

function dbQuery(sql) {
  const result = execSync(`sqlite3 "${DB}" "${sql}"`).toString().trim();
  return result;
}

function httpsGet(url, token) {
  return new Promise((resolve, reject) => {
    const u = new URL(url);
    https.get({ hostname: u.hostname, path: u.pathname + u.search,
      headers: { Authorization: `Zoho-oauthtoken ${token}` }
    }, res => {
      let d = '';
      res.on('data', c => d += c);
      res.on('end', () => { try { resolve(JSON.parse(d)); } catch (_) { resolve(d); } });
    }).on('error', reject);
  });
}

function httpsPost(url, params) {
  return new Promise((resolve, reject) => {
    const u = new URL(url);
    const payload = params;
    const req = https.request({ hostname: u.hostname, path: u.pathname,
      method: 'POST', headers: { 'Content-Type': 'application/x-www-form-urlencoded', 'Content-Length': payload.length }
    }, res => {
      let d = '';
      res.on('data', c => d += c);
      res.on('end', () => { try { resolve(JSON.parse(d)); } catch (_) { resolve(d); } });
    });
    req.on('error', reject);
    req.write(payload);
    req.end();
  });
}

async function getValidToken() {
  // Get stored tokens for nora@powerclubglobal.com
  const row = dbQuery(`SELECT access_token, refresh_token, token_expires_at FROM email_accounts WHERE email_address='nora@powerclubglobal.com' AND owner_type='agent' ORDER BY updated_at DESC LIMIT 1`);
  if (!row) throw new Error('No email account found for Nora');

  const [accessToken, refreshToken, expiresAt] = row.split('|');
  const expired = expiresAt && new Date(expiresAt) < new Date();

  if (!expired && accessToken) {
    console.log('[TOKEN] Using stored access token (expires:', expiresAt, ')');
    return accessToken;
  }

  console.log('[TOKEN] Token expired, refreshing...');
  const clientId = process.env.ZOHO_CLIENT_ID;
  const clientSecret = process.env.ZOHO_CLIENT_SECRET;
  const params = `refresh_token=${refreshToken}&client_id=${clientId}&client_secret=${clientSecret}&grant_type=refresh_token`;

  const resp = await httpsPost('https://accounts.zoho.com/oauth/v2/token', params);
  if (!resp.access_token) throw new Error('Token refresh failed: ' + JSON.stringify(resp));

  // Update DB
  const newExpiry = new Date(Date.now() + (resp.expires_in || 3600) * 1000).toISOString();
  dbQuery(`UPDATE email_accounts SET access_token='${resp.access_token}', token_expires_at='${newExpiry}', updated_at=datetime('now','subsec') WHERE email_address='nora@powerclubglobal.com' AND owner_type='agent'`);
  console.log('[TOKEN] Token refreshed successfully');
  return resp.access_token;
}

async function main() {
  console.log('[START] Checking Nora inbox for meet invite from Sirak...');

  const token = await getValidToken();

  // Get Zoho account ID
  const meta = dbQuery(`SELECT metadata FROM email_accounts WHERE email_address='nora@powerclubglobal.com' AND owner_type='agent' ORDER BY updated_at DESC LIMIT 1`);
  const metadata = JSON.parse(meta || '{}');
  const accountId = metadata.zoho_account_id || '1462441000000008002';
  const domain = metadata.zoho_domain || 'com';

  console.log('[ZOHO] Account ID:', accountId);

  // Search inbox for recent emails
  const inbox = await httpsGet(
    `https://mail.zoho.${domain}/api/accounts/${accountId}/messages/view?folderId=inbox&limit=30&sortBy=date&sortOrder=desc`,
    token
  );

  if (!inbox?.data) {
    console.error('[ZOHO] Unexpected response:', JSON.stringify(inbox).slice(0, 300));
    process.exit(1);
  }

  console.log('[ZOHO] Found', inbox.data.length, 'messages in inbox');

  // Look for meet links in subjects/senders
  let meetUrl = null;
  for (const msg of inbox.data) {
    const from = (msg.fromAddress || '').toLowerCase();
    const subject = (msg.subject || '').toLowerCase();
    const summary = msg.summary || '';
    const combined = from + ' ' + subject + ' ' + summary;

    console.log(`  [MSG] From: ${msg.fromAddress} | Subject: ${msg.subject}`);

    const m = combined.match(/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/i);
    if (m) {
      meetUrl = `https://meet.google.com/${m[1]}`;
      console.log('[FOUND] Meet link in message summary:', meetUrl);
      break;
    }

    // If from Sirak or about fashion week, fetch full message
    if (from.includes('sirak') || subject.includes('fashion') || subject.includes('fusion') || subject.includes('invite') || subject.includes('meet')) {
      console.log('[FETCH] Fetching full message body...');
      const full = await httpsGet(
        `https://mail.zoho.${domain}/api/accounts/${accountId}/messages/${msg.messageId}`,
        token
      );
      const body = full?.data?.content || full?.data?.htmlContent || JSON.stringify(full);
      const m2 = body.match(/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/i);
      if (m2) {
        meetUrl = `https://meet.google.com/${m2[1]}`;
        console.log('[FOUND] Meet link in full message:', meetUrl);
        break;
      }
    }
  }

  if (!meetUrl) {
    console.log('[RESULT] No Google Meet link found in recent inbox messages.');
    console.log('[RESULT] Please paste the meet URL manually.');
    process.exit(1);
  }

  console.log('\n[JOIN] Found meet URL:', meetUrl);
  console.log('[JOIN] Launching Nora into the meeting NOW...');

  // Kill old scheduler
  try { execSync('pkill -f join-meet-at-10pm 2>/dev/null'); } catch (_) {}

  const sessionId = require('crypto').randomUUID();
  const child = spawn('node', [BOT, meetUrl, sessionId, SERVER], {
    detached: false,
    stdio: 'inherit',
    env: { ...process.env },
    cwd: path.join(__dirname, '..'),
  });
  child.on('error', e => { console.error('[JOIN] Error:', e.message); process.exit(1); });
  child.on('exit', code => { console.log('[JOIN] meet-bot exited:', code); process.exit(code || 0); });
}

main().catch(e => { console.error('Fatal:', e.message); process.exit(1); });
