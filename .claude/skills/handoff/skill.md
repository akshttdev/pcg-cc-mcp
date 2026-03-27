---
name: handoff
description: Write a session handoff file capturing current work state, then prompt the user to /clear and continue. Use when context is getting large or before switching focus.
user-invocable: true
allowed-tools: Bash, Read, Write, Grep, Glob
---

# Session Handoff

Write a structured handoff file so the next session can pick up exactly where this one left off. This skill is the **only** way to persist session context across /clear boundaries.

## When to use

- Context window is getting large (you'll be warned)
- Before switching to a different area of work
- At natural stopping points in multi-session tasks
- When the user says "handoff", "save state", "context is full"

## Handoff file location

Always write to: `planning/SESSION-HANDOFF.md`

This is a **singleton** — overwrite any existing handoff file. There should only ever be one active handoff at a time. The file is checked at session start (see CLAUDE.md).

## What to capture

Gather this information automatically (don't ask the user):

```bash
# Current state
git branch --show-current
pwd
git worktree list
git status --short
git log --oneline -5

# Server state
source .env 2>/dev/null
lsof -i :${FRONTEND_PORT:-3000} -P 2>/dev/null | grep LISTEN
lsof -i :${BACKEND_PORT:-3002} -P 2>/dev/null | grep LISTEN
```

## Handoff file format

```markdown
# Session Handoff

**Date**: YYYY-MM-DD
**Branch**: `branch-name`
**Worktree**: `/path/to/worktree`
**Last commit**: `hash` — message

## What Was Done This Session

[Bullet list of completed work — be specific about files changed and features implemented]

## What Remains

[Numbered list of remaining tasks, ordered by priority. Include:]
- Exact file paths and line numbers where relevant
- Specific error messages or test failures
- Commands that were in progress

## Active Bugs / Issues Being Fixed

[If mid-fix, describe the bug, what's been tried, what's left]

## Servers

- Backend: port NNNN (running/stopped)
- Frontend: port NNNN (running/stopped)
- Env vars: SIMULATE_LLM=1, ENABLE_AGENT_FLOW_ENGINE=1, etc.

## Key Context

[Any decisions made, approaches chosen, or gotchas the next session needs to know]
```

## After writing the handoff

1. Tell the user: "Handoff saved to `planning/SESSION-HANDOFF.md`. You can `/clear` now and I'll pick up where we left off."
2. Do NOT commit the handoff file — it's ephemeral working state
3. Do NOT /clear automatically — let the user do it

## At session start (loading a handoff)

When a new session starts and `planning/SESSION-HANDOFF.md` exists:
1. Read it
2. Confirm the worktree matches `pwd`
3. Delete the file (it's been consumed)
4. Continue with the remaining work
