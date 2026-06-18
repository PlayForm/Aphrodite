"""
Dynamic hook adapter — STABLE bridge between Hermes plugin system and Rust dylib.

This file NEVER changes when you add hooks or logic.
All real work lives in the Rust .dylib — rebuild + swap, no Python reload needed.

Contract:
  - Dylib exports: aphrodite_hooks(), aphrodite_call_hook(name, json), aphrodite_version(), aphrodite_free_string(s)
  - Adapter discovers hooks dynamically → registers with Hermes → forwards calls
  - Mtime-based reload: dylib rebuilds are picked up mid-session without /new
"""

import ctypes
import json
import os
import sys
from pathlib import Path


# ── Paths ────────────────────────────────────────────────────────────────
_HERE = Path(__file__).resolve().parent

if sys.platform == "darwin":
    _DYLIB_NAME = "libaphrodite_dynamic.dylib"
else:
    _DYLIB_NAME = "libaphrodite_dynamic.so"

_DYLIB = (_HERE.parent / "rust" / "target" / "release" / _DYLIB_NAME).resolve()


# ── State ─────────────────────────────────────────────────────────────────
_lib: ctypes.CDLL | None = None
_lib_mtime: float = 0.0
_lib_hooks: list[str] = []


# ── Reload-aware library access ───────────────────────────────────────────
def _get_lib() -> ctypes.CDLL:
    """Return the dylib handle, re-loading if the file was rebuilt."""
    global _lib, _lib_mtime, _lib_hooks

    try:
        mtime = os.stat(_DYLIB).st_mtime
    except FileNotFoundError:
        raise FileNotFoundError(
            f"Dynamic library not found at {_DYLIB}\n"
            f"  Build it: cd tests/dynamic_hooks/rust && cargo build --release"
        )

    if _lib is not None and mtime == _lib_mtime:
        return _lib

    # ── (Re)load ──────────────────────────────────────────────────────
    _lib = ctypes.CDLL(str(_DYLIB))

    # Signatures — these NEVER change (the dylib adds hooks internally)
    _lib.aphrodite_hooks.argtypes = []
    _lib.aphrodite_hooks.restype = ctypes.c_void_p

    _lib.aphrodite_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    _lib.aphrodite_call_hook.restype = ctypes.c_void_p

    _lib.aphrodite_version.argtypes = []
    _lib.aphrodite_version.restype = ctypes.c_void_p

    _lib.aphrodite_free_string.argtypes = [ctypes.c_void_p]
    _lib.aphrodite_free_string.restype = None

    _lib_mtime = mtime

    # Discover hooks dynamically
    _lib_hooks = json.loads(_call("aphrodite_hooks"))

    return _lib


def _call(fn_name: str, *args: str) -> str:
    """Call a dylib function that returns a string, free it, return Python str."""
    lib = _get_lib()
    raw_fn = getattr(lib, fn_name)
    c_args = [a.encode("utf-8") for a in args]
    raw = raw_fn(*c_args)
    result = ctypes.cast(raw, ctypes.c_char_p).value.decode("utf-8")
    lib.aphrodite_free_string(raw)
    return result


# ── Hermes plugin registration ────────────────────────────────────────────
def register(ctx):
    """
    Called by Hermes at session start.
    Discovers hooks from the dylib and registers them dynamically.
    THIS FUNCTION NEVER CHANGES — new hooks are added in Rust, not here.
    """
    try:
        lib = _get_lib()
    except FileNotFoundError as e:
        import logging
        logging.getLogger("aphrodite").warning("dylib not available: %s", e)
        return

    for hook_name in _lib_hooks:
        # Create a closure that captures hook_name and forwards to the dylib
        def make_handler(name: str):
            def handler(**kwargs):
                try:
                    # Re-load dylib on every call (mtime check is cheap — 1µs)
                    _get_lib()
                    args_json = json.dumps(kwargs)
                    result_json = _call("aphrodite_call_hook", name, args_json)
                    result = json.loads(result_json)
                except Exception as exc:
                    return None
                return result
            return handler

        ctx.hook(hook_name)(make_handler(hook_name))

    # Also fire session_start immediately for the current session
    from hermes_cli.plugins import invoke_hook as _invoke
    try:
        _invoke("on_session_start", session_id=ctx.session_id if hasattr(ctx, 'session_id') else "")
    except Exception:
        pass


def version() -> str:
    """Return dylib version string."""
    return _call("aphrodite_version")


# ── Quick smoke test ──────────────────────────────────────────────────────
if __name__ == "__main__":
    lib = _get_lib()
    print(f"  version: {version()}")
    print(f"  hooks:   {_lib_hooks}")

    r = _call("aphrodite_call_hook", "session_start", '{"session_id":"test"}')
    print(f"  session_start: {r}")

    r = _call("aphrodite_call_hook", "transform_tool_result",
              '{"content":"error: broke\\nline2","tool_name":"test"}')
    print(f"  transform:     {r}")

    print("\n✓ adapter ready — add hooks in Rust, never touch this file")
