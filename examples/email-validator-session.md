# Example Session: Email Validator

This example shows a real GB session implementing an email validator in Rust.

## Requirements

```
Create a Rust function that validates email addresses using regex.
Include doc comments and unit tests.
```

## Command

```bash
gb --autonomous --requirements "Create a Rust function that validates email addresses using regex. Include doc comments and unit tests." --max-turns 3
```

## Session Output

### Turn 1/3 - Player Mode (Gretchen)

Gretchen starts with enthusiasm:

```
OMG hi bestie! I'm SO ready to implement this email validator!
Let me make this FETCH happen!

Okay so I need to create a Rust project with regex validation.
I'm literally going to try SO hard on this, no cap!
```

She creates:
- `Cargo.toml` with regex dependency
- `src/lib.rs` with comprehensive implementation

When tests fail initially:

```
OH NO The tests failed! I'm literally crying right now!
Let me see what went wrong...

OH I see the problem! The regex is accepting "user@com"
when it should need a dot! Let me fix this fr fr!
```

After fixing the regex:

```
YESSS! That worked! Now let me run the tests again!

WE MADE FETCH HAPPEN!!!
```

Final summary from Gretchen:

```
Bestie, I literally delivered the MOST PERFECT email validation module!

What I Created:
- Cargo.toml - Rust project with regex dependency (v1.11)
- src/lib.rs - Complete email validation module with:
  - Comprehensive doc comments (module and function level)
  - is_valid_email() function that validates emails using regex
  - 8 unit tests covering:
    - Valid email formats (11 test cases)
    - Invalid email formats (12 test cases)
    - Edge cases (empty strings, long inputs)
    - Case sensitivity testing
    - Special characters in local part
    - Subdomain support
    - Numeric TLDs
    - Unicode rejection

Code Quality:
- All tests passing (10 total tests: 8 unit + 2 doctests)
- Code formatted with rustfmt
- Zero clippy warnings
- Clean, idiomatic Rust code

Regina please... let me sit with you! I made fetch happen!
```

### Turn 1/3 - Coach Mode (Regina)

Regina's review:

```
Let me check if you can sit with us today.
```

She examines the code structure, runs tests, checks formatting...

Final verdict:

```
IMPLEMENTATION_APPROVED
```

## Session Statistics

| Metric | Value |
|--------|-------|
| Total Duration | 6m 23s |
| Turns Taken | 1/3 |
| Final Status | APPROVED |
| Token Usage | 21.3% of budget |

## Generated Code

The session produced a fully functional `email_validator` Rust crate with:

```rust
//! Email Validation Module
//!
//! This module provides functionality for validating email addresses using regex.
//! The validator follows standard email format rules and provides a simple, efficient API.

use regex::Regex;

/// Validates an email address against standard email format rules.
///
/// # Arguments
/// * `email` - A string slice containing the email address to validate
///
/// # Returns
/// * `true` if the email is valid
/// * `false` if the email is invalid
///
/// # Examples
/// ```
/// use email_validator::is_valid_email;
///
/// assert!(is_valid_email("user@example.com"));
/// assert!(!is_valid_email("invalid"));
/// ```
pub fn is_valid_email(email: &str) -> bool {
    let email_regex = Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$"
    ).expect("Invalid regex pattern");

    email_regex.is_match(email)
}
```

## Key Observations

1. **Persona Authenticity**: Gretchen's emotional reactions ("OH NO", "fr fr", "no cap") are consistent and engaging
2. **Self-Correction**: The player identified and fixed the regex bug autonomously
3. **Quality Standards**: Despite the theatrical persona, the code meets production standards
4. **Efficient Review**: Regina approved in just 1 turn - the implementation was solid

---

*On Wednesdays, we ship pink code.*
