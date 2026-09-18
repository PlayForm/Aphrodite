# HERMES-UPDATE-ERROR - PLUGIN-SIDE ROOT-CAUSE INVESTIGATION

Date: 2026-09-17 (post-mortem, ~20:40 EEST)
Scope: `plugins/aphrodite/__init__.py` startup/registration/version-handshake paths only.
Read-only investigation: no repo or `~/.hermes` files modified.

---

## 0. Observed timeline (from logs, crash reports, filesystem mtimes)

**Time (EEST):** 20:08-20:18

**Event:** Pre-update CPython heap-corruption crashes (`python-*.ips`, worker-thread, same strlen-in-getattr signature). Plugin was active in these processes.

---

**Time (EEST):** 20:24:40-20:24:47

**Event:** Gateway children SIGSEGV burst (crash reports `python3-2026-09-17-2024*.ips`, pids 94314/94331/94337/94342) - **starts 19 s BEFORE `hermes update`** (started 20:24:59, receipt `update_20260917_202603_94438.json`, outcome success, no version change: 0.21.3 → 0.21.3).

---

**Time (EEST):** 20:25:03 → 20:34:12

**Event:** Plugin loads every ~30 s (16+ loads in `agent.log`): `capability_check tools.override allow (legacy)` → `dylib loaded: ~/.hermes/aphrodite/binaries/...` → 2 layout warnings. **No `registered N hooks` / `registered N tools` / proxy lines after any of these loads.**

---

**Time (EEST):** 20:33:11 / 20:33:37 / 20:33:41

**Event:** The "3x load" the user saw - three gateway children in the crash-loop window.

---

**Time (EEST):** 20:33:14

**Event:** `.update_check` written (`ver 0.21.3, behind 0, head==target 64ea66b0`) - an update _check_, nothing to pull.

---

**Time (EEST):** 20:33:17-20:33:38

**Event:** SIGSEGV burst 2 (crash reports `python3-2026-09-17-2033*.ips`, pids 97516/97543/97548/97553/97583/97592; `tui_gateway_crash.log` lifecycle lines).

---

**Time (EEST):** ~20:34:34

**Event:** `~/.hermes/aphrodite/` recreated empty (`binaries/`, `hotreload/`), `~/.hermes/plugins/` touched, `config.yaml` rewritten (mtime 20:34) - **aphrodite added to `plugins.disabled`** (it was NOT disabled before: 0.21.3 gates loading on `plugins.disabled` - `hermes_cli/plugins.py:1324-1334` - and the plugin demonstrably loaded 20:25-20:34).

---

**Time (EEST):** 20:34:43

**Event:** Gateway restarted (98245); 20:34:45 `capability_check tools.override decision=deny evidence="not granted"` (plugin no longer resolvable - symlink gone).

---

**Time (EEST):** 20:34:46

**Event:** **The ERROR line the user saw** (in `errors.log` + `gateway.error.log`): `ERROR aphrodite: aphrodite-hermes dylib could not be loaded (Dylib not found. Tried: [6 paths]) - plugin disabled` preceded by `WARNING aphrodite: download.sh exited 1 ... ERROR: curl download failed ... curl: (56) The requested URL returned error: 404`.

---

**Time (EEST):** 20:35:08

**Event:** Gateway child 98508 - **stable, no further crashes after the plugin stopped loading**.

Current state (verified): `~/.hermes/plugins/aphrodite` symlink is **GONE** (only `hermes-achievements` remains); `~/.hermes/aphrodite/binaries/` and `hotreload/` exist but are **empty** (created 20:34:34); no dylib/binary in any of the 6 candidate paths.

---

## (a) Plugin startup failure points (`plugins/aphrodite/__init__.py`, line refs to current file)

**Defensive (wrapped - degrade to warning or graceful plugin-disable, cannot abort Hermes):**

1. `_process_state()` L68-94 - sys.modules.setdefault guarded; fallback holder. Safe.
2. `_data_dir()` L107-114 - wrapped. Safe.
3. `_reap_stale_hotreloads()` L197-241 + module-level sweep L837-838 - fully wrapped. Safe.
4. `_check_version()` L482-504 - **fully wrapped, "warn (never raise)" by design**. Cannot be the error.
5. `_ensure_binaries()` L543-587 - download.sh failures only WARN (L570-587); never raise. But see #8.
6. `register()` L851-866 - `_load_dylib()` wrapped: ANY dylib failure (missing, corrupt, wrong-arch, missing symbol) → `ERROR ... plugin disabled` + return. This is the 20:34:46 path working as designed.
7. `check_and_heal` L875-882 (layout_check.py) - wrapped; verified non-destructive to the runtime home: it moves _forbidden content out of the plugin dir into_ `~/.hermes/aphrodite/binaries` and removes _plugin source files_ straying into the runtime home (`runtime_home_forbidden` = **init**.py/plugin.yaml/README/download.sh/BINARY_VERSION/layout_check.py). It can NOT wipe `~/.hermes/aphrodite/binaries/` - exonerated as the wiper.
8. `materialize_directives` L890-896 - wrapped (Python-side). **However, the FFI call itself can native-crash (SIGSEGV) - no Python try/except can catch that.** The Rust export exists (`aphrodite_hermes_materialize_directives`, `crates/aphrodite-hermes/src/lib.rs:540`).
9. `register_tool` loop L935-941 - wrapped per-tool. Safe.

**UNWRAPPED - can raise out of `register()` and abort plugin loading with a Python error:**

10. **L899 `hooks = _call_json(dylib, "aphrodite_hermes_get_hooks")`** - `JSONDecodeError` (non-JSON return), `TypeError`, or `AttributeError` propagates. (Symbol absence is already caught earlier at L410-429 → RuntimeError → graceful, so the live risk here is a JSON-contract mismatch on a symbol-compatible dylib.)
11. **L932 `schemas = _call_json(dylib, "aphrodite_hermes_get_schemas")`** - same.
12. **L923-928 `register_hook` loop** - NOT wrapped, while the parallel `register_tool` loop IS (asymmetry). A raise from `ctx.register_hook` (Hermes API change, bad hook name) aborts registration. (Verified 0.21.3 still ships `register_hook` - `hermes_cli/plugins.py:916` - so this did NOT fire at 20:33; latent risk for future Hermes releases.)

**Native-crash points (no Python exception possible - kill the whole process):**

13. `_load_dylib()` L392 `ctypes.CDLL(load_path)` and every subsequent FFI call (L410-420 restype setup; `_call_json` L445-456; `_read_str` L437-442 `ctypes.cast(ptr, c_char_p).value` on a bad pointer). A buggy/mismatched dylib segfaults the gateway with **no traceback** - exactly what the 20:24-20:33 SIGSEGV crash reports show (see (d)).
14. `_call_json` L455 `aphrodite_hermes_free_string(ptr)` - documented F4 hazard: freeing a pointer allocated by a _different_ hot-reload dylib image is UB → heap corruption. The shim's own comments flag this.
15. `_load_dylib` L346/`_start_proxy` L672 `_ensure_binaries()` → download.sh - network fetch at registration time. If it fails (observed: **curl 404**), the plugin hits the assert at L366 → disabled. Not a raise, but a hard functional failure.
16. Minor: `_start_proxy` L700-701 `os.chmod(binary, 0o755)` unguarded (EROFS/PermissionError on read-only mounts); `_hotreload_dir()` L128 mkdir unguarded when reached via `_load_fresh_copy` L265 (OSError → caught by register's try → disabled). Low-probability.

**Hard failure observed in this incident:** `_load_dylib` L366 `assert os.path.exists(path)` → AssertionError ("Dylib not found. Tried: [...]") after `_ensure_binaries` failed - caught by register's L851-866 → logged as ERROR + plugin disabled.

---

## (b) Version handshake - does 1.4.6 / 1.4.6 pass?

**Yes - it passes silently.**

- `_check_version()` (L482-504) reads the dylib's `aphrodite_hermes_version()` JSON (`{"version": env!("CARGO_PKG_VERSION")}`, `lib.rs:279-280`) and compares it against the `BINARY_VERSION` file (plugin dir, currently `"1.4.6"`, 6 bytes, `.strip()`ed).
- Task-verified dylib reports `1.4.6` → **equal → no warning, no error**. The handshake is **incapable of raising** (entire body wrapped, L489-504).
- What a hermes-update-triggered re-download would do: `download.sh` keys off `BINARY_VERSION` (L41-43), so a fetch for `1.4.6` hits the GitHub tag `Aphrodite/v1.4.6` → **that release does not exist** (GitHub API verified: latest published release is **Aphrodite/v1.4.5**; the `v1.4.6` tag returns 404, asset URL returns 404). Any mismatch would only ever WARN - the real hazard is contract drift: a 1.4.5-or-older dylib lacks `materialize_directives` (added for 1.4.6) → caught at `_load_dylib` L421-429 as a clear RuntimeError (graceful), or - worse - a symbol-compatible but semantically different dylib → native crash / unwrapped `_call_json` errors (points 10-11, 13).
- **Conclusion: the version handshake is NOT the error source.** `BINARY_VERSION=1.4.6` is itself the latent blocker: the auto-fetch cannot self-heal until the release is published (or the pin is dropped to a published tag).

---

## (c) Capability / config findings

- `~/.hermes/config.yaml` (rewritten 20:34): `plugins.entries.aphrodite.allow_tool_override: true` (L609-610) - the **deprecated legacy key**. Hermes 0.21.3 logs `capability_check plugin=aphrodite capability=tools.override decision=allow checked_by=plugin_capability_granted evidence=legacy key plugins.entries.aphrodite.allow_tool_override (deprecated)` for it - informational, `decision=allow`, **not an error**.
- The SAME config lists `aphrodite` under `plugins.disabled` (L566). Since 0.21.3 gates plugin loading on `plugins.disabled` (`hermes_cli/plugins.py:1324-1334`) and the plugin demonstrably loaded 20:25-20:34, **the disabled entry was added during the ~20:34 cleanup - a consequence/mitigation, not the cause**. A hermes update does not rewrite `plugins.entries` (update receipt shows only config-format migrations v42→v45 for `dev-aphrodite`; no plugin-entry migration), so the update did not change this key - the "(deprecated)" note is Hermes-side informational about the key's status.
- `~/.hermes/profiles/dev-aphrodite/config.yaml` L628-631: `plugins.enabled: [aphrodite]`, `disabled: []` - dev-aphrodite still enables the plugin and would load it on next start **if the symlink and binaries were restored**.
- At 20:34:45 the capability flipped to `decision=deny evidence="not granted"` - the plugin simply was no longer present (symlink removed); nothing about the key changed.

---

## (d) The 3x-load analysis

- The three loads at 20:33:11 / 20:33:37 / 20:33:41 are **the same home, same symlink, same canonical dylib** - three _successive gateway children_ in a crash-restart loop, not three different homes. `agent.log` shows ~30 s cadence from 20:25:03 to 20:34:12 (16+ loads); the user's "3x" is a ~60 s window of it.
- Per-home: root (default profile) gateway = the crash-looping process. `dev-aphrodite` profile runs a separate gateway (pid 666, manual restart mechanism per update.log) - not in this loop. `plugins.disabled` was not yet set during these loads.
- Every load: `capability allow` → `dylib loaded` → 2 layout warnings → **then nothing**: no `registered N hooks`, no `registered N tools`, no proxy lines. Either the process died natively right after (SIGSEGV), or `get_hooks`/`get_schemas` returned falsy. Given the crash reports, the former.
- macOS crash reports (`~/Library/Logs/DiagnosticReports/python3-2026-09-17-2024*.ips`, `-2033*.ips`): **EXC_BAD_ACCESS / SIGSEGV, KERN_INVALID_ADDRESS**, faulting in `_platform_strlen` ← `z_get` ← `PyObject_GenericGetAttr` ← `_PyEval_EvalFrameDefault` ← `builtin_exec` (module exec at startup) - i.e., **CPython heap corruption surfacing as attribute lookup on a garbage string pointer**, a classic signature of a native FFI misuse earlier in the process (no Python traceback can exist for a segfault - consistent with zero tracebacks in logs).
- The identical signature appears in **pre-update** worker-thread crashes (20:08-20:18, `python-*.ips`) - the corruption predates the update; the update's gateway restart loop amplified it into a startup crash-loop. The loop **stopped only once the plugin stopped loading** (post-20:34).
- What differs per-load: nothing on the plugin side. What differs per-home (root vs dev-aphrodite): config only (`disabled` vs `enabled`), same dylib.

---

## (e) Verdict - most probable plugin-side root cause

The plugin's startup path is defensive almost everywhere and the version handshake is clean. Two plugin-side failure modes are real in this incident:

1. **PRIMARY (probable): native SIGSEGV during gateway-child startup while the plugin's ctypes FFI registration runs** (points 8/13/14). The gateway children crash-looped (SIGSEGV, heap-corruption signature) from 20:24:40 to 20:33:38 - the user's window - and stabilized only after the plugin was removed. Attribution is **circumstantial but strong**: the dylib is the only native FFI consumer in the startup path, the shim's own comments document the free_string-across-hot-reload-image (F4) and pointer-validity hazards, and the crash signature is the canonical "heap corrupted by native code" pattern. It is **not proven**: identical crashes existed pre-update (20:08-20:18), and the faulting stack is deep in CPython, not in `libaphrodite_hermes`. A core-dump/ASAN repro (plugin on vs off) is the deciding test.
2. **CONFIRMED (the error line the user most likely saw): the 20:34:46 `ERROR aphrodite: dylib could not be loaded ... plugin disabled`** - after the runtime home was wiped (~20:34:34), `_load_dylib` found no dylib anywhere, `_ensure_binaries` ran `download.sh`, which **failed with curl 404 because the GitHub release `Aphrodite/v1.4.6` does not exist** (latest published: v1.4.5). The plugin then disabled itself gracefully via the L851-866 guard. This is a deterministic, reproducible plugin-side failure: **with `BINARY_VERSION=1.4.6` and no release published, the auto-fetch can never recover.**

**What to check on next restart:**

- `~/.hermes/plugins/aphrodite` symlink is **gone** - the plugin will not load in any home until re-linked. Re-link (or restore via setup) before anything else.
- With `BINARY_VERSION=1.4.6` the fetch will 404 again: publish the `Aphrodite/v1.4.6` release with the standard assets (binary per-platform + `libaphrodite_hermes-*.dylib`), or temporarily pin `BINARY_VERSION` to `1.4.5` (a published tag) to recover.
- Root `config.yaml` has `aphrodite` in `plugins.disabled` → it will not load in the root home even after re-linking (dev-aphrodite still enables it). Remove it from `disabled` if the root home should load it.
- After a successful load, look for `INFO aphrodite: registered N hooks` / `registered N tools` - their absence (as in every 20:25-20:34 load) means registration stopped before `get_hooks`/`get_schemas`.
- Watch `~/Library/Logs/DiagnosticReports/` for new `python3-*.ips` after re-enabling: if SIGSEGV resumes, isolate by setting `APHRODITE_NO_AUTO_DOWNLOAD=1`+`APHRODITE_NO_AUTO_LAUNCH=1` and disabling the plugin, then bisect the dylib build (1.4.5 vs 1.4.6) and audit the `materialize_directives` FFI + cross-image `free_string` (F4).

## Appendix - key evidence files

- `~/.hermes/logs/errors.log` L6138-6148 (download.sh 404 + dylib-not-found ERROR)
- `~/.hermes/logs/agent.log` L14534-14610 (capability_check + dylib loaded ×N, 20:33-20:34)
- `~/.hermes/logs/gateway.error.log` L2326-2336 (gateway-side 20:34:46 error)
- `~/.hermes/logs/tui_gateway_crash.log` (SIGSEGV lifecycle 20:24-20:35)
- `~/Library/Logs/DiagnosticReports/python3-2026-09-17-203319.ips` (+ 2024*/2033* siblings) - EXC_BAD_ACCESS in strlen/getattr during builtin_exec
- `~/.hermes/logs/update_receipts/update_20260917_202603_94438.json` (update success, no version change)
- GitHub API: `repos/PlayForm/Aphrodite/releases` → latest is `Aphrodite/v1.4.5`; `.../tags/Aphrodite%2Fv1.4.6` → 404
