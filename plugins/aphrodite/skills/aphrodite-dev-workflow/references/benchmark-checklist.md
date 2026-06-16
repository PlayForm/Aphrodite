# Comprehensive Benchmark Checklist

Manual end-to-end validation beyond `aphrodite_test`. Use when the user says
"comprehensive benchmark", "test everything", "retest", or "find issues".

## 1. Manual Compress → Retrieve Roundtrip

Pick a real large file (~10KB+, not synthetic test data):

```python
# 1. Compress real content
aphrodite_compress(content=<full file contents>, type="code")

# 2. Retrieve the hash (verify full content restored)
aphrodite_retrieve(hash=<hash>)

# 3. Search for known substrings
aphrodite_search(query="<expected keyword>", type="code")
```

Verify: compression ratio > 1.0x for real content, roundtrip returns exact content,
search finds entries.

## 2. Proxy Health (curl — independent of plugin)

```bash
# Both proxies
curl -s http://127.0.0.1:9797/health   # Cache
curl -s http://127.0.0.1:9798/health   # Token

# Full stats (JSON)
curl -s http://127.0.0.1:9798/stats

# Prometheus metrics
curl -s http://127.0.0.1:9798/metrics
```

Verify: both return `"status":"healthy"`, version matches BIN_VERSION, stats show
non-zero ccr_created and tokens_saved for an active session.

## 3. Automated Suite (last step)

Run after manual checks pass:

```
aphrodite_test with mode=full    # 13 tests: quick + large + search + thresholds
aphrodite_test with mode=matrix  # 13 + 15 settings sweep combinations
aphrodite_test with mode=pipeline # 9 + 4 feature toggles + regression tracking
```

Verify: all tests PASS, no failures, feature toggles all green.

## 4. Terminal / Sandbox Check

Run a terminal command through the proxy to verify terminal hook and sandbox:

```bash
# Long enough to trigger compression (>2048 chars)
echo "..." && curl -s http://127.0.0.1:9798/stats | head -c 500
```

Verify: no sandbox errors, terminal output not eaten, proxy stats increment.

## Common Issues

- **search returns 0**: Content just-compressed may not be indexed yet — wait one
  more compress cycle for the inline store to populate (mirrored in _compress_handler).
- **"done?" mid-benchmark**: User expects FULL cycle — compress + retrieve + search +
  curl + full suite. Not just the automated test tool. Keep going until all 4
  sections above are complete.
- **CCR markers in output**: Terminal output >2KB gets compressed by terminal hook.
  Use aphrodite_retrieve to expand, or pipe to file for raw output.
- **Test regression DEGRADED / delta negative**: The regression delta compares the
  *current mode's test count* against the *previous run's test count*. Pipeline
  (9 tests) run after Full (13 tests) shows delta -4 — this is normal and harmless.
  Only worry when the SAME mode shows DEGRADED with identical test counts (actual
  failures). Each mode has a different test count: quick=9, full=13, matrix=13+15,
  pipeline=9+4 toggles.
- **Test output CCR-compressed**: All 3 modes (full/matrix/pipeline) produce output
  large enough to trigger CCR compression. You must call `aphrodite_retrieve(hash)`
  to read the full results. This is normal operation — the output flows through
  the token proxy which compresses >1KB.
