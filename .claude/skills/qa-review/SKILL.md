---
name: qa-review
description: Comprehensive QA review — static checks, regression analysis, and browser smoke tests
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, Agent, mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_console_messages, mcp__playwright__browser_take_screenshot
---

# QA Review

Run a comprehensive quality assurance pass before merging. Execute in this order:

## 1. Static Checks (run /check first)

Run the `/check` skill to verify tsc, lint, clippy, and format all pass.

## 2. Regression Analysis

Enumerate all changed files:
```bash
git diff main --stat
```

For each changed file, check for:
- **Broken imports**: grep for old import paths that reference moved/renamed files
- **Stale references**: hardcoded paths in Makefile, Dockerfile, package.json, CLAUDE.md that should have been updated
- **Missing barrel exports**: after file splits, verify `index.tsx` exports all public components
- **Conflict markers**: `grep -r "<<<<<<" frontend/src/` to catch unresolved merges

Report findings as:
| Category | File | Issue |
|----------|------|-------|

## 3. Playwright MCP Smoke Tests

Start dev servers if not already running:
```bash
pnpm run dev
```

Then use Playwright MCP to navigate to each affected page and verify:
- Page loads without blank screen
- No console errors (check via browser_console_messages)
- Key interactive elements are present (take snapshot to verify)

Default pages to test (if no specific pages affected):
1. `/login` — login page loads
2. `/` — dashboard loads after login
3. `/settings` — settings page loads
4. `/workflows` — workflows page loads

For each page, report:
| Page | Status | Console Errors | Notes |
|------|--------|---------------|-------|

## 4. Summary

Produce a final pass/fail summary:
- Static checks: PASS/FAIL
- Regression analysis: N issues found
- Smoke tests: N/N pages passed
- **Overall**: READY TO MERGE / NEEDS FIXES
