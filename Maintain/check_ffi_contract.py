#!/usr/bin/env python3
"""Static FFI-contract checker for the Aphrodite Rust<->ctypes boundary.

Verifies that every pointer-returning ``#[no_mangle] pub extern "C"`` export
in ``crates/aphrodite-hermes/src/lib.rs`` that the Python shim binds or calls
is configured with ``restype = c_void_p``. A missing restype is the historical
SIGSEGV root cause: ctypes' default restype (``c_int``) reads the 64-bit
``*mut c_char`` return truncated and sign-extended, and ``_read_str`` then
strlen()s a bogus low address.

Restype ground truth is read from either the inline ``_load_dylib`` setup
block in ``plugins/aphrodite/__init__.py`` or, when present, a generated
``plugins/aphrodite/_bindings.py`` (the plugin prefers generated bindings).
``_REQUIRED_VOID_P`` in the plugin is parsed as additional ground truth: every
symbol it lists must exist as a pointer-returning export AND be configured.

The checker also enforces the argtypes-count contract on every export the shim
binds: an ``argtypes = [...]`` assignment's length must equal the export's Rust
parameter count. ctypes marshals arguments positionally and never verifies the
count itself, so a mismatch silently misaligns the ABI frame; the codegen
finalize step validates the same invariant at build time, and this checker
guards the committed artifact against hand edits.

All parsing is AST-based (``ast`` for Python, structural regex for Rust), so
string literals - e.g. the ``_probe_dylib`` subprocess script that mentions
``d.aphrodite_hermes_version.restype`` - cannot pollute the results.

Run directly:  python3 Maintain/check_ffi_contract.py
Exit code is 0 when the contract holds, 1 on violations, 2 on usage errors.
"""

import argparse
import ast
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_LIBRS = REPO_ROOT / "crates" / "aphrodite-hermes" / "src" / "lib.rs"
DEFAULT_PLUGIN = REPO_ROOT / "plugins" / "aphrodite" / "__init__.py"
DEFAULT_BINDINGS = REPO_ROOT / "plugins" / "aphrodite" / "_bindings.py"

EXPORT_PREFIX = "aphrodite_hermes_"
VOID_P_SUFFIX = "c_void_p"

# Matches `#[no_mangle]` followed by `pub [unsafe] extern "C" fn NAME(args)
# [-> RET] {`. Single-line signatures like the ones in lib.rs; tolerant of
# whitespace/newlines between the signature and the body brace. Group 2 is
# the argument list (counted by parse_export_arg_counts); group 3 the return.
EXPORT_RE = re.compile(
    r'#\[no_mangle\]\s+pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+(\w+)\s*\(([^)]*)\)\s*(?:->\s*([^{]+))?\s*\{',
    re.MULTILINE,
)


def classify_return(ret):
    """'pointer' for any raw-pointer return, 'void' for no/() return, else 'other'."""
    if ret is None:
        return "void"
    ret = ret.strip()
    if not ret or ret == "()":
        return "void"
    if "*" in ret:
        return "pointer"
    return "other"


def parse_exports(librs_source):
    """{symbol: return-kind} for every #[no_mangle] extern "C" fn in lib.rs."""
    exports = {}
    for match in EXPORT_RE.finditer(librs_source):
        exports[match.group(1)] = classify_return(match.group(3))
    return exports


def parse_export_arg_counts(librs_source):
    """{symbol: parameter count} for every #[no_mangle] extern "C" fn in lib.rs.

    The parameter list group is `([^)]*)`, so nested parens (e.g. function
    pointers) are not counted - lib.rs exports are plain pointer/scalar args.
    """
    counts = {}
    for match in EXPORT_RE.finditer(librs_source):
        args = match.group(2)
        counts[match.group(1)] = 0 if not args.strip() else len(
            [param for param in args.split(",") if param.strip()]
        )
    return counts


def parse_required_void_p(plugin_source):
    """Set of symbols listed in the plugin's _REQUIRED_VOID_P tuple."""
    tree = ast.parse(plugin_source)
    for node in tree.body:
        value = None
        if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
            if node.target.id == "_REQUIRED_VOID_P":
                value = node.value
        elif isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == "_REQUIRED_VOID_P":
                    value = node.value
        if value is None or not isinstance(value, ast.Tuple):
            continue
        return {
            element.value
            for element in value.elts
            if isinstance(element, ast.Constant) and isinstance(element.value, str)
        }
    return set()


def parse_call_json_symbols(plugin_source):
    """Set of symbol names passed as the 2nd positional arg to _call_json(...)."""
    tree = ast.parse(plugin_source)
    symbols = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        func = node.func
        if not (isinstance(func, ast.Name) and func.id == "_call_json"):
            continue
        if len(node.args) >= 2:
            arg = node.args[1]
            if isinstance(arg, ast.Constant) and isinstance(arg.value, str):
                symbols.add(arg.value)
    return symbols


def parse_direct_dylib_calls(plugin_source):
    """Set of symbols invoked directly as dylib.SYMBOL(...)."""
    tree = ast.parse(plugin_source)
    symbols = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        func = node.func
        if (
            isinstance(func, ast.Attribute)
            and isinstance(func.value, ast.Name)
            and func.value.id == "dylib"
        ):
            symbols.add(func.attr)
    return symbols


def parse_restype_assignments(source):
    """{symbol: restype-expression} for every ``<obj>.SYM.restype = ...`` or
    ``SYM.restype = ...`` assignment whose symbol uses the export prefix.

    AST-based on purpose: string literals (the _probe_dylib subprocess script)
    and unrelated objects (k32, wt) are structurally excluded.
    """
    tree = ast.parse(source)
    restypes = {}
    for node in ast.walk(tree):
        if not isinstance(node, ast.Assign):
            continue
        for target in node.targets:
            if not (isinstance(target, ast.Attribute) and target.attr == "restype"):
                continue
            base = target.value
            if isinstance(base, ast.Name):
                symbol = base.id
            elif isinstance(base, ast.Attribute):
                symbol = base.attr
            else:
                continue
            if not symbol.startswith(EXPORT_PREFIX):
                continue
            restypes[symbol] = ast.unparse(node.value).strip()
    return restypes


def parse_argtypes_assignments(source):
    """{symbol: [argtype-expressions]} for every ``<obj>.SYM.argtypes = [...]``.

    Shares the AST discipline of parse_restype_assignments: only assignments
    whose symbol uses the export prefix are kept; string literals (the
    _probe_dylib subprocess script) and unrelated objects (k32, wt) are
    structurally excluded. The LIST LENGTH is the contract: ctypes marshals
    arguments positionally and never verifies the count itself, so it must
    equal the export's Rust parameter count.
    """
    tree = ast.parse(source)
    argtypes = {}
    for node in ast.walk(tree):
        if not isinstance(node, ast.Assign):
            continue
        for target in node.targets:
            if not (isinstance(target, ast.Attribute) and target.attr == "argtypes"):
                continue
            base = target.value
            if isinstance(base, ast.Name):
                symbol = base.id
            elif isinstance(base, ast.Attribute):
                symbol = base.attr
            else:
                continue
            if not symbol.startswith(EXPORT_PREFIX):
                continue
            if isinstance(node.value, (ast.List, ast.Tuple)):
                argtypes[symbol] = [ast.unparse(element).strip() for element in node.value.elts]
    return argtypes


def check(librs_source, plugin_source, bindings_source=None):
    """Diff the export surface against the restype ground truth.

    Returns (violations, warnings, restype_source_name). Violations exit 1;
    warnings exit 0 (surfaced so unbound exports are visible, not fatal).
    """
    exports = parse_exports(librs_source)
    arg_counts = parse_export_arg_counts(librs_source)
    required = parse_required_void_p(plugin_source)
    called = parse_call_json_symbols(plugin_source)
    direct = parse_direct_dylib_calls(plugin_source)
    if bindings_source is not None:
        restypes = parse_restype_assignments(bindings_source)
        argtypes = parse_argtypes_assignments(bindings_source)
        restype_source_name = "generated bindings"
    else:
        restypes = parse_restype_assignments(plugin_source)
        argtypes = parse_argtypes_assignments(plugin_source)
        restype_source_name = "inline setup block"

    violations = []
    warnings = []
    bound = required | called | direct

    for symbol in sorted(bound):
        kind = exports.get(symbol)
        if kind is None:
            if symbol in required:
                violations.append(
                    f"[required-not-exported] {symbol}: listed in _REQUIRED_VOID_P "
                    "but not a #[no_mangle] export in lib.rs"
                )
            if symbol in called:
                violations.append(
                    f"[called-not-exported] {symbol}: called via _call_json but not "
                    "a #[no_mangle] export in lib.rs"
                )
            if symbol in direct:
                violations.append(
                    f"[direct-call-not-exported] {symbol}: invoked as "
                    f"dylib.{symbol}(...) but not a #[no_mangle] export in lib.rs"
                )
            continue
        if symbol in argtypes:
            expected = arg_counts.get(symbol)
            actual = len(argtypes[symbol])
            if expected is not None and actual != expected:
                violations.append(
                    f"[wrong-argcount] {symbol}: argtypes declares {actual} "
                    f"parameter(s) but lib.rs declares {expected} - ctypes "
                    "marshals arguments positionally and never verifies the "
                    "count, so a mismatch misaligns the ABI frame"
                )
        if kind == "pointer":
            if symbol in restypes:
                restype = restypes[symbol]
                if restype.rsplit(".", 1)[-1] != VOID_P_SUFFIX:
                    violations.append(
                        f"[wrong-restype] {symbol}: restype is {restype}, expected "
                        "c_void_p - any other restype truncates or mangles the "
                        "64-bit pointer return"
                    )
            else:
                violations.append(
                    f"[missing-restype] {symbol}: pointer-returning export bound by "
                    "the shim without restype=c_void_p - ctypes defaults to c_int, "
                    "truncating the 64-bit pointer (historical SIGSEGV root cause)"
                )
        elif symbol in called:
            violations.append(
                f"[called-non-pointer] {symbol}: routed through _call_json but lib.rs "
                f"declares a non-pointer return ({kind}) - _call_json reads every "
                "return as a pointer"
            )
        elif symbol in required:
            violations.append(
                f"[required-non-pointer] {symbol}: listed in _REQUIRED_VOID_P but "
                f"lib.rs declares a non-pointer return ({kind})"
            )

    for symbol in sorted(restypes):
        if symbol not in exports:
            violations.append(
                f"[unknown-export-configured] {symbol}: restype configured but symbol "
                "is not a #[no_mangle] export in lib.rs"
            )

    for symbol in sorted(argtypes):
        if symbol not in exports:
            violations.append(
                f"[unknown-export-argtypes] {symbol}: argtypes configured but symbol "
                "is not a #[no_mangle] export in lib.rs"
            )

    for symbol in sorted(exports):
        if exports[symbol] == "pointer" and symbol not in restypes and symbol not in bound:
            warnings.append(
                f"[unconfigured-export] {symbol}: pointer-returning export never "
                "bound by the shim - safe today, SIGSEGV the moment it is called "
                "without restype=c_void_p"
            )

    return violations, warnings, restype_source_name


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="check_ffi_contract.py",
        description="Static Rust<->ctypes FFI-contract checker for Aphrodite.",
    )
    parser.add_argument(
        "--librs",
        default=str(DEFAULT_LIBRS),
        help="path to crates/aphrodite-hermes/src/lib.rs",
    )
    parser.add_argument(
        "--plugin",
        default=str(DEFAULT_PLUGIN),
        help="path to plugins/aphrodite/__init__.py",
    )
    parser.add_argument(
        "--bindings",
        default=None,
        help="path to a generated _bindings.py used as the restype source of "
        "truth; auto-detected at plugins/aphrodite/_bindings.py when omitted",
    )
    args = parser.parse_args(argv)

    librs_path = Path(args.librs)
    plugin_path = Path(args.plugin)
    if not librs_path.is_file():
        print(f"ERROR: lib.rs not found: {librs_path}", file=sys.stderr)
        return 2
    if not plugin_path.is_file():
        print(f"ERROR: plugin not found: {plugin_path}", file=sys.stderr)
        return 2

    bindings_path = None
    if args.bindings:
        bindings_path = Path(args.bindings)
        if not bindings_path.is_file():
            print(f"ERROR: --bindings file not found: {bindings_path}", file=sys.stderr)
            return 2
    elif DEFAULT_BINDINGS.is_file():
        bindings_path = DEFAULT_BINDINGS

    librs_source = librs_path.read_text()
    plugin_source = plugin_path.read_text()
    bindings_source = bindings_path.read_text() if bindings_path else None

    violations, warnings, restype_source_name = check(
        librs_source, plugin_source, bindings_source
    )

    print("FFI contract check")
    print(f"  restype source : {restype_source_name}")
    if bindings_path:
        print(f"  bindings file  : {bindings_path}")
    print(f"  exports        : {len(parse_exports(librs_source))}")
    print(f"  required       : {len(parse_required_void_p(plugin_source))}")
    print(f"  called (json)  : {len(parse_call_json_symbols(plugin_source))}")
    argtypes_source = bindings_source if bindings_source is not None else plugin_source
    print(f"  argtypes       : {len(parse_argtypes_assignments(argtypes_source))}")
    for violation in violations:
        print(f"  VIOLATION {violation}")
    for warning in warnings:
        print(f"  WARNING {warning}")

    status = "PASS" if not violations else "FAIL"
    print(
        f"FFI contract check: {len(violations)} violation(s), "
        f"{len(warnings)} warning(s) -> {status}"
    )
    return 1 if violations else 0


if __name__ == "__main__":
    sys.exit(main())
