---
paths:
  - "frontend/**/*.{ts,tsx}"
---

# Frontend Code Standards

## Type Safety

- NEVER use `any` type — use proper interfaces or `unknown` with type guards
- If a type is generated from Rust (via ts-rs), import from `shared/types`
- After modifying Rust structs with `#[derive(TS)]`, run `npm run generate-types`
- Prefer `satisfies` over `as` for type assertions

## Component Architecture

- Maximum component file size: 500 lines. If larger, extract sub-components
- Each component file should export ONE primary component
- Co-locate sub-components in a directory (e.g., `sidebar/Sidebar.tsx`, `sidebar/OrgSection.tsx`)
- Use barrel exports (`index.tsx`) for backward compatibility when splitting files

## Error Handling

- Wrap major page sections in `<ErrorBoundary>` — never let one tab crash the whole page
- All `useQuery` calls should handle error state in the UI
- All `useEffect` with async operations must have cleanup (AbortController or return fn)
- Never silently swallow errors — at minimum `console.error()`, preferably user-visible feedback

## State Management

- Use React Query for server state (API data)
- Use Zustand stores for client state (UI preferences, expand/collapse)
- Do NOT mix — don't put API data in Zustand, don't put UI state in React Query
- Use `useExpandable` store for any collapsible/expandable UI that should persist across navigation

## Performance

- Only add `useMemo`/`useCallback` when there's a measured performance problem or an expensive computation
- Use `React.lazy()` for tabs within large pages — don't load all tab content eagerly
- Keep dependencies array in hooks accurate — use ESLint `exhaustive-deps` rule

## Constants

- No magic strings or numbers inline — extract to constants files
- Colors, timeouts, batch sizes, and validation limits should be in `src/constants/`
