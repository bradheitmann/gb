//! Quick test to verify final_output rendering works with streaming markdown
//! Run with: cargo test -p g3-cli --test test_final_output -- --nocapture

use std::io::{self, Write};

#[test]
fn test_final_output_visual() {
    use g3_cli::streaming_markdown::StreamingMarkdownFormatter;
    use termimad::MadSkin;

    // Create the test markdown
    let test_markdown = r##"# Task Completed Successfully

Here's a summary of what was accomplished:

## Rust Code Example

Created a new function to handle user authentication:

```rust
use std::collections::HashMap;

/// Authenticates a user with the given credentials
pub async fn authenticate(username: &str, password: &str) -> Result<User, AuthError> {
    let hash = hash_password(password)?;
    
    if let Some(user) = db.find_user(username).await? {
        if user.password_hash == hash {
            Ok(user)
        } else {
            Err(AuthError::InvalidPassword)
        }
    } else {
        Err(AuthError::UserNotFound)
    }
}
```

## Python Example

Also added a Python script for data processing:

```python
import pandas as pd
from typing import List, Dict

def process_data(items: List[Dict]) -> pd.DataFrame:
    """Process raw items into a cleaned DataFrame."""
    df = pd.DataFrame(items)
    df['timestamp'] = pd.to_datetime(df['timestamp'])
    df = df.dropna(subset=['value'])
    return df.sort_values('timestamp')
```

## JavaScript/TypeScript

Frontend component:

```typescript
interface User {
  id: string;
  name: string;
  email: string;
}

const UserCard: React.FC<{ user: User }> = ({ user }) => {
  return (
    <div className="user-card">
      <h3>{user.name}</h3>
      <p>{user.email}</p>
    </div>
  );
};
```

## Shell Commands

Deployment script:

```bash
#!/bin/bash
set -euo pipefail

echo "Building project..."
cargo build --release

echo "Running tests..."
cargo test --all

echo "Deploying to production..."
rsync -avz ./target/release/app server:/opt/app/
```

## JSON Configuration

```json
{
  "name": "my-project",
  "version": "1.0.0",
  "dependencies": {
    "serde": "1.0",
    "tokio": { "version": "1.0", "features": ["full"] }
  }
}
```

## Other Markdown Features

This section tests that **bold text**, *italic text*, and `inline code` still work correctly.

### Lists

- First item
- Second item with **bold**
- Third item with `code`

### Numbered List

1. Step one
2. Step two
3. Step three

### Blockquote

> This is a blockquote that should be rendered
> with proper styling by termimad.

### Table

| Language | Extension | Use Case |
|----------|-----------|----------|
| Rust | .rs | Systems |
| Python | .py | Scripts |
| TypeScript | .ts | Frontend |

## Code Without Language

```
This is a code block without a language specified.
It should still be rendered as code, just without
syntax highlighting.
```

## Final Notes

All changes have been tested and verified. The implementation:

- ✅ Handles multiple languages
- ✅ Preserves markdown formatting
- ✅ Works with nested structures
- ✅ Gracefully handles edge cases
"##;

    // Create a styled markdown skin (same as in print_final_output)
    let mut skin = MadSkin::default();
    skin.bold.set_fg(termimad::crossterm::style::Color::Green);
    skin.italic.set_fg(termimad::crossterm::style::Color::Cyan);
    skin.inline_code.set_fg(termimad::crossterm::style::Color::Rgb {
        r: 216,
        g: 177,
        b: 114,
    });

    // Print header
    println!("\n\x1b[1;35m━━━ Summary ━━━\x1b[0m\n");

    // Use the streaming markdown formatter (same as print_final_output now uses)
    let mut formatter = StreamingMarkdownFormatter::new(skin);
    let formatted = formatter.process(test_markdown);
    print!("{}", formatted);
    let remaining = formatter.finish();
    print!("{}", remaining);

    // Print footer
    println!("\n\x1b[1;35m━━━━━━━━━━━━━━━\x1b[0m");

    let _ = io::stdout().flush();
}
