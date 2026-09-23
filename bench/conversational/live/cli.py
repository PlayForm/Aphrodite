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
under results/<run_id>/ (via harness.paths.RESULTS_DIR).

Usage:
  python3 -m live                          # Hermes-configured provider, all scenarios
  python3 live_runner.py                   # equivalent via the flat shim
  python3 -m live --model gpt-4o --provider openai
  python3 -m live --scenario full --conversation coding_task
  python3 -m live --dry-run
"""

from __future__ import annotations

import json
import os
import sys
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
        # live/.. = bench/conversational - make it importable for `-m live`
        _bench_dir = Path(__file__).resolve().parent.parent
        _env = os.environ.copy()
        _existing_pypath = _env.get("PYTHONPATH", "")
        _env["PYTHONPATH"] = str(_bench_dir) + (
            os.pathsep + _existing_pypath if _existing_pypath else ""
        )
        os.execvpe(
            str(HERMES_PYTHON),
            [str(HERMES_PYTHON), "-m", "live", *sys.argv[1:]],
            _env,
        )

if str(HERMES_SRC) not in sys.path:
    sys.path.insert(0, str(HERMES_SRC))

# Imports reached only under Hermes' interpreter (post re-exec)
from harness.binary import resolve_aphrodite_binary  # noqa: E402
from harness.paths import RESULTS_DIR  # noqa: E402
from harness.provider import API_KEY, BASE_URL, MODEL  # noqa: E402
from harness.proxy_manager import ProxyManager  # noqa: E402
from harness.scenarios import Scenario  # noqa: E402
from fixtures import ALL_CONVERSATIONS  # noqa: E402

from .agent import make_agent  # noqa: E402
from .scenario import run_scenario_conversation  # noqa: E402
from .task_prompt import workspace_for  # noqa: E402

# The bench's DEFAULT model: cheapest live option on this Cloudflare account
# (per Auth-Cloudflare pricing fixtures). Override with --model.
DEFAULT_BENCH_MODEL = "@cf/zai-org/glm-5.3-flash"


def main():
    import argparse

    parser = argparse.ArgumentParser(description="Live Aphrodite conversational benchmark")
    parser.add_argument("--provider", default=None, help="Provider (default: Hermes-configured)")
    parser.add_argument(
        "--model",
        default=None,
        help=f"Model (default: {DEFAULT_BENCH_MODEL} - the bench's cheapest live option)",
    )
    parser.add_argument(
        "--base-url", default=None, help="API base URL (default: Hermes-configured)"
    )
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
        print(
            f"✓ Provider: base_url={'set' if BASE_URL else 'MISSING'} · api_key={'set' if API_KEY else 'MISSING'}"
        )
        print(
            f"✓ Model: {MODEL or '(override via --model)'} (bench default: {DEFAULT_BENCH_MODEL})"
        )
        print(f"✓ Conversations: {len(ALL_CONVERSATIONS)}")
        for c in ALL_CONVERSATIONS:
            print(f"    {c.name}: {len(c.turns)} scripted turns ({c.description})")
        print("✓ Agent API:", "importable" if (HERMES_SRC / "run_agent.py").exists() else "MISSING")
        sys.exit(0)

    if not (API_KEY or args.api_key):
        print(
            "ERROR: no provider credential resolved. Check ~/.hermes/config.yaml + .env, or pass --api-key."
        )
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
    results_dir = RESULTS_DIR / run_id
    results_dir.mkdir(parents=True, exist_ok=True)

    bin_path = resolve_aphrodite_binary()
    model_used = args.model or DEFAULT_BENCH_MODEL
    print(f"[live] run {run_id} | binary {bin_path} | model {model_used}")

    all_results = []
    proxy_manager = ProxyManager(bin_path, results_dir)
    for scenario in scenarios:
        for conv in conversations:
            print(f"  ── {scenario.value} / {conv.name} ──")
            conv_dir = results_dir / scenario.value / conv.name
            conv_dir.mkdir(parents=True, exist_ok=True)
            # One agent per cell: session_cwd (terminal pin) is set at agent
            # construction, and each workspace needs its own cwd.
            ws = workspace_for(conv)
            agent = make_agent(
                model_used,
                args.provider,
                args.base_url,
                args.api_key,
                args.api_mode,
                args.max_turns,
                cwd=ws,
            )
            r = run_scenario_conversation(
                scenario, conv, agent, proxy_manager, conv_dir, args.max_turns
            )
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
