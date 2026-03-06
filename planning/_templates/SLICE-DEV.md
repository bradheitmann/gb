<!-- Template Version: 3.0.0 | Updated: 2026-03-05 | Protocol: Unified Planning System -->

# DEV Slice Template

## YAML Frontmatter (Required)

```yaml
---
artifact_type: slice
slice_id: SLICE-{ID}
slice_type: DEV
title: {Title}
parent_id: STORY-{NAME}-{N}
parent_linear_id: {uuid}
status: Backlog | To Do | In Progress | Done | Archived
phase: DRAFT | PLANNED | IMPLEMENTED | VERIFIED | COMPLETE
priority: P0 | P1 | P2 | P3
effort: S | M | L | XL
task_list_id: SLICE-{ID}
claimed_by: null
claimed_at: null
session_id: null
linear_issue_id: {uuid}
linear_identifier: {TEAM-NNN}
linear_url: TBD
created_at: {ISO 8601}
updated_at: {ISO 8601}
---
```

## Body Template

```
================================================================================
SLICE ASSIGNMENT: {SLICE-ID} - {Brief Description}
================================================================================

STATUS: To Do
PHASE: PLANNED
PRIORITY: {P0 | P1 | P2 | P3}
EFFORT: {S | M | L | XL}
ASSIGNED: {Agent/Developer}
TYPE: DEV

--------------------------------------------------------------------------------
CLAIM PROTOCOL
--------------------------------------------------------------------------------

When starting this slice:
1. git pull (get latest)
2. git mv planning/slices/todo/{SLICE-ID}.md planning/slices/active/{SLICE-ID}.md
3. Update claimed_by, claimed_at, session_id in frontmatter
4. git commit -m "claim: {SLICE-ID}"
5. git push (if push fails, claim lost — check _STATUS.md for next available)
6. Create task list: task_list_id = {SLICE-ID}

--------------------------------------------------------------------------------
BACKGROUND
--------------------------------------------------------------------------------

**Problem:** {Clear statement of what's broken or missing}

**Why it matters:** {Business/technical impact}

**Current state:** {Where we are now}

**Desired state:** {Where we need to be}

--------------------------------------------------------------------------------
READING LIST
--------------------------------------------------------------------------------

Files to read before starting (minimal, specific):

1. {file1.ts}
   - Lines: {X-Y or ALL}
   - Focus: {specific functions/sections to understand}

2. {file2.ts}
   - Lines: {X-Y or ALL}
   - Focus: {specific functions/sections to understand}

--------------------------------------------------------------------------------
GUARDRAILS
--------------------------------------------------------------------------------

WRITE-ALLOWED (explicit file list):
- {src/feature/file.ts} (NEW|MODIFY)
- {tests/feature/file.test.ts} (NEW)

WRITE-PROHIBITED:
- planning/holdout/**              (NEVER -- test integrity)
- planning/_STATUS.md              (auto-generated, do not edit)
- planning/_GUIDE.md               (system file)
- planning/_NORTH-STAR.md          (system file)
- planning/escalations/**          (read only -- create your own via template)
- planning/story-reviews/**        (PM/Story QA only)
- src/core/** (shared infrastructure - requires separate slice)
- package.json (dependency changes require separate slice)
- Any file not explicitly listed above

MUST:
- [ ] Add tests for all new functionality
- [ ] Follow existing code patterns
- [ ] Maintain backward compatibility
- [ ] Pass all quality gates

MUST NOT:
- [ ] Modify unrelated files
- [ ] Read QA slices or holdout tests
- [ ] Communicate with other Dev agents
- [ ] Skip tests

--------------------------------------------------------------------------------
ACCEPTANCE CRITERIA
--------------------------------------------------------------------------------

Functional:
1. [ ] {Specific, measurable criterion}
2. [ ] {Specific, measurable criterion}

Quality:
1. [ ] All tests pass
2. [ ] Lint passes with zero errors
3. [ ] Build succeeds

--------------------------------------------------------------------------------
EVIDENCE GATES
--------------------------------------------------------------------------------

Gate 1: Tests Pass
  Command: {test command}
  Expected: Exit 0, all tests pass

Gate 2: Scope Compliance
  Command: git diff --name-only
  Expected: Only WRITE-ALLOWED files modified

--------------------------------------------------------------------------------
EVIDENCE REQUIRED
--------------------------------------------------------------------------------

Bundle Path: .edge-agentic/local/evidence/{SLICE-ID}/

Required Contents:
- before/           (baseline state)
- after/            (final state)
- verify.log        (all gate verifications)
- Summary.md        (<=10 lines summary)

--------------------------------------------------------------------------------
DEPENDENCIES
--------------------------------------------------------------------------------

PREREQUISITES:
  [ ] {SLICE-XX} - {Title} (REQUIRED)

BLOCKS:
  -> {SLICE-YY} - {Title}

PARALLEL EXECUTION:
  {YES/NO - reason}

--------------------------------------------------------------------------------
DELIVERABLE
--------------------------------------------------------------------------------

{Complete description of what will be delivered}

--------------------------------------------------------------------------------
ROLLBACK PLAN
--------------------------------------------------------------------------------

1. git revert {commit}
2. Verify: {verification step}

================================================================================
```

## Sizing Guide

| Size | Effort | Files | Complexity |
|------|--------|-------|------------|
| S | 1-2 hours | 1-3 | Simple, isolated |
| M | 3-6 hours | 3-10 | Moderate |
| L | 1-3 days | 10-20 | Significant |
| XL | 3-5 days | 20+ | Consider splitting |
