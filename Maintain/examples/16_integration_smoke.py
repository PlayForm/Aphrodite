"""Atomic test 16 - integration smoke: full marker→compress→retrieve round-trip.

This test wires together the fixed versions of all key subsystems and runs a
complete round-trip without a live proxy or API key:

  1. A large tool output arrives in a Hermes message.
  2. _transform_tool_result compresses it: stores in inline cache, returns marker.
  3. The compressed marker is what the LLM sees in context.
  4. The LLM calls headroom_retrieve with the hash (possibly with pipe suffix).
  5. _resolve_one strips the suffix, looks up the inline cache, returns content.
  6. The original content is verified to be intact.

Run:  python examples/16_integration_smoke.py
Pass: prints OK
"""

import hashlib
import re
import json

# ── constants ───────────────────────────────────────────────────────────────

GLYPH_OPEN = "\u2ab7"  # ⫷
GLYPH_CLOSE = "\u2ab8"  # ⫸
MARKER_RE = re.compile(rf"{GLYPH_OPEN}CCR:([^{GLYPH_CLOSE}]+){GLYPH_CLOSE}")
INLINE_THRESHOLD = 512  # small for the test

# ── inline store (replaces the proxy for this test) ─────────────────────────

_store: dict[str, str] = {}


def _inline_store_put(content: str) -> str:
    h = hashlib.sha256(content.encode()).hexdigest()[:16]
    _store[h] = content
    return h


def _inline_store_get(h: str) -> str | None:
    return _store.get(h)


# ── smart_marker (fixed glyphs) ──────────────────────────────────────────────


def smart_marker(hash_val: str, kind: str, size: int) -> str:
    return f"{GLYPH_OPEN}CCR:{hash_val}|{kind}|{size}{GLYPH_CLOSE}"


# ── _transform_tool_result (fixed: no [:2000] truncation, correct glyphs) ───


def transform_tool_result(message: dict) -> dict:
    content = message.get("content", "")
    if not isinstance(content, str) or len(content) <= INLINE_THRESHOLD:
        return message
    h = _inline_store_put(content)
    marker = smart_marker(h, "tool", len(content))
    return {**message, "content": marker}


# ── retrieve (fixed: strip pipe suffix, try inline cache first) ──────────────


def resolve_one(hash_arg: str) -> str | None:
    clean = hash_arg.split("|")[0].strip()  # fix 14
    return _inline_store_get(clean)


# ── should_compress (fixed threshold check) ─────────────────────────────────


def should_compress(
    prompt_tokens: int, context_length: int = 128_000, threshold_pct: float = 75.0
) -> bool:
    if not prompt_tokens:
        return True
    return (prompt_tokens / context_length * 100) >= threshold_pct


# ── health check (fixed JSON parser) ─────────────────────────────────────────


def alive(body: str) -> bool:
    if body.strip() == "ok":
        return True
    try:
        return json.loads(body).get("status") in ("healthy", "ok")
    except Exception:
        return False


# ── SMOKE TEST ───────────────────────────────────────────────────────────────

ORIGINAL = "X" * 4096  # large tool output

# Step 1: compress
msg = {"role": "tool", "content": ORIGINAL}
compressed = transform_tool_result(msg)
marker_str = compressed["content"]

assert MARKER_RE.search(marker_str), f"Marker not found in: {marker_str!r}"
assert len(marker_str) < len(ORIGINAL), "Marker must be shorter than original"

# Step 2: extract hash as the LLM would pass it (with pipe suffix)
match = MARKER_RE.search(marker_str)
full_token = match.group(1)  # e.g. "abc123|tool|4096"
assert "|" in full_token, "Token should include pipe-delimited metadata"

# Step 3: retrieve with the suffixed token (simulates LLM passing full token)
retrieved = resolve_one(full_token)
assert retrieved == ORIGINAL, "Retrieved content must exactly match original"

# Step 4: verify health check parses serde_json correctly
proxy_body = '{"status": "healthy", "version": "0.2.0"}'  # note: space after colon
assert alive(proxy_body), "Fixed health check must accept serde_json response"

# Step 5: threshold gate
assert should_compress(10_000) is False, "10k/128k = 7.8% - below 75% threshold"
assert should_compress(100_000) is True, "100k/128k = 78% - above threshold"

print("16 OK - full integration smoke test passed")
print(f"  original size  : {len(ORIGINAL):,} chars")
print(f"  marker size    : {len(marker_str):,} chars")
print(f"  retrieved size : {len(retrieved):,} chars  (exact match)")
print(f"  health check   : alive({proxy_body[:30]!r}...) = True")
print(f"  compress gate  : 10k tokens → False,  100k tokens → True")
