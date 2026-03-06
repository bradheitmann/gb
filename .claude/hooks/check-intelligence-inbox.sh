#!/usr/bin/env bash
# check-intelligence-inbox.sh (v2.0.0)
# SessionStart hook: DETERMINISTIC auto-read of intelligence briefs
#
# v2.0.0 (2026-03-05): Rewrote from notification-only to deterministic auto-read.
#   Old behavior: print count + "run /intelligence-check" (advisory, ignored by agents)
#   New behavior: read full brief content into session context (deterministic, cannot be ignored)
#
# v1.0.0 (2026-02-11): Notification-only (count + topic summaries)
#
# Design principle: Hooks are deterministic. This hook READS briefs and injects
# their full content into the agent's context via stdout. It does NOT print
# "run /intelligence-check" — the reading IS the hook.
#
# Priority order:
#   1. PRIORITY-*.md files (blocking escalations from other PMs) — read FIRST
#   2. IB-*.md files sorted by urgency (critical > high > normal > low)
#
# After reading, briefs are moved to inbox-archive/ with read timestamp.
# Non-blocking: always exits 0

set -uo pipefail

INBOX_DIR=".edge-agentic/local/intelligence/inbox"
ARCHIVE_DIR=".edge-agentic/local/intelligence/inbox-archive"
MUTE_FILE=".edge-agentic/local/intelligence/MUTED"

# Skip if no intelligence directory (repo not configured for IDN)
[ ! -d "$INBOX_DIR" ] && exit 0

# Skip if muted (PM requested DND — create MUTED file to suppress)
[ -f "$MUTE_FILE" ] && exit 0

# Ensure archive directory exists
mkdir -p "$ARCHIVE_DIR"

# Auto-expire briefs older than 30 days (stale intelligence is worse than none)
find "$INBOX_DIR" -name "*.md" -not -name ".gitkeep" -type f -mtime +30 2>/dev/null | while read -r stale; do
  mv "$stale" "$ARCHIVE_DIR/" 2>/dev/null
done

# Collect briefs into arrays
PRIORITY_FILES=()
IB_FILES=()

while IFS= read -r -d '' f; do
  PRIORITY_FILES+=("$f")
done < <(find "$INBOX_DIR" -name "PRIORITY-*.md" -type f -print0 2>/dev/null | sort -z)

while IFS= read -r -d '' f; do
  IB_FILES+=("$f")
done < <(find "$INBOX_DIR" -name "IB-*.md" -type f -print0 2>/dev/null | sort -z)

TOTAL=$(( ${#PRIORITY_FILES[@]} + ${#IB_FILES[@]} ))

# Nothing to read
[ "$TOTAL" -eq 0 ] && exit 0

# ── Header ──
echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║  INTELLIGENCE INBOX: $TOTAL brief(s) — AUTO-READ ACTIVE             ║"
echo "║  Briefs are injected below. No /intelligence-check needed.   ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# ── PRIORITY briefs (BLOCKING — act on these before any other work) ──
if [ ${#PRIORITY_FILES[@]} -gt 0 ]; then
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  PRIORITY ESCALATION(S) — ACT ON THESE BEFORE ANY OTHER WORK"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""

  for brief in "${PRIORITY_FILES[@]}"; do
    brief_name=$(basename "$brief")
    echo "──── $brief_name ────"
    echo ""
    cat "$brief"
    echo ""
    echo "──── END $brief_name ────"
    echo ""
    # Archive with read timestamp
    mv "$brief" "$ARCHIVE_DIR/$(date +%Y%m%d-%H%M%S)-${brief_name}" 2>/dev/null
  done
fi

# ── Regular IB briefs (sorted by urgency: critical > high > normal > low) ──
if [ ${#IB_FILES[@]} -gt 0 ]; then
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  INTELLIGENCE BRIEFS (${#IB_FILES[@]})"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""

  # Track which files we've already output (by index)
  declare -A printed

  for urgency in critical high normal low; do
    for i in "${!IB_FILES[@]}"; do
      brief="${IB_FILES[$i]}"
      [ ! -f "$brief" ] && continue
      [ -n "${printed[$i]:-}" ] && continue

      # Extract urgency from YAML frontmatter or key-value metadata
      brief_urgency=$(grep -m1 "^urgency:" "$brief" 2>/dev/null | sed 's/^urgency:[[:space:]]*//' | tr -d '"' | tr '[:upper:]' '[:lower:]')
      # Fallback to priority field
      if [ -z "$brief_urgency" ]; then
        brief_urgency=$(grep -m1 "^priority:" "$brief" 2>/dev/null | sed 's/^priority:[[:space:]]*//' | tr -d '"' | tr '[:upper:]' '[:lower:]')
      fi

      [ "$brief_urgency" != "$urgency" ] && continue

      brief_name=$(basename "$brief")
      echo "──── $brief_name [$urgency] ────"
      echo ""
      cat "$brief"
      echo ""
      echo "──── END $brief_name ────"
      echo ""
      mv "$brief" "$ARCHIVE_DIR/$(date +%Y%m%d-%H%M%S)-${brief_name}" 2>/dev/null
      printed[$i]=1
    done
  done

  # Catch any briefs without a recognized urgency field
  for i in "${!IB_FILES[@]}"; do
    brief="${IB_FILES[$i]}"
    [ ! -f "$brief" ] && continue
    [ -n "${printed[$i]:-}" ] && continue

    brief_name=$(basename "$brief")
    echo "──── $brief_name [unclassified] ────"
    echo ""
    cat "$brief"
    echo ""
    echo "──── END $brief_name ────"
    echo ""
    mv "$brief" "$ARCHIVE_DIR/$(date +%Y%m%d-%H%M%S)-${brief_name}" 2>/dev/null
    printed[$i]=1
  done
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  All $TOTAL brief(s) read and archived. Intelligence is now in context."
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

exit 0
