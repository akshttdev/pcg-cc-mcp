/**
 * QA test script for role-based UI feature.
 * Tests: SidebarUserCard, view-as switcher, settings scope tabs,
 *        ViewAsBanner, localStorage persistence, navbar cleanup.
 */
const { chromium } = require('playwright');

const BASE = 'http://127.0.0.1:3200';
const CREDS = { username: 'admin', password: 'admin123' };

const results = [];
let testNum = 0;

function log(status, name, detail = '') {
  testNum++;
  const icon = status === 'PASS' ? '✅' : status === 'FAIL' ? '❌' : '⚠️';
  const line = `${icon} [${testNum}] ${name}${detail ? ' — ' + detail : ''}`;
  console.log(line);
  results.push({ num: testNum, status, name, detail });
}

async function getAsideText(page) {
  return page.evaluate(() => {
    const aside = document.querySelector('aside');
    return aside ? aside.textContent : '';
  });
}

async function run() {
  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();

  const consoleErrors = [];
  page.on('console', msg => { if (msg.type() === 'error') consoleErrors.push(msg.text()); });
  page.on('pageerror', err => consoleErrors.push(err.message));

  try {
    // ─── Phase 1: Login ───────────────────────────────────────
    console.log('\n═══ Phase 1: Login & Initial Load ═══\n');

    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 15000 });
    await page.fill('input[name="username"], input[type="text"]', CREDS.username);
    await page.fill('input[type="password"]', CREDS.password);
    await page.click('button[type="submit"]');
    await page.waitForURL('**/*', { timeout: 10000 });
    await page.waitForTimeout(3000);

    log(!page.url().includes('/login') ? 'PASS' : 'FAIL', 'Login succeeds', page.url());

    // ─── Phase 2: Navbar — ProfileSection Removed ─────────────
    console.log('\n═══ Phase 2: Navbar Cleanup ═══\n');

    const navbarHasAvatar = await page.evaluate(() => {
      const navbar = document.querySelector('.sticky.top-0');
      if (!navbar) return false;
      return navbar.querySelectorAll('[class*="avatar"], [class*="Avatar"]').length > 0;
    });
    log(!navbarHasAvatar ? 'PASS' : 'FAIL', 'ProfileSection removed from navbar');

    // ─── Phase 3: SidebarUserCard ─────────────────────────────
    console.log('\n═══ Phase 3: Sidebar User Card ═══\n');

    const sidebarContent = await page.evaluate(() => {
      const sidebar = document.querySelector('.sidebar-container');
      return sidebar ? sidebar.textContent : '';
    });
    log(sidebarContent.includes('Administrator') ? 'PASS' : 'FAIL',
      'User name visible in sidebar', 'Looking for "Administrator"');
    log(sidebarContent.includes('Platform Admin') ? 'PASS' : 'FAIL',
      'Role label visible in sidebar', 'Looking for "Platform Admin"');

    // ─── Phase 4: Popover ─────────────────────────────────────
    console.log('\n═══ Phase 4: View-As Popover ═══\n');

    // Click the user card button
    const clicked = await page.evaluate(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const userBtn = buttons.find(b => b.textContent.includes('Platform Admin'));
      if (userBtn) { userBtn.click(); return true; }
      return false;
    });
    log(clicked ? 'PASS' : 'FAIL', 'User card button found and clicked');
    await page.waitForTimeout(500);

    // Get popover content (Radix portals to body)
    const popoverText = await page.evaluate(() => {
      const popovers = document.querySelectorAll('[data-radix-popper-content-wrapper]');
      const last = popovers[popovers.length - 1];
      return last ? last.textContent : '';
    });

    log(popoverText.includes('Platform') && popoverText.includes('Organization') && popoverText.includes('Client')
      ? 'PASS' : 'FAIL',
      'Popover has grouped role sections (Platform/Organization/Client)');

    log(popoverText.includes('Org Admin') && popoverText.includes('Org Editor') && popoverText.includes('Org Viewer')
      ? 'PASS' : 'FAIL',
      'All org role options present');

    log(popoverText.includes('Client Admin') && popoverText.includes('Client Editor') && popoverText.includes('Client Viewer')
      ? 'PASS' : 'FAIL',
      'All client role options present');

    log(popoverText.includes('Platform Member') ? 'PASS' : 'FAIL',
      'Platform Member role option present');

    log(popoverText.includes('Project Member') && popoverText.includes('Authenticated')
      ? 'PASS' : 'FAIL',
      'Other roles present (Project Member, Authenticated)');

    log(popoverText.includes('Sign out') ? 'PASS' : 'FAIL', 'Sign out button in popover');

    log(popoverText.includes('admin@') || popoverText.includes('Administrator')
      ? 'PASS' : 'FAIL', 'User info displayed in popover');

    log(popoverText.includes('View as') ? 'PASS' : 'FAIL', '"View as..." header in popover');

    // ─── Phase 5: Select View-As Role ─────────────────────────
    console.log('\n═══ Phase 5: View-As Selection ═══\n');

    // Click "Org Editor"
    const roleSelected = await page.evaluate(() => {
      const popovers = document.querySelectorAll('[data-radix-popper-content-wrapper]');
      const last = popovers[popovers.length - 1];
      if (!last) return false;
      const buttons = last.querySelectorAll('button');
      const orgEditor = Array.from(buttons).find(b => b.textContent.trim() === 'Org Editor');
      if (orgEditor) { orgEditor.click(); return true; }
      return false;
    });
    log(roleSelected ? 'PASS' : 'FAIL', 'Org Editor role selected');
    await page.waitForTimeout(1000);

    // Check banner
    const bannerText = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="view-as-banner"]');
      return el ? el.textContent : '';
    });
    log(bannerText.includes('Viewing as') ? 'PASS' : 'FAIL', 'View-as banner appears');
    log(bannerText.includes('Org Editor') ? 'PASS' : 'FAIL',
      'Banner shows "Org Editor"', `Banner: "${bannerText.trim().slice(0, 60)}"`);
    log(bannerText.includes('Reset') ? 'PASS' : 'FAIL', 'Reset button visible in banner');

    // ─── Phase 6: Sidebar Visibility ──────────────────────────
    console.log('\n═══ Phase 6: Sidebar Sections Update ═══\n');

    const sidebarAfterOverride = await page.evaluate(() => {
      const sidebar = document.querySelector('.sidebar-container');
      return sidebar ? sidebar.textContent : '';
    });

    // Admin-only sections should be hidden
    log(!sidebarAfterOverride.includes('Site Directory') ? 'PASS' : 'FAIL',
      'Site Directory hidden for org_editor');
    log(!sidebarAfterOverride.includes('Mission Control') ? 'PASS' : 'FAIL',
      'Mission Control hidden for org_editor');

    // Management section hidden (requires platform_member+)
    const mgmtHidden = !sidebarAfterOverride.includes('Management');
    log(mgmtHidden ? 'PASS' : 'FAIL', 'Management section hidden for org_editor');

    // Creative tools visible (org_editor should see these)
    const creativeVisible = sidebarAfterOverride.includes('VIBE');
    log(creativeVisible ? 'PASS' : 'FAIL', 'Creative tools visible for org_editor');

    // ─── Phase 7: localStorage Persistence ────────────────────
    console.log('\n═══ Phase 7: localStorage Persistence ═══\n');

    const stored = await page.evaluate(() => localStorage.getItem('pcg:view-as-role'));
    log(stored === 'org_editor' ? 'PASS' : 'FAIL', 'View-as stored in localStorage', `"${stored}"`);

    await page.reload({ waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);

    const bannerAfterReload = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="view-as-banner"]');
      return el ? el.textContent : '';
    });
    log(bannerAfterReload.includes('Viewing as') ? 'PASS' : 'FAIL', 'Banner persists after reload');

    const storedAfterReload = await page.evaluate(() => localStorage.getItem('pcg:view-as-role'));
    log(storedAfterReload === 'org_editor' ? 'PASS' : 'FAIL', 'localStorage survives reload');

    // ─── Phase 8: Reset ───────────────────────────────────────
    console.log('\n═══ Phase 8: Reset View-As ═══\n');

    const resetClicked = await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const reset = btns.find(b => b.textContent.includes('Reset'));
      if (reset) { reset.click(); return true; }
      return false;
    });
    log(resetClicked ? 'PASS' : 'FAIL', 'Reset button clicked');
    await page.waitForTimeout(1000);

    const bannerGone = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="view-as-banner"]');
      return !el || !el.textContent.includes('Viewing as');
    });
    log(bannerGone ? 'PASS' : 'FAIL', 'Banner disappears after reset');

    const storedAfterReset = await page.evaluate(() => localStorage.getItem('pcg:view-as-role'));
    log(!storedAfterReset ? 'PASS' : 'FAIL', 'localStorage cleared after reset');

    const sidebarAfterReset = await page.evaluate(() => {
      const sidebar = document.querySelector('.sidebar-container');
      return sidebar ? sidebar.textContent : '';
    });
    log(sidebarAfterReset.includes('Admin Platforms') || sidebarAfterReset.includes('Site Directory')
      ? 'PASS' : 'FAIL', 'Admin sections restored after reset');

    // ─── Phase 9: Settings Scope Tabs ─────────────────────────
    console.log('\n═══ Phase 9: Settings Scope Tabs ═══\n');

    await page.goto(`${BASE}/settings/general`, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);

    const settingsText = await getAsideText(page);

    log(settingsText.includes('User') ? 'PASS' : 'FAIL', 'User tab visible');
    log(settingsText.includes('System Admin') ? 'PASS' : 'FAIL', 'System Admin tab visible');
    log(settingsText.includes('Organization') ? 'PASS' : 'FAIL', 'Organization tab visible');
    log(settingsText.includes('Client') ? 'PASS' : 'FAIL', 'Client tab visible');

    // Default is User tab — should show personal items
    log(settingsText.includes('General') && settingsText.includes('Profile')
      ? 'PASS' : 'FAIL', 'User tab: personal settings shown (General, Profile)');
    log(settingsText.includes('Wallet') && settingsText.includes('Privacy')
      ? 'PASS' : 'FAIL', 'User tab: Wallet, Privacy shown');
    log(settingsText.includes('API Keys') ? 'PASS' : 'FAIL', 'User tab: API Keys shown');
    log(settingsText.includes('Coming soon') ? 'PASS' : 'FAIL',
      'User tab: planned items with "Coming soon"');

    // Click System Admin tab
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const sysBtn = btns.find(b => b.textContent.trim() === 'System Admin');
      if (sysBtn) sysBtn.click();
    });
    await page.waitForTimeout(500);

    const sysText = await getAsideText(page);
    log(sysText.includes('Users') && sysText.includes('Organizations') && sysText.includes('Projects')
      ? 'PASS' : 'FAIL', 'System Admin: admin management items shown');
    log(sysText.includes('Agents') && sysText.includes('Models') && sysText.includes('MCP')
      ? 'PASS' : 'FAIL', 'System Admin: org items shown (Agents, Models, MCP)');
    log(sysText.includes('Pulse') ? 'PASS' : 'FAIL', 'System Admin: Pulse shown');
    log(sysText.includes('Developer') ? 'PASS' : 'FAIL', 'System Admin: Developer shown');
    log(sysText.includes('General') && sysText.includes('Wallet')
      ? 'PASS' : 'FAIL', 'System Admin: user items also shown');

    // Click Org tab
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const orgBtn = btns.find(b => b.textContent.trim() === 'Organization');
      if (orgBtn) orgBtn.click();
    });
    await page.waitForTimeout(500);

    const orgText = await getAsideText(page);
    log(orgText.includes('Agents') && orgText.includes('Models') && orgText.includes('MCP')
      ? 'PASS' : 'FAIL', 'Org tab: Agents, Models, MCP shown');
    log(orgText.includes('Network') ? 'PASS' : 'FAIL', 'Org tab: Network & Mesh shown');
    log(orgText.includes('Coming soon') ? 'PASS' : 'FAIL', 'Org tab: planned items shown');
    log(orgText.includes('Billing') ? 'PASS' : 'FAIL', 'Org tab: planned Billing shown');

    // Click Client tab
    await page.evaluate(() => {
      const btns = Array.from(document.querySelectorAll('button'));
      const clientBtn = btns.find(b => b.textContent.trim() === 'Client');
      if (clientBtn) clientBtn.click();
    });
    await page.waitForTimeout(500);

    const clientText = await getAsideText(page);
    log(clientText.includes('Pulse') ? 'PASS' : 'FAIL', 'Client tab: Pulse Engine shown');
    log(clientText.includes('Branding') ? 'PASS' : 'FAIL', 'Client tab: planned Branding shown');
    log(clientText.includes('Client Portal') ? 'PASS' : 'FAIL', 'Client tab: planned Client Portal shown');
    log(clientText.includes('Coming soon') ? 'PASS' : 'FAIL', 'Client tab: "Coming soon" badges');

    // ─── Phase 10: Settings with Override ─────────────────────
    console.log('\n═══ Phase 10: Settings Tabs with View-As Override ═══\n');

    await page.evaluate(() => localStorage.setItem('pcg:view-as-role', 'client_editor'));
    await page.reload({ waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);

    const overriddenSettings = await getAsideText(page);
    log(!overriddenSettings.includes('System Admin') ? 'PASS' : 'FAIL',
      'System Admin tab hidden for client_editor');
    log(!overriddenSettings.includes('Organization') ? 'PASS' : 'FAIL',
      'Org tab hidden for client_editor');
    log(overriddenSettings.includes('User') ? 'PASS' : 'FAIL',
      'User tab visible for client_editor');
    log(overriddenSettings.includes('Client') ? 'PASS' : 'FAIL',
      'Client tab visible for client_editor');

    await page.evaluate(() => localStorage.removeItem('pcg:view-as-role'));

    // ─── Phase 11: Edge Cases ─────────────────────────────────
    console.log('\n═══ Phase 11: Edge Cases ═══\n');

    // Same role as actual
    await page.evaluate(() => localStorage.setItem('pcg:view-as-role', 'platform_admin'));
    await page.reload({ waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    const noBannerSame = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="view-as-banner"]');
      return !el || !el.textContent.includes('Viewing as');
    });
    log(noBannerSame ? 'PASS' : 'FAIL', 'No banner when view-as = actual role');

    // Invalid role
    await page.evaluate(() => localStorage.setItem('pcg:view-as-role', 'garbage_role'));
    await page.reload({ waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    const noBannerInvalid = await page.evaluate(() => {
      const el = document.querySelector('[data-testid="view-as-banner"]');
      return !el || !el.textContent.includes('Viewing as');
    });
    log(noBannerInvalid ? 'PASS' : 'FAIL', 'No banner for invalid localStorage role');

    await page.evaluate(() => localStorage.removeItem('pcg:view-as-role'));

    // ─── Phase 12: Collapsed Sidebar ──────────────────────────
    console.log('\n═══ Phase 12: Collapsed Sidebar ═══\n');

    await page.goto(BASE, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);

    // Collapse via keyboard
    await page.keyboard.press('Meta+b');
    await page.waitForTimeout(500);

    const isCollapsed = await page.evaluate(() => {
      const sidebar = document.querySelector('.sidebar-container');
      return sidebar ? sidebar.offsetWidth < 100 : false;
    });
    log(isCollapsed ? 'PASS' : 'WARN', 'Sidebar collapses on Cmd+B');

    // User card should still have avatar in collapsed mode
    if (isCollapsed) {
      const hasAvatarCollapsed = await page.evaluate(() => {
        const sidebar = document.querySelector('.sidebar-container');
        if (!sidebar) return false;
        const avatars = sidebar.querySelectorAll('[class*="avatar"], [class*="Avatar"]');
        return avatars.length > 0;
      });
      log(hasAvatarCollapsed ? 'PASS' : 'WARN', 'Avatar visible in collapsed sidebar');
    }

    // Expand again
    await page.keyboard.press('Meta+b');
    await page.waitForTimeout(500);

    // ─── Phase 13: Console Errors ─────────────────────────────
    console.log('\n═══ Phase 13: Console Errors ═══\n');

    const criticalErrors = consoleErrors.filter(e =>
      !e.includes('favicon') && !e.includes('404') &&
      !e.includes('ResizeObserver') && !e.includes('net::ERR') &&
      !e.includes('Failed to load resource')
    );
    log(criticalErrors.length === 0 ? 'PASS' : 'WARN',
      'No critical console errors',
      criticalErrors.length > 0 ? `${criticalErrors.length}: ${criticalErrors[0].slice(0, 100)}` : 'Clean');

  } catch (err) {
    log('FAIL', 'Test execution error', err.message);
    await page.screenshot({ path: '/tmp/role-ui-error.png' }).catch(() => {});
  } finally {
    await browser.close();
  }

  // ─── Summary ───────────────────────────────────────────────
  console.log('\n═══════════════════════════════════════');
  console.log('           TEST SUMMARY');
  console.log('═══════════════════════════════════════\n');

  const pass = results.filter(r => r.status === 'PASS').length;
  const fail = results.filter(r => r.status === 'FAIL').length;
  const warn = results.filter(r => r.status === 'WARN').length;
  console.log(`  PASS: ${pass}  |  FAIL: ${fail}  |  WARN: ${warn}  |  TOTAL: ${results.length}\n`);

  if (fail > 0) {
    console.log('  FAILURES:');
    results.filter(r => r.status === 'FAIL').forEach(r =>
      console.log(`    [${r.num}] ${r.name}${r.detail ? ' — ' + r.detail : ''}`)
    );
    console.log('');
  }
  if (warn > 0) {
    console.log('  WARNINGS:');
    results.filter(r => r.status === 'WARN').forEach(r =>
      console.log(`    [${r.num}] ${r.name}${r.detail ? ' — ' + r.detail : ''}`)
    );
    console.log('');
  }

  return { pass, fail, warn, total: results.length, results };
}

run().then(summary => {
  process.exit(summary.fail > 0 ? 1 : 0);
}).catch(err => {
  console.error('Fatal:', err);
  process.exit(1);
});
