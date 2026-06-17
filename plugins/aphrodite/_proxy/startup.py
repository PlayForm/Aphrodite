"""aphrodite - startup observability log."""

import logging
import os
import time

from .._core import BIN_VERSION, BINARY_DIR, PLUGIN_VERSION

_log = logging.getLogger("aphrodite")


def _write_startup_log(cache_ok: bool, token_ok: bool, auto_summary: str) -> None:
    """Write structured startup log to ~/.hermes/aphrodite/startup-<ts>.log."""
    ts = int(time.time())
    log_path = os.path.expanduser(f"~/.hermes/aphrodite/startup-{ts}.log")
    try:
        lines = [
            f"=== aphrodite startup [{time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(ts))}] ===",
            f"plugin_version={PLUGIN_VERSION}  binary_version={BIN_VERSION}",
            f"proxy_cache={'UP' if cache_ok else 'DOWN'}  proxy_token={'UP' if token_ok else 'DOWN'}",
            f"env: APHRODITE_DEBUG={os.environ.get('APHRODITE_DEBUG', '')}",
            f"env: QUIET={os.environ.get('QUIET', '')}",
            f"env: APHRODITE_CONTEXT_ENGINE={os.environ.get('APHRODITE_CONTEXT_ENGINE', '')}",
        ]
        if auto_summary:
            lines.append("--- auto ---")
            lines.append(auto_summary)
        with open(log_path, "w") as f:
            f.write("\n".join(lines) + "\n")
        _log.debug("startup log written: %s", log_path)
    except Exception as exc:
        _log.warning("failed to write startup log: %s", exc)
