"""Self-contained tests for Maintain/check_ffi_contract.py (no pytest).

Run directly:  python3 Maintain/tests/test_check_ffi_contract.py
Exit code is 0 when every case passes, 1 otherwise.
"""

import sys
import tempfile
from pathlib import Path

MAINTAIN_DIR = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(MAINTAIN_DIR))

import check_ffi_contract as ffi  # noqa: E402

REPO_ROOT = MAINTAIN_DIR.parent

_ASSERTS = 0

# ── Fixtures ────────────────────────────────────────────────────────────────
# Mirror of the real export surface (10 exports, all *mut c_char except
# free_string which returns void). Multi-line signature included on purpose.
FIXTURE_LIBRS = """\
//! Minimal fixture mirroring crates/aphrodite-hermes/src/lib.rs.

#[no_mangle]
pub extern "C" fn aphrodite_hermes_dispatch_tool(
    tool_name: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_list_tools() -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_schema(tool_name: *const c_char) -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_free_string(s: *mut c_char) {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_version() -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_call_hook(hook_name: *const c_char, args_json: *const c_char) -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_schemas() -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_hooks() -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_proxy_health() -> *mut c_char {
    todo!()
}

#[no_mangle]
pub extern "C" fn aphrodite_hermes_materialize_directives(home_dir: *const c_char) -> *mut c_char {
    todo!()
}
"""

# Base plugin fixture: fully-configured setup block (the fixed/hardened state),
# _REQUIRED_VOID_P, _call_json call sites, a direct dylib call, plus deliberate
# noise (k32-style unrelated restype assignments and a _probe_dylib-style
# triple-quoted script string) that AST parsing must ignore.
FIXTURE_INIT = """\
import ctypes

# String-literal noise that text-based parsing would misread as a real
# restype assignment - AST parsing must ignore it.
_PROBE_SCRIPT = (
    "try:\\n"
    "    d = ctypes.CDLL(path)\\n"
    "    d.aphrodite_hermes_version.restype = ctypes.c_void_p\\n"
    "    ptr = d.aphrodite_hermes_version()\\n"
)

_REQUIRED_VOID_P: tuple[str, ...] = (
    "aphrodite_hermes_dispatch_tool",
    "aphrodite_hermes_call_hook",
    "aphrodite_hermes_proxy_health",
    "aphrodite_hermes_version",
    "aphrodite_hermes_materialize_directives",
    "aphrodite_hermes_get_schemas",
    "aphrodite_hermes_get_hooks",
)


def _win_process_alive(pid):
    k32 = ctypes.windll.kernel32
    k32.OpenProcess.restype = ctypes.wintypes.HANDLE
    k32.GetExitCodeProcess.restype = ctypes.wintypes.BOOL
    return True


def _load_dylib():
    dylib = ctypes.CDLL("libaphrodite_hermes.dylib")
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
    return dylib


def _call_json(dylib, fn_name, *args):
    fn = getattr(dylib, fn_name)
    fn.restype = ctypes.c_void_p
    ptr = fn(*args)
    if ptr:
        dylib.aphrodite_hermes_free_string(ptr)
    return None


def register():
    dylib = _load_dylib()
    result = _call_json(dylib, "aphrodite_hermes_materialize_directives", b"")
    hooks = _call_json(dylib, "aphrodite_hermes_get_hooks")
    schemas = _call_json(dylib, "aphrodite_hermes_get_schemas")
    version = _call_json(dylib, "aphrodite_hermes_version")
    tools = _call_json(dylib, "aphrodite_hermes_dispatch_tool", b"{}")
    _call_json(dylib, "aphrodite_hermes_proxy_health")
    _call_json(dylib, "aphrodite_hermes_call_hook", b"{}")
    return dylib
"""

MATERIALIZE_RESTYPE_LINE = (
    "    dylib.aphrodite_hermes_materialize_directives.restype = ctypes.c_void_p"
)

# ctypesgen-style generated bindings covering every required symbol.
BINDINGS_ALL = """\
from ctypes import c_void_p, c_char_p

aphrodite_hermes_dispatch_tool.argtypes = [c_char_p, c_char_p]
aphrodite_hermes_dispatch_tool.restype = c_void_p
aphrodite_hermes_call_hook.argtypes = [c_char_p, c_char_p]
aphrodite_hermes_call_hook.restype = c_void_p
aphrodite_hermes_proxy_health.restype = c_void_p
aphrodite_hermes_version.restype = c_void_p
aphrodite_hermes_materialize_directives.argtypes = [c_char_p]
aphrodite_hermes_materialize_directives.restype = c_void_p
aphrodite_hermes_get_schemas.restype = c_void_p
aphrodite_hermes_get_hooks.restype = c_void_p
aphrodite_hermes_free_string.argtypes = [c_void_p]
"""


def ok(condition, message=""):
    global _ASSERTS
    _ASSERTS += 1
    if not condition:
        raise AssertionError(message or "assertion failed")


# ── Parser unit tests ────────────────────────────────────────────────────────
def test_parsers():
    exports = ffi.parse_exports(FIXTURE_LIBRS)
    ok(len(exports) == 10, f"expected 10 exports, got {sorted(exports)}")
    ok(exports["aphrodite_hermes_free_string"] == "void", "free_string not void")
    ok(exports["aphrodite_hermes_materialize_directives"] == "pointer", "materialize not pointer")
    ok(exports["aphrodite_hermes_list_tools"] == "pointer", "list_tools not pointer")

    required = ffi.parse_required_void_p(FIXTURE_INIT)
    ok(len(required) == 7, f"expected 7 required, got {sorted(required)}")
    ok("aphrodite_hermes_materialize_directives" in required, "materialize missing from required")

    called = ffi.parse_call_json_symbols(FIXTURE_INIT)
    ok("aphrodite_hermes_version" in called, "version not detected as called")
    ok("aphrodite_hermes_materialize_directives" in called, "materialize not detected as called")
    ok("aphrodite_hermes_get_hooks" in called, "get_hooks not detected as called")
    ok(len(called) == 7, f"expected 7 called symbols, got {sorted(called)}")

    direct = ffi.parse_direct_dylib_calls(FIXTURE_INIT)
    ok(direct == {"aphrodite_hermes_free_string"}, f"unexpected direct calls: {direct}")

    restypes = ffi.parse_restype_assignments(FIXTURE_INIT)
    ok(
        restypes.get("aphrodite_hermes_materialize_directives") == "ctypes.c_void_p",
        "materialize restype not parsed",
    )
    ok("OpenProcess" not in restypes, "k32 restype leaked into results")
    ok("GetExitCodeProcess" not in restypes, "k32 restype leaked into results")
    ok("fn" not in restypes, "_call_json's fn.restype clamp leaked into results")
    ok(len(restypes) == 7, f"expected 7 restype assignments, got {sorted(restypes)}")


# ── Contract tests ──────────────────────────────────────────────────────────
def test_clean_contract():
    violations, warnings, source = ffi.check(FIXTURE_LIBRS, FIXTURE_INIT)
    ok(violations == [], f"unexpected violations: {violations}")
    ok(source == "inline setup block", f"unexpected restype source: {source}")
    ok(
        any("list_tools" in w for w in warnings),
        "expected unconfigured-export warning for list_tools",
    )
    ok(
        any("get_schema" in w for w in warnings),
        "expected unconfigured-export warning for get_schema",
    )


def test_historical_missing_restype_flagged():
    # The SIGSEGV root cause: materialize_directives called via _call_json (and
    # listed in _REQUIRED_VOID_P) but with no restype in the setup block.
    init = FIXTURE_INIT.replace(MATERIALIZE_RESTYPE_LINE + "\n", "")
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any("materialize_directives" in v and "missing-restype" in v for v in violations),
        f"historical bug not flagged: {violations}",
    )


def test_wrong_restype_flagged():
    init = FIXTURE_INIT.replace(
        MATERIALIZE_RESTYPE_LINE,
        "    dylib.aphrodite_hermes_materialize_directives.restype = ctypes.c_int",
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any(
            "materialize_directives" in v and "wrong-restype" in v and "c_int" in v
            for v in violations
        ),
        f"c_int restype not flagged: {violations}",
    )
    # c_char_p is full-width but violates the c_void_p contract (Python 3.14
    # malloc-mismatch SIGABRT class) - must also be flagged.
    init = FIXTURE_INIT.replace(
        MATERIALIZE_RESTYPE_LINE,
        "    dylib.aphrodite_hermes_materialize_directives.restype = ctypes.c_char_p",
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any(
            "materialize_directives" in v and "wrong-restype" in v and "c_char_p" in v
            for v in violations
        ),
        f"c_char_p restype not flagged: {violations}",
    )


def test_call_json_without_setup_flagged():
    # New pointer-returning export wired into _call_json but never configured:
    # the "future export added without a setup-block entry" class.
    librs = (
        FIXTURE_LIBRS
        + '\n#[no_mangle]\npub extern "C" fn aphrodite_hermes_future_export() -> *mut c_char {\n    todo!()\n}\n'
    )
    init = FIXTURE_INIT.replace(
        '    result = _call_json(dylib, "aphrodite_hermes_materialize_directives", b"")\n',
        '    result = _call_json(dylib, "aphrodite_hermes_materialize_directives", b"")\n'
        '    future = _call_json(dylib, "aphrodite_hermes_future_export")\n',
    )
    violations, _, _ = ffi.check(librs, init)
    ok(
        any("aphrodite_hermes_future_export" in v and "missing-restype" in v for v in violations),
        f"unconfigured _call_json symbol not flagged: {violations}",
    )
    # Ghost symbol: called via _call_json but not exported at all.
    init = FIXTURE_INIT.replace(
        '    hooks = _call_json(dylib, "aphrodite_hermes_get_hooks")\n',
        '    hooks = _call_json(dylib, "aphrodite_hermes_get_hooks")\n'
        '    ghost = _call_json(dylib, "aphrodite_hermes_nonexistent_export")\n',
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any(
            "aphrodite_hermes_nonexistent_export" in v and "called-not-exported" in v
            for v in violations
        ),
        f"nonexistent _call_json symbol not flagged: {violations}",
    )


def test_required_symbol_not_exported_flagged():
    init = FIXTURE_INIT.replace(
        '"aphrodite_hermes_get_hooks",\n)',
        '"aphrodite_hermes_get_hooks",\n    "aphrodite_hermes_ghost_symbol",\n)',
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any(
            "aphrodite_hermes_ghost_symbol" in v and "required-not-exported" in v
            for v in violations
        ),
        f"ghost required symbol not flagged: {violations}",
    )


def test_unknown_export_configured_flagged():
    init = FIXTURE_INIT.replace(
        MATERIALIZE_RESTYPE_LINE,
        "    dylib.aphrodite_hermes_typo_symbol.restype = ctypes.c_void_p",
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any(
            "aphrodite_hermes_typo_symbol" in v and "unknown-export-configured" in v
            for v in violations
        ),
        f"typo setup entry not flagged: {violations}",
    )


# ── Generated-bindings mode ─────────────────────────────────────────────────
def test_bindings_clean():
    # Bindings REPLACE the inline setup block as restype source of truth.
    violations, warnings, source = ffi.check(
        FIXTURE_LIBRS, FIXTURE_INIT, bindings_source=BINDINGS_ALL
    )
    ok(source == "generated bindings", f"unexpected restype source: {source}")
    ok(violations == [], f"bindings violations: {violations}")
    ok(any("list_tools" in w for w in warnings), "expected list_tools warning in bindings mode")


def test_bindings_missing_restype_flagged():
    bindings = BINDINGS_ALL.replace(
        "aphrodite_hermes_materialize_directives.restype = c_void_p\n", ""
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, FIXTURE_INIT, bindings_source=bindings)
    ok(
        any("materialize_directives" in v and "missing-restype" in v for v in violations),
        f"bindings gap not flagged: {violations}",
    )


def test_bindings_wrong_restype_flagged():
    bindings = BINDINGS_ALL.replace(
        "aphrodite_hermes_materialize_directives.restype = c_void_p",
        "aphrodite_hermes_materialize_directives.restype = c_int",
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, FIXTURE_INIT, bindings_source=bindings)
    ok(
        any(
            "materialize_directives" in v and "wrong-restype" in v and "c_int" in v
            for v in violations
        ),
        f"bindings c_int not flagged: {violations}",
    )


def test_argtypes_count_mismatch_flagged():
    # The codegen finalize step validates argtypes against the header's
    # parameter counts at build time; the static checker must catch the same
    # class so a hand-edited artifact cannot silently misalign the ABI frame
    # (ctypes marshals args positionally and never verifies the count).
    # Inline setup: argtypes declares 2 params for a 1-param export.
    init = FIXTURE_INIT.replace(
        "    dylib.aphrodite_hermes_materialize_directives.argtypes = [ctypes.c_char_p]",
        "    dylib.aphrodite_hermes_materialize_directives.argtypes = [ctypes.c_char_p, ctypes.c_char_p]",
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any("materialize_directives" in v and "wrong-argcount" in v for v in violations),
        f"inline argtypes count mismatch not flagged: {violations}",
    )
    # Generated bindings: same class, same violation.
    bindings = BINDINGS_ALL.replace(
        "aphrodite_hermes_materialize_directives.argtypes = [c_char_p]",
        "aphrodite_hermes_materialize_directives.argtypes = [c_char_p, c_char_p]",
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, FIXTURE_INIT, bindings_source=bindings)
    ok(
        any("materialize_directives" in v and "wrong-argcount" in v for v in violations),
        f"bindings argtypes count mismatch not flagged: {violations}",
    )
    # Ghost: argtypes configured for a symbol that is not an export at all.
    init = FIXTURE_INIT.replace(
        "    dylib.aphrodite_hermes_materialize_directives.argtypes = [ctypes.c_char_p]",
        "    dylib.aphrodite_hermes_typo_symbol.argtypes = [ctypes.c_char_p]",
    )
    violations, _, _ = ffi.check(FIXTURE_LIBRS, init)
    ok(
        any(
            "aphrodite_hermes_typo_symbol" in v and "unknown-export-argtypes" in v
            for v in violations
        ),
        f"ghost argtypes symbol not flagged: {violations}",
    )


# ── Real repo + CLI end-to-end ───────────────────────────────────────────────
def test_real_repo_clean():
    librs = REPO_ROOT / "crates" / "aphrodite-hermes" / "src" / "lib.rs"
    plugin = REPO_ROOT / "plugins" / "aphrodite" / "__init__.py"
    ok(librs.is_file(), f"lib.rs missing: {librs}")
    ok(plugin.is_file(), f"plugin missing: {plugin}")
    violations, warnings, source = ffi.check(librs.read_text(), plugin.read_text())
    ok(violations == [], f"real repo violations: {violations}")
    ok(source == "inline setup block", f"unexpected restype source: {source}")
    ok(any("list_tools" in w for w in warnings), "expected list_tools unconfigured warning")
    ok(any("get_schema" in w for w in warnings), "expected get_schema unconfigured warning")
    # CLI end-to-end against the real repo.
    ok(
        ffi.main(["--librs", str(librs), "--plugin", str(plugin)]) == 0,
        "CLI exit != 0 on real repo",
    )


def test_cli_exit_codes_and_missing_files():
    # The checker auto-detects the committed generated bindings
    # (DEFAULT_BINDINGS) as the restype ground truth, so removing an inline
    # restype line is MASKED when the artifact covers the symbol. Masking is a
    # toolchain property, NOT a relaxation of the contract: a missing restype
    # on a pointer-returning export must fail validation whenever it is absent
    # from the ground-truth source the checker actually uses. This test pins
    # both halves - the masked behavior and the real contract.
    with tempfile.TemporaryDirectory() as td:
        root = Path(td)
        (root / "lib.rs").write_text(FIXTURE_LIBRS)
        init = root / "init.py"
        bindings = root / "bindings.py"
        original_default = ffi.DEFAULT_BINDINGS

        def cli(*args):
            return ffi.main(["--librs", str(root / "lib.rs"), "--plugin", str(init), *args])

        try:
            # Clean fixture + explicit --bindings flag path: exit 0.
            init.write_text(FIXTURE_INIT)
            bindings.write_text(BINDINGS_ALL)
            ok(cli("--bindings", str(bindings)) == 0, "clean fixture CLI exit != 0")

            # Ground truth = generated bindings: a missing restype THERE is a
            # real violation -> exit 1.
            bindings.write_text(
                BINDINGS_ALL.replace(
                    "aphrodite_hermes_materialize_directives.restype = c_void_p\n", ""
                )
            )
            ok(cli("--bindings", str(bindings)) == 1, "regressed bindings CLI exit != 1")

            # Ground truth = inline setup block with the artifact covering the
            # symbol: the inline removal is masked -> exit 0. Pinned so the
            # auto-detection semantics stay explicit and regression-detectable.
            bindings.write_text(BINDINGS_ALL)
            ffi.DEFAULT_BINDINGS = bindings
            init.write_text(FIXTURE_INIT.replace(MATERIALIZE_RESTYPE_LINE + "\n", ""))
            ok(cli() == 0, "masked inline regression unexpectedly failed")

            # Same inline regression with the artifact NEUTRALIZED: the
            # violation is visible again -> exit 1 (the historical SIGSEGV
            # class this test was written to guard).
            ffi.DEFAULT_BINDINGS = root / "absent-bindings.py"
            ok(cli() == 1, "regressed fixture CLI exit != 1")

            # Clean inline setup, no artifact at all: exit 0.
            init.write_text(FIXTURE_INIT)
            ok(cli() == 0, "clean inline fixture CLI exit != 0")
        finally:
            ffi.DEFAULT_BINDINGS = original_default

        # Usage errors: exit 2.
        missing = root / "nope.py"
        ok(cli("--bindings", str(missing)) == 2, "missing --bindings exit != 2")
        ok(
            ffi.main(["--librs", str(root / "lib.rs"), "--plugin", str(root / "absent.py")]) == 2,
            "missing plugin exit != 2",
        )
        ok(
            ffi.main(["--librs", str(root / "absent.rs"), "--plugin", str(init)]) == 2,
            "missing lib.rs exit != 2",
        )


def main():
    tests = [
        fn for name, fn in sorted(globals().items()) if name.startswith("test_") and callable(fn)
    ]
    failed = []
    for fn in tests:
        try:
            fn()
            print(f"PASS {fn.__name__}")
        except Exception as exc:  # noqa: BLE001 - test harness
            failed.append((fn.__name__, exc))
            print(f"FAIL {fn.__name__}: {exc}")
    passed = len(tests) - len(failed)
    print(f"{passed}/{len(tests)} cases passed, {_ASSERTS} asserts")
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main()
