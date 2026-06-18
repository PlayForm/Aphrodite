# Proxy Handlers

Origin: Aphrodite exposes HTTP endpoints for proxying LLM API requests, managing
CCR entries, executing tool relay calls, and health checks.

Source of truth: `crates/aphrodite/src/proxy.rs` (lines 563, 1497, 1635, 1751,
1771, 1796), `crates/aphrodite/src/retrieve.rs` (line 33)

## proxy_handler (line 563)

Catch-all handler. Forwards any request to the upstream LLM API.

### Endpoint

```
ANY /{*path}  (e.g., POST /v1/chat/completions)
```

Registered at `main.rs:349` as fallback route.

### Signature

```rust
pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    path: axum::extract::OriginalUri,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse
```

### Flow

```
1. Increment requests_total, request_body_bytes
2. Generate UUID request ID (short: first 8 chars)
3. If dev mode: log incoming headers (authorization redacted)
4. Build upstream URL: {api_url}/{path}
5. Determine if Chat Completions request
6. Compute cache key (FNV-1a(api_key:model:messages)) for Chat Completions
7. Check LLM response cache:
   a. HIT → return cached response with X-Aphrodite-Cache: HIT
   b. MISS → continue
8. Retry loop (3 attempts):
   a. Build reqwest request (strip host, auth, content-length, x-aphrodite-* headers)
   b. Forward to upstream
   c. On transport error: exponential backoff + jitter
9. On success: extract response
   a. Track upstream_errors_4xx/5xx
   b. Read response body
   c. Track upstream_latency_micros, response_body_bytes
   d. If Chat Completions + CCR enabled:
      - Extract x-headroom-budget from inbound headers
      - compress_chat_completion()
      - If compressed: set X-Aphrodite-Compressed: true, store in response_cache
   e. Otherwise: return raw, store in response_cache
10. On failure: track upstream_timeouts, return 502 BAD_GATEWAY
```

### Response Headers

| Header                   | Value                           | When                                 |
| ------------------------ | ------------------------------- | ------------------------------------ |
| `Content-Type`           | application/json; charset=utf-8 | Always                               |
| `X-Aphrodite-Cache`      | HIT or MISS                     | Chat Completions                     |
| `X-Aphrodite-Compressed` | true                            | When compression occurred            |
| `X-Aphrodite-Fill-Pct`   | float (0.0-99.0)                | Chat Completions (from fill_pct/100) |

### Forwarded Headers

Strip before forwarding:

- `host`
- `authorization` (replaced with configured API key)
- `content-length` (recalculated from body)
- `x-aphrodite-*` (internal)

## handle_tool_relay (line 1497)

Executes aphrodite tools (retrieve, compress, list) with optional async
callback.

### Endpoint

```
POST /tool/relay
```

Registered at `main.rs:333`.

### Request

```json
{
	"tool": "aphrodite_retrieve",
	"params": { "hash": "abc123..." },
	"callback_url": "https://..." // optional
}
```

### Response

```json
{
    "success": true,
    "result": { ... },
    "error": null,
    "async_call": false
}
```

### Tools Handled

- `aphrodite_retrieve`: inline_ccr → CCR store
- `aphrodite_compress`: inline (<256B) or CCR store
- `aphrodite_list`: ccr.len()

### Validation

- `aphrodite_retrieve` requires `hash` param (400 if missing, even with query)

### Callback (async mode)

When `callback_url` is provided:

1. SSRF protection: only `https://` URLs accepted
2. Tool execution spawns on `task_tracker`
3. Result POSTed to callback_url with 5s timeout
4. Response: `async_call: true`, stateless

### Auth

Callback uses Bearer token via `notify_key` (from config).

## handle_ccr_create (line 1635)

Programmatic CCR entry creation. Accepts JSON or raw octet-stream.

### Endpoint

```
POST /ccr/create
```

Registered at `main.rs:334`.

### JSON Mode (Content-Type: application/json)

```json
{
	"content": "string content to store",
	"key": "optional_custom_hash",
	"ttl_seconds": 3600,
	"tags": ["tag1", "tag2"]
}
```

### Octet-Stream Mode

Raw bytes treated as content. Hash computed via `compute_key()` (BLAKE3).

### Response

```json
{
	"hash": "abc123...",
	"token_savings_ratio": 2.5,
	"original_size": 100,
	"compressed_size": 40,
	"marker_size": 40
}
```

### Notification

If `notify_url` configured: fires async POST with `CcrNotification`:

```json
{
	"event": "ccr_created",
	"hash": "...",
	"created_at": 1234567890,
	"ttl": 3600,
	"tags": ["tag1"]
}
```

Auth: Bearer token via `notify_key`. Timeout: 5s.

## handle_ccr_list (line 1751)

List CCR entry count.

### Endpoint

```
GET /ccr/list
```

### Response

```json
{
	"entries": 42,
	"backend": "sqlite",
	"mode": "token"
}
```

## handle_ccr_delete (line 1771)

Delete a CCR entry by hash.

### Endpoint

```
DELETE /ccr/{hash}
```

### Response (200 OK)

```json
{ "deleted": true, "hash": "abc123" }
```

### Response (404)

```json
{ "deleted": false, "hash": "abc123", "error": "not found" }
```

### Response (503)

```json
{ "error": "CCR not enabled" }
```

## health_check (line 1796)

Health check endpoint. Always returns 200 - capability state conveyed in body.

### Endpoint

```
GET /health
```

Registered at `main.rs:357`. Public (no loopback enforcement).

### Response

```json
{
	"status": "healthy",
	"ccr": true,
	"mode": "token",
	"version": "0.5.69",
	"fill_pct": 90.0
}
```

## handle_retrieve (retrieve.rs:33)

Resolve CCR markers to original content.

### Endpoint

```
POST /retrieve
```

Registered at `main.rs:332`.

### Request

```json
{
	"hash": "abc123...",
	"query": "optional filter string",
	"offset": 0,
	"limit": 0
}
```

### Response

```json
{
	"found": true,
	"content": "original content...",
	"source": "ccr",
	"error": null
}
```

### Retrieve Flow

1. Validate hash (required, 400 if missing)
2. Check inline_ccr (LruCache, lock dropped before await)
3. Fallback to CCR backend (SQLite/InMemory via blocking thread)
4. Decompress zstd if magic bytes present (0x28, 0xB5, 0x2F, 0xFD)
5. Apply query filter (case-insensitive, max 512 chars)
6. Apply pagination (offset + limit)
7. Return with source tracking
