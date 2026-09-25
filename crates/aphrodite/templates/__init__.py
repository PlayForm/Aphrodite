"""aphrodite - CCR compression plugin for Hermes Agent (Rust-powered).

All logic in libaphrodite_hermes.dylib. This file is a thin registration shim.
Architecture: __init__.py → ctypes → libaphrodite_hermes.dylib → aphrodite crate (rlib)
"""

import contextlib
import ctypes
import json
import logging
import os
import subprocess
import sys
import tempfile
import threading
import types
from collections.abc import Callable
from pathlib import Path
from typing import Any

_log = logging.getLogger("aphrodite")

# ── Dylib path resolution ──
_PLUGIN_DIR = Path(__file__).resolve().parent
_DYLIB_NAME = (
    "libaphrodite_hermes.dylib"
    if sys.platform == "darwin"
    else "libaphrodite_hermes.so"
    if sys.platform == "linux"
    else "aphrodite_hermes.dll"
)
# Canonical runtime home: every runtime artifact (binaries, dylib,
# aphrodite.toml, ccr.db) lives under <hermes-home>/aphrodite, never inside
# the plugin tree. The Hermes home is $HERMES_HOME when set (the catalog
# validate probe runs register() against a scratch HERMES_HOME - honoring it
# keeps every runtime write inside the scratch home, never the real
# ~/.hermes), else ~/.hermes. Env overrides stay first; the plugin-dir
# binaries/ paths survive only as shipped-binary fallbacks.
def _hermes_home() -> Path:
    """Hermes home: ``$HERMES_HOME`` when set (expanded), else ``~/.hermes``.

    Mirrors Hermes' own resolution (hermes_constants.get_hermes_home):
    context override -> env var -> platform default. The plugin must never
    hardcode ``Path.home()/".hermes"`` - `hermes plugins validate` runs the
    capability probe in a scratch child with a throwaway HERMES_HOME
    (hermes_cli/plugin_validate.py), and writing runtime artifacts into the
    real home from there is exactly the pollution the catalog review
    flagged (teknium review, PR 118488).
    """
    override = os.environ.get("HERMES_HOME", "").strip()
    if override:
        return Path(override).expanduser()
    return Path.home() / ".hermes"


def _home_holds_install(home: Path) -> bool:
    """True when ``home`` already carries a live Aphrodite install: a
    ``binaries/`` directory, an ``aphrodite.toml``, or a ``ccr.db``.

    Feeds the legacy-adoption rule (F2) so an upgrade never abandons the
    home a previous version actually used - never migrate, adopt-and-warn.
    """
    try:
        return (
            (home / "binaries").is_dir()
            or (home / "aphrodite.toml").is_file()
            or (home / "ccr.db").is_file()
        )
    except OSError:
        return False


def _home_is_scratch(home: Path) -> bool:
    """True when ``home`` lives under the OS temp dir - a throwaway/scratch
    home (the catalog-validate probe runs ``register()`` in a scratch child
    whose ``HERMES_HOME`` is a ``tempfile.TemporaryDirectory``,
    hermes_cli/plugin_validate.py). Legacy adoption must NEVER redirect a
    throwaway home onto a real install: under the probe that would make the
    layout heal and directives materialize write into the live
    ``~/.hermes/aphrodite`` - the exact pollution PR 118488 removed.
    """
    try:
        tmp = Path(tempfile.gettempdir())
        resolved = home.resolve()
        return resolved.is_relative_to(tmp.resolve()) or home.is_relative_to(tmp)
    except (OSError, ValueError):
        return False


def _runtime_home() -> tuple[Path, str]:
    """THE single runtime-home decision for the whole plugin process.

    Precedence (first match wins):
      1. ``$APHRODITE_HOME`` - explicit override; never second-guessed.
      2. ``<hermes-home>/aphrodite`` - the canonical home, matching the
         shim's Hermes-home resolution and Hermes' per-Hermes-home plugin
         managers (profiles).
      3. Legacy ``$HOME/.hermes/aphrodite`` - ADOPTED when it is the one
         that actually holds an install and the canonical home does not
         (issue 40 F2: never pick a home other than the one a previous
         version used while that one still exists - adopt-and-warn instead
         of migrating).

    Returns ``(home, decision)`` where ``decision`` names the source for the
    startup log line (F5). The decision is exported to the Rust half via
    ``os.environ.setdefault("APHRODITE_HOME", ...)`` at import, so the dylib
    and the spawned proxy binary (a child process inheriting this env)
    resolve the same directory - the two halves cannot diverge structurally
    (F1), and per-Hermes-home isolation (profiles) extends to the Rust half.
    """
    override = os.environ.get("APHRODITE_HOME", "").strip()
    if override:
        return Path(override).expanduser(), "APHRODITE_HOME override"
    canonical = _hermes_home() / "aphrodite"
    legacy = Path.home() / ".hermes" / "aphrodite"
    if (
        not _home_holds_install(canonical)
        and _home_holds_install(legacy)
        and not _home_is_scratch(_hermes_home())
    ):
        _log.warning(
            "adopting the pre-2.2 runtime home %s (no install under %s); "
            "set APHRODITE_HOME to pin the location explicitly",
            legacy,
            canonical,
        )
        return legacy, "legacy ~/.hermes/aphrodite adoption"
    if os.environ.get("HERMES_HOME", "").strip():
        return canonical, "HERMES_HOME"
    return canonical, "default"


# ── Runtime home: ONE decision, exported to the Rust half ──
# Every runtime artifact (binaries, dylib, aphrodite.toml, ccr.db,
# directives, logs) lives under the runtime home, never inside the plugin
# tree. The dylib and the proxy binary resolve APHRODITE_HOME /
# APHRODITE_DIRECTIVES_DIR when set; exporting the shim's decision here
# makes the two halves agree by construction (issue 40 F1). setdefault
# keeps a user-provided override authoritative.
_RUNTIME_HOME, _HOME_DECISION = _runtime_home()
os.environ.setdefault("APHRODITE_HOME", str(_RUNTIME_HOME))
os.environ.setdefault("APHRODITE_DIRECTIVES_DIR", str(_RUNTIME_HOME / "directives"))
_log.info("aphrodite runtime home: %s (decided by %s)", _RUNTIME_HOME, _HOME_DECISION)

_BINARIES_DIR = _RUNTIME_HOME / "binaries"
_DYLIB_PATH = os.environ.get("APHRODITE_HERMES_DYLIB_PATH", str(_BINARIES_DIR / _DYLIB_NAME))
_BINARY_NAME = "aphrodite.exe" if sys.platform == "win32" else "aphrodite"
_BINARY_PATH = os.environ.get("APHRODITE_BINARY_PATH", str(_BINARIES_DIR / _BINARY_NAME))

# ── Pointer-returning exports the shim calls through ctypes ──
# Every export listed here MUST be configured with restype=c_void_p in the
# FFI setup path (_load_dylib). ctypes' default restype is c_int: a 64-bit
# *mut c_char return is read truncated and sign-extended, and _read_str then
# strlen()s a bogus low address - the SIGSEGV this plugin shipped when
# materialize_directives was configured everywhere except the setup block.
# The declarations now come from the generated _bindings.py (cbindgen →
# ctypesgen, regenerated by crates/aphrodite-hermes/build.rs) when present,
# with the manual restype setup below as fallback; _load_dylib asserts the
# invariant on every load so an export added without a setup-block entry
# fails registration loudly (graceful disable) instead of crashing the
# gateway with a silent SIGSEGV (harden item 3).
_REQUIRED_VOID_P: tuple[str, ...] = (
    "aphrodite_hermes_dispatch_tool",
    "aphrodite_hermes_call_hook",
    "aphrodite_hermes_proxy_health",
    "aphrodite_hermes_version",
    "aphrodite_hermes_materialize_directives",
    "aphrodite_hermes_get_schemas",
    "aphrodite_hermes_get_hooks",
)

# ── Generated FFI bindings (cbindgen → ctypesgen → _bindings.py) ──
# build.rs regenerates plugins/aphrodite/_bindings.py whenever the C ABI
# surface changes (graceful skip when the codegen tools are missing - the
# committed artifact stays in effect, and its import is best-effort here).
# The generated module never loads a library itself: the plugin owns the
# CDLL handle and replays the declarations onto it
# via _GENERATED_BINDINGS.bind_to(dylib). Any failure - absent file (fresh
# checkout before the artifact lands, standalone `aphrodite setup` install
# whose embedded template ships without it), corrupt artifact, or a bind_to
# mismatch against the live dylib - degrades to the manual restype setup,
# never to an unconfigured ctypes default.
try:
    from . import _bindings as _GENERATED_BINDINGS  # noqa: N812  # type: ignore[attr-defined]
except Exception as _bindings_err:  # ImportError (absent), SyntaxError (corrupt), ...
    _GENERATED_BINDINGS = None  # type: ignore[assignment]
    _log.debug("generated _bindings.py unavailable (%s); using the manual FFI setup", _bindings_err)

# ── Per-process dylib state ──
# Hermes builds one PluginManager per Hermes home (root home + every
# profile) and exec_module()s this shim once per home under a distinct
# module name. Keeping the state in a synthetic module registered under a
# fixed name in sys.modules makes it survive re-exec: later copies of the
# shim find the holder and reuse the CDLL handle that is already mapped
# (dlopen memoizes by canonical path, so repeated loads of the same path
# converge on one image). A ModuleType (not a namespace/class instance) so
# it is keyed by a stable name rather than by the identity of whichever
# shim created it.
_STATE_MODULE_NAME = "aphrodite_hermes._process_state"


def _process_state() -> types.ModuleType:
    """Return the process-wide state holder, creating it on first use."""
    holder = types.ModuleType(
        _STATE_MODULE_NAME, "Process-global aphrodite dylib state shared by every loaded shim copy."
    )
    holder.dylib = None  # type: ignore[attr-defined]  # ctypes.CDLL | None
    # Guards dylib: ctypes releases the GIL during foreign calls, so two
    # Hermes threads can race through _load_dylib (worst case one thread
    # reads a half-swapped reference).
    holder.lock = threading.Lock()  # type: ignore[attr-defined]
    # Dylib source paths already subprocess smoke-tested in THIS process
    # (harden item 6): the probe runs once per unique path, never again.
    holder.probed_paths: set[str] = set()  # type: ignore[attr-defined]
    try:
        # dict.setdefault is atomic under the GIL, so two shim copies
        # importing concurrently still converge on a single holder.
        return sys.modules.setdefault(_STATE_MODULE_NAME, holder)
    except Exception as e:
        # Defensive: never crash registration on a poisoned sys.modules -
        # fall back to a private holder (each shim copy then loads its own
        # image; dlopen memoizes by path so handles still converge).
        _log.warning("_process_state: sys.modules unavailable (%s); using a private holder", e)
        return holder


_state = _process_state()


def _data_dir() -> Path:
    """Plugin data directory (proxy-stderr.log, ccr state): the process-wide
    runtime home resolved once at import (``_RUNTIME_HOME``) and exported to
    the Rust half through ``$APHRODITE_HOME`` - both halves share one
    decision (issue 40 F1), so this is never a second, shadow home.
    """
    return _RUNTIME_HOME


def _binary_version_path() -> Path:
    """BINARY_VERSION pin: the loader's own checkout pins the version it was
    shipped against (source mode); installed layouts carry the pin in the
    runtime home, written by `aphrodite setup`. The plugin dir itself is
    never a runtime data location - in an installed hooks-only dir this
    fallback simply misses and the runtime home serves."""
    own = _PLUGIN_DIR / "BINARY_VERSION"
    if own.exists():
        return own
    return _data_dir() / "BINARY_VERSION"


def _download_script() -> Path:
    """download.sh location: the source checkout ships it beside the loader
    (dev loops); installed layouts would keep it in the runtime home since
    everything the plugin downloads lives under ~/.hermes/aphrodite. Never
    a write - the plugin never modifies its own directory."""
    own = _PLUGIN_DIR / "download.sh"
    if own.exists():
        return own
    return _data_dir() / "download.sh"


def _dylib_candidates(plugin_dir: Path) -> list[str]:
    """Ordered dylib candidates, env override first, parent-depth guarded.

    Shipping model (catalog review, PR 118488): NO binaries ever ship in
    the plugin tree. The plugin downloads from the GitHub release (its tag
    and checksums are pinned in this tree) into the canonical runtime home
    <hermes-home>/aphrodite/binaries, and validates the download against
    the in-tree checksum list (generated by Build.yml). The canonical
    runtime home is therefore the primary location; plugin-dir binaries
    never exist by design. Shallow installs (e.g. /opt/Aphrodite-Hermes)
    have fewer than 4 parents; the monorepo target/release fallbacks simply
    do not exist there, so indexing is guarded instead of crashing
    (issue 5).
    """
    plugin_dir = Path(plugin_dir).resolve()
    canonical = str(_BINARIES_DIR / _DYLIB_NAME)
    candidates = [
        _DYLIB_PATH,  # APHRODITE_HERMES_DYLIB_PATH override (or canonical default) - wins when it exists
        canonical,  # canonical runtime home (where download.sh writes the verified binary)
    ]
    parents = plugin_dir.parents
    for depth in (2, 3):
        if depth < len(parents):
            candidates.append(str(parents[depth] / "target" / "release" / _DYLIB_NAME))
    return candidates


def _resolve_binary_path() -> str:
    """Proxy binary path: env override first, then the canonical runtime
    home, then legacy plugin-dir copies.

    APHRODITE_BINARY_PATH wins when it exists. Otherwise use
    <hermes-home>/aphrodite/binaries (where download.sh writes the
    checksum-verified binary). Returns the last candidate (missing) when
    nothing exists - callers log the final miss.
    """
    if os.path.exists(_BINARY_PATH):
        return _BINARY_PATH
    canonical = str(_BINARIES_DIR / _BINARY_NAME)
    if os.path.exists(canonical):
        return canonical
    return _BINARY_PATH


def _probe_dylib(path: str) -> bool:
    """Smoke-test a dylib in a SUBPROCESS before ctypes loads it in-process.

    A ctypes SIGSEGV (a faulting export, a stale image whose symbols/contract
    mismatch the shim) CANNOT be caught by try/except - it kills the whole
    Hermes gateway. Loading the dylib in a fresh ``sys.executable`` child and
    calling aphrodite_hermes_version there confines any crash to that child:
    a nonzero returncode means "refuse to load this in-process", and
    register() then degrades to a graceful 'plugin disabled' + ERROR log
    instead of a gateway SIGSEGV.

    Runs ONCE per unique dylib source path per process (see
    _state.probed_paths) - the probe is a one-time gate, never repeated on
    every load.
    """
    probe_script = (
        "import ctypes, sys\n"
        f"path = {path!r}\n"
        "try:\n"
        "    d = ctypes.CDLL(path)\n"
        "    d.aphrodite_hermes_version.restype = ctypes.c_void_p\n"
        "    ptr = d.aphrodite_hermes_version()\n"
        "    if not ptr:\n"
        "        sys.exit(3)\n"
        "except Exception as e:\n"
        "    sys.stderr.write(f'probe error: {e}\\n')\n"
        "    sys.exit(2)\n"
    )
    try:
        probe = subprocess.run(
            [sys.executable, "-c", probe_script],
            timeout=5,
            capture_output=True,
            text=True,
            errors="replace",
        )
    except subprocess.TimeoutExpired:
        _log.warning("dylib smoke-test timed out for %s", path)
        return False
    except Exception as e:
        _log.warning("dylib smoke-test could not run for %s (%s)", path, e)
        return False
    if probe.returncode != 0:
        stderr = (probe.stderr or "").strip()
        _log.error(
            "dylib smoke-test FAILED for %s (rc=%s%s) - refusing to load it "
            "in-process (a faulting dylib would SIGSEGV the gateway)",
            path,
            probe.returncode,
            f"; stderr: {stderr}" if stderr else "",
        )
        return False
    return True


def _configure_ffi(dylib: ctypes.CDLL, path: str) -> None:
    """Apply FFI restype/argtypes, preferring the generated _bindings.py.

    The generated module (cbindgen → ctypesgen → _bindings.py) is the
    authoritative declaration source; bind_to() replays its restype/argtypes
    onto THIS dylib handle (the generated module never hardcodes a library
    path). If the
    generated bindings are absent (fresh checkout / standalone `aphrodite
    setup` install) or cannot be applied (stale artifact, mismatched dylib),
    fall back to the manual restype setup. _ensure_ffi_argtypes then
    guarantees the arg-carrying entry points regardless of which source won.
    """
    if _GENERATED_BINDINGS is not None:
        try:
            _GENERATED_BINDINGS.bind_to(dylib)
            _log.debug("FFI declarations applied from generated _bindings.py")
            return
        except Exception as e:
            # A stale/mismatched artifact (e.g. built against a different
            # lib.rs) must not half-configure: fall back, then the
            # _REQUIRED_VOID_P assertion below still guards every load.
            _log.warning(
                "generated bindings could not be applied to %s (%s); "
                "falling back to the manual restype setup",
                path,
                e,
            )
    _manual_ffi_setup(dylib)


def _manual_ffi_setup(dylib: ctypes.CDLL) -> None:
    """Manual restype/argtypes fallback.

    Used when _bindings.py is absent (a fresh checkout before the artifact
    lands, or a standalone `aphrodite setup` install whose embedded template
    ships without it) or failed to apply. Mirrors the generated declarations
    exactly - c_void_p for every pointer-returning export.
    """
    # c_void_p avoids Python 3.14 c_char_p malloc mismatch → SIGABRT
    dylib.aphrodite_hermes_get_schemas.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_get_hooks.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_dispatch_tool.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    dylib.aphrodite_hermes_dispatch_tool.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    dylib.aphrodite_hermes_call_hook.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_proxy_health.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_version.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_materialize_directives.argtypes = [ctypes.c_char_p]
    dylib.aphrodite_hermes_materialize_directives.restype = ctypes.c_void_p
    dylib.aphrodite_hermes_free_string.argtypes = [ctypes.c_void_p]


def _ensure_ffi_argtypes(dylib: ctypes.CDLL) -> None:
    """Guarantee argtypes for the arg-carrying entry points the plugin calls.

    Runs after BOTH the generated and manual paths so a generated artifact
    that somehow omitted argtypes could never cause a ctypes TypeError deep
    inside a hook/tool call; the values match the header's parameters
    exactly and accept the bytes the plugin passes.
    """
    dylib.aphrodite_hermes_dispatch_tool.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    dylib.aphrodite_hermes_call_hook.argtypes = [ctypes.c_char_p, ctypes.c_char_p]


def _load_dylib() -> ctypes.CDLL:
    """Load libaphrodite_hermes.dylib with ctypes.

    State lives in the process-global holder (see `_process_state`), so a
    second exec of this shim in the same process (one per Hermes home)
    finds the already-mapped CDLL handle and reuses it (dlopen memoizes by
    canonical path, so repeated loads of the same path converge on one
    image). No hot-reload: the dylib is resolved once per process from the
    pinned binary set (catalog review, PR 118488 - the load must be
    deterministic from the installed tree, never racing a rebuild).
    """
    with _state.lock:
        if _state.dylib is not None:
            return _state.dylib

        # Find current dylib path: env override first, then the canonical
        # runtime home, then legacy plugin-dir copies.
        path = _DYLIB_PATH
        candidates = _dylib_candidates(_PLUGIN_DIR)
        for p in candidates:
            if os.path.exists(p):
                path = p
                break
        # Auto-fetch is OFF by default (catalog review, PR 118488): the
        # binaries ship in the pinned tree's binaries/ (gitignored, pulled
        # by the publish action); download.sh is an explicit setup step.
        # No-op when present.
        _ensure_binaries()
        if not os.path.exists(path):
            # download.sh may have just populated the canonical home -
            # re-resolve so the freshly fetched copy wins over legacy.
            for p in candidates:
                if os.path.exists(p):
                    path = p
                    break
        # Defensive: prefer the canonical runtime home. A legacy
        # plugin-dir/target hit means the canonical copy is missing - warn
        # but keep loading, never crash registration over a path preference.
        # Silence when the env override or the canonical path itself won.
        if path != _DYLIB_PATH and path != str(_BINARIES_DIR / _DYLIB_NAME):
            _log.warning(
                "canonical dylib %s not found - using %s instead (legacy "
                "install; run download.sh to migrate to %s)",
                _BINARIES_DIR / _DYLIB_NAME,
                path,
                _BINARIES_DIR,
            )
        assert os.path.exists(path), f"Dylib not found. Tried: {candidates}"

        # Smoke-test the dylib in a SUBPROCESS before loading it in-process:
        # a ctypes SIGSEGV cannot be caught by try/except and would kill the
        # whole gateway, so a faulting image is rejected here - once per
        # unique path - and register() degrades to a graceful disable.
        if path not in _state.probed_paths:
            if not _probe_dylib(path):
                raise RuntimeError(f"dylib smoke-test failed for {path} - plugin disabled")
            _state.probed_paths.add(path)

        dylib = ctypes.CDLL(path)

        try:
            # c_void_p avoids Python 3.14 c_char_p malloc mismatch → SIGABRT;
            # declarations come from the generated _bindings.py when present,
            # with the manual restype setup as fallback (see _configure_ffi).
            _configure_ffi(dylib, path)
            # Completeness assertion (harden item 3): every pointer-returning
            # export we call must read its return at full 64-bit width. An
            # export missing from the FFI setup keeps the ctypes default
            # restype (c_int), which truncates the pointer and SIGSEGVs in
            # _read_str; assert the invariant so the failure is a caught
            # RuntimeError (graceful disable), never a silent gateway crash.
            for _sym in _REQUIRED_VOID_P:
                if getattr(dylib, _sym).restype is not ctypes.c_void_p:
                    raise RuntimeError(
                        f"dylib at {path} configures {_sym} with restype "
                        f"{getattr(dylib, _sym).restype!r}, expected c_void_p "
                        f"- pointer-returning exports must be read at full "
                        f"width (a truncated pointer SIGSEGVs in _read_str)"
                    )
        except AttributeError as e:
            # A stale/mismatched dylib (see _check_version) surfacing as a
            # raw AttributeError deep in ctypes gives no context on which
            # binary or which symbol - name both here.
            raise RuntimeError(
                f"dylib at {path} is missing an expected symbol ({e}) - "
                f"it may be built from a different aphrodite-hermes version "
                f"than this plugin expects"
            ) from e

        _state.dylib = dylib  # pyright: ignore[reportAttributeAccessIssue]
        return dylib


def _read_str(ptr: int | None) -> str | None:
    """Read a null-terminated C string from a void pointer.

    NULL must be checked BEFORE the cast: ctypes.cast(ptr, c_char_p).value
    runs strlen() on the pointer (z_get), so a NULL/bogus pointer from Rust
    would segfault inside the C getter - and a SIGSEGV is not a Python
    exception, it kills the process (harden item 2). errors='replace' keeps
    a non-UTF-8 payload from raising on a best-effort string read.
    """
    if not ptr:
        return None
    value = ctypes.cast(ptr, ctypes.c_char_p).value
    return value.decode("utf-8", errors="replace") if value is not None else None


def _call_json(dylib: ctypes.CDLL, fn_name: str, *args: bytes) -> Any:
    """Call C function by name, decode JSON, free through the SAME dylib
    object that produced the pointer (F4: allocating and freeing through
    different dylib handles is only safe by accident today - benign
    while both use the system allocator, undefined behavior the day a
    custom global allocator is added).

    Every function routed through here returns a *mut c_char. Force
    restype=c_void_p so a 64-bit pointer return is read at full width:
    the default restype (c_int) truncates the pointer to 32 bits and
    sign-extends it, and _read_str then strlen()s a bogus low address -
    the SIGSEGV in z_get this plugin shipped (materialize_directives was
    configured everywhere except here). Setting it unconditionally is a
    no-op for already-configured fns and a correctness clamp for any
    future one added without a setup-block entry.

    F4 constraint: ptr is ALWAYS freed through THIS `dylib` handle - the
    same image that allocated it - never through a cached/reloaded handle
    (e.g. _state.dylib). Freeing through a different generation's
    free_string is undefined behavior the day a custom global allocator is
    added; the handle must travel with the pointer it produced.
    """
    fn = getattr(dylib, fn_name)
    fn.restype = ctypes.c_void_p
    ptr = fn(*args)
    result = _read_str(ptr)
    if ptr:
        dylib.aphrodite_hermes_free_string(ptr)
    return json.loads(result) if result else None


def _make_handler(tool_name: str) -> Callable[..., str]:
    """Create tool handler that dispatches via dylib.

    Resolves the dylib fresh on every call (not captured at registration
    time) so tool calls always use the current image - see the hook
    registration loop below for why hooks need the same treatment.
    """

    def handler(args: dict[str, Any] | None = None, **kwargs: Any) -> str:
        args_json = json.dumps(args or {})
        dylib = _load_dylib()
        return json.dumps(
            _call_json(
                dylib,
                "aphrodite_hermes_dispatch_tool",
                tool_name.encode("utf-8"),
                args_json.encode("utf-8"),
            )
        )

    return handler


def _check_version(dylib: ctypes.CDLL) -> None:
    """Warn (never raise) if the loaded dylib's version disagrees with
    BINARY_VERSION - the plugin's own pin of what it was built/shipped
    against (F6). The dylib is fetched separately (download.sh, keyed off
    BINARY_VERSION); a stale or newer dylib with a
    changed JSON contract should fail loudly at registration, not misbehave
    silently at runtime."""
    try:
        loaded = _call_json(dylib, "aphrodite_hermes_version")
        loaded_version = (loaded or {}).get("version") if isinstance(loaded, dict) else None
        expected_path = _binary_version_path()
        expected_version = expected_path.read_text().strip() if expected_path.exists() else None
        if loaded_version and expected_version and loaded_version != expected_version:
            _log.warning(
                "aphrodite-hermes dylib version mismatch: loaded %s, "
                "BINARY_VERSION expects %s - the JSON contract (hook/tool "
                "schemas) may have changed between these versions",
                loaded_version,
                expected_version,
            )
    except Exception as e:
        # Version check is best-effort diagnostics, never a hard requirement.
        _log.debug("version handshake skipped: %s", e)


def _env_bool(var: str) -> bool:
    """One consistent truthiness rule for boolean env vars (F12): `"1"`/
    `"true"` (case-insensitive) is true, anything else (including unset) is
    false. Mirrors `aphrodite::config::env_bool` on the Rust side - this
    plugin previously checked `== "1"` only, while the Rust proxy's
    APHRODITE_LOG_COMPACT accepted any value (even "0") via presence-only
    `.is_ok()`, and the dead `config_loader.rs` accepted `"true"` OR `"1"`."""
    return os.environ.get(var, "").lower() in ("1", "true")


def _parse_port_env(var: str, default: int) -> int:
    """Parse a port env var, falling back to `default` when unset OR
    malformed (F11) - this used to be a bare `int()` call, so e.g.
    APHRODITE_CACHE_PORT=abc raised uncaught and aborted plugin
    registration entirely. The proxy itself would still start (the Rust
    side warns-and-falls-back on the same var), so the plugin's own
    tools/hooks failing to register was a worse outcome than falling back
    to the same default the proxy already chose."""
    raw = os.environ.get(var)
    if raw is None:
        return default
    try:
        return int(raw)
    except ValueError:
        _log.warning("%s=%r is not a valid port; falling back to %d", var, raw, default)
        return default


def _tail_log(path: Path, n: int = 15) -> str:
    """Last `n` lines of a log file; "(no stderr log yet)" when unreadable."""
    with contextlib.suppress(OSError):
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        return "\n".join(lines[-n:])
    return "(no stderr log yet)"


def _check_version_published() -> None:
    """Warn (never raise) if BINARY_VERSION's GitHub release carries no assets.

    download.sh builds asset URLs from
    https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite%2Fv{V}
    (URL-encoded slash, REPO=PlayForm/Aphrodite). A tag with no published
    assets - or no tag at all - makes every asset 404 and the fetch
    crash-loops at every registration. Checked via the GitHub API
    releases/tags endpoint (3s timeout) BEFORE download.sh is invoked
    (from _ensure_binaries); any network/API failure degrades to a
    warning, never a raise (harden item 4).
    """
    try:
        import urllib.request
        from urllib.parse import quote

        version_file = _binary_version_path()
        if not version_file.exists():
            return
        version = version_file.read_text().strip()
        if not version:
            return
        url = (
            "https://api.github.com/repos/PlayForm/Aphrodite/releases/tags/"
            f"{quote(f'Aphrodite/v{version}', safe='')}"
        )
        req = urllib.request.Request(
            url,
            method="GET",
            headers={
                "User-Agent": "aphrodite-hermes-plugin",
                "Accept": "application/vnd.github+json",
            },
        )
        with urllib.request.urlopen(req, timeout=3) as resp:
            payload = json.loads(resp.read(65536).decode("utf-8", errors="replace"))
        assets = payload.get("assets") if isinstance(payload, dict) else None
        if isinstance(assets, list) and len(assets) == 0:
            _log.warning(
                "BINARY_VERSION %s points at GitHub release Aphrodite/v%s "
                "which has NO assets - download.sh will 404 on every asset "
                "URL (auto-fetch crash-loop). Publish the release artifacts "
                "or bump BINARY_VERSION to a published tag",
                version,
                version,
            )
    except Exception as e:
        # Network/API hiccups must never block registration - the check is
        # best-effort diagnostics ahead of a download that may still succeed.
        _log.warning("version-published check skipped (%s)", e)


def _ensure_binaries() -> None:
    """Ensure the proxy binary + dylib are present - NEVER downloads.

    Catalog review (PR 118488): a download is an explicit, documented setup
    step (`aphrodite setup` / `bash download.sh`), never something
    register() does. NO binaries ever ship inside the plugin tree: the
    plugin downloads from the GitHub release (tag + checksums pinned in
    this tree) into the canonical runtime home <hermes-home>/aphrodite/
    binaries and validates against the in-tree checksum list. This
    function only verifies presence and, when a binary is missing, logs
    the explicit setup command - it never fetches from the network and
    never writes into the plugin directory.

    Legacy escape hatch: APHRODITE_AUTO_DOWNLOAD=1 (1/true, like
    _env_bool) restores the old register-time fetch for dev-loop setups
    that build from source and want download.sh as a convenience. Default
    is OFF - matching the review requirement that register() must not
    download.
    """
    if _env_bool("APHRODITE_AUTO_DOWNLOAD") and not (
        os.path.exists(_BINARY_PATH) and os.path.exists(_DYLIB_PATH)
    ):
        # Explicitly opted-in legacy convenience: fetch missing binaries.
        _check_version_published()
        try:
            env = os.environ.copy()
            env["BINARY_DIR"] = str(_BINARIES_DIR)
            result = subprocess.run(
                ["bash", str(_download_script())],
                timeout=180,
                capture_output=True,
                text=True,
                errors="replace",
                env=env,
            )
        except Exception as e:
            _log.warning(
                "failed to run %s (%s) - run `aphrodite setup` to install the aphrodite binaries",
                _download_script(),
                e,
            )
            return
        if result.returncode != 0:
            tail = "\n".join(((result.stdout or "") + (result.stderr or "")).splitlines()[-15:])
            _log.warning(
                "download.sh exited %d - run `aphrodite setup` to install the "
                "aphrodite binaries; output tail:\n%s",
                result.returncode,
                tail,
            )
        return
    if os.path.exists(_BINARY_PATH) and os.path.exists(_DYLIB_PATH):
        return
    missing = []
    for label, p in (
        ("proxy binary", _BINARY_PATH),
        ("dylib", _DYLIB_PATH),
    ):
        if not os.path.exists(p):
            missing.append(f"{label} ({p})")
    _log.warning(
        "aphrodite binaries missing (%s) - the plugin will not register. "
        "Run `aphrodite setup` once to install them into the runtime home "
        "(explicit setup step; register() never downloads, and the plugin "
        "never writes into its own directory), or set "
        "APHRODITE_AUTO_DOWNLOAD=1 for the legacy auto-fetch",
        "; ".join(missing),
    )


_health_opener: Any = None


def _proxy_healthy(port: int) -> bool:
    """One direct GET /health against a local proxy port; True only on a
    CONFIRMED aphrodite answer.

    MEDIUM (PR#8 review): the decisional pre-launch probe must not route
    through the ambient HTTP(S)_PROXY env vars - a corporate proxy
    answering 200 on the health path would make the probe falsely report
    healthy and skip launching the real proxy, silently killing CCR. An
    opener built with ProxyHandler({}) (empty proxy map) forces direct
    connections only. And the bare status code is not enough either: the
    response body must be JSON with ``status == "healthy"`` (the Rust
    health endpoint always answers 200 with that field), so an unrelated
    service squatting on the port cannot cause a false skip. Any error,
    timeout, or foreign body is unhealthy - the probe can only skip a
    launch on a confirmed aphrodite answer.
    """
    import urllib.request

    global _health_opener
    if _health_opener is None:
        try:
            _health_opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        except Exception as e:
            # Defensive: never skip on an opener failure - report and treat
            # as unhealthy so the launch still proceeds.
            _log.warning(
                "_proxy_healthy: cannot build direct opener (%s); treating as unhealthy",
                e,
            )
            return False
    try:
        req = urllib.request.Request(f"http://127.0.0.1:{port}/health", method="GET")
        with _health_opener.open(req, timeout=0.5) as resp:
            if resp.status != 200:
                return False
            body = resp.read(4096).decode("utf-8", errors="replace")
            try:
                payload = json.loads(body)
            except Exception:
                return False
            return isinstance(payload, dict) and payload.get("status") == "healthy"
    except Exception:
        return False


def _start_proxy():
    """Start the aphrodite proxy binary and verify both proxies are healthy.

    Probes both health endpoints BEFORE launching: every Hermes process
    (CLI runs, kanban workers, each profile's PluginManager) calls this on
    registration, but only one proxy can own the ports. Launching
    unconditionally made each extra process spawn a binary that immediately
    died on `failed to bind listener` (os error 10048 on Windows), appending
    that plus a `no API key configured` line to proxy-stderr.log every time.
    If both proxies already answer a CONFIRMED aphrodite health body (see
    `_proxy_healthy`), the launch is skipped at INFO level.

    Pipes stderr to a log file (not DEVNULL) so startup errors are
    diagnosable.  After launch, detects immediate death (stderr tail +
    API-key hint) and polls both proxy health endpoints for up to 5
    seconds, logging a warning for each one that doesn't come up.
    """
    import time

    # F13: README.md and this function's own env.setdefault() below have
    # advertised this guard since it was added, but nothing ever READ it -
    # `cargo watch`/dev-loop instructions telling developers to set
    # APHRODITE_NO_AUTO_LAUNCH=1 to stop the plugin from fighting them for
    # the ports had no effect at all.
    if os.environ.get("APHRODITE_NO_AUTO_LAUNCH", "0") in ("1", "true"):
        _log.info("APHRODITE_NO_AUTO_LAUNCH set - skipping proxy auto-launch")
        return

    # Fetch the proxy binary + dylib on first use if either is missing
    # (download.sh verifies SHA-256 and writes ~/.hermes/aphrodite/binaries/).
    # No-op when both already exist - repeated register() calls don't
    # re-download.
    _ensure_binaries()

    # Read custom ports from env vars (matching the Rust dylib's
    # configured_ports() in aphrodite-hermes/src/lib.rs).
    _cache_port = _parse_port_env("APHRODITE_CACHE_PORT", 9797)
    _token_port = _parse_port_env("APHRODITE_TOKEN_PORT", 9798)
    proxies = [
        ("cache", _cache_port),
        ("token", _token_port),
    ]

    # ── Pre-launch probe: skip launch iff another process already owns
    # BOTH proxies with a confirmed aphrodite answer (never on error,
    # never on a foreign body - see _proxy_healthy) ──
    up: set[str] = {name for name, port in proxies if _proxy_healthy(port)}
    if len(up) == len(proxies):
        _log.info(
            "aphrodite proxies already healthy on :%d/:%d - reusing the running "
            "instance, skipping launch",
            _cache_port,
            _token_port,
        )
        return

    binary = _resolve_binary_path()
    if not os.path.exists(binary):
        _log.warning("aphrodite binary not found at %s", binary)
        return
    if not os.access(binary, os.X_OK):
        os.chmod(binary, 0o755)
    env = os.environ.copy()
    env.setdefault("APHRODITE_NO_AUTO_LAUNCH", "0")

    # Write stderr to a log file so startup errors (e.g. SQLite "unable
    # to open database file") are visible.  Previously stderr was piped
    # to DEVNULL, making every startup failure silent.
    log_dir = _data_dir()
    try:
        log_dir.mkdir(parents=True, exist_ok=True)
    except OSError as e:
        # Defensive: a weird/unwritable APHRODITE_HOME must never crash
        # registration - fall back to the default location, then give up
        # with a warning rather than raising.
        _log.warning(
            "aphrodite data dir %s unusable (%s); falling back to <hermes-home>/aphrodite",
            log_dir,
            e,
        )
        log_dir = _hermes_home() / "aphrodite"
        try:
            log_dir.mkdir(parents=True, exist_ok=True)
        except OSError as e2:
            _log.warning("cannot create %s either (%s) - skipping proxy launch", log_dir, e2)
            return
    try:
        with open(log_dir / "proxy-stderr.log", "a") as stderr_log:
            proc = subprocess.Popen(
                [binary],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=stderr_log,
                cwd=os.getcwd(),
            )
    except Exception as e:
        _log.warning("failed to start aphrodite proxy: %s", e)
        return

    # Detect immediate death: a proxy binary that dies on launch (bad
    # config, missing API key, port conflict) used to be silently invisible
    # - the health check below would only log "did not become healthy" with
    # no clue why. Surface the stderr tail right away instead. A single
    # immediate poll() races the child's exec, so give a fast-exiting
    # binary a brief grace period before declaring it started.
    rc = proc.poll()
    if rc is None:
        with contextlib.suppress(subprocess.TimeoutExpired):
            rc = proc.wait(timeout=0.25)
    if rc is not None:
        _log.warning("aphrodite proxy exited immediately (rc=%s); last stderr:", rc)
        tail = _tail_log(log_dir / "proxy-stderr.log")
        _log.warning("%s", tail)
        if "API key" in tail:
            _log.warning(
                "set APHRODITE_API_KEY env var, run `aphrodite setup`, or "
                "add api_key to the TOML at %s",
                _data_dir() / "aphrodite.toml",
            )
        return

    _log.info("aphrodite proxy started (%s)", binary)

    # ── Health check: poll both proxies for up to 5 seconds ──────
    deadline = time.monotonic() + 5.0
    up = set()
    while time.monotonic() < deadline:
        for name, port in proxies:
            if name in up:
                continue
            if _proxy_healthy(port):
                up.add(name)
                _log.info("aphrodite %s proxy healthy on :%d", name, port)
        if len(up) == len(proxies):
            break
        time.sleep(0.5)

    for name, port in proxies:
        if name not in up:
            if proc.poll() is not None:
                _log.warning(
                    "aphrodite proxy process exited early (rc=%s) while "
                    "waiting for %s on :%d; last stderr:",
                    proc.poll(),
                    name,
                    port,
                )
                tail = _tail_log(log_dir / "proxy-stderr.log")
                _log.warning("%s", tail)
                if "API key" in tail:
                    _log.warning(
                        "set APHRODITE_API_KEY env var, run `aphrodite setup`, "
                        "or add api_key to the TOML at %s",
                        _data_dir() / "aphrodite.toml",
                    )
            else:
                _log.warning(
                    "aphrodite %s proxy on :%d did not become healthy within 5s "
                    "- check %s for errors",
                    name,
                    port,
                    log_dir / "proxy-stderr.log",
                )


# ── Plugin registration ──


def register(ctx: Any) -> None:
    """Register hooks, tools, and (optionally) a context engine with Hermes.

    Targets the Hermes v0.17.0 PluginContext API:
      register_hook(hook_name, callback)
      register_tool(name, toolset, schema, handler, ...)
      register_skill(name, path: Path, description="")
      register_context_engine(engine)   # engine must subclass ContextEngine
    Each registration is isolated so one failure never aborts the whole plugin.
    """
    try:
        dylib = _load_dylib()
    except Exception as e:
        # Defensive (issue triage): a missing/unloadable dylib (auto-download
        # failed, wrong architecture, corrupted binary) must disable the
        # plugin with a clear log - never propagate and abort Hermes' plugin
        # loading. The download.sh + APHRODITE_NO_AUTO_DOWNLOAD story above
        # covers the recovery path; without the dylib there is nothing to
        # register, so we log and return.
        _log.error(
            "aphrodite-hermes dylib could not be loaded (%s) - plugin disabled; "
            "run `aphrodite setup` to install the binaries into the runtime "
            "home (explicit setup step), then restart Hermes",
            e,
        )
        return
    _log.info("aphrodite-hermes dylib loaded: %s", _DYLIB_PATH)
    _check_version(dylib)

    # ── Layout self-heal (best-effort, never raises) ──
    # Ensure the ~/.hermes runtime layout matches the canonical schema before
    # anything consumes it: runtime state belongs in ~/.hermes/aphrodite, and
    # the plugin dir stays hooks-only. The layout_check module ships with the
    # source checkout; installed hooks-only layouts simply skip it (debug
    # level, no noise) - `aphrodite setup` already produced the canonical
    # layout.
    try:
        if (Path(__file__).resolve().parent / "layout_check.py").exists():
            from .layout_check import check_and_heal

            report = check_and_heal()
            for w in report.get("warnings", []):
                _log.warning("layout self-heal: %s", w)
        else:
            _log.debug("layout_check.py not shipped with this install - self-heal skipped")
    except Exception as e:
        _log.warning("layout self-heal skipped: %s", e)

    # ── Directives materialize (best-effort, never raises) ──
    # The binary provides the directives (embedded builtins); unpack them into
    # the user-data home ~/.hermes/aphrodite/directives/ so the plugin dir
    # never has to hold them. User-modified files are never overwritten.
    # home_dir=b"" -> the dylib falls back to its home resolution
    # (APHRODITE_DIRECTIVES_DIR / APHRODITE_HOME / ~/.hermes/aphrodite).
    try:
        result = _call_json(dylib, "aphrodite_hermes_materialize_directives", b"")
        if result:
            for w in result.get("warnings", []):
                _log.warning("directives materialize: %s", w)
    except Exception as e:
        _log.warning("directives materialize skipped: %s", e)

    # Register hooks - dispatch to Rust dylib via aphrodite_hermes_call_hook
    hooks = _call_json(dylib, "aphrodite_hermes_get_hooks")
    if hooks:

        def _hook_dispatch(hook_name: str, **kwargs: Any) -> Any:
            """Dispatch hook to Rust dylib and return parsed result.

            Resolves the dylib fresh on every call instead of closing over
            the `dylib` captured above (F4): hooks and tool handlers both
            re-resolve per call so every dispatch goes through the current
            image (the process-global holder returns the single mapped
            handle - no per-call copy or reload).
            """
            # Hermes passes hook args as kwargs (result, output, tool_name, ...).
            # default=str keeps any non-JSON-serializable extras (e.g. message
            # objects on pre/post_llm_call) from crashing the hook.
            args_json = json.dumps(kwargs, default=str)
            return _call_json(
                _load_dylib(),
                "aphrodite_hermes_call_hook",
                hook_name.encode("utf-8"),
                args_json.encode("utf-8"),
            )

        registered_hooks = 0
        for hook_name in hooks:

            def _dispatch(*a: Any, name: str = hook_name, **kw: Any) -> Any:
                return _hook_dispatch(name, **kw)

            try:
                ctx.register_hook(hook_name, _dispatch)
                registered_hooks += 1
            except Exception as e:
                # Isolate each hook like register_tool below: one broken
                # registration must never abort the whole plugin (harden
                # item 5).
                _log.warning("failed to register hook %s: %s", hook_name, e)
        _log.info("registered %d hooks", registered_hooks)

    # Register tools. Hermes API: register_tool(name, toolset, schema, handler).
    schemas = _call_json(dylib, "aphrodite_hermes_get_schemas")
    if schemas:
        registered: list[str] = []
        for schema in schemas:
            name = schema["name"]
            try:
                ctx.register_tool(name, "aphrodite", schema, _make_handler(name))
                registered.append(name)
            except Exception as e:
                _log.warning("failed to register tool %s: %s", name, e)
        _log.info("registered %d tools: %s", len(registered), registered)

    # Skills are NOT shipped with the plugin - they live dev-side in the
    # monorepo's .hermes/skills/ (Development branch only, never on Current).
    # The plugin registers tools and hooks only.

    # Context engine is opt-in (APHRODITE_CONTEXT_ENGINE=1). Hermes expects a
    # ContextEngine subclass instance here; the per-turn catalog summary is
    # already injected via the pre_llm_call hook above, so the default path
    # needs no engine. Registering anything other than a ContextEngine instance
    # is silently rejected by Hermes, so we only attempt it when asked.
    if _env_bool("APHRODITE_CONTEXT_ENGINE"):
        try:
            _register_context_engine(ctx, dylib)
        except Exception as e:
            _log.warning(
                "context engine opt-in requested but not registered (%s); "
                "falling back to hooks + proxy",
                e,
            )

    # Proxy launch is skipped under `hermes plugins validate`: the capability
    # probe runs register() in a scratch child (RecordingContext,
    # plugin_id == "hermes_validate_probe_plugin") and must have no side
    # effects on the live machine - no port binding, no subprocesses
    # (catalog review, PR 118488: the probe downloaded binaries, materialized
    # directives and bound :9797/:9798 in the real environment).
    if getattr(ctx, "plugin_id", "") == "hermes_validate_probe_plugin":
        _log.info("validate probe context - skipping proxy launch")
    else:
        _start_proxy()


def _register_context_engine(ctx: Any, dylib: ctypes.CDLL) -> None:
    """Best-effort context-engine registration (opt-in).

    Builds a thin ContextEngine subclass whose pre-flight summary comes from the
    dylib catalog. Raises if the host Hermes does not expose ContextEngine, so
    the caller can fall back to the hook + proxy path.
    """
    # Resolved dynamically: `agent.context_engine` only exists inside the Hermes
    # runtime, so a static import would break standalone lint/type checks.
    import importlib

    context_engine_cls = importlib.import_module("agent.context_engine").ContextEngine

    class AphroditeContextEngine(context_engine_cls):
        @property
        def name(self) -> str:
            return "aphrodite"

        def update_from_response(self, usage: dict[str, Any]) -> None:
            self.last_prompt_tokens = usage.get("prompt_tokens", 0)
            self.last_completion_tokens = usage.get("completion_tokens", 0)
            self.last_total_tokens = usage.get("total_tokens", 0)

        def should_compress(self, prompt_tokens: int | None = None) -> bool:
            # Defer to Hermes' own threshold accounting; the proxy + hooks do the
            # heavy lifting, so the engine itself never forces a compaction.
            return False

        def compress(
            self,
            messages: list[Any],
            current_tokens: int | None = None,
            focus_topic: str | None = None,
        ) -> list[Any]:
            # Non-destructive: the proxy and transform hooks already shrink tool
            # output, so the engine returns the transcript unchanged.
            return messages

    ctx.register_context_engine(AphroditeContextEngine())
