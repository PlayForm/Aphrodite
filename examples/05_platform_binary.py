"""Atomic test 05 — _detect_platform() unused in binary download URL.

Bug:  _download_binary() builds the URL as
        .../releases/download/{version}/aphrodite
      ignoring the platform tag entirely, so on Linux x64 it still downloads
      the macOS arm64 binary (or 404s if releases are per-platform).
Fix:  Embed the platform tag in the filename.

Run:  python examples/05_platform_binary.py
Pass: prints OK
"""
import sys
import platform

# ---------- replica of _detect_platform ----------

def _detect_platform() -> str:
    system = sys.platform          # 'darwin', 'linux', 'win32'
    machine = platform.machine()   # 'x86_64', 'arm64', 'AMD64'
    mapping = {
        ("darwin",  "arm64"):  "macos-arm64",
        ("darwin",  "x86_64"): "macos-x64",
        ("linux",   "x86_64"): "linux-x64",
        ("linux",   "aarch64"):"linux-arm64",
        ("win32",   "AMD64"):  "windows-x64",
    }
    return mapping.get((system, machine), "linux-x64")  # safe fallback

# ---------- buggy URL builder ----------

REPO = "PlayForm/Aphrodite"
BIN_VERSION = "v0.2.0"

def _download_url_buggy() -> str:
    return f"https://github.com/{REPO}/releases/download/{BIN_VERSION}/aphrodite"

# ---------- fixed URL builder ----------

def _download_url_fixed() -> str:
    plat = _detect_platform()
    return f"https://github.com/{REPO}/releases/download/{BIN_VERSION}/aphrodite-{plat}"

# ---------- assertions ----------

buggy_url = _download_url_buggy()
fixed_url = _download_url_fixed()
plat = _detect_platform()

assert not buggy_url.endswith(("-arm64", "-x64", "-windows")), \
    "Buggy URL must have no platform suffix"
assert plat in fixed_url, f"Fixed URL must embed platform '{plat}'"

print("05 OK — platform tag integrated into download URL")
print(f"  current platform : {plat}")
print(f"  buggy URL : {buggy_url}")
print(f"  fixed URL : {fixed_url}")
