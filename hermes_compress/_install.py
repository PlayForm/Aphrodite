"""
Post-install patcher - modifies hermes-agent core to inject headroom compression.

Idempotent: safe to run multiple times. Creates backups before patching.

Patches applied:
  1. Copy agent/headroom_compression.py → hermes-agent/agent/
  2. Patch agent/conversation_loop.py - compress api_messages before LLM call
  3. Patch agent/agent_init.py - init HeadroomCompressor from config
  4. Patch agent/agent_runtime_helpers.py - update model on switch
"""

from __future__ import annotations

import filecmp
import logging
import os
import shutil
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

logger = logging.getLogger(__name__)

# ── Sentinel markers (unique, won't appear in normal code) ──────────
MARKER = "# <hermes-compress:installed>"

# ── Patches ──────────────────────────────────────────────────────────

CONVERSATION_LOOP_PATCH = (
    "        _sanitize_messages_surrogates(api_messages)",
    """        _sanitize_messages_surrogates(api_messages)

        # ── Headroom in-process compression ────────────────────────── {marker}
        _hr = getattr(agent, "headroom_compressor", None)
        if _hr is not None and _hr.enabled:
            try:
                _hr_result = _hr.compress(api_messages)
                if _hr_result.compressed:
                    api_messages = _hr_result.messages
            except Exception:
                pass
""".format(marker=MARKER),
)

AGENT_INIT_PATCH = (
    "    agent.compression_enabled = compression_enabled",
    """    agent.compression_enabled = compression_enabled

    # ── Headroom in-process compression ────────────────────────────── {marker}
    _headroom_cfg = _compression_cfg.get("headroom", {{}})
    if not isinstance(_headroom_cfg, dict):
        _headroom_cfg = {{}}
    _hr_enabled = str(_headroom_cfg.get("enabled", False)).lower() in {{"true", "1", "yes"}}
    _hr_mode = str(_headroom_cfg.get("mode", "token")).lower()
    _hr_protect = int(_headroom_cfg.get("protect_recent", 4))
    _hr_target = _headroom_cfg.get("target_ratio")
    if _hr_target is not None:
        try:
            _hr_target = float(_hr_target)
        except (TypeError, ValueError):
            _hr_target = None
    _hr_min_tokens = int(_headroom_cfg.get("min_tokens_to_compress", 250))
    _hr_precompress = str(_headroom_cfg.get("precompress_tools", False)).lower() in {{"true", "1", "yes"}}
    _hr_aggressive = str(_headroom_cfg.get("aggressive_kompress", False)).lower() in {{"true", "1", "yes"}}
    _hr_dedup = str(_headroom_cfg.get("deduplicate_results", False)).lower() in {{"true", "1", "yes"}}
    _hr_verbose = str(_headroom_cfg.get("verbose_stats", False)).lower() in {{"true", "1", "yes"}}
    try:
        from agent.headroom_compression import HeadroomCompressor
        agent.headroom_compressor = HeadroomCompressor(
            model=agent.model,
            enabled=_hr_enabled,
            mode=_hr_mode,
            protect_recent=_hr_protect,
            target_ratio=_hr_target,
            min_tokens_to_compress=_hr_min_tokens,
            precompress_tools=_hr_precompress,
            aggressive_kompress=_hr_aggressive,
            deduplicate_results=_hr_dedup,
            verbose_stats=_hr_verbose,
        )
    except Exception:
        agent.headroom_compressor = None
""".format(marker=MARKER),
)

RUNTIME_HELPERS_PATCH = (
    "            api_mode=agent.api_mode,\n        )",
    """            api_mode=agent.api_mode,
        )

        # ── Update headroom compressor model ── {marker}
        _hr = getattr(agent, "headroom_compressor", None)
        if _hr is not None:
            try:
                _hr.update_model(agent.model)
            except Exception:
                pass
""".format(marker=MARKER),
)


# ── Result ───────────────────────────────────────────────────────────


@dataclass
class InstallResult:
    success: bool = False
    patched: list[str] = field(default_factory=list)
    skipped: list[str] = field(default_factory=list)
    errors: list[str] = field(default_factory=list)
    agent_dir: str = ""


# ── Installer ────────────────────────────────────────────────────────


def find_agent_dir() -> Optional[Path]:
    """Find hermes-agent installation directory."""
    candidates = [
        Path.home() / ".hermes" / "hermes-agent",
        Path.home() / ".hermes-agent",
    ]
    for d in candidates:
        if (d / "agent" / "conversation_loop.py").exists():
            return d
    return None


def is_installed(agent_dir: Path) -> bool:
    """Check if hermes-compress patches are already applied."""
    target = agent_dir / "agent" / "conversation_loop.py"
    try:
        return MARKER in target.read_text()
    except Exception:
        return False


def install(agent_dir: Optional[Path] = None) -> InstallResult:
    """Install hermes-compress patches into hermes-agent.

    Idempotent - safe to run multiple times. Creates .bak files.
    """
    result = InstallResult()

    if agent_dir is None:
        agent_dir = find_agent_dir()
    if agent_dir is None:
        result.errors.append("hermes-agent not found at ~/.hermes/hermes-agent/")
        return result

    result.agent_dir = str(agent_dir)
    agent_pkg = agent_dir / "agent"

    if not agent_pkg.is_dir():
        result.errors.append(f"agent/ directory not found: {agent_pkg}")
        return result

    # 1. Copy headroom_compression.py
    src = Path(__file__).resolve().parent / "_headroom_compression.py"
    if not src.exists():
        # Fallback: bundled alongside this file
        src = Path(__file__).resolve().parent.parent / "hermes_compress" / "_headroom_compression.py"
    dst = agent_pkg / "headroom_compression.py"

    if src.exists():
        _safe_copy(src, dst, result)
    else:
        result.errors.append(f"headroom_compression.py not found at {src}")

    # 2-4. Patch core files
    _patch_file(
        agent_pkg / "conversation_loop.py",
        *CONVERSATION_LOOP_PATCH,
        result,
    )
    _patch_file(
        agent_pkg / "agent_init.py",
        *AGENT_INIT_PATCH,
        result,
    )
    _patch_file(
        agent_pkg / "agent_runtime_helpers.py",
        *RUNTIME_HELPERS_PATCH,
        result,
    )

    result.success = len(result.errors) == 0 and len(result.patched) > 0

    # Verify
    if result.success:
        _verify(agent_dir, result)

    return result


def uninstall(agent_dir: Optional[Path] = None) -> InstallResult:
    """Remove hermes-compress patches from hermes-agent.

    Restores from .bak files if available.
    """
    result = InstallResult()

    if agent_dir is None:
        agent_dir = find_agent_dir()
    if agent_dir is None:
        result.errors.append("hermes-agent not found")
        return result

    result.agent_dir = str(agent_dir)
    agent_pkg = agent_dir / "agent"

    # Remove headroom_compression.py
    dst = agent_pkg / "headroom_compression.py"
    if dst.exists():
        dst.unlink()
        result.patched.append(f"removed: {dst.name}")

    # Restore from backups
    for name in ["conversation_loop.py", "agent_init.py", "agent_runtime_helpers.py"]:
        target = agent_pkg / name
        backup = agent_pkg / f"{name}.hermes-compress.bak"
        if backup.exists():
            shutil.move(str(backup), str(target))
            result.patched.append(f"restored: {name}")
        elif target.exists() and MARKER in target.read_text():
            # No backup - strip the patch lines
            lines = target.read_text().splitlines()
            stripped = [l for l in lines if MARKER not in l]
            target.write_text("\n".join(stripped) + "\n")
            result.patched.append(f"unpatched: {name}")

    result.success = len(result.errors) == 0
    return result


def status(agent_dir: Optional[Path] = None) -> dict:
    """Check installation status."""
    if agent_dir is None:
        agent_dir = find_agent_dir()
    if agent_dir is None:
        return {"installed": False, "error": "hermes-agent not found"}

    installed = is_installed(agent_dir)
    info = {
        "installed": installed,
        "agent_dir": str(agent_dir),
        "files": {},
    }

    for name in ["headroom_compression.py", "conversation_loop.py", "agent_init.py", "agent_runtime_helpers.py"]:
        target = agent_dir / "agent" / name
        info["files"][name] = {
            "exists": target.exists(),
            "patched": MARKER in target.read_text() if target.exists() else False,
        }

    return info


# ── Helpers ──────────────────────────────────────────────────────────


def _safe_copy(src: Path, dst: Path, result: InstallResult) -> None:
    """Copy file, overwrite if newer or different."""
    try:
        if dst.exists():
            if filecmp.cmp(str(src), str(dst), shallow=False):
                result.skipped.append(f"unchanged: {dst.name}")
                return
            result.patched.append(f"updated: {dst.name}")
        else:
            result.patched.append(f"copied: {dst.name}")
        shutil.copy2(str(src), str(dst))
    except Exception as e:
        result.errors.append(f"copy {dst.name}: {e}")


def _patch_file(
    target: Path,
    old: str,
    new: str,
    result: InstallResult,
) -> None:
    """Apply a text patch to a file. Creates .bak if not already patched."""
    if not target.exists():
        result.errors.append(f"not found: {target.name}")
        return

    try:
        content = target.read_text()
    except Exception as e:
        result.errors.append(f"read {target.name}: {e}")
        return

    if MARKER in content:
        # Check if the existing patch matches our current version
        if new in content:
            result.skipped.append(f"already patched: {target.name}")
            return
        # Patch exists but is outdated — restore from backup, then re-patch
        backup = target.with_suffix(target.suffix + ".hermes-compress.bak")
        if backup.exists():
            content = backup.read_text()
            result.patched.append(f"updated: {target.name} (restored from backup)")
        else:
            result.skipped.append(f"already patched (outdated, no backup): {target.name}")
            return

    if old not in content:
        result.errors.append(f"anchor not found in {target.name}: {old[:60]}...")
        return

    # Create backup
    backup = target.with_suffix(target.suffix + ".hermes-compress.bak")
    if not backup.exists():
        shutil.copy2(str(target), str(backup))

    # Apply patch
    patched = content.replace(old, new)
    target.write_text(patched)
    result.patched.append(f"patched: {target.name}")


def _verify(agent_dir: Path, result: InstallResult) -> None:
    """Verify patches compiled correctly."""
    import subprocess
    import sys

    try:
        r = subprocess.run(
            [sys.executable, "-c",
             f"import py_compile; py_compile.compile(r'{agent_dir}/agent/conversation_loop.py', doraise=True)"],
            capture_output=True, text=True, timeout=10,
        )
        if r.returncode != 0:
            result.errors.append(f"conversation_loop.py compile error: {r.stderr[:200]}")
    except Exception as e:
        result.errors.append(f"verify: {e}")
