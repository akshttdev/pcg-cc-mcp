/**
 * Exact reproduction of user-reported bug:
 * Clicking Hudson's Car Club Profile/Intel buttons from CRM Contacts
 * redirects to Sirak Studios org page instead of company profile.
 */
import { test, expect } from '@playwright/test';
import { loginAndGoto, ORG_ID } from './helpers/auth';

test.setTimeout(60000);

test('EXACT FLOW: CRM > Contacts > Companies > Hudson Profile button', async ({ page }) => {
  // Track every URL change
  const navLog: string[] = [];
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) {
      navLog.push(`[${new Date().toISOString().slice(11,19)}] ${frame.url()}`);
    }
  });

  // Step 1: Go to org CRM contacts page (exact URL from screenshot)
  await loginAndGoto(page, `/organizations/${ORG_ID}/crm/contacts`);
  await page.waitForTimeout(3000);
  console.log('Step 1 URL:', page.url());

  // Step 2: Click "Companies" sub-tab
  const companiesTab = page.locator('button').filter({ hasText: /^Companies/ }).first();
  await expect(companiesTab).toBeVisible({ timeout: 5000 });
  await companiesTab.click();
  await page.waitForTimeout(2000);
  console.log('Step 2 URL (after Companies tab):', page.url());

  // Step 3: Find Hudson's Car Club card
  const hudsonText = page.locator('text=Hudson\'s Car Club');
  const hudsonCount = await hudsonText.count();
  console.log('Hudson appearances:', hudsonCount);

  // Step 4: Find and inspect ALL links/buttons near Hudson
  const allLinks = await page.locator('a[href]').all();
  const hudsonLinks: string[] = [];
  for (const link of allLinks) {
    const href = await link.getAttribute('href');
    const text = await link.textContent();
    if (text?.includes('Hudson') || (href && text && (text.includes('Profile') || text.includes('Intel')))) {
      hudsonLinks.push(`[${text?.trim().slice(0, 40)}] → ${href}`);
    }
  }
  console.log('Links on page:', hudsonLinks);

  // Step 5: Click the Hudson Profile button specifically
  // The Profile button is a <Link> inside the card
  const hudsonProfileLinks = page.locator('a[href*="/companies/"]').filter({ hasText: /Profile/ });
  const profileCount = await hudsonProfileLinks.count();
  console.log('Profile buttons linking to /companies/:', profileCount);

  // Find the one closest to "Hudson's Car Club" text
  // Strategy: get all profile links, check which one is near Hudson
  let clickedHref = '';
  for (let i = 0; i < profileCount; i++) {
    const link = hudsonProfileLinks.nth(i);
    const href = await link.getAttribute('href');
    // Check if this link's parent card contains "Hudson"
    const card = link.locator('xpath=ancestor::div[contains(@class,"rounded-lg")][1]');
    const cardText = await card.textContent().catch(() => '');
    if (cardText?.includes('Hudson')) {
      clickedHref = href ?? '';
      console.log('Found Hudson Profile link:', href);
      console.log('Clicking it now...');
      await link.click();
      break;
    }
  }

  if (!clickedHref) {
    // Fallback: just click any Profile link that has /companies/ and the right ID
    const anyProfile = page.locator('a[href*="/companies/5b3d9e7c"]').first();
    if (await anyProfile.count() > 0) {
      clickedHref = (await anyProfile.getAttribute('href')) ?? '';
      console.log('Fallback: clicking Profile link:', clickedHref);
      await anyProfile.click();
    } else {
      console.log('NO Profile link found for Hudson!');
    }
  }

  // Step 6: Wait and check where we ended up
  await page.waitForTimeout(5000);

  console.log('');
  console.log('=== NAVIGATION LOG ===');
  navLog.forEach(n => console.log(n));
  console.log('=== END LOG ===');
  console.log('');
  console.log('FINAL URL:', page.url());

  const body = await page.textContent('body');
  if (page.url().includes('/organizations/')) {
    console.log('BUG CONFIRMED: Ended up on org page!');
    console.log('Body preview:', body?.slice(0, 300));
  } else if (page.url().includes('/companies/')) {
    console.log('SUCCESS: Landed on company profile page');
    console.log('Shows Hudson:', body?.includes('Hudson'));
  }

  expect(page.url()).toContain('/companies/');
});

test('EXACT FLOW: CRM > Contacts > Companies > Hudson Intel button', async ({ page }) => {
  const navLog: string[] = [];
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) {
      navLog.push(`[${new Date().toISOString().slice(11,19)}] ${frame.url()}`);
    }
  });

  await loginAndGoto(page, `/organizations/${ORG_ID}/crm/contacts`);
  await page.waitForTimeout(3000);

  const companiesTab = page.locator('button').filter({ hasText: /^Companies/ }).first();
  await companiesTab.click();
  await page.waitForTimeout(2000);

  // Click Hudson Intel button
  const hudsonIntelLinks = page.locator('a[href*="/companies/"]').filter({ hasText: /Intel/ });
  const intelCount = await hudsonIntelLinks.count();
  console.log('Intel buttons linking to /companies/:', intelCount);

  for (let i = 0; i < intelCount; i++) {
    const link = hudsonIntelLinks.nth(i);
    const card = link.locator('xpath=ancestor::div[contains(@class,"rounded-lg")][1]');
    const cardText = await card.textContent().catch(() => '');
    if (cardText?.includes('Hudson')) {
      const href = await link.getAttribute('href');
      console.log('Found Hudson Intel link:', href);
      await link.click();
      break;
    }
  }

  await page.waitForTimeout(5000);

  console.log('');
  console.log('=== NAVIGATION LOG ===');
  navLog.forEach(n => console.log(n));
  console.log('');
  console.log('FINAL URL:', page.url());

  if (page.url().includes('/organizations/')) {
    console.log('BUG CONFIRMED: Intel click ended up on org page!');
  }

  expect(page.url()).toContain('/companies/');
});

test('EXACT FLOW: Pipeline > Deal panel > OverviewTab > Org Open button', async ({ page }) => {
  const navLog: string[] = [];
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) {
      navLog.push(`[${new Date().toISOString().slice(11,19)}] ${frame.url()}`);
    }
  });

  // Go to pipeline (exact URL from screenshot 3)
  await loginAndGoto(page, `/organizations/${ORG_ID}/crm/pipeline`);
  await page.waitForSelector('[class*="inline-grid"]', { timeout: 20000 });
  await page.waitForTimeout(2000);

  // Open Joshua Marotta deal
  const kanban = page.locator('[class*="inline-grid"]');
  const dealCard = kanban.locator('p').filter({ hasText: /Joshua Marotta/ }).first();
  await dealCard.scrollIntoViewIfNeeded();
  await dealCard.click();
  await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
  await page.waitForTimeout(1000);

  const dialog = page.locator('[role="dialog"]');

  // Find ALL links in the dialog and log them
  const dialogLinks = await dialog.locator('a[href]').all();
  console.log('All links in deal panel:');
  for (const link of dialogLinks) {
    const href = await link.getAttribute('href');
    const text = (await link.textContent())?.trim().slice(0, 50);
    console.log(`  [${text}] → ${href}`);
  }

  // The "Open" button next to "Sirak Studios" → should go to /organizations/
  const openOrgBtn = dialog.locator('a').filter({ hasText: /^Open/ }).first();
  if (await openOrgBtn.count() > 0) {
    const href = await openOrgBtn.getAttribute('href');
    console.log('"Open" button href:', href);
    console.log('This CORRECTLY goes to org page (it\'s the org Open button, not company)');
    expect(href).toContain('/organizations/');
  }

  // The company name "Hudson's Car Club" in the Org section → should link to /companies/
  const hudsonLink = dialog.locator('a[href*="/companies/"]').filter({ hasText: /Hudson/ });
  if (await hudsonLink.count() > 0) {
    const href = await hudsonLink.first().getAttribute('href');
    console.log('Hudson company link in Org section:', href);
    expect(href).toContain('/companies/');
  } else {
    console.log('GAP: Hudson company name in Org section is NOT a link');
  }

  // The "Co. Profile" button → should link to /companies/
  const coProfileBtn = dialog.locator('a').filter({ hasText: /Co\. Profile|Profile/ }).first();
  if (await coProfileBtn.count() > 0) {
    const href = await coProfileBtn.getAttribute('href');
    console.log('Co. Profile button href:', href);
    expect(href).toContain('/companies/');
  }
});

test('EXACT FLOW: Pipeline > Deal panel > IntelTab > PROFILE button', async ({ page }) => {
  const navLog: string[] = [];
  page.on('framenavigated', (frame) => {
    if (frame === page.mainFrame()) {
      navLog.push(`[${new Date().toISOString().slice(11,19)}] ${frame.url()}`);
    }
  });

  await loginAndGoto(page, `/organizations/${ORG_ID}/crm/pipeline`);
  await page.waitForSelector('[class*="inline-grid"]', { timeout: 20000 });
  await page.waitForTimeout(2000);

  const kanban = page.locator('[class*="inline-grid"]');
  const dealCard = kanban.locator('p').filter({ hasText: /Joshua Marotta/ }).first();
  await dealCard.scrollIntoViewIfNeeded();
  await dealCard.click();
  await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
  await page.waitForTimeout(1000);

  const dialog = page.locator('[role="dialog"]');

  // Click Intel tab
  await dialog.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /^Intel$/ }).first().click();
  await page.waitForTimeout(1500);

  // Find the PROFILE button next to HUDSON'S CAR CLUB (screenshot 3)
  const profileLink = dialog.locator('a[href*="/companies/"]').filter({ hasText: /PROFILE|Profile/ }).first();
  if (await profileLink.count() > 0) {
    const href = await profileLink.getAttribute('href');
    console.log('IntelTab PROFILE href:', href);
    expect(href).toContain('/companies/');

    // Actually click it
    await profileLink.click();
    await page.waitForTimeout(5000);

    console.log('FINAL URL after PROFILE click:', page.url());
    console.log('Nav log:', navLog.filter(n => !n.includes('about:blank')));

    if (page.url().includes('/organizations/')) {
      console.log('BUG CONFIRMED: PROFILE button navigated to org page!');
    }
    expect(page.url()).toContain('/companies/');
  } else {
    console.log('PROFILE button not found in IntelTab');
  }
});
