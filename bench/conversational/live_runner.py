#!/usr/bin/env python3
"""Live conversational benchmark for Aphrodite - model-agnostic, real LLM turns.

A formal, repeatable method for benchmarking CCR compression with ANY model
provider. Runs real agent conversations (via Hermes' own AIAgent) through the
four routing scenarios and measures what a real user gets:

  baseline      - direct to the provider, no proxy, no CCR
  full          - both cache + token proxies (tool-output compression + context offload)
  hermes_proxy  - cache proxy only (tool-output compression)
  proxy_api     - token proxy only (context-window offload)

Headline metrics per scenario x conversation:
  - task completion      (did the agent finish the task? AIAgent `completed`)
  - real elapsed         (wall-clock, incl. proxy + retrieval overhead)
  - CCR create/retrieve  (real compression + retrieval activity from proxy /stats)
  - tokens               (from the response envelope when measurable)
  - net savings          (compressed bytes vs retrieved bytes when measurable)

The model is fully parameterized: --provider/--model/--base-url/--api-key
default to Hermes' own configuration (the same provider a normal Hermes
session uses), so anyone can run the same benchmark against any model.

Isolation: agents run with cwd = ./bench/conversational (this directory) so
they never modify or run commands against the repo root; results write only
under results/<run_id>/.

Usage:
  python3 live_runner.py                     # Hermes-configured provider, all scenarios
  python3 live_runner.py --model gpt-4o --provider openai
  python3 live_runner.py --scenario full --conversation coding_task
  python3 live_runner.py --dry-run
"""

from __future__ import annotations

import json
import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# Make Hermes' agent API importable (it lives in the Hermes source tree)
HERMES_SRC = Path(os.environ.get("HERMES_HOME", Path.home() / ".hermes")) / "hermes-agent"

# Hermes' own interpreter: its source uses modern typing (str | None) that
# system pythons (3.9) can't parse, and it needs Hermes' venv deps. Re-exec
# under Hermes' venv python when we're not already running on it.
HERMES_PYTHON = HERMES_SRC / "venv" / "bin" / "python"
if HERMES_PYTHON.exists():
    _running_under_hermes = Path(sys.executable).resolve() == HERMES_PYTHON.resolve()
    if not _running_under_hermes and not os.environ.get("LIVE_RUNNER_REEVEC"):
        os.environ["LIVE_RUNNER_REEVEC"] = "1"
        os.execv(str(HERMES_PYTHON), [str(HERMES_PYTHON), __file__, *sys.argv[1:]])

if str(HERMES_SRC) not in sys.path:
    sys.path.insert(0, str(HERMES_SRC))

BENCH_DIR = Path(__file__).resolve().parent
REPO_ROOT = BENCH_DIR.parent.parent

# Reuse the harness's scenario metadata + proxy lifecycle (same ports, same binary)
sys.path.insert(0, str(BENCH_DIR))
from harness import (  # noqa: E402
    BASE_URL,
    API_KEY,
    MODEL,
    BENCH_CACHE_PORT,
    BENCH_TOKEN_PORT,
    ProxyManager,
    Scenario,
    SCENARIO_METADATA,
    resolve_aphrodite_binary,
)
from conversations import ALL_CONVERSATIONS  # noqa: E402

# AIAgent is imported lazily inside the runner so --dry-run works without it.


def task_prompt_for(conversation) -> str:
    """Extract the runnable task prompt from a scripted conversation fixture.

    Uses the first user turn (the actual task instruction) + the description
    as context so the live agent gets the same task the simulation scripts.
    """
    first_user = next((t.content for t in conversation.turns if t.role == "user"), "")
    return f"[bench task: {conversation.description}]\n\n{first_user}"


def make_agent(model, provider, base_url, api_key, api_mode, max_turns):
    """Build an AIAgent for the requested model (Hermes' own agent API)."""
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


def run_scenario_conversation(
    scenario: Scenario,
    conversation,
    agent,
    proxy_manager: ProxyManager,
    output_dir: Path,
    max_turns: int,
) -> dict:
    """Run one live conversation under one scenario; return a metrics dict."""
    meta = SCENARIO_METADATA[scenario]
    prompt = task_prompt_for(conversation)
    result = {
        "scenario": scenario.value,
        "conversation": conversation.name,
        "prompt_turns": len(conversation.turns),
        "started": datetime.now(timezone.utc).isoformat(),
    }

    # Start the proxies this scenario needs
    proxy_manager.start_for_scenario(scenario)

    t0 = time.time()
    try:
        run_result = agent.run_conversation(prompt, task_id=f"bench-{scenario.value}-{conversation.name}")
        elapsed = time.time() - t0
    finally:
        proxy_manager.stop_all()

    result["elapsed_s"] = round(elapsed, 3)
    result["completed"] = bool(run_result.get("completed"))
    messages = run_result.get("messages") or []
    result["message_count"] = len(messages)
    last = messages[-1] if messages else {}
    result["final_role"] = last.get("role", "")
    result["final_content_preview"] = str(last.get("content", ""))[:300]

    # CCR activity from proxy stats (real create/retrieve counts)
    ccr = {"creates": 0, "retrieves": 0, "markers": 0}
    for snap in proxy_manager_ccr_stats(proxy_manager, meta):
        ccr["creates"] += snap.get("ccr_creates", 0)
        ccr["retrieves"] += snap.get("ccr_retrieves", 0)
        ccr["markers"] += snap.get("ccr_markers", 0)
    result["ccr"] = ccr

    # Token accounting: prefer real usage from the response envelope; fall back
    # to message-length estimate labeled as such.
    usage = extract_usage(messages)
    if usage:
        result["tokens"] = usage
        result["token_source"] = "response_usage"
    else:
        est = sum(len(str(m.get("content", ""))) // 4 for m in messages)
        result["tokens"] = {"estimated": est}
        result["token_source"] = "estimate"

    (output_dir / "result.json").write_text(json.dumps(result, indent=2, default=str))
    return result


def proxy_manager_ccr_stats(proxy_manager: ProxyManager, meta: dict) -> list[dict]:
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


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Live Aphrodite conversational benchmark")
    parser.add_argument("--provider", default=None, help="Provider (default: Hermes-configured)")
    parser.add_argument("--model", default=None, help="Model (default: Hermes-configured)")
    parser.add_argument("--base-url", default=None, help="API base URL (default: Hermes-configured)")
    parser.add_argument("--api-key", default=None, help="API key (default: Hermes-configured)")
    parser.add_argument("--api-mode", default=None, help="API mode (e.g. chat_completions)")
    parser.add_argument("--scenario", default=None, help="Single scenario (default: all 4)")
    parser.add_argument("--conversation", default=None, help="Single conversation (default: all)")
    parser.add_argument("--max-turns", type=int, default=40, help="Max agent iterations per task")
    parser.add_argument("--run-id", default=None, help="Custom run ID (default: timestamp)")
    parser.add_argument("--dry-run", action="store_true", help="Validate setup without running")
    args = parser.parse_args()

    if args.dry_run:
        bin_path = resolve_aphrodite_binary()
        print(f"✓ Aphrodite binary: {bin_path}")
        print(f"✓ Provider: base_url={'set' if BASE_URL else 'MISSING'} · api_key={'set' if API_KEY else 'MISSING'}")
        print(f"✓ Model: {MODEL or '(override via --model)'}")
        print(f"✓ Conversations: {len(ALL_CONVERSATIONS)}")
        for c in ALL_CONVERSATIONS:
            print(f"    {c.name}: {len(c.turns)} scripted turns ({c.description})")
        print("✓ Agent API:", "importable" if (HERMES_SRC / "run_agent.py").exists() else "MISSING")
        sys.exit(0)

    if not (API_KEY or args.api_key):
        print("ERROR: no provider credential resolved. Check ~/.hermes/config.yaml + .env, or pass --api-key.")
        sys.exit(1)

    scenarios = [Scenario(args.scenario)] if args.scenario else list(Scenario)
    conversations = (
        [c for c in ALL_CONVERSATIONS if c.name == args.conversation]
        if args.conversation
        else ALL_CONVERSATIONS
    )
    if args.conversation and not conversations:
        print(f"Unknown conversation: {args.conversation}")
        sys.exit(1)

    run_id = args.run_id or datetime.now(timezone.utc).strftime("%Y-%m-%d_%H%M%S")
    results_dir = BENCH_DIR / "results" / run_id
    results_dir.mkdir(parents=True, exist_ok=True)

    bin_path = resolve_aphrodite_binary()
    model_used = args.model or MODEL
    print(f"[live] run {run_id} | binary {bin_path} | model {model_used}")

    agent = make_agent(args.model, args.provider, args.base_url, args.api_key, args.api_mode, args.max_turns)

    all_results = []
    proxy_manager = ProxyManager(bin_path, results_dir)
    for scenario in scenarios:
        for conv in conversations:
            print(f"  ── {scenario.value} / {conv.name} ──")
            conv_dir = results_dir / scenario.value / conv.name
            conv_dir.mkdir(parents=True, exist_ok=True)
            r = run_scenario_conversation(scenario, conv, agent, proxy_manager, conv_dir, args.max_turns)
            print(
                f"    ✓ completed={r['completed']} elapsed={r['elapsed_s']}s "
                f"ccr={r['ccr']} tokens={r['tokens']}"
            )
            all_results.append(r)

    manifest = {
        "run_id": run_id,
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "binary": bin_path,
        "provider": args.provider or "hermes-configured",
        "model": model_used,
        "base_url": args.base_url or BASE_URL,
        "max_turns": args.max_turns,
        "scenarios": [s.value for s in scenarios],
        "conversations": [c.name for c in conversations],
        "results": all_results,
    }
    (results_dir / "manifest.json").write_text(json.dumps(manifest, indent=2, default=str))

    print(f"\n[live] results: {results_dir}")
    print(f"[live] manifest: {results_dir / 'manifest.json'}")
    completed = sum(1 for r in all_results if r["completed"])
    print(f"[live] completion: {completed}/{len(all_results)}")


if __name__ == "__main__":
    main()