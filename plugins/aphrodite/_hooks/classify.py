"""aphrodite — classifier poll for content retrieval decisions."""

import logging

from .._core import CLASSIFIER_POLL

_log = logging.getLogger("aphrodite.hooks.classify")


def _classifier_says_skip(klass: dict) -> bool:
    """Classifier poll: does the content have nothing worth retrieving?

    If the classifier signals clean/inert output (0E/0W build, exit=0 terminal,
    0 match search, etc.), we skip CCR marker emission. The preview IS the
    complete story — creating a ``<<<CCR:hash>>>``` marker just baits the LLM
    into a wasteful retrieval round-trip.

    The content IS still stored in CCR for search/history. We just don't
    show the marker to the LLM.
    """
    if not CLASSIFIER_POLL:
        return False
    ctype = klass.get("type", "")
    if ctype in ("build_output", "build_error") and klass.get("errors", "0") in ("0", "") and klass.get("warnings", "0") in ("0", ""):
            return True
    if ctype == "terminal" and klass.get("exit") == "0":
        return True
    return bool(ctype in ("search_files", "search_results") and klass.get("total", "0") in ("0", ""))
