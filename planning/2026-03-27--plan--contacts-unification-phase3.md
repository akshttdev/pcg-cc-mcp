# Phase 3: Intake Pipeline + Frontend Merge

**Date**: 2026-03-27
**Branch**: `feature/contacts-unification-phase3`
**Base**: Phase 2 branch (or main after Phase 2 merges)
**Duration**: 2 days
**Depends on**: Phase 2 (child tables re-FK'd, intelligence routes on contacts)
**Unblocks**: Intake pipeline creates contacts directly, unified person/contact detail page

---

## Goal

Update the intake pipeline to create contacts (not persons) and merge the person detail page into the contact detail view — social profiles, company roles, org associations, intelligence, research passes all visible.

---

## W1: Intake pipeline — create contacts directly

**`crates/server/src/routes/intake/pipeline.rs`**:
- `associate_participants()` currently: `INSERT INTO persons (...) VALUES (...)`
- Change to: `INSERT INTO crm_contacts (...) VALUES (...)`
- Set `intelligence_status = 'idle'` on the contact
- Set `source = 'intake'` on the contact
- Link to organization and company same as before
- Remove person creation code path

**`crates/server/src/routes/intake/report.rs`**:
- Update any person references to use contacts

### Files
| File | Action |
|------|--------|
| `crates/server/src/routes/intake/pipeline.rs` | MODIFY — create contacts instead of persons |
| `crates/server/src/routes/intake/report.rs` | MODIFY — use contacts |

---

## W2: Frontend — merge person detail into contact detail

**Current state**:
- `/people/:id` page shows person profile with social profiles, company roles, org associations, intelligence
- CRM contact detail (deal panel) shows only basic contact info + intel from persons JOIN

**Target**:
- CRM contact detail shows everything the person page shows
- `/people/:id` redirects to contact detail (or reads from contacts)

**Frontend changes**:

1. **Contact detail component** (in deal-detail or contacts page):
   - Add social profiles section (fetch from contact_social_profiles)
   - Add company roles section
   - Add research passes section (fetch from contact_research_passes)
   - Add intelligence section (already done in Phase 1 via Intel tab)

2. **IntelTab.tsx** updates:
   - "Trigger Research" button targets `POST /api/crm/contacts/:id/research`
   - Research passes list fetched from `contact_research_passes`
   - Remove person_id dependency

3. **Route redirect**:
   - `/people/:id` → look up contact by person_id, redirect to contact view

### Files
| File | Action |
|------|--------|
| `frontend/src/components/crm/deal-detail/tabs/IntelTab.tsx` | MODIFY — target contacts API |
| `frontend/src/components/crm/deal-detail/tabs/ReviewTab.tsx` | MODIFY — person links → contact links |
| `frontend/src/pages/people/` | MODIFY — redirect to contacts or read from contacts |
| `frontend/src/lib/api/crm.ts` | MODIFY — add intelligence API methods |

---

## W3: CRM contacts page — add intelligence column

**`frontend/src/pages/` (contacts list page)**:
- Add intelligence_status column to contacts table
- Show research badge on contacts with completed research
- "Trigger Research" action in contact row menu

### Files
| File | Action |
|------|--------|
| Contacts list page component | MODIFY — add intel status column |

---

## Verification

1. `npx tsc --noEmit` — 0 errors
2. Intake flow: process a call intake → verify contact created (not person)
3. Open contact in deal detail → verify social profiles, research passes visible
4. Navigate to `/people/:id` → verify redirect to contact view
5. MCP walkthrough: full pipeline from intake → deal → Intel tab → research


---

## Status: IMPLEMENTED (2026-03-27)

Commits: `800de2fe0` (W1), `8bf10a2cb` (W2). Functionality audit pending.
