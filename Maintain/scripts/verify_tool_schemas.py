"""Verify the rebuilt dylib's tool schemas over the real C ABI.

Mirrors what Hermes does at registration time (aphrodite_hermes_get_schemas)
and what tool_describe does per tool (aphrodite_hermes_get_schema).
"""

import ctypes
import json
import sys

lib = ctypes.CDLL("target/release/libaphrodite_hermes.dylib")
lib.aphrodite_hermes_get_schema.argtypes = [ctypes.c_char_p]
lib.aphrodite_hermes_get_schema.restype = ctypes.c_void_p
lib.aphrodite_hermes_get_schemas.restype = ctypes.c_void_p
lib.aphrodite_hermes_free_string.argtypes = [ctypes.c_void_p]


def consume(ptr: int) -> str:
    value = ctypes.cast(ptr, ctypes.c_char_p).value.decode()
    lib.aphrodite_hermes_free_string(ptr)
    return value


schemas = json.loads(consume(lib.aphrodite_hermes_get_schemas()))
print(f"TOOLS: {len(schemas)}")
for s in schemas:
    props = s["parameters"]["properties"]
    first = s["description"].split(".")[0]
    print(
        f"  {s['name']:<28} desc={len(s['description']):>4}c params={len(props)} lead={len(first)}c"
    )

print("\n--- tool_describe('aphrodite_stats') as the agent would see it ---")
print(
    json.dumps(
        json.loads(consume(lib.aphrodite_hermes_get_schema(b"aphrodite_stats"))),
        indent=2,
    )
)

print("\n--- tool_describe('aphrodite_retrieve') ---")
print(
    json.dumps(
        json.loads(consume(lib.aphrodite_hermes_get_schema(b"aphrodite_retrieve"))),
        indent=2,
    )
)

sys.exit(0)
