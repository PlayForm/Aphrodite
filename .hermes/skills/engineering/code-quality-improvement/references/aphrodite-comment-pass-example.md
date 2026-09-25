# Code Quality Pass - Aphrodite Example

## Scope

Systematic comment-quality renovation across `crates/aphrodite-hermes/src/`
(1088 Rust files).

## Issues found and fixed

| Category                         | Count    | Action taken                                                                                   |
| -------------------------------- | -------- | ---------------------------------------------------------------------------------------------- |
| Module-level `//! TODO:` markers | 44       | Converted to `## Status`, `## Planned Work`, `## Stub`, or `## No-op shim` sections            |
| Typographic dashes in comments   | 19 files | Replaced em/en dashes (-/-) with ASCII hyphens (-)                                             |
| Large inline comment blocks      | 7 files  | Verified legitimate; kept (technical deep-dives, migration roadmaps, race-condition rationale) |

## Files transformed

64 files modified across 10 categories (proxy stubs, CCR store, retrieval,
search, directives, context engine, telemetry, update, commands,
environment). The scan script enumerates candidates rather than a static
list.

## Pattern distilled

For each `//! TODO: zero callers as of YYYY-MM-DD`:

```rust
//! ## Status
//!
//! Zero callers as of 2026-05-02. [Explain what needs to happen and from where.]
```

For TODO lists at module level:

```rust
//! ## Planned Work
//!
//! - Feature item one
//! - Feature item two
```

For stub command handlers:

```rust
//! ## Stub
//!
//! Real implementation through `ProviderTrait::Method` should handle [missing behavior].
```

## Verification performed

- `grep -rn '//! TODO:' crates/` → 0 hits
- `grep -rn $'[\u2013\u2014]' crates/` → 0 hits after dash replacement
- Manual spot-check of 20+ files confirmed structural integrity
- `cargo check -p aphrodite-hermes` passed (pre-existing vendor warnings
  only)

## Tools used

- `patch` for targeted file edits
- `execute_code` + Python for bulk dash replacement (19 files)
- Custom scan script:
  `.hermes/skills/engineering/rust-comment-quality/scripts/scan-module-todos.py`
- **Passive:** a `post_tool_call` normalize-dashes.sh hook may run in some
  environments on every `write_file`/`patch`, automatically replacing any
  residual typographic Unicode dashes. The hook caught ~5-10 incidental
  dash characters across the 43 edited files that were missed by the
  primary pass. The mtime-change warning on the next `read_file` call is
  the observable signature of the hook firing.

## Takeaway for future passes

1. Always survey first: classify TODO locations (module vs inline) and
   character issues
2. Batch-replace typographic dashes with a simple global find-replace after
   confirming no legitimate use
3. Distinguish module-level docs (can be 20+ lines) from inline bloat
   (rarely justified)
4. Keep inline `// TODO:` markers - they are implementation-site next-steps
5. No rebuild until all comment/doc phases are done; verification via grep +
   spot-read
6. Docs under `.hermes/**/*.md` must stay `npx prettier --check` clean
