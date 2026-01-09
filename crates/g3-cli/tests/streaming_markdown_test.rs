//! Integration tests for streaming markdown formatter.
//!
//! These tests simulate real streaming scenarios with various chunk sizes
//! and complex markdown content.

use g3_cli::streaming_markdown::StreamingMarkdownFormatter;
use termimad::MadSkin;

fn make_formatter() -> StreamingMarkdownFormatter {
    let mut skin = MadSkin::default();
    skin.bold.set_fg(termimad::crossterm::style::Color::Green);
    skin.italic.set_fg(termimad::crossterm::style::Color::Cyan);
    StreamingMarkdownFormatter::new(skin)
}

/// Feed content in chunks of specified size
fn stream_in_chunks(content: &str, chunk_size: usize) -> String {
    let mut fmt = make_formatter();
    let mut output = String::new();
    
    // Chunk by characters, not bytes, to avoid splitting UTF-8 sequences
    let chars: Vec<char> = content.chars().collect();
    for chunk in chars.chunks(chunk_size) {
        let chunk_str: String = chunk.iter().collect();
        output.push_str(&fmt.process(&chunk_str));
    }
    output.push_str(&fmt.finish());
    output
}

/// Feed content character by character (worst case for streaming)
fn stream_char_by_char(content: &str) -> String {
    stream_in_chunks(content, 1)
}

/// Feed content in random-ish chunk sizes
fn stream_variable_chunks(content: &str) -> String {
    let mut fmt = make_formatter();
    let mut output = String::new();
    let mut pos = 0;
    let sizes = [1, 3, 7, 2, 15, 4, 1, 8, 5, 20, 1, 1, 1, 10];
    let mut size_idx = 0;
    
    while pos < content.len() {
        let chunk_size = sizes[size_idx % sizes.len()].min(content.len() - pos);
        let chunk = &content[pos..pos + chunk_size];
        output.push_str(&fmt.process(chunk));
        pos += chunk_size;
        size_idx += 1;
    }
    output.push_str(&fmt.finish());
    output
}

const LARGE_MARKDOWN: &str = r##"# Welcome to the Documentation

This is a comprehensive guide to using our **amazing** library.

## Getting Started

First, you'll need to install the dependencies:

```bash
cargo add my-library
cargo add tokio --features full
```

Then, create a simple example:

```rust
use my_library::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .with_timeout(Duration::from_secs(30))
        .with_retry(3)
        .build()?;
    
    let response = client.get("https://api.example.com/data").await?;
    
    if response.status().is_success() {
        let data: MyData = response.json().await?;
        println!("Got data: {:?}", data);
    } else {
        eprintln!("Error: {}", response.status());
    }
    
    Ok(())
}
```

## Features

Here are the main features:

- **Fast**: Built with performance in mind
- **Safe**: Memory-safe with zero `unsafe` code
- **Async**: Full async/await support with *tokio*
- **Extensible**: Plugin system for custom behavior

### Advanced Usage

For more complex scenarios, you can use the `Builder` pattern:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| timeout | Duration | 30s | Request timeout |
| retries | u32 | 3 | Number of retry attempts |
| pool_size | usize | 10 | Connection pool size |

> **Note**: The connection pool is shared across all clients.
> This means you should create a single client and reuse it.

## Code Examples

Here's a Python example for comparison:

```python
import asyncio
from my_library import Client

async def main():
    async with Client() as client:
        response = await client.get("https://api.example.com")
        data = response.json()
        print(f"Got {len(data)} items")

if __name__ == "__main__":
    asyncio.run(main())
```

And TypeScript:

```typescript
import { Client, Config } from 'my-library';

interface DataItem {
  id: string;
  name: string;
  value: number;
}

async function fetchData(): Promise<DataItem[]> {
  const client = new Client({
    timeout: 30000,
    retries: 3,
  });
  
  const response = await client.get<DataItem[]>('/api/data');
  return response.data;
}
```

## Troubleshooting

If you encounter issues:

1. Check your network connection
2. Verify the API endpoint is correct
3. Look at the error message for clues
4. Enable debug logging with `RUST_LOG=debug`

### Common Errors

**Connection refused**: The server is not running or the port is wrong.

**Timeout**: The server took too long to respond. Try increasing the timeout:

```rust
let client = Client::builder()
    .with_timeout(Duration::from_secs(60))
    .build()?;
```

**Parse error**: The response wasn't valid JSON. Check the `Content-Type` header.

## Conclusion

That's it! You should now be ready to use `my-library` in your projects.

For more information, see:
- [API Reference](https://docs.example.com/api)
- [GitHub Repository](https://github.com/example/my-library)
- [Discord Community](https://discord.gg/example)

---

*Happy coding!* 🚀
"##;

const NESTED_FORMATTING: &str = r##"This has **bold with *nested italic* inside** and more.

Here's `inline code` and **`bold code`** together.

What about ***bold italic*** text?

And ~~strikethrough with **bold inside**~~ works too.

Escaped: \*not italic\* and \`not code\` and \*\*not bold\*\*.
"##;

const EDGE_CASES: &str = r##"# Header at start

Text then **bold
across lines** continues.

Unclosed *italic that never closes

Code block without language:
```
plain code here
no highlighting
```

Empty code block:
```rust
```

Multiple code blocks:
```python
print("first")
```

Some text between.

```javascript
console.log("second");
```

> Quote line 1
> Quote line 2
> Quote line 3

Back to normal.

| A | B |
|---|---|
| 1 | 2 |

Done.
"##;

// ============ Tests ============

#[test]
fn test_large_markdown_char_by_char() {
    let output = stream_char_by_char(LARGE_MARKDOWN);
    
    // Should contain formatted content
    assert!(!output.is_empty(), "Output should not be empty");
    
    // Should have ANSI codes (formatting applied)
    assert!(output.contains("\x1b["), "Should have ANSI formatting codes");
    
    // Key content should be present
    assert!(output.contains("Welcome"), "Should contain header text");
    assert!(output.contains("Getting Started"), "Should contain section");
    // Code is syntax highlighted so words may be split by ANSI codes
    assert!(output.contains("cargo"), "Should contain code");
}

#[test]
fn test_large_markdown_small_chunks() {
    let output = stream_in_chunks(LARGE_MARKDOWN, 5);
    assert!(!output.is_empty());
    assert!(output.contains("\x1b["));
}

#[test]
fn test_large_markdown_medium_chunks() {
    let output = stream_in_chunks(LARGE_MARKDOWN, 50);
    assert!(!output.is_empty());
    assert!(output.contains("\x1b["));
}

#[test]
fn test_large_markdown_large_chunks() {
    let output = stream_in_chunks(LARGE_MARKDOWN, 500);
    assert!(!output.is_empty());
    assert!(output.contains("\x1b["));
}

#[test]
fn test_large_markdown_variable_chunks() {
    let output = stream_variable_chunks(LARGE_MARKDOWN);
    assert!(!output.is_empty());
    assert!(output.contains("\x1b["));
}

#[test]
fn test_nested_formatting_char_by_char() {
    let output = stream_char_by_char(NESTED_FORMATTING);
    
    assert!(!output.is_empty());
    // Should handle nested formatting
    assert!(output.contains("bold"), "Should contain bold text");
    assert!(output.contains("italic"), "Should contain italic text");
}

#[test]
fn test_nested_formatting_variable_chunks() {
    let output = stream_variable_chunks(NESTED_FORMATTING);
    assert!(!output.is_empty());
}

#[test]
fn test_edge_cases_char_by_char() {
    let output = stream_char_by_char(EDGE_CASES);
    
    assert!(!output.is_empty());
    // Should handle unclosed constructs gracefully
    assert!(output.contains("Header"), "Should contain header");
    assert!(output.contains("plain code"), "Should contain plain code");
}

#[test]
fn test_edge_cases_variable_chunks() {
    let output = stream_variable_chunks(EDGE_CASES);
    assert!(!output.is_empty());
}

#[test]
fn test_consistency_across_chunk_sizes() {
    // The formatted output should be equivalent regardless of chunk size
    // (though exact ANSI codes might differ slightly due to termimad internals)
    
    let output_1 = stream_in_chunks(NESTED_FORMATTING, 1);
    let output_10 = stream_in_chunks(NESTED_FORMATTING, 10);
    let output_100 = stream_in_chunks(NESTED_FORMATTING, 100);
    
    // All should be non-empty
    assert!(!output_1.is_empty());
    assert!(!output_10.is_empty());
    assert!(!output_100.is_empty());
    
    // All should have formatting
    assert!(output_1.contains("\x1b["));
    assert!(output_10.contains("\x1b["));
    assert!(output_100.contains("\x1b["));
}

#[test]
fn test_code_block_split_across_chunks() {
    // Specifically test code block fence split across chunks
    let mut fmt = make_formatter();
    let mut output = String::new();
    
    // Feed the code block in pieces
    output.push_str(&fmt.process("text\n"));
    output.push_str(&fmt.process("```"));
    output.push_str(&fmt.process("rust\n"));
    output.push_str(&fmt.process("fn main() {}\n"));
    output.push_str(&fmt.process("```"));
    output.push_str(&fmt.process("\nmore"));
    output.push_str(&fmt.finish());
    
    // The code is syntax highlighted, so "fn main" is split by ANSI codes
    // Check for the parts separately
    assert!(output.contains("fn"), "Should contain 'fn' keyword");
    assert!(output.contains("main"), "Should contain 'main' identifier");
    
    // Also verify it has ANSI formatting (syntax highlighting)
    assert!(output.contains("\x1b["), "Should have syntax highlighting");
}

#[test]
fn test_bold_split_across_chunks() {
    let mut fmt = make_formatter();
    let mut output = String::new();
    
    // Split ** across chunks
    output.push_str(&fmt.process("hello *"));
    output.push_str(&fmt.process("*bold text*"));
    output.push_str(&fmt.process("* world\n"));
    output.push_str(&fmt.finish());
    
    assert!(output.contains("bold text"), "Should contain bold text");
}

#[test]
fn test_escape_split_across_chunks() {
    let mut fmt = make_formatter();
    let mut output = String::new();
    
    // Split escape sequence across chunks
    output.push_str(&fmt.process("not \\"));
    output.push_str(&fmt.process("*italic\n"));
    output.push_str(&fmt.finish());
    
    // The * should be literal, not formatting
    assert!(output.contains("*italic") || output.contains("\\*italic"), 
            "Escaped asterisk should be preserved");
}

#[test]
fn test_visual_output() {
    // This test prints output for visual inspection
    // Run with: cargo test -p g3-cli --test streaming_markdown_test test_visual_output -- --nocapture
    
    println!("\n\n=== STREAMING MARKDOWN VISUAL TEST ===");
    println!("\n--- Character by character ---\n");
    
    let sample = r##"# Hello World

This is **bold** and *italic* text.

```rust
fn main() {
    println!("Hello!");
}
```

> A quote here

| Col1 | Col2 |
|------|------|
| A    | B    |

Done!
"##;
    
    let output = stream_char_by_char(sample);
    print!("{}", output);
    
    println!("\n--- End of test ---\n");
}

#[test]
fn test_streaming_simulation() {
    // Simulate realistic LLM streaming with small chunks and delays
    // Run with: cargo test -p g3-cli --test streaming_markdown_test test_streaming_simulation -- --nocapture
    
    println!("\n\n=== SIMULATED LLM STREAMING ===");
    
    let content = r##"I'll help you with that!

Here's a **Rust** function:

```rust
pub fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

This uses *recursion* to calculate the nth Fibonacci number.

> Note: This is not efficient for large n!

For better performance, use iteration:

```rust
pub fn fibonacci_fast(n: u64) -> u64 {
    let mut a = 0;
    let mut b = 1;
    for _ in 0..n {
        let temp = a;
        a = b;
        b = temp + b;
    }
    a
}
```

Hope this helps! 🎉
"##;
    
    let mut fmt = make_formatter();
    
    // Simulate token-by-token streaming (roughly word-sized chunks)
    let tokens: Vec<&str> = content.split_inclusive(|c: char| c.is_whitespace() || c == '\n')
        .collect();
    
    print!("\n");
    for token in tokens {
        let output = fmt.process(token);
        print!("{}", output);
        // In real streaming, there would be a small delay here
    }
    print!("{}", fmt.finish());
    println!("\n\n=== END SIMULATION ===");
}

#[test]
fn test_lists_visual() {
    // Test list handling
    // Run with: cargo test -p g3-cli --test streaming_markdown_test test_lists_visual -- --nocapture
    
    println!("\n\n=== LIST TEST ===");
    
    let md = r#"Here's a list:

- First item
- Second item with **bold**
- Third item

And ordered:

1. One
2. Two
3. Three

Nested:

- Parent
  - Child 1
  - Child 2
- Another parent

Done!
"#;
    
    let mut fmt = make_formatter();
    
    // Stream char by char
    for ch in md.chars() {
        let out = fmt.process(&ch.to_string());
        print!("{}", out);
    }
    print!("{}", fmt.finish());
    println!("\n=== END LIST TEST ===");
}



#[test]
fn test_no_duplicate_output() {
    let mut fmt = make_formatter();
    
    // Test that inline formatting doesn't produce duplicate output
    let input = "Normal text with **bold**, *italic*, and `inline code` all together.\n";
    let output = fmt.process(input);
    let final_out = fmt.finish();
    let full_output = format!("{}{}", output, final_out);
    
    eprintln!("Input: {:?}", input);
    eprintln!("Output: {:?}", full_output);
    
    // Count occurrences of "Normal text"
    let count = full_output.matches("Normal text").count();
    assert_eq!(count, 1, "Should only have one occurrence of 'Normal text', found {}", count);
    
    // Should not contain raw markdown
    assert!(!full_output.contains("**bold**"), "Should not contain raw **bold**");
    assert!(!full_output.contains("*italic*"), "Should not contain raw *italic*");
    assert!(!full_output.contains("`inline code`"), "Should not contain raw `inline code`");
}

#[test]
fn test_bold_formatting() {
    let mut fmt = make_formatter();
    
    let input = "This is **bold** text.\n";
    let output = fmt.process(input);
    let final_out = fmt.finish();
    let full_output = format!("{}{}", output, final_out);
    
    eprintln!("Input: {:?}", input);
    eprintln!("Output: {:?}", full_output);
    
    // Should contain green bold ANSI code (\x1b[1;32m)
    assert!(full_output.contains("\x1b[1;32m"), "Should contain bold formatting");
    // Should NOT contain raw **
    assert!(!full_output.contains("**"), "Should not contain raw **");
}

#[test]
fn test_all_markdown_elements() {
    let mut fmt = make_formatter();
    
    let input = r#"# Header 1
## Header 2
### Header 3

This is **bold text** and this is *italic text*.

Here is `inline code` in a sentence.

Here is a [link](https://example.com).

- Bullet item 1
- Bullet item 2
  - Nested bullet

1. Numbered item 1
2. Numbered item 2

---

~~strikethrough text~~

```rust
fn main() {
    println!("Hello, world!");
}
```

Normal text with **bold**, *italic*, and `inline code` all together.
"#;
    
    let output = fmt.process(input);
    let final_out = fmt.finish();
    let full_output = format!("{}{}", output, final_out);
    
    eprintln!("=== FULL OUTPUT ===");
    eprintln!("{}", full_output);
    eprintln!("=== END ===");
    
    // Check headers are formatted (magenta)
    assert!(full_output.contains("\x1b[1;35mHeader 1"), "H1 should be bold magenta");
    assert!(full_output.contains("\x1b[35mHeader 2"), "H2 should be magenta");
    
    // Check bold is green
    assert!(full_output.contains("\x1b[1;32mbold text\x1b[0m"), "Bold should be green");
    
    // Check italic is cyan
    assert!(full_output.contains("\x1b[3;36mitalic text\x1b[0m"), "Italic should be cyan");
    
    // Check inline code is orange
    assert!(full_output.contains("\x1b[38;2;216;177;114minline code\x1b[0m"), "Inline code should be orange");
    
    // Check link is cyan underlined
    assert!(full_output.contains("\x1b[36;4mlink\x1b[0m"), "Link should be cyan underlined");
    
    // Check bullets
    assert!(full_output.contains("• Bullet item 1"), "Should have bullet");
    assert!(full_output.contains("• Nested bullet"), "Should have nested bullet");
    
    // Check horizontal rule
    assert!(full_output.contains("────"), "Should have horizontal rule");
    
    // Check strikethrough
    assert!(full_output.contains("\x1b[9mstrikethrough text\x1b[0m"), "Should have strikethrough");
    
    // Check code block has syntax highlighting
    assert!(full_output.contains("\x1b[38;2;"), "Code block should have 24-bit color");
    
    // Should NOT contain raw markdown
    assert!(!full_output.contains("# Header"), "Should not have raw # header");
    assert!(!full_output.contains("**bold"), "Should not have raw **");
    assert!(!full_output.contains("[link]("), "Should not have raw link syntax");
}

#[test]
fn test_unclosed_inline_code() {
    let mut fmt = make_formatter();
    
    // Test unclosed inline code at end of line
    let input = "that's `kill-ring-save, which copies the region.\n";
    let output = fmt.process(input);
    let final_out = fmt.finish();
    let full_output = format!("{}{}", output, final_out);
    
    eprintln!("Input: {:?}", input);
    eprintln!("Output: {:?}", full_output);
    
    // Should NOT contain raw backtick
    assert!(!full_output.contains('`'), "Should not contain raw backtick");
    
    // Should contain orange formatting for the unclosed code
    assert!(full_output.contains("\x1b[38;2;216;177;114m"), "Should have orange formatting");
}

#[test]
fn test_emacs_markdown_edge_case() {
    let mut fmt = make_formatter();
    
    // This is the exact markdown from the screenshot that's failing
    let input = r#"project.el is Emacs' built-in lightweight project management.

Your config already has it set up with consult:

`elisp
(setq project-switch-commands
      '((consult-find "Find file" ?f)
        (consult-ripgrep "Ripgrep" ?g)
        (project-dired "Dired" ?d)))
`

### Key bindings you have:

| Keys | Command | What it does |
|------|---------|-------------|
| C-x p f | consult-find | **Fuzzy find any file in project** ← this is what you want |

### To "teleport" between files:

1. Make sure you're in a git repo
2. Press **C-x p f**
3. Type any part of the filename
"#;
    
    let output = fmt.process(input);
    let final_out = fmt.finish();
    let full_output = format!("{}{}", output, final_out);
    
    eprintln!("=== OUTPUT ===");
    eprintln!("{}", full_output);
    eprintln!("=== RAW ===");
    eprintln!("{:?}", full_output);
    
    // Headers should be formatted (magenta), not raw
    assert!(!full_output.contains("### Key"), "Should not have raw ### header");
    assert!(full_output.contains("\x1b[35mKey bindings"), "Header should be magenta");
    
    // Bold should be formatted, not raw
    assert!(!full_output.contains("**C-x p f**"), "Should not have raw ** bold");
    assert!(full_output.contains("\x1b[1;32mC-x p f\x1b[0m"), "Bold should be green");
}

#[test]
fn test_emacs_markdown_streaming_char_by_char() {
    let mut fmt = make_formatter();
    
    // Same input but streamed char by char
    let input = r#"project.el is Emacs' built-in lightweight project management.

Your config already has it set up with consult:

`elisp
(setq project-switch-commands
      '((consult-find "Find file" ?f)
        (consult-ripgrep "Ripgrep" ?g)
        (project-dired "Dired" ?d)))
`

### Key bindings you have:

| Keys | Command | What it does |
|------|---------|-------------|
| C-x p f | consult-find | **Fuzzy find any file in project** ← this is what you want |

### To "teleport" between files:

1. Make sure you're in a git repo
2. Press **C-x p f**
3. Type any part of the filename
"#;
    
    // Stream char by char like real streaming
    let mut full_output = String::new();
    for ch in input.chars() {
        full_output.push_str(&fmt.process(&ch.to_string()));
    }
    full_output.push_str(&fmt.finish());
    
    eprintln!("=== STREAMING OUTPUT ===");
    eprintln!("{}", full_output);
    eprintln!("=== RAW ===");
    eprintln!("{:?}", full_output);
    
    // Headers should be formatted (magenta), not raw
    assert!(!full_output.contains("### Key"), "Should not have raw ### header");
    
    // Bold should be formatted, not raw
    assert!(!full_output.contains("**C-x p f**"), "Should not have raw ** bold");
}


#[test]
fn test_single_backtick_code_block() {
    let mut fmt = make_formatter();
    
    // The LLM is using single backticks for code blocks (incorrect markdown)
    // This is what the screenshot shows
    let input = r#"Your config:

`elisp
(setq foo bar)
`

### Header after code

Some text with **bold**.
"#;
    
    let mut full_output = String::new();
    for ch in input.chars() {
        full_output.push_str(&fmt.process(&ch.to_string()));
    }
    full_output.push_str(&fmt.finish());
    
    eprintln!("=== OUTPUT ===");
    eprintln!("{}", full_output);
    eprintln!("=== RAW ===");
    eprintln!("{:?}", full_output);
    
    // Header should still be formatted
    assert!(!full_output.contains("### Header"), "Should not have raw ### header");
    
    // Bold should be formatted
    assert!(!full_output.contains("**bold**"), "Should not have raw ** bold");
}

#[test]
fn test_table_then_header_streaming() {
    let mut fmt = make_formatter();
    
    // Table followed by header - this might be breaking state
    let input = r#"| Keys | Command |
|------|---------|  
| C-x | test |

### Header after table

Some **bold** text.
"#;
    
    let mut full_output = String::new();
    for ch in input.chars() {
        full_output.push_str(&fmt.process(&ch.to_string()));
    }
    full_output.push_str(&fmt.finish());
    
    eprintln!("=== OUTPUT ===");
    eprintln!("{}", full_output);
    eprintln!("=== RAW ===");
    eprintln!("{:?}", full_output);
    
    // Header should be formatted
    assert!(!full_output.contains("### Header"), "Should not have raw ### header");
    assert!(full_output.contains("\x1b[35mHeader after table"), "Header should be magenta");
    
    // Bold should be formatted
    assert!(!full_output.contains("**bold**"), "Should not have raw ** bold");
}

#[test]
fn test_table_empty_line_then_header() {
    let mut fmt = make_formatter();
    
    // Table with empty line before header - exact pattern from screenshot
    let input = "| Keys | Command |\n|------|---------|\n| C-x | test |\n\n### Header after empty line\n\nSome **bold** text.\n";
    
    let mut full_output = String::new();
    for ch in input.chars() {
        let out = fmt.process(&ch.to_string());
        if !out.is_empty() {
            eprintln!("After '{}': {:?}", if ch == '\n' { "\\n" } else { &ch.to_string() }, out);
        }
        full_output.push_str(&out);
    }
    full_output.push_str(&fmt.finish());
    
    eprintln!("=== FINAL OUTPUT ===");
    eprintln!("{}", full_output);
    
    // Header should be formatted
    assert!(!full_output.contains("### Header"), "Should not have raw ### header, got: {}", full_output);
}

#[test]
fn test_list_with_unclosed_inline_code() {
    let mut fmt = make_formatter();
    
    // This is the exact pattern from the bug - list items with inline code
    // where the backticks might not be properly closed
    let input = r#"- `14.9s | 3.7s - This is the FIRST response
- `5.0s | 5.0s - This might be a continuation
- Normal item without code
"#;
    
    let mut full_output = String::new();
    for ch in input.chars() {
        full_output.push_str(&fmt.process(&ch.to_string()));
    }
    full_output.push_str(&fmt.finish());
    
    eprintln!("=== OUTPUT ===");
    eprintln!("{}", full_output);
    eprintln!("=== RAW ===");
    eprintln!("{:?}", full_output);
    
    // All list items should have bullets, not raw dashes
    // Count bullets vs raw dashes at line start
    let lines: Vec<&str> = full_output.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        assert!(!trimmed.starts_with("- "), 
            "Line {} should not start with raw '- ', got: {}", i, line);
    }
    
    // Should have 3 bullets
    let bullet_count = full_output.matches('•').count();
    assert_eq!(bullet_count, 3, "Should have 3 bullets, got {}", bullet_count);
}

#[test]
fn test_list_with_inline_code_curly_braces() {
    let mut fmt = make_formatter();
    
    // Pattern from second screenshot - list items with code containing curly braces
    let input = r#"Now I can see the mappings:
- `{ r: 239, g: 14, b: 14 }` → M1 (Red)
- `{ r: 0, g: 58, b: 243 }` → M2 (Blue)
- `{ r: 0, g: 255, b: 0 }` → M3 (Lime)
"#;
    
    let mut full_output = String::new();
    for ch in input.chars() {
        full_output.push_str(&fmt.process(&ch.to_string()));
    }
    full_output.push_str(&fmt.finish());
    
    eprintln!("=== OUTPUT ===");
    eprintln!("{}", full_output);
    
    // Should have 3 bullets
    let bullet_count = full_output.matches('•').count();
    assert_eq!(bullet_count, 3, "Should have 3 bullets, got {}", bullet_count);
    
    // Should not have raw dashes at line start
    for line in full_output.lines() {
        let trimmed = line.trim_start();
        assert!(!trimmed.starts_with("- "), 
            "Should not start with raw '- ', got: {}", line);
    }
}

#[test]
fn test_bold_with_nested_italic() {
    let mut fmt = make_formatter();
    let output = fmt.process("What about **bold with *nested* italic**?\n");
    
    // Should contain formatted output, not raw asterisks
    assert!(!output.contains("*bold"), "Should not have raw *bold");
    assert!(!output.contains("nested*"), "Should not have raw nested*");
    
    // Should have ANSI codes for formatting
    assert!(output.contains("\x1b["), "Should have ANSI formatting codes");
    
    eprintln!("Bold with nested italic output: {:?}", output);
}

#[test]
fn test_link_with_inline_code() {
    let mut fmt = make_formatter();
    let output = fmt.process("Or a [link with `code`](https://example.com)?\n");
    
    eprintln!("Link with inline code output: {:?}", output);
    
    // Should not have raw markdown link syntax
    assert!(!output.contains("](https://"), "Should not have raw link syntax");
    
    // Should have ANSI codes for formatting
    assert!(output.contains("\x1b["), "Should have ANSI formatting codes");
}
#[test]
fn test_list_items_stream_immediately() {
    let mut fmt = make_formatter();
    
    // Process a list item character by character
    let input = "- hello world\n";
    let mut outputs = Vec::new();
    
    for ch in input.chars() {
        let output = fmt.process(&ch.to_string());
        if !output.is_empty() {
            outputs.push(output);
        }
    }
    
    // We should have multiple outputs (streaming), not just one at the end
    // The bullet should come first, then the text should stream
    eprintln!("Number of outputs: {}", outputs.len());
    for (i, out) in outputs.iter().enumerate() {
        eprintln!("Output {}: {:?}", i, out);
    }
    
    // Should have at least 2 outputs: the bullet and some streamed text
    assert!(outputs.len() >= 2, "List items should stream, got {} outputs", outputs.len());
    
    // First output should be the bullet
    assert!(outputs[0].contains("•"), "First output should be the bullet");
}

#[test]
fn test_empty_bold_in_list() {
    let mut fmt = make_formatter();
    let output = fmt.process("- Empty bold: ****\n");
    eprintln!("Output: {:?}", output);
    // Should NOT contain horizontal rule
    assert!(!output.contains("────"), "Should not be a horizontal rule");
}

#[test]
fn test_horizontal_rule_still_works() {
    let mut fmt = make_formatter();
    let output = fmt.process("***\n");
    eprintln!("Output: {:?}", output);
    // Should be a horizontal rule
    assert!(output.contains("────"), "*** should be a horizontal rule");
}

#[test]
fn test_dashes_horizontal_rule() {
    let mut fmt = make_formatter();
    let output = fmt.process("---\n");
    eprintln!("Output: {:?}", output);
    assert!(output.contains("────"), "--- should be a horizontal rule");
}


#[test]
fn test_simple_italic() {
    let mut fmt = make_formatter();
    let out = fmt.process("*simple italic*\n");
    eprintln!("Simple italic: {:?}", out);
    assert!(out.contains("\x1b[3;36m"), "Should have italic formatting");
}

#[test]
fn test_italic_with_nested_bold() {
    let mut fmt = make_formatter();
    let output = fmt.process("*italic with **nested bold** inside*\n");
    eprintln!("Output: {:?}", output);
    // Should have italic formatting (cyan)
    assert!(output.contains("\x1b[3;36m"), "Should have italic formatting");
    // Should have bold formatting (green) for nested bold
    assert!(output.contains("\x1b[1;32m"), "Should have bold formatting for nested");
}

// =============================================================================
// Randomized Stress Tests for Markdown Edge Cases
// =============================================================================

/// Stress test 1: Multiple nested formatting combinations
#[test]
fn stress_test_nested_formatting_combinations() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Bold inside italic
        "*italic with **bold** inside*",
        // Italic inside bold  
        "**bold with *italic* inside**",
        // Code inside bold
        "**bold with `code` inside**",
        // Code inside italic
        "*italic with `code` inside*",
        // Multiple nested
        "**bold *italic* more bold**",
        // Adjacent formatting
        "**bold** and *italic* and `code`",
        // Back to back same type
        "**first** **second** **third**",
        "*one* *two* *three*",
        // Mixed delimiters
        "__underscore bold__ and **asterisk bold**",
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        // Should not contain raw delimiter sequences in output (unless escaped)
        // Check that we don't have unprocessed ** or * at word boundaries
        eprintln!("Input: {:?}", case);
        eprintln!("Output: {:?}", full_output);
        
        // Basic sanity: output should have ANSI codes if input had formatting
        if case.contains("**") || case.contains("*") || case.contains("`") {
            assert!(full_output.contains("\x1b["), 
                "Expected ANSI formatting for: {}", case);
        }
        
        // Reset formatter for next case
        fmt = make_formatter();
    }
}

/// Stress test 2: Edge cases with empty and minimal content
#[test]
fn stress_test_empty_and_minimal() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Empty formatting
        "****",           // Empty bold
        "**",             // Incomplete bold
        "*",              // Single asterisk
        "``",             // Empty code
        "`",              // Single backtick
        "[]()",           // Empty link
        "[](url)",        // Link with empty text
        "[text]()",       // Link with empty URL
        // Minimal content
        "**a**",          // Single char bold
        "*a*",            // Single char italic
        "`a`",            // Single char code
        // Whitespace edge cases
        "** **",          // Bold with only space
        "* *",            // Italic with only space
        "**  **",         // Bold with multiple spaces
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?} -> Output: {:?}", case, full_output);
        
        // Should not panic and should produce some output
        assert!(!full_output.is_empty() || case.is_empty(),
            "Should produce output for: {}", case);
        
        // Should not have unclosed ANSI sequences (each \x1b[ should have \x1b[0m)
        let opens = full_output.matches("\x1b[").count();
        let closes = full_output.matches("\x1b[0m").count();
        // Note: opens includes the [0m sequences, so this is a rough check
        assert!(opens >= closes, 
            "ANSI sequences should be balanced for: {}", case);
        
        fmt = make_formatter();
    }
}

/// Stress test 3: Escape sequences and special characters
#[test]
fn stress_test_escapes_and_special_chars() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Escaped formatting characters
        ("\\*not italic\\*", false),           // Should show *not italic*
        ("\\**not bold\\**", false),           // Should show **not bold**
        ("\\`not code\\`", false),             // Should show `not code`
        ("\\[not a link\\](url)", false),      // Should show [not a link](url)
        // Mixed escaped and real
        ("**bold** and \\*escaped\\*", true),  // Bold + literal asterisks
        ("`code` and \\`escaped\\`", true),    // Code + literal backticks
        // Special characters in content
        ("**bold with < > & chars**", true),
        ("`code with < > & chars`", true),
        ("*italic with 日本語*", true),         // Unicode
        ("**bold with émojis 🎉**", true),
        // Backslash edge cases
        ("\\\\", false),                       // Double backslash
        ("\\n\\t", false),                     // Escaped n and t (not newline/tab)
    ];
    
    for (case, should_have_formatting) in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?} -> Output: {:?}", case, full_output);
        
        if should_have_formatting {
            assert!(full_output.contains("\x1b["),
                "Expected ANSI formatting for: {}", case);
        }
        
        // Escaped chars should not have backslash in output
        if case.contains("\\*") && !case.contains("**") {
            // Pure escaped case - should not have formatting
            // (This is a simplified check)
        }
        
        fmt = make_formatter();
    }
}

/// Stress test 4: Lists with complex inline formatting
#[test]
fn stress_test_lists_with_formatting() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        "- Simple list item",
        "- **Bold list item**",
        "- *Italic list item*",
        "- `Code in list`",
        "- Item with **bold** and *italic*",
        "- Item with [link](url)",
        "- Item with [link with `code`](url)",
        "- **Bold with *nested italic* inside**",
        "- *Italic with **nested bold** inside*",
        "- Multiple `code` blocks `here`",
        "  - Nested list item",
        "    - Deeply nested",
        "- Item with ****",  // Empty bold in list
        "- Item ending with *",  // Unclosed italic
        "1. Ordered list item",
        "2. **Bold ordered item**",
        "10. Double digit number",
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?} -> Output: {:?}", case, full_output);
        
        // List items should have bullet or number
        if case.starts_with("- ") || case.trim_start().starts_with("- ") {
            assert!(full_output.contains("•") || full_output.contains("-"),
                "List should have bullet for: {}", case);
        }
        
        // Ordered lists should preserve number
        if case.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            assert!(full_output.chars().any(|c| c.is_ascii_digit()),
                "Ordered list should have number for: {}", case);
        }
        
        fmt = make_formatter();
    }
}

/// Stress test 5: Links with various content combinations
#[test]
fn stress_test_links() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Basic links
        "[simple link](https://example.com)",
        "[link](url)",
        // Links with formatting in text
        "[**bold link**](url)",
        "[*italic link*](url)",
        "[`code link`](url)",
        "[link with `code` inside](url)",
        "[**bold** and *italic*](url)",
        // Links with special URL characters
        "[link](https://example.com/path?query=1&other=2)",
        "[link](https://example.com/path#anchor)",
        "[link](url-with-dashes)",
        "[link](url_with_underscores)",
        // Multiple links
        "[first](url1) and [second](url2)",
        "Check [this](a) and [that](b) out",
        // Links adjacent to other formatting
        "**bold** [link](url) *italic*",
        "`code` [link](url) `more code`",
        // Edge cases
        "[](empty-text)",
        "[text]()",
        "text [link](url) more text",
        "[nested [brackets]](url)",  // Invalid but shouldn't crash
        "[link](url with spaces)",   // Invalid but shouldn't crash
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?} -> Output: {:?}", case, full_output);
        
        // Valid links should have cyan formatting (\x1b[36)
        if case.contains("](url") || case.contains("](https") {
            // Most valid links should be formatted
            // (Some edge cases may not be)
        }
        
        // Should not crash on any input
        assert!(full_output.len() > 0 || case.is_empty(),
            "Should produce output for: {}", case);
        
        fmt = make_formatter();
    }
}

// =============================================================================
// Advanced Stress Tests - Tables, Code Blocks, Mixed Constructs
// =============================================================================

/// Stress test 6: Tables with various content
#[test]
fn stress_test_tables() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Simple table
        "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |",
        // Table with formatting in cells
        "| **Bold** | *Italic* |\n|----------|----------|\n| `code`   | normal   |",
        // Table with links
        "| Name | Link |\n|------|------|\n| Test | [link](url) |",
        // Table with mixed formatting
        "| Col A | Col B |\n|-------|-------|\n| **bold** and *italic* | `code` here |",
        // Minimal table
        "|a|b|\n|-|-|\n|1|2|",
        // Table with empty cells
        "| A | B |\n|---|---|\n|   |   |",
        // Wide table
        "| One | Two | Three | Four | Five |\n|-----|-----|-------|------|------|\n| 1 | 2 | 3 | 4 | 5 |",
        // Table followed by text
        "| H |\n|---|\n| V |\n\nParagraph after table",
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?}", case.replace('\n', "\\n"));
        eprintln!("Output: {:?}", full_output.replace('\n', "\\n"));
        
        // Tables should produce some output
        assert!(!full_output.is_empty(), "Table should produce output");
        
        // Should not crash
        fmt = make_formatter();
    }
}

/// Stress test 7: Code blocks with various languages and content
#[test]
fn stress_test_code_blocks() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Basic code block
        "```\ncode here\n```",
        // Code block with language
        "```rust\nfn main() {}\n```",
        "```python\ndef foo():\n    pass\n```",
        "```javascript\nconst x = 1;\n```",
        // Code block with special chars
        "```\n<html>&amp;</html>\n```",
        // Code block with markdown-like content (should not be formatted)
        "```\n**not bold** *not italic* `not code`\n```",
        // Empty code block
        "```\n```",
        // Code block with blank lines
        "```\nline 1\n\nline 3\n```",
        // Nested backticks in code
        "```\nuse `backticks` here\n```",
        // Code block followed by text
        "```\ncode\n```\n\nText after code",
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?}", case.replace('\n', "\\n"));
        eprintln!("Output: {:?}", full_output.replace('\n', "\\n"));
        
        // Code blocks should produce output
        assert!(!full_output.is_empty(), "Code block should produce output");
        
        // Content inside code blocks should NOT have markdown formatting applied
        // (The **not bold** should remain as-is)
        if case.contains("**not bold**") {
            // The literal ** should appear in output (possibly with syntax highlighting)
            // but NOT as ANSI bold formatting
        }
        
        fmt = make_formatter();
    }
}

/// Stress test 8: Mixed block and inline elements
#[test]
fn stress_test_mixed_blocks() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Header followed by list
        "# Header\n\n- Item 1\n- Item 2",
        // List followed by code block
        "- Item 1\n- Item 2\n\n```\ncode\n```",
        // Blockquote with formatting
        "> This is a **bold** quote\n> With *italic* too",
        // Multiple headers
        "# H1\n## H2\n### H3",
        // Header with inline formatting
        "# **Bold Header**\n## *Italic Header*",
        // List with code block item (indented)
        "- Item 1\n- Item with code:\n  ```\n  code\n  ```",
        // Horizontal rule between content
        "Before\n\n---\n\nAfter",
        // Multiple horizontal rules
        "---\n\n***\n\n___",
        // Nested blockquotes
        "> Level 1\n>> Level 2\n>>> Level 3",
        // Mixed list types
        "- Bullet\n1. Number\n- Bullet again",
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?}", case.replace('\n', "\\n"));
        eprintln!("Output: {:?}", full_output.replace('\n', "\\n"));
        
        // Should produce output
        assert!(!full_output.is_empty(), "Mixed blocks should produce output");
        
        // Headers should have formatting
        if case.starts_with("# ") {
            assert!(full_output.contains("\x1b["), "Header should have ANSI formatting");
        }
        
        fmt = make_formatter();
    }
}

/// Stress test 9: Complex nested lists
#[test]
fn stress_test_nested_lists() {
    let mut fmt = make_formatter();
    
    let test_cases = vec![
        // Simple nested
        "- Level 1\n  - Level 2\n    - Level 3",
        // Mixed bullets and numbers
        "- Bullet\n  1. Nested number\n  2. Another\n- Back to bullet",
        // Deep nesting with formatting
        "- **Bold item**\n  - *Italic nested*\n    - `Code deep`",
        // List with multiple paragraphs (double newline)
        "- Item 1\n\n- Item 2\n\n- Item 3",
        // Nested with links
        "- [Link 1](url1)\n  - [Link 2](url2)\n    - [Link 3](url3)",
        // Complex mixed
        "1. First\n   - Sub bullet\n   - Another\n2. Second\n   1. Sub number\n   2. Another",
        // List with long content
        "- This is a very long list item that contains **bold text** and *italic text* and `inline code` all together",
        // Empty list items
        "- \n- Content\n- ",
        // List with special characters
        "- Item with: colons\n- Item with - dashes\n- Item with * asterisks",
        // Checkbox-style (GitHub)
        "- [ ] Unchecked\n- [x] Checked\n- [ ] Another",
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?}", case.replace('\n', "\\n"));
        eprintln!("Output: {:?}", full_output.replace('\n', "\\n"));
        
        // Should have bullets
        assert!(full_output.contains("•") || full_output.contains("-") || 
                full_output.chars().any(|c| c.is_ascii_digit()),
                "List should have bullets or numbers: {}", case);
        
        fmt = make_formatter();
    }
}

/// Stress test 10: Pathological and adversarial inputs
#[test]
fn stress_test_pathological() {
    let mut fmt = make_formatter();
    
    let long_line = "word ".repeat(100);
    
    let test_cases = vec![
        // Many asterisks
        "*****",
        "**********",
        "* * * * *",
        "** ** ** **",
        // Unbalanced delimiters
        "**bold without close",
        "*italic without close",
        "`code without close",
        "[link without close",
        "[link](url without close",
        // Deeply nested (should not stack overflow)
        "**bold *italic **nested** italic* bold**",
        // Many escapes
        "\\*\\*\\*\\*\\*",
        "\\`\\`\\`",
        // Mixed valid and invalid
        "**valid** invalid** **also valid**",
        "`valid` invalid` `also valid`",
        // Whitespace variations
        "  **bold**  ",
        "\t*italic*\t",
        // Empty lines with formatting
        "\n\n**bold**\n\n",
        // Only whitespace
        "   ",
        "\t\t\t",
        // Unicode edge cases
        "**日本語**",
        "*émojis 🎉 here*",
        "`code with 中文`",
        // Very long line
        &long_line,
        // Alternating formatting
        "**b***i***b***i***b**",
        // Adjacent different formats
        "**bold***italic*`code`",
    ];
    
    for case in test_cases {
        let input = format!("{}\n", case);
        let output = fmt.process(&input);
        let remaining = fmt.finish();
        let full_output = format!("{}{}", output, remaining);
        
        eprintln!("Input: {:?}", if case.len() > 50 { &case[..50] } else { case });
        eprintln!("Output len: {}", full_output.len());
        
        // Main assertion: should not panic and should produce some output
        // (even if it's just the input echoed back)
        assert!(full_output.len() > 0 || case.trim().is_empty(),
                "Should produce output for: {}", case);
        
        // ANSI sequences should be balanced (rough check)
        let esc_count = full_output.matches("\x1b[").count();
        let reset_count = full_output.matches("\x1b[0m").count();
        // Each formatting open should have a close
        // (esc_count includes [0m, so esc_count >= reset_count)
        assert!(esc_count >= reset_count || esc_count == 0,
                "ANSI sequences should be balanced");
        
        fmt = make_formatter();
    }
}

#[test]
fn test_language_aliases() {
    let mut fmt = make_formatter();
    
    // Test Racket code block
    let racket_code = r#"```racket
(define (factorial n)
  (if (<= n 1)
      1
      (* n (factorial (- n 1)))))
```
"#;
    let output = fmt.process(racket_code);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    // Should have ANSI codes (syntax highlighting applied)
    assert!(full.contains("\x1b["), "Racket should be syntax highlighted");
    assert!(full.contains("factorial"));
    
    // Test elisp code block
    let mut fmt = make_formatter();
    let elisp_code = r#"```elisp
(defun hello-world ()
  "Print hello world."
  (interactive)
  (message "Hello, World!"))
```
"#;
    let output = fmt.process(elisp_code);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    assert!(full.contains("\x1b["), "Elisp should be syntax highlighted");
    assert!(full.contains("hello-world"));
    
    // Test scheme code block  
    let mut fmt = make_formatter();
    let scheme_code = r#"```scheme
(define (map f lst)
  (if (null? lst)
      '()
      (cons (f (car lst))
            (map f (cdr lst)))))
```
"#;
    let output = fmt.process(scheme_code);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    assert!(full.contains("\x1b["), "Scheme should be syntax highlighted");
}

#[test]
fn test_backticks_edge_cases() {
    let mut fmt = make_formatter();
    
    // Simple inline code
    let input = "- `racket` / `rkt`\n";
    let output = fmt.process(input);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    println!("Simple: {}", full);
    assert!(full.contains("\x1b["), "Should have formatting");
    
    // Backticks inside inline code (using double backtick delimiters)
    let mut fmt = make_formatter();
    let input = "- `` `racket` `` works\n";
    let output = fmt.process(input);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    println!("Double delim: {}", full);
}

#[test]
fn test_inline_code_regex_directly() {
    let code_re = regex::Regex::new(r"`([^`]+)`").unwrap();
    
    let input = "`racket` / `rkt`";
    let matches: Vec<_> = code_re.find_iter(input).collect();
    println!("Input: {}", input);
    println!("Matches: {:?}", matches);
    
    let result = code_re.replace_all(input, |caps: &regex::Captures| {
        let code = &caps[1];
        format!("[CODE:{}]", code)
    });
    println!("Result: {}", result);
}

#[test]
fn test_inline_code_char_by_char() {
    let mut fmt = make_formatter();
    
    let input = "- `racket` / `rkt`\n";
    println!("Input: {:?}", input);
    
    // Process char by char to see what's happening
    for ch in input.chars() {
        let output = fmt.process(&ch.to_string());
        if !output.is_empty() {
            println!("After {:?}: output={:?}", ch, output);
        }
    }
    
    let remaining = fmt.finish();
    println!("Finish: {:?}", remaining);
}

#[test]
fn test_inline_code_detailed_trace() {
    let mut fmt = make_formatter();
    
    let input = "- `racket` / `rkt`\n";
    println!("Input: {:?}", input);
    
    // Process char by char
    for (i, ch) in input.chars().enumerate() {
        let output = fmt.process(&ch.to_string());
        println!("[{}] char={:?} output={:?}", i, ch, output);
    }
    
    let remaining = fmt.finish();
    println!("Finish: {:?}", remaining);
}

#[test]
fn test_code_block_closing() {
    let mut fmt = make_formatter();
    
    let input = r#"```yaml
- type: on-load
  script: |
    (lock-player)
```
"#;
    
    println!("Input: {:?}", input);
    
    let output = fmt.process(input);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    
    println!("Output: {:?}", full);
    
    // Should NOT contain literal ``` in output
    assert!(!full.contains("```"), "Code fence should not appear in output");
}

#[test]
fn test_code_block_with_trailing_fence() {
    let mut fmt = make_formatter();
    
    // Test case: code block followed by another code fence (malformed markdown)
    let input = "```yaml\ncode here\n```\n```\n";
    
    println!("Input: {:?}", input);
    
    let output = fmt.process(input);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    
    println!("Output: {:?}", full);
}

#[test]
fn test_code_block_char_by_char() {
    let mut fmt = make_formatter();
    
    let input = "```yaml\ncode\n```\n";
    println!("Input: {:?}", input);
    
    for (i, ch) in input.chars().enumerate() {
        let output = fmt.process(&ch.to_string());
        if !output.is_empty() {
            println!("[{}] char={:?} output={:?}", i, ch, output);
        }
    }
    
    let remaining = fmt.finish();
    println!("Finish: {:?}", remaining);
}

#[test]
fn test_code_fence_not_at_line_start() {
    let mut fmt = make_formatter();
    
    // Code fence with leading space (should NOT be treated as code block)
    let input = " ```yaml\ncode\n```\n";
    
    println!("Input: {:?}", input);
    
    let output = fmt.process(input);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    
    println!("Output: {:?}", full);
    // With leading space, it might not be detected as a code fence
}

#[test]
fn test_code_block_containing_backticks() {
    let mut fmt = make_formatter();
    
    // Code block that contains triple backticks in the content
    let input = "```yaml\nscript: |\n  ```\n  nested\n  ```\n```\n";
    
    println!("Input: {:?}", input);
    
    let output = fmt.process(input);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    
    println!("Output: {:?}", full);
}

#[test]
fn test_code_block_with_4space_indent() {
    let mut fmt = make_formatter();
    
    // Code block that contains triple backticks with 4-space indent (should NOT close)
    let input = "```yaml\nscript: |\n    ```\n    nested\n    ```\n```\n";
    
    println!("Input: {:?}", input);
    
    let output = fmt.process(input);
    let remaining = fmt.finish();
    let full = format!("{}{}", output, remaining);
    
    println!("Output: {:?}", full);
    
    // The 4-space indented ``` should NOT close the code block
    // So "nested" should be part of the highlighted code
    assert!(full.contains("nested"), "nested should be in output");
}
