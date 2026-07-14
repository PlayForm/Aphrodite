#!/usr/bin/env python3
"""Pytest suite: aphrodite-hermes dylib FFI (current handle-based ABI).

Loads `target/release/libaphrodite_hermes.{dylib,so}` built from this
workspace (NOT the vendored headroom-ffi fork - see `docs/` for the ABI
split) and exercises the `aphrodite_hermes_*` C functions directly via
ctypes: tool dispatch, hook dispatch, schema listing, and string freeing.

Locally, if the dylib hasn't been built yet, tests are skipped with a
clear message. In CI, set `APHRODITE_REQUIRE_DYLIB=1` so a missing dylib
fails the run instead of silently skipping (see `.github/workflows/Check.yml`).
"""

import ctypes
import json
import os
import platform
import sys

import pytest

ROOT = os.path.join(os.path.dirname(__file__), "..")


def _dylib_path():
    system = platform.system()
    if system == "Darwin":
        name = "libaphrodite_hermes.dylib"
    elif system == "Windows":
        name = "aphrodite_hermes.dll"
    else:
        name = "libaphrodite_hermes.so"
    return os.path.join(ROOT, "target", "release", name)


DYLIB_PATH = _dylib_path()

if not os.path.exists(DYLIB_PATH):
    if os.environ.get("APHRODITE_REQUIRE_DYLIB") == "1":
        pytest.fail(
            f"dylib not built - run `cargo build --release -p aphrodite-hermes` "
            f"(expected at {DYLIB_PATH})"
        )
    pytest.skip(
        f"aphrodite-hermes dylib not built at {DYLIB_PATH}; "
        f"run `cargo build --release -p aphrodite-hermes` to enable this suite",
        allow_module_level=True,
    )

lib = ctypes.CDLL(DYLIB_PATH)

lib.aphrodite_hermes_dispatch_tool.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
lib.aphrodite_hermes_dispatch_tool.restype = ctypes.c_void_p
lib.aphrodite_hermes_list_tools.argtypes = []
lib.aphrodite_hermes_list_tools.restype = ctypes.c_void_p
lib.aphrodite_hermes_list_skills.argtypes = []
lib.aphrodite_hermes_list_skills.restype = ctypes.c_void_p
lib.aphrodite_hermes_get_schema.argtypes = [ctypes.c_char_p]
lib.aphrodite_hermes_get_schema.restype = ctypes.c_void_p
lib.aphrodite_hermes_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
lib.aphrodite_hermes_call_hook.restype = ctypes.c_void_p
lib.aphrodite_hermes_version.argtypes = []
lib.aphrodite_hermes_version.restype = ctypes.c_void_p
lib.aphrodite_hermes_free_string.argtypes = [ctypes.c_void_p]
lib.aphrodite_hermes_free_string.restype = None


def _consume(ptr):
    """Decode a returned C string and free it. Returns '' for NULL."""
    if not ptr:
        return ""
    s = ctypes.cast(ptr, ctypes.c_char_p).value.decode()
    lib.aphrodite_hermes_free_string(ptr)
    return s


def dispatch_tool(name: str, args: dict) -> dict:
    ptr = lib.aphrodite_hermes_dispatch_tool(name.encode(), json.dumps(args).encode())
    return json.loads(_consume(ptr))


def call_hook(name: str, args: dict) -> dict:
    ptr = lib.aphrodite_hermes_call_hook(name.encode(), json.dumps(args).encode())
    raw = _consume(ptr)
    return json.loads(raw) if raw else None


def test_version_is_semver():
    ptr = lib.aphrodite_hermes_version()
    v = json.loads(_consume(ptr))["version"]
    assert v[0].isdigit() and "." in v


def test_list_tools_returns_array_with_known_entries():
    ptr = lib.aphrodite_hermes_list_tools()
    schemas = json.loads(_consume(ptr))
    assert isinstance(schemas, list)
    names = {s["name"] for s in schemas}
    assert "aphrodite_compress" in names
    assert "aphrodite_retrieve" in names


def test_list_skills_returns_array():
    ptr = lib.aphrodite_hermes_list_skills()
    skills = json.loads(_consume(ptr))
    assert isinstance(skills, list)


def test_get_schema_known_tool():
    ptr = lib.aphrodite_hermes_get_schema(b"aphrodite_compress")
    schema = json.loads(_consume(ptr))
    assert schema["name"] == "aphrodite_compress"


def test_get_schema_unknown_tool_is_error():
    ptr = lib.aphrodite_hermes_get_schema(b"not_a_real_tool")
    result = json.loads(_consume(ptr))
    assert "error" in result


def test_compress_then_retrieve_roundtrip():
    content = 'fn hello() -> &\'static str { "world" }\n'
    compressed = dispatch_tool("aphrodite_compress", {"content": content})
    assert compressed["hash"]
    retrieved = dispatch_tool("aphrodite_retrieve", {"hash": compressed["hash"]})
    assert retrieved["found"] is True
    assert retrieved["content"] == content


def test_retrieve_unknown_hash_not_found():
    result = dispatch_tool("aphrodite_retrieve", {"hash": "deadbeefdeadbeefdeadbeef"})
    assert result["found"] is False


def test_dispatch_unknown_tool_is_error():
    result = dispatch_tool("not_a_real_tool", {})
    assert "error" in result


def test_call_hook_session_start():
    result = call_hook("on_session_start", {})
    assert result["status"] == "ok"


def test_call_hook_unknown_is_error():
    result = call_hook("not_a_real_hook", {})
    assert "error" in result


def test_free_string_null_is_noop():
    lib.aphrodite_hermes_free_string(None)
