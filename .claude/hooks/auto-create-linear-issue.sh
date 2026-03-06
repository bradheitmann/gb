#!/usr/bin/env bash
# =============================================================================
# PostToolUse Hook: Auto-Create Linear Issue for New Slice Files
# Version: 1.0.0
# Created: 2026-03-05
#
# PURPOSE: When a new slice file is written with LINEAR_ISSUE: TBD (or missing),
#          this hook automatically creates the Linear issue and back-fills the
#          file. If Linear is not configured for the repo, it prints wizard
#          instructions for the agent to ask the user.
#
# TRIGGERS: PostToolUse on Write (registered in .claude/settings.json)
#
# EXIT CODES:
#   0 = Always (PostToolUse should never block after a completed Write)
#
# DEPENDENCIES:
#   - .edge-agentic/lib/linear-auto-create-issue.sh (via direct path or symlink)
#   - jq, yq (for JSON/YAML processing)
#   - curl (for Linear API)
# =============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------
# Parse hook input
# ---------------------------------------------------------------------------
INPUT=$(cat)
TOOL_NAME=$(printf '%s' "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null || true)
FILE_PATH=$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null || true)

# ---------------------------------------------------------------------------
# Gate 1: Only process Write operations (not Edit)
# Edit to an existing slice shouldn't trigger new issue creation
# ---------------------------------------------------------------------------
if [[ "$TOOL_NAME" != "Write" ]]; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Gate 2: Only process slice files
# ---------------------------------------------------------------------------
if [[ ! "$FILE_PATH" =~ (planning/slices/|slices/|\.edge-agentic/local/slices/).*\.md$ ]]; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Gate 3: Only process DEV/QA/PM/PROCESS slices (not arbitrary markdown)
# ---------------------------------------------------------------------------
if [[ ! "$FILE_PATH" =~ -(DEV|QA|PM|PROCESS)-[0-9]+\.md$ ]]; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Resolve project root and full file path
# ---------------------------------------------------------------------------
PROJECT_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

FULL_PATH="$FILE_PATH"
if [[ "$FILE_PATH" != /* ]]; then
    FULL_PATH="$PROJECT_ROOT/$FILE_PATH"
fi

if [[ ! -f "$FULL_PATH" ]]; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Gate 4: Check if LINEAR_ISSUE is already set (not TBD, not empty)
# ---------------------------------------------------------------------------
HAS_LINEAR_ISSUE=false

# Check YAML frontmatter format
if head -1 "$FULL_PATH" | grep -q '^---$'; then
    LI=$(sed -n '1,/^---$/{ /^linear_identifier:/p }' "$FULL_PATH" | head -1 | sed 's/^[^:]*:[[:space:]]*//' | tr -d '"'"'" 2>/dev/null || true)
    if [[ -n "$LI" && "$LI" != "TBD" && "$LI" != "null" && "$LI" != "~" ]]; then
        HAS_LINEAR_ISSUE=true
    fi
fi

# Check legacy metadata format
if [[ "$HAS_LINEAR_ISSUE" == "false" ]]; then
    LI=$(grep -E '^LINEAR_ISSUE: ' "$FULL_PATH" 2>/dev/null | head -1 | awk '{print $2}' || true)
    if [[ -n "$LI" && "$LI" != "TBD" && "$LI" != "null" ]]; then
        HAS_LINEAR_ISSUE=true
    fi
fi

if [[ "$HAS_LINEAR_ISSUE" == "true" ]]; then
    exit 0  # Already has a Linear issue — nothing to do
fi

# ==========================================================================
# AUTO-CREATE: Slice needs a Linear issue
# ==========================================================================

# Locate the library (canonical repo → downstream repo via symlink)
LIB_SCRIPT=""
if [[ -f "$PROJECT_ROOT/.edge-agentic/lib/linear-auto-create-issue.sh" ]]; then
    LIB_SCRIPT="$PROJECT_ROOT/.edge-agentic/lib/linear-auto-create-issue.sh"
elif [[ -f "$PROJECT_ROOT/.edge-agentic/system/lib/linear-auto-create-issue.sh" ]]; then
    LIB_SCRIPT="$PROJECT_ROOT/.edge-agentic/system/lib/linear-auto-create-issue.sh"
fi

if [[ -z "$LIB_SCRIPT" ]]; then
    # Library not found — degrade gracefully
    printf '\n'
    printf '  [INFO] Linear auto-create library not found.\n'
    printf '  Slice written without Linear issue — create manually or run /setup-edge-dev.\n'
    printf '\n'
    exit 0
fi

# shellcheck source=/dev/null
source "$LIB_SCRIPT"

# Capture both stdout (JSON) and stderr (progress)
RESULT_JSON=""
RESULT_STDERR=""
RESULT_JSON=$(auto_create_linear_issue "$FULL_PATH" "$PROJECT_ROOT" 2>/tmp/.lac-stderr-$$ || true)
EXIT_CODE=${PIPESTATUS[0]:-$?}
RESULT_STDERR=$(cat /tmp/.lac-stderr-$$ 2>/dev/null || true)
rm -f /tmp/.lac-stderr-$$

case $EXIT_CODE in
    0)
        # SUCCESS: Issue created and file back-filled
        IDENTIFIER=$(printf '%s' "$RESULT_JSON" | jq -r '.identifier // empty' 2>/dev/null || true)
        EXISTING=$(printf '%s' "$RESULT_JSON" | jq -r '.existing // false' 2>/dev/null || true)
        printf '\n'
        if [[ "$EXISTING" == "true" ]]; then
            printf '  ✓ LINEAR AUTO-LINK: Found existing issue %s — linked to slice.\n' "$IDENTIFIER"
        else
            printf '  ✓ LINEAR AUTO-CREATE: Issue %s created and linked to slice.\n' "$IDENTIFIER"
        fi
        printf '\n'
        ;;
    2)
        # NO CONFIG: Print wizard instructions for the agent
        REPO_NAME=$(basename "$PROJECT_ROOT" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g')
        printf '\n'
        printf '  ════════════════════════════════════════════════════════════\n'
        printf '  LINEAR CONFIGURATION REQUIRED\n'
        printf '  ════════════════════════════════════════════════════════════\n'
        printf '\n'
        printf '  This slice needs a Linear issue but no config exists for\n'
        printf '  this repository.\n'
        printf '\n'
        printf '  Use AskUserQuestion to ask the user:\n'
        printf '\n'
        printf '    1. Which Linear account should this project use?\n'

        # Detect available accounts from env vars
        for key_var in LINEAR_API_KEY_BRADHEITMANN LINEAR_API_KEY_PERSONAL \
                       LINEAR_API_KEY_OKOABH LINEAR_API_KEY_OKOA \
                       LINEAR_API_KEY_IMDOGZILLA LINEAR_API_KEY_FORMULIST; do
            if [[ -n "${!key_var:-}" ]]; then
                printf '       - %s (key available)\n' "$key_var"
            fi
        done

        printf '\n'
        printf '    2. Which team in that account?\n'
        printf '    3. Use existing project or create new one?\n'
        printf '       Suggestion: create project named "%s"\n' "$REPO_NAME"
        printf '    4. OR: "I don'\''t want to use Linear for this project"\n'
        printf '\n'
        printf '  After user responds:\n'
        printf '    - If Linear chosen: create .edge-agentic/local/integrations/linear-config.yaml\n'
        printf '    - If no Linear: create .edge-agentic/local/preferences.yaml with linear.enabled: false\n'
        printf '    - Then re-save the slice file to trigger auto-create.\n'
        printf '\n'
        printf '  ════════════════════════════════════════════════════════════\n'
        printf '\n'
        ;;
    3)
        # OPTED OUT: Silent — user chose not to use Linear
        ;;
    *)
        # FAILURE: API error or other issue
        printf '\n'
        printf '  ⚠ Linear auto-create failed: %s\n' "$RESULT_STDERR"
        printf '  Slice saved without Linear issue. Create manually or fix the error.\n'
        printf '\n'
        ;;
esac

exit 0
