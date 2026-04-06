const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  await page.setViewportSize({ width: 3000, height: 6000 });
  
  await page.goto('http://localhost:8765/vertical-preview.html', { 
    waitUntil: 'networkidle', timeout: 15000 
  });
  await page.waitForTimeout(2000);
  
  // Check first card label bounding box
  const cardLabel = page.locator('.card-label').first();
  const box = await cardLabel.boundingBox();
  console.log('First card-label box:', box);
  
  // Check page content
  const count = await page.locator('.card-label').count();
  console.log('Total card-labels:', count);
  
  // Get viewport
  const vp = page.viewportSize();
  console.log('Viewport:', vp);
  
  // Take full page screenshot to debug
  await page.screenshot({ path: '/tmp/debug_full.png', fullPage: true });
  console.log('Debug screenshot saved');
  
  await browser.close();
})().catch(e => console.error(e.message));
