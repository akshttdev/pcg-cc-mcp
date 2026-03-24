# Frontend Implementation Standards

These are implementation-level patterns and conventions for the frontend codebase. For universal principles (type safety, component architecture, state management, styling, testability), see `.claude/rules/frontend-standards.md`.

## React Query Patterns

- Configure `staleTime` appropriately (default: 5 minutes via QueryClient defaults)
- **NEVER use inline query keys** like `queryKey: ['tasks', id]` — use factory functions from `lib/query-keys.ts` (e.g., `taskKeys.detail(id)`)
- Invalidate related queries on mutation success: `queryClient.invalidateQueries({ queryKey: [...] })`
- Use optimistic updates via `queryClient.setQueryData()` for responsive UI
- Always handle `error` state in the component render — never show blank on failure
- **Query keys**: Always use factories from `lib/query-keys.ts` — never inline string arrays. Mismatched keys cause silent cache invalidation bugs (mutations invalidate one key, queries use another → stale UI).
- **Mutations**: Prefer `useMutationWithToast` over raw `useMutation` for standard CRUD operations. Use raw `useMutation` only for complex cases (optimistic updates, conditional toasts, `setQueryData`).

## API Client

- All HTTP requests go through `frontend/src/lib/api/` domain modules (e.g., `import { tasksApi } from '@/lib/api'`)
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

## Post-Feature QA

- After completing feature work, run `/review-frontend` on changed files
- Run `/playwright-smoke <affected-pages>` to verify rendering
- If feature involves mutations: verify toast appears + cache invalidation works
