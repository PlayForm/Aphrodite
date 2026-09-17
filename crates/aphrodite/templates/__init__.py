"""aphrodite - CCR compression plugin for Hermes Agent (Rust-powered).

All logic in libaphrodite_hermes.dylib. This file is a thin registration shim.
Architecture: __init__.py → ctypes → libaphrodite_hermes.dylib → aphrodite crate (rlib)
"""

import contextlib
import ctypes
import itertools
import json
import logging
import os
import shutil
import subprocess
import sys
import threading
import types
from collections.abc import Callable
from pathlib import Path
from typing import Any

_log = logging.getLogger("aphrodite")

# ── Dylib path resolution ──
_PLUGIN_DIR = Path(__file__).resolve().parent
_DYLIB_NAME = (
    "libaphrodite_hermes.dylib"
    if sys.platform == "darwin"
    else "libaphrodite_hermes.so"
    if sys.platform == "linux"
    else "aphrodite_hermes.dll"
)
# Canonical runtime home: every runtime artifact (binaries, dylib,
# aphrodite.toml, ccr.db, hotreload cache) lives under
# ~/.hermes/aphrodite, never inside the plugin tree. Env overrides stay
# first; the plugin-dir binaries/ paths survive only as legacy fallbacks.
_BINARIES_DIR = Path.home() / ".hermes" / "aphrodite" / "binaries"
_DYLIB_PATH = os.environ.get(
    "APHRODITE_HERMES_DYLIB_PATH", str(_BINARIES_DIR / _DYLIB_NAME)
)
_BINARY_NAME = "aphrodite.exe" if sys.platform == "win32" else "aphrodite"
_BINARY_PATH = os.environ.get("APHRODITE_BINARY_PATH", str(_BINARIES_DIR / _BINARY_NAME))

# Directives are provided by the BINARY (embedded in libaphrodite_hermes.dylib
# via the core crate's builtin_directives) and materialized into the user-data
# home at setup/startup - they are NOT shipped in the plugin dir anymore.
# Point the dylib's discovery at the canonical runtime home so it reads the
# materialized set; os.environ.setdefault keeps a user-provided
# APHRODITE_DIRECTIVES_DIR override authoritative.
os.environ.setdefault("APHRODITE_DIRECTIVES_DIR", str(Path.home() / ".hermes" / "aphrodite" / "directives"))

# ── Per-process dylib state ──
# Hermes builds one PluginManager per Hermes home (root home + every
# profile) and exec_module()s this shim once per home under a distinct
# module name. Plain module globals therefore restart from scratch on the
# second load: `_dylib_gen` yields 0 again, `_load_fresh_copy` targets the
# very same `<name>.<pid>.0` path the first load already mapped, and on
# Windows overwriting a mapped DLL fails with PermissionError [Errno 13].
# Keeping the state in a synthetic module registered under a fixed name in
# sys.modules makes it survive re-exec: later copies of the shim find the
# holder, hit the mtime early-return in `_load_dylib`, and reuse the CDLL
# handle that is already mapped instead of copying (and leaking) a second
# image. A ModuleType (not a namespace/class instance) so it is keyed by a
# stable name rather than by the identity of whichever shim created it.
_STATE_MODULE_NAME = "aphrodite_hermes._process_state"


def _process_state() -> types.ModuleType:
    """Return the process-wide state holder, creating it on first use."""
    holder = types.ModuleType(
        _STATE_MODULE_NAME, "Process-global aphrodite dylib state shared by every loaded shim copy."
    )
    holder.dylib = None  # type: ignore[attr-defined]  # ctypes.CDLL | None
    holder.dylib_mtime = 0.0  # type: ignore[attr-defined]
    holder.dylib_copy_path = None  # type: ignore[attr-defined]  # str | None
    holder.dylib_gen = itertools.count()  # type: ignore[attr-defined]
    # Guards dylib/dylib_mtime: ctypes releases the GIL during foreign calls,
    # so two Hermes threads can race through _load_dylib during a reload
    # window (F12) - worst case one thread frees a string through a
    # half-swapped reference, compounding the split-brain hazard below (F4).
    holder.lock = threading.Lock()  # type: ignore[attr-defined]
    holder.atexit_registered = False  # type: ignore[attr-defined]
    try:
        # dict.setdefault is atomic under the GIL, so two shim copies
        # importing concurrently still converge on a single holder.
        return sys.modules.setdefault(_STATE_MODULE_NAME, holder)
    except Exception as e:
        # Defensive: never crash registration on a poisoned sys.modules -
        # fall back to a private holder (each shim copy then loads its own
        # image; dlopen memoizes by path so handles still converge).
        _log.warning(
            "_process_state: sys.modules unavailable (%s); using a private holder", e
        )
        return holder


_state = _process_state()


def _data_dir() -> Path:
    """Plugin data directory (hotreload cache, proxy-stderr.log).

    Defaults to ``~/.hermes/aphrodite``; ``APHRODITE_HOME`` overrides it.
    Only the Python side honours the override - the Rust proxy/dylib keep
    resolving ``aphrodite.toml`` and ``ccr.db`` on their own.
    """
    try:
        override = os.environ.get("APHRODITE_HOME")
        if override:
            return Path(override).expanduser()
        return Path.home() / ".hermes" / "aphrodite"
    except Exception as e:
        _log.warning("_data_dir: %s; falling back to ~/.hermes/aphrodite", e)
        return Path.home() / ".hermes" / "aphrodite"


def _hotreload_dir() -> str:
    """Location for hot-reload dylib copies.

    Lives under the Aphrodite namespace in the OS user directory
    (~/.hermes/aphrodite/hotreload) rather than inside the plugin source
    tree. This keeps the copies out of the plugin checkout so `hermes
    plugins doctor` never stages/copies them into tmpfs (which previously
    caused ENOSPC failures) and so they don't accumulate in version
    control or released artifacts.
    """
    d = _data_dir() / "hotreload"
    d.mkdir(parents=True, exist_ok=True)
    return str(d)


def _pid_alive(pid: int) -> bool:
    """Best-effort "is this PID still running?" check, cross-platform.

    Windows MUST NOT use ``os.kill(pid, 0)``: CPython's ``os.kill`` on Windows
    is ``TerminateProcess`` for any signal other than ``CTRL_C_EVENT`` /
    ``CTRL_BREAK_EVENT``, and ``0`` is neither - so "probe" *kills* the target.
    This reaper runs in every process that loads the plugin (CLI runs, kanban
    workers), and the tombstones it walks are named after other live Hermes
    processes; the gateway was being terminated on every worker spawn.
    """
    if pid <= 0:
        return False
    # Linux: /proc/<pid> exists iff the process is alive.
    if os.path.isdir(f"/proc/{pid}"):
        return True
    if sys.platform == "win32":
        try:
            import ctypes.wintypes as wt

            SYNCHRONIZE = 0x00100000  # noqa: N806 - Win32 API constant
            PROCESS_QUERY_LIMITED_INFORMATION = 0x1000  # noqa: N806 - Win32 API constant
            STILL_ACTIVE = 259  # noqa: N806 - Win32 API constant
            ERROR_ACCESS_DENIED = 5  # noqa: N806 - Win32 API constant
            k32 = ctypes.windll.kernel32
            k32.OpenProcess.argtypes = [wt.DWORD, wt.BOOL, wt.DWORD]
            k32.OpenProcess.restype = wt.HANDLE
            k32.GetExitCodeProcess.argtypes = [wt.HANDLE, ctypes.POINTER(wt.DWORD)]
            k32.GetExitCodeProcess.restype = wt.BOOL
            k32.CloseHandle.argtypes = [wt.HANDLE]
            k32.CloseHandle.restype = wt.BOOL
            h = k32.OpenProcess(
                SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, False, pid
            )
            if not h:
                # NULL handle: process not found (or access denied).
                # ERROR_ACCESS_DENIED: exists but isn't ours - treat as alive
                # (don't reap it).
                return k32.GetLastError() == ERROR_ACCESS_DENIED
            try:
                code = wt.DWORD()
                if k32.GetExitCodeProcess(h, ctypes.byref(code)):
                    return code.value == STILL_ACTIVE
                return True
            finally:
                k32.CloseHandle(h)
        except Exception:
            # Defensive: never reap what we cannot probe. NEVER fall back to
            # os.kill on Windows (it is TerminateProcess and kills the target).
            _log.warning(
                "_pid_alive: win32 probe failed for pid %s; treating as alive", pid
            )
            return True
    # macOS/BSD: signal 0 probes existence without side effects.
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        # PID exists but isn't ours - treat as alive (don't reap it).
        return True
    except OSError:
        return False


def _reap_stale_hotreloads() -> None:
    """Delete hot-reload copies left behind by processes that are no longer
    running, and trim generations for live PIDs to the most recent few.

    A copy filename is `<base>.<pid>.<gen>` (e.g.
    `libaphrodite_hermes.dylib.4242.3`). Copies whose PID is dead are
    unconditionally removed; for each live PID we keep only the newest
    generation so a long-lived process can't grow unbounded either.
    """
    try:
        d = _hotreload_dir()
    except Exception:
        return
    by_pid: dict[int, list[tuple[int, str]]] = {}
    prefix = os.path.basename(_DYLIB_PATH)
    try:
        entries = os.listdir(d)
    except OSError:
        return
    for name in entries:
        if not name.startswith(prefix + "."):
            continue
        # <prefix>.<pid>.<gen>
        rest = name[len(prefix) + 1 :]
        parts = rest.split(".")
        if len(parts) != 2:
            continue
        try:
            pid = int(parts[0])
            gen = int(parts[1])
        except ValueError:
            continue
        by_pid.setdefault(pid, []).append((gen, os.path.join(d, name)))
    for pid, gens in by_pid.items():
        if not _pid_alive(pid):
            # Process is gone - every generation it left is garbage.
            for _, path in gens:
                with contextlib.suppress(OSError):
                    os.remove(path)
        else:
            # Still alive: keep only the newest generation for that PID.
            gens.sort(reverse=True)
            for _, path in gens[1:]:
                with contextlib.suppress(OSError):
                    os.remove(path)


def _load_fresh_copy(src_path: str) -> str:
    """Copy `src_path` to a uniquely-named file so a subsequent
    `ctypes.CDLL()` genuinely loads the new bytes instead of a cached image.

    dlopen (both macOS dyld and Linux glibc) memoizes loaded images by
    canonical path and hands back the SAME cached handle on a repeat dlopen
    of the same path - even when the file's mtime/content on disk changed,
    and even though nothing here ever calls dlclose. This silently defeated
    the mtime check below for every reload after the first: the check fired
    and logged a "hot-reloading" warning, `ctypes.CDLL(path)` ran again, but
    the OS just returned the original in-memory image untouched. Verified
    empirically: a rebuilt dylib with a changed embedded constant, re-opened
    via `ctypes.CDLL()` on the same live process, kept returning the OLD
    constant and the identical `_handle` value. Loading each generation from
    a fresh path sidesteps the path-keyed cache entirely.

    Copies are written to the relocated hotreload cache
    (~/.hermes/aphrodite/hotreload), NOT inside the plugin tree, and are
    named `<base>.<pid>.<gen>` so stale copies from terminated processes can
    be reaped by `_reap_stale_hotreloads`.
    """
    hotreload_dir = _hotreload_dir()
    # Reap first so we don't pile generations on top of dead processes'
    # leftovers, and so a fresh start can't grow unbounded.
    _reap_stale_hotreloads()
    dst = os.path.join(
        hotreload_dir, f"{os.path.basename(src_path)}.{os.getpid()}.{next(_state.dylib_gen)}"
    )
    shutil.copy2(src_path, dst)
    return dst


def _dylib_candidates(plugin_dir: Path) -> list[str]:
    """Ordered dylib candidates, env override first, parent-depth guarded.

    Canonical runtime home (~/.hermes/aphrodite/binaries) comes first; the
    plugin-dir binaries/ paths are legacy fallbacks so a pre-relocation
    install still loads. Shallow installs (e.g. /opt/Aphrodite-Hermes) have
    fewer than 4 parents; the monorepo target/release fallbacks simply do
    not exist there, so indexing is guarded instead of crashing (issue 5).
    """
    plugin_dir = Path(plugin_dir).resolve()
    canonical = str(_BINARIES_DIR / _DYLIB_NAME)
    candidates = [
        _DYLIB_PATH,  # APHRODITE_HERMES_DYLIB_PATH override (or canonical default) - wins when it exists
        canonical,  # canonical runtime home (dedup no-op when the env var is unset)
        str(plugin_dir / "binaries" / _DYLIB_NAME),  # legacy fallback (old installs)
        str(plugin_dir.parent / "binaries" / _DYLIB_NAME),
    ]
    parents = plugin_dir.parents
    for depth in (2, 3):
        if depth < len(parents):
            candidates.append(str(parents[depth] / "target" / "release" / _DYLIB_NAME))
    return candidates


def _resolve_binary_path() -> str:
    """Proxy binary path: env override first, then canonical runtime home,
    then the legacy plugin-dir copy as a warning-flagged fallback.

    APHRODITE_BINARY_PATH wins when it exists. Otherwise prefer
    ~/.hermes/aphrodite/binaries; a pre-relocation install may only have
    the plugin-dir binaries/ copy, which we keep loading (with a warning)
    so an old install still works. Returns the last candidate (missing)
    when nothing exists - callers log the final miss.
    """
    if os.path.exists(_BINARY_PATH):
        return _BINARY_PATH
    canonical = str(_BINARIES_DIR / _BINARY_NAME)
    if os.path.exists(canonical):
        return canonical
    legacy = str(_PLUGIN_DIR / "binaries" / _BINARY_NAME)
    if os.path.exists(legacy):
        _log.warning(
            "canonical binary %s not found - using legacy plugin-dir copy "
            "%s (old install; run download.sh to migrate)",
            canonical,
            legacy,
        )
        return legacy
    return _BINARY_PATH


def _load_dylib() -> ctypes.CDLL:
    """Load libaphrodite_hermes.dylib with ctypes. Hot-reloads on mtime change.

    State lives in the process-global holder (see `_process_state`), so a
    second exec of this shim in the same process (one per Hermes home)
    takes the mtime early-return below and reuses the mapped handle.
    """
    with _state.lock:
        # Find current dylib path: env override first, then the canonical
        # runtime home, then legacy plugin-dir copies.
        path = _DYLIB_PATH
        candidates = _dylib_candidates(_PLUGIN_DIR)
        for p in candidates:
            if os.path.exists(p):
                path = p
                break
        # Auto-fetch the dylib on first use if it's missing (download.sh
        # verifies SHA-256 and writes ~/.hermes/aphrodite/binaries/).
        # No-op when present.
        _ensure_binaries()
        if not os.path.exists(path):
            # download.sh may have just populated the canonical home -
            # re-resolve so the freshly fetched copy wins over legacy.
            for p in candidates:
                if os.path.exists(p):
                    path = p
                    break
        # Defensive: prefer the canonical runtime home. A legacy
        # plugin-dir/target hit means the canonical copy is missing - warn
        # but keep loading, never crash registration over a path preference.
        # Silence when the env override or the canonical path itself won.
        if path != _DYLIB_PATH and path != str(_BINARIES_DIR / _DYLIB_NAME):
            _log.warning(
                "canonical dylib %s not found - using %s instead (legacy "
                "install; run download.sh to migrate to "
                "~/.hermes/aphrodite/binaries)",
                _BINARIES_DIR / _DYLIB_NAME,
                path,
            )
        assert os.path.exists(path), f"Dylib not found. Tried: {candidates}"

        # Hot-reload: check mtime, reload if changed
        current_mtime = os.path.getmtime(path)
        if _state.dylib is not None and current_mtime == _state.dylib_mtime:
            return _state.dylib

        if _state.dylib is not None:
            # Reloading mid-session discards ALL prior compressions: the
            # Rust side keeps its session state in a per-image OnceLock, so
            # every existing <<<CCR:...>>> marker already in the transcript
            # becomes an unresolvable dead reference the moment the new
            # image loads (F4) - this is not a cosmetic reload, it silently
            # wipes session state.
            _log.warning(
                "dylib mtime changed (%.2f -> %.2f) - hot-reloading %s; "
                "this resets ALL session CCR state - existing markers in "
                "the transcript will no longer resolve via aphrodite_retrieve",
                _state.dylib_mtime,
                current_mtime,
                path,
            )

        # Load from a fresh unique-path copy, not `path` directly - see
        # `_load_fresh_copy`'s docstring for why a repeat dlopen of the same
        # path silently returns stale, cached code on every platform.
        load_path = _load_fresh_copy(path)
        dylib = ctypes.CDLL(load_path)

        # The previous generation's copy is no longer needed - its mapped
        # pages stay valid for any in-flight call even after the directory
        # entry is removed (standard POSIX unlink-while-mapped semantics),
        # so deleting it here is safe and keeps this process's own copies
        # from growing unboundedly across a long dev session.
        if _state.dylib_copy_path is not None:
            with contextlib.suppress(OSError):
                os.remove(_state.dylib_copy_path)

        # Register a one-shot shutdown sweep for THIS process's own copy.
        # Copies from terminated processes are reaped at startup via
        # `_reap_stale_hotreloads`; this guarantees our own final
        # generation is removed on a clean exit too. Idempotent.
        _register_atexit_cleanup()

        try:
            # c_void_p avoids Python 3.14 c_char_p malloc mismatch → SIGABRT
            dylib.aphrodite_hermes_get_schemas.restype = ctypes.c_void_p
            dylib.aphrodite_hermes_get_hooks.restype = ctypes.c_void_p
            dylib.aphrodite_hermes_list_skills.restype = ctypes.c_void_p
            dylib.aphrodite_hermes_dispatch_tool.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
            dylib.aphrodite_hermes_dispatch_tool.restype = ctypes.c_void_p
            dylib.aphrodite_hermes_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
            dylib.aphrodite_hermes_call_hook.restype = ctypes.c_void_p
            dylib.aphrodite_hermes_proxy_health.restype = ctypes.c_void_p
            dylib.aphrodite_hermes_version.restype = ctypes.c_void_p
            dylib.aphrodite_hermes_free_string.argtypes = [ctypes.c_void_p]
        except AttributeError as e:
            # A stale/mismatched dylib (see _check_version) surfacing as a
            # raw AttributeError deep in ctypes gives no context on which
            # binary or which symbol - name both here.
            raise RuntimeError(
                f"dylib at {path} is missing an expected symbol ({e}) - "
                f"it may be built from a different aphrodite-hermes version "
                f"than this plugin expects"
            ) from e

        _state.dylib = dylib  # pyright: ignore[reportAttributeAccessIssue]
        _state.dylib_mtime = current_mtime  # pyright: ignore[reportAttributeAccessIssue]
        _state.dylib_copy_path = load_path  # pyright: ignore[reportAttributeAccessIssue]
        return dylib


def _read_str(ptr: int | None) -> str | None:
    """Read a null-terminated C string from a void pointer."""
    if ptr is None or ptr == 0:
        return None
    value = ctypes.cast(ptr, ctypes.c_char_p).value
    return value.decode("utf-8") if value else None


def _call_json(dylib: ctypes.CDLL, fn_name: str, *args: bytes) -> Any:
    """Call C function by name, decode JSON, free through the SAME dylib
    object that produced the pointer (F4: allocating and freeing through
    different hot-reloaded images is only safe by accident today - benign
    while both use the system allocator, undefined behavior the day a
    custom global allocator is added)."""
    fn = getattr(dylib, fn_name)
    ptr = fn(*args)
    result = _read_str(ptr)
    if ptr:
        dylib.aphrodite_hermes_free_string(ptr)
    return json.loads(result) if result else None


def _make_handler(tool_name: str) -> Callable[..., str]:
    """Create tool handler that dispatches via dylib.

    Resolves the dylib fresh on every call (not captured at registration
    time) so tool calls always use the current image - see the hook
    registration loop below for why hooks need the same treatment.
    """

    def handler(args: dict[str, Any] | None = None, **kwargs: Any) -> str:
        args_json = json.dumps(args or {})
        dylib = _load_dylib()
        return json.dumps(
            _call_json(
                dylib,
                "aphrodite_hermes_dispatch_tool",
                tool_name.encode("utf-8"),
                args_json.encode("utf-8"),
            )
        )

    return handler


def _check_version(dylib: ctypes.CDLL) -> None:
    """Warn (never raise) if the loaded dylib's version disagrees with
    BINARY_VERSION - the plugin's own pin of what it was built/shipped
    against (F6). The dylib is fetched separately (download.sh, keyed off
    BINARY_VERSION) and hot-reloaded by mtime; a stale or newer dylib with a
    changed JSON contract should fail loudly at registration, not misbehave
    silently at runtime."""
    try:
        loaded = _call_json(dylib, "aphrodite_hermes_version")
        loaded_version = (loaded or {}).get("version") if isinstance(loaded, dict) else None
        expected_path = _PLUGIN_DIR / "BINARY_VERSION"
        expected_version = expected_path.read_text().strip() if expected_path.exists() else None
        if loaded_version and expected_version and loaded_version != expected_version:
            _log.warning(
                "aphrodite-hermes dylib version mismatch: loaded %s, "
                "BINARY_VERSION expects %s - the JSON contract (hook/tool "
                "schemas) may have changed between these versions",
                loaded_version,
                expected_version,
            )
    except Exception as e:
        # Version check is best-effort diagnostics, never a hard requirement.
        _log.debug("version handshake skipped: %s", e)


def _env_bool(var: str) -> bool:
    """One consistent truthiness rule for boolean env vars (F12): `"1"`/
    `"true"` (case-insensitive) is true, anything else (including unset) is
    false. Mirrors `aphrodite::config::env_bool` on the Rust side - this
    plugin previously checked `== "1"` only, while the Rust proxy's
    APHRODITE_LOG_COMPACT accepted any value (even "0") via presence-only
    `.is_ok()`, and the dead `config_loader.rs` accepted `"true"` OR `"1"`."""
    return os.environ.get(var, "").lower() in ("1", "true")


def _parse_port_env(var: str, default: int) -> int:
    """Parse a port env var, falling back to `default` when unset OR
    malformed (F11) - this used to be a bare `int()` call, so e.g.
    APHRODITE_CACHE_PORT=abc raised uncaught and aborted plugin
    registration entirely. The proxy itself would still start (the Rust
    side warns-and-falls-back on the same var), so the plugin's own
    tools/hooks failing to register was a worse outcome than falling back
    to the same default the proxy already chose."""
    raw = os.environ.get(var)
    if raw is None:
        return default
    try:
        return int(raw)
    except ValueError:
        _log.warning("%s=%r is not a valid port; falling back to %d", var, raw, default)
        return default


def _tail_log(path: Path, n: int = 15) -> str:
    """Last `n` lines of a log file; "(no stderr log yet)" when unreadable."""
    with contextlib.suppress(OSError):
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        return "\n".join(lines[-n:])
    return "(no stderr log yet)"


def _ensure_binaries() -> None:
    """Fetch the proxy binary + dylib via download.sh when either is missing.

    README.md has always promised binaries are fetched automatically; this
    makes that promise real. Returns immediately when both files exist, so
    repeated register() calls never re-download. APHRODITE_NO_AUTO_DOWNLOAD
    (1/true, like _env_bool) opts out for offline/dev-loop setups.

    download.sh's BINARY_DIR defaults to ~/.hermes/aphrodite/binaries; we
    also pass BINARY_DIR explicitly so an ambient BINARY_DIR in the caller's
    environment can never redirect the fetch back into the plugin tree.
    """
    if _env_bool("APHRODITE_NO_AUTO_DOWNLOAD"):
        return
    if os.path.exists(_BINARY_PATH) and os.path.exists(_DYLIB_PATH):
        return
    try:
        env = os.environ.copy()
        env["BINARY_DIR"] = str(_BINARIES_DIR)
        result = subprocess.run(
            ["bash", str(_PLUGIN_DIR / "download.sh")],
            timeout=180,
            capture_output=True,
            text=True,
            errors="replace",
            env=env,
        )
    except Exception as e:
        _log.warning(
            "failed to run %s (%s) - run download.sh manually to fetch the "
            "aphrodite binaries",
            _PLUGIN_DIR / "download.sh",
            e,
        )
        return
    if result.returncode != 0:
        tail = "\n".join(
            ((result.stdout or "") + (result.stderr or "")).splitlines()[-15:]
        )
        _log.warning(
            "download.sh exited %d - run download.sh manually to fetch the "
            "aphrodite binaries; output tail:\n%s",
            result.returncode,
            tail,
        )


_health_opener: Any = None


def _proxy_healthy(port: int) -> bool:
    """One direct GET /health against a local proxy port; True only on a
    CONFIRMED aphrodite answer.

    MEDIUM (PR#8 review): the decisional pre-launch probe must not route
    through the ambient HTTP(S)_PROXY env vars - a corporate proxy
    answering 200 on the health path would make the probe falsely report
    healthy and skip launching the real proxy, silently killing CCR. An
    opener built with ProxyHandler({}) (empty proxy map) forces direct
    connections only. And the bare status code is not enough either: the
    response body must be JSON with ``status == "healthy"`` (the Rust
    health endpoint always answers 200 with that field), so an unrelated
    service squatting on the port cannot cause a false skip. Any error,
    timeout, or foreign body is unhealthy - the probe can only skip a
    launch on a confirmed aphrodite answer.
    """
    import urllib.request

    global _health_opener
    if _health_opener is None:
        try:
            _health_opener = urllib.request.build_opener(
                urllib.request.ProxyHandler({})
            )
        except Exception as e:
            # Defensive: never skip on an opener failure - report and treat
            # as unhealthy so the launch still proceeds.
            _log.warning(
                "_proxy_healthy: cannot build direct opener (%s); treating as unhealthy",
                e,
            )
            return False
    try:
        req = urllib.request.Request(f"http://127.0.0.1:{port}/health", method="GET")
        with _health_opener.open(req, timeout=0.5) as resp:
            if resp.status != 200:
                return False
            body = resp.read(4096).decode("utf-8", errors="replace")
            try:
                payload = json.loads(body)
            except Exception:
                return False
            return isinstance(payload, dict) and payload.get("status") == "healthy"
    except Exception:
        return False


def _start_proxy():
    """Start the aphrodite proxy binary and verify both proxies are healthy.

    Probes both health endpoints BEFORE launching: every Hermes process
    (CLI runs, kanban workers, each profile's PluginManager) calls this on
    registration, but only one proxy can own the ports. Launching
    unconditionally made each extra process spawn a binary that immediately
    died on `failed to bind listener` (os error 10048 on Windows), appending
    that plus a `no API key configured` line to proxy-stderr.log every time.
    If both proxies already answer a CONFIRMED aphrodite health body (see
    `_proxy_healthy`), the launch is skipped at INFO level.

    Pipes stderr to a log file (not DEVNULL) so startup errors are
    diagnosable.  After launch, detects immediate death (stderr tail +
    API-key hint) and polls both proxy health endpoints for up to 5
    seconds, logging a warning for each one that doesn't come up.
    """
    import time

    # F13: README.md and this function's own env.setdefault() below have
    # advertised this guard since it was added, but nothing ever READ it -
    # `cargo watch`/dev-loop instructions telling developers to set
    # APHRODITE_NO_AUTO_LAUNCH=1 to stop the plugin from fighting them for
    # the ports had no effect at all.
    if os.environ.get("APHRODITE_NO_AUTO_LAUNCH", "0") in ("1", "true"):
        _log.info("APHRODITE_NO_AUTO_LAUNCH set - skipping proxy auto-launch")
        return

    # Fetch the proxy binary + dylib on first use if either is missing
    # (download.sh verifies SHA-256 and writes ~/.hermes/aphrodite/binaries/).
    # No-op when both already exist - repeated register() calls don't
    # re-download.
    _ensure_binaries()

    # Read custom ports from env vars (matching the Rust dylib's
    # configured_ports() in aphrodite-hermes/src/lib.rs).
    _cache_port = _parse_port_env("APHRODITE_CACHE_PORT", 9797)
    _token_port = _parse_port_env("APHRODITE_TOKEN_PORT", 9798)
    proxies = [
        ("cache", _cache_port),
        ("token", _token_port),
    ]

    # ── Pre-launch probe: skip launch iff another process already owns
    # BOTH proxies with a confirmed aphrodite answer (never on error,
    # never on a foreign body - see _proxy_healthy) ──
    up: set[str] = {name for name, port in proxies if _proxy_healthy(port)}
    if len(up) == len(proxies):
        _log.info(
            "aphrodite proxies already healthy on :%d/:%d - reusing the running "
            "instance, skipping launch",
            _cache_port,
            _token_port,
        )
        return

    binary = _resolve_binary_path()
    if not os.path.exists(binary):
        _log.warning("aphrodite binary not found at %s", binary)
        return
    if not os.access(binary, os.X_OK):
        os.chmod(binary, 0o755)
    env = os.environ.copy()
    env.setdefault("APHRODITE_NO_AUTO_LAUNCH", "0")

    # Write stderr to a log file so startup errors (e.g. SQLite "unable
    # to open database file") are visible.  Previously stderr was piped
    # to DEVNULL, making every startup failure silent.
    log_dir = _data_dir()
    try:
        log_dir.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        # Defensive: a weird/unwritable APHRODITE_HOME must never crash
        # registration - fall back to the default location, then give up
        # with a warning rather than raising.
        _log.warning(
            "aphrodite data dir %s unusable (%s); falling back to "
            "~/.hermes/aphrodite",
            log_dir,
            e,
        )
        log_dir = Path.home() / ".hermes" / "aphrodite"
        try:
            log_dir.mkdir(parents=True, exist_ok=True)
        except OSError as e2:
            _log.warning(
                "cannot create %s either (%s) - skipping proxy launch", log_dir, e2
            )
            return
    try:
        with open(log_dir / "proxy-stderr.log", "a") as stderr_log:
            proc = subprocess.Popen(
                [binary],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=stderr_log,
                cwd=os.getcwd(),
            )
    except Exception as e:
        _log.warning("failed to start aphrodite proxy: %s", e)
        return

    # Detect immediate death: a proxy binary that dies on launch (bad
    # config, missing API key, port conflict) used to be silently invisible
    # - the health check below would only log "did not become healthy" with
    # no clue why. Surface the stderr tail right away instead. A single
    # immediate poll() races the child's exec, so give a fast-exiting
    # binary a brief grace period before declaring it started.
    rc = proc.poll()
    if rc is None:
        with contextlib.suppress(subprocess.TimeoutExpired):
            rc = proc.wait(timeout=0.25)
    if rc is not None:
        _log.warning(
            "aphrodite proxy exited immediately (rc=%s); last stderr:", rc
        )
        tail = _tail_log(log_dir / "proxy-stderr.log")
        _log.warning("%s", tail)
        if "API key" in tail:
            _log.warning(
                "set APHRODITE_API_KEY env var, run `aphrodite setup`, or "
                "add api_key to the TOML at ~/.hermes/aphrodite/aphrodite.toml"
            )
        return

    _log.info("aphrodite proxy started (%s)", binary)

    # ── Health check: poll both proxies for up to 5 seconds ──────
    deadline = time.monotonic() + 5.0
    up = set()
    while time.monotonic() < deadline:
        for name, port in proxies:
            if name in up:
                continue
            if _proxy_healthy(port):
                up.add(name)
                _log.info("aphrodite %s proxy healthy on :%d", name, port)
        if len(up) == len(proxies):
            break
        time.sleep(0.5)

    for name, port in proxies:
        if name not in up:
            if proc.poll() is not None:
                _log.warning(
                    "aphrodite proxy process exited early (rc=%s) while "
                    "waiting for %s on :%d; last stderr:",
                    proc.poll(),
                    name,
                    port,
                )
                tail = _tail_log(log_dir / "proxy-stderr.log")
                _log.warning("%s", tail)
                if "API key" in tail:
                    _log.warning(
                        "set APHRODITE_API_KEY env var, run `aphrodite setup`, "
                        "or add api_key to the TOML at "
                        "~/.hermes/aphrodite/aphrodite.toml"
                    )
            else:
                _log.warning(
                    "aphrodite %s proxy on :%d did not become healthy within 5s "
                    "- check %s for errors",
                    name,
                    port,
                    log_dir / "proxy-stderr.log",
                )


# ── Plugin registration ──


def _register_atexit_cleanup() -> None:
    """Register a one-shot atexit handler that removes this process's own
    hot-reload copy on interpreter shutdown, and reaps any copies left by
    processes that have since died. Idempotent across shim copies (the
    flag lives in the process-global holder)."""
    if _state.atexit_registered:
        return
    _state.atexit_registered = True  # pyright: ignore[reportAttributeAccessIssue]
    import atexit

    def _cleanup() -> None:
        # Remove our own final-generation copy.
        if _state.dylib_copy_path is not None:
            with contextlib.suppress(OSError):
                os.remove(_state.dylib_copy_path)
        # And sweep up anything abandoned by dead processes.
        _reap_stale_hotreloads()

    atexit.register(_cleanup)


# Startup sweep: reclaim hot-reload copies abandoned by processes that died
# before they could clean up (e.g. crashed/terminated Hermes sessions). This
# replaces the old in-tree `.hotreload/` (which grew to ~19 GB across many
# terminated processes) with a bounded, reaped cache.
with contextlib.suppress(Exception):
    _reap_stale_hotreloads()


def register(ctx: Any) -> None:
    """Register hooks, tools, and (optionally) a context engine with Hermes.

    Targets the Hermes v0.17.0 PluginContext API:
      register_hook(hook_name, callback)
      register_tool(name, toolset, schema, handler, ...)
      register_skill(name, path: Path, description="")
      register_context_engine(engine)   # engine must subclass ContextEngine
    Each registration is isolated so one failure never aborts the whole plugin.
    """
    try:
        dylib = _load_dylib()
    except Exception as e:
        # Defensive (issue triage): a missing/unloadable dylib (auto-download
        # failed, wrong architecture, corrupted binary) must disable the
        # plugin with a clear log - never propagate and abort Hermes' plugin
        # loading. The download.sh + APHRODITE_NO_AUTO_DOWNLOAD story above
        # covers the recovery path; without the dylib there is nothing to
        # register, so we log and return.
        _log.error(
            "aphrodite-hermes dylib could not be loaded (%s) - plugin disabled; "
            "run download.sh (or unset APHRODITE_NO_AUTO_DOWNLOAD) to fetch "
            "the binaries, then restart Hermes",
            e,
        )
        return
    _log.info("aphrodite-hermes dylib loaded: %s", _DYLIB_PATH)
    _check_version(dylib)

    # ── Layout self-heal (best-effort, never raises) ──
    # Ensure the ~/.hermes runtime layout matches the canonical schema before
    # anything consumes it: relocates misplaced config/binaries out of the
    # plugin dir and warns on anything ambiguous. Failures degrade to a
    # warning - the plugin still registers.
    try:
        from .layout_check import check_and_heal

        report = check_and_heal()
        for w in report.get("warnings", []):
            _log.warning("layout self-heal: %s", w)
    except Exception as e:
        _log.warning("layout self-heal skipped: %s", e)

    # ── Directives materialize (best-effort, never raises) ──
    # The binary provides the directives (embedded builtins); unpack them into
    # the user-data home ~/.hermes/aphrodite/directives/ so the plugin dir
    # never has to hold them. User-modified files are never overwritten.
    # home_dir=b"" -> the dylib falls back to its home resolution
    # (APHRODITE_DIRECTIVES_DIR / APHRODITE_HOME / ~/.hermes/aphrodite).
    try:
        result = _call_json(dylib, "aphrodite_hermes_materialize_directives", b"")
        if result:
            for w in result.get("warnings", []):
                _log.warning("directives materialize: %s", w)
    except Exception as e:
        _log.warning("directives materialize skipped: %s", e)

    # Register hooks - dispatch to Rust dylib via aphrodite_hermes_call_hook
    hooks = _call_json(dylib, "aphrodite_hermes_get_hooks")
    if hooks:

        def _hook_dispatch(hook_name: str, **kwargs: Any) -> Any:
            """Dispatch hook to Rust dylib and return parsed result.

            Resolves the dylib fresh on every call instead of closing over
            the `dylib` captured above (F4): previously hooks stayed pinned
            to whichever image was loaded at registration time forever,
            while tool handlers already re-resolved per call - so after a
            hot-reload, a marker emitted by a hook (old image) became
            unresolvable by aphrodite_retrieve (new image, fresh state).
            """
            # Hermes passes hook args as kwargs (result, output, tool_name, ...).
            # default=str keeps any non-JSON-serializable extras (e.g. message
            # objects on pre/post_llm_call) from crashing the hook.
            args_json = json.dumps(kwargs, default=str)
            return _call_json(
                _load_dylib(),
                "aphrodite_hermes_call_hook",
                hook_name.encode("utf-8"),
                args_json.encode("utf-8"),
            )

        for hook_name in hooks:

            def _dispatch(*a: Any, name: str = hook_name, **kw: Any) -> Any:
                return _hook_dispatch(name, **kw)

            ctx.register_hook(hook_name, _dispatch)
        _log.info("registered %d hooks", len(hooks))

    # Register tools. Hermes API: register_tool(name, toolset, schema, handler).
    schemas = _call_json(dylib, "aphrodite_hermes_get_schemas")
    if schemas:
        registered: list[str] = []
        for schema in schemas:
            name = schema["name"]
            try:
                ctx.register_tool(name, "aphrodite", schema, _make_handler(name))
                registered.append(name)
            except Exception as e:
                _log.warning("failed to register tool %s: %s", name, e)
        _log.info("registered %d tools: %s", len(registered), registered)

    # Skills are NOT shipped with the plugin - they live dev-side in the
    # monorepo's .hermes/skills/ (Development branch only, never on Current).
    # The plugin registers tools and hooks only.

    # Context engine is opt-in (APHRODITE_CONTEXT_ENGINE=1). Hermes expects a
    # ContextEngine subclass instance here; the per-turn catalog summary is
    # already injected via the pre_llm_call hook above, so the default path
    # needs no engine. Registering anything other than a ContextEngine instance
    # is silently rejected by Hermes, so we only attempt it when asked.
    if _env_bool("APHRODITE_CONTEXT_ENGINE"):
        try:
            _register_context_engine(ctx, dylib)
        except Exception as e:
            _log.warning(
                "context engine opt-in requested but not registered (%s); "
                "falling back to hooks + proxy",
                e,
            )

    _start_proxy()


def _register_context_engine(ctx: Any, dylib: ctypes.CDLL) -> None:
    """Best-effort context-engine registration (opt-in).

    Builds a thin ContextEngine subclass whose pre-flight summary comes from the
    dylib catalog. Raises if the host Hermes does not expose ContextEngine, so
    the caller can fall back to the hook + proxy path.
    """
    # Resolved dynamically: `agent.context_engine` only exists inside the Hermes
    # runtime, so a static import would break standalone lint/type checks.
    import importlib

    context_engine_cls = importlib.import_module("agent.context_engine").ContextEngine

    class AphroditeContextEngine(context_engine_cls):
        @property
        def name(self) -> str:
            return "aphrodite"

        def update_from_response(self, usage: dict[str, Any]) -> None:
            self.last_prompt_tokens = usage.get("prompt_tokens", 0)
            self.last_completion_tokens = usage.get("completion_tokens", 0)
            self.last_total_tokens = usage.get("total_tokens", 0)

        def should_compress(self, prompt_tokens: int | None = None) -> bool:
            # Defer to Hermes' own threshold accounting; the proxy + hooks do the
            # heavy lifting, so the engine itself never forces a compaction.
            return False

        def compress(
            self,
            messages: list[Any],
            current_tokens: int | None = None,
            focus_topic: str | None = None,
        ) -> list[Any]:
            # Non-destructive: the proxy and transform hooks already shrink tool
            # output, so the engine returns the transcript unchanged.
            return messages

    ctx.register_context_engine(AphroditeContextEngine())
