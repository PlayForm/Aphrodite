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
_DYLIB_PATH = os.environ.get(
    "APHRODITE_HERMES_DYLIB_PATH", str(_PLUGIN_DIR / "binaries" / _DYLIB_NAME)
)
_BINARY_NAME = "aphrodite.exe" if sys.platform == "win32" else "aphrodite"
_BINARY_PATH = os.environ.get("APHRODITE_BINARY_PATH", str(_PLUGIN_DIR / "binaries" / _BINARY_NAME))

_dylib: ctypes.CDLL | None = None
_dylib_mtime: float = 0.0
_dylib_copy_path: str | None = None
_dylib_gen = itertools.count()
# Guards _dylib/_dylib_mtime: ctypes releases the GIL during foreign calls, so
# two Hermes threads can race through _load_dylib during a reload window
# (F12) - worst case one thread frees a string through a half-swapped
# reference, compounding the split-brain hazard below (F4).
_dylib_lock = threading.Lock()


def _hotreload_dir() -> str:
    """Location for hot-reload dylib copies.

    Lives under the Aphrodite namespace in the OS user directory
    (~/.hermes/aphrodite/hotreload) rather than inside the plugin source
    tree. This keeps the copies out of the plugin checkout so `hermes
    plugins doctor` never stages/copies them into tmpfs (which previously
    caused ENOSPC failures) and so they don't accumulate in version
    control or released artifacts.
    """
    d = Path.home() / ".hermes" / "aphrodite" / "hotreload"
    d.mkdir(parents=True, exist_ok=True)
    return str(d)


def _pid_alive(pid: int) -> bool:
    """Best-effort "is this PID still running?" check, cross-platform."""
    if pid <= 0:
        return False
    # Linux: /proc/<pid> exists iff the process is alive.
    if os.path.isdir(f"/proc/{pid}"):
        return True
    # macOS/BSD/Windows: signal 0 probes existence without side effects.
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
        hotreload_dir, f"{os.path.basename(src_path)}.{os.getpid()}.{next(_dylib_gen)}"
    )
    shutil.copy2(src_path, dst)
    return dst


def _load_dylib() -> ctypes.CDLL:
    """Load libaphrodite_hermes.dylib with ctypes. Hot-reloads on mtime change."""
    global _dylib, _dylib_mtime, _dylib_copy_path

    with _dylib_lock:
        # Find current dylib path
        path = _DYLIB_PATH
        candidates = [
            path,
            str(_PLUGIN_DIR / "binaries" / _DYLIB_NAME),
            str(_PLUGIN_DIR.parent / "binaries" / _DYLIB_NAME),
        ]
        # Monorepo dev-build fallback: for `<repo>/plugins/aphrodite/__init__.py`,
        # parents[2] is `<repo>` (where `target/release` actually lives) - not
        # darwin-specific (Linux dev builds want the .so equally), and
        # parents[3] covers a one-deeper nesting some checkouts use.
        for depth in (2, 3):
            candidates.append(
                str(Path(__file__).resolve().parents[depth] / "target" / "release" / _DYLIB_NAME)
            )
        for p in candidates:
            if os.path.exists(p):
                path = p
                break
        assert os.path.exists(path), f"Dylib not found. Tried: {candidates}"

        # Hot-reload: check mtime, reload if changed
        current_mtime = os.path.getmtime(path)
        if _dylib is not None and current_mtime == _dylib_mtime:
            return _dylib

        if _dylib is not None:
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
                _dylib_mtime,
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
        if _dylib_copy_path is not None:
            with contextlib.suppress(OSError):
                os.remove(_dylib_copy_path)

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

        _dylib = dylib
        _dylib_mtime = current_mtime
        _dylib_copy_path = load_path
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


def _start_proxy():
    """Start the aphrodite proxy binary and verify both proxies are healthy.

    Pipes stderr to a log file (not DEVNULL) so startup errors are
    diagnosable.  After launch, polls both proxy health endpoints for up
    to 5 seconds and logs a warning for each one that doesn't come up.
    """
    import time
    import urllib.request

    # F13: README.md and this function's own env.setdefault() below have
    # advertised this guard since it was added, but nothing ever READ it -
    # `cargo watch`/dev-loop instructions telling developers to set
    # APHRODITE_NO_AUTO_LAUNCH=1 to stop the plugin from fighting them for
    # the ports had no effect at all.
    if os.environ.get("APHRODITE_NO_AUTO_LAUNCH", "0") in ("1", "true"):
        _log.info("APHRODITE_NO_AUTO_LAUNCH set - skipping proxy auto-launch")
        return

    binary = _BINARY_PATH
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
    log_dir = Path.home() / ".hermes" / "aphrodite"
    log_dir.mkdir(parents=True, exist_ok=True)
    try:
        with open(log_dir / "proxy-stderr.log", "a") as stderr_log:
            subprocess.Popen(
                [binary],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=stderr_log,
                cwd=os.getcwd(),
            )
        _log.info("aphrodite proxy started (%s)", binary)
    except Exception as e:
        _log.warning("failed to start aphrodite proxy: %s", e)
        return

    # ── Health check: poll both proxies for up to 5 seconds ──────
    # Read custom ports from env vars (matching the Rust dylib's
    # configured_ports() in aphrodite-hermes/src/lib.rs).
    _cache_port = _parse_port_env("APHRODITE_CACHE_PORT", 9797)
    _token_port = _parse_port_env("APHRODITE_TOKEN_PORT", 9798)
    proxies = [
        ("cache", _cache_port),
        ("token", _token_port),
    ]
    deadline = time.monotonic() + 5.0
    up: set[str] = set()
    while time.monotonic() < deadline:
        for name, port in proxies:
            if name in up:
                continue
            try:
                req = urllib.request.Request(
                    f"http://127.0.0.1:{port}/health",
                    method="GET",
                )
                with urllib.request.urlopen(req, timeout=0.5) as resp:
                    if resp.status == 200:
                        up.add(name)
                        _log.info("aphrodite %s proxy healthy on :%d", name, port)
            except Exception:
                pass
        if len(up) == len(proxies):
            break
        time.sleep(0.5)

    for name, port in proxies:
        if name not in up:
            _log.warning(
                "aphrodite %s proxy on :%d did not become healthy within 5s "
                "- check ~/.hermes/aphrodite/proxy-stderr.log for errors",
                name,
                port,
            )


# ── Plugin registration ──

_atexit_registered = False


def _register_atexit_cleanup() -> None:
    """Register a one-shot atexit handler that removes this process's own
    hot-reload copy on interpreter shutdown, and reaps any copies left by
    processes that have since died. Idempotent."""
    global _atexit_registered
    if _atexit_registered:
        return
    _atexit_registered = True
    import atexit

    def _cleanup() -> None:
        # Remove our own final-generation copy.
        if _dylib_copy_path is not None:
            with contextlib.suppress(OSError):
                os.remove(_dylib_copy_path)
        # And sweep up anything abandoned by dead processes.
        _reap_stale_hotreloads()

    atexit.register(_cleanup)


# Startup sweep: reclaim hot-reload copies abandoned by processes that died
# before they could clean up (e.g. crashed/terminated Hermes sessions). This
# replaces the old in-tree `.hotreload/` (which grew to ~19 GB across many
# terminated processes) with a bounded, reaped cache.
try:
    _reap_stale_hotreloads()
except Exception:
    pass


def register(ctx: Any) -> None:
    """Register hooks, tools, and (optionally) a context engine with Hermes.

    Targets the Hermes v0.17.0 PluginContext API:
      register_hook(hook_name, callback)
      register_tool(name, toolset, schema, handler, ...)
      register_skill(name, path: Path, description="")
      register_context_engine(engine)   # engine must subclass ContextEngine
    Each registration is isolated so one failure never aborts the whole plugin.
    """
    dylib = _load_dylib()
    _log.info("aphrodite-hermes dylib loaded: %s", _DYLIB_PATH)
    _check_version(dylib)

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

    # Register skills - probe candidate layouts in order (Hermes wants a Path):
    # a `skills/` copied alongside this plugin (the standalone/`aphrodite setup`
    # install target), the monorepo layout (`<repo>/skills`, two levels up from
    # `<repo>/plugins/aphrodite`), and one level further for a deeper nesting.
    # Under a resolved symlink install (`~/.hermes/plugins/aphrodite` ->
    # `~/.hermes/aphrodite`), the old hardcoded `parent.parent.parent` guess
    # landed on `~/skills` (never exists) - 0 of the 9 advertised skills ever
    # registered outside a monorepo checkout, with only an info log to notice.
    _skills_dir_candidates = [
        _PLUGIN_DIR / "skills",
        _PLUGIN_DIR.parents[1] / "skills",
        _PLUGIN_DIR.parents[2] / "skills",
    ]
    _skills_dir = next((p for p in _skills_dir_candidates if p.is_dir()), None)
    if _skills_dir is None:
        _log.warning(
            "no skills/ directory found (tried %s) - 0 skills will register",
            _skills_dir_candidates,
        )
        _skills_dir = _skills_dir_candidates[0]
    skills = _call_json(dylib, "aphrodite_hermes_list_skills")
    if skills:
        count = 0
        for skill in skills:
            name = skill["name"]
            desc = skill.get("description", "")
            skill_path = _skills_dir / name / "SKILL.md"
            # Hermes skill identifiers must match [a-zA-Z0-9_-]+ (no dots), so
            # sanitize names like "aphrodite-v0.8.6-patterns" for registration
            # while still loading from the real on-disk directory.
            reg_name = "".join(c if (c.isalnum() or c in "_-") else "-" for c in name)
            if skill_path.exists():
                try:
                    ctx.register_skill(reg_name, skill_path, desc)
                    count += 1
                except Exception as e:
                    _log.warning("failed to register skill %s: %s", name, e)
        _log.info("registered %d skills from %s", count, _skills_dir)

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
