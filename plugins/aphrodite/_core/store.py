"""aphrodite — inline compression store with trigram index, LRU eviction, size formatting."""

from collections import OrderedDict


# ── Inline compression store + trigram index (session-scoped, capped at 500) ──
class _CappedStore(OrderedDict):
    """OrderedDict that auto-evicts oldest entries when exceeding MAX_STORE."""

    MAX_STORE = 500

    def __setitem__(self, key, value):
        super().__setitem__(key, value)
        if len(self) > self.MAX_STORE:
            self.popitem(last=False)

    def popitem(self, last=True):
        key, value = super().popitem(last=last)
        global _inline_bytes
        _inline_bytes -= len(value) if value else 0
        if _inline_index_enabled:
            _remove_trigram_index(key)
        return key, value


_inline_store: _CappedStore = _CappedStore()
_inline_index: dict = {}  # {trigram: set_of_hashes} for O(1) search
_inline_bytes: int = 0  # tracked byte count (avoids sum(len(v) for v ...))
_inline_index_enabled: bool = False  # lazily enabled on first index build
_hash_to_trigrams: dict = {}  # {hash: set_of_trigrams} reverse index for O(1) eviction


# ── Shared utilities ──────────────────────────────────────────

def _fmt_size(b):
    if b >= 1_000_000:
        return f"{b / 1_000_000:.1f}MB"
    if b >= 1000:
        return f"{b / 1000:.1f}KB"
    return f"{b}B"


def _inline_clear():
    """Clear the inline store (called on session reset)."""
    global _inline_bytes, _inline_index_enabled
    _inline_store.clear()
    _inline_index.clear()
    _hash_to_trigrams.clear()
    _inline_bytes = 0
    _inline_index_enabled = False


def _init_trigram_index():
    """Build trigram index from all inline store entries (one-time)."""
    global _inline_index_enabled
    _inline_index.clear()
    for h, content in _inline_store.items():
        _index_trigrams(h, content)
    _inline_index_enabled = True


def _index_trigrams(h, content):
    """Split content into trigrams and index under hash. Populates both
    forward (_inline_index) and reverse (_hash_to_trigrams) indices."""
    lower = content.lower()
    trigrams = {lower[i : i + 3] for i in range(len(lower) - 2)}
    _hash_to_trigrams[h] = trigrams
    for tri in trigrams:
        _inline_index.setdefault(tri, set()).add(h)


def _remove_trigram_index(h):
    """Remove all index entries for a given hash (O(1) via reverse index)."""
    trigrams = _hash_to_trigrams.pop(h, ())
    for tri in trigrams:
        s = _inline_index.get(tri)
        if s:
            s.discard(h)
            if not s:
                del _inline_index[tri]


def _inline_store_put(h, content):
    """Store content in inline store with LRU eviction at MAX=500.

    __setitem__ handles ordering and eviction automatically. On update,
    old trigrams are unindexed before re-indexing the new content.
    Returns True if the entry was newly added, False if updated.
    """
    global _inline_bytes
    is_new = h not in _inline_store
    if not is_new:
        old_len = len(_inline_store[h])
        _inline_bytes -= old_len
        if _inline_index_enabled:
            _remove_trigram_index(h)
    _inline_store[h] = content
    _inline_bytes += len(content)
    # Index trigrams for search
    if _inline_index_enabled:
        _index_trigrams(h, content)
    return is_new
