const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

// Cards to capture: [card_label_substring, output_filename]
const CARDS = [
  ['A — Thumbnail',         'thumb_A.png'],
  ['B — Show Open Bug',     'bug_B.png'],
  ['C — Host ID',           'hostid_C.png'],
  ['D — Animated Logo',     'logointro_D.png'],
  ['E — AI B-Roll',         'broll_ai_E.png'],
  ['F — Crypto B-Roll',     'broll_crypto_F.png'],
  ['G — PCG B-Roll',        'broll_pcg_G.png'],
  ['H — Sign-Off',          'signoff_H.png'],
  ['I — Outro',             'outro_I.png'],
  ['01 — B-Roll Reporter',  'broll_reporter_01.png'],
  ['02 — On-Camera Person', 'oncam_id_02.png'],
  ['08 — Breaking Alert',   'breaking_08.png'],
  ['09 — Stat Callout',     'stat_callout_09.png'],
  ['10 — Key Points',       'keypoints_10.png'],
  ['12 — Pull Quote',       'pullquote_12.png'],
  ['15 — Ticker Strip',     'ticker_15.png'],
  ['16 — Persistent Water', 'watermark_16.png'],
  ['18 — PCG Positioning',  'pcg_pos_18.png'],
  ['19 — CTA Overlay',      'cta_19.png'],
];

const OUT_DIR = '/tmp/pcg_html_overlays';
fs.mkdirSync(OUT_DIR, { recursive: true });

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  
  // Set viewport for 9:16 at 2x scale (720x1280 @ 2x = 1440x2560 device pixel)
  await page.setViewportSize({ width: 1440, height: 2560 });
  
  await page.goto('http://localhost:8765/vertical-preview.html', { 
    waitUntil: 'networkidle',
    timeout: 10000 
  });
  
  // Wait for fonts to load
  await page.waitForTimeout(2000);

  let captured = 0;
  for (const [label, filename] of CARDS) {
    // Find the card by its label text
    const cardLabel = page.locator('.card-label').filter({ hasText: label }).first();
    const count = await cardLabel.count();
    if (count === 0) {
      console.log(`NOT FOUND: ${label}`);
      continue;
    }
    
    // Get the parent .card, then the .canvas inside it
    const canvas = cardLabel.locator('..').locator('.canvas').first();
    const canvasCount = await canvas.count();
    if (canvasCount === 0) {
      console.log(`NO CANVAS for: ${label}`);
      continue;
    }
    
    const outPath = path.join(OUT_DIR, filename);
    await canvas.screenshot({ 
      path: outPath,
      type: 'png',
      scale: 'device'  // use device pixels (2x = 720px wide canvas)
    });
    
    const stats = fs.statSync(outPath);
    console.log(`✓ ${filename} (${Math.round(stats.size/1024)}KB)`);
    captured++;
  }
  
  await browser.close();
  console.log(`\nDone: ${captured}/${CARDS.length} cards captured → ${OUT_DIR}`);
})().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
