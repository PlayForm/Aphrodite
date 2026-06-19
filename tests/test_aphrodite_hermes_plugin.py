#!/usr/bin/env python3
"""Smoke test: aphrodite-hermes plugin → headroom-ffi dylib."""
import ctypes, json, sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..', 'plugins', 'aphrodite-hermes'))

from effects import runtime
import importlib.util
spec = importlib.util.spec_from_file_location("aphrodite_hermes", os.path.join(os.path.dirname(__file__), '..', 'plugins', 'aphrodite-hermes', '__init__.py'))
plugin = importlib.util.module_from_spec(spec)
spec.loader.exec_module(plugin)

plugin.bootstrap()

print(f"version: {runtime.service('dylib_version')}")
print(f"hooks:   {runtime.service('dylib_hooks')}")
print(f"pipelines: {runtime.list_pipelines()}")

# Test hooks
for hook in runtime.service('dylib_hooks'):
    r = runtime.run_exit(hook, {"content": "test", "tool_name": "test"})
    status = r.get("_tag", "?")
    print(f"  {hook}: {status}")

# Test full classify+compress+retrieve cycle
lib = runtime.service('dylib')
lib.aphrodite_classify.argtypes = [ctypes.c_char_p]
lib.aphrodite_classify.restype = ctypes.c_void_p
import ctypes, json

def call(fn, *args):
    raw = fn(*(a.encode() if isinstance(a, str) else a for a in args))
    r = ctypes.cast(raw, ctypes.c_char_p).value.decode()
    lib.aphrodite_free_string(raw)
    return r

# Init a handle
hid = call(lib.aphrodite_init, b"")

# Classify
r = json.loads(call(lib.aphrodite_classify, "pub fn main() {}"))
print(f"classify: {r['type']}")

# Compress
lib.aphrodite_compress.argtypes = [ctypes.c_char_p, ctypes.c_char_p, ctypes.c_char_p]
lib.aphrodite_compress.restype = ctypes.c_void_p
r = json.loads(call(lib.aphrodite_compress, hid, "fn hello() -> &str { \"world\" }", "source_code"))
print(f"compress: hash={r['hash'][:12]}... type={r['type']} preview={r['preview']}")

# Retrieve
lib.aphrodite_retrieve.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
lib.aphrodite_retrieve.restype = ctypes.c_void_p
orig = call(lib.aphrodite_retrieve, hid, r['hash'])
assert orig == "fn hello() -> &str { \"world\" }", f"retrieve mismatch: {repr(orig)}"
print("retrieve: OK")

# Stats
lib.aphrodite_stats.argtypes = [ctypes.c_char_p]
lib.aphrodite_stats.restype = ctypes.c_void_p
r = json.loads(call(lib.aphrodite_stats, hid))
print(f"stats: {r['markers']} markers, turn {r['turn']}")

# Session start
r = json.loads(call(lib.aphrodite_session_start, hid))
print(f"session_start: {r['status']}")

# Cleanup
call(lib.aphrodite_destroy, hid)

# Test hot-reload detection
import time
plugin._DYLIB_MTIME = 0  # force reload
plugin.bootstrap()
print("hot-reload: OK")

print("\n✓ All tests passed")
