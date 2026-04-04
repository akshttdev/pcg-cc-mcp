// v5: Section 6 (Sami Satoshi Ep.01) overlays only — transparent backgrounds
// A/D/I keep their dark bg (standalone frames). B/C/E/F/G/H get transparent canvas.
const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');
const { execSync } = require('child_process');

// Cards from Section 6 specifically
// standalone = true → keep canvas dark background (full-frame segments)
// standalone = false → transparent canvas (overlay-only, composited over video)
const CARDS = [
  { label: 'A — Thumbnail',     file: 'thumb_A.png',        standalone: true  },
  { label: 'B — Show Open Bug', file: 'bug_B.png',           standalone: false },
  { label: 'C — Host ID',       file: 'hostid_C.png',        standalone: false },
  { label: 'D — Animated Logo', file: 'logointro_D.png',     standalone: true  },
  { label: 'E — AI B-Roll',     file: 'broll_ai_E.png',      standalone: false },
  { label: 'F — Crypto B-Roll', file: 'broll_crypto_F.png',  standalone: false },
  { label: 'G — PCG B-Roll',    file: 'broll_pcg_G.png',     standalone: false },
  { label: 'H — Sign-Off',      file: 'signoff_H.png',       standalone: false },
  { label: 'I — Outro',         file: 'outro_I.png',         standalone: true  },
];

const OUT_DIR = '/tmp/pcg_ep01_overlays';
fs.mkdirSync(OUT_DIR, { recursive: true });

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  await page.setViewportSize({ width: 1600, height: 30000 });
  await page.goto('http://localhost:8765/vertical-preview.html', {
    waitUntil: 'networkidle', timeout: 15000
  });
  await page.waitForTimeout(2000);

  // Make body transparent so canvas alpha actually works
  await page.addStyleTag({ content: `
    html, body { background: transparent !important; }
    .canvas { background: transparent !important; border: none !important; }
    .canvas::before { display: none !important; }
  `});
  await page.waitForTimeout(600);

  let captured = 0;
  for (const card of CARDS) {
    const cardLabel = page.locator('.card-label').filter({ hasText: card.label }).first();
    if (await cardLabel.count() === 0) { console.log(`SKIP (not found): ${card.label}`); continue; }

    const canvas = cardLabel.locator('..').locator('.canvas').first();
    if (await canvas.count() === 0) { console.log(`NO CANVAS: ${card.label}`); continue; }

    if (!card.standalone) {
      // Make canvas transparent so only overlay UI elements are captured
      await canvas.evaluate(el => {
        el.style.background = 'transparent';
        el.style.border = 'none';
        // Hide pseudo-element (tech grid) via a style tag trick
        const style = document.createElement('style');
        style.textContent = '.canvas::before { display: none !important; }';
        document.head.appendChild(style);
      });
      // Also hide the broll-pip placeholder (we use real Sami PiP circles instead)
      const pipEl = canvas.locator('.broll-pip').first();
      if (await pipEl.count() > 0) {
        await pipEl.evaluate(el => el.style.display = 'none');
      }
    }

    await canvas.scrollIntoViewIfNeeded();
    await page.waitForTimeout(300);

    const tmpPath = path.join(OUT_DIR, card.file + '.tmp.png');
    const outPath = path.join(OUT_DIR, card.file);

    await canvas.screenshot({
      path: tmpPath,
      omitBackground: !card.standalone, // transparent for overlays
      type: 'png'
    });

    // Resize to exact 720x1280
    execSync(`ffmpeg -y -i "${tmpPath}" -vf "scale=720:1280:flags=lanczos" -pix_fmt rgba "${outPath}" 2>/dev/null`);
    fs.unlinkSync(tmpPath);

    const stats = fs.statSync(outPath);
    const type = card.standalone ? '[full-frame]' : '[transparent]';
    console.log(`✓ ${card.file} ${type} (${Math.round(stats.size/1024)}KB)`);
    captured++;

    // Restore canvas for subsequent captures if we modified it
    if (!card.standalone) {
      await canvas.evaluate(el => {
        el.style.background = '';
        el.style.border = '';
      });
    }
  }

  await browser.close();
  console.log(`\nDone: ${captured}/${CARDS.length} → ${OUT_DIR}`);
})().catch(err => { console.error(err.message); process.exit(1); });
