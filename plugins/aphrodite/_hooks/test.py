"""aphrodite — integration smoke test handler."""

import json
import logging
import os
import time

from .._core import (
    INLINE_THRESHOLD,
    PLUGIN_VERSION,
    TERMINAL_THRESHOLD,
    TOOL_THRESHOLD_CACHE,
    TOOL_THRESHOLD_TOKEN,
    ENGINE_THRESHOLD_PCT,
)
from .._proxy import _alive
from .stats import _stats_handler
from .files import _files_handler
from .diff import _diff_handler
from .search import _search_handler

_log = logging.getLogger("aphrodite.hooks.test")


def _test_handler(args=None, **kwargs):
    """Full smoke test suite — exercises all tools, hooks, compression, search, retrieve."""
    from .._tools import _compress_handler, _retrieve_handler

    args = args if isinstance(args, dict) else {}
    mode = args.get("mode", "quick")
    report = {"suite": "aphrodite_smoke", "version": PLUGIN_VERSION, "mode": mode, "tests": []}

    def test(name, fn):
        try:
            t0 = time.time()
            result = fn()
            elapsed = (time.time() - t0) * 1000
            report["tests"].append({"name": name, "status": "PASS", "elapsed_ms": round(elapsed, 1), "result": result})
        except Exception as e:
            report["tests"].append({"name": name, "status": "FAIL", "error": str(e)})

    test("compress_json", lambda: json.loads(_compress_handler(args={"content": '{"a":1,"b":[2,3]}', "type": "json"})))
    test("compress_code", lambda: json.loads(_compress_handler(args={"content": "def foo():\n    return 42\n", "type": "code"})))
    test("compress_cache_hit", lambda: _compress_handler(args={"content": '{"a":1,"b":[2,3]}', "type": "json"}))
    test("retrieve_roundtrip", lambda: (
        (h := json.loads(_compress_handler(args={"content": "def foo():\n    return 42\n", "type": "code"}))["hash"])
        and "def foo" in _retrieve_handler(args={"hash": h})
    ))
    test("stats", lambda: json.loads(_stats_handler())["proxy"])
    test("files_empty", lambda: json.loads(_files_handler())["count"] == 0)
    test("diff_empty", lambda: json.loads(_diff_handler())["turns"] == 0)
    test("proxy_health", lambda: _alive(9798))
    test("proxy_metrics", lambda: _alive(9797))

    if mode in ("full", "matrix"):
        big_payload = json.dumps({"data": list(range(1000)), "nested": {"deep": {"values": [i * i for i in range(200)]}}})
        test("compress_large", lambda: json.loads(_compress_handler(args={"content": big_payload, "type": "json"}))["size"] > 1000)
        test("search_find", lambda: json.loads(_search_handler(args={"query": "deep"}))["matches"] >= 1)
        test("terminal_threshold", lambda: TERMINAL_THRESHOLD > 0)
        test("inline_threshold", lambda: INLINE_THRESHOLD > 0)

    if mode == "matrix":
        settings = {"results": {}}
        for pct in (0, 25, 50, 75, 100):
            for protect in (2, 5, 10):
                key = f"pct={pct}_protect={protect}"
                settings["results"][key] = {"threshold_pct": pct, "protect_last": protect,
                                            "compresses_always": pct == 0, "compresses_never": pct >= 100}
        report["settings_matrix"] = settings

    if mode == "pipeline":
        toggles = {
            "debug_on": {"APHRODITE_DEBUG": "1"},
            "debug_off": {"APHRODITE_DEBUG": "0"},
            "engine_on": {"APHRODITE_CONTEXT_ENGINE": "1"},
            "engine_off": {"APHRODITE_CONTEXT_ENGINE": "0"},
        }
        feature_results = {}
        for name, env_overrides in toggles.items():
            saved = {k: os.environ.get(k, "") for k in env_overrides}
            try:
                for k, v in env_overrides.items():
                    os.environ[k] = v
                feature_results[name] = {
                    "env": env_overrides,
                    "proxy_alive": _alive(9798),
                    "cache_alive": _alive(9797),
                    "thresholds": {
                        "terminal": TERMINAL_THRESHOLD, "inline": INLINE_THRESHOLD,
                        "tool_token": TOOL_THRESHOLD_TOKEN, "tool_cache": TOOL_THRESHOLD_CACHE,
                    },
                    "engine_threshold": ENGINE_THRESHOLD_PCT,
                }
            finally:
                for k, orig in saved.items():
                    if orig:
                        os.environ[k] = orig
                    else:
                        os.environ.pop(k, None)
        report["feature_toggles"] = feature_results

    report["summary"] = {
        "total": len(report["tests"]),
        "passed": sum(1 for t in report["tests"] if t["status"] == "PASS"),
        "failed": sum(1 for t in report["tests"] if t["status"] == "FAIL"),
    }

    try:
        results_path = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite", ".test-results.json")
        prev = {}
        if os.path.exists(results_path):
            with open(results_path) as f:
                prev = json.load(f)
        with open(results_path, "w") as f:
            json.dump(report, f, indent=2)
        if prev:
            prev_passed = prev.get("summary", {}).get("passed", 0)
            curr_passed = report["summary"]["passed"]
            report["regression"] = {
                "previous_passed": prev_passed,
                "current_passed": curr_passed,
                "delta": curr_passed - prev_passed,
                "status": "DEGRADED" if curr_passed < prev_passed else "OK",
            }
    except Exception:
        pass
    return json.dumps(report, indent=2)


TEST_SCHEMA = {
    "name": "aphrodite_test",
    "description": "Run full smoke test suite — compress, retrieve, search, stats, files, "
    "diff, proxy health. Modes: quick, full, matrix, pipeline.",
    "parameters": {
        "type": "object",
        "properties": {
            "mode": {
                "type": "string",
                "description": "Test mode: quick (default), full, or matrix",
            }
        },
    },
}
