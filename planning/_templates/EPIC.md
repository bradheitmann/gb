<!-- Template Version: 3.0.0 | Updated: 2026-03-05 | Protocol: Unified Planning System -->

# Epic Template

## YAML Frontmatter (Required)

```yaml
---
artifact_type: epic
epic_id: EPIC-{N}
title: {Title}
parent_id: {INITIATIVE-ID}
parent_linear_id: {uuid}
status: DRAFT | READY | IN_PROGRESS | COMPLETE | ARCHIVED
priority: P0 | P1 | P2 | P3
stories_total: 0
stories_complete: 0
holdout_not_applicable: true
linear_issue_id: {uuid}
linear_identifier: {TEAM-NNN}
linear_url: https://linear.app/{team}/issue/{TEAM-NNN}
created_at: {ISO 8601}
updated_at: {ISO 8601}
---
```

## Body Template

```markdown
# EPIC-{N}: {Title}

## Summary

[2-3 sentences describing what this epic delivers and why it matters]

## Business Value

- **User Impact:** {How users benefit}
- **Technical Impact:** {How the system improves}
- **Strategic Impact:** {How this advances the initiative}

## Problem Statement

[What problem does this epic solve?]

## Scope

### In Scope

- {Capability 1}
- {Capability 2}

### Out of Scope

- {Exclusion 1}
- {Exclusion 2}

## Success Criteria

- [ ] {Measurable criterion 1}
- [ ] {Measurable criterion 2}

## Story Decomposition

| Story ID | Title | Priority | Status | Effort |
|----------|-------|----------|--------|--------|
| STORY-{NAME}-001 | {Title} | P1 | DRAFT | M |
| STORY-{NAME}-002 | {Title} | P1 | DRAFT | M |

## Technical Architecture

### Components Affected

- {Component 1}: {Changes}

### Key Technical Decisions

1. {Decision 1}: {Rationale}

## Dependencies

### Prerequisites

- [ ] {Dependency 1}

### Blocks

- {What this epic unblocks}

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| {Risk 1} | Medium | High | {Strategy} |

## Linear Integration

- **Linear Issue:** [{TEAM-NNN}]({linear_url})
- **Label:** `type-epic`
- **Children:** {N} Stories linked

---

> Epic auto-completes when all stories reach COMPLETE status.
> Update `stories_total` and `stories_complete` in frontmatter as stories progress.

*Created by: {Agent/Human}*
*Last updated: {Date}*
```

## Status Transitions

```
DRAFT -> READY -> IN_PROGRESS -> COMPLETE -> ARCHIVED
```

| From | To | Trigger |
|------|----|---------|
| DRAFT | READY | All Stories defined |
| READY | IN_PROGRESS | First Story starts |
| IN_PROGRESS | COMPLETE | All Stories COMPLETE |
| COMPLETE | ARCHIVED | Retrospective done |
