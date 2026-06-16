# Atomic Regression Tests & Capability Comparison

The `examples/` directory contains 16 self-contained Python atomic regression tests
and 4 Rust benchmark programs. Every test exercises exactly one bug from the audit.

## Quickest Way In

```bash
# Simplest test (30 chars of bug + fix)
python3 examples/01_env_var_typo.py

# Full integration smoke (compress → marker → retrieve round-trip)
python3 examples/16_integration_smoke.py

# Sequential one-by-one (user preference — see each test pass individually)
for f in examples/[0-1][0-9]_*.py; do echo "--- $f ---"; python3 "$f"; done

# All Python tests at once (batch)
for f in examples/[0-1][0-9]_*.py; do python3 "$f"; done
```

When the user says "run more one by one", execute sequentially with individual
`terminal()` calls (2-3 per turn) so each result is visible. Do NOT batch all 16
into a single `for` loop — the user wants to see each test's output individually.

## Test Pattern

Each test follows a 'buggy vs fixed' sandwich:

1. Define buggy function (reproduces the bug)
2. Define fixed function (the correction)
3. Assert buggy fails, fixed passes
4. Print "NN OK — description"

No proxy, no binary, no API key — pure Python self-verification.
Run: `python3 examples/NN_name.py`. Pass condition: prints `NN OK`.

## Capability Comparison (Normal vs CCR)

Best demo pattern — compress a real file through the proxy and verify round-trip:

```python
# 1. Load real file
with open('plugins/aphrodite/_engine.py') as f:
    original = f.read()

# 2. aphrodite_compress(content, type='code') → stores full file, returns marker hash
# 3. aphrodite_retrieve(hash) → resolves marker to full content
# 4. Verify: retrieved == original (exact match)
```

Typical result with _engine.py (9 KB):
- NORMAL: 9,035 chars in context
- CCR: 24-char marker in context (376x compression)
- RETRIEVED: 9,035 chars on demand
- Round-trip: EXACT MATCH

## CCR Marker in Test Results

`aphrodite_test mode=pipeline` returns results as a CCR marker (not raw JSON).
You MUST `aphrodite_retrieve(hash)` to get the actual test summary.
This is by design — large test output is compressed to save context.

## Rust Benchmarks

```bash
# Build then run
cargo build --release -p aphrodite
cargo run --example bench_01_corpus --release
```

bench_01_corpus: 12 content types (text, code, JSON, logs, CJK), cache + token proxy modes
bench_02_threshold: threshold boundary testing
bench_03_retrieve: retrieval stress
bench_04_ema: exponential moving average on compression stats
