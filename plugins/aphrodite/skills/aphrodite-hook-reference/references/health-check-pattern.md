# Proxy Health + Retry Patterns (v1.9.0+)

## Python `_alive()` — 5-Second TTL Cache

```python
_alive_cache = {}  # {port: (result, timestamp)}

def _alive(port, timeout=3):
    now = time.time()
    if port in _alive_cache:
        result, ts = _alive_cache[port]
        if now - ts < 5:
            return result
    try:
        r = urllib.request.urlopen(f"http://127.0.0.1:{port}/health", timeout=timeout)
        body = r.read().decode().strip()
        try:
            data = json.loads(body)
            result = data.get("status") in ("healthy", "ok", "degraded")
        except Exception:
            result = body.strip() == "ok"
    except Exception:
        result = False
    _alive_cache[port] = (result, now)
    return result
```

Key points:
- 5-second cache prevents ~12 seconds of socket overhead per turn
- Uses `json.loads()` for proper health status parsing (not fragile string match)
- Accepts "healthy", "ok", "degraded" as valid states
- Falls back to `body.strip() == "ok"` for legacy proxies

## Python `_wait_alive()` — Startup Retry Loop

```python
def _wait_alive(port, retries=10, delay=0.3):
    for _ in range(retries):
        if _alive(port):
            return True
        time.sleep(delay)
    return False
```

Replaces the old `time.sleep(0.5)` fixed wait. Parameters:
- 10 retries × 0.3s = 3s max wait (vs fixed 0.5s that often failed)
- Configurable for slow machines

## Python `_resolve_one()` — Dual Proxy

Tries BOTH proxy ports (9797 cache, 9798 token) sequentially, not just token.
Content compressed via either proxy is retrievable from either since they share CCR stores.

## Rust Health Check — Decoupled

`/health` (GET) — local-only, no upstream API call:
```rust
pub async fn health_check(State(state): ...) -> impl IntoResponse {
    let ccr_ok = state.ccr.is_some();
    Json(json!({"status": if ccr_ok { "healthy" } else { "degraded" }, ...}))
}
```

`/health/upstream` (GET) — separate diagnostic endpoint for DeepSeek probe.

## Rust Inline Retry Backoff

`reqwest::RequestBuilder` doesn't implement `Clone`. Use inline loop:

```rust
let mut upstream_result = Err("unreachable".to_string());
for attempt in 1..=3u32 {
    let req = state.client.request(method.clone(), &url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", state.api_key));
    // ... add forwarded headers ...
    match req.body(body_vec.clone()).send().await {
        Ok(r) => { upstream_result = Ok(r); break; }
        Err(e) => {
            if attempt < 3 {
                let ms = 100 * 2u64.pow(attempt - 1);
                tokio::time::sleep(Duration::from_millis(ms)).await;
            } else {
                upstream_result = Err(format!("{}", e));
            }
        }
    }
}
```
