"""Self-contained tests for plugins/aphrodite/layout_check.py (no pytest).

Run directly:  python3 tests/test_layout_check.py
Exit code is 0 when every case passes, 1 otherwise.
"""

import os
import sys
import tempfile
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
PLUGIN_DIR = REPO_ROOT / "plugins" / "aphrodite"
sys.path.insert(0, str(PLUGIN_DIR))

from layout_check import check_and_heal  # noqa: E402

_ASSERTS = 0


def ok(condition, message=""):
    global _ASSERTS
    _ASSERTS += 1
    if not condition:
        raise AssertionError(message or "assertion failed")


def make_tree(root):
    """Build a canonical fake install under ``root``; return (home, src)."""
    home = root / "home"
    src = root / "plugin-src"
    plugin = src / "plugins" / "aphrodite"
    plugin.mkdir(parents=True, exist_ok=True)
    (plugin / "__init__.py").write_text("# fake plugin\n")
    (plugin / "plugin.yaml").write_text("name: aphrodite\n")
    runtime = home / ".hermes" / "aphrodite"
    runtime.mkdir(parents=True, exist_ok=True)
    (runtime / "aphrodite.toml").write_text("key = 'value'\n")
    (runtime / "ccr.db").write_text("db-bytes")
    (home / ".hermes" / "plugins").mkdir(parents=True, exist_ok=True)
    (home / ".hermes" / "plugins" / "aphrodite").symlink_to(plugin)
    return home, src


def tree_snapshot(root):
    out = {}
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames.sort()
        for name in sorted(dirnames + filenames):
            p = Path(dirpath) / name
            rel = str(p.relative_to(root))
            if p.is_symlink():
                out[rel] = ("symlink", os.readlink(p))
            elif p.is_dir():
                out[rel] = ("dir", None)
            else:
                out[rel] = ("file", p.read_bytes())
    return out


def test_happy_path():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        report = check_and_heal(
            home_dir=home, dry_run=False, plugin_dir=src / "plugins" / "aphrodite"
        )
        ok(report["mismatches"] == [], f"unexpected mismatches: {report['mismatches']}")
        ok(report["actions_taken"] == [], f"unexpected actions: {report['actions_taken']}")
        ok(report["warnings"] == [], f"unexpected warnings: {report['warnings']}")


def test_plugin_dir_contents_never_touched():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        runtime = home / ".hermes" / "aphrodite"
        (plugin / "aphrodite.toml").write_text("plugin-cfg\n")
        (runtime / "aphrodite.toml").unlink()
        (plugin / "ccr.db").write_text("db-bytes")
        (plugin / "binaries").mkdir()
        (plugin / "binaries" / "aphrodite").write_bytes(b"BIN\x00\x01")
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        # The plugin dir is Hermes-owned: the plugin never moves, removes, or
        # modifies ANYTHING inside it - report-only (catalog review, PR 118488).
        ok((plugin / "aphrodite.toml").read_text() == "plugin-cfg\n", "config was moved out of plugin dir")
        ok((plugin / "ccr.db").read_text() == "db-bytes", "ccr.db was moved out of plugin dir")
        ok((plugin / "binaries" / "aphrodite").read_bytes() == b"BIN\x00\x01", "plugin-dir binaries were touched")
        ok(
            not any("moved" in a or "copy" in a for a in report["actions_taken"]),
            f"plugin dir was modified: {report['actions_taken']}",
        )


def test_plugin_dir_not_symlink():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        link = home / ".hermes" / "plugins" / "aphrodite"
        link.unlink()
        link.mkdir()
        (link / "aphrodite.toml").write_text("cfg\n")
        (home / ".hermes" / "aphrodite" / "aphrodite.toml").unlink()
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        # The plugin never rewrites the install layout under
        # <hermes-home>/plugins/ at register time (catalog review, PR 118488):
        # a real directory stays a real directory - report-only.
        ok(not link.is_symlink(), "plugin path was converted to a symlink")
        # Config in the plugin dir stays put - nothing inside the
        # Hermes-owned plugin dir is ever moved (report-only).
        ok((link / "aphrodite.toml").read_text() == "cfg\n", "config was moved out of plugin dir")
        ok(
            not any("created symlink" in a for a in report["actions_taken"]),
            "symlink creation action present",
        )


def test_plugin_dir_not_symlink_nonempty():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        link = home / ".hermes" / "plugins" / "aphrodite"
        link.unlink()
        link.mkdir()
        (link / "user-notes.txt").write_text("keep me")
        report = check_and_heal(
            home_dir=home, dry_run=False, plugin_dir=src / "plugins" / "aphrodite"
        )
        ok(not link.is_symlink(), "non-empty plugin dir was clobbered")
        ok(not link.exists() or (link / "user-notes.txt").exists(), "user file was removed")
        ok(
            not any(a.startswith("created") or "symlink" in a for a in report["actions_taken"]),
            f"unexpected actions: {report['actions_taken']}",
        )


def test_dangling_plugin_link():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        link = home / ".hermes" / "plugins" / "aphrodite"
        link.unlink()
        link.symlink_to(Path(td) / "gone" / "aphrodite")
        report = check_and_heal(
            home_dir=home, dry_run=False, plugin_dir=src / "plugins" / "aphrodite"
        )
        ok(any("dangling" in m for m in report["mismatches"]), "dangling link not reported")
        ok(any("dangling" in w for w in report["warnings"]), "no warn-and-skip warning")
        ok(link.is_symlink() and not link.resolve().exists(), "dangling link was touched")
        ok(report["actions_taken"] == [], f"unexpected actions: {report['actions_taken']}")


def test_binary_symlink_into_plugin():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        (plugin / "binaries").mkdir()
        (plugin / "binaries" / "aphrodite").write_bytes(b"BIN\x00\x01")
        runtime_bin = home / ".hermes" / "aphrodite" / "binaries"
        runtime_bin.mkdir()
        (runtime_bin / "aphrodite").symlink_to(plugin / "binaries" / "aphrodite")
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        rb = runtime_bin / "aphrodite"
        ok(not rb.is_symlink(), "runtime binary still a symlink")
        ok(rb.read_bytes() == b"BIN\x00\x01", "runtime binary content wrong")
        # binaries/ inside the plugin dir is excluded from the scan - the
        # runtime copy is replaced from it, but the plugin dir itself is
        # never rewritten.
        ok((plugin / "binaries" / "aphrodite").exists(), "plugin binaries dir was cleaned")
        ok(any("replaced symlink" in a for a in report["actions_taken"]), "no replace action")


def test_dry_run_no_changes():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        (plugin / "aphrodite.toml").write_text("cfg\n")
        (plugin / "binaries").mkdir()
        (plugin / "binaries" / "aphrodite").write_bytes(b"BB")
        (home / ".hermes" / "aphrodite" / "aphrodite.toml").unlink()
        before = tree_snapshot(home / ".hermes")
        report = check_and_heal(home_dir=home, dry_run=True, plugin_dir=plugin)
        after = tree_snapshot(home / ".hermes")
        ok(before == after, "dry run modified the tree")
        ok(report["dry_run"] is True, "dry_run flag not reported")
        ok(len(report["mismatches"]) >= 1, f"expected mismatches, got {report['mismatches']}")
        # With the plugin dir Hermes-owned/report-only there is nothing to
        # move; a dry run plans no actions at all.
        ok(report["actions_taken"] == [], f"unexpected actions: {report['actions_taken']}")


def test_env_config_override():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        (plugin / "aphrodite.toml").write_text("env-cfg\n")
        (home / ".hermes" / "aphrodite" / "aphrodite.toml").unlink()
        custom = Path(td) / "custom" / "config.toml"
        custom.parent.mkdir()
        old = os.environ.get("APHRODITE_CONFIG_PATH")
        os.environ["APHRODITE_CONFIG_PATH"] = str(custom)
        try:
            report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        finally:
            if old is None:
                os.environ.pop("APHRODITE_CONFIG_PATH", None)
            else:
                os.environ["APHRODITE_CONFIG_PATH"] = old
        # config inside the Hermes-owned plugin dir is never moved - not even
        # to an env override target (report-only; the plugin dir is untouchable).
        ok((plugin / "aphrodite.toml").read_text() == "env-cfg\n", "config was moved out of plugin dir")
        ok(
            not any("moved" in a for a in report["actions_taken"]),
            f"plugin dir was modified: {report['actions_taken']}",
        )


def test_missing_plugin_link_not_created():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        link = home / ".hermes" / "plugins" / "aphrodite"
        link.unlink()
        report = check_and_heal(
            home_dir=home, dry_run=False, plugin_dir=src / "plugins" / "aphrodite"
        )
        # Register-time layout rewrite under <hermes-home>/plugins/ is
        # forbidden (catalog review, PR 118488) - a missing link is reported,
        # never created by the plugin.
        ok(not link.exists(), "plugin symlink was created")
        ok(
            not any("created symlink" in a for a in report["actions_taken"]),
            f"unexpected actions: {report['actions_taken']}",
        )


def test_missing_home_entirely():
    with tempfile.TemporaryDirectory() as td:
        home = Path(td) / "home"
        src = Path(td) / "plugin-src"
        plugin = src / "plugins" / "aphrodite"
        plugin.mkdir(parents=True)
        (plugin / "__init__.py").write_text("# fake\n")
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        ok((home / ".hermes" / "aphrodite").is_dir(), "runtime home not created")
        # plugin link under <hermes-home>/plugins/ is report-only - never
        # created at register time (the plugin must not touch that layout).
        ok(not (home / ".hermes" / "plugins" / "aphrodite").exists(), "plugin link was created")
        ok(
            any(a.startswith("created directory") for a in report["actions_taken"]),
            "runtime home action missing",
        )


def test_destination_differs_skipped():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        (plugin / "aphrodite.toml").write_text("plugin version\n")
        (home / ".hermes" / "aphrodite" / "aphrodite.toml").write_text("runtime version\n")
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        # plugin-dir config is never moved, compared, or warned about - the
        # plugin dir is Hermes-owned and untouchable.
        ok((plugin / "aphrodite.toml").read_text() == "plugin version\n", "plugin-dir config was modified")
        ok(
            (home / ".hermes" / "aphrodite" / "aphrodite.toml").read_text() == "runtime version\n",
            "runtime config was modified",
        )
        ok(
            not any("moved" in a for a in report["actions_taken"]),
            f"unexpected actions: {report['actions_taken']}",
        )


def test_stray_plugin_source_in_runtime_home():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        runtime = home / ".hermes" / "aphrodite"
        (runtime / "__init__.py").write_text("# OLD PLUGIN VERSION\n")
        (runtime / "plugin.yaml").write_text("name: old\n")
        (plugin / "__init__.py").write_text(
            "# CURRENT PLUGIN VERSION\n"
        )  # differs from the stray copy
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        ok(not (runtime / "__init__.py").exists(), "stray __init__.py not quarantined")
        ok(not (runtime / "plugin.yaml").exists(), "stray plugin.yaml not quarantined")
        ok(
            (runtime / ".stale-backup" / "__init__.py").read_text() == "# OLD PLUGIN VERSION\n",
            "backup missing",
        )
        ok(
            (runtime / ".stale-backup" / "plugin.yaml").read_text() == "name: old\n",
            "backup missing",
        )
        ok(any("moved" in a for a in report["actions_taken"]), "no quarantine action")


def test_stray_source_ambiguous_skipped():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        runtime = home / ".hermes" / "aphrodite"
        (runtime / "__init__.py").write_text("orphan\n")
        plugin_dir = Path(td) / "elsewhere" / "aphrodite"
        plugin_dir.mkdir(parents=True)
        (plugin_dir / "plugin.yaml").write_text("name: aphrodite\n")
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin_dir)
        ok((runtime / "__init__.py").exists(), "ambiguous stray file was removed")
        ok(any("cannot compare" in w for w in report["warnings"]), "no ambiguous warning")


def test_binaries_excluded_from_plugin_scan():
    with tempfile.TemporaryDirectory() as td:
        home, src = make_tree(Path(td))
        plugin = src / "plugins" / "aphrodite"
        runtime_bin = home / ".hermes" / "aphrodite" / "binaries"
        runtime_bin.mkdir()
        (runtime_bin / "aphrodite").write_bytes(b"OLD")
        (plugin / "binaries").mkdir()
        newer = plugin / "binaries" / "aphrodite"
        newer.write_bytes(b"NEW")
        future = time.time() + 100
        os.utime(newer, (future, future))
        report = check_and_heal(home_dir=home, dry_run=False, plugin_dir=plugin)
        # binaries/ inside the plugin dir is excluded from the scan entirely
        # (never ships there; checking it is noise) - nothing is moved,
        # warned about, or touched.
        ok((plugin / "binaries" / "aphrodite").exists(), "plugin binaries dir was touched")
        ok((runtime_bin / "aphrodite").read_bytes() == b"OLD", "runtime binary was touched")
        ok(
            not any("binaries" in w for w in report["warnings"]),
            f"unexpected binaries warnings: {report['warnings']}",
        )


def main():
    tests = [
        fn for name, fn in sorted(globals().items()) if name.startswith("test_") and callable(fn)
    ]
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
