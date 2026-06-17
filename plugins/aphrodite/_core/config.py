"""aphrodite — thresholds, TOML loader, config resolvers, model family mapping."""

import logging
import os
from collections import deque

from . import settings as _settings
from .state import _state

# ── Pre-baked constants ───────────────────────────────────────
PORTS = {"cache": 9797, "token": 9798}
REPO = "PlayForm/Aphrodite"
BIN_VERSION = "v0.8.7"  # binary download version (must match Cargo.toml)
PLUGIN_VERSION = "1.62.22"  # plugin version
BINARY_DIR = os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite")
BINARY = os.path.join(BINARY_DIR, "aphrodite")
ENV_FILE = os.path.join(os.path.expanduser("~"), ".hermes", ".env")
_log = logging.getLogger("aphrodite")

# ── TOML config loader ──────────────────────────────────────────
# Priority: env var > aphrodite.toml > hardcoded default
# aphrodite.toml is searched in: cwd, ~/.hermes/aphrodite/, REPO_ROOT

_CONFIG: dict | None = None


def _load_toml_config() -> dict:
    """Load aphrodite.toml from disk; returns {} on any failure.

    Cached after first load. Call _reload_config() to clear the cache
    and re-evaluate all module-level config constants.
    """
    global _CONFIG
    if _CONFIG is not None:
        return _CONFIG
    try:
        import tomllib as _toml
    except ImportError:
        try:
            import tomli as _toml  # type: ignore[import-not-found]
        except ImportError:
            _log.debug("toml: no tomllib/tomli — TOML config skipped")
            _CONFIG = {}
            return _CONFIG

    search_paths = [
        "aphrodite.toml",
        os.path.join(os.path.expanduser("~"), ".hermes", "aphrodite", "aphrodite.toml"),
    ]
    # Also try relative to this file's parent (repo root)
    _this_dir = os.path.dirname(os.path.abspath(__file__))
    _repo_root = os.path.dirname(os.path.dirname(_this_dir))
    search_paths.append(os.path.join(_repo_root, "aphrodite.toml"))

    for path in search_paths:
        try:
            with open(path, "rb") as f:
                _CONFIG = _toml.load(f)
            _log.debug("toml: loaded %s (%d keys)", path, len(_CONFIG))
            return _CONFIG
        except FileNotFoundError:
            continue
        except Exception as e:
            _log.debug("toml: parse error %s: %s", path, e)
            continue

    _CONFIG = {}
    return _CONFIG


def _toml_section(section: str) -> dict:
    """Return a TOML section dict, or {} if missing."""
    return _load_toml_config().get(section, {})


# ── Config value resolution: env var → TOML → hardcoded default ─


def _cfg_int(name: str, default: int, toml_key: tuple[str, str] | None = None) -> int:
    """Resolve int config: runtime-settings → env var → toml[section][key] → default."""
    # Check dynamic settings store first (API-driven overrides)
    if toml_key:
        setting_val = _settings.get(toml_key[1])
        if setting_val is not None:
            try:
                return int(setting_val)
            except (ValueError, TypeError):
                pass
    env_val = os.environ.get(name)
    if env_val is not None:
        try:
            return int(env_val)
        except ValueError:
            pass
    if toml_key:
        section, key = toml_key
        toml_val = _toml_section(section).get(key)
        if toml_val is not None:
            try:
                return int(toml_val)
            except (ValueError, TypeError):
                pass
    return default


def _cfg_bool(name: str, default: bool, toml_key: tuple[str, str] | None = None) -> bool:
    """Resolve bool config: runtime-settings → env var → toml[section][key] → default."""
    if toml_key:
        setting_val = _settings.get(toml_key[1])
        if setting_val is not None:
            if isinstance(setting_val, bool):
                return setting_val
            return str(setting_val).lower() in ("1", "true", "yes", "on")
    env_val = os.environ.get(name)
    if env_val is not None:
        return env_val.lower() in ("1", "true", "yes", "on")
    if toml_key:
        section, key = toml_key
        toml_val = _toml_section(section).get(key)
        if toml_val is not None:
            if isinstance(toml_val, bool):
                return toml_val
            return str(toml_val).lower() in ("1", "true", "yes", "on")
    return default


def _cfg_str(name: str, default: str, toml_key: tuple[str, str] | None = None) -> str:
    """Resolve str config: runtime-settings → env var → toml[section][key] → default."""
    if toml_key:
        setting_val = _settings.get(toml_key[1])
        if setting_val is not None:
            return str(setting_val)
    env_val = os.environ.get(name)
    if env_val is not None:
        return env_val
    if toml_key:
        section, key = toml_key
        toml_val = _toml_section(section).get(key)
        if toml_val is not None:
            return str(toml_val)
    return default


def _cfg_float(name: str, default: float, toml_key: tuple[str, str] | None = None) -> float:
    """Resolve float config: runtime-settings → env var → toml[section][key] → default."""
    if toml_key:
        setting_val = _settings.get(toml_key[1])
        if setting_val is not None:
            try:
                return float(setting_val)
            except (ValueError, TypeError):
                pass
    env_val = os.environ.get(name)
    if env_val is not None:
        try:
            return float(env_val)
        except ValueError:
            pass
    if toml_key:
        section, key = toml_key
        toml_val = _toml_section(section).get(key)
        if toml_val is not None:
            try:
                return float(toml_val)
            except (ValueError, TypeError):
                pass
    return default


# ── Compression knobs ───────────────────────────────────────────
# All config values are set by _init_config() at import time and
# can be hot-reloaded via reload_config() (called by on_start).

ENGINE_THRESHOLD_PCT: int = 45
ENGINE_PROTECT_FIRST: int = 2
ENGINE_PROTECT_LAST: int = 5
ENGINE_MIN_MSGS: int = 8
TOOL_THRESHOLD_TOKEN: int = 512
TOOL_THRESHOLD_CACHE: int = 4096
TERMINAL_THRESHOLD: int = 1024
INLINE_THRESHOLD: int = 2048
RECURSIVE_DEPTH: int = 3
AUTO_EXPAND_LIMIT: int = 0
CATALOG_MODE: str = "compact"
CLASSIFIER_POLL: bool = True
CODE_MULTIPLIER: float = 3.0
CONTEXT_ENGINE: bool = True
MAX_REQUEST_BODY_SIZE: int = 104_857_600
MODEL_FAMILY: str = "code_first"
CODE_STRUCTURE_MAP: bool = True
PREVIEW_MAX_CHARS: int = 120
RETRIEVE_GUIDANCE: str = "minimal"
CCR_MARKER_HINT: bool = False
CATALOG_INTENT_HINTS: bool = False
DEBUG_LOGGING: bool = False
_recent_markers: deque = deque(maxlen=500)


def _init_config() -> None:
    """Evaluate all config values from env → TOML → defaults.

    Called at import time (module level) and on hot-reload.
    """
    global ENGINE_THRESHOLD_PCT, ENGINE_PROTECT_FIRST, ENGINE_PROTECT_LAST
    global ENGINE_MIN_MSGS, TOOL_THRESHOLD_TOKEN, TOOL_THRESHOLD_CACHE
    global TERMINAL_THRESHOLD, INLINE_THRESHOLD, AUTO_EXPAND_LIMIT
    global CATALOG_MODE, CLASSIFIER_POLL, CODE_MULTIPLIER, CONTEXT_ENGINE
    global MAX_REQUEST_BODY_SIZE, MODEL_FAMILY, CODE_STRUCTURE_MAP
    global PREVIEW_MAX_CHARS, RETRIEVE_GUIDANCE, CCR_MARKER_HINT
    global CATALOG_INTENT_HINTS, RECURSIVE_DEPTH, DEBUG_LOGGING, _recent_markers

    _log.debug("config: (re)loading from %s", "env/TOML/defaults")

    # Populate in-memory settings store from TOML (runtime overrides preserved)
    _settings.reload_from_toml(_load_toml_config())

    # Engine
    ENGINE_THRESHOLD_PCT = _cfg_int("APHRODITE_ENGINE_THRESHOLD_PCT", 45, ("compression", "engine_threshold_pct"))
    ENGINE_PROTECT_FIRST = _cfg_int("APHRODITE_ENGINE_PROTECT_FIRST", 2, ("compression", "engine_protect_first"))
    ENGINE_PROTECT_LAST = _cfg_int("APHRODITE_ENGINE_PROTECT_LAST", 5, ("compression", "engine_protect_last"))
    ENGINE_MIN_MSGS = _cfg_int("APHRODITE_ENGINE_MIN_MSGS", 8, ("compression", "engine_min_msgs"))

    # Thresholds
    TOOL_THRESHOLD_TOKEN = _cfg_int("APHRODITE_TOOL_THRESHOLD_TOKEN", 512, ("compression", "tool_threshold_token"))
    TOOL_THRESHOLD_CACHE = _cfg_int("APHRODITE_TOOL_THRESHOLD_CACHE", 4096, ("compression", "tool_threshold_cache"))
    TERMINAL_THRESHOLD = _cfg_int("APHRODITE_TERMINAL_THRESHOLD", 1024, ("compression", "terminal_threshold"))
    INLINE_THRESHOLD = _cfg_int("APHRODITE_INLINE_THRESHOLD", 2048, ("compression", "inline_threshold"))
    RECURSIVE_DEPTH = _cfg_int("APHRODITE_RECURSIVE_DEPTH", 3)

    # HEADROOM_SSE_BUFFER_MAX_BYTES check
    if os.environ.get("HEADROOM_SSE_BUFFER_MAX_BYTES"):
        INLINE_THRESHOLD = max(INLINE_THRESHOLD, 1_048_576)

    # Auto-expand (off by default)
    AUTO_EXPAND_LIMIT = _cfg_int("APHRODITE_AUTO_EXPAND_LIMIT", 0, ("compression", "auto_expand_limit"))
    if os.environ.get("APHRODITE_AUTO_EXPAND") == "1":
        AUTO_EXPAND_LIMIT = _cfg_int("APHRODITE_AUTO_EXPAND_LIMIT", 51200)

    CATALOG_MODE = _cfg_str("APHRODITE_CATALOG", "compact", ("compression", "catalog_mode"))
    CLASSIFIER_POLL = _cfg_bool("APHRODITE_CLASSIFIER_POLL", True, ("compression", "classifier_poll"))
    CODE_MULTIPLIER = _cfg_float("APHRODITE_CODE_MULTIPLIER", 3.0, ("compression", "code_multiplier"))
    CONTEXT_ENGINE = _cfg_bool("APHRODITE_CONTEXT_ENGINE", True, ("compression", "context_engine"))

    # Big-payload guard
    MAX_REQUEST_BODY_SIZE = _cfg_int("APHRODITE_MAX_REQUEST_BODY_SIZE", 104_857_600)

    # Previews
    MODEL_FAMILY = _cfg_str("APHRODITE_MODEL_FAMILY", "code_first", ("previews", "model_family"))
    CODE_STRUCTURE_MAP = _cfg_bool("APHRODITE_CODE_STRUCTURE_MAP", True, ("previews", "code_structure_map"))
    PREVIEW_MAX_CHARS = _cfg_int("APHRODITE_PREVIEW_MAX_CHARS", 120, ("previews", "preview_max_chars"))

    # Prompts
    RETRIEVE_GUIDANCE = _cfg_str("APHRODITE_RETRIEVE_GUIDANCE", "minimal", ("prompts", "retrieve_guidance"))
    CCR_MARKER_HINT = _cfg_bool("APHRODITE_CCR_MARKER_HINT", False, ("prompts", "ccr_marker_hint"))
    CATALOG_INTENT_HINTS = _cfg_bool("APHRODITE_CATALOG_INTENT_HINTS", False, ("prompts", "catalog_intent_hints"))

    # Recent markers deque
    _recent_markers = deque(maxlen=_cfg_int("APHRODITE_RECENT_MARKERS_MAX", 500))

    # Debug logging (env-only, no TOML key)
    DEBUG_LOGGING = os.environ.get("APHRODITE_DEBUG", "") == "1"

    if DEBUG_LOGGING:
        _log.setLevel(logging.DEBUG)
        _log.debug(
            "aphrodite v%s debug logging enabled | engine_threshold=%s protect_first=%s protect_last=%s "
            "min_msgs=%s tool_token=%s tool_cache=%s term=%s inline=%s",
            PLUGIN_VERSION,
            ENGINE_THRESHOLD_PCT,
            ENGINE_PROTECT_FIRST,
            ENGINE_PROTECT_LAST,
            ENGINE_MIN_MSGS,
            TOOL_THRESHOLD_TOKEN,
            TOOL_THRESHOLD_CACHE,
            TERMINAL_THRESHOLD,
            INLINE_THRESHOLD,
        )


def reload_config() -> None:
    """Hot-reload TOML config — clear cache and re-evaluate all constants.

    Called by on_start() at session start so aphrodite.toml edits take
    effect without a full Hermes restart.
    """
    global _CONFIG
    _CONFIG = None
    _init_config()
    _log.info(
        "config hot-reloaded: auto_expand_limit=%d engine_threshold=%d "
        "catalog=%s context_engine=%s",
        AUTO_EXPAND_LIMIT, ENGINE_THRESHOLD_PCT, CATALOG_MODE, CONTEXT_ENGINE,
    )


# Evaluate at import time
_init_config()

_DEV = os.environ.get("APHRODITE_PASSTHROUGH", "") == "1" or os.environ.get("HERMES_DEV", "") == "1"

if _DEV:
    _log.warning("aphrodite PASSTHROUGH MODE - plugin disabled, use cargo watch for proxies")

# ── Model-aware template dispatch ──────────────────────────────────────────
# Different LLM families process structured vs code-excerpt previews
# differently. The model family selects a preview strategy:
#   compact    — [type:key=val]  (Claude, default)
#   code_first — code excerpts before metadata  (DeepSeek, coding models)
#   balance    — metadata + short excerpt  (GPT, general-purpose)

MODEL_FAMILY_MAP: dict[str, str] = {
    "claude": "compact",
    "deepseek": "code_first",
    "gpt": "balance",
    "gemini": "balance",
    "llama": "compact",
    "mistral": "compact",
    "mixtral": "compact",
    "qwen": "code_first",
}


def _detect_model_family(model_name: str | None = None) -> str:
    """Detect model family from model name, env var, or state.

    Priority: explicit arg → APHRODITE_MODEL env → _state["model"] → "compact"
    """
    name = model_name or os.environ.get("APHRODITE_MODEL", "") or str(_state.get("model", ""))
    if not name:
        return MODEL_FAMILY  # TOML config default (code_first/compact/balance)
    name_lower: str = name.lower().replace("-", "").replace("_", "")
    for prefix, family in MODEL_FAMILY_MAP.items():
        if name_lower.startswith(prefix):
            return family
    return MODEL_FAMILY  # TOML config default


def _set_session_model(model_name: str) -> None:
    """Record the active model name in session state for preview dispatch."""
    _state["model"] = model_name
