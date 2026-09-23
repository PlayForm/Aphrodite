"""CCR proxy-stats snapshots + token-usage extraction for live runs."""

from __future__ import annotations

from harness.scenarios import BENCH_CACHE_PORT, BENCH_TOKEN_PORT


def proxy_manager_ccr_stats(proxy_manager, meta: dict) -> list[dict]:
    """Snapshot CCR counters from the proxies the scenario used."""
    snaps = []
    if meta["uses_cache_proxy"]:
        stats = proxy_manager.get_stats(BENCH_CACHE_PORT)
        snaps.append(
            {
                "ccr_creates": stats.get("ccr_creates", stats.get("creates", 0)),
                "ccr_markers": stats.get("ccr_markers", stats.get("markers", 0)),
            }
        )
    if meta["uses_token_proxy"]:
        stats = proxy_manager.get_stats(BENCH_TOKEN_PORT)
        snaps.append(
            {
                "ccr_retrieves": stats.get("ccr_retrieves", stats.get("retrieves", 0)),
            }
        )
    return snaps


def extract_usage(messages: list[dict]) -> dict | None:
    """Pull real token usage from the API response envelope if present."""
    for m in messages:
        usage = m.get("usage") or (m.get("response") or {}).get("usage")
        if isinstance(usage, dict) and usage:
            return {
                "prompt": usage.get("prompt_tokens", 0),
                "completion": usage.get("completion_tokens", 0),
                "total": usage.get("total_tokens", 0),
            }
    return None