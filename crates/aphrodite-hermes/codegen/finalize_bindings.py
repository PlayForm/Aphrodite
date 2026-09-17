#!/usr/bin/env python3
"""Post-process, validate, and install the ctypesgen FFI bindings.

Invoked by crates/aphrodite-hermes/build.rs AFTER cbindgen produced
``aphrodite_hermes.h`` and ctypesgen produced the raw module::

    python3 codegen/finalize_bindings.py \
        --header <aphrodite_hermes.h> \
        --raw-bindings <_bindings.raw.py> \
        --output plugins/aphrodite/_bindings.py \
        --required aphrodite_hermes_a,aphrodite_hermes_b,...

What it does (design 2 - the FFI restype bug class made structurally
impossible):

1. NEUTRALIZE the import-time library load. ctypesgen emits
   ``_libs["X"] = load_library("X")`` at module import; the plugin's
   hot-reload machinery loads the dylib from a fresh unique-path copy per
   generation, so the library path can never be baked into the artifact.
   The declaration loops are moved inside ``bind_to(dylib)``, which replays
   the generated restype/argtypes onto whichever ``ctypes.CDLL`` handle the
   plugin produced. Importing ``_bindings.py`` NEVER touches a library. The
   loops' ctypesgen ``has()``/``get()`` calls are rewritten to
   ``hasattr()``/``getattr()``, so the artifact carries no lookup-adapter
   class (it binds the plugin's raw CDLL handle directly).

2. REWRITE pointer restypes to ``c_void_p`` and argtypes to ``c_char_p``.
   ctypesgen types ``char *`` returns as its String/ReturnString helper
   pair (a multi-line if/else block) and ``const char *`` returns as a bare
   single-line ``c_char_p`` (upstream ctypdescs.py CtypesFunction); both
   forms - plus WideString (``wchar_t *``) - are rewritten to ``c_void_p``.
   Because the String helper class is then referenced by NOTHING (argtypes
   ``String`` -> ``c_char_p``; ``free_string`` -> ``c_void_p``, since the
   plugin passes it the raw pointer int - mirroring its ``_manual_ffi_setup``),
   the entire string machinery of the preamble
   (UserString/MutableString/String/ReturnString, ~300 lines) and the
   library-loader section (~390 lines) become dead code and are stripped
   from the final artifact. ``errcheck`` is stripped: a String-returning
   errcheck would break ``_call_json``'s raw-pointer protocol (see the
   comment at ``_ERRCHECK_RE`` for the CtypesPointerCast nuance).

3. VALIDATE the contract with a PURE-AST validator - nothing in the
   generated source is ever EXECUTED at build time (ast.parse + compile
   without exec; mirrors Maintain/check_ffi_contract.py's approach): every
   pointer-returning export parsed from the header must be declared with a
   pointer-width restype (``c_void_p`` after the rewrite - NEVER ``c_int``),
   argtypes must match the header's parameter counts, errcheck must be
   absent, and the declared set must equal the header's export set. Any
   violation exits 1 and build.rs PANICS - tools were available, so a
   silently-wrong committed artifact would re-open the SIGSEGV bug class.

4. CONTAIN the namespace bleed: ``__all__ = ["bind_to"]`` on the final
   artifact, so ``from _bindings import *`` re-exports none of the names the
   preamble's ``from ctypes import *`` pulled in.

5. FORK/unknown-shape detection. The rewrite patterns target the upstream
   ctypesgen loop shape (``for _lib in _libs.values():``). The pypdfium2
   fork (pypdfium2-team/ctypesgen, ``pypdfium2`` branch) emits a flat
   ``PN = _libs[L][CN]`` form with single-line ``restype = String`` and no
   if/else block; any non-upstream shape is reported through a
   ``CTYPESGEN_FORK_WARNING:`` marker (build.rs re-emits it as a
   cargo:warning) and finalize exits 2 - the committed artifact stays in
   effect. build.rs additionally classifies ``ctypesgen --version`` and
   warns when the version string does not identify upstream ctypesgen.

6. COPY-ON-CHANGE install to plugins/aphrodite/_bindings.py (only written
   when the content actually changed), with machine-specific paths in the
   ctypesgen banner/comments normalized away so the committed artifact is
   byte-stable across machines.

Exit codes: 0 = generated + installed (or unchanged); 1 = CONTRACT
VIOLATION (build must fail); 2 = other failure (build.rs warns and skips -
the committed artifact stays in effect).
"""

import argparse
import ast
import os
import re
import sys
import tempfile
from contextlib import suppress
from pathlib import Path

PLACEHOLDER_LIB = "__APHRODITE_DYLIB__"

# cbindgen renders the crate's ABI (char*/void only) as:
#   char *aphrodite_hermes_dispatch_tool(const char *tool_name, const char *args_json);
#   void aphrodite_hermes_free_string(char *s);
#   char *aphrodite_hermes_version(void);
# (`char *` has NO space between the asterisk and the name - `\s*`, not `\s+`.
# cbindgen puts pointee const BEFORE the type: `const char *`, never `char const *`.)
_HEADER_FN_RE = re.compile(
    r"^(?:char\s*\*|void)\s*(aphrodite_hermes_\w+)\s*\(([^)]*)\)", re.MULTILINE
)
_HEADER_PTR_RE = re.compile(r"^char\s*\*\s*(aphrodite_hermes_\w+)\s*\(", re.MULTILINE)

# ctypesgen declaration loops start here and run to EOF (upstream shape,
# verified against 2.7.4-27202 output with `-l`):
#   for _lib in _libs.values():
#       if not _lib.has("NAME", "cdecl"):
#           continue
#       NAME = _lib.get("NAME", "cdecl")
#       NAME.argtypes = [...]
#       [if sizeof(c_int) == sizeof(c_void_p): NAME.restype = ReturnString
#        else: NAME.restype = String; NAME.errcheck = ReturnString]
#       break
# (The has()/get() calls are rewritten to hasattr()/getattr() below, so the
# artifact needs NO lookup-adapter class - _libs[PLACEHOLDER] holds the
# plugin's raw CDLL handle and the loops call hasattr/getattr on it.)
_BLOCK_START_RE = re.compile(r"^for _lib in _libs\.values\(\):", re.MULTILINE)
# pypdfium2-team fork (pypdfium2 branch) flat shape:
#   NAME = _libs["LIB"]["NAME"]      (and single-line restype, no if/else)
_FORK_FLAT_RE = re.compile(r"^\s*\w+\s*=\s*_libs\[[^\]]+\]\[", re.MULTILINE)
# ctypesgen resolves symbols through a loader Lookup object (.has/.get with a
# calling_convention kwarg); the plugin's live CDLL is a plain ctypes.CDLL,
# so rewrite the loop calls to hasattr/getattr - purely declarative artifact.
_LOOKUP_CALL_RE = re.compile(r'_lib\.(has|get)\("([A-Za-z_]\w*)"(?:, "[^"]*")?\)')
_LOAD_LINE_RE = re.compile(
    r'_libs\[["\'][^"\']+["\']\] = load_library\(["\'][^"\']*["\']\)'
)
# char* restype declaration: the multi-line if/else block ctypesgen emits for
# every char* return, OR a bare single-line form - one consolidated pattern
# (ordered alternation: the if/else block first, so its inner lines are
# consumed whole, then the single-line forms) replaces the former two-pattern
# pair (_IF_ELSE_RESTYPE_RE + _RESTYPE_SINGLE_RE).
# Single-line forms cover every restype ctypesgen can emit for a pointer
# return: `String`/`ReturnString` (char*, non-const), `c_char_p` (const
# char* - upstream ctypdescs.py CtypesFunction rewrites POINTER(c_char) with
# the const qualifier to CtypesSpecial("c_char_p")), and `WideString`
# (wchar_t*). The plugin contract wants c_void_p for ALL of them.
_RESTYPE_RE = re.compile(
    r"(?:"
    r"    if sizeof\(c_int\) == sizeof\(c_void_p\):\n"
    r"        (?P<name>[A-Za-z_]\w*)\.restype = ReturnString\n"
    r"    else:\n"
    r"        (?P=name)\.restype = String\n"
    r"        (?P=name)\.errcheck = ReturnString\n"
    r"|"
    r"^(?P<indent>[ \t]+)(?P<single>[A-Za-z_]\w*)\.restype = (?:ReturnString|String|c_char_p|WideString)$"
    r")",
    re.MULTILINE,
)
# ctypesgen argtypes use its String helper class (`[String, String]`); the
# helper is dead code once restypes are c_void_p, so rewrite argtypes to
# plain c_char_p and strip the whole string machinery with the preamble.
# Applied AFTER the restype/errcheck rewrites: `\bString\b` then only ever
# matches argtypes entries (ReturnString/WideString have no word boundary
# at the capital, and restype lines no longer contain String).
_ARGYPES_STRING_RE = re.compile(r"\bString\b")
# errcheck lines to strip: `NAME.errcheck = ReturnString` (char* errcheck -
# would break _call_json's raw-pointer protocol) and ctypesgen's
# `NAME.errcheck = lambda v,*a : cast(v, c_void_p)` cast for void* returns.
# NOTE (CtypesNoErrorCheck/CtypesPointerCast): ctypesgen's default errcheck
# is CtypesNoErrorCheck, whose __bool__ is False - printer.py emits NO
# errcheck line for it, so `restype = None` (void fns) has no errcheck line
# to strip. The CtypesPointerCast(c_void_p) cast on void* returns IS
# safe-to-keep (it only casts the POINTER(c_ubyte) restype to c_void_p), but
# stripping it is correct in THIS pipeline: the restype rewrite runs FIRST,
# so any pointer return is already a full-width c_void_p read and _call_json
# clamps restype to c_void_p anyway - the cast would be a no-op.
_ERRCHECK_RE = re.compile(r"^(\s+)([A-Za-z_]\w*)\.errcheck = [^\n]*$", re.MULTILINE)
# ctypesgen comments its declaration loops with the input header's full
# machine-specific path (`# /Volumes/.../out/aphrodite_hermes.h: 16`);
# normalize so the committed artifact is byte-stable on every machine.
_HEADER_COMMENT_RE = re.compile(r"# .*?aphrodite_hermes\.h: ", re.MULTILINE)

# The generated module's docstring embeds the exact ctypesgen command line
# (with machine-specific OUT_DIR paths); the final artifact uses a canonical
# docstring instead, so the committed artifact is byte-identical everywhere.
CANONICAL_DOCSTRING = (
    'r"""Wrapper for aphrodite_hermes.h (generated by ctypesgen)\n'
    "\n"
    "Canonical provenance: crates/aphrodite-hermes/build.rs -> cbindgen ->\n"
    "aphrodite_hermes.h -> ctypesgen -> codegen/finalize_bindings.py.\n"
    "Do not modify this file.\n"
    '"""'
)
# The final head is built from scratch (the raw preamble's string machinery,
# c_ptrdiff_t loop, and loader section are dead code after the rewrites).
MINIMAL_HEAD = (
    "\n"
    "__docformat__ = \"restructuredtext\"\n"
    "\n"
    "from ctypes import *  # noqa: F401, F403 - ctypesgen preamble (c_int, c_void_p, c_char_p, sizeof, ...)\n"
    "\n"
    "# `__all__` contains the public surface: the preamble's `from ctypes import *`\n"
    "# bleeds ctypes' names into this module, and without __all__ a wildcard import\n"
    "# of _bindings would re-export all of them.\n"
    "__all__ = [\"bind_to\"]\n"
    "\n"
    "_libs = {}\n"
)

BINDER_HEADER = """

# ── Runtime binder (post-processed by codegen/finalize_bindings.py) ──────────
# Importing this module NEVER loads a library (no hardcoded dylib path): the
# plugin owns the live CDLL handle (hot-reload unique-path copy) and calls
# bind_to(dylib) to replay the declarations below onto it. The loops call
# hasattr/getattr directly - no lookup adapter class is needed. errcheck is
# stripped unconditionally: pointer restypes are rewritten to c_void_p FIRST
# (a String-returning errcheck would only corrupt the raw-pointer protocol),
# and ctypesgen's CtypesPointerCast(c_void_p) casts on void* returns are safe
# to drop because c_void_p already reads the full pointer width. argtypes use
# plain c_char_p (free_string is [c_void_p] - the plugin passes it the raw
# pointer int, mirroring its _manual_ffi_setup); the String helper class is
# stripped with the dead preamble.

def bind_to(_dylib):
    \"\"\"Replay the generated declarations onto an already-loaded CDLL handle.

    `_dylib` is the plugin's live handle (env override -> canonical home ->
    legacy copies -> fresh unique-path copy per generation), so the library
    path is deliberately never baked in. Pointer restypes are c_void_p
    (full-width reads - _call_json clamps to c_void_p anyway), void fns get
    None. errcheck is never applied: the plugin reads raw pointers and frees
    them through the same handle that produced them.
    \"\"\"
    _libs[{placeholder!r}] = _dylib
""".replace("{placeholder!r}", repr(PLACEHOLDER_LIB))  # UP032: .format() trips the f-string rule; .replace() is equivalent (single slot)


def _restype_repl(m):
    """If/else block -> ``NAME.restype = c_void_p`` (fixed indent; the match
    consumed the trailing newline, so one must be re-emitted); a bare
    single-line declaration -> same rewrite keeping its own indent."""
    if m.group("name") is not None:
        return f"    {m.group('name')}.restype = c_void_p\n"
    return f"{m.group('indent')}{m.group('single')}.restype = c_void_p"


def _lookup_repl(m):
    """ctypesgen ``_lib.has/get("NAME", "cdecl")`` -> ``hasattr/getattr(_lib, "NAME")``."""
    fn = "hasattr" if m.group(1) == "has" else "getattr"
    return f'{fn}(_lib, "{m.group(2)}")'


def parse_header(header_text):
    """{name: parameter_count} for every exported fn, from the C header."""
    fns = {}
    for m in _HEADER_FN_RE.finditer(header_text):
        params = m.group(2).strip()
        n = 0 if params in ("", "void") else len(params.split(","))
        fns[m.group(1)] = n
    return fns


def classify_shape(raw):
    """Identify which ctypesgen variant produced the raw module.

    Returns 'upstream-loop' (ctypesgen/ctypesgen with `-l`: the
    ``for _lib in _libs.values():`` shape - the only shape the rewrite
    patterns target), 'fork-flat' (pypdfium2-team fork: ``NAME = _libs[L][N]``
    with single-line restype), or 'unknown' (neither - treat as unavailable).
    """
    if _BLOCK_START_RE.search(raw):
        return "upstream-loop"
    if _FORK_FLAT_RE.search(raw):
        return "fork-flat"
    return "unknown"


def postprocess(raw, header_path):
    """Return the final _bindings.py source: neutralized load, bind_to(),
    c_void_p restypes / c_char_p argtypes, stripped dead preamble, __all__.
    Raises ValueError on an unexpected ctypesgen shape."""
    shape = classify_shape(raw)
    if shape != "upstream-loop":
        # The rewrite patterns are the UNION of the known upstream forms (the
        # if/else block + every single-line pointer restype). A fork or
        # unknown shape cannot be rewritten safely - report it through the
        # CTYPESGEN_FORK_WARNING marker (build.rs re-emits it as a
        # cargo:warning) and fail gracefully: exit 2 keeps the committed
        # artifact in effect.
        hint = " (pypdfium2-team fork flat form - no _libs.values() loop, single-line restype)" if shape == "fork-flat" else ""
        print(
            f"CTYPESGEN_FORK_WARNING: ctypesgen output shape is '{shape}'{hint}; "
            "the upstream pattern set (for _lib in _libs.values() loops + "
            "if/else restype block + single-line pointer restypes) does not apply; "
            "skipping install - the committed plugins/aphrodite/_bindings.py remains in effect",
            file=sys.stderr,
        )
        raise ValueError(f"unexpected ctypesgen output shape: {shape} (not the upstream loop form)")

    m = _BLOCK_START_RE.search(raw)
    if m is None:
        raise ValueError("no ctypesgen declaration loops found in raw output")
    loops = raw[m.start() :]

    # 1. Never load a library at import time (the plugin owns the handle).
    # The replacement is discarded with the dead preamble - this is a SHAPE
    # check: upstream ctypesgen always emits the load line.
    _, n_load = _LOAD_LINE_RE.subn(
        f'_libs["{PLACEHOLDER_LIB}"] = None  # bound at runtime via bind_to()',
        raw,
    )
    if n_load == 0:
        raise ValueError("no import-time library load line found (unexpected ctypesgen output)")

    # 2. Pointer restypes -> c_void_p; strip errcheck. Order matters: the
    # restype rewrite must run FIRST (it consumes the if/else block including
    # its errcheck line), then the argtypes String -> c_char_p rewrite may
    # safely assume every remaining `String` token is an argtypes entry.
    loops = _RESTYPE_RE.sub(_restype_repl, loops)
    loops = _ERRCHECK_RE.sub("", loops)
    loops = _ARGYPES_STRING_RE.sub("c_char_p", loops)
    # free_string is the ONE arg-carrying entry point that receives a raw
    # pointer INT, not str/bytes: the plugin's _call_json passes it the
    # c_void_p restype value of the call that produced the pointer
    # (plugins/aphrodite/__init__.py::_call_json). ctypesgen's String helper
    # class happened to accept ints - the leniency the plugin relies on - and
    # plain c_char_p rejects them with TypeError. Mirror the plugin's
    # _manual_ffi_setup, which declares `[ctypes.c_void_p]`.
    loops = loops.replace(
        "aphrodite_hermes_free_string.argtypes = [c_char_p]",
        "aphrodite_hermes_free_string.argtypes = [c_void_p]",
    )

    # 2b. _lib.has/get("NAME", "cdecl") -> hasattr/getattr(_lib, "NAME"): the
    # artifact binds the plugin's raw CDLL handle, so no lookup-adapter class.
    loops = _LOOKUP_CALL_RE.sub(_lookup_repl, loops)
    if 'hasattr(_lib, "' not in loops:
        raise ValueError("no has/get lookup calls found in ctypesgen loops (unexpected output)")

    # 3. Wrap the loops in bind_to() (re-indented +4).
    indented = "\n".join(
        ("    " + line) if line.strip() else line for line in loops.splitlines()
    )

    final = CANONICAL_DOCSTRING + MINIMAL_HEAD + BINDER_HEADER + indented + "\n"

    # 4. Normalize machine-specific paths so the artifact is byte-stable
    # (the header line-numbers comments inside the loops embed OUT_DIR).
    final = _HEADER_COMMENT_RE.sub("# aphrodite_hermes.h: ", final)
    return final


def _is_pointer_width_ast(expr_str):
    """Mirror the former runtime _pointer_width() on the AST-unparsed string:
    c_void_p / c_char_p / any POINTER(...) / String / ReturnString. c_int
    (the ctypes default) and None are REJECTED - c_int truncates a 64-bit
    pointer (the SIGSEGV bug class this pipeline exists to prevent)."""
    s = expr_str
    if s in ("c_void_p", "c_char_p", "String", "ReturnString"):
        return True
    return s.startswith("POINTER(")


def validate(final_source, header_fns, header_text, required, raw_path):
    """AST-based FFI contract validation - NOTHING is executed.

    Parses the finalized source with ``ast`` and inspects only ``ast.Assign``
    nodes whose target is ``NAME.restype/argtypes/errcheck`` (the exact
    approach Maintain/check_ffi_contract.py uses), then verifies: the
    declared export set equals the header's, every pointer-returning export
    has a pointer-width restype (c_void_p after the rewrite), argtypes counts
    match the header, errcheck is never applied, and every --required export
    is present. A pure compile() (no exec) additionally catches syntax errors
    the parser alone would miss. Raises ContractViolationError on any breach;
    build.rs turns that into a build failure (the tools were available)."""
    try:
        tree = ast.parse(final_source)
    except SyntaxError as e:  # noqa: S314 - AST parse of trusted generated source
        raise ContractViolationError([f"generated bindings are not valid Python: {e}"]) from e
    compile(final_source, str(raw_path), "exec")  # full syntax check, never executes

    restypes, argtypes, errchecks = {}, {}, {}
    for node in ast.walk(tree):
        if not isinstance(node, ast.Assign) or len(node.targets) != 1:
            continue
        target = node.targets[0]
        if not isinstance(target, ast.Attribute):
            continue  # _libs[...] = ... / NAME = getattr(...) are not declarations
        obj = target.value
        if isinstance(obj, ast.Name):
            name = obj.id
        elif isinstance(obj, ast.Attribute):
            name = obj.attr
        else:
            continue
        if not name.startswith("aphrodite_hermes_"):
            continue
        if target.attr == "restype":
            restypes[name] = ast.unparse(node.value).strip()
        elif target.attr == "argtypes":
            if isinstance(node.value, (ast.List, ast.Tuple)):
                argtypes[name] = len(node.value.elts)
            else:
                argtypes[name] = -1  # not a literal list/tuple -> violation
        elif target.attr == "errcheck":
            errchecks[name] = ast.unparse(node.value).strip()

    declared = set(restypes) | set(argtypes)
    header_names = set(header_fns)
    ptr_names = set(_HEADER_PTR_RE.findall(header_text))

    violations = []

    if declared != header_names:
        missing = header_names - declared
        extra = declared - header_names
        if missing:
            violations.append(
                f"ctypesgen omitted exports: {sorted(missing)} - the plugin "
                "would fall back to manual setup instead of using generated bindings"
            )
        if extra:
            violations.append(
                f"ctypesgen declared unknown exports: {sorted(extra)} - these "
                "would trip the static checker's [unknown-export-configured] rule"
            )

    for name in sorted(header_names & declared):
        if name in ptr_names:
            if name not in restypes:
                violations.append(
                    f"{name}: no restype declaration at all - ctypes defaults to "
                    "c_int, truncating the 64-bit pointer (historical SIGSEGV bug class)"
                )
            elif not _is_pointer_width_ast(restypes[name]):
                violations.append(
                    f"{name}: restype is {restypes[name]!r}, NOT pointer-width - "
                    "ctypes would read the 64-bit pointer return truncated "
                    "(c_int default = the historical SIGSEGV bug class)"
                )
        if name in argtypes:
            if argtypes[name] < 0:
                violations.append(f"{name}: argtypes is not a literal list/tuple")
            elif header_fns[name] != argtypes[name]:
                violations.append(
                    f"{name}: declared argtypes has {argtypes[name]} entries but "
                    f"the header declares {header_fns[name]} parameter(s)"
                )
        else:
            violations.append(f"{name}: no argtypes declaration at all")
        if name in errchecks:
            violations.append(
                f"{name}: errcheck would be applied ({errchecks[name]!r}) - a "
                "String-returning errcheck breaks _call_json's raw-pointer protocol"
            )

    for name in required:
        if name not in declared:
            violations.append(
                f"{name}: required by build.rs (pointer-returning export) but "
                "missing from the generated bindings"
            )

    if violations:
        raise ContractViolationError(violations)


class ContractViolationError(Exception):
    def __init__(self, violations):
        super().__init__("\n".join(violations))
        self.violations = violations


def install(final_source, output_path):
    """Copy-on-change write: only touch the committed artifact when the bytes
    actually differ (atomic replace via temp file)."""
    data = final_source.encode("utf-8")
    if output_path.exists() and output_path.read_bytes() == data:
        print(f"finalize_bindings.py: {output_path} unchanged (copy-on-change)")
        return False
    output_path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(output_path.parent), prefix=".bindings-", suffix=".py")
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(data)
        # mkstemp creates 0600; the committed artifact must be world-readable
        # (644) like the rest of the plugin tree.
        os.chmod(tmp, 0o644)
        os.replace(tmp, output_path)
    except BaseException:
        with suppress(OSError):
            os.remove(tmp)
        raise
    print(f"finalize_bindings.py: wrote {output_path} ({len(data)} bytes)")
    return True


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--header", required=True, help="cbindgen-generated C header")
    ap.add_argument("--raw-bindings", required=True, help="raw ctypesgen module")
    ap.add_argument("--output", required=True, help="final _bindings.py path")
    ap.add_argument("--required", default="", help="comma-separated required export names")
    args = ap.parse_args(argv)

    header_path = Path(args.header)
    raw_path = Path(args.raw_bindings)
    output_path = Path(args.output)
    required = [s for s in args.required.split(",") if s]

    try:
        raw = raw_path.read_text(encoding="utf-8")
        header_text = header_path.read_text(encoding="utf-8")
        header_fns = parse_header(header_text)
        final = postprocess(raw, header_path)
        validate(
            final_source=final,
            header_fns=header_fns,
            header_text=header_text,
            required=required,
            raw_path=raw_path,
        )
    except ContractViolationError as e:
        for v in e.violations:
            print(f"CONTRACT VIOLATION: {v}", file=sys.stderr)
        print(
            "finalize_bindings.py: FFI contract violated while the codegen tools "
            "WERE available - build.rs will fail this build (a silently-wrong "
            "committed _bindings.py would re-open the SIGSEGV bug class)",
            file=sys.stderr,
        )
        return 1
    except Exception as e:  # noqa: BLE001 - tool/script failure => warn + skip
        print(f"finalize_bindings.py: failed ({e!r}) - skipping install; the committed "
              f"plugins/aphrodite/_bindings.py remains in effect", file=sys.stderr)
        return 2

    install(final, output_path)
    print(
        f"finalize_bindings.py: validated {len(header_fns)} exports "
        f"({len(required)} required pointer-returning) - all pointer restypes "
        "are c_void_p, argtypes are c_char_p and match the header"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
