<!-- Template Version: 3.0.0 | Updated: 2026-03-05 | Protocol: Unified Planning System -->

# Holdout Test Specification

## YAML Frontmatter (Required)

```yaml
---
artifact_type: holdout
story_id: STORY-{NAME}-{N}
epic_id: EPIC-{N}
created_at: {ISO 8601}
created_by: PM
run_by: null
run_at: null
result: NOT_RUN | PASS | FAIL
---
```

## Body Template

```markdown
# HOLDOUT: STORY-{NAME}-{N}

## Scenario Description

{End-to-end user flow to test. Be specific — Story QA will follow this exactly.}

## Pre-conditions

- [ ] {Precondition 1 — e.g., "All story slices in done/"}
- [ ] {Precondition 2 — e.g., "Test database seeded with fixtures"}
- [ ] {Precondition 3 — e.g., "Service running on localhost:3000"}

## Execution Steps

1. {Exact step 1 — e.g., "Run: curl -X POST localhost:3000/api/auth/login ..."}
2. {Exact step 2 — e.g., "Verify response contains access_token field"}
3. {Exact step 3}
4. {Exact step 4}

## Expected Outputs

- {Exact, measurable output 1 — e.g., "HTTP 200 with JSON body containing 'session_id'"}
- {Exact, measurable output 2 — e.g., "Database table 'sessions' has new row"}
- {Exact, measurable output 3}

## Pass Criteria (ALL must be true)

- [ ] {Binary criterion 1}
- [ ] {Binary criterion 2}
- [ ] {Binary criterion 3}

## Fail Criteria (ANY one fails the story)

- [ ] {Failure condition 1 — e.g., "Any HTTP 5xx response"}
- [ ] {Failure condition 2 — e.g., "Response time > 2 seconds"}
- [ ] {Failure condition 3 — e.g., "Leaked credentials in response body"}

## Remediation Hint

{Which component area to investigate on failure — helps PM create targeted fix slices}

- Primary suspect: {component/module}
- Secondary suspect: {component/module}

---

> This file is SEALED after creation. Only Story QA reads it.
> Dev agents and slice QA agents must NEVER see this file.

*Created by: PM*
*Created at: {Date}*
```

## Access Control

| Role | Can Read | Can Write |
|------|----------|-----------|
| PM | Yes (creator) | Yes (at creation only) |
| Story QA | Yes (at review time) | No |
| Dev | NEVER | NEVER |
| Slice QA | NEVER | NEVER |
