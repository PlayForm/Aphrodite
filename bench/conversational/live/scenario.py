"""Single live scenario x conversation run."""

from __future__ import annotations

import json
import time
from datetime import datetime, timezone
from pathlib import Path

from harness.proxy_manager import ProxyManager
from harness.scenarios import BENCH_CACHE_PORT, BENCH_TOKEN_PORT, SCENARIO_METADATA, Scenario

from .stats import extract_usage, proxy_manager_ccr_stats
from .task_prompt import task_prompt_for, workspace_for


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
    workspace = workspace_for(conversation)
    prompt = task_prompt_for(conversation, workspace)
    result = {
        "scenario": scenario.value,
        "conversation": conversation.name,
        "prompt_turns": len(conversation.turns),
        "workspace": str(workspace) if workspace else None,
        "started": datetime.now(timezone.utc).isoformat(),
    }

    # Start the proxies this scenario needs
    proxy_manager.start_for_scenario(scenario)

    t0 = time.time()
    try:
        run_result = agent.run_conversation(
            prompt,
            task_id=f"bench-{scenario.value}-{conversation.name}",
        )
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