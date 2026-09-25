---
name: code-quality-improvement
description: Use when improving code quality and docs across the Aphrodite monorepo (Rust crates, plugin Python, .hermes docs).
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos, linux, windows]
category: engineering
category_taxonomy: engineering/code-quality-improvement
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, documenting, surveying, converting, escaping]
        related_skills: [rust-comment-quality, markdown-readme-audit-fix]
status: active
---

# Code Quality Improvement

Systematically improve code quality and documentation across a large codebase (100+ files) without breaking existing functionality. This is not a rewrite - it is a disciplined pass through existing code: better comments, safer error handling, stale TODOs converted to structured docs, and alignment with project conventions, preserving all functionality.

## When to Use

- User asks to "improve code quality and comments" across a module or crate
  (target trees: crates/aphrodite, crates/aphrodite-hermes, vendor/headroom
  fork, plugins/aphrodite Python, .hermes docs)
- Documentation coverage must rise from minimal to comprehensive
- Codebase has scattered `unwrap()`, broad `#[allow(...)]`, stale TODOs, or undocumented modules
- Systematic cleanup after a period of rapid feature development
- Overhauling stale TODO markers across a full codebase - see `rust-comment-quality` for the systematic pattern
- Auditing `.md` files (READMEs, doc trees, Content docs) for broken links, formatting, structure accuracy - see `markdown-readme-audit-fix`

## Prerequisites

- Read the project's conventions files (AGENTS.md, naming conventions, style guides) BEFORE defining your own standards
- Survey the codebase first; quantify the problem before editing anything

## How to Run

- **Edit-only pass**: never rebuild or commit during the improvement pass - the user verifies and commits on their own timeline
- **One patch at a time (mandatory)**: never batch multiple `patch` calls in one tool-call block. Each patch targets exactly one file. Send, wait for the result, then proceed. Batching obscures which edit succeeded or failed
- **Never rebuild after every edit**: edit all files first, verify once at the end

## Procedure

### 1. Survey Phase

Scan the codebase and quantify the problem: file count and total lines, doc-comment coverage (% of files with `//!` or `///`), `unwrap()`/`expect()` counts, `#[allow(...)]` count, TODO/FIXME/HACK count, long functions (>80 lines between `fn` declarations), files with zero documentation.

Iterate all source files with Python `os.walk` or `pathlib`, count pattern occurrences, and compute a doc-coverage ratio (`doc_line_count / total_line_count`). Group files into quality tiers: excellent (>15% doc lines), good (8-15%), minimal (2-8%), none (<2%). Identify hotspots: files with the most combined issues - these are the priority targets.

### 2. Define Standards

- **Module-level `//!` comments**: `//! # <ModuleName>` heading, one-line purpose statement, key architectural notes, important cross-references
- **Public items `///`**: summary line + details for struct fields, enum variants, function parameters, return types
- **Cross-references**: use `[`TypeName`]` syntax, not raw paths
- **Comment style**: permanent technical documentation, as if authored by a human developer on the team. Never include AI/LLM/assistant/auto-generation meta-references
- **ASCII only**: plain ASCII characters in comments - no em dashes (U+2014), en dashes (U+2013), curly quotes (U+201C/D). Use hyphens, straight quotes, colons. Write it right the first time; do not rely on post-processing hooks
- **Error handling**: prefer `map_err` / `?` over `unwrap()` where type context allows; keep `unwrap()` in test code only
- **`#[allow(...)]`**: keep only where the warning is genuinely unavoidable; suppressors that mask real issues get fixed at the source

### 3. Execute Phase

Work **top-down**: crate root (`lib.rs`), then core modules (error types, environment, application state), then outward to peripheral modules. Core modules are referenced most, so improving them first yields the highest documentation ROI.

Typical improvements:

| Issue                                                                             | Improvement                                                                                                        |
| --------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Module doc has raw `//! TODO:` marker                                             | Convert to structured `## Status` / `## Planned Work` / `## Stub` / `## No-op shim` (see `rust-comment-quality`)   |
| No module doc                                                                     | Add `//!` block with heading, purpose, structure                                                                   |
| Enum variants / struct fields / fns undocumented                                  | Add `///` explaining meaning, role, params, return                                                                 |
| Typographic dashes/quotes in comments                                             | Replace with ASCII hyphens and straight quotes                                                                     |
| `unwrap()` on `Option`/`Result` in production code                                | Convert to `map_err` / `?` / proper error handling                                                                 |
| Broad `#[allow(unused_variables)]`                                                | Narrow with `#[cfg_attr(not(feature = "X"), allow(...))]` so suppression applies only when the feature is disabled |
| Bloated boilerplate module docs (`## RESPONSIBILITIES`, `## KEY COMPONENTS`, ...) | Replace with a 3-5 line `//!` summary that actually describes the module                                           |
| Stale TODOs that no longer apply                                                  | Remove or update with current status                                                                               |
| Long functions (>200 lines)                                                       | Document internal structure with section comments                                                                  |

Rules during execution:

- **Never delete functionality.** If a block is dead code but still compiled in, document its status; do not remove it
- **Follow project conventions** - PascalCase naming, tab indentation, import patterns, existing error types; adapt to the codebase, do not impose external style
- **Watch for pre-existing lints** - CRLF files edited with an LF-expecting linter show ambient errors; they existed before your edit
- **`patch` normalizes whitespace** - when identical content repeats at different indentation levels (common in JSX), `patch` cannot distinguish instances. Rewrite the whole file with `write_file` instead, then verify with `git diff`
- **TODO → structured sections**: status fact → `## Status`; missing implementation → `## Stub`; feature list → `## Planned Work`; single-line note → fold into existing docs. Never use the word "TODO" in final output

### 4. Shell Script Maintenance (backslash-escaping)

When maintaining shell scripts where user aliases could override stock POSIX commands, prefix native commands with `\`:

```bash
\pwd        # instead of pwd
\echo "..." # instead of echo "..."
\cd "$dir"  # instead of cd "$dir"
```

Escape builtins and POSIX utilities (`pwd echo find grep sed awk ls cat cd cp mv rm mkdir chmod sort uniq wc head tail cut tr xargs tee printf exit source read eval set trap test` etc.). Do NOT escape external tools (`git`, `gh`, `jq`, `node`, `npm`, `cargo`, `rustc`), keywords/control flow (`if`, `then`, `for`, `while`, `do`, `case`), or variable assignments. A word is a command position at: start of line, after pipe/`||`/`&&`/`;`, inside `$(...)` and backticks, after `while`/`until`/`if`.

Full list and position rules: `references/escape-command-checklist.md`.

## Pitfalls

- **Launch `cargo check` as a background process** - large codebases on external or slow drives routinely exceed the default 180s timeout. Use `background=true` and poll; a timeout is not a failure, the compiler is still working
- **Do not delete "dead code" modules** - zero callers can mean a future migration path; the maintainers chose to keep it. Document the status
- **Do not flag large docblocks as bloat** - a 20-line comment run before any `pub`/`fn` is usually a legitimate module rustdoc (`//!`/`///`); only dense inline runs inside function bodies are suspect
- **Do not over-comment trivial code** - `self.field = value` needs no doc; reserve docs for non-obvious behavior, error conditions, invariants, cross-component relationships
- **Never add AI-assistant meta-text to comments** - no "improved during a quality pass" or tooling references; comments must read as permanent codebase documentation
- **Do not change working code "for style"** - improve docs and unsafe patterns, but leave working logic and algorithms alone
- **Backtick only whole words** - `Wind` inside `Windows` is NOT a match; use word-boundary matching (`\bWind\b`). Verify with a `grep` pass after formatting
- **Survey before editing** - for bulk pattern removal, catalogue every occurrence into a mapping document first and let the user review the plan before execution
- **Never use `\t` in `patch` old/new strings** - the tool interprets it literally as backslash+t, not a tab; use real TAB characters
- **`replace_all=true` double-escapes** - replacing `cd` → `\cd` with replace_all turns existing `\cd` into `\\cd`; always use unique context strings
- **Emoji spacing corrupts in three ways** - missing em-quad before a heading emoji, double em-quad, and a Fitzpatrick modifier separated from its base emoji. Fix per instance with one `patch` per file; do not batch-script it (see `markdown-readme-audit-fix`)

## Verification

- [ ] Survey complete with quantitative metrics
- [ ] Standards defined and consistent with project conventions
- [ ] Core modules improved (crate root + Error + Environment)
- [ ] No functionality deleted - only improved
- [ ] No AI/meta terms in any new or modified comments
- [ ] `unwrap()` reduced where proper error handling was possible
- [ ] Stale TODOs cleaned up or updated
- [ ] Typographic dashes and curly quotes replaced with ASCII equivalents
- [ ] `cargo check -p <crate>` passes (background, if verification is requested); `cargo fmt --check`, `cargo clippy -p <crate> -- -D warnings`, `ruff check plugins/aphrodite/`, `npx prettier --check .hermes/**/*.md` as further gates
- [ ] No auto-commits - changes left for user review; if asked to commit, run `git status` first to confirm only intended files are staged
