# Strongly Connected Components Analysis

## Crate-Level SCCs

**Result: No cycles detected**

All 10 crates form a directed acyclic graph (DAG). Each crate is its own trivial SCC of size 1.

### Topological Order (Bottom to Top)

1. **Leaf crates** (no internal dependencies):
   - g3-config
   - g3-providers
   - g3-execution
   - g3-computer-control

2. **Mid-level crates**:
   - g3-core (depends on: g3-providers, g3-config, g3-execution, g3-computer-control)
   - g3-console (standalone, no workspace dependencies)

3. **Higher-level crates**:
   - g3-planner (depends on: g3-providers, g3-core, g3-config)
   - g3-ensembles (depends on: g3-core, g3-config)

4. **Application layer**:
   - g3-cli (depends on: g3-core, g3-config, g3-planner, g3-providers, g3-ensembles)

5. **Root binary**:
   - g3 (depends on: g3-cli, g3-providers)

## File-Level SCCs Within Crates

### g3-core

No non-trivial SCCs detected. Internal module dependencies are acyclic:

```
lib.rs
├── ui_writer.rs
├── context_window.rs
│   └── paths.rs
├── streaming_parser.rs
├── tool_dispatch.rs
│   └── tools/mod.rs
│       ├── executor.rs
│       │   ├── background_process.rs
│       │   ├── paths.rs
│       │   └── webdriver_session.rs
│       ├── file_ops.rs → utils.rs, ui_writer.rs
│       ├── shell.rs → utils.rs, ui_writer.rs
│       ├── misc.rs → ui_writer.rs
│       ├── todo.rs → ui_writer.rs
│       └── webdriver.rs → webdriver_session.rs, ui_writer.rs
├── error_handling.rs
├── retry.rs → error_handling.rs, ui_writer.rs
├── feedback_extraction.rs → ui_writer.rs
├── task_result.rs → context_window.rs
└── provider_config.rs
```

### g3-providers

No non-trivial SCCs. Provider implementations depend on lib.rs types:

```
lib.rs (types: Message, MessageRole, LLMProvider, etc.)
├── anthropic.rs
├── databricks.rs
├── openai.rs
├── embedded.rs
└── oauth.rs
```

### g3-planner

No non-trivial SCCs:

```
lib.rs
├── planner.rs
│   ├── git.rs
│   ├── history.rs
│   ├── llm.rs → prompts.rs
│   └── state.rs
├── code_explore.rs
└── prompts.rs
```

### g3-computer-control

No non-trivial SCCs:

```
lib.rs
├── types.rs
├── platform/mod.rs
│   ├── macos.rs → ocr/mod.rs, types.rs
│   ├── linux.rs → types.rs
│   └── windows.rs → types.rs
├── ocr/mod.rs → types.rs
│   ├── vision.rs → types.rs
│   └── tesseract.rs → types.rs
├── webdriver/mod.rs
│   ├── safari.rs
│   └── chrome.rs
└── macax/mod.rs
    └── controller.rs
```

### g3-console

No non-trivial SCCs:

```
lib.rs
├── models/mod.rs
│   ├── instance.rs
│   └── message.rs
├── api/mod.rs
│   ├── instances.rs → logs.rs, models, process/detector.rs
│   ├── control.rs → models, process/controller.rs
│   ├── logs.rs → logs.rs, process/detector.rs
│   └── state.rs → launch.rs
├── process/mod.rs
│   ├── controller.rs → models
│   └── detector.rs → models
├── logs.rs → models
└── launch.rs
```

### g3-ensembles

No non-trivial SCCs:

```
lib.rs
├── flock.rs → status.rs
└── status.rs
```

### g3-cli

No non-trivial SCCs:

```
lib.rs
├── filter_json.rs
├── ui_writer_impl.rs → filter_json.rs
├── machine_ui_writer.rs
├── retro_tui.rs → theme.rs
├── theme.rs
├── tui.rs
└── simple_output.rs
```

## Summary

| Scope | Non-Trivial SCCs | Largest SCC Size |
|-------|------------------|------------------|
| Crate-level | 0 | 1 (all trivial) |
| File-level (g3-core) | 0 | 1 |
| File-level (g3-providers) | 0 | 1 |
| File-level (g3-planner) | 0 | 1 |
| File-level (g3-computer-control) | 0 | 1 |
| File-level (g3-console) | 0 | 1 |
| File-level (g3-ensembles) | 0 | 1 |
| File-level (g3-cli) | 0 | 1 |

The codebase exhibits clean layered architecture with no circular dependencies.
