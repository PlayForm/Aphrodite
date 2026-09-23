"""Scenario definitions, metadata, and bench proxy ports."""

from __future__ import annotations

from enum import Enum

from .provider import BASE_URL

# Proxy config (matching real aphrodite.toml ports)
CACHE_PORT = 9797
TOKEN_PORT = 9798

# Bench-specific ports (isolated from production)
BENCH_CACHE_PORT = 49797
BENCH_TOKEN_PORT = 49798


class Scenario(Enum):
    BASELINE = "baseline"  # Direct to the resolved provider, no proxy
    FULL = "full"  # Both cache + token proxies
    HERMES_PROXY = "hermes_proxy"  # Cache proxy only (tool output compression)
    PROXY_API = "proxy_api"  # Token proxy only (context window compression)


SCENARIO_METADATA = {
    Scenario.BASELINE: {
        "description": "1:1 baseline - direct LLM API, no proxy, no CCR",
        "uses_cache_proxy": False,
        "uses_token_proxy": False,
        "api_url": f"{BASE_URL}/v1/chat/completions",
    },
    Scenario.FULL: {
        "description": "1:1 with full compression - both cache + token proxies",
        "uses_cache_proxy": True,
        "uses_token_proxy": True,
        "cache_proxy_url": f"http://127.0.0.1:{BENCH_CACHE_PORT}/v1/chat/completions",
        "token_proxy_url": f"http://127.0.0.1:{BENCH_TOKEN_PORT}/v1/chat/completions",
        "api_url": f"http://127.0.0.1:{BENCH_TOKEN_PORT}/v1/chat/completions",
    },
    Scenario.HERMES_PROXY: {
        "description": "1:1 with compression between Hermes and proxy (cache only)",
        "uses_cache_proxy": True,
        "uses_token_proxy": False,
        "cache_proxy_url": f"http://127.0.0.1:{BENCH_CACHE_PORT}/v1/chat/completions",
        "api_url": f"{BASE_URL}/v1/chat/completions",
    },
    Scenario.PROXY_API: {
        "description": "1:1 with compression between proxy and external API (token only)",
        "uses_cache_proxy": False,
        "uses_token_proxy": True,
        "token_proxy_url": f"http://127.0.0.1:{BENCH_TOKEN_PORT}/v1/chat/completions",
        "api_url": f"http://127.0.0.1:{BENCH_TOKEN_PORT}/v1/chat/completions",
    },
}
