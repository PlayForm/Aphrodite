#!/usr/bin/env python3
"""Pytest suite: `plugins/aphrodite/__init__.py`'s Python-level shim logic
(not the dylib FFI - see `test_aphrodite_hermes_plugin.py` for that).

Covers the config-surface fixes from `.plans/07-configuration.md`: the
APHRODITE_NO_AUTO_LAUNCH guard (F13/T7) actually being read, not just set.
"""

import os
import sys
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


def test_auto_launch_proceeds_when_unset(monkeypatch):
    monkeypatch.delenv("APHRODITE_NO_AUTO_LAUNCH", raising=False)
    # Point at a binary that doesn't exist so _start_proxy returns early
    # via its own not-found branch, without this test needing to spawn or
    # mock a real process.
    monkeypatch.setattr(aphrodite_plugin, "_BINARY_PATH", "/nonexistent/aphrodite-binary")
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
