"""Estimated token counting (tiktoken when available, char/4 fallback)."""
from __future__ import annotations

def estimate_tokens(text: str) -> int:
    """Estimate token count for a string. Uses tiktoken if available."""
    try:
        import tiktoken

        enc = tiktoken.get_encoding("cl100k_base")  # GPT-4 / DeepSeek-class encoding
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
