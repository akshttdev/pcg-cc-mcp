# Role-Based UI: View-As Context Switcher & Settings Scoping

**Branch:** `feature/role-based-ui` (from `origin/feature/blob-to-text-scoped`)
**Status:** COMPLETE — implemented (view-as switcher, role hierarchy in `lib/roles.ts`, settings scoping)

## Problem

The app has a multi-layer role system (Platform → Org → Client → Project) but the frontend only partially surfaces it. Admins can't preview lower-role views, settings use flat `adminOnly` flags, and some settings naturally span multiple scopes. The editorial scope tiers (admin/editor/viewer) at org and client levels aren't represented in the UI role hierarchy.

## Design Decisions

- **Three editorial tiers** at org/client: admin (invite + edit), editor (edit), viewer (read-only)
- **Permission types defined now** as TS interfaces — architecture ready for granular RBAC, no backend
- **Grouped view-as switcher** (Platform / Org / Client sections)
- **Collapsed sidebar**: avatar click opens popover directly

## Role Hierarchy

```
platform_admin(11) > platform_member(10)
  > org_admin(9) > org_editor(8) > org_viewer(7)
    > client_admin(6) > client_editor(5) > client_viewer(4)
      > project_member(3) > authenticated(1)
```

| Level | Roles | Editorial Scope |
|-------|-------|-----------------|
| Platform | admin, member | Full control / operational access |
| Org | admin, editor, viewer | Invite+edit / edit / read-only |
| Client | admin, editor, viewer | Invite+edit / edit / read-only |
| Project | owner, admin, editor, viewer | Existing — no changes |

## Settings Tabs

- **User** (always): General, Profile, Wallet, Privacy, Activity, API Keys, `(planned)` Personal Integrations
- **System Admin** (platform admin): ALL 15 existing items preserved
- **Org** (org admin/editor): Agents, Models, MCP, API Keys, Network, `(planned)` Org Integrations/Billing
- **Client** (client admin/editor): Pulse, `(planned)` Client Integrations/Branding/Portal

## Implementation (10 Steps)

1. `frontend/src/lib/roles.ts` — Role types, hierarchy, Permission type + RolePermissionMap
2. `useEffectiveRole.ts` — Expand to 10 roles, add client role, override support
3. `frontend/src/contexts/view-context.tsx` — View-as state, localStorage, grouped available roles
4. `App.tsx` — Wire ViewContextProvider
5. `SidebarUserCard.tsx` — Avatar + role badge + grouped view-as popover + sign out
6. `sidebar.tsx` — Insert user card at bottom
7. `navbar.tsx` — Remove ProfileSection
8. `SettingsLayout.tsx` — Scope tabs, multi-scope items, planned placeholders
9. `ViewAsBanner.tsx` — Override indicator banner
10. Cleanup — RoleRoute refactor, verify all sections respond to view-as

## Schema Changes (Document Only)

- `user_platform_roles`: alias `platform_admin`→`admin`, `operator`→`member`
- `organization_members`: rename `member`→`editor` (align with editorial scope)
- Update TS type generation bindings

## Future: Granular RBAC

- Permission enforcement middleware (backend)
- `hasPermission()` replacing capability booleans
- Custom roles with configurable permission sets
- Per-resource permission overrides
