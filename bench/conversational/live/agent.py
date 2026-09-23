"""AIAgent construction for live benchmark runs (Hermes' own agent API)."""

from __future__ import annotations

import os
import sys
from pathlib import Path


def make_agent(model, provider, base_url, api_key, api_mode, max_turns):
    """Build an AIAgent for the requested model (Hermes' own agent API)."""
    hermes_src = Path(os.environ.get("HERMES_HOME", Path.home() / ".hermes")) / "hermes-agent"
    if str(hermes_src) not in sys.path:
        sys.path.insert(0, str(hermes_src))
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
    return AIAgent(**kwargs)