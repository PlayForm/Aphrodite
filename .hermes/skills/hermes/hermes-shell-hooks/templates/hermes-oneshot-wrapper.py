#!/usr/bin/env python3
"""
Hermes oneshot stdin wrapper - reference implementation.

Calling convention:
    <venv-python3> hermes-oneshot-wrapper.py <prompt_file> [--model M] [--provider P] [--toolsets T]

Reads the prompt from a file (not CLI arg), calls AIAgent directly with
session_db=None to avoid polluting Hermes's session history, and writes
the response to stdout.

Used by the Aphrodite hook pipeline as the fire-and-forget worker vehicle
(e.g. the detached QA agent spawned by post_llm_call): the wrapper writes no
session rows, so background QA runs never pollute the session history of the
Aphrodite dev loop. Prompt files are staged under ~/.hermes/tmp/ and removed
immediately after reading.
"""

import sys
import os
import logging

# ── resolves at the calling side, not here ──
# The venv's Python already has hermes_cli on sys.path.
# Never use system Python to run this wrapper.

logging.disable(logging.CRITICAL)

from hermes_cli.config import load_config
from hermes_cli.models import detect_provider_for_model
from hermes_cli.runtime_provider import resolve_runtime_provider
from hermes_cli.tools_config import _get_platform_tools
from hermes_cli.fallback_config import get_fallback_chain
from hermes_cli.oneshot import _normalize_toolsets, _oneshot_clarify_callback
from run_agent import AIAgent


def run() -> int:
    if len(sys.argv) < 2:
        sys.stderr.write(
            "Usage: hermes-oneshot-wrapper.py <prompt_file> [--model M] [--provider P] [--toolsets T]\n"
        )
        return 1

    prompt_file = sys.argv[1]
    try:
        with open(prompt_file, "r", encoding="utf-8") as f:
            prompt = f.read()
    except FileNotFoundError:
        sys.stderr.write(f"Prompt file not found: {prompt_file}\n")
        return 1

    # Clean up immediately after reading
    try:
        os.remove(prompt_file)
    except OSError:
        pass

    # Parse optional flags from remaining args
    model = provider = toolsets = None
    argv = sys.argv[2:]
    i = 0
    while i < len(argv):
        if argv[i] == "--model" and i + 1 < len(argv):
            model = argv[i + 1]
            i += 2
        elif argv[i] == "--provider" and i + 1 < len(argv):
            provider = argv[i + 1]
            i += 2
        elif argv[i] == "--toolsets" and i + 1 < len(argv):
            toolsets = argv[i + 1]
            i += 2
        else:
            i += 1

    # ── Build agent (mirrors _run_agent but without session_db) ──
    os.environ["HERMES_YOLO_MODE"] = "1"
    os.environ["HERMES_ACCEPT_HOOKS"] = "1"

    cfg = load_config()
    model_cfg = cfg.get("model") or {}
    cfg_model = (
        model_cfg
        if isinstance(model_cfg, str)
        else model_cfg.get("default") or model_cfg.get("model") or ""
    )
    env_model = os.getenv("HERMES_INFERENCE_MODEL", "").strip()
    effective_model = (model or "").strip() or env_model or cfg_model

    effective_provider = (provider or "").strip() or None
    explicit_base_url = None
    if effective_provider is None and (model or env_model):
        explicit_model = (model or "").strip() or env_model
        if explicit_model:
            try:
                from hermes_cli import model_switch as _ms

                _ms._ensure_direct_aliases()
                direct = _ms.DIRECT_ALIASES.get(explicit_model.strip().lower())
            except Exception:
                direct = None
            if direct is not None:
                effective_model = direct.model
                effective_provider = direct.provider
                if direct.base_url:
                    explicit_base_url = direct.base_url.rstrip("/")
            else:
                cfg_provider = ""
                if isinstance(model_cfg, dict):
                    cfg_provider = str(model_cfg.get("provider") or "").strip().lower()
                current_provider = (
                    cfg_provider
                    or os.getenv("HERMES_INFERENCE_PROVIDER", "").strip().lower()
                    or "auto"
                )
                detected = detect_provider_for_model(explicit_model, current_provider)
                if detected:
                    effective_provider, effective_model = detected

    runtime = resolve_runtime_provider(
        requested=effective_provider,
        target_model=effective_model or None,
        explicit_base_url=explicit_base_url,
    )

    toolsets_list = _normalize_toolsets(toolsets)
    if toolsets_list is None:
        toolsets_list = sorted(_get_platform_tools(cfg, "cli"))

    _fb = get_fallback_chain(cfg)

    # ═══ KEY: no session_db → nothing saved to Hermes history ═══
    agent = AIAgent(
        api_key=runtime.get("api_key"),
        base_url=runtime.get("base_url"),
        provider=runtime.get("provider"),
        api_mode=runtime.get("api_mode"),
        model=effective_model,
        enabled_toolsets=toolsets_list,
        quiet_mode=True,
        platform="cli",
        session_db=None,
        save_trajectories=False,
        credential_pool=runtime.get("credential_pool"),
        fallback_model=_fb or None,
        clarify_callback=_oneshot_clarify_callback,
    )
    agent.suppress_status_output = True
    agent.stream_delta_callback = None
    agent.tool_gen_callback = None

    response = agent.chat(prompt) or ""
    sys.stdout.write(response)
    if not response.endswith("\n"):
        sys.stdout.write("\n")
    sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(run())
