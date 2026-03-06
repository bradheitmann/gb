# Escalations

Agents create escalation files here when they hit blockers they cannot resolve.

## When to Escalate

- External dependency unavailable (API down, credentials missing)
- Slice requirements are ambiguous or contradictory
- Required file/module doesn't exist and isn't in WRITE-ALLOWED
- Acceptance criteria impossible to meet with current architecture
- Need a decision from PM or human before continuing

## Do NOT Escalate

- Build/test failures you can debug yourself
- Missing context you can find by reading the codebase
- Questions answered in `_GUIDE.md` or `_NORTH-STAR.md`

## File Format

```
ESC-{SLICE-ID}-{YYYYMMDD-HHMMSS}.md
```

## Required Content

```yaml
---
slice_id: SLICE-XXX
agent_id: {your session/agent identifier}
created_at: {ISO 8601}
---
```

```markdown
## Blocker Description

{What is blocking you? Be specific.}

## What Was Tried

1. {Step 1 you attempted}
2. {Step 2 you attempted}
3. {Why it didn't work}

## Unblock Options

{Suggest 2-3 ways PM could unblock this:}

1. {Option A — e.g., "Provide staging credentials via Doppler"}
2. {Option B — e.g., "Relax AC #3 to allow mock data"}
3. {Option C — e.g., "Create a separate slice for the missing module"}
```

## What Happens Next

PM reads this file (surfaced in `_STATUS.md` ESCALATIONS section).
PM either resolves the blocker directly or creates a new slice to address it.
Your slice stays in `active/` until the blocker clears.
