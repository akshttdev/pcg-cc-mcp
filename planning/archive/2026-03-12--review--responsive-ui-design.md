# Responsive & UI Design Review — Role-Based UI

**Branch:** `feature/role-based-ui`
**Date:** 2026-03-12
**Method:** Automated Playwright responsive testing (6 viewports) + visual screenshot review + code review
**Test script:** `scripts/review-responsive-ui.js`

## Responsive Test Results

**6 viewports tested:** mobile-sm (375px), mobile-lg (428px), tablet (768px), laptop (1280px), desktop (1440px), wide (1920px)

### After fixes: 0 Critical | 0 Major | 0 Minor

## Bug Found & Fixed

### Settings tab bar overflow (FIXED)

**Severity:** Major — tabs overflowed their 256px aside container at all desktop viewports
**Root cause:** Tab labels "System Admin" and "Organization" were too long for the constrained aside width
**Fix:**
1. Shortened labels: "System Admin" → "Admin", "Organization" → "Org" (`roles.ts:SETTINGS_SCOPE_LABELS`)
2. Added `flex-wrap` and `whitespace-nowrap` to tab container as safety net (`SettingsLayout.tsx`)

## Design Review — Component-by-Component

### SidebarUserCard ✅ Well designed

**Strengths:**
- Clean expanded layout: avatar + name + role label + chevron affordance
- Collapsed mode: avatar-only with tooltip showing name + override status
- Override indicator (amber dot) is visible without being intrusive
- Popover grouping (Platform/Organization/Client/Other) provides clear information hierarchy
- `min-w-0` + `truncate` on name/email prevents text overflow

**Minor observations (not blocking):**
- The double group header labels ("VIEW AS..." section header + tier labels like "PLATFORM") creates slight visual heaviness. The "VIEW AS..." header could potentially be removed since the context is clear from the popover position. This is a taste call.
- Role items have good hover states and clear selection highlighting

### ViewAsBanner ✅ Clean and effective

**Strengths:**
- Slim (37px) — doesn't eat significant vertical space
- Amber color clearly signals "temporary/debug state" (well-established convention)
- Eye icon + bold role name provides instant context
- Reset button is prominent and accessible
- Works perfectly at all viewport sizes including mobile (375px)
- `data-testid` attribute added for reliable test targeting

**Behavior check:**
- Banner correctly renders inside the content area (below navbar, above page content)
- On mobile, banner spans full width with text left-aligned and Reset right-aligned — good

### Settings Scope Tabs ✅ Solid after fix

**Strengths:**
- Tab labels now compact enough for 256px aside: "User | Admin | Org | Client"
- Active tab uses subtle `bg-primary/10` highlight — clear without being loud
- Tab visibility correctly respects effective role
- Planned items clearly dimmed with "Coming soon" badge
- `flex-wrap` safety net prevents future overflow if labels change

**Observations:**
- Tabs are `text-xs` (12px) which is fine for the compact aside but could be bumped to `text-sm` if touch target size becomes an issue on tablet
- On mobile/tablet the settings page stacks aside above main — tabs fit comfortably

### Popover (Radix) ✅ Good positioning

- Desktop: opens above the user card (side="top"), stays within viewport
- Collapsed: opens to the right (side="right"), correct
- Mobile: popover triggered from sidebar overlay, stays within viewport
- 256px fixed width is appropriate for the role list content
- No offscreen issues detected at any viewport

### Sidebar Integration ✅ Clean

- User card sits at the bottom of the sidebar before the collapse toggle — standard placement (VS Code, Slack, etc.)
- Separator before user card provides visual boundary
- On mobile: sidebar is an overlay with backdrop — user card accessible via hamburger → sidebar → bottom
- View-as override correctly filters sidebar sections (Admin Platforms, Management, Global Views hidden for non-admin roles)

### Banner + Navbar + Breadcrumb Stack ✅ No collision

- Vertical stack: DevBanner → Navbar → BreadcrumbNav → ViewAsBanner → Page content
- No overlap or z-index issues
- Total header height reasonable even with all banners showing

## Responsive Behavior Summary

| Component | Mobile (375px) | Tablet (768px) | Laptop (1280px) | Desktop (1440px+) |
|-----------|---------------|----------------|-----------------|-------------------|
| Sidebar | Hidden, overlay on toggle | Hidden, overlay on toggle | Visible, expanded | Visible, expanded |
| User Card | In overlay sidebar | In overlay sidebar | Bottom of sidebar | Bottom of sidebar |
| Popover | Within viewport | Within viewport | Above card | Above card |
| Banner | Full width, compact | Full width, compact | Content area width | Content area width |
| Settings tabs | Stacked, fits well | Stacked, fits well | Side-by-side, no overflow | Side-by-side, no overflow |

## Recommendations for Future Improvement

1. **Mobile user menu access**: On mobile, the user must open sidebar (hamburger) → scroll to bottom → tap user card. Consider adding a user avatar/icon to the navbar for quicker access on mobile. Low priority since mobile is not primary use case.

2. **View-as keyboard shortcut**: Power users would benefit from a keyboard shortcut to toggle the view-as popover (e.g., Cmd+Shift+V). Could wire into the existing keyboard shortcuts system.

3. **Animated transitions**: The sidebar section show/hide when switching roles is instant. A subtle `transition-all` on section height would feel more polished. Very low priority.

4. **Settings tab persistence**: Currently resets to "User" on navigation. Could persist active tab in URL params (`?scope=admin`) for shareable deep links.

## Files Changed in This Review

| File | Change |
|------|--------|
| `frontend/src/lib/roles.ts` | Shortened `SETTINGS_SCOPE_LABELS` ("System Admin" → "Admin", "Organization" → "Org") |
| `frontend/src/pages/settings/SettingsLayout.tsx` | Added `flex-wrap` + `whitespace-nowrap` to tab container |
