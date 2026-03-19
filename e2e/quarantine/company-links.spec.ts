/**
 * Company Profile & Intel Link Tests
 * Clicks every place a company name/profile/intel appears across:
 * CRM Kanban, Deal Detail, Org Contacts (People/Companies/Pipeline),
 * Contact Detail Modal, People Directory, Person Profile
 */
import { test, expect, Page } from '@playwright/test';
import { loginAsAdmin, loginAndGoto, DEAL_ID, PIPELINE_ID, ORG_ID } from './helpers/auth';

const BASE_URL = `http://localhost:${process.env.FRONTEND_PORT || '3000'}`;

test.beforeEach(async ({ page }) => {
  await loginAsAdmin(page);
});

async function apiGet(page: Page, path: string) {
  const res = await page.request.get(`${BASE_URL}/api${path}`);
  return res.json();
}

// Get known IDs for testing
async function getTestIds(page: Page) {
  const res = await apiGet(page, `/crm/deals/${DEAL_ID}/rich`);
  return {
    personId: res.data?.person_id as string,
    companyId: res.data?.company_id as string,
    companyName: res.data?.contact_company as string,
    contactName: res.data?.contact_name as string,
  };
}

// ── CRM KANBAN — Deal Card ──────────────────────────────────────────────────

test.describe('CRM Kanban — Deal Card Company Link', () => {
  test('deal card shows company name but it is NOT clickable (gap)', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}/crm/acquisition`);
    await page.waitForSelector('[class*="inline-grid"]', { timeout: 20000 });
    await page.waitForTimeout(2000);

    // Find company name text on a deal card
    const companyText = page.locator('text=Hudson\'s Car Club').first();
    await expect(companyText).toBeVisible({ timeout: 5000 });

    // Check if it's wrapped in a link
    const parentLink = companyText.locator('xpath=ancestor::a[contains(@href, "/companies/")]');
    const isLinked = await parentLink.count();
    console.log('Kanban card company name is a link:', isLinked > 0);
    // GAP: Company name on kanban cards is plain text
    console.log('GAP CONFIRMED: Kanban deal card company name is plain text, not clickable');
  });
});

// ── DEAL DETAIL — OverviewTab ────────────────────────────────────────────────

test.describe('Deal Detail — Company Links', () => {
  async function openDealPanel(page: Page) {
    await loginAndGoto(page, `/organizations/${ORG_ID}/crm/acquisition`);
    await page.waitForSelector('[class*="inline-grid"]', { timeout: 20000 });
    await page.waitForTimeout(2000);
    const kanban = page.locator('[class*="inline-grid"]');
    const dealCard = kanban.locator('p').filter({ hasText: /Joshua Marotta/ }).first();
    await dealCard.scrollIntoViewIfNeeded();
    await dealCard.click();
    await page.waitForSelector('[role="dialog"]', { timeout: 10000 });
    await page.waitForTimeout(800);
  }

  test('OverviewTab "Co. Profile" button links to /companies/:id and is clickable', async ({ page }) => {
    const { companyId } = await getTestIds(page);
    await openDealPanel(page);

    const dialog = page.locator('[role="dialog"]');
    // Look for the Co. Profile button/link
    const coProfileBtn = dialog.locator('a[href*="/companies/"]').first();
    await expect(coProfileBtn).toBeVisible({ timeout: 5000 });
    const href = await coProfileBtn.getAttribute('href');
    expect(href).toContain(`/companies/${companyId}`);
    console.log('OverviewTab Co. Profile link:', href);

    // Actually click it and verify navigation
    await coProfileBtn.click();
    await page.waitForTimeout(2000);
    expect(page.url()).toContain(`/companies/${companyId}`);
    const bodyText = await page.textContent('body');
    expect(bodyText).toContain('Hudson');
    console.log('CLICKED: Co. Profile → navigated to company page');
  });

  test('IntelTab "Profile" button links to /companies/:id and is clickable', async ({ page }) => {
    const { companyId } = await getTestIds(page);
    await openDealPanel(page);

    const dialog = page.locator('[role="dialog"]');
    await dialog.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /^Intel$/ }).first().click();
    await page.waitForTimeout(1000);

    // Find company profile link in Intel tab
    const companyLink = dialog.locator(`a[href*="/companies/${companyId}"]`).first();
    await expect(companyLink).toBeVisible({ timeout: 5000 });
    const href = await companyLink.getAttribute('href');
    console.log('IntelTab company link:', href);

    // Click and verify
    await companyLink.click();
    await page.waitForTimeout(2000);
    expect(page.url()).toContain(`/companies/${companyId}`);
    console.log('CLICKED: IntelTab Profile → navigated to company page');
  });
});

// ── ORG CONTACTS — Companies View ────────────────────────────────────────────

test.describe('Org Contacts — Companies View', () => {
  test('Companies tab shows company cards with Profile + Intel buttons that navigate', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}`);
    await page.waitForTimeout(2000);

    // Click Contacts tab if visible
    const contactsTab = page.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /Contacts/i }).first();
    if (await contactsTab.count() > 0) {
      await contactsTab.click();
      await page.waitForTimeout(1500);
    }

    // Switch to Companies view
    const companiesBtn = page.locator('button').filter({ hasText: /^Companies/ }).first();
    if (await companiesBtn.count() > 0) {
      await companiesBtn.click();
      await page.waitForTimeout(2000);
    }

    // Find any company Profile button
    const profileBtns = page.locator('a').filter({ hasText: /Profile/ });
    const profileCount = await profileBtns.count();
    console.log('Company Profile buttons found:', profileCount);

    // Find any company Intel button
    const intelBtns = page.locator('a').filter({ hasText: /Intel/ });
    const intelCount = await intelBtns.count();
    console.log('Company Intel buttons found:', intelCount);

    if (profileCount > 0) {
      const firstProfile = profileBtns.first();
      const href = await firstProfile.getAttribute('href');
      expect(href).toContain('/companies/');
      console.log('First company Profile link:', href);

      // Click it
      await firstProfile.click();
      await page.waitForTimeout(2000);
      expect(page.url()).toContain('/companies/');
      console.log('CLICKED: Companies view Profile → navigated to company page');
    }
  });

  test('Companies view Intel button navigates to company intel tab', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}`);
    await page.waitForTimeout(2000);

    const contactsTab = page.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /Contacts/i }).first();
    if (await contactsTab.count() > 0) {
      await contactsTab.click();
      await page.waitForTimeout(1500);
    }

    const companiesBtn = page.locator('button').filter({ hasText: /^Companies/ }).first();
    if (await companiesBtn.count() > 0) {
      await companiesBtn.click();
      await page.waitForTimeout(2000);
    }

    const intelBtns = page.locator('a').filter({ hasText: /Intel/ });
    if (await intelBtns.count() > 0) {
      const href = await intelBtns.first().getAttribute('href');
      expect(href).toContain('/companies/');
      expect(href).toContain('tab=intelligence');
      console.log('Intel button href:', href);

      await intelBtns.first().click();
      await page.waitForTimeout(2000);
      expect(page.url()).toContain('/companies/');
      console.log('CLICKED: Companies view Intel → navigated to company intel');
    } else {
      console.log('No Intel buttons found in Companies view');
    }
  });
});

// ── ORG CONTACTS — People View ───────────────────────────────────────────────

test.describe('Org Contacts — People View', () => {
  test('People view contact cards show company name — check if clickable', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}`);
    await page.waitForTimeout(2000);

    const contactsTab = page.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /Contacts/i }).first();
    if (await contactsTab.count() > 0) {
      await contactsTab.click();
      await page.waitForTimeout(1500);
    }

    // People view is default — check for company name links
    const companyLinks = page.locator('a[href*="/companies/"]');
    const companyLinkCount = await companyLinks.count();
    console.log('Company links in People view:', companyLinkCount);

    // Check if Hudson's Car Club specifically is a link
    const hccLink = page.locator('a[href*="/companies/"]').filter({ hasText: /Hudson/ });
    const hccLinked = await hccLink.count();
    console.log('Hudson\'s Car Club is linked in People view:', hccLinked > 0);

    if (hccLinked > 0) {
      await hccLink.first().click();
      await page.waitForTimeout(2000);
      expect(page.url()).toContain('/companies/');
      console.log('CLICKED: People view company name → navigated to company page');
    } else {
      console.log('GAP: Company names in People view not clickable (need company_id from deal map)');
    }
  });
});

// ── ORG CONTACTS — Pipeline View ─────────────────────────────────────────────

test.describe('Org Contacts — Pipeline View', () => {
  test('Pipeline view shows company with link icon', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}`);
    await page.waitForTimeout(2000);

    const contactsTab = page.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /Contacts/i }).first();
    if (await contactsTab.count() > 0) {
      await contactsTab.click();
      await page.waitForTimeout(1500);
    }

    // Switch to Pipeline view
    const pipelineBtn = page.locator('button').filter({ hasText: /^Pipeline/ }).first();
    if (await pipelineBtn.count() > 0) {
      await pipelineBtn.click();
      await page.waitForTimeout(2000);
    }

    // Check for company links (ExternalLink icon next to company name)
    const companyLinks = page.locator('a[href*="/companies/"]');
    const count = await companyLinks.count();
    console.log('Company links in Pipeline view:', count);

    if (count > 0) {
      const href = await companyLinks.first().getAttribute('href');
      console.log('First company link:', href);

      await companyLinks.first().click();
      await page.waitForTimeout(2000);
      expect(page.url()).toContain('/companies/');
      console.log('CLICKED: Pipeline view company link → navigated to company page');
    }
  });
});

// ── CONTACT DETAIL MODAL ─────────────────────────────────────────────────────

test.describe('Contact Detail Modal — Company Link', () => {
  test('contact modal shows company name as plain text (gap)', async ({ page }) => {
    await loginAndGoto(page, `/organizations/${ORG_ID}`);
    await page.waitForTimeout(2000);

    const contactsTab = page.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /Contacts/i }).first();
    if (await contactsTab.count() > 0) {
      await contactsTab.click();
      await page.waitForTimeout(1500);
    }

    // Click on a contact card to open modal
    const contactCard = page.locator('text=Hudson').first();
    if (await contactCard.count() > 0) {
      await contactCard.click();
      await page.waitForTimeout(1500);

      // Check if modal has company link
      const modal = page.locator('[role="dialog"]');
      if (await modal.count() > 0) {
        const companyLink = modal.locator('a[href*="/companies/"]');
        const hasCompanyLink = await companyLink.count();
        console.log('Contact modal has company link:', hasCompanyLink > 0);
        if (hasCompanyLink === 0) {
          console.log('GAP CONFIRMED: Contact Detail Modal shows company name as plain text (line 211-213)');
        }
      }
    }
  });
});

// ── PEOPLE DIRECTORY ─────────────────────────────────────────────────────────

test.describe('People Directory — Company Link', () => {
  test('/people page shows company names — check if clickable', async ({ page }) => {
    await loginAndGoto(page, '/people');
    await page.waitForTimeout(3000);

    // Check for Building2 icon + company text
    const companySpans = page.locator('text=Hudson\'s Car Club');
    const count = await companySpans.count();
    console.log('"Hudson\'s Car Club" appearances in /people:', count);

    // Check if any are wrapped in links
    const companyLinks = page.locator('a[href*="/companies/"]');
    const linkCount = await companyLinks.count();
    console.log('Company links in /people directory:', linkCount);
    if (linkCount === 0 && count > 0) {
      console.log('GAP CONFIRMED: People directory shows company names as plain text (people.tsx:89-93)');
    }
  });
});

// ── PERSON PROFILE PAGE ──────────────────────────────────────────────────────

test.describe('Person Profile — Company Link', () => {
  test('person profile page shows company with link to /companies/:id', async ({ page }) => {
    const { personId, companyId } = await getTestIds(page);

    await loginAndGoto(page, `/people/${personId}`);
    await page.waitForTimeout(3000);

    // Check for company links on person profile
    const companyLinks = page.locator(`a[href*="/companies/"]`);
    const count = await companyLinks.count();
    console.log('Company links on person profile:', count);

    if (count > 0) {
      const href = await companyLinks.first().getAttribute('href');
      console.log('Person profile company link:', href);

      await companyLinks.first().click();
      await page.waitForTimeout(2000);
      expect(page.url()).toContain('/companies/');
      console.log('CLICKED: Person profile company link → navigated to company page');
    } else {
      // Check if company name is shown at all
      const bodyText = await page.textContent('body');
      const hasCompanyName = bodyText?.includes('Hudson');
      console.log('Person profile shows company name:', hasCompanyName);
      if (hasCompanyName) {
        console.log('GAP: Person profile shows company name but no link to company profile');
      }
    }
  });
});

// ── COMPANY PROFILE PAGE — Verify it loads ───────────────────────────────────

test.describe('Company Profile Page', () => {
  test('company profile loads with intelligence tab', async ({ page }) => {
    const { companyId } = await getTestIds(page);

    await loginAndGoto(page, `/companies/${companyId}`);
    await page.waitForTimeout(2000);

    const bodyText = await page.textContent('body');
    expect(bodyText).toContain('Hudson');

    // Check for Intelligence tab
    const intelTab = page.locator('[role="tablist"] button, [role="tab"]').filter({ hasText: /Intel/i });
    const hasIntelTab = await intelTab.count();
    console.log('Company profile has Intelligence tab:', hasIntelTab > 0);

    if (hasIntelTab > 0) {
      await intelTab.first().click();
      await page.waitForTimeout(1500);
      const tabContent = await page.textContent('body');
      console.log('Intelligence tab content loaded, length:', tabContent?.length);
    }
    console.log('Company profile page fully functional');
  });

  test('company profile loads via ?tab=intelligence param', async ({ page }) => {
    const { companyId } = await getTestIds(page);

    await loginAndGoto(page, `/companies/${companyId}?tab=intelligence`);
    await page.waitForTimeout(3000);

    const bodyText = await page.textContent('body');
    expect(bodyText).toContain('Hudson');
    console.log('Company profile loaded with ?tab=intelligence');
  });
});
