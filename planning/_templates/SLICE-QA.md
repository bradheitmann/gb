<!-- Template Version: 3.0.0 | Updated: 2026-03-05 | Protocol: Unified Planning System -->
<!--
================================================================================
ADVERSARIAL REVIEW INSTRUCTIONS (PRIMACY PLACEMENT)
================================================================================

You are a QA agent. Your job is to FIND PROBLEMS, not confirm success.

ASSUME the evidence is fabricated until you independently verify it.
ASSUME the implementation is incomplete until you prove otherwise.
ASSUME scope was violated until you check every modified file.

Do NOT read Dev agent handoffs, session logs, or reasoning.
Do NOT trust summary files without re-running the commands yourself.
Do NOT skip any acceptance criterion check.

Your fresh perspective IS the value you provide. Contaminating it with
Dev context defeats the purpose of independent verification.

If you cannot independently reproduce a claimed result, it FAILS.
================================================================================
-->

# QA Slice Template

## YAML Frontmatter (Required)

```yaml
---
artifact_type: slice
slice_id: SLICE-{ID}
slice_type: QA
title: {Title} - QA Review
parent_id: STORY-{NAME}-{N}
parent_linear_id: {uuid}
depends_on: SLICE-{PARENT-DEV-ID}
fresh_context: REQUIRED
evidence_path: .edge-agentic/local/evidence/SLICE-{PARENT-DEV-ID}/
status: Backlog | To Do | In Progress | Done | Archived
phase: DRAFT | PLANNED | IMPLEMENTED | VERIFIED | COMPLETE
priority: P0 | P1 | P2 | P3
effort: S | M
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
QA SLICE ASSIGNMENT: {SLICE-ID} - {Brief Description} Review
================================================================================

STATUS: Backlog
PHASE: PLANNED
PRIORITY: {P0 | P1 | P2 | P3}
EFFORT: {S | M}
ASSIGNED: {QA Agent}
TYPE: QA

DEPENDS ON: {PARENT-DEV-SLICE-ID} (must be in done/ before starting)
FRESH CONTEXT: REQUIRED (never read Dev handoffs or session logs)
EVIDENCE PATH: .edge-agentic/local/evidence/{PARENT-DEV-SLICE-ID}/

--------------------------------------------------------------------------------
CLAIM PROTOCOL
--------------------------------------------------------------------------------

When starting this slice:
1. Verify parent DEV slice is in planning/slices/done/
2. git pull (get latest)
3. git mv planning/slices/todo/{SLICE-ID}.md planning/slices/active/{SLICE-ID}.md
4. Update claimed_by, claimed_at, session_id in frontmatter
5. git commit -m "claim: {SLICE-ID}"
6. git push
7. Create task list: task_list_id = {SLICE-ID}

--------------------------------------------------------------------------------
CONTEXT SCOPE (YOUR CONTEXT IS FRESH)
--------------------------------------------------------------------------------

YOU RECEIVE:
- This QA slice file
- The evidence bundle at {evidence_path}
- The codebase (git log, file contents, test results)
- The parent DEV slice file (in done/ — read only)

YOU NEVER RECEIVE:
- Dev agent handoff documents
- Dev agent session logs or reasoning
- Dev agent task lists or scratch notes
- planning/holdout/** (Story QA only)
- Other QA agent results for different slices

This isolation is BY DESIGN. Your fresh perspective is the value.

--------------------------------------------------------------------------------
WHAT TO VERIFY
--------------------------------------------------------------------------------

For each acceptance criterion in the parent DEV slice:

1. READ the criterion
2. FIND the implementation (don't trust the evidence — look yourself)
3. RUN the verification command independently
4. COMPARE your result to the claimed result
5. SCORE: PASS or FAIL with evidence

--------------------------------------------------------------------------------
ACCEPTANCE CRITERIA (QA-SPECIFIC)
--------------------------------------------------------------------------------

1. [ ] All parent DEV slice acceptance criteria independently verified
2. [ ] Evidence bundle exists and is complete (before/, after/, verify.log, Summary.md)
3. [ ] Evidence is authentic (re-run commands, compare output)
4. [ ] Only WRITE-ALLOWED files were modified (scope compliance)
5. [ ] No regressions introduced (full test suite passes)
6. [ ] No security issues (secrets, injection, path traversal)

--------------------------------------------------------------------------------
GUARDRAILS
--------------------------------------------------------------------------------

WRITE-ALLOWED:
- This QA slice file (frontmatter updates)
- .edge-agentic/local/evidence/{SLICE-ID}/ (QA evidence)
- planning/remediation/REM-{PARENT-DEV-SLICE-ID}-*.md (if FAIL)

WRITE-PROHIBITED:
- planning/slices/done/{PARENT-DEV-SLICE-ID}.md (read only)
- planning/holdout/** (Story QA only -- NEVER)
- Any Dev handoff document
- Any source code file (QA does not fix -- QA reports)
- planning/_STATUS.md (auto-generated)

--------------------------------------------------------------------------------
SCORING
--------------------------------------------------------------------------------

| Category | Max | Score |
|----------|-----|-------|
| Functional correctness | 4 | /4 |
| Evidence authenticity | 4 | /4 |
| Scope compliance | 4 | /4 |
| Code quality & security | 4 | /4 |
| **TOTAL** | **16** | **/16** |

PASS: >= 14/16 (no CRITICAL issues)
CONDITIONAL PASS: 12-13/16 (minor issues, can proceed)
FAIL: < 12/16 or any CRITICAL issue

--------------------------------------------------------------------------------
ON PASS
--------------------------------------------------------------------------------

1. Update this slice: status: Done, phase: VERIFIED
2. git mv planning/slices/active/{SLICE-ID}.md planning/slices/done/{SLICE-ID}.md
3. Commit and push

--------------------------------------------------------------------------------
ON FAIL
--------------------------------------------------------------------------------

1. Create: planning/remediation/REM-{PARENT-DEV-SLICE-ID}-{YYYYMMDD-HHMMSS}.md
2. Move parent DEV slice: git mv planning/slices/done/ planning/slices/failed/
3. Move this QA slice back: git mv planning/slices/active/ planning/slices/todo/
4. Commit with message: "qa-fail: {PARENT-DEV-SLICE-ID} — {brief reason}"
5. Push — PM reads remediation/ to create fix slices

================================================================================
```

<!--
================================================================================
ADVERSARIAL REVIEW INSTRUCTIONS (RECENCY PLACEMENT)
================================================================================

REMINDER: You are adversarial. You are looking for PROBLEMS.

Before marking PASS, ask yourself:
- Did I actually run every verification command myself?
- Did I check EVERY file in the git diff, not just the ones mentioned?
- Could the evidence have been fabricated? Did I reproduce it?
- Are there edge cases the Dev agent didn't test?
- Is there a security concern I haven't checked?

If you have ANY doubt, it's a FAIL or CONDITIONAL PASS.
The cost of a false PASS is much higher than a false FAIL.

Reference: arXiv:2307.03172 — adversarial placement at primacy and
recency positions maximizes compliance with adversarial instructions.
================================================================================
-->
