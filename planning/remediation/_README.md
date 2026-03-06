# Remediation

QA agents create remediation notes here when a slice fails verification.

## When to Create

- Slice QA finds acceptance criteria not met
- Evidence bundle is incomplete or fabricated
- Scope violation detected (files outside WRITE-ALLOWED modified)
- Story QA holdout test fails

## File Format

```
REM-{SLICE-ID}-{YYYYMMDD-HHMMSS}.md
```

For story-level failures:
```
REM-STORY-{STORY-ID}-{YYYYMMDD-HHMMSS}.md
```

## Required Content

```yaml
---
slice_id: SLICE-XXX
qa_agent_id: {your session/agent identifier}
created_at: {ISO 8601}
severity: CRITICAL | HIGH | MEDIUM
---
```

```markdown
## Failed Criteria

{Which acceptance criteria failed? List each with observed vs expected.}

1. AC #N: {criterion text}
   - Expected: {what should happen}
   - Observed: {what actually happened}

## Observed Behavior

{Detailed description of what went wrong. Include command output if relevant.}

## Expected Behavior

{What correct behavior looks like.}

## Remediation Hypothesis

{Your best guess at what needs to change to fix this:}

1. {Hypothesis A — e.g., "Edge case in input validation not handled"}
2. {Hypothesis B — e.g., "Missing test for empty array scenario"}

## Evidence

{Link to or inline the evidence that demonstrates the failure.}
```

## What Happens Next

1. QA moves the slice from `active/` to `failed/`
2. PM reads this remediation note (surfaced in `_STATUS.md`)
3. PM creates a targeted remediation slice (not a full redo)
4. The failed slice stays in `failed/` as a record
