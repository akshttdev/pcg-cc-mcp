import { test } from '@playwright/test';

import { loginAndGoto, ORG_ID } from './helpers/auth';

test('inspect board DOM 3', async ({ page }) => {
  await loginAndGoto(page, `/organizations/${ORG_ID}/crm/acquisition`);

  // Wait specifically for tab-content-pipelines to be visible and loaded
  await page
    .waitForSelector('[data-testid="tab-content-pipelines"]', {
      timeout: 15000,
    })
    .catch(() => null);
  await page.waitForTimeout(5000);

  const inlineGrid = await page.locator('[class*="inline-grid"]').count();
  console.log('inline-grid elements:', inlineGrid);

  // Check what's inside tab-content-pipelines
  const pipelinesContent = page.locator(
    '[data-testid="tab-content-pipelines"]'
  );
  const count = await pipelinesContent.count();
  if (count > 0) {
    const text = await pipelinesContent.textContent();
    console.log('pipelines tab text (first 300):', text?.substring(0, 300));
    const innerHTML = await pipelinesContent.innerHTML();
    console.log(
      'pipelines tab HTML (first 500):',
      innerHTML?.substring(0, 500)
    );
  } else {
    console.log('tab-content-pipelines not found');
  }

  // Check for Acquisition text
  const acqCount = await page.locator('text=Acquisition').count();
  const hudsonCount = await page.locator('text=Hudson').count();
  console.log('Acquisition text:', acqCount, '| Hudson text:', hudsonCount);
});
