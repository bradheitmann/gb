# Changelog

All notable changes to GB (G3-Glitter-Bomb) will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-01-27

### Fixed
- **Critical:** UTF-8 streaming in OpenAI provider now correctly handles multi-byte characters (emoji, Chinese, Japanese, etc.) split across chunk boundaries
  - Previously: Multi-byte characters were silently lost when split across SSE chunks
  - Now: Uses buffered UTF-8 decoder to preserve incomplete sequences
  - Impact: Fixes silent data corruption for all OpenAI-compatible providers (OpenRouter, Groq, Together)
- Pre-existing compilation error in streaming_markdown_test.rs (borrow checker issue)

### Added
- 10 comprehensive UTF-8 streaming tests in `crates/g3-providers/tests/streaming_utf8_test.rs`:
  - 4-byte emoji split across chunks (🎭)
  - 3-byte Chinese characters (中文)
  - Mixed ASCII + emoji + Chinese content
  - Multiple consecutive emoji sequences
  - GB persona emoji preservation in streaming (👑💖✨)
  - Consecutive multibyte sequences (real-world scenario)
  - ASCII regression test (no behavior change)
  - Edge cases: empty buffer, single byte, partial emoji at stream end

### Changed
- OpenAI provider now uses `decode_utf8_streaming()` utility (matches Anthropic provider implementation)
- Total: 9 lines of production code changed, 215 lines of tests added

### Technical Details
- Zero performance impact (buffering overhead is negligible)
- Fully backward compatible (ASCII-only streams unchanged)
- No new dependencies
- Upstreamable to G3 (no GB-specific code in fix)
- QA approved: 15/16 (adversarial review)
- All 634 tests pass (zero regressions)

## [0.2.1] - 2026-01-24

### Fixed
- VisionBridge dylib auto-copy to ~/.cargo/bin for cargo install
- `cargo install g3-glitter-bomb` now works out of the box on macOS without manual dylib handling

### Changed
- Updated all workspace crates to 0.2.1 for consistent crates.io publication

### Technical Details
- Modified build.rs to auto-copy libVisionBridge.dylib to ~/.cargo/bin during installation
- Ensures computer-control features work immediately after `cargo install`

## [0.2.0] - 2026-01-09

### Added (GB-Specific Features)
- **gb-personas crate:** 8 theatrical personas with Gen-Z dialect
  - Regina (👑 Coach) - Maximum fluent weaponized QA
  - Gretchen (💖 Player) - Maximum fluent devastating implementation
  - Monica (🧹 Architect) - Trying her best organizational excellence
  - Phoebe (🔮 Debugger) - Mystical misuse intuitive debugging
  - Rachel (👗 Refactorer) - Fashion-forward aesthetic code
  - Daria (😐 Security) - Deadpan ironic security auditing
  - FleaB (👀 Explorer) - Self-aware meta fourth-wall breaking
  - Maxine (✨ Frontend) - Maximum unhinged chaos UI sparkle

- **Inter-agent dialogue system:** Persona-aware logging for multi-agent orchestration
- **Enhanced session tracking:** Persona context preserved across sessions
- **Glitter mode:** Theatrical personality injection (configurable)

### Changed
- Forked from G3 v0.1.0 with theatrical enhancements
- Enhanced prompts with persona-specific language patterns
- Added persona activation system for autonomous mode

### Maintained from G3
- All 54 integration tests
- Streaming module extraction (309 lines)
- Provider/session module separation
- Unicode space handling (U+202F normalization)
- Auto-continue fix for interactive mode
- Background process management
- Full G3 feature compatibility

---

## G3 Upstream Compatibility

GB maintains compatibility with G3 by:
- Using the same core architecture and crates
- Preserving all G3 features and improvements
- Contributing fixes back upstream (v0.2.2 UTF-8 fix)
- Following G3 code conventions and patterns

GB adds theatrical personality as an optional layer on top of G3's solid foundation.

---

**For more information:**
- Repository: (GB GitHub URL if available)
- Issues: (GB Issues URL if available)
- G3 Upstream: (G3 repository reference)
- License: MIT/Apache-2.0
