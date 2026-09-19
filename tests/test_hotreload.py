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
import subprocess
import sys
import tempfile
import time
import urllib.request

import pytest

ROOT = os.path.join(os.path.dirname(__file__), "..")


def _dylib_name():
    system = platform.system()
    if system == "Darwin":
        return "libaphrodite_hermes.dylib"
    if system == "Windows":
        return "aphrodite_hermes.dll"
    return "libaphrodite_hermes.so"


def _target_triple() -> str:
    """Platform triple used by download.sh's release asset names."""
    arch = platform.machine().lower()
    if arch in ("arm64", "aarch64"):
        arch = "aarch64"
    elif arch in ("x86_64", "amd64"):
        arch = "x86_64"
    system = platform.system().lower()
    if system == "darwin":
        return f"{arch}-apple-darwin"
    if system == "linux":
        return f"{arch}-unknown-linux-gnu"
    if system in ("windows", "mingw", "msys", "cygwin"):
        return f"{arch}-pc-windows-msvc"
    return f"{arch}-{system}"


def _download_published_dylib(dest: str) -> bool:
    """Fallback: fetch the dylib from the latest PUBLISHED release.

    BINARY_VERSION (v1.4.6) is unreleased - the release ceremony hasn't run
    - so the plugin's pinned download.sh URLs 404 on every asset. v1.4.5 is
    the newest published tag; mirror download.sh's asset naming so the
    fixture stays independent of the unreleased binary.
    """
    version = "1.4.5"
    triple = _target_triple()
    if "windows" in triple:
        asset = f"libaphrodite_hermes-{triple}.dll"
    elif "apple" in triple:
        asset = f"libaphrodite_hermes-{triple}.dylib"
    else:
        asset = f"libaphrodite_hermes-{triple}.so"
    url = f"https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite%2Fv{version}/{asset}"
    try:
        with urllib.request.urlopen(url, timeout=30) as resp:
            data = resp.read()
    except Exception as e:
        print(f"WARNING: could not download published dylib {url}: {e}")
        return False
    if len(data) < 1024:
        print(f"WARNING: downloaded {url} is empty or truncated ({len(data)} bytes)")
        return False
    with open(dest, "wb") as f:
        f.write(data)
    return True


def _obtain_dylib():
    """A real dylib for the fixture, independent of the unreleased v1.4.6
    release. Preference order (fix-ci-tests-instructions):
      1. an existing workspace build (target/release, then target/debug)
      2. `cargo build -p aphrodite-hermes` (deterministic, offline)
      3. the latest PUBLISHED release (v1.4.5) asset
    Returns the source dylib path, or None when all three fail (skip).
    """
    name = _dylib_name()
    for sub in ("release", "debug"):
        cand = os.path.join(ROOT, "target", sub, name)
        if os.path.exists(cand):
            return cand
    try:
        build = subprocess.run(
            ["cargo", "build", "-p", "aphrodite-hermes"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            errors="replace",
            timeout=600,
        )
    except Exception as e:
        print(
            f"WARNING: cargo build -p aphrodite-hermes could not run ({e}); falling back to published release"
        )
    else:
        if build.returncode == 0:
            cand = os.path.join(ROOT, "target", "debug", name)
            if os.path.exists(cand):
                return cand
        else:
            print(
                f"WARNING: cargo build -p aphrodite-hermes failed (rc={build.returncode}); "
                "falling back to published release"
            )
    dest = os.path.join(tempfile.mkdtemp(prefix="aphrodite-test-dylib-"), name)
    if _download_published_dylib(dest):
        return dest
    return None


SRC_DYLIB = _obtain_dylib()

if not SRC_DYLIB:
    pytest.skip(
        "no aphrodite-hermes dylib available (local build failed and the "
        "published-release fallback failed); run `cargo build -p "
        "aphrodite-hermes` to enable this suite",
        allow_module_level=True,
    )


@pytest.fixture
def plugin_module(tmp_path, monkeypatch):
    """A fresh import of the plugin shim, pointed at a scratch binaries dir."""
    binaries = tmp_path / "binaries"
    binaries.mkdir()
    dylib_copy = binaries / _dylib_name()
    shutil.copy2(SRC_DYLIB, dylib_copy)
    # copy2 preserves the source mtime, and the plugin's hotreload state is
    # process-global (mtime-keyed) - a second fixture copy with the same
    # mtime would hit the cached-handle early return and never load or copy
    # anything. Touch it so each test's copy looks like a fresh rebuild.
    os.utime(dylib_copy, None)
    monkeypatch.setenv("APHRODITE_HERMES_DYLIB_PATH", str(dylib_copy))
    # Isolate the runtime home (hotreload cache + reaper sweep) inside the
    # sandbox instead of the real ~/.hermes/aphrodite, and disable the
    # auto-download bootstrap: the fixture provides the dylib itself, and
    # the pinned BINARY_VERSION (v1.4.6) is unreleased, so download.sh
    # would 404 on every asset (the CI failure this fixture now avoids).
    monkeypatch.setenv("APHRODITE_HOME", str(binaries))
    monkeypatch.setenv("APHRODITE_NO_AUTO_DOWNLOAD", "1")

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

    hotreload_dir = dylib_path.parent / "hotreload"
    assert hotreload_dir.is_dir()
    assert len(list(hotreload_dir.iterdir())) == 1


def test_unchanged_mtime_returns_cached_handle_without_new_copy(plugin_module):
    mod, dylib_path = plugin_module
    d1 = mod._load_dylib()
    d2 = mod._load_dylib()
    assert d1._handle == d2._handle

    hotreload_dir = dylib_path.parent / "hotreload"
    assert len(list(hotreload_dir.iterdir())) == 1


def test_mtime_change_loads_fresh_copy_and_cleans_up_previous(plugin_module):
    mod, dylib_path = plugin_module
    d1 = mod._load_dylib()
    hotreload_dir = dylib_path.parent / "hotreload"
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
