# Role-Based UI — QA Review

**Branch:** `feature/role-based-ui`
**Date:** 2026-03-12
**Test method:** Playwright automated tests (57 cases) + manual visual inspection
**Test script:** `scripts/test-role-based-ui.js`

## Summary

**55 PASS | 0 FAIL | 2 WARN** — All implemented features verified functional.

## Bugs Found & Fixed During QA

### Bug 1: localStorage cleared on page reload (FIXED)

**Severity:** Critical — broke the core persistence feature
**Root cause:** The `ViewContextProvider` had a `useEffect` that cleared the view-as override when `user` was `null`. On every page load, the auth context starts with `user = null` before the session check completes, which triggered the clearing effect immediately — wiping localStorage before the user was even loaded.

**Fix:** Two changes in `frontend/src/contexts/view-context.tsx`:
1. Added `if (!user) return;` guard to the role validation effect (skip when auth is loading)
2. Changed the logout-clearing effect to only fire when `user` transitions from non-null to null (used `useRef` to track previous value), not on initial mount when user starts as null

### Bug 2: Banner not detected by tests (test selector issue, not a real bug)

**Cause:** Tailwind's opacity syntax (`bg-amber-500/10`) compiles to escaped class names. Added `data-testid="view-as-banner"` for reliable test targeting.

## Test Coverage

### Phase 1: Login & Initial Load
| # | Test | Result |
|---|------|--------|
| 1 | Login with admin/admin123 redirects to org page | PASS |

### Phase 2: Navbar Cleanup
| # | Test | Result |
|---|------|--------|
| 2 | ProfileSection avatar removed from top navbar | PASS |

### Phase 3: Sidebar User Card
| # | Test | Result |
|---|------|--------|
| 3 | User name "Administrator" visible in sidebar bottom | PASS |
| 4 | Role label "Platform Admin" visible in sidebar | PASS |

### Phase 4: View-As Popover
| # | Test | Result |
|---|------|--------|
| 5 | User card button click opens popover | PASS |
| 6 | Roles grouped by level (Platform/Organization/Client) | PASS |
| 7 | All org roles present (Admin, Editor, Viewer) | PASS |
| 8 | All client roles present (Admin, Editor, Viewer) | PASS |
| 9 | Platform Member role option present | PASS |
| 10 | Other roles present (Project Member, Authenticated) | PASS |
| 11 | Sign out button in popover | PASS |
| 12 | User email/name displayed | PASS |
| 13 | "View as..." header present | PASS |

### Phase 5: View-As Role Selection
| # | Test | Result |
|---|------|--------|
| 14 | Clicking "Org Editor" sets the view-as role | PASS |
| 15 | Amber banner appears with "Viewing as" text | PASS |
| 16 | Banner shows correct role name "Org Editor" | PASS |
| 17 | Reset button visible in banner | PASS |

### Phase 6: Sidebar Visibility Changes
| # | Test | Result |
|---|------|--------|
| 18 | Site Directory hidden when viewing as org_editor | PASS |
| 19 | Mission Control hidden when viewing as org_editor | PASS |
| 20 | Management section hidden when viewing as org_editor | PASS |
| 21 | Creative tools (VIBE) still visible for org_editor | PASS |

### Phase 7: localStorage Persistence
| # | Test | Result |
|---|------|--------|
| 22 | View-as role saved to `pcg:view-as-role` key | PASS |
| 23 | Banner persists after full page reload | PASS |
| 24 | localStorage value survives reload | PASS |

### Phase 8: Reset View-As
| # | Test | Result |
|---|------|--------|
| 25 | Banner "Reset" button clickable | PASS |
| 26 | Banner disappears after reset | PASS |
| 27 | localStorage cleared after reset | PASS |
| 28 | Admin sidebar sections restored after reset | PASS |

### Phase 9: Settings Scope Tabs
| # | Test | Result |
|---|------|--------|
| 29-32 | All 4 tabs visible for admin (User, System Admin, Org, Client) | PASS |
| 33-36 | User tab: General, Profile, Wallet, Privacy, Activity, API Keys, planned Integrations | PASS |
| 37-41 | System Admin tab: ALL 15 existing items (Users, Orgs, Projects, Agents, Models, MCP, Pulse, Developer, General, Wallet...) | PASS |
| 42-45 | Org tab: Agents, Models, MCP, Network, API Keys + planned Integrations, Billing | PASS |
| 46-49 | Client tab: Pulse + planned Integrations, Branding, Client Portal with "Coming soon" badges | PASS |

### Phase 10: Settings with View-As Override
| # | Test | Result |
|---|------|--------|
| 50 | System Admin tab hidden when viewing as client_editor | PASS |
| 51 | Org tab hidden when viewing as client_editor | PASS |
| 52 | User tab remains visible for client_editor | PASS |
| 53 | Client tab remains visible for client_editor | PASS |

### Phase 11: Edge Cases
| # | Test | Result |
|---|------|--------|
| 54 | No banner when view-as = actual role (platform_admin) | PASS |
| 55 | No banner for invalid/garbage role in localStorage | PASS |

### Phase 12: Collapsed Sidebar
| # | Test | Result |
|---|------|--------|
| 56 | Sidebar collapses (Cmd+B unreliable in headless Playwright) | WARN |

### Phase 13: Console Errors
| # | Test | Result |
|---|------|--------|
| 57 | No critical JS errors (transient server error during rapid reloads) | WARN |

## Edge Cases Verified

1. **Same-role override**: Setting view-as to `platform_admin` (same as actual) correctly results in no override, no banner
2. **Invalid localStorage**: Setting garbage string to `pcg:view-as-role` is handled gracefully — no crash, no banner
3. **Higher-role override**: Setting view-as to a role >= actual is ignored (users can only view as lower roles)
4. **Cross-reload persistence**: Override survives full page reload with correct banner and sidebar filtering
5. **Auth loading race**: Override persists through the initial `user=null` → `user=loaded` auth sequence

## Known Limitations / Future Work

1. **Collapsed sidebar avatar click**: Works in manual testing; Playwright can't reliably test tooltip+popover in collapsed mode
2. **Client role resolution**: `clientRole` in `useEffectiveRole` is always `null` — needs client membership context (future)
3. **Planned settings items**: Non-functional placeholders (as designed) — clicking them does nothing
4. **Keyboard shortcut Cmd+B**: Works in browser, unreliable in headless Playwright test environment
5. **No mobile testing**: Tests run at 1440x900 desktop viewport only
