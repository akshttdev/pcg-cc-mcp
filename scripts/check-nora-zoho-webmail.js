/**
 * Check Nora's Zoho webmail for Google Meet invite, extract link, join.
 */

const { chromium } = require('playwright');
const { spawn, execSync } = require('child_process');
const path = require('path');
const os = require('os');

const PROFILE_DIR = path.join(os.homedir(), 'nora-zoho-profile');
const SERVER = `http://127.0.0.1:${process.env.SERVER_PORT || '3000'}`;
const BOT = path.join(__dirname, 'meet-bot.js');

const ZOHO_USER = process.env.SMTP_USERNAME || 'nora@powerclubglobal.com';
const ZOHO_PASS = process.env.SMTP_PASSWORD || 'caHZ9rneFix8';

async function findMeetLinkInZoho() {
  // Kill any existing browser on this profile
  try { execSync(`pkill -f "${PROFILE_DIR.replace(/\//g, '.')}"`); } catch (_) {}
  await new Promise(r => setTimeout(r, 1000));

  console.log('[ZOHO] Launching browser...');
  const browser = await chromium.launchPersistentContext(PROFILE_DIR, {
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });

  const page = await browser.newPage();

  try {
    // Navigate to Zoho Mail
    console.log('[ZOHO] Opening Zoho Mail...');
    await page.goto('https://mail.zoho.com', { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(3000);

    const url = page.url();
    console.log('[ZOHO] URL:', url.slice(0, 80));

    // Check if we need to log in
    const needsLogin = url.includes('login') || url.includes('signin') || url.includes('accounts.zoho');
    if (needsLogin) {
      console.log('[ZOHO] Need to log in...');

      // Enter email
      const emailInput = await page.$('input[type="email"], input[name="email"], #login_id, input[placeholder*="mail"]');
      if (emailInput) {
        await emailInput.fill(ZOHO_USER);
        await page.keyboard.press('Enter');
        await page.waitForTimeout(2000);
      }

      // Enter password
      const passInput = await page.$('input[type="password"], #password');
      if (passInput) {
        await passInput.fill(ZOHO_PASS);
        await page.keyboard.press('Enter');
        await page.waitForTimeout(4000);
      }

      const newUrl = page.url();
      console.log('[ZOHO] After login:', newUrl.slice(0, 80));
    }

    // Search for the meet invite
    console.log('[ZOHO] Searching inbox for meet invite...');
    await page.waitForTimeout(2000);

    // Try search
    const searchBox = await page.$('input[type="search"], [placeholder*="search"], [aria-label*="search"]');
    if (searchBox) {
      await searchBox.fill('meet.google.com OR "Business Fusion" OR "Fashion Week"');
      await page.keyboard.press('Enter');
      await page.waitForTimeout(3000);
    }

    // Scan page content for meet links
    let content = await page.content();
    let links = extractMeetLinks(content);

    if (links.length > 0) {
      console.log('[ZOHO] Found meet links:', links);
      await browser.close();
      return links[0];
    }

    // Click on first email if any
    const emails = await page.$$('[class*="mail-row"], [class*="message-row"], [role="row"]');
    console.log('[ZOHO] Emails visible:', emails.length);
    for (const email of emails.slice(0, 10)) {
      try {
        await email.click();
        await page.waitForTimeout(2000);
        content = await page.content();
        links = extractMeetLinks(content);
        if (links.length > 0) {
          console.log('[ZOHO] Found meet link in email:', links[0]);
          await browser.close();
          return links[0];
        }
      } catch (_) {}
    }

    // Dump a snippet for debugging
    console.log('[ZOHO] No meet links found. Page snippet:');
    console.log(content.slice(0, 600));

    await browser.close();
    return null;

  } catch (e) {
    console.error('[ZOHO] Error:', e.message);
    try { await browser.close(); } catch (_) {}
    return null;
  }
}

function extractMeetLinks(text) {
  const matches = [...text.matchAll(/https?:\/\/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/gi)];
  return [...new Set(matches.map(m => `https://meet.google.com/${m[1]}`))];
}

async function joinMeet(meetUrl) {
  console.log('\n[JOIN] Nora joining:', meetUrl);
  const sessionId = require('crypto').randomUUID();

  // Kill old scheduler
  try { execSync('pkill -f join-meet-at-10pm'); } catch (_) {}

  const child = spawn('node', [BOT, meetUrl, sessionId, SERVER], {
    detached: false,
    stdio: 'inherit',
    env: { ...process.env },
    cwd: path.join(__dirname, '..'),
  });

  child.on('error', err => { console.error('[JOIN] Error:', err.message); process.exit(1); });
  child.on('exit', code => { console.log('[JOIN] Exited:', code); process.exit(code || 0); });
  console.log('[JOIN] Session:', sessionId);
}

async function main() {
  console.log('[START] Checking nora@powerclubglobal.com for meeting invite from Sirak...');

  const meetUrl = await findMeetLinkInZoho();
  if (!meetUrl) {
    console.error('[RESULT] Could not find a Google Meet link.');
    console.error('[RESULT] Please paste the meet link directly.');
    process.exit(1);
  }

  await joinMeet(meetUrl);
}

main().catch(e => { console.error('Fatal:', e.message); process.exit(1); });
