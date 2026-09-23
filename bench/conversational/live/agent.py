"""AIAgent construction for live benchmark runs (Hermes' own agent API).

ISOLATION MODEL (2026-09-23): every cell runs under its own HERMES_HOME
staged under results/ (never the user's real ~/.hermes). The variant
controls whether the aphrodite plugin is present:

  full     - plugin + proxies      (aphrodite_retrieve etc. available, CCR on)
  baseline - plugin only, no proxy (inline CCR still active)
  off      - EMPTY plugins dir     (no aphrodite at all - the true control)

The cell home contains an empty `plugins/` (off) or a symlink to the real
installed plugin (full/baseline), plus read-only symlinks to the user's
config.yaml + .env so provider resolution works WITHOUT modifying local
config. HERMES_HOME is switched per cell before agent construction; the
Hermes source is imported from the real home regardless.
"""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

REAL_HOME = Path(os.environ.get("HERMES_HOME", Path.home() / ".hermes"))
HERMES_SRC = REAL_HOME / "hermes-agent"


def stage_cell_home(cell_dir: Path, variant: str) -> Path:
    """Create this cell's isolated HERMES_HOME under cell_dir, return its path.

    variant: "full" | "baseline" | "off".
    - off: empty plugins/ dir - aphrodite is completely absent.
    - full/baseline: plugins/aphrodite symlinked to the real install.
    - config.yaml + .env symlinked read-only from the real home (provider
      resolution works; nothing in the user's config is modified).
    """
    cell_home = cell_dir / "hermes-home"
    if cell_home.exists():
        shutil.rmtree(cell_home)
    plugins_dir = cell_home / "plugins"
    plugins_dir.mkdir(parents=True, exist_ok=True)

    if variant in ("full", "baseline"):
        real_plugin = REAL_HOME / "plugins" / "aphrodite"
        if real_plugin.exists():
            (plugins_dir / "aphrodite").symlink_to(real_plugin)
        else:
            (plugins_dir / "README.txt").write_text(
                "aphrodite plugin not found in the real home; variant degraded to off.\n"
            )
        # The plugin's _ensure_binaries() resolves binaries from
        # HERMES_HOME/aphrodite/binaries - WITHOUT them the plugin disables
        # itself (no compression hooks). Copy the real runtime home (binaries,
        # directives, config) into the cell home so full/baseline actually
        # compress; read-only copy, never touching the real home.
        real_runtime = REAL_HOME / "aphrodite"
        if real_runtime.is_dir():
            cell_runtime = cell_home / "aphrodite"
            shutil.copytree(
                real_runtime, cell_runtime,
                ignore=shutil.ignore_patterns("hotreload", "*.log", "*.db-shm", "*.db-wal", "session.current"),
            )

    # Read-only provider config: symlinks never touch the originals.
    for name in ("config.yaml", ".env"):
        src = REAL_HOME / name
        if src.exists():
            dst = cell_home / name
            if dst.exists() or dst.is_symlink():
                dst.unlink()
            dst.symlink_to(src)
    return cell_home


def make_agent(
    model,
    provider,
    base_url,
    api_key,
    api_mode,
    max_turns,
    cwd=None,
    hermes_home: Path | None = None,
    variant: str = "full",
):
    """Build an AIAgent for the requested model (Hermes' own agent API).

    hermes_home: per-cell isolated HERMES_HOME (see stage_cell_home). When
    set, HERMES_HOME switches to it for this agent's construction so the
    plugin manager + state resolve to the cell - never the user's real home.
    """
    # Import Hermes source from the REAL home regardless of the cell switch.
    if str(HERMES_SRC) not in sys.path:
        sys.path.insert(0, str(HERMES_SRC))

    if hermes_home is not None:
        os.environ["HERMES_HOME"] = str(hermes_home)

    from run_agent import AIAgent

    kwargs = dict(
        model=model or "",
        max_iterations=max_turns,
        save_trajectories=False,
        verbose_logging=False,
        quiet_mode=False,
        skip_context_files=True,  # clean bench: no SOUL.md/AGENTS.md bleed
        skip_memory=True,  # no persistent-memory bleed between runs
        platform="bench",
    )
    if provider:
        kwargs["provider"] = provider
    if base_url:
        kwargs["base_url"] = base_url
    if api_key:
        kwargs["api_key"] = api_key
    if api_mode:
        kwargs["api_mode"] = api_mode
    if cwd:
        # session_cwd pins the terminal tool's working dir; the prompt also
        # anchors the agent to the workbench (file tools take absolute paths).
        kwargs["cwd"] = str(cwd)
    return AIAgent(**kwargs)
