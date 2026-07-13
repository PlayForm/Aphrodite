"""
Dynamic hook adapter - STABLE runtime bootstrap.

This file initialises the Effect runtime with built-in services
(dylib loading, hook dispatch) and registers Hermes hook pipelines.

This file NEVER changes when hooks or logic are added.
  - New hooks → added to dylib → discovered via aphrodite_hooks()
  - New effects → registered by extensions via runtime.prepend()/append()
  - New pipeline steps → extensions compose via Effect.map/flat_map
"""

import ctypes
import json
import sys
from pathlib import Path

from effects import Effect, runtime


# ── Paths ────────────────────────────────────────────────────────────────
_HERE = Path(__file__).resolve().parent
_DYLIB_NAME = "libaphrodite_dynamic.dylib" if sys.platform == "darwin" else "libaphrodite_dynamic.so"
_DYLIB = (_HERE.parent / "rust" / "target" / "release" / _DYLIB_NAME).resolve()


# ── Built-in services ────────────────────────────────────────────────────

def _load_dylib() -> ctypes.CDLL:
    """Load the Rust dylib with C ABI signatures."""
    if not _DYLIB.is_file():
        raise FileNotFoundError(
            f"Dynamic library not found at {_DYLIB}\n"
            f"  Build it: cd Maintain/dynamic_hooks/rust && cargo build --release"
        )

    lib = ctypes.CDLL(str(_DYLIB))

    # ── C ABI signatures (stable - NEVER changes) ──────────────────────
    lib.aphrodite_hooks.argtypes = []
    lib.aphrodite_hooks.restype = ctypes.c_void_p

    lib.aphrodite_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    lib.aphrodite_call_hook.restype = ctypes.c_void_p

    lib.aphrodite_version.argtypes = []
    lib.aphrodite_version.restype = ctypes.c_void_p

    lib.aphrodite_free_string.argtypes = [ctypes.c_void_p]
    lib.aphrodite_free_string.restype = None

    return lib


def _dylib_call(lib: ctypes.CDLL, fn_name: str, *args: str) -> str:
    """Call a dylib string-returning function, free C string, return Python str."""
    raw_fn = getattr(lib, fn_name)
    c_args = [a.encode("utf-8") for a in args]
    raw = raw_fn(*c_args)
    if not raw:
        # The fixture's own null-check (aphrodite_call_hook rejects null
        # args) always returns a JSON error string, never a null pointer -
        # but a null return is possible in principle (e.g. a CString::new
        # failure on the Rust side) and .value on a null c_char_p raises
        # AttributeError with no context, so raise something diagnosable.
        raise RuntimeError(f"{fn_name} returned a null pointer")
    result = ctypes.cast(raw, ctypes.c_char_p).value.decode("utf-8")
    lib.aphrodite_free_string(raw)
    return result


# ── Bootstrap ─────────────────────────────────────────────────────────────

_BOOTSTRAPPED = False


def bootstrap():
    """Initialise the runtime. Idempotent - safe to call multiple times."""
    global _BOOTSTRAPPED
    if _BOOTSTRAPPED:
        return
    _BOOTSTRAPPED = True

    try:
        lib = _load_dylib()
    except FileNotFoundError as e:
        import logging
        logging.getLogger("aphrodite").warning("dylib bootstrap failed: %s", e)
        return

    version = _dylib_call(lib, "aphrodite_version")
    hooks = json.loads(_dylib_call(lib, "aphrodite_hooks"))

    # ── Provide built-in services (available to all effects) ───────────
    runtime.provide("dylib", lib)
    runtime.provide("dylib_version", version)
    runtime.provide("dylib_hooks", hooks)

    # ── Register default pipeline: one dylib-call effect per hook ──────
    # Only register if no pipeline exists yet (extensions may have pre-registered)
    for hook_name in hooks:
        if hook_name in runtime.list_pipelines():
            continue

        def make_pipeline_fn(name: str):
            def pipeline_fn(args: dict) -> Effect:
                def _call() -> dict:
                    dylib = runtime.service("dylib")
                    result_json = _dylib_call(dylib, "aphrodite_call_hook", name, json.dumps(args))
                    return json.loads(result_json)
                return Effect.try_(_call)
            return pipeline_fn

        runtime.pipeline(hook_name, [make_pipeline_fn(hook_name)])

    import logging
    logging.getLogger("aphrodite").info(
        "aphrodite runtime bootstrapped - v%s hooks=%s", version, hooks,
    )


# ── Hermes plugin interface ──────────────────────────────────────────────

def register(ctx):
    """
    Called by Hermes at session start.
    Bootstraps the runtime and registers hooks dynamically.

    THIS FUNCTION NEVER CHANGES.
    """
    bootstrap()

    hooks = runtime.service("dylib_hooks") if _has_service("dylib_hooks") else []

    for hook_name in hooks:
        def make_handler(name: str):
            def handler(**kwargs):
                exit_result = runtime.run_exit(name, kwargs)
                if exit_result["_tag"] == "Success":
                    return exit_result["value"]
                return None
            return handler

        ctx.hook(hook_name)(make_handler(hook_name))


def _has_service(name: str) -> bool:
    try:
        runtime.service(name)
        return True
    except KeyError:
        return False


# ── Quick smoke test ──────────────────────────────────────────────────────
if __name__ == "__main__":
    bootstrap()
    print(f"  version: {runtime.service('dylib_version')}")
    print(f"  hooks:   {runtime.service('dylib_hooks')}")
    print(f"  pipelines: {runtime.list_pipelines()}")

    r = runtime.run_exit("session_start", {"session_id": "test"})
    print(f"  session_start: {r}")

    r = runtime.run_exit("transform_tool_result",
                         {"content": "error: broke\nline2", "tool_name": "test"})
    print(f"  transform:     {r}")

    print("\n✓ Effect runtime ready - extensions load via runtime.prepend()/append()")
