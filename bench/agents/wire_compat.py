#!/usr/bin/env python3
"""Agent wire-schema compatibility matrix for the aphrodite token proxy.

Stdlib only. Spawns:
  - mock upstream (mock_upstream.py) on 127.0.0.1:19899
  - aphrodite token proxy on 127.0.0.1:19898, upstream pointed at the stub
    via --api-url (the upstream-override mechanism; port set via --listen /
    APHRODITE_TOKEN_PORT-equivalent CLI override)

For each agent dialect (from .plans/16/17 §2 tables) it sends a
representative request through the proxy and directly to the stub, then
records:
  - verdict: proxied-ok / mangled / rejected
  - latency overhead vs direct-to-stub (median of N)
  - request_tool_msgs_compressed: were role:"tool" request messages rewritten
    before reaching upstream? (15-P1 baseline: expected False)
  - response_tool_calls_intact: did response tool_calls[].function.arguments
    survive byte-identical? (15 §2.3 hazard / wave-1 02-F3)

Outputs JSON to bench/results/wire-compat-<ts>.json and a stdout table.

Usage: python3 wire_compat.py [--proxy-bin PATH] [--keep]
"""

import argparse
import json
import os
import signal
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
RESULTS_DIR = os.path.join(REPO, "bench", "results")
TMP_DIR = os.path.join(RESULTS_DIR, "tmp")

PROXY_PORT = 19898
STUB_PORT = 19899
PROXY = f"http://127.0.0.1:{PROXY_PORT}"
STUB = f"http://127.0.0.1:{STUB_PORT}"

PROCS = []


def cleanup(*_):
    for p in PROCS:
        try:
            p.terminate()
            p.wait(timeout=5)
        except Exception:
            try:
                p.kill()
            except Exception:
                pass
    PROCS.clear()


def http(method, url, body=None, headers=None, timeout=30):
    data = body.encode() if isinstance(body, str) else body
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    for k, v in (headers or {}).items():
        req.add_header(k, v)
    t0 = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            raw = r.read()
            return {"status": r.status, "headers": {k.lower(): v for k, v in r.headers.items()},
                    "body": raw, "ms": (time.perf_counter() - t0) * 1000}
    except urllib.error.HTTPError as e:
        return {"status": e.code, "headers": {k.lower(): v for k, v in (e.headers or {}).items()},
                "body": e.read(), "ms": (time.perf_counter() - t0) * 1000}
    except Exception as e:
        return {"status": -1, "headers": {}, "body": repr(e).encode(),
                "ms": (time.perf_counter() - t0) * 1000}


def wait_healthy(url, tries=50):
    for _ in range(tries):
        r = http("GET", url, timeout=2)
        if r["status"] == 200:
            return True
        time.sleep(0.2)
    return False


def stub_last(bench_id):
    r = http("GET", f"{STUB}/__bench/last?{bench_id}")
    try:
        return json.loads(r["body"])
    except Exception:
        return {}


BIG_TOOL_OUTPUT = ("error[E0308]: mismatched types\n --> src/proxy.rs:1?\n" * 700)  # ~38KB


def dialect_requests():
    """Representative request per dialect. Each body embeds a unique bench_id
    so the stub can be interrogated and the proxy response cache never hits."""
    ts = int(time.time() * 1000)

    def bid(name):
        return f"{name}-{ts}"

    d = {}

    # OpenAI chat completions - Cline / OpenCode / Goose / OpenHands shape.
    # Includes a large role:"tool" message: the 15-P1 baseline probe.
    d["openai-chat"] = {
        "path": "/v1/chat/completions",
        "body": {
            "bench_id": bid("openai-chat"),
            "model": "bench-model",
            "messages": [
                {"role": "system", "content": "You are a coding agent."},
                {"role": "user", "content": "Fix the build."},
                {"role": "assistant", "content": None, "tool_calls": [
                    {"id": "call_1", "type": "function",
                     "function": {"name": "bash", "arguments": "{\"cmd\":\"cargo build\"}"}}]},
                {"role": "tool", "tool_call_id": "call_1", "content": BIG_TOOL_OUTPUT},
            ],
        },
        "headers": {"Authorization": "Bearer client-key-should-be-replaced"},
    }

    # OpenAI with tools: response carries big tool_calls arguments -> does the
    # proxy mangle them? (15 §2.3 "compresses tool-call args" hazard)
    d["openai-tools"] = {
        "path": "/v1/chat/completions",
        "body": {
            "bench_id": bid("openai-tools"),
            "model": "bench-model",
            "want_tool_calls": True,
            "messages": [{"role": "user", "content": "write the file (want_tool_calls)"}],
            "tools": [{"type": "function", "function": {
                "name": "write_file", "description": "write a file",
                "parameters": {"type": "object", "properties": {
                    "file_path": {"type": "string"}, "content": {"type": "string"}}}}}],
        },
        "headers": {},
    }

    # OpenAI streaming.
    d["openai-stream"] = {
        "path": "/v1/chat/completions",
        "body": {
            "bench_id": bid("openai-stream"),
            "model": "bench-model",
            "stream": True,
            "messages": [{"role": "user", "content": "stream me"}],
        },
        "headers": {"Accept": "text/event-stream"},
    }

    # Anthropic Messages - Claude Code shape.
    d["anthropic-messages"] = {
        "path": "/v1/messages",
        "body": {
            "bench_id": bid("anthropic"),
            "model": "bench-model",
            "max_tokens": 128,
            "system": "You are a coding agent.",
            "messages": [
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "toolu_1",
                     "content": [{"type": "text", "text": BIG_TOOL_OUTPUT}]},
                    {"type": "text", "text": "now fix it"}]},
            ],
        },
        "headers": {"x-api-key": "client-anthropic-key", "anthropic-version": "2023-06-01"},
    }

    # Gemini generateContent.
    d["gemini-generatecontent"] = {
        "path": "/v1beta/models/gemini-2.5-pro:generateContent",
        "body": {
            "bench_id": bid("gemini"),
            "contents": [
                {"role": "user", "parts": [{"text": "fix the build"}]},
                {"role": "function", "parts": [{"functionResponse": {
                    "name": "bash", "response": {"output": BIG_TOOL_OUTPUT}}}]},
            ],
        },
        "headers": {"x-goog-api-key": "client-gemini-key"},
    }

    return d


def find_ccr_markers(text):
    out, i = [], 0
    while True:
        i = text.find("<<<CCR:", i)
        if i < 0:
            return out
        j = text.find(">>>", i)
        out.append(text[i:j + 3] if j > 0 else text[i:i + 60])
        i = i + 7


def run_dialect(name, spec, reps=5):
    body_json = json.dumps(spec["body"])
    result = {"dialect": name, "path": spec["path"]}

    # 1 through-proxy request (recorded), then latency reps both ways.
    r = http("POST", PROXY + spec["path"], body_json, spec["headers"])
    result["status_via_proxy"] = r["status"]
    resp_text = r["body"].decode("utf-8", "replace")
    seen = stub_last(spec["body"]["bench_id"])

    # Request-side integrity: did the body reach upstream byte-identical?
    upstream_body = seen.get("body", "")
    result["request_reached_upstream"] = bool(upstream_body)
    result["request_byte_identical"] = upstream_body == body_json
    result["request_tool_msgs_compressed"] = ("<<<CCR:" in upstream_body) if upstream_body else None
    up_headers = seen.get("headers", {})
    result["upstream_auth_header"] = up_headers.get("authorization", "")
    result["client_auth_forwarded"] = "client-" in up_headers.get("authorization", "")
    result["x_api_key_forwarded"] = "x-api-key" in up_headers

    # Response-side integrity.
    result["response_markers"] = find_ccr_markers(resp_text)
    result["response_compressed_header"] = r["headers"].get("x-aphrodite-compressed", "")
    result["response_content_type"] = r["headers"].get("content-type", "")

    tool_calls_intact = None
    if name == "openai-tools" and r["status"] == 200:
        try:
            direct = json.loads(http("POST", STUB + spec["path"], body_json, spec["headers"])["body"])
            via = json.loads(resp_text)
            a = direct["choices"][0]["message"].get("tool_calls")
            b = via["choices"][0]["message"].get("tool_calls")
            tool_calls_intact = (json.dumps(a, sort_keys=True) == json.dumps(b, sort_keys=True))
        except Exception as e:
            tool_calls_intact = f"parse-error: {e}"
    result["response_tool_calls_intact"] = tool_calls_intact

    if name == "openai-stream":
        result["stream_is_sse"] = "text/event-stream" in result["response_content_type"]
        direct = http("POST", STUB + spec["path"], body_json, spec["headers"])
        result["stream_byte_identical"] = direct["body"] == r["body"]
        result["stream_note"] = ("proxy buffers whole upstream body before replying "
                                 "(proxy.rs response.bytes().await) - bytes intact but "
                                 "no incremental delivery")

    # Latency: median over reps, fresh bench_id per rep to dodge response cache.
    prox_ms, direct_ms = [], []
    for i in range(reps):
        b = dict(spec["body"])
        b["bench_id"] = f'{spec["body"]["bench_id"]}-r{i}'
        bj = json.dumps(b)
        prox_ms.append(http("POST", PROXY + spec["path"], bj, spec["headers"])["ms"])
        direct_ms.append(http("POST", STUB + spec["path"], bj, spec["headers"])["ms"])
    result["latency_proxy_ms_median"] = round(statistics.median(prox_ms), 2)
    result["latency_direct_ms_median"] = round(statistics.median(direct_ms), 2)
    result["latency_overhead_ms"] = round(result["latency_proxy_ms_median"] - result["latency_direct_ms_median"], 2)

    # Verdict.
    if r["status"] != 200:
        verdict = "rejected"
    elif tool_calls_intact is False or result["response_markers"] and name == "openai-tools":
        verdict = "mangled"
    elif name == "openai-stream" and not result.get("stream_byte_identical", False):
        verdict = "mangled"
    elif not result["request_byte_identical"]:
        verdict = "mangled"
    else:
        verdict = "proxied-ok"
    result["verdict"] = verdict
    return result


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--proxy-bin", default=os.path.join(REPO, "target", "release", "aphrodite"))
    ap.add_argument("--reps", type=int, default=5)
    ap.add_argument("--keep", action="store_true", help="leave proxy+stub running")
    args = ap.parse_args()

    os.makedirs(TMP_DIR, exist_ok=True)
    signal.signal(signal.SIGINT, lambda *a: (cleanup(), sys.exit(130)))
    signal.signal(signal.SIGTERM, lambda *a: (cleanup(), sys.exit(143)))

    stub = subprocess.Popen([sys.executable, os.path.join(HERE, "mock_upstream.py"), str(STUB_PORT)],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    PROCS.append(stub)

    env = dict(os.environ)
    env["APHRODITE_CONFIG_PATH"] = "/nonexistent/aphrodite.toml"  # force CLI mode, not repo aphrodite.toml
    proxy_log = open(os.path.join(TMP_DIR, "proxy-19898.log"), "w")
    proxy = subprocess.Popen(
        [args.proxy_bin,
         "--mode", "token",
         "--listen", f"127.0.0.1:{PROXY_PORT}",   # port-override: CLI --listen (env APHRODITE_LISTEN also works)
         "--api-url", STUB,                        # upstream override: --api-url / APHRODITE_API_URL
         "--api-key", "bench-proxy-key",
         "--ccr-db-path", os.path.join(TMP_DIR, "ccr-bench.db"),
         "--tool-relay"],
        env=env, stdout=proxy_log, stderr=proxy_log)
    PROCS.append(proxy)

    try:
        if not wait_healthy(f"{STUB}/__bench/last?x"):
            print("FATAL: stub did not come up", file=sys.stderr)
            return 1
        if not wait_healthy(f"{PROXY}/health"):
            print("FATAL: proxy did not come up on 19898 - see bench/results/tmp/proxy-19898.log",
                  file=sys.stderr)
            return 1

        results = []
        for name, spec in dialect_requests().items():
            results.append(run_dialect(name, spec, args.reps))

        # CCR retrieval round-trip check for any response marker minted.
        retrieve_check = None
        for res in results:
            for m in res["response_markers"]:
                h = m.split("<<<CCR:")[1].split("|")[0]
                rr = http("POST", f"{PROXY}/retrieve", json.dumps({"hash": h}))
                retrieve_check = {"hash": h, "status": rr["status"],
                                  "found": b"content" in rr["body"] or rr["status"] == 200}
                break
            if retrieve_check:
                break

        out = {
            "bench": "wire_compat",
            "ts": time.strftime("%Y-%m-%dT%H:%M:%S"),
            "proxy_bin": args.proxy_bin,
            "proxy_mode": "token",
            "ports": {"proxy": PROXY_PORT, "stub": STUB_PORT},
            "compress_threshold_note": "token mode base threshold 1024B (proxy.rs TOKEN_COMPRESS_THRESHOLD)",
            "retrieve_roundtrip": retrieve_check,
            "dialects": results,
        }
        ts = time.strftime("%Y%m%d-%H%M%S")
        out_path = os.path.join(RESULTS_DIR, f"wire-compat-{ts}.json")
        with open(out_path, "w") as f:
            json.dump(out, f, indent=2)

        # stdout table
        cols = ["dialect", "verdict", "status_via_proxy", "latency_overhead_ms",
                "request_tool_msgs_compressed", "response_tool_calls_intact"]
        widths = [24, 12, 7, 10, 10, 10]
        print("\n" + " | ".join(c[:w].ljust(w) for c, w in zip(
            ["dialect", "verdict", "status", "ovhd_ms", "req_tool_z", "tc_intact"], widths)))
        print("-" * 90)
        for res in results:
            row = [str(res.get(c, "")) for c in cols]
            print(" | ".join(v[:w].ljust(w) for v, w in zip(row, widths)))
        print(f"\nwrote {out_path}")
        return 0
    finally:
        if not args.keep:
            cleanup()
        proxy_log.close()


if __name__ == "__main__":
    sys.exit(main())
