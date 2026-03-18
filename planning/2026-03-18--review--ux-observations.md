# UX Review — Sloperation317 Integration Sprint

**Date**: 2026-03-18
**Reviewer**: Claude (Playwright MCP, Firefox)
**Servers**: Frontend :3010, Backend :3012
**Branch**: `integration/sloperation317-quality`

---

## E1. CRM Pipeline

### Pipeline page (Powerclub Global / Sirak Studios)
- **Screenshot**: `ux-review-02-crm-pipeline.png`, `ux-review-03-sirak-pipeline.png`
- **Status**: "Pipeline not found" on both orgs
- **Root cause**: Dev DB lacks pipeline stage data — the BLOB→TEXT migration (20260406) and dealflow pipeline migration (20260408) haven't fully run on the dev DB due to missing column dependencies
- **Breadcrumbs**: Working correctly (Home > Org > CRM > Pipeline)
- **Tabs**: All present (Overview, Pipelines, Contacts, Projects, Social, Intelligence, Wiki, Members, Integrations, Cloud)
- **Org header**: Stats display correctly (Projects, Clients, Members, Pipeline value)

#### Issues
| ID | Severity | Description | Category |
|----|----------|-------------|----------|
| E1-1 | **BLOCKING** | "Pipeline not found" message is not actionable — doesn't tell user what to do | In-sprint fix |
| E1-2 | MEDIUM | Pipeline Acquisition/Lifecycle tabs show but no pipeline exists — confusing | Next sprint |
| E1-3 | LOW | Pipeline value shows "$0" even when no pipeline exists | Next sprint |

#### Recommendation for E1-1
Change "Pipeline not found" to: "No pipeline configured for this organization. Create a pipeline in the CRM settings to start tracking deals." with a CTA button.

---

## E2. Company Profiles & Navigation

### Companies page (empty state)
- **Screenshot**: `ux-review-04-companies-empty.png`
- **Status**: Good empty state with icon, description, and "New Company" CTA
- **Breadcrumbs**: Working (Home > Sirak Studios > CRM > Companies)

### Company brand guide (invalid ID)
- **Screenshot**: `ux-review-05-brand-guide-loading.png`
- **Status**: Full black screen with infinite loading spinner

#### Issues
| ID | Severity | Description | Category |
|----|----------|-------------|----------|
| E2-1 | **BLOCKING** | Brand guide with invalid company ID shows black screen + infinite spinner instead of error state | In-sprint fix |
| E2-2 | MEDIUM | 6+ API errors in console when company not found — no user-facing error message | In-sprint fix |
| E2-3 | LOW | Brand guide route shows breadcrumb "Companies" but isn't linked from companies list page | Next sprint |

#### Recommendation for E2-1
Add error handling in CompanyBrandGuidePage: if `company` query returns null/error, show "Company not found" with back button instead of black loading screen.

---

## E3. Sidebar & Routing

### Observations
- **Screenshot**: `ux-review-01-home.png`
- Sidebar correctly shows all organizations with CRM sub-items
- CRM submenu expands with: Overview, Contacts, Companies, Pipeline, Deliverables
- Active route highlighting works on Pipeline link
- Org scope indicator ("Org: Sirak Studios") appears in navbar when viewing org pages
- Topsi widget shows connection status (Connected vs Disconnected depending on org context)

#### Issues
| ID | Severity | Description | Category |
|----|----------|-------------|----------|
| E3-1 | LOW | Sidebar doesn't show which org section the user is currently viewing when scrolled — org name could be offscreen | Next sprint |
| E3-2 | LOW | "Org: X" pill in top bar disappears on non-org pages — could be persistent for context | Next sprint |

---

## Summary

| Category | Count |
|----------|-------|
| Blocking (in-sprint fix) | 2 |
| Medium (in-sprint if time) | 2 |
| Low (next sprint) | 4 |

### Blocking Fixes Required
1. **E2-1**: CompanyBrandGuidePage error state — replace black screen + spinner with "Company not found"
2. **E1-1**: Pipeline empty state — replace "Pipeline not found" with actionable message + CTA

### Deferred to Backlog
- E1-2: Pipeline tab UX when no pipeline exists
- E1-3: Pipeline $0 display
- E2-3: Brand guide not linked from companies list
- E3-1: Sidebar org context when scrolled
- E3-2: Persistent org scope indicator
