---
name: simplify-code
description: Use when simplifying recent Aphrodite code. Parallel agent review.
version: 1.2.0
author: Hermes Agent # inspired by Claude Code /simplify
license: MIT
platforms: [linux, macos, windows]
category: engineering
category_taxonomy: engineering/simplify-code
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, simplifying, reviewing, refactoring, aggregating, delegating]
        related_skills: [test-driven-development]
status: active
---

# Simplify Code

Review recent code changes with four focused reviewers running in parallel -
reuse, quality, efficiency, altitude - aggregate their findings, and apply
the fixes worth applying. This is a cleanup pass, not a bug hunt: it improves
code that already works. Correctness review is a different pass with
different verification standards (`aphrodite-testing-discipline`).

**Core principle:** four narrow reviewers beat one broad reviewer. Each
searches the codebase for a single class of problem without diluting its
attention; they run concurrently, so you pay the latency of one review, not
four.

## When to Use

- User says "simplify", "simplify my changes", "simplify these changes",
  "review my code", "review my recent changes", "clean up my changes", or
  "/simplify"
- Honor modifiers: **focus** (`reuse`, `quality`/`simplification`,
  `efficiency`, `altitude` → run only that reviewer or weight aggregation
  toward it); **dry run** ("just report" → present findings, apply nothing,
  ask before applying); **scope** ("last commit", "staged",
  `crates/aphrodite-hermes/src/` → narrow the diff source)

Do NOT auto-run after every edit or tack it onto unrelated tasks - it costs
four subagents' worth of tokens. Invoke only when the user explicitly asks.

## Prerequisites

- A git repo (or session-tracked files) with changes to review.
- `delegate_task` available with `delegation.max_concurrent_children` ≥ 4. If
  delegation is unavailable (leaf subagent, disabled, budget exhausted), do
  NOT skip angles - work all four reviewer angles yourself, sequentially,
  with the same search standards and finding format, and say in your summary
  that it was a single-pass inline review, not the parallel fan-out.
- Free-tier provider quotas can kill reviewer subagents mid-run (HTTP 401/429
  at the first or final model call). Re-dispatch a dead reviewer SOLO with
  the same brief rather than re-fanning out at the original width - small
  retries land reliably, wide re-fans do not.

## How to Run

Capture the diff with `terminal` (Phase 1), then launch the reviewers via
`delegate_task` in **batch mode** - all four tasks in one `tasks` array so
they run concurrently.

## Quick Reference

| Reviewer      | Searches for                                                       | Example flags                                                   |
| ------------- | ------------------------------------------------------------------ | --------------------------------------------------------------- |
| 1. Reuse      | Duplication of existing functionality                              | Hand-rolled logic an existing utility covers                    |
| 2. Quality    | Redundant state, parameter sprawl, copy-paste-with-variation, slop | `as any` casts, comments restating obvious code                 |
| 3. Efficiency | Unnecessary work, missed concurrency, silent failures              | N+1, repeated file reads, empty catch blocks                    |
| 4. Altitude   | Fixes at the wrong depth - band-aids on shared infrastructure      | Special case for one caller, symptom patch, stacked workarounds |

Risk tiers: **SAFE** = proven not to affect behavior (auto-apply) ·
**CAREFUL** = improves without changing semantics (apply with test
verification) · **RISKY** = may change behavior or break public contracts
(flag for human review, never auto-apply).

## Procedure

### Phase 1 - Identify the changes

```bash
git diff                                              # 1. default: uncommitted working-tree changes
git diff HEAD                                         # 2. if empty, include staged
git diff --staged                                     # "staged changes"
git diff HEAD~1                                       # "the last commit"
git diff main...HEAD                                  # "this branch" / "my PR"
git diff -- crates/aphrodite-hermes/src/engine/mod.rs # specific file(s)
```

Empty everywhere → fall back to files the user named or edited this session;
if there's genuinely nothing, say so and stop. Large diff (say >2000 changed
lines)? Warn that four subagents carrying the full diff will be token-heavy;
offer to scope it down (per-directory, per-commit) before proceeding.

### Phase 2 - Launch four reviewers in parallel

Give EVERY reviewer the COMPLETE diff (cross-file issues hide in the gaps)
plus the absolute repo path, with `terminal`/`file`/`search` toolsets.
Instruct each reviewer to: search the existing codebase for evidence, don't
reason from the diff alone; apply Chesterton's Fence - `git blame` the line
before flagging anything for removal, and mark unknown purpose
`confidence: low`; report `file:line → problem → cost → suggested fix |
confidence: high/medium/low | risk: SAFE/CAREFUL/RISKY` - the **cost** field
forces each finding to justify itself (a finding that can't state what it
costs is probably a nit); skip nits and style-only churn.

**Reviewer 1 - Code Reuse.** Flag new code that duplicates functionality
already in the codebase: new functions duplicating existing ones; hand-rolled
logic an existing utility already does (manual string/path manipulation,
custom env checks, ad-hoc type guards, re-implemented parsing). Search
utility modules and shared helpers with `search_files`; name the existing
thing and where it lives.

**Reviewer 2 - Code Quality.** Flag redundant state (values derivable from
existing state, unnecessary caches); parameter sprawl; copy-paste-with-
variation; leaky abstractions; stringly-typed code (check canonical
registries before flagging); deeply nested conditionals (ternary chains,
3+-level if/else pyramids - flatten with guard clauses, early returns, or a
lookup table); AI slop (comments restating obvious code, defensive
null-checks on already-validated inputs, `as any` casts bypassing the type
system, patterns inconsistent with the file). Give the concrete refactor.

**Reviewer 3 - Efficiency.** Flag unnecessary work (redundant computation,
repeated file reads, duplicate API calls, N+1 access patterns); missed
concurrency (independent ops run sequentially); hot-path bloat (heavy or
blocking work on startup or per-request paths); TOCTOU anti-patterns
(existence pre-checks instead of doing the op and handling the error); memory
issues (unbounded growth, missing cleanup, listener/handle leaks, closures
capturing the whole enclosing scope - prefer a small class that copies only
what it needs); overly broad reads; silent failures (empty catch blocks,
ignored error returns, `except: pass`, `.catch(() => {})`, error propagation
gaps - at minimum log before swallowing).

**Reviewer 4 - Altitude.** Flag changes implemented at the wrong depth: a
special case added to a generic code path for one caller (an
`if (caller == X)` branch, a type check, a magic-value escape hatch); a
symptom patched at the call site while sibling call sites keep the same
flaw; a workaround stacked on an earlier workaround; config or flags routing
around a broken default instead of fixing it. Identify the underlying
mechanism and describe the deeper fix - generalize the shared path, fix the
root default, fix the whole bug class - and note honestly when the deeper
fix is large enough to be its own task. `git blame` and read surrounding
comments first: compat shims, staged migrations, and vendored-code isolation
are deliberate boundaries, not band-aids - don't flag those.

### Phase 3 - Aggregate and apply

1. **Merge** findings into one list, deduping where reviewers overlap (same
   line or same underlying mechanism → collapse).
2. **Discard false positives** silently - you have the most context; don't
   argue with a reviewer, just drop weak or wrong suggestions.
3. **Resolve conflicts.** Reviewers can disagree ("use existing util X" vs
   "X is slow, inline it"). Default order: **correctness > the user's stated
   focus > readability/reuse > micro-perf.** Don't apply a perf "fix" that
   hurts clarity unless the path is genuinely hot. Mutually exclusive and
   both defensible? Pick the one touching less code and note the
   alternative.
4. **Apply in risk-tier order:** SAFE first (unused imports, commented-out
   code, pass-through wrappers - run tests after); CAREFUL next, one file at
   a time (rename locals, flatten ternaries, extract helpers - tests after
   each file, revert any that break); RISKY last (N+1 restructuring, public
   API changes, concurrency fixes, error-handling changes - present each
   with risk and test coverage; altitude findings usually land here). Dry
   run? Present all three tiers, apply nothing.
5. **Verify:** targeted tests for the touched files (not the full suite)
   plus any linter/type check the repo uses. Aphrodite gates: `cargo test -p
<crate>`, `cargo fmt --check`, `cargo clippy --workspace --lib -- -D
warnings`, `ruff check plugins/aphrodite/`, `npx prettier --check
.hermes/**/*.md`. A fix that breaks a test gets reverted and reported.
6. **Summarize:** applied fixes grouped by reviewer category and risk tier,
   findings deliberately skipped and why, and whether the review ran inline.

## Pitfalls

- **Don't fan out wider than four** - more reviewers means more cost and
  more conflicting suggestions to reconcile, not better coverage.
- **Give the WHOLE diff to each reviewer.** Splitting it hides cross-file
  duplication and N+1s.
- **Reviewers search, they don't guess.** A reuse finding without a pointer
  to the existing utility is noise - require `file:line` evidence, drop
  findings that lack it.
- **Apply ≠ rewrite.** Keep edits scoped to what the diff touched plus the
  minimal surrounding change a fix requires. When the right fix is deeper
  than the diff (altitude), FLAG it - don't unilaterally rebuild shared
  infrastructure inside a cleanup pass.
- **Don't drift into bug-hunting.** A genuine correctness bug gets reported
  prominently as a separate "found a bug" note, not folded into cleanup
  fixes.
- **Respect project conventions.** Fold .hermes/AGENTS.md and the linter
  config (rustfmt.toml, clippy, ruff, .prettierrc) into the reviewer
  prompts so suggestions match house style.
- **Scope down huge diffs before delegating** - four subagents each carrying
  a 5000-line diff is expensive and may truncate.
- **Don't trust dead-code tools blindly.** `knip`, `ts-prune`, and
  `depcheck` flag exports that ARE used dynamically (string-based imports,
  reflection) - `search_files` the symbol before removing; a clean tool
  report is not proof. In Aphrodite, `cargo clippy` unused-code findings are
  grounded, but string-driven dispatch in the plugin Python can look dead to
  `pyright` - search before removing.
- **Never auto-rename public contracts.** Export names, API route paths, DB
  column names, and config keys are contracts - tag changes as RISKY.
- **Don't remove "unnecessary" error handling.** An empty catch or ignored
  error may be intentional (expected and benign in context). Flag it; let
  the human decide.
- **Not every special case is a band-aid.** Compat shims, staged migrations,
  and isolation layers around vendored code are deliberate design - check
  `git blame` and comments first; unclear intent → `confidence: low`.

## Verification

- [ ] Targeted tests pass after each applied tier; reverted fixes reported
- [ ] Linter/type check clean
- [ ] Applied fixes scoped to the diff (no unrelated refactors)
- [ ] RISKY findings surfaced to the user, none auto-applied
- [ ] Summary lists applied fixes, skipped findings, and inline-vs-parallel
      mode

## Related

Parallel review during implementation is covered per task by `delegate_task`
fan-out; this skill is the standalone after-the-fact cleanup pass.
`aphrodite-testing-discipline` is the probe/verification gate - the bug
hunt, where this is the cleanup.
