"""aphrodite - auto-setup functions: health, version, profile, skills, build, commit.

Each ``_auto_*()`` function returns a status string (or empty string for no-news),
allowing callers to aggregate and display a one-shot automation summary.
"""

import json
import os
import socket
import subprocess
import time
from pathlib import Path

from ._core import (
    BIN_VERSION,
    BINARY,
    BINARY_DIR,
    PORTS,
    _log,
)

# ── helpers ────────────────────────────────────────────────────────────

_ACTIVE_PROFILE: str | None = None  # cached by _get_active_profile


def _get_active_profile() -> str | None:
    """Return the Hermes profile name currently active, or None."""
    global _ACTIVE_PROFILE
    if _ACTIVE_PROFILE is not None:
        return _ACTIVE_PROFILE
    # Check environment first (most reliable when hermes launched with --profile)
    env_profile = os.environ.get("HERMES_PROFILE")
    if env_profile:
        _ACTIVE_PROFILE = env_profile
        return env_profile
    # Fallback: read ~/.hermes/current-profile
    try:
        cur = Path(os.path.expanduser("~/.hermes/current-profile")).read_text().strip()
        if cur:
            _ACTIVE_PROFILE = cur
            return cur
    except Exception:
        pass
    return None


# ── health ─────────────────────────────────────────────────────────────


def _auto_health_check() -> str:
    """curl both proxies, warn if either is down."""
    lines: list[str] = []
    health_data: dict[str, dict] = {}
    for name, port in PORTS.items():
        try:
            sock = socket.create_connection(("127.0.0.1", port), timeout=2)
            with sock:
                sock.sendall(b"GET /health HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n")
                body = b""
                while True:
                    chunk = sock.recv(4096)
                    if not chunk:
                        break
                    body += chunk
                if b"\r\n\r\n" in body:
                    body = body.split(b"\r\n\r\n", 1)[1]
            status = body.decode().strip()
            lines.append(f"  {name}:{port:>5} UP")
            health_data[name] = {"port": port, "status": "UP", "detail": status[:200]}
        except Exception:
            lines.append(f"  {name}:{port:>5} DOWN - proxy may need restart")
            health_data[name] = {"port": port, "status": "DOWN", "detail": None}
    _write_health_json(health_data)
    return "\n".join(lines) if lines else ""


# ── version ────────────────────────────────────────────────────────────


def _auto_version_check() -> str:
    """Compare running proxy -version vs plugin's BIN_VERSION."""
    if not os.path.isfile(BINARY) or not os.access(BINARY, os.X_OK):
        return "  binary: not found - install via aphrodite_rebuild or download"
    try:
        r = subprocess.run(
            [BINARY, "--version"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        running = (r.stdout or r.stderr or "").strip()
        # Dynamic label uses the one from _core that user keeps updated
        expected = BIN_VERSION
        if running and expected in running:
            return ""
        return f"  binary version: running={running[:40]}, plugin expects={expected} - mismatch (aphrodite_rebuild recommended)"
    except Exception as exc:
        return f"  binary version: check failed ({exc})"


# ── profile ────────────────────────────────────────────────────────────


def _auto_profile_check() -> str:
    """Verify model.provider is 'token' (not 'cache') for compression profiles.

    Cache proxy lacks the tool-relay endpoint required for CCR. Profiles
    named *-compress-* should use aphrodite-token.
    """
    profile_name = _get_active_profile()
    if not profile_name:
        return ""

    # Only warn for profiles that sound like compression ones
    if "-compress-" not in profile_name:
        return ""

    profile_dir = Path(os.path.expanduser(f"~/.hermes/profiles/{profile_name}/"))
    cfg = profile_dir / "config.yaml"
    if not cfg.exists():
        return ""

    try:
        text = cfg.read_text()
        # Cheap YAML parse - look for model.provider line
        for line in text.splitlines():
            stripped = line.strip()
            if stripped.startswith("provider:"):
                provider_val = stripped.split(":", 1)[1].strip()
                if "cache" in provider_val.lower():
                    return (
                        f"  profile: {profile_name} uses provider={provider_val} "
                        f"(cache proxy - no tool relay). "
                        f"Set provider: aphrodite-token for full compression support."
                    )
                if "token" in provider_val.lower():
                    return ""
                return f"  profile: {profile_name} provider={provider_val}"
    except Exception:
        pass
    return ""


# ── skills ─────────────────────────────────────────────────────────────


def _auto_skills_load() -> str:
    """Log skill availability for this session. Doesn't auto-register -
    Hermes plugin registration already bundles them via register().
    We only report which are available."""
    skills_dir = Path(__file__).parent / "skills"
    names = sorted(d.name for d in skills_dir.iterdir() if d.is_dir() and (d / "SKILL.md").exists())
    if not names:
        return "  skills: none bundled"
    return ""


# ── build watch ────────────────────────────────────────────────────────


def _auto_build_watch() -> str:
    """Check .hermes/build-status.json and note latest build state."""
    for candidate in (
        Path(BINARY_DIR).resolve().parent.parent / ".hermes" / "build-status.json",
        Path(os.path.expanduser("~/.hermes/build-status.json")),
    ):
        if candidate.exists():
            try:
                data = json.loads(candidate.read_text())
                status = data.get("status", "?")
                last = data.get("last_build", "?")
                ver = data.get("version", "?")
                errors = data.get("errors", [])
                parts = [f"build: {ver} ({last}) - {status}"]
                if errors:
                    parts.append(f"  errors: {len(errors)}")
                return "  " + " | ".join(parts)
            except Exception:
                pass
    return ""


# ── commit reminder ────────────────────────────────────────────────────


def _auto_commit_reminder() -> str:
    """If there are uncommitted changes, return a short catalog-style line."""
    repo = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    try:
        r = subprocess.run(
            ["git", "status", "--porcelain"],
            capture_output=True,
            text=True,
            timeout=3,
            cwd=repo,
        )
        lines = [l for l in r.stdout.strip().splitlines() if l.strip()]
        if not lines:
            return ""
        # Count unique modified files
        modified = len(lines)
        return f"  git: {modified} uncommitted file{'s' if modified != 1 else ''} (use cc/gcommit)"
    except Exception:
        return ""


def _write_health_json(health_data: dict) -> None:
    """Write structured health snapshot to ~/.hermes/aphrodite/health-<ts>.json."""
    ts = int(time.time())
    json_path = os.path.expanduser(f"~/.hermes/aphrodite/health-{ts}.json")
    try:
        snapshot = {
            "timestamp": ts,
            "iso": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(ts)),
            "proxies": health_data,
        }
        with open(json_path, "w") as f:
            json.dump(snapshot, f, indent=2)
        _log.debug("health json written: %s (%d proxies)", json_path, len(health_data))
    except Exception as exc:
        _log.warning("health json write failed: %s", exc)


# ── aggregate ──────────────────────────────────────────────────────────


def run_all() -> str:
    """Run all auto checks and return a multi-line summary string.

    Each function that has something to report appends its line(s).
    The caller (on_start in _proxy.py) should log DEBUG + print this.
    """
    checks = [
        ("health", _auto_health_check()),
        ("version", _auto_version_check()),
        ("profile", _auto_profile_check()),
        ("skills", _auto_skills_load()),
        ("build", _auto_build_watch()),
        ("commit", _auto_commit_reminder()),
    ]

    results = []
    for label, text in checks:
        if text:
            results.append(text)
            _log.debug("[auto:%s] %s", label, text.replace("\n", " | "))

    if not results:
        return ""

    summary = "── [AUTO] ──────────────────────────────────────────"
    for r in results:
        summary += "\n" + r
    return summary
