#!/usr/bin/env bash
# =============================================================================
# PreToolUse Hook: --no-verify Authorization Gate
# Version: 1.0.0
# Created: 2026-03-05
#
# PURPOSE: Block any git command containing --no-verify (or the equivalent
#          short flag -n for git commit) unless the agent has obtained
#          explicit user permission in the current session.
#
# POLICY:
#   No agent role (Dev, QA, PM) is authorized to use --no-verify
#   unilaterally. The agent MUST ask the user with AskUserQuestion.
#   The agent's question MUST contain the literal string "--no-verify".
#   If the user says yes, the agent re-runs the command prefixed with
#   EA_ALLOW_NO_VERIFY=1.
#
#   Self-authorization (setting EA_ALLOW_NO_VERIFY=1 without asking the
#   user) is a constitutional violation — it will be visible in the
#   session transcript and treated as a policy breach.
#
# TRIGGERS: PreToolUse on Bash commands
#
# EXIT CODES:
#   0 = Allow (no --no-verify detected, or EA_ALLOW_NO_VERIFY=1 set)
#   2 = Block (--no-verify detected without authorization)
# =============================================================================

set -uo pipefail

# ---------------------------------------------------------------------------
# Parse hook input
# ---------------------------------------------------------------------------
INPUT=$(cat)
COMMAND=$(printf '%s' "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null || true)

# Nothing to check if command is empty
[[ -z "$COMMAND" ]] && exit 0

# ---------------------------------------------------------------------------
# Detection: --no-verify flag in any git command
#
# Catches:
#   git commit --no-verify
#   git commit -n                   (short form)
#   git push --no-verify
#   GIT_AUTHOR_NAME=x git commit --no-verify -m "msg"
#   git commit --no-verify --amend
#
# Does NOT catch (correctly):
#   git commit -m "$(cat <<'EOF'   ← heredoc body stripped before scan
#     explanation of --no-verify
#   EOF
#   )"
# ---------------------------------------------------------------------------

# Strip heredoc bodies before scanning — content between <<'EOF'/<<EOF and
# the closing EOF marker is literal data, not command flags.
SCAN_COMMAND=$(printf '%s' "$COMMAND" | awk '
    /<<['"'"'"]*[A-Z_]+['"'"'"]*/ { print; in_heredoc=1; next }
    /^[A-Z_]+$/ && in_heredoc      { in_heredoc=0; next }
    in_heredoc                      { next }
    { print }
')

NO_VERIFY_DETECTED=false

if printf '%s' "$SCAN_COMMAND" | grep -qE '(^|[[:space:]])git[[:space:]]'; then
    # Check for --no-verify flag in the non-heredoc portions
    if printf '%s' "$SCAN_COMMAND" | grep -qE -- '--no-verify'; then
        NO_VERIFY_DETECTED=true
    fi
    # Check for -n short flag on git commit specifically
    # Must be: git commit ... -n (not part of a longer flag like -nm or -nv)
    if printf '%s' "$SCAN_COMMAND" | grep -qE '(^|[[:space:]])git[[:space:]]+commit\b' &&
       printf '%s' "$SCAN_COMMAND" | grep -qE '(^|[[:space:]])-[[:alpha:]]*n([[:space:]]|$)'; then
        NO_VERIFY_DETECTED=true
    fi
fi

[[ "$NO_VERIFY_DETECTED" == "false" ]] && exit 0

# ---------------------------------------------------------------------------
# Authorization check
# ---------------------------------------------------------------------------

if [[ "${EA_ALLOW_NO_VERIFY:-}" == "1" ]]; then
    # User has granted permission this invocation — log it and allow
    printf '\n'
    printf '  ⚠️  --no-verify authorized (EA_ALLOW_NO_VERIFY=1)\n'
    printf '  This bypass is visible in the session transcript.\n'
    printf '\n'
    exit 0
fi

# ---------------------------------------------------------------------------
# BLOCKED — print instructions and exit 2
# ---------------------------------------------------------------------------

RED=$'\033[0;31m'
YELLOW=$'\033[1;33m'
BOLD=$'\033[1m'
BLUE=$'\033[0;34m'
NC=$'\033[0m'

printf '\n'
printf '%s' "${BOLD}${RED}"
printf '══════════════════════════════════════════════════════════════\n'
printf '  🚫 BLOCKED: --no-verify requires explicit user authorization\n'
printf '══════════════════════════════════════════════════════════════\n'
printf '%s' "${NC}"
printf '\n'
printf '  %sCommand blocked:%s %s\n' "${YELLOW}" "${NC}" "$COMMAND"
printf '\n'
printf '  %s--no-verify bypasses ALL commit hooks:%s\n' "${BOLD}" "${NC}"
printf '    - Linear issue enforcement\n'
printf '    - Secrets scanner\n'
printf '    - Evidence bundle validation\n'
printf '    - Constitutional format checks\n'
printf '    - DEV/QA pair enforcement\n'
printf '\n'
printf '  %sNo agent role is authorized to use --no-verify unilaterally.%s\n' "${RED}" "${NC}"
printf '\n'
printf '  %sRequired steps to proceed:%s\n' "${BOLD}" "${NC}"
printf '\n'
printf '  %s[1]%s Use the AskUserQuestion tool.\n' "${BLUE}" "${NC}"
printf '  %s[2]%s Your message MUST contain the literal text "--no-verify".\n' "${BLUE}" "${NC}"
printf '       (e.g. "Can I use --no-verify to bypass commit hooks because...")\n'
printf '  %s[3]%s If user says yes, re-run prefixed with EA_ALLOW_NO_VERIFY=1:\n' "${BLUE}" "${NC}"
printf '\n'
printf '       %sEA_ALLOW_NO_VERIFY=1 %s%s\n' "${YELLOW}" "$COMMAND" "${NC}"
printf '\n'
printf '  %s[4]%s If user says no, find an alternative path.\n' "${BLUE}" "${NC}"
printf '\n'
printf '  %sWARNING:%s Self-authorizing (setting EA_ALLOW_NO_VERIFY=1 without\n' "${RED}" "${NC}"
printf '  asking the user) is a constitutional violation visible in the\n'
printf '  session transcript and subject to review.\n'
printf '\n'
printf '%s══════════════════════════════════════════════════════════════%s\n' "${BOLD}${RED}" "${NC}"
printf '\n'

exit 2
