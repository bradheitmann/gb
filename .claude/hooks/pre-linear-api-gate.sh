#!/usr/bin/env bash
# =============================================================================
# PreToolUse Hook: Linear API Gate
# Version: 1.0.0
# Created: 2026-02-17
#
# PURPOSE: Block Linear API calls (issueCreate, issueUpdate, issueDelete)
#          unless the agent has a valid preflight token from
#          scripts/linear-preflight-check.sh.
#
# TRIGGERS: PreToolUse on Bash commands
#
# EXIT CODES:
#   0 = Allow command (not a Linear call, or valid token exists)
#   2 = Block command (Linear API call without valid preflight token)
#
# ENVIRONMENT:
#   CLAUDE_TOOL_INPUT_COMMAND - The Bash command being executed
#   CLAUDE_SESSION_ID         - Current session identifier
#   CLAUDE_PROJECT_DIR        - Project root directory
# =============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
REPO_ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
TOKEN_FILE="$REPO_ROOT/.edge-agentic/local/.linear-preflight-token"

# ---------------------------------------------------------------------------
# Extract command from Claude Code hook protocol
# ---------------------------------------------------------------------------
# Claude Code passes the Bash command via CLAUDE_TOOL_INPUT_COMMAND env var
COMMAND="${CLAUDE_TOOL_INPUT_COMMAND:-}"

# If env var is empty, try reading JSON from stdin (fallback)
if [[ -z "$COMMAND" ]]; then
    if [[ -t 0 ]]; then
        # No stdin, no env var — nothing to check
        exit 0
    fi
    INPUT=$(cat)
    COMMAND=$(echo "$INPUT" | jq -r '.input.command // empty' 2>/dev/null || true)
fi

# No command to check — allow
[[ -z "$COMMAND" ]] && exit 0

# ---------------------------------------------------------------------------
# Pattern matching: Is this a Linear API call?
# ---------------------------------------------------------------------------
is_linear_api_call() {
    local cmd="$1"
    # Convert to lowercase for case-insensitive matching
    local lower_cmd
    lower_cmd=$(echo "$cmd" | tr '[:upper:]' '[:lower:]')

    # Match Linear GraphQL API endpoint
    if [[ "$lower_cmd" == *"linear.app/graphql"* ]]; then
        return 0
    fi

    # Match Linear GraphQL mutations (write operations)
    if [[ "$lower_cmd" == *"issuecreate"* ]] || \
       [[ "$lower_cmd" == *"issueupdate"* ]] || \
       [[ "$lower_cmd" == *"issuedelete"* ]] || \
       [[ "$lower_cmd" == *"issuearchive"* ]] || \
       [[ "$lower_cmd" == *"issuelabelcreate"* ]]; then
        return 0
    fi

    # Match MCP Linear tool calls
    if [[ "$lower_cmd" == *"mcp__linear__create_issue"* ]] || \
       [[ "$lower_cmd" == *"mcp__linear__update_issue"* ]] || \
       [[ "$lower_cmd" == *"mcp__linear__delete_issue"* ]]; then
        return 0
    fi

    # Match Linear SDK/CLI calls
    if [[ "$lower_cmd" == *"linear-cli"* ]] && \
       [[ "$lower_cmd" == *"issue"* ]]; then
        return 0
    fi

    # Not a Linear API call
    return 1
}

# Quick exit for non-Linear commands (most commands hit this path)
if ! is_linear_api_call "$COMMAND"; then
    exit 0
fi

# ---------------------------------------------------------------------------
# Token validation: Does a valid preflight token exist?
# ---------------------------------------------------------------------------

# Check token file exists
if [[ ! -f "$TOKEN_FILE" ]]; then
    cat >&2 <<EOF

╔══════════════════════════════════════════════════════════════════════╗
║  BLOCKED: Linear API call requires preflight check                  ║
╚══════════════════════════════════════════════════════════════════════╝

No preflight token found. You must validate your planned Linear
operations against constitutional policy before making API calls.

Run:
  ./scripts/linear-preflight-check.sh --manifest <your-manifest.json>

The manifest must include all planned Linear operations. See:
  scripts/linear-preflight-check.sh --help

Constitutional references:
  - Amendment 002: config/constitutional/amendments/AMENDMENT-002-LINEAR-LABEL-DISCIPLINE.md
  - Amendment 003: config/constitutional/amendments/AMENDMENT-003-SLICE-SYNC-DISCIPLINE.md

EOF
    exit 2
fi

# Parse token
EXPIRES=""
SESSION_ID_TOKEN=""
MANIFEST_SHA256=""
MANIFEST_PATH=""
while IFS='=' read -r key value; do
    case "$key" in
        EXPIRES) EXPIRES="$value" ;;
        SESSION_ID) SESSION_ID_TOKEN="$value" ;;
        MANIFEST_SHA256) MANIFEST_SHA256="$value" ;;
        MANIFEST_PATH) MANIFEST_PATH="$value" ;;
    esac
done < "$TOKEN_FILE"

# Check token has expiry
if [[ -z "$EXPIRES" ]]; then
    echo "BLOCKED: Preflight token is malformed (missing EXPIRES). Re-run preflight check." >&2
    exit 2
fi

# Check token is not expired
# Convert ISO 8601 to epoch for comparison
if date -j -f "%Y-%m-%dT%H:%M:%SZ" "$EXPIRES" +%s &>/dev/null 2>&1; then
    # macOS BSD date
    EXPIRES_EPOCH=$(date -j -f "%Y-%m-%dT%H:%M:%SZ" "$EXPIRES" +%s 2>/dev/null || echo 0)
    NOW_EPOCH=$(date +%s)
else
    # GNU date
    EXPIRES_EPOCH=$(date -d "$EXPIRES" +%s 2>/dev/null || echo 0)
    NOW_EPOCH=$(date +%s)
fi

if [[ "$NOW_EPOCH" -gt "$EXPIRES_EPOCH" ]]; then
    cat >&2 <<EOF

╔══════════════════════════════════════════════════════════════════════╗
║  BLOCKED: Preflight token expired                                    ║
╚══════════════════════════════════════════════════════════════════════╝

Your preflight token has expired (30-minute TTL).
Token expired at: $EXPIRES

Re-run the preflight check:
  ./scripts/linear-preflight-check.sh --manifest <your-manifest.json>

EOF
    exit 2
fi

# SHA256 validation — block if manifest changed since token was issued
if [[ -n "$MANIFEST_PATH" && -n "$MANIFEST_SHA256" ]]; then
    if [[ -f "$MANIFEST_PATH" ]]; then
        # Compute current SHA256
        if command -v shasum &>/dev/null; then
            CURRENT_SHA256=$(shasum -a 256 "$MANIFEST_PATH" | cut -d' ' -f1)
        elif command -v sha256sum &>/dev/null; then
            CURRENT_SHA256=$(sha256sum "$MANIFEST_PATH" | cut -d' ' -f1)
        else
            CURRENT_SHA256=""
        fi
        if [[ -n "$CURRENT_SHA256" && "$CURRENT_SHA256" != "$MANIFEST_SHA256" ]]; then
            cat >&2 <<EOFSHA

╔══════════════════════════════════════════════════════════════════════╗
║  BLOCKED: Manifest changed since preflight check                     ║
╚══════════════════════════════════════════════════════════════════════╝

The manifest file has been modified since the preflight token was issued.
This invalidates the preflight approval.

Manifest: $MANIFEST_PATH
Expected SHA256: ${MANIFEST_SHA256:0:16}...
Current  SHA256: ${CURRENT_SHA256:0:16}...

Re-run the preflight check:
  ./scripts/linear-preflight-check.sh --manifest $MANIFEST_PATH

EOFSHA
            exit 2
        fi
    else
        echo "WARNING: Manifest file not found at $MANIFEST_PATH — cannot verify SHA256." >&2
    fi
fi

# Session ID mismatch — warn but allow
CURRENT_SESSION="${CLAUDE_SESSION_ID:-}"
if [[ -n "$CURRENT_SESSION" && -n "$SESSION_ID_TOKEN" && "$CURRENT_SESSION" != "$SESSION_ID_TOKEN" ]]; then
    echo "WARNING: Preflight token was generated in a different session ($SESSION_ID_TOKEN). Current: $CURRENT_SESSION" >&2
fi

# Token is valid — allow the command
exit 0
