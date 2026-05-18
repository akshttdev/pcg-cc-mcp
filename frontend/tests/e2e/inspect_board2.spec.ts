import { test } from '@playwright/test';

import { loginAndGoto, ORG_ID } from './helpers/auth';

test('inspect board DOM 2', async ({ page }) => {
  await loginAndGoto(page, `/organizations/${ORG_ID}/crm/acquisition`);
  await page.waitForTimeout(5000);

  // Check for inline-grid
  const inlineGrid = await page.locator('[class*="inline-grid"]').count();
  console.log('inline-grid elements:', inlineGrid);

  // Check data-testid
  const testIds = await page.evaluate(() => {
    const els = document.querySelectorAll('[data-testid]');
    return Array.from(els)
      .map((e) => e.getAttribute('data-testid'))
      .slice(0, 20);
  });
  console.log('data-testid elements:', JSON.stringify(testIds));

  // Find all text nodes containing "Intel" or "Proposal"
  const stageTexts = await page.locator('text=Intel').count();
  const proposalTexts = await page.locator('text=Proposal').count();
  console.log(
    'Intel occurrences:',
    stageTexts,
    '| Proposal occurrences:',
    proposalTexts
  );

  // Current URL and body length
  console.log('URL:', page.url());
  const bodyLen = (await page.textContent('body'))?.length;
  console.log('Body length:', bodyLen);
});
