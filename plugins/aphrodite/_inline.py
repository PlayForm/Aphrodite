"""aphrodite - inline compression (zlib fallback when proxy is down)."""

import base64
import hashlib
import zlib

from ._core import _inline_store


def _inline_compress(content):
    """Compress content locally using zlib, store in session dict. Returns (hash, compressed_size)."""
    compressed = base64.urlsafe_b64encode(zlib.compress(content.encode("utf-8"), 9)).decode("ascii")
    h = "i:" + hashlib.sha256(content.encode("utf-8")).hexdigest()[:14]
    _inline_store[h] = content
    # Keep store bounded
    if len(_inline_store) > 500:
        oldest = next(iter(_inline_store))
        del _inline_store[oldest]
    return h, len(compressed)


def _inline_retrieve(hash_val):
    """Retrieve content from inline store. Returns content or None."""
    return _inline_store.get(hash_val[2:] if hash_val.startswith("i:") else hash_val)
