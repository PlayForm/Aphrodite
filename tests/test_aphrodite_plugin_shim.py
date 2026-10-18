#!/usr/bin/env python3
"""Pytest suite: `plugins/aphrodite/__init__.py`'s Python-level shim logic
(not the dylib FFI - see `test_aphrodite_hermes_plugin.py` for that).

Covers the config-surface fixes from `.plans/07-configuration.md`: the
APHRODITE_NO_AUTO_LAUNCH guard (F13/T7) actually being read, not just set.
"""

import os
import sys
from pathlib import Path
from unittest import mock

ROOT = os.path.join(os.path.dirname(__file__), "..")
sys.path.insert(0, ROOT)

import plugins.aphrodite as aphrodite_plugin  # noqa: E402


def test_no_auto_launch_env_skips_proxy_spawn(monkeypatch):
    monkeypatch.setenv("APHRODITE_NO_AUTO_LAUNCH", "1")
    with mock.patch("subprocess.Popen") as popen:
        aphrodite_plugin._start_proxy()
    popen.assert_not_called()


def test_no_auto_launch_true_string_also_skips(monkeypatch):
    monkeypatch.setenv("APHRODITE_NO_AUTO_LAUNCH", "true")
    with mock.patch("subprocess.Popen") as popen:
        aphrodite_plugin._start_proxy()
    popen.assert_not_called()


class _FailedDownloadPopen:
    """Minimal subprocess.Popen stand-in for the auto-download bootstrap.

    subprocess.run (the download.sh runner inside _ensure_binaries) drives
    Popen through __enter__/communicate/poll and unpacks a (stdout, stderr)
    tuple; a bare MagicMock breaks that unpack. returncode=1 makes
    _ensure_binaries take its download-failed branch: no real download, no
    raise, and _start_proxy proceeds to the binary not-found branch.
    """

    returncode = 1

    def __init__(self, *args, **kwargs):
        self.args = args

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        return False

    def communicate(self, *args, **kwargs):
        return (b"", b"")

    def poll(self):
        return self.returncode


def test_auto_launch_proceeds_when_unset(monkeypatch):
    monkeypatch.delenv("APHRODITE_NO_AUTO_LAUNCH", raising=False)
    # register() never downloads by default (catalog review, PR 118488);
    # APHRODITE_AUTO_DOWNLOAD=1 is the legacy escape hatch that still runs
    # download.sh. This test drives that opt-in path: with the binary
    # missing, _start_proxy runs download.sh BEFORE deciding there is
    # nothing to launch.
    monkeypatch.setenv("APHRODITE_AUTO_DOWNLOAD", "1")
    # Point at a binary that doesn't exist so _start_proxy returns early
    # via its own not-found branch, without this test needing to spawn or
    # mock a real process. Also isolate from any real install under
    # ~/.hermes/aphrodite so the auto-download bootstrap is the only way a
    # binary could appear.
    monkeypatch.setattr(aphrodite_plugin, "_BINARY_PATH", "/nonexistent/aphrodite-binary")
    monkeypatch.setattr(
        aphrodite_plugin, "_BINARIES_DIR", Path("/nonexistent/aphrodite-home/binaries")
    )
    with mock.patch("subprocess.Popen", side_effect=_FailedDownloadPopen) as popen:
        with mock.patch.object(aphrodite_plugin, "_check_version_published", lambda: None):
            aphrodite_plugin._start_proxy()

    # Auto-download bootstrap: with the binary missing, _start_proxy runs
    # download.sh (via _ensure_binaries' subprocess.run, which constructs a
    # Popen) BEFORE deciding there is nothing to launch...
    assert popen.call_count == 1
    cmd = popen.call_args[0][0]
    assert cmd[0] == "bash"
    assert "download.sh" in cmd[1]
    # ...and the not-found branch returns WITHOUT spawning the proxy binary
    # (call_count == 1 already proves no second Popen for the launch).


def test_missing_binaries_never_download_without_opt_in(monkeypatch):
    # Catalog review (PR 118488): register()/_start_proxy NEVER downloads -
    # a download is an explicit setup step (`aphrodite setup` /
    # `bash download.sh`). Without APHRODITE_AUTO_DOWNLOAD, a missing
    # binary must produce no download attempt at all (no Popen), just the
    # warn-and-return path.
    monkeypatch.delenv("APHRODITE_NO_AUTO_LAUNCH", raising=False)
    monkeypatch.delenv("APHRODITE_AUTO_DOWNLOAD", raising=False)
    monkeypatch.setattr(aphrodite_plugin, "_BINARY_PATH", "/nonexistent/aphrodite-binary")
    monkeypatch.setattr(
        aphrodite_plugin, "_BINARIES_DIR", Path("/nonexistent/aphrodite-home/binaries")
    )
    with mock.patch("subprocess.Popen") as popen:
        aphrodite_plugin._start_proxy()
    popen.assert_not_called()


def test_parse_port_env_missing_returns_default(monkeypatch):
    monkeypatch.delenv("APHRODITE_CACHE_PORT", raising=False)
    assert aphrodite_plugin._parse_port_env("APHRODITE_CACHE_PORT", 9797) == 9797


def test_parse_port_env_valid_value(monkeypatch):
    monkeypatch.setenv("APHRODITE_CACHE_PORT", "19797")
    assert aphrodite_plugin._parse_port_env("APHRODITE_CACHE_PORT", 9797) == 19797


def test_parse_port_env_malformed_value_falls_back_without_raising(monkeypatch):
    monkeypatch.setenv("APHRODITE_CACHE_PORT", "not-a-port")
    assert aphrodite_plugin._parse_port_env("APHRODITE_CACHE_PORT", 9797) == 9797
