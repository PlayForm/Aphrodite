"""aphrodite — binary download and platform detection."""
import os
import stat
import platform
import urllib.request
import logging
from ._core import BINARY, BINARY_DIR, BIN_VERSION, REPO

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
        urllib.request.urlretrieve(download_url, BINARY)
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        _log.info("aphrodite binary installed to %s", BINARY)
        return True
    except Exception as e:
        _log.warning("download failed: %s - falling back to cargo build", e)
        return False


def _ensure_binary() -> bool:
    """Ensure the aphrodite binary exists, downloading if needed."""
    if os.path.exists(BINARY) and os.access(BINARY, os.X_OK):
        return True
    if _download_binary():
        return True
    # Fallback: try local build
    repo_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    local_bin = os.path.join(repo_dir, "crates", "aphrodite", "target", "release", "aphrodite")
    if os.path.exists(local_bin):
        import shutil
        shutil.copy2(local_bin, BINARY)
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        _log.info("copied local binary to %s", BINARY)
        return True
    _log.error("no binary found - install cargo or download manually from %s/releases", REPO)
    return False
