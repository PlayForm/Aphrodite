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

## Pitfalls

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
