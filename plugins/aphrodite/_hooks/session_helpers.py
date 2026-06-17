"""aphrodite — session helper utilities: turn grouping, preview extraction, read keywords."""


def _group_into_turns(conversation_history):
    """Group messages into turns (user → assistant → tools)."""
    turns, current, turn_num = [], None, 0
    for msg in conversation_history:
        role, content = msg.get("role", ""), msg.get("content", "")
        if role == "user":
            if current:
                turns.append(current)
            turn_num += 1
            current = {"id": turn_num, "user": str(content)[:1000]}
        elif role == "assistant" and current:
            current["assistant"] = str(content)[:1000]
        elif role == "tool" and current:
            raw = str(content)[:200] if content else ""
            if raw:
                current.setdefault("tools", []).append(raw)
    if current:
        turns.append(current)
    return turns


def _extract_preview(marker, conversation_history):
    """Extract a short preview for a CCR marker from conversation history (fallback)."""
    h = marker["hash"]
    for msg in conversation_history:
        c = msg.get("content", "")
        if isinstance(c, str) and h in c:
            idx = c.find(h)
            after = c[idx + len(h):].strip()
            if ">>>" in after:
                after = after.split(">>>", 1)[-1].strip()
            return after[:80].strip()
    return ""


_READ_KEYWORDS: frozenset = frozenset({
    "read", "show", "view", "get", "cat", "display", "retrieve",
    "fetch", "look", "see", "open", "inspect", "check", "print",
    "dump", "output",
})
