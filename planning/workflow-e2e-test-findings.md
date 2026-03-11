# End-to-End Workflow Test: Data Source → Workflow → Staging → CRM

**Date**: 2026-03-11
**Branch**: `feature/fraze-2026-03-11`
**Tested by**: Playwright MCP browser automation

## Test Summary

Tested the full pipeline: create a text data source with meeting notes containing 3 contacts, 3 companies, and 3 deals → run the "CRM Intake Pipeline" workflow → review staged records → commit to CRM → verify in CRM views.

**Overall verdict**: The pipeline works end-to-end. Data flows from source → extraction → staging → CRM. However, there are significant extraction quality issues and several UX gaps.

---

## Pipeline Steps Tested

### 1. Data Source Creation (PASS)
- **Path**: Data Library (`/organizations/:id/data-sources`) → "Add Text" button
- **Input**: Meeting notes with 3 contacts (Rachel Torres, David Kim, Priya Sharma), 3 companies (Meridian Labs, CloudPeak Systems, NexGen Analytics), and 3 deals ($150k, $85k, $200k)
- **Result**: Text source added successfully, appears in file list with "document" type badge
- **Toast notification**: "Text source added" — works correctly

### 2. Workflow Execution (PASS with issues)
- **Path**: My Workflows → Builder tab → "Run workflow" on CRM Intake Pipeline → select data source → Run
- **Result**: Workflow completed in ~40ms, auto-navigated to Staging tab with `?tab=staging&run=<id>` URL
- **Records produced**: 13 total (3 contacts, 7 companies, 3 deals)

### 3. Staging Review (PASS — UI works well)
- **Features tested**:
  - Edit inline fields (first_name, last_name, job_title, company_name, email, phone) — **works**
  - Reject individual records — **works**
  - Bulk "Approve & commit N" — **works**
  - Status badges (pending → committed/rejected) — **works**
  - Confidence scores displayed — **works**
- **Result**: Rejected 4 bad company records, edited 1 contact, approved & committed remaining 9

### 4. CRM Verification (PASS)
- **Contacts**: All 3 new contacts appear in org CRM Contacts tab with "Workflow" import badge
- **Companies**: All 3 new companies appear in Companies sub-view
- **Deals**: All 3 deals appear in Pipeline view (Acquisition Pipeline, Lead stage) with correct amounts
- **Pipeline total**: Updated to $660k (includes pre-existing deals)

---

## Extraction Quality Issues (Backend/AI)

These are issues with the mock/AI extraction logic, not the UI:

### Critical

| # | Issue | Expected | Got |
|---|-------|----------|-----|
| 1 | **Contact name parsing** | `first_name: "Priya"`, `last_name: "Sharma"` | `first_name: "priya"`, `last_name: ""` (empty) |
| 2 | **Job title truncation** | `job_title: "Head of Product"` | `job_title: "Head"` |
| 3 | **Company name pollution** | `company_name: "NexGen Analytics"` | `company_name: "Product at NexGen Analytics"` (job title fragment leaked in) |
| 4 | **Person names extracted as companies** | Only real companies | "Priya Sharma", "David Kim" extracted as company records |
| 5 | **Job titles extracted as companies** | Only real companies | "Business Development", "Product" extracted as company records |
| 6 | **All deals named after first company** | "Deal with Meridian Labs", "Deal with CloudPeak Systems", "Deal with NexGen Analytics" | All 3 named "Deal #N with Meridian Labs" |
| 7 | **All deals attributed to first contact** | Each deal linked to its respective contact | All 3 have `contact_name: "Rachel Torres"` |

### Root Cause Analysis

The extraction is using **mock data from the preview endpoint** (`POST /api/workflows/preview`), not actual LLM processing. The mock logic in `crates/server/src/routes/data_source_workflows.rs` uses simple regex/heuristic extraction that:
- Splits names incorrectly (takes first word only for first_name)
- Picks up partial phrases as company names
- Doesn't properly associate deals with their respective contacts/companies
- Uses sequential numbering ("Deal #1", "#2", "#3") tied to the first company found

**Recommendation**: When real LLM extraction is wired up, the prompt template in the workflow builder is well-structured and should produce much better results. The mock should be improved to more closely match expected LLM output quality for testing purposes.

---

## UX/Functionality Gaps

### High Priority

1. **No "Create Workflow from Data Source" flow**
   - After creating a data source, there's no direct path to "Process this with a workflow"
   - User must navigate away to My Workflows → Builder → Run → select the data source
   - **Suggestion**: Add a "Run Workflow" button on the data source detail panel or in the row actions

2. **Rachel Torres company_name committed as separate company**
   - `company_name: "Business Development at Meridian Labs"` on the contact record was also auto-created as a company in the Companies list
   - This is the full `job_title + " at " + company` string, not the actual company
   - **Suggestion**: Company commit should match against existing companies before creating new ones, or contact `company_name` should not auto-create company records

3. **Deal descriptions contain raw source text**
   - Deal descriptions show the full paragraph from the source document instead of a clean summary
   - e.g., "1. Rachel Torres, VP of Business Development at Meridian Labs (rachel.torres@meridianlabs.com, +1-555-0199)..."
   - **Suggestion**: Extract a concise deal description rather than copying source paragraphs

4. **No way to link data source to workflow run**
   - In the Runs tab, completed runs show workflow name and duration but not which data source was processed
   - **Suggestion**: Add data source name/link to run history entries

### Medium Priority

5. **Staging tab shows stale data from previous runs**
   - The main Staging tab (`/workflows?tab=staging`) shows 10 pending records from a previous run alongside the current run
   - No clear way to filter by run or dismiss old staging batches
   - **Suggestion**: Add run-based filtering or auto-archive old staging batches

6. **No duplicate detection across runs in the commit flow**
   - When committing via the per-run staging view (`?run=<id>`), there's no warning about duplicates with existing CRM data
   - The main Staging tab does show "2 duplicates" with a "Reject 2 duplicates" batch action, which is good
   - **Suggestion**: Surface duplicate warnings in the per-run commit flow too

7. **Phone field shows "null" string**
   - In the edit form, the phone field displays the literal string "null" instead of being empty
   - **Suggestion**: Treat null/undefined as empty string in edit form

8. **No bulk edit in staging**
   - Can edit records one at a time but no way to bulk-edit (e.g., fix company_name for all contacts at once)
   - **Suggestion**: Add multi-select and batch edit for common fields

9. **Workflow run count in sidebar badge jumps**
   - "My Workflows" sidebar badge showed "10", then jumped to "20" during testing
   - Badge might be counting staging records rather than workflow definitions
   - **Suggestion**: Clarify what the badge count represents

### Low Priority

10. **No undo for reject**
    - Once a record is rejected, there's no way to un-reject it from the per-run view
    - **Suggestion**: Add an "Undo" or "Restore" action for rejected records

11. **"New Workflow" creation not tested**
    - The "New Workflow" button exists but was not tested in this session
    - The visual builder UI for editing existing workflows works well

12. **No progress indicator during workflow execution**
    - After clicking "Run Workflow", it instantly shows results (because mock is fast)
    - With real LLM processing, there should be a loading/progress state
    - **Suggestion**: Add a progress bar or streaming status during workflow execution

---

## What Works Well

- **Visual workflow builder**: Clean node-based editor with proper configuration panels
- **Staging review UX**: Edit/approve/reject per-record with confidence scores is intuitive
- **Bulk commit**: "Approve & commit N" button works smoothly
- **Workflow badge on contacts**: Clear "Imported via workflow" badge distinguishes sourced contacts
- **Pipeline integration**: Deals correctly appear in the kanban pipeline view with amounts
- **Data source management**: Add/view/delete text sources with search and type filtering
- **URL-based deep linking**: Staging tab preserves run ID in URL for bookmarking

---

## Changes Implemented

Based on user review of these findings, the following changes were made:

### Backend (mock extraction improvements)
- **Company extraction**: Changed `at_of_re` regex to only match `\bat` (not "of"), added job-title words to false positives ("Business", "Product", "Engineering" etc.), required 2+ word company names from "at X" pattern
- **Contact role/company splitting**: Changed all `role_lower.find(" of ")` to prefer `role_lower.rfind(" at ")` first — fixes "VP of Business Development at Meridian Labs" → role="VP of Business Development", company="Meridian Labs"
- **Deal attribution**: Each deal now matched to nearest contact by checking context line, not always using primary_contact/primary_company
- **Deal descriptions**: Truncated to 200 chars in mock output
- **Person-as-company filter**: In multi_extract, extracted contacts are cross-referenced and filtered from company list

### Frontend
- **Phone "null" fix**: `StagingReviewPanel.tsx` and `workflows.tsx` — null/undefined values now display as empty string, not "null"
- **"Run Workflow" from Data Sources**: Added Play button in table row actions + PreviewPanel, with `RunWorkflowFromSourceDialog` (workflow selector + run)
- **Data Source node**: Added `data_source` node type to WorkflowEditor with `DataSourceNodeConfig` (data source dropdown in node config panel)
- **My Workflows vs Intelligence separation**: Added TODO comments in `workflows.tsx`, `organization-profile.tsx`, and `sidebar.tsx` clarifying the intended separation of user-level vs org-level workflow views
- **Sidebar badge clarification**: Added comment explaining badge counts pending staging records (correct behavior, not a bug)

### Backlogged
- Bulk edit in staging
- Undo reject
- Progress indicator for long-running workflows
- Low-priority items

---

## Test Data Used

**Data Source Title**: "Q2 2026 Partnership Pipeline Notes"
**Content**:
```
Meeting notes from Q2 partnership review - March 2026

Key Contacts:
1. Rachel Torres, VP of Business Development at Meridian Labs
   (rachel.torres@meridianlabs.com, +1-555-0199) — $150k deal, negotiation phase
2. David Kim, CTO at CloudPeak Systems
   (david.kim@cloudpeak.io, +1-555-0234) — $85k integration services deal
3. Priya Sharma, Head of Product at NexGen Analytics
   (priya@nexgenanalytics.com) — $200k enterprise deal

Companies: Meridian Labs (biotech, San Diego), CloudPeak Systems (cloud, Austin),
NexGen Analytics (AI/ML, NYC)
```
