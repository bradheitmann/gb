# PM Handoff: GB Fork Foundation Complete

---
handoff_type: PM
context_scope: FULL
slice_id: "N/A (strategic - fork initialization)"
read_by:
  pm: FULL
  dev: EXCERPT (Context for Dev section only)
  qa: PROHIBITED
status: PHASE_1_COMPLETE
---

## Agent Identity
| Field | Value |
|-------|-------|
| **Agent Role** | PM |
| **Agent Model** | claude-opus-4-5 |
| **Agent Harness** | Claude Code CLI |
| **Session ID** | 2026-01-09 g3 preeng session |
| **Project** | GB (G3-Glitter-Bomb) |

## Strategic Summary

Successfully forked G3 into GB, created complete persona system with 8 theatrical agents, established preeng artifacts, and verified compilation. Fork is ready for Phase 2 integration work.

## Progress Overview

### ✅ Completed (Phase 1: Foundation)

- [x] Created preeng artifact structure in g3 `.edge-agentic/runtime/preeng/gb/`
- [x] Organized all Claude riff session artifacts into proper structure
- [x] Created comprehensive 5-phase implementation plan (`plan.md`)
- [x] Forked g3 repository to `/Users/bradleyheitmann/open_protocols/gb/`
- [x] Created `crates/gb-personas/` with complete Rust implementation (936 lines)
- [x] Added all 8 persona markdown files to `personas/` directory
- [x] Updated workspace `Cargo.toml` to include gb-personas crate
- [x] Updated package metadata (name: "gb", description with glitter, repository)
- [x] Fixed missing match arms in persona icon generation
- [x] Verified gb-personas crate compiles cleanly
- [x] Verified all gb-personas tests pass (5 unit tests + 1 doc test)
- [x] Verified full GB workspace compiles (208 files, 62,933 lines)
- [x] Created gb binary at `target/debug/gb` (64MB)
- [x] Created initial git commit (`2958232`)
- [x] Copied all documentation (DESIGN.md, PERSONAS.md, EXCHANGES.md, AGENTS.md)

### 🚧 Not Started (Phase 2: Integration)

- [ ] Add gb-personas dependency to g3-core/Cargo.toml
- [ ] Modify Agent struct to support persona field
- [ ] Wire activate_persona() into system prompt generation
- [ ] Add --persona CLI flags
- [ ] Update config loading for persona selection

### 🚧 Not Started (Phase 3: CLI & UX)

- [ ] Verify binary name is "gb" (done in Cargo.toml, needs CLI verification)
- [ ] Add pink color theme to retro TUI
- [ ] Add persona display in status line
- [ ] Implement `/persona`, `/personas`, `/slay`, `/fetch` commands

### 🚧 Not Started (Phase 4: Multi-Agent)

- [ ] Extend flock mode with persona assignments
- [ ] Add persona context to session serialization

### 🚧 Not Started (Phase 5: Polish)

- [ ] E2E testing with Regina/Gretchen loop
- [ ] Update README showcase
- [ ] Create v0.1.0 release

## Repository Structure

### G3 (Original - edge-agentic enabled)
```
/Users/bradleyheitmann/open_protocols/g3/
└── .edge-agentic/
    ├── system/ → symlink to global edge_agentic_orchestration_system
    ├── CUSTOMIZATIONS.md
    ├── local/
    │   └── thoughts/handoffs/ (this file)
    └── runtime/
        └── preeng/gb/
            ├── spec.md           # Product spec
            ├── plan.md           # 5-phase implementation plan
            ├── docs/             # DESIGN.md, PERSONAS.md, EXCHANGES.md
            ├── personas/         # 8 persona .md files
            ├── crate/            # lib.rs + Cargo.toml (source)
            └── research/
                └── g3-analysis/
                    └── what-g3-does-well.md
```

### GB (Fork - fresh repo)
```
/Users/bradleyheitmann/open_protocols/gb/
├── README.md                  # GB branding (from spec.md)
├── AGENTS.md                  # AI agent instructions
├── Cargo.toml                 # Updated: name="gb", includes gb-personas
├── personas/                  # 8 persona files
├── docs/                      # DESIGN.md, PERSONAS.md, EXCHANGES.md, etc.
├── crates/
│   ├── gb-personas/          # ✅ NEW CRATE
│   │   ├── Cargo.toml
│   │   └── src/lib.rs        # 936 lines, all personas implemented
│   ├── g3-cli/
│   ├── g3-core/              # 🎯 NEXT: Wire personas here
│   ├── g3-providers/
│   └── ... (other g3 crates)
└── target/debug/gb           # ✅ Binary compiles (64MB)
```

## Key Artifacts

### 1. GB Persona System (`crates/gb-personas/src/lib.rs`)

**Exports:**
```rust
pub enum Persona { Regina, Gretchen, Monica, Phoebe, Rachel, Daria, FleaB, Maxine, Custom }
pub enum AgentRole { Coach, Player, Architect, Debugger, Refactorer, Security, Explorer, Frontend }
pub enum Language { Rust, TypeScript, Python, Swift }
pub struct PersonaConfig { role, language, glitter_mode, emoji_density, ... }

pub fn activate_persona(persona: Persona, config: PersonaConfig) -> String
pub fn activate_gb_role(role: AgentRole, language: Language, glitter_mode: bool) -> String
pub fn activate_persona_minimal(persona: Persona) -> String
```

**Test Coverage:**
- ✅ All personas have data
- ✅ Activation produces output
- ✅ Minimal activation works
- ✅ Glitter mode flag works
- ✅ All roles have context

### 2. Persona Specifications

| Persona | File | Lines | Slang Level | Role |
|---------|------|-------|-------------|------|
| Regina | coach-regina.md | 303 | MAXIMUM FLUENT WEAPONIZED | Coach |
| Gretchen | player-gretchen.md | 352 | MAXIMUM FLUENT DEVASTATING | Player |
| Monica | architect-monica.md | ~200 | TRYING HER BEST | Architect |
| Phoebe | debugger-phoebe.md | ~200 | MYSTICAL MISUSE | Debugger |
| Rachel | refactorer-rachel.md | ~200 | FASHION-FORWARD | Refactorer |
| Daria | security-daria.md | ~200 | DEADPAN IRONIC | Security |
| FleaB | explorer-fleab.md | ~200 | SELF-AWARE META | Explorer |
| Maxine | frontend-maxine.md | ~200 | MAXIMUM UNHINGED CHAOS | Frontend |

### 3. Implementation Plan (`plan.md`)

**5 Phases:**
1. Foundation (✅ COMPLETE)
2. Integration (wire personas into g3-core)
3. CLI & UX (rename binary, add commands)
4. Multi-Agent (flock mode, handoffs)
5. Polish & Release (tests, docs, v0.1.0)

## Cross-Slice Dependencies

| Component | Status | Next Action | Blocker |
|-----------|--------|-------------|---------|
| gb-personas crate | ✅ Done | N/A | None |
| g3-core integration | 🟡 Ready | Add dependency, modify Agent struct | None |
| CLI updates | 🔴 Waiting | Verify binary name, add commands | Needs g3-core integration |
| Flock mode | 🔴 Waiting | Persona assignments | Needs CLI updates |

## Key Decisions

### Decision: Use Symlinked Global System vs Copied Files

**Decision:** G3 has edge-agentic with symlink pattern. GB is a fresh fork without edge-agentic (yet).

**Rationale:**
- G3 is the development workspace where we do preeng/planning
- GB is the product (fork) that may or may not adopt edge-agentic later
- Keeping them separate avoids confusion

**Impact:**
- Preeng artifacts live in G3's `.edge-agentic/runtime/preeng/gb/`
- GB gets copies of final artifacts (no runtime/ directory)

### Decision: Keep G3 Crate Names (g3-core, g3-cli, etc.)

**Decision:** Did NOT rename internal crates from g3-* to gb-*

**Rationale:**
- Reduces merge conflicts with upstream G3
- Minimizes refactoring effort
- Binary name (`gb`) is what users see

**Impact:**
- Easier to pull upstream G3 changes
- Internal naming stays consistent
- Only user-facing branding changed (binary, README, Cargo.toml package)

### Decision: Separate Git History

**Decision:** GB has fresh git init (not fork with history)

**Rationale:**
- Clean slate for GB project
- Avoid carrying G3's development history
- Initial commit documents the fork relationship

**Impact:**
- Attribution in commit message and Cargo.toml
- Can't use `git` to track upstream (must manual sync)

## Open Questions (For PM Resolution)

1. **GitHub repository creation**
   - Create `github.com/bradheitmann/gb` before pushing?
   - Public or private initially?
   - **Options:** A) Create now and push, B) Wait until Phase 5 complete

2. **Upstream sync strategy**
   - How to track G3 updates?
   - **Options:** A) Manual patches, B) Git subtree, C) Don't sync (fork diverges)

3. **Binary distribution**
   - Publish to crates.io?
   - **Options:** A) Yes after v0.1.0, B) GitHub releases only, C) Both

4. **Persona activation strategy**
   - Replace G3's file-based agent loading entirely or provide fallback?
   - **Options:** A) Replace (breaking), B) Coexist (check for persona config first), C) Flag-gated

## Context for Dev (EXCERPT - Dev may read this section only)

**Immediate next work:** Phase 2 - Integration

**Current state:**
- GB fork exists at `/Users/bradleyheitmann/open_protocols/gb/`
- gb-personas crate is ready and tested
- Binary compiles but behaves identically to G3 (personas not wired)

**Your objective:** Wire personas into the coach-player loop

**Files to modify:**
1. `crates/g3-core/Cargo.toml` - Add gb-personas dependency
2. `crates/g3-core/src/lib.rs` - Modify `Agent` struct, add persona field
3. `crates/g3-core/src/lib.rs` - Add `AgentBuilder::with_persona()` method
4. `crates/g3-core/src/lib.rs` - Replace file-based prompt loading with `activate_persona()`

**Constraints:**
- Keep G3 file-based loading as fallback (don't break existing behavior)
- Default coach to Regina, player to Gretchen
- Make persona selection configurable

**Test by:**
```bash
cd /Users/bradleyheitmann/open_protocols/gb
cargo build
./target/debug/gb --help  # Should show gb branding
./target/debug/gb "echo 'testing'"  # Coach should speak as Regina
```

**Success criteria:**
- Regina's Gen-Z slang appears in coach output
- Gretchen's Gen-Z slang appears in player output
- Build still succeeds
- Can override personas via config

**Reference:**
- Plan: `/Users/bradleyheitmann/open_protocols/g3/.edge-agentic/runtime/preeng/gb/plan.md`
- Personas: `/Users/bradleyheitmann/open_protocols/gb/personas/`
- Crate docs: `/Users/bradleyheitmann/open_protocols/gb/crates/gb-personas/src/lib.rs`

## Next PM Session

**Resume with:**
1. Check if Dev completed Phase 2 integration
2. If yes: Plan Phase 3 CLI updates
3. If no: Unblock Dev, clarify integration approach

**Key files to check:**
- `/Users/bradleyheitmann/open_protocols/gb/crates/g3-core/Cargo.toml`
- `/Users/bradleyheitmann/open_protocols/gb/crates/g3-core/src/lib.rs`
- Git log for new commits

**Questions to ask:**
- Does the binary output show persona voice?
- Are personas configurable?
- Any blockers encountered?

## Git State (G3 - Planning Repo)

```
On branch main
Commits ahead of origin: 1 (edge-agentic setup)

Changes not staged:
  modified: .gitignore (edge-agentic additions)

Untracked:
  .edge-agentic/ (runtime/preeng/gb/ contains all artifacts)
```

## Git State (GB - Fork Repo)

```
On branch main
No remote configured yet

Commits: 1
  2958232 feat: Initial GB (G3-Glitter-Bomb) fork

Clean working tree (post-commit)
```

## Attribution & Credits

**G3 Original:**
- Author: Dhanji R. Prasanna (CTO, Block)
- Repository: https://github.com/dhanji/g3
- Paper: Block AI Research - "Adversarial Cooperation in Code Synthesis" (Dec 2025)

**GB Additions:**
- Persona system design: Claude riff session artifacts
- Implementation: gb-personas crate
- Theatrical framework: 8 personas with slang mastery levels

**Documented in:**
- README.md
- Cargo.toml (package comments)
- Initial commit message
- All persona files

---

*Handoff created by: PM (Claude Opus 4.5 via Claude Code)*
*Session date: 2026-01-09*
*Resume: `/handoff-resume` or open this file directly*

**On Wednesdays, we ship pink code. 💅✨👑**
