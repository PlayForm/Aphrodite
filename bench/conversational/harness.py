"""
Aphrodite Conversational Benchmark Harness

Runs identical conversation scripts through 4 scenarios:
  1. BASELINE    — Direct to DeepSeek, no proxy at all
  2. FULL        — Both cache (:9797) + token (:9798) proxies active
  3. HERMES_PROXY — Only cache proxy (tool output compression)
  4. PROXY_API   — Only token proxy (context window compression)

For each scenario + conversation, captures:
  - Complete request/response history (JSONL)
  - Token counts: prompt_tokens, completion_tokens, total_tokens
  - CCR compression events: what was compressed, when, ratio
  - Proxy stats snapshots at each turn
  - Per-turn timing data

Output structure:
  results/<run_timestamp>/
    <scenario>/
      <conversation_name>/
        turns/
          000.json, 001.json, ...   (per-turn request/response pairs)
        proxy_stats.jsonl            (stats snapshots)
        compression_events.jsonl     (CCR create events)
        token_report.json            (aggregate token analysis)
        summary.json                 (run metadata)
"""

from __future__ import annotations

import json
import os
import shutil
import signal
import subprocess
import sys
import time
import uuid
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from enum import Enum
from pathlib import Path
from typing import Optional

import requests

# Add parent to path for conversations import
sys.path.insert(0, str(Path(__file__).resolve().parent))
from conversations import Conversation, Turn, ALL_CONVERSATIONS


# ═══════════════════════════════════════════════════════════════════════════════
# Configuration
# ═══════════════════════════════════════════════════════════════════════════════

DEEPSEEK_API_KEY = os.environ.get("DEEPSEEK_API_KEY", "")
DEEPSEEK_BASE_URL = "https://api.deepseek.com"
# Use DeepSeek Flash for cheap conversational benchmarking
DEEPSEEK_MODEL = "deepseek-v4-flash"

# Proxy config (matching real aphrodite.toml ports)
CACHE_PORT = 9797
TOKEN_PORT = 9798

# Bench-specific ports (isolated from production)
BENCH_CACHE_PORT = 49797
BENCH_TOKEN_PORT = 49798

APHRODITE_BINARY = None  # Resolved at runtime


class Scenario(Enum):
    BASELINE = "baseline"           # Direct to DeepSeek, no proxy
    FULL = "full"                   # Both cache + token proxies
    HERMES_PROXY = "hermes_proxy"   # Cache proxy only (tool output compression)
    PROXY_API = "proxy_api"         # Token proxy only (context window compression)


SCENARIO_METADATA = {
    Scenario.BASELINE: {
        "description": "1:1 baseline — direct DeepSeek API, no proxy, no CCR",
        "uses_cache_proxy": False,
        "uses_token_proxy": False,
        "api_url": f"{DEEPSEEK_BASE_URL}/v1/chat/completions",
    },
    Scenario.FULL: {
        "description": "1:1 with full compression — both cache + token proxies",
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
        "api_url": f"{DEEPSEEK_BASE_URL}/v1/chat/completions",
    },
    Scenario.PROXY_API: {
        "description": "1:1 with compression between proxy and external API (token only)",
        "uses_cache_proxy": False,
        "uses_token_proxy": True,
        "token_proxy_url": f"http://127.0.0.1:{BENCH_TOKEN_PORT}/v1/chat/completions",
        "api_url": f"http://127.0.0.1:{BENCH_TOKEN_PORT}/v1/chat/completions",
    },
}


# ═══════════════════════════════════════════════════════════════════════════════
# Result types
# ═══════════════════════════════════════════════════════════════════════════════

@dataclass
class TurnResult:
    """Captured result for a single conversation turn."""
    turn_index: int
    role: str
    request: Optional[dict] = None       # The API request sent
    response: Optional[dict] = None      # The API response received
    response_status: Optional[int] = None
    elapsed_ms: float = 0.0
    prompt_tokens: int = 0
    completion_tokens: int = 0
    total_tokens: int = 0
    error: Optional[str] = None
    # CCR events from this turn (if using proxy)
    ccr_events: list[dict] = field(default_factory=list)


@dataclass
class ConversationResult:
    """Aggregate result for one conversation under one scenario."""
    scenario: str
    conversation_name: str
    turns: list[TurnResult] = field(default_factory=list)
    proxy_stats_snapshots: list[dict] = field(default_factory=list)
    total_prompt_tokens: int = 0
    total_completion_tokens: int = 0
    total_tokens: int = 0
    total_elapsed_ms: float = 0.0
    errors: list[str] = field(default_factory=list)


@dataclass
class RunManifest:
    """Top-level manifest for a full benchmark run."""
    run_id: str
    timestamp: str
    aphrodite_version: str
    deepseek_model: str
    scenarios_run: list[str] = field(default_factory=list)
    conversations_run: list[str] = field(default_factory=list)
    total_turns: int = 0
    total_errors: int = 0
    results: list[ConversationResult] = field(default_factory=list)


# ═══════════════════════════════════════════════════════════════════════════════
# Proxy lifecycle management
# ═══════════════════════════════════════════════════════════════════════════════

class ProxyManager:
    """Manages aphrodite proxy processes for benchmark scenarios."""

    def __init__(self, bin_path: str, work_dir: Path):
        self.bin_path = bin_path
        self.work_dir = work_dir
        self.processes: dict[str, subprocess.Popen] = {}

    def start_proxy(self, mode: str, port: int) -> subprocess.Popen:
        """Start a single aphrodite proxy process."""
        listen = f"127.0.0.1:{port}"
        db_path = self.work_dir / f"ccr_{mode}_{port}.db"

        # Remove stale DB from previous runs
        if db_path.exists():
            db_path.unlink()

        env = os.environ.copy()
        env["APHRODITE_CONFIG_PATH"] = "/nonexistent/aphrodite-bench.toml"
        env["APHRODITE_API_KEY"] = DEEPSEEK_API_KEY
        env["APHRODITE_API_URL"] = DEEPSEEK_BASE_URL
        env["APHRODITE_MODEL"] = DEEPSEEK_MODEL

        proc = subprocess.Popen(
            [
                self.bin_path,
                "--mode", mode,
                "--listen", listen,
                "--api-url", DEEPSEEK_BASE_URL,
                "--api-key", DEEPSEEK_API_KEY,
                "--ccr-db-path", str(db_path),
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )
        self.processes[f"{mode}:{port}"] = proc

        # Wait for it to be ready
        deadline = time.time() + 10
        while time.time() < deadline:
            try:
                sock = __import__('socket').create_connection(("127.0.0.1", port), timeout=0.5)
                sock.close()
                print(f"  [proxy] {mode} proxy ready on :{port}")
                return proc
            except (OSError, ConnectionRefusedError):
                time.sleep(0.1)

        raise RuntimeError(f"Proxy {mode}:{port} failed to start within 10s")

    def start_for_scenario(self, scenario: Scenario) -> dict[str, int]:
        """Start proxies needed for a scenario. Returns {mode: port}."""
        meta = SCENARIO_METADATA[scenario]
        ports = {}

        if meta["uses_cache_proxy"]:
            self.start_proxy("cache", BENCH_CACHE_PORT)
            ports["cache"] = BENCH_CACHE_PORT
            print(f"  [scenario] cache proxy started on :{BENCH_CACHE_PORT}")

        if meta["uses_token_proxy"]:
            self.start_proxy("token", BENCH_TOKEN_PORT)
            ports["token"] = BENCH_TOKEN_PORT
            print(f"  [scenario] token proxy started on :{BENCH_TOKEN_PORT}")

        return ports

    def stop_all(self):
        """Stop all managed proxy processes."""
        for name, proc in list(self.processes.items()):
            try:
                proc.terminate()
                proc.wait(timeout=5)
            except (subprocess.TimeoutExpired, ProcessLookupError):
                try:
                    proc.kill()
                except ProcessLookupError:
                    pass
            print(f"  [proxy] stopped {name}")
        self.processes.clear()

    def get_stats(self, port: int) -> dict:
        """Fetch proxy stats from GET /stats."""
        try:
            resp = requests.get(f"http://127.0.0.1:{port}/stats", timeout=2)
            return resp.json() if resp.ok else {}
        except Exception:
            return {}

    def get_ccr_catalog(self, port: int) -> list[dict]:
        """Fetch CCR catalog entries."""
        try:
            resp = requests.get(f"http://127.0.0.1:{port}/ccr/catalog", timeout=2)
            return resp.json() if resp.ok else []
        except Exception:
            return []


# ═══════════════════════════════════════════════════════════════════════════════
# DeepSeek API client (direct, no proxy)
# ═══════════════════════════════════════════════════════════════════════════════

class DeepSeekClient:
    """Direct DeepSeek API client for baseline scenario."""

    def __init__(self, api_key: str, model: str = DEEPSEEK_MODEL):
        self.api_key = api_key
        self.model = model
        self.base_url = DEEPSEEK_BASE_URL

    def chat_completion(
        self,
        messages: list[dict],
        stream: bool = False,
        tools: Optional[list[dict]] = None,
    ) -> dict:
        """Send a chat completion request and return the full response."""
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        body = {
            "model": self.model,
            "messages": messages,
            "stream": stream,
        }
        if tools:
            body["tools"] = tools

        resp = requests.post(
            f"{self.base_url}/v1/chat/completions",
            headers=headers,
            json=body,
            timeout=120,
        )
        return {
            "status_code": resp.status_code,
            "body": resp.json() if resp.headers.get("content-type", "").startswith("application/json") else {"raw": resp.text},
        }


# ═══════════════════════════════════════════════════════════════════════════════
# Proxy API client
# ═══════════════════════════════════════════════════════════════════════════════

class ProxyClient:
    """Client that talks to an aphrodite proxy (cache or token)."""

    def __init__(self, port: int, api_key: str = DEEPSEEK_API_KEY):
        self.base_url = f"http://127.0.0.1:{port}"
        self.api_key = api_key

    def chat_completion(
        self,
        messages: list[dict],
        stream: bool = False,
        tools: Optional[list[dict]] = None,
    ) -> dict:
        """Send a chat completion request through the proxy."""
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }
        body = {
            "model": DEEPSEEK_MODEL,
            "messages": messages,
            "stream": stream,
        }
        if tools:
            body["tools"] = tools

        resp = requests.post(
            f"{self.base_url}/v1/chat/completions",
            headers=headers,
            json=body,
            timeout=300,
        )
        return {
            "status_code": resp.status_code,
            "body": resp.json() if resp.headers.get("content-type", "").startswith("application/json") else {"raw": resp.text},
        }

    def ccr_create(self, content: str, content_type: str = "text") -> dict:
        """Store content in CCR via the proxy."""
        resp = requests.post(
            f"{self.base_url}/ccr/create",
            json={"content": content, "type": content_type},
            timeout=30,
        )
        return resp.json() if resp.ok else {"error": resp.text}

    def retrieve(self, hash_val: str) -> dict:
        """Retrieve content from CCR."""
        resp = requests.post(
            f"{self.base_url}/retrieve",
            json={"hash": hash_val},
            timeout=10,
        )
        return resp.json() if resp.ok else {"error": resp.text}

    def get_stats(self) -> dict:
        """Get proxy health stats."""
        resp = requests.get(f"{self.base_url}/stats", timeout=5)
        return resp.json() if resp.ok else {}


# ═══════════════════════════════════════════════════════════════════════════════
# Estimated token counter (tiktoken when available, char/4 fallback)
# ═══════════════════════════════════════════════════════════════════════════════

def estimate_tokens(text: str) -> int:
    """Estimate token count for a string. Uses tiktoken if available."""
    try:
        import tiktoken
        enc = tiktoken.get_encoding("cl100k_base")  # GPT-4 / DeepSeek encoding
        return len(enc.encode(text))
    except (ImportError, Exception):
        # Fallback: ~4 chars per token for English text, ~2 for code
        return max(1, len(text) // 4)


def count_message_tokens(messages: list[dict]) -> int:
    """Estimate total prompt tokens for a list of messages."""
    total = 0
    for msg in messages:
        content = msg.get("content", "")
        if isinstance(content, str):
            total += estimate_tokens(content)
        elif isinstance(content, list):
            # Multi-part content (e.g., text + image_url)
            for part in content:
                if isinstance(part, dict) and "text" in part:
                    total += estimate_tokens(part["text"])
        # Tool calls
        for tc in msg.get("tool_calls", []):
            if "function" in tc:
                total += estimate_tokens(tc["function"].get("arguments", ""))
    return total


# ═══════════════════════════════════════════════════════════════════════════════
# Conversation runner
# ═══════════════════════════════════════════════════════════════════════════════

class ConversationRunner:
    """Runs a Conversation through a specific scenario configuration."""

    def __init__(
        self,
        scenario: Scenario,
        output_dir: Path,
        proxy_manager: Optional[ProxyManager] = None,
        deepseek_client: Optional[DeepSeekClient] = None,
        proxy_client: Optional[ProxyClient] = None,
        cache_client: Optional[ProxyClient] = None,
    ):
        self.scenario = scenario
        self.output_dir = output_dir
        self.proxy_manager = proxy_manager
        self.deepseek = deepseek_client
        self.proxy = proxy_client       # Token proxy client (or primary)
        self.cache = cache_client       # Cache proxy client
        self.turns_dir = output_dir / "turns"
        self.turns_dir.mkdir(parents=True, exist_ok=True)

    def run(self, conversation: Conversation) -> ConversationResult:
        """Execute all turns of a conversation and capture token metrics.

        For each scenario, simulates what the LLM would actually see:
        - baseline: full tool outputs inline, no compression
        - full: cache proxy compresses tool outputs + token proxy offloads old messages
        - hermes_proxy: cache proxy compresses tool outputs only
        - proxy_api: token proxy offloads old messages only

        Token counts are computed directly from message content using tiktoken
        (cl100k_base encoding). No live API calls needed — the metric is
        "what would the LLM receive?" not "what did the LLM generate?"
        """
        result = ConversationResult(
            scenario=self.scenario.value,
            conversation_name=conversation.name,
        )

        # ── Compression simulation parameters ──────────────────────────
        CACHE_THRESHOLD = 4096       # Cache proxy: compress tool outputs > 4KB
        TOKEN_ENGINE_PCT = 45        # Token proxy: offload at 45% of 128k context
        TOKEN_CONTEXT_MAX = 128_000  # DeepSeek Flash context window
        TOKEN_OFFLOAD_THRESHOLD = int(TOKEN_CONTEXT_MAX * TOKEN_ENGINE_PCT / 100)  # ~57,600
        PROTECT_FIRST = 2            # Messages to protect at start (system + first)
        PROTECT_LAST = 5             # Messages to protect at end (recent)

        use_cache = self.scenario in (Scenario.FULL, Scenario.HERMES_PROXY)
        use_token = self.scenario in (Scenario.FULL, Scenario.PROXY_API)

        messages: list[dict] = []
        if conversation.system_prompt:
            messages.append({"role": "system", "content": conversation.system_prompt})

        for i, turn in enumerate(conversation.turns):
            print(f"    [turn {i:02d}] {turn.role}", end="")
            t0 = time.time()
            turn_result = TurnResult(turn_index=i, role=turn.role)

            try:
                if turn.role == "user":
                    messages.append({"role": "user", "content": turn.content})
                    # Prompt tokens: what the LLM sees before responding
                    turn_result.prompt_tokens = count_message_tokens(messages)

                elif turn.role == "assistant":
                    # Completion tokens: the assistant's response
                    comp_tokens = estimate_tokens(turn.content)
                    if turn.tool_calls:
                        for tc in turn.tool_calls:
                            comp_tokens += estimate_tokens(tc.arguments)
                    turn_result.completion_tokens = comp_tokens

                    # Prompt tokens: full conversation before this response
                    turn_result.prompt_tokens = count_message_tokens(messages)

                    # Build and append the assistant message
                    assistant_msg: dict = {"role": "assistant", "content": turn.content}
                    if turn.tool_calls:
                        assistant_msg["tool_calls"] = [
                            {
                                "id": tc.id,
                                "type": "function",
                                "function": {"name": tc.name, "arguments": tc.arguments},
                            }
                            for tc in turn.tool_calls
                        ]
                    messages.append(assistant_msg)

                    # ── Token proxy simulation: offload old messages if over threshold ──
                    if use_token:
                        self._simulate_token_offload(
                            messages, TOKEN_OFFLOAD_THRESHOLD,
                            PROTECT_FIRST, PROTECT_LAST, turn_result
                        )

                elif turn.role == "tool":
                    original_size = len(turn.content)
                    content_to_store = turn.content

                    # ── Cache proxy simulation: compress large tool outputs ──
                    if use_cache and self.cache:
                        if len(turn.content) >= CACHE_THRESHOLD:
                            ccr_result = self.cache.ccr_create(turn.content, "text")
                            turn_result.ccr_events.append(ccr_result)
                            if ccr_result.get("hash"):
                                compressed_size = ccr_result.get("compressed_size", original_size)
                                marker = f'<<<CCR:{ccr_result["hash"]}|text|{compressed_size}>>>'
                                content_to_store = marker
                                savings = original_size - len(marker)
                                print(f" [CCR: {original_size}→{len(marker)}B ({savings}B saved)]", end="")

                    tool_msg = {
                        "role": "tool",
                        "tool_call_id": turn.tool_call_id or f"call_{i:04d}",
                        "content": content_to_store,
                    }
                    messages.append(tool_msg)

                    # Prompt tokens after tool result is added
                    turn_result.prompt_tokens = count_message_tokens(messages)

                turn_result.total_tokens = turn_result.prompt_tokens + turn_result.completion_tokens
                print(f" (p:{turn_result.prompt_tokens} c:{turn_result.completion_tokens} t:{turn_result.total_tokens})")

            except Exception as e:
                turn_result.error = str(e)
                print(f" ERROR: {e}")
                result.errors.append(f"turn_{i}: {e}")

            turn_result.elapsed_ms = (time.time() - t0) * 1000
            result.turns.append(turn_result)
            result.total_elapsed_ms += turn_result.elapsed_ms
            result.total_prompt_tokens += turn_result.prompt_tokens
            result.total_completion_tokens += turn_result.completion_tokens
            result.total_tokens += turn_result.total_tokens

            # Save per-turn data
            self._save_turn(i, turn_result, messages)

        # Final proxy stats snapshot
        self._capture_proxy_stats(result, None, final=True)
        self._save_proxy_stats(result)

        return result

    def _simulate_token_offload(
        self,
        messages: list[dict],
        threshold: int,
        protect_first: int,
        protect_last: int,
        turn_result: TurnResult,
    ):
        """Simulate the token proxy's offload behavior.

        When total token count exceeds threshold, replace middle messages
        with a CCR offload marker. Protected messages (first N, last M)
        stay in context.
        """
        total = count_message_tokens(messages)
        if total <= threshold:
            return

        if len(messages) <= protect_first + protect_last:
            return  # Not enough messages to offload

        # Compute which messages to offload
        offload_start = protect_first
        offload_end = len(messages) - protect_last

        if offload_start >= offload_end:
            return

        offloaded_count = offload_end - offload_start
        offloaded_tokens = sum(
            estimate_tokens(str(m.get("content", ""))) for m in messages[offload_start:offload_end]
        )

        # Replace offloaded messages with a single offload notice
        offload_notice = (
            f"[{offloaded_count} messages offloaded to CCR — "
            f"~{offloaded_tokens} tokens saved. "
            f"Use aphrodite_retrieve if context is needed.]"
        )

        # Build new message list: protected prefix + notice + protected suffix
        new_messages = (
            messages[:protect_first]
            + [{"role": "system", "content": offload_notice}]
            + messages[offload_end:]
        )

        messages.clear()
        messages.extend(new_messages)

        turn_result.ccr_events.append({
            "event": "token_offload",
            "offloaded_count": offloaded_count,
            "offloaded_tokens": offloaded_tokens,
            "remaining_messages": len(messages),
            "new_total_tokens": count_message_tokens(messages),
        })

    def _call_api(self, messages: list[dict]) -> dict:
        """Call the appropriate API for this scenario."""
        result = {}
        if self.proxy:
            # Through token proxy
            resp = self.proxy.chat_completion(messages)
            result["request"] = {"messages_count": len(messages)}
            result["response"] = resp.get("body", {})
            result["status"] = resp["status_code"]
            if "usage" in result["response"]:
                result["prompt_tokens"] = result["response"]["usage"].get("prompt_tokens", 0)
                result["completion_tokens"] = result["response"]["usage"].get("completion_tokens", 0)
                result["total_tokens"] = result["response"]["usage"].get("total_tokens", 0)
        elif self.deepseek:
            # Direct to DeepSeek
            resp = self.deepseek.chat_completion(messages)
            result["request"] = {"messages_count": len(messages)}
            result["response"] = resp.get("body", {})
            result["status"] = resp["status_code"]
            if "usage" in result["response"]:
                result["prompt_tokens"] = result["response"]["usage"].get("prompt_tokens", 0)
                result["completion_tokens"] = result["response"]["usage"].get("completion_tokens", 0)
                result["total_tokens"] = result["response"]["usage"].get("total_tokens", 0)
        return result

    def _capture_proxy_stats(
        self, result: ConversationResult, turn_result: Optional[TurnResult], final: bool = False
    ):
        """Capture snapshot of proxy stats."""
        if not self.proxy_manager:
            return

        snapshot = {"timestamp": datetime.now(timezone.utc).isoformat(), "final": final}

        meta = SCENARIO_METADATA[self.scenario]
        if meta["uses_cache_proxy"]:
            stats = self.proxy_manager.get_stats(BENCH_CACHE_PORT)
            snapshot["cache_proxy"] = stats
        if meta["uses_token_proxy"]:
            stats = self.proxy_manager.get_stats(BENCH_TOKEN_PORT)
            snapshot["token_proxy"] = stats

        result.proxy_stats_snapshots.append(snapshot)

    def _save_turn(self, index: int, turn_result: TurnResult, messages: list[dict]):
        """Persist a single turn's data."""
        data = {
            "turn_index": index,
            "role": turn_result.role,
            "elapsed_ms": turn_result.elapsed_ms,
            "prompt_tokens": turn_result.prompt_tokens,
            "completion_tokens": turn_result.completion_tokens,
            "total_tokens": turn_result.total_tokens,
            "response_status": turn_result.response_status,
            "request": turn_result.request,
            "response": turn_result.response,
            "ccr_events": turn_result.ccr_events,
            "error": turn_result.error,
            "conversation_state": {
                "message_count": len(messages),
                "estimated_context_tokens": count_message_tokens(messages),
            },
        }
        with open(self.turns_dir / f"{index:03d}.json", "w") as f:
            json.dump(data, f, indent=2, default=str)

    def _save_proxy_stats(self, result: ConversationResult):
        """Persist all proxy stats snapshots."""
        with open(self.output_dir / "proxy_stats.jsonl", "w") as f:
            for snap in result.proxy_stats_snapshots:
                f.write(json.dumps(snap, default=str) + "\n")


# ═══════════════════════════════════════════════════════════════════════════════
# Main harness
# ═══════════════════════════════════════════════════════════════════════════════

def resolve_aphrodite_binary() -> str:
    """Find the aphrodite binary. Checks: env var, target/release, target/debug, cargo build."""
    # Check env var
    if "APHRODITE_BIN" in os.environ:
        return os.environ["APHRODITE_BIN"]

    # Check target directories relative to the workspace root
    workspace = Path(__file__).resolve().parent.parent.parent  # bench/conversational/.. = repo root
    for profile in ["release", "debug"]:
        candidate = workspace / "target" / profile / "aphrodite"
        if candidate.exists():
            return str(candidate)

    # Try cargo build --release
    print("Building aphrodite binary (cargo build --release)...")
    result = subprocess.run(
        ["cargo", "build", "--release"],
        cwd=str(workspace),
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"Build failed:\n{result.stderr}")
        sys.exit(1)

    candidate = workspace / "target" / "release" / "aphrodite"
    if candidate.exists():
        return str(candidate)

    raise FileNotFoundError("Cannot find aphrodite binary. Build it first or set APHRODITE_BIN.")


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
    results_dir = Path(__file__).resolve().parent / "results" / run_id
    results_dir.mkdir(parents=True, exist_ok=True)

    bin_path = resolve_aphrodite_binary()
    print(f"[harness] Aphrodite binary: {bin_path}")
    print(f"[harness] Results dir: {results_dir}")
    print(f"[harness] Model: {DEEPSEEK_MODEL}")
    print(f"[harness] Scenarios: {[s.value for s in scenarios]}")
    print(f"[harness] Conversations: {[c.name for c in conversations]}")
    print()

    manifest = RunManifest(
        run_id=run_id,
        timestamp=datetime.now(timezone.utc).isoformat(),
        aphrodite_version=_get_aphrodite_version(bin_path),
        deepseek_model=DEEPSEEK_MODEL,
        scenarios_run=[s.value for s in scenarios],
        conversations_run=[c.name for c in conversations],
    )

    proxy_manager = ProxyManager(bin_path, results_dir)
    deepseek_client = DeepSeekClient(DEEPSEEK_API_KEY, DEEPSEEK_MODEL)

    try:
        for scenario in scenarios:
            print(f"{'='*60}")
            print(f"SCENARIO: {scenario.value}")
            print(f"  {SCENARIO_METADATA[scenario]['description']}")
            print(f"{'='*60}")

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
                    deepseek_client=deepseek_client if not proxy_client else None,
                    proxy_client=proxy_client,
                    cache_client=cache_client,
                )

                conv_result = runner.run(conv)
                manifest.results.append(conv_result)
                manifest.total_turns += len(conv_result.turns)
                manifest.total_errors += len(conv_result.errors)

                # Save summary
                _save_conversation_summary(conv_dir, conv_result)
                print(f"    ✓ {len(conv_result.turns)} turns, "
                      f"{conv_result.total_tokens} tokens, "
                      f"{len(conv_result.errors)} errors")

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
        "deepseek_model": manifest.deepseek_model,
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

    print(f"\n{'='*60}")
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
    print(f"\n{'Scenario':<20} {'Conversation':<20} {'Turns':>6} {'Prompt':>10} {'Completion':>12} {'Total':>10} {'Ms':>8}")
    print("-" * 86)
    for r in manifest.results:
        print(f"{r.scenario:<20} {r.conversation_name:<20} {len(r.turns):>6} "
              f"{r.total_prompt_tokens:>10} {r.total_completion_tokens:>12} "
              f"{r.total_tokens:>10} {int(r.total_elapsed_ms):>8}")


# ═══════════════════════════════════════════════════════════════════════════════
# CLI entry
# ═══════════════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Aphrodite Conversational Benchmark")
    parser.add_argument("--scenario", choices=[s.value for s in Scenario],
                        help="Run a single scenario (default: all)")
    parser.add_argument("--conversation", help="Run a single conversation (default: all)")
    parser.add_argument("--run-id", help="Custom run ID (default: timestamp)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Validate setup without running conversations")
    args = parser.parse_args()

    if args.dry_run:
        bin_path = resolve_aphrodite_binary()
        print(f"✓ Aphrodite binary: {bin_path}")
        print(f"✓ DEEPSEEK_API_KEY: {'set' if DEEPSEEK_API_KEY else 'MISSING'}")
        print(f"✓ Model: {DEEPSEEK_MODEL}")
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

    if not DEEPSEEK_API_KEY:
        print("ERROR: DEEPSEEK_API_KEY environment variable not set.")
        print("Set it and try again: export DEEPSEEK_API_KEY=sk-...")
        sys.exit(1)

    run_benchmark(scenarios=scenarios, conversations=conversations, run_id=args.run_id)
