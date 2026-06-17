"""aphrodite - proxy lifecycle package (env, health, lifecycle, markers, startup)."""

from .lifecycle import _PROCS, _kill, _start, _wait_alive, on_start
from .health import (
    _alive,
    _alive_cache,
    _alive_cached,
    _alive_turn_cache,
    _proxy_version,
    _query_and_set_headroom_budget,
    _query_proxy_version,
)
from .env import (
    _PROXY_ENV_KEYS,
    _expand_guidance,
    _headroom_context,
    _inject_expand_guidance,
    _load_env,
    _update_headroom_context,
)
from .markers import _MARKERS_PATH, _restore_markers, _save_markers
from .startup import _write_startup_log

__all__ = [
    "_alive",
    "_alive_cache",
    "_alive_cached",
    "_alive_turn_cache",
    "_expand_guidance",
    "_headroom_context",
    "_inject_expand_guidance",
    "_kill",
    "_load_env",
    "_MARKERS_PATH",
    "_PROCS",
    "_PROXY_ENV_KEYS",
    "_proxy_version",
    "_query_and_set_headroom_budget",
    "_query_proxy_version",
    "_restore_markers",
    "_save_markers",
    "_start",
    "_update_headroom_context",
    "_wait_alive",
    "_write_startup_log",
    "on_start",
]
