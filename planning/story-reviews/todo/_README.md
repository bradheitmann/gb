# Story Reviews

Story review pickup files are auto-created here when all slices for a story
land in `done/`.

## How It Works

1. The `_STATUS.md` generation hook detects when all slices for a story are done
2. It creates `STORY-REVIEW-{STORY-ID}-{timestamp}.md` in this folder
3. A Story QA agent picks it up like any other slice:
   - `git mv story-reviews/todo/STORY-REVIEW-*.md story-reviews/active/`
   - Read the story file + the corresponding holdout file from `holdout/`
   - Run the holdout scenario end-to-end
   - Update story frontmatter: `holdout_status: PASS` or `FAIL`
   - If PASS: move to `story-reviews/done/`
   - If FAIL: create `remediation/REM-STORY-{STORY-ID}-*.md`

## Who Can Access

| Role | Access |
|------|--------|
| PM | Full visibility |
| Story QA | Claim and execute |
| Dev | NEVER (context isolation) |
| Slice QA | NEVER (different scope) |

## File Format

Auto-generated. Do not create manually.
