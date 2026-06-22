"""aphrodite — CCR compression plugin for Hermes Agent.
Installed via `cargo install aphrodite`.
"""
import ctypes, json, logging, os, subprocess, sys
from pathlib import Path

_log = logging.getLogger("aphrodite")
_PLUGIN_DIR = Path(__file__).resolve().parent
_DYLIB_NAME = "libaphrodite_hermes.dylib" if sys.platform == "darwin" else \
              "libaphrodite_hermes.so" if sys.platform == "linux" else "aphrodite_hermes.dll"
_DYLIB_PATH = os.environ.get("APHRODITE_HERMES_DYLIB_PATH",
    str(_PLUGIN_DIR / "binaries" / _DYLIB_NAME))
_BINARY_NAME = "aphrodite.exe" if sys.platform == "win32" else "aphrodite"
_BINARY_PATH = str(_PLUGIN_DIR / "binaries" / _BINARY_NAME)
_dylib = None
_dylib_mtime = 0.0

def _load_dylib():
    global _dylib, _dylib_mtime
    assert os.path.exists(_DYLIB_PATH), f"Dylib not found: {_DYLIB_PATH}"
    # Hot-reload: check mtime, reload if changed
    current_mtime = os.path.getmtime(_DYLIB_PATH)
    if _dylib is not None and current_mtime == _dylib_mtime:
        return _dylib
    if _dylib is not None:
        _log.info("dylib mtime changed — hot-reloading %s", _DYLIB_PATH)
    dylib = ctypes.CDLL(_DYLIB_PATH)
    dylib.aphrodite_hermes_get_schemas.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_get_hooks.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_list_skills.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_dispatch_tool.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    dylib.aphrodite_hermes_dispatch_tool.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    dylib.aphrodite_hermes_call_hook.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_proxy_health.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_free_string.argtypes = [ctypes.c_void_p]
    _dylib = dylib
    _dylib_mtime = current_mtime
    return dylib

def _read_str(ptr):
    if ptr is None or ptr == 0:
        return None
    return ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")

def _call_json(fn, *args):
    ptr = fn(*args)
    result = _read_str(ptr)
    if ptr:
        _load_dylib().aphrodite_hermes_free_string(ptr)
    return json.loads(result) if result else None

def _make_handler(tool_name):
    def handler(args=None, **kwargs):
        args_json = json.dumps(args or {})
        return json.dumps(_call_json(
            _load_dylib().aphrodite_hermes_dispatch_tool,
            tool_name.encode("utf-8"),
            args_json.encode("utf-8"),
        ))
    return handler

def _start_proxy():
    binary = _BINARY_PATH
    if not os.path.exists(binary):
        _log.warning("aphrodite binary not found at %s", binary)
        return
    if not os.access(binary, os.X_OK):
        os.chmod(binary, 0o755)
    env = os.environ.copy()
    env.setdefault("APHRODITE_NO_AUTO_LAUNCH", "0")
    try:
        subprocess.Popen([binary], env=env,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        _log.info("aphrodite proxy started (%s)", binary)
    except Exception as e:
        _log.warning("failed to start aphrodite proxy: %s", e)

def _proxy_health():
    try:
        return _call_json(_load_dylib().aphrodite_hermes_proxy_health)
    except Exception:
        return {}

def register(ctx):
    dylib = _load_dylib()
    _log.info("aphrodite-hermes dylib loaded: %s", _DYLIB_PATH)
    hooks = _call_json(dylib.aphrodite_hermes_get_hooks)
    if hooks:
        def _hook_dispatch(hook_name, **kwargs):
            args_json = json.dumps(kwargs)
            return _call_json(
                dylib.aphrodite_hermes_call_hook,
                hook_name.encode("utf-8"),
                args_json.encode("utf-8"),
            )
        for hook_name in hooks:
            ctx.register_hook(hook_name,
                lambda *a, name=hook_name, **kw: _hook_dispatch(name, **kw))
        _log.info("registered %d hooks", len(hooks))
    schemas = _call_json(dylib.aphrodite_hermes_get_schemas)
    if schemas:
        for schema in schemas:
            name = schema["name"]
            ctx.register_tool(schema, _make_handler(name))
        _log.info("registered %d tools", len(schemas))
    ctx.register_context_engine(
        name="aphrodite",
        pre_llm_call=lambda ctx, **kw: _call_json(
            dylib.aphrodite_hermes_dispatch_tool, b"context_engine_pre_llm", b"{}"
        ),
    )
    _start_proxy()
