# Callbacks

Origin: Tool relay and CCR create operations can optionally notify the Hermes agent via HTTP callback when done, enabling asynchronous patterns. The callback fires on the proxy's TaskTracker, ensuring graceful shutdown waits for in-flight callbacks.

Source of truth: `crates/aphrodite/src/proxy.rs:handle_tool_relay()` (line 1522), `handle_ccr_create()` (line 1659)

## Architecture

```
Hermes Agent
    │
    │ POST /tool/relay
    │  {tool, params, callback_url}
    ▼
Aphrodite Proxy
    │
    ├─ Validate callback_url (https only)
    ├─ Execute tool synchronously
    ├─ Build ToolRelayResponse
    ├─ POST callback_url (5s timeout, Bearer auth)
    │  → Hermes receives result asynchronously
    │
    └─ Return {async_call: true} immediately
```

## Tool Relay Callback

### Request

```json
POST /tool/relay
{
    "tool": "aphrodite_retrieve",
    "params": {"hash": "abc123..."},
    "callback_url": "https://hermes.internal/callback"
}
```

### Response (immediate)

```json
{
    "success": true,
    "result": null,
    "error": null,
    "async_call": true
}
```

### Callback Delivery

After tool execution, a POST is made to `callback_url`:

```json
POST https://hermes.internal/callback
Authorization: Bearer {notify_key}
Content-Type: application/json

{
    "success": true,
    "result": {"found": true, "content": "..."},
    "error": null,
    "async_call": false
}
```

## CCR Create Notification

Fires when `handle_ccr_create` creates a new entry AND `notify_url` is configured.

### Notification Payload

```json
POST {notify_url}
Authorization: Bearer {notify_key}
Content-Type: application/json

{
    "event": "ccr_created",
    "hash": "abc123...",
    "created_at": 1234567890,
    "ttl": 3600,
    "tags": ["tag1", "tag2"]
}
```

## Security

### SSRF Protection

From `proxy.rs:1522`:
```rust
let parsed_url = match url::Url::parse(cb) {
    Ok(u) if u.scheme() == "https" => u,
    _ => {
        tracing::warn!("callback_url rejected: only https scheme allowed");
        return Json(ToolRelayResponse { success: true, result: None, error: None, async_call: false });
    }
};
```

Only `https://` URLs accepted. HTTP, file, and other schemes silently dropped.

### Authentication

```rust
if let Some(k) = &key {
    req = req.header("Authorization", format!("Bearer {k}"));
}
```

Both callback and notification use Bearer token auth via `notify_key`.

## Timeouts

| Operation | Timeout | Source |
|-----------|---------|--------|
| Callback POST | 5 seconds | proxy.rs:1542 |
| Notification POST | 5 seconds | proxy.rs:1680 |

No retries — fire-and-forget. Success/failure tracked via `notify_success` / `notify_failure` counters.

## Task Tracker

All callback/notification tasks spawn on `task_tracker`:

```rust
let tracker = state.task_tracker.clone();
tracker.spawn(async move {
    let result = execute_tool_relay(&state, &tool, &params).await;
    let _ = state.client
        .post(&cb)
        .json(&result)
        .timeout(Duration::from_secs(5))
        .send()
        .await;
});
```

On shutdown:
```rust
task_tracker.close();    // stop accepting new tasks
task_tracker.wait().await;  // wait for in-flight tasks
```

Ensures no callback is lost during graceful shutdown.

## Configuration

From `config.rs:ProxyConfig`:
```toml
[[proxies]]
notify_url = "https://hermes.internal/aphrodite/callback"
notify_key = "hermes-api-key-123"
```

Both must be set for callbacks to fire. If `notify_url` is `None` (default), no callbacks are sent.

## Metrics

| Counter | Description |
|---------|-------------|
| `notify_success` | Incremented when callback/notification POST returns success status |
| `notify_failure` | Incremented on timeout, connection error, or non-success status |

Exposed at `/metrics` as `aphrodite_notify_success` and `aphrodite_notify_failure`.
