import { test } from '@playwright/test';

import { loginAndGoto, ORG_ID } from './helpers/auth';

test('inspect board DOM', async ({ page }) => {
  await loginAndGoto(page, `/organizations/${ORG_ID}/crm/acquisition`);
  await page.waitForTimeout(3000);

  // Find the element containing Hudson's Car Club and get its parent classes
  const hudsonEl = page.locator("text=Hudson's Car Club").first();
  if ((await hudsonEl.count()) > 0) {
    // Walk up the DOM to find grid/kanban container
    const parent1 = await hudsonEl.evaluate((el) => {
      let node = el.parentElement;
      const result = [];
      for (let i = 0; i < 8; i++) {
        if (!node) break;
        result.push({
          tag: node.tagName,
          class: node.className.substring(0, 80),
        });
        node = node.parentElement;
      }
      return result;
    });
    console.log('Hudson parent chain:', JSON.stringify(parent1, null, 2));
  }

  // Also find any grid-like containers
  const gridEls = await page.evaluate(() => {
    const results: string[] = [];
    document.querySelectorAll('*').forEach((el) => {
      const cls = el.className;
      if (
        typeof cls === 'string' &&
        (cls.includes('grid') ||
          cls.includes('kanban') ||
          cls.includes('board') ||
          cls.includes('pipeline'))
      ) {
        results.push(el.tagName + ': ' + cls.substring(0, 80));
      }
    });
    return results.slice(0, 10);
  });
  console.log('Grid-like elements:', JSON.stringify(gridEls));
});
