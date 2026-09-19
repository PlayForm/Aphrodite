# Proxy Retry

Transient network failures to the upstream LLM API don't fail the entire
request. A bounded retry loop with exponential backoff and jitter avoids both
immediate failure and thundering-herd retry storms - but only failures that
happen before the request reaches the upstream are retried, so a request the
upstream may have accepted is never re-sent.

## Algorithm

```
for attempt in 1..=3:
    build_request()
    match send():
        Ok(response) → return response
        Err(error):
            if attempt < 3 && error.is_connect():
                sleep(backoff)
                continue
            else:
                fail fast → 502 BAD_GATEWAY
```

## Backoff Formula

```
base_ms = 100 × 2^(attempt - 1)
jitter  = random(0.75 .. 1.25)
sleep_ms = base_ms × jitter
```

| Attempt | Base (ms)                    | Range (ms) |
| ------- | ---------------------------- | ---------- |
| 1       | 100                          | 75 - 125   |
| 2       | 200                          | 150 - 250  |
| 3       | (not retried, final attempt) | -          |

As a Rust struct:

```rust
let base_ms = 100 * 2u64.pow(attempt - 1);
let jitter = rand::random::<f64>() * 0.5 + 0.75; // 0.75x to 1.25x
let ms = (base_ms as f64 * jitter) as u64;
```

## Retry Scope

**Only connect-phase failures** are retried - errors reqwest classifies as
`is_connect()`, which means the request never left and resending is safe:
connection refused, DNS resolution failure, TLS handshake error, and connect
timeouts. Everything else fails fast on the first attempt.

Why the narrow scope: a post-send request timeout (`e.is_timeout()` after the
body was already transmitted) may have been accepted by the upstream. Blindly
retrying a non-idempotent `POST /v1/chat/completions` risks double token
billing, and combined with the 300s per-attempt timeout, 3 blind retries could
hold a client for ~15 minutes. Non-connect errors now fail fast on the first
attempt instead.

| Retried (connect-phase) | Not Retried (fail fast)                      |
| ----------------------- | -------------------------------------------- |
| Connection refused      | HTTP 4xx (tracked as `upstream_errors_4xx`)  |
| DNS resolution failure  | HTTP 5xx (tracked as `upstream_errors_5xx`)  |
| TLS handshake error     | Post-send request timeouts                   |
| Connect timeout         | Mid-body connection reset (request was sent) |
|                         | Any other transport error                    |

## Final Failure

After the retry budget is exhausted (or a non-connect error fails on the first
attempt):

- Increment `upstream_timeouts` when the final error is a timeout, otherwise
  increment `upstream_connect_errors` - the two counters are disjoint.
- Record the specific error in the `last_errors` ring buffer (max 100),
  visible via `/stats`.
- Return `502 BAD_GATEWAY` with a deliberately generic JSON error body:

```json
{ "error": "upstream request failed" }
```

The specific transport error is never sent to the client - `reqwest::Error`'s
`Display` can embed the upstream URL/host, which would leak the configured
`api_url` to whoever hit the proxy. The detail lives server-side in
`last_errors`/`/stats` only.

## Upstream Timeout

Separate from retry: the HTTP client has a global timeout that applies to each
individual attempt. A single slow request can consume up to `timeout` seconds
before the retry mechanism (for connect-phase failures only) kicks in.

```rust
.timeout(Duration::from_secs(cli.timeout))  // default 300s, max 600s
```

Timeout clamping:

```rust
let t = cfg.timeout.unwrap_or(300);
if t > 600 {
    tracing::warn!("timeout {}s exceeds maximum 600s, clamping", t);
    600
} else { t }
```

**Streaming exemption**: `"stream": true` requests go out on a separate client
with **no total timeout** - reqwest's client-level `.timeout()` bounds the
whole request including the response body stream, which used to cut off
legitimately slow but progressing SSE streams mid-answer. Hang protection for
streams comes from `connect_timeout` + `tcp_keepalive` instead. See
[Architecture: Streaming (SSE)](https://github.com/PlayForm/Aphrodite/tree/Current/docs/proxy/architecture.md#streaming-sse).
