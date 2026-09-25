#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""post_tool_call hook: format edited files using the appropriate tool.

Adapts the Aphrodite repo formatting gates (cargo fmt / shfmt / prettier /
black) for the Hermes hook pipeline. Fires after write_file or patch tool
calls - keeps the Aphrodite monorepo formatted: plugin source under
plugins/aphrodite and crate sources under crates/aphrodite and
crates/aphrodite-hermes.

Format routing by extension:
  .sh  -> shfmt (reads .editorconfig)
  .js/.ts/.jsx/.tsx/.css/.json/.md/.yaml/.yml/.html/.toml/.sql -> prettier
  .rs  -> cargo fmt (locates nearest Cargo.toml in parent dirs)
  .py  -> black (falls back to ruff)

Skips node_modules, target/, vendor/, .git/ directories.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERMES_HOME = Path(os.path.expanduser("~/.hermes"))
CACHE_DIR = HERMES_HOME / "cache" / "post-edit"
QA_TIMEOUT = 90
FORMAT_TIMEOUT = 30

FORMAT_EXTENSIONS = {
    "py": "python",
    "js": "javascript",
    "ts": "typescript",
    "jsx": "javascript",
    "tsx": "typescript",
    "css": "css",
    "json": "json",
    "md": "markdown",
    "yaml": "yaml",
    "yml": "yaml",
    "sh": "shell",
    "bash": "shell",
    "rs": "rust",
    "html": "html",
    "toml": "toml",
    "sql": "sql",
}

FORMATTERS = {
    "python": "black",
    "javascript": "prettier",
    "typescript": "prettier",
    "css": "prettier",
    "json": "prettier",
    "markdown": "prettier",
    "yaml": "prettier",
    "shell": "shfmt",
    "rust": "cargo fmt",
    "html": "prettier",
    "toml": "prettier",
    "sql": "prettier",
}

PRETTIER_PATHS = [
    "/opt/homebrew/bin/prettier",
]


def find_prettier():
    for path in PRETTIER_PATHS:
        expanded = os.path.expanduser(path)
        if os.path.exists(expanded):
            return expanded
    return None


PRETTIER_BIN = find_prettier()


def should_process(payload):
    tool_name = payload.get("tool_input", {}).get("tool_name", payload.get("tool_name", ""))
    if tool_name not in ("write_file", "patch"):
        return False
    path = payload.get("tool_input", {}).get("path", "")
    if not path:
        return False
    if ".hermes" in path and not path.endswith(".py"):
        return False
    skip_dirs = {"node_modules", "target", "dist", ".git", "vendor"}
    for d in skip_dirs:
        if d in path:
            return False
    return True


def get_lang(path):
    ext = Path(path).suffix.lstrip(".")
    return FORMAT_EXTENSIONS.get(ext.lower())


def run_shfmt(path):
    try:
        result = subprocess.run(
            ["shfmt", "-w", path], capture_output=True, text=True, timeout=FORMAT_TIMEOUT
        )
        return {"tool": "shfmt", "success": result.returncode == 0, "path": path}
    except Exception as e:
        return {"tool": "shfmt", "success": False, "error": str(e)}


def run_prettier(path):
    if not PRETTIER_BIN:
        return {"tool": "prettier", "success": False, "error": "prettier not found"}
    try:
        result = subprocess.run(
            [PRETTIER_BIN, "--write", path, "--log-level", "warn"],
            capture_output=True,
            text=True,
            timeout=FORMAT_TIMEOUT,
            cwd=os.path.dirname(path) or ".",
        )
        return {"tool": "prettier", "success": result.returncode == 0, "path": path}
    except Exception as e:
        return {"tool": "prettier", "success": False, "error": str(e)}


def run_cargo_fmt(path):
    try:
        cargo_path = None
        parent = Path(path).parent
        for _ in range(10):
            if (parent / "Cargo.toml").exists():
                cargo_path = parent
                break
            if parent == parent.parent:
                break
            parent = parent.parent
        if not cargo_path:
            return {"tool": "cargo fmt", "success": False, "error": "no Cargo.toml found"}
        result = subprocess.run(
            ["cargo", "fmt", "--", path],
            capture_output=True,
            text=True,
            timeout=FORMAT_TIMEOUT,
            cwd=str(cargo_path),
        )
        return {"tool": "cargo fmt", "success": result.returncode == 0, "path": path}
    except Exception as e:
        return {"tool": "cargo fmt", "success": False, "error": str(e)}


def run_black(path):
    try:
        result = subprocess.run(
            ["black", "-q", path], capture_output=True, text=True, timeout=FORMAT_TIMEOUT
        )
        return {"tool": "black", "success": result.returncode == 0, "path": path}
    except Exception as e:
        return {"tool": "black", "success": False, "error": str(e)}


def format_file(path, lang):
    formatter = FORMATTERS.get(lang)
    if formatter == "shfmt":
        return run_shfmt(path)
    elif formatter == "prettier":
        return run_prettier(path)
    elif formatter == "cargo fmt":
        return run_cargo_fmt(path)
    elif formatter == "black":
        return run_black(path)
    return None


def main():
    try:
        raw = sys.stdin.read()
        if not raw.strip():
            return
        payload = json.loads(raw)
    except (json.JSONDecodeError, ValueError):
        return

    if not should_process(payload):
        return

    path = payload.get("tool_input", {}).get("path", "")
    if not path or not os.path.exists(path):
        return
    path = os.path.expanduser(path)

    lang = get_lang(path)
    if not lang:
        return

    result = format_file(path, lang)
    if result:
        if result.get("success"):
            msg = f"Formatted {lang} file with {result['tool']}"
        else:
            msg = f"Format failed: {result.get('error', 'unknown error')}"
        print(json.dumps({"action": "info", "message": msg, "file": path}))


if __name__ == "__main__":
    main()
