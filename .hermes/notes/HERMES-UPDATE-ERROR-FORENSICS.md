# HERMES-UPDATE-ERROR-FORENSICS - Aphrodite plugin failure at 2026-09-17 ~20:33

**Date:** 2026-09-17 (EEST, UTC+3)

**Event under investigation:** Aphrodite Hermes plugin errored during a `hermes update`-adjacent gateway fleet restart (~20:33).

**Method:** read-only log forensics of `~/.hermes` (logs, config, plugin dirs, profiles) + macOS crash reports + live verification of the download URL. No repo file was modified; no commit was made.

---

## (a) The exact error(s) found - verbatim

### ERROR A - plugin disabled after failed dylib load (the ERROR the user was looking for)

`~/.hermes/logs/errors.log:6148` (identical at `~/.hermes/logs/agent.log:14697` and `~/.hermes/logs/gateway.error.log:2336`):

```
2026-09-17 20:34:46,909 ERROR aphrodite: aphrodite-hermes dylib could not be loaded (Dylib not found. Tried: ['~/.hermes/aphrodite/binaries/libaphrodite_hermes.dylib', '~/.hermes/aphrodite/binaries/libaphrodite_hermes.dylib', '…/PlayForm/Aphrodite/plugins/aphrodite/binaries/libaphrodite_hermes.dylib', '…/PlayForm/Aphrodite/plugins/binaries/libaphrodite_hermes.dylib', '…/PlayForm/target/release/libaphrodite_hermes.dylib', '…/Application/target/release/libaphrodite_hermes.dylib']) - plugin disabled; run download.sh (or unset APHRODITE_NO_AUTO_DOWNLOAD) to fetch the binaries, then restart Hermes
```

Preceding it, the auto-download failure (`errors.log:6138-6147`, `agent.log:14687-14696`):

```
2026-09-17 20:34:46,907 WARNING aphrodite: download.sh exited 1 - run download.sh manually to fetch the aphrodite binaries; output tail:
WARNING: SHA256SUMS-aarch64-apple-darwin.txt not found - skipping checksum verification for this release (older release, or the sums asset failed to publish)
aphrodite: downloading v1.4.6 for aarch64-apple-darwin from https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite%2Fv1.4.6
  aphrodite-aarch64-apple-darwin -> ~/.hermes/aphrodite/binaries/aphrodite
ERROR: curl download failed: https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite%2Fv1.4.6/aphrodite-aarch64-apple-darwin
curl: (56) The requested URL returned error: 404
```

**Live-verified:** `curl -sI` returns **404** for both
`.../releases/download/Aphrodite%2Fv1.4.6/aphrodite-aarch64-apple-darwin` and
`.../Aphrodite%2Fv1.4.6/SHA256SUMS-aarch64-apple-darwin.txt`. The v1.4.6 GitHub release does not carry the aarch64 binary asset, so the plugin's download fallback can never succeed for this version.

### ERROR B - native SIGSEGV of the gateway by the loaded dylib (the actual root cause)

The 20:33:12/38/42 "dylib loaded" lines the user saw are the _last successful dlopen()s before the process crashed_. Every gateway process that loaded `libaphrodite_hermes.dylib` from `~/.hermes/aphrodite/binaries/` segfaulted ~2 s after startup. macOS crash reports (`~/Library/Logs/DiagnosticReports/`):

`python3-2026-09-17-203329.ips` (pid 97583 - matches `tui_gateway_crash.log` pid 97583):

```
proc: python3, pid 97583
exception: {"type": "EXC_BAD_ACCESS", "signal": "SIGSEGV", "subtype": "KERN_INVALID_ADDRESS at 0x000000000cdfcfc0"}
termination: Segmentation fault: 11
faulting thread:
  libsystem_platform.dylib  _platform_strlen
  python3  z_get
  python3  PyObject_GenericGetAttr
  python3  _PyEval_EvalFrameDefault
  python3  builtin_exec
  python3  pymain_run_module
usedImages contains: libaphrodite_hermes.dylib.97583.0  (path ~/.hermes/aphrodite/binaries/libaphrodite_hermes.dylib)
```

Signature: **EXC_BAD_ACCESS → `_platform_strlen` on an invalid pointer, called through a native `z_get` trampoline during `PyObject_GenericGetAttr` inside `builtin_exec`** - i.e. during the plugin's Python init/exec phase, right after the dylib is loaded. Identical signature in **41 crash reports spanning 19:27-20:33** on 2026-09-17 (19:27:30, 19:28:40, 19:29:18, 19:41:47, 20:06:46 → 20:24:47 every ~30 s, then 20:33:19/22/24/27/29/32/38); every single one has `libaphrodite_hermes.dylib` in `usedImages`.

`~/.hermes/logs/tui_gateway_crash.log:11692-11723` - the visible crash-loop at 20:33:

```
[tui-parent] 2026-09-17T17:33:17.320Z [lifecycle] spawned gateway child pid=97516 ... cwd=…/PlayForm/Aphrodite
[tui-parent] 2026-09-17T17:33:19.662Z [lifecycle] child exit pid=97516 signal=SIGSEGV
... (repeats: 97543 @20:33:22, 97548 @20:33:24, 97553 @20:33:27, 97583 @20:33:29, 97592 @20:33:32 - all SIGSEGV ~1.5-2.5s after spawn)
[tui-parent] 2026-09-17T17:33:33.311Z [lifecycle] GatewayClient.kill reason=app.die pid=97596 ...
```

---

## (b) Which component errored

1. **`aphrodite` plugin's dylib (libaphrodite_hermes.dylib v1.4.6, plugin v2.1.4, plugin.yaml `name: aphrodite`)** - root cause. The dylib loaded from `~/.hermes/aphrodite/binaries/` deterministically segfaults the Hermes gateway process during plugin init (strlen on a bad pointer inside a native callback during `exec`).
2. **The plugin's `download.sh` auto-download fallback** - fails with curl 404 because the GitHub release `Aphrodite/v1.4.6` has no `aphrodite-aarch64-apple-darwin` asset (verified live).
3. The Hermes-side `ERROR aphrodite: ... plugin disabled` at 20:34:46 is a _consequence_: loader found no dylib, auto-download failed, plugin disabled.

Not implicated: no Python traceback / exception in `agent.log`/`errors.log` other than the aphrodite lines above; no unrelated plugin error; `hermes update` itself succeeded ("Already up to date! [main @ 64ea66b03d]" - receipt `update_20260917_202603_94438.json`, outcome `success`, v0.21.3, sha 64ea66b0 before and after).

---

## (c) 20:33 timeline reconstruction

All times EEST (UTC+3).

**Time:** ~19:27-19:28

**Event:** dylib appears at `~/.hermes/aphrodite/binaries/` (loads switch from repo-checkout path `…/plugins/aphrodite/binaries/` to `~/.hermes/aphrodite/binaries/` in agent.log). First SIGSEGV crash reports begin.

---

**Time:** 19:27-20:24

**Event:** **Continuous crash-restart loop:** dylib "loaded" every ~30 s (agent.log), each followed by a SIGSEGV crash report ~2 s later. 40+ crashes, all with the dylib loaded.

---

**Time:** 20:24:59-20:26:03

**Event:** `hermes update` (pid 94438): already up to date; runs **pending fleet restart** ("Restarting gateways left on pre-update code... ✓ Service restarted"), restarting gateways per profile. 3 more crashes at 20:24:40/43/45/47 during the restart.

---

**Time:** 20:26-20:32

**Event:** Fleet-restart gateway cycles in agent.log (plugin re-registration every ~30 s; dylib loaded at 20:26:04/34, 20:27:04, 20:30:39, 20:31:10, 20:31:40, 20:32:11, 20:32:41).

---

**Time:** **20:33:12 / 20:33:38 / 20:33:42**

**Event:** **The 3x "dylib loaded" the user pasted** (`agent.log:14549/14571/14591`, from `~/.hermes/aphrodite/binaries/`) - last successful dlopens of the crash loop.

---

**Time:** 20:33:17-33

**Event:** TUI gateway children **SIGSEGV loop** (pids 97516→97592, 6 crashes in 16 s; `tui_gateway_crash.log`); TUI kills next child at 20:33:33 (`app.die`). Crash `203338.ips` (pid 97664) at 20:33:38.

---

**Time:** 20:34:12

**Event:** One more successful load (`agent.log:14611`), then the cycle stops.

---

**Time:** **20:34:34**

**Event:** `~/.hermes/config.yaml` **replaced** (birth 20:34:34); `aphrodite` now in `plugins.disabled` (`config.yaml:566`); `plugins.entries.aphrodite.allow_tool_override` gone - capability check flips `allow`→`deny (evidence=not granted)` at 20:34:45 (`agent.log:14686`).

---

**Time:** **20:34:43**

**Event:** `~/.hermes/plugins/` mtime 20:34:43 - the `aphrodite` symlink is removed (dir now contains only `hermes-achievements`). Gateway pid 98245 starts (`gateway-exit-diag.log` 17:34:43Z).

---

**Time:** **20:34:45**

**Event:** `~/.hermes/aphrodite` **deleted and recreated empty** (birth 20:34:45; `binaries/` + `hotreload/` now empty).

---

**Time:** **20:34:46**

**Event:** `download.sh exited 1` → curl 404 → **`ERROR aphrodite: ... dylib could not be loaded ... plugin disabled`** (`errors.log:6138-6148`).

---

**Time:** 20:34:47-20:35:03

**Event:** Default gateway (pid 98245) comes up cleanly with the plugin gone - telegram connected 20:35:01, no further segfaults.

**What changed at the update:** Hermes code did NOT change (already at 64ea66b0). The update's _pending fleet restart_ re-ran every profile gateway through the broken dylib, producing the burst of load lines and crashes the user saw at 20:33. The plugin was then disabled/removed (config rewrite 20:34:34, symlink removal 20:34:43, dir deletion 20:34:45) - consistent with either a deliberate disable to stop the crash loop or an automated cleanup; no log line records which, and the removal itself is not logged (agent.log is silent 20:34:13→20:34:42 apart from plugin-discovery lines).

---

## (d) Is the 3x dylib load normal?

**No.** One dylib load per Hermes home/profile gateway start is normal; 69 "dylib loaded" lines on 2026-09-17 with a rigid ~30 s cadence (20:06:54→20:34:12) is a **crash-restart loop** - each load is followed ~2 s later by a SIGSEGV. The "3x at 20:33:12/38/42" is just three of those cycles, and each one ends in a crash, not a healthy start. Normal operation is one load per gateway start with the process surviving.

---

## (e) Verdict from the log side

**Root cause: `libaphrodite_hermes.dylib` v1.4.6 is broken - it segfaults the Hermes gateway process on load/init** (EXC_BAD_ACCESS in `_platform_strlen` via a native callback during `builtin_exec`/attribute access; 41 identical crash reports 19:27-20:33, all with the dylib loaded; deterministic). The "hermes update at 20:33" was actually: update already up to date → pending fleet restart → every restarted gateway tried to load the crashing dylib → repeated SIGSEGV → the plugin ended up disabled and its binaries deleted → the loader's auto-download fallback **404s** because the v1.4.6 GitHub release has no aarch64 asset → final `ERROR aphrodite: dylib could not be loaded ... plugin disabled` at 20:34:46.

**Two independent bugs to fix:**

1. **dylib 1.4.6 native crash** (must fix to re-enable): strlen/GC-safety bug in the dylib's Python-attribute path. Fix in the Rust code and ship a new binary.
2. **release asset missing**: `Aphrodite/v1.4.6` release lacks `aphrodite-aarch64-apple-darwin` (+ SHA256SUMS), so `download.sh` cannot ever restore the binary - attach the asset or rebuild locally.

**Current state:** plugin disabled in `~/.hermes/config.yaml`, `~/.hermes/plugins/aphrodite` symlink gone, `~/.hermes/aphrodite/binaries/` empty, `~/.hermes/aphrodite/hotreload/` empty. Until a working dylib is built (or the release asset published), the plugin stays disabled. No other Hermes components failed; `hermes update` exit code 0.

**Checked and clean (absence of evidence noted):** `agent.log` (whole 20:26-20:35 window, no Python traceback/CRITICAL), `errors.log` (only the aphrodite ERROR above), `gateway.error.log`, `gateway.log`, `gateway-exit-diag.log`, `tui_gateway_crash.log`, `update.log`, update receipts, `action-*.log` (all 0 bytes), `~/.hermes/aphrodite/` (empty; no proxy-stderr.log or hotreload artifacts exist), 41 macOS crash reports (all the same aphrodite-dylib SIGSEGV signature).
