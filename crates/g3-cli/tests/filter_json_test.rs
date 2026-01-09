//! Tests for JSON tool call filtering.
//!
//! These tests verify that the filter correctly identifies and removes JSON tool calls
//! from LLM output streams while preserving all other content.

#[cfg(test)]
mod filter_json_tests {
    use g3_cli::filter_json::{filter_json_tool_calls, reset_json_tool_state};
    use regex::Regex;

    /// Test that regular text without tool calls passes through unchanged.
    #[test]
    fn test_no_tool_call_passthrough() {
        reset_json_tool_state();
        let input = "This is regular text without any tool calls.";
        let result = filter_json_tool_calls(input);
        assert_eq!(result, input);
    }

    /// Test detection and removal of a complete tool call in a single chunk.
    #[test]
    fn test_simple_tool_call_detection() {
        reset_json_tool_state();
        let input = r#"Some text before
{"tool": "shell", "args": {"command": "ls"}}
Some text after"#;

        let result = filter_json_tool_calls(input);
        let expected = "Some text before\n\nSome text after";
        assert_eq!(result, expected);
    }

    /// Test handling of tool calls that arrive across multiple streaming chunks.
    #[test]
    fn test_streaming_chunks() {
        reset_json_tool_state();

        // Simulate streaming where the tool call comes in multiple chunks
        let chunks = vec![
            "Some text before\n",
            "{\"tool\": \"",
            "shell\", \"args\": {",
            "\"command\": \"ls\"",
            "}}\nText after",
        ];

        let mut results = Vec::new();
        for chunk in chunks {
            let result = filter_json_tool_calls(chunk);
            results.push(result);
        }

        // The final accumulated result should have the JSON filtered out
        let final_result: String = results.join("");
        let expected = "Some text before\n\nText after";
        assert_eq!(final_result, expected);
    }

    /// Test correct handling of nested braces within JSON strings.
    #[test]
    fn test_nested_braces_in_tool_call() {
        reset_json_tool_state();

        let input = r#"Text before
{"tool": "write_file", "args": {"file_path": "test.json", "content": "{\"nested\": \"value\"}"}}
Text after"#;

        let result = filter_json_tool_calls(input);
        let expected = "Text before\n\nText after";
        assert_eq!(result, expected);
    }

    /// Verify the regex pattern matches the specification with flexible whitespace.
    #[test]
    fn test_regex_pattern_specification() {
        // Test the corrected regex pattern that's more flexible with whitespace
        let pattern = Regex::new(r#"(?m)^\s*\{\s*"tool"\s*:"#).unwrap();

        let test_cases = vec![
            (
                r#"line
{"tool":"#,
                true,
            ),
            (
                r#"line
{"tool" :"#,
                true,
            ),
            (
                r#"line
{ "tool":"#,
                true,
            ), // Space after { DOES match with \s*
            (
                r#"line
{"tool123":"#,
                false,
            ), // "tool123" is not exactly "tool"
            (
                r#"line
{"tool" : "#,
                true,
            ),
        ];

        for (input, should_match) in test_cases {
            let matches = pattern.is_match(input);
            assert_eq!(
                matches, should_match,
                "Pattern matching failed for: {}",
                input
            );
        }
    }

    /// Test that tool calls must appear at the start of a line (after newline).
    #[test]
    fn test_newline_requirement() {
        reset_json_tool_state();

        // According to spec, tool call should be detected "on the very next newline"
        // Our current regex matches any line that contains the pattern, not just after newlines
        let input_with_newline = "Text\n{\"tool\": \"shell\", \"args\": {\"command\": \"ls\"}}";
        let input_without_newline = "Text {\"tool\": \"shell\", \"args\": {\"command\": \"ls\"}}";

        let result1 = filter_json_tool_calls(input_with_newline);
        reset_json_tool_state();
        let result2 = filter_json_tool_calls(input_without_newline);

        // With the new aggressive filtering, only the newline case should trigger suppression
        // The pattern requires { to be at the start of a line (after ^)
        assert_eq!(result1, "Text\n");
        // Without newline before {, it should pass through unchanged
        assert_eq!(result2, input_without_newline);
    }

    /// Test handling of escaped quotes within JSON strings.
    #[test]
    fn test_json_with_escaped_quotes() {
        reset_json_tool_state();

        let input = r#"Text
{"tool": "write_file", "args": {"content": "He said \"hello\" to me"}}
More text"#;

        let result = filter_json_tool_calls(input);
        let expected = "Text\n\nMore text";
        assert_eq!(result, expected);
    }

    /// Test graceful handling of incomplete/malformed JSON.
    #[test]
    fn test_edge_case_malformed_json() {
        reset_json_tool_state();

        // Test what happens with malformed JSON that starts like a tool call
        let input = r#"Text
{"tool": "shell", "args": {"command": "ls"
More text"#;

        let result = filter_json_tool_calls(input);
        // Should handle gracefully - since JSON is incomplete, it should return content before JSON
        let expected = "Text\n";
        assert_eq!(result, expected);
    }

    /// Test processing multiple independent tool calls sequentially.
    #[test]
    fn test_multiple_tool_calls_sequential() {
        reset_json_tool_state();

        // Test processing multiple tool calls one at a time
        let input1 = r#"First text
{"tool": "shell", "args": {"command": "ls"}}
Middle text"#;
        let result1 = filter_json_tool_calls(input1);
        let expected1 = "First text\n\nMiddle text";
        assert_eq!(result1, expected1);

        // Reset and process second tool call
        reset_json_tool_state();
        let input2 = r#"More text
{"tool": "read_file", "args": {"file_path": "test.txt"}}
Final text"#;
        let result2 = filter_json_tool_calls(input2);
        let expected2 = "More text\n\nFinal text";
        assert_eq!(result2, expected2);
    }

    /// Test tool calls with complex multi-line arguments.
    #[test]
    fn test_tool_call_with_complex_args() {
        reset_json_tool_state();

        let input = r#"Before
{"tool": "str_replace", "args": {"file_path": "test.rs", "diff": "--- old\n-old line\n+++ new\n+new line", "start": 0, "end": 100}}
After"#;

        let result = filter_json_tool_calls(input);
        let expected = "Before\n\nAfter";
        assert_eq!(result, expected);
    }

    /// Test input containing only a tool call with no surrounding text.
    #[test]
    fn test_tool_call_only() {
        reset_json_tool_state();

        let input = r#"
{"tool": "final_output", "args": {"summary": "Task completed successfully"}}"#;

        let result = filter_json_tool_calls(input);
        let expected = "\n";
        assert_eq!(result, expected);
    }

    /// Test accurate brace counting with deeply nested structures.
    #[test]
    fn test_brace_counting_accuracy() {
        reset_json_tool_state();

        // Test complex nested structure
        let input = r#"Start
{"tool": "write_file", "args": {"content": "function() { return {a: 1, b: {c: 2}}; }", "file_path": "test.js"}}
End"#;

        let result = filter_json_tool_calls(input);
        let expected = "Start\n\nEnd";
        assert_eq!(result, expected);
    }

    /// Test that braces within strings don't affect brace counting.
    #[test]
    fn test_string_escaping_in_json() {
        reset_json_tool_state();

        // Test JSON with escaped quotes and braces in strings
        let input = r#"Text
{"tool": "shell", "args": {"command": "echo \"Hello {world}\" > file.txt"}}
More"#;

        let result = filter_json_tool_calls(input);
        let expected = "Text\n\nMore";
        assert_eq!(result, expected);
    }

    /// Verify compliance with the exact specification requirements.
    #[test]
    fn test_specification_compliance() {
        reset_json_tool_state();

        // Test the exact specification requirements:
        // 1. Detect start with regex '\w*{\w*"tool"\w*:\w*"' on newline
        // 2. Enter suppression mode and use brace counting
        // 3. Elide only JSON between first '{' and last '}' (inclusive)
        // 4. Return everything else

        let input = "Before text\nSome more text\n{\"tool\": \"test\", \"args\": {}}\nAfter text\nMore after";
        let result = filter_json_tool_calls(input);
        let expected = "Before text\nSome more text\n\nAfter text\nMore after";
        assert_eq!(result, expected);
    }

    /// Test that non-tool JSON objects are not filtered.
    #[test]
    fn test_no_false_positives() {
        reset_json_tool_state();

        // Test that we don't incorrectly identify non-tool JSON as tool calls
        let input = r#"Some text
{"not_tool": "value", "other": "data"}
More text"#;
        let result = filter_json_tool_calls(input);
        // Should pass through unchanged since it doesn't match the tool pattern
        assert_eq!(result, input);
    }

    /// Test patterns that look similar to tool calls but aren't exact matches.
    #[test]
    fn test_partial_tool_patterns() {
        reset_json_tool_state();

        // Test patterns that look like tool calls but aren't complete
        let test_cases = vec![
            "Text\n{\"too\": \"value\"}",   // "too" not "tool"
            "Text\n{\"tools\": \"value\"}", // "tools" not "tool"
            "Text\n{\"tool\": }",           // Missing value after colon
        ];

        for input in test_cases {
            reset_json_tool_state();
            let result = filter_json_tool_calls(input);
            // These should all pass through unchanged
            assert_eq!(result, input, "Input should pass through: {}", input);
        }
    }

    /// Test streaming with very small chunks (character-by-character).
    #[test]
    fn test_streaming_edge_cases() {
        reset_json_tool_state();

        // Test streaming with very small chunks
        let chunks = vec![
            "Text\n", "{", "\"", "tool", "\"", ":", " ", "\"", "test", "\"", "}", "\nAfter",
        ];

        let mut results = Vec::new();
        for chunk in chunks {
            let result = filter_json_tool_calls(chunk);
            results.push(result);
        }

        let final_result: String = results.join("");
        // With the new aggressive filtering, the JSON should be completely filtered out
        // even when it arrives in very small chunks
        let expected = "Text\n\nAfter";
        assert_eq!(final_result, expected);
    }

    /// Debug test with detailed logging for streaming behavior.
    #[test]
    fn test_streaming_debug() {
        reset_json_tool_state();

        // Debug the exact failing case
        let chunks = vec![
            "Some text before\n",
            "{\"tool\": \"",
            "shell\", \"args\": {",
            "\"command\": \"ls\"",
            "}}\nText after",
        ];

        let mut results = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let result = filter_json_tool_calls(chunk);
            println!("Chunk {}: {:?} -> {:?}", i, chunk, result);
            results.push(result);
        }

        let final_result: String = results.join("");
        println!("Final result: {:?}", final_result);
        println!("Expected: {:?}", "Some text before\n\nText after");

        let expected = "Some text before\n\nText after";
        assert_eq!(final_result, expected);
    }

    /// Test handling of truncated JSON followed by complete JSON (the json_err pattern)
    #[test]
    fn test_truncated_then_complete_json() {
        reset_json_tool_state();

        // Simulate the pattern from json_err trace:
        // 1. Incomplete/truncated JSON appears
        // 2. Then the same complete JSON appears
        let chunks = vec![
            "Some text\n",
            r#"{"tool": "str_replace", "args": {"diff":"...","file_path":"./crates/g3-cli"#, // Truncated
            r#"{"tool": "str_replace", "args": {"diff":"...","file_path":"./crates/g3-cli/src/lib.rs"}}"#, // Complete
            "\nMore text",
        ];

        let mut results = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let result = filter_json_tool_calls(chunk);
            println!("Chunk {}: {:?} -> {:?}", i, chunk, result);
            results.push(result);
        }

        let final_result: String = results.join("");
        println!("Final result: {:?}", final_result);

        // The truncated JSON should be discarded when the complete one appears
        // Both JSONs should be filtered out, leaving only the text
        let expected = "Some text\n\nMore text";
        assert_eq!(
            final_result, expected,
            "Failed to handle truncated JSON followed by complete JSON"
        );
    }

    // ============================================================================
    // Edge Case Tests - These test the bugs that were fixed in the rewrite
    // ============================================================================

    /// CRITICAL: Test that closing braces inside JSON strings don't break filtering.
    /// This was the main bug in the original implementation.
    #[test]
    fn test_brace_inside_json_string_value() {
        reset_json_tool_state();

        // The } inside "echo }" should NOT cause premature exit from suppression
        let input = r#"Text before
{"tool": "shell", "args": {"command": "echo }"}}
Text after"#;

        let result = filter_json_tool_calls(input);
        let expected = "Text before\n\nText after";
        assert_eq!(
            result, expected,
            "Brace inside string value caused premature suppression exit"
        );
    }

    /// Test multiple braces inside string values.
    #[test]
    fn test_multiple_braces_in_string() {
        reset_json_tool_state();

        let input = r#"Before
{"tool": "shell", "args": {"command": "echo {{{}}}"}}
After"#;

        let result = filter_json_tool_calls(input);
        let expected = "Before\n\nAfter";
        assert_eq!(result, expected);
    }

    /// Test escaped quotes followed by braces in strings.
    #[test]
    fn test_escaped_quotes_with_braces() {
        reset_json_tool_state();

        let input = r#"Before
{"tool": "shell", "args": {"command": "echo \"test}\" done"}}
After"#;

        let result = filter_json_tool_calls(input);
        let expected = "Before\n\nAfter";
        assert_eq!(result, expected);
    }

    /// Test braces in strings across streaming chunks.
    #[test]
    fn test_brace_in_string_across_chunks() {
        reset_json_tool_state();

        // The } appears in a separate chunk while we're inside a string
        let chunks = vec![
            "Before\n",
            r#"{"tool": "shell", "args": {"command": "echo "#,
            r#"}"}}"#,
            "\nAfter",
        ];

        let mut results = Vec::new();
        for chunk in chunks {
            results.push(filter_json_tool_calls(chunk));
        }

        let final_result: String = results.join("");
        let expected = "Before\n\nAfter";
        assert_eq!(
            final_result, expected,
            "Brace in string across chunks caused incorrect filtering"
        );
    }

    /// Test complex nested JSON with braces in multiple string values.
    #[test]
    fn test_complex_nested_with_string_braces() {
        reset_json_tool_state();

        let input = r#"Start
{"tool": "write_file", "args": {"path": "test.json", "content": "{\"key\": \"value with } brace\"}"}}
End"#;

        let result = filter_json_tool_calls(input);
        let expected = "Start\n\nEnd";
        assert_eq!(result, expected);
    }

    /// Test the real-world case from jsonfilter_err - str_replace with diff containing braces
    #[test]
    fn test_str_replace_with_diff_content() {
        reset_json_tool_state();

        // This is a real case where str_replace tool call wasn't being filtered
        // The diff content contains braces in the code being replaced
        let input = r#"{"tool": "str_replace", "args": {"diff":"--- a/crates/g3-cli/src/ui_writer_impl.rs\n+++ b/crates/g3-cli/src/ui_writer_impl.rs\n@@ -355,11 +355,11 @@\n     fn filter_json_tool_calls(&self, content: &str) -> String {\n         // Apply JSON tool call filtering for display\n-        fixed_filter_json_tool_calls(content)\n+        filter_json_tool_calls(content)\n     }\n \n     fn reset_json_filter(&self) {\n         // Reset the filter state for a new response\n-        reset_fixed_json_tool_state();\n+        reset_json_tool_state();\n     }\n }","file_path":"crates/g3-cli/src/ui_writer_impl.rs"}}"#;

        let result = filter_json_tool_calls(input);
        
        // The entire tool call should be filtered out
        assert!(
            result.is_empty() || result.trim().is_empty(),
            "str_replace tool call was not filtered out. Got: {:?}",
            result
        );
    }

    /// Test tool call that appears after other content (from jsonfilter_err)
    /// The filter requires tool calls to start at the beginning of a line
    #[test]
    fn test_tool_call_after_other_content() {
        reset_json_tool_state();

        // This simulates the jsonfilter_err case where a read_file result
        // is followed by a str_replace tool call
        let input = r#"┌─ read_file | ./crates/g3-cli/src/ui_writer_impl.rs [13000..13300]
│     }
│ (11 lines)
└─ ⚡️ 1ms

{"tool": "str_replace", "args": {"diff":"--- a/file.rs\n+++ b/file.rs\n-old\n+new","file_path":"file.rs"}}"#;

        let result = filter_json_tool_calls(input);
        
        // The tool call starts on its own line after the read_file output,
        // so it should be filtered out. Only the read_file output should remain.
        let expected = r#"┌─ read_file | ./crates/g3-cli/src/ui_writer_impl.rs [13000..13300]
│     }
│ (11 lines)
└─ ⚡️ 1ms

"#;
        assert_eq!(
            result, expected,
            "Tool call after other content was not filtered correctly"
        );
    }

    /// Test case from jsonfilter_err2 - tool call at line start should be filtered,
    /// but tool call patterns inside string values should be preserved
    #[test]
    fn test_tool_call_with_nested_tool_pattern_in_string() {
        reset_json_tool_state();

        // From jsonfilter_err2: A shell tool call that contains another tool call
        // pattern inside its command string (a heredoc with code that references tool calls)
        // The outer shell tool call starts at line beginning -> should be filtered
        // The inner str_replace pattern is inside a string -> should NOT trigger filtering
        let input = "Let me create a test case:\n\n{\"tool\": \"shell\", \"args\": {\"command\":\"cat file.rs\\nlet x = r#\\\"{\\\"tool\\\": \\\"test\\\"}\\\"#;\"}}\n\nDone.";

        let result = filter_json_tool_calls(input);
        
        // The shell tool call starts at line beginning, so it should be filtered out
        // Only the surrounding text should remain
        // Note: The tool call is on its own line, so filtering leaves an empty line
        let expected = "Let me create a test case:\n\n\n\nDone.";
        
        assert_eq!(
            result, expected,
            "Tool call with nested pattern was not filtered correctly. Got: {:?}",
            result
        );
    }
}
