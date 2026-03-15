---
name: review-frontend
description: Review frontend changes for any types, missing error boundaries, component size, and platform standard violations
user-invocable: true
allowed-tools: Bash, Read, Grep, Glob, Agent
---

# Frontend Code Review

Review staged or recent frontend changes for platform standard violations.

## What to check

1. **Type safety**: Search changed `.ts`/`.tsx` files for `: any`, `as any`, `<any>`. Flag every instance.

2. **Component size**: Flag any component file over 500 lines. Suggest extraction strategy.

3. **Error handling**: Check for `useQuery` without error state handling. Check for `useEffect` with async operations missing cleanup.

4. **Constants**: Flag hardcoded magic numbers, color hex strings, timeout values, or URL strings that should be constants.

5. **State management**: Flag any API data stored in Zustand (should be React Query) or UI state in React Query (should be Zustand).

## Scope

If `$ARGUMENTS` is provided, review that specific file or directory.
Otherwise, review files changed since the last commit: `git diff HEAD~1 --name-only -- 'frontend/**/*.{ts,tsx}'`

## Output

Report findings as:
```
## Frontend Review: [scope]

### Violations Found
- [ ] `file:line` — description (severity)

### Clean
- All type safety checks passed
```
