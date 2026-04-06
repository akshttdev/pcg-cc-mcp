const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

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
  ['08 — Breaking Alert',   'breaking_08.png'],
  ['09 — Stat Callout',     'stat_callout_09.png'],
  ['10 — Key Points',       'keypoints_10.png'],
  ['12 — Pull Quote',       'pullquote_12.png'],
  ['15 — Ticker Strip',     'ticker_15.png'],
  ['18 — PCG Positioning',  'pcg_pos_18.png'],
  ['19 — CTA Overlay',      'cta_19.png'],
];

const OUT_DIR = '/tmp/pcg_html_overlays';
fs.mkdirSync(OUT_DIR, { recursive: true });

(async () => {
  const browser = await chromium.launch({ headless: true });
  
  let captured = 0;
  for (const [label, filename] of CARDS) {
    // Each card gets its own page at 720x1280 (1:1 pixel ratio)
    const page = await browser.newPage();
    await page.setViewportSize({ width: 720, height: 1280 });
    
    await page.goto('http://localhost:8765/vertical-preview.html', { 
      waitUntil: 'networkidle', timeout: 15000 
    });
    await page.waitForTimeout(1500);
    
    // Find the canvas for this card
    const cardLabel = page.locator('.card-label').filter({ hasText: label }).first();
    if (await cardLabel.count() === 0) {
      console.log(`NOT FOUND: ${label}`);
      await page.close();
      continue;
    }
    
    const canvas = cardLabel.locator('..').locator('.canvas').first();
    if (await canvas.count() === 0) {
      console.log(`NO CANVAS: ${label}`);
      await page.close();
      continue;
    }
    
    // Make canvas transparent + hide grid dots + scale to fill viewport
    await canvas.evaluate(el => {
      // Remove background (make transparent for overlay use)
      el.style.background = 'transparent';
      el.style.border = 'none';
      // Also make ::before pseudo (grid) invisible
      const style = document.createElement('style');
      style.textContent = `.canvas::before { display: none !important; }`;
      document.head.appendChild(style);
    });
    
    // Get the canvas bounding box and screenshot it at its natural size
    const box = await canvas.boundingBox();
    const outPath = path.join(OUT_DIR, filename);
    
    // Screenshot full page but clip to canvas element, with transparent bg
    await page.screenshot({
      path: outPath,
      clip: box,
      omitBackground: true,  // transparent background
      type: 'png'
    });
    
    const stats = fs.statSync(outPath);
    // Get actual dimensions
    console.log(`✓ ${filename} (${Math.round(stats.size/1024)}KB) clip=${Math.round(box.width)}x${Math.round(box.height)}`);
    captured++;
    await page.close();
  }
  
  await browser.close();
  console.log(`\nDone: ${captured}/${CARDS.length} → ${OUT_DIR}`);
})().catch(err => { console.error(err.message); process.exit(1); });
