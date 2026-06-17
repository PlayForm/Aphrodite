#!/usr/bin/env python3
"""
Hermes poll worker — oneshot agent that runs via the Save/prompt-file
pattern WITHOUT polluting session history. Same architecture as the
gcommit flow: prompt goes to a temp file, agent reads it, stdout only.

Usage (from hermes-z-execution skill):
  TMP=$(mktemp ~/.hermes/temporary/poll.XXXXXX.md)
  printf "<instructions>" > "$TMP"
  python3 scripts/poll_worker.py "$TMP" --model deepseek-v4-flash --toolsets terminal,file

Called by the Save binary or directly via shell.
"""
import os
import sys

# Add hermes to the Python path
HERMES_VENV = os.path.expanduser("~/.hermes/hermes-agent/venv")
HERMES_SITE = os.path.join(
    HERMES_VENV,
    "lib",
    f"python{sys.version_info.major}.{sys.version_info.minor}",
    "site-packages",
)
sys.path.insert(0, HERMES_SITE)

# Silence all logging
import logging  # noqa: E402

logging.disable(logging.CRITICAL)

from contextlib import suppress  # noqa: E402

from hermes_cli.config import load_config  # noqa: E402
from hermes_cli.fallback_config import get_fallback_chain  # noqa: E402
from hermes_cli.models import detect_provider_for_model  # noqa: E402
from hermes_cli.oneshot import _normalize_toolsets, _oneshot_clarify_callback  # noqa: E402
from hermes_cli.runtime_provider import resolve_runtime_provider  # noqa: E402
from hermes_cli.tools_config import _get_platform_tools  # noqa: E402
from run_agent import AIAgent  # noqa: E402


def run() -> int:
    if len(sys.argv) < 2:
        sys.stderr.write("Usage: poll_worker.py <prompt_file> [--model M] [--provider P] [--toolsets T]\n")
        return 1

    prompt_file = sys.argv[1]
    try:
        with open(prompt_file, encoding="utf-8") as f:
            prompt = f.read()
    except FileNotFoundError:
        sys.stderr.write(f"Prompt file not found: {prompt_file}\n")
        return 1

    # Delete temp file immediately after reading
    with suppress(OSError):
        os.remove(prompt_file)

    # Parse optional flags
    model = None
    provider = None
    toolsets = None
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

    # ── Build agent (CLEAN — no session_db, no history pollution) ──
    os.environ["HERMES_YOLO_MODE"] = "1"
    os.environ["HERMES_ACCEPT_HOOKS"] = "1"

    cfg = load_config()

    # Resolve model
    model_cfg = cfg.get("model") or {}
    if isinstance(model_cfg, str):
        cfg_model = model_cfg
    else:
        cfg_model = model_cfg.get("default") or model_cfg.get("model") or ""

    env_model = os.getenv("HERMES_INFERENCE_MODEL", "").strip()
    effective_model = (model or "").strip() or env_model or cfg_model

    # Resolve provider
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
    # Always include aphrodite — workers need CCR compression hooks
    if "aphrodite" not in toolsets_list:
        toolsets_list = list(toolsets_list) + ["aphrodite"]

    _fb = get_fallback_chain(cfg)

    # ═══ CLEAN ONESHOT: no session_db, no reasoning ═══
    agent = AIAgent(
        api_key=runtime.get("api_key"),
        base_url=runtime.get("base_url"),
        provider=runtime.get("provider"),
        api_mode=runtime.get("api_mode"),
        model=effective_model,
        enabled_toolsets=toolsets_list,
        quiet_mode=True,
        platform="cli",
        session_db=None,              # ← no session database = no history pollution
        save_trajectories=False,      # ← explicit: don't save trajectories either
        credential_pool=runtime.get("credential_pool"),
        fallback_model=_fb or None,
        clarify_callback=_oneshot_clarify_callback,
        reasoning_config={"enabled": False},  # ← poll workers don't need reasoning
    )
    agent.suppress_status_output = True
    agent.stream_delta_callback = None
    agent.tool_gen_callback = None

    response = agent.chat(prompt) or ""
    sys.stdout.write(response)
    if not response.endswith("\n"):
        sys.stdout.write("\n")
    sys.stdout.flush()

    # Clean up any stray session files
    with suppress(Exception):
        import glob
        for f in glob.glob(os.path.expanduser("~/.hermes/data/*oneshot*")):
            with suppress(OSError):
                os.remove(f)

    return 0


if __name__ == "__main__":
    sys.exit(run())
