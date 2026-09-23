"""Main harness entry: run_benchmark, helpers, and the CLI."""

from __future__ import annotations
import json
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# The bench dir is the import anchor for the sibling fixtures package
# (mirrors the original flat harness.py sys.path.insert + flat import).
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from fixtures import ALL_CONVERSATIONS, Conversation  # noqa: E402
from .binary import resolve_aphrodite_binary
from .paths import RESULTS_DIR
from .provider import API_KEY, BASE_URL, MODEL
from .provider_client import ProviderClient
from .proxy_client import ProxyClient
from .proxy_manager import ProxyManager
from .results import ConversationResult, RunManifest
from .runner import ConversationRunner
from .scenarios import BENCH_CACHE_PORT, BENCH_TOKEN_PORT, SCENARIO_METADATA, Scenario


def run_benchmark(
    scenarios: Optional[list[Scenario]] = None,
    conversations: Optional[list[Conversation]] = None,
    run_id: Optional[str] = None,
) -> RunManifest:
    """Execute the full conversational benchmark suite."""

    if scenarios is None:
        scenarios = list(Scenario)
    if conversations is None:
        conversations = ALL_CONVERSATIONS

    run_id = run_id or datetime.now(timezone.utc).strftime("%Y-%m-%d_%H%M%S")
    results_dir = RESULTS_DIR / run_id
    results_dir.mkdir(parents=True, exist_ok=True)

    bin_path = resolve_aphrodite_binary()
    print(f"[harness] Aphrodite binary: {bin_path}")
    print(f"[harness] Results dir: {results_dir}")
    print(f"[harness] Model: {MODEL}")
    print(f"[harness] Scenarios: {[s.value for s in scenarios]}")
    print(f"[harness] Conversations: {[c.name for c in conversations]}")
    print()

    manifest = RunManifest(
        run_id=run_id,
        timestamp=datetime.now(timezone.utc).isoformat(),
        aphrodite_version=_get_aphrodite_version(bin_path),
        model=MODEL,
        scenarios_run=[s.value for s in scenarios],
        conversations_run=[c.name for c in conversations],
    )

    proxy_manager = ProxyManager(bin_path, results_dir)
    provider_client = ProviderClient()

    try:
        for scenario in scenarios:
            print(f"{'=' * 60}")
            print(f"SCENARIO: {scenario.value}")
            print(f"  {SCENARIO_METADATA[scenario]['description']}")
            print(f"{'=' * 60}")

            # Start proxies for this scenario
            proxy_manager.start_for_scenario(scenario)
            meta = SCENARIO_METADATA[scenario]

            # Set up clients
            proxy_client = None
            cache_client = None

            if meta["uses_token_proxy"]:
                proxy_client = ProxyClient(BENCH_TOKEN_PORT)
            if meta["uses_cache_proxy"]:
                cache_client = ProxyClient(BENCH_CACHE_PORT)

            for conv in conversations:
                print(f"\n  ── {conv.name}: {conv.description} ──")

                conv_dir = results_dir / scenario.value / conv.name
                conv_dir.mkdir(parents=True, exist_ok=True)

                runner = ConversationRunner(
                    scenario=scenario,
                    output_dir=conv_dir,
                    proxy_manager=proxy_manager,
                    provider_client=provider_client if not proxy_client else None,
                    proxy_client=proxy_client,
                    cache_client=cache_client,
                )

                conv_result = runner.run(conv)
                manifest.results.append(conv_result)
                manifest.total_turns += len(conv_result.turns)
                manifest.total_errors += len(conv_result.errors)

                # Save summary
                _save_conversation_summary(conv_dir, conv_result)
                print(
                    f"    ✓ {len(conv_result.turns)} turns, "
                    f"{conv_result.total_tokens} tokens, "
                    f"{len(conv_result.errors)} errors"
                )

            # Stop proxies between scenarios
            proxy_manager.stop_all()
            # Brief pause to let ports release
            time.sleep(1)

    finally:
        proxy_manager.stop_all()

    # Save run manifest
    manifest_path = results_dir / "manifest.json"
    manifest_data = {
        "run_id": manifest.run_id,
        "timestamp": manifest.timestamp,
        "aphrodite_version": manifest.aphrodite_version,
        "model": manifest.model,
        "scenarios_run": manifest.scenarios_run,
        "conversations_run": manifest.conversations_run,
        "total_turns": manifest.total_turns,
        "total_errors": manifest.total_errors,
        "results": [
            {
                "scenario": r.scenario,
                "conversation": r.conversation_name,
                "turns": len(r.turns),
                "total_prompt_tokens": r.total_prompt_tokens,
                "total_completion_tokens": r.total_completion_tokens,
                "total_tokens": r.total_tokens,
                "total_elapsed_ms": r.total_elapsed_ms,
                "errors": len(r.errors),
            }
            for r in manifest.results
        ],
    }
    with open(manifest_path, "w") as f:
        json.dump(manifest_data, f, indent=2, default=str)

    print(f"\n{'=' * 60}")
    print(f"BENCHMARK COMPLETE")
    print(f"  Run ID: {run_id}")
    print(f"  Results: {results_dir}")
    print(f"  Total turns: {manifest.total_turns}")
    print(f"  Total errors: {manifest.total_errors}")
    _print_token_summary(manifest)

    return manifest


def _save_conversation_summary(conv_dir: Path, result: ConversationResult):
    """Save per-conversation summary JSON."""
    summary = {
        "scenario": result.scenario,
        "conversation": result.conversation_name,
        "turn_count": len(result.turns),
        "total_prompt_tokens": result.total_prompt_tokens,
        "total_completion_tokens": result.total_completion_tokens,
        "total_tokens": result.total_tokens,
        "total_elapsed_ms": result.total_elapsed_ms,
        "errors": result.errors,
        "turns": [
            {
                "index": t.turn_index,
                "role": t.role,
                "elapsed_ms": t.elapsed_ms,
                "prompt_tokens": t.prompt_tokens,
                "completion_tokens": t.completion_tokens,
                "total_tokens": t.total_tokens,
                "response_status": t.response_status,
                "ccr_events_count": len(t.ccr_events),
                "error": t.error,
            }
            for t in result.turns
        ],
        "proxy_stats_count": len(result.proxy_stats_snapshots),
    }
    with open(conv_dir / "summary.json", "w") as f:
        json.dump(summary, f, indent=2, default=str)


def _get_aphrodite_version(bin_path: str) -> str:
    """Get aphrodite version from binary."""
    try:
        result = subprocess.run([bin_path, "--version"], capture_output=True, text=True, timeout=5)
        return result.stdout.strip() or "unknown"
    except Exception:
        return "unknown"


def _print_token_summary(manifest: RunManifest):
    """Print a summary table of token usage across scenarios."""
    print(
        f"\n{'Scenario':<20} {'Conversation':<20} {'Turns':>6} {'Prompt':>10} {'Completion':>12} {'Total':>10} {'Ms':>8}"
    )
    print("-" * 86)
    for r in manifest.results:
        print(
            f"{r.scenario:<20} {r.conversation_name:<20} {len(r.turns):>6} "
            f"{r.total_prompt_tokens:>10} {r.total_completion_tokens:>12} "
            f"{r.total_tokens:>10} {int(r.total_elapsed_ms):>8}"
        )


def main():
    """CLI entry point (mirrors the original flat harness.py __main__ block)."""
    import argparse

    parser = argparse.ArgumentParser(description="Aphrodite Conversational Benchmark")
    parser.add_argument(
        "--scenario",
        choices=[s.value for s in Scenario],
        help="Run a single scenario (default: all)",
    )
    parser.add_argument("--conversation", help="Run a single conversation (default: all)")
    parser.add_argument("--run-id", help="Custom run ID (default: timestamp)")
    parser.add_argument(
        "--dry-run", action="store_true", help="Validate setup without running conversations"
    )
    args = parser.parse_args()

    if args.dry_run:
        bin_path = resolve_aphrodite_binary()
        print(f"✓ Aphrodite binary: {bin_path}")
        print(
            f"✓ Provider: base_url={'set' if BASE_URL else 'MISSING'} · api_key={'set' if API_KEY else 'MISSING'} · model={MODEL or 'MISSING'}"
        )
        print(f"✓ Conversations: {len(ALL_CONVERSATIONS)}")
        for c in ALL_CONVERSATIONS:
            print(f"    {c.name}: {len(c.turns)} turns ({c.description})")
        sys.exit(0)

    scenarios = None
    if args.scenario:
        scenarios = [Scenario(args.scenario)]

    conversations = None
    if args.conversation:
        conversations = [c for c in ALL_CONVERSATIONS if c.name == args.conversation]
        if not conversations:
            print(f"Unknown conversation: {args.conversation}")
            print(f"Available: {[c.name for c in ALL_CONVERSATIONS]}")
            sys.exit(1)

    if not API_KEY:
        print("ERROR: no provider credential resolved (API_KEY empty).")
        print(
            "Hermes provider resolution failed - check ~/.hermes/config.yaml and .env, or export a provider key."
        )
        sys.exit(1)

    run_benchmark(scenarios=scenarios, conversations=conversations, run_id=args.run_id)


if __name__ == "__main__":
    main()


if __name__ == "__main__":
    main()
