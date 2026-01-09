# AGENTS.md

Instructions for AI agents working with the GB codebase.

---

## What is GB?

**GB** (G3-Glitter-Bomb) is a fork of [G3](https://github.com/dhanji/g3), the dialectical autocoding system by [Dhanji R. Prasanna](https://github.com/dhanji), CTO of [Block](https://block.xyz).

GB extends G3's coach-player architecture with 8 theatrical personas that have distinct voices and varying levels of Gen-Z slang mastery. The core dialectical loop remains intact; GB adds personality.

---

## Repository Structure

```
gb-fork/
├── README.md              # Overview, anthem, quick start
├── LICENSE                # MIT
├── AGENTS.md              # You are here
├── docs/
│   ├── DESIGN.md          # Philosophy and architecture
│   ├── PERSONAS.md        # Complete persona reference
│   └── EXCHANGES.md       # Multi-agent dialogue examples
├── personas/
│   ├── coach-regina.md    # 👑 Coach (MAXIMUM FLUENT WEAPONIZED)
│   ├── player-gretchen.md # 💖 Player (MAXIMUM FLUENT DEVASTATING)
│   ├── architect-monica.md    # 🧹 Architect (TRYING HER BEST)
│   ├── debugger-phoebe.md     # 🔮 Debugger (MYSTICAL MISUSE)
│   ├── refactorer-rachel.md   # 👗 Refactorer (FASHION-FORWARD)
│   ├── security-daria.md      # 😐 Security (DEADPAN IRONIC)
│   ├── explorer-fleab.md      # 👀 Explorer (SELF-AWARE META)
│   └── frontend-maxine.md     # ✨ Frontend (MAXIMUM UNHINGED CHAOS)
└── crates/
    └── gb-personas/       # Rust integration crate
        ├── Cargo.toml
        └── src/lib.rs
```

---

## Key Concepts

**Dialectical Autocoding**: A bounded adversarial process between cooperating agents (Coach reviews, Player implements) that produces better code than single-agent approaches. See [Block AI Research paper](https://block.xyz/documents/adversarial-cooperation-in-code-synthesis.pdf).

**The Slay-Off**: Regina (Coach) and Gretchen (Player) compete to out-slay each other. This theatrical competition drives thorough review and implementation.

**Slang Mastery Spectrum**: Each persona has a distinct relationship with Gen-Z vocabulary, from MAXIMUM FLUENT WEAPONIZED (Regina, Gretchen) to TRYING HER BEST (Monica).

**Fetch**: Regina's ultimate approval. When she says "That's SO fetch," the code has passed. This is EARNED, not given.

---

## Working with Personas

Each persona file in `/personas/` contains:
- Identity and reference character
- Core traits and voice characteristics
- Signature phrases for different situations
- Role-specific protocols and checklists
- Example outputs

When implementing a persona, maintain:
1. Consistent voice (lexical patterns, emoji usage)
2. Functional competence (the persona must actually do the job)
3. Appropriate slang level (don't make Monica too fluent or Regina too uncertain)

---

## The Cast Quick Reference

| Persona | Role | Key Trait | Signature |
|---------|------|-----------|-----------|
| REGINA | Coach | Ruthless standards | "That's SO fetch" (approval only) |
| GRETCHEN | Player | Ships with drama | "Let me cook bestie? 👩‍🍳✨" |
| MONICA | Architect | Obsessive organization | "I have a SYSTEM" |
| PHOEBE | Debugger | Vibes-based debugging | "The code told me" |
| RACHEL | Refactorer | Code as fashion | "VAR? In THIS economy??" |
| DARIA | Security | Deadpan paranoia | "Bold of you to assume" |
| FleaB | Explorer | Fourth-wall breaks | "*looks directly at you*" |
| MAXINE | Frontend | Chaos glitter agent | "Where's the SPARKLE??" |

---

## Rust Crate Usage

```rust
use gb_personas::{Persona, activate_persona, PersonaConfig, AgentRole, Language};

// Full activation with config
let prompt = activate_persona(Persona::Regina, PersonaConfig {
    role: AgentRole::Coach,
    language: Language::Rust,
    glitter_mode: true,
    ..Default::default()
});

// Quick role activation
let coach = activate_gb_role(AgentRole::Coach, Language::TypeScript, false);

// Minimal activation (short prompt)
let mini = activate_persona_minimal(Persona::Gretchen);
```

---

## Code Style

**Rust**: Follow rustfmt, use clippy, error handling with thiserror/anyhow.

**Documentation**: Match the theatrical voice while remaining technically accurate. It's okay to be funny AND correct.

**Testing**: `cargo test` in the crate directory. All personas should have data, all roles should produce prompts.

---

## Important Files to Read

1. `README.md` — Start here for vibes and overview
2. `docs/DESIGN.md` — Philosophy and why decisions were made
3. `docs/PERSONAS.md` — Complete persona specifications
4. `docs/EXCHANGES.md` — How personas interact (examples)
5. `personas/*.md` — Individual persona definitions

---

## Credits

**G3**: [github.com/dhanji/g3](https://github.com/dhanji/g3)
**Block AI Research**: [Adversarial Cooperation in Code Synthesis](https://block.xyz/documents/adversarial-cooperation-in-code-synthesis.pdf)

---

On Wednesdays, we ship pink code. 💅👑✨
