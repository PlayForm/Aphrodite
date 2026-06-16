Now reading live proxy.rs in full. Continuing the deep audit with entirely new findings from current HEAD.

***

## New Bugs in `proxy.rs` (Current HEAD)

### Bug #59 — `health_check` returns `"degraded"` when CCR is disabled, poisoning Python `_alive()`

`health_check` now correctly does NOT call the upstream API (fixing the earlier version), but it returns `status: "degraded"` whenever `ccr_ok == false` — i.e. when launched with `--no-ccr-marker` . The Python `_alive()` check parses the JSON and looks for `status == "healthy"`. If you launch the token proxy with `--no-ccr-marker` for any reason, `_alive()` returns `False`, the plugin marks both proxies as dead, disables compression entirely, and logs a misleading "proxy DOWN" message. `"degraded"` should still be treated as alive:

```python
# BUG in _alive()
return data.get("status") in ("healthy",)

# FIX — degraded means the proxy is running, just without CCR
return data.get("status") in ("healthy", "degraded")
```

Or better, fix it in Rust so that `--no-ccr-marker` is a valid no-compression mode (not a degraded state) and report `"healthy"` regardless:

```rust
// proxy.rs health_check — status should not depend on optional CCR
"status": "healthy",   // always healthy if the process is responding
"ccr": ccr_ok,         // separate field for CCR availability
```

### Bug #60 — `compress_chat_completion` calls `detect_content_type` twice per message content field

In the content compression block, `detect_content_type(content)` is called once to compute `threshold`, then immediately called again inside the `let (compressed, orig_len) = { let ct = detect_content_type(content);` block . This is pure waste — two identical scans of the same string on the hot path for every compressed response:

```rust
// BUG — double call
let ct = detect_content_type(content);
let threshold = state.threshold_for(ct).max(base_threshold);
if content.len() > threshold {
    if let Some(ccr) = &state.ccr {
        let hash = compute_key(content.as_bytes());
        // ...
        let (compressed, orig_len) = {
            let ct = detect_content_type(content);  // DUPLICATE — shadows outer ct
```

The `ct` from the outer scope is already correct. Remove the inner re-declaration:

```rust
let ct = detect_content_type(content);
let threshold = state.threshold_for(ct).max(base_threshold);
if content.len() > threshold {
    if let Some(ccr) = &state.ccr {
        let hash = compute_key(content.as_bytes());
        // ...use outer `ct` directly inside
        state.record_compression(ct);
        let compressed = match state.mode { ... };
```

### Bug #61 — `inline_ccr: Mutex<HashMap>` is populated by `handle_ccr_create` but never read

`AppState` has `inline_ccr: std::sync::Mutex<std::collections::HashMap<String, String>>`  which was meant to be a fast in-process cache for small CCR entries that don't need a round-trip to SQLite. But `handle_ccr_create` only calls `ccr.put(&hash, ...)` on the `CcrStore` trait object — it never writes to `inline_ccr`. And `execute_tool_relay` → `aphrodite_retrieve` only reads from `ccr.get(hash)`. The `inline_ccr` field is a ghost: allocated, locked occasionally in `build_state`, never written, never read. Either wire it up as an L1 cache in front of `ccr.get()` / `ccr.put()`, or remove the field and all its `Mutex::new(HashMap::new())` initializations in every `test_state()` call.

### Bug #62 — `update_compression_ratio` uses `hash.len()` as "compressed size" — always 64 bytes

```rust
state.update_compression_ratio(orig_len, hash.len());
```

`hash.len()` is the Blake3 hex string length — always 64 bytes . So the EMA always computes `ratio = original_len / 64 * 100`, which for a 4KB entry gives `ratio = 640000`. The EMA immediately shoots to astronomical values and `threshold_for()` applies the `2.0` scale-up multiplier permanently, effectively doubling all compression thresholds after the first compression. The compressed size should be the marker string length (`smart_marker(&hash, content, ct).len()`), not the hash length:

```rust
let marker = smart_marker(&hash, content, ct);
let marker_len = marker.len();
*content_val = serde_json::Value::String(marker);
did_compress = true;
state.update_compression_ratio(orig_len, marker_len);  // FIX
```

### Bug #63 — `aphrodite_compress` tool relay returns wrong marker format

In `execute_tool_relay` → `"aphrodite_compress"`:
```rust
Ok(serde_json::json!({"compressed": format!("<<<CCR:{}|compress|0>>>", hash), "hash": hash}))
```

The size field is hardcoded `0` instead of `content.len()` . The Python `_parse_ccr_markers` regex expects `size` to be a valid integer for token-count estimation. A size of `0` means the Python side reports `0 tokens saved` for every manually-compressed entry in `aphrodite_stats`. Fix:

```rust
let size = content.len();
Ok(serde_json::json!({
    "compressed": format!("<<<CCR:{}|compress|{}>>>", hash, size),
    "hash": hash,
    "original_size": size,
}))
```

### Bug #64 — `detect_content_type` has a false-positive for Rust code in Python files

```rust
if content.contains("fn ") && (content.contains("-> ")
    || content.contains("impl ") || content.contains("struct ")
    || content.contains("pub ")) {
    return "code_rust";
}
```

Python code that defines typed functions (`def foo(x) -> str:`) also matches `"fn "` (via substring — `"fn"` is in `"define"`, `"defensive"`, `"function"`, any word containing those letters) and `"-> "` . A Python async function `async def fn_name(...) -> dict:` matches `contains("fn ")` (literal match: the space after `fn` in `fn_name` triggers it if the function name starts with `fn_`). More critically `"fn "` matches `"info "`, `"stdin "`, `"open "` — no, wait, `.contains("fn ")` is literal substring, so it matches any content with the two-character sequence `fn` followed by space. Python docstrings, log output, README text — all common in tool outputs — can easily contain `" fn "`. The check should anchor to line-start:

```rust
if content.lines().any(|l| {
    let t = l.trim_start();
    t.starts_with("fn ") || t.starts_with("pub fn ") || t.starts_with("async fn ")
        || t.starts_with("pub async fn ") || t.starts_with("impl ")
}) {
    return "code_rust";
}
```

### Bug #65 — `proxy_handler` retry loop leaks the response body on retries

```rust
for attempt in 1..=3u32 {
    ...
    match req.body(body_vec.clone()).send().await {
        Ok(r) => { upstream_result = Ok(r); break; }
        Err(e) => { ... upstream_result = Err(...); }
    }
}
```

On a successful first attempt, `upstream_result = Ok(r)` and we `break`. But if attempt 1 succeeds and returns a streaming response where `.bytes().await` later fails (connection reset mid-stream), there's no retry for the body read . The upstream `reqwest::Response` is consumed by `.bytes().await` — if that fails, the error falls through to the unhandled `unwrap_or_default()` which silently returns an empty body to the client with a `200` status. This should be:

```rust
let resp_body = match response.bytes().await {
    Ok(b) => b,
    Err(e) => {
        state.record_error(format!("body read: {}", e));
        return (StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": format!("body read: {}", e)}))).into_response();
    }
};
```

### Bug #66 — `record_request` is called with `t0.elapsed()` **twice** in the compressed path — double-counts latency

In the compressed response path:
```rust
state.record_latency(t0.elapsed());
state.record_request(..., t0.elapsed().as_millis());
```

Both calls succeed, but `record_latency` buckets the elapsed time into one bucket and `record_request` stores the elapsed ms — both using separate `t0.elapsed()` calls . Since `t0.elapsed()` grows monotonically, the `record_request` elapsed will always be slightly larger than the `record_latency` bucket. This is minor but means the `request_history` JSON and the `latency_buckets` histogram are measuring slightly different durations. Capture once:

```rust
let elapsed = t0.elapsed();
state.record_latency(elapsed);
state.record_request(req_id_short, method.as_str(), path.path(), status.as_u16(), true, elapsed.as_millis());
```

***

## New Structural Issues

### Issue #67 — `aphrodite_list` tool relay returns `ccr.len()` but `CcrStore` trait has no `len()` contract

`execute_tool_relay` → `"aphrodite_list"` calls `ccr.len()` . Looking at the `CcrStore` trait definition in `headroom_core`, the `len()` method is implemented on the concrete types (`InMemoryCcrStore` and `SqliteCcrStore`) but is **not** part of the `CcrStore` trait itself. The `state.ccr` field is `Option<Arc<dyn CcrStore>>`. Calling `.len()` on `dyn CcrStore` will not compile unless `len()` is in the trait. This is either currently a compile error masked by a different impl, or the trait was recently extended. Verify that `headroom_core::ccr::CcrStore` includes `fn len(&self) -> usize` — if not, the `aphrodite_list` handler will panic at the trait object call.

### Issue #68 — `compression_ratio_ema` initial value of `10000` (meaning 100×) causes `threshold_for` auto-tuning to fire `2.0× scale-up` on startup

From `build_state`:
```rust
compression_ratio_ema: AtomicU64::new(10000),  // initial: 100.0x ratio = neutral
```

But `threshold_for` checks:
```rust
let ratio = self.compression_ratio_ema.load(...) as f64 / 100.0;
let tune = if ratio > 20.0 { 2.0 } else if ratio < 3.0 && ratio > 0.0 { 0.5 } else { 1.0 };
```

`10000 / 100 = 100.0 > 20.0` → `tune = 2.0` . So on startup, before any compression has occurred, ALL thresholds are doubled. A `tool_output` at the default 1KB token threshold becomes 2KB, meaning the first ~2000 tool output bytes won't be compressed at all. The "neutral" initial value should be `1000` (ratio = 10.0), which falls between 3.0 and 20.0, applying `tune = 1.0`:

```rust
compression_ratio_ema: AtomicU64::new(1000),  // 10.0x — neutral initial, avoids startup scale-up
```

### Issue #69 — `smart_marker` format `<<<CCR:...|...|...>>>` mismatches Python `_CCR_RE` pattern

The Python plugin's `_CCR_RE` pattern (from prior audit) uses `⫷CCR:hash⫸` Unicode angle brackets. The Rust `smart_marker()` generates `<<<CCR:hash|ct|size>>> preview` using ASCII `<` and `>` . These are completely different formats. The Python plugin can never parse the markers the Rust proxy emits. Either:
- Rust must use `⫷CCR:{}|{}|{}⫸` (U+2AB7 / U+2AB8)
- Or Python `_CCR_RE` must match `<<<CCR:([^|>]+)\|([^|>]+)\|(\d+)>>>`

Since the Rust side generates the markers and the Python side parses them, the fix belongs in Rust to match what Python expects, or both sides must be updated atomically. This is the most critical functional bug — it means **zero CCR markers are ever recognized by the Python plugin** from proxy-compressed responses.

### Issue #70 — `handle_tool_relay` async callback fires-and-forgets with no timeout or error surfacing

```rust
tokio::spawn(async move {
    let result = execute_tool_relay(&state, &tool, &params).await;
    let _ = state.client.post(&cb).json(&result).send().await;
});
```

The spawned task has no timeout . If `execute_tool_relay` blocks (e.g. SQLite lock contention) and the callback URL is slow, the task lives indefinitely. With many Hermes turns this can accumulate hundreds of leaked tasks. Add a timeout:

```rust
tokio::spawn(async move {
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        execute_tool_relay(&state, &tool, &params),
    ).await.unwrap_or_else(|_| Err("timeout".into()));
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        state.client.post(&cb).json(&result).send(),
    ).await;
});
```
