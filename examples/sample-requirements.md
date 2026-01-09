# Sample Requirements for GB

Try these requirements with GB in autonomous mode:

## Quick Test (1-2 minutes)
```bash
gb --autonomous --requirements "Create a Rust function that reverses a string" --max-turns 1
```

## Email Validator (5-10 minutes)
```bash
gb --autonomous --requirements "Create a Rust function that validates email addresses using regex. Include doc comments and unit tests." --max-turns 3
```

## URL Parser (5-10 minutes)
```bash
gb --autonomous --requirements "Create a Rust module that parses URLs and extracts components (scheme, host, port, path, query). Include comprehensive tests." --max-turns 3
```

## JSON Config Parser (10-15 minutes)
```bash
gb --autonomous --requirements "Create a Rust module that reads JSON config files and validates them against a schema. Support nested objects, arrays, and provide clear error messages for validation failures." --max-turns 5
```

## CLI Todo App (15-20 minutes)
```bash
gb --autonomous --requirements "Create a simple CLI todo app in Rust with commands: add, list, done, remove. Store todos in a JSON file. Include clap for argument parsing." --max-turns 5
```

---

## Tips

1. **Turns**: More complex tasks need more turns for the Regina/Gretchen loop
2. **Specificity**: Be specific about what you want (tests, error handling, etc.)
3. **Watch the show**: The personas make the process entertaining!

---

*Let me cook bestie?*
