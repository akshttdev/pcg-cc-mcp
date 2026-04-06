# Phase 4: Retire Persons Table + User Profiles

**Date**: 2026-03-27
**Branch**: `feature/contacts-unification-phase4`
**Base**: Phase 3 branch (or main after Phase 3 merges)
**Duration**: 1.5 days
**Depends on**: Phase 3 (intake creates contacts, frontend merged)
**Unblocks**: Single canonical people table, clean data model

---

## Goal

Create a thin `user_profiles` table for platform user identity, migrate remaining persons-only fields to contacts custom_fields, drop the persons table.

---

## W1: user_profiles table

**Migration `20260327000003_user_profiles.sql`**:
```sql
CREATE TABLE user_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    crm_contact_id TEXT REFERENCES crm_contacts(id),
    display_name TEXT,
    role TEXT DEFAULT 'member',
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(user_id)
);

-- Backfill from persons where user_id is set
INSERT INTO user_profiles (id, user_id, crm_contact_id, display_name, created_at, updated_at)
SELECT id, user_id, crm_contact_id, full_name, created_at, updated_at
FROM persons
WHERE user_id IS NOT NULL;
```

### Files
| File | Action |
|------|--------|
| `crates/db/migrations/20260327000003_user_profiles.sql` | CREATE |
| `crates/db/src/models/user_profile.rs` | CREATE — simple CRUD model |

---

## W2: Migrate persons-only fields to contacts custom_fields

**person_type, financial_role, client_profile, business_stage** → stored in `crm_contacts.custom_fields` JSON:

```sql
UPDATE crm_contacts SET custom_fields = json_set(
    COALESCE(custom_fields, '{}'),
    '$.person_type', (SELECT person_type FROM persons WHERE crm_contact_id = crm_contacts.id),
    '$.financial_role', (SELECT financial_role FROM persons WHERE crm_contact_id = crm_contacts.id),
    '$.client_profile', (SELECT client_profile FROM persons WHERE crm_contact_id = crm_contacts.id),
    '$.business_stage', (SELECT business_stage FROM persons WHERE crm_contact_id = crm_contacts.id)
)
WHERE id IN (SELECT crm_contact_id FROM persons WHERE crm_contact_id IS NOT NULL);
```

---

## W3: Drop persons table

**Migration `20260327000004_drop_persons.sql`**:
```sql
-- Final cleanup: verify no remaining FK references
-- Drop person-specific tables that have been re-FK'd
DROP TABLE IF EXISTS person_notes;

-- Drop persons table
DROP TABLE IF EXISTS persons;
```

**Code cleanup**:
- Remove `crates/db/src/models/person.rs` (or mark deprecated)
- Remove `crates/server/src/routes/persons.rs` (or redirect to contacts)
- Remove persons-related query methods from other models
- Update `crates/db/src/models/mod.rs` to remove person module

### Files
| File | Action |
|------|--------|
| `crates/db/migrations/20260327000004_drop_persons.sql` | CREATE |
| `crates/db/src/models/person.rs` | DELETE or deprecate |
| `crates/server/src/routes/persons.rs` | DELETE or redirect |
| `crates/db/src/models/mod.rs` | MODIFY — remove person module |
| `crates/server/src/routes/mod.rs` | MODIFY — remove persons routes |

---

## W4: Update remaining references

Search and replace across the codebase:
- `persons.intelligence_summary` → `crm_contacts.intelligence_summary`
- `Person::find_by_id` → `CrmContact::find_by_id`
- `person_id` references that pointed to persons table → point to crm_contacts or user_profiles
- Update any remaining JOIN to persons table

### Files
| File | Action |
|------|--------|
| All files with `use.*person` imports | MODIFY — update imports |
| `crates/server/src/routes/sidebar.rs` | MODIFY — read from contacts |
| `crates/server/src/routes/invite_dispatch.rs` | MODIFY — read from contacts |
| `crates/services/src/services/workflow_execution.rs` | MODIFY — read from contacts |

---

## Verification

1. `cargo check --workspace` — 0 errors, no references to persons table
2. `grep -r "persons" crates/ --include="*.rs" | grep -v test | grep -v migration` → zero results
3. Full E2E: create deal → Intel → Scout → review → advance → Won
4. Database: `SELECT COUNT(*) FROM persons` → table doesn't exist
5. All existing functionality still works (intake, pipeline, intelligence)


---

## Status: PARTIALLY IMPLEMENTED (2026-03-27)

Commit: `46c958a13` (user_profiles + custom_fields migration). Persons table NOT dropped — 15+ files still reference it. Tracked as PC-6 in backlog.
