"""
Auto-update for hermes-compress.

Checks GitHub releases for newer versions, downloads and installs
updates. Configurable: auto-check on startup, notification only,
or full auto-update.

Usage:
    from hermes_compress._update import check_for_updates, UpdateResult
    result = check_for_updates()
    if result.update_available:
        print(f"v{result.latest_version} available")
"""

from __future__ import annotations

import json
import logging
import os
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional
from urllib.request import Request, urlopen

logger = logging.getLogger(__name__)

GITHUB_API = "https://api.github.com/repos/PlayForm/HermesCompress/releases/latest"
UPDATE_CHECK_FILE = Path.home() / ".hermes" / "plugins" / "hermes-compress" / ".last_update_check"


@dataclass
class UpdateResult:
    update_available: bool = False
    current_version: str = ""
    latest_version: str = ""
    release_url: str = ""
    release_notes: str = ""
    error: Optional[str] = None
    checked_at: float = field(default_factory=time.time)


def get_current_version() -> str:
    try:
        from hermes_compress import __version__
        return __version__
    except ImportError:
        return "0.0.0"


def check_for_updates(force: bool = False) -> UpdateResult:
    """Check GitHub for newer releases.

    Caches results for 1 hour unless force=True.
    """
    current = get_current_version()

    # Cache: skip if checked recently
    if not force and UPDATE_CHECK_FILE.exists():
        try:
            age = time.time() - UPDATE_CHECK_FILE.stat().st_mtime
            if age < 3600:  # 1 hour
                return UpdateResult(
                    update_available=False,
                    current_version=current,
                    checked_at=UPDATE_CHECK_FILE.stat().st_mtime,
                )
        except Exception:
            pass

    result = UpdateResult(current_version=current)

    try:
        req = Request(GITHUB_API, headers={
            "Accept": "application/vnd.github.v3+json",
            "User-Agent": "hermes-compress-auto-update",
        })
        with urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read().decode())

        latest_tag = data.get("tag_name", "").lstrip("v")
        result.latest_version = latest_tag
        result.release_url = data.get("html_url", "")
        result.release_notes = data.get("body", "")[:500]

        if _version_greater(latest_tag, current):
            result.update_available = True
            logger.info("hermes-compress: update available v%s -> v%s", current, latest_tag)
        else:
            logger.debug("hermes-compress: up to date (v%s)", current)

    except Exception as e:
        result.error = str(e)
        logger.debug("hermes-compress: update check failed: %s", e)

    # Update cache
    try:
        UPDATE_CHECK_FILE.parent.mkdir(parents=True, exist_ok=True)
        UPDATE_CHECK_FILE.write_text(json.dumps({
            "checked_at": time.time(),
            "version": current,
            "latest": result.latest_version,
        }))
    except Exception:
        pass

    return result


def install_update() -> bool:
    """Download and install the latest version from GitHub.

    Uses pip to install from the GitHub repo directly.
    Returns True on success.
    """
    result = check_for_updates(force=True)
    if not result.update_available:
        logger.info("hermes-compress: already at latest version")
        return False

    try:
        logger.info("hermes-compress: installing v%s...", result.latest_version)
        subprocess.check_call([
            sys.executable, "-m", "pip", "install", "--upgrade",
            f"git+https://github.com/PlayForm/HermesCompress.git@v{result.latest_version}",
        ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

        # Re-run install to patch hermes-agent
        try:
            from hermes_compress._install import install
            install()
        except Exception:
            pass

        logger.info("hermes-compress: updated to v%s", result.latest_version)
        return True

    except Exception as e:
        logger.error("hermes-compress: update failed: %s", e)
        return False


def auto_update_check() -> Optional[UpdateResult]:
    """Check for updates on plugin load. Non-blocking, silent on failure."""
    try:
        # Only check if enabled in config or env
        if os.getenv("HERMES_COMPRESS_NO_UPDATE"):
            return None
        return check_for_updates()
    except Exception:
        return None


def _version_greater(a: str, b: str) -> bool:
    """Compare semantic versions. Returns True if a > b."""
    try:
        parts_a = [int(x) for x in a.split(".")]
        parts_b = [int(x) for x in b.split(".")]
        # Pad to same length
        while len(parts_a) < len(parts_b):
            parts_a.append(0)
        while len(parts_b) < len(parts_a):
            parts_b.append(0)
        return parts_a > parts_b
    except (ValueError, AttributeError):
        return a != b  # fallback: string compare
