"""aphrodite - inline compression (hash-based session store when proxy is down)."""

import hashlib

from ._core import _inline_store, _inline_store_put


def _inline_compress(content):
    """Compress content locally using zlib, store in session dict. Returns (hash, compressed_size)."""
    raw_bytes = content.encode("utf-8")
    h_bare = hashlib.sha256(raw_bytes).hexdigest()[:24]
    _inline_store_put(h_bare, content)
    return "i:" + h_bare, len(content)


def _inline_retrieve(hash_val):
    """Retrieve content from inline store. Returns content or None."""
    h_bare = hash_val[2:] if hash_val.startswith("i:") else hash_val
    if h_bare not in _inline_store:
        return None
    _inline_store.move_to_end(h_bare)  # LRU promotion
    return _inline_store[h_bare]
