const { chromium } = require('playwright');
const os = require('os'), path = require('path');

(async () => {
  const browser = await chromium.launchPersistentContext(
    path.join(os.homedir(), 'nora-chrome-profile'),
    { headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage'] }
  );
  const page = await browser.newPage();

  await page.goto('https://accounts.google.com', { waitUntil: 'domcontentloaded', timeout: 15000 });
  const url = page.url();
  const signedIn = url.indexOf('signin') === -1 && url.indexOf('ServiceLogin') === -1;
  console.log('Signed in:', signedIn);
  console.log('URL:', url.slice(0, 100));

  if (!signedIn) {
    console.log('Not signed in yet.');
    await browser.close();
    return;
  }

  console.log('Creating Google Meet...');
  await page.goto('https://meet.google.com/new', { waitUntil: 'commit', timeout: 20000 });
  await page.waitForTimeout(5000);

  const meetUrl = page.url();
  console.log('MEET_URL=' + meetUrl);
  await browser.close();
})().catch(e => { console.error('Error:', e.message); process.exit(1); });
