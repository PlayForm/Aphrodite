# Proxy Retry

Transient network failures to the upstream LLM API don't fail the entire
request. A bounded retry loop with exponential backoff and jitter avoids both
immediate failure and thundering-herd retry storms.

## Algorithm

```
attempt in 1..=3:
    build_request()
    match send():
        Ok(response) → return response
        Err(error):
            if attempt < 3:
                sleep(backoff)
                continue
            else:
                return 502 BAD_GATEWAY
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

**Only transport errors** - connection failures, DNS resolution failures, TLS
handshake errors. NOT HTTP error status codes (4xx, 5xx). When the upstream
responds with an error status, the response body is returned to the client
without retries.

```rust
Err(e) => {
    if attempt < 3 {
        // ... backoff and retry
    } else {
        upstream_result = Err(format!("{}", e));
    }
}
```

## Error Classification

| Retried                                                                 | Not Retried                                 |
| ----------------------------------------------------------------------- | ------------------------------------------- |
| Connection refused                                                      | HTTP 4xx (tracked as `upstream_errors_4xx`) |
| DNS resolution failure                                                  | HTTP 5xx (tracked as `upstream_errors_5xx`) |
| TLS handshake error                                                     | Returned to the client directly             |
| Timeout (reqwest `send()` error - different from upstream HTTP timeout) |                                             |
| Connection reset                                                        |                                             |

## Final Failure

After 3 failed attempts:

- Track `upstream_timeouts` counter
- Record the specific error in the `last_errors` ring buffer (max 100),
  visible via `/stats`
- Return `502 BAD_GATEWAY` with a deliberately generic JSON error body:

```json
{ "error": "upstream request failed" }
```

The specific transport error is never sent to the client - `reqwest::Error`'s
`Display` can embed the upstream URL/host, which would leak the configured
`api_url` to whoever hit the proxy. The detail lives server-side in
`last_errors`/`/stats` only (v1.3.2).

## Upstream Timeout

Separate from retry: the HTTP client has a global timeout:

```rust
.timeout(Duration::from_secs(cli.timeout))  // default 300s, max 600s
```

This timeout applies to each individual attempt. A single slow request can
consume up to `timeout` seconds before the retry mechanism kicks in.

Timeout clamping:

```rust
let t = cfg.timeout.unwrap_or(300);
if t > 600 {
    tracing::warn!("timeout {}s exceeds maximum 600s, clamping", t);
    600
} else { t }
```

**Streaming exemption**: `"stream": true` requests go out on a separate
client with **no total timeout** - reqwest's client-level `.timeout()` bounds
the whole request including the response body stream, which used to cut off
legitimately slow but progressing SSE streams mid-answer. Hang protection for
streams comes from `connect_timeout` + `tcp_keepalive` instead. See
[Architecture: Streaming (SSE)](architecture.md#streaming-sse).
