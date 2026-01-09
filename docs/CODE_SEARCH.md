# G3 Code Search Guide

**Last updated**: January 2025  
**Source of truth**: `crates/g3-core/src/code_search/`, `crates/g3-core/src/tool_definitions.rs`

## Purpose

G3 includes a syntax-aware code search tool powered by tree-sitter. Unlike text-based search (grep), it understands code structure and finds actual functions, classes, methods, and other constructs—ignoring matches in comments and strings.

## Why Use Code Search?

| Feature | grep/ripgrep | code_search |
|---------|--------------|-------------|
| Finds text in comments | ✅ | ❌ |
| Finds text in strings | ✅ | ❌ |
| Understands code structure | ❌ | ✅ |
| Finds function definitions | Regex needed | Native |
| Finds class hierarchies | ❌ | ✅ |
| Language-aware | ❌ | ✅ |

**Use code_search when**:
- Finding function/method definitions
- Finding class/struct declarations
- Searching for specific code constructs
- Need accurate results without false positives

**Use grep when**:
- Searching non-code files (logs, markdown)
- Simple string searches
- Searching comments or documentation
- Regex for text patterns

## Supported Languages

- Rust
- Python
- JavaScript
- TypeScript
- Go
- Java
- C
- C++
- Kotlin

## Basic Usage

```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "my_search",
    "query": "(function_item name: (identifier) @name)",
    "language": "rust"
  }]
}}
```

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `searches` | array | Yes | Array of search objects (max 20) |
| `max_concurrency` | integer | No | Parallel searches (default: 4) |
| `max_matches_per_search` | integer | No | Max matches (default: 500) |

### Search Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Label for this search |
| `query` | string | Yes | Tree-sitter query (S-expression) |
| `language` | string | Yes | Programming language |
| `paths` | array | No | Paths to search (default: current dir) |
| `context_lines` | integer | No | Lines of context (0-20, default: 0) |

## Query Syntax

Tree-sitter queries use S-expression syntax. The basic pattern is:

```
(node_type field: (child_type) @capture_name)
```

- `node_type`: The AST node to match
- `field`: Optional field name
- `child_type`: Type of child node
- `@capture_name`: Name for the captured node

## Common Query Patterns

### Rust

```lisp
;; All functions
(function_item name: (identifier) @name)

;; Async functions
(function_item (function_modifiers) name: (identifier) @name)

;; Structs
(struct_item name: (type_identifier) @name)

;; Enums
(enum_item name: (type_identifier) @name)

;; Impl blocks
(impl_item type: (type_identifier) @name)

;; Trait definitions
(trait_item name: (type_identifier) @name)

;; Macros
(macro_definition name: (identifier) @name)

;; Constants
(const_item name: (identifier) @name)

;; Static variables
(static_item name: (identifier) @name)

;; Type aliases
(type_item name: (type_identifier) @name)

;; Modules
(mod_item name: (identifier) @name)
```

### Python

```lisp
;; Functions
(function_definition name: (identifier) @name)

;; Async functions
(function_definition name: (identifier) @name) @fn

;; Classes
(class_definition name: (identifier) @name)

;; Methods (functions inside classes)
(class_definition
  body: (block
    (function_definition name: (identifier) @name)))

;; Decorators
(decorator) @decorator

;; Imports
(import_statement) @import
(import_from_statement) @import
```

### JavaScript / TypeScript

```lisp
;; Function declarations
(function_declaration name: (identifier) @name)

;; Arrow functions assigned to variables
(variable_declarator
  name: (identifier) @name
  value: (arrow_function))

;; Classes
(class_declaration name: (identifier) @name)

;; Methods
(method_definition name: (property_identifier) @name)

;; Exports
(export_statement) @export

;; Imports
(import_statement) @import
```

### Go

```lisp
;; Functions
(function_declaration name: (identifier) @name)

;; Methods
(method_declaration name: (field_identifier) @name)

;; Structs
(type_declaration
  (type_spec name: (type_identifier) @name
    type: (struct_type)))

;; Interfaces
(type_declaration
  (type_spec name: (type_identifier) @name
    type: (interface_type)))
```

### Java

```lisp
;; Classes
(class_declaration name: (identifier) @name)

;; Interfaces
(interface_declaration name: (identifier) @name)

;; Methods
(method_declaration name: (identifier) @name)

;; Constructors
(constructor_declaration name: (identifier) @name)

;; Fields
(field_declaration
  declarator: (variable_declarator name: (identifier) @name))
```

### C / C++

```lisp
;; Functions
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @name))

;; Structs (C)
(struct_specifier name: (type_identifier) @name)

;; Classes (C++)
(class_specifier name: (type_identifier) @name)

;; Namespaces (C++)
(namespace_definition name: (identifier) @name)
```

## Advanced Queries

### Wildcards

Use `_` to match any node:

```lisp
;; Any function with any name
(function_item name: (_) @name)
```

### Alternatives

Match multiple patterns:

```lisp
;; Functions or methods
[(function_item) (impl_item)] @item
```

### Predicates

Filter matches:

```lisp
;; Functions starting with "test_"
(function_item name: (identifier) @name
  (#match? @name "^test_"))

;; Functions NOT starting with "_"
(function_item name: (identifier) @name
  (#not-match? @name "^_"))
```

### Nested Matches

```lisp
;; Methods inside impl blocks
(impl_item
  body: (declaration_list
    (function_item name: (identifier) @method_name)))
```

## Batch Searches

Run multiple searches in parallel:

```json
{"tool": "code_search", "args": {
  "searches": [
    {
      "name": "functions",
      "query": "(function_item name: (identifier) @name)",
      "language": "rust"
    },
    {
      "name": "structs",
      "query": "(struct_item name: (type_identifier) @name)",
      "language": "rust"
    },
    {
      "name": "tests",
      "query": "(function_item name: (identifier) @name (#match? @name \"^test_\"))",
      "language": "rust",
      "paths": ["tests/"]
    }
  ],
  "max_concurrency": 4
}}
```

## Context Lines

Include surrounding code:

```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "functions",
    "query": "(function_item name: (identifier) @name)",
    "language": "rust",
    "context_lines": 3
  }]
}}
```

This shows 3 lines before and after each match.

## Path Filtering

Search specific directories:

```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "core_functions",
    "query": "(function_item name: (identifier) @name)",
    "language": "rust",
    "paths": ["src/core", "src/lib.rs"]
  }]
}}
```

## Output Format

Results include:
- File path
- Line number
- Matched code
- Context (if requested)

```
=== functions (15 matches) ===

src/lib.rs:42
  fn process_request(req: Request) -> Response {

src/lib.rs:78
  fn handle_error(err: Error) -> Result<()> {

src/utils.rs:15
  fn format_output(data: &str) -> String {
```

## Tips

### Finding the Right Query

1. **Start simple**: Begin with basic node types
2. **Use AST explorer**: Understand your language's AST
3. **Iterate**: Refine queries based on results

### Performance

- **Limit paths**: Search specific directories when possible
- **Use concurrency**: Batch related searches
- **Set max_matches**: Prevent overwhelming output

### Debugging Queries

If a query returns no results:
1. Check language spelling (lowercase)
2. Verify node type names for your language
3. Start with simpler query, add constraints
4. Check if files exist in search paths

## Examples by Task

### Find all public functions in Rust

```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "public_fns",
    "query": "(function_item (visibility_modifier) name: (identifier) @name)",
    "language": "rust"
  }]
}}
```

### Find all test functions

```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "tests",
    "query": "(function_item name: (identifier) @name (#match? @name \"^test_\"))",
    "language": "rust",
    "paths": ["tests/"]
  }]
}}
```

### Find all API endpoints (Python Flask)

```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "routes",
    "query": "(decorated_definition (decorator) @dec (function_definition name: (identifier) @name))",
    "language": "python"
  }]
}}
```

### Find all React components

```json
{"tool": "code_search", "args": {
  "searches": [{
    "name": "components",
    "query": "(function_declaration name: (identifier) @name (#match? @name \"^[A-Z]\"))",
    "language": "javascript",
    "paths": ["src/components/"]
  }]
}}
```
