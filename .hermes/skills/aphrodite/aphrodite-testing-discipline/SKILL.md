---
name: aphrodite-testing-discipline
description: "Use when probing, testing, or verifying the Aphrodite plugin or dylib: always exercise the ACTUAL plugin source code, never hand-rolled ctypes probes; always test with real code paths; keep all scratch under .hermes/tmp."
version: 2.1.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-testing-discipline
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, testing, ffi, ctypes, probes, discipline, negative-tests, hermeticity]
        related_skills: [aphrodite-boundaries, aphrodite-orientation, aphrodite-tool-testing]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - Probe/test discipline (real plugin paths, never raw ctypes)
    - Scratch-location policy (.hermes/tmp only, never /tmp)
    - Env-var hermeticity rules for config-precedence tests
    - Negative-tests table (past failures -> permanent prevention)
    - Bounded investigation workflow (6 steps)
    - Failure-behavior policy applied to test harnesses
    - Documentation linting checklist for .hermes/**/*.md
    - Confidence labels (Invariant / Source-derived / Runtime-derived / Release-derived / Historical)
    - Claim-to-test matrix policy
depends_on:
    - aphrodite-boundaries (failure-behavior policy, git repair taxonomy, stop/recovery semantics)
    - aphrodite-orientation (preflight gate before any probe run)
supersedes: []
verification:
    source_of_truth:
        - plugins/aphrodite/__init__.py (_load_dylib, _call_json, hook wrappers)
        - plugins/aphrodite/BINARY_VERSION (live version-handshake value)
        - .hermes/AGENTS.md (recorded gate numbers)
mutation_level: local
---

# Aphrodite Testing Discipline

Hard rules for probing, testing, and verifying the Aphrodite plugin and dylib.
Violating these caused real SIGSEGV crash dialogs and wasted sessions. Every
rule below exists because a past failure proved it necessary; the negative-tests
table at the end converts each failure into a permanent prevention.

## Rule 1 - Always exercise the ACTUAL plugin source

Never hand-roll a raw `ctypes.CDLL` probe against the dylib. The raw-ctypes
path defaults `restype` to `c_int`, truncates 64-bit pointers to 32 bits, and
SIGSEGVs inside `strlen` - the exact bug class the FFI pipeline was built to
eliminate. Two separate probe scripts crashed the machine this way (23:57 and
00:04, same `_platform_strlen <- string_at` signature; `EXC_BAD_ACCESS
KERN_INVALID_ADDRESS` on a sign-extended low-32-bit value). The plugin path
never crashed - the probes did.

Correct pattern - import the real plugin and drive its own functions:

```python
import sys, importlib.util
sys.path.insert(0, "plugins/aphrodite")
spec = importlib.util.spec_from_file_location("aphrodite_pkg", "plugins/aphrodite/__init__.py")
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)
dylib = mod._load_dylib()                    # real loader: hot-reload copy + mtime
# _configure_ffi is applied by _load_dylib: generated _bindings.py restype/argtypes
result = mod._call_json(dylib, "aphrodite_hermes_version")  # forces c_void_p, frees same handle
```

`_load_dylib` applies the generated `_bindings.py` (restype `c_void_p`,
argtypes `[c_char_p, c_char_p]`) and `_call_json` forces `c_void_p` and frees
through the same handle. Everything the plugin itself does, you inherit for
free. If you MUST use raw ctypes (never for the dylib - only for unrelated
libraries): set `restype = ctypes.c_void_p` AND `argtypes` BEFORE every call,
and free via the same handle that allocated.

**Contract:** the plugin's `_load_dylib`/`_call_json` pair is the only safe dylib
entry path.
**Confidence:** source-derived.
**Verify:** inspect `plugins/aphrodite/__init__.py` for `_load_dylib`,
`_configure_ffi`, and `_call_json`.
**If different:** update this skill and the FFI contract test before writing any
probe.

## Rule 2 - Always test with REAL code

- Run the REAL dylib from `target/release/` (rebuild first:
  `cargo build --release -p aphrodite -p aphrodite-hermes`), not a stale copy
  and not the debug build.
- Verify the version handshake before trusting results:
  `mod._call_json(dylib, "aphrodite_hermes_version")` must equal the CURRENT
  value read from `plugins/aphrodite/BINARY_VERSION` at handshake time.
  **Confidence:** runtime-derived - the number is a live property, never a
  remembered constant.
- A stale dylib masks source changes: after any dylib change, require a
  fresh-process (or reload) test so results cannot be attributed to an
  already-loaded dylib (negative test N-10).
- The plugin's own test suites are the ground truth: `cargo test -p
aphrodite`, `cargo test -p aphrodite-hermes`, targeted suites such as
  `cargo test -p aphrodite --lib setup::tests` (config template/setup), the
  drift-guard
  (`diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`
  must be identical), ruff, clippy `-D warnings`, pyright, prettier.
  Recorded numbers as of 2026-09-18 (`cargo test -p aphrodite`: 406 passed,
  377 lib + 29 bins, 0 failed, 1 ignored; `cargo test -p aphrodite-hermes`:
  52 passed; gates clean) are a baseline, not a substitute - run the suites
  and record the ACTUAL numbers on every verification, never report "should
  pass", report what the command printed.
- A battery/probe harness must be a real executable script whose output IS the
  evidence (rows classified, exit codes), never a report generator that
  fabricates the table from memory.

## Rule 3 - Scratch lives in .hermes/tmp, NOT /tmp

All probe scripts, generated fixtures, research dumps, and intermediate
artifacts go in `.hermes/tmp/` (repo-local, gitignored contents, tracked
`.gitkeep`). Never write scratch to `/tmp` - it is cleaned by the OS, invisible
to the repo, and scattered artifacts there caused the "where is the code?"
confusion. NEVER create or write to any other scratch location: no stray scratch
dir anywhere (repo root, .hermes, notes, or any other scratch tree) - the user
explicitly removed the stray scratch dir and forbids it; an agent that creates it is killed
and the dir deleted. The ONLY scratch dirs are `.hermes/tmp/` (repo work) and
and a separate personal scratch dir (under the user's scratch tree).

## Rule 4 - Verify through the path the USER sees

The preview the model sees must be the FINAL, honest representation. When
verifying preview or transform behavior, exercise BOTH:

1. the direct path (`_call_json` -> `aphrodite_hermes_dispatch_tool` with the
   tool payload), and
2. the hook path (the plugin's `register()`-registered hook wrappers -
   `transform_tool_result` etc. - what the LLM actually reads).

The hook path is the production path; a fix that only works direct is not done.

## Rule 5 - Env-var hermeticity

A stray exported env var is a SUITE-WIDE landmine. `APHRODITE_PREVIEW_MAX_CHARS`
(and every `APHRODITE_*` knob) beats TOML in the resolution chain (env > TOML >
default), so a value left exported from a battery probe's shell silently failed
21-25 `cargo test -p aphrodite` config-precedence tests per run with VARYING
failures. Config/precedence tests must be hermetic:

- Back up the prior value, `std::env::remove_var(...)` (Python:
  `monkeypatch.delenv`) at test start, restore it at the end.
- A subagent's clean env does NOT prove the parent's env is clean - after any
  env-var change, re-run the suite in your own shell.
- Every env var named in docs carries consumer status: active, inactive, or
  removed (documentation lint L-7). Never assume a knob that has no consumer is
  functional.
- `APHRODITE_API_KEY` is a fail-loud landmine: the proxy dies with
  `no API key configured - set APHRODITE_API_KEY env var` when the key is
  absent OR commented out in the private environment file - verify the var
  is actually exported (`env | grep APHRODITE_API_KEY`), never assume
  presence from a file's contents.

## Confidence labels

Apply a compact label immediately after any claim that is likely to drift.
Format:

**Contract:** <behavior claim>
**Confidence:** <label>
**Verify:** <read-only probe or source to inspect>
**If different:** <what to update before relying on the claim>

Labels:

| Label               | Meaning                                                                               |
| ------------------- | ------------------------------------------------------------------------------------- |
| **Invariant**       | Expected to remain true unless architecture changes; state as a permanent safety rule |
| **Source-derived**  | Must be checked in the currently checked-out source before edits                      |
| **Runtime-derived** | Must be read from the active process/configuration at probe time                      |
| **Release-derived** | Must be checked at the exact proposed release commit                                  |
| **Historical**      | Explanatory only; never copy into live implementation                                 |

A live skill never presents an implementation property as an invariant. If the
behavior differs from the documented contract, update the canonical contract
and its test matrix - do not add an unstructured note to a random reference
file (Step 6 of the investigation workflow).

## Negative-tests table (past failures -> permanent prevention)

Every known past failure becomes a permanent negative test or lint rule. The
goal is not recording lessons; it is ensuring they cannot silently recur. A
prevention must be an automated test or a lint rule - a prose warning is not a
prevention.

| Past failure                                          | Permanent prevention                                                              |
| ----------------------------------------------------- | --------------------------------------------------------------------------------- |
| `stdout` used instead of `output`                     | Hook signature/invocation compatibility test (sentinel-output test)               |
| `sessionstart` registered instead of `onsessionstart` | Valid-hook registration test                                                      |
| Pre-LLM history edited in place                       | Test asserts original conversation remains unchanged; compression uses engine API |
| Retrieval result recompressed                         | End-to-end nested-marker test (no re-marking of resolved payloads)                |
| Auto-expand setting assumed functional                | Inert-setting test; documentation lint (env-var consumer status)                  |
| Old hook script recreated                             | Repository policy/lint rejects `.githooks` restoration (removed 2026-09-17)       |
| Self-referential gitlink returns                      | Recursive mode-160000 scan (submodule diagnosis, owner: aphrodite-orientation)    |
| Tag triggers unexpected publish                       | Mandatory pre-tag workflow trigger audit (owner: aphrodite-release-flow)          |
| Missing embedded template key                         | Template/live-config drift test (e.g. drift-guard diff)                           |
| Stale dylib masks source change                       | Fresh-process version/behavior test (Rule 2 handshake after rebuild)              |

## Failure-behavior policy applied to test harnesses

`aphrodite-boundaries` is the canonical owner of the policy definitions; this
skill applies them to probes and test harnesses. Every harness path picks one
policy explicitly:

| Harness situation                                       | Policy              | Behavior in a harness                                                                     |
| ------------------------------------------------------- | ------------------- | ----------------------------------------------------------------------------------------- |
| Version handshake mismatch or FFI contract violation    | **Fail closed**     | Stop the harness with a readable diagnostic; never report results from a mismatched dylib |
| Compression/preview transform bug                       | **Fail open**       | Log structured error; return original content - a compression bug must never erase output |
| Proxy/upstream unavailable                              | **Degrade**         | Retain local/raw checks, expose degraded status, continue what is testable locally        |
| Transient socket/process start                          | **Retry boundedly** | Limited retries with backoff; then degrade                                                |
| SIGSEGV/crash dialog, invalid marker grammar, data loss | **Escalate**        | Halt the workflow; require a human decision                                               |

## Bounded investigation workflow

Debugging follows a fixed progression so it does not jump straight into edits.
Troubleshooting begins with ONE observed symptom, never a suspected root cause.

### Step 1 - Classify the symptom

- Plugin load failure
- Hook does not fire
- Hook fires but output changes unexpectedly
- Context engine absent or not selected
- CCR marker is malformed, unresolved, or unexpectedly raw
- Preview is misleading
- Cache/store behavior is unexpected
- Proxy unavailable
- Release/install/download failure
- Git/submodule/release-line inconsistency

### Step 2 - Collect immutable evidence

Capture only read-only data:

```sh
git rev-parse --show-toplevel
git branch --show-current
git status --short
git submodule status --recursive
git log -1 --oneline
```

Then collect component evidence: configuration source, process/version identity,
relevant logs, source invocation site, and a smallest reproducible input.

### Step 3 - State the expected contract

Write a single sentence: "For input X, component Y must produce output Z without
invoking subsystem W." Example: "A terminal hook given a non-empty sentinel
output must return that same output when compression is bypassed."

### Step 4 - Run the smallest discriminating test

Change one dimension only. Do not simultaneously alter environment variables,
thresholds, source code, branch, and plugin installation.

### Step 5 - Choose one bounded repair

A repair must name: exact files or state modified; expected new observation;
reversal method; and a validation that succeeds before another repair is
attempted. Never continue after a failed verification because a later step might
"fix it."

### Step 6 - Update the contract

If source behavior differed from documented behavior, update the canonical
contract and its related test matrix. Do not add an unstructured note to a
random reference file.

## Documentation linting

Skill documentation itself is tested. A `.hermes/**/*.md` document is compliant
only when:

- L-1: Exactly one `status` field.
- L-2: Every `deprecated`/`archived` document names a successor.
- L-3: No active skill references a deprecated skill without a
  "Historical only - do not execute" label.
- L-4: Every mutation command appears in a step with `Preconditions`, `Verify`,
  `Stop if`, and `Recovery`.
- L-5: Every hard-coded source line is marked historical or paired with a
  path/function search.
- L-6: Every threshold is labeled live-read, default, or test fixture.
- L-7: Every environment variable has defined consumer status: active,
  inactive, or removed.
- L-8: Every mention of a branch has declared permitted branch scope.
- L-9: Every release action has a human-approval boundary.
- L-10: No secret-like variable is printed in a sample command.

All `.hermes/**/*.md` stay prettier-clean (`npx prettier --check
.hermes/**/*.md`; tabs, width 100, proseWrap preserve).

## Claim-to-test matrix policy

If a claim cannot be tested, do not write an operational instruction that
relies on it. Every live skill ends with a claim-to-test matrix; every row is:
Claim | Evidence source | Test | Pass condition | Failure response. A claim
whose test fails does not get an exception - the claim or the implementation
changes.

## Pitfalls

- Two raw-ctypes probe scripts (Sep 17 23:57, Sep 18 00:04) SIGSEGV'd:
  `string_at` -> `_platform_strlen` on a truncated pointer. Same signature:
  `EXC_BAD_ACCESS KERN_INVALID_ADDRESS` on a sign-extended low-32-bit value.
  The plugin path never crashed - the probes did.
- A "generated" battery report with no runner script is fabrication. The
  evidence is the runner's exit codes and row table.
- The battery threshold changed between versions (terminal compression
  threshold was 256B on 1.4.5, 512B on 1.4.6).
  **Confidence:** historical observation - re-derive empirically from the live
  config, never assume the old fixture sizes still compress.
- Tests that mutate process-global state (e.g. the `preview_max_chars` cap)
  must take the module's shared test guard, or parallel cargo test runs race
  and fail intermittently - the `cap_guard` lesson from preview.rs.
- A stray exported env var is a SUITE-WIDE landmine (see Rule 5).
- **The dylib has NO tracing subscriber inside the Hermes host** -
  `tracing::warn!`/`info!` are silent no-ops there. When a probe expects log
  output from the dylib, add a stderr fallback (check
  `tracing::dispatcher::has_been_set()`) or surface a state field instead;
  never conclude absence-of-behavior from absent logs.
- **Skill content is hash-scanned on load (skills_guard)** - flagged content
  quarantines the skill until a re-scan passes; keep skill templates and
  test fixtures benign (no secret-like payloads, no executable-looking
  samples).
- **Adapt documents into notes, never copy them.** When asked to bring an
  external doc into the repo, REWRITE it into `.hermes/notes/` as an original
  adaptation: read the source, distill its substance, produce new prose in the
  repo's voice - never paste the source's sections or code blocks verbatim,
  never drop a copy of the source file into the repo.
- **Huge one-row-per-line tables: never write escaped newlines.** A bad write
  emits literal `\n` escape sequences that smash dozens of rows onto ONE
  physical line. The signature: `grep -c '\\n  |' file` > 0, or a line >5KB.
  Prevention: (1) always patch/write with REAL newlines; (2) after ANY edit,
  check the max line length stays in the low thousands; (3) verify
  `grep -c '\\n  |' file` == 0.
- **Never `git reset`/`checkout` a file to "restore" it - repair in place.** A
  corrupted write still contains the intended content; restoring from git
  THROWS THE WORK AWAY. Read the file, find the improper section, fix exactly
  that. If the working tree was already reset, recover the original blob from
  git's object store (`git fsck --lost-found`, `git cat-file -s <hash>`,
  `git cat-file blob <hash>`), repair, and re-write. Git repair taxonomy
  (canonical): `aphrodite-boundaries`.

## Verification Checklist

- [ ] Dylib loaded via the plugin's `_load_dylib()` / `_call_json()`, zero raw
      `ctypes.CDLL` in the probe
- [ ] Version handshake: dylib version == BINARY_VERSION (fresh process, not
      stale dylib)
- [ ] Release dylib rebuilt, not stale
- [ ] Real test suites run and ACTUAL numbers recorded
- [ ] No stray `APHRODITE_*` env vars exported (config tests hermetic:
      remove_var/restore, or monkeypatch.delenv)
- [ ] Scratch in `.hermes/tmp/`, never `/tmp`
- [ ] No crash dialogs, repro SURVIVED, zero new SIGSEGV
- [ ] Docs pass documentation linting (L-1..L-10) and prettier

## Local test matrix

| Claim                                    | Evidence source                    | Test                                                         | Pass condition                                        | Failure response                             |
| ---------------------------------------- | ---------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------- | -------------------------------------------- |
| Raw-ctypes probes SIGSEGV (historical)   | Crash logs, Sep 17-18              | Grep all probes for `ctypes.CDLL`                            | Zero raw CDLL in probes; plugin path used             | Rewrite probe via `_load_dylib`/`_call_json` |
| Version handshake catches stale dylib    | `plugins/aphrodite/BINARY_VERSION` | Fresh-process version probe after rebuild                    | Reported version equals file value                    | Rebuild and re-run; do not trust old process |
| Env leak breaks config tests             | `cargo test -p aphrodite`          | Export `APHRODITE_PREVIEW_MAX_CHARS`, run suite, then remove | Failures appear when leaked; green when hermetic      | Add remove_var/restore guard to the test     |
| Negative tests prevent recurrence        | This table                         | For each row, confirm the prevention test exists             | Prevention red on the old bug, green on the fix       | Add the missing test/lint before closing     |
| Investigation workflow bounds repair     | This skill                         | Simulated failure: run Steps 1-6                             | One bounded repair after smallest discriminating test | Re-run workflow with immutable evidence      |
| Thresholds are live-read, not remembered | Active config / proxy              | Read threshold from config, test at T-1/T/T+1                | Observed transition matches read value                | Update the skill's recorded threshold        |
