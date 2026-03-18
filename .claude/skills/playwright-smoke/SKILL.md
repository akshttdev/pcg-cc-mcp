---
name: playwright-smoke
description: Smoke test pages via Playwright MCP — navigate, snapshot, check console errors
user-invocable: true
allowed-tools: Bash, mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_console_messages, mcp__playwright__browser_take_screenshot, mcp__playwright__browser_click, mcp__playwright__browser_fill_form, mcp__playwright__browser_wait_for
---

# Playwright Smoke Test

Smoke test application pages using the Playwright MCP browser tools.

## Arguments

`$ARGUMENTS` — comma-separated page paths to test (e.g., `/settings,/workflows,/crm`).

If no arguments provided, test the default critical pages:
- `/login`
- `/` (dashboard/home)
- `/settings`
- `/workflows`

## Prerequisites

Ensure dev servers are running:
```bash
pnpm run dev
```

The app should be accessible at `http://localhost:3000`.

## For each page:

1. **Navigate**: `browser_navigate` to `http://localhost:3000{path}`
2. **Wait**: `browser_wait_for` for the page to settle (network idle or 2s)
3. **Snapshot**: `browser_snapshot` to verify page content rendered
4. **Console**: `browser_console_messages` to check for errors
5. **Screenshot** (optional): `browser_take_screenshot` if issues found

## Login handling

If redirected to `/login`, authenticate first:
1. Fill username field with `admin`
2. Fill password field with `admin123`
3. Click the login/submit button
4. Wait for redirect to dashboard

## Report format

| Page | Status | Console Errors | Notes |
|------|--------|---------------|-------|
| `/login` | OK | 0 | Login form rendered |
| `/settings` | OK | 0 | Settings tabs visible |
| `/workflows` | ERROR | 2 | TypeError in console |

If any page fails, include the error details and a screenshot.
