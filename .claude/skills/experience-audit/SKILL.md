---
name: experience-audit
description: UX audit — walk real user journeys via Playwright, identify blockers/pain points/friction, produce prioritized recommendations with effort/value classification
user-invocable: true
allowed-tools: Agent, Bash, Read, Write, Edit, Glob, Grep, mcp__sequential-thinking__sequentialthinking, mcp__playwright__browser_navigate, mcp__playwright__browser_snapshot, mcp__playwright__browser_click, mcp__playwright__browser_fill_form, mcp__playwright__browser_wait_for, mcp__playwright__browser_console_messages, mcp__playwright__browser_take_screenshot, mcp__playwright__browser_hover, mcp__playwright__browser_press_key, mcp__playwright__browser_resize, mcp__playwright__browser_select_option
---

# Experience Audit

Evaluate the application from a user's perspective. This isn't about whether code compiles or routes are wired — it's about whether a real person can accomplish their goals without confusion, frustration, or dead ends.

Complements `/functionality-audit` (does it work?) and `/qa-review` (is it correct?) by asking: **is it good to use?**

## Arguments

`$ARGUMENTS` — one of:
- A planning file path (audit features from that sprint)
- A comma-separated list of user journeys to test (e.g., `"create a task, submit friction report, view CRM pipeline"`)
- A page or area to focus on (e.g., `/settings`, `CRM`, `onboarding`)
- No argument: audit the 5 core journeys (see Phase 1)

## Severity Taxonomy

Every finding gets a severity:

- **BLOCKER**: User cannot complete the task. Dead end, crash, or infinite spinner with no escape.
- **PAIN POINT**: User can complete the task but with significant confusion, wrong mental model, or wasted effort. They'd complain about this.
- **FRICTION**: Minor annoyance. User pauses, re-reads, or takes an extra click. Adds up over time.
- **POLISH**: Works fine, user wouldn't complain, but a better version exists. Delight opportunities.

## Recommendation Categories

Every recommendation gets an effort/value classification:

- **QUICK WIN**: < 1 day effort, noticeable improvement. Fix these first.
- **INVESTMENT**: Multi-day effort but unlocks meaningfully better experience. Needs planning.
- **STRUCTURAL**: Requires architectural change (new component system, navigation redesign, state management refactor). High value but high cost — flag for roadmap discussion.

---

## Phase 0: Define Scope & Journeys

### If a planning file is provided:
Use Sequential Thinking (3-5 thoughts) to extract user-facing features and translate them into **user journeys** — not "endpoint X returns data" but "user opens the dashboard and sees their cost breakdown."

### If no argument provided, audit these 5 core journeys:
1. **First login → orient → find work**: Login → understand the layout → find a project → find a task
2. **Create and manage a task**: Create task → assign → track status → view result
3. **Navigate CRM**: Find a pipeline → view deals → move a deal between stages
4. **Configure settings**: Find settings → change a preference → verify it took effect
5. **Report a problem**: Find the feedback mechanism → submit a friction report → see confirmation

### For all audits, also check:
- **Navigation coherence**: Can a new user build a mental model of where things live?
- **State communication**: Does the UI always tell the user what's happening?
- **Error recovery**: When things go wrong, can the user get back on track?

## Phase 1: Environment Check

Verify servers are running before attempting Playwright:

```bash
source .env 2>/dev/null
FPORT=${FRONTEND_PORT:-3000}
BPORT=${BACKEND_PORT:-3002}
lsof -i :$FPORT -P 2>/dev/null | grep LISTEN
lsof -i :$BPORT -P 2>/dev/null | grep LISTEN
```

If servers aren't running, ask the user to start them. **This skill requires Playwright** — code trace is not sufficient for UX evaluation. If servers can't be started, abort with explanation.

### Login

Navigate to `http://localhost:$FPORT`. If redirected to login:
1. Read credentials from `.env` (`E2E_USERNAME`, `E2E_PASSWORD`) or fall back to defaults from the test seed
2. Fill username and password fields
3. Click Sign in, wait for dashboard
4. Take snapshot to confirm

## Phase 2: First Impressions & Information Architecture

Evaluate the app the way a new user would encounter it. Use Sequential Thinking (5-8 thoughts).

### 2a. Landing orientation (after login)
- Take a snapshot of the dashboard
- **Mental model test**: From the snapshot alone, can you answer:
  - What is this app for?
  - What are the main things I can do?
  - Where do I start?
- Flag: cluttered layouts, unclear hierarchy, competing CTAs, jargon without context

### 2b. Navigation audit
- Take a snapshot of the sidebar in expanded state
- Count: how many top-level items? How many require expanding/scrolling?
- **Cognitive load**: If there are >7 top-level nav groups, flag as PAIN POINT (Miller's law)
- **Grouping logic**: Do the groupings make sense? Would a user know whether "Intelligence" is under "My Workspace" or an org?
- Check: "More" menus and hidden sections — what's buried? Would a user know to look there?
- Check: breadcrumbs — do they show where you are and how to get back?

### 2c. Settings discoverability
- Navigate to Settings
- Take a snapshot of each scope tab (User, Admin, Org, Client)
- Flag: settings that are hard to find, unclear labels, scope confusion (when should a user look in "Admin" vs "Org"?)
- Check: does changing a setting give feedback (toast, save indicator)?

## Phase 3: Task Flow Walkthroughs

For each journey identified in Phase 0, walk through it step by step using Playwright.

### Per journey, evaluate:

#### 3a. Discoverability
- Can the user find where to START the task?
- Is the entry point visible without scrolling or expanding menus?
- Click count from dashboard to starting the action

#### 3b. Execution
- Walk through each step. At each step, take a snapshot and ask:
  - Is it obvious what to do next?
  - Are required fields clearly marked?
  - Are labels/placeholders helpful or generic?
  - Is the form asking for too much at once?
- Flag: steps where the user must guess, unlabeled icons, buttons with no hover text

#### 3c. Feedback
- After completing an action:
  - Does a toast/notification confirm success?
  - Does the UI update immediately (optimistic) or after a delay?
  - If the action created something, can the user find it?
- Try submitting an incomplete/invalid form:
  - Does validation feedback appear inline or only on submit?
  - Are error messages actionable ("Title is required") or generic ("Invalid input")?

#### 3d. Recovery
- Navigate away mid-task — is work preserved or lost?
- If an error occurs, is there a way back? Or is the user stuck?

#### 3e. Consistency
- Does this flow use the same patterns (button styles, form layout, confirmation style) as other flows?
- Same action in different contexts — does it work the same way?

### Check console errors after each navigation:
```
browser_console_messages level: error
```
Console errors during a user journey indicate broken state even if the UI looks fine.

## Phase 4: State Quality Audit

Navigate to each major page area and check these states:

### 4a. Empty states
- For pages that show data (projects, tasks, pipelines, workflows, people, companies):
  - What does the page look like with no data?
  - Does the empty state explain what belongs here?
  - Does it offer a CTA to create the first item?
  - Or does it just show a blank area / "No data"?
- Flag: blank pages with no guidance (PAIN POINT), generic "No data" without context (FRICTION)

### 4b. Loading states
- Navigate to data-heavy pages and observe:
  - Is there a skeleton/spinner while data loads?
  - Or does the page flash blank then populate?
  - Does the skeleton match the shape of the actual content?
- Flag: content layout shift after load (FRICTION), no loading indicator at all (PAIN POINT)

### 4c. Error states
- Check pages that depend on API data:
  - If the API returns an error, what does the user see?
  - Is the error message actionable?
  - Can the user retry without refreshing?
- Check: do error boundaries catch component crashes, or does the whole page go white?

### 4d. Long content
- For pages with lists/tables: what happens with many items?
  - Is there pagination, infinite scroll, or does it just grow?
  - Can the user filter/search?
  - Is the sort order obvious and useful?

## Phase 5: Accessibility Quick-Check

This is not a full WCAG audit — it's a practical check of the most impactful issues.

### 5a. Keyboard navigation
- Starting from the dashboard, try Tab through the page:
  - Can you reach all interactive elements?
  - Is the focus indicator visible?
  - Can you operate the sidebar, open dialogs, submit forms without a mouse?
- Try the command palette (⌘K) — does it work? Can you navigate to key pages?

### 5b. Responsive behavior
- Resize browser to mobile width (375px):
  ```
  browser_resize width: 375, height: 812
  ```
  - Does the sidebar collapse/transform?
  - Is content readable without horizontal scrolling?
  - Are touch targets large enough (min 44x44px)?
  - Take a screenshot at mobile width for key pages

- Resize back to desktop:
  ```
  browser_resize width: 1440, height: 900
  ```

### 5c. Color and contrast
- Snapshot pages in both light and dark mode (toggle in Settings → General → Theme)
- Flag: text hard to read against background, icons that disappear in dark mode, status colors that rely solely on color (no icon/label for colorblind users)

### 5d. Screen reader basics
- Check snapshot output for meaningful labels:
  - Are buttons labeled (not just icons)?
  - Do images have alt text?
  - Are form fields associated with their labels?
  - Do status indicators have text equivalents?
- Flag: icon-only buttons with no aria-label (FRICTION), unlabeled form fields (PAIN POINT)

## Phase 6: Generate Report

Create `planning/YYYY-MM-DD--review--experience-audit.md`:

```markdown
# Experience Audit Report

**Date**: YYYY-MM-DD
**Scope**: [journeys or areas audited]
**Branch**: `current-branch`
**Viewport**: Desktop (1440×900) + Mobile (375×812)

## Executive Summary

[2-3 sentences: overall UX quality, biggest wins, biggest risks]

## Findings by Severity

### BLOCKERS
<!-- User cannot complete the task -->
| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|

### PAIN POINTS
<!-- User can complete but with significant confusion -->
| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|

### FRICTION
<!-- Minor annoyances that add up -->
| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|

### POLISH
<!-- Works fine but could be better -->
| # | Journey/Page | Finding | Screenshot | Recommendation | Category |
|---|-------------|---------|------------|----------------|----------|

## Journey Scorecard

| Journey | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|------------|----------|------|----------|-------|
<!-- One row per journey tested. Grade: A (smooth) / B (minor issues) / C (confusing) / D (broken) / F (impossible) -->

## Quick Wins (do first)

Ranked by impact-to-effort ratio:

1. [finding #] — [one-line fix description] — [effort estimate]
2. ...

## Investments (plan for)

High-value improvements that need design/architecture work:

1. [finding #] — [what it enables] — [rough effort] — [why it matters]
2. ...

## Structural Recommendations (roadmap discussion)

Changes that require rethinking a pattern or system:

1. [description] — [what's wrong now] → [what good looks like] — [scope]
2. ...

## Accessibility Summary

| Check | Status | Issues |
|-------|--------|--------|
| Keyboard navigation | PASS/PARTIAL/FAIL | [details] |
| Mobile responsive | PASS/PARTIAL/FAIL | [details] |
| Color/contrast | PASS/PARTIAL/FAIL | [details] |
| Screen reader basics | PASS/PARTIAL/FAIL | [details] |

## State Quality Summary

| State | Coverage | Issues |
|-------|----------|--------|
| Empty states | N/M pages have good empty states | [details] |
| Loading states | N/M pages have skeletons | [details] |
| Error states | N/M pages handle errors gracefully | [details] |
| Validation feedback | Inline/on-submit/missing | [details] |
```

## Phase 7: Update Planning File (if provided)

If a planning file was provided as argument, append:

```markdown
## Experience Audit (YYYY-MM-DD)

**Report**: `planning/YYYY-MM-DD--review--experience-audit.md`
**Blockers**: N | **Pain Points**: N | **Friction**: N | **Polish**: N
**Quick Wins**: N identified | **Investments**: N identified
**Journey Grades**: [summary of grades]
```

---

## Important Rules

- **Be the user, not the developer.** Don't excuse bad UX because you can read the code. If a user would be confused, it's a finding.
- **Screenshots are evidence.** Take a screenshot for every BLOCKER and PAIN POINT. Snapshots (accessibility tree) are better for analysis, screenshots are better for communicating findings to the team.
- **Recommendations must be specific.** Not "improve the empty state" but "add a CTA button labeled 'Create your first pipeline' with a link to /organizations/{orgId}/crm/pipelines/new."
- **Don't boil the ocean.** Audit the journeys in scope, not every page in the app. Depth over breadth.
- **Quick wins are gold.** A 30-minute fix that removes daily friction is worth more than a week-long redesign. Prioritize accordingly.
- **Consistency findings compound.** One inconsistent button isn't worth reporting. A pattern of inconsistency (some dialogs confirm with "Save", others with "Submit", others with "Done") is a FRICTION finding.
- **Empty state quality is a leading indicator.** Apps with thoughtful empty states almost always have better UX elsewhere. Check these first for a quick read on UX maturity.
- **Test with the sidebar both expanded and collapsed.** Many layout issues only appear in one state.
- **Check console errors at every step.** Silent JS errors during user journeys mean broken state even if the UI looks fine — flag as hidden PAIN POINT.
- Do NOT fix issues — this is an audit, not implementation. Document everything for follow-up.
- Do NOT modify source code (only planning/report files).
- Always confirm worktree before writing report files.
