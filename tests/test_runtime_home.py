"""Regression tests for issue 40: the plugin shim's runtime-home decision.

The shim used to resolve the runtime home from $HERMES_HOME while the Rust
half resolved it from $HOME, so whenever HERMES_HOME != $HOME/.hermes the
plugin disabled itself. The fix makes ONE decision (`_runtime_home`),
exports it to the Rust half via `os.environ.setdefault("APHRODITE_HOME", ...)`
(F1), adopts the legacy ~/.hermes/aphrodite home when it holds the existing
install (F2, adopt-and-warn), and logs the decision (F5).

Each case imports the real plugin shim in a SUBPROCESS with a controlled
environment, so env mutations and the module-level import side effects
(runtime-home computation, env setdefault) stay isolated.

Run directly:  python3 tests/test_runtime_home.py
Exit code is 0 when every case passes, 1 otherwise.
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PLUGIN = REPO_ROOT / "plugins" / "aphrodite" / "__init__.py"

_ASSERTS = 0


def ok(condition, message=""):
    global _ASSERTS
    _ASSERTS += 1
    if not condition:
        raise AssertionError(message or "assertion failed")


def shim_probe(extra_env):
    """Import the shim in a subprocess with a scrubbed + controlled env.

    Returns (stdout_lines, stderr) with one line per probed value.
    """
    script = (
        "import importlib.util, os, sys\n"
        f"spec = importlib.util.spec_from_file_location('_runtime_home_under_test', {str(PLUGIN)!r})\n"
        "m = importlib.util.module_from_spec(spec)\n"
        "spec.loader.exec_module(m)\n"
        "print(m._RUNTIME_HOME)\n"
        "print(m._HOME_DECISION)\n"
        "print(os.environ.get('APHRODITE_HOME', ''))\n"
        "print(os.environ.get('APHRODITE_DIRECTIVES_DIR', ''))\n"
        "print(m._BINARIES_DIR)\n"
    )
    base = dict(os.environ)
    # Scrub every home-affecting var so the child sees only what the case
    # sets (the parent session may legitimately have any of them set).
    for key in ("APHRODITE_HOME", "HERMES_HOME", "APHRODITE_DIRECTIVES_DIR"):
        base.pop(key, None)
    base.update(extra_env)
    proc = subprocess.run(
        [sys.executable, "-c", script],
        env=base,
        capture_output=True,
        text=True,
        errors="replace",
        timeout=60,
    )
    ok(proc.returncode == 0, f"shim import failed (rc={proc.returncode}): {proc.stderr}")
    return proc.stdout.splitlines(), proc.stderr


def test_default_home():
    # No overrides: canonical == legacy == ~/.hermes/aphrodite; the env var
    # is exported so the Rust half resolves the same directory.
    lines, _ = shim_probe({})
    home = Path.home() / ".hermes" / "aphrodite"
    ok(lines[0] == str(home), f"runtime home {lines[0]!r} != {home!r}")
    ok(lines[1] == "default", f"decision {lines[1]!r} != 'default'")
    ok(lines[2] == str(home), f"APHRODITE_HOME not exported: {lines[2]!r}")
    ok(lines[3] == str(home / "directives"), f"directives dir not exported: {lines[3]!r}")
    ok(lines[4] == str(home / "binaries"), f"binaries dir {lines[4]!r} != {home / 'binaries'!r}")


def test_hermes_home_wins_over_home():
    # The issue-40 shape: HERMES_HOME set to something that is NOT
    # $HOME/.hermes. HOME is a scratch dir WITHOUT an install, so the legacy
    # adoption rule must NOT fire - the canonical home wins.
    with tempfile.TemporaryDirectory() as td:
        lines, _ = shim_probe({"HERMES_HOME": td, "HOME": td})
        expected = Path(td) / "aphrodite"
        ok(lines[0] == str(expected), f"runtime home {lines[0]!r} != {expected!r}")
        ok(lines[1] == "HERMES_HOME", f"decision {lines[1]!r} != 'HERMES_HOME'")
        ok(lines[2] == str(expected), f"APHRODITE_HOME not exported: {lines[2]!r}")
        ok(lines[3] == str(expected / "directives"), f"directives dir: {lines[3]!r}")
        ok(lines[4] == str(expected / "binaries"), f"binaries dir: {lines[4]!r}")


def test_aphrodite_home_override_wins():
    # Explicit APHRODITE_HOME is never second-guessed and the export keeps
    # the user's value (setdefault is a no-op).
    with tempfile.TemporaryDirectory() as td:
        override = Path(td) / "custom-runtime"
        lines, _ = shim_probe({"APHRODITE_HOME": str(override), "HERMES_HOME": td})
        ok(lines[0] == str(override), f"runtime home {lines[0]!r} != {override!r}")
        ok(lines[1] == "APHRODITE_HOME override", f"decision {lines[1]!r}")
        ok(lines[2] == str(override), f"override lost: {lines[2]!r}")
        ok(lines[3] == str(override / "directives"), f"directives dir: {lines[3]!r}")


def test_legacy_home_adopted_when_it_holds_the_install():
    # F2: upgrading an install whose runtime home lives at the pre-2.2
    # location (~/.hermes/aphrodite) while the HERMES_HOME-derived home is
    # empty must ADOPT the legacy home (never migrate), so binaries/ and
    # aphrodite.toml keep resolving. The sandbox is placed OUTSIDE the OS
    # temp dir: a HERMES_HOME under the temp dir is a throwaway/scratch home
    # (catalog-validate probe) and adoption must never fire there.
    sandbox_base = Path(tempfile.gettempdir()).parent
    with tempfile.TemporaryDirectory(dir=str(sandbox_base)) as td:
        base = Path(td)
        legacy = base / "home" / ".hermes" / "aphrodite"
        (legacy / "binaries").mkdir(parents=True)
        (legacy / "binaries" / "aphrodite").write_bytes(b"BIN")
        (legacy / "aphrodite.toml").write_text("key = 'value'\n")
        lines, stderr = shim_probe({"HERMES_HOME": str(base / "hermes"), "HOME": str(base / "home")})
        ok(lines[0] == str(legacy), f"runtime home {lines[0]!r} != adopted legacy {legacy!r}")
        ok(lines[1] == "legacy ~/.hermes/aphrodite adoption", f"decision {lines[1]!r}")
        ok(lines[2] == str(legacy), f"APHRODITE_HOME not exported to the legacy home: {lines[2]!r}")
        ok(
            "adopting the pre-2.2 runtime home" in stderr,
            f"adoption must warn, got stderr: {stderr!r}",
        )
        ok(lines[4] == str(legacy / "binaries"), f"binaries dir: {lines[4]!r}")


def test_scratch_home_never_adopts_legacy():
    # The catalog-validate probe (hermes_cli/plugin_validate.py) runs
    # register() in a scratch child whose HERMES_HOME is a
    # tempfile.TemporaryDirectory - adoption there would redirect the probe
    # onto the real ~/.hermes/aphrodite and the layout heal + directives
    # materialize would write into the live install (PR 118488 pollution).
    # A throwaway HERMES_HOME must keep fresh-install semantics.
    with tempfile.TemporaryDirectory() as td:
        base = Path(td)
        hermes_home = base / "hermes"
        legacy = base / "home" / ".hermes" / "aphrodite"
        (legacy / "binaries").mkdir(parents=True)
        (legacy / "binaries" / "aphrodite").write_bytes(b"BIN")
        (legacy / "aphrodite.toml").write_text("key = 'value'\n")
        lines, stderr = shim_probe({"HERMES_HOME": str(hermes_home), "HOME": str(base / "home")})
        expected = hermes_home / "aphrodite"
        ok(lines[0] == str(expected), f"runtime home {lines[0]!r} != scratch {expected!r}")
        ok(lines[1] == "HERMES_HOME", f"decision {lines[1]!r} != 'HERMES_HOME' (no adoption)")
        ok(
            "adopting the pre-2.2 runtime home" not in stderr,
            f"adoption must NOT fire for a scratch home, got stderr: {stderr!r}",
        )
        ok(lines[4] == str(expected / "binaries"), f"binaries dir: {lines[4]!r}")


def test_hermes_scratch_cache_home_never_adopts_legacy():
    # Hermes' documented scratch-home pattern ($HOME/.hermes/cache/scratch,
    # used by plugin tests and throwaway runs) must also never adopt the
    # real install. Sandbox placed OUTSIDE the OS temp dir so this case
    # exercises the cache/scratch check specifically.
    sandbox_base = Path(tempfile.gettempdir()).parent
    with tempfile.TemporaryDirectory(dir=str(sandbox_base)) as td:
        base = Path(td)
        home = base / "home"
        scratch = home / ".hermes" / "cache" / "scratch" / "hermes-plugin-test"
        legacy = home / ".hermes" / "aphrodite"
        (legacy / "binaries").mkdir(parents=True)
        (legacy / "binaries" / "aphrodite").write_bytes(b"BIN")
        (legacy / "aphrodite.toml").write_text("key = 'value'\n")
        lines, stderr = shim_probe({"HERMES_HOME": str(scratch), "HOME": str(home)})
        expected = scratch / "aphrodite"
        ok(lines[0] == str(expected), f"runtime home {lines[0]!r} != scratch {expected!r}")
        ok(lines[1] == "HERMES_HOME", f"decision {lines[1]!r} != 'HERMES_HOME' (no adoption)")
        ok(
            "adopting the pre-2.2 runtime home" not in stderr,
            f"adoption must NOT fire for a cache/scratch home, got stderr: {stderr!r}",
        )


def test_import_never_raises_on_hostile_env():
    # Pathological env values (empty overrides, relative override) must
    # never crash the shim import - registration degrades, it never aborts.
    lines, stderr = shim_probe({"APHRODITE_HOME": "", "HERMES_HOME": ""})
    home = Path.home() / ".hermes" / "aphrodite"
    ok(lines[0] == str(home), f"empty overrides must fall through to default: {lines[0]!r}")
    ok(lines[1] == "default", f"decision {lines[1]!r} != 'default'")

    lines, _ = shim_probe({"APHRODITE_HOME": "relative/aph"})
    ok(lines[0] == "relative/aph", f"relative override must be used as-is: {lines[0]!r}")
    ok(lines[1] == "APHRODITE_HOME override", f"decision {lines[1]!r}")
    ok(lines[2] == "relative/aph", f"relative override lost in the export: {lines[2]!r}")
    ok(lines[3] == "relative/aph/directives", f"directives dir: {lines[3]!r}")


def test_canonical_home_wins_when_it_already_holds_the_install():
    # When BOTH homes hold an install the canonical (HERMES_HOME-derived)
    # home wins - the user has already migrated; never roll back.
    with tempfile.TemporaryDirectory() as td:
        base = Path(td)
        canonical = base / "hermes" / "aphrodite"
        (canonical / "binaries").mkdir(parents=True)
        (canonical / "binaries" / "aphrodite").write_bytes(b"BIN")
        legacy = base / "home" / ".hermes" / "aphrodite"
        (legacy / "binaries").mkdir(parents=True)
        (legacy / "binaries" / "aphrodite").write_bytes(b"OLD")
        lines, _ = shim_probe({"HERMES_HOME": str(base / "hermes"), "HOME": str(base / "home")})
        ok(lines[0] == str(canonical), f"runtime home {lines[0]!r} != canonical {canonical!r}")
        ok(lines[1] == "HERMES_HOME", f"decision {lines[1]!r} != 'HERMES_HOME'")


def main():
    tests = [fn for name, fn in sorted(globals().items()) if name.startswith("test_") and callable(fn)]
    failed = []
    for fn in tests:
        try:
            fn()
            print(f"PASS {fn.__name__}")
        except Exception as exc:  # noqa: BLE001 - test harness
            failed.append((fn.__name__, exc))
            print(f"FAIL {fn.__name__}: {exc}")
    passed = len(tests) - len(failed)
    print(f"{passed}/{len(tests)} cases passed, {_ASSERTS} asserts")
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main()