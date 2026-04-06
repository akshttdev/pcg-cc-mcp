const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');
const { execSync } = require('child_process');

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
  const page = await browser.newPage();
  // Load at 4x the card width (327*4=1308 → scale to 720)
  await page.setViewportSize({ width: 3000, height: 6000 });
  
  await page.goto('http://localhost:8765/vertical-preview.html', { 
    waitUntil: 'networkidle', timeout: 15000 
  });
  await page.waitForTimeout(2000);
  
  // Inject style to remove canvas backgrounds & grid
  await page.addStyleTag({ content: `
    .canvas { background: transparent !important; border: none !important; }
    .canvas::before { display: none !important; }
    body { background: transparent !important; }
  `});

  let captured = 0;
  for (const [label, filename] of CARDS) {
    const cardLabel = page.locator('.card-label').filter({ hasText: label }).first();
    if (await cardLabel.count() === 0) { console.log(`SKIP: ${label}`); continue; }
    
    const canvas = cardLabel.locator('..').locator('.canvas').first();
    if (await canvas.count() === 0) { console.log(`NO CANVAS: ${label}`); continue; }
    
    const box = await canvas.boundingBox();
    if (!box || box.width < 1) { console.log(`EMPTY BOX: ${label}`); continue; }
    
    const tmpPath = path.join(OUT_DIR, `_tmp_${filename}`);
    const outPath = path.join(OUT_DIR, filename);
    
    await page.screenshot({
      path: tmpPath,
      clip: { x: Math.round(box.x), y: Math.round(box.y), 
               width: Math.round(box.width), height: Math.round(box.height) },
      omitBackground: true,
      type: 'png'
    });
    
    // Resize to 720x1280
    execSync(`ffmpeg -y -i "${tmpPath}" -vf "scale=720:1280:flags=lanczos" "${outPath}" 2>/dev/null`);
    fs.unlinkSync(tmpPath);
    
    const stats = fs.statSync(outPath);
    console.log(`✓ ${filename} src=${Math.round(box.width)}x${Math.round(box.height)} → 720x1280 (${Math.round(stats.size/1024)}KB)`);
    captured++;
  }
  
  await browser.close();
  console.log(`\nDone: ${captured}/${CARDS.length} → ${OUT_DIR}`);
})().catch(err => { console.error(err.message); process.exit(1); });
