#!/usr/bin/env python3
"""Self-contained test for aphrodite_hermes_materialize_directives.

No pytest dependency: run with `python3 tests/test_directives_materialize.py`.

Requires the dylib to be built first:

    cargo build -p aphrodite-hermes
    python3 tests/test_directives_materialize.py

If the dylib is missing (or a stale build lacks the new symbol), the test
prints a warning and exits 0 so pre-commit/CI runs don't hard-fail before
the rebuild happens.
"""

import ctypes
import json
import os
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Both build profiles are accepted; release wins when both exist.
DYLIB_CANDIDATES = [
    REPO_ROOT / "target" / "release" / "libaphrodite_hermes.dylib",
    REPO_ROOT / "target" / "debug" / "libaphrodite_hermes.dylib",
    REPO_ROOT / "target" / "release" / "libaphrodite_hermes.so",
    REPO_ROOT / "target" / "debug" / "libaphrodite_hermes.so",
]

# The embed source of truth: the core crate's builtin_directives/ (included
# into the binary at compile time via include_str!).
EMBED_SOURCE_DIR = REPO_ROOT / "crates" / "aphrodite" / "src" / "builtin_directives"
EMBEDDED_NAMES = (
    sorted(name for name in os.listdir(EMBED_SOURCE_DIR) if name.endswith(".md"))
    if EMBED_SOURCE_DIR.is_dir()
    else [
        "ccr-handling.md",
        "cleanup.md",
        "explore.md",
        "focus.md",
        "foresight.md",
        "lazy-eval.md",
        "lazy.md",
    ]
)


def main():
    dylib = next((p for p in DYLIB_CANDIDATES if p.is_file()), None)
    if dylib is None:
        print(
            "SKIP: libaphrodite_hermes not built "
            "(run `cargo build -p aphrodite-hermes`); skipping directives test"
        )
        return 0

    lib = ctypes.CDLL(str(dylib))
    try:
        materialize = lib.aphrodite_hermes_materialize_directives
        free_string = lib.aphrodite_hermes_free_string
    except AttributeError:
        print(
            "SKIP: dylib is stale (missing aphrodite_hermes_materialize_directives); "
            "rebuild with `cargo build -p aphrodite-hermes`"
        )
        return 0

    materialize.restype = ctypes.c_void_p
    materialize.argtypes = [ctypes.c_char_p]
    free_string.restype = None
    free_string.argtypes = [ctypes.c_void_p]

    with tempfile.TemporaryDirectory(prefix="aphrodite-directives-test-") as tmp:
        home = Path(tmp)
        _assert_first_run(materialize, free_string, home)
        _assert_idempotent_rerun(materialize, free_string, home)
        _assert_no_overwrite_of_user_modified(materialize, free_string, home)

    _assert_env_overrides(materialize, free_string)

    print("PASS: directives materialization contract verified")
    return 0


def _call(materialize, free_string, home=None):
    """Call the FFI with an optional home; returns the parsed JSON report."""
    ptr = materialize(str(home).encode() if home else None)
    assert ptr, "materialize returned a null pointer"
    try:
        raw = ctypes.string_at(ptr).decode("utf-8", "replace")
    finally:
        free_string(ptr)
    report = json.loads(raw)
    assert {"status", "dir", "written", "skipped", "warnings"} <= set(report), report
    assert report["status"] == "ok", report
    return report


def _assert_on_disk(directives_dir):
    for name in EMBEDDED_NAMES:
        dest = directives_dir / name
        assert dest.is_file(), f"{name} missing in {directives_dir}"
        assert dest.stat().st_size > 0, f"{name} empty"
        src = EMBED_SOURCE_DIR / name
        if src.is_file():
            assert dest.read_bytes() == src.read_bytes(), f"{name} differs from embedded source"


def _assert_first_run(materialize, free_string, home):
    report = _call(materialize, free_string, home)
    assert sorted(report["written"]) == EMBEDDED_NAMES, report
    assert report["skipped"] == [], report
    assert report["warnings"] == [], report
    directives_dir = home / "directives"
    assert str(directives_dir) == report["dir"], report
    assert directives_dir.is_dir()
    _assert_on_disk(directives_dir)


def _assert_idempotent_rerun(materialize, free_string, home):
    report = _call(materialize, free_string, home)
    assert report["written"] == [], report
    assert sorted(report["skipped"]) == EMBEDDED_NAMES, report
    assert report["warnings"] == [], report
    _assert_on_disk(home / "directives")


def _assert_no_overwrite_of_user_modified(materialize, free_string, home):
    directives_dir = home / "directives"
    victim = directives_dir / "focus.md"
    user_content = b"# focus\nUSER-CUSTOMIZED-DIRECTIVE\n"
    victim.write_bytes(user_content)

    report = _call(materialize, free_string, home)
    assert "focus.md" in report["skipped"], report
    assert "focus.md" not in report["written"], report
    assert any("user-modified" in w for w in report["warnings"]), report
    assert victim.read_bytes() == user_content, "user-modified file must be left untouched"
    for name in EMBEDDED_NAMES:
        if name != "focus.md":
            assert (directives_dir / name).is_file(), f"{name} must still be materialized"


def _assert_env_overrides(materialize, free_string):
    saved_home = os.environ.get("APHRODITE_HOME")
    saved_dir = os.environ.get("APHRODITE_DIRECTIVES_DIR")
    try:
        # APHRODITE_HOME: null home arg falls back to the env override.
        os.environ.pop("APHRODITE_DIRECTIVES_DIR", None)
        with tempfile.TemporaryDirectory(prefix="aphrodite-directives-env-") as tmp:
            env_home = Path(tmp)
            os.environ["APHRODITE_HOME"] = str(env_home)
            report = _call(materialize, free_string)  # home=None -> env resolution
            assert Path(report["dir"]) == env_home / "directives", report
            assert sorted(report["written"]) == EMBEDDED_NAMES, report
            assert (env_home / "directives" / "focus.md").is_file()

        # APHRODITE_DIRECTIVES_DIR: exact-directory override (loader candidate 0).
        with tempfile.TemporaryDirectory(prefix="aphrodite-directives-env2-") as tmp:
            exact = Path(tmp)
            os.environ["APHRODITE_DIRECTIVES_DIR"] = str(exact)
            report = _call(materialize, free_string)
            assert Path(report["dir"]) == exact, report
            assert (exact / "focus.md").is_file()
    finally:
        for var, val in (("APHRODITE_HOME", saved_home), ("APHRODITE_DIRECTIVES_DIR", saved_dir)):
            if val is None:
                os.environ.pop(var, None)
            else:
                os.environ[var] = val


if __name__ == "__main__":
    sys.exit(main())
