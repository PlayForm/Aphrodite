#!/usr/bin/env python3
"""
Add headroom proxy providers to ~/.hermes/config.yaml.

Adds two new providers:
    headroom-cache - response caching via headroom on :9799
    headroom-token - full compression via headroom on :9800

Usage:
    python3 scripts/setup-headroom-providers.py
"""
import os
import yaml

HERMES_CONFIG = os.path.expanduser("~/.hermes/config.yaml")


def main():
    if not os.path.exists(HERMES_CONFIG):
        print(f"ERROR: {HERMES_CONFIG} not found", file=__import__("sys").stderr)
        return

    with open(HERMES_CONFIG) as f:
        config = yaml.safe_load(f)

    providers = config.setdefault("providers", {})

    # Both providers authenticate via APHRODITE_API_KEY at runtime.
    # HEADROOM_DEEPSEEK_KEY is the env-var name used for the api_key_env field
    # in the Hermes provider config, but the actual key value is APHRODITE_API_KEY.
    providers["headroom-cache"] = {
        "api_key_env": "HEADROOM_DEEPSEEK_KEY",
        "base_url": "http://127.0.0.1:9799",
        "max_tokens": 32768,
        "provider": "openai",
    }
    providers["headroom-token"] = {
        "api_key_env": "HEADROOM_DEEPSEEK_KEY",
        "base_url": "http://127.0.0.1:9800",
        "max_tokens": 32768,
        "provider": "openai",
    }

    with open(HERMES_CONFIG, "w") as f:
        yaml.dump(config, f, default_flow_style=False, allow_unicode=True, sort_keys=False)

    print("✓ Added to ~/.hermes/config.yaml:")
    print("  headroom-cache → http://127.0.0.1:9799  (response caching, provider: openai)")
    print("  headroom-token → http://127.0.0.1:9800  (full compression, provider: openai)")
    print()
    print("Usage:")
    print("  hermes --provider headroom-cache -m deepseek-v4-pro")
    print("  hermes --provider headroom-token -m deepseek-v4-pro")


if __name__ == "__main__":
    main()
