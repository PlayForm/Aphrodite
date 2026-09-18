# PREVIEW-RACE-FIX - root cause: leaked APHRODITE_PREVIEW_MAX_CHARS env var

Date: 2026-09-18 (EEST) · Branch: Development · Binary 1.4.6 / plugin 2.1.4 · macOS

## The mystery

After Agent B's Issue #11 residual rewrite landed (commits 3b8b5d3, 1d202b2
regex-drop, 7674456 detection), `cargo test -p aphrodite --lib preview`
failed with a VARYING 21-25 failures per run. A race-fix agent added
`cap_guard()` to 20 tests (lib.rs, marker.rs, proxy.rs) and reported "3x
green" - but its baseline runs never reproduced the failure (0 failing),
while the parent session consistently saw 21-25. Its guard additions did NOT
fix the suite (still 22-24 failing per run when the parent re-ran it).

## The actual root cause (found by the parent, not the race-fix agent)

The session shell had `APHRODITE_PREVIEW_MAX_CHARS=20` exported - left over
from the Issue #11 battery agent's cap verification probe (it set the env
var to prove the WS4 cap works end-to-end, and the export persisted in the
terminal session env, inherited by every `cargo test`).

`config_loader::tests::test_apply_previews_wires_preview_max_chars` reads the
precedence chain override -> env -> toml -> default via `get_u64`. With the
env var set:

1. First assert `preview_max_chars() == 77` FAILED (left: 20 from env, right:
   77 from TOML - env wins per precedence).
2. The test PANICKED before its final restore step.
3. The cap_guard mutex released on unwind, but the process-global
   `PREVIEW_MAX_CHARS` atomic stayed at the env value (20).
4. Every subsequent content-asserting preview test read the leaked cap and
   truncated its preview to ~21 chars (`[gitlog:2 commits ...]`,
   `[html:2h 2a 1img 1...]`, `[build:1E 1W 4L | ...]`) -> cascade of failures.

Why the variance: test ordering/thread scheduling determined how many
content tests ran after the poisoned point before the process ended; the
race-fix agent's subprocess did not inherit the env var, so its runs were
green while the parent's terminal did inherit it.

## The fix

1. Cleared the leaked env var from the session.
2. Made the test HERMETIC (crates/aphrodite/src/config_loader.rs:510):
   `unsafe { std::env::remove_var("APHRODITE_PREVIEW_MAX_CHARS") }` at the
   start (backing up the prior value), restored at the end. A stray env var
   in ANY future session can no longer poison the suite.

## Verification

- WITH `APHRODITE_PREVIEW_MAX_CHARS=20` set: `cargo test -p aphrodite --lib
preview` 3x -> 65 passed / 0 failed each (hermetic fix holds).
- Full suite: 377 + 16 + 5 + 2 + 4 + 2 = 406 passed / 0 failed (1 ignored).
- `Maintain/check_ffi_contract.py`: PASS (0 violations).
- repro: SURVIVED (no crash).
- prettier --check on config_loader.rs: clean.

## Lessons

- An exported env var that a test reads by name is a SUITE-WIDE landmine:
  `get_u64`'s env branch is order-2 in precedence, so any stray value beats
  TOML. Config tests that assert precedence must be hermetic (remove the env
  var for their duration) or use a dedicated override only.
- A "3x green" report from a subagent whose subprocess does not inherit the
  parent's env is not proof - the parent must re-run the suite in ITS OWN
  env. The race-fix agent's guard additions (cap_guard on 20 tests) were
  still correct defensive hardening, but the real cause was environmental.
- When failure counts VARY between runs but also reproduce single-threaded,
  suspect a leaked process-global with an order-dependent poison point, not
  a pure race.
