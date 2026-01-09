# Edge Agentic Customizations

Tracks divergences from the global edge_agentic_orchestration_system.

**Principle:** Reference global resources. Only customize when project-specific context is required.

## Customized Resources

| Resource | Type | Reason | Date |
|----------|------|--------|------|
| (none) | - | Using global | - |

## How to Customize

1. Copy resource from `system/` to `local/`
2. Modify for your project context
3. Add entry to this table
4. Use local version instead of global

## Sync with Global Updates

```bash
# Check if global version changed
diff .edge-agentic/system/agents/dev-agent.md .edge-agentic/local/prompts/dev-agent-custom.md

# Review changes and merge if valuable
```

---

**Setup Date:** 2026-01-09
**Setup By:** /setup-edge-dev
