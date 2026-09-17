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

2. REWRITE pointer restypes to ``c_void_p``. ctypesgen types ``char *``
   returns as its ``String``/``ReturnString`` wrapper; the FFI contract
   (and Maintain/check_ffi_contract.py) requires ``restype = c_void_p`` -
   the plugin reads raw pointers and frees them through the same handle,
   and ``_call_json`` clamps restype to ``c_void_p`` anyway. ``errcheck``
   is stripped: a String-returning errcheck would break ``_call_json``'s
   raw-pointer protocol.

3. VALIDATE the contract (only meaningful when generation actually ran):
   every pointer-returning export parsed from the header must be declared
   with a pointer-width restype (``c_void_p`` after the rewrite - NEVER
   ``c_int``), argtypes must match the header's parameter counts, and the
   declared set must equal the header's export set. Any violation exits 1
   and build.rs PANICS - tools were available, so a silently-wrong
   committed artifact would re-open the SIGSEGV bug class.

4. COPY-ON-CHANGE install to plugins/aphrodite/_bindings.py (only written
   when the content actually changed), with machine-specific paths in the
   ctypesgen banner/comments normalized away so the committed artifact is
   byte-stable across machines.

Exit codes: 0 = generated + installed (or unchanged); 1 = CONTRACT
VIOLATION (build must fail); 2 = other failure (build.rs warns and skips -
the committed artifact stays in effect).
"""

import argparse
import ctypes
import os
import re
import sys
import tempfile
from collections import defaultdict
from contextlib import suppress
from pathlib import Path
from types import SimpleNamespace

PLACEHOLDER_LIB = "__APHRODITE_DYLIB__"

# cbindgen renders the crate's ABI (char*/void only) as:
#   char *aphrodite_hermes_dispatch_tool(char const *tool_name, char const *args_json);
#   void aphrodite_hermes_free_string(char *s);
#   char *aphrodite_hermes_version(void);
# (`char *` has NO space between the asterisk and the name - `\s*`, not `\s+`.)
_HEADER_FN_RE = re.compile(
    r"^(?:char\s*\*|void)\s*(aphrodite_hermes_\w+)\s*\(([^)]*)\)", re.MULTILINE
)
_HEADER_PTR_RE = re.compile(r"^char\s*\*\s*(aphrodite_hermes_\w+)\s*\(", re.MULTILINE)

# ctypesgen declaration loops start here and run to EOF:
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
# consumed whole, then the single-line form) replaces the former two-pattern
# pair (_IF_ELSE_RESTYPE_RE + _RESTYPE_SINGLE_RE).
_RESTYPE_RE = re.compile(
    r"(?:"
    r"    if sizeof\(c_int\) == sizeof\(c_void_p\):\n"
    r"        (?P<name>[A-Za-z_]\w*)\.restype = ReturnString\n"
    r"    else:\n"
    r"        (?P=name)\.restype = String\n"
    r"        (?P=name)\.errcheck = ReturnString\n"
    r"|"
    r"^(?P<indent>[ \t]+)(?P<single>[A-Za-z_]\w*)\.restype = (?:ReturnString|String)$"
    r")",
    re.MULTILINE,
)


def _restype_repl(m):
    """If/else block -> ``NAME.restype = c_void_p`` (fixed indent; the match
    consumed the trailing newline, so one must be re-emitted); a bare
    single-line declaration -> same rewrite keeping its own indent."""
    if m.group("name") is not None:
        return f"    {m.group('name')}.restype = c_void_p\n"
    return f"{m.group('indent')}{m.group('single')}.restype = c_void_p"
_ERRCHECK_RE = re.compile(r"^(\s+)([A-Za-z_]\w*)\.errcheck = [^\n]*$", re.MULTILINE)
# Declared-name scan runs on the POST-rewrite source (hasattr form).
_DECLARED_NAME_RE = re.compile(r'hasattr\(_lib, "([A-Za-z_]\w*)"')

# The generated module's docstring embeds the exact ctypesgen command line
# (with machine-specific OUT_DIR paths); normalize it so the committed
# artifact is byte-identical on every machine.
_DOCSTRING_RE = re.compile(r'r?"""Wrapper for .*?"""', re.DOTALL)
_HEADER_COMMENT_RE = re.compile(r"# .*?aphrodite_hermes\.h: ", re.MULTILINE)

CANONICAL_DOCSTRING = (
    'r"""Wrapper for aphrodite_hermes.h (generated by ctypesgen)\n'
    "\n"
    "Canonical provenance: crates/aphrodite-hermes/build.rs -> cbindgen ->\n"
    "aphrodite_hermes.h -> ctypesgen -> codegen/finalize_bindings.py.\n"
    "Do not modify this file.\n"
    '"""'
)

BINDER_HEADER = """

# ── Runtime binder (post-processed by codegen/finalize_bindings.py) ──────────
# Importing this module NEVER loads a library (no hardcoded dylib path): the
# plugin owns the live CDLL handle (hot-reload unique-path copy) and calls
# bind_to(dylib) to replay the declarations below onto it. The loops call
# hasattr/getattr directly - no lookup adapter class is needed.

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
""".format(placeholder=PLACEHOLDER_LIB)


class _StubDylib:
    """Simulates the plugin's CDLL: getattr returns a per-name record.

    SimpleNamespace-based (lean-up proposal 5): a defaultdict of
    SimpleNamespace records replaces the former _FnRec/_StubDylib pair.
    has()/get() resolve through __getattr__ - any name yields (and lazily
    creates) a record, so bind_to()'s replay runs verbatim.
    """

    def __init__(self):
        self._recs = defaultdict(
            lambda: SimpleNamespace(restype=None, argtypes=None, errcheck=None)
        )

    def __getattr__(self, name):
        return self._recs[name]

    @property
    def recs(self):
        return self._recs


def _pointer_width(t):
    """True when a ctypes restype reads a return at full pointer width.

    Accepts c_void_p / c_char_p / any POINTER(_) subclass plus ctypesgen's
    String/ReturnString char* declarations. c_int (the ctypes default) and
    None are REJECTED - c_int truncates a 64-bit pointer (the SIGSEGV bug
    class this pipeline exists to make structurally impossible).
    """
    if t is None:
        return False
    if t in (ctypes.c_void_p, ctypes.c_char_p):
        return True
    if isinstance(t, type) and issubclass(t, ctypes._Pointer):
        return True
    return getattr(t, "__name__", "") in ("String", "ReturnString")


def parse_header(header_text):
    """{name: parameter_count} for every exported fn, from the C header."""
    fns = {}
    for m in _HEADER_FN_RE.finditer(header_text):
        params = m.group(2).strip()
        n = 0 if params in ("", "void") else len(params.split(","))
        fns[m.group(1)] = n
    return fns


def _lookup_repl(m):
    """ctypesgen ``_lib.has/get("NAME", "cdecl")`` -> ``hasattr/getattr(_lib, "NAME")``."""
    fn = "hasattr" if m.group(1) == "has" else "getattr"
    return f'{fn}(_lib, "{m.group(2)}")'


def postprocess(raw, header_path):
    """Return the final _bindings.py source: neutralized load, bind_to(), and
    c_void_p restypes. Raises ValueError on an unexpected ctypesgen shape."""
    m = _BLOCK_START_RE.search(raw)
    if m is None:
        raise ValueError("no ctypesgen declaration loops found in raw output")
    head = raw[: m.start()]
    loops = raw[m.start() :]

    # 1. Never load a library at import time (the plugin owns the handle).
    head, n_load = _LOAD_LINE_RE.subn(
        f'_libs["{PLACEHOLDER_LIB}"] = None  # bound at runtime via bind_to()',
        head,
    )
    if n_load == 0:
        raise ValueError("no import-time library load line found (unexpected ctypesgen output)")

    # 2. Pointer restypes -> c_void_p; strip errcheck.
    loops = _RESTYPE_RE.sub(_restype_repl, loops)
    loops = _ERRCHECK_RE.sub("", loops)

    # 2b. _lib.has/get("NAME", "cdecl") -> hasattr/getattr(_lib, "NAME"): the
    # artifact binds the plugin's raw CDLL handle, so no lookup-adapter class.
    loops = _LOOKUP_CALL_RE.sub(_lookup_repl, loops)
    if 'hasattr(_lib, "' not in loops:
        raise ValueError("no has/get lookup calls found in ctypesgen loops (unexpected output)")

    # 3. Wrap the loops in bind_to() (re-indented +4).
    indented = "\n".join(
        ("    " + line) if line.strip() else line for line in loops.splitlines()
    )

    final = head + BINDER_HEADER + indented + "\n"

    # 4. Normalize machine-specific paths so the artifact is byte-stable.
    final = _DOCSTRING_RE.sub(CANONICAL_DOCSTRING, final, count=1)
    final = _HEADER_COMMENT_RE.sub("# aphrodite_hermes.h: ", final)
    return final


def validate(final_source, header_fns, header_text, required, raw_path):
    """Run bind_to() against a stub and verify the FFI contract.

    Raises a ContractViolation with a readable message on any breach; build.rs
    turns that into a build failure (the tools were available)."""
    ns = {}
    exec(compile(final_source, str(raw_path), "exec"), ns)  # noqa: S102 - trusted generated source

    declared = set(_DECLARED_NAME_RE.findall(final_source))
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

    stub = _StubDylib()
    try:
        ns["bind_to"](stub)
    except Exception as e:  # noqa: BLE001 - surfaced as a contract violation
        violations.append(f"bind_to(dylib) replay failed against a stub dylib: {e!r}")

    for name in header_names:
        rec = stub.recs.get(name)
        if rec is None:
            continue  # already reported under declared != header_names
        if name in ptr_names:
            if not _pointer_width(rec.restype):
                violations.append(
                    f"{name}: restype is {rec.restype!r}, NOT pointer-width - "
                    "ctypes would read the 64-bit pointer return truncated "
                    "(c_int default = the historical SIGSEGV bug class)"
                )
        if rec.argtypes is None:
            violations.append(f"{name}: no argtypes declaration at all")
        elif header_fns[name] != len(rec.argtypes):
            violations.append(
                f"{name}: declared argtypes {rec.argtypes!r} has "
                f"{len(rec.argtypes)} entries but the header declares "
                f"{header_fns[name]} parameter(s)"
            )
        if rec.errcheck is not None:
            violations.append(
                f"{name}: errcheck would be applied ({rec.errcheck!r}) - a "
                "String-returning errcheck breaks _call_json's raw-pointer protocol"
            )

    for name in required:
        if name not in declared:
            violations.append(
                f"{name}: required by build.rs (pointer-returning export) but "
                "missing from the generated bindings"
            )

    if violations:
        raise ContractViolation(violations)


class ContractViolation(Exception):
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
    except ContractViolation as e:
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
        "are c_void_p, argtypes match the header"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
