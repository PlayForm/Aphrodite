#!/usr/bin/env python3
"""
Add headroom proxy providers to Hermes config.

Adds two new providers to ~/.hermes/config.yaml:
    headroom-cache — response caching via headroom on :9799
    headroom-token — full compression via headroom on :9800

Run after starting the headroom proxy:
    python3 scripts/run-headroom-proxy.py cache &
    python3 scripts/run-headroom-proxy.py token &

Then add providers:
    python3 scripts/setup-headroom-providers.py

Then use in Hermes:
    hermes --provider headroom-cache
    hermes --provider headroom-token
"""
import os
import sys

HERMES_CONFIG = os.path.expanduser("~/.hermes/config.yaml")


def main():
    if not os.path.exists(HERMES_CONFIG):
        print(f"ERROR: {HERMES_CONFIG} not found", file=sys.stderr)
        sys.exit(1)

    with open(HERMES_CONFIG) as f:
        content = f.read()

    providers_block = """  headroom-cache:
    api_key_env: HEADROOM_DEEPSEEK_KEY
    base_url: http://127.0.0.1:9799
    max_tokens: 32768
    provider: deepseek
  headroom-token:
    api_key_env: HEADROOM_DEEPSEEK_KEY
    base_url: http://127.0.0.1:9800
    max_tokens: 32768
    provider: deepseek
"""

    if "headroom-cache:" in content:
        print("headroom-cache provider already exists — updating base_url")
        # Replace existing
        import re
        content = re.sub(
            r"  headroom-cache:\n    api_key_env:.*?\n    base_url:.*?\n    max_tokens:.*?\n    provider:.*?\n",
            providers_block.split("\n  headroom-token")[0] + "\n",
            content,
            flags=re.DOTALL,
        )
    else:
        # Find providers: section and insert before the closing of that block
        # Insert after the last existing provider
        marker = "credential_pool_strategies:"
        if marker in content:
            insert_pos = content.find(marker)
            content = content[:insert_pos] + providers_block + "\n" + content[insert_pos:]

    with open(HERMES_CONFIG, "w") as f:
        f.write(content)

    print("✓ Added to ~/.hermes/config.yaml:")
    print("  headroom-cache → http://127.0.0.1:9799  (response caching)")
    print("  headroom-token → http://127.0.0.1:9800  (full compression)")
    print()
    print("Usage:")
    print("  hermes --provider headroom-cache")
    print("  hermes --provider headroom-token")
    print()
    print("API key: set HEADROOM_DEEPSEEK_KEY in ~/.hermes/.env")


if __name__ == "__main__":
    main()
