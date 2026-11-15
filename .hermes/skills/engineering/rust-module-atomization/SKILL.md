---
name: rust-module-atomization
description: "Use when atomizing a Rust file into a per-item module tree in the Aphrodite crates, keeping the suite green at every phase."
version: 1.1.0
author: Hermes Agent
license: MIT
platforms: [macos, linux]
category: engineering
category_taxonomy: engineering/rust-module-atomization
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, atomizing, extracting, migrating, delegating, verifying]
        related_skills:
            - test-driven-development
            - parallel-delegation-execution
            - surgical-file-editing
status: active
---

# Rust Module Atomization

Split a monolithic `.rs` file into a reverse-taxonomy module tree (leaf = most
specific: one detector/arm/predicate per file) WITHOUT losing behavior. The
suite being green at every phase is the safety property; never skip a phase
to save a build.

## Procedure (phase-gated; run the full suite after each phase)

0. **Baseline move.** `mkdir -p <mod-dir>` THEN `git mv file.rs file/mod.rs`
   (`git mv` does not create the target dir and fails "No such file or
   directory" if it is missing). Run the suite; the existing tests are the
   frozen contract.
1. **Pure extraction (zero logic edits).** Move helpers VERBATIM into
   per-domain submodules (`line/`, `text/`, `builders/`, `state.rs`, ...).
   The facade `mod.rs` declares submodules and `pub use`s the exact public
   names the rest of the crate (and sibling crates via re-export) depend on
    - grep every call site for those names first. Visibility → `pub(crate)`
      where cross-module. Suite must pass identically.
2. **Typed shared input.** Add one pre-computed view struct built ONCE per
   blob (`raw`/`trimmed`/lines/`total`/`bytes`/`first`/`last` + `count`/
   `any`/`majority` helpers) so detectors and builders stop re-scanning
   `content.lines()`. If the constructor returns `None` on empty input, add
   an `empty()` fallback - builder paths that must never early-return (e.g.
   preview building for `""`) need it.
3. **Declarative pipeline.** Replace the imperative if-chain with an
   `Option::or_else` chain over per-shape detector fns; chain position IS
   priority, so order must stay byte-identical to the old arms. One file per
   shape. Stub unimplemented shapes with a `fn detect(_) -> bool { false }`
   so the chain is complete and the ordering contract is testable.
4. **Split builders/dispatch.** One file per output arm, each taking the
   shared input; a thin dispatcher matches the type string → arm; the
   universal transform (cap/truncate) applies last on every path.
5. **Regex elimination.** Kill each static `LazyLock<Regex>` with a
   structural scan (token walk before a delimiter, `splitn` field checks,
   majority votes). Keep the regex as a `#[cfg(test)]` oracle during the
   swap, cross-check against fixtures, then delete both.
6. **Collapse parallel implementations.** When a second code path (proxy,
   bridge) duplicates the classifier/builder, delete the duplicate and route
   call sites through the single pipeline. Before deleting, `grep -rn` the
   symbol across the WHOLE tree including tests - pin tests recompile the
   old shape and fail on a shard you did not run. Update pin tests to the
   converged formats deliberately (LLM-visible output converges; call it out
   in the CHANGELOG).
7. **Cleanup.** fmt, clippy `-D warnings`, docs, changelog entry.
8. **Verify completeness against the plan.** If a REFACTOR-PLAN / design note
   specifies the target tree, diff its file list against the actual tree per
   directory (one file per shape/arm/predicate + the facade files) - a 1:1
   match confirms the atomization is complete; a plan file that claims
   "preview.rs is one file" while the module holds 66 `.rs` files is the
   drift tell.
9. **Prove no test was lost in the move.** Count `#[test]` fns before the
   baseline (`grep -c "#\[test\]" src/<file>.rs`) and after across the new
   module dir (`grep -c "#\[test\]" src/<mod>/*.rs | awk -F: '{s+=$2} END
{print s}'`) - the totals must match. A green suite is the contract, but
   the count is the fast explicit proof that a file that went missing was
   not silently dropping its tests. Pair with the full post-wave battery
   run by the parent: `cargo test -p <pkg>`, `cargo test -p <pkg>-hermes`,
   clippy `--workspace --lib -- -D warnings`, the FFI contract checker,
   the drift-guard diff (plugin `__init__.py` vs template), bindings regen
   test, and ruff on the plugin - then a LIVE smoke through the real
   `_load_dylib()`/`_call_json()` path (compress→retrieve round trip +
   any per-session feature matrix) so the installed dylib, not just the
   crate, is verified.

## Multi-file orchestration (parallel agents)

**Gate a card to the parent whose OUTPUT it consumes, not to the card that happens
to precede it in your head.** A sibling gate means "these two may run together":
when the anyhow card and the `if`/`match` card were both children of the
`pub use` removal card, completing that card spawned all three at once - one
rewriting return types crate-wide while the other rewrote bodies crate-wide, on
the same files. Two rules that prevent it: cards that change a *signature* must
be parents of cards that change a *body*, and the only cards that may be
siblings are ones owning disjoint trees (one per crate, one per module group).
A mis-gate is recoverable but expensive: `complete` refuses a `scheduled` or
`running` card, `reclaim` releases the claim without stopping the process, and
`--force` closes the run record while the worker keeps draining - so the
orphans keep writing for a while and can revert a file the live worker just
finished.

To atomize a whole crate, dispatch one agent per file with DISJOINT file
ownership. The migration is parallel-safe because `pub mod X;` in lib.rs
needs NO change when `X.rs` becomes `X/mod.rs` - a directory module
auto-resolves `mod.rs` - so no agent ever writes a shared file; each owns
only its module dir. Procedure:

1. **Pin the facade contract per agent.** Grep the whole tree (sibling
   crates included - a bridge crate imports `crate::state::{...}`,
   `crate::resolve::{...}` etc. by module path) for each file's public
   names, and paste each file's exact pin list + its specifics (feature
   gates, include_str! files, process statics) into that agent's context.
   Every agent gets the same procedure block: phases, pitfalls,
   verification commands, the peer list, and the standing "leave changes
   uncommitted for the sweeper" rule.
2. **Size the wave to the provider's rate budget, not the runtime cap.**
   Parallel agents share one model endpoint; a per-minute inference quota
   that comfortably serves 2 agents will mass-429 a wave of 7 (all die at
   ~40s, work untouched). Default to 2-wide waves; widen only when the
   endpoint's budget is known. On 429/401 deaths re-dispatch immediately -
   the failure is a short window (retry succeeds reliably), not a config
   problem. A dead agent's work is either absent (instant death → clean
   re-dispatch) or fully landed (late death → verify the tree, do not redo).
3. **Peer triage protocol** (each agent, one shared tree): compiler/test
   errors naming only your module → fix; naming peer modules → ignore
   (mid-flight); iterate with `cargo check -p <pkg> --all-targets`; run the
   full suite once at the END and report peer-attributed failures honestly.
   The parent runs the authoritative full suite + clippy after the wave.
   Pre-existing lints live outside your module (examples/, lib.rs, proxy.rs)
    - verify none of the clippy findings are inside your module; do not fix
      other files.
4. **Agents die, work survives.** On a rate-limited provider an agent that
   finished its verification can still 429 on its final reporting turn - the
   report is lost, the work is not. Verify each module from the tree + live
   transcript (child-owned background cargo notifications re-enter the
   parent as verification evidence) instead of re-dispatching.
5. **Auto-committer interplay.** The sweeper may commit the `git mv` mid-
   work: agents check `git status` first, skip phase 0 when the file is
   already at `<file>/mod.rs`, retry on an index lock after a few seconds,
   never commit themselves. The pristine original stays recoverable as
   `git show HEAD:src/<file>.rs` even after the rename is committed - give
   every agent this recovery path for byte-exact slices of tests/helpers
   (user tip: "if your agents have trouble, they can take a look at the
   files removed").
6. **Docs sweep stays with the parent.** Concurrent agents must NOT update
   classification passes / CHANGELOG / docs (same files, guaranteed
   conflicts); the parent does the reference sweep (grep for the old
   monolithic paths) after the wave.

## Parallel waves (delegation)

- One agent per file, disjoint ownership (each owns ONLY its `src/<file>` and
  `<file>/` dir; never lib.rs/main.rs or a peer's module). Facade re-exports
  keep every `crate::X::name` call site compiling, so concurrent agents on
  different files never collide at the module-path level.
- A shared single-provider endpoint (e.g. Cloudflare Workers AI) trips its
  per-minute inference limit under 7 concurrent agents: HTTP 429s and
  intermittent HTTP 401 code-10000 auth blips kill agents at random points
  (often the final report turn). Dispatch waves of 2; a spawn that 401s at
  ~15s succeeds on immediate re-dispatch (the blip window is short).
- Instruct agents to keep the tree COMPILING at every checkpoint (cargo check
  after each extraction step) so a mid-flight provider kill leaves a green
  tree, not a half-moved module.
- Verify each agent's work from the TREE, not the report: agents that die on
  their last turn lose only the summary - `git show HEAD:src/<file>.rs`
  recovers the pristine original for byte-exact test slicing even after the
  rename was committed. Run the full suite yourself after the wave.
- A `patch` result can report success while the row is unchanged when the
  old_string had escape drift (`\|` vs `|` in table cells): re-grep the line
  after patching before declaring it applied.

## Kind-named leaves (the naming pass after atomization)

Atomization gives one item per file; the naming pass then renames each item to
its kind (`Fn`/`Struct`/`Enum`/`Const`) so the directory path carries the domain
and the item carries the kind (`Preview::Builder::Code::Fn`). Five rules, each of
which has broken a build:

1. **A single-item leaf stays a FILE; a multi-item leaf becomes a DIRECTORY.**
   `Code.rs` holding one fn becomes `Code.rs` holding `pub fn Fn` - do not turn
   it into `Code/Fn.rs` + `Code/mod.rs`. A file holding several items cannot give
   them all the name `Fn`, so it becomes a directory of one-item leaves whose
   names are each item's _domain_ (`Line/Grep/{Hit,Search}.rs`, each holding
   `pub fn Fn`).
2. **The alias lives in the parent facade, not in the leaf - unless a caller
   names the leaf path.** `Preview::build_preview` needs
   `pub use Entry::Fn as build_preview;` in `Builder/mod.rs`, but
   `crate::Preview::Language::detect_language` names `Language` explicitly, so
   that alias must sit inside `Language/mod.rs`. Grep the call-site FORM
   (`grep -rhoE '(crate::)?Module::[A-Za-z_]+'`) before choosing.
3. **Never alias a name nothing uses.** `pub(crate) use X::Fn as old_name;` with
   no caller is an `unused_imports` warning, and `cargo clippy -- -D warnings`
   is usually a required gate. Alias only names with real out-of-scope callers;
   report the ones you deliberately did not add.
4. **`Domain::Struct` resolves to the MODULE, not the type.** After
   `Input/Struct.rs` + `Input/mod.rs`, `use crate::Preview::Input::Struct;`
   imports the module and `Struct<'_>` fails E0573 "expected type, found
   module". Import `crate::Preview::Input::Struct::Struct`, and expect the
   house-style stutter (`Type::Fn::Fn(...)`) wherever a facade module and its
   single item share the kind name.
5. **A doc line starting with `- ` becomes a clippy list item.** Prose wrapped so
   the next line begins `- a false positive ...` trips
   `clippy::doc_lazy_continuation` ("doc list item without indentation") under
   `-D warnings`. Reword the sentence so no continuation line starts with a
   bullet; do not indent it.

Splitting the test battery alongside (`Test/<Area>.rs`) is proven by a sorted
name-set diff, not by the suite: `grep -oE '^\s*fn (test_|prop_|regression_)\w+'
<old file> | sort` against the same over `Test/` - the lists must be identical and
the counts equal.

## Phase 3: the alias sweep (deleting the compatibility re-exports)

Atomization leaves `pub use Leaf::Fn as old_name;` in each facade. The sweep
deletes every one and repoints the call sites. Seven rules, each of which
produced a broken build or a silently wrong rewrite:

1. **The compiler enumerates the call sites, not a text search.** A rename
   regex cannot see a glob import (`use super::*;`), a path through the other
   crate (`aphrodite::Debug::add_timing` from the binary), or a `cfg(test)`-only
   use. Delete the alias lines first, then drive the repair from
   `cargo check --all-targets` diagnostics; a green lib build alone hides the
   test-target breakage.
2. **A facade re-exporting a child facade's alias is itself a call site.** A
   uniform `use Builder::build_preview;` inside `Preview/mod.rs` resolves
   relative to that module, not the crate root - so the resolver must fall
   back to "module-relative" for a bare head, and to the package name for
   `aphrodite::`.
3. **Every module on a leaf path must be visible.** A private `mod Log;` behind
   a deleted alias turns `crate::Debug::Log::Bridge::Fn` into E0603. Flip the
   ones the paths cross to `pub(crate) mod`, and to `pub mod` when the binary
   target names them (a bin is a separate crate; `pub(crate)` is invisible).
4. **Rebuild a use statement, do not patch an item inside it.** In
   `use crate::Preview::{Input::Input, Line::Error}` the replacement's base is
   the group's prefix, not the crate root; emitting one absolute
   `use crate::...;` line per leaf is the only unambiguous rewrite.
5. **Preserve the original path head.** `crate::` inside `Source/Boot/**` is the
   BINARY's root, so `crate::Configuration` there names a module that does not
   exist. Rewrite `aphrodite::Proxy::AppState` to `aphrodite::Proxy::State::Struct::Struct`,
   never to `crate::...`.
6. **Drop only the globs the explicit imports made redundant.** `clippy
-D warnings` fails on the now-unused `use crate::Proxy::*;`, but rustc also
   calls a `cfg(test)`-only import unused in the lib build - deleting that one
   breaks the test target. Restrict the deletion to leaves whose name is `*`.
7. **A deleted alias takes its doc block with it.** A `///` line or a
   `#[cfg(test)]` attribute left above a removed `use` parses as "expected item
   after attributes" and takes the whole module out of the tree.

Worked mechanics (the parse, the rewrite, the three failure signatures) live in
`references/alias-sweep.md`.

## No-build sweeps (structural self-check)

A sweep card that forbids `cargo check`/`clippy`/`fmt` (the build belongs to a
later verification card) still has to be verifiably structural. These checks
catch the error classes a build catches first, without compiling:

- Every `mod X;` in a facade resolves to `X.rs` or `X/`, every non-`mod.rs`
  `.rs` file is declared by its parent facade, and no directory lacks a
  `mod.rs`. A missing declaration drops a whole module silently - it is not a
  warning. Probe: a script that walks the area, regexes
  `^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;` per facade and `stat`s the
  target.
- No duplicate `mod X;` in one facade (same script, count per name).
- Zero remaining references to every item you renamed or deleted:
  `grep -rn '<OldPath>' --include=*.rs .`. A consumer in a directory the card
  does not own still breaks the build, so its one-line update is part of the
  change - not scope creep. Record it in the handoff.
- Brace/paren/bracket balance per file, with a tokenizer that skips strings and
  comments: `write_file` reports `lint: skipped (untrusted workspace)` for `.rs`
  outside a trusted rustfmt workspace, so it proves nothing about syntax.
  `rustc -Zparse-only` is NOT an option - removed as of nightly 1.102
  (`error: unknown unstable option: 'parse-only'`).

What this cannot prove - types, trait bounds, borrows, `dead_code` - stays for
the verification card. Keep new items' accessors and variants each reachable
from a real call site in the same change, or `-D warnings` fails on dead code
before anything else does.

## Pitfalls

- **A rename pass that rewrites module identifiers also hits four things that
  are not module paths.** Each one has produced a broken build: (a) a
  module whose item shares its name - `pub use expand::expand;` becomes
  `pub use Expand::Expand;` and the item disappears; (b) method turbofish -
  `s.parse::<u64>()` becomes `s.Parse::<u64>()`, so the path rule needs a
  `(?<!\.)` guard; (c) `std` module names that collide with a module name -
  `use std::env;` + `env::var` and `use std::{path::PathBuf}` become
  `Environment::var` / `Path::PathBuf`, so the rule needs
  `(?<!std::)(?<!core::)(?<!alloc::)` **and** a line-scoped repair for
  multi-line `use std::{` groups (a guard only sees the token, not the
  enclosing brace group); (d) an inline `mod tests {` block - the
  declaration has no semicolon so the `mod name;` rule misses it while every
  reference to it is rewritten. Verify a rename pass by compiling, not by
  reading the diff.
- **Grep the tree for the pre-move paths before declaring the layout done.**
  Packaging guards, workflow `paths:` filters, a `--librs` default and an
  integration test that normalises `include_str!` targets all hard-code the
  old layout and fail silently later: `.github/workflows/Publish.yml`,
  `.github/workflows/ffi-check.yml`, `Maintain/check_ffi_contract.py`,
  `crates/*/tests/packaging.rs`. `include_str!` paths gain one `../` per
  directory the embedding file moves down.
- **A moved test module can stop compiling while the suite stays green.** After
  a split, a `Test/` directory that no facade declares is never compiled: the
  file exists, the test names exist in a text search, and `cargo test` passes
  with two fewer tests. Audit every `Test/` directory against its parent
  `mod.rs` (`find crates/*/Source -type d -name Test` then check the parent
  declares `mod Test;`), and compare the per-binary `test result:` counts
  before and after the wave, not just pass/fail.
- **A test that captures a `tracing` event through `with_default` is flaky, not
  wrong.** A callsite's interest is cached process-wide, so whichever test
  reaches that callsite first under the no-op subscriber can cache it as
  disabled and the capture never fires - measured at roughly one failing run in
  six on an otherwise correct test. Split the message from its emission
  (`parse_failure_message(path, action) -> String`, then a `tracing::warn!`
  that emits it) and assert the message builder; the emission is one line that
  cannot drift from it.
- **`tracing::dispatcher::has_been_set()` reports only the GLOBAL dispatcher.**
  A function that branches on it to choose `tracing::warn!` vs `eprintln!`
  mis-routes for a thread-local `with_default` subscriber (`with_default` never
  sets the global flag). Emit the event unconditionally and gate only the
  stderr fallback on `!has_been_set()`.
- `git mv` needs the target directory to exist - `mkdir -p` first, or the
  rename fails "No such file or directory".
- `include_str!`/`include_bytes!`/`include!` paths resolve relative to the
  CURRENT source file's directory - after `git mv file.rs file/mod.rs`
  every embedded path gains one `../` level
  (`include_str!("builtin_directives/x.md")` →
  `"../builtin_directives/x.md"`). Grep for them before the move; the
  tests that assert embedded content prove the fix.
- A file named after a Rust keyword (`type.rs`) needs `r#type` escaping in
  EVERY reference: `pub mod r#type;` in the facade and
  `crate::mod::r#type::fn` at call sites. Never leave both the escaped and
  unescaped `pub mod` lines in one facade - E0428 "redefined here".
- `Cow<'static, str>` cannot return a borrowed `&'a str` - tie the lifetime:
  `fn resolve<'a>(hint: &'a str, ...) -> Cow<'a, str>`.
- Compute derived fields BEFORE the struct literal when the literal moves a
  Vec: `let n = v.len();` then `Self { v, n }` - reading `v.len()` after the
  move is E0382.
- Never trust `EXIT:$?` after a pipe: `cargo test | tail` reports tail's
  status, not cargo's. Run the command without the pipe (full output is
  auto-saved) and read the real exit code / test summary line.
- Grep for a symbol's call sites (tests included) before changing its
  signature or deleting it - the compile error may live in a file you never
  opened.
- Keep the public facade's exported names byte-identical across the whole
  migration (lib.rs re-exports pin them); a facade rename mid-migration
  breaks every downstream crate at once.
- After atomizing, grep the WHOLE tree - docs, classification passes,
  architecture notes, not just source - for references to the old monolithic
  path (`src/preview.rs`). Such a doc goes silently stale the moment
  `file.rs` becomes `file/mod.rs`: the classification row must be re-pointed
  at the module facade (`src/preview/`) with the sub-tree breakdown, and a
  live architecture note must name `preview/mod.rs`. Historical session logs
  and issue forensics are the exception - they record what was true when
  written and stay untouched.
- Docs updated during cleanup that are prettier-managed: run
  `prettier --write` AFTER the content patch, not before - it reflows wide
  tables, so a one-row semantic change shows dozens of cosmetic diff lines;
  then verify `prettier --check` passes before declaring done.
- A green `cargo build` does NOT verify phase 7's clippy gate - CI runs
  `clippy --lib -D warnings` and catches lints a plain build never flags
  (e.g. `collapsible_if`: nested `if let` that should be a let-chain,
  fixed exactly as the lint's help suggests). Run the gate locally before
  push: `cargo clippy --workspace --lib -- -D warnings`, and read the exit
  code - a lint that fails CI is a failed migration.
