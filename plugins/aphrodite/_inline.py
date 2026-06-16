"""aphrodite - inline compression (zlib fallback when proxy is down)."""

import zlib

from ._core import _inline_store, _inline_store_put


def _inline_compress(content):
    """Compress content locally using zlib, store in session dict. Returns (hash, compressed_size)."""
    try:
        raw_bytes = content.encode("utf-8")
    except UnicodeEncodeError:
        # Non-UTF-8 content: encode with lossy replacement, fall back to latin-1
        raw_bytes = content.encode("utf-8", errors="replace")
        if not raw_bytes:
            raw_bytes = content.encode("latin-1")
    raw = zlib.compress(raw_bytes, 1)
    h_bare = "{:08x}".format(zlib.crc32(raw_bytes) & 0xFFFFFFFF)
    _inline_store_put(h_bare, content)
    return "i:" + h_bare, len(raw)


def _inline_retrieve(hash_val):
    """Retrieve content from inline store. Returns content or None."""
    h_bare = hash_val[2:] if hash_val.startswith("i:") else hash_val
    if h_bare not in _inline_store:
        return None
    _inline_store.move_to_end(h_bare)  # LRU promotion
    return _inline_store[h_bare]
