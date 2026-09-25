---
name: test-driven-development
description: Use when coding test-first in the Aphrodite monorepo. Enforce RED-GREEN-REFACTOR against the repo's real test commands and quality gates.
version: 1.3.0
author: Hermes Agent # adapted from obra/superpowers (Jesse Vincent)
license: MIT
platforms: [linux, macos, windows]
category: engineering
category_taxonomy: engineering/test-driven-development
date: 2026-09-25
metadata:
    hermes:
        tags: [engineering, testing, refactoring, verifying, red-green-refactor]
        related_skills: [plan]
status: active
---

# Test-Driven Development

Write the test first, watch it fail, write the minimal code to pass, then
refactor. If you didn't watch a test fail, you don't know it tests the right
thing - so the rule is absolute: no production code without a failing test
first, and no exceptions.

## When to Use

**Always** for new features, bug fixes, refactoring, and behavior changes in
the Aphrodite crates (`crates/aphrodite`, `crates/aphrodite-hermes`, the
`vendor/headroom` fork) or the plugin Python (`plugins/aphrodite/`).
**Exceptions - ask the user first:** throwaway prototypes, generated code,
configuration files. Thinking "skip TDD just this once" is rationalization.

## Prerequisites

- Know the repo's test commands (canonical list in `.hermes/AGENTS.md`):
  `cargo test -p aphrodite`, `cargo test -p aphrodite-hermes`; a single test:
  `cargo test -p <crate> <test_name>`. Plugin Python is verified with
  `ruff check plugins/aphrodite/` and `npx pyright plugins/aphrodite/`, not
  with cargo.
- A test framework that fails loudly on missing symbols (Rust's `#[test]`
  harness does).

## How to Run

Run tests with the `terminal` tool at every step: the specific test in RED,
the specific test in GREEN, then the full suite for regressions.

## The Iron Law

```
NO PRODUCTION CODE WITHOUT A FAILING TEST FIRST
```

Wrote code before the test? Delete it - don't keep it "as reference", don't
"adapt" it while writing tests, don't look at it. Delete means delete.
Implement fresh from tests.

## Quick Reference

| Phase    | Action                                                                  | Verify                                                         |
| -------- | ----------------------------------------------------------------------- | -------------------------------------------------------------- |
| RED      | Write ONE minimal test for one behavior                                 | Test fails for the expected reason (feature missing, not typo) |
| GREEN    | Write the simplest code to pass - cheating is OK (hardcode, copy-paste) | Test passes; full suite still green                            |
| REFACTOR | Remove duplication, improve names, extract helpers                      | Tests stay green throughout                                    |

**Final rule:** production code → a test exists and failed first. Otherwise
it's not TDD.

## Procedure

### RED - write the failing test

One behavior per test; a descriptive name ("and" in the name? split it);
test real code, not mocks (unless truly unavoidable); name the behavior, not
the implementation.

```rust
#[test]
fn retries_failed_operations_3_times() {
    let mut attempts = 0;
    let result = retry_operation(|| {
        attempts += 1;
        if attempts < 3 {
            Err("fail")
        } else {
            Ok("success")
        }
    });
    assert_eq!(result, Ok("success"));
    assert_eq!(attempts, 3);
}
```

**Verify RED - mandatory, never skip.** Run the specific test with `terminal`
(`cargo test -p <crate> retries_failed_operations_3_times`). Confirm it fails
because the feature is missing, not because of typos. Test passes
immediately? You're testing existing behavior - fix the test. Test errors?
Fix the error, re-run until it fails correctly.

### GREEN - minimal code

Write the simplest code that passes. No extra logging, no "while I'm here"
improvements. Cheating is OK in GREEN: hardcode return values, copy-paste,
duplicate, skip edge cases - REFACTOR fixes it next.

**Verify GREEN - mandatory.** Run the specific test, then the full suite.
Output pristine (no errors, warnings). Test fails? Fix the code, not the test.

### REFACTOR - clean up

After green only: remove duplication, improve names, extract helpers,
simplify expressions. Don't add behavior. If tests fail during refactor,
undo immediately and take smaller steps.

### Repeat vertically, not horizontally

One failing test → one minimal implementation → one refactor. Do NOT write
all tests first (horizontal slicing): RED becomes "write a pile of imagined
tests", GREEN becomes "make the pile pass", and the tests are brittle because
they were designed before the implementation taught you the real interface.
Use vertical tracer bullets - each one is an end-to-end behavior slice that
proves the path works and grounds the next test:

```text
WRONG:  RED: test1 test2 test3 test4  →  GREEN: impl1 impl2 impl3 impl4
RIGHT:  RED→GREEN: test1→impl1, then test2→impl2, then test3→impl3
```

## Aphrodite specifics

- **The dylib path has no tracing subscriber.** Unit tests run inside a
  harness that installs one; the live `libaphrodite_hermes.dylib` path does
  not. A test that passes in the harness can still fail live when the
  difference is a silent `tracing` call - verify behavior changes against the
  real binary (see `aphrodite-development`), not just the harness.
- **Gates after GREEN/REFACTOR** (the repo's real quality gates,
  `.hermes/AGENTS.md`): `cargo fmt --check`, `cargo clippy -p <crate> -- -D
warnings`, `ruff check plugins/aphrodite/`, `npx pyright
plugins/aphrodite/`, and `npx prettier --check .hermes/**/*.md` when docs
  changed.
- **vendor/headroom is a fork, not upstream.** Tests there run in the
  workspace; do not "fix" upstream-style code that fails your taste - the
  fork is edition-2024 and its behavior is pinned by the crates that consume
  it.

## Why Order Matters

| Excuse                                            | Reality                                                                                                                               |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| "I'll write tests after"                          | Tests-after pass immediately, proving nothing - they may test the wrong thing, miss edge cases, and never catch the bug you're fixing |
| "Already manually tested"                         | Ad-hoc: no record, can't re-run, easy to forget cases under pressure                                                                  |
| "Deleting X hours is wasteful"                    | Sunk cost. Keeping code you can't trust is the real waste                                                                             |
| "TDD is dogmatic; being pragmatic means adapting" | TDD IS pragmatic: finds bugs before commit, prevents regressions, documents behavior, enables refactoring                             |
| "Tests after achieve the same goals"              | Tests-after answer "what does this do?"; tests-first answer "what should this do?"                                                    |

## Red Flags - STOP and Start Over

- Code before test; test passes on first run; tests added "later"
- Can't explain why the test failed
- Rationalizing: "just this once", "already manually tested", "keep as
  reference" / "adapt existing code", "deleting is wasteful", "this is
  different because..."
- "Need to explore first" - fine, but throw the exploration away and start
  with TDD

All of these mean: delete the code, start over with TDD.

## Pitfalls

- **Never fix a bug without a test.** Write the failing reproduction first
  and follow the cycle - the test proves the fix and prevents regression.
  Pair with `aphrodite-testing-discipline`.
- **Never test mocks instead of real behavior.** Mocks verify interactions,
  not the system under test.
- **Never test implementation details.** Test behavior and results;
  refactoring shouldn't break tests.
- **Never test happy path only.** Cover edge cases, errors, and boundaries.
- **Never write brittle tests.** Verify behavior, not structure.
- **Never keep code written before the test** - delete it; reimplement fresh
  from tests.
- **When delegating implementation, enforce TDD in the goal.** Put "write
  failing test FIRST, run it, minimal code, verify pass, refactor - leave the
  change uncommitted for the repo's sweeper" in the `delegate_task` context
  so subagents follow the cycle.
- **Treat a hard-to-test interface as a design signal.** Must mock
  everything? Code is too coupled - use dependency injection. Huge test
  setup? Extract helpers or simplify the design.
- **Don't invent test commands.** Use the repo's actual test runner and
  invocation (`.hermes/AGENTS.md`); for the plugin that means ruff/pyright
  and its Python checks, not cargo.
- **Never trust `EXIT:$?` after a pipe.** `cargo test | tail` reports tail's
  status, not cargo's. Run the command without the pipe (full output is
  auto-saved) and read the real exit code / test summary line.

## Verification

Before marking work complete:

- [ ] Every new function/method has a test
- [ ] Watched each test fail before implementing - for the expected reason
      (feature missing, not typo)
- [ ] Wrote minimal code to pass each test
- [ ] All tests pass; output pristine
- [ ] Tests use real code (mocks only if unavoidable)
- [ ] Edge cases and errors covered

Can't check all boxes? You skipped TDD. Start over.
