# G3 Control Commands

**Last updated**: January 2025  
**Source of truth**: `crates/g3-cli/src/lib.rs`

## Purpose

Control commands are special commands you can use during an interactive G3 session to manage context, refresh documentation, and view statistics. They start with `/` and are processed by the CLI, not sent to the LLM.

## Available Commands

| Command | Description |
|---------|-------------|
| `/compact` | Manually trigger conversation compaction |
| `/thinnify` | Replace large tool results with file references (first third) |
| `/skinnify` | Full context thinning (entire context window) |
| `/readme` | Reload README.md and AGENTS.md from disk |
| `/stats` | Show detailed context and performance statistics |
| `/help` | Display all available control commands |

---

## /compact

Manually trigger conversation compaction to reduce context size.

**When to use**:
- Context usage is getting high (70%+)
- You want to start a new phase of work
- Conversation has accumulated irrelevant history

**What it does**:
1. Sends conversation history to LLM for compaction
2. Replaces detailed history with concise summary
3. Preserves key decisions and context
4. Significantly reduces token usage

**Example**:
```
g3> /compact
📝 Compacting conversation history...
✅ Reduced context from 45,000 to 8,000 tokens (82% reduction)
```

**Notes**:
- Summarization uses tokens, so there's a small cost
- Some detail is lost; use before major context shifts
- Auto-triggered at 80% context usage if `auto_compact = true`

---

## /thinnify

Replace large tool results with file references to save context space.

**When to use**:
- Large file contents are consuming context
- Tool outputs are taking up space
- You want to preserve conversation structure but reduce size

**What it does**:
1. Scans the first third of context for large tool results
2. Saves content to `.g3/sessions/<session>/thinned/`
3. Replaces inline content with file reference
4. Preserves the ability to re-read if needed

**Example**:
```
g3> /thinnify
🔧 Thinning context window...
✅ Thinned 3 large tool results, saved 12,000 characters
```

**Notes**:
- Only processes the first third of context (older content)
- Recent tool results are preserved inline
- Auto-triggered at 50%, 60%, 70%, 80% thresholds

---

## /skinnify

Full context thinning - processes the entire context window.

**When to use**:
- Context is critically full
- `/thinnify` wasn't enough
- You need maximum space recovery

**What it does**:
- Same as `/thinnify` but processes entire context
- More aggressive space recovery
- May thin recent tool results too

**Example**:
```
g3> /skinnify
🔧 Full context thinning...
✅ Thinned 8 tool results, saved 35,000 characters
```

**Notes**:
- Use sparingly; may thin content you still need inline
- Consider `/compact` first for better context preservation

---

## /readme

Reload README.md and AGENTS.md from disk without restarting.

**When to use**:
- You've updated project documentation
- AGENTS.md has new instructions
- README.md has changed

**What it does**:
1. Re-reads README.md from workspace root
2. Re-reads AGENTS.md from workspace root
3. Updates the agent's system context
4. New instructions take effect immediately

**Example**:
```
g3> /readme
📖 Reloading documentation...
✅ Loaded README.md (5,234 chars)
✅ Loaded AGENTS.md (2,100 chars)
```

**Notes**:
- Useful during iterative documentation updates
- Changes apply to subsequent messages
- Previous context retains old documentation

---

## /stats

Show detailed context and performance statistics.

**What it shows**:
- Current context usage (tokens and percentage)
- Session duration
- Token usage breakdown
- Tool call metrics
- Thinning and compaction events
- First-token latency statistics

**Example**:
```
g3> /stats
📊 Session Statistics
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Context Usage:     45,230 / 200,000 tokens (22.6%)
Session Duration:  1h 23m 45s
Total Tokens Used: 125,430
Tool Calls:        47 (45 successful, 2 failed)
Thinning Events:   3 (saved 28,000 chars)
Summarizations:    1 (saved 35,000 chars)
Avg First Token:   1.2s
```

---

## /help

Display all available control commands with brief descriptions.

**Example**:
```
g3> /help
📚 Available Commands:
  /compact   - Summarize conversation to reduce context
  /thinnify  - Replace large tool results with file refs
  /skinnify  - Full context thinning (entire window)
  /readme    - Reload README.md and AGENTS.md
  /stats     - Show context and performance statistics
  /help      - Show this help message
```

---

## Context Management Strategy

G3 automatically manages context, but manual intervention can help:

### Proactive Management

1. **Check stats regularly**: Use `/stats` to monitor usage
2. **Thin early**: Use `/thinnify` before hitting thresholds
3. **Compact at transitions**: Use `/compact` when switching tasks

### Reactive Management

When context gets high:

1. **50-70%**: Consider `/thinnify`
2. **70-80%**: Use `/compact`
3. **80-90%**: Use `/skinnify` then `/compact`
4. **90%+**: Auto-compaction triggers

### Best Practices

- **Long sessions**: Compact periodically to maintain quality
- **Large files**: Thin after reading large codebases
- **Documentation updates**: Use `/readme` instead of restarting
- **Before complex tasks**: Ensure adequate context space

---

## Automatic Context Management

G3 performs automatic context management:

| Threshold | Action |
|-----------|--------|
| 50% | Thin oldest third of context |
| 60% | Thin oldest third of context |
| 70% | Thin oldest third of context |
| 80% | Auto-compaction (if `auto_compact = true`) |
| 90% | Aggressive thinning before tool calls |

Manual commands give you finer control over when and how this happens.
