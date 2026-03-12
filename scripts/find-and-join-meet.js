/**
 * Find a Google Meet invitation in Gmail/Calendar and join it.
 * Checks Gmail for recent meeting invites, extracts the Meet link, then launches meet-bot.js.
 */

const { chromium } = require('playwright');
const { spawn } = require('child_process');
const os = require('os'), path = require('path');

const PROFILE_DIR = path.join(os.homedir(), 'nora-chrome-profile');
const SERVER = `http://127.0.0.1:${process.env.SERVER_PORT || '3000'}`;
const BOT_SCRIPT = path.join(__dirname, 'meet-bot.js');

async function findMeetLink(page) {
  // Strategy 1: check Google Calendar for upcoming events with Meet links
  console.log('[FIND] Checking Google Calendar...');
  try {
    await page.goto('https://calendar.google.com', { waitUntil: 'domcontentloaded', timeout: 20000 });
    await page.waitForTimeout(3000);

    const content = await page.content();
    const meetMatches = [...content.matchAll(/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/gi)];
    if (meetMatches.length > 0) {
      const unique = [...new Set(meetMatches.map(m => `https://meet.google.com/${m[1]}`))];
      console.log('[FIND] Found meet links in Calendar:', unique);
      return unique[0];
    }
    console.log('[FIND] No meet links found in Calendar page');
  } catch (e) {
    console.warn('[FIND] Calendar check failed:', e.message);
  }

  // Strategy 2: check Gmail for recent meeting invitations
  console.log('[FIND] Checking Gmail for meeting invites...');
  try {
    await page.goto('https://mail.google.com/mail/u/0/#search/from%3Asirak+OR+subject%3Afashion+week+OR+subject%3Ameeting+invite', {
      waitUntil: 'domcontentloaded',
      timeout: 20000,
    });
    await page.waitForTimeout(4000);

    const content = await page.content();
    const meetMatches = [...content.matchAll(/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/gi)];
    if (meetMatches.length > 0) {
      const unique = [...new Set(meetMatches.map(m => `https://meet.google.com/${m[1]}`))];
      console.log('[FIND] Found meet links in Gmail search:', unique);
      return unique[0];
    }

    // Try opening the first email result
    const firstEmail = await page.$('[role="row"]');
    if (firstEmail) {
      await firstEmail.click();
      await page.waitForTimeout(3000);
      const emailContent = await page.content();
      const emailMatches = [...emailContent.matchAll(/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/gi)];
      if (emailMatches.length > 0) {
        const link = `https://meet.google.com/${emailMatches[0][1]}`;
        console.log('[FIND] Found meet link in email:', link);
        return link;
      }
    }
    console.log('[FIND] No meet links found in Gmail');
  } catch (e) {
    console.warn('[FIND] Gmail check failed:', e.message);
  }

  // Strategy 3: check Google Calendar month view directly for today's events
  console.log('[FIND] Checking Calendar for today\'s events...');
  try {
    await page.goto('https://calendar.google.com/calendar/r/day', {
      waitUntil: 'domcontentloaded',
      timeout: 20000,
    });
    await page.waitForTimeout(3000);

    // Click on any event to see details
    const events = await page.$$('[data-eventid], [data-eventchip]');
    for (const evt of events) {
      await evt.click();
      await page.waitForTimeout(1500);
      const popup = await page.content();
      const m = popup.match(/meet\.google\.com\/([a-z]{3}-[a-z]{4}-[a-z]{3})/i);
      if (m) {
        const link = `https://meet.google.com/${m[1]}`;
        console.log('[FIND] Found meet link in calendar event:', link);
        return link;
      }
    }
  } catch (e) {
    console.warn('[FIND] Calendar day view check failed:', e.message);
  }

  return null;
}

async function main() {
  console.log('[START] Looking for meeting invite from Sirak...');

  // Close any headed browser holding the profile
  const { execSync } = require('child_process');
  try { execSync('pkill -f "nora-chrome-profile"'); } catch (_) {}
  await new Promise(r => setTimeout(r, 2000));

  const browser = await chromium.launchPersistentContext(PROFILE_DIR, {
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage', '--disable-blink-features=AutomationControlled'],
  });

  const page = await browser.newPage();

  // Verify signed in
  await page.goto('https://accounts.google.com', { waitUntil: 'domcontentloaded', timeout: 15000 });
  const url = page.url();
  const signedIn = url.indexOf('signin') === -1 && url.indexOf('ServiceLogin') === -1;
  console.log('[AUTH] Signed in:', signedIn, '| URL:', url.slice(0, 80));

  if (!signedIn) {
    console.error('[AUTH] Not signed in to Google. Please sign in via the headed browser first.');
    await browser.close();
    process.exit(1);
  }

  // Find the meet link
  const meetUrl = await findMeetLink(page);
  await browser.close();

  if (!meetUrl) {
    console.error('[FIND] Could not find a Google Meet link in Gmail or Calendar.');
    console.error('[FIND] Please share the meet link manually.');
    process.exit(1);
  }

  console.log('\n[JOIN] Meet URL found:', meetUrl);
  console.log('[JOIN] Launching Nora into the meeting...');

  const sessionId = require('crypto').randomUUID();
  const child = spawn('node', [BOT_SCRIPT, meetUrl, sessionId, SERVER], {
    detached: false,
    stdio: 'inherit',
    env: { ...process.env },
    cwd: path.join(__dirname, '..'),
  });

  child.on('error', err => {
    console.error('[JOIN] meet-bot error:', err.message);
    process.exit(1);
  });

  child.on('exit', code => {
    console.log('[JOIN] meet-bot exited with code', code);
    process.exit(code || 0);
  });

  console.log('[JOIN] Nora is joining... session:', sessionId);
}

main().catch(e => { console.error('Fatal:', e); process.exit(1); });
