---
name: aphrodite-testing-discipline
description: "Use when probing, testing, or verifying the Aphrodite plugin or dylib: always exercise the ACTUAL plugin source code, never hand-rolled ctypes probes; always test with real code paths; keep all scratch under .hermes/tmp."
version: 2.2.0
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

Hard rules for probing, testing, and verifying the Aphrodite plugin and its dylib. Every rule is a response to a past failure; references/negative-tests.md converts each failure into a permanent prevention.

## Stop if / Recovery

- Stop if a probe opens the dylib through raw `ctypes.CDLL` instead of the plugin's own loader. Recovery: delete the probe and rewrite it through `_load_dylib()` / `_call_json()` (Rule 1) before running anything else.
- Stop if the observed symptom changes while a repair is in progress. Recovery: restart the bounded investigation workflow at Step 1 with the new symptom.

## Rule 1 - Always exercise the ACTUAL plugin source

Never hand-roll a raw `ctypes.CDLL` probe against the dylib because the raw-ctypes path defaults `restype` to `c_int`, truncates 64-bit pointers to 32 bits, and SIGSEGVs inside `strlen` - the exact bug class the FFI pipeline was built to eliminate. Two separate probe scripts crashed the machine this way (`_platform_strlen <- string_at` signature; `EXC_BAD_ACCESS KERN_INVALID_ADDRESS` on a sign-extended low-32-bit value). The plugin path never crashed. The probes did. **Confidence:** historical - the matrix row below tests the prevention, not the memory.

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

`_load_dylib` applies the generated `_bindings.py` (restype `c_void_p`, argtypes `[c_char_p, c_char_p]`); `_call_json` forces `c_void_p` and frees through the same handle. Everything the plugin itself does, you inherit for free. Raw ctypes is never for the dylib - only for unrelated libraries: set `restype = ctypes.c_void_p` AND `argtypes` BEFORE every call, and free via the same handle that allocated.

**Contract:** the plugin's `_load_dylib`/`_call_json` pair is the only safe dylib entry path.
**Confidence:** source-derived.
**Verify:** inspect `plugins/aphrodite/__init__.py` for `_load_dylib`, `_configure_ffi`, and `_call_json`.
**If different:** update this skill and the FFI contract test before writing any probe.

## Rule 2 - Always test with REAL code

- Run the REAL dylib from `target/release/` (rebuild first: `cargo build --release -p aphrodite -p aphrodite-hermes`). A stale copy is not the release dylib. The debug build is not the release dylib.
- Verify the version handshake before trusting results: `mod._call_json(dylib, "aphrodite_hermes_version")` must equal the CURRENT value read from `plugins/aphrodite/BINARY_VERSION` at handshake time. **Confidence:** runtime-derived - the number is a live property, never a remembered constant.
- A stale dylib masks source changes: after any dylib change, require a fresh-process (or reload) test so results cannot be attributed to an already-loaded dylib (negative test N-10).
- The plugin's own test suites are the ground truth: `cargo test -p aphrodite`, `cargo test -p aphrodite-hermes`, targeted suites such as `cargo test -p aphrodite --lib setup::tests` (config template/setup), the drift-guard (`diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` must be identical), ruff, clippy `-D warnings`, pyright, prettier. Recorded baseline (**Confidence:** historical, CLAIM - re-run to re-derive): `cargo test -p aphrodite`: 406 passed, 377 lib + 29 bins, 0 failed, 1 ignored; `cargo test -p aphrodite-hermes`: 52 passed; gates clean. A baseline is not a substitute for a fresh run. Run the suites and record the ACTUAL numbers on every verification; never report "should pass" - report what the command printed.
- A battery/probe harness must be a real executable script whose output IS the evidence (rows classified, exit codes). It is not a report generator that fabricates the table from memory.

## Rule 3 - Scratch lives in .hermes/tmp, NOT /tmp

All probe scripts, generated fixtures, research dumps, and intermediate artifacts go in `.hermes/tmp/` (repo-local, gitignored contents, tracked `.gitkeep`). Never write scratch to `/tmp` because the OS cleans it, the repo cannot see it, and scattered artifacts there caused the "where is the code?" confusion. NEVER create or write to any other scratch location (repo root, .hermes, notes, or any other scratch tree) because the user explicitly removed the stray scratch dir and forbids it - an agent that creates it is killed and the dir deleted. The ONLY scratch dirs are `.hermes/tmp/` (repo work) and a separate personal scratch dir (under the user's scratch tree).

## Rule 4 - Verify through the path the USER sees

The preview the model sees must be the FINAL, honest representation. When verifying preview or transform behavior, exercise BOTH paths:

1. the direct path (`_call_json` -> `aphrodite_hermes_dispatch_tool` with the tool payload), and
2. the hook path (the plugin's `register()`-registered hook wrappers - `transform_tool_result` etc. - what the LLM actually reads).

The hook path is the production path. A fix that passes only the direct path is not done.

## Rule 5 - Env-var hermeticity

A stray exported env var is a SUITE-WIDE landmine. `APHRODITE_PREVIEW_MAX_CHARS` (and every `APHRODITE_*` knob) beats TOML in the resolution chain (env > TOML > default), so a value left exported from a battery probe's shell silently failed 21-25 `cargo test -p aphrodite` config-precedence tests per run with VARYING failures. Config/precedence tests must be hermetic:

- Back up the prior value, `std::env::remove_var(...)` (Python: `monkeypatch.delenv`) at test start, restore it at the end.
- A subagent's clean env does NOT prove the parent's env is clean: after any env-var change, re-run the suite in your own shell.
- Every env var named in docs carries consumer status: active, inactive, or removed (documentation lint L-7). Never assume a knob that has no consumer is functional because a knob without a consumer is a knob no code path reads.
- `APHRODITE_API_KEY` is a fail-loud landmine: the proxy dies with `no API key configured - set APHRODITE_API_KEY env var` when the key is absent OR commented out in the private environment file. Verify the var is actually exported (`env | grep APHRODITE_API_KEY`). Presence in a file's contents is not proof the var is exported.

## Confidence labels

Apply a compact label immediately after any claim that is likely to drift. Format:

**Contract:** <behavior claim>
**Confidence:** <label>
**Verify:** <read-only probe or source to inspect>
**If different:** <what to update before relying on the claim>

Labels:

| Label | Meaning |
| --- | --- |
| **Invariant** | Expected to remain true unless architecture changes; state as a permanent safety rule |
| **Source-derived** | Must be checked in the currently checked-out source before edits |
| **Runtime-derived** | Must be read from the active process/configuration at probe time |
| **Release-derived** | Must be checked at the exact proposed release commit |
| **Historical** | Explanatory only; never copy into live implementation |

A live skill never presents an implementation property as an invariant. If the behavior differs from the documented contract, update the canonical contract and its test matrix - do not add an unstructured note to a random reference file (Step 6 of the investigation workflow).

## Negative-tests table (past failures -> permanent prevention)

Every known past failure becomes a permanent negative test or lint rule. The goal is not recording lessons; it is ensuring they cannot silently recur. A prevention must be an automated test or a lint rule - a prose warning is not a prevention. The full failure-to-prevention table is in references/negative-tests.md.

## Failure-behavior policy applied to test harnesses

`aphrodite-boundaries` is the canonical owner of the policy definitions; this skill applies them to probes and test harnesses. Every harness path picks one policy explicitly:

| Harness situation | Policy | Behavior in a harness |
| --- | --- | --- |
| Version handshake mismatch or FFI contract violation | **Fail closed** | Stop the harness with a readable diagnostic; never report results from a mismatched dylib |
| Compression/preview transform bug | **Fail open** | Log structured error; return original content - a compression bug must never erase output |
| Proxy/upstream unavailable | **Degrade** | Retain local/raw checks, expose degraded status, continue what is testable locally |
| Transient socket/process start | **Retry boundedly** | Limited retries with backoff; then degrade |
| SIGSEGV/crash dialog, invalid marker grammar, data loss | **Escalate** | Halt the workflow; require a human decision |

## Bounded investigation workflow

Debugging follows a fixed progression so it does not jump straight into edits. Troubleshooting begins with ONE observed symptom, never a suspected root cause.

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

Then collect component evidence: configuration source, process/version identity, relevant logs, source invocation site, and a smallest reproducible input.

### Step 3 - State the expected contract

Write a single sentence: "For input X, component Y must produce output Z without invoking subsystem W." Example: "A terminal hook given a non-empty sentinel output must return that same output when compression is bypassed."

### Step 4 - Run the smallest discriminating test

Change one dimension only. Do not simultaneously alter environment variables, thresholds, source code, branch, and plugin installation.

### Step 5 - Choose one bounded repair

A repair must name: exact files or state modified; expected new observation; reversal method; and a validation that succeeds before another repair is attempted. Never continue after a failed verification because a later step might "fix it."

### Step 6 - Update the contract

If source behavior differed from documented behavior, update the canonical contract and its related test matrix. Do not add an unstructured note to a random reference file.

## Documentation linting

Skill documentation itself is tested. A `.hermes/**/*.md` document is compliant only when the L-1..L-10 rules in references/documentation-lint.md all pass. All `.hermes/**/*.md` stay prettier-clean (`npx prettier --check .hermes/**/*.md`; tabs, width 100, proseWrap preserve).

## Claim-to-test matrix policy

If a claim cannot be tested, do not write an operational instruction that relies on it. Every live skill ends with a claim-to-test matrix; every row is: Claim | Evidence source | Test | Pass condition | Failure response. A claim whose test fails does not get an exception - the claim or the implementation changes.

## Pitfalls

- Never use raw `ctypes.CDLL` in a probe because the truncated-pointer crash is fatal (see Rule 1). The plugin path never crashed - the probes did.
- A "generated" battery report with no runner script is fabrication: the evidence is the runner's exit codes and row table.
- The battery threshold changed between versions (terminal compression threshold was 256B on 1.4.5, 512B on 1.4.6). **Confidence:** historical observation - re-derive empirically from the live config, never assume the old fixture sizes still compress.
- Tests that mutate process-global state (e.g. the `preview_max_chars` cap) must take the module's shared test guard, or parallel cargo test runs race and fail intermittently - the `cap_guard` lesson from preview.rs.
- A stray exported env var is a SUITE-WIDE landmine (see Rule 5).
- **The dylib has NO tracing subscriber inside the Hermes host** - `tracing::warn!`/`info!` are silent no-ops there. When a probe expects log output from the dylib, add a stderr fallback (check `tracing::dispatcher::has_been_set()`) or surface a state field instead; never conclude absence-of-behavior from absent logs.
- **Skill content is hash-scanned on load (skills_guard)** (CLAIM - no probe given) - flagged content quarantines the skill until a re-scan passes; keep skill templates and test fixtures benign (no secret-like payloads, no executable-looking samples).
- **Adapt documents into notes, never copy them.** When asked to bring an external doc into the repo, REWRITE it into `.hermes/notes/` as an original adaptation: read the source, distill its substance, produce new prose in the repo's voice - never paste the source's sections or code blocks verbatim, never drop a copy of the source file into the repo.
- **Huge one-row-per-line tables: never write escaped newlines** because a bad write emits literal `\n` escape sequences that smash dozens of rows onto ONE physical line. The signature: `grep -c '\\n  |' file` > 0, or a line >5KB. Prevention: (1) always patch/write with REAL newlines; (2) after ANY edit, check the max line length stays in the low thousands; (3) verify `grep -c '\\n  |' file` == 0.
- **Never `git reset`/`checkout` a file to "restore" it - repair in place** because a corrupted write still contains the intended content, and restoring from git THROWS THE WORK AWAY. Read the file, find the improper section, fix exactly that. If the working tree was already reset, recover the original blob from git's object store (`git fsck --lost-found`, `git cat-file -s <hash>`, `git cat-file blob <hash>`), repair, and re-write. Git repair taxonomy (canonical): `aphrodite-boundaries`.

## Verification checklist

The finish gate is in references/verification-checklist.md; run it before closing any verification session.

## Local test matrix

| Claim | Evidence source | Test | Pass condition | Failure response |
| --- | --- | --- | --- | --- |
| Raw-ctypes probes SIGSEGV (historical) | Crash logs (historical) | Grep all probes for `ctypes.CDLL` | Zero raw CDLL in probes; plugin path used | Rewrite probe via `_load_dylib`/`_call_json` |
| Version handshake catches stale dylib | `plugins/aphrodite/BINARY_VERSION` | Fresh-process version probe after rebuild | Reported version equals file value | Rebuild and re-run; do not trust old process |
| Env leak breaks config tests | `cargo test -p aphrodite` | Export `APHRODITE_PREVIEW_MAX_CHARS`, run suite, then remove | Failures appear when leaked; green when hermetic | Add remove_var/restore guard to the test |
| Negative tests prevent recurrence | references/negative-tests.md | For each row, confirm the corresponding prevention test is defined in references/negative-tests.md | Prevention red on the old bug, green on the fix | Add the missing test/lint before closing |
| Investigation workflow bounds repair | This skill | Simulated failure: run Steps 1-6 | One bounded repair after smallest discriminating test | Re-run workflow with immutable evidence |
| Thresholds are live-read, not remembered | Active config / proxy | Read threshold from config, test at T-1/T/T+1 | Observed transition matches read value | Update the skill's recorded threshold |