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

    # Live persistence: append every text delta to stream.jsonl as it
    # arrives (stream_callback), so data is on disk DURING the run - usable
    # for graphs/readability even if the cell is interrupted. The full
    # trajectory + result.json land at the end (or on failure).
    stream_path = output_dir / "stream.jsonl"
    stream_fh = open(stream_path, "a", buffering=1)

    def _stream_callback(delta):
        stream_fh.write(json.dumps({"delta": delta}) + "\n")

    t0 = time.time()
    try:
        run_result = agent.run_conversation(
            prompt,
            task_id=f"bench-{scenario.value}-{conversation.name}",
            stream_callback=_stream_callback,
        )
        elapsed = time.time() - t0
    except Exception as exc:
        elapsed = time.time() - t0
        stream_fh.close()
        # Partial persistence: a crashed cell still leaves its trace on disk.
        partial = {
            "scenario": scenario.value,
            "conversation": conversation.name,
            "started": result["started"],
            "elapsed_s": round(elapsed, 3),
            "completed": False,
            "error": str(exc),
            "partial": True,
        }
        (output_dir / "result.json").write_text(json.dumps(partial, indent=2, default=str))
        raise
    finally:
        if not stream_fh.closed:
            stream_fh.close()
        proxy_manager.stop_all()

    result["elapsed_s"] = round(elapsed, 3)
    result["completed"] = bool(run_result.get("completed"))
    messages = run_result.get("messages") or []
    result["message_count"] = len(messages)
    last = messages[-1] if messages else {}
    result["final_role"] = last.get("role", "")
    result["final_content_preview"] = str(last.get("content", ""))[:300]

    # Task overview: what the task is + what the agent actually did
    # (tool-call summary from the trajectory, not just numbers).
    result["task"] = {
        "name": conversation.name,
        "description": conversation.description,
        "prompt_turns_scripted": len(conversation.turns),
        "workspace": str(workspace) if workspace else None,
    }
    result["activity"] = summarize_activity(messages)

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

    # Preserve the full trajectory + raw message envelope per cell, so results
    # are auditable and re-analyzable (methodology: "each result traces to
    # exact artifacts, markers, retrievals, tools, task outcomes").
    (output_dir / "trajectory.json").write_text(
        json.dumps({"messages": messages}, indent=2, default=str)
    )
    (output_dir / "result.json").write_text(json.dumps(result, indent=2, default=str))
    return result


def summarize_activity(messages: list[dict]) -> dict:
    """High-level overview of what the agent actually did this session.

    Counts tool calls by name, extracts CCR markers/retrievals from message
    content, and summarizes the turn roles - the 'basic overview of the task
    they're performing' the operator needs at a glance.
    """
    tool_calls = {}
    ccr_markers = 0
    ccr_retrieves = 0
    reads = 0
    edits = 0
    terminal_runs = 0
    for m in messages:
        content = str(m.get("content") or "")
        ccr_markers += content.count("<<<CCR:")
        # tool-call messages carry tool_calls lists
        for tc in m.get("tool_calls") or []:
            name = tc.get("function", {}).get("name", "?") if isinstance(tc, dict) else "?"
            tool_calls[name] = tool_calls.get(name, 0) + 1
            if name == "aphrodite_retrieve":
                ccr_retrieves += 1
            elif name == "read_file":
                reads += 1
            elif name in ("write_file", "patch"):
                edits += 1
            elif name == "terminal":
                terminal_runs += 1
        # tool-result messages: role "tool" = tool output (possibly a marker)
        if m.get("role") == "tool":
            ccr_markers += content.count("<<<CCR:")
    return {
        "tool_calls": tool_calls,
        "ccr_markers_seen": ccr_markers,
        "ccr_retrieves": ccr_retrieves,
        "file_reads": reads,
        "file_edits": edits,
        "terminal_runs": terminal_runs,
    }
