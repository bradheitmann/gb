# Observed Layering

## Layer Structure

Based on crate dependencies and file imports, the following layers are observed:

```
┌─────────────────────────────────────────────────────────────┐
│                      LAYER 5: Binaries                      │
│                                                             │
│   g3 (src/main.rs)           g3-console (src/main.rs)       │
│         │                           │                       │
└─────────┼───────────────────────────┼───────────────────────┘
          │                           │
          ▼                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   LAYER 4: Application                      │
│                                                             │
│   g3-cli                      g3-console (lib)              │
│   ├── Interactive CLI         ├── Web API                   │
│   ├── TUI rendering           ├── Process management        │
│   └── Session management      └── Log parsing               │
│                                                             │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                   LAYER 3: Orchestration                    │
│                                                             │
│   g3-planner                  g3-ensembles                  │
│   ├── Planning workflow       ├── Flock mode                │
│   ├── Git integration         └── Multi-agent status        │
│   └── LLM coordination                                      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                      LAYER 2: Core                          │
│                                                             │
│   g3-core                                                   │
│   ├── Agent engine (lib.rs)                                 │
│   ├── Context window management                             │
│   ├── Tool dispatch & execution                             │
│   ├── Streaming parser                                      │
│   ├── Error handling & retry                                │
│   └── Code search (tree-sitter)                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                   LAYER 1: Foundation                       │
│                                                             │
│   g3-providers        g3-config       g3-execution          │
│   ├── Anthropic       ├── TOML        └── Coverage tools    │
│   ├── Databricks      │   parsing                           │
│   ├── OpenAI          └── Settings    g3-computer-control   │
│   ├── Embedded                        ├── Platform (macOS/  │
│   └── OAuth                           │   Linux/Windows)    │
│                                       ├── OCR               │
│                                       ├── WebDriver         │
│                                       └── MacAX             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Layer Dependency Rules

| From Layer | May Depend On |
|------------|---------------|
| 5 (Binaries) | 4, 1 |
| 4 (Application) | 3, 2, 1 |
| 3 (Orchestration) | 2, 1 |
| 2 (Core) | 1 |
| 1 (Foundation) | (none - leaf) |

## Observed Violations

**None detected.**

All observed dependencies follow the layering rules:
- Higher layers depend only on lower layers
- No skip-level violations that bypass intermediate layers inappropriately
- No upward dependencies

## Cross-Cutting Concerns

### g3-config
Used by: g3-cli, g3-core, g3-planner, g3-ensembles
- Provides `Config` struct across all layers
- Appropriate as foundation-level shared configuration

### g3-providers
Used by: g3, g3-cli, g3-core, g3-planner
- Provides `Message`, `MessageRole`, `LLMProvider` types
- Core abstraction for LLM communication
- Appropriate as foundation-level abstraction

### UiWriter trait (g3-core/src/ui_writer.rs)
Used by: 10 files within g3-core, implemented by g3-cli
- Abstraction for output rendering
- Defined in core, implemented in application layer
- Follows dependency inversion principle

## Module Groupings by Path Convention

| Path Pattern | Purpose | Crates |
|--------------|---------|--------|
| `src/tools/*` | Tool implementations | g3-core |
| `src/api/*` | HTTP API handlers | g3-console |
| `src/models/*` | Data structures | g3-console |
| `src/process/*` | Process management | g3-console |
| `src/platform/*` | OS-specific code | g3-computer-control |
| `src/ocr/*` | OCR implementations | g3-computer-control |
| `src/webdriver/*` | Browser automation | g3-computer-control |
| `src/macax/*` | macOS accessibility | g3-computer-control |
| `src/code_search/*` | Tree-sitter search | g3-core |

## Confidence Assessment

| Aspect | Confidence | Notes |
|--------|------------|-------|
| Crate-level layering | High | Derived from Cargo.toml |
| File-level layering | Medium | Based on import analysis |
| Violation detection | Medium | May miss dynamic/conditional deps |
| Module groupings | High | Based on path conventions |
