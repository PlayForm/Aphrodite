---
name: rust-comment-quality
description: Use when replacing stale //! TODO markers with structured module documentation in the Aphrodite Rust crates.
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos, linux, windows]
category: engineering
category_taxonomy: engineering/rust-comment-quality
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, documenting, converting, scanning, ascii-writing]
        related_skills: [code-quality-improvement]
status: active
---

# Rust Comment Quality - Replace TODO markers

Upgrade stale action-marker TODOs (`//! TODO: ...`) into structured module
documentation that states implementation status: what's wired, what's
stubbed, what's planned.

## When to use

Module doc comments that read like action items rather than documentation:

```rust
//! Some feature description.
//!
//! TODO: zero callers. Wire from the engine's main entry.
```

Transform them into declarative prose - the reader sees status, not a chore
list.

## The transformation pattern

**Before:**

```rust
//! Terminal-emulation RPC service. Placeholder for the PTY +
//! shell-integration roadmap. TODO: zero callers.
```

**After:**

```rust
//! Terminal-emulation RPC service. Placeholder for the PTY +
//! shell-integration roadmap. Status: not yet wired; all exports are
//! cfg-gated behind the `terminals` feature.
```

### Section templates

- **Status** (stubs / wiring not done): `//! ## Status` + current status +
  what needs to happen and from where
- **Planned Work** (feature roadmaps): `//! ## Planned Work` + bullet list
  of planned items
- **Stub notices** (unimplemented command handlers): `//! ## Stub` + what
  the real implementation should handle
- **No-op shim notes** (intentionally empty placeholders): `//! ## No-op
shim` + why it's a no-op and what it preserves

Full templates: `templates/module-status-section.txt`,
`templates/module-planned-work-section.txt`.

## What to leave unchanged

**Inline implementation TODOs** inside function bodies stay as-is - they
describe "the next line of code to write" at a specific site, or legitimate
external work (e.g. "call the proxy's health-check endpoint"). The Aphrodite
codebase intentionally preserves line-level markers inside the proxy's
connection handlers in `crates/aphrodite-hermes/`; only module-doc-level
`//! TODO:` entries get converted. A file with 0 module-level TODOs but
several inline ones is healthy.

## Scope

Apply to `.rs`, `.ts`, `.js`, `.md`. Focus on **module-level doc comments**
(`//! ...`), not inline `// ...` in bodies. Target trees: `crates/aphrodite`,
`crates/aphrodite-hermes`, and the `vendor/headroom` fork (edition-2024;
keep its comments ASCII and gate-checked like the workspace crates).

Checklist:

- [ ] Scan for `//! TODO:` in the first ~25 lines of each source file
- [ ] Convert each into the appropriate structured section
- [ ] Do NOT touch inline implementation `// TODO:` markers (function-body
      level)
- [ ] Verify `cargo check -p <crate>` still passes after doc-only changes
- [ ] Verify `cargo fmt --check`; `npx prettier --check .hermes/**/*.md` if
      docs changed
- [ ] Single files: `patch`. Multi-file batches: `delegate_task`.

Batch-rewrite identical TODO phrases seen in >5 files with one consistent
phrasing template, preserving file-specific context (wire-from site, module
name).

## Lessons

- **Keep inline TODOs inline** - module-level sections capture category debt
  (what system X is missing); line-specific implementation debt stays in the
  function body. Moving TODOs up when they belong inline pollutes module
  docs.
- **Never make TODOs sound like finished work** - say "not yet wired" or
  "planned", not "implemented". Honest status phrasing only.
- **Read line-wrapped TODO text as prose before find-replacing** - multi-
  line mirror TODOs need horizontal whitespace cleanup when moved to
  `## Planned Work` lists.
- **Don't flag large docblocks as bloat** - 20+ line comment runs before any
  `pub`/`fn` are usually module rustdocs (`//!` or `///`); only dense inline
  runs inside function bodies are suspect.
- **Write ASCII-only on first attempt** - some environments install a
  `normalize-dashes.sh` post_tool_call hook that rewrites em/en dashes to
  ASCII hyphens, but relying on it causes mtime-change warnings and dirty
  diffs. If you see "file was modified since you last read it" after an
  edit, re-read to confirm the hook didn't corrupt intent.

## Verification (post-cleanup)

- `grep -rn '//! TODO:' crates/ --include='*.rs'` - expect zero hits (plus
  `vendor/headroom` if in scope)
- `scripts/scan-module-todos.py crates/` - no module-level TODO in the first
  N lines (default 25)
- `grep -rn $'[\u2013\u2014]' crates/ --include='*.rs'` - zero typographic
  dashes if the style guide forbids them
- Spot-check modified files with `read_file` for structural integrity (pub
  items, fn signatures, mod declarations)
- Don't rebuild mid-pass - run the rebuild once all comment phases are done

Reference with before/after diff patterns:
`references/module-todo-cleanup.md`.
