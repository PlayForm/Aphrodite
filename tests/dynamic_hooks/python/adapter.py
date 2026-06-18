"""
Dynamic hook adapter — proof of concept.

This is the STABLE adapter file (never changes).
All real logic lives in the Rust .dylib — rebuild + swap, no Python reload needed.
"""

import ctypes
import os
import sys
from pathlib import Path


_HERE = Path(__file__).resolve().parent
_DYLIB_DIR = _HERE.parent / "rust" / "target" / "release"
_DYLIB_NAME = {
    "darwin": "libaphrodite_dynamic.dylib",
    "linux": "libaphrodite_dynamic.so",
}.get(sys.platform, "libaphrodite_dynamic.so")

_dylib_path = _DYLIB_DIR / _DYLIB_NAME


def load():
    """Load (or reload) the dynamic library."""
    if not _dylib_path.is_file():
        raise FileNotFoundError(
            f"dylib not found at {_dylib_path}\n"
            f"  Build it: cd tests/dynamic_hooks/rust && cargo build --release"
        )

    lib = ctypes.CDLL(str(_dylib_path))

    # ── Define signatures ───────────────────────────────────────────────
    lib.aphrodite_classify.argtypes = [ctypes.c_char_p]
    lib.aphrodite_classify.restype = ctypes.c_void_p

    lib.aphrodite_version.argtypes = []
    lib.aphrodite_version.restype = ctypes.c_void_p

    lib.aphrodite_free_string.argtypes = [ctypes.c_void_p]
    lib.aphrodite_free_string.restype = None

    return lib


def classify(lib, content: str) -> str:
    """Classify content via the dynamic library."""
    raw = lib.aphrodite_classify(content.encode("utf-8"))
    result = ctypes.cast(raw, ctypes.c_char_p).value.decode("utf-8")
    lib.aphrodite_free_string(raw)
    return result


def version(lib) -> str:
    """Get the dynamic library version."""
    raw = lib.aphrodite_version()
    ver = ctypes.cast(raw, ctypes.c_char_p).value.decode("utf-8")
    lib.aphrodite_free_string(raw)
    return ver


# ── Quick smoke test ────────────────────────────────────────────────────
if __name__ == "__main__":
    lib = load()
    print(f"   version: {version(lib)}")

    e = classify(lib, "error: something broke\nline 2")
    print(f"   error:   {e}")

    w = classify(lib, "warning: disk full\ncontinuing")
    print(f"   warn:    {w}")

    o = classify(lib, "hello world\nall good")
    print(f"   ok:      {o}")

    print("\n\u2713 dylib loaded and called successfully")
