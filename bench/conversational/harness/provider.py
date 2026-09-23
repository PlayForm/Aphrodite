"""Hermes provider resolution for the conversational benchmark.
The benchmark uses whatever LLM provider Hermes has configured - it reads
~/.hermes/config.yaml (model.base_url / model.default / provider) and the
credential env vars Hermes itself uses. No hardcoded provider: the harness
inherits the user's real setup (the same provider a normal Hermes session
uses).
"""
from __future__ import annotations
import os
from pathlib import Path

def _resolve_hermes_provider() -> tuple[str, str, str]:
    """Resolve (base_url, api_key, model) from Hermes' own configuration.

    Reads ~/.hermes/config.yaml for model.base_url / model.default / provider,
    then finds the credential: provider-specific API key env vars (what Hermes
    itself uses), then ~/.hermes/.env. Returns empty strings when nothing is
    resolvable - callers degrade gracefully (dry-run fails with a clear
    message instead of a silent 401).
    """
    config_path = Path(os.environ.get("HERMES_HOME", Path.home() / ".hermes")) / "config.yaml"
    base_url = model = provider = ""
    try:
        # config.yaml is YAML, not TOML - parse just the top-level `model:`
        # block with a dependency-free scanner (covers key: value lines; the
        # nested fallback_providers list is ignored).
        in_model = False
        for line in config_path.read_text().splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("#"):
                continue
            indent = len(line) - len(line.lstrip())
            if indent == 0 and not in_model:
                in_model = stripped.startswith("model:")
                continue
            if in_model:
                if indent == 0:
                    break  # next top-level key
                if ":" in stripped and not stripped.startswith("-"):
                    k, _, v = stripped.partition(":")
                    v = v.strip().strip("\"'")
                    if k.strip() == "base_url" and not base_url:
                        base_url = v
                    elif k.strip() == "default" and not model:
                        model = v
                    elif k.strip() == "provider" and not provider:
                        provider = v
    except Exception:
        pass

    api_key = ""
    # Provider-specific env keys (names only, never printed)
    for key in (
        "CLOUDFLARE_API_TOKEN",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "GROQ_API_KEY",
        "MISTRAL_API_KEY",
    ):
        if os.environ.get(key):
            api_key = os.environ[key]
            break
    if not api_key:
        # ~/.hermes/.env (Hermes' secrets file) - same vars, loaded like Hermes does
        env_path = Path(os.environ.get("HERMES_HOME", Path.home() / ".hermes")) / ".env"
        try:
            for line in env_path.read_text().splitlines():
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    k, _, v = line.partition("=")
                    if k.strip() in (
                        "CLOUDFLARE_API_TOKEN",
                        "OPENAI_API_KEY",
                        "ANTHROPIC_API_KEY",
                        "GEMINI_API_KEY",
                        "GROQ_API_KEY",
                        "MISTRAL_API_KEY",
                    ):
                        api_key = v.strip().strip("\"'")
                        break
        except Exception:
            pass
    return base_url, api_key, model

BASE_URL, API_KEY, MODEL = _resolve_hermes_provider()
