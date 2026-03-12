/**
 * Responsive UI review — captures screenshots at multiple viewports
 * and checks for layout issues.
 */
const { chromium } = require('playwright');

const BASE = 'http://127.0.0.1:3200';
const VIEWPORTS = [
  { name: 'mobile-sm', width: 375, height: 812 },   // iPhone SE
  { name: 'mobile-lg', width: 428, height: 926 },   // iPhone 14 Pro Max
  { name: 'tablet', width: 768, height: 1024 },      // iPad
  { name: 'laptop', width: 1280, height: 800 },      // 13" laptop
  { name: 'desktop', width: 1440, height: 900 },     // Standard desktop
  { name: 'wide', width: 1920, height: 1080 },       // Full HD
];

const issues = [];

function issue(severity, viewport, component, detail) {
  const icon = severity === 'critical' ? '🔴' : severity === 'major' ? '🟠' : '🟡';
  console.log(`${icon} [${viewport}] ${component}: ${detail}`);
  issues.push({ severity, viewport, component, detail });
}

async function run() {
  const browser = await chromium.launch({ headless: true });

  for (const vp of VIEWPORTS) {
    console.log(`\n═══ ${vp.name} (${vp.width}x${vp.height}) ═══\n`);
    const page = await browser.newPage({ viewport: { width: vp.width, height: vp.height } });

    // Login
    await page.goto(`${BASE}/login`, { waitUntil: 'networkidle', timeout: 15000 });
    await page.fill('input[type="text"]', 'admin');
    await page.fill('input[type="password"]', 'admin123');
    await page.click('button[type="submit"]');
    await page.waitForURL('**/*', { timeout: 10000 });
    await page.waitForTimeout(3000);

    // --- Main page checks ---

    // 1. Sidebar visibility
    const sidebarInfo = await page.evaluate(() => {
      const sidebar = document.querySelector('.sidebar-container');
      if (!sidebar) return { visible: false, width: 0 };
      const rect = sidebar.getBoundingClientRect();
      const style = window.getComputedStyle(sidebar);
      return {
        visible: rect.width > 0 && style.display !== 'none',
        width: rect.width,
        overflow: style.overflow,
      };
    });
    console.log(`  Sidebar: visible=${sidebarInfo.visible}, width=${sidebarInfo.width}`);

    if (vp.width < 1024 && sidebarInfo.visible && sidebarInfo.width > 100) {
      issue('major', vp.name, 'Sidebar', `Sidebar visible on mobile/tablet (${sidebarInfo.width}px wide) — should be hidden`);
    }

    // 2. User card in sidebar bottom
    const userCardInfo = await page.evaluate(() => {
      const sidebar = document.querySelector('.sidebar-container');
      if (!sidebar) return { found: false };
      const text = sidebar.textContent;
      const hasUserCard = text.includes('Platform Admin') || text.includes('Administrator');
      // Check for text overflow
      const buttons = sidebar.querySelectorAll('button');
      let overflowing = false;
      buttons.forEach(b => {
        if (b.scrollWidth > b.clientWidth + 2) overflowing = true;
      });
      return { found: hasUserCard, overflowing };
    });
    if (sidebarInfo.visible) {
      console.log(`  UserCard: found=${userCardInfo.found}, overflow=${userCardInfo.overflowing}`);
      if (userCardInfo.overflowing) {
        issue('minor', vp.name, 'SidebarUserCard', 'Text overflow detected in sidebar buttons');
      }
    }

    // 3. Navbar checks
    const navbarInfo = await page.evaluate(() => {
      const navbar = document.querySelector('.sticky.top-0');
      if (!navbar) return { height: 0, hasAvatar: false };
      return {
        height: navbar.getBoundingClientRect().height,
        hasAvatar: navbar.querySelectorAll('[class*="avatar"], [class*="Avatar"]').length > 0,
        overflow: navbar.scrollWidth > navbar.clientWidth,
      };
    });
    console.log(`  Navbar: height=${navbarInfo.height}, hasAvatar=${navbarInfo.hasAvatar}, overflow=${navbarInfo.overflow}`);
    if (navbarInfo.hasAvatar) {
      issue('major', vp.name, 'Navbar', 'ProfileSection avatar still present in navbar');
    }
    if (navbarInfo.overflow) {
      issue('minor', vp.name, 'Navbar', 'Horizontal overflow in navbar');
    }

    // 4. ViewAs banner test
    await page.evaluate(() => localStorage.setItem('pcg:view-as-role', 'org_editor'));
    await page.reload({ waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(3000);

    const bannerInfo = await page.evaluate(() => {
      const banner = document.querySelector('[data-testid="view-as-banner"]');
      if (!banner) return { visible: false };
      const rect = banner.getBoundingClientRect();
      return {
        visible: rect.height > 0,
        height: rect.height,
        textTruncated: banner.scrollWidth > banner.clientWidth,
        fullyVisible: rect.right <= window.innerWidth,
      };
    });
    console.log(`  Banner: visible=${bannerInfo.visible}, height=${bannerInfo.height}, truncated=${bannerInfo.textTruncated}`);
    if (bannerInfo.visible && !bannerInfo.fullyVisible) {
      issue('minor', vp.name, 'ViewAsBanner', 'Banner extends beyond viewport');
    }

    await page.screenshot({ path: `/tmp/role-ui-${vp.name}-banner.png`, fullPage: false });

    // Clean up
    await page.evaluate(() => localStorage.removeItem('pcg:view-as-role'));

    // 5. Settings page
    await page.goto(`${BASE}/settings/general`, { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);

    const settingsInfo = await page.evaluate(() => {
      const aside = document.querySelector('aside');
      const main = document.querySelector('main');
      if (!aside || !main) return { layout: 'unknown' };

      const asideRect = aside.getBoundingClientRect();
      const mainRect = main.getBoundingClientRect();

      // Check if side-by-side or stacked
      const isSideBySide = asideRect.right < mainRect.left + 10;

      // Check tab bar
      const buttons = aside.querySelectorAll('button');
      const tabButtons = Array.from(buttons).filter(b => {
        const t = b.textContent.trim();
        return ['User', 'System Admin', 'Organization', 'Client'].includes(t);
      });

      // Check tab overflow
      const tabContainer = tabButtons[0]?.parentElement;
      const tabOverflow = tabContainer ? tabContainer.scrollWidth > tabContainer.clientWidth + 2 : false;

      return {
        layout: isSideBySide ? 'side-by-side' : 'stacked',
        asideWidth: asideRect.width,
        tabCount: tabButtons.length,
        tabOverflow,
      };
    });
    console.log(`  Settings: layout=${settingsInfo.layout}, aside=${settingsInfo.asideWidth}px, tabs=${settingsInfo.tabCount}, tabOverflow=${settingsInfo.tabOverflow}`);

    if (settingsInfo.tabOverflow) {
      issue('major', vp.name, 'SettingsTabs', 'Tab bar overflows its container');
    }

    await page.screenshot({ path: `/tmp/role-ui-${vp.name}-settings.png`, fullPage: false });

    // 6. Settings with view-as popover open (check popover positioning)
    // Open sidebar on mobile first
    if (vp.width < 1024) {
      const menuBtn = await page.evaluate(() => {
        const btn = document.querySelector('button[aria-label="Toggle navigation"]');
        if (btn) { btn.click(); return true; }
        return false;
      });
      if (menuBtn) await page.waitForTimeout(500);
    }

    // Try to open popover
    const popoverOpened = await page.evaluate(() => {
      const sidebar = document.querySelector('.sidebar-container');
      if (!sidebar) return false;
      const buttons = sidebar.querySelectorAll('button');
      const userBtn = Array.from(buttons).find(b => b.textContent.includes('Platform Admin') || b.textContent.includes('Administrator'));
      if (userBtn) { userBtn.click(); return true; }
      return false;
    });
    await page.waitForTimeout(500);

    if (popoverOpened) {
      const popoverPos = await page.evaluate(() => {
        const popovers = document.querySelectorAll('[data-radix-popper-content-wrapper]');
        const last = popovers[popovers.length - 1];
        if (!last) return null;
        const rect = last.getBoundingClientRect();
        return {
          top: rect.top,
          left: rect.left,
          right: rect.right,
          bottom: rect.bottom,
          width: rect.width,
          offscreen: rect.right > window.innerWidth || rect.bottom > window.innerHeight || rect.left < 0 || rect.top < 0,
        };
      });
      if (popoverPos) {
        console.log(`  Popover: ${popoverPos.width}px wide, offscreen=${popoverPos.offscreen}`);
        if (popoverPos.offscreen) {
          issue('major', vp.name, 'ViewAsPopover', `Popover extends offscreen (left=${popoverPos.left}, right=${popoverPos.right}, bottom=${popoverPos.bottom})`);
        }
        await page.screenshot({ path: `/tmp/role-ui-${vp.name}-popover.png`, fullPage: false });
      }
    }

    await page.close();
  }

  await browser.close();

  // Summary
  console.log('\n═══════════════════════════════════════');
  console.log('       RESPONSIVE REVIEW SUMMARY');
  console.log('═══════════════════════════════════════\n');

  const critical = issues.filter(i => i.severity === 'critical');
  const major = issues.filter(i => i.severity === 'major');
  const minor = issues.filter(i => i.severity === 'minor');

  console.log(`  Critical: ${critical.length} | Major: ${major.length} | Minor: ${minor.length}\n`);

  if (issues.length === 0) {
    console.log('  No issues found! ✨');
  } else {
    issues.forEach(i => {
      const icon = i.severity === 'critical' ? '🔴' : i.severity === 'major' ? '🟠' : '🟡';
      console.log(`  ${icon} [${i.viewport}] ${i.component}: ${i.detail}`);
    });
  }
}

run().catch(e => { console.error(e); process.exit(1); });
