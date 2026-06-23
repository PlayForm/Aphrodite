#!/usr/bin/env python3
"""Comprehensive headroom-ffi dylib integration test - 18 tests."""
import ctypes, json, os, sys

DYLIB = os.path.join(os.path.dirname(__file__), '..', 'vendor/headroom/target/release/libheadroom_ffi.dylib')
lib = ctypes.CDLL(DYLIB)

lib.aphrodite_classify.argtypes = [ctypes.c_char_p]
lib.aphrodite_classify.restype = ctypes.c_void_p
lib.aphrodite_compress.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
lib.aphrodite_compress.restype = ctypes.c_void_p
lib.aphrodite_retrieve.argtypes = [ctypes.c_char_p]
lib.aphrodite_retrieve.restype = ctypes.c_void_p
lib.aphrodite_version.argtypes = []
lib.aphrodite_version.restype = ctypes.c_void_p
lib.aphrodite_free_string.argtypes = [ctypes.c_void_p]
lib.aphrodite_free_string.restype = None

def call1(fn, arg):
    ptr = fn(arg.encode())
    r = ctypes.cast(ptr, ctypes.c_char_p).value.decode() if ptr else ''
    lib.aphrodite_free_string(ptr)
    return r

def call2(fn, a, b):
    ptr = fn(a.encode(), b.encode())
    r = ctypes.cast(ptr, ctypes.c_char_p).value.decode() if ptr else ''
    lib.aphrodite_free_string(ptr)
    return r

passed = 0
failed = 0

def check(name, ok):
    global passed, failed
    if ok:
        passed += 1; print(f"  PASS {name}")
    else:
        failed += 1; print(f"  FAIL {name}")

# 1. version
v = ctypes.cast(lib.aphrodite_version(), ctypes.c_char_p).value.decode()
check("version == 0.1.0", v == "0.1.0")

# 2-7. classify (short snippets may fall back to "text")
for content, acceptable in [
    ("diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,4 @@\n+fn new() {}\n", ["diff"]),
    ('{"status": "ok", "data": [{"name": "Alice"}]}', ["json_array"]),
    ("Just plain text.\n", ["text"]),
    ("fn main() {}\n", ["source_code", "text"]),
    ("   Compiling foo v1.0\n    Finished release\n", ["build", "text"]),
    ("src/main.rs:10:5: let x = 1;\n", ["search", "text", "source_code"]),
]:
    r = json.loads(call1(lib.aphrodite_classify, content))
    check(f"classify {acceptable[0]}", r['type'] in acceptable)

# 8-12. compress + retrieve roundtrip
samples = [
    ("fn hello() -> &'static str { \"world\" }", "source_code"),
    ("error[E0308]: mismatched types\n --> src/main.rs:10:5\n", "build"),
    ('{"status": "ok", "data": {"key": "val"}}', "json_array"),
    ("src/main.rs:10:5: let x: i32 = 1;\nsrc/lib.rs:20:3: fn f() {}\n", "search"),
    ("| Name | Value |\n|------|-------|\n| foo  | 1     |\n", "text"),
]
for content, hint in samples:
    r = json.loads(call2(lib.aphrodite_compress, content, hint))
    orig = call1(lib.aphrodite_retrieve, r['hash'])
    ok = (orig == content and len(r['hash']) == 40 and r['size'] == len(content))
    check(f"roundtrip [{hint}] {len(content)}B", ok)

# 13. empty compress
r = json.loads(call2(lib.aphrodite_compress, "", "text"))
check("empty -> error", "error" in r)

# 14. missing retrieve
r = json.loads(call1(lib.aphrodite_retrieve, "deadbeef1234567890abcdef1234567890abcdef"))
check("missing -> error", "error" in r)

# 15. free_string(null)
lib.aphrodite_free_string(None)
check("free_string(null)", True)

# 16-18. preview formats
r = json.loads(call2(lib.aphrodite_compress, "   Compiling foo\nerror: x\nwarning: y\n", "build"))
check("preview build", "[build:" in r['preview'])

r = json.loads(call2(lib.aphrodite_compress, "fn a() {}\nfn b() {}\npub fn c() {}\n", "source_code"))
check("preview code", "fns" in r['preview'] and "[code:" in r['preview'])

r = json.loads(call2(lib.aphrodite_compress, "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-old\n+new\n", "diff"))
check("preview diff", "[diff:" in r['preview'])

print(f"\n{passed} passed, {failed} failed")
sys.exit(0 if failed == 0 else 1)
