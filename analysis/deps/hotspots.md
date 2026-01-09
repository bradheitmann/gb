# Dependency Hotspots

## Definition

Hotspots are code artifacts with disproportionate coupling, measured by:
- **Fan-in**: Number of files/crates that depend on this artifact
- **Fan-out**: Number of files/crates this artifact depends on
- **Cross-group edges**: Dependencies that cross module/crate boundaries

## Crate-Level Hotspots

### High Fan-In Crates

| Crate | Fan-In | Evidence |
|-------|--------|----------|
| g3-config | 5 | Depended on by: g3-cli, g3-core, g3-planner, g3-ensembles, (tests) |
| g3-providers | 4 | Depended on by: g3, g3-cli, g3-core, g3-planner |
| g3-core | 3 | Depended on by: g3-cli, g3-planner, g3-ensembles |

### High Fan-Out Crates

| Crate | Fan-Out | Evidence |
|-------|---------|----------|
| g3-cli | 5 | Depends on: g3-core, g3-config, g3-planner, g3-providers, g3-ensembles |
| g3-core | 4 | Depends on: g3-providers, g3-config, g3-execution, g3-computer-control |
| g3-planner | 3 | Depends on: g3-providers, g3-core, g3-config |

## File-Level Hotspots

### High Fan-In Files

| File | Fan-In | Importing Files |
|------|--------|----------------|
| `g3-core/src/ui_writer.rs` | 10 | lib.rs, tool_dispatch.rs, retry.rs, feedback_extraction.rs, tools/file_ops.rs, tools/shell.rs, tools/misc.rs, tools/todo.rs, tools/webdriver.rs, tools/executor.rs |
| `g3-core/src/lib.rs` | 8 | streaming_parser.rs, feedback_extraction.rs, retry.rs, task_result_comprehensive_tests.rs, (external: g3-cli, g3-planner, g3-ensembles) |
| `g3-providers/src/lib.rs` | 7 | anthropic.rs, databricks.rs, openai.rs, embedded.rs, (external: g3-core, g3-planner, examples) |
| `g3-config/src/lib.rs` | 5 | (external: g3-core, g3-cli, g3-planner, g3-ensembles, tools/executor.rs) |
| `g3-computer-control/src/types.rs` | 5 | platform/macos.rs, platform/linux.rs, platform/windows.rs, ocr/mod.rs, ocr/vision.rs, ocr/tesseract.rs |
| `g3-console/src/models/mod.rs` | 5 | api/instances.rs, api/control.rs, logs.rs, process/controller.rs, process/detector.rs |

### High Fan-Out Files

| File | Fan-Out | Dependencies |
|------|---------|-------------|
| `g3-core/src/tools/executor.rs` | 5 | background_process.rs, paths.rs, ui_writer.rs, webdriver_session.rs, g3-config |
| `g3-core/src/tool_dispatch.rs` | 4 | tools/executor.rs, tools/mod.rs, ui_writer.rs, ToolCall |
| `g3-core/src/retry.rs` | 4 | error_handling.rs, ui_writer.rs, lib.rs (Agent, TaskResult) |
| `g3-planner/src/llm.rs` | 5 | g3-config, g3-core/project, g3-core/Agent, g3-core/error_handling, g3-providers, prompts.rs |
| `g3-console/src/api/instances.rs` | 3 | logs.rs, models, process/detector.rs |

## Cross-Boundary Dependencies

### External Crate Imports (Cross-Crate)

| From Crate | To Crate | Import Count | Key Types |
|------------|----------|--------------|----------|
| g3-core | g3-providers | 5 | Message, MessageRole, Usage, CacheControl, Tool, CompletionRequest, ProviderRegistry |
| g3-core | g3-config | 3 | Config |
| g3-core | g3-computer-control | 2 | WebDriverController, ChromeDriver, SafariDriver, WebElement |
| g3-cli | g3-core | 3 | Agent, UiWriter, DiscoveryOptions, Project, error_handling |
| g3-planner | g3-core | 3 | Agent, Project, error_handling |
| g3-planner | g3-providers | 2 | Message, MessageRole, LLMProvider, CompletionRequest |
| g3-ensembles | g3-core | 1 | (via g3-config) |

## Coupling Metrics Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Max crate fan-in | 5 (g3-config) | - | Expected for config |
| Max crate fan-out | 5 (g3-cli) | - | Expected for CLI |
| Max file fan-in | 10 (ui_writer.rs) | - | Trait abstraction |
| Max file fan-out | 5 (executor.rs, llm.rs) | - | Orchestration files |
| Cross-crate edges | 16 | - | Moderate |

## Observations

1. **ui_writer.rs** has highest file-level fan-in (10 dependents)
   - This is a trait definition, high fan-in is expected
   - Implements dependency inversion pattern

2. **g3-config** has highest crate-level fan-in (5 dependents)
   - Configuration is appropriately centralized
   - No code duplication observed

3. **g3-cli** has highest crate-level fan-out (5 dependencies)
   - Expected for application entry point
   - Orchestrates all major subsystems

4. **tools/executor.rs** has high fan-out within g3-core
   - Central tool execution context
   - Coordinates background processes, paths, webdriver, config

5. **g3-console** is isolated
   - No dependencies on other workspace crates
   - Standalone monitoring application
