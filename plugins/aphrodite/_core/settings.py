"""aphrodite — in-memory settings store (API-driven, survives reloads).

All compression/preview/prompt knobs live here. Any part of the plugin reads
from this store; the proxy and API can update entries at runtime via a shared
JSON file at ~/.hermes/aphrodite/runtime-settings.json.

Priority: runtime-setting > env var > aphrodite.toml > hardcoded default.
"""

import json
import logging
import os
import threading

_log = logging.getLogger("aphrodite.settings")

# ── Shared file path ────────────────────────────────────────────
_RUNTIME_FILE = os.path.join(
    os.path.expanduser("~"), ".hermes", "aphrodite", "runtime-settings.json"
)

# ── In-memory store ─────────────────────────────────────────────
_store: dict = {}
_lock = threading.Lock()


def _load_runtime_file() -> dict:
    """Read runtime-settings.json; return {} on any failure."""
    try:
        with open(_RUNTIME_FILE, "r") as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError, OSError):
        return {}


def _save_runtime_file(data: dict) -> None:
    """Persist the full settings dict to the runtime file."""
    try:
        os.makedirs(os.path.dirname(_RUNTIME_FILE), exist_ok=True)
        with open(_RUNTIME_FILE, "w") as f:
            json.dump(data, f, indent=2)
    except OSError as exc:
        _log.warning("failed to write runtime-settings.json: %s", exc)


# ── Public API ──────────────────────────────────────────────────

def get(key: str, default=None):
    """Get a single setting from the in-memory store."""
    with _lock:
        return _store.get(key, default)


def set(key: str, value) -> None:
    """Set a single setting in memory and persist to file."""
    with _lock:
        _store[key] = value
    _save_runtime_file(dict(_store))
    _log.info("setting %s = %s", key, value)


def set_many(updates: dict) -> None:
    """Batch-update multiple settings."""
    with _lock:
        _store.update(updates)
    _save_runtime_file(dict(_store))
    _log.info("settings updated: %s", list(updates.keys()))


def all_settings() -> dict:
    """Return a shallow copy of all current settings."""
    with _lock:
        return dict(_store)


def reload_from_toml(config: dict) -> None:
    """Populate the store from a parsed aphrodite.toml dict.

    Does NOT override entries that were explicitly set at runtime
    (runtime-file entries take priority over TOML).
    Called at session start by config._init_config().
    """
    runtime = _load_runtime_file()
    comp = config.get("compression", {})
    previews = config.get("previews", {})
    prompts = config.get("prompts", {})

    toml_defaults: dict = {}
    # Compression
    for key in (
        "engine_threshold_pct", "engine_protect_first", "engine_protect_last",
        "engine_min_msgs", "tool_threshold_token", "tool_threshold_cache",
        "terminal_threshold", "inline_threshold", "auto_expand_limit",
        "catalog_mode", "classifier_poll", "code_multiplier", "context_engine",
        "auto_expand",
    ):
        if key in comp:
            toml_defaults[key] = comp[key]
    # Previews
    for key in ("model_family", "code_structure_map", "preview_max_chars"):
        if key in previews:
            toml_defaults[key] = previews[key]
    # Prompts
    for key in ("retrieve_guidance", "ccr_marker_hint", "catalog_intent_hints"):
        if key in prompts:
            toml_defaults[key] = prompts[key]

    with _lock:
        # Start with TOML defaults, then override with runtime values
        _store.clear()
        _store.update(toml_defaults)
        _store.update(runtime)  # runtime wins over TOML

    _log.debug(
        "settings: loaded %d TOML defaults + %d runtime overrides",
        len(toml_defaults), len(runtime),
    )


def reload_from_file() -> None:
    """Re-read runtime-settings.json into the in-memory store.

    Used by the background poller thread. Does NOT touch TOML.
    """
    runtime = _load_runtime_file()
    if not runtime:
        return
    with _lock:
        _store.update(runtime)
    _log.debug("settings: refreshed %d entries from runtime file", len(runtime))
