#!/usr/bin/env node
/**
 * Render a JavaScript page using Playwright
 *
 * Usage: node render-page.js <url> [timeout_ms]
 *
 * Output: JSON to stdout with:
 *   - url: final URL after redirects
 *   - html: rendered HTML content
 *   - title: page title
 *   - success: boolean
 *   - error: error message if failed
 *
 * Install: pnpm add -D playwright
 *          npx playwright install chromium
 */

const url = process.argv[2];
const timeoutMs = parseInt(process.argv[3]) || 30000;

if (!url) {
  console.error(JSON.stringify({
    success: false,
    error: 'Usage: node render-page.js <url> [timeout_ms]',
    url: '',
    html: '',
    title: null
  }));
  process.exit(1);
}

async function renderPage() {
  let browser;

  try {
    // Dynamic import to allow graceful failure if not installed
    const { chromium } = await import('playwright');

    browser = await chromium.launch({
      headless: true,
      args: [
        '--no-sandbox',
        '--disable-setuid-sandbox',
        '--disable-dev-shm-usage',
        '--disable-accelerated-2d-canvas',
        '--no-first-run',
        '--no-zygote',
        '--disable-gpu'
      ]
    });

    const context = await browser.newContext({
      userAgent: 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
      viewport: { width: 1920, height: 1080 }
    });

    const page = await context.newPage();

    // Navigate with timeout
    await page.goto(url, {
      waitUntil: 'networkidle',
      timeout: timeoutMs
    });

    // Wait a bit for any lazy-loaded content
    await page.waitForTimeout(1000);

    // Get the final URL (after redirects)
    const finalUrl = page.url();

    // Get the rendered HTML
    const html = await page.content();

    // Get the title
    const title = await page.title();

    await browser.close();

    console.log(JSON.stringify({
      success: true,
      url: finalUrl,
      html: html,
      title: title || null,
      error: null
    }));

  } catch (error) {
    if (browser) {
      await browser.close().catch(() => {});
    }

    console.log(JSON.stringify({
      success: false,
      url: url,
      html: '',
      title: null,
      error: error.message || String(error)
    }));

    process.exit(1);
  }
}

renderPage();
