"""aphrodite - binary download and platform detection."""

import logging
import os
import platform
import re
import ssl
import stat
import subprocess
import urllib.request

import certifi

from ._core import BIN_VERSION, BINARY, BINARY_DIR, REPO

_log = logging.getLogger("aphrodite")

# Version-check cache: skip subprocess after first successful check
_cached_version_ok: bool = False

# Valid binary magic bytes: ELF, Mach-O, PE
_BINARY_MAGICS = frozenset((
    b'\x7fELF',           # ELF
    b'\xfe\xed\xfa\xce',  # Mach-O 32-bit
    b'\xfe\xed\xfa\xcf',  # Mach-O 64-bit
    b'\xcf\xfa\xed\xfe',  # Mach-O reverse-endian 64-bit
    b'\xca\xfe\xba\xbe',  # Mach-O universal (fat binary)
    b'MZ\x90\x00',        # PE (portable executable)
))


def _restore_bak(bak):
    """Restore .bak to original binary path on download failure."""
    if os.path.exists(bak):
        try:
            os.replace(bak, bak[:-4])  # strip ".bak" → BINARY
        except Exception as e:
            _log.warning("restoring .bak failed: %s", e)


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
    # Rename existing binary to .bak so we don't leave the user with nothing on failure
    bak = BINARY + ".bak"
    if os.path.exists(BINARY):
        try:
            os.replace(BINARY, bak)
        except Exception:
            pass
    plat = _detect_platform()
    download_url = f"https://github.com/{REPO}/releases/download/{BIN_VERSION}/aphrodite-{plat}"
    _log.info("downloading aphrodite %s from %s", BIN_VERSION, download_url)
    try:
        ctx = ssl.create_default_context(cafile=certifi.where())
        with urllib.request.urlopen(download_url, timeout=30, context=ctx) as r, open(BINARY, "wb") as f:
            while True:
                chunk = r.read(65536)
                if not chunk:
                    break
                f.write(chunk)
        size = os.path.getsize(BINARY)
        if size == 0:
            _log.warning("downloaded binary is empty (0 bytes)")
            _restore_bak(bak)
            return False
        # 🛡 Magic-byte validation
        with open(BINARY, "rb") as f:
            magic = f.read(4)
        if magic not in _BINARY_MAGICS and not magic.startswith(b'MZ'):
            _log.warning("downloaded binary has invalid magic bytes: %r", magic)
            _restore_bak(bak)
            return False
        os.chmod(BINARY, os.stat(BINARY).st_mode | stat.S_IEXEC)
        if not os.access(BINARY, os.X_OK):
            _log.warning("downloaded binary is not executable after chmod")
            _restore_bak(bak)
            return False
        # Success - remove .bak
        try:
            os.unlink(bak)
        except Exception:
            pass
        _log.info("aphrodite binary installed to %s (%s bytes)", BINARY, size)
        return True
    except Exception as e:
        _log.warning("download failed: %s - falling back to cargo build", e)
        _restore_bak(bak)
        return False


def _check_binary_version() -> bool:
    """Check if the installed binary matches BIN_VERSION. Returns True if match."""
    global _cached_version_ok
    if _cached_version_ok:
        return True
    try:
        r = subprocess.run(
            [BINARY, "--version"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        version_str = r.stdout.strip() or r.stderr.strip()
        if version_str and re.search(r'\b' + re.escape(BIN_VERSION.lstrip("v")) + r'\b', version_str):
            _cached_version_ok = True
            return True
        _log.info(
            "binary version mismatch: got %r, expected %s - re-downloading",
            version_str, BIN_VERSION,
        )
    except (FileNotFoundError, subprocess.TimeoutExpired, OSError) as e:
        _log.info("binary version check failed: %s - re-downloading", e)
    return False


def _ensure_binary(existence_check: bool = False) -> bool:
    """Ensure the aphrodite binary exists and matches BIN_VERSION."""
    if existence_check:
        return bool(os.path.exists(BINARY) and os.access(BINARY, os.X_OK) and _check_binary_version())
    if os.path.exists(BINARY) and os.access(BINARY, os.X_OK):
        if _check_binary_version():
            return True
        # Rename to .bak before re-download so we can restore on failure
        bak = BINARY + ".bak"
        try:
            os.replace(BINARY, bak)
        except Exception:
            pass
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
