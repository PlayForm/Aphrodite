"""aphrodite - proxy environment loading and guidance."""

import logging
import os

from .._core import ENV_FILE

_log = logging.getLogger("aphrodite")

# ── Proxy environment keys (whitelist) ──────────────────────
_PROXY_ENV_KEYS = {"PATH", "HOME", "APHRODITE_API_KEY", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH", "SSL_CERT_FILE", "TMPDIR", "TMP", "TEMP"}

# ── Auto-expand guidance (set by on_start after proxy launch) ──
_expand_guidance: str = ""


def _load_env() -> dict[str, str]:
    """Load .env file into a dict."""
    env = {}
    try:
        with open(ENV_FILE) as f:
            for line in f:
                line = line.strip()
                if line.startswith("export "):
                    kv = line[7:].split("=", 1)
                    if len(kv) == 2:
                        env[kv[0]] = _env_val(kv[1], kv[0])
                elif "=" in line and not line.startswith("#"):
                    kv = line.split("=", 1)
                    env[kv[0]] = _env_val(kv[1], kv[0])
    except Exception as exc:
        _log.warning("_load_env: failed to read %s - %s", ENV_FILE, exc)
    return env


def _env_val(val: str, key_name: str = "") -> str:
    """Parse a .env value: extract between matching quotes or strip inline # comment.

    If ``key_name`` is supplied and the value contains ``#`` followed by hex-like
    characters (common in truncated API keys), a warning is logged instead of silently
    stripping what may be part of the credential.
    """
    val = val.strip()
    if val.startswith('"'):
        end = val.find('"', 1)
        if end != -1:
            return val[1:end]
    elif val.startswith("'"):
        end = val.find("'", 1)
        if end != -1:
            return val[1:end]
    # Unquoted: split on # to remove inline comment
    if "#" in val:
        before, after = val.split("#", 1)
        after_stripped = after.strip()
        # If the suffix looks like a credential fragment (≥4 hex-like chars) warn
        if key_name and after_stripped and len(after_stripped) >= 4:
            _log.warning(
                "_env_val: %s contains '#' followed by '%s...' - "
                "possible key truncation, consider quoting the value",
                key_name,
                after_stripped[:3],
            )
        return before.strip()
    return val


def _inject_expand_guidance() -> str:
    """Return auto-expand guidance string explaining that tool CCR markers are resolved inline."""
    return (
        "💋 Tool outputs are auto-expanded — you see full content inline, "
        "no <<<CCR:...>>> markers for tool results. "
        "If you see a CCR marker, retrieve only if the preview hints at useful content."
    )
