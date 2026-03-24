---
paths:
  - "frontend/**/*.{ts,tsx}"
---

# Frontend Code Standards

## Type Safety

- NEVER use `any` type — use proper interfaces or `unknown` with type guards
- Import types from `shared/types` when generated from Rust (via ts-rs)
- After modifying Rust structs with `#[derive(TS)]`, run `npm run generate-types`
- Prefer `satisfies` over `as` for type assertions
- Define component props as `interface ComponentProps { }` — not inline object types

## Component Architecture

- Maximum component file size: 500 lines. If larger, extract sub-components into a directory
- Each component file should export ONE primary component
- Co-locate sub-components in a directory (e.g., `sidebar/Sidebar.tsx`, `sidebar/OrgSection.tsx`)
- Use barrel exports (`index.tsx`) for backward compatibility when splitting files
- Functional components only — no class components

## State Management

- **React Query** for server state (API data) — all fetching/caching through `useQuery`/`useMutation`
- **Zustand stores** for client state (UI preferences, expand/collapse, selection, filters)
- Do NOT mix — don't put API data in Zustand, don't put UI state in React Query
- Use `useExpandable` store for any collapsible/expandable UI that should persist across navigation
- Use `persist` middleware for Zustand stores that should survive page refresh

## React Query Patterns

- Configure `staleTime` appropriately (default: 5 minutes via QueryClient defaults)
- **NEVER use inline query keys** like `queryKey: ['tasks', id]` — use factory functions from `lib/query-keys.ts` (e.g., `taskKeys.detail(id)`)
- Invalidate related queries on mutation success: `queryClient.invalidateQueries({ queryKey: [...] })`
- Use optimistic updates via `queryClient.setQueryData()` for responsive UI
- Always handle `error` state in the component render — never show blank on failure
- **Query keys**: Always use factories from `lib/query-keys.ts` — never inline string arrays. Mismatched keys cause silent cache invalidation bugs (mutations invalidate one key, queries use another → stale UI).
- **Mutations**: Prefer `useMutationWithToast` over raw `useMutation` for standard CRUD operations. Use raw `useMutation` only for complex cases (optimistic updates, conditional toasts, `setQueryData`).

## API Client

- All HTTP requests go through `lib/api.ts` domain modules (e.g., `tasksApi.create()`)
- Never use raw `fetch()` directly — use `makeRequest()` from the API client
- API methods return typed promises: `async (data: CreateTask): Promise<Task>`
- API errors include status, endpoint, and timestamp in logs

## Dialogs & Modals

- Use NiceModal (`@ebay/nice-modal-react`) for all dialogs
- Register dialogs in `main.tsx` with `NiceModal.register('dialog-name', Component)`
- Dialog components export via `NiceModal.create<PropsType>()`
- Close with `modal.hide()`, return data with `modal.resolve(data)`
- Always wrap `NiceModal.show()` in try/catch — dismiss throws

## Routing

- Use `React.lazy()` for all page-level components — never import pages eagerly
- Wrap lazy pages in `<Suspense fallback={<PageLoader />}>`
- Use `ProtectedRoute`, `AdminRoute`, `RoleRoute` HOCs for access control
- URL patterns: `/projects/:projectId/tasks`, `/organizations/:orgId/members`, etc.

## Error Handling

- Wrap major page sections in `<ErrorBoundary>` — never let one tab crash the whole page
- All `useEffect` with async operations must have cleanup (AbortController or return fn)
- All WebSocket/SSE hooks must clean up connections in the useEffect return function
- Never silently swallow errors — at minimum `console.error()`, preferably toast via `sonner`
- **Role-scoped error messages**: Platform/admin users see system details (service names, config); end users see actionable guidance without platform internals. Never leak infrastructure details (ports, file paths, internal service names) to non-admin users.
- **Actionable errors**: Error messages should tell the user what they can do about it, not just what went wrong. "Workflow failed" → "Workflow failed: model unavailable. Try selecting a different model or contact your administrator."

## Styling

- Use Tailwind CSS utility classes — no custom CSS unless absolutely necessary
- Use `cn()` from `@/lib/utils` for conditional/merged class names
- Use shadcn/ui components from `@/components/ui/` — don't create custom primitives
- Icons from `lucide-react` — import only the icons you need (tree-shake)
- Support dark mode with `dark:` prefix variants

## Performance

- Only add `useMemo`/`useCallback` when there's a measured performance problem or expensive computation
- Memoize context provider values with `useMemo` to prevent unnecessary re-renders
- Keep `useEffect` dependency arrays accurate — use ESLint `exhaustive-deps` rule

## Modularity & Functional Design

- Prefer pure functions for data transformations — no side effects, easy to test
- Extract business logic into custom hooks — components should focus on rendering
- Keep components small and composable: one component = one responsibility
- Extract repeated UI patterns into shared components in `components/ui/`
- Extract repeated data logic into custom hooks in `hooks/`
- Use composition over configuration — pass children/render props instead of adding boolean flags
- Prefer `.map()`, `.filter()`, `.reduce()` over imperative loops for data transformations
- Avoid deeply nested ternaries — extract to helper functions or early returns
- Co-locate related code: hook + component + types in the same directory
- Split pages into container (data fetching) + presentational (rendering) when complex

## Constants

- No magic strings or numbers inline — extract to constants files
- Colors, timeouts, batch sizes, and validation limits should be in named constants
- Task status values: `todo`, `inprogress`, `inreview`, `done`, `cancelled` (no underscores)

## Testability

- Every interactive element MUST have a `data-testid` attribute — buttons, menu triggers, cards, form containers
- **Testid helpers are centralized in `shared/testids.ts`** — this is the single source of truth used by both frontend components and E2E tests
- Components import helpers: `import { deck as tid } from 'shared/testids'` then use `data-testid={tid.sendInvoice}`
- **NEVER hardcode testid strings** in components or specs — always use the helpers from `shared/testids.ts`
- Naming convention: `{feature}-{element}` or `{feature}-{element}-{qualifier}` (e.g., `pipeline-add-deal`, `deal-card-{id}`, `deal-menu-{id}`)
- When adding a new testid: add the helper to `shared/testids.ts` first, then import it in both the component and the spec
- Custom wrapper components (KanbanCard, ListItem, EmptyState, etc.) MUST accept and forward `data-testid` as a prop
- If a test needs a fragile CSS/positional selector to reach an element, that's a component bug — add a testid
- Add testids when building components, not retroactively when writing tests

## Post-Feature QA

- After completing feature work, run `/review-frontend` on changed files
- Run `/playwright-smoke <affected-pages>` to verify rendering
- If feature involves mutations: verify toast appears + cache invalidation works
