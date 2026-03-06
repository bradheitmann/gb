#!/usr/bin/env bash
# =============================================================================
# PostToolUse Hook: Auto-Generate planning/_STATUS.md
# Version: 1.0.0
# Created: 2026-03-05
#
# PURPOSE: When any file in planning/ is written, regenerate _STATUS.md to
#          reflect current planning state. Also detects story completion
#          and creates story-review pickup files.
#
# TRIGGERS: PostToolUse on Write|Edit (registered in .claude/settings.json)
#
# EXIT CODES:
#   0 = Always (PostToolUse should never block)
# =============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------
# Parse hook input — only act on planning/ files
# ---------------------------------------------------------------------------
INPUT=$(cat)
FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)

# Find planning dir relative to repo root
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
PLANNING_DIR=""

# Check common planning locations
for candidate in "project/planning" "planning"; do
    if [ -d "$REPO_ROOT/$candidate" ]; then
        PLANNING_DIR="$REPO_ROOT/$candidate"
        PLANNING_REL="$candidate"
        break
    fi
done

# No planning directory found — nothing to do
[ -z "$PLANNING_DIR" ] && exit 0

# Only act if the written file is in planning/
case "$FILE_PATH" in
    *planning/*) ;;
    *) exit 0 ;;
esac

# Don't regenerate when _STATUS.md itself is being written (avoid infinite loop)
case "$FILE_PATH" in
    *_STATUS.md) exit 0 ;;
esac

SLICES_DIR="$PLANNING_DIR/slices"
STATUS_FILE="$PLANNING_DIR/_STATUS.md"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# ---------------------------------------------------------------------------
# Helper: extract frontmatter field from a file
# ---------------------------------------------------------------------------
get_field() {
    local file="$1" field="$2"
    # Try YAML frontmatter first
    sed -n '/^---$/,/^---$/p' "$file" 2>/dev/null | grep "^${field}:" | head -1 | sed "s/^${field}:[[:space:]]*//" | tr -d '"' || true
}

# Try legacy key-value format
get_field_legacy() {
    local file="$1" field="$2"
    grep "^${field}:" "$file" 2>/dev/null | head -1 | sed "s/^${field}:[[:space:]]*//" | tr -d '"' || true
}

# Get a field, trying YAML frontmatter first, then legacy format
get_any_field() {
    local file="$1" field="$2"
    local val
    val=$(get_field "$file" "$field")
    if [ -z "$val" ]; then
        val=$(get_field_legacy "$file" "$field")
    fi
    printf '%s' "$val"
}

# ---------------------------------------------------------------------------
# Scan slices
# ---------------------------------------------------------------------------
available=""
active=""
blocked=""
failed=""
done_count=0
total_count=0

for state_dir in todo active blocked failed done backlog; do
    dir="$SLICES_DIR/$state_dir"
    [ -d "$dir" ] || continue

    for slice_file in "$dir"/SLICE-*.md; do
        [ -f "$slice_file" ] || continue
        total_count=$((total_count + 1))

        filename=$(basename "$slice_file" .md)
        slice_type=$(get_any_field "$slice_file" "slice_type")
        [ -z "$slice_type" ] && slice_type=$(get_any_field "$slice_file" "TYPE")
        priority=$(get_any_field "$slice_file" "priority")
        [ -z "$priority" ] && priority=$(get_any_field "$slice_file" "PRIORITY")
        claimed=$(get_any_field "$slice_file" "claimed_by")
        claimed_at=$(get_any_field "$slice_file" "claimed_at")
        depends=$(get_any_field "$slice_file" "depends_on")

        case "$state_dir" in
            todo)
                # Check if dependency is met (for QA slices)
                dep_met="yes"
                if [ -n "$depends" ] && [ "$depends" != "null" ]; then
                    # Check if the dependency is in done/
                    if [ ! -f "$SLICES_DIR/done/${depends}.md" ]; then
                        dep_met="no"
                        blocked="${blocked}${filename}|Waiting for ${depends}
"
                        continue
                    fi
                fi
                available="${available}${priority:-P2}|${filename}|${slice_type:-DEV}|available
"
                ;;
            active)
                active="${active}${filename}|${slice_type:-DEV}|${claimed:-unclaimed}|${claimed_at:-unknown}
"
                ;;
            failed)
                failed="${failed}${filename}|${priority:-P0}
"
                ;;
            done)
                done_count=$((done_count + 1))
                ;;
        esac
    done
done

active_count=$(printf '%s' "$active" | grep -c '.' 2>/dev/null || true)
active_count=${active_count:-0}; active_count=$(echo "$active_count" | tr -d '[:space:]')
blocked_count=$(printf '%s' "$blocked" | grep -c '.' 2>/dev/null || true)
blocked_count=${blocked_count:-0}; blocked_count=$(echo "$blocked_count" | tr -d '[:space:]')
failed_count=$(printf '%s' "$failed" | grep -c '.' 2>/dev/null || true)
failed_count=${failed_count:-0}; failed_count=$(echo "$failed_count" | tr -d '[:space:]')

# ---------------------------------------------------------------------------
# Scan escalations
# ---------------------------------------------------------------------------
escalations=""
if [ -d "$PLANNING_DIR/escalations" ]; then
    for esc_file in "$PLANNING_DIR/escalations"/ESC-*.md; do
        [ -f "$esc_file" ] || continue
        esc_name=$(basename "$esc_file" .md)
        # Get first line of blocker description
        desc=$(grep -A1 "## Blocker" "$esc_file" 2>/dev/null | tail -1 | head -c 70)
        escalations="${escalations}${esc_name}|${desc}
"
    done
fi

# ---------------------------------------------------------------------------
# Scan story reviews
# ---------------------------------------------------------------------------
story_reviews=""
if [ -d "$PLANNING_DIR/story-reviews/todo" ]; then
    for rev_file in "$PLANNING_DIR/story-reviews/todo"/STORY-REVIEW-*.md; do
        [ -f "$rev_file" ] || continue
        rev_name=$(basename "$rev_file" .md)
        story_reviews="${story_reviews}${rev_name}
"
    done
fi

# ---------------------------------------------------------------------------
# Story completion detection
# ---------------------------------------------------------------------------
# For each story file, check if all its slices are in done/
for story_file in "$PLANNING_DIR"/stories/STORY-*.md "$PLANNING_DIR"/STORY-*.md; do
    [ -f "$story_file" ] || continue

    story_id=$(get_any_field "$story_file" "story_id")
    [ -z "$story_id" ] && continue

    holdout_status=$(get_any_field "$story_file" "holdout_status")
    [ "$holdout_status" = "PASS" ] || [ "$holdout_status" = "FAIL" ] && continue

    # Find all slices that reference this story as parent
    story_slices_total=0
    story_slices_done=0

    for slice_file in "$SLICES_DIR"/*/SLICE-*.md; do
        [ -f "$slice_file" ] || continue
        parent=$(get_any_field "$slice_file" "parent_id")
        [ "$parent" = "$story_id" ] || continue
        story_slices_total=$((story_slices_total + 1))
        case "$slice_file" in
            */done/*) story_slices_done=$((story_slices_done + 1)) ;;
        esac
    done

    # If all slices done and > 0, check if review already exists
    if [ "$story_slices_total" -gt 0 ] && [ "$story_slices_done" -eq "$story_slices_total" ]; then
        review_exists=false
        for existing in "$PLANNING_DIR/story-reviews"/*/STORY-REVIEW-*"${story_id}"*.md; do
            [ -f "$existing" ] && review_exists=true && break
        done

        if [ "$review_exists" = "false" ]; then
            review_ts=$(date -u +"%Y%m%d-%H%M%S")
            review_file="$PLANNING_DIR/story-reviews/todo/STORY-REVIEW-${story_id}-${review_ts}.md"
            cat > "$review_file" << REVIEW_EOF
---
story_id: ${story_id}
created_at: ${TIMESTAMP}
auto_created: true
---

# Story Review: ${story_id}

All slices for this story are in \`done/\`. This story is ready for holdout review.

## Instructions

1. Read the story file: \`planning/stories/${story_id}.md\` (or \`planning/${story_id}.md\`)
2. Read the holdout spec: \`planning/holdout/HOLDOUT-${story_id}.md\`
3. Run the holdout scenario end-to-end
4. Update story frontmatter: \`holdout_status: PASS\` or \`FAIL\`
5. If PASS: move this file to \`story-reviews/done/\`
6. If FAIL: create \`remediation/REM-STORY-${story_id}-*.md\`
REVIEW_EOF
            story_reviews="${story_reviews}STORY-REVIEW-${story_id}-${review_ts}
"
        fi
    fi
done

# ---------------------------------------------------------------------------
# Epic progress
# ---------------------------------------------------------------------------
epic_progress=""
for epic_file in "$PLANNING_DIR"/epics/EPIC-*.md "$PLANNING_DIR"/EPIC-*.md; do
    [ -f "$epic_file" ] || continue
    epic_id=$(get_any_field "$epic_file" "epic_id")
    [ -z "$epic_id" ] && continue
    title=$(get_any_field "$epic_file" "title")
    stories_total=$(get_any_field "$epic_file" "stories_total")
    stories_complete=$(get_any_field "$epic_file" "stories_complete")
    [ -z "$stories_total" ] && stories_total="?"
    [ -z "$stories_complete" ] && stories_complete="?"
    epic_progress="${epic_progress}${epic_id}: ${title} (${stories_complete}/${stories_total} stories)
"
done

# ---------------------------------------------------------------------------
# Generate _STATUS.md
# ---------------------------------------------------------------------------
cat > "$STATUS_FILE" << STATUS_EOF
# Planning Status

> Auto-generated: ${TIMESTAMP} | Do not edit manually

**North Star:** See \`_NORTH-STAR.md\` for product vision

---

## AVAILABLE NOW (Ready to claim)

STATUS_EOF

if [ -n "$available" ]; then
    printf '| Priority | Slice ID | Type |\n' >> "$STATUS_FILE"
    printf '|----------|----------|------|\n' >> "$STATUS_FILE"
    printf '%s' "$available" | sort | while IFS='|' read -r pri sid stype _rest; do
        [ -z "$pri" ] && continue
        printf '| %s | %s | %s |\n' "$pri" "$sid" "$stype" >> "$STATUS_FILE"
    done
else
    printf '(none)\n' >> "$STATUS_FILE"
fi

cat >> "$STATUS_FILE" << STATUS_EOF

## ACTIVE (Claimed by an agent)

STATUS_EOF

if [ -n "$active" ]; then
    printf '| Slice ID | Type | Claimed By | Since |\n' >> "$STATUS_FILE"
    printf '|----------|------|------------|-------|\n' >> "$STATUS_FILE"
    printf '%s' "$active" | while IFS='|' read -r sid stype claimed since; do
        [ -z "$sid" ] && continue
        printf '| %s | %s | %s | %s |\n' "$sid" "$stype" "$claimed" "$since" >> "$STATUS_FILE"
    done
else
    printf '(none)\n' >> "$STATUS_FILE"
fi

cat >> "$STATUS_FILE" << STATUS_EOF

## BLOCKED (Dependencies not met)

STATUS_EOF

if [ -n "$blocked" ]; then
    printf '| Slice ID | Waiting For |\n' >> "$STATUS_FILE"
    printf '|----------|-------------|\n' >> "$STATUS_FILE"
    printf '%s' "$blocked" | while IFS='|' read -r sid waiting; do
        [ -z "$sid" ] && continue
        printf '| %s | %s |\n' "$sid" "$waiting" >> "$STATUS_FILE"
    done
else
    printf '(none)\n' >> "$STATUS_FILE"
fi

cat >> "$STATUS_FILE" << STATUS_EOF

## STORY REVIEWS READY

STATUS_EOF

if [ -n "$story_reviews" ]; then
    printf '%s' "$story_reviews" | while read -r rev; do
        [ -z "$rev" ] && continue
        printf '- `%s` -> `planning/story-reviews/todo/%s.md`\n' "$rev" "$rev" >> "$STATUS_FILE"
    done
else
    printf '(none)\n' >> "$STATUS_FILE"
fi

cat >> "$STATUS_FILE" << STATUS_EOF

## ESCALATIONS

STATUS_EOF

if [ -n "$escalations" ]; then
    printf '%s' "$escalations" | while IFS='|' read -r esc_name desc; do
        [ -z "$esc_name" ] && continue
        printf '- **%s**: %s\n' "$esc_name" "$desc" >> "$STATUS_FILE"
    done
else
    printf '(none)\n' >> "$STATUS_FILE"
fi

cat >> "$STATUS_FILE" << STATUS_EOF

## FAILED (Need remediation)

STATUS_EOF

if [ -n "$failed" ]; then
    printf '%s' "$failed" | while IFS='|' read -r sid pri; do
        [ -z "$sid" ] && continue
        printf '- **%s** (%s)\n' "$sid" "$pri" >> "$STATUS_FILE"
    done
else
    printf '(none)\n' >> "$STATUS_FILE"
fi

cat >> "$STATUS_FILE" << STATUS_EOF

## PROGRESS

STATUS_EOF

if [ -n "$epic_progress" ]; then
    printf '%s' "$epic_progress" | while read -r line; do
        [ -z "$line" ] && continue
        printf '%s\n' "$line" >> "$STATUS_FILE"
    done
else
    printf '(no epics found)\n' >> "$STATUS_FILE"
fi

printf '\nTotal slices: %d/%d done | %d active | %d blocked | %d failed\n' \
    "$done_count" "$total_count" "$active_count" "$blocked_count" "$failed_count" >> "$STATUS_FILE"

exit 0
