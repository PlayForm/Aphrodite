#!/usr/bin/env python3
"""Unit tests for codegen/finalize_bindings.py (stdlib unittest, no deps).

Run:  python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py
Covers every regex form (if/else block, single-line ReturnString/String/
c_char_p/WideString), the argtypes String -> c_char_p rewrite, dead-code
strip, __all__ containment, fork/unknown-shape detection, and the AST-based
validate() contract checks.
"""

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import finalize_bindings as fb  # noqa: E402

# ── Synthetic raw ctypesgen module (mini preamble + loader, real shapes) ──
RAW_HEAD = (
    'r"""Wrapper for aphrodite_hermes.h\n'
    "\n"
    "Generated with:\n"
    "/opt/homebrew/bin/ctypesgen -l __APHRODITE_DYLIB__ "
    "-o /Volumes/MACHINE/out/_bindings.raw.py /Volumes/MACHINE/out/aphrodite_hermes.h\n"
    "\n"
    "Do not modify this file.\n"
    '"""\n'
    '\n__docformat__ = "restructuredtext"\n'
    "\n# Begin preamble for Python\n"
    "\nimport ctypes\n"
    "import sys\n"
    "from ctypes import *  # noqa: F401, F403\n"
    "\n_int_types = (ctypes.c_int16, ctypes.c_int32)\n"
    "for t in _int_types:\n"
    "    if ctypes.sizeof(t) == ctypes.sizeof(ctypes.c_size_t):\n"
    "        c_ptrdiff_t = t\n"
    "del t\n"
    "del _int_types\n"
    "\n"
    "class UserString:\n    pass\n"
    "class MutableString(UserString):\n    pass\n"
    "class String(MutableString, ctypes.Union):\n    pass\n"
    "def ReturnString(obj, func=None, arguments=None):\n    return obj\n"
    "def UNCHECKED(type):\n    return type\n"
    "class _variadic_function(object):\n    pass\n"
    "def ord_if_char(value):\n    return value\n"
    "\n# End preamble\n"
    "\n_libs = {}\n"
    "\nimport ctypes.util\n"
    "import glob\n"
    "import os.path\n"
    "import platform\n"
    "import re\n"
    "\nclass LibraryLoader:\n    pass\n"
    "load_library = LibraryLoader()\n"
    "\n"
    "def add_library_search_dirs(other_dirs):\n    pass\n"
    "del LibraryLoader\n"
    "\nadd_library_search_dirs([])\n"
    "\n# Begin libraries\n"
    '_libs["__APHRODITE_DYLIB__"] = load_library("__APHRODITE_DYLIB__")\n'
    "\n# 1 libraries\n# End libraries\n"
    "\n# No modules\n"
    "\n# /Volumes/MACHINE/out/aphrodite_hermes.h: 16\n"
)

IF_ELSE_BLOCK = (
    "for _lib in _libs.values():\n"
    '    if not _lib.has("NAME", "cdecl"):\n'
    "        continue\n"
    '    NAME = _lib.get("NAME", "cdecl")\n'
    "    NAME.argtypes = [String, String]\n"
    "    if sizeof(c_int) == sizeof(c_void_p):\n"
    "        NAME.restype = ReturnString\n"
    "    else:\n"
    "        NAME.restype = String\n"
    "        NAME.errcheck = ReturnString\n"
    "    break\n"
)

SINGLE_LINE = (
    "for _lib in _libs.values():\n"
    '    if not _lib.has("NAME", "cdecl"):\n'
    "        continue\n"
    '    NAME = _lib.get("NAME", "cdecl")\n'
    "    NAME.argtypes = [String]\n"
    "    NAME.restype = {restype}\n"
    "    break\n"
)

HEADER = (
    "char *aphrodite_hermes_dispatch_tool(char const *tool_name, char const *args_json);\n"
    "void aphrodite_hermes_free_string(char *s);\n"
    "char *aphrodite_hermes_version(void);\n"
)


def make_raw(loops):
    return RAW_HEAD + loops


def final_artifact(raw):
    """postprocess + validate against the synthetic header, return the source."""
    final = fb.postprocess(raw, Path("/tmp/aphrodite_hermes.h"))
    fb.validate(
        final_source=final,
        header_fns=fb.parse_header(HEADER),
        header_text=HEADER,
        required=["aphrodite_hermes_dispatch_tool"],
        raw_path=Path("/tmp/_bindings.raw.py"),
    )
    return final


def real_loops():
    """Raw declaration loops for the three synthetic exports (dispatch_tool
    if/else block, free_string void, version if/else block)."""
    return (
        IF_ELSE_BLOCK.replace("NAME", "aphrodite_hermes_dispatch_tool")
        + "\n"
        + (
            "for _lib in _libs.values():\n"
            '    if not _lib.has("aphrodite_hermes_free_string", "cdecl"):\n'
            "        continue\n"
            '    aphrodite_hermes_free_string = _lib.get("aphrodite_hermes_free_string", "cdecl")\n'
            "    aphrodite_hermes_free_string.argtypes = [String]\n"
            "    aphrodite_hermes_free_string.restype = None\n"
            "    break\n"
        )
        + "\n"
        + (
            "for _lib in _libs.values():\n"
            '    if not _lib.has("aphrodite_hermes_version", "cdecl"):\n'
            "        continue\n"
            '    aphrodite_hermes_version = _lib.get("aphrodite_hermes_version", "cdecl")\n'
            "    aphrodite_hermes_version.argtypes = []\n"
            "    if sizeof(c_int) == sizeof(c_void_p):\n"
            "        aphrodite_hermes_version.restype = ReturnString\n"
            "    else:\n"
            "        aphrodite_hermes_version.restype = String\n"
            "        aphrodite_hermes_version.errcheck = ReturnString\n"
            "    break\n"
        )
    )


class TestRestypeRegex(unittest.TestCase):
    """Item 1 - regex coverage for every ctypesgen restype emission form."""

    def test_if_else_block_rewritten(self):
        raw = make_raw(IF_ELSE_BLOCK)
        final = fb.postprocess(raw, Path("/tmp/x.h"))
        self.assertIn("NAME.restype = c_void_p", final)
        self.assertNotIn("ReturnString", final)
        self.assertNotIn("NAME.errcheck", final)
        # the block's trailing newline must be re-emitted (the next loop's
        # comment must not be glued to the restype line)
        self.assertRegex(final, r"NAME\.restype = c_void_p\n")

    def test_single_line_returnstring(self):
        final = fb.postprocess(
            make_raw(SINGLE_LINE.format(restype="ReturnString")), Path("/tmp/x.h")
        )
        self.assertIn("NAME.restype = c_void_p", final)
        self.assertNotIn("ReturnString", final)

    def test_single_line_string(self):
        final = fb.postprocess(make_raw(SINGLE_LINE.format(restype="String")), Path("/tmp/x.h"))
        self.assertIn("NAME.restype = c_void_p", final)

    def test_single_line_c_char_p(self):
        """const char* return: upstream ctypdescs.py CtypesFunction rewrites
        POINTER(c_char) with the const qualifier to CtypesSpecial('c_char_p'),
        and printer.py emits the bare single-line form."""
        final = fb.postprocess(make_raw(SINGLE_LINE.format(restype="c_char_p")), Path("/tmp/x.h"))
        self.assertIn("NAME.restype = c_void_p", final)
        self.assertNotIn("c_char_p\n", final)

    def test_single_line_widestring(self):
        final = fb.postprocess(make_raw(SINGLE_LINE.format(restype="WideString")), Path("/tmp/x.h"))
        self.assertIn("NAME.restype = c_void_p", final)

    def test_void_restype_none_stays(self):
        """void return: printer emits `NAME.restype = None` with NO errcheck
        line (CtypesNoErrorCheck.__bool__ is False) - must stay None."""
        loops = (
            "for _lib in _libs.values():\n"
            '    if not _lib.has("NAME", "cdecl"):\n'
            "        continue\n"
            '    NAME = _lib.get("NAME", "cdecl")\n'
            "    NAME.argtypes = [String]\n"
            "    NAME.restype = None\n"
            "    break\n"
        )
        final = fb.postprocess(make_raw(loops), Path("/tmp/x.h"))
        self.assertIn("NAME.restype = None", final)


class TestArgtypesRewrite(unittest.TestCase):
    def test_string_argtypes_to_c_char_p(self):
        """dispatch/call_hook contract: argtypes are c_char_p x2, never the
        String helper class (which is stripped with the dead preamble)."""
        final = fb.postprocess(make_raw(IF_ELSE_BLOCK), Path("/tmp/x.h"))
        self.assertIn("NAME.argtypes = [c_char_p, c_char_p]", final)
        self.assertNotIn("[String", final)

    def test_single_argtype(self):
        final = fb.postprocess(
            make_raw(SINGLE_LINE.format(restype="ReturnString")), Path("/tmp/x.h")
        )
        self.assertIn("NAME.argtypes = [c_char_p]", final)

    def test_free_string_argtypes_c_void_p(self):
        """free_string receives a raw pointer int from _call_json (not
        str/bytes): its argtypes must be [c_void_p], mirroring the plugin's
        _manual_ffi_setup - c_char_p would raise TypeError on the int."""
        loops = IF_ELSE_BLOCK.replace("NAME", "aphrodite_hermes_free_string")
        loops = loops.replace("[String, String]", "[String]")
        final = fb.postprocess(make_raw(loops), Path("/tmp/x.h"))
        self.assertIn("aphrodite_hermes_free_string.argtypes = [c_void_p]", final)
        self.assertNotIn("aphrodite_hermes_free_string.argtypes = [c_char_p]", final)


class TestDeadCodeStrip(unittest.TestCase):
    """Item 3 - UserString class, c_ptrdiff_t loop, loader, String machinery
    must be gone from the finalized artifact."""

    def test_dead_code_absent(self):
        final = fb.postprocess(make_raw(IF_ELSE_BLOCK), Path("/tmp/x.h"))
        for dead in (
            "UserString",
            "MutableString",
            "load_library",
            "c_ptrdiff_t",
            "_int_types",
            "LibraryLoader",
            "ReturnString",
            "add_library_search_dirs",
        ):
            self.assertNotIn(dead, final, f"{dead} should be stripped")
        for alive in (
            "from ctypes import *",
            '__all__ = ["bind_to"]',
            "_libs = {}",
            "def bind_to(_dylib):",
            "c_void_p",
            "c_char_p",
        ):
            self.assertIn(alive, final, f"{alive} must survive")

    def test_lookup_rewritten(self):
        final = fb.postprocess(make_raw(IF_ELSE_BLOCK), Path("/tmp/x.h"))
        self.assertIn('if not hasattr(_lib, "NAME"):', final)
        self.assertNotIn(".has(", final)
        self.assertNotIn(".get(", final)

    def test_no_hardcoded_paths(self):
        final = fb.postprocess(make_raw(IF_ELSE_BLOCK), Path("/tmp/x.h"))
        self.assertNotIn("/Volumes/", final)
        self.assertNotIn("/tmp/", final)
        self.assertNotIn("MACHINE", final)

    def test_byte_stable(self):
        raw = make_raw(IF_ELSE_BLOCK)
        a = fb.postprocess(raw, Path("/tmp/x.h"))
        b = fb.postprocess(raw, Path("/tmp/x.h"))
        self.assertEqual(a, b)


class TestAll(unittest.TestCase):
    """Item 4 - __all__ contains the namespace bleed from `from ctypes import *`."""

    def test_wildcard_import_only_bind_to(self):
        final = final_artifact(make_raw(real_loops()))
        with tempfile.TemporaryDirectory() as td:
            mod_path = Path(td) / "_bindings.py"
            mod_path.write_text(final, encoding="utf-8")
            spec = importlib.util.spec_from_file_location("_bindings_test", mod_path)
            assert spec is not None and spec.loader is not None
            mod = importlib.util.module_from_spec(spec)
            sys.modules["_bindings_test"] = mod  # star-import needs a findable module
            spec.loader.exec_module(mod)  # never loads a library - by design
            # `from ctypes import *` bleeds ctypes names into the module, but
            # `__all__ = ["bind_to"]` must contain the wildcard re-export.
            ns = {}
            exec("from _bindings_test import *", ns)
            exported = {k for k in ns if not k.startswith("__")}
            self.assertEqual(exported, {"bind_to"})
            self.assertEqual(set(mod.__all__), {"bind_to"})


class TestForkDetection(unittest.TestCase):
    """Item 2 - pypdfium2 fork / unknown shapes are detected and fail gracefully."""

    FORK_RAW = (
        'r"""Wrapper for aphrodite_hermes.h"""\n'
        "\nfrom ctypes import *\n"
        "import ctypes\n"
        "_libs = {}\n"
        'NAME = _libs["__APHRODITE_DYLIB__"]["NAME"]\n'
        "NAME.argtypes = (String, String)\n"
        "NAME.restype = String\n"
    )

    def test_classify_upstream(self):
        self.assertEqual(fb.classify_shape(make_raw(IF_ELSE_BLOCK)), "upstream-loop")

    def test_classify_fork_flat(self):
        self.assertEqual(fb.classify_shape(self.FORK_RAW), "fork-flat")

    def test_classify_unknown(self):
        self.assertEqual(fb.classify_shape("random junk\n"), "unknown")

    def test_fork_flat_raises_gracefully(self):
        with self.assertRaises(ValueError):
            fb.postprocess(self.FORK_RAW, Path("/tmp/x.h"))


class TestValidate(unittest.TestCase):
    """Item 5 - AST-based contract validation (never executes the source)."""

    def test_validate_passes(self):
        # Real-ish shape: if/else block for dispatch_tool + version,
        # restype=None for free_string - full pipeline, no violations.
        loops = real_loops()
        final = final_artifact(make_raw(loops))  # raises if violations
        # sanity: the validator must not have executed the source
        self.assertIn("def bind_to(_dylib):", final)

    def test_validate_rejects_c_int_restype(self):
        """The historical SIGSEGV class: a c_int restype on a pointer export."""
        loops = (
            "for _lib in _libs.values():\n"
            '    if not _lib.has("aphrodite_hermes_dispatch_tool", "cdecl"):\n'
            "        continue\n"
            '    aphrodite_hermes_dispatch_tool = _lib.get("aphrodite_hermes_dispatch_tool", "cdecl")\n'
            "    aphrodite_hermes_dispatch_tool.argtypes = [String, String]\n"
            "    aphrodite_hermes_dispatch_tool.restype = c_int\n"
            "    break\n"
        )
        final = fb.postprocess(make_raw(loops), Path("/tmp/x.h"))
        with self.assertRaises(fb.ContractViolationError) as cm:
            fb.validate(final, fb.parse_header(HEADER), HEADER, [], Path("/tmp/r.py"))
        self.assertTrue(any("NOT pointer-width" in v for v in cm.exception.violations))

    def test_validate_rejects_errcheck(self):
        """errcheck must never survive: postprocess strips it, so validate()
        must flag a source that still carries one (e.g. an unhandled form)."""
        loops = (
            "for _lib in _libs.values():\n"
            '    if not _lib.has("aphrodite_hermes_dispatch_tool", "cdecl"):\n'
            "        continue\n"
            '    aphrodite_hermes_dispatch_tool = _lib.get("aphrodite_hermes_dispatch_tool", "cdecl")\n'
            "    aphrodite_hermes_dispatch_tool.argtypes = [String, String]\n"
            "    aphrodite_hermes_dispatch_tool.restype = c_void_p\n"
            "    aphrodite_hermes_dispatch_tool.errcheck = ReturnString\n"
            "    break\n"
        )
        final = fb.postprocess(make_raw(loops), Path("/tmp/x.h"))
        # the strip already removed it; inject one to exercise the validator
        final = final.replace(
            "        aphrodite_hermes_dispatch_tool.restype = c_void_p\n",
            "        aphrodite_hermes_dispatch_tool.restype = c_void_p\n"
            "        aphrodite_hermes_dispatch_tool.errcheck = ReturnString\n",
        )
        with self.assertRaises(fb.ContractViolationError) as cm:
            fb.validate(final, fb.parse_header(HEADER), HEADER, [], Path("/tmp/r.py"))
        self.assertTrue(any("errcheck" in v for v in cm.exception.violations))

    def test_validate_rejects_argtypes_count_mismatch(self):
        loops = IF_ELSE_BLOCK.replace("NAME", "aphrodite_hermes_dispatch_tool")
        loops = loops.replace("[String, String]", "[String]")  # header wants 2
        final = fb.postprocess(make_raw(loops), Path("/tmp/x.h"))
        with self.assertRaises(fb.ContractViolationError) as cm:
            fb.validate(final, fb.parse_header(HEADER), HEADER, [], Path("/tmp/r.py"))
        self.assertTrue(any("parameter(s)" in v for v in cm.exception.violations))

    def test_validate_rejects_missing_export(self):
        loops = IF_ELSE_BLOCK.replace("NAME", "aphrodite_hermes_dispatch_tool")
        final = fb.postprocess(make_raw(loops), Path("/tmp/x.h"))
        with self.assertRaises(fb.ContractViolationError) as cm:
            fb.validate(final, fb.parse_header(HEADER), HEADER, [], Path("/tmp/r.py"))
        self.assertTrue(any("omitted exports" in v for v in cm.exception.violations))


if __name__ == "__main__":
    unittest.main(verbosity=2)
