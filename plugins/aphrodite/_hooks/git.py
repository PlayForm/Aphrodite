"""aphrodite — git summary, auto-commit reminder, auto-build watch."""

import logging
import os
import time

from .._core import _git_cache

_log = logging.getLogger("aphrodite.hooks.git")


def _git_summary(cwd: str | None = None):
    """Get cached git diff --stat summary. Returns string or None.
    ``cwd`` defaults to the session's current working directory."""
    if cwd is None:
        cwd = os.getcwd()
    now = time.time()
    if _git_cache.get("ts", 0) > now - 30:
        return _git_cache.get("summary")
    try:
        import subprocess

        r = subprocess.run(["git", "diff", "--stat"], capture_output=True, text=True, timeout=3, cwd=cwd)
        if r.returncode == 0 and r.stdout.strip():
            summary = r.stdout.strip().split("\n")[-1] if r.stdout.strip() else None
            _git_cache["ts"] = now
            _git_cache["summary"] = summary
            return summary
    except Exception as exc:
        _log.debug("_git_summary: %s", exc)
    return None
