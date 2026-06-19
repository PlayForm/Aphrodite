"""aphrodite-hermes v1.63.0 — Dynamic dylib plugin for Hermes Agent.

This plugin is a THIN LOADER. All compression logic lives in the Rust cdylib
(headroom-ffi). This file:
  1. Finds and loads the dylib via ctypes
  2. Watches the dylib file for mtime changes (hot-reload)
  3. Discovers hooks from aphrodite_hooks()
  4. Registers them with Hermes

When the dylib is rebuilt (cargo build --release -p headroom-ffi), this plugin
detects the new mtime on the next hook call and reloads — no Hermes restart.

Architecture:
  Hermes → aphrodite-hermes (this file) → headroom-ffi.dylib (Rust, via ctypes)
"""

import ctypes
import json
import logging
import os
import sys
from pathlib import Path

from effects import Runtime, runtime

_log = logging.getLogger("aphrodite")

# ── Dylib paths ──────────────────────────────────────────────────────

_HERE = Path(__file__).resolve().parent
_DYLIB_NAME = "libheadroom_ffi.dylib" if sys.platform == "darwin" else "libheadroom_ffi.so"

# Search paths in priority order
_DYLIB_PATHS = [
    _HERE.parent.parent / "vendor" / "headroom" / "target" / "release" / _DYLIB_NAME,
    Path.home() / ".hermes" / "aphrodite" / _DYLIB_NAME,
]

_DYLIB = None
_DYLIB_MTIME: float = 0.0
_DYLIB_LOADED = False


def _find_dylib() -> Path:
    """Find the headroom-ffi dylib."""
    for p in _DYLIB_PATHS:
        if p.is_file():
            return p
    raise FileNotFoundError(
        f"headroom-ffi dylib not found. Searched: {_DYLIB_PATHS}\n"
        f"Build: cd vendor/headroom && cargo build --release -p headroom-ffi"
    )


# ── Dylib loader (hot-reload on mtime change) ────────────────────────

def _load_dylib() -> ctypes.CDLL:
    """Load the dylib, re-loading if file changed since last load."""
    global _DYLIB, _DYLIB_MTIME, _DYLIB_LOADED

    dylib_path = _find_dylib()
    current_mtime = dylib_path.stat().st_mtime

    if _DYLIB is not None and current_mtime == _DYLIB_MTIME:
        return _DYLIB  # unchanged

    _log.info("loading dylib: %s (mtime=%.0f)", dylib_path, current_mtime)

    lib = ctypes.CDLL(str(dylib_path))

    # Stable C ABI signatures (NEVER changes)
    lib.aphrodite_init.argtypes = [ctypes.c_char_p]
    lib.aphrodite_init.restype = ctypes.c_void_p

    lib.aphrodite_destroy.argtypes = [ctypes.c_char_p]
    lib.aphrodite_destroy.restype = None

    lib.aphrodite_hooks.argtypes = []
    lib.aphrodite_hooks.restype = ctypes.c_void_p

    lib.aphrodite_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    lib.aphrodite_call_hook.restype = ctypes.c_void_p

    lib.aphrodite_version.argtypes = []
    lib.aphrodite_version.restype = ctypes.c_void_p

    lib.aphrodite_free_string.argtypes = [ctypes.c_void_p]
    lib.aphrodite_free_string.restype = None

    _DYLIB = lib
    _DYLIB_MTIME = current_mtime
    _DYLIB_LOADED = True
    return lib


def _dylib_call(fn_name: str, *args: str) -> str:
    """Call a dylib function, free the C string, return Python str."""
    lib = _load_dylib()
    raw_fn = getattr(lib, fn_name)
    c_args = [a.encode("utf-8") if isinstance(a, str) else a for a in args]
    raw = raw_fn(*c_args)
    result = ctypes.cast(raw, ctypes.c_char_p).value.decode("utf-8")
    lib.aphrodite_free_string(raw)
    return result


# ── Bootstrap: load dylib, register services, wire hooks ─────────────

_BOOTSTRAPPED = False


def bootstrap():
    """Initialize the runtime. Idempotent — safe to call multiple times."""
    global _BOOTSTRAPPED
    if _BOOTSTRAPPED:
        return
    _BOOTSTRAPPED = True

    try:
        lib = _load_dylib()
    except FileNotFoundError as e:
        _log.warning("dylib bootstrap failed: %s", e)
        return

    version = _dylib_call("aphrodite_version")
    hooks = json.loads(_dylib_call("aphrodite_hooks"))

    # Provide services
    runtime.provide("dylib", lib)
    runtime.provide("dylib_version", version)
    runtime.provide("dylib_hooks", hooks)

    # Register default pipeline: one dylib-call per hook
    for hook_name in hooks:
        if hook_name in runtime.list_pipelines():
            continue

        def make_pipeline(name: str):
            def step(_prev):
                from effects import Effect
                def call():
                    return json.loads(_dylib_call("aphrodite_call_hook", name, "{}"))
                return Effect.try_(call)
            return step

        runtime.pipeline(hook_name, [make_pipeline(hook_name)])

    _log.info("aphrodite-hermes bootstrapped — v%s hooks=%s", version, hooks)


# ── Hermes plugin interface ──────────────────────────────────────────

def register(ctx):
    """Called by Hermes at session start. Bootstraps + registers hooks."""
    bootstrap()

    hooks = runtime.service("dylib_hooks") if _has_service("dylib_hooks") else []

    for hook_name in hooks:
        def make_handler(name: str):
            def handler(**kwargs):
                lib = _load_dylib()
                args_json = json.dumps(kwargs)
                result_json = _dylib_call("aphrodite_call_hook", name, args_json)
                result = json.loads(result_json)
                if isinstance(result, dict) and result.get("status") == "ok":
                    return result
                return result
            return handler

        ctx.hook(hook_name)(make_handler(hook_name))


def _has_service(name: str) -> bool:
    try:
        runtime.service(name)
        return True
    except KeyError:
        return False
