"""
Unit + integration tests for hermes-tool-fix plugin.

Patch 1: terminal_tool — monitors empty output on exit 0 (logs warning)
Patch 2: read_file_tool — recovers empty content via direct file I/O
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch


# ─── Unit Tests: terminal_tool patch ──────────────────────────────

class TerminalToolUnitTests(unittest.TestCase):
    """Unit tests for the terminal_tool monkey-patch."""

    def setUp(self):
        # Create a mock original terminal_tool
        self.mock_orig = MagicMock()
        self.log_messages = []

        # Recreate the patched wrapper inline for testability
        import functools

        log_messages = self.log_messages

        @functools.wraps(self.mock_orig)
        def patched(
            command: str,
            background: bool = False,
            timeout=None,
            task_id: str = None,
            force: bool = False,
            workdir: str = None,
            pty: bool = False,
            notify_on_complete: bool = False,
            watch_patterns=None,
        ) -> str:
            result = self.mock_orig(
                command=command,
                background=background,
                timeout=timeout,
                task_id=task_id,
                force=force,
                workdir=workdir,
                pty=pty,
                notify_on_complete=notify_on_complete,
                watch_patterns=watch_patterns,
            )
            try:
                data = json.loads(result)
            except (json.JSONDecodeError, TypeError):
                return result

            output = str(data.get("output", ""))
            exit_code = data.get("exit_code", 0)

            if exit_code == 0 and not output.strip() and command.strip():
                log_messages.append(f"WARNING: terminal exit=0 but empty output for: {command[:120]}")

            return result

        self.patched = patched

    def test_passes_through_normal_output(self):
        """Normal terminal output passes through unchanged."""
        self.mock_orig.return_value = json.dumps({
            "output": "Hello World\n",
            "exit_code": 0,
        })
        result = self.patched("echo hello")
        self.assertIn("Hello World", result)
        self.assertNotIn("WARNING", result)
        self.assertEqual(len(self.log_messages), 0)

    def test_empty_output_exit_zero_triggers_warning(self):
        """Empty output with exit_code 0 triggers warning log."""
        self.mock_orig.return_value = json.dumps({
            "output": "",
            "exit_code": 0,
        })
        result = self.patched("some-command --quiet")
        self.assertEqual(len(self.log_messages), 1)
        self.assertIn("some-command", self.log_messages[0])

    def test_empty_output_exit_nonzero_no_warning(self):
        """Empty output with non-zero exit_code does NOT trigger warning."""
        self.mock_orig.return_value = json.dumps({
            "output": "",
            "exit_code": 1,
        })
        result = self.patched("failing-cmd")
        self.assertEqual(len(self.log_messages), 0)

    def test_non_json_response_passes_through(self):
        """Non-JSON responses (e.g., raw text) pass through unchanged."""
        self.mock_orig.return_value = "plain text error"
        result = self.patched("bad-command")
        self.assertEqual(result, "plain text error")
        self.assertEqual(len(self.log_messages), 0)

    def test_empty_command_skips_warning(self):
        """Empty/whitespace command does not trigger warning."""
        self.mock_orig.return_value = json.dumps({
            "output": "",
            "exit_code": 0,
        })
        result = self.patched("   ")
        self.assertEqual(len(self.log_messages), 0)


# ─── Unit Tests: read_file_tool patch ─────────────────────────────

class ReadFileToolUnitTests(unittest.TestCase):
    """Unit tests for the read_file_tool monkey-patch."""

    def setUp(self):
        self.mock_orig = MagicMock()
        self.log_messages = []
        import functools

        log_messages = self.log_messages

        @functools.wraps(self.mock_orig)
        def patched(
            path: str, offset: int = 1, limit: int = 500, task_id: str = "default"
        ) -> str:
            result = self.mock_orig(path=path, offset=offset, limit=limit, task_id=task_id)

            try:
                data = json.loads(result)
            except (json.JSONDecodeError, TypeError):
                return result

            if "error" in data:
                return result

            content = data.get("content", "")
            total_lines = data.get("total_lines", 0)

            if not content and total_lines and total_lines > 0:
                log_messages.append(f"WARNING: read_file content empty but total_lines={total_lines} for {path}")
                try:
                    with open(path, "r", encoding="utf-8") as f:
                        lines = f.readlines()
                    start = max(0, offset - 1)
                    end = min(len(lines), start + limit)
                    data["content"] = "".join(
                        f"{i+1}|{line}" for i, line in enumerate(lines[start:end], start=start)
                    )
                    data["_fixed_by"] = "hermes-tool-fix"
                    return json.dumps(data, ensure_ascii=False)
                except Exception:
                    pass

            return result

        self.patched = patched

    def test_passes_through_normal_content(self):
        """Normal content passes through unchanged."""
        self.mock_orig.return_value = json.dumps({
            "content": "1|line one\n2|line two\n",
            "total_lines": 2,
        })
        result = self.patched("/some/file.txt")
        self.assertIn("line one", result)
        self.assertNotIn("_fixed_by", result)

    def test_empty_content_with_lines_triggers_recovery(self):
        """Empty content with total_lines > 0 triggers recovery via direct I/O."""
        import tempfile
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
            f.write("alpha\nbeta\ngamma\n")
            tmp_path = f.name

        try:
            self.mock_orig.return_value = json.dumps({
                "content": "",
                "total_lines": 3,
            })
            result = self.patched(tmp_path)
            data = json.loads(result)
            self.assertIn("alpha", data["content"])
            self.assertIn("beta", data["content"])
            self.assertIn("gamma", data["content"])
            self.assertEqual(data["_fixed_by"], "hermes-tool-fix")
            self.assertEqual(len(self.log_messages), 1)
        finally:
            os.unlink(tmp_path)

    def test_content_present_no_recovery(self):
        """Content present — no recovery needed, no _fixed_by."""
        self.mock_orig.return_value = json.dumps({
            "content": "1|hello\n",
            "total_lines": 1,
        })
        result = self.patched("/some/other.txt")
        data = json.loads(result)
        self.assertNotIn("_fixed_by", data)

    def test_error_response_passes_through(self):
        """Error response passes through unchanged."""
        self.mock_orig.return_value = json.dumps({
            "error": "File not found",
        })
        result = self.patched("/nonexistent")
        self.assertIn("File not found", result)
        self.assertEqual(len(self.log_messages), 0)

    def test_total_lines_zero_no_trigger(self):
        """total_lines=0 (empty file) does not trigger recovery."""
        self.mock_orig.return_value = json.dumps({
            "content": "",
            "total_lines": 0,
        })
        result = self.patched("/empty_file")
        self.assertNotIn("_fixed_by", result)
        self.assertEqual(len(self.log_messages), 0)

    def test_non_json_passes_through(self):
        """Non-JSON responses pass through unchanged."""
        self.mock_orig.return_value = "some error string"
        result = self.patched("/bad-path")
        self.assertEqual(result, "some error string")

    def test_offset_and_limit_respected_in_recovery(self):
        """Recovered content honors offset and limit parameters."""
        import tempfile
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
            for i in range(10):
                f.write(f"line {i+1}\n")
            tmp_path = f.name

        try:
            self.mock_orig.return_value = json.dumps({
                "content": "",
                "total_lines": 10,
            })
            result = self.patched(tmp_path, offset=4, limit=3)
            data = json.loads(result)
            content = data["content"]
            self.assertIn("line 4", content)
            self.assertIn("line 5", content)
            self.assertIn("line 6", content)
            self.assertNotIn("line 3", content)
            self.assertNotIn("line 7", content)
        finally:
            os.unlink(tmp_path)

    def test_recovery_marks_result(self):
        """Recovered result includes _fixed_by marker."""
        import tempfile
        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
            f.write("just one line\n")
            tmp_path = f.name

        try:
            self.mock_orig.return_value = json.dumps({
                "content": "",
                "total_lines": 1,
            })
            result = self.patched(tmp_path)
            data = json.loads(result)
            self.assertEqual(data["_fixed_by"], "hermes-tool-fix")
        finally:
            os.unlink(tmp_path)


# ─── Integration Tests ────────────────────────────────────────────

class PluginIntegrationTests(unittest.TestCase):
    """Integration tests using the actual plugin in a hermetic environment."""

    @classmethod
    def setUpClass(cls):
        # Ensure we can import the plugin module
        sys.path.insert(0, os.path.join(
            os.path.dirname(__file__), "..", "plugins", "hermes-tool-fix"
        ))
        import __init__ as plugin_mod
        cls.plugin_mod = plugin_mod

    def test_import_and_signatures(self):
        """Plugin imports cleanly and exposes register()."""
        self.assertTrue(hasattr(self.plugin_mod, "register"))
        self.assertTrue(callable(self.plugin_mod.register))
        self.assertTrue(hasattr(self.plugin_mod, "_patch_terminal_tool"))
        self.assertTrue(hasattr(self.plugin_mod, "_patch_read_file_tool"))

    def test_debug_env_controls_logging(self):
        """HERMES_TOOL_FIX_DEBUG env var controls debugging."""
        # Reset the module's DEBUG flag via reload
        import importlib
        # Ensure off by default
        with patch.dict(os.environ, {}, clear=True):
            importlib.reload(self.plugin_mod)
            self.assertFalse(self.plugin_mod.DEBUG)
        # Enable via env
        with patch.dict(os.environ, {"HERMES_TOOL_FIX_DEBUG": "1"}):
            importlib.reload(self.plugin_mod)
            self.assertTrue(self.plugin_mod.DEBUG)

    def test_register_applies_patches(self):
        """register() attempts to apply both patches (graceful if
        tools module not available outside Hermes runtime)."""
        mock_ctx = MagicMock()
        # register should not raise even when tools aren't importable
        try:
            self.plugin_mod.register(mock_ctx)
        except Exception as e:
            # Only acceptable failure is import error for tools module
            msg = str(e).lower()
            self.assertTrue("tools" in msg or "import" in msg)


if __name__ == "__main__":
    unittest.main(verbosity=2)
