#!/usr/bin/env bash
# Version: 1.2.0
# Updated: 2026-02-06
# Verify edge-agentic compliance on session start
# Ensures project stays compliant with edge-agentic protocols
#
# v1.2.0 (2026-02-06): Added capability reminder injection to combat attention decay
# v1.1.0 (2026-01-22): Added source repo detection to skip symlink checks in source repo

# Skip if no .edge-agentic directory
[ ! -d .edge-agentic ] && exit 0

# Skip if explicitly disabled
[ "${SKIP_EDGE_AGENTIC_VERIFICATION:-}" = "1" ] && exit 0

# Detect if we're in the SOURCE repo (not a downstream consumer)
IS_SOURCE_REPO=0
if [ -f VERSION ] && grep -q "^protocol:" VERSION 2>/dev/null; then
  # Source repo has VERSION with "protocol:" field
  IS_SOURCE_REPO=1
fi

# ── Repo-Level PM Task List (auto-detect repo name, Doppler-aware) ──
if [[ -n "${CLAUDE_CODE_TASK_LIST_ID:-}" ]]; then
    echo "TASK LIST: $CLAUDE_CODE_TASK_LIST_ID"
elif command -v doppler >/dev/null 2>&1; then
    DOPPLER_VALUE=$(timeout 3 doppler secrets get CLAUDE_CODE_TASK_LIST_ID --plain 2>/dev/null || true)
    if [[ -n "$DOPPLER_VALUE" ]]; then
        echo "TASK LIST: in Doppler -> use: doppler run -- claude"
    else
        REPO_NAME=$(basename "$(pwd)" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g')
        echo "TASK LIST: not set -> use CLAUDE_CODE_TASK_LIST_ID=${REPO_NAME}-PM"
    fi
else
    REPO_NAME=$(basename "$(pwd)" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g')
    echo "TASK LIST: not set -> use CLAUDE_CODE_TASK_LIST_ID=${REPO_NAME}-PM"
fi

# ── Capability Reminder (always output, combats attention decay) ──
echo "TASKS: Use TaskCreate/TaskUpdate/TaskList (NOT TodoWrite)"
echo "PROMPTS: Use /prompt-optimization skill (agent only for complex cases)"

# Silent mode - only output on errors
ERRORS=0
WARNINGS=0

# Check 0: Version check
if [ -L .edge-agentic/system ]; then
  global_path=$(readlink .edge-agentic/system)
  if [ -f "$global_path/VERSION" ] && [ -f .edge-agentic/VERSION ]; then
    global_version=$(grep "^protocol:" "$global_path/VERSION" | awk '{print $2}')
    local_version=$(grep "^protocol_version:" .edge-agentic/VERSION | awk '{print $2}')

    if [ "$local_version" != "$global_version" ]; then
      echo "⚠️  Version mismatch: installed=$local_version, current=$global_version"
      echo "   Run: /setup-edge-dev to upgrade"
      ((WARNINGS++))
    fi
  elif [ ! -f .edge-agentic/VERSION ]; then
    echo "⚠️  No version file (.edge-agentic/VERSION)"
    echo "   Run: /setup-edge-dev to install"
    ((WARNINGS++))
  fi
fi

# Check 1: Verify .edge-agentic/system symlink exists and resolves
# Skip this check if we're in the source repo itself
if [ $IS_SOURCE_REPO -eq 0 ]; then
  if [ ! -L .edge-agentic/system ]; then
    echo "❌ .edge-agentic/system symlink missing"
    echo "   Run: /setup-edge-dev"
    ((ERRORS++))
  elif [ ! -d .edge-agentic/system/src/agents ]; then
    echo "❌ .edge-agentic/system symlink broken or points to old structure"
    echo "   Expected: src/agents/ directory"
    echo "   Run: /setup-edge-dev to fix"
    ((ERRORS++))
  fi
fi

# Check 2: Verify skills are discoverable
if [ -d .claude/skills ]; then
  skill_count=$(find .claude/skills -type l 2>/dev/null | wc -l | tr -d ' ')

  if [ "$skill_count" -lt 5 ]; then
    echo "⚠️  Only $skill_count skills discovered in .claude/skills/"
    echo "   Expected: 30+ skills"
    echo "   Run: /setup-edge-dev to regenerate skill symlinks"
    ((ERRORS++))
  fi

  # Check if skills point to new structure
  for skill_link in .claude/skills/commit .claude/skills/plan .claude/skills/debug; do
    [ ! -L "$skill_link" ] && continue

    target=$(readlink "$skill_link")
    if [[ ! "$target" =~ /src/skills/ ]]; then
      echo "⚠️  Skill symlinks point to old structure (not src/skills/)"
      echo "   Run: /setup-edge-dev to update paths"
      ((ERRORS++))
      break
    fi
  done
fi

# Check 3: Verify CLAUDE.md exists and has edge-agentic protocols
if [ ! -f CLAUDE.md ]; then
  echo "⚠️  CLAUDE.md missing - agents won't know edge-agentic protocols"
  echo "   Run: /setup-edge-dev to generate from template"
  ((ERRORS++))
elif ! grep -q "Edge-Agentic" CLAUDE.md 2>/dev/null; then
  echo "⚠️  CLAUDE.md exists but missing edge-agentic protocols"
  echo "   Run: /setup-edge-dev to add protocols"
  ((ERRORS++))
fi

# Check 4: Intelligence inbox — handled deterministically by auto-read-intelligence-inbox.sh
# (Removed: advisory "run /intelligence-check" replaced by auto-read hook that injects full content)

# Check 5: Detect stale duplicate directories
# Skip this check if we're in the source repo (it has src/agents/, src/templates/ intentionally)
if [ $IS_SOURCE_REPO -eq 0 ]; then
  stale_dirs=""
  [ -d "agents" ] && stale_dirs="$stale_dirs agents/"
  [ -d "commands" ] && stale_dirs="$stale_dirs commands/"
  [ -d "templates" ] && stale_dirs="$stale_dirs templates/"

  if [ -n "$stale_dirs" ]; then
    echo "⚠️  Stale duplicate directories found: $stale_dirs"
    echo "   These conflict with .edge-agentic/system/ structure"
    echo "   Run: /setup-edge-dev to clean up"
    ((ERRORS++))
  fi
fi

# Output result
if [ $ERRORS -gt 0 ]; then
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "⚠️  Edge-Agentic Compliance Issues Detected"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""
  echo "Run /setup-edge-dev to fix all issues automatically."
  echo ""
  echo "To disable this check:"
  echo "  export SKIP_EDGE_AGENTIC_VERIFICATION=1"
  echo ""
fi

exit 0  # Never block session start, just warn
