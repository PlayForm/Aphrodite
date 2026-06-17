"""aphrodite - recent markers persistence across sessions."""

import atexit
import json
import logging
import os

from .._core import BINARY_DIR

_log = logging.getLogger("aphrodite")

_MARKERS_PATH = os.path.join(BINARY_DIR, "recent-markers.json")


def _save_markers() -> None:
    """Persist _recent_markers to disk for session resume. Called on shutdown."""
    try:
        from .._core import _recent_markers
    except ImportError:
        from .._hooks import _recent_markers  # type: ignore[assignment]
    try:
        data = list(_recent_markers)
        with open(_MARKERS_PATH, "w") as f:
            json.dump(data[-100:], f)
        _log.debug("saved %d markers to %s", min(len(data), 100), _MARKERS_PATH)
    except Exception as e:
        _log.debug("failed to save markers: %s", e)


def _restore_markers() -> None:
    """Load recent markers from disk into session state. Called by on_start()."""
    try:
        from .._core import _recent_markers
    except ImportError:
        from .._hooks import _recent_markers  # type: ignore[assignment]
    try:
        if os.path.exists(_MARKERS_PATH):
            with open(_MARKERS_PATH) as f:
                data = json.load(f)
            _recent_markers.clear()
            for entry in data:
                if isinstance(entry, dict) and "hash" in entry:
                    _recent_markers.append(entry)
            _log.info("restored %d markers from previous session", len(_recent_markers))
    except Exception as e:
        _log.debug("no markers to restore: %s", e)


# Register save on process exit
atexit.register(_save_markers)
