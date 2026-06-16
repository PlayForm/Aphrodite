"""aphrodite - binary download and platform detection."""

import logging
import os
import platform
import stat
import subprocess
import urllib.request

from ._core import BIN_VERSION, BINARY, BINARY_DIR, REPO

_log = logging.getLogger("aphrodite")


def _detect_platform() -> str:
    """Return platform tag for download URL."""
    system = platform.system().lower()
    machine = platform.machine().lower()
    if system == "darwin":
        return "macos-arm64" if machine in ("arm64", "aarch64") else "macos-x64"
    elif system == "linux":
        return "linux-x64" if machine == "x86_64" else "linux-arm64"
    return f"{system}-{machine}"


def _download_binary() -> bool:
    """Download aphrodite binary from GitHub releases."""
    os.makedirs(BINARY_DIR, exist_ok=True)
    plat = _detect_platform()
    download_url = f"https://github.com/{REPO}/releases/download/{BIN_VERSION}/aphrodite-{plat}"
    _log.info("downloading aphrodite %s from %s", BIN_VERSION, download_url)
    try:
        with urllib.request.urlopen(download_url, timeout=30) as r:
            with open(BINARY, "wb") as f:
                f.write(r.read())
        size = os.path.getsize(BINARY)
        if size == 0:
            _log.warning("downloaded binary is empty (0 bytes)")
            return False
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        if not os.access(BINARY, os.X_OK):
            _log.warning("downloaded binary is not executable after chmod")
            return False
        _log.info("aphrodite binary installed to %s (%s bytes)", BINARY, size)
        return True
    except Exception as e:
        _log.warning("download failed: %s - falling back to cargo build", e)
        return False


def _check_binary_version() -> bool:
    """Check if the installed binary matches BIN_VERSION. Returns True if match."""
    try:
        r = subprocess.run(
            [BINARY, "--version"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        version_str = r.stdout.strip() or r.stderr.strip()
        if version_str and BIN_VERSION.lstrip("v") in version_str:
            return True
        _log.info(
            "binary version mismatch: got %r, expected %s - re-downloading",
            version_str, BIN_VERSION,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError) as e:
        _log.info("binary version check failed: %s - re-downloading", e)
    return False


def _ensure_binary() -> bool:
    """Ensure the aphrodite binary exists and matches BIN_VERSION."""
    if os.path.exists(BINARY) and os.access(BINARY, os.X_OK):
        if _check_binary_version():
            return True
        os.remove(BINARY)
    if _download_binary():
        return True
    # Fallback: try local build
    repo_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    local_bin = os.path.join(repo_dir, "target", "release", "aphrodite")
    if os.path.exists(local_bin):
        import shutil

        shutil.copy2(local_bin, BINARY)
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        _log.info("copied local binary to %s", BINARY)
        return True
    _log.error("no binary found - install cargo or download manually from %s/releases", REPO)
    return False
