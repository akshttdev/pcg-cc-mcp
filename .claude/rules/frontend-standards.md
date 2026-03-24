---
paths:
  - "frontend/**/*.{ts,tsx}"
---

# Frontend Code Standards

> Implementation details (React Query patterns, API client, dialogs, routing, error handling, performance, modularity, constants, QA) are in `frontend/STANDARDS.md`.

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

## Styling

- Use Tailwind CSS utility classes — no custom CSS unless absolutely necessary
- Use `cn()` from `@/lib/utils` for conditional/merged class names
- Use shadcn/ui components from `@/components/ui/` — don't create custom primitives
- Icons from `lucide-react` — import only the icons you need (tree-shake)
- Support dark mode with `dark:` prefix variants

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
