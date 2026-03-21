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

## Phase 0: Define Scope & Generate Interaction Scripts

### If a planning file is provided:
Use Sequential Thinking (scale with feature count: `min(features, 10)` thoughts) to:

1. Read the planning file and extract every **user-facing feature** — skip backend-only/infrastructure items
2. **Classify each feature's actor** — who performs this action?
   - **User action**: The user clicks, fills, submits via the UI → generate a full interaction script
   - **Agent action**: An AI agent or background worker performs this → generate an **observation script** instead (the user doesn't DO the action, but should be able to SEE the result)
   - **Hybrid**: Both user and agent can trigger it (e.g., stage transitions) → generate scripts for both paths
3. For each feature, translate it into the appropriate script type

### Actor classification matters

Many planning files describe features that are executed by agents, not users. Examples:
- "Agent flow executor advances from Planning → Executing" — agent action
- "User submits friction report" — user action
- "Deal stage transition via dashboard (permissive) or agent (strict FSM)" — hybrid

**For agent actions, don't generate DO steps — generate OBSERVE steps:**
- Can the user SEE that the agent did something? (status badge changed, card moved, toast appeared)
- Is the agent's action reflected in the UI without a page refresh?
- Does the UI explain what the agent did and why?

### Interaction script format

**An interaction script is NOT "navigate to the page and check it renders." It IS:**
- The specific clicks, form fills, submissions, and verifications a user would perform
- Derived directly from the feature's claimed behavior in the planning file
- Including the **verification step** — how do you confirm the action worked?

Each script has 5 action types:
- **FIND**: Locate the entry point — measures discoverability
- **DO**: Perform an interaction (user actions only) — measures execution quality
- **OBSERVE**: Check that an agent/system action is visible to the user — measures state communication
- **VERIFY**: Check the outcome of a DO or OBSERVE — measures feedback quality
- **BREAK**: Try to break it — measures error handling and edge cases

**Example — user action (friction report):**
```
Journey: Report friction [USER ACTION]
1. FIND: Click sidebar More → Feedback & Support
2. DO: Select "Friction Report" from Feedback Type dropdown
3. VERIFY: Friction-specific fields appear (frustration level, what went wrong)
4. DO: Fill fields, click Submit
5. VERIFY: Toast confirms, dialog closes
6. BREAK: Submit with empty required fields — does validation fire?
```

**Example — agent action (agent flow execution):**
```
Journey: Monitor agent flow progression [AGENT ACTION — observe only]
1. FIND: Navigate to Agent Flows page
2. OBSERVE: Is there a flow in "Executing" status? Does the status badge show correctly?
3. OBSERVE: If flow moved from Planning → Executing, is there a timestamp or activity log showing when?
4. OBSERVE: If flow is in NeedsClarification, does the UI explain what the agent is asking?
5. VERIFY: No way to test the transition itself (agent-driven), but verify the USER can see the result
6. NOTE: "User cannot trigger this action — it's agent-driven. Test observability only."
```

**Example — hybrid action (deal stage transition):**
```
Journey: Move deal between stages [HYBRID — user + agent paths]
User path:
1. FIND: Navigate to CRM Pipeline
2. DO: Drag deal card from Lead to Business Analysis
3. VERIFY: Deal moves, toast confirms, column counts update
4. BREAK: Try dragging to a non-adjacent stage — does FSM block it?
Agent path (observe only):
5. OBSERVE: If an agent moved a deal, does the kanban reflect it without refresh?
6. OBSERVE: Is there an activity log showing who/what moved the deal and when?
```

### If no argument provided, generate scripts for these 5 core journeys:
1. **First login → orient → find work**: Login → understand the layout → find a project → open a task
2. **Create and manage a task**: Create task → fill fields → submit → verify it appears → change status
3. **Navigate CRM**: Find a pipeline → view stages → add a deal → move it between stages
4. **Configure settings**: Find settings → change theme → verify it takes effect → change back
5. **Report a problem**: Find feedback → select friction type → fill fields → submit → verify confirmation

### For all audits, also check:
- **Navigation coherence**: Can a new user build a mental model of where things live?
- **State communication**: Does the UI always tell the user what's happening?
- **Error recovery**: When things go wrong, can the user get back on track?

Output the full list of interaction scripts before proceeding to Phase 1.

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

## Phase 3: Execute Interaction Scripts

**Do not just navigate and screenshot. Execute every step in the interaction scripts from Phase 0.**

For each journey, run through its script step by step using Playwright. The script has FIND, DO, VERIFY, and BREAK actions — execute ALL of them.

### Execution rules

1. **FIND steps**: Navigate from the dashboard. Record the exact click path and click count. Take a snapshot at the destination. If the feature can't be found within 5 clicks, flag as PAIN POINT.

2. **DO steps**: Actually perform the interaction — click buttons, fill forms with realistic test data, submit, select options, drag items. If a form needs data, use descriptive test values (e.g., title: "Test deal from UX audit", not "asdf"). Use `browser_fill_form` for form fields, `browser_click` for buttons, `browser_select_option` for dropdowns.

3. **OBSERVE steps** (for agent/system actions): The user didn't trigger this — an agent or background process did. Check:
   - Is the result visible in the UI? (status badge, card position, list entry)
   - Is there an activity log or timeline showing what happened and when?
   - Does the UI explain WHO performed the action (agent name, system, user)?
   - Would the user know the action happened without being told? Or would they need to refresh?
   - If no test data exists to observe (e.g., no agent has run), note it: "Cannot test — no agent-driven data in seed. Verified via code trace that [component] renders [status] for this state."

4. **VERIFY steps**: After each action, check:
   - Did a toast/notification confirm success? (snapshot for toast, or check if toast region updated)
   - Did the UI update? (take snapshot, compare to before)
   - If something was created, can you find it? (navigate to the list/page where it should appear)
   - Check `browser_console_messages level: error` after every submit — new console errors during a user action are a finding

4. **BREAK steps**: Try to trigger error states:
   - Submit forms with empty required fields — does validation appear inline?
   - Enter invalid data (wrong format, too long) — is the error message helpful?
   - Navigate away mid-form — is work lost? Is there a confirmation dialog?
   - Double-click submit — does it create duplicates?

### Per journey, also evaluate:

#### Discoverability
- Click count from dashboard to starting the action
- Would a new user find this without being told where it is?
- Is the entry point visible without scrolling or expanding menus?

#### Execution quality
- At each step: is it obvious what to do next?
- Are required fields clearly marked?
- Are labels/placeholders helpful or generic?
- Is the form asking for too much at once?

#### Feedback quality
- Does the UI tell the user what happened after each action?
- Optimistic update (instant) or delayed (spinner then update)?
- Are error messages actionable ("Title is required") or generic ("Invalid input")?

#### Recovery
- If the user makes a mistake, can they undo/fix it?
- If an API call fails, does the UI show an error or silently fail?

#### Consistency
- Does this flow use the same patterns as other flows? (button styles, form layout, toast style)
- Same action in different contexts — does it work the same way?

### If a script step can't be executed

Some steps may be impossible due to missing seed data, feature not wired, or backend not supporting it. When this happens:
- **Don't skip silently.** Record it as a finding: "Step N could not be executed because [reason]"
- If the feature has no seed data, try to create test data via the UI as part of the script (e.g., create a deal before trying to move it between stages)
- If creation isn't possible (no create button, API error), flag the entire journey and note what blocked it

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

Create `planning/reviews/YYYY-MM-DD--review--experience-audit.md`:

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

| Journey | Actor | Steps | Completion | Blockers | Pain | Friction | Grade |
|---------|-------|-------|------------|----------|------|----------|-------|
<!-- Actor: USER / AGENT (observe) / HYBRID. Grade: A (smooth) / B (minor issues) / C (confusing) / D (broken) / F (impossible) -->

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

**Report**: `planning/reviews/YYYY-MM-DD--review--experience-audit.md`
**Blockers**: N | **Pain Points**: N | **Friction**: N | **Polish**: N
**Quick Wins**: N identified | **Investments**: N identified
**Journey Grades**: [summary of grades]
```

---

## Important Rules

- **DO, don't just look.** The biggest failure mode of this skill is navigating to pages, taking screenshots, and calling it an audit. That's a rendering check, not a UX audit. You must actually click buttons, fill forms, submit data, and verify results. If you finish Phase 3 without having filled a single form or clicked a single submit button, you did it wrong.
- **Be the user, not the developer.** Don't excuse bad UX because you can read the code. If a user would be confused, it's a finding.
- **Screenshots are evidence.** Take a screenshot for every BLOCKER and PAIN POINT. Snapshots (accessibility tree) are better for analysis, screenshots are better for communicating findings to the team.
- **Recommendations must be specific.** Not "improve the empty state" but "add a CTA button labeled 'Create your first pipeline' with a link to /organizations/{orgId}/crm/pipelines/new."
- **Create test data when needed.** If a feature has no data to test (empty pipeline, no agent flows), create it via the UI as part of the journey. "No test data" is not an excuse to skip interaction testing — the creation flow IS part of the experience.
- **Don't boil the ocean.** Audit the journeys in scope, not every page in the app. Depth over breadth.
- **Quick wins are gold.** A 30-minute fix that removes daily friction is worth more than a week-long redesign. Prioritize accordingly.
- **Consistency findings compound.** One inconsistent button isn't worth reporting. A pattern of inconsistency (some dialogs confirm with "Save", others with "Submit", others with "Done") is a FRICTION finding.
- **Empty state quality is a leading indicator.** Apps with thoughtful empty states almost always have better UX elsewhere. Check these first for a quick read on UX maturity.
- **Test with the sidebar both expanded and collapsed.** Many layout issues only appear in one state.
- **Check console errors at every step.** Silent JS errors during user journeys mean broken state even if the UI looks fine — flag as hidden PAIN POINT.
- Do NOT fix issues — this is an audit, not implementation. Document everything for follow-up.
- Do NOT modify source code (only planning/report files).
- Always confirm worktree before writing report files.
