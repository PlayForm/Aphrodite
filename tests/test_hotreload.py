#!/usr/bin/env python3
"""Pytest suite: the Hermes plugin shim's dylib hot-reload mechanism.

Regression coverage for the bug where `ctypes.CDLL(path)` silently returned
a cached, stale image on every reload after the first (dlopen memoizes by
canonical path on both macOS dyld and Linux glibc, regardless of mtime or
file content) - see the docstring on `_load_fresh_copy` in
`plugins/aphrodite/__init__.py` for the full root-cause writeup. The fix
loads each generation from a freshly-named copy instead of the fixed path;
these tests cover that copy/cleanup bookkeeping without paying for a second
full `cargo build` per test run.
"""

import ctypes
import importlib.util
import os
import platform
import shutil
import sys
import time

import pytest

ROOT = os.path.join(os.path.dirname(__file__), "..")


def _dylib_name():
    system = platform.system()
    if system == "Darwin":
        return "libaphrodite_hermes.dylib"
    if system == "Windows":
        return "aphrodite_hermes.dll"
    return "libaphrodite_hermes.so"


SRC_DYLIB = os.path.join(ROOT, "target", "release", _dylib_name())

if not os.path.exists(SRC_DYLIB):
    pytest.skip(
        f"aphrodite-hermes dylib not built at {SRC_DYLIB}; "
        f"run `cargo build --release -p aphrodite-hermes` to enable this suite",
        allow_module_level=True,
    )


@pytest.fixture
def plugin_module(tmp_path, monkeypatch):
    """A fresh import of the plugin shim, pointed at a scratch binaries dir."""
    binaries = tmp_path / "binaries"
    binaries.mkdir()
    dylib_copy = binaries / _dylib_name()
    shutil.copy2(SRC_DYLIB, dylib_copy)
    monkeypatch.setenv("APHRODITE_HERMES_DYLIB_PATH", str(dylib_copy))

    spec = importlib.util.spec_from_file_location(
        "aphrodite_plugin_under_test",
        os.path.join(ROOT, "plugins", "aphrodite", "__init__.py"),
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    yield mod, dylib_copy


def _version(dylib):
    dylib.aphrodite_hermes_version.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_free_string.argtypes = [ctypes.c_void_p]
    ptr = dylib.aphrodite_hermes_version()
    val = ctypes.cast(ptr, ctypes.c_char_p).value.decode()
    dylib.aphrodite_hermes_free_string(ptr)
    return val


def test_first_load_creates_one_hotreload_copy(plugin_module):
    mod, dylib_path = plugin_module
    dylib = mod._load_dylib()
    assert _version(dylib)

    hotreload_dir = dylib_path.parent / ".hotreload"
    assert hotreload_dir.is_dir()
    assert len(list(hotreload_dir.iterdir())) == 1


def test_unchanged_mtime_returns_cached_handle_without_new_copy(plugin_module):
    mod, dylib_path = plugin_module
    d1 = mod._load_dylib()
    d2 = mod._load_dylib()
    assert d1._handle == d2._handle

    hotreload_dir = dylib_path.parent / ".hotreload"
    assert len(list(hotreload_dir.iterdir())) == 1


def test_mtime_change_loads_fresh_copy_and_cleans_up_previous(plugin_module):
    mod, dylib_path = plugin_module
    d1 = mod._load_dylib()
    hotreload_dir = dylib_path.parent / ".hotreload"
    first_gen = set(os.listdir(hotreload_dir))

    # Simulate a rebuild landing at the same path: content need not change for
    # this test (the mechanism keys off mtime, matching `_load_dylib`'s own
    # check), but the touch must move the mtime forward by more than the
    # filesystem's timestamp resolution.
    time.sleep(0.05)
    os.utime(dylib_path, None)

    d2 = mod._load_dylib()
    second_gen = set(os.listdir(hotreload_dir))

    # A genuinely fresh copy was loaded (different path -> different dlopen
    # image on the OSes this matters on), not the cached handle reused.
    assert first_gen != second_gen
    assert len(second_gen) == 1
    # The previous generation's file was cleaned up, not left to accumulate.
    assert not first_gen & second_gen
    assert _version(d2)
