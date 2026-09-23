"""ConversationRunner: executes a Conversation through a scenario."""
from __future__ import annotations
import json
import time
from datetime import datetime, timezone
from .proxy_client import ProxyClient
from .proxy_manager import ProxyManager
from .provider_client import ProviderClient
from .results import ConversationResult, TurnResult
from .scenarios import BENCH_CACHE_PORT, BENCH_TOKEN_PORT, SCENARIO_METADATA, Scenario
from .tokens import count_message_tokens, estimate_tokens

class ConversationRunner:
    """Runs a Conversation through a specific scenario configuration."""

    def __init__(
        self,
        scenario: Scenario,
        output_dir: Path,
        proxy_manager: Optional[ProxyManager] = None,
        provider_client: Optional[ProviderClient] = None,
        proxy_client: Optional[ProxyClient] = None,
        cache_client: Optional[ProxyClient] = None,
    ):
        self.scenario = scenario
        self.output_dir = output_dir
        self.proxy_manager = proxy_manager
        self.provider = provider_client
        self.proxy = proxy_client  # Token proxy client (or primary)
        self.cache = cache_client  # Cache proxy client
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
        (cl100k_base encoding). No live API calls needed - the metric is
        "what would the LLM receive?" not "what did the LLM generate?"
        """
        result = ConversationResult(
            scenario=self.scenario.value,
            conversation_name=conversation.name,
        )

        # ── Compression simulation parameters ──────────────────────────
        CACHE_THRESHOLD = 4096  # Cache proxy: compress tool outputs > 4KB
        TOKEN_ENGINE_PCT = 45  # Token proxy: offload at 45% of 128k context
        TOKEN_CONTEXT_MAX = 128_000  # Provider context window
        TOKEN_OFFLOAD_THRESHOLD = int(TOKEN_CONTEXT_MAX * TOKEN_ENGINE_PCT / 100)  # ~57,600
        PROTECT_FIRST = 2  # Messages to protect at start (system + first)
        PROTECT_LAST = 5  # Messages to protect at end (recent)

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
                            messages,
                            TOKEN_OFFLOAD_THRESHOLD,
                            PROTECT_FIRST,
                            PROTECT_LAST,
                            turn_result,
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
                                marker = f"<<<CCR:{ccr_result['hash']}|text|{compressed_size}>>>"
                                content_to_store = marker
                                savings = original_size - len(marker)
                                print(
                                    f" [CCR: {original_size}→{len(marker)}B ({savings}B saved)]",
                                    end="",
                                )

                    tool_msg = {
                        "role": "tool",
                        "tool_call_id": turn.tool_call_id or f"call_{i:04d}",
                        "content": content_to_store,
                    }
                    messages.append(tool_msg)

                    # Prompt tokens after tool result is added
                    turn_result.prompt_tokens = count_message_tokens(messages)

                turn_result.total_tokens = turn_result.prompt_tokens + turn_result.completion_tokens
                print(
                    f" (p:{turn_result.prompt_tokens} c:{turn_result.completion_tokens} t:{turn_result.total_tokens})"
                )

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
            f"[{offloaded_count} messages offloaded to CCR - "
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

        turn_result.ccr_events.append(
            {
                "event": "token_offload",
                "offloaded_count": offloaded_count,
                "offloaded_tokens": offloaded_tokens,
                "remaining_messages": len(messages),
                "new_total_tokens": count_message_tokens(messages),
            }
        )

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
                result["completion_tokens"] = result["response"]["usage"].get(
                    "completion_tokens", 0
                )
                result["total_tokens"] = result["response"]["usage"].get("total_tokens", 0)
        elif self.provider:
            # Direct to the Hermes-resolved provider
            resp = self.provider.chat_completion(messages)
            result["request"] = {"messages_count": len(messages)}
            result["response"] = resp.get("body", {})
            result["status"] = resp["status_code"]
            if "usage" in result["response"]:
                result["prompt_tokens"] = result["response"]["usage"].get("prompt_tokens", 0)
                result["completion_tokens"] = result["response"]["usage"].get(
                    "completion_tokens", 0
                )
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
