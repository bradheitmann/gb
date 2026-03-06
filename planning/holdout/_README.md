# STOP. Story QA Only.

This directory contains holdout test specifications for story-level verification.

## Why This Is Restricted

Holdout tests verify that a story's implementation actually works end-to-end.
If Dev or slice-level QA agents see these tests, they can (intentionally or not)
optimize their implementation to pass specific test cases rather than solving the
actual problem. This defeats the purpose of independent verification.

**Anti-collusion principle:** The agent verifying the work must not have seen the
verification criteria during implementation.

## Who Can Access

| Role | Access |
|------|--------|
| PM | Write (creates holdout at story creation time) |
| Story QA | Read (runs holdout after all slices complete) |
| Dev | NEVER |
| Slice QA | NEVER |

## What to Do Instead

- **Dev agents:** Focus on your slice file's acceptance criteria. Those are your tests.
- **Slice QA agents:** Verify the slice's evidence bundle and run the slice's gates.
- **Story QA agents:** You're in the right place. Read the holdout spec, run it, report.

## File Format

```
HOLDOUT-{STORY-ID}.md
```

Extracted from the HOLDOUT SPECIFICATION section of the story file at creation time.
