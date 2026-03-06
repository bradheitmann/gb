<!-- Template Version: 3.0.0 | Updated: 2026-03-05 | Protocol: Unified Planning System -->

# Story Template

## YAML Frontmatter (Required)

```yaml
---
artifact_type: story
story_id: STORY-{NAME}-{N}
title: {Title}
parent_id: EPIC-{N}
parent_linear_id: {uuid}
status: DRAFT | READY | IN_PROGRESS | COMPLETE | ARCHIVED
priority: P0 | P1 | P2 | P3
holdout_status: NOT_RUN | PASS | FAIL
holdout_file: planning/holdout/HOLDOUT-STORY-{NAME}-{N}.md
story_review_status: PENDING | IN_REVIEW | COMPLETE
linear_issue_id: {uuid}
linear_identifier: {TEAM-NNN}
linear_url: https://linear.app/{team}/issue/{TEAM-NNN}
created_at: {ISO 8601}
updated_at: {ISO 8601}
---
```

## Body Template

```markdown
# STORY-{NAME}-{N}: {Title}

## User Story

**As a** {role/persona}
**I want to** {capability/feature}
**So that** {benefit/value}

## Summary

[2-3 sentences describing what this story delivers]

## Acceptance Criteria

- [ ] **AC1:** {Specific, testable criterion}
- [ ] **AC2:** {Specific, testable criterion}
- [ ] **AC3:** {Specific, testable criterion}

## Technical Notes

### Approach

{Brief description of the technical approach}

### Components Affected

- `{file/module 1}`: {Changes}
- `{file/module 2}`: {Changes}

## Slice Decomposition

| Slice ID | Type | Title | Status | Effort |
|----------|------|-------|--------|--------|
| SLICE-{X}-DEV-001 | DEV | {Title} | To Do | M |
| SLICE-{X}-QA-001 | QA | {Title} | Backlog | S |

### Slice Dependencies

```
SLICE-{X}-DEV-001 --> SLICE-{X}-QA-001
```

## Dependencies

### Prerequisites

- [ ] {Prerequisite 1}

### Blocks

- {What this story unblocks}

## Out of Scope

- {Explicit exclusion 1}

## HOLDOUT SPECIFICATION

> **IMPORTANT:** This section is written at story creation time, BEFORE any
> implementation begins. It is extracted to `planning/holdout/HOLDOUT-STORY-{NAME}-{N}.md`
> during story setup. Slice agents NEVER see it. Story QA receives it fresh.

### End-to-End Scenario

{Describe the complete user flow to test}

### Input Conditions

- {Precondition 1}
- {Precondition 2}

### Expected Outputs

- {Specific, measurable output 1}
- {Specific, measurable output 2}

### Pass/Fail Criteria

**PASS when ALL of these are true:**
- [ ] {Binary criterion 1}
- [ ] {Binary criterion 2}

**FAIL when ANY of these are true:**
- [ ] {Failure condition 1}
- [ ] {Failure condition 2}

## Definition of Done

- [ ] All acceptance criteria met
- [ ] All DEV slices in `done/`
- [ ] All QA slices in `done/`
- [ ] Holdout test: PASS
- [ ] No regressions introduced

## Linear Integration

- **Linear Issue:** [{TEAM-NNN}]({linear_url})
- **Parent Issue:** [{EPIC-NNN}]
- **Label:** `type-story`

---

> Story auto-completes when `holdout_status: PASS`.
> Story review is auto-triggered when all slices reach `done/`.

*Created by: {Agent/Human}*
*Last updated: {Date}*
```

## Holdout Extraction

When creating a story, the PM agent must:
1. Write the HOLDOUT SPECIFICATION section in the story file
2. Extract it to `planning/holdout/HOLDOUT-STORY-{NAME}-{N}.md`
3. The holdout file is sealed — only Story QA reads it
