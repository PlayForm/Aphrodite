# Direct Dylib Micro-Benchmark (µs/op)

Times a compress round-trip through `libaphrodite_hermes.dylib` WITHOUT a
full Hermes session. Session/hook overhead swamps sub-ms measurements; this
harness isolates the engine's own latency.

## Why the plugin's own load path (never raw ctypes)

The dylib needs its init sequence before dispatch: FFI argtypes/restype
setup and `aphrodite_hermes_materialize_directives`. A hand-rolled ctypes
harness that loads the dylib and calls `aphrodite_hermes_dispatch_tool`
directly SIGSEGVs on the first call - the setup steps are not optional.

## Harness

```python
#!/usr/bin/env python3
import json, os, sys, time

sys.path.insert(0, "<repo-root>/plugins")  # the plugin's own load path
DYLIB = os.path.abspath(sys.argv[1])      # artifact under test
N = int(sys.argv[2]) if len(sys.argv) > 2 else 300
os.environ["APHRODITE_HERMES_DYLIB_PATH"] = DYLIB   # env override wins

import aphrodite  # noqa: E402

PAYLOAD = ("Aphrodite intercepts tool output before it reaches the LLM..." * 20)  # ~5.2KB

dylib = aphrodite._load_dylib()
print(f"version: {aphrodite._call_json(dylib, 'aphrodite_hermes_version')}")
# materialize directives exactly like the plugin init does
aphrodite._call_json(dylib, "aphrodite_hermes_materialize_directives", b"")

name = b"aphrodite_compress"
args = json.dumps({"content": PAYLOAD, "type": "text"}).encode()

for _ in range(5):  # warmup
    aphrodite._call_json(dylib, "aphrodite_hermes_dispatch_tool", name, args)

t0 = time.perf_counter()
for _ in range(N):
    aphrodite._call_json(dylib, "aphrodite_hermes_dispatch_tool", name, args)
t1 = time.perf_counter()

total_ms = (t1 - t0) * 1000
print(f"iterations: {N}\ntotal_ms: {total_ms:.2f}\navg_us_per_op: {total_ms * 1000 / N:.2f}")
```

## Protocol

1. Build the profile you want to measure: `cargo build` (debug) or
   `cargo build --release`.
2. Point the harness at `target/<profile>/libaphrodite_hermes.dylib` - the
   env override is per-process, so the installed dylib and the live session
   are untouched.
3. Run the same iteration count for both profiles; 300 ops on a ~5KB
   payload is stable within ~10%.

## Measured numbers (1.5.0, macOS arm64, 5.2KB payload, 300 ops)

| Metric           | Release     | Debug         | Δ          |
| ---------------- | ----------- | ------------- | ---------- |
| compress latency | 47-52 µs/op | 383-392 µs/op | ~8× slower |
| dylib size       | 4.4 MB      | 12.2 MB       | 2.8×       |
| binary size      | 12.8 MB     | 41 MB         | 3.2×       |

Output is byte-identical across profiles: same content → same CCR hash →
same marker. Debug is a debugging aid (panics carry line numbers, overflow
checks enabled), not a production install.

## Hot-reload state wipe (why you never swap the installed dylib)

The plugin hot-reloads its dylib on mtime change. A reload resets ALL
session CCR state - every existing `<<<CCR:...>>>` marker in the transcript
stops resolving via `aphrodite_retrieve` (the plugin logs this warning at
the reload point in `__init__.py`). Copying a benchmark build over
`~/.hermes/aphrodite/binaries/` mid-session therefore destroys the
session's markers. Always benchmark through the env override in a separate
process; only install a different profile deliberately, at session
boundary, with a backup of the previous build.
